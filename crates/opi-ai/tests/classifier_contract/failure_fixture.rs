//! Shared local peer setup for HTTP acquisition and response syntax checks.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};

use opi_ai::AuthProvenanceSource;
use opi_ai::api::cloudflare_workers_ai_system_one::{
    CloudflareSystemOneOptions, CloudflareWorkersAiSystemOneApi,
    default_cloudflare_classifier_records,
};
use opi_ai::api::llama_cpp_classify::{LlamaCppClassifyApi, LlamaCppClassifyOptions};
use opi_ai::api::openai_decisions::{OpenAiDecisionsApi, OpenAiDecisionsOptions};
use opi_ai::api::system_one::{
    TypesafeSystemOneApi, TypesafeSystemOneOptions, default_typesafe_classifier_record,
};
use opi_ai::auth::AuthScheme;
use opi_ai::classifier::{ClassifierCall, ClassifierResult, ClassifierStopReason};
use opi_ai::http::HttpClient;
use opi_ai::models::Models;
use opi_ai::provider::ProviderError;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::fixtures::bool_context;
use super::llama_cpp_classify::{LlamaServer, mount_llama};

pub(super) fn endpoint(protocol: &str) -> &str {
    match protocol {
        "typesafe" => "/systemone",
        "cloudflare" => "/run",
        "decisions" => "/decisions",
        "llama" => "/completion",
        _ => unreachable!(),
    }
}

pub(super) fn success(protocol: &str) -> ResponseTemplate {
    let answer = serde_json::json!({"q": {"type": "noul", "noul": 0.7}});
    ResponseTemplate::new(200).set_body_json(match protocol {
        "typesafe" => serde_json::json!({"answers": answer}),
        "cloudflare" => serde_json::json!({"success": true, "result": {"answers": answer}}),
        "decisions" => {
            serde_json::json!({"answers": [{"name": "q", "type": "predicate", "probability": 0.7}]})
        }
        _ => unreachable!(),
    })
}

pub(super) async fn classify(
    base_url: String,
    protocol: &str,
    retries: Option<u32>,
    call: ClassifierCall,
) -> ClassifierResult {
    let mut record = match protocol {
        "typesafe" => default_typesafe_classifier_record(),
        "cloudflare" => default_cloudflare_classifier_records().remove(0),
        "decisions" => opi_ai::api::openai_decisions::default_decisions_classifier_record(),
        "llama" => opi_ai::api::llama_cpp_classify::default_llama_classifier_record(),
        _ => unreachable!(),
    };
    record.base_url = base_url;
    let mut models = Models::new();
    let auth = Arc::new(opi_ai::auth::StaticAuthResolver::new(
        AuthScheme::Bearer,
        "fixture-key".into(),
    ));
    let context = bool_context(&["q"]);
    macro_rules! dispatch {
        ($adapter:ty, $options:expr) => {{
            let handle = models
                .register_classifier_api(
                    Arc::new(<$adapter>::new(HttpClient::new(), record.clone())),
                    auth,
                    AuthProvenanceSource::Static,
                )
                .unwrap();
            match retries {
                Some(_) => {
                    models
                        .classify_native(record, context, call, handle.options($options))
                        .await
                }
                None => models.classify(record, context, call).await,
            }
        }};
    }
    match protocol {
        "typesafe" => dispatch!(
            TypesafeSystemOneApi,
            TypesafeSystemOneOptions {
                max_retries: retries.unwrap_or(2),
                ..Default::default()
            }
        ),
        "cloudflare" => dispatch!(
            CloudflareWorkersAiSystemOneApi,
            CloudflareSystemOneOptions {
                max_retries: retries.unwrap_or(2),
                ..Default::default()
            }
        ),
        "decisions" => dispatch!(
            OpenAiDecisionsApi,
            OpenAiDecisionsOptions {
                max_retries: retries.unwrap_or(2),
                ..Default::default()
            }
        ),
        "llama" => dispatch!(
            LlamaCppClassifyApi,
            LlamaCppClassifyOptions {
                max_retries: retries.unwrap_or(2),
                temperature: None,
                max_retry_delay_ms: None
            }
        ),
        _ => unreachable!(),
    }
}

pub(super) async fn assert_syntax_failure(protocol: &str) {
    let endpoints = if protocol == "llama" {
        vec!["/tokenize", "/apply-template", "/completion"]
    } else {
        vec![endpoint(protocol)]
    };
    for endpoint in endpoints {
        for retries in [None, Some(0), Some(1), Some(2)] {
            let server = MockServer::start().await;
            Mock::given(method("POST"))
                .and(path(endpoint))
                .respond_with(ResponseTemplate::new(200).set_body_string("{incomplete"))
                .up_to_n_times(1)
                .mount(&server)
                .await;
            if protocol == "llama" {
                mount_llama(&server, LlamaServer::new()).await;
            } else {
                Mock::given(method("POST"))
                    .respond_with(success(protocol))
                    .mount(&server)
                    .await;
            }
            let payloads = Arc::new(AtomicU32::new(0));
            let responses = Arc::new(AtomicU32::new(0));
            let payload_counter = payloads.clone();
            let response_counter = responses.clone();
            let result = classify(
                server.uri(),
                protocol,
                retries,
                ClassifierCall {
                    callbacks: opi_ai::callbacks::RequestCallbacks::new()
                        .with_on_payload(Arc::new(move |_| {
                            payload_counter.fetch_add(1, AtomicOrdering::SeqCst);
                            Ok(None)
                        }))
                        .with_on_response(Arc::new(move |_, _| {
                            response_counter.fetch_add(1, AtomicOrdering::SeqCst);
                            Ok(())
                        })),
                    ..Default::default()
                },
            )
            .await;
            assert_eq!(
                result.stop_reason,
                ClassifierStopReason::Error,
                "{protocol} {endpoint} {retries:?}"
            );
            assert!(
                matches!(result.error, Some(ProviderError::StreamError(_))),
                "{protocol}: {:?}",
                result.error
            );
            assert_eq!(
                server
                    .received_requests()
                    .await
                    .unwrap()
                    .iter()
                    .filter(|request| request.url.path() == endpoint)
                    .count(),
                1
            );
            let observes = u32::from(protocol != "llama" || endpoint == "/completion");
            assert_eq!(payloads.load(AtomicOrdering::SeqCst), observes);
            assert_eq!(responses.load(AtomicOrdering::SeqCst), 0);
        }
    }
}

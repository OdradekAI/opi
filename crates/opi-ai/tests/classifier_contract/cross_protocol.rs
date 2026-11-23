//! Shared classifier dispatch, HTTP acquisition and request callbacks.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};
use std::time::Duration;

use opi_ai::AuthProvenanceSource;
use opi_ai::api::cloudflare_workers_ai_system_one::CloudflareWorkersAiSystemOneApi;
use opi_ai::api::llama_cpp_classify::LlamaCppClassifyApi;
use opi_ai::api::openai_decisions::OpenAiDecisionsApi;
use opi_ai::api::system_one::TypesafeSystemOneApi;
use opi_ai::auth::AuthScheme;
use opi_ai::classifier::{
    ClassifierAnswer, ClassifierCall, ClassifierContext, ClassifierDeclarations,
    ClassifierModelRecord, ClassifierQuestion, ClassifierStopReason,
};
use opi_ai::http::HttpClient;
use opi_ai::models::Models;
use opi_ai::provider::ProviderError;
use secrecy::SecretString;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::cloudflare_workers_ai_system_one::{
    CloudflareAccountResolver, cloudflare_jev, run_record_response,
};
use super::failure_fixture::{classify, endpoint, success};
use super::llama_cpp_classify::{
    LlamaAuthResolver, LlamaServer, llama_context, llama_models, llama_pick_context, llama_record,
    mount_llama,
};
use super::openai_decisions::{decisions_models, decisions_record};
use super::typesafe_system_one::{classifier_models, classifier_record};

#[tokio::test]
async fn four_classifier_protocols_share_one_models_dispatch() {
    // One Models serves all four real classifier protocols; each record
    // routes through the same public classify dispatch while keeping its
    // own wire shape (System One `noul` vs Decisions `predicate` bool are
    // not interchangeable).
    let typesafe_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/systemone"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "answers": { "q": { "type": "noul", "noul": 0.95 } },
        })))
        .expect(1)
        .mount(&typesafe_server)
        .await;

    let cloudflare_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/accounts/account-id/ai/run"))
        .respond_with(run_record_response(
            "Completed",
            serde_json::json!({ "answers": { "q": { "type": "noul", "noul": 0.9 } } }),
        ))
        .expect(1)
        .mount(&cloudflare_server)
        .await;

    let llama_server = MockServer::start().await;
    mount_llama(&llama_server, LlamaServer::new()).await;

    let decisions_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/decisions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "answers": [ { "type": "predicate", "name": "q", "probability": 0.7 } ],
        })))
        .expect(1)
        .mount(&decisions_server)
        .await;

    let decorate = |mut record: ClassifierModelRecord| {
        record.headers = vec![
            ("x-model".into(), Some("complete-record".into())),
            ("x-override".into(), Some("model".into())),
            ("x-remove".into(), Some("on".into())),
            ("X-Remove".into(), Some("off".into())),
            ("x-remove".into(), None),
        ];
        record
    };
    let call = || ClassifierCall {
        extra_headers: vec![("X-Override".into(), "call".into())],
        ..Default::default()
    };
    let mut models = Models::new();
    let typesafe_record = decorate(classifier_record(typesafe_server.uri()));
    models
        .register_classifier_api(
            Arc::new(TypesafeSystemOneApi::new(
                HttpClient::new(),
                typesafe_record.clone(),
            )),
            Arc::new(opi_ai::auth::StaticAuthResolver::new(
                AuthScheme::Bearer,
                SecretString::from("test-key"),
            )),
            AuthProvenanceSource::Static,
        )
        .expect("register typesafe");
    let cloudflare_record = decorate(cloudflare_jev(&cloudflare_server.uri()));
    models
        .register_classifier_api(
            Arc::new(CloudflareWorkersAiSystemOneApi::new(
                HttpClient::new(),
                cloudflare_record.clone(),
            )),
            Arc::new(CloudflareAccountResolver {
                account_id: Some("account-id".into()),
            }),
            AuthProvenanceSource::Static,
        )
        .expect("register cloudflare");
    let llama_record = decorate(llama_record(&llama_server.uri()));
    models
        .register_classifier_api(
            Arc::new(LlamaCppClassifyApi::new(
                HttpClient::new(),
                llama_record.clone(),
            )),
            Arc::new(LlamaAuthResolver),
            AuthProvenanceSource::Static,
        )
        .expect("register llama");
    let decisions_record = decorate(decisions_record(&decisions_server.uri()));
    models
        .register_classifier_api(
            Arc::new(OpenAiDecisionsApi::new(
                HttpClient::new(),
                decisions_record.clone(),
            )),
            Arc::new(opi_ai::auth::StaticAuthResolver::new(
                AuthScheme::Bearer,
                SecretString::from("secret"),
            )),
            AuthProvenanceSource::Static,
        )
        .expect("register decisions");

    let mut question = ClassifierDeclarations::new();
    question.insert(
        "q".to_owned(),
        ClassifierQuestion::Bool {
            instructions: "Is this true?".into(),
            true_criteria: "Yes".into(),
            false_criteria: "No".into(),
        },
    );
    let bool_context = ClassifierContext {
        state: serde_json::json!({ "text": "ok" }),
        questions: question,
        images: None,
    };

    // System One bool rides as `noul`.
    let result = models
        .classify(typesafe_record, bool_context.clone(), call())
        .await;
    assert_eq!(
        result.stop_reason,
        ClassifierStopReason::Stop,
        "message was {:?}",
        result.error_message
    );
    assert_eq!(
        result.answers.get("q"),
        Some(&ClassifierAnswer::Bool { probability: 0.95 })
    );

    // Cloudflare wraps the same System One shape in its run envelope.
    let result = models
        .classify(cloudflare_record, bool_context.clone(), call())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Stop);
    assert_eq!(
        result.answers.get("q"),
        Some(&ClassifierAnswer::Bool { probability: 0.9 })
    );

    // llama.cpp reads the bool from Yes/No label log-probabilities.
    let result = models.classify(llama_record, llama_context(), call()).await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Stop);
    assert!(result.answers.contains_key("urgent"));

    // Decisions maps the same public bool to a `predicate` question.
    let result = models
        .classify(decisions_record, bool_context, call())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Stop);
    assert_eq!(
        result.answers.get("q"),
        Some(&ClassifierAnswer::Bool { probability: 0.7 })
    );

    // The wires differ: `noul` for System One, `predicate` for Decisions.
    let typesafe_sent = typesafe_server.received_requests().await.unwrap();
    let body: serde_json::Value =
        serde_json::from_slice(&typesafe_sent[0].body).expect("json body");
    assert_eq!(body["questions"]["q"]["type"], "noul");
    let decisions_sent = decisions_server.received_requests().await.unwrap();
    let body: serde_json::Value =
        serde_json::from_slice(&decisions_sent[0].body).expect("json body");
    assert_eq!(body["questions"][0]["type"], "predicate");
    for server in [
        &typesafe_server,
        &cloudflare_server,
        &llama_server,
        &decisions_server,
    ] {
        let requests = server.received_requests().await.unwrap();
        assert!(!requests.is_empty());
        for sent in requests {
            assert_eq!(sent.headers["x-model"], "complete-record");
            assert_eq!(sent.headers["x-override"], "call");
            assert!(!sent.headers.contains_key("x-remove"));
        }
    }
}

#[tokio::test]
async fn classifiers_reused_callbacks_keep_each_calls_payload() {
    for decisions in [false, true] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(if decisions {
                serde_json::json!({"answers": []})
            } else {
                serde_json::json!({"answers": {}})
            }))
            .expect(2)
            .mount(&server)
            .await;
        let record = if decisions {
            decisions_record(&server.uri())
        } else {
            classifier_record(server.uri())
        };
        let models = if decisions {
            decisions_models(&record, "fixture-key").0
        } else {
            classifier_models(&record).0
        };
        let observed = Arc::new(AtomicU32::new(0));
        let counter = observed.clone();
        let callbacks =
            opi_ai::callbacks::RequestCallbacks::new().with_on_payload(Arc::new(move |_| {
                counter.fetch_add(1, AtomicOrdering::SeqCst);
                Ok(None)
            }));
        for state in ["first state", "second state"] {
            let result = models
                .classify(
                    record.clone(),
                    ClassifierContext {
                        state: serde_json::json!({"text": state}),
                        ..Default::default()
                    },
                    ClassifierCall {
                        callbacks: callbacks.clone(),
                        ..Default::default()
                    },
                )
                .await;
            assert_eq!(
                result.stop_reason,
                ClassifierStopReason::Stop,
                "{:?}",
                result.error
            );
        }
        let sent = server.received_requests().await.unwrap();
        assert!(String::from_utf8_lossy(&sent[0].body).contains("first state"));
        assert!(String::from_utf8_lossy(&sent[1].body).contains("second state"));
        assert_eq!(observed.load(AtomicOrdering::SeqCst), 2);
    }
}

#[tokio::test]
async fn classifiers_response_callback_is_outside_attempt_timeout() {
    for decisions in [false, true] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(if decisions {
                serde_json::json!({"answers": []})
            } else {
                serde_json::json!({"answers": {}})
            }))
            .mount(&server)
            .await;
        let record = if decisions {
            decisions_record(&server.uri())
        } else {
            classifier_record(server.uri())
        };
        let models = if decisions {
            decisions_models(&record, "fixture-key").0
        } else {
            classifier_models(&record).0
        };
        let result = models
            .classify(
                record,
                ClassifierContext::default(),
                ClassifierCall {
                    timeout: Some(Duration::from_millis(300)),
                    callbacks: opi_ai::callbacks::RequestCallbacks::new().with_async_on_response(
                        |_, _| async {
                            tokio::time::sleep(Duration::from_millis(600)).await;
                            Ok(())
                        },
                    ),
                    ..Default::default()
                },
            )
            .await;
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
        assert_eq!(
            result.stop_reason,
            ClassifierStopReason::Stop,
            "{:?}",
            result.error
        );
    }
}

#[tokio::test]
async fn classifiers_protected_headers_never_reach_transport() {
    for api in ["typesafe", "decisions", "llama"] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"answers": {}})),
            )
            .mount(&server)
            .await;
        let (record, models, context) = match api {
            "decisions" => {
                let record = decisions_record(&server.uri());
                let models = decisions_models(&record, "fixture-key").0;
                (record, models, ClassifierContext::default())
            }
            "llama" => {
                let record = llama_record(&server.uri());
                let models = llama_models(&record).0;
                (record, models, llama_pick_context())
            }
            _ => {
                let record = classifier_record(server.uri());
                let models = classifier_models(&record).0;
                (record, models, ClassifierContext::default())
            }
        };
        for header in ["Authorization", "aPi-KeY", "Host"] {
            let result = models
                .classify(
                    record.clone(),
                    context.clone(),
                    ClassifierCall {
                        extra_headers: vec![(header.into(), "untrusted-override".into())],
                        ..Default::default()
                    },
                )
                .await;
            assert_eq!(result.stop_reason, ClassifierStopReason::Error);
            assert!(
                matches!(result.error, Some(ProviderError::RequestFailed(_))),
                "{api}: {:?}",
                result.error
            );
        }
        assert!(
            server.received_requests().await.unwrap().is_empty(),
            "{api}"
        );
    }
}

#[tokio::test]
async fn classifiers_response_callbacks_require_successful_json_acquisition() {
    for decisions in [false, true] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_string("{incomplete"))
            .mount(&server)
            .await;
        let record = if decisions {
            decisions_record(&server.uri())
        } else {
            classifier_record(server.uri())
        };
        let models = if decisions {
            decisions_models(&record, "fixture-key").0
        } else {
            classifier_models(&record).0
        };
        let observed = Arc::new(AtomicU32::new(0));
        let counter = observed.clone();
        let result = models
            .classify(
                record,
                ClassifierContext::default(),
                ClassifierCall {
                    callbacks: opi_ai::callbacks::RequestCallbacks::new().with_on_response(
                        Arc::new(move |_, _| {
                            counter.fetch_add(1, AtomicOrdering::SeqCst);
                            Ok(())
                        }),
                    ),
                    ..Default::default()
                },
            )
            .await;
        assert_eq!(result.stop_reason, ClassifierStopReason::Error);
        assert_eq!(observed.load(AtomicOrdering::SeqCst), 0);
    }
}

#[tokio::test]
async fn http_retry_headers_and_business_validation_keep_distinct_boundaries() {
    for protocol in ["typesafe", "cloudflare", "decisions", "llama"] {
        for (status, retry_header, expected, succeeds) in [
            (429, None, 2, true),
            (400, Some("true"), 2, true),
            (503, Some("false"), 1, false),
            (200, Some("true"), 1, false),
        ] {
            let server = MockServer::start().await;
            let mut first = ResponseTemplate::new(status)
                .insert_header("retry-after-ms", "0")
                .set_body_json(serde_json::json!({}));
            if let Some(value) = retry_header {
                first = first.insert_header("x-should-retry", value);
            }
            Mock::given(method("POST"))
                .and(path(endpoint(protocol)))
                .respond_with(first)
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
            let result = classify(server.uri(), protocol, Some(2), Default::default()).await;
            assert_eq!(
                result.stop_reason == ClassifierStopReason::Stop,
                succeeds,
                "{protocol} {status}"
            );
            let sent = server.received_requests().await.unwrap();
            let sent: Vec<_> = sent
                .iter()
                .filter(|request| request.url.path() == endpoint(protocol))
                .collect();
            assert_eq!(sent.len(), expected, "{protocol} {status}");
            if expected == 2 {
                assert_eq!(sent[0].body, sent[1].body);
            }
        }
    }
}

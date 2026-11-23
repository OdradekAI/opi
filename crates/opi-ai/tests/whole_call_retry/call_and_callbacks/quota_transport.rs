//! OpenAI-family quota, transient hints and acquisition budget controls.

use crate::fixtures::{logical_request, transient_policy};
use opi_ai::auth::{AuthProvenanceSource, AuthScheme, StaticAuthResolver};
use opi_ai::models::{CompatMetadata, Models};
use opi_ai::provider::{Provider, ProviderError};
use opi_ai::retry_assistant_call;
use opi_ai::stream::{ChatStream, StopReason};
use secrecy::SecretString;
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};
use wiremock::{Mock, MockServer, ResponseTemplate, matchers};

async fn exercise(responses: bool, via_models: bool, body: serde_json::Value, expected: u32) {
    let server = MockServer::start().await;
    let endpoint = if responses {
        "/v1/responses"
    } else {
        "/v1/chat/completions"
    };
    Mock::given(matchers::method("POST"))
        .and(matchers::path(endpoint))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "1")
                .set_body_json(body),
        )
        .mount(&server)
        .await;
    let provider: Box<dyn Provider> = if responses {
        Box::new(
            opi_ai::providers::openai_responses::OpenAiResponsesProvider::new(Some(server.uri())),
        )
    } else {
        Box::new(opi_ai::providers::openai::OpenAiChatProvider::new(Some(
            server.uri(),
        )))
    };
    let record =
        opi_ai::ChatModelRecord::new(provider.id(), provider.models().first().unwrap().clone());
    let requested_model = provider.models().first().unwrap().id.clone();
    let expected_model = requested_model.clone();
    let mut models = Models::new();
    // Direct and Models consumers use the same frozen local endpoint.
    let direct: Option<Arc<dyn Provider>> = if via_models {
        models
            .register_route(
                provider,
                Arc::new(StaticAuthResolver::new(
                    AuthScheme::Bearer,
                    SecretString::from("quota-test-key"),
                )),
                AuthProvenanceSource::Static,
                CompatMetadata {
                    openai_compatible: true,
                    profile: None,
                },
            )
            .unwrap();
        None
    } else {
        Some(Arc::from(provider))
    };
    let models = Arc::new(models);
    let calls = Arc::new(AtomicU32::new(0));
    let counter = calls.clone();
    let result = retry_assistant_call(
        move || {
            let models = models.clone();
            let direct = direct.clone();
            let record = record.clone();
            let requested_model = requested_model.clone();
            counter.fetch_add(1, Ordering::SeqCst);
            async move {
                let mut request = logical_request();
                request.model = requested_model;
                let failure = if let Some(provider) = direct {
                    ChatStream::new(
                        provider
                            .stream_prepared(request, opi_ai::test_support::resolved_bearer_auth()),
                    )
                    .result()
                    .await
                    .unwrap_err()
                } else {
                    models
                        .complete_simple(record, request, Default::default())
                        .await
                        .unwrap_err()
                };
                assert!(
                    matches!(
                        failure.cause(),
                        ProviderError::RateLimited {
                            retry_after_ms: Some(1000)
                        }
                    ),
                    "typed acquisition cause is preserved: {failure:?}"
                );
                failure.into_parts().0
            }
        },
        Some(&transient_policy(1, 0)),
        tokio_util::sync::CancellationToken::new(),
        None,
    )
    .await
    .unwrap();
    assert_eq!(
        calls.load(Ordering::SeqCst),
        expected,
        "responses={responses} Models={via_models}"
    );
    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent.len(), expected as usize);
    for request in sent {
        let payload: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
        assert_eq!(payload["model"], expected_model);
        assert!(request.body.windows(5).any(|bytes| bytes == b"Hello"));
    }
    assert_eq!(result.stop_reason, StopReason::Error);
    assert!(result.content.is_empty());
    assert!(!format!("{result:?}").contains("PRIVATE_CANARY"));
}

#[tokio::test]
async fn quota_429_veto_reaches_whole_call_retry_through_real_adapters() {
    for responses in [false, true] {
        for via_models in [false, true] {
            for body in [
                serde_json::json!({"error":{"code":"insufficient_quota", "message":"PRIVATE_CANARY"}}),
                serde_json::json!({"error":{"type":"FreeUsageLimitError", "message":"PRIVATE_CANARY"}}),
                serde_json::json!({"error":{"message":"billing limit reached PRIVATE_CANARY"}}),
                serde_json::json!({"code":"subscription_sharing_workplace_individual_usage_limit_exceeded", "message":"PRIVATE_CANARY"}),
                serde_json::json!({"code":"1308", "message":"PRIVATE_CANARY"}),
                serde_json::json!({"error":{"code":1308, "message":"PRIVATE_CANARY"}}),
            ] {
                exercise(responses, via_models, body, 1).await;
            }
        }
    }
}

#[tokio::test]
async fn transient_429_keeps_explicit_whole_call_retry() {
    for responses in [false, true] {
        for via_models in [false, true] {
            exercise(responses, via_models, serde_json::json!({"error":{"code":"rate_limit_exceeded","message":"PRIVATE_CANARY transient"}}), 2).await;
        }
    }
}

#[tokio::test]
async fn unrelated_quota_text_does_not_override_status_classification() {
    for responses in [false, true] {
        exercise(responses, true, serde_json::json!({"metadata":{"message":"billing PRIVATE_CANARY"},"error":{"code":429}}), 2).await;
        exercise(responses, true, serde_json::json!({"metadata":{"code":"1308"},"error":{"message":"request reference 1308 PRIVATE_CANARY"}}), 2).await;
        exercise(
            responses,
            true,
            serde_json::json!({"code":"13080", "message":"PRIVATE_CANARY"}),
            2,
        )
        .await;
        exercise(responses, true, serde_json::Value::Null, 2).await;
    }
}

#[tokio::test]
async fn quota_detail_does_not_change_acquisition_budget_or_payload_freeze() {
    for responses in [false, true] {
        for max_retries in [0, 1] {
            let server = MockServer::start().await;
            Mock::given(matchers::method("POST"))
                .respond_with(
                    ResponseTemplate::new(429)
                        .insert_header("retry-after", "0")
                        .set_body_json(serde_json::json!({"code":"1308"})),
                )
                .mount(&server)
                .await;
            let provider: Box<dyn Provider> = if responses {
                Box::new(
                    opi_ai::providers::openai_responses::OpenAiResponsesProvider::new(Some(
                        server.uri(),
                    )),
                )
            } else {
                Box::new(opi_ai::providers::openai::OpenAiChatProvider::new(Some(
                    server.uri(),
                )))
            };
            let target = format!("{}:{}", provider.id(), provider.models()[0].id);
            let mut models = Models::new();
            models
                .register_route(
                    provider,
                    Arc::new(StaticAuthResolver::new(
                        AuthScheme::Bearer,
                        SecretString::from("quota-test-key"),
                    )),
                    AuthProvenanceSource::Static,
                    CompatMetadata {
                        openai_compatible: true,
                        profile: None,
                    },
                )
                .unwrap();
            let payload_calls = Arc::new(AtomicU32::new(0));
            let response_calls = Arc::new(AtomicU32::new(0));
            let payload_sink = payload_calls.clone();
            let response_sink = response_calls.clone();
            let mut request = logical_request();
            request.model = target.clone();
            request.callbacks = opi_ai::callbacks::RequestCallbacks::new()
                .with_on_payload(Arc::new(move |_| {
                    payload_sink.fetch_add(1, Ordering::SeqCst);
                    Ok(None)
                }))
                .with_on_response(Arc::new(move |_, _| {
                    response_sink.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }));
            let call = models
                .prepare_call_with_policy(
                    &target,
                    request,
                    opi_ai::retry::AcquisitionPolicy::with_max_retries(max_retries),
                )
                .await
                .unwrap();
            for attempt in 0..=max_retries {
                let failure = ChatStream::new(call.start_attempt().unwrap())
                    .result()
                    .await
                    .unwrap_err();
                assert!(matches!(
                    failure.cause(),
                    ProviderError::RateLimited {
                        retry_after_ms: Some(0)
                    }
                ));
                assert!(
                    failure
                        .message()
                        .error_message
                        .as_ref()
                        .unwrap()
                        .contains("quota exceeded")
                );
                let admission = call.retry_admission();
                if attempt < max_retries {
                    let admission = admission.unwrap();
                    tokio::time::sleep(std::time::Duration::from_millis(admission.delay_ms.max(1)))
                        .await;
                } else {
                    assert!(admission.is_none());
                }
            }
            assert_eq!(payload_calls.load(Ordering::SeqCst), 1);
            assert_eq!(response_calls.load(Ordering::SeqCst), 0);
            let sent = server.received_requests().await.unwrap();
            assert_eq!(sent.len(), (max_retries + 1) as usize);
            assert!(sent.iter().all(|request| request.body == sent[0].body));
        }
    }
}

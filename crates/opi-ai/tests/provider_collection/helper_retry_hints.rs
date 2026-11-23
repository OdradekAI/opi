use super::*;
use opi_ai::http::HttpClient;
use opi_ai::providers::openai::OpenAiChatProvider;
use opi_ai::retry::{AcquisitionPolicy, AcquisitionSeal, RetryAdmission};
use std::sync::Arc;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

async fn assert_hint_admission(hint_seconds: u64, cap_ms: u64, expected_delay: Option<u64>) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", hint_seconds.to_string())
                .set_body_json(serde_json::json!({"error": {"message": "rate limited"}})),
        )
        .expect(1)
        .mount(&server)
        .await;
    let provider = OpenAiChatProvider::with_client(
        Some(server.uri()),
        "openai".into(),
        vec![],
        Arc::new(HttpClient::new()),
    );
    let mut models = Models::new();
    models
        .register_route(
            Box::new(provider),
            Arc::new(StaticAuthResolver::new(
                AuthScheme::Bearer,
                secrecy::SecretString::from("synthetic-hint-cap-key"),
            )),
            AuthProvenanceSource::Static,
            CompatMetadata {
                openai_compatible: true,
                profile: None,
            },
        )
        .unwrap();
    let prepared = models
        .prepare_call_with_policy(
            "openai:gpt-4o",
            minimal_request("openai:gpt-4o"),
            AcquisitionPolicy {
                max_retries: 1,
                max_retry_delay_ms: cap_ms,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let failure = drain_to_completion(prepared.start_attempt().unwrap())
        .await
        .expect_err("scripted HTTP 429");
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
    assert_eq!(
        failure.cause(),
        &ProviderError::RateLimited {
            retry_after_ms: Some(hint_seconds * 1000)
        }
    );
    let expected = expected_delay.map(|delay_ms| RetryAdmission {
        retry_index: 1,
        delay_ms,
    });
    assert_eq!(prepared.retry_admission(), expected);
    for _ in 0..2 {
        if let Some(delay_ms) = expected_delay {
            assert_eq!(prepared.acquisition_seal(), None);
            match prepared.start_attempt() {
                Err(ModelsError::RetryNotReady { retry_ready_in_ms }) => {
                    assert!(retry_ready_in_ms > 0 && retry_ready_in_ms <= delay_ms);
                }
                _ => panic!("admitted hint must refuse an early send"),
            }
        } else {
            assert_eq!(prepared.acquisition_seal(), Some(AcquisitionSeal::Finished));
            assert!(matches!(
                prepared.start_attempt(),
                Err(ModelsError::CallSealed {
                    seal: AcquisitionSeal::Finished
                })
            ));
        }
        assert_eq!(prepared.retry_admission(), expected);
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }
}

#[tokio::test]
async fn helper_retry_hint_below_cap_is_preserved() {
    assert_hint_admission(59, 60_000, Some(59_000)).await;
}

#[tokio::test]
async fn helper_retry_hint_equal_to_cap_is_preserved() {
    assert_hint_admission(60, 60_000, Some(60_000)).await;
}

#[tokio::test]
async fn helper_retry_hint_above_cap_seals_without_retry() {
    assert_hint_admission(120, 60_000, None).await;
}

#[tokio::test]
async fn helper_retry_hint_zero_cap_disables_the_bound() {
    assert_hint_admission(120, 0, Some(120_000)).await;
}

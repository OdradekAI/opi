//! Azure OpenAI Responses endpoint and stream wire assertions.

use super::*;

#[test]
fn azure_endpoint_resolution_prefers_explicit_then_resource_then_model_base() {
    assert_eq!(
        resolve_azure_base_url(
            Some(" https://custom.example.com/ "),
            Some("myresource"),
            Some("https://model.example.com"),
        )
        .unwrap(),
        "https://custom.example.com"
    );
    assert_eq!(
        resolve_azure_base_url(None, Some("myresource"), None).unwrap(),
        "https://myresource.openai.azure.com"
    );
    assert_eq!(
        resolve_azure_base_url(None, None, Some("https://model.example.com/")).unwrap(),
        "https://model.example.com"
    );
    assert!(
        matches!(
            resolve_azure_base_url(None, None, None),
            Err(opi_ai::provider::ProviderError::Config(_))
        ),
        "missing endpoint resolves to a typed failure"
    );
}

#[test]
fn azure_endpoint_normalizes_azure_host_paths_to_resource_root() {
    for base in [
        "https://myresource.openai.azure.com/openai",
        "https://myresource.openai.azure.com/openai/v1",
        "https://myresource.openai.azure.com/openai/v1/responses",
        "https://myresource.cognitiveservices.azure.com/openai/v1/",
    ] {
        let expected = if base.contains("cognitiveservices") {
            "https://myresource.cognitiveservices.azure.com"
        } else {
            "https://myresource.openai.azure.com"
        };
        assert_eq!(
            resolve_azure_base_url(Some(base), None, None).unwrap(),
            expected,
            "azure host base {base} normalizes to the resource root"
        );
    }
    assert_eq!(
        resolve_azure_base_url(Some("https://gateway.example.com/azure"), None, None).unwrap(),
        "https://gateway.example.com/azure",
        "non-azure hosts keep their configured path"
    );
}

#[test]
fn azure_api_version_defaults_to_v1_and_accepts_override() {
    assert_eq!(resolve_azure_api_version(None), "v1");
    assert_eq!(resolve_azure_api_version(Some("2024-10-21")), "2024-10-21");
}

#[test]
fn azure_deployment_resolution_explicit_map_then_model_id() {
    assert_eq!(
        resolve_deployment_name(Some("prod-deploy"), Some("o3=o3-prod"), "o3"),
        "prod-deploy"
    );
    assert_eq!(
        resolve_deployment_name(None, Some("o3=o3-prod, gpt4o=gpt4o-prod"), "gpt4o"),
        "gpt4o-prod"
    );
    assert_eq!(
        resolve_deployment_name(None, Some("o3=o3-prod"), "o4-mini"),
        "o4-mini"
    );
    assert_eq!(resolve_deployment_name(None, None, "o3"), "o3");
}

#[test]
fn azure_responses_url_construction() {
    let provider = AzureOpenAIProvider::from_config(
        Some("https://myresource.openai.azure.com".into()),
        vec!["my-gpt4o".into()],
        Some("2024-10-21".into()),
    )
    .unwrap();
    assert_eq!(
        provider.build_azure_responses_url("my-gpt4o"),
        "https://myresource.openai.azure.com/openai/v1/responses?api-version=2024-10-21"
    );
    // Default version follows the pinned source default v1.
    let provider = azure_responses_provider("https://myresource.openai.azure.com".into());
    assert!(
        provider
            .build_azure_responses_url("my-gpt4o")
            .ends_with("/openai/v1/responses?api-version=v1")
    );
}

#[tokio::test]
async fn azure_responses_failed_sse_preserves_the_code_classification() {
    use opi_ai::retry::{WholeCallRetryClassification, classify_whole_call_retry};
    let server = wiremock::MockServer::start().await;
    mount_azure_responses_mock(
        &server,
        "data: {\"type\":\"response.failed\",\"response\":{\"status\":\"failed\",\"error\":{\"code\":\"upstream_stream_read_error\",\"message\":\"PRIVATE_CANARY\"}}}\n\n".into(),
    )
    .await;
    let provider = azure_responses_provider(format!("{}/openai/v1", server.uri()));
    let failure = opi_ai::stream::ChatStream::new(provider.stream_prepared(
        azure_responses_request("my-gpt4o", None),
        azure_api_key_auth(),
    ))
    .result()
    .await
    .unwrap_err();
    assert!(matches!(failure.cause(), ProviderError::StreamError(_)));
    assert_eq!(failure.message().wire, Some(WireApi::AzureOpenAiResponses));
    assert_eq!(failure.message().raw_stop_reason.as_deref(), Some("failed"));
    assert_eq!(
        classify_whole_call_retry(failure.message()),
        WholeCallRetryClassification::Transient
    );
    assert!(!format!("{failure:?}").contains("PRIVATE_CANARY"));
    server.verify().await;
}

#[tokio::test]
async fn azure_responses_sse_error_uses_the_sdk_field_selection() {
    use opi_ai::retry::{WholeCallRetryClassification, classify_whole_call_retry};
    use serde_json::Value;
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("../../data/responses-error.json")).unwrap();
    for case in cases.iter().filter(|case| case["alsoAzure"] == true) {
        let mut body = String::new();
        if let Some(event) = case["event"].as_str() {
            body.push_str(&format!("event: {event}\n"));
        }
        body.push_str(&format!("data: {}\n\n", case["data"]));
        let server = wiremock::MockServer::start().await;
        mount_azure_responses_mock(&server, body).await;
        let provider = azure_responses_provider(format!("{}/openai/v1", server.uri()));
        let failure = opi_ai::stream::ChatStream::new(provider.stream_prepared(
            azure_responses_request("my-gpt4o", None),
            azure_api_key_auth(),
        ))
        .result()
        .await
        .unwrap_err();
        assert!(matches!(failure.cause(), ProviderError::StreamError(_)));
        assert_eq!(failure.message().wire, Some(WireApi::AzureOpenAiResponses));
        assert_eq!(failure.message().raw_stop_reason, None);
        assert_eq!(
            classify_whole_call_retry(failure.message()),
            WholeCallRetryClassification::Transient
        );
        assert!(!format!("{failure:?}").contains("PRIVATE_CANARY"));
        server.verify().await;
    }
}

#[tokio::test]
async fn azure_responses_direct_adapter_rejects_wrong_wire_records() {
    let provider = azure_responses_provider("https://unused.example.com".into());

    let chat_record = {
        let mut record = azure_responses_record("my-gpt4o");
        record.wire_api = WireApi::AzureOpenAiCompletions;
        record
    };
    let failure = AzureOpenAIResponsesChatApi::new(provider.clone(), chat_record)
        .expect_err("chat-wire records fail before authentication");
    assert!(
        matches!(failure, ProviderError::Config(_)),
        "typed rejection, got {failure:?}"
    );

    let openai_record = {
        let mut record = azure_responses_record("my-gpt4o");
        record.wire_api = WireApi::OpenAiResponses;
        record
    };
    let failure = AzureOpenAIResponsesChatApi::new(provider, openai_record)
        .expect_err("plain openai-responses records fail before authentication");
    assert!(
        matches!(failure, ProviderError::Config(_)),
        "typed rejection, got {failure:?}"
    );
}

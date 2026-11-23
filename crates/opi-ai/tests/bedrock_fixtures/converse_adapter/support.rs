//! Shared helpers for Bedrock Converse adapter topics.

#![allow(unused_imports)]

use super::super::*;
pub(crate) use opi_ai::api::bedrock::{
    BedrockBodyBuilder, BedrockConverseStreamChatApi, BedrockOptions,
};
pub(crate) use opi_ai::message::{OutputContent, ToolNotExecutedReason};
pub(crate) use opi_ai::model_info::{
    ModelCapabilities, ModelInfo, ThinkingLevel, ThinkingLevelMap, WireApi,
};
#[cfg(feature = "telemetry-tracing")]
pub(crate) use opi_ai::provider::Provider;
pub(crate) use opi_ai::provider::{CacheRetention, ProviderError};
pub(crate) use opi_ai::providers::bedrock::BedrockProvider;
pub(crate) use opi_ai::stream::AssistantStreamEvent;
pub(crate) use opi_ai::{ChatModelRecord, Models};
pub(crate) use std::collections::BTreeMap;

pub(crate) const SONNET4: &str = "anthropic.claude-sonnet-4-20250514-v2";

pub(crate) fn model_42(id: &str, name: &str) -> ModelInfo {
    ModelInfo::new(
        id,
        name,
        WireApi::BedrockConverseStream,
        ModelCapabilities::new(200_000, 8_192)
            .with_images(true)
            .with_streaming(true)
            .with_thinking(true),
    )
}

pub(crate) fn req_42(system: Option<&str>, user_text: &str) -> Request {
    let mut request = text_stream_request();
    request.model = SONNET4.into();
    request.context = opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
        system_prompt: system.map(str::to_owned),
        messages: vec![Message::User(UserMessage {
            content: vec![InputContent::Text {
                text: user_text.to_owned(),
            }],
            timestamp_ms: 0,
        })],
        tools: vec![],
    })
    .expect("valid context declarations");
    request
}

pub(crate) struct StaticAwsResolver42 {
    pub(crate) credentials: AwsCredentials,
}

impl AuthResolver for StaticAwsResolver42 {
    fn resolve<'a>(&'a self) -> BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
        let auth = ResolvedAuth::aws_sigv4(
            self.credentials.clone(),
            AuthProvenance {
                source: AuthProvenanceSource::AwsSigV4 {
                    source: AwsCredentialSource::ExplicitConfig,
                },
                fallback: AuthFallback::NotAttempted,
            },
        );
        Box::pin(async move { Ok(auth) })
    }
}

pub(crate) fn native() -> BedrockOptions {
    BedrockOptions::default()
}

pub(crate) fn body_42(provider: &impl BedrockBodyBuilder, request: &Request) -> serde_json::Value {
    provider
        .build_converse_body(request)
        .expect("bedrock body compiles")
}

pub(crate) fn reasoning_body_42(
    id: &str,
    reasoning: Option<ThinkingLevel>,
    budgets: BTreeMap<ThinkingLevel, u64>,
) -> serde_json::Value {
    let mut options = native();
    options.reasoning = reasoning;
    options.thinking_budgets = budgets;
    let provider = BedrockProvider::new(None, Arc::new(HttpClient::new()))
        .with_models(vec![model_42(id, "")])
        .with_native_options(options);
    let mut request = req_42(None, "go");
    request.model = id.to_owned();
    body_42(&provider, &request)
}

pub(crate) fn assistant_42() -> AssistantMessage {
    AssistantMessage {
        projection: None,
        content: vec![],
        api: opi_ai::ApiKind::Anthropic,
        provider: "bedrock".into(),
        deferred: None,
        model: SONNET4.into(),
        response_model: None,
        wire: Some(WireApi::BedrockConverseStream),
        response_id: None,
        usage: Default::default(),
        stop_reason: opi_ai::stream::StopReason::Stop,
        error_message: None,
        timestamp_ms: 0,
        diagnostics: vec![],
        duration_ms: None,
        raw_stop_reason: None,
        provider_thinking_level: None,
    }
}

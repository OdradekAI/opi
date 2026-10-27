//! Source-qualified native options for the subscription Codex wire.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::OpenAiCodexResponsesProvider;
use crate::auth::ResolvedAuth;
use crate::codex_resources::{CallResourceLease, SessionResourceOwner};
use crate::model_info::{ModelInfo, ThinkingLevel, WireApi};
use crate::models::{ApiId, ChatApi, SimpleOptions, SimpleToolChoice};
use crate::provider::{
    AttemptContext, EventStream, PayloadVariant, Provider, ProviderError, ProviderErrorSummary,
    Request,
};

/// Native effort spelling. Native calls retain the requested level rather
/// than applying the neutral entry's capability clamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CodexReasoningEffort {
    None,
    Minimal,
    Low,
    Medium,
    High,
    XHigh,
    Max,
}

impl CodexReasoningEffort {
    fn level(self) -> ThinkingLevel {
        match self {
            Self::None => ThinkingLevel::None,
            Self::Minimal => ThinkingLevel::Minimal,
            Self::Low => ThinkingLevel::Low,
            Self::Medium => ThinkingLevel::Medium,
            Self::High => ThinkingLevel::High,
            Self::XHigh => ThinkingLevel::XHigh,
            Self::Max => ThinkingLevel::Max,
        }
    }
}

/// Native summary request. Explicit null is distinct from omission in the
/// options value; the source maps either to `auto` when an effort is supplied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CodexReasoningSummary {
    Auto,
    Concise,
    Detailed,
    Off,
    On,
    #[serde(untagged)]
    Null,
}

/// Native service-tier request. `Some(Null)` sends JSON null; `None` omits
/// the wire field. Pricing assembly remains independent of this serializer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CodexServiceTier {
    Auto,
    Default,
    Flex,
    Scale,
    Priority,
    Fast,
    #[serde(untagged)]
    Null,
}

/// Native visible-text verbosity; omission uses `low`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CodexTextVerbosity {
    Low,
    Medium,
    High,
}

/// Native tool selection; omission uses `auto`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CodexToolChoice {
    Auto,
    None,
    Required,
}

/// Typed Codex-only parameters. Common temperature, transport, cache,
/// cancellation and transcript inputs stay on [`Request`].
///
/// Construct with [`Self::default`] and set the admitted fields. A native
/// default does not consume neutral `Request::thinking`; the simple entry
/// supplies that mapping separately.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct CodexOptions {
    pub reasoning_effort: Option<CodexReasoningEffort>,
    pub reasoning_summary: Option<CodexReasoningSummary>,
    pub service_tier: Option<CodexServiceTier>,
    pub text_verbosity: Option<CodexTextVerbosity>,
    pub tool_choice: Option<CodexToolChoice>,
    simple: bool,
}

/// A real Codex adapter registered through [`crate::Models::register_chat_api`].
/// Each dispatch clones its immutable provider configuration and retains the
/// shared transport owner; native options never mutate a registered provider.
pub struct CodexChatApi {
    api_id: ApiId,
    provider: Arc<OpenAiCodexResponsesProvider>,
    model: ModelInfo,
}

impl CodexChatApi {
    /// Bind a complete Codex record. Wrong-wire or invalid records fail
    /// before authentication or network activity.
    pub fn new(
        provider: Arc<OpenAiCodexResponsesProvider>,
        model: ModelInfo,
    ) -> Result<Self, ProviderError> {
        if model.wire_api != WireApi::OpenAiCodexResponses
            || model.id.trim().is_empty()
            || model.validate().is_err()
        {
            return Err(ProviderError::Config(
                ProviderErrorSummary::attested_static(
                    "Codex native options require a valid complete Codex model record",
                ),
            ));
        }
        Ok(Self {
            api_id: ApiId::new("openai-codex-responses"),
            provider,
            model,
        })
    }

    fn configured(&self, model: ModelInfo, options: CodexOptions) -> OpenAiCodexResponsesProvider {
        let mut provider = (*self.provider).clone();
        provider.models = vec![model];
        provider.native_options = Some(options);
        provider
    }
}

impl ChatApi<CodexOptions> for CodexChatApi {
    fn validate_simple_options(
        &self,
        model: &ModelInfo,
        _options: &SimpleOptions,
    ) -> Result<(), ProviderError> {
        super::validate_simple_model(model)
    }
    fn api_id(&self) -> &ApiId {
        &self.api_id
    }

    fn provider_id(&self) -> &str {
        super::PROVIDER_ID
    }

    fn complete_record(&self) -> ModelInfo {
        self.model.clone()
    }

    fn simple_options(&self) -> CodexOptions {
        simple_options(&SimpleOptions::default())
    }

    fn simple_options_for(&self, options: &SimpleOptions) -> CodexOptions {
        simple_options(options)
    }

    fn session_resources(&self) -> Option<&dyn SessionResourceOwner> {
        Some(self.provider.as_ref())
    }

    fn validate_native_request(
        &self,
        model: &ModelInfo,
        request: &Request,
        _options: &CodexOptions,
    ) -> Result<(), ProviderError> {
        super::validate_simple_model(model)?;
        if model.wire_api != WireApi::OpenAiCodexResponses {
            return Err(ProviderError::Config(
                ProviderErrorSummary::attested_static(
                    "Codex native options require a complete Codex model record",
                ),
            ));
        }
        crate::provider::validate_request_structure_for_model(
            self.provider_id(),
            Some(model),
            request,
        )
    }

    fn stream_prepared(
        &self,
        request: Request,
        options: CodexOptions,
        auth: ResolvedAuth,
    ) -> EventStream {
        self.stream_model_resource_prepared(self.model.clone(), request, options, auth, None)
    }

    fn stream_resource_prepared(
        &self,
        request: Request,
        options: CodexOptions,
        auth: ResolvedAuth,
        resources: Option<CallResourceLease>,
    ) -> EventStream {
        self.stream_model_resource_prepared(self.model.clone(), request, options, auth, resources)
    }

    fn stream_model_resource_prepared(
        &self,
        model: ModelInfo,
        mut request: Request,
        options: CodexOptions,
        auth: ResolvedAuth,
        resources: Option<CallResourceLease>,
    ) -> EventStream {
        if let Err(error) = self.validate_native_request(&model, &request, &options) {
            return Box::pin(futures_util::stream::once(async move { Err(error) }));
        }
        if !options.simple {
            // Native effort is validated and projected independently of the
            // neutral thinking fields, which are unused on this path.
            request.thinking = Default::default();
        }
        let provider = self.configured(model, options);
        if resources.is_none() {
            return provider.stream_prepared(request, auth);
        }
        let attempt = request
            .callbacks
            .inherited_attempt()
            .unwrap_or_else(|| AttemptContext {
                invocation_id: uuid::Uuid::new_v4().to_string(),
                attempt: 1,
                max_attempts: 1,
                payload_variant: PayloadVariant::Ordinary,
            });
        provider.stream_resource_attempt(request, auth, attempt, resources)
    }
}

pub(super) fn simple_options(options: &SimpleOptions) -> CodexOptions {
    CodexOptions {
        simple: true,
        tool_choice: options.tool_choice.map(|choice| match choice {
            SimpleToolChoice::Auto => CodexToolChoice::Auto,
            SimpleToolChoice::None => CodexToolChoice::None,
        }),
        ..CodexOptions::default()
    }
}

pub(super) fn apply_native_options(
    body: &mut Value,
    request: &Request,
    options: Option<&CodexOptions>,
    model: Option<&ModelInfo>,
) -> Result<(), ProviderError> {
    let object = body.as_object_mut().ok_or_else(|| {
        ProviderError::Config(ProviderErrorSummary::attested_static(
            "Codex request body must be an object",
        ))
    })?;
    object.remove("reasoning");
    let effort = match options {
        Some(options) if !options.simple => {
            options.reasoning_effort.map(CodexReasoningEffort::level)
        }
        _ => (request.thinking.enabled && request.thinking.level != ThinkingLevel::None)
            .then_some(request.thinking.level),
    };
    if let Some(level) = effort {
        // The source treats off/null differently from null at a non-off
        // level: explicit off/null omits reasoning; other levels fall back
        // to their literal native spelling.
        let mapped = model.map(|model| model.thinking_level_map.resolve(level));
        let wire_effort = match mapped {
            Some(Ok(Some(value))) => Some(value),
            Some(Err(_))
                if options.is_none_or(|options| options.simple) || level == ThinkingLevel::None =>
            {
                None
            }
            _ => Some(level.wire_name().unwrap_or("none").to_owned()),
        };
        if let Some(wire_effort) = wire_effort {
            let summary = options
                .and_then(|options| options.reasoning_summary)
                .filter(|summary| *summary != CodexReasoningSummary::Null)
                .unwrap_or(CodexReasoningSummary::Auto);
            object.insert(
                "reasoning".into(),
                serde_json::json!({"effort":wire_effort,"summary":summary}),
            );
        }
    } else if let Some(model) = model.filter(|model| model.capabilities.supports_thinking)
        && let Ok(off) = model.thinking_level_map.resolve(ThinkingLevel::None)
    {
        object.insert(
            "reasoning".into(),
            serde_json::json!({"effort":off.unwrap_or_else(|| "none".into())}),
        );
    }
    if let Some(options) = options {
        if let Some(tier) = options.service_tier {
            object.insert("service_tier".into(), serde_json::json!(tier));
        }
        if let Some(verbosity) = options.text_verbosity {
            object.insert("text".into(), serde_json::json!({"verbosity":verbosity}));
        }
        if let Some(choice) = options.tool_choice {
            object.insert("tool_choice".into(), serde_json::json!(choice));
        }
    }
    Ok(())
}

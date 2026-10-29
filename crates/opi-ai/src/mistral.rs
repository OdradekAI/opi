//! Mistral provider profile  - routes through the OpenAI-compatible adapter.
//!
//! Mistral AI (<https://mistral.ai>) provides an OpenAI-compatible Chat
//! Completions API at `https://api.mistral.ai/v1/chat/completions`. This
//! module creates a pre-configured [`OpenAiChatProvider`] with Mistral's
//! base URL and a curated model list.

use crate::model_info::WireApi;
use crate::openai_chat::{CompatConfig, OpenAiChatProvider};
use crate::provider::ModelInfo;
use crate::registry::ModelCapabilities;

/// Default Mistral API base URL (without the `/v1` suffix, which the adapter adds).
const BASE_URL: &str = "https://api.mistral.ai";

/// Create a Mistral-configured provider.
///
/// The provider resolves `mistral:model` specs and routes through the
/// OpenAI Chat Completions adapter. Authentication is no longer baked into
/// the provider; it arrives per call via the resolved auth passed to
/// [`Provider::stream_prepared`].
pub fn mistral_provider(base_url: Option<String>) -> OpenAiChatProvider {
    let base = base_url.unwrap_or_else(|| BASE_URL.into());
    OpenAiChatProvider::new_for_profile(
        base,
        "mistral".into(),
        CompatConfig::default(),
        vec![],
        model_catalog(),
    )
}

/// Built-in Mistral model metadata without credentials or HTTP construction.
pub fn model_catalog() -> Vec<ModelInfo> {
    vec![
        ModelInfo::new(
            "mistral-large-latest",
            "Mistral Large",
            WireApi::OpenAiCompletions,
            ModelCapabilities::new(128000, 8192).with_streaming(true),
        ),
        ModelInfo::new(
            "mistral-medium-latest",
            "Mistral Medium",
            WireApi::OpenAiCompletions,
            ModelCapabilities::new(32000, 8192).with_streaming(true),
        ),
        ModelInfo::new(
            "mistral-small-latest",
            "Mistral Small",
            WireApi::OpenAiCompletions,
            ModelCapabilities::new(32000, 8192).with_streaming(true),
        ),
        ModelInfo::new(
            "codestral-latest",
            "Codestral",
            WireApi::OpenAiCompletions,
            ModelCapabilities::new(256000, 8192).with_streaming(true),
        ),
    ]
}

// ---------------------------------------------------------------------------
// Task 20.43 — native mistral-conversations adapter (DEL-11)
//
// Pi's `mistral-conversations` API keeps its API identity but posts the
// OpenAI-style chat completions endpoint. This adapter owns the Mistral
// wire shapes: the typed native options (full tool-choice enum, reasoning
// effort and prompt mode), prompt-cache affinity, the headers-only response
// deadline, the 9-character tool-call id normalizer and the SSE consumption
// contract (chunked text/thinking content, index-keyed tool accumulation,
// cached-prompt usage split, the stop-reason map with the retryable
// in-stream error). The OpenAI-compatible profile above stays untouched.
// ---------------------------------------------------------------------------

use crate::provider::{EventStream, Provider, ProviderError, Request};
use crate::stream::{AssistantStreamEvent, StopReason};
use base64::Engine as _;
use futures_util::StreamExt as _;
use secrecy::ExposeSecret as _;
use std::collections::HashMap;
use std::time::Duration;

/// Default wait for response headers (annex row 33): long streams must not
/// be cut off by a fixed deadline, so the budget covers only the header
/// wait; body stalls stay with the HTTP client's idle handling.
const DEFAULT_HEADER_TIMEOUT: Duration = Duration::from_secs(60);
/// Mistral requires short alphanumeric tool-call ids on replay (pi
/// `MISTRAL_TOOL_CALL_ID_LENGTH`).
const MISTRAL_TOOL_CALL_ID_LENGTH: usize = 9;
/// Error bodies truncate at this many characters (pi
/// `MAX_MISTRAL_ERROR_BODY_CHARS`).
const MAX_MISTRAL_ERROR_BODY_CHARS: usize = 4000;

/// Native tool selection (pi `MistralOptions.toolChoice`). Wire omission —
/// not `none` — leaves the choice to the model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MistralToolChoice {
    Auto,
    None_,
    Any,
    Required,
    Named(String),
}

impl MistralToolChoice {
    fn wire_value(&self) -> serde_json::Value {
        match self {
            Self::Auto => serde_json::json!("auto"),
            Self::None_ => serde_json::json!("none"),
            Self::Any => serde_json::json!("any"),
            Self::Required => serde_json::json!("required"),
            Self::Named(name) => {
                serde_json::json!({"type": "function", "function": {"name": name}})
            }
        }
    }
}

/// Native reasoning effort (pi `MistralReasoningEffort`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MistralReasoningEffort {
    None_,
    Low,
    Medium,
    High,
    Max,
}

impl MistralReasoningEffort {
    fn wire_str(&self) -> &'static str {
        match self {
            Self::None_ => "none",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Max => "max",
        }
    }
}

/// Native prompt mode (pi currently defines only `reasoning`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MistralPromptMode {
    Reasoning,
}

impl MistralPromptMode {
    fn wire_str(&self) -> &'static str {
        match self {
            Self::Reasoning => "reasoning",
        }
    }
}

/// Provider-specific options for the native Mistral entry (pi
/// `MistralOptions`). Neutral entries map through [`Self::from_simple`].
#[derive(Clone, Debug, Default)]
pub struct MistralOptions {
    pub tool_choice: Option<MistralToolChoice>,
    pub reasoning_effort: Option<MistralReasoningEffort>,
    pub prompt_mode: Option<MistralPromptMode>,
    simple: bool,
}

impl MistralOptions {
    /// Build native (non-simple) options from their public fields.
    pub fn native(
        tool_choice: Option<MistralToolChoice>,
        reasoning_effort: Option<MistralReasoningEffort>,
        prompt_mode: Option<MistralPromptMode>,
    ) -> Self {
        Self {
            tool_choice,
            reasoning_effort,
            prompt_mode,
            simple: false,
        }
    }

    /// The neutral mapping (pi `streamSimple` base): only tool choice
    /// carries; reasoning entries stay with the model-driven simple
    /// resolution in [`MistralConversationsProvider::stream_simple`].
    pub fn from_simple(options: &crate::models::SimpleOptions) -> Self {
        Self {
            tool_choice: options.tool_choice.map(|choice| match choice {
                crate::models::SimpleToolChoice::Auto => MistralToolChoice::Auto,
                crate::models::SimpleToolChoice::None => MistralToolChoice::None_,
            }),
            reasoning_effort: None,
            prompt_mode: None,
            simple: true,
        }
    }

    fn is_simple(&self) -> bool {
        self.simple
    }
}

/// Stateful tool-call id normalizer (pi `createMistralToolCallIdNormalizer`):
/// alphanumeric-only, exactly-9 passthrough, hash-derived otherwise, with
/// collision retries. Ids normalize identically on the assistant call and
/// the paired tool result so replay stays consistent.
#[derive(Default)]
struct MistralToolCallIdNormalizer {
    forward: HashMap<String, String>,
    reverse: HashMap<String, String>,
}

impl MistralToolCallIdNormalizer {
    fn normalize(&mut self, id: &str) -> String {
        if let Some(existing) = self.forward.get(id) {
            return existing.clone();
        }
        let mut attempt = 0;
        loop {
            let candidate = Self::derive(id, attempt);
            match self.reverse.get(&candidate) {
                None => {
                    self.forward.insert(id.to_owned(), candidate.clone());
                    self.reverse.insert(candidate.clone(), id.to_owned());
                    return candidate;
                }
                Some(owner) if owner == id => return candidate,
                Some(_) => attempt += 1,
            }
        }
    }

    fn derive(id: &str, attempt: usize) -> String {
        let normalized: String = id.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
        if attempt == 0 && normalized.len() == MISTRAL_TOOL_CALL_ID_LENGTH {
            return normalized;
        }
        let seed_base = if normalized.is_empty() {
            id.to_owned()
        } else {
            normalized
        };
        let seed = if attempt == 0 {
            seed_base
        } else {
            format!("{seed_base}:{attempt}")
        };
        // pi derives from its internal short hash; the wire contract is only
        // "nine alphanumeric characters", so opi derives from SHA-256.
        use sha2::Digest as _;
        let digest = sha2::Sha256::digest(seed.as_bytes());
        let encoded = base64::engine::general_purpose::STANDARD.encode(digest);
        encoded
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .take(MISTRAL_TOOL_CALL_ID_LENGTH)
            .collect()
    }
}

/// The native `mistral-conversations` provider.
#[derive(Clone)]
pub struct MistralConversationsProvider {
    base_url: String,
    client: std::sync::Arc<crate::http::HttpClient>,
    models: Vec<crate::provider::ModelInfo>,
    native_options: Option<MistralOptions>,
}

impl std::fmt::Debug for MistralConversationsProvider {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MistralConversationsProvider")
            .field("base_url", &self.base_url)
            .field("models", &self.models)
            .finish_non_exhaustive()
    }
}

impl MistralConversationsProvider {
    pub fn new(base_url: Option<String>) -> Self {
        Self {
            base_url: base_url.unwrap_or_else(|| BASE_URL.into()),
            client: std::sync::Arc::new(crate::http::HttpClient::new()),
            models: Vec::new(),
            native_options: None,
        }
    }

    pub fn with_models(mut self, models: Vec<crate::provider::ModelInfo>) -> Self {
        self.models = models;
        self
    }

    pub fn with_native_options(mut self, options: MistralOptions) -> Self {
        self.native_options = Some(options);
        self
    }

    fn resolve_model(&self, requested: &str) -> Option<&crate::provider::ModelInfo> {
        let id = requested.strip_prefix("mistral:").unwrap_or(requested);
        self.models
            .iter()
            .find(|model| model.id == id)
            .or_else(|| self.models.iter().find(|model| model.id == requested))
    }

    /// The neutral entry (pi `streamSimple`): the simple options map tool
    /// choice, then the model record resolves the reasoning members — an
    /// explicit level map sends `reasoning_effort`, a mapless reasoning
    /// model sends `prompt_mode`, and non-reasoning models send neither.
    pub fn stream_simple(&self, request: Request, auth: crate::auth::ResolvedAuth) -> EventStream {
        let model = match self.resolve_model(&request.model).cloned() {
            Some(model) => model,
            None => {
                let error = self.unknown_model_error(&request.model);
                return Box::pin(futures_util::stream::once(async move { Err(error) }));
            }
        };
        let mut options = MistralOptions::from_simple(&crate::models::SimpleOptions::default());
        let (effort, mode) = simple_reasoning_members(&model, &request.thinking);
        options.reasoning_effort = effort;
        options.prompt_mode = mode;
        let provider = self.clone().with_native_options(options);
        provider.stream_prepared(request, auth)
    }

    fn unknown_model_error(&self, requested: &str) -> ProviderError {
        ProviderError::Config(crate::provider::ProviderErrorSummary::sanitized(format!(
            "unknown mistral model: {requested}"
        )))
    }

    fn build_request_body(&self, request: &Request) -> Result<serde_json::Value, ProviderError> {
        let model = self
            .resolve_model(&request.model)
            .ok_or_else(|| self.unknown_model_error(&request.model))?;
        let supports_images = model.capabilities.supports_images;
        let options = self.native_options.as_ref();

        let resolved = crate::transcript::resolve_transcript(
            &request.context,
            false,
            crate::transcript::TranscriptProjection::NativeOrCollapse,
        )
        .map_err(|error| {
            ProviderError::Config(crate::provider::ProviderErrorSummary::sanitized(
                error.to_string(),
            ))
        })?;
        let mut normalizer = MistralToolCallIdNormalizer::default();
        let messages = to_chat_messages(resolved.messages(), supports_images, &mut normalizer);

        let mut body = serde_json::json!({
            "model": model.id,
            "stream": true,
            "messages": messages,
        });
        let tools = crate::transcript::get_current_tools(resolved.messages());
        if !tools.is_empty() {
            body["tools"] = serde_json::Value::Array(
                tools
                    .iter()
                    .map(function_tool_value)
                    .collect::<Result<Vec<_>, _>>()?,
            );
        }
        if let Some(temperature) = request.temperature {
            body["temperature"] = serde_json::json!(temperature);
        }
        if let Some(max_tokens) = request.max_tokens {
            body["max_tokens"] = serde_json::json!(max_tokens);
        }
        if let Some(options) = options {
            if let Some(choice) = &options.tool_choice {
                body["tool_choice"] = choice.wire_value();
            }
            if let Some(mode) = options.prompt_mode {
                body["prompt_mode"] = serde_json::json!(mode.wire_str());
            }
            if let Some(effort) = options.reasoning_effort {
                body["reasoning_effort"] = serde_json::json!(effort.wire_str());
            }
        }
        if use_prompt_caching(request) {
            body["prompt_cache_key"] =
                serde_json::json!(request.session_id.as_deref().unwrap_or_default());
        }
        Ok(body)
    }
}

/// pi `shouldUsePromptCaching`: caching rides the session id unless the
/// retention hint suppresses it. `None` keeps provider defaults (active).
fn use_prompt_caching(request: &Request) -> bool {
    request.cache_retention != crate::provider::CacheRetention::Disabled
        && request.session_id.is_some()
}

/// The simple reasoning resolution (pi `streamSimple` effort/prompt-mode
/// branches). Only an explicit mapped entry counts as a level map: opi's
/// synthetic identity default is not a source-provided map, so mapless
/// reasoning models ride the prompt-mode branch.
fn simple_reasoning_members(
    model: &crate::provider::ModelInfo,
    thinking: &crate::provider::ThinkingConfig,
) -> (Option<MistralReasoningEffort>, Option<MistralPromptMode>) {
    if !model.capabilities.supports_thinking {
        return (None, None);
    }
    let has_effort_map = model.thinking_level_map.has_explicit_mappings();
    let level = if thinking.enabled {
        model.thinking_level_map.nearest_supported(thinking.level)
    } else {
        Some(crate::model_info::ThinkingLevel::None)
    };
    let Some(level) = level else {
        return (None, None);
    };
    if !has_effort_map {
        if level == crate::model_info::ThinkingLevel::None {
            return (None, None);
        }
        return (None, Some(MistralPromptMode::Reasoning));
    }
    // pi `effortMap[level] ?? "high"`; the off entry covers a disabled
    // request (pi `effortMap.off`).
    let mapped = model
        .thinking_level_map
        .resolve(level)
        .ok()
        .flatten()
        .and_then(|value| match value.as_str() {
            "none" => Some(MistralReasoningEffort::None_),
            "low" => Some(MistralReasoningEffort::Low),
            "medium" => Some(MistralReasoningEffort::Medium),
            "high" => Some(MistralReasoningEffort::High),
            "max" => Some(MistralReasoningEffort::Max),
            _ => None,
        })
        .unwrap_or(MistralReasoningEffort::High);
    (Some(mapped), None)
}

/// Project one tool (pi `toFunctionTools`): the strict projection defaults
/// on, so an unannotated tool rides `strict: true` with the converted
/// schema.
fn function_tool_value(tool: &crate::message::ToolDef) -> Result<serde_json::Value, ProviderError> {
    use crate::constrained_sampling::{
        NoRejectedKeywords, make_strict_json_schema, resolve_json_schema_strict_sampling,
    };
    let strict = resolve_json_schema_strict_sampling(tool, true, &NoRejectedKeywords)
        .map_err(|error| {
            ProviderError::Config(crate::provider::ProviderErrorSummary::sanitized(
                error.detail(),
            ))
        })?
        .unwrap_or(true);
    let parameters = if strict {
        make_strict_json_schema(&tool.input_schema, &NoRejectedKeywords)
            .expect("resolve succeeded so the conversion succeeds")
    } else {
        tool.input_schema.clone()
    };
    Ok(serde_json::json!({
        "type": "function",
        "function": {
            "name": tool.name,
            "description": tool.description,
            "parameters": parameters,
            "strict": strict,
        }
    }))
}

/// Convert the transcript to Mistral chat messages (pi `toChatMessages`).
fn to_chat_messages(
    messages: &[crate::message::Message],
    supports_images: bool,
    normalizer: &mut MistralToolCallIdNormalizer,
) -> Vec<serde_json::Value> {
    use crate::message::{AssistantContent, InputContent, Message, OutputContent};

    let mut result = Vec::new();
    for (index, message) in messages.iter().enumerate() {
        match message {
            Message::System(system) => {
                if index != 0 {
                    // collapse_system_messages already folded updates; a
                    // defensive skip keeps the leading-message contract.
                    continue;
                }
                let text = crate::transcript::system_message_text(system);
                if !text.is_empty() {
                    result.push(serde_json::json!({"role": "system", "content": text}));
                }
            }
            Message::User(user) => {
                let content: Vec<serde_json::Value> = user
                    .content
                    .iter()
                    .filter_map(|item| match item {
                        InputContent::Text { text } => {
                            Some(serde_json::json!({"type": "text", "text": text}))
                        }
                        InputContent::Image { source, media_type } => {
                            if !supports_images {
                                return None;
                            }
                            Some(serde_json::json!({
                                "type": "image_url",
                                "image_url": image_data_uri(source, media_type),
                            }))
                        }
                    })
                    .collect();
                if !content.is_empty() {
                    // A single text-only turn rides the plain string shape
                    // (pi's string user content); mixed content keeps the
                    // chunk array.
                    if content.len() == 1
                        && content[0].get("type") == Some(&serde_json::json!("text"))
                    {
                        result.push(serde_json::json!({
                            "role": "user",
                            "content": content[0]["text"],
                        }));
                    } else {
                        result.push(serde_json::json!({
                            "role": "user",
                            "content": content,
                        }));
                    }
                    continue;
                }
                let had_images = user
                    .content
                    .iter()
                    .any(|item| matches!(item, InputContent::Image { .. }));
                if had_images {
                    result.push(serde_json::json!({
                        "role": "user",
                        "content": "(image omitted: model does not support images)",
                    }));
                }
            }
            Message::Assistant(assistant) => {
                let mut content_parts = Vec::new();
                let mut tool_calls = Vec::new();
                for block in &assistant.content {
                    match block {
                        AssistantContent::Text { text, .. } => {
                            if !text.trim().is_empty() {
                                content_parts.push(serde_json::json!({
                                    "type": "text",
                                    "text": text,
                                }));
                            }
                        }
                        AssistantContent::Thinking { thinking, .. } => {
                            if !thinking.trim().is_empty() {
                                content_parts.push(serde_json::json!({
                                    "type": "thinking",
                                    "thinking": [{"type": "text", "text": thinking}],
                                }));
                            }
                        }
                        AssistantContent::ToolCall { tool_call } => {
                            let arguments = if tool_call.arguments.trim().is_empty() {
                                "{}".to_owned()
                            } else {
                                tool_call.arguments.clone()
                            };
                            tool_calls.push(serde_json::json!({
                                "id": normalizer.normalize(&tool_call.id),
                                "type": "function",
                                "function": {
                                    "name": tool_call.name,
                                    "arguments": arguments,
                                },
                                "index": 0,
                            }));
                        }
                        _ => {}
                    }
                }
                if !content_parts.is_empty() || !tool_calls.is_empty() {
                    let mut wire = serde_json::json!({"role": "assistant", "prefix": false});
                    if !content_parts.is_empty() {
                        wire["content"] = serde_json::Value::Array(content_parts);
                    }
                    if !tool_calls.is_empty() {
                        wire["tool_calls"] = serde_json::Value::Array(tool_calls);
                    }
                    result.push(wire);
                }
            }
            Message::ToolResult(tool) => {
                let text_result = tool
                    .content
                    .iter()
                    .filter_map(|part| match part {
                        OutputContent::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                let has_images = tool
                    .content
                    .iter()
                    .any(|part| matches!(part, OutputContent::Image { .. }));
                let tool_text = build_tool_result_text(
                    &text_result,
                    has_images,
                    supports_images,
                    tool.is_error,
                );
                let mut content = vec![serde_json::json!({"type": "text", "text": tool_text})];
                for part in &tool.content {
                    if let OutputContent::Image { source, media_type } = part
                        && supports_images
                    {
                        content.push(serde_json::json!({
                            "type": "image_url",
                            "image_url": image_data_uri(source, media_type),
                        }));
                    }
                }
                result.push(serde_json::json!({
                    "role": "tool",
                    "tool_call_id": normalizer.normalize(&tool.tool_call_id),
                    "name": tool.tool_name,
                    "content": content,
                }));
            }
        }
    }
    result
}

fn image_data_uri(
    source: &crate::message::ImageSource,
    media_type: &crate::message::MediaType,
) -> String {
    let encoded = match source {
        crate::message::ImageSource::Bytes { data } => {
            base64::engine::general_purpose::STANDARD.encode(data)
        }
        crate::message::ImageSource::Base64 { data } => data.clone(),
        crate::message::ImageSource::Url { url } => return url.clone(),
    };
    format!("data:{};base64,{}", media_type.as_str(), encoded)
}

/// pi `buildToolResultText`: error prefix, image notes, and the empty-output
/// placeholder.
fn build_tool_result_text(
    text: &str,
    has_images: bool,
    supports_images: bool,
    is_error: bool,
) -> String {
    let trimmed = text.trim();
    let error_prefix = if is_error { "[tool error] " } else { "" };
    if !trimmed.is_empty() {
        let image_suffix = if has_images && !supports_images {
            "\n[tool image omitted: model does not support images]"
        } else {
            ""
        };
        return format!("{error_prefix}{trimmed}{image_suffix}");
    }
    if has_images {
        if supports_images {
            return if is_error {
                "[tool error] (see attached image)".to_owned()
            } else {
                "(see attached image)".to_owned()
            };
        }
        return if is_error {
            "[tool error] (image omitted: model does not support images)".to_owned()
        } else {
            "(image omitted: model does not support images)".to_owned()
        };
    }
    if is_error {
        "[tool error] (no tool output)".to_owned()
    } else {
        "(no tool output)".to_owned()
    }
}

fn truncate_error_text(text: &str, max_chars: usize) -> String {
    let chars = text.chars().count();
    if chars <= max_chars {
        return text.to_owned();
    }
    format!(
        "{}... [truncated {} chars]",
        text.chars().take(max_chars).collect::<String>(),
        chars - max_chars
    )
}

fn format_mistral_http_error(status: u16, body: &str) -> String {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        format!("Mistral API error ({status})")
    } else {
        format!(
            "Mistral API error ({status}): {}",
            truncate_error_text(trimmed, MAX_MISTRAL_ERROR_BODY_CHARS)
        )
    }
}

fn map_mistral_status(
    status: reqwest::StatusCode,
    headers: &reqwest::header::HeaderMap,
    body: &str,
) -> ProviderError {
    let message = format_mistral_http_error(status.as_u16(), body);
    match status.as_u16() {
        401 | 403 => {
            ProviderError::AuthFailed(crate::provider::ProviderErrorSummary::sanitized(message))
        }
        429 => ProviderError::RateLimited {
            retry_after_ms: crate::retry::parse_retry_after(headers),
        },
        408 | 504 => ProviderError::Timeout,
        code => ProviderError::ServiceFault {
            status: code,
            error_name: None,
            retry_after_ms: None,
            detail: crate::provider::ProviderErrorSummary::sanitized(message),
        },
    }
}

impl crate::provider::Provider for MistralConversationsProvider {
    fn id(&self) -> &str {
        "mistral"
    }

    fn models(&self) -> &[crate::provider::ModelInfo] {
        &self.models
    }

    fn stream_prepared(
        &self,
        mut request: Request,
        auth: crate::auth::ResolvedAuth,
    ) -> EventStream {
        request.callbacks = request.callbacks.ensure_call_scope();
        let bearer = match auth.scheme {
            crate::auth::AuthScheme::Bearer => {
                format!("Bearer {}", auth.secret.expose_secret())
            }
            crate::auth::AuthScheme::ApiKey | crate::auth::AuthScheme::AwsSigV4(_) => {
                return Box::pin(futures_util::stream::iter(vec![Err(
                    ProviderError::Config(crate::provider::ProviderErrorSummary::attested_static(
                        "Mistral accepts only Bearer authentication",
                    )),
                )]));
            }
        };
        let model = match self.resolve_model(&request.model).cloned() {
            Some(model) => model,
            None => {
                let error = self.unknown_model_error(&request.model);
                return Box::pin(futures_util::stream::iter(vec![Err(error)]));
            }
        };
        let body = match self.build_request_body(&request) {
            Ok(body) => body,
            Err(error) => return Box::pin(futures_util::stream::iter(vec![Err(error)])),
        };
        let url = crate::endpoint::join_endpoint(&self.base_url, "v1/chat/completions");
        let caching = use_prompt_caching(&request);
        let has_explicit_affinity = request
            .extra_headers
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case("x-affinity"));
        let mut req = self
            .client
            .client()
            .post(&url)
            .header("authorization", bearer)
            .header("accept", "text/event-stream")
            .header("content-type", "application/json");
        for (name, value) in &request.extra_headers {
            req = req.header(name.as_str(), value.as_str());
        }
        if caching
            && !has_explicit_affinity
            && let Some(session) = &request.session_id
        {
            req = req.header("x-affinity", session.as_str());
        }

        let requested_model = model.id.clone();
        let callbacks = request.callbacks.clone();
        let cancel = request.cancel;
        let header_timeout = request.timeout.unwrap_or(DEFAULT_HEADER_TIMEOUT);
        let telemetry = request.telemetry_context.clone();
        let snapshot = crate::stream::ProducerSnapshot::default();
        let producer_snapshot = snapshot.clone();
        let (tx, rx) = tokio::sync::mpsc::channel(64);
        let producer_cancel = cancel.clone();

        let tx_closed = tx.clone();
        let producer = async move {
            let run = async {
                let result = async {
                    let body = callbacks
                        .apply_payload_async(crate::callbacks::ORDINARY_PAYLOAD_VARIANT, body)
                        .await?;
                    stream_mistral_http(
                        req,
                        body,
                        header_timeout,
                        &requested_model,
                        producer_cancel.clone(),
                        callbacks,
                        producer_snapshot,
                        &tx,
                    )
                    .await
                }
                .await;
                match result {
                    Ok(()) | Err(ProviderError::Cancelled) => {}
                    Err(error) => {
                        let _ =
                            crate::stream::send_or_cancel(&producer_cancel, &tx, Err(error)).await;
                    }
                }
            };
            tokio::select! {
                biased;
                _ = producer_cancel.cancelled() => (),
                _ = tx_closed.closed() => (),
                _ = run => (),
            }
        };
        if let Some(context) = telemetry {
            tokio::spawn(context.instrument(producer));
        } else {
            tokio::spawn(producer);
        }
        Box::pin(crate::stream::CancelAwareReceiverStream::with_snapshot(
            rx, cancel, snapshot,
        ))
    }
}

#[allow(clippy::too_many_arguments)]
async fn stream_mistral_http(
    req: reqwest::RequestBuilder,
    body: serde_json::Value,
    header_timeout: Duration,
    requested_model: &str,
    cancel: tokio_util::sync::CancellationToken,
    callbacks: crate::callbacks::RequestCallbacks,
    snapshot: crate::stream::ProducerSnapshot,
    tx: &tokio::sync::mpsc::Sender<Result<AssistantStreamEvent, ProviderError>>,
) -> Result<(), ProviderError> {
    // The header deadline covers only the wait for response headers (annex
    // row 33): once send() resolves, the body streams without this budget.
    let response = tokio::time::timeout(header_timeout, req.json(&body).send())
        .await
        .map_err(|_| ProviderError::Timeout)?
        .map_err(|error| {
            if error.is_timeout() {
                ProviderError::Timeout
            } else if error.is_redirect() {
                ProviderError::Network(crate::provider::ProviderErrorSummary::attested_static(
                    "too many redirects",
                ))
            } else {
                ProviderError::Network(crate::provider::ProviderErrorSummary::sanitized(
                    error.to_string(),
                ))
            }
        })?;

    let observed_headers: Vec<(String, String)> = response
        .headers()
        .iter()
        .map(|(name, value)| {
            (
                name.as_str().to_ascii_lowercase(),
                value.to_str().unwrap_or_default().to_string(),
            )
        })
        .collect();
    let status = response.status();
    // The response observer fires before the success check and before any
    // body processing (annex row 33, pi onResponse ordering).
    callbacks
        .notify_response_async(status.as_u16(), &observed_headers)
        .await?;
    if !status.is_success() {
        let headers = response.headers().clone();
        let body_text = response.text().await.unwrap_or_default();
        return Err(map_mistral_status(status, &headers, &body_text));
    }
    snapshot.received_response();

    let mut mapper = MistralSseMapper::new(requested_model);
    let mut byte_stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut decoder = crate::text_decode::StreamTextDecoder::new();
    loop {
        let chunk = tokio::select! {
            _ = cancel.cancelled() => {
                return Err(ProviderError::Cancelled);
            }
            chunk = byte_stream.next() => {
                match chunk {
                    Some(c) => c,
                    None => break,
                }
            }
        };
        let chunk = chunk.map_err(|error| {
            if error.is_timeout() {
                ProviderError::Timeout
            } else if error.is_connect() {
                ProviderError::Network(crate::provider::ProviderErrorSummary::sanitized(
                    error.to_string(),
                ))
            } else {
                ProviderError::StreamError(crate::provider::ProviderErrorSummary::sanitized(
                    error.to_string(),
                ))
            }
        })?;
        buffer.push_str(&decoder.push(&chunk));
        for raw in extract_sse_frames(&mut buffer) {
            match parse_mistral_event(&raw) {
                MistralParsedEvent::Done => {
                    let events = mapper.finish();
                    for event in events {
                        crate::stream::send_or_cancel(&cancel, tx, event).await?;
                    }
                    return Ok(());
                }
                MistralParsedEvent::Empty => {}
                MistralParsedEvent::Chunk(value) => {
                    for event in mapper.consume(&value) {
                        crate::stream::send_or_cancel(&cancel, tx, event).await?;
                    }
                }
                MistralParsedEvent::Invalid(reason) => {
                    return Err(ProviderError::StreamError(
                        crate::provider::ProviderErrorSummary::attested_static(reason),
                    ));
                }
            }
        }
    }
    // EOF without [DONE]: flush whatever remains, then hold the terminal
    // contract (a missing finish reason fails typed).
    for raw in extract_sse_frames(&mut buffer) {
        match parse_mistral_event(&raw) {
            MistralParsedEvent::Done => break,
            MistralParsedEvent::Empty => {}
            MistralParsedEvent::Chunk(value) => {
                for event in mapper.consume(&value) {
                    crate::stream::send_or_cancel(&cancel, tx, event).await?;
                }
            }
            MistralParsedEvent::Invalid(reason) => {
                return Err(ProviderError::StreamError(
                    crate::provider::ProviderErrorSummary::attested_static(reason),
                ));
            }
        }
    }
    let events = mapper.finish();
    for event in events {
        crate::stream::send_or_cancel(&cancel, tx, event).await?;
    }
    Ok(())
}

enum MistralParsedEvent {
    Empty,
    Done,
    Chunk(serde_json::Value),
    Invalid(&'static str),
}

/// Split complete SSE frames (any CR/LF blank-line boundary, pi
/// `findMistralEventBoundary`) off the buffer front.
fn extract_sse_frames(buffer: &mut String) -> Vec<String> {
    let mut frames = Vec::new();
    while let Some((index, length)) = find_boundary(buffer) {
        let frame: String = buffer.drain(..index + length).collect();
        frames.push(frame[..index].to_owned());
    }
    frames
}

fn find_boundary(buffer: &str) -> Option<(usize, usize)> {
    // One newline sequence starts at `start` (a lone CR/LF or a CR-LF/LF-CR
    // pair); a blank line is two consecutive sequences (pi
    // `findMistralEventBoundary`).
    fn seq_len(bytes: &[u8], start: usize) -> Option<usize> {
        match (bytes.get(start), bytes.get(start + 1)) {
            (Some(b'\r'), Some(b'\n')) | (Some(b'\n'), Some(b'\r')) => Some(2),
            (Some(b'\r'), _) | (Some(b'\n'), _) => Some(1),
            _ => None,
        }
    }
    let bytes = buffer.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let Some(first) = seq_len(bytes, index) else {
            index += 1;
            continue;
        };
        if let Some(second) = seq_len(bytes, index + first) {
            return Some((index, first + second));
        }
        index += first;
    }
    None
}

fn parse_mistral_event(raw: &str) -> MistralParsedEvent {
    let data = raw
        .split(['\r', '\n'])
        .filter_map(|line| line.strip_prefix("data:"))
        .map(str::trim_start)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_owned();
    if data.is_empty() {
        return MistralParsedEvent::Empty;
    }
    if data == "[DONE]" {
        return MistralParsedEvent::Done;
    }
    match serde_json::from_str::<serde_json::Value>(&data) {
        Ok(value)
            if value.is_object()
                && value
                    .get("choices")
                    .is_some_and(|choices| choices.is_array()) =>
        {
            MistralParsedEvent::Chunk(value)
        }
        Ok(_) | Err(_) => MistralParsedEvent::Invalid("Invalid Mistral streaming event"),
    }
}

enum MistralBlockKind {
    Text,
    Thinking,
}

struct MistralSseMapper {
    message: crate::message::AssistantMessage,
    current_block: Option<(MistralBlockKind, usize, String)>,
    tool_blocks: Vec<(String, usize, String)>,
    stop_reason: Option<StopReason>,
    stop_error: Option<String>,
}

impl MistralSseMapper {
    fn new(requested_model: &str) -> Self {
        let message = crate::message::AssistantMessage {
            content: Vec::new(),
            api: crate::ApiKind::Mistral,
            provider: "mistral".into(),
            model: requested_model.to_owned(),
            response_model: None,
            wire: Some(crate::model_info::WireApi::MistralConversations),
            response_id: None,
            usage: crate::stream::Usage::default(),
            stop_reason: StopReason::Stop,
            error_message: None,
            timestamp_ms: crate::time::now_ms(),
            diagnostics: Vec::new(),
            duration_ms: None,
            provider_thinking_level: None,
        };
        Self {
            message,
            current_block: None,
            tool_blocks: Vec::new(),
            stop_reason: None,
            stop_error: None,
        }
    }

    fn consume(
        &mut self,
        chunk: &serde_json::Value,
    ) -> Vec<Result<AssistantStreamEvent, ProviderError>> {
        let mut events = Vec::new();
        if let Some(id) = chunk.get("id").and_then(|id| id.as_str())
            && !id.is_empty()
            && self.message.response_id.is_none()
        {
            self.message.response_id = Some(id.to_owned());
        }
        if let Some(usage) = chunk.get("usage").filter(|usage| usage.is_object()) {
            self.apply_usage(usage);
        }
        let Some(choice) = chunk
            .get("choices")
            .and_then(|choices| choices.as_array())
            .and_then(|choices| choices.first())
        else {
            return events;
        };
        if let Some(reason) = choice.get("finish_reason").and_then(|r| r.as_str()) {
            match reason {
                "stop" => self.stop_reason = Some(StopReason::Stop),
                "length" | "model_length" => self.stop_reason = Some(StopReason::Length),
                "tool_calls" => self.stop_reason = Some(StopReason::ToolUse),
                "error" => {
                    // Mistral reports transient server failures this way; the
                    // typed message keeps the stream retryable for the outer
                    // acquisition owner.
                    self.stop_reason = Some(StopReason::Error);
                    self.stop_error =
                        Some("Provider stopped with: error (server error)".to_owned());
                }
                other => {
                    self.stop_reason = Some(StopReason::Error);
                    self.stop_error = Some(format!("Provider stopped with: {other}"));
                }
            }
        }
        let delta = choice.get("delta").cloned().unwrap_or_default();
        if let Some(content) = delta.get("content") {
            let items: Vec<serde_json::Value> = match content {
                serde_json::Value::String(text) => {
                    vec![serde_json::json!({"type": "text", "text": text})]
                }
                serde_json::Value::Array(items) => items.clone(),
                _ => Vec::new(),
            };
            for item in items {
                let item_type = item.get("type").and_then(|t| t.as_str()).unwrap_or("");
                match item_type {
                    "thinking" => {
                        let text = item
                            .get("thinking")
                            .and_then(|t| t.as_array())
                            .map(|parts| {
                                parts
                                    .iter()
                                    .filter_map(|part| {
                                        part.get("text").and_then(|text| text.as_str())
                                    })
                                    .collect::<String>()
                            })
                            .unwrap_or_default();
                        if !text.is_empty() {
                            self.push_block(MistralBlockKind::Thinking, &text, &mut events);
                        }
                    }
                    "text" => {
                        let text = item.get("text").and_then(|t| t.as_str()).unwrap_or("");
                        if !text.is_empty() {
                            self.push_block(MistralBlockKind::Text, text, &mut events);
                        }
                    }
                    _ => {}
                }
            }
        }
        if let Some(tool_calls) = delta.get("tool_calls").and_then(|t| t.as_array()) {
            for tool_call in tool_calls {
                self.close_current_block(&mut events);
                let id = tool_call
                    .get("id")
                    .and_then(|id| id.as_str())
                    .filter(|id| !id.is_empty() && *id != "null")
                    .map(str::to_owned)
                    .unwrap_or_else(|| {
                        let index = tool_call.get("index").and_then(|i| i.as_u64()).unwrap_or(0);
                        MistralToolCallIdNormalizer::derive(&format!("toolcall:{index}"), 0)
                    });
                let key = match tool_call.get("index").and_then(|i| i.as_u64()) {
                    Some(index) => format!("index:{index}"),
                    None => format!("id:{id}"),
                };
                let function = tool_call.get("function").cloned().unwrap_or_default();
                let entry = self
                    .tool_blocks
                    .iter_mut()
                    .find(|(existing, _, _)| *existing == key);
                match entry {
                    Some((_, content_index, args)) => {
                        args.push_str(&arguments_text(&function));
                        let _ = content_index;
                    }
                    None => {
                        let name = function
                            .get("name")
                            .and_then(|n| n.as_str())
                            .unwrap_or_default()
                            .to_owned();
                        self.message
                            .content
                            .push(crate::message::AssistantContent::ToolCall {
                                tool_call: crate::message::ToolCall {
                                    id: id.clone(),
                                    name,
                                    arguments: String::new(),
                                    arguments_error: None,
                                    signature: None,
                                },
                            });
                        let content_index = self.message.content.len() - 1;
                        self.tool_blocks.push((
                            key.clone(),
                            content_index,
                            arguments_text(&function),
                        ));
                        events.push(Ok(AssistantStreamEvent::ToolCallStart {
                            content_index,
                            partial: self.message.clone(),
                        }));
                    }
                }
            }
        }
        events
    }

    fn apply_usage(&mut self, usage: &serde_json::Value) {
        let prompt_tokens = usage
            .get("prompt_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let cached = mistral_cached_tokens(usage, prompt_tokens);
        let input = prompt_tokens.saturating_sub(cached);
        let output = usage
            .get("completion_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        self.message.usage.input_tokens = input.min(u32::MAX as u64) as u32;
        self.message.usage.output_tokens = output.min(u32::MAX as u64) as u32;
        self.message.usage.cache_read_tokens = cached.min(u32::MAX as u64) as u32;
        self.message.usage.cache_write_tokens = 0;
        let sum = input + output + cached;
        self.message.usage.total_tokens = Some(
            usage
                .get("total_tokens")
                .and_then(|v| v.as_u64())
                .filter(|total| *total > 0)
                .unwrap_or(sum),
        );
        self.message.usage.reported = true;
    }

    fn push_block(
        &mut self,
        kind: MistralBlockKind,
        text: &str,
        events: &mut Vec<Result<AssistantStreamEvent, ProviderError>>,
    ) {
        let matches_current = matches!(
            (&self.current_block, &kind),
            (Some((MistralBlockKind::Text, _, _)), MistralBlockKind::Text)
                | (
                    Some((MistralBlockKind::Thinking, _, _)),
                    MistralBlockKind::Thinking
                )
        );
        if matches_current {
            if let Some((_, index, buffer)) = &mut self.current_block {
                buffer.push_str(text);
                let index = *index;
                let is_text = matches!(kind, MistralBlockKind::Text);
                let partial = self.message.clone();
                events.push(Ok(if is_text {
                    AssistantStreamEvent::TextDelta {
                        content_index: index,
                        delta: text.to_owned(),
                        partial,
                    }
                } else {
                    AssistantStreamEvent::ThinkingDelta {
                        content_index: index,
                        delta: text.to_owned(),
                        partial,
                    }
                }));
            }
            return;
        }
        self.close_current_block(events);
        let is_text = matches!(kind, MistralBlockKind::Text);
        self.message.content.push(if is_text {
            crate::message::AssistantContent::Text {
                text: String::new(),
                signature: None,
            }
        } else {
            crate::message::AssistantContent::Thinking {
                thinking: String::new(),
                signature: None,
            }
        });
        let index = self.message.content.len() - 1;
        self.current_block = Some((kind, index, text.to_owned()));
        let partial = self.message.clone();
        events.push(Ok(if is_text {
            AssistantStreamEvent::TextStart {
                content_index: index,
                partial,
            }
        } else {
            AssistantStreamEvent::ThinkingStart {
                content_index: index,
                partial,
            }
        }));
        let partial = self.message.clone();
        events.push(Ok(if is_text {
            AssistantStreamEvent::TextDelta {
                content_index: index,
                delta: text.to_owned(),
                partial,
            }
        } else {
            AssistantStreamEvent::ThinkingDelta {
                content_index: index,
                delta: text.to_owned(),
                partial,
            }
        }));
    }

    fn close_current_block(
        &mut self,
        events: &mut Vec<Result<AssistantStreamEvent, ProviderError>>,
    ) {
        let Some((kind, index, buffer)) = self.current_block.take() else {
            return;
        };
        let partial = self.message.clone();
        events.push(Ok(match kind {
            MistralBlockKind::Text => {
                if let Some(crate::message::AssistantContent::Text { text, .. }) =
                    self.message.content.get_mut(index)
                {
                    *text = buffer.clone();
                }
                AssistantStreamEvent::TextEnd {
                    content_index: index,
                    content: buffer,
                    partial,
                }
            }
            MistralBlockKind::Thinking => {
                if let Some(crate::message::AssistantContent::Thinking { thinking, .. }) =
                    self.message.content.get_mut(index)
                {
                    *thinking = buffer.clone();
                }
                AssistantStreamEvent::ThinkingEnd {
                    content_index: index,
                    content: buffer,
                    partial,
                }
            }
        }));
    }

    fn finish(&mut self) -> Vec<Result<AssistantStreamEvent, ProviderError>> {
        let mut events = Vec::new();
        self.close_current_block(&mut events);
        for (_, index, args) in std::mem::take(&mut self.tool_blocks) {
            let (arguments, arguments_error) = crate::message::finalize_tool_arguments(args);
            if let Some(crate::message::AssistantContent::ToolCall { tool_call, .. }) =
                self.message.content.get_mut(index)
            {
                tool_call.arguments = arguments.clone();
                tool_call.arguments_error = arguments_error;
                events.push(Ok(AssistantStreamEvent::ToolCallEnd {
                    content_index: index,
                    tool_call: tool_call.clone(),
                    partial: self.message.clone(),
                }));
            }
        }
        if let Some(error) = self.stop_error.take() {
            self.message.stop_reason = StopReason::Error;
            self.message.error_message = Some(error.clone());
            events.push(Err(ProviderError::StreamError(
                crate::provider::ProviderErrorSummary::sanitized(error),
            )));
            return events;
        }
        let Some(reason) = self.stop_reason else {
            events.push(Err(ProviderError::StreamError(
                crate::provider::ProviderErrorSummary::attested_static(
                    "Mistral stream ended without a finish reason",
                ),
            )));
            return events;
        };
        self.message.stop_reason = reason;
        events.push(Ok(AssistantStreamEvent::Done {
            reason,
            message: self.message.clone(),
        }));
        events
    }
}

/// pi `getMistralCachedPromptTokens`: the cached-token detail key family
/// across snake/camel spellings, clamped into the prompt budget.
fn mistral_cached_tokens(usage: &serde_json::Value, prompt_tokens: u64) -> u64 {
    let read = |keys: [&str; 2]| {
        usage
            .get(keys[0])
            .or_else(|| usage.get(keys[1]))
            .and_then(|detail| detail.get(keys[0]).or_else(|| detail.get(keys[1])))
            .and_then(|value| value.as_u64())
            .unwrap_or(0)
    };
    let raw = read(["cached_tokens", "cachedTokens"])
        .max(read(["num_cached_tokens", "numCachedTokens"]))
        .max({
            usage
                .get("prompt_tokens_details")
                .or_else(|| usage.get("promptTokensDetails"))
                .or_else(|| usage.get("prompt_token_details"))
                .or_else(|| usage.get("promptTokenDetails"))
                .and_then(|detail| {
                    detail
                        .get("cached_tokens")
                        .or_else(|| detail.get("cachedTokens"))
                })
                .and_then(|value| value.as_u64())
                .unwrap_or(0)
        });
    raw.min(prompt_tokens)
}

fn arguments_text(function: &serde_json::Value) -> String {
    match function.get("arguments") {
        Some(serde_json::Value::String(text)) => text.clone(),
        Some(value) => value.to_string(),
        None => String::new(),
    }
}

/// The public direct adapter registering through `Models::register_chat_api`
/// with api id `mistral-conversations` and provider id `mistral`.
pub struct MistralConversationsChatApi {
    provider: MistralConversationsProvider,
    model: crate::provider::ModelInfo,
    api_id: crate::models::ApiId,
}

impl std::fmt::Debug for MistralConversationsChatApi {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MistralConversationsChatApi")
            .field("model", &self.model)
            .field("api_id", &self.api_id)
            .finish_non_exhaustive()
    }
}

impl MistralConversationsChatApi {
    pub fn new(
        provider: MistralConversationsProvider,
        model: crate::provider::ModelInfo,
    ) -> Result<Self, ProviderError> {
        if model.wire_api != crate::model_info::WireApi::MistralConversations
            || model.id.trim().is_empty()
            || model.validate().is_err()
        {
            return Err(ProviderError::Config(
                crate::provider::ProviderErrorSummary::attested_static(
                    "mistral-conversations adapters require a valid complete Chat model record",
                ),
            ));
        }
        Ok(Self {
            provider,
            model,
            api_id: crate::models::ApiId::new("mistral-conversations"),
        })
    }
}

impl crate::models::ChatApi<MistralOptions> for MistralConversationsChatApi {
    fn api_id(&self) -> &crate::models::ApiId {
        &self.api_id
    }

    fn provider_id(&self) -> &str {
        "mistral"
    }

    fn complete_record(&self) -> crate::provider::ModelInfo {
        self.model.clone()
    }

    fn simple_options(&self) -> MistralOptions {
        MistralOptions::from_simple(&crate::models::SimpleOptions::default())
    }

    fn simple_options_for(&self, options: &crate::models::SimpleOptions) -> MistralOptions {
        MistralOptions::from_simple(options)
    }

    fn validate_native_request(
        &self,
        model: &crate::provider::ModelInfo,
        request: &Request,
        _options: &MistralOptions,
    ) -> Result<(), ProviderError> {
        if model.wire_api != crate::model_info::WireApi::MistralConversations {
            return Err(ProviderError::Config(
                crate::provider::ProviderErrorSummary::attested_static(
                    "mistral-conversations adapters require a valid complete Chat model record",
                ),
            ));
        }
        crate::provider::validate_request_for_model(self.provider_id(), Some(model), request)
    }

    fn stream_prepared(
        &self,
        request: Request,
        options: MistralOptions,
        auth: crate::auth::ResolvedAuth,
    ) -> EventStream {
        self.stream_model_resource_prepared(self.model.clone(), request, options, auth, None)
    }

    fn stream_model_resource_prepared(
        &self,
        model: crate::provider::ModelInfo,
        mut request: Request,
        options: MistralOptions,
        auth: crate::auth::ResolvedAuth,
        _resources: Option<crate::codex_resources::CallResourceLease>,
    ) -> EventStream {
        if let Err(error) = self.validate_native_request(&model, &request, &options) {
            return Box::pin(futures_util::stream::once(async move { Err(error) }));
        }
        if !options.is_simple() {
            // Native reasoning owns the wire projection; neutral thinking
            // fields stay unused on this path.
            request.thinking = Default::default();
        }
        let provider = self
            .provider
            .clone()
            .with_models(vec![model])
            .with_native_options(options);
        provider.stream_prepared(request, auth)
    }
}

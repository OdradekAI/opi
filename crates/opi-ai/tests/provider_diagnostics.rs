//! Phase 7 task 7.2 — provider error diagnostic classification (opi-ai layer).
//!
//! `opi-ai` cannot depend on `opi-agent` (the dependency graph runs the other
//! way), so it cannot see the shared [`opi_agent::Diagnostic`] type. The
//! provider-side classification surface therefore lives here as a stable
//! taxonomy: [`ProviderError::category`] returns a [`ProviderErrorCategory`],
//! and `opi-agent` maps each category into the shared diagnostic vocabulary
//! (`code`/`severity`/`source`). These tests pin the taxonomy and its
//! consistency with retryability, with no network access and no provider
//! backend.

use std::collections::HashSet;

use opi_ai::http::safe_excerpt;
use opi_ai::message::{ImageSource, InputContent, MediaType, Message, UserMessage};
use opi_ai::provider::{
    CacheRetention, ModelInfo, ProviderError, ProviderErrorCategory, Request, ThinkingConfig,
    validate_request_capabilities,
};
use opi_ai::registry::ModelCapabilities;
use opi_ai::test_support::MockProvider;
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// category(): each variant maps to a stable diagnostic category
// ---------------------------------------------------------------------------

#[test]
fn rate_limited_classifies_as_rate_limit() {
    assert_eq!(
        ProviderError::RateLimited {
            retry_after_ms: Some(5_000)
        }
        .category(),
        ProviderErrorCategory::RateLimit
    );
    assert_eq!(
        ProviderError::RateLimited {
            retry_after_ms: None
        }
        .category(),
        ProviderErrorCategory::RateLimit
    );
}

#[test]
fn timeout_classifies_as_network() {
    // Phase 12 spec: the `network` class covers "DNS, TLS, proxy, timeout" --
    // timeout is a network sub-case, not a top-level class.
    assert_eq!(
        ProviderError::Timeout.category(),
        ProviderErrorCategory::Network
    );
}

#[test]
fn network_error_classifies_as_network() {
    assert_eq!(
        ProviderError::Network("dns lookup failed".into()).category(),
        ProviderErrorCategory::Network
    );
}

#[test]
fn config_error_classifies_as_config() {
    assert_eq!(
        ProviderError::Config("invalid endpoint".into()).category(),
        ProviderErrorCategory::Config
    );
}

#[test]
fn provider_side_error_classifies_as_provider() {
    assert_eq!(
        ProviderError::ProviderSide("HTTP 500: internal error".into()).category(),
        ProviderErrorCategory::Provider
    );
}

#[test]
fn unsupported_capability_classifies_as_capability() {
    assert_eq!(
        ProviderError::UnsupportedCapability("model does not support image input".into())
            .category(),
        ProviderErrorCategory::Capability
    );
}

#[test]
fn cancelled_classifies_as_cancelled() {
    assert_eq!(
        ProviderError::Cancelled.category(),
        ProviderErrorCategory::Cancelled
    );
}

#[test]
fn taxonomy_exposes_exactly_the_nine_phase12_classes() {
    // Phase 12 design spec Error Taxonomy:
    // auth, config, request, network, rate_limit, provider, stream,
    // capability, cancelled.
    let all = [
        ProviderErrorCategory::Auth,
        ProviderErrorCategory::Config,
        ProviderErrorCategory::Request,
        ProviderErrorCategory::Network,
        ProviderErrorCategory::RateLimit,
        ProviderErrorCategory::Provider,
        ProviderErrorCategory::Stream,
        ProviderErrorCategory::Capability,
        ProviderErrorCategory::Cancelled,
    ];
    let set: HashSet<&ProviderErrorCategory> = all.iter().collect();
    assert_eq!(
        set.len(),
        9,
        "taxonomy must expose exactly the nine Phase 12 classes"
    );
}

#[test]
fn request_failed_classifies_as_request() {
    assert_eq!(
        ProviderError::RequestFailed("internal server error".into()).category(),
        ProviderErrorCategory::Request
    );
}

#[test]
fn stream_error_classifies_as_stream() {
    assert_eq!(
        ProviderError::StreamError("connection reset".into()).category(),
        ProviderErrorCategory::Stream
    );
}

#[test]
fn auth_failed_classifies_as_auth() {
    assert_eq!(
        ProviderError::AuthFailed("invalid api key".into()).category(),
        ProviderErrorCategory::Auth
    );
}

// ---------------------------------------------------------------------------
// retry_after_ms(): only RateLimited carries a server-advised delay
// ---------------------------------------------------------------------------

#[test]
fn rate_limited_exposes_retry_after_ms() {
    assert_eq!(
        ProviderError::RateLimited {
            retry_after_ms: Some(7_500)
        }
        .retry_after_ms(),
        Some(7_500)
    );
    assert_eq!(
        ProviderError::RateLimited {
            retry_after_ms: None
        }
        .retry_after_ms(),
        None
    );
}

#[test]
fn non_rate_limit_errors_have_no_retry_after() {
    for error in [
        ProviderError::Timeout,
        ProviderError::Network("boom".into()),
        ProviderError::RequestFailed("boom".into()),
        ProviderError::Config("boom".into()),
        ProviderError::ProviderSide("boom".into()),
        ProviderError::StreamError("boom".into()),
        ProviderError::AuthFailed("boom".into()),
        ProviderError::UnsupportedCapability("boom".into()),
        ProviderError::Cancelled,
    ] {
        assert_eq!(error.retry_after_ms(), None);
    }
}

// ---------------------------------------------------------------------------
// category() is consistent with is_retryable()
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Phase 12 task 12.2 — safe body excerpts for provider error diagnostics.
//
// Provider error bodies flow into `ProviderError` strings that may be logged or
// surfaced. `safe_excerpt` is the adapter-layer defense that strips known
// credential patterns (API keys, bearer tokens, GitHub PATs, JWTs, credentialed
// URL userinfo) and caps length, so a provider body echoing a secret cannot
// leak even before the diagnostic-layer redaction runs.
// ---------------------------------------------------------------------------

#[test]
fn safe_excerpt_redacts_anthropic_style_api_key() {
    let secret = "sk-ant-api03-1234567890abcdefghijklmnopqrstuv";
    let out = safe_excerpt(&format!("error: token {secret} rejected"));
    assert!(out.contains("[REDACTED]"), "got: {out}");
    assert!(!out.contains(secret), "api key must be redacted: {out}");
}

#[test]
fn safe_excerpt_redacts_openai_style_api_key() {
    let secret = "sk-proj-1234567890abcdefghijklmnopqrstuv";
    let out = safe_excerpt(&format!("bad key {secret}"));
    assert!(!out.contains(secret), "api key must be redacted: {out}");
}

#[test]
fn safe_excerpt_redacts_github_pat() {
    let secret = "ghp_01234567890123456789012345678901234567";
    let out = safe_excerpt(&format!("leaked {secret} in body"));
    assert!(!out.contains(secret), "pat must be redacted: {out}");
}

#[test]
fn safe_excerpt_redacts_google_api_key() {
    // Google AI Studio keys: "AIza" + 35+ base64url chars.
    let secret = "AIzaSyabcdefghijklmnopqrstuvwxyz0123456789ABCDEFGHIJKLMNOP";
    let out = safe_excerpt(&format!("bad key {secret}"));
    assert!(
        !out.contains(secret),
        "google api key must be redacted: {out}"
    );
}

#[test]
fn safe_excerpt_redacts_github_fine_grained_pat() {
    // Fine-grained PAT: "github_pat_" + 82+ chars.
    let token = format!("github_pat_{}", "0".repeat(82));
    let out = safe_excerpt(&format!("leaked {token}"));
    assert!(
        !out.contains("github_pat_"),
        "fine-grained PAT must be redacted: {out}"
    );
}

#[test]
fn safe_excerpt_redacts_bearer_token() {
    let secret = "opaqueTok1234567890abcdefghijklmnopqrstuv";
    let out = safe_excerpt(&format!("Authorization: Bearer {secret}"));
    assert!(
        !out.contains(secret),
        "bearer token must be redacted: {out}"
    );
    assert!(out.contains("Bearer"), "bearer label preserved: {out}");
}

#[test]
fn safe_excerpt_redacts_jwt_bearer() {
    let jwt = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIg.abc123def456";
    let out = safe_excerpt(&format!("Authorization: Bearer {jwt}"));
    assert!(!out.contains("eyJhbGci"), "jwt must be redacted: {out}");
}

#[test]
fn safe_excerpt_redacts_credentialed_url_userinfo() {
    let out = safe_excerpt("https://alice:s3cr3t@gitlab.example.com/owner/repo.git");
    assert!(!out.contains("alice"), "username must be redacted: {out}");
    assert!(!out.contains("s3cr3t"), "password must be redacted: {out}");
    assert!(out.contains("gitlab.example.com"), "host preserved: {out}");
}

#[test]
fn safe_excerpt_truncates_long_bodies() {
    let out = safe_excerpt(&"x".repeat(600));
    assert!(
        out.chars().count() <= 257,
        "excerpt must be capped (256 + ellipsis): got {} chars",
        out.chars().count()
    );
    assert!(
        out.ends_with('…'),
        "truncated excerpt ends with ellipsis: {out}"
    );
}

#[test]
fn safe_excerpt_preserves_benign_content() {
    let out = safe_excerpt("internal server error");
    assert_eq!(out, "internal server error");
}

// ---------------------------------------------------------------------------
// Phase 12 task 12.2 — capability preflight (validate_request_capabilities).
//
// DoD: "Unsupported image/tool/thinking maps to capability before a live call
// when possible." Image and thinking support are known per ModelInfo, so the
// preflight rejects them locally with the `capability` class. Tool capability
// has no per-model signal today, so it remains "when possible" (not possible).
// ---------------------------------------------------------------------------

fn text_only_model(id: &str) -> ModelInfo {
    ModelInfo::new(
        id,
        id,
        opi_ai::WireApi::OpenAiCompletions,
        ModelCapabilities::new(100_000, 4_096).with_streaming(true),
    )
}

fn image_request(model: &str) -> Request {
    Request {
        model: model.into(),
        system: None,
        messages: vec![Message::User(UserMessage {
            content: vec![InputContent::Image {
                source: ImageSource::Url {
                    url: "https://example.test/img.png".into(),
                },
                media_type: MediaType::Png,
            }],
            timestamp_ms: 0,
        })],
        tools: vec![],
        max_tokens: Some(1024),
        temperature: None,
        thinking: ThinkingConfig::default(),
        stop_sequences: vec![],
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: vec![],
        cache_retention: CacheRetention::None,
        session_id: None,
    }
}

#[test]
fn validate_rejects_image_on_text_only_model_as_capability() {
    let provider =
        MockProvider::new_with_models("mock", vec![text_only_model("text-only")], vec![]);
    let err = validate_request_capabilities(&provider, &image_request("mock:text-only"))
        .expect_err("text-only model must reject image input before the call");
    assert_eq!(
        err.category(),
        ProviderErrorCategory::Capability,
        "image preflight must classify as capability: {err:?}"
    );
    assert!(
        matches!(err, ProviderError::UnsupportedCapability(_)),
        "must be the UnsupportedCapability variant: {err:?}"
    );
}

#[test]
fn validate_allows_image_on_image_capable_model() {
    let mut model = text_only_model("image-capable");
    model.capabilities.supports_images = true;
    let provider = MockProvider::new_with_models("mock", vec![model], vec![]);
    validate_request_capabilities(&provider, &image_request("mock:image-capable"))
        .expect("image-capable model must accept image input");
}

#[test]
fn validate_rejects_unsupported_thinking_on_chat_wire_before_request() {
    // C8: unsupported thinking levels are rejected before request construction
    // on every wire (including OpenAI Chat/Responses), so the request never
    // reaches a serializer that would silently omit the reasoning field.
    let provider =
        MockProvider::new_with_models("mock", vec![text_only_model("text-only")], vec![]);
    let mut request = image_request("mock:text-only");
    request.thinking = ThinkingConfig {
        enabled: true,
        budget_tokens: Some(1024),
        level: opi_ai::ThinkingLevel::Medium,
    };
    request.messages = vec![Message::User(UserMessage {
        content: vec![InputContent::Text {
            text: "think about this".into(),
        }],
        timestamp_ms: 0,
    })];
    let err = validate_request_capabilities(&provider, &request)
        .expect_err("unsupported thinking must be rejected before request construction");
    assert!(
        matches!(err, ProviderError::UnsupportedCapability(_)),
        "got {err:?}"
    );
}

#[test]
fn retryable_categories_are_rate_limit_and_network_only() {
    // Retryable: rate limits and transient network/timeout failures.
    // Non-retryable: local request/config validation, provider-side 4xx/5xx,
    // mid-stream failures, auth, capability preflight, and cancellation.
    for error in [
        ProviderError::RateLimited {
            retry_after_ms: None,
        },
        ProviderError::Timeout,
        ProviderError::Network("transient dns failure".into()),
    ] {
        assert!(
            error.is_retryable(),
            "{:?} should be retryable",
            error.category()
        );
    }
    for error in [
        ProviderError::RequestFailed("boom".into()),
        ProviderError::Config("boom".into()),
        ProviderError::ProviderSide("boom".into()),
        ProviderError::StreamError("boom".into()),
        ProviderError::AuthFailed("boom".into()),
        ProviderError::UnsupportedCapability("boom".into()),
        ProviderError::Cancelled,
    ] {
        assert!(
            !error.is_retryable(),
            "{:?} should not be retryable",
            error.category()
        );
    }
}

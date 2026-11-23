//! Overflow and recoverable-length classification on canonical assistant messages.

use opi_ai::ApiKind;
use opi_ai::message::{AssistantMessage, is_context_overflow, is_recoverable_length};
use opi_ai::stream::{StopReason, Usage};

fn diagnostic_message(
    provider: &str,
    stop_reason: StopReason,
    error_message: Option<&str>,
    usage: Usage,
) -> AssistantMessage {
    AssistantMessage {
        projection: None,
        content: vec![],
        api: ApiKind::Anthropic,
        provider: provider.to_owned(),
        model: "test-model".to_owned(),
        response_model: None,
        wire: None,
        response_id: None,
        usage,
        stop_reason,
        error_message: error_message.map(str::to_owned),
        timestamp_ms: 0,
        diagnostics: vec![],
        duration_ms: None,
        raw_stop_reason: None,
        provider_thinking_level: None,

        deferred: None,
    }
}

fn known_usage(input: u32, cache_read: u32, output: u32) -> Usage {
    Usage {
        input_tokens: input,
        cache_read_tokens: cache_read,
        output_tokens: output,
        reported: true,
        ..Usage::unknown()
    }
}

#[test]
fn context_overflow_matches_pinned_provider_error_messages() {
    let error_messages = [
        "prompt is too long: 213462 tokens > 200000 maximum",
        "Prompt too long",
        "Prompt exceeds max length",
        "413 {\"error\":{\"type\":\"request_too_large\",\"message\":\"Request exceeds the maximum size\"}}",
        "input is too long for requested model",
        "Your input exceeds the context window of this model",
        "Requested token count exceeds the model's maximum context length of 131072 tokens",
        "Input length (265330) exceeds maximum context length (262144).",
        "400: litellm.ContextWindowExceededError: Context Window exceeded for given call.",
        "The input token count (1196265) exceeds the maximum number of tokens allowed (1048575)",
        "This model's maximum prompt length is 131072 but the request contains 537812 tokens",
        "Please reduce the length of the messages or completion",
        "This endpoint's maximum context length is 131072 tokens. However, you requested about 140000 tokens",
        "Input length 300000 exceeds the maximum allowed input length of 262144 tokens.",
        "The input (537812 tokens) is longer than the model's context length (131072 tokens).",
        "prompt token count of 537812 exceeds the limit of 131072",
        "the request exceeds the available context size, try increasing it",
        "tokens to keep from the initial prompt is greater than the context length",
        "invalid params, context window exceeds limit",
        "Your request exceeded model token limit: 131072 (requested: 537812)",
        "Prompt contains 537812 tokens ... too large for model with 131072 maximum context length",
        "Prompt has 537,812 tokens, but the configured context size is 131,072 tokens",
        "model_context_window_exceeded",
        "prompt too long; exceeded max context length by 8192 tokens",
        "Range of input length should be [1, 1048575]",
        "context_length_exceeded",
        "context length exceeded",
        "too many tokens",
        "token limit exceeded",
    ];
    for error_message in error_messages {
        let message = diagnostic_message(
            "openai",
            StopReason::Error,
            Some(error_message),
            Usage::unknown(),
        );
        assert!(
            is_context_overflow(&message, None),
            "expected overflow for {error_message:?}"
        );
    }
}

#[test]
fn context_overflow_uses_the_leading_http_413_status() {
    for (diagnostic, expected) in [
        ("413", true),
        ("413: Payload Too Large", true),
        (r#"413 {"message":"failed to parse request"}"#, true),
        ("413 (no body)", true),
        ("413\u{FEFF}Payload Too Large", true),
        (
            "github-copilot API error (413): failed to parse request",
            true,
        ),
        ("Custom_Provider-2 API ERROR (413): Payload Too Large", true),
        ("github-copilot API error (400): invalid value 413", false),
        (
            "custom-provider API error (400): upstream mentioned OpenAI API error (413): invalid input",
            false,
        ),
        ("custom-provider API error (4130): invalid input", false),
        ("4130 failed to parse request", false),
        ("413\u{0085}Payload Too Large", false),
        ("ÅI API error (413): Payload Too Large", false),
    ] {
        let message = diagnostic_message(
            "openai",
            StopReason::Error,
            Some(diagnostic),
            Usage::unknown(),
        );
        assert_eq!(
            is_context_overflow(&message, None),
            expected,
            "{diagnostic:?}"
        );
    }

    for stop in [StopReason::Aborted, StopReason::Stop] {
        let message = diagnostic_message("openai", stop, Some("413"), Usage::unknown());
        assert!(!is_context_overflow(&message, None), "stop={stop:?}");
    }
}

#[test]
fn context_overflow_excludes_non_overflow_patterns() {
    let excluded = [
        "Throttling error: Too many tokens, please wait before trying again.",
        "Service unavailable: too many tokens",
        "rate limit hit while checking too many tokens",
        "Too many requests: token limit exceeded",
        "Throttling error: Context Window exceeded for given call.",
        "413: rate limit reached",
        "github-copilot API error (413): Too many requests",
    ];
    for error_message in excluded {
        let message = diagnostic_message(
            "bedrock",
            StopReason::Error,
            Some(error_message),
            Usage::unknown(),
        );
        assert!(
            !is_context_overflow(&message, None),
            "expected exclusion for {error_message:?}"
        );
    }
    // Unrelated failures and absent error text never classify as overflow.
    let unrelated = diagnostic_message(
        "openai",
        StopReason::Error,
        Some("internal hiccup"),
        Usage::unknown(),
    );
    assert!(!is_context_overflow(&unrelated, None));
    let silent = diagnostic_message("openai", StopReason::Error, None, Usage::unknown());
    assert!(!is_context_overflow(&silent, None));
}

#[test]
fn context_overflow_cerebras_bodyless_pattern_is_provider_gated() {
    for diagnostic in ["400 status code (no body)", "413(no body)"] {
        let bodyless = diagnostic_message(
            "cerebras",
            StopReason::Error,
            Some(diagnostic),
            Usage::unknown(),
        );
        assert!(is_context_overflow(&bodyless, None), "{diagnostic:?}");
        let other_provider = diagnostic_message(
            "openai",
            StopReason::Error,
            Some(diagnostic),
            Usage::unknown(),
        );
        assert!(
            !is_context_overflow(&other_provider, None),
            "{diagnostic:?}"
        );
    }
    let other_status = diagnostic_message(
        "cerebras",
        StopReason::Error,
        Some("500 (no body)"),
        Usage::unknown(),
    );
    assert!(!is_context_overflow(&other_status, None));
}

#[test]
fn context_overflow_silent_stop_branch_requires_known_usage_and_window() {
    let overflow = diagnostic_message("zai", StopReason::Stop, None, known_usage(213_462, 0, 10));
    assert!(is_context_overflow(&overflow, Some(200_000)));
    let with_cache = diagnostic_message(
        "zai",
        StopReason::Stop,
        None,
        known_usage(210_000, 4_000, 10),
    );
    assert!(is_context_overflow(&with_cache, Some(200_000)));
    // No window, zero window, under-window, unknown usage and non-stop
    // stop reasons never fire the silent branch.
    assert!(!is_context_overflow(&overflow, None));
    assert!(!is_context_overflow(&overflow, Some(0)));
    let under = diagnostic_message("zai", StopReason::Stop, None, known_usage(199_999, 0, 10));
    assert!(!is_context_overflow(&under, Some(200_000)));
    let unknown = diagnostic_message("zai", StopReason::Stop, None, Usage::unknown());
    assert!(!is_context_overflow(&unknown, Some(200_000)));
    let errored = diagnostic_message("zai", StopReason::Error, None, known_usage(213_462, 0, 0));
    assert!(!is_context_overflow(&errored, Some(200_000)));
}

#[test]
fn context_overflow_length_stop_branch_detects_filled_window() {
    let filled = diagnostic_message("mimo", StopReason::Length, None, known_usage(99_000, 0, 0));
    assert!(is_context_overflow(&filled, Some(100_000)));
    let exactly_99 = diagnostic_message(
        "mimo",
        StopReason::Length,
        None,
        known_usage(98_000, 1_000, 0),
    );
    assert!(is_context_overflow(&exactly_99, Some(100_000)));
    let below = diagnostic_message("mimo", StopReason::Length, None, known_usage(98_998, 0, 0));
    assert!(!is_context_overflow(&below, Some(100_000)));
    let with_output =
        diagnostic_message("mimo", StopReason::Length, None, known_usage(99_000, 0, 1));
    assert!(!is_context_overflow(&with_output, Some(100_000)));
    let unknown = diagnostic_message("mimo", StopReason::Length, None, Usage::unknown());
    assert!(!is_context_overflow(&unknown, Some(100_000)));
    assert!(!is_context_overflow(&filled, None));
    assert!(!is_context_overflow(&filled, Some(0)));
}

#[test]
fn is_recoverable_length_matches_source_comparison() {
    let truncated = diagnostic_message("openai", StopReason::Length, None, known_usage(100, 0, 50));
    assert!(is_recoverable_length(&truncated, 100));
    let at_limit = diagnostic_message("openai", StopReason::Length, None, known_usage(100, 0, 100));
    assert!(!is_recoverable_length(&at_limit, 100));
    let over_limit =
        diagnostic_message("openai", StopReason::Length, None, known_usage(100, 0, 120));
    assert!(!is_recoverable_length(&over_limit, 100));
    assert!(!is_recoverable_length(&truncated, 0));
    let stopped = diagnostic_message("openai", StopReason::Stop, None, known_usage(100, 0, 50));
    assert!(!is_recoverable_length(&stopped, 100));
    // Unknown usage never converts to zero tokens.
    let unknown = diagnostic_message("openai", StopReason::Length, None, Usage::unknown());
    assert!(!is_recoverable_length(&unknown, 100));
}

#[test]
fn context_overflow_length_stop_handles_full_u64_window_range() {
    let full = diagnostic_message(
        "mimo",
        StopReason::Length,
        None,
        known_usage(u32::MAX, u32::MAX, 0),
    );
    // 8_589_934_590 input tokens fill at least 99% through this last window.
    assert!(is_context_overflow(&full, Some(8_676_701_606)));
    assert!(!is_context_overflow(&full, Some(8_676_701_607)));
    for window in [
        u64::MAX / 100,
        u64::MAX / 100 + 1,
        u64::MAX / 99,
        u64::MAX / 99 + 1,
        186_330_748_219_288_401,
        u64::MAX - 1,
        u64::MAX,
    ] {
        assert!(!is_context_overflow(&full, Some(window)), "window={window}");
    }
}

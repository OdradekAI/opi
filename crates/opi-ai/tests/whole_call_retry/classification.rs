//! Retry classification for canonical assistant results and diagnostic text.

use crate::fixtures::message;
use opi_ai::retry::WholeCallRetryClassification;
use opi_ai::stream::StopReason;

#[test]
fn classification_vetoes_quota_billing_and_usage_limits_first() {
    let vetoed = [
        "GoUsageLimitError: weekly cap",
        "FreeUsageLimitError hit",
        "Monthly usage limit reached, enable spending",
        "insufficient_quota: you exceeded your quota",
        "out of budget",
        "quota exceeded for this key",
        "billing hard limit reached",
        "subscription_sharing_usage_limit_exceeded",
        // A transient-looking token inside a vetoed message stays vetoed.
        "insufficient_quota despite the 429",
    ];
    for text in vetoed {
        let classified =
            opi_ai::retry::classify_whole_call_retry(&message(StopReason::Error, Some(text)));
        assert_eq!(
            classified,
            WholeCallRetryClassification::Terminal,
            "must veto first: {text}"
        );
    }
}

#[test]
fn classification_vetoes_context_overflow_before_transient_patterns() {
    for text in [
        "maximum context length is 500000 tokens",
        "HTTP 500: prompt is too long",
        "input token count 504000 exceeds the maximum 500000",
        "input (503000 tokens) is longer than the model's context length (500000 tokens)",
        "too large for model with 429000 maximum context length",
        "503 litellm.ContextWindowExceededError: Context Window exceeded for given call.",
        "custom-provider API error (413): upstream returned 503",
    ] {
        let failed = message(StopReason::Error, Some(text));
        assert!(
            opi_ai::message::is_context_overflow(&failed, None),
            "fixture must match the existing overflow utility: {text}"
        );
        assert_eq!(
            opi_ai::retry::classify_whole_call_retry(&failed),
            WholeCallRetryClassification::Terminal,
            "overflow vetoes transient-looking token counts or statuses: {text}"
        );
    }
}

#[test]
fn classification_keeps_context_overflow_utility_exclusions() {
    for text in [
        "rate limit exceeded: maximum context length is 500000 tokens",
        "Throttling error: prompt is too long (HTTP 503)",
        "Service unavailable: input token count 504000 exceeds the maximum 500000",
        "too many requests; maximum context length is 500000 tokens",
        "413: rate limit reached",
    ] {
        let failed = message(StopReason::Error, Some(text));
        assert!(
            !opi_ai::message::is_context_overflow(&failed, None),
            "fixture must retain the existing overflow exclusion: {text}"
        );
        assert_eq!(
            opi_ai::retry::classify_whole_call_retry(&failed),
            WholeCallRetryClassification::Transient,
            "the overflow veto must respect source throttling exclusions: {text}"
        );
    }
}

#[test]
fn classification_flags_source_transient_families() {
    let transient = [
        "The server is overloaded",
        "server_busy",
        "servers are currently busy",
        "currently experiencing high demand",
        "model is at capacity",
        "rate limit exceeded",
        "Too Many Requests",
        "HTTP 429",
        "HTTP 500",
        "502 Bad Gateway",
        "503 Service Unavailable",
        "503 Service Unavailable: request body mentioned 413",
        "504 Gateway Timeout",
        "520",
        "524",
        "The service is temporarily unavailable",
        "service unavailable",
        "server error",
        "internal error",
        "Provider returned error",
        "The upstream provider returned an error",
        "unable to forward upstream",
        "exceeded request buffer limit while retrying upstream",
        "network error",
        "connection error",
        "connection refused",
        "connection lost",
        "other side closed",
        "fetch failed",
        "getaddrinfo ENOTFOUND",
        "EAI_AGAIN",
        "upstream connect error",
        "reset before headers",
        "socket hang up",
        "socket connection was closed",
        "request timed out",
        "timeout while waiting",
        "terminated",
        "websocket closed",
        "websocket error",
        "stream ended without a message_stop",
        "stream ended before message_stop",
        "stream ended before a terminal response event",
        "http2 request did not get a response",
        "pending stream has been canceled",
        "retry delay exceeded the maximum",
        "You can retry your request",
        "Please try your request again after 30s",
        "ResourceExhausted",
        "subscription_sharing_usage_unavailable",
        "subscription_sharing_user_unavailable",
    ];
    for text in transient {
        let classified =
            opi_ai::retry::classify_whole_call_retry(&message(StopReason::Error, Some(text)));
        assert_eq!(
            classified,
            WholeCallRetryClassification::Transient,
            "must be transient: {text}"
        );
    }
}

#[test]
fn classification_marks_valid_stops_aborted_and_bare_errors() {
    for stop in [StopReason::Stop, StopReason::Length, StopReason::ToolUse] {
        assert_eq!(
            opi_ai::retry::classify_whole_call_retry(&message(stop, None)),
            WholeCallRetryClassification::Valid
        );
    }
    assert_eq!(
        opi_ai::retry::classify_whole_call_retry(&message(StopReason::Aborted, None)),
        WholeCallRetryClassification::Aborted
    );
    assert_eq!(
        opi_ai::retry::classify_whole_call_retry(&message(StopReason::Error, None)),
        WholeCallRetryClassification::Terminal
    );
    // Deterministic errors fail fast.
    assert_eq!(
        opi_ai::retry::classify_whole_call_retry(&message(
            StopReason::Error,
            Some("no such model: unknown-model-id")
        )),
        WholeCallRetryClassification::Terminal
    );
}

#[test]
fn classification_matches_frozen_retry_rule_changes() {
    let mut mismatches = Vec::new();
    for (diagnostic, expected) in [
        ("upstream_error: Upstream request failed", true),
        ("upstream_stream_read_error", true),
        ("Stream was interrupted", true),
        ("Upstream HTTP/2 stream failed", true),
        ("Upstream HTTP2 stream failed", true),
        ("upstream\rrequest failed", false),
        (
            "request to https://oauth2.googleapis.com/token failed, reason:",
            true,
        ),
        (
            "request to https://example.invalid/\u{FEFF}token failed, reason:",
            false,
        ),
        (
            "stream disconnected before completion: stream closed before response.completed",
            true,
        ),
        ("HTTP 429", true),
        ("_503_", true),
        ("Å503Å", true),
        ("Status 503XYZ", false),
        ("Status ABC503", false),
        (r#"401 {"request_id":"req_011CU503xyz"}"#, false),
        (
            "400 image exceeds 5 MB maximum: 5316784 bytes > 5242880 bytes",
            false,
        ),
        (
            r#"OpenAI API error (429): {"code":"subscription_sharing_workplace_individual_usage_limit_exceeded"}"#,
            false,
        ),
        (r#"429 {"code":"1308"}"#, false),
        ("429 {\"code\"\u{FEFF}:\t1308}", false),
        ("429 {\"code\"\u{0085}:1308}", true),
        (r#"429 {"code":13080}"#, true),
        ("429 request reference 1308", true),
        (
            "503 litellm.ContextWindowExceededError: Context Window exceeded for given call.",
            false,
        ),
    ] {
        let actual =
            opi_ai::retry::classify_whole_call_retry(&message(StopReason::Error, Some(diagnostic)))
                == WholeCallRetryClassification::Transient;
        println!(
            "retry classification observation {}",
            serde_json::json!({"diagnostic": diagnostic, "retryable": actual, "expected": expected})
        );
        if actual != expected {
            mismatches.push(diagnostic);
        }
    }
    assert!(
        mismatches.is_empty(),
        "retry classification differs for {mismatches:?}"
    );
}

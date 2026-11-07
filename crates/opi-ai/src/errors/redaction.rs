//! Canonical diagnostic redaction, independent of transports.

/// Maximum number of characters retained in a provider error body excerpt.
const SAFE_EXCERPT_MAX_CHARS: usize = 256;

static SECRET_KEY_RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
static BEARER_RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
static CREDENTIALED_URL_RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
static QUERY_SECRET_RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();

fn secret_key_re() -> &'static regex::Regex {
    SECRET_KEY_RE.get_or_init(|| {
        regex::Regex::new(
            r"sk-[A-Za-z0-9-]{20,}|gh[pousr]_[A-Za-z0-9]{36,}|github_pat_[A-Za-z0-9_]{82,}|AIza[0-9A-Za-z_-]{35,}|eyJ[A-Za-z0-9_-]{8,}\.eyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}",
        )
        .expect("valid secret-key regex")
    })
}

fn bearer_re() -> &'static regex::Regex {
    BEARER_RE.get_or_init(|| {
        regex::Regex::new(r"(?i)bearer\s+[A-Za-z0-9._-]+").expect("valid bearer regex")
    })
}

fn credentialed_url_re() -> &'static regex::Regex {
    CREDENTIALED_URL_RE.get_or_init(|| {
        regex::Regex::new(r"(?P<scheme>[a-zA-Z][a-zA-Z0-9+.-]*://)[^/\s:@]+:[^/\s@]+@")
            .expect("valid credentialed-url regex")
    })
}

fn query_secret_re() -> &'static regex::Regex {
    QUERY_SECRET_RE.get_or_init(|| {
        regex::Regex::new(
            r"(?i)([?&](?:api[_-]?key|key|token|access[_-]?token|refresh[_-]?token|session[_-]?token|access[_-]?key[_-]?id|secret[_-]?access[_-]?key|secret|password|authorization|proxy[_-]?authorization)=)[^&#\s]+",
        )
        .expect("valid query-secret regex")
    })
}

/// Produce a redacted, length-capped excerpt of diagnostic text.
///
/// The constructor-enforced [`crate::provider::ProviderErrorSummary`] is the
/// producer boundary for public provider errors. This helper is a secondary
/// defense for locally-produced context and transport errors: it strips known
/// credential patterns (including credential-bearing URL query keys) and caps
/// the excerpt length.
pub fn safe_excerpt(body: &str) -> String {
    let scrubbed = secret_key_re().replace_all(body, "[REDACTED]").into_owned();
    let scrubbed = bearer_re()
        .replace_all(&scrubbed, "Bearer [REDACTED]")
        .into_owned();
    let scrubbed = credentialed_url_re()
        .replace_all(&scrubbed, "${scheme}[REDACTED]@")
        .into_owned();
    let scrubbed = query_secret_re()
        .replace_all(&scrubbed, "${1}[REDACTED]")
        .into_owned();
    if scrubbed.chars().count() > SAFE_EXCERPT_MAX_CHARS {
        let truncated: String = scrubbed.chars().take(SAFE_EXCERPT_MAX_CHARS).collect();
        format!("{truncated}…")
    } else {
        scrubbed
    }
}

//! Internal URL joining helper for provider endpoint paths.

/// Join a provider base URL and endpoint path without duplicating slashes.
///
/// `path` must be an absolute URL path. Query strings are allowed.
pub(crate) fn join_endpoint(base_url: &str, path: &str) -> String {
    let base = base_url.trim_end_matches('/');
    let path = if path.starts_with('/') {
        path.to_owned()
    } else {
        format!("/{path}")
    };
    format!("{base}{path}")
}

#[cfg(test)]
mod tests {
    use super::join_endpoint;

    #[test]
    fn joins_without_double_slashes() {
        assert_eq!(
            join_endpoint("https://api.example.com/", "/v1/chat/completions"),
            "https://api.example.com/v1/chat/completions"
        );
        assert_eq!(
            join_endpoint("https://api.example.com/root", "v1/messages"),
            "https://api.example.com/root/v1/messages"
        );
    }
}

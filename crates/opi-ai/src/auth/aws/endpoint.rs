//! STS endpoint resolution.
//!
//! Pure partition/region arithmetic: no SDK default resolver, no
//! `SMITHY_CLIENT_SDK_CUSTOM_PARTITION`, no ambient reads. An explicit
//! override always wins; China-partition regions resolve to the `.cn`
//! suffix; every other known partition keeps the standard `amazonaws.com`
//! host. Unknown regions are never silently rewritten to us-east-1 — the
//! resolved region (which itself defaults only per the verified corpus)
//! flows through unchanged.

/// Resolve the regional STS endpoint for `region`.
///
/// `endpoint_override` (an explicitly configured endpoint) wins whenever it
/// is non-blank.
pub fn sts_endpoint_url(region: &str, endpoint_override: Option<&str>) -> String {
    if let Some(endpoint) = endpoint_override.filter(|value| !value.trim().is_empty()) {
        return endpoint.trim().to_owned();
    }
    let region = region.trim();
    if region.starts_with("cn-") {
        format!("https://sts.{region}.amazonaws.com.cn")
    } else {
        format!("https://sts.{region}.amazonaws.com")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_and_china_partitions_resolve_regional_endpoints() {
        assert_eq!(
            sts_endpoint_url("us-west-2", None),
            "https://sts.us-west-2.amazonaws.com"
        );
        assert_eq!(
            sts_endpoint_url("cn-north-1", None),
            "https://sts.cn-north-1.amazonaws.com.cn"
        );
        assert_eq!(
            sts_endpoint_url("us-gov-west-1", None),
            "https://sts.us-gov-west-1.amazonaws.com"
        );
    }

    #[test]
    fn explicit_override_wins_and_blank_overrides_are_ignored() {
        assert_eq!(
            sts_endpoint_url("us-west-2", Some("https://sts.example.invalid")),
            "https://sts.example.invalid"
        );
        assert_eq!(
            sts_endpoint_url("eu-central-1", Some("  ")),
            "https://sts.eu-central-1.amazonaws.com"
        );
    }

    #[test]
    fn unknown_regions_never_become_us_east_1() {
        assert_eq!(
            sts_endpoint_url("not-a-known-region", None),
            "https://sts.not-a-known-region.amazonaws.com"
        );
    }
}

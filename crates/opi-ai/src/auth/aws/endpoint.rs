//! STS, SSO, SSO OIDC, and SignIn endpoint resolution.
//!
//! Pure partition/region arithmetic: no SDK default resolver, no
//! `SMITHY_CLIENT_SDK_CUSTOM_PARTITION`, no ambient reads. An explicit
//! override always wins; China-partition regions resolve to the `.cn`
//! suffix; every other known partition keeps the standard `amazonaws.com`
//! host. Unknown regions are never silently rewritten to us-east-1 — the
//! resolved region (which itself defaults only per the verified corpus)
//! flows through unchanged.
//!
//! The SSO portal (`portal.sso.*`) and SSO OIDC (`oidc.*`) hosts follow the
//! standard published regional endpoints. The corpus pins the SignIn wire
//! path (`POST /v1/token`) but not its host; the `signin.{region}` host is
//! a documented opi default behind the injectable override.

/// Resolve the regional STS endpoint for `region`.
///
/// `endpoint_override` (an explicitly configured endpoint) wins whenever it
/// is non-blank.
pub fn sts_endpoint_url(region: &str, endpoint_override: Option<&str>) -> String {
    regional_endpoint("sts", region, endpoint_override)
}

/// Resolve the regional SSO portal endpoint (GetRoleCredentials).
pub fn sso_endpoint_url(region: &str, endpoint_override: Option<&str>) -> String {
    regional_endpoint("portal.sso", region, endpoint_override)
}

/// Resolve the regional SSO OIDC endpoint (CreateToken refresh).
pub fn oidc_endpoint_url(region: &str, endpoint_override: Option<&str>) -> String {
    regional_endpoint("oidc", region, endpoint_override)
}

/// Resolve the SignIn endpoint (CreateOAuth2Token refresh). The corpus pins
/// the wire path (`POST /v1/token`), not the host; this construction is a
/// documented opi default behind the injectable override.
pub fn signin_endpoint_url(region: &str, endpoint_override: Option<&str>) -> String {
    regional_endpoint("signin", region, endpoint_override)
}

/// Shared regional endpoint arithmetic: override wins, China partition
/// takes the `.cn` suffix, unknown regions pass through unchanged.
fn regional_endpoint(service: &str, region: &str, endpoint_override: Option<&str>) -> String {
    if let Some(endpoint) = endpoint_override.filter(|value| !value.trim().is_empty()) {
        return endpoint.trim().to_owned();
    }
    let region = region.trim();
    if region.starts_with("cn-") {
        format!("https://{service}.{region}.amazonaws.com.cn")
    } else {
        format!("https://{service}.{region}.amazonaws.com")
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

    #[test]
    fn sso_oidc_and_signin_follow_the_same_rules() {
        assert_eq!(
            sso_endpoint_url("us-east-1", None),
            "https://portal.sso.us-east-1.amazonaws.com"
        );
        assert_eq!(
            oidc_endpoint_url("us-west-2", None),
            "https://oidc.us-west-2.amazonaws.com"
        );
        assert_eq!(
            signin_endpoint_url("us-east-1", None),
            "https://signin.us-east-1.amazonaws.com"
        );
        assert_eq!(
            oidc_endpoint_url("cn-north-1", None),
            "https://oidc.cn-north-1.amazonaws.com.cn"
        );
        assert_eq!(
            sso_endpoint_url("eu-west-1", Some("https://sso.example.invalid")),
            "https://sso.example.invalid"
        );
    }
}

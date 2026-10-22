//! STS `AssumeRoleWithWebIdentity` source exchanges.
//!
//! Web identity reads the *selected* token file and exchanges it at STS with
//! the same pinned Query codec — but never signs with source SigV4: the
//! request is anonymous and the injected token is the only identity
//! material. A configured-but-unreadable token file, an STS denial, or an
//! invalid response is terminal; the provider never discovers another
//! account or invents source credentials. Subject/audience/provider
//! metadata from the response is retained as non-secret provenance.

use secrecy::SecretString;

use crate::auth::aws::role::{DEFAULT_ROLE_SESSION_NAME, StsDenial, StsExchange, StsResult};
use crate::auth::aws::source_http::{AwsSourceTransport, SourceHttpMethod, SourceHttpRequest};

/// Read the selected web-identity token file. Failure is typed and terminal.
pub(crate) async fn read_web_identity_token(
    path: &std::path::Path,
) -> Result<SecretString, WebIdentityError> {
    let contents = tokio::fs::read_to_string(path)
        .await
        .map_err(|_| WebIdentityError::TokenFileUnreadable)?;
    let trimmed = contents.trim();
    if trimmed.is_empty() {
        return Err(WebIdentityError::TokenFileUnreadable);
    }
    Ok(SecretString::from(trimmed.to_owned()))
}

/// Web-identity-specific typed failures (secret-free).
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub(crate) enum WebIdentityError {
    /// The selected token file could not be read or was empty.
    #[error("web identity token file is unreadable")]
    TokenFileUnreadable,
    /// Transport-level failure of the anonymous exchange.
    #[error("web identity STS exchange failed")]
    StsTransport,
    /// STS denied the exchange; the typed denial is retained.
    #[error("web identity STS exchange denied")]
    StsDenied(StsDenial),
    /// The response did not satisfy the fixed shape.
    #[error("web identity STS response was invalid")]
    StsInvalid,
}

/// One anonymous `AssumeRoleWithWebIdentity` exchange.
///
/// `duration_seconds` is not part of the fixed web-identity form this slice;
/// the parameter is accepted for call-site parity and ignored.
pub(crate) async fn assume_role_with_web_identity(
    transport: &dyn AwsSourceTransport,
    sts_endpoint: &str,
    token: &SecretString,
    role_arn: &str,
    role_session_name: Option<&str>,
    _duration_seconds: Option<&str>,
) -> Result<StsExchange, WebIdentityError> {
    let mut body = String::new();
    {
        let mut writer = aws_smithy_query::QueryWriter::new(
            &mut body,
            "AssumeRoleWithWebIdentity",
            "2011-06-15",
        );
        writer.prefix("RoleArn").string(role_arn);
        writer.prefix("RoleSessionName").string(
            role_session_name
                .filter(|value| !value.trim().is_empty())
                .unwrap_or(DEFAULT_ROLE_SESSION_NAME),
        );
        writer
            .prefix("WebIdentityToken")
            .string(secrecy::ExposeSecret::expose_secret(token));
        writer.finish();
    }

    // Anonymous exchange: the token is the only identity material; no
    // Authorization header is attached (no source SigV4 is invented).
    let request = SourceHttpRequest {
        method: SourceHttpMethod::Post,
        url: format!("{sts_endpoint}/"),
        headers: vec![(
            "Content-Type".to_owned(),
            "application/x-www-form-urlencoded".to_owned(),
        )],
        body: Some(body.into_bytes()),
        timeout: None,
    };
    let response = transport
        .send(request)
        .await
        .map_err(|_| WebIdentityError::StsTransport)?;
    let outcome: StsResult = crate::auth::aws::role::classify_sts_response(response);
    let exchange = outcome.map_err(|error| match error {
        crate::auth::aws::role::StsExchangeError::Transport => WebIdentityError::StsTransport,
        crate::auth::aws::role::StsExchangeError::Denied(denial) => {
            WebIdentityError::StsDenied(denial)
        }
        crate::auth::aws::role::StsExchangeError::InvalidResponse => WebIdentityError::StsInvalid,
    })?;
    // The web-identity root shape is required; a plain AssumeRole response
    // must not satisfy the web-identity route.
    if exchange.subject.is_none() && exchange.audience.is_none() && exchange.provider.is_none() {
        return Err(WebIdentityError::StsInvalid);
    }
    Ok(exchange)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn token_file_read_failure_is_typed() {
        let error = read_web_identity_token(std::path::Path::new("/nonexistent/token.jwt"))
            .await
            .expect_err("missing token file");
        assert_eq!(error, WebIdentityError::TokenFileUnreadable);
    }

    #[test]
    fn exchange_errors_stay_secret_free() {
        let error = WebIdentityError::StsDenied(StsDenial {
            status: 403,
            code: Some("AccessDenied".to_owned()),
            request_id: Some("req-3".to_owned()),
        });
        let rendered = format!("{error:?} {error}");
        assert!(rendered.contains("AccessDenied"));
        assert!(rendered.contains("req-3"));
    }
}

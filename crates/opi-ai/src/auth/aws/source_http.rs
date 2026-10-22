//! Controlled HTTP transport for AWS auth-source exchanges.
//!
//! STS/web-identity/ECS/IMDS legs are owned by the source coordinator: they
//! run through this injectable seam instead of ambient clients, so tests
//! script and count every send (source-exchange sends stay separate from
//! generation attempts) and no leg ever follows redirects or stacks vendor
//! retry on top of the source-owned attempt schedule. Only the per-source
//! fixed budgets (ECS/IMDS 1s per attempt, modern ECS at most four sends,
//! IMDS zero retries) exist; no global send cap is invented.

use std::time::Duration;

use futures_util::future::BoxFuture;

use crate::http::{HttpClient, HttpClientBuilder, RedirectPolicy};

/// One auth-source HTTP request.
///
/// The custom `Debug` redacts headers and body: both can carry credential
/// material (SigV4 authorization, bearer tokens, web-identity tokens).
#[derive(Clone)]
pub struct SourceHttpRequest {
    /// HTTP method.
    pub method: SourceHttpMethod,
    /// Absolute request URL.
    pub url: String,
    /// Header pairs, order preserved as signed/sent.
    pub headers: Vec<(String, String)>,
    /// Request body bytes.
    pub body: Option<Vec<u8>>,
    /// Per-attempt timeout; `None` leaves the request unbounded so the
    /// caller's cancellation remains the only deadline.
    pub timeout: Option<Duration>,
}

impl std::fmt::Debug for SourceHttpRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SourceHttpRequest")
            .field("method", &self.method)
            .field("url", &self.url)
            .field(
                "headers",
                &self
                    .headers
                    .iter()
                    .map(|(name, _)| name.as_str())
                    .collect::<Vec<_>>(),
            )
            .field("body_len", &self.body.as_ref().map(Vec::len))
            .field("timeout", &self.timeout)
            .finish()
    }
}

/// Methods used by the AWS source legs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceHttpMethod {
    /// `GET`.
    Get,
    /// `PUT` (IMDSv2 token acquisition).
    Put,
    /// `POST` (STS form exchanges).
    Post,
}

impl SourceHttpMethod {
    fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Put => "PUT",
            Self::Post => "POST",
        }
    }
}

/// One auth-source HTTP response. Headers are not retained: source legs
/// consume only status and body, and response headers are not diagnostic
/// surface.
#[derive(Debug, Clone)]
pub struct SourceHttpResponse {
    /// HTTP status code.
    pub status: u16,
    /// Response body bytes.
    pub body: Vec<u8>,
}

/// Typed, secret-free transport failures.
#[derive(Debug, thiserror::Error, Clone, Copy, PartialEq, Eq)]
pub enum SourceHttpError {
    /// The exchange failed at the transport level (connect, TLS, read).
    #[error("source http exchange failed")]
    Transport,
    /// The exchange exceeded its per-attempt timeout.
    #[error("source http exchange timed out")]
    Timeout,
    /// The response carried a redirect; auth-source legs never follow.
    #[error("source http exchange refused a redirect")]
    Redirect,
}

/// Injectable transport for AWS auth-source HTTP legs.
pub trait AwsSourceTransport: Send + Sync {
    /// Send one request and await its response. Dropping the returned future
    /// cancels the exchange and releases owned work.
    fn send(
        &self,
        request: SourceHttpRequest,
    ) -> BoxFuture<'_, Result<SourceHttpResponse, SourceHttpError>>;
}

/// Real HTTP transport backed by the controlled opi-ai client.
///
/// Redirects are disabled (a redirect is a typed [`SourceHttpError::Redirect`]
/// surfaced to the source owner, never silently followed) so source identity
/// can never be redirected to another destination.
pub struct ReqwestSourceTransport {
    client: HttpClient,
}

impl Default for ReqwestSourceTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl ReqwestSourceTransport {
    /// Build the source transport with source-qualified defaults.
    pub fn new() -> Self {
        Self {
            client: HttpClientBuilder::new()
                .redirects(RedirectPolicy::None)
                .build()
                .expect("source transport construction should not fail with valid defaults"),
        }
    }
}

impl AwsSourceTransport for ReqwestSourceTransport {
    fn send(
        &self,
        request: SourceHttpRequest,
    ) -> BoxFuture<'_, Result<SourceHttpResponse, SourceHttpError>> {
        Box::pin(async move {
            let method = reqwest::Method::from_bytes(request.method.as_str().as_bytes())
                .expect("source methods are valid HTTP methods");
            let mut builder = self.client.client().request(method, &request.url);
            for (name, value) in &request.headers {
                builder = builder.header(name, value);
            }
            if let Some(timeout) = request.timeout {
                builder = builder.timeout(timeout);
            }
            if let Some(body) = request.body.as_deref() {
                builder = builder.body(body.to_vec());
            }
            let response = builder.send().await.map_err(map_reqwest_error)?;
            let status = response.status().as_u16();
            if response.status().is_redirection() {
                return Err(SourceHttpError::Redirect);
            }
            let body = response.bytes().await.map_err(map_reqwest_error)?;
            Ok(SourceHttpResponse {
                status,
                body: body.to_vec(),
            })
        })
    }
}

fn map_reqwest_error(error: reqwest::Error) -> SourceHttpError {
    if error.is_timeout() {
        SourceHttpError::Timeout
    } else if error.is_redirect() {
        SourceHttpError::Redirect
    } else {
        SourceHttpError::Transport
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_debug_redacts_headers_and_body() {
        let request = SourceHttpRequest {
            method: SourceHttpMethod::Post,
            url: "https://sts.us-west-2.amazonaws.com/".to_owned(),
            headers: vec![("Authorization".to_owned(), "secret-canary".to_owned())],
            body: Some(b"WebIdentityToken=secret-token".to_vec()),
            timeout: None,
        };
        let debug = format!("{request:?}");
        assert!(debug.contains("url"), "{debug}");
        assert!(!debug.contains("secret-canary"), "{debug}");
        assert!(!debug.contains("secret-token"), "{debug}");
        assert!(
            debug.contains("Authorization"),
            "header names are non-secret"
        );
    }
}

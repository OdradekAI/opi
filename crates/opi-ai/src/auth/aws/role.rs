//! STS `AssumeRole` source exchanges.
//!
//! The owned wire: `POST /` with `application/x-www-form-urlencoded`
//! (`Action=AssumeRole`, `Version=2011-06-15`) encoded through the pinned
//! private `aws-smithy-query` codec, signed with the *source* credentials
//! (SigV4, service `sts`). Responses are prechecked lexically/structurally
//! by the owned `xmlparser` walk and then read through the pinned
//! `aws-smithy-xml` decoder against the fixed STS shape; unknown elements
//! are preserved as ignorable, required credential fields follow the real
//! schema, and diagnostics never carry secrets. MFA presents an injected
//! `SerialNumber`/`TokenCode` pair; a required MFA without a presenter is a
//! typed terminal failure (never an interactive prompt).
//!
//! Auth-source retry scheduling is owned by the AWS retry-mode task; each
//! exchange here is exactly one send.

use secrecy::{ExposeSecret, SecretString};
use time::OffsetDateTime;

use crate::auth::AwsSigV4Credentials;
use crate::auth::aws::source_http::{
    AwsSourceTransport, SourceHttpError, SourceHttpMethod, SourceHttpRequest,
};
use crate::bedrock::sigv4::{SignedRequest, sign_request};

/// Default `RoleSessionName` when no profile/explicit session name is set.
/// The fixed corpus does not pin a default; this owned constant keeps
/// exchanges deterministic without claiming SDK identity.
pub(crate) const DEFAULT_ROLE_SESSION_NAME: &str = "opi-assume-role";

/// Injectable MFA presenter: returns the `TokenCode` for a `SerialNumber`.
pub trait MfaTokenPresenter: Send + Sync {
    /// Produce one MFA token code for the given device serial.
    fn token_code(&self, serial: &str) -> Result<String, MfaPresentationError>;
}

/// Typed MFA presentation failures (secret-free by construction).
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum MfaPresentationError {
    /// The presenter refused to produce a token code.
    #[error("MFA presentation failed")]
    PresentationFailed,
}

/// AssumeRole parameters retained from the shared-config profile.
#[derive(Debug, Clone, Default)]
pub(crate) struct RoleAssumption {
    /// `role_arn` (required).
    pub role_arn: String,
    /// `role_session_name`; defaults to [`DEFAULT_ROLE_SESSION_NAME`].
    pub role_session_name: Option<String>,
    /// `external_id`.
    pub external_id: Option<String>,
    /// `duration_seconds`.
    pub duration_seconds: Option<String>,
    /// `mfa_serial`; requires an injected presenter.
    pub mfa_serial: Option<String>,
}

/// One parsed STS `AssumeRole*` response.
#[derive(Debug, Clone)]
pub(crate) struct StsExchange {
    /// Temporary access key id.
    pub access_key_id: SecretString,
    /// Temporary secret access key.
    pub secret_access_key: SecretString,
    /// Temporary session token.
    pub session_token: Option<SecretString>,
    /// Source-reported expiration.
    pub expiration: Option<OffsetDateTime>,
    /// `AssumedRoleUser/Arn`.
    pub assumed_role_user_arn: Option<String>,
    /// `AssumedRoleUser/AssumedRoleId`.
    pub assumed_role_user_id: Option<String>,
    /// Web-identity `Subject` (web-identity exchanges only).
    pub subject: Option<String>,
    /// Web-identity `Audience`.
    pub audience: Option<String>,
    /// Web-identity `Provider`.
    pub provider: Option<String>,
    /// `ResponseMetadata/RequestId`.
    pub request_id: Option<String>,
}

impl StsExchange {
    /// Build the temporary credentials with the source-reported expiry.
    pub fn into_credentials(self, region: &str) -> AwsSigV4Credentials {
        AwsSigV4Credentials {
            access_key_id: self.access_key_id,
            secret_access_key: self.secret_access_key,
            session_token: self.session_token,
            region: region.to_owned(),
            expires_at: self.expiration,
        }
    }
}

impl PartialEq for StsExchange {
    fn eq(&self, other: &Self) -> bool {
        // Secret fields compare by presence only; no secret participates.
        self.access_key_id.expose_secret().len() == other.access_key_id.expose_secret().len()
            && self.secret_access_key.expose_secret().len()
                == other.secret_access_key.expose_secret().len()
            && self.session_token.is_some() == other.session_token.is_some()
            && self.expiration == other.expiration
            && self.assumed_role_user_arn == other.assumed_role_user_arn
            && self.assumed_role_user_id == other.assumed_role_user_id
            && self.subject == other.subject
            && self.audience == other.audience
            && self.provider == other.provider
            && self.request_id == other.request_id
    }
}

impl Eq for StsExchange {}

/// Typed STS exchange failure. Secrets, raw messages, and response bodies
/// never enter these variants.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub(crate) enum StsExchangeError {
    /// Transport-level failure of the exchange.
    #[error("STS exchange failed")]
    Transport,
    /// Non-2xx response with the classified error shape.
    #[error("STS denied the exchange ({0})")]
    Denied(StsDenial),
    /// 2xx response that failed the fixed-shape validation.
    #[error("STS returned an invalid credential response")]
    InvalidResponse,
}

/// Non-secret denial classification retained from an STS `ErrorResponse`:
/// status, standard error code, and request id. Raw messages are dropped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StsDenial {
    /// HTTP status code.
    pub status: u16,
    /// Standard STS error code when present.
    pub code: Option<String>,
    /// Response `RequestId`.
    pub request_id: Option<String>,
}

impl std::fmt::Display for StsDenial {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "status {}, code {}, request id {}",
            self.status,
            self.code.as_deref().unwrap_or("unknown"),
            self.request_id.as_deref().unwrap_or("unknown")
        )
    }
}

/// Outcome of one STS leg.
pub(crate) type StsResult = Result<StsExchange, StsExchangeError>;

/// Encode the AssumeRole form body with the pinned query codec.
fn assume_role_body(params: &RoleAssumption, token_code: Option<&str>) -> String {
    let mut output = String::new();
    let mut writer = aws_smithy_query::QueryWriter::new(&mut output, "AssumeRole", "2011-06-15");
    writer.prefix("RoleArn").string(&params.role_arn);
    writer.prefix("RoleSessionName").string(
        params
            .role_session_name
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(DEFAULT_ROLE_SESSION_NAME),
    );
    if let Some(external_id) = params
        .external_id
        .as_deref()
        .filter(|v| !v.trim().is_empty())
    {
        writer.prefix("ExternalId").string(external_id);
    }
    if let Some(duration) = params
        .duration_seconds
        .as_deref()
        .filter(|v| !v.trim().is_empty())
    {
        writer.prefix("DurationSeconds").string(duration);
    }
    if let Some(serial) = params
        .mfa_serial
        .as_deref()
        .filter(|v| !v.trim().is_empty())
    {
        writer.prefix("SerialNumber").string(serial);
        if let Some(code) = token_code {
            writer.prefix("TokenCode").string(code);
        }
    }
    writer.finish();
    output
}

/// One `AssumeRole` exchange signed with the source credentials.
pub(crate) async fn assume_role(
    transport: &dyn AwsSourceTransport,
    source: &AwsSigV4Credentials,
    sts_endpoint: &str,
    params: &RoleAssumption,
    token_code: Option<&str>,
) -> StsResult {
    let body = assume_role_body(params, token_code);
    send_sts_form(transport, source, sts_endpoint, body).await
}

/// Shared STS form exchange: SigV4 sign with the source credentials
/// (`Anonymous` requests are handled by the web-identity module instead).
pub(crate) async fn send_sts_form(
    transport: &dyn AwsSourceTransport,
    source: &AwsSigV4Credentials,
    sts_endpoint: &str,
    body: String,
) -> StsResult {
    let host = url_host(sts_endpoint);
    let now = OffsetDateTime::now_utc();
    let date_stamp = now
        .format(&time::format_description::parse("[year][month][day]").unwrap())
        .expect("date stamp formats");
    let amz_date = now
        .format(
            &time::format_description::parse("[year][month][day]T[hour][minute][second]Z").unwrap(),
        )
        .expect("amz date formats");
    let signed: SignedRequest = sign_request(
        "POST",
        "/",
        "",
        &[
            ("host", host.as_str()),
            ("content-type", "application/x-www-form-urlencoded"),
            ("x-amz-date", &amz_date),
        ],
        body.as_bytes(),
        source,
        "sts",
        &date_stamp,
        &amz_date,
    );
    let mut headers = vec![
        (
            "Content-Type".to_owned(),
            "application/x-www-form-urlencoded".to_owned(),
        ),
        ("X-Amz-Date".to_owned(), signed.x_amz_date.clone()),
        (
            "Authorization".to_owned(),
            secrecy::ExposeSecret::expose_secret(&signed.authorization).to_owned(),
        ),
    ];
    if let Some(token) = signed
        .x_amz_security_token
        .as_ref()
        .map(secrecy::ExposeSecret::expose_secret)
    {
        headers.push(("X-Amz-Security-Token".to_owned(), token.to_owned()));
    }

    let request = SourceHttpRequest {
        method: SourceHttpMethod::Post,
        url: format!("{sts_endpoint}/"),
        headers,
        body: Some(body.into_bytes()),
        timeout: None,
    };
    let response = transport.send(request).await.map_err(map_transport)?;
    classify_sts_response(response)
}

/// Extract the host:port authority of an endpoint URL (best effort; the
/// endpoint is coordinator-controlled).
pub(crate) fn url_host(endpoint: &str) -> String {
    endpoint
        .trim()
        .strip_prefix("https://")
        .or_else(|| endpoint.trim().strip_prefix("http://"))
        .unwrap_or(endpoint.trim())
        .trim_end_matches('/')
        .to_owned()
}

pub(crate) fn map_transport(error: SourceHttpError) -> StsExchangeError {
    match error {
        SourceHttpError::Timeout | SourceHttpError::Transport | SourceHttpError::Redirect => {
            StsExchangeError::Transport
        }
    }
}

/// Classify one STS HTTP response against the fixed wire.
pub(crate) fn classify_sts_response(
    response: crate::auth::aws::source_http::SourceHttpResponse,
) -> StsResult {
    let text = String::from_utf8_lossy(&response.body).into_owned();
    if (200..300).contains(&response.status) {
        return parse_sts_response(&text).ok_or(StsExchangeError::InvalidResponse);
    }
    let error = parse_error_response(&text);
    match error {
        Some((code, request_id)) => Err(StsExchangeError::Denied(StsDenial {
            status: response.status,
            code,
            request_id,
        })),
        None => Err(StsExchangeError::Denied(StsDenial {
            status: response.status,
            code: None,
            request_id: None,
        })),
    }
}

/// One owned XML element: local name and fully collected leaf text.
///
/// The owned `xmlparser` walk concatenates every text and CDATA segment
/// between comments (the smithy `try_data` helper stops at the first
/// segment, so it cannot provide complete credential values); unknown
/// elements are preserved as ignorable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OwnedElement {
    /// Local element name.
    pub local: String,
    /// Fully collected text (text + CDATA segments joined).
    pub text: String,
    /// Child elements in document order.
    pub children: Vec<OwnedElement>,
}

impl OwnedElement {
    /// First child with the given local name.
    pub fn child(&self, local: &str) -> Option<&OwnedElement> {
        self.children.iter().find(|child| child.local == local)
    }
}

/// Full lexical/structure precheck with the owned `xmlparser` walk.
///
/// Rejects tokenizer errors, unbalanced elements, and any DTD/entity
/// declaration markup (no external entity resources are ever fetched or
/// resolved); no size or depth budgets are invented. The pinned smithy
/// `Document` reader must additionally accept the document — it gates
/// structure, while the owned walk provides complete values.
pub(crate) fn xml_precheck(text: &str) -> Result<OwnedElement, StsExchangeError> {
    use xmlparser::{Token, Tokenizer};

    let invalid = StsExchangeError::InvalidResponse;
    let mut root: Option<OwnedElement> = None;
    let mut stack: Vec<OwnedElement> = Vec::new();
    for token in Tokenizer::from(text) {
        match token {
            Ok(Token::ElementStart { local, .. }) => {
                let element = OwnedElement {
                    local: local.as_str().to_owned(),
                    text: String::new(),
                    children: Vec::new(),
                };
                stack.push(element);
            }
            Ok(Token::ElementEnd { end, .. }) => match end {
                xmlparser::ElementEnd::Open => {}
                xmlparser::ElementEnd::Close(_, _) | xmlparser::ElementEnd::Empty => {
                    let Some(element) = stack.pop() else {
                        return Err(invalid);
                    };
                    match stack.last_mut() {
                        Some(parent) => parent.children.push(element),
                        None => {
                            if root.replace(element).is_some() {
                                return Err(invalid);
                            }
                        }
                    }
                }
            },
            Ok(Token::Text { text }) => {
                if let Some(current) = stack.last_mut() {
                    current.text.push_str(text.as_str());
                }
            }
            Ok(Token::Cdata { text, .. }) => {
                if let Some(current) = stack.last_mut() {
                    current.text.push_str(text.as_str());
                }
            }
            Ok(Token::DtdStart { .. })
            | Ok(Token::EmptyDtd { .. })
            | Ok(Token::EntityDeclaration { .. })
            | Ok(Token::DtdEnd { .. }) => return Err(invalid),
            Ok(_) => {}
            Err(_) => return Err(invalid),
        }
    }
    if !stack.is_empty() {
        return Err(invalid);
    }
    let root = root.ok_or_else(|| invalid.clone())?;
    // The pinned smithy decoder must also accept the document: it gates
    // structure, while the owned walk provides complete values.
    let mut doc =
        aws_smithy_xml::decode::Document::try_from(text.as_bytes()).map_err(|_| invalid.clone())?;
    doc.root_element().map_err(|_| invalid.clone())?;
    Ok(root)
}

/// Parse one STS `ErrorResponse` shape (code and request id only; messages
/// are deliberately dropped).
fn parse_error_response(text: &str) -> Option<(Option<String>, Option<String>)> {
    let root = xml_precheck(text).ok()?;
    if root.local != "ErrorResponse" {
        return None;
    }
    let code = root
        .child("Error")
        .and_then(|error| error.child("Code"))
        .map(|code| code.text.trim().to_owned());
    let request_id = root
        .child("RequestId")
        .map(|request_id| request_id.text.trim().to_owned());
    Some((code, request_id))
}

/// Parse one `AssumeRole`/`AssumeRoleWithWebIdentity` success response
/// against the fixed shape.
pub(crate) fn parse_sts_response(text: &str) -> Option<StsExchange> {
    let root = xml_precheck(text).ok()?;
    if root.local != "AssumeRoleResponse" && root.local != "AssumeRoleWithWebIdentityResponse" {
        return None;
    }
    let result = root
        .child("AssumeRoleResult")
        .or_else(|| root.child("AssumeRoleWithWebIdentityResult"))?;
    let credentials = result.child("Credentials")?;
    let access_key_id = credentials.child("AccessKeyId")?.text.trim().to_owned();
    let secret_access_key = credentials.child("SecretAccessKey")?.text.trim().to_owned();
    if access_key_id.is_empty() || secret_access_key.is_empty() {
        return None;
    }
    let assumed_role = result.child("AssumedRoleUser");
    Some(StsExchange {
        access_key_id: SecretString::from(access_key_id),
        secret_access_key: SecretString::from(secret_access_key),
        session_token: credentials
            .child("SessionToken")
            .map(|token| SecretString::from(token.text.trim().to_owned()))
            .filter(|token: &SecretString| !ExposeSecret::expose_secret(token).trim().is_empty()),
        expiration: credentials.child("Expiration").and_then(|exp| {
            OffsetDateTime::parse(
                exp.text.trim(),
                &time::format_description::well_known::Rfc3339,
            )
            .ok()
        }),
        assumed_role_user_arn: assumed_role
            .and_then(|user| user.child("Arn"))
            .map(|arn| arn.text.trim().to_owned()),
        assumed_role_user_id: assumed_role
            .and_then(|user| user.child("AssumedRoleId"))
            .map(|id| id.text.trim().to_owned()),
        subject: result
            .child("Subject")
            .map(|subject| subject.text.trim().to_owned()),
        audience: result
            .child("Audience")
            .map(|audience| audience.text.trim().to_owned()),
        provider: result
            .child("Provider")
            .map(|provider| provider.text.trim().to_owned()),
        request_id: root
            .child("ResponseMetadata")
            .and_then(|metadata| metadata.child("RequestId"))
            .map(|request_id| request_id.text.trim().to_owned()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const RESPONSE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<AssumeRoleResponse xmlns="https://sts.amazonaws.com/doc/2011-06-15/">
<AssumeRoleResult>
<Credentials>
<!-- comment between text segments -->
<AccessKeyId>ASIA<!-- inline comment -->ACCESS</AccessKeyId>
<SecretAccessKey>SECRET<![CDATA[-WITH-CDATA]]></SecretAccessKey>
<SessionToken>SESSION</SessionToken>
<Expiration>2027-06-01T12:00:00Z</Expiration>
<UnknownFutureField>ignored</UnknownFutureField>
</Credentials>
<AssumedRoleUser><Arn>arn:aws:sts::123456789012:assumed-role/x/y</Arn><AssumedRoleId>AROA</AssumedRoleId></AssumedRoleUser>
</AssumeRoleResult>
<ResponseMetadata><RequestId>req-1</RequestId></ResponseMetadata>
</AssumeRoleResponse>"#;

    #[test]
    fn response_shape_validates_credentials_expiry_and_provenance() {
        let exchange = parse_sts_response(RESPONSE).expect("shape validates");
        assert_eq!(
            exchange.access_key_id.expose_secret(),
            "ASIAACCESS",
            "text segments join across comments"
        );
        assert_eq!(
            exchange.secret_access_key.expose_secret(),
            "SECRET-WITH-CDATA",
            "CDATA segments join into text"
        );
        use secrecy::ExposeSecret as _;
        assert_eq!(
            exchange
                .session_token
                .as_ref()
                .map(|token| ExposeSecret::expose_secret(token).to_owned()),
            Some("SESSION".to_owned())
        );
        assert_eq!(
            exchange.expiration,
            Some(
                OffsetDateTime::parse(
                    "2027-06-01T12:00:00Z",
                    &time::format_description::well_known::Rfc3339
                )
                .unwrap()
            )
        );
        assert_eq!(
            exchange.assumed_role_user_arn.as_deref(),
            Some("arn:aws:sts::123456789012:assumed-role/x/y")
        );
        assert_eq!(exchange.request_id.as_deref(), Some("req-1"));
    }

    #[test]
    fn malformed_and_ambiguous_credentials_are_rejected() {
        for evil in [
            "<AssumeRoleResponse><Credentials>", // unbalanced (unclosed)
            "<AssumeRoleResponse><Credentials><AccessKeyId>A</AccessKeyId></Credentials>", // missing final close
            "<AssumeRoleResponse><AssumeRoleResult><Credentials><SecretAccessKey>only-secret</SecretAccessKey></Credentials></AssumeRoleResult></AssumeRoleResponse>", // missing access key
            "<!DOCTYPE AssumeRoleResponse [<!ENTITY xxe SYSTEM \"file:///etc/passwd\">]><AssumeRoleResponse/>", // DTD
            "<AssumeRoleResponse><Unclosed>", // lexical error
            "<Credentials xmlns=\"https://sts.amazonaws.com/doc/2011-06-15/\"><AccessKeyId>A</AccessKeyId><SecretAccessKey>S</SecretAccessKey></Credentials>", // wrong root
        ] {
            assert!(parse_sts_response(evil).is_none(), "must reject: {evil}");
        }
    }

    #[test]
    fn error_response_keeps_code_and_request_id_without_messages() {
        let error = r#"<ErrorResponse><Error><Type>Sender</Type><Code>AccessDenied</Code><Message>secret-ish detail</Message></Error><RequestId>req-9</RequestId></ErrorResponse>"#;
        let (code, request_id) = parse_error_response(error).expect("error shape");
        assert_eq!(code.as_deref(), Some("AccessDenied"));
        assert_eq!(request_id.as_deref(), Some("req-9"));
    }

    #[test]
    fn assume_role_body_encodes_the_fixed_form_shape() {
        let params = RoleAssumption {
            role_arn: "arn:aws:iam::123456789012:role/fixture".to_owned(),
            role_session_name: Some("session".to_owned()),
            external_id: Some("external".to_owned()),
            duration_seconds: Some("3600".to_owned()),
            mfa_serial: Some("arn:aws:iam::123456789012:mfa/fixture".to_owned()),
        };
        let body = assume_role_body(&params, Some("123456"));
        for expected in [
            "Action=AssumeRole",
            "Version=2011-06-15",
            "RoleArn=arn%3Aaws%3Aiam%3A%3A123456789012%3Arole%2Ffixture",
            "RoleSessionName=session",
            "ExternalId=external",
            "DurationSeconds=3600",
            "SerialNumber=arn%3Aaws%3Aiam%3A%3A123456789012%3Amfa%2Ffixture",
            "TokenCode=123456",
        ] {
            assert!(body.contains(expected), "form body: {body}");
        }
    }

    #[test]
    fn exchange_debug_never_carries_secrets() {
        let error = StsExchangeError::Denied(StsDenial {
            status: 403,
            code: Some("AccessDenied".to_owned()),
            request_id: Some("req-9".to_owned()),
        });
        let rendered = format!("{error}");
        assert!(rendered.contains("AccessDenied"));
        assert!(rendered.contains("req-9"));
        let exchange = StsExchange {
            access_key_id: SecretString::from("secret-access"),
            secret_access_key: SecretString::from("secret-key"),
            session_token: Some(SecretString::from("secret-session")),
            expiration: None,
            assumed_role_user_arn: None,
            assumed_role_user_id: None,
            subject: None,
            audience: None,
            provider: None,
            request_id: None,
        };
        let debug = format!("{exchange:?}");
        assert!(!debug.contains("secret-access"), "{debug}");
        assert!(!debug.contains("secret-key"), "{debug}");
        assert!(!debug.contains("secret-session"), "{debug}");
    }
}

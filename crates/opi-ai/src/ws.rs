//! Strict HTTP Upgrade bridge and private WebSocket client protocol state.
//!
//! The public bridge owns the request headers, nonce and mask entropy, and
//! strict validation of the 101 response (status, HTTP version, accept token,
//! singleton fields, offered subprotocols and permessage-deflate parameters).
//! It returns the detached byte stream and validated negotiation facts.
//!
//! Private frame, compression and async IO modules own RFC 6455 / RFC 7692
//! protocol state. Borrowed operations retain partial progress in one exclusive
//! connection owner; payload storage follows received bytes, compression tails
//! are injected once, and Close completion requires both frames, write shutdown
//! and peer EOF under the caller's deadline. Dropping the owner releases IO and
//! compression resources without automatic protocol output.
//!
//! The transport stays source-qualified: the shared [`HttpClient`] keeps its
//! explicit proxy profile, TLS profile and redirect policy. Upgrades require
//! [`RedirectPolicy::None`], so a 3xx is terminal. There is no hidden auth
//! discovery, automatic resend, reconnect or fallback.

// The private protocol state machines are consumed by the real Codex
// WebSocket transport (task 20.16) through the crate-internal `io` driver.
pub(crate) mod frame;
pub(crate) mod io;
pub(crate) mod pmd;

#[cfg(test)]
mod tests;

use crate::http::{HttpClient, RedirectPolicy};
use base64::Engine as _;
use thiserror::Error;
use url::Url;

/// Fixed WebSocket GUID from RFC 6455 §1.3.
const GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

/// Base64 alphabet used by RFC 6455 handshake fields.
fn b64<T: AsRef<[u8]>>(input: T) -> String {
    base64::engine::general_purpose::STANDARD.encode(input)
}

/// Errors of the strict Upgrade bridge. All are typed and terminal for the
/// attempted upgrade; none carry secrets.
#[derive(Debug, Error)]
pub enum WsUpgradeError {
    /// Transport-level failure from the source-qualified HTTP client.
    #[error("WebSocket upgrade transport failed")]
    Transport(#[from] reqwest::Error),
    /// The response was not `101 Switching Protocols`.
    #[error("WebSocket upgrade requires status 101, got {0}")]
    Status(u16),
    /// The response was not HTTP/1.1.
    #[error("WebSocket upgrade requires HTTP/1.1, got {0:?}")]
    Version(reqwest::Version),
    /// `Sec-WebSocket-Accept` was absent, duplicated or mismatched.
    #[error("WebSocket upgrade Sec-WebSocket-Accept invalid: {0}")]
    Accept(&'static str),
    /// The `Connection` header did not contain the `upgrade` token.
    #[error("WebSocket upgrade Connection tokens invalid: {0}")]
    ConnectionTokens(&'static str),
    /// The `Upgrade` header was absent, duplicated or not `websocket`.
    #[error("WebSocket upgrade Upgrade header invalid: {0}")]
    UpgradeHeader(&'static str),
    /// An unsolicited or duplicate `Sec-WebSocket-*` singleton field.
    #[error("WebSocket upgrade header invalid: {field}")]
    UnsolicitedField { field: &'static str },
    /// A subprotocol was selected that was not offered.
    #[error("WebSocket upgrade selected unoffered subprotocol")]
    UnofferedSubprotocol,
    /// Extension negotiation failed strict validation.
    #[error("WebSocket upgrade extension negotiation failed: {reason}")]
    Extension { reason: &'static str },
    /// The negotiated stream could not be detached from the client.
    #[error("WebSocket upgrade could not detach the upgraded stream")]
    Detach(String),
    /// A caller header collided with a handshake-managed header name.
    #[error("WebSocket upgrade reserved caller header: {0}")]
    ReservedHeader(String),
}

/// Fresh 16-byte nonce from the pinned platform entropy (aws-lc-rs).
pub fn fresh_nonce() -> [u8; 16] {
    let mut bytes = [0_u8; 16];
    aws_lc_rs::rand::fill(&mut bytes).expect("aws-lc-rs system entropy cannot fail for 16 bytes");
    bytes
}

/// Fresh 4-byte client mask from the pinned platform entropy.
pub fn fresh_mask() -> [u8; 4] {
    let mut bytes = [0_u8; 4];
    aws_lc_rs::rand::fill(&mut bytes).expect("aws-lc-rs system entropy cannot fail for 4 bytes");
    bytes
}

/// RFC 6455 §1.3 accept token: base64(SHA-1(key || GUID)). The digest is
/// mandated by the handshake protocol (protocol-required SHA-1, not a
/// security-collision context).
pub fn accept_token(key: &[u8; 16]) -> String {
    let mut input = b64(key).into_bytes();
    input.extend_from_slice(GUID.as_bytes());
    let digest = aws_lc_rs::digest::digest(&aws_lc_rs::digest::SHA1_FOR_LEGACY_USE_ONLY, &input);
    b64(digest.as_ref())
}

/// The permessage-deflate offer this bridge sends.
#[derive(Debug, Clone, Copy)]
pub struct PmdOffer {
    /// Offer `client_no_context_takeover`.
    pub client_no_context_takeover: bool,
    /// Offer `client_max_window_bits`; `None` offers the bare hint
    /// (no value), `Some(bits)` offers a concrete 8..=15 ceiling.
    pub client_max_window_bits: Option<Option<u8>>,
}

/// Strictly validated permessage-deflate negotiation result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NegotiatedPmd {
    /// The server will not reuse its compression context across messages.
    pub server_no_context_takeover: bool,
    /// The server acknowledged `client_no_context_takeover`.
    pub client_no_context_takeover: bool,
    /// The server's compression window (8..=15), retained as negotiated.
    pub server_max_window_bits: u8,
    /// The server-capped deflate window for this client (present only when
    /// offered), always within 8..=15.
    pub client_max_window_bits: Option<u8>,
}

/// What the upgraded connection negotiated.
#[derive(Debug, Clone)]
pub struct Negotiation {
    /// The selected subprotocol, always one the caller offered.
    pub subprotocol: Option<String>,
    /// Strictly negotiated permessage-deflate parameters; `None` means the
    /// extension was not negotiated (no compression).
    pub pmd: Option<NegotiatedPmd>,
}

impl Negotiation {
    /// The complete direction-specific PMD facts for the caller's neutral
    /// driver mapping; `None` means compression was not negotiated.
    /// Map both context flags and both windows by name; an omitted client
    /// window means the RFC default of 15.
    pub fn driver_parameters(&self) -> Option<NegotiatedPmd> {
        self.pmd
    }
}

/// An upgrade request under construction.
#[derive(Debug, Clone)]
pub struct WsUpgradeRequest {
    uri: Url,
    subprotocols: Vec<String>,
    pmd: Option<PmdOffer>,
    headers: Vec<(String, String)>,
}

impl WsUpgradeRequest {
    /// Prepare an upgrade against `uri` (scheme `http`/`https`; `ws`/`wss`
    /// map to their HTTP counterparts like browsers do).
    pub fn new(uri: Url) -> Self {
        Self {
            uri,
            subprotocols: Vec::new(),
            pmd: None,
            headers: Vec::new(),
        }
    }

    /// Add caller headers to the handshake request (for example provider
    /// authentication on the real Codex WebSocket transport). Handshake-
    /// managed names (`Connection`, `Upgrade`, `Sec-WebSocket-*`) are
    /// reserved and rejected with [`WsUpgradeError::ReservedHeader`].
    pub fn headers(mut self, headers: impl IntoIterator<Item = (String, String)>) -> Self {
        self.headers.extend(headers);
        self
    }

    /// Offer subprotocols; the response may select at most one of them.
    pub fn subprotocols(mut self, protocols: impl IntoIterator<Item = String>) -> Self {
        self.subprotocols.extend(protocols);
        self
    }

    /// Offer permessage-deflate with the given parameters.
    pub fn permessage_deflate(mut self, offer: PmdOffer) -> Self {
        self.pmd = Some(offer);
        self
    }

    fn http_url(&self) -> Url {
        let mut url = self.uri.clone();
        match url.scheme() {
            "ws" => url.set_scheme("http").ok(),
            "wss" => url.set_scheme("https").ok(),
            _ => None,
        };
        url
    }

    fn offer_header(&self) -> Option<String> {
        let offer = self.pmd.as_ref()?;
        let mut value = String::from("permessage-deflate");
        if offer.client_no_context_takeover {
            value.push_str("; client_no_context_takeover");
        }
        match offer.client_max_window_bits {
            Some(Some(bits)) => value.push_str(&format!("; client_max_window_bits={bits}")),
            Some(None) => value.push_str("; client_max_window_bits"),
            None => {}
        }
        Some(value)
    }
}

/// A successfully upgraded WebSocket transport: the detached byte stream and
/// the strictly validated negotiation facts.
pub struct UpgradedWs {
    pub stream: reqwest::Upgraded,
    pub negotiation: Negotiation,
}

fn managed_header(name: &str) -> bool {
    let lowered = name.to_ascii_lowercase();
    lowered == "connection" || lowered == "upgrade" || lowered.starts_with("sec-websocket-")
}

fn token_list_contains(
    values: reqwest::header::GetAll<'_, reqwest::header::HeaderValue>,
    token: &str,
) -> bool {
    values
        .iter()
        .flat_map(|value| value.to_str().ok().into_iter().flat_map(|t| t.split(',')))
        .any(|item| item.trim().eq_ignore_ascii_case(token))
}

/// Validate the permessage-deflate response entry against the offer.
fn negotiate_pmd(entries: &[String], offer: &PmdOffer) -> Result<NegotiatedPmd, WsUpgradeError> {
    use WsUpgradeError::Extension;
    if entries.len() > 1 {
        return Err(Extension {
            reason: "permessage-deflate negotiated more than once",
        });
    }
    let entry = entries[0].as_str();
    let mut parts = entry.split(';').map(str::trim);
    let name = parts.next().unwrap_or_default();
    if !name.eq_ignore_ascii_case("permessage-deflate") {
        return Err(Extension {
            reason: "only the offered permessage-deflate extension is accepted",
        });
    }
    let mut server_no_context_takeover = false;
    let mut client_no_context_takeover = false;
    let mut server_max_window_bits: Option<u8> = None;
    let mut client_max_window_bits: Option<u8> = None;
    for part in parts {
        let (key, value) = match part.split_once('=') {
            Some((k, v)) => (k.trim(), Some(v.trim())),
            None => (part, None),
        };
        let reject = |reason: &'static str| Err(Extension { reason });
        match key.to_ascii_lowercase().as_str() {
            "server_no_context_takeover" => {
                if value.is_some() || server_no_context_takeover {
                    return reject("server_no_context_takeover must appear once without a value");
                }
                server_no_context_takeover = true;
            }
            "client_no_context_takeover" => {
                if !offer.client_no_context_takeover {
                    return reject("client_no_context_takeover was not offered");
                }
                if value.is_some() || client_no_context_takeover {
                    return reject("client_no_context_takeover must appear once without a value");
                }
                client_no_context_takeover = true;
            }
            "server_max_window_bits" => {
                let bits: u8 = value
                    .and_then(|v| v.parse().ok())
                    .filter(|b| (8..=15).contains(b))
                    .ok_or(Extension {
                        reason: "server_max_window_bits requires a value in 8..=15",
                    })?;
                if server_max_window_bits.is_some() {
                    return reject("server_max_window_bits must appear once");
                }
                server_max_window_bits = Some(bits);
            }
            "client_max_window_bits" => {
                if offer.client_max_window_bits.is_none() {
                    return reject("client_max_window_bits was not offered");
                }
                let bits: u8 = value
                    .and_then(|v| v.parse().ok())
                    .filter(|b| (8..=15).contains(b))
                    .ok_or(Extension {
                        reason: "client_max_window_bits requires a value in 8..=15",
                    })?;
                if let Some(ceiling) = offer.client_max_window_bits.flatten()
                    && bits > ceiling
                {
                    return reject("client_max_window_bits exceeded the offered ceiling");
                }
                if client_max_window_bits.is_some() {
                    return reject("client_max_window_bits must appear once");
                }
                client_max_window_bits = Some(bits);
            }
            _ => return reject("unknown permessage-deflate parameter"),
        }
    }
    Ok(NegotiatedPmd {
        server_no_context_takeover,
        client_no_context_takeover,
        server_max_window_bits: server_max_window_bits.unwrap_or(15),
        client_max_window_bits,
    })
}

/// Perform the strict dynamic Upgrade through the source-qualified
/// [`HttpClient`] and detach the upgraded stream.
///
/// The client must have been built with [`RedirectPolicy::None`]: any 3xx
/// is reported as [`WsUpgradeError::Status`] and never followed. The
/// returned stream and [`Negotiation::driver_parameters`] remain caller-owned;
/// this function does not start the private protocol state machine.
pub async fn upgrade(
    client: &HttpClient,
    request: WsUpgradeRequest,
) -> Result<UpgradedWs, WsUpgradeError> {
    if client.redirect_policy() != RedirectPolicy::None {
        return Err(WsUpgradeError::Detach(
            "upgrades require an HttpClient with RedirectPolicy::None".into(),
        ));
    }
    let nonce = fresh_nonce();
    let key = b64(nonce);
    let url = request.http_url();
    let mut builder = client
        .websocket_client()
        .get(url.as_str())
        .header("Connection", "Upgrade")
        .header("Upgrade", "websocket")
        .header("Sec-WebSocket-Version", "13")
        .header("Sec-WebSocket-Key", &key);
    if !request.subprotocols.is_empty() {
        builder = builder.header("Sec-WebSocket-Protocol", request.subprotocols.join(", "));
    }
    if let Some(offer) = request.offer_header() {
        builder = builder.header("Sec-WebSocket-Extensions", offer);
    }
    for (name, value) in &request.headers {
        if managed_header(name) {
            return Err(WsUpgradeError::ReservedHeader(name.clone()));
        }
        builder = builder.header(name.as_str(), value.as_str());
    }
    let response = builder.send().await?;
    let status = response.status();
    if status.as_u16() != 101 {
        return Err(WsUpgradeError::Status(status.as_u16()));
    }
    if response.version() != reqwest::Version::HTTP_11 {
        return Err(WsUpgradeError::Version(response.version()));
    }
    let headers = response.headers();
    let mut accept_values = headers.get_all("Sec-WebSocket-Accept").iter();
    let accept = match (accept_values.next(), accept_values.next()) {
        (Some(value), None) => value,
        (None, _) => return Err(WsUpgradeError::Accept("header missing")),
        (Some(_), Some(_)) => return Err(WsUpgradeError::Accept("header duplicated")),
    };
    if accept.as_bytes() != accept_token(&nonce).as_bytes() {
        return Err(WsUpgradeError::Accept("value does not match the nonce"));
    }
    if !token_list_contains(headers.get_all("Connection"), "upgrade") {
        return Err(WsUpgradeError::ConnectionTokens("upgrade token missing"));
    }
    let upgrade_values: Vec<_> = headers.get_all("Upgrade").iter().collect();
    match upgrade_values.as_slice() {
        [single] if single.as_bytes().eq_ignore_ascii_case(b"websocket") => {}
        [] => return Err(WsUpgradeError::UpgradeHeader("header missing")),
        _ => {
            return Err(WsUpgradeError::UpgradeHeader(
                "must be a single websocket value",
            ));
        }
    }
    if headers
        .get_all("Sec-WebSocket-Version")
        .iter()
        .next()
        .is_some()
    {
        return Err(WsUpgradeError::UnsolicitedField {
            field: "Sec-WebSocket-Version",
        });
    }
    let offered_protocols: Vec<_> = headers.get_all("Sec-WebSocket-Protocol").iter().collect();
    let subprotocol = match offered_protocols[..] {
        [] => None,
        [single] => {
            let name = single
                .to_str()
                .map_err(|_| WsUpgradeError::UnsolicitedField {
                    field: "Sec-WebSocket-Protocol",
                })?;
            if !request.subprotocols.iter().any(|offered| offered == name) {
                return Err(WsUpgradeError::UnofferedSubprotocol);
            }
            Some(name.to_owned())
        }
        _ => {
            return Err(WsUpgradeError::UnsolicitedField {
                field: "Sec-WebSocket-Protocol",
            });
        }
    };
    let mut entries: Vec<String> = Vec::new();
    for value in headers.get_all("Sec-WebSocket-Extensions") {
        let text = value.to_str().map_err(|_| WsUpgradeError::Extension {
            reason: "extension header is not ASCII",
        })?;
        entries.extend(
            text.split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from),
        );
    }
    let pmd = match (request.pmd, entries.is_empty()) {
        (None, true) => None,
        (None, false) => {
            return Err(WsUpgradeError::Extension {
                reason: "extensions were negotiated without an offer",
            });
        }
        (Some(_), true) => None,
        (Some(offer), false) => Some(negotiate_pmd(&entries, &offer)?),
    };
    let stream = response
        .upgrade()
        .await
        .map_err(|error| WsUpgradeError::Detach(error.to_string()))?;
    Ok(UpgradedWs {
        stream,
        negotiation: Negotiation { subprotocol, pmd },
    })
}

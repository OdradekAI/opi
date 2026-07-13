//! Shared test-support helpers for `opi-coding-agent` integration tests.
//!
//! Each `tests/*.rs` binary pulls this in via `mod common;` using Cargo's
//! standard `tests/common/mod.rs` pattern: files in subdirectories of `tests/`
//! are NOT compiled as separate test binaries, only included as modules. This
//! module is compiled once per binary that includes it and never participates
//! in the published crate surface — `opi-coding-agent` exposes its library
//! through `src/`, not `tests/`, so `tests/common` cannot leak into the
//! crates.io API.
//!
//! Bodies here are kept byte-identical to the per-binary copies they replace.

// Each test binary compiles this module independently, so a helper used by some
// binaries and not others (e.g. `create_gitignore` is unused by `ls_tool`) is
// expected to be dead code in those binaries. Suppress per-binary dead_code
// rather than forcing every binary to touch every helper.
#![allow(dead_code)]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use opi_agent::tool::ToolResult;
use opi_ai::auth::LoginPresenter;
use opi_ai::credential::BoxAuthFuture;
use opi_ai::provider::ProviderError;

/// Concatenate every `OutputContent::Text` fragment of a tool result into one
/// string. Byte-identical to the per-binary copies it replaces.
pub fn tool_result_text(result: &ToolResult) -> String {
    result
        .content
        .iter()
        .filter_map(|c| match c {
            opi_ai::message::OutputContent::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}

/// Write a `.gitignore` file containing `content` into `dir`.
pub fn create_gitignore(dir: &std::path::Path, content: &str) {
    std::fs::write(dir.join(".gitignore"), content).unwrap();
}

// ---------------------------------------------------------------------------
// Phase 14.2 OAuth flow test seam
// ---------------------------------------------------------------------------

/// Mock `LoginPresenter` for OAuth flow tests. Captures every presenter call;
/// `await_manual_code` is controllable via a oneshot so a test can drive the
/// manual-paste path or leave it pending (so the loopback callback or timeout
/// wins the `select!` race). Never logs secrets — captured values are held in
/// memory for assertions only.
pub struct MockLoginPresenter {
    pub captured_urls: Arc<Mutex<Vec<String>>>,
    pub captured_device_codes: Arc<Mutex<Vec<(String, String)>>>,
    manual_code_rx: Arc<Mutex<Option<tokio::sync::oneshot::Receiver<String>>>>,
    pub notify_success_count: Arc<AtomicUsize>,
    pub notify_failure_reasons: Arc<Mutex<Vec<String>>>,
    /// Number of times `await_manual_code` was polled (so a test can prove a
    /// flow that must NOT use manual paste — e.g. Copilot device-code — never
    /// calls it).
    pub manual_code_calls: Arc<AtomicUsize>,
    auth_url_notify: Arc<tokio::sync::Notify>,
}

impl MockLoginPresenter {
    pub fn new() -> Self {
        Self {
            captured_urls: Arc::new(Mutex::new(Vec::new())),
            captured_device_codes: Arc::new(Mutex::new(Vec::new())),
            manual_code_rx: Arc::new(Mutex::new(None)),
            notify_success_count: Arc::new(AtomicUsize::new(0)),
            notify_failure_reasons: Arc::new(Mutex::new(Vec::new())),
            manual_code_calls: Arc::new(AtomicUsize::new(0)),
            auth_url_notify: Arc::new(tokio::sync::Notify::new()),
        }
    }

    /// Install a manual code that `await_manual_code` resolves with. If never
    /// called, `await_manual_code` stays pending (callback/timeout win).
    pub fn supply_manual_code(&self, code: impl Into<String>) {
        let (tx, rx) = tokio::sync::oneshot::channel();
        *self.manual_code_rx.lock().unwrap() = Some(rx);
        let _ = tx.send(code.into());
    }

    /// Wait until `present_auth_url` has been called at least once.
    pub async fn wait_for_auth_url(&self) {
        self.auth_url_notify.notified().await;
    }

    /// First captured authorize URL, if any.
    pub fn captured_url(&self) -> Option<String> {
        self.captured_urls.lock().unwrap().first().cloned()
    }
}

impl Default for MockLoginPresenter {
    fn default() -> Self {
        Self::new()
    }
}

impl LoginPresenter for MockLoginPresenter {
    fn present_auth_url<'a>(
        &'a self,
        url: &'a str,
    ) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
        let urls = self.captured_urls.clone();
        let notify = self.auth_url_notify.clone();
        let url = url.to_owned();
        Box::pin(async move {
            urls.lock().unwrap().push(url);
            notify.notify_one();
            Ok(())
        })
    }

    fn present_device_code<'a>(
        &'a self,
        user_code: &'a str,
        verification_uri: &'a str,
    ) -> BoxAuthFuture<'a, Result<(), ProviderError>> {
        let codes = self.captured_device_codes.clone();
        let user_code = user_code.to_owned();
        let uri = verification_uri.to_owned();
        Box::pin(async move {
            codes.lock().unwrap().push((user_code, uri));
            Ok(())
        })
    }

    fn await_manual_code<'a>(&'a self) -> BoxAuthFuture<'a, Result<String, ProviderError>> {
        let slot = self.manual_code_rx.clone();
        let calls = self.manual_code_calls.clone();
        Box::pin(async move {
            calls.fetch_add(1, Ordering::SeqCst);
            // Take the receiver out of the guard and drop the guard before
            // awaiting, so the future is `Send` (no MutexGuard held across await).
            let rx = slot.lock().unwrap().take();
            match rx {
                Some(rx) => rx
                    .await
                    .map_err(|_| ProviderError::Config("manual code sender dropped".into())),
                None => std::future::pending::<Result<String, ProviderError>>().await,
            }
        })
    }

    fn notify_success(&self) {
        self.notify_success_count.fetch_add(1, Ordering::SeqCst);
    }

    fn notify_failure(&self, reason: &str) {
        self.notify_failure_reasons
            .lock()
            .unwrap()
            .push(reason.to_owned());
    }
}

/// Extract a percent-decoded query parameter from a URL by key. Returns the
/// first match, or `None` if absent. Used to read the `state` token back out of
/// a captured authorize URL so a callback test can echo the correct state (or a
/// deliberately wrong one for the mismatch case).
pub fn extract_query_param(auth_url: &str, key: &str) -> Option<String> {
    let query = auth_url.split_once('?').map(|(_, q)| q).unwrap_or("");
    let prefix = format!("{key}=");
    for pair in query.split('&') {
        if let Some(value) = pair.strip_prefix(&prefix) {
            return Some(percent_decode(value));
        }
    }
    None
}

/// Extract the loopback TCP port from the `redirect_uri` query parameter of an
/// authorize URL, returning `None` if the param is absent or the host is not
/// loopback (`127.0.0.1` / `::1`). Used by PKCE callback tests to discover the
/// ephemeral port the provider bound, then drive the callback via a plain GET.
pub fn extract_redirect_port(auth_url: &str) -> Option<u16> {
    let query = auth_url.split_once('?').map(|(_, q)| q).unwrap_or("");
    for pair in query.split('&') {
        if let Some(value) = pair.strip_prefix("redirect_uri=") {
            let decoded = percent_decode(value);
            return extract_port_from_url(&decoded);
        }
    }
    None
}

fn extract_port_from_url(url: &str) -> Option<u16> {
    let after_scheme = url.split_once("://").map(|(_, rest)| rest).unwrap_or(url);
    let host_port = after_scheme.split('/').next().unwrap_or("");
    let (host, port) = host_port.rsplit_once(':')?;
    let host = host.trim_start_matches('[').trim_end_matches(']');
    if host != "127.0.0.1" && host != "::1" {
        return None;
    }
    port.parse().ok()
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(h), Some(l)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2]))
        {
            out.push(h * 16 + l);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_default()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

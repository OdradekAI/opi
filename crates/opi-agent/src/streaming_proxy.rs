//! Streaming proxy for forwarding command/event JSONL streams.
//!
//! **Unstable 0.x API** — this module may change between minor versions without
//! notice. Consumers MUST pin an exact version and test against upgrades.
//!
//! # Overview
//!
//! The [`StreamingProxy`] reads JSONL commands from any reader, dispatches them
//! to a [`ProxyHandler`], and writes JSONL responses/events to any writer. It
//! bridges the SDK command model ([`SdkCommand`]/[`SdkResponse`]) with an
//! external transport without requiring a live provider or specific I/O backend.
//!
//! # Framing
//!
//! Strict JSONL: one JSON object per line, `\n` delimiter, flushed after each
//! write. Empty lines are silently skipped. Malformed JSON produces a
//! `proxy_error` response with the line number and parse error.
//!
//! # Backpressure
//!
//! Events emitted by the handler are buffered in a bounded synchronous channel
//! (configurable via [`ProxyConfig::event_channel_capacity`], default 256).
//! When the buffer is full, new events are dropped with a tracing warning.
//! This prevents a slow consumer from blocking the handler.
//!
//! # Cancellation
//!
//! The proxy accepts a [`tokio_util::sync::CancellationToken`]. Cancellation is
//! observed between blocking input reads. When observed, the proxy emits a
//! `proxy_cancelled` event, stops reading new commands, drains remaining
//! buffered events, and exits cleanly.
//!
//! # Client Disconnect
//!
//! If a write to the output fails (broken pipe, closed connection), the proxy
//! stops processing, logs the error, and returns. It does not panic.
//!
//! # Secret Redaction
//!
//! When enabled (default), [`SecretRedactor`] scans event JSON for common
//! secret patterns:
//! - `sk-ant-*` (Anthropic API keys)
//! - `sk-*` (OpenAI API keys)
//! - `gh[pousr]_*` and `github_pat_*` (GitHub tokens)
//! - `AIza*` (Google API keys)
//! - Credentialed URL userinfo (`scheme://user:password@host`)
//! - Credential-bearing URL query parameters (`api_key`, `token`, etc.)
//! - Bearer tokens / JWTs (`eyJ*`)
//! - JSON fields named `password`, `secret`, `token`, `api_key`, `apikey`,
//!   `private_key`, `access_token`, `refresh_token`, `authorization`,
//!   `proxy-authorization`
//!
//! Matching values are replaced with `[REDACTED]`. Custom patterns can be
//! added via [`SecretRedactor::new`].
//!
//! # Protocol Sequence
//!
//! ```text
//! → {"type":"proxy_ready","schema_version":3}     // first output
//! ← {"type":"session_info"}                       // command from client
//! → {"type":"response","command":"session_info","success":true,"data":{...}}
//! → {"type":"AgentStart"}                         // async event
//! → {"type":"AgentEnd","messages":[...]}           // async event
//! ← {"type":"quit"}                               // client ends session
//! → {"type":"response","command":"quit","success":true}
//!                                                 // proxy exits
//! ```

use std::io::{BufRead, Write};

use crate::sdk::{SDK_SCHEMA_VERSION, SdkCommand, SdkResponse};
use serde_json::json;
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// ProxyEvent
// ---------------------------------------------------------------------------

/// An event emitted through the proxy's event channel.
#[derive(Debug, Clone)]
pub enum ProxyEvent {
    /// An agent event to forward as JSONL.
    Agent(serde_json::Value),
}

// ---------------------------------------------------------------------------
// ProxyHandler trait
// ---------------------------------------------------------------------------

/// Handler for incoming proxy commands.
///
/// Implementations receive a parsed [`SdkCommand`] and return an [`SdkResponse`].
/// They can also emit [`ProxyEvent`]s through the provided callback for async
/// event forwarding (e.g., streaming agent events).
pub trait ProxyHandler: Send + Sync {
    /// Handle a single command. Use `event_sink` to emit async events.
    fn handle_command(&self, command: SdkCommand, event_sink: &dyn Fn(ProxyEvent)) -> SdkResponse;
}

// ---------------------------------------------------------------------------
// ProxyConfig
// ---------------------------------------------------------------------------

/// Configuration for the streaming proxy.
#[derive(Debug, Clone)]
pub struct ProxyConfig {
    /// Bounded channel capacity for event buffering.
    ///
    /// When full, backpressure is applied (new events are dropped with a
    /// tracing warning). Default: 256.
    pub event_channel_capacity: usize,

    /// Whether to apply secret redaction to outgoing events.
    /// Default: true.
    pub redact_secrets: bool,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            event_channel_capacity: 256,
            redact_secrets: true,
        }
    }
}

// ---------------------------------------------------------------------------
// StreamingProxy
// ---------------------------------------------------------------------------

/// A streaming proxy that reads JSONL commands, dispatches to a handler, and
/// writes JSONL responses/events.
///
/// The proxy is transport-agnostic: it works with any [`BufRead`] + [`Write`]
/// pair (stdin/stdout, TCP streams, Unix sockets, etc.).
pub struct StreamingProxy<H> {
    handler: H,
    config: ProxyConfig,
}

impl<H: ProxyHandler> StreamingProxy<H> {
    /// Create a new proxy with the given handler and configuration.
    pub fn new(handler: H, config: ProxyConfig) -> Self {
        Self { handler, config }
    }

    /// Run the proxy until input is exhausted, a quit command is received,
    /// or cancellation fires.
    ///
    /// Returns the writer on success, or an error if the proxy failed.
    pub fn run<R: BufRead, W: Write>(
        self,
        reader: R,
        writer: W,
        cancel: CancellationToken,
    ) -> Result<W, StreamingProxyError> {
        let redactor = if self.config.redact_secrets {
            Some(SecretRedactor::default())
        } else {
            None
        };

        // Event channel for event forwarding.
        let (event_tx, event_rx) =
            std::sync::mpsc::sync_channel::<ProxyEvent>(self.config.event_channel_capacity);

        let mut engine = ProxyEngine {
            reader,
            writer,
            handler: self.handler,
            redactor,
            event_rx,
            event_tx,
            cancel,
            line_number: 0,
        };

        // Emit proxy_ready header
        let ready = json!({
            "type": "proxy_ready",
            "schema_version": SDK_SCHEMA_VERSION,
        });
        engine.write_json(&ready)?;

        engine.run_loop()
    }
}

// ---------------------------------------------------------------------------
// ProxyEngine (internal)
// ---------------------------------------------------------------------------

struct ProxyEngine<R, W, H> {
    reader: R,
    writer: W,
    handler: H,
    redactor: Option<SecretRedactor>,
    event_rx: std::sync::mpsc::Receiver<ProxyEvent>,
    event_tx: std::sync::mpsc::SyncSender<ProxyEvent>,
    cancel: CancellationToken,
    line_number: usize,
}

impl<R: BufRead, W: Write, H: ProxyHandler> ProxyEngine<R, W, H> {
    fn run_loop(mut self) -> Result<W, StreamingProxyError> {
        let mut line = String::new();

        loop {
            // Check cancellation
            if self.cancel.is_cancelled() {
                let _ = self.write_json(&json!({"type": "proxy_cancelled"}));
                self.drain_events()?;
                return Ok(self.writer);
            }

            line.clear();
            match self.reader.read_line(&mut line) {
                Ok(0) => {
                    // EOF
                    self.drain_events()?;
                    return Ok(self.writer);
                }
                Ok(_n) => {}
                Err(e) => return Err(StreamingProxyError::Io(e.to_string())),
            };

            self.line_number += 1;
            let trimmed = line.trim();

            // Skip empty lines
            if trimmed.is_empty() {
                continue;
            }

            // Parse command
            let command: SdkCommand = match serde_json::from_str(trimmed) {
                Ok(cmd) => cmd,
                Err(e) => {
                    let error_resp = json!({
                        "type": "proxy_error",
                        "line_number": self.line_number,
                        "error": format!("parse error: {e}"),
                        "raw": trimmed,
                    });
                    self.write_json(&error_resp)?;
                    continue;
                }
            };

            // Dispatch to handler
            let tx = self.event_tx.clone();
            let sink = move |event: ProxyEvent| {
                // Apply backpressure: if channel full, drop the event
                if tx.try_send(event).is_err() {
                    tracing::warn!("proxy event channel full, dropping event");
                }
            };

            let response = self.handler.handle_command(command, &sink);
            self.write_json(
                &serde_json::to_value(&response)
                    .unwrap_or(json!({"type":"response","success":false})),
            )?;

            // Drain any events the handler emitted
            self.drain_events()?;

            // Check for quit
            if response.command == "quit" {
                return Ok(self.writer);
            }
        }
    }

    /// Drain all buffered events and write them as JSONL.
    fn drain_events(&mut self) -> Result<(), StreamingProxyError> {
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                ProxyEvent::Agent(mut value) => {
                    if let Some(ref redactor) = self.redactor {
                        value = redactor.redact(&value);
                    }
                    self.write_json(&value)?;
                }
            }
        }
        Ok(())
    }

    /// Write a JSON value as a single JSONL line.
    fn write_json(&mut self, value: &serde_json::Value) -> Result<(), StreamingProxyError> {
        let mut line = serde_json::to_string(value).unwrap_or_else(|_| {
            r#"{"type":"proxy_error","error":"serialization failed"}"#.to_owned()
        });
        line.push('\n');
        self.writer
            .write_all(line.as_bytes())
            .map_err(|e| StreamingProxyError::Io(e.to_string()))?;
        self.writer
            .flush()
            .map_err(|e| StreamingProxyError::Io(e.to_string()))?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// SecretRedactor
// ---------------------------------------------------------------------------

/// Redacts common secret patterns from JSON event payloads.
///
/// # Default patterns
///
/// The default instance matches:
/// - API keys in values: `sk-ant-*`, `sk-*`
/// - GitHub tokens in values: `gh[pousr]_*`, `github_pat_*`
/// - Google API keys in values: `AIza*`
/// - Credentialed URL userinfo in values: `scheme://user:password@host`
/// - Credential-bearing URL query parameters in values: API keys, tokens,
///   AWS access/secret keys, passwords, and authorization values. Bare `key=`
///   is conservatively treated as credential-bearing, so non-secret query keys
///   with that exact name are also redacted.
/// - JWT/Bearer tokens in values: `eyJ*`
/// - Sensitive JSON field names: `password`, `secret`, `token`, `api_key`,
///   `apikey`, `private_key`, `access_token`, `refresh_token`,
///   `authorization`, `proxy-authorization`
///
/// Custom patterns can be provided via [`SecretRedactor::new`].
#[derive(Debug, Clone)]
pub struct SecretRedactor {
    /// Regex-like patterns for value matching (applied to string values).
    value_patterns: Vec<String>,
    /// Compiled value patterns.
    value_regexes: Vec<regex::Regex>,
    /// Field names whose values should always be redacted.
    sensitive_fields: Vec<String>,
}

impl Default for SecretRedactor {
    fn default() -> Self {
        let value_patterns = vec![
            // Anthropic API keys
            r"sk-ant-[a-zA-Z0-9-]{20,}".to_owned(),
            // GitHub tokens (classic PATs, OAuth, app/server, refresh, and
            // fine-grained PATs), including credentialed package-source URLs.
            r"gh[pousr]_[A-Za-z0-9]{36,}".to_owned(),
            r"github_pat_[A-Za-z0-9_]{82,}".to_owned(),
            // Google API keys
            r"AIza[0-9A-Za-z_-]{35,}".to_owned(),
            // Credentialed URL userinfo: scheme://user:password@host. Catches
            // inline basic-auth credentials embedded in git/https URLs even
            // when the credential is not a recognized token prefix.
            r"[a-zA-Z][a-zA-Z0-9+.-]*://[^/\s@]+:[^/\s@]+@".to_owned(),
            // Credential-bearing URL query parameters. This is intentionally
            // value based (not field-name based) because provider and session
            // errors often arrive as plain strings. Bare `key=` deliberately
            // accepts false positives to align with the provider boundary and
            // fail closed when the key's semantics are unknown.
            r"(?i)[?&](?:api[_-]?key|key|token|access[_-]?token|refresh[_-]?token|session[_-]?token|access[_-]?key[_-]?id|secret[_-]?access[_-]?key|secret|password|authorization|proxy[_-]?authorization)=[^&#\s]+"
                .to_owned(),
            // OpenAI-style API keys
            r"sk-[a-zA-Z0-9-]{20,}".to_owned(),
            // JWT/Bearer tokens (eyJ header plus two token segments)
            r"eyJ[a-zA-Z0-9_-]{10,}\.[a-zA-Z0-9_-]+\.[a-zA-Z0-9_-]+".to_owned(),
        ];

        Self {
            value_regexes: compile_value_patterns(&value_patterns),
            value_patterns,
            sensitive_fields: vec![
                "password".to_owned(),
                "secret".to_owned(),
                "token".to_owned(),
                "api_key".to_owned(),
                "apikey".to_owned(),
                "private_key".to_owned(),
                "access_token".to_owned(),
                "refresh_token".to_owned(),
                // Authorization/Bearer headers are redacted by name so an opaque
                // bearer value is scrubbed regardless of token prefix (7.6).
                "authorization".to_owned(),
                "proxy-authorization".to_owned(),
            ],
        }
    }
}

fn compile_value_patterns(patterns: &[String]) -> Vec<regex::Regex> {
    patterns
        .iter()
        .filter_map(|pattern| match regex::Regex::new(pattern) {
            Ok(regex) => Some(regex),
            Err(err) => {
                tracing::warn!(pattern, error = %err, "invalid secret redaction pattern ignored");
                None
            }
        })
        .collect()
}

impl SecretRedactor {
    /// Create a redactor with custom value patterns (regex strings).
    ///
    /// The default sensitive field names are still included.
    pub fn new(value_patterns: Vec<String>) -> Self {
        Self {
            value_regexes: compile_value_patterns(&value_patterns),
            value_patterns,
            sensitive_fields: Self::default().sensitive_fields,
        }
    }

    /// Return the active value patterns.
    pub fn patterns(&self) -> &[String] {
        &self.value_patterns
    }

    /// Redact secrets from a JSON value, returning a cleaned copy.
    pub fn redact(&self, value: &serde_json::Value) -> serde_json::Value {
        self.redact_value(value)
    }

    fn redact_value(&self, value: &serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Object(map) => {
                let mut new_map = serde_json::Map::new();
                for (k, v) in map {
                    if self.is_sensitive_field(k) {
                        new_map.insert(
                            k.clone(),
                            serde_json::Value::String("[REDACTED]".to_owned()),
                        );
                    } else {
                        new_map.insert(k.clone(), self.redact_value(v));
                    }
                }
                serde_json::Value::Object(new_map)
            }
            serde_json::Value::Array(arr) => {
                serde_json::Value::Array(arr.iter().map(|v| self.redact_value(v)).collect())
            }
            serde_json::Value::String(s) => {
                if self.matches_value_pattern(s) {
                    serde_json::Value::String("[REDACTED]".to_owned())
                } else {
                    serde_json::Value::String(s.clone())
                }
            }
            other => other.clone(),
        }
    }

    fn is_sensitive_field(&self, name: &str) -> bool {
        let lower = name.to_lowercase();
        self.sensitive_fields.iter().any(|f| f == &lower)
    }

    fn matches_value_pattern(&self, value: &str) -> bool {
        self.value_regexes.iter().any(|regex| regex.is_match(value))
    }
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors from the streaming proxy.
#[derive(Debug)]
pub enum StreamingProxyError {
    /// I/O error (read or write failure).
    Io(String),
}

impl std::fmt::Display for StreamingProxyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(msg) => write!(f, "proxy I/O error: {msg}"),
        }
    }
}

impl std::error::Error for StreamingProxyError {}

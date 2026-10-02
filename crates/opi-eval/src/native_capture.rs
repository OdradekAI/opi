//! Binary-local bounded native capture filter (P19-AUD-003, decision D1).
//!
//! Private thinking never reaches a first capture sink: the hidden
//! `__native-capture` mode of this executable spawns the selected Agent
//! through bounded pipes and persists only validated, privacy-projected
//! bytes. The module is deliberately binary-local: it exposes no SDK
//! contract, crate, dependency, trait or configuration flag beyond the
//! receipts it writes.
//!
//! Admission is strict and fail-closed. One record (one NDJSON line)
//! admits only against the exact grammar proved by the current sources
//! and importers: Opi `AgentSessionEvent`/`AgentEvent` envelopes with
//! their exact envelope/event/message/block field sets, and pi session
//! v2/v3 streams with the importer's event vocabulary. Unknown envelope,
//! event or content-block types, unknown fields on the message-bearing
//! events, duplicate JSON keys, invalid UTF-8/JSON, incomplete records
//! or streams exceeding the existing 64 MiB ceiling refuse before that
//! record is written; the already-written sanitized prefix stays. Safe
//! records that need no transformation retain their exact original
//! bytes, whitespace and newline; only transformed records are
//! re-serialized.
//!
//! The privacy projection removes thinking text, declared private
//! signatures and `redacted_thinking` data wherever assistant messages
//! repeat (message, turn, AgentEnd snapshots and stream partials),
//! emptying private thinking delta/end content while preserving block
//! placeholders and `content_index`/`contentIndex`. Model, API,
//! provider, ids, usage, cost, stop/retry/settlement and non-private
//! tool identity stay truthful. Tool-owned JSON (arguments, results,
//! details) is preserved verbatim: a tool property named `thinking` is
//! agent data, not private assistant thinking, and is never touched.

// Several items below (the stream ceiling, the policy spellings, the
// stream kinds and the refusal type) serve the binary-only capture
// runtime (`native_capture_runtime.rs`); this library target reaches
// them only through the unit contract at the bottom of this file, so
// non-test library builds see them as dead code by design.
#![allow(dead_code)]

use std::borrow::Cow;

/// Receipt schema written by the hidden capture mode.
pub(crate) const CAPTURE_RECEIPT_SCHEMA: &str = "opi-eval-native-capture/1";
/// Helper identity protocol printed by the task-side self-check.
pub(crate) const CAPTURE_HELPER_PROTOCOL: &str = "opi-eval-native-capture-helper/1";
/// The existing per-stream capture ceiling; no smaller cap is invented.
pub(crate) const STREAM_LIMIT_BYTES: usize = 64 * 1024 * 1024;

/// The capture policy sealed by the experiment's storage block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CapturePolicy {
    /// Persist every sanitized record.
    Full,
    /// Same first-sink boundary; terminal facts derive from safe bytes.
    TerminalOnly,
}

impl CapturePolicy {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::TerminalOnly => "terminal-only",
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value {
            "full" => Some(Self::Full),
            "terminal-only" => Some(Self::TerminalOnly),
            _ => None,
        }
    }
}

/// The admitted stream grammar of one captured Agent stdout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StreamKind {
    /// Opi `AgentSessionEvent` NDJSON.
    OpiAgent,
    /// pi session v2/v3 NDJSON.
    PiSession,
}

/// One fail-closed capture refusal; the offending record is never written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CaptureRefusal {
    pub(crate) reason: String,
}

impl CaptureRefusal {
    pub(crate) fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

/// Per-stream statistics recorded in the capture receipt.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct StreamSummary {
    pub(crate) records: u64,
    pub(crate) transformed_records: u64,
    pub(crate) output_bytes: u64,
    pub(crate) truncated: bool,
}

/// Strict JSON object parse that refuses duplicate keys at any depth.
///
/// `serde_json` alone silently keeps the last duplicate; capture
/// admission must not accept a record whose meaning changed under a
/// repeated key, so maps are built through a visitor that fails on the
/// second insertion of the same key.
fn parse_object_strict(
    line: &str,
) -> Result<serde_json::Map<String, serde_json::Value>, CaptureRefusal> {
    use serde::de::DeserializeSeed;
    /// A `serde_json::Value` whose maps refuse duplicate keys at any depth.
    struct StrictValue;
    impl<'de> serde::de::Visitor<'de> for StrictValue {
        type Value = serde_json::Value;
        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("any JSON value")
        }
        fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
            Ok(serde_json::Value::Bool(value))
        }
        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
            Ok(value.into())
        }
        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
            Ok(value.into())
        }
        fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E> {
            Ok(value.into())
        }
        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
            Ok(serde_json::Value::String(value.to_owned()))
        }
        fn visit_none<E>(self) -> Result<Self::Value, E> {
            Ok(serde_json::Value::Null)
        }
        fn visit_unit<E>(self) -> Result<Self::Value, E> {
            Ok(serde_json::Value::Null)
        }
        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_any(self)
        }
        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            let mut out = Vec::new();
            while let Some(value) = seq.next_element_seed(StrictValue)? {
                out.push(value);
            }
            Ok(serde_json::Value::Array(out))
        }
        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            let mut fields = serde_json::Map::new();
            while let Some(key) = map.next_key::<String>()? {
                if fields.contains_key(&key) {
                    return Err(serde::de::Error::custom(format!(
                        "duplicate JSON key {key:?}"
                    )));
                }
                let value = map.next_value_seed(StrictValue)?;
                fields.insert(key, value);
            }
            Ok(serde_json::Value::Object(fields))
        }
    }
    impl<'de> serde::de::DeserializeSeed<'de> for StrictValue {
        type Value = serde_json::Value;
        fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_any(StrictValue)
        }
    }
    struct StrictObject;
    impl<'de> serde::de::Visitor<'de> for StrictObject {
        type Value = serde_json::Map<String, serde_json::Value>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a JSON object")
        }
        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            let mut fields = serde_json::Map::new();
            while let Some(key) = map.next_key::<String>()? {
                if fields.contains_key(&key) {
                    return Err(serde::de::Error::custom(format!(
                        "duplicate JSON key {key:?}"
                    )));
                }
                let value = map.next_value_seed(StrictValue)?;
                fields.insert(key, value);
            }
            Ok(fields)
        }
    }
    struct Root(StrictObject);
    impl<'de> DeserializeSeed<'de> for Root {
        type Value = serde_json::Map<String, serde_json::Value>;
        fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_any(self.0)
        }
    }
    let mut deserializer = serde_json::Deserializer::from_str(line);
    Root(StrictObject)
        .deserialize(&mut deserializer)
        .map_err(|error| CaptureRefusal::new(format!("invalid JSON record: {error}")))
}

/// Fields helper: exact key-set matching plus typed accessors that
/// convert serde errors into capture refusals.
struct Fields<'a> {
    fields: &'a serde_json::Map<String, serde_json::Value>,
}

impl<'a> Fields<'a> {
    fn new(fields: &'a serde_json::Map<String, serde_json::Value>) -> Self {
        Self { fields }
    }

    pub(crate) fn type_of(&self) -> Result<&str, CaptureRefusal> {
        self.fields
            .get("type")
            .and_then(|value| value.as_str())
            .ok_or_else(|| CaptureRefusal::new("record field \"type\" is not a string"))
    }

    fn exact(&self, required: &[&str], optional: &[&str]) -> Result<(), CaptureRefusal> {
        for key in required {
            if !self.fields.contains_key(*key) {
                return Err(CaptureRefusal::new(format!(
                    "missing required field {key:?}"
                )));
            }
        }
        for key in self.fields.keys() {
            if !required.contains(&key.as_str()) && !optional.contains(&key.as_str()) {
                return Err(CaptureRefusal::new(format!("unknown field {key:?}")));
            }
        }
        Ok(())
    }

    fn str_of(&self, key: &str) -> Result<&str, CaptureRefusal> {
        self.fields
            .get(key)
            .and_then(|value| value.as_str())
            .ok_or_else(|| CaptureRefusal::new(format!("field {key:?} is not a string")))
    }

    fn object_of(
        &self,
        key: &str,
    ) -> Result<&serde_json::Map<String, serde_json::Value>, CaptureRefusal> {
        self.fields
            .get(key)
            .and_then(|value| value.as_object())
            .ok_or_else(|| CaptureRefusal::new(format!("field {key:?} is not an object")))
    }

    fn array_of(&self, key: &str) -> Result<&Vec<serde_json::Value>, CaptureRefusal> {
        self.fields
            .get(key)
            .and_then(|value| value.as_array())
            .ok_or_else(|| CaptureRefusal::new(format!("field {key:?} is not an array")))
    }

    fn u64_of(&self, key: &str) -> Result<u64, CaptureRefusal> {
        self.fields
            .get(key)
            .and_then(|value| value.as_u64())
            .ok_or_else(|| CaptureRefusal::new(format!("field {key:?} is not a u64")))
    }

    fn bool_of(&self, key: &str) -> Result<bool, CaptureRefusal> {
        self.fields
            .get(key)
            .and_then(|value| value.as_bool())
            .ok_or_else(|| CaptureRefusal::new(format!("field {key:?} is not a bool")))
    }

    fn optional_str(&self, key: &str) -> Result<Option<&str>, CaptureRefusal> {
        match self.fields.get(key) {
            None | Some(serde_json::Value::Null) => Ok(None),
            Some(value) => value
                .as_str()
                .map(Some)
                .ok_or_else(|| CaptureRefusal::new(format!("field {key:?} is not a string"))),
        }
    }
}

/// Detects the stream grammar from the first record.
pub(crate) fn detect_stream_kind(header: &str) -> Result<StreamKind, CaptureRefusal> {
    let fields = parse_object_strict(header)?;
    let fields = Fields::new(&fields);
    match fields.type_of()? {
        "Agent" => {
            fields.exact(&["type", "event"], &[])?;
            Ok(StreamKind::OpiAgent)
        }
        "session" => {
            fields.exact(&["type", "version", "id", "timestamp", "cwd"], &[])?;
            match fields
                .fields
                .get("version")
                .and_then(|value| value.as_u64())
            {
                Some(2 | 3) => Ok(StreamKind::PiSession),
                other => Err(CaptureRefusal::new(format!(
                    "pi session version {other:?} is not admitted"
                ))),
            }
        }
        other => Err(CaptureRefusal::new(format!(
            "unknown stream header type {other:?}"
        ))),
    }
}

/// Validates and privacy-filters one NDJSON record. Unchanged records
/// keep their exact original bytes; transformed records are serialized.
pub(crate) fn filter_record<'a>(
    line: &'a str,
    kind: StreamKind,
) -> Result<Cow<'a, str>, CaptureRefusal> {
    let parsed = parse_object_strict(line)?;
    let fields = Fields::new(&parsed);
    match kind {
        StreamKind::OpiAgent => opi_record(line, fields),
        StreamKind::PiSession => pi_record(line, fields),
    }
}

/// Serializes a transformed record. Transformed records do not promise
/// byte preservation; safe records never take this path.
fn serialize(fields: &serde_json::Map<String, serde_json::Value>) -> String {
    let value = serde_json::Value::Object(fields.clone());
    serde_json::to_string(&value).expect("validated JSON serializes")
}

// ---------------------------------------------------------------------------
// Opi grammar
// ---------------------------------------------------------------------------

fn opi_record<'a>(original: &'a str, fields: Fields<'_>) -> Result<Cow<'a, str>, CaptureRefusal> {
    let record = fields.type_of()?;
    match record {
        "Agent" => {
            fields.exact(&["type", "event"], &[])?;
            let event = fields.object_of("event")?;
            let event_fields = Fields::new(event);
            let event_type = event_fields.type_of()?.to_owned();
            match opi_event(&event_type, event_fields)? {
                Some(filtered) => {
                    let mut out = fields.fields.clone();
                    out.insert("event".to_owned(), serde_json::Value::Object(filtered));
                    Ok(Cow::Owned(serialize(&out)))
                }
                None => Ok(Cow::Borrowed(original)),
            }
        }
        "QueueUpdate" => {
            fields.exact(&["type", "steering", "follow_up"], &[])?;
            for key in ["steering", "follow_up"] {
                for entry in fields.array_of(key)? {
                    if !entry.is_string() {
                        return Err(CaptureRefusal::new(format!(
                            "QueueUpdate {key:?} entry is not a string"
                        )));
                    }
                }
            }
            Ok(Cow::Borrowed(original))
        }
        "CompactionStart" => {
            fields.exact(&["type", "reason"], &[])?;
            opi_compaction_reason(fields.str_of("reason")?)?;
            Ok(Cow::Borrowed(original))
        }
        "CompactionEnd" => {
            fields.exact(
                &[
                    "type",
                    "reason",
                    "result",
                    "aborted",
                    "will_retry",
                    "error_message",
                ],
                &[],
            )?;
            opi_compaction_reason(fields.str_of("reason")?)?;
            fields.bool_of("aborted")?;
            fields.bool_of("will_retry")?;
            let result = fields.fields.get("result").expect("required");
            match result {
                serde_json::Value::Null => {}
                serde_json::Value::Object(fields) => {
                    Fields::new(fields).exact(
                        &[
                            "summary",
                            "first_kept_entry_id",
                            "tokens_before",
                            "tokens_after",
                        ],
                        &[],
                    )?;
                }
                _ => return Err(CaptureRefusal::new("CompactionEnd result is not an object")),
            }
            Ok(Cow::Borrowed(original))
        }
        "session_summary" => {
            fields.exact(
                &[
                    "type",
                    "session_id",
                    "model",
                    "turns",
                    "provider_turns",
                    "tokens",
                ],
                &["cost_usd", "diagnostics"],
            )?;
            let tokens = fields.object_of("tokens")?;
            Fields::new(tokens).exact(&["input", "output", "cache_read", "cache_write"], &[])?;
            for key in ["turns", "provider_turns"] {
                fields.u64_of(key)?;
            }
            Ok(Cow::Borrowed(original))
        }
        "SessionInfoChanged" => {
            fields.exact(&["type", "session_id", "name"], &[])?;
            fields.optional_str("name")?;
            Ok(Cow::Borrowed(original))
        }
        "ThinkingLevelChanged" => {
            fields.exact(&["type", "level"], &[])?;
            fields.str_of("level")?;
            Ok(Cow::Borrowed(original))
        }
        "StartupDiagnostics" => {
            fields.exact(&["type", "diagnostics"], &[])?;
            if fields
                .array_of("diagnostics")?
                .iter()
                .any(|v| !v.is_object())
            {
                return Err(CaptureRefusal::new(
                    "StartupDiagnostics entry is not an object",
                ));
            }
            Ok(Cow::Borrowed(original))
        }
        "CredentialNeeded" | "CredentialRevoked" => {
            fields.exact(&["type", "provider_id", "remediation", "diagnostic"], &[])?;
            fields.str_of("provider_id")?;
            fields.str_of("remediation")?;
            if !fields
                .fields
                .get("diagnostic")
                .is_some_and(|v| v.is_object())
            {
                return Err(CaptureRefusal::new("diagnostic payload is not an object"));
            }
            Ok(Cow::Borrowed(original))
        }
        other => Err(CaptureRefusal::new(format!(
            "unknown Opi envelope type {other:?}"
        ))),
    }
}

fn opi_compaction_reason(reason: &str) -> Result<(), CaptureRefusal> {
    match reason {
        "manual" | "threshold" | "overflow" => Ok(()),
        other => Err(CaptureRefusal::new(format!(
            "unknown compaction reason {other:?}"
        ))),
    }
}

/// Filters one Opi `AgentEvent`; `Some(fields)` when transformed.
fn opi_event(
    event_type: &str,
    event: Fields<'_>,
) -> Result<Option<serde_json::Map<String, serde_json::Value>>, CaptureRefusal> {
    let mut out = event.fields.clone();
    let mut changed = false;
    match event_type {
        "AgentStart" | "TurnStart" => {
            event.exact(&["type"], &[])?;
        }
        "MessageStart" | "MessageEnd" => {
            event.exact(&["type", "message"], &[])?;
            if let Some(message) = filter_opi_message(event.object_of("message")?)? {
                out.insert("message".to_owned(), serde_json::Value::Object(message));
                changed = true;
            }
        }
        "MessageUpdate" => {
            event.exact(&["type", "message", "assistant_event"], &[])?;
            if let Some(message) = filter_opi_message(event.object_of("message")?)? {
                out.insert("message".to_owned(), serde_json::Value::Object(message));
                changed = true;
            }
            if let Some(filtered) = filter_opi_assistant_event(event.object_of("assistant_event")?)?
            {
                out.insert(
                    "assistant_event".to_owned(),
                    serde_json::Value::Object(filtered),
                );
                changed = true;
            }
        }
        "TurnEnd" => {
            event.exact(&["type", "message", "tool_results"], &[])?;
            if let Some(message) = filter_opi_message(event.object_of("message")?)? {
                out.insert("message".to_owned(), serde_json::Value::Object(message));
                changed = true;
            }
            // Tool results are agent-owned provider-facing outputs; they
            // carry no assistant thinking and pass through untouched.
            for result in event.array_of("tool_results")? {
                if !result.is_object() {
                    return Err(CaptureRefusal::new("tool result is not an object"));
                }
            }
        }
        "AgentEnd" => {
            event.exact(&["type", "messages"], &[])?;
            let messages = event.array_of("messages")?;
            let mut any_changed = false;
            let mut filtered = Vec::with_capacity(messages.len());
            for message in messages {
                let Some(fields) = message.as_object() else {
                    return Err(CaptureRefusal::new("AgentEnd entry is not an object"));
                };
                match filter_opi_message(fields)? {
                    Some(new) => {
                        any_changed = true;
                        filtered.push(serde_json::Value::Object(new));
                    }
                    None => filtered.push(message.clone()),
                }
            }
            if any_changed {
                out.insert("messages".to_owned(), serde_json::Value::Array(filtered));
                changed = true;
            }
        }
        "ToolExecutionStart" => {
            event.exact(&["type", "tool_call_id", "tool_name", "args"], &[])?;
            event.str_of("tool_call_id")?;
            event.str_of("tool_name")?;
        }
        "ToolExecutionUpdate" => {
            event.exact(
                &[
                    "type",
                    "tool_call_id",
                    "tool_name",
                    "args",
                    "partial_result",
                ],
                &[],
            )?;
            event.str_of("tool_call_id")?;
            event.str_of("tool_name")?;
        }
        "ToolExecutionEnd" => {
            event.exact(
                &[
                    "type",
                    "tool_call_id",
                    "tool_name",
                    "result",
                    "details",
                    "is_error",
                    "truncated",
                ],
                &["diagnostics"],
            )?;
            event.str_of("tool_call_id")?;
            event.str_of("tool_name")?;
            event.bool_of("is_error")?;
            event.bool_of("truncated")?;
        }
        "AutoRetryStart" => {
            event.exact(
                &[
                    "type",
                    "attempt",
                    "max_attempts",
                    "delay_ms",
                    "error_message",
                ],
                &[],
            )?;
            event.u64_of("attempt")?;
            event.u64_of("max_attempts")?;
            event.u64_of("delay_ms")?;
            event.str_of("error_message")?;
        }
        "AutoRetryEnd" => {
            event.exact(&["type", "success", "attempt", "final_error"], &[])?;
            event.bool_of("success")?;
            event.u64_of("attempt")?;
            event.optional_str("final_error")?;
        }
        "CompactionStart" => {
            event.exact(&["type", "reason"], &[])?;
            opi_compaction_reason(event.str_of("reason")?)?;
        }
        "CompactionEnd" => {
            event.exact(
                &[
                    "type",
                    "reason",
                    "result",
                    "aborted",
                    "will_retry",
                    "error_message",
                ],
                &[],
            )?;
            opi_compaction_reason(event.str_of("reason")?)?;
            event.bool_of("aborted")?;
            event.bool_of("will_retry")?;
        }
        "SessionPersistError" => {
            event.exact(&["type", "message"], &[])?;
            event.str_of("message")?;
        }
        other => {
            return Err(CaptureRefusal::new(format!(
                "unknown Opi event type {other:?}"
            )));
        }
    }
    Ok(if changed { Some(out) } else { None })
}

/// Filters one Opi agent message; `Some(fields)` when transformed.
fn filter_opi_message(
    message: &serde_json::Map<String, serde_json::Value>,
) -> Result<Option<serde_json::Map<String, serde_json::Value>>, CaptureRefusal> {
    let fields = Fields::new(message);
    match fields.type_of()? {
        "Llm" => {
            let role = fields.str_of("role")?;
            match role {
                "assistant" => {
                    fields.exact(
                        &[
                            "role",
                            "type",
                            "content",
                            "api",
                            "provider",
                            "model",
                            "response_model",
                            "response_id",
                            "usage",
                            "stop_reason",
                            "error_message",
                            "timestamp_ms",
                        ],
                        &[],
                    )?;
                    fields.str_of("api")?;
                    fields.str_of("provider")?;
                    fields.str_of("model")?;
                    fields.optional_str("response_model")?;
                    fields.optional_str("response_id")?;
                    fields.optional_str("error_message")?;
                    fields.str_of("stop_reason")?;
                    fields.u64_of("timestamp_ms")?;
                    if !message.get("usage").is_some_and(|v| v.is_object()) {
                        return Err(CaptureRefusal::new("assistant usage is not an object"));
                    }
                    match filter_assistant_blocks(message.get("content").expect("required"))? {
                        Some(content) => {
                            let mut out = message.clone();
                            out.insert("content".to_owned(), content);
                            Ok(Some(out))
                        }
                        None => Ok(None),
                    }
                }
                "user" => {
                    fields.exact(&["role", "type", "content", "timestamp_ms"], &[])?;
                    fields.u64_of("timestamp_ms")?;
                    validate_io_blocks(message.get("content").expect("required"), "user")?;
                    Ok(None)
                }
                "tool_result" => {
                    fields.exact(
                        &[
                            "role",
                            "type",
                            "tool_call_id",
                            "tool_name",
                            "content",
                            "details",
                            "is_error",
                            "timestamp_ms",
                        ],
                        &["truncated"],
                    )?;
                    fields.str_of("tool_call_id")?;
                    fields.str_of("tool_name")?;
                    fields.bool_of("is_error")?;
                    fields.u64_of("timestamp_ms")?;
                    validate_io_blocks(message.get("content").expect("required"), "tool")?;
                    Ok(None)
                }
                other => Err(CaptureRefusal::new(format!(
                    "unknown Llm message role {other:?}"
                ))),
            }
        }
        "CompactionSummary" => {
            fields.exact(
                &[
                    "type",
                    "summary",
                    "first_kept_entry_id",
                    "tokens_before",
                    "tokens_after",
                ],
                &[],
            )?;
            Ok(None)
        }
        "BranchSummary" => {
            fields.exact(
                &["type", "parent_session_id", "summary", "entry_count"],
                &[],
            )?;
            Ok(None)
        }
        "Custom" => {
            fields.exact(&["type", "kind", "data", "include_in_llm_context"], &[])?;
            Ok(None)
        }
        other => Err(CaptureRefusal::new(format!(
            "unknown Opi message type {other:?}"
        ))),
    }
}

/// Non-assistant (user/tool) content blocks: text and image only.
fn validate_io_blocks(content: &serde_json::Value, owner: &str) -> Result<(), CaptureRefusal> {
    let blocks = content
        .as_array()
        .ok_or_else(|| CaptureRefusal::new(format!("{owner} content is not an array")))?;
    for block in blocks {
        let block = Fields::new(
            block
                .as_object()
                .ok_or_else(|| CaptureRefusal::new(format!("{owner} block is not an object")))?,
        );
        match block.type_of()? {
            "text" => block.exact(&["type", "text"], &[])?,
            "image" => block.exact(&["type", "source", "media_type"], &[])?,
            other => {
                return Err(CaptureRefusal::new(format!(
                    "unknown {owner} content block {other:?}"
                )));
            }
        }
    }
    Ok(())
}

/// Filters assistant content blocks; `Some(content)` when transformed.
fn filter_assistant_blocks(
    content: &serde_json::Value,
) -> Result<Option<serde_json::Value>, CaptureRefusal> {
    let blocks = content
        .as_array()
        .ok_or_else(|| CaptureRefusal::new("assistant content is not an array"))?;
    let mut changed = false;
    let mut out = Vec::with_capacity(blocks.len());
    for block in blocks {
        let fields = block
            .as_object()
            .ok_or_else(|| CaptureRefusal::new("assistant block is not an object"))?;
        let block_fields = Fields::new(fields);
        let mut block = block.clone();
        match block_fields.type_of()? {
            "text" => block_fields.exact(&["type", "text"], &[])?,
            "tool_call" => {
                block_fields.exact(&["type", "tool_call"], &[])?;
                let call = block_fields.object_of("tool_call")?;
                Fields::new(call).exact(&["id", "name", "arguments"], &[])?;
            }
            "thinking" => {
                block_fields.exact(&["type", "thinking"], &["signature", "thinkingSignature"])?;
                if block
                    .get("thinking")
                    .and_then(|v| v.as_str())
                    .is_some_and(|text| !text.is_empty())
                    || block.as_object().is_some_and(|object| {
                        object.contains_key("signature") || object.contains_key("thinkingSignature")
                    })
                {
                    if let Some(object) = block.as_object_mut() {
                        object.insert(
                            "thinking".to_owned(),
                            serde_json::Value::String(String::new()),
                        );
                        object.remove("signature");
                        object.remove("thinkingSignature");
                    }
                    changed = true;
                }
            }
            "redacted_thinking" => {
                block_fields.exact(&["type", "data"], &[])?;
                if block
                    .get("data")
                    .and_then(|v| v.as_str())
                    .is_some_and(|text| !text.is_empty())
                {
                    if let Some(object) = block.as_object_mut() {
                        object.insert("data".to_owned(), serde_json::Value::String(String::new()));
                    }
                    changed = true;
                }
            }
            other => {
                return Err(CaptureRefusal::new(format!(
                    "unknown assistant content block {other:?}"
                )));
            }
        }
        out.push(block);
    }
    Ok(if changed {
        Some(serde_json::Value::Array(out))
    } else {
        None
    })
}

/// The raw (untagged) assistant snapshot carried by Opi stream events:
/// the full message field set minus `role`/`type`.
fn filter_opi_snapshot(
    message: &serde_json::Map<String, serde_json::Value>,
) -> Result<Option<serde_json::Map<String, serde_json::Value>>, CaptureRefusal> {
    let fields = Fields::new(message);
    fields.exact(
        &[
            "api",
            "content",
            "error_message",
            "model",
            "provider",
            "response_id",
            "response_model",
            "stop_reason",
            "timestamp_ms",
            "usage",
        ],
        &[],
    )?;
    match filter_assistant_blocks(message.get("content").expect("required"))? {
        Some(content) => {
            let mut out = message.clone();
            out.insert("content".to_owned(), content);
            Ok(Some(out))
        }
        None => Ok(None),
    }
}

/// Filters one Opi stream `assistant_event`; `Some(fields)` when changed.
fn filter_opi_assistant_event(
    event: &serde_json::Map<String, serde_json::Value>,
) -> Result<Option<serde_json::Map<String, serde_json::Value>>, CaptureRefusal> {
    let fields = Fields::new(event);
    let kind = fields.type_of()?;
    let mut out = event.clone();
    let mut changed = false;
    let partial = |out: &mut serde_json::Map<String, serde_json::Value>,
                   changed: &mut bool|
     -> Result<(), CaptureRefusal> {
        if let Some(snapshot) = filter_opi_snapshot(
            out.get("partial")
                .and_then(|v| v.as_object())
                .ok_or_else(|| CaptureRefusal::new("partial is not an object"))?,
        )? {
            out.insert("partial".to_owned(), serde_json::Value::Object(snapshot));
            *changed = true;
        }
        Ok(())
    };
    match kind {
        "start" => {
            fields.exact(&["type", "partial"], &[])?;
            partial(&mut out, &mut changed)?;
        }
        "text_start" | "thinking_start" | "tool_call_start" => {
            fields.exact(&["type", "content_index", "partial"], &[])?;
            fields.u64_of("content_index")?;
            partial(&mut out, &mut changed)?;
        }
        "text_delta" | "tool_call_delta" => {
            fields.exact(&["type", "content_index", "delta", "partial"], &[])?;
            fields.u64_of("content_index")?;
            fields.str_of("delta")?;
            partial(&mut out, &mut changed)?;
        }
        "text_end" => {
            fields.exact(&["type", "content_index", "content", "partial"], &[])?;
            fields.u64_of("content_index")?;
            fields.str_of("content")?;
            partial(&mut out, &mut changed)?;
        }
        "thinking_delta" => {
            fields.exact(&["type", "content_index", "delta", "partial"], &[])?;
            fields.u64_of("content_index")?;
            partial(&mut out, &mut changed)?;
            if out
                .get("delta")
                .and_then(|v| v.as_str())
                .is_some_and(|text| !text.is_empty())
            {
                out.insert("delta".to_owned(), serde_json::Value::String(String::new()));
                changed = true;
            }
        }
        "thinking_end" => {
            fields.exact(&["type", "content_index", "content", "partial"], &[])?;
            fields.u64_of("content_index")?;
            partial(&mut out, &mut changed)?;
            if out
                .get("content")
                .and_then(|v| v.as_str())
                .is_some_and(|text| !text.is_empty())
            {
                out.insert(
                    "content".to_owned(),
                    serde_json::Value::String(String::new()),
                );
                changed = true;
            }
        }
        "tool_call_end" => {
            fields.exact(&["type", "content_index", "tool_call", "partial"], &[])?;
            fields.u64_of("content_index")?;
            let call = fields.object_of("tool_call")?;
            Fields::new(call).exact(&["id", "name", "arguments"], &[])?;
            partial(&mut out, &mut changed)?;
        }
        "done" | "error" => {
            fields.exact(&["type", "reason", "message"], &[])?;
            fields.str_of("reason")?;
            if let Some(snapshot) = filter_opi_snapshot(
                out.get("message")
                    .and_then(|v| v.as_object())
                    .ok_or_else(|| CaptureRefusal::new("message is not an object"))?,
            )? {
                out.insert("message".to_owned(), serde_json::Value::Object(snapshot));
                changed = true;
            }
        }
        other => {
            return Err(CaptureRefusal::new(format!(
                "unknown assistant stream event {other:?}"
            )));
        }
    }
    Ok(if changed { Some(out) } else { None })
}

// ---------------------------------------------------------------------------
// pi grammar
// ---------------------------------------------------------------------------

/// pi event types that carry assistant message payloads.
const PI_MESSAGE_EVENTS: &[(&str, &[&str], &[&str])] = &[
    ("message_start", &["type", "message"], &[]),
    ("message_end", &["type", "message"], &[]),
    ("turn_end", &["type", "message", "toolResults"], &[]),
    ("agent_end", &["type", "messages"], &["willRetry"]),
    (
        "message_update",
        &["type", "usage", "assistantMessageEvent"],
        &[],
    ),
];

/// pi event types admitted as opaque pass-through: the importer's exact
/// vocabulary minus the message-bearing events above. None of them may
/// carry a message-bearing key.
const PI_OPAQUE_EVENTS: &[&str] = &[
    "agent_start",
    "turn_start",
    "tool_execution_start",
    "tool_execution_update",
    "tool_execution_end",
    "entry_appended",
    "session_before_compact",
    "session_before_tree",
    "session_compact",
    "session_compact_failed",
    "session_shutdown",
    "session_start",
    "session_tree",
    "session_info_changed",
    "auto_retry_start",
    "auto_retry_end",
    "compaction_start",
    "compaction_end",
    "model_select",
    "thinking_level_changed",
    "thinking_level_select",
    "queue_update",
    "bash_execution_update",
    "tool_call",
    "tool_result",
    "text",
    "summarization_retry_attempt_start",
    "summarization_retry_finished",
    "summarization_retry_scheduled",
    "agent_settled",
];

fn pi_record<'a>(original: &'a str, fields: Fields<'_>) -> Result<Cow<'a, str>, CaptureRefusal> {
    let record = fields.type_of()?;
    // The session header passes through unchanged: stream detection
    // already validated its exact five-field shape.
    if record == "session" {
        return Ok(Cow::Borrowed(original));
    }
    if let Some((_, required, optional)) = PI_MESSAGE_EVENTS
        .iter()
        .find(|(kind, _, _)| *kind == record)
    {
        fields.exact(required, optional)?;
        let mut out = fields.fields.clone();
        let mut changed = false;
        match record {
            "message_update" => {
                if !fields.fields.get("usage").is_some_and(|v| v.is_object()) {
                    return Err(CaptureRefusal::new("message_update usage is not an object"));
                }
                if let Some(event) =
                    filter_pi_assistant_event(fields.object_of("assistantMessageEvent")?)?
                {
                    out.insert(
                        "assistantMessageEvent".to_owned(),
                        serde_json::Value::Object(event),
                    );
                    changed = true;
                }
            }
            "agent_end" => {
                let messages = fields.array_of("messages")?;
                let mut any_changed = false;
                let mut filtered = Vec::with_capacity(messages.len());
                for message in messages {
                    let Some(object) = message.as_object() else {
                        return Err(CaptureRefusal::new("agent_end entry is not an object"));
                    };
                    match filter_pi_message(object)? {
                        Some(new) => {
                            any_changed = true;
                            filtered.push(serde_json::Value::Object(new));
                        }
                        None => filtered.push(message.clone()),
                    }
                }
                if any_changed {
                    out.insert("messages".to_owned(), serde_json::Value::Array(filtered));
                    changed = true;
                }
                if let Some(will_retry) = fields.fields.get("willRetry")
                    && !will_retry.is_boolean()
                {
                    return Err(CaptureRefusal::new("willRetry is not a bool"));
                }
            }
            _ => {
                let message = fields.object_of("message")?;
                if let Some(new) = filter_pi_message(message)? {
                    out.insert("message".to_owned(), serde_json::Value::Object(new));
                    changed = true;
                }
                if record == "turn_end"
                    && fields
                        .array_of("toolResults")?
                        .iter()
                        .any(|result| !result.is_object())
                {
                    return Err(CaptureRefusal::new("toolResults entry is not an object"));
                }
            }
        }
        return Ok(if changed {
            Cow::Owned(serialize(&out))
        } else {
            Cow::Borrowed(original)
        });
    }
    if PI_OPAQUE_EVENTS.contains(&record) {
        for key in ["message", "messages", "assistantMessageEvent"] {
            if fields.fields.contains_key(key) {
                return Err(CaptureRefusal::new(format!(
                    "pi event {record:?} carries message-bearing field {key:?}"
                )));
            }
        }
        return Ok(Cow::Borrowed(original));
    }
    Err(CaptureRefusal::new(format!(
        "unknown pi event type {record:?}"
    )))
}

/// Filters one pi assistant message; `Some(fields)` when transformed.
fn filter_pi_message(
    message: &serde_json::Map<String, serde_json::Value>,
) -> Result<Option<serde_json::Map<String, serde_json::Value>>, CaptureRefusal> {
    let fields = Fields::new(message);
    if fields.str_of("role")? != "assistant" {
        // Non-assistant messages carry no private thinking; admit their
        // declared core shape only.
        fields.exact(&["role", "content"], &["id", "timestamp"])?;
        return Ok(None);
    }
    fields.exact(
        &[
            "role",
            "content",
            "api",
            "timestamp",
            "provider",
            "model",
            "stopReason",
            "usage",
        ],
        &["id", "responseId", "errorMessage"],
    )?;
    fields.str_of("api")?;
    fields.str_of("provider")?;
    fields.str_of("model")?;
    fields.str_of("stopReason")?;
    if !message.get("usage").is_some_and(|v| v.is_object()) {
        return Err(CaptureRefusal::new("pi usage is not an object"));
    }
    let blocks = message
        .get("content")
        .and_then(|v| v.as_array())
        .ok_or_else(|| CaptureRefusal::new("assistant content is not an array"))?;
    let mut changed = false;
    let mut out = Vec::with_capacity(blocks.len());
    for block in blocks {
        let block_fields = Fields::new(
            block
                .as_object()
                .ok_or_else(|| CaptureRefusal::new("assistant block is not an object"))?,
        );
        let mut block = block.clone();
        match block_fields.type_of()? {
            "text" => block_fields.exact(&["type", "text"], &[])?,
            "thinking" => {
                block_fields.exact(&["type", "thinking"], &["signature", "thinkingSignature"])?;
                if block
                    .get("thinking")
                    .and_then(|v| v.as_str())
                    .is_some_and(|text| !text.is_empty())
                    || block.as_object().is_some_and(|object| {
                        object.contains_key("signature") || object.contains_key("thinkingSignature")
                    })
                {
                    if let Some(object) = block.as_object_mut() {
                        object.insert(
                            "thinking".to_owned(),
                            serde_json::Value::String(String::new()),
                        );
                        object.remove("signature");
                        object.remove("thinkingSignature");
                    }
                    changed = true;
                }
            }
            "redacted_thinking" => {
                block_fields.exact(&["type", "data"], &[])?;
                if block
                    .get("data")
                    .and_then(|v| v.as_str())
                    .is_some_and(|text| !text.is_empty())
                {
                    if let Some(object) = block.as_object_mut() {
                        object.insert("data".to_owned(), serde_json::Value::String(String::new()));
                    }
                    changed = true;
                }
            }
            // Tool-bearing shapes are non-private agent payloads whose
            // field sets the pi importer admits opaquely; only the block
            // type is admitted here.
            "tool_use" | "tool_result" => {}
            other => {
                return Err(CaptureRefusal::new(format!(
                    "unknown pi assistant content block {other:?}"
                )));
            }
        }
        out.push(block);
    }
    Ok(if changed {
        let mut new = message.clone();
        new.insert("content".to_owned(), serde_json::Value::Array(out));
        Some(new)
    } else {
        None
    })
}

/// Filters one pi `assistantMessageEvent`; `Some(fields)` when changed.
fn filter_pi_assistant_event(
    event: &serde_json::Map<String, serde_json::Value>,
) -> Result<Option<serde_json::Map<String, serde_json::Value>>, CaptureRefusal> {
    let fields = Fields::new(event);
    let kind = fields.type_of()?;
    const KINDS: &[&str] = &[
        "start",
        "text_start",
        "text_delta",
        "text_end",
        "thinking_start",
        "thinking_delta",
        "thinking_end",
        "toolcall_start",
        "toolcall_delta",
        "toolcall_end",
        "done",
    ];
    if !KINDS.contains(&kind) {
        return Err(CaptureRefusal::new(format!(
            "unknown pi assistant stream event {kind:?}"
        )));
    }
    if let Some(index) = event.get("contentIndex")
        && !index.is_u64()
    {
        return Err(CaptureRefusal::new("contentIndex is not a u64"));
    }
    let mut out = event.clone();
    let mut changed = false;
    match kind {
        "thinking_delta" => {
            if out
                .get("delta")
                .and_then(|v| v.as_str())
                .is_some_and(|text| !text.is_empty())
            {
                out.insert("delta".to_owned(), serde_json::Value::String(String::new()));
                changed = true;
            }
        }
        "thinking_end"
            if out
                .get("content")
                .and_then(|v| v.as_str())
                .is_some_and(|text| !text.is_empty()) =>
        {
            out.insert(
                "content".to_owned(),
                serde_json::Value::String(String::new()),
            );
            changed = true;
        }
        _ => {}
    }
    Ok(if changed { Some(out) } else { None })
}

#[cfg(test)]
mod tests {
    use super::*;

    const OPI_MESSAGE_END: &str = r#"{"type":"Agent","event":{"message":{"api":"Anthropic","content":[{"signature":"SIG","thinking":"SECRET","type":"thinking"},{"text":"hi","type":"text"},{"data":"REDACTED","type":"redacted_thinking"},{"tool_call":{"arguments":"{}","id":"t1","name":"bash"},"type":"tool_call"}],"error_message":null,"model":"glm","provider":"zhipu","response_id":"resp-1","response_model":null,"role":"assistant","stop_reason":"stop","timestamp_ms":123,"type":"Llm","usage":{"input_tokens":1,"output_tokens":2,"cache_read_tokens":0,"cache_write_tokens":0,"cache_write_1h_tokens":null,"reasoning_tokens":null,"reported":true}},"type":"MessageEnd"}}"#;

    const OPI_TOOL_JSON: &str = r#"{"type":"Agent","event":{"args":{"thinking":"keep-me","nested":{"thinking":"also-kept"}},"tool_call_id":"t1","tool_name":"bash","type":"ToolExecutionStart"}}"#;

    fn pi_line(value: serde_json::Value) -> String {
        serde_json::to_string(&value).unwrap()
    }

    #[test]
    fn opi_thinking_is_removed_and_placeholders_preserved() {
        let filtered = filter_record(OPI_MESSAGE_END, StreamKind::OpiAgent).unwrap();
        let value: serde_json::Value = serde_json::from_str(&filtered).unwrap();
        let content = &value["event"]["message"]["content"];
        assert_eq!(content[0]["thinking"], "");
        assert!(content[0].get("signature").is_none());
        assert_eq!(content[0]["type"], "thinking");
        assert_eq!(content[2]["data"], "");
        assert_eq!(content[1]["text"], "hi");
        assert_eq!(content[3]["tool_call"]["name"], "bash");
        assert_eq!(value["event"]["message"]["usage"]["input_tokens"], 1);
        assert_eq!(value["event"]["message"]["model"], "glm");
        let text = filtered.as_ref();
        assert!(!text.contains("SECRET"));
        assert!(!text.contains("SIG"));
        assert!(!text.contains("REDACTED"));
    }

    #[test]
    fn safe_records_keep_exact_original_bytes() {
        for line in [
            OPI_TOOL_JSON,
            r#"{"type":"Agent","event":{"type":"AgentStart"}}"#,
            r#"{"type":"session_summary","session_id":"s","model":"m","turns":1,"provider_turns":2,"tokens":{"input":1,"output":2,"cache_read":3,"cache_write":4}}"#,
        ] {
            let filtered = filter_record(line, StreamKind::OpiAgent).unwrap();
            assert_eq!(filtered, line);
        }
    }

    #[test]
    fn tool_json_named_thinking_is_preserved() {
        let filtered = filter_record(OPI_TOOL_JSON, StreamKind::OpiAgent).unwrap();
        assert!(filtered.contains("keep-me"));
        assert!(filtered.contains("also-kept"));
    }

    #[test]
    fn opi_stream_thinking_deltas_are_emptied() {
        let line = r#"{"type":"Agent","event":{"assistant_event":{"content_index":0,"delta":"SECRETDELTA","partial":{"api":"Anthropic","content":[{"signature":"S","thinking":"SECRET","type":"thinking"}],"error_message":null,"model":"m","provider":"p","response_id":null,"response_model":null,"stop_reason":"stop","timestamp_ms":1,"usage":{"input_tokens":0,"output_tokens":0,"cache_read_tokens":0,"cache_write_tokens":0,"cache_write_1h_tokens":null,"reasoning_tokens":null,"reported":false}},"type":"thinking_delta"},"message":{"api":"Anthropic","content":[{"thinking":"","type":"thinking"}],"error_message":null,"model":"m","provider":"p","response_id":null,"response_model":null,"role":"assistant","stop_reason":"stop","timestamp_ms":1,"type":"Llm","usage":{"input_tokens":0,"output_tokens":0,"cache_read_tokens":0,"cache_write_tokens":0,"cache_write_1h_tokens":null,"reasoning_tokens":null,"reported":false}},"type":"MessageUpdate"}}"#;
        let filtered = filter_record(line, StreamKind::OpiAgent).unwrap();
        let text = filtered.as_ref();
        assert!(!text.contains("SECRETDELTA"));
        assert!(!text.contains("\"SECRET\""));
        let value: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(value["event"]["assistant_event"]["delta"], "");
        assert_eq!(value["event"]["assistant_event"]["content_index"], 0);
        assert_eq!(
            value["event"]["assistant_event"]["partial"]["content"][0]["thinking"],
            ""
        );
    }

    #[test]
    fn opi_agent_end_messages_are_transformed() {
        let line = format!(
            r#"{{"type":"Agent","event":{{"messages":[{msg},{{"content":[{{"text":"q","type":"text"}}],"role":"user","timestamp_ms":5,"type":"Llm"}}],"type":"AgentEnd"}}}}"#,
            msg = {
                let value: serde_json::Value = serde_json::from_str(OPI_MESSAGE_END).unwrap();
                serde_json::to_string(&value["event"]["message"]).unwrap()
            }
        );
        let filtered = filter_record(&line, StreamKind::OpiAgent).unwrap();
        assert!(!filtered.contains("SECRET"));
        let value: serde_json::Value = serde_json::from_str(&filtered).unwrap();
        assert_eq!(value["event"]["messages"][1]["role"], "user");
    }

    #[test]
    fn unknown_types_fields_and_duplicate_keys_refuse() {
        let unknown_envelope = r#"{"type":"Surprise","x":1}"#;
        assert!(filter_record(unknown_envelope, StreamKind::OpiAgent).is_err());
        let unknown_event = r#"{"type":"Agent","event":{"type":"Surprise"}}"#;
        assert!(filter_record(unknown_event, StreamKind::OpiAgent).is_err());
        let unknown_block =
            OPI_MESSAGE_END.replace(r#"{"text":"hi","type":"text"}"#, r#"{"type":"gif"}"#);
        assert!(filter_record(&unknown_block, StreamKind::OpiAgent).is_err());
        let unknown_field = r#"{"type":"Agent","event":{"type":"AgentStart","extra":1}}"#;
        assert!(filter_record(unknown_field, StreamKind::OpiAgent).is_err());
        let duplicate = r#"{"type":"Agent","type":"Agent","event":{"type":"AgentStart"}}"#;
        assert!(filter_record(duplicate, StreamKind::OpiAgent).is_err());
        let nested_duplicate = r#"{"type":"QueueUpdate","steering":["a","a"],"follow_up":[]}"#;
        let filtered = filter_record(nested_duplicate, StreamKind::OpiAgent);
        // Array duplicates are list entries, not map keys: admitted.
        assert!(filtered.is_ok());
        let map_duplicate = r#"{"type":"Agent","event":{"type":"ToolExecutionStart","tool_call_id":"a","tool_name":"b","args":{"x":1,"x":2}}}"#;
        assert!(filter_record(map_duplicate, StreamKind::OpiAgent).is_err());
    }

    #[test]
    fn pi_streams_are_detected_and_filtered() {
        let header = r#"{"type":"session","version":3,"id":"i","timestamp":"t","cwd":"/w"}"#;
        assert_eq!(detect_stream_kind(header).unwrap(), StreamKind::PiSession);
        let message = serde_json::json!({
            "type": "message_end",
            "message": {
                "role": "assistant",
                "id": "m-1",
                "content": [
                    {"type": "thinking", "thinking": "SECRET", "signature": "SIG"},
                    {"type": "text", "text": "done"},
                    {"type": "redacted_thinking", "data": "REDACTED"},
                    {"type": "tool_use", "id": "t", "name": "bash", "input": {"cmd": "ls"}}
                ],
                "api": "pi-messages",
                "provider": "local",
                "model": "scripted",
                "usage": {"input": 1, "output": 2, "cacheRead": 0, "cacheWrite": 0, "totalTokens": 3, "cost": {"total": 0}},
                "stopReason": "stop",
                "timestamp": 1756296001000i64
            }
        });
        let line = pi_line(message);
        let filtered = filter_record(&line, StreamKind::PiSession).unwrap();
        assert!(!filtered.contains("SECRET"));
        assert!(!filtered.contains("REDACTED"));
        let value: serde_json::Value = serde_json::from_str(&filtered).unwrap();
        let content = &value["message"]["content"];
        assert_eq!(content[0]["thinking"], "");
        assert!(content[0].get("signature").is_none());
        assert_eq!(content[2]["data"], "");
        assert_eq!(content[3]["input"]["cmd"], "ls");
        assert_eq!(value["message"]["usage"]["totalTokens"], 3);

        let delta = serde_json::json!({
            "type": "message_update",
            "usage": {"input": 0, "output": 0, "cacheRead": 0, "cacheWrite": 0, "totalTokens": 0, "cost": {}},
            "assistantMessageEvent": {"type": "thinking_delta", "contentIndex": 0, "delta": "SECRET"}
        });
        let delta_line = pi_line(delta);
        let filtered = filter_record(&delta_line, StreamKind::PiSession).unwrap();
        let value: serde_json::Value = serde_json::from_str(&filtered).unwrap();
        assert_eq!(value["assistantMessageEvent"]["delta"], "");
        assert_eq!(value["assistantMessageEvent"]["contentIndex"], 0);

        let safe = serde_json::json!({"type": "tool_execution_start", "toolCallId": "c"});
        let line = pi_line(safe);
        assert_eq!(
            filter_record(&line, StreamKind::PiSession).unwrap(),
            line.as_str()
        );

        let unknown = pi_line(serde_json::json!({"type": "mystery"}));
        assert!(filter_record(&unknown, StreamKind::PiSession).is_err());
        let leaking = pi_line(serde_json::json!({
            "type": "tool_result", "message": {"role": "assistant", "content": []}
        }));
        assert!(filter_record(&leaking, StreamKind::PiSession).is_err());
    }

    #[test]
    fn opi_stream_kind_detection_and_bad_headers() {
        assert_eq!(
            detect_stream_kind(r#"{"type":"Agent","event":{"type":"AgentStart"}}"#).unwrap(),
            StreamKind::OpiAgent
        );
        assert!(
            detect_stream_kind(
                r#"{"type":"session","version":1,"id":"i","timestamp":"t","cwd":"/w"}"#
            )
            .is_err()
        );
        assert!(
            detect_stream_kind(
                r#"{"type":"session","version":2,"id":"i","timestamp":"t","cwd":"/w","extra":1}"#
            )
            .is_err()
        );
        assert!(detect_stream_kind("not json").is_err());
    }
}

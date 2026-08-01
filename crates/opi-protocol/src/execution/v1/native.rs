//! Lossless native strings: arbitrary byte sequences carried reversibly over
//! the UTF-8 JSONL wire.
//!
//! Used for `execute` program/args/cwd/env values that may be non-UTF-8 on the
//! host. Command stdout/stderr chunk payloads use base64 ([`super::Base64Bytes`])
//! instead; the two encodings must not be mixed.

use std::borrow::Cow;

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};

/// The escape introducer (BMP Private Use Area). A literal U+E000 in input is
/// emitted twice; each invalid byte `xx` is emitted as U+E000 followed by
/// U+00xx.
const ESCAPE: char = '\u{E000}';

/// Error decoding a malformed [`NativeString`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NativeStringError {
    /// A trailing escape introducer with no following scalar.
    #[error("trailing escape introducer with no following scalar")]
    TrailingEscape,
    /// An escape introducer followed by a scalar outside U+0000..=U+00FF (and
    /// not the doubled introducer).
    #[error("escape introducer followed by an invalid scalar")]
    InvalidEscapeData,
}

/// A native string: an arbitrary byte sequence carried reversibly over a UTF-8
/// JSON string.
///
/// # Wire encoding
///
/// Valid UTF-8 chunks pass through verbatim except a literal U+E000, which is
/// emitted twice. Each invalid byte `xx` (0x00..=0xFF) is emitted as U+E000
/// followed by the scalar U+00xx. All of U+0000..=U+00FF are valid Unicode
/// scalars (the surrogate range U+D800..=U+DFFF lies entirely above U+00FF), so
/// no surrogate is ever emitted and serde accepts every emitted string. The
/// scheme is collision-free because a literal U+E000 is always doubled and
/// escape-data scalars are all <= U+00FF. `decode(encode(b)) == b` for every
/// byte sequence `b`.
///
/// Decoding is total: a corrupted wire form (trailing introducer, or an
/// introducer followed by a scalar outside U+0000..=U+00FF and not U+E000)
/// returns [`NativeStringError`] rather than panicking; the codec maps both
/// variants to [`super::FailureCode::ProtocolViolation`] at the session layer.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NativeString(Vec<u8>);

impl NativeString {
    /// Wrap an arbitrary byte sequence.
    pub fn from_bytes(bytes: impl Into<Vec<u8>>) -> Self {
        Self(bytes.into())
    }

    /// Wrap a `&str` (always valid input; no escaping needed for ASCII/UTF-8).
    pub fn from_utf8(s: &str) -> Self {
        Self(s.as_bytes().to_vec())
    }

    /// The underlying bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Consume into the underlying bytes.
    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }

    /// Encode these bytes to the wire string form.
    pub fn to_wire_string(&self) -> String {
        encode(&self.0)
    }

    /// Decode a wire string form back to bytes.
    pub fn from_wire_string(wire: &str) -> Result<Vec<u8>, NativeStringError> {
        decode(wire)
    }
}

/// Encode bytes to the U+E000-escaped wire string.
fn encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for chunk in bytes.utf8_chunks() {
        for c in chunk.valid().chars() {
            if c == ESCAPE {
                out.push(ESCAPE);
                out.push(ESCAPE);
            } else {
                out.push(c);
            }
        }
        for &byte in chunk.invalid() {
            out.push(ESCAPE);
            // 0..=255 are all valid Unicode scalars; `byte as char` never fails.
            out.push(byte as char);
        }
    }
    out
}

/// Decode a U+E000-escaped wire string back to bytes.
fn decode(wire: &str) -> Result<Vec<u8>, NativeStringError> {
    let mut out: Vec<u8> = Vec::with_capacity(wire.len());
    let mut chars = wire.chars();
    while let Some(c) = chars.next() {
        if c == ESCAPE {
            match chars.next() {
                None => return Err(NativeStringError::TrailingEscape),
                Some(ESCAPE) => {
                    let mut buf = [0u8; 4];
                    out.extend_from_slice(ESCAPE.encode_utf8(&mut buf).as_bytes());
                }
                Some(n) if (n as u32) <= 0xFF => out.push(n as u8),
                Some(_) => return Err(NativeStringError::InvalidEscapeData),
            }
        } else {
            let mut buf = [0u8; 4];
            out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
        }
    }
    Ok(out)
}

impl Serialize for NativeString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        encode(&self.0).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for NativeString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = <String as Deserialize>::deserialize(deserializer)?;
        decode(&wire)
            .map(NativeString)
            .map_err(serde::de::Error::custom)
    }
}

impl JsonSchema for NativeString {
    fn schema_name() -> Cow<'static, str> {
        "NativeString".into()
    }

    fn schema_id() -> Cow<'static, str> {
        "opi-protocol::execution::v1::NativeString".into()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "string",
            "description": "Native string: arbitrary bytes carried reversibly over UTF-8 via a U+E000 escape introducer. Valid UTF-8 passes through (literal U+E000 is doubled); each invalid byte xx is encoded as U+E000 U+00xx. See the opi-protocol rustdoc for the full algorithm and the per-platform OsString->bytes domain. Not base64."
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_ascii_and_multibyte() {
        for s in [
            "",
            "hello",
            "café",
            "日本語",
            "\u{E000}",
            "mixed \u{E000} text",
        ] {
            let native = NativeString::from_utf8(s);
            let wire = native.to_wire_string();
            let back = NativeString::from_wire_string(&wire).unwrap();
            assert_eq!(back, native.into_bytes(), "failed for {s:?}");
        }
    }

    #[test]
    fn round_trips_arbitrary_bytes_including_invalid_utf8() {
        let cases: &[&[u8]] = &[
            &[0xFF],
            &[0xC0, 0xAF],
            &[0xE4, 0xB8],
            &[0x00, 0x01, 0x7F, 0x80, 0xBF, 0xFE, 0xFF],
            b"\xFF\xFE\x00abc\xFF",
        ];
        for bytes in cases {
            let native = NativeString::from_bytes(*bytes);
            let wire = native.to_wire_string();
            // serde_json must accept the wire string (no surrogate, all valid scalars).
            let json = serde_json::to_string(&wire).unwrap();
            let reparsed: String = serde_json::from_str(&json).unwrap();
            assert_eq!(reparsed, wire, "serde_json round-trip failed");
            let back = NativeString::from_wire_string(&reparsed).unwrap();
            assert_eq!(back, *bytes, "byte round-trip failed for {bytes:?}");
        }
    }

    #[test]
    fn no_collision_between_literal_and_escaped() {
        // A literal U+00FF (valid UTF-8 0xC3 0xBF) must decode back to those
        // exact bytes, distinct from an escaped byte 0xFF (U+E000 U+00FF).
        let literal = NativeString::from_utf8("ÿ"); // U+00FF
        let escaped_byte = NativeString::from_bytes([0xFF]);
        assert_ne!(literal.as_bytes(), escaped_byte.as_bytes());
        // Both survive a full serde round-trip distinctly.
        let lit_json = serde_json::to_string(&literal).unwrap();
        let esc_json = serde_json::to_string(&escaped_byte).unwrap();
        let lit_back: NativeString = serde_json::from_str(&lit_json).unwrap();
        let esc_back: NativeString = serde_json::from_str(&esc_json).unwrap();
        assert_eq!(lit_back.as_bytes(), &[0xC3, 0xBF]);
        assert_eq!(esc_back.as_bytes(), &[0xFF]);
    }

    #[test]
    fn decode_rejects_malformed_wire() {
        // Trailing introducer.
        let trailing = format!("abc{ESCAPE}");
        assert_eq!(
            NativeString::from_wire_string(&trailing),
            Err(NativeStringError::TrailingEscape)
        );
        // Introducer followed by a scalar above U+00FF (and not the doubled one).
        let invalid = format!("{ESCAPE}\u{0100}");
        assert_eq!(
            NativeString::from_wire_string(&invalid),
            Err(NativeStringError::InvalidEscapeData)
        );
    }
}

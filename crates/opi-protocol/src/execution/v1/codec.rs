//! Stateless bounded JSONL codec: a capped line reader, typed decoders,
//! per-frame validators, and an encoder.
//!
//! The decoder is intentionally **not** built on [`std::io::BufRead::read_until`]
//! or [`serde_json::from_reader`]: both materialize an arbitrary-length line
//! before any size check can fire, so an oversized line would exhaust memory
//! before being rejected. [`LineReader`] reads with a running byte counter and
//! rejects a line that exceeds `max_line_size` *before* it is fully buffered.

use std::io::Read;

use serde::Serialize;

use super::bounds::Bounds;
use super::frames::{BackendToHost, HostToBackend};

/// A codec error: I/O, an oversized line, a bound violation, or invalid JSON.
#[derive(Debug, thiserror::Error)]
pub enum CodecError {
    /// Underlying I/O error.
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    /// A JSONL line exceeded `max_line_size` bytes before a newline was seen.
    #[error("a line exceeded max_line_size ({max_line_size} bytes)")]
    OversizedLine {
        /// The configured line-size cap that was exceeded.
        max_line_size: usize,
    },
    /// An `initialize` adapter configuration exceeded `max_configuration_size`.
    #[error("adapter configuration of {actual} bytes exceeded max_configuration_size ({limit})")]
    ConfigurationTooLarge { actual: usize, limit: usize },
    /// A `diagnostic` message exceeded `max_diagnostics_size`.
    #[error("diagnostic message of {actual} bytes exceeded max_diagnostics_size ({limit})")]
    DiagnosticsTooLarge { actual: usize, limit: usize },
    /// A decoded stdout/stderr chunk exceeded `max_decoded_chunk_size`.
    #[error("decoded output chunk of {actual} bytes exceeded max_decoded_chunk_size ({limit})")]
    OutputChunkTooLarge { actual: usize, limit: usize },
    /// Invalid JSON, or a frame that failed to deserialize.
    #[error("invalid json: {0}")]
    Json(#[from] serde_json::Error),
}

/// A buffered reader that yields one JSONL line at a time, capped at
/// `bounds.max_line_size`.
///
/// A line longer than the cap is rejected as soon as the cap is reached,
/// without buffering the whole line. On [`CodecError::OversizedLine`] the reader
/// is left mid-line; the caller (host) treats this as a terminal protocol
/// violation and tears down the stream rather than continuing to read.
pub struct LineReader<R> {
    inner: std::io::BufReader<R>,
    max_line_size: usize,
}

impl<R: Read> LineReader<R> {
    /// Create a reader with the given bounds.
    pub fn new(reader: R, bounds: Bounds) -> Self {
        Self {
            inner: std::io::BufReader::new(reader),
            max_line_size: bounds.max_line_size,
        }
    }

    /// Read one JSONL line into `out` (clearing it first), without the trailing
    /// newline. Carriage returns are stripped.
    ///
    /// Returns `Ok(true)` if a line was read, `Ok(false)` at clean EOF (no bytes
    /// remaining), or `Err(OversizedLine)` if a line exceeds the cap.
    pub fn read_line(&mut self, out: &mut Vec<u8>) -> Result<bool, CodecError> {
        out.clear();
        let cap = self.max_line_size;
        let mut byte = [0u8; 1];
        loop {
            match self.inner.read(&mut byte)? {
                0 => return Ok(!out.is_empty()),
                _ => {
                    if byte[0] == b'\n' {
                        // Strip a trailing CR.
                        if out.last() == Some(&b'\r') {
                            out.pop();
                        }
                        return Ok(true);
                    }
                    if out.len() >= cap {
                        return Err(CodecError::OversizedLine { max_line_size: cap });
                    }
                    out.push(byte[0]);
                }
            }
        }
    }
}

/// Deserialize a typed frame from one already-capped JSONL line (no newline).
pub fn decode_host(line: &[u8]) -> Result<HostToBackend, CodecError> {
    Ok(serde_json::from_slice(line)?)
}

/// Deserialize a typed frame from one already-capped JSONL line (no newline).
pub fn decode_backend(line: &[u8]) -> Result<BackendToHost, CodecError> {
    Ok(serde_json::from_slice(line)?)
}

/// Validate per-frame bounds (configuration size, diagnostics size) for a host
/// frame. Line/message size is enforced by the reader on input and by
/// [`encode_line`] on output.
pub fn validate_host(frame: &HostToBackend, bounds: &Bounds) -> Result<(), CodecError> {
    if let HostToBackend::Initialize(p) = frame {
        let actual = serde_json::to_vec(&p.adapter_config)?.len();
        if actual > bounds.max_configuration_size {
            return Err(CodecError::ConfigurationTooLarge {
                actual,
                limit: bounds.max_configuration_size,
            });
        }
    }
    Ok(())
}

/// Validate per-frame bounds (diagnostics size) for a backend frame.
pub fn validate_backend(frame: &BackendToHost, bounds: &Bounds) -> Result<(), CodecError> {
    let output_bytes = frame.output_bytes();
    if output_bytes > bounds.max_decoded_chunk_size {
        return Err(CodecError::OutputChunkTooLarge {
            actual: output_bytes,
            limit: bounds.max_decoded_chunk_size,
        });
    }

    let diagnostics = match frame {
        BackendToHost::Diagnostic(payload) => std::slice::from_ref(&payload.message),
        BackendToHost::Completed(payload) => {
            for diagnostic in &payload.diagnostics {
                validate_diagnostic(&diagnostic.message, bounds)?;
            }
            &[]
        }
        BackendToHost::Failed(payload) => {
            for diagnostic in &payload.diagnostics {
                validate_diagnostic(&diagnostic.message, bounds)?;
            }
            &[]
        }
        _ => &[],
    };
    for message in diagnostics {
        validate_diagnostic(message, bounds)?;
    }
    Ok(())
}

fn validate_diagnostic(message: &str, bounds: &Bounds) -> Result<(), CodecError> {
    let actual = message.len();
    if actual > bounds.max_diagnostics_size {
        return Err(CodecError::DiagnosticsTooLarge {
            actual,
            limit: bounds.max_diagnostics_size,
        });
    }
    Ok(())
}

/// Serialize a frame to a JSONL line (no trailing newline), rejecting output
/// longer than `max_line_size`.
pub fn encode_line<F: Serialize>(frame: &F, bounds: &Bounds) -> Result<String, CodecError> {
    let line = serde_json::to_string(frame)?;
    if line.len() > bounds.max_line_size {
        return Err(CodecError::OversizedLine {
            max_line_size: bounds.max_line_size,
        });
    }
    Ok(line)
}

/// Validate then encode a host frame.
pub fn encode_host(frame: &HostToBackend, bounds: &Bounds) -> Result<String, CodecError> {
    validate_host(frame, bounds)?;
    encode_line(frame, bounds)
}

/// Validate then encode a backend frame.
pub fn encode_backend(frame: &BackendToHost, bounds: &Bounds) -> Result<String, CodecError> {
    validate_backend(frame, bounds)?;
    encode_line(frame, bounds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::v1::frames::{AcceptedPayload, CompletedPayload, StdoutPayload};
    use crate::execution::v1::{Base64Bytes, CleanupState, Diagnostic, RequestId};

    fn accepted() -> BackendToHost {
        BackendToHost::Accepted(AcceptedPayload {
            request_id: RequestId::new("r1".to_string()).unwrap(),
        })
    }

    #[test]
    fn capped_reader_rejects_oversized_line_without_buffering_it() {
        // A Read wrapper that counts total bytes pulled from the source. The
        // decoder is wrapped in a BufReader whose fill is bounded by its own
        // capacity, so memory stays O(1) in the oversized line's length: a 64
        // KiB line with no newline must NOT be materialized.
        struct Counting<R> {
            inner: R,
            pulled: usize,
        }
        impl<R: Read> Read for Counting<R> {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                let n = self.inner.read(buf)?;
                self.pulled += n;
                Ok(n)
            }
        }

        let bounds = Bounds {
            max_line_size: 32,
            max_decoded_chunk_size: 8,
            max_configuration_size: 8,
            max_diagnostics_size: 8,
            max_cumulative_output: 64,
        };
        let input_len = 65_536usize;
        let payload = vec![b'A'; input_len];
        let mut counting = Counting {
            inner: std::io::Cursor::new(payload),
            pulled: 0,
        };
        let mut reader = LineReader::new(&mut counting, bounds);
        let mut out = Vec::new();
        let err = reader.read_line(&mut out).unwrap_err();
        assert!(matches!(
            err,
            CodecError::OversizedLine { max_line_size: 32 }
        ));
        // The codec buffer is capped at max_line_size.
        assert!(out.len() <= 32, "out buffered {} bytes", out.len());
        // And the whole oversized line was never pulled: bytes read are bounded
        // by the BufReader fill capacity, far below the 64 KiB line length.
        assert!(
            counting.pulled < input_len / 2,
            "decoder pulled {} of {} bytes (memory must be O(1) in line size)",
            counting.pulled,
            input_len
        );
    }

    #[test]
    fn capped_reader_accepts_line_at_cap() {
        let cap = 64;
        let bounds = Bounds {
            max_line_size: cap,
            max_decoded_chunk_size: 16,
            max_configuration_size: 16,
            max_diagnostics_size: 16,
            max_cumulative_output: 64,
        };
        // Exactly `cap` bytes then a newline.
        let mut input = vec![b'A'; cap];
        input.push(b'\n');
        let mut reader = LineReader::new(std::io::Cursor::new(input), bounds);
        let mut out = Vec::new();
        assert!(reader.read_line(&mut out).unwrap());
        assert_eq!(out.len(), cap);
    }

    #[test]
    fn encode_rejects_oversized_line() {
        let bounds = Bounds {
            max_line_size: 8,
            max_decoded_chunk_size: 4,
            max_configuration_size: 4,
            max_diagnostics_size: 4,
            max_cumulative_output: 16,
        };
        let frame = accepted();
        assert!(matches!(
            encode_backend(&frame, &bounds),
            Err(CodecError::OversizedLine { .. })
        ));
    }

    #[test]
    fn encode_enforces_decoded_chunk_limit() {
        let bounds = Bounds {
            max_line_size: 4096,
            max_decoded_chunk_size: 8,
            max_configuration_size: 16,
            max_diagnostics_size: 8,
            max_cumulative_output: 64,
        };
        let make = |size| {
            BackendToHost::Stdout(StdoutPayload {
                request_id: RequestId::new("r1".to_string()).unwrap(),
                data: Base64Bytes::from_bytes(vec![0; size]),
            })
        };
        assert!(encode_backend(&make(8), &bounds).is_ok());
        assert!(matches!(
            encode_backend(&make(9), &bounds),
            Err(CodecError::OutputChunkTooLarge {
                actual: 9,
                limit: 8
            })
        ));
    }

    #[test]
    fn encode_enforces_nested_diagnostic_limit() {
        let bounds = Bounds {
            max_line_size: 4096,
            max_decoded_chunk_size: 8,
            max_configuration_size: 16,
            max_diagnostics_size: 8,
            max_cumulative_output: 64,
        };
        let make = |message: &str| {
            BackendToHost::Completed(CompletedPayload {
                request_id: RequestId::new("r1".to_string()).unwrap(),
                exit: Some(0),
                signal: None,
                timed_out: false,
                cancelled: false,
                cleanup: CleanupState::Confirmed,
                diagnostics: vec![Diagnostic {
                    message: message.to_string(),
                }],
            })
        };
        assert!(encode_backend(&make("12345678"), &bounds).is_ok());
        assert!(matches!(
            encode_backend(&make("123456789"), &bounds),
            Err(CodecError::DiagnosticsTooLarge {
                actual: 9,
                limit: 8
            })
        ));
    }
}

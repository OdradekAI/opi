//! Incremental byte-to-text decoding for streaming adapters (AIR-006).
//!
//! Rust equivalent of the streaming `TextDecoder` semantics the reference
//! adapters rely on: multi-byte UTF-8 sequences split across network chunks
//! are carried across pushes, and each maximal invalid subsequence becomes one
//! U+FFFD replacement character instead of a fatal framing error. A per-chunk
//! `String::from_utf8_lossy` cannot provide this: it replaces both halves of a
//! sequence split at a chunk boundary.

/// Incremental UTF-8 decoder with streaming replacement semantics.
pub(crate) struct StreamTextDecoder {
    /// Incomplete trailing bytes of a possibly-valid sequence (or the tail of
    /// an invalid subsequence that cannot be classified until more input or
    /// the final flush arrives).
    carry: Vec<u8>,
}

impl StreamTextDecoder {
    pub(crate) fn new() -> Self {
        Self { carry: Vec::new() }
    }

    /// Decode one network chunk, returning all text that is provably complete.
    ///
    /// Valid prefixes are returned immediately; each maximal invalid
    /// subsequence becomes one U+FFFD; an incomplete trailing sequence stays
    /// buffered until the next push or [`Self::finish`].
    pub(crate) fn push(&mut self, chunk: &[u8]) -> String {
        self.carry.extend_from_slice(chunk);
        let mut decoded = String::new();
        loop {
            match std::str::from_utf8(&self.carry) {
                Ok(text) => {
                    decoded.push_str(text);
                    self.carry.clear();
                    return decoded;
                }
                Err(error) => {
                    let valid = error.valid_up_to();
                    if valid > 0 {
                        decoded.push_str(
                            std::str::from_utf8(&self.carry[..valid]).expect("validated prefix"),
                        );
                    }
                    match error.error_len() {
                        // A concrete invalid subsequence: one U+FFFD, then
                        // continue scanning the remaining bytes.
                        Some(invalid) => {
                            decoded.push('\u{FFFD}');
                            self.carry.drain(..valid + invalid);
                        }
                        // Incomplete tail: emit what precedes it and keep it
                        // buffered for the next push or the final flush.
                        None => {
                            self.carry.drain(..valid);
                            return decoded;
                        }
                    }
                }
            }
        }
    }

    /// Flush the buffered tail at end of stream: any remaining incomplete
    /// sequence becomes one U+FFFD replacement.
    pub(crate) fn finish(&mut self) -> String {
        let tail = String::from_utf8_lossy(&self.carry).into_owned();
        self.carry.clear();
        tail
    }
}

impl Default for StreamTextDecoder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::StreamTextDecoder;

    fn decode_all(chunks: &[&[u8]]) -> String {
        let mut decoder = StreamTextDecoder::new();
        let mut text = String::new();
        for chunk in chunks {
            text.push_str(&decoder.push(chunk));
        }
        text.push_str(&decoder.finish());
        text
    }

    /// The canonical legal fixture: ASCII, 2/3/4-byte characters and CRLF
    /// separators must survive every possible byte-boundary split.
    const LEGAL_FIXTURE: &str = "line1: 你好\n\r\n🌍 emoji ✅ trailing\rdata: done\n\n";

    #[test]
    fn legal_utf8_survives_every_byte_boundary_split() {
        let bytes = LEGAL_FIXTURE.as_bytes();
        for split in 0..=bytes.len() {
            let chunks = [&bytes[..split], &bytes[split..]];
            assert_eq!(
                decode_all(&chunks),
                LEGAL_FIXTURE,
                "two-way split at byte {split}"
            );
        }
    }

    #[test]
    fn legal_utf8_survives_single_byte_chunks() {
        let bytes = LEGAL_FIXTURE.as_bytes();
        let chunks: Vec<&[u8]> = bytes.iter().map(std::slice::from_ref).collect();
        assert_eq!(decode_all(&chunks), LEGAL_FIXTURE);
    }

    #[test]
    fn split_four_byte_sequence_across_three_chunks() {
        // 🌍 = F0 9F 8C 8D split as 1+1+2 bytes.
        let chunks: [&[u8]; 3] = [b"\xF0", b"\x9F", b"\x8C\x8D"];
        assert_eq!(decode_all(&chunks), "🌍");
    }

    #[test]
    fn each_maximal_invalid_subsequence_becomes_one_replacement() {
        // Lone continuation byte, then an overlong encoding start, then a
        // valid character: three maximal invalid subsequences, then 'A'.
        let chunks: [&[u8]; 1] = [b"\x80\x80\x80A"];
        assert_eq!(decode_all(&chunks), "\u{FFFD}\u{FFFD}\u{FFFD}A");
    }

    #[test]
    fn invalid_bytes_split_across_chunks_stay_replacement_not_fatal() {
        // E2 82 AC is a valid €; E2 82 28 is invalid (E2-start with wrong
        // continuation): the invalid pair is replaced and '(' survives.
        let chunks: [&[u8]; 2] = [b"\xE2\x82", b"\x28"];
        assert_eq!(decode_all(&chunks), "\u{FFFD}(");
    }

    #[test]
    fn truncated_sequence_at_eof_becomes_one_replacement() {
        // A truncated 3-byte sequence with no continuation at EOF.
        let chunks: [&[u8]; 2] = [b"ok\xE2\x82", b""];
        assert_eq!(decode_all(&chunks), "ok\u{FFFD}");
    }

    #[test]
    fn invalid_subsequence_then_incomplete_tail_keeps_its_bytes() {
        // The lone 0x80 is replaced immediately; the truncated 你 sequence
        // stays buffered across pushes instead of losing its lead byte.
        let mut decoder = StreamTextDecoder::new();
        assert_eq!(decoder.push(b"\x80\xE4\xBD"), "\u{FFFD}");
        assert_eq!(decoder.push(b"\xA0!"), "你!");
        assert_eq!(decoder.finish(), "");
    }

    #[test]
    fn invalid_then_valid_after_chunk_boundary() {
        // First chunk ends mid-way inside an invalid subsequence; the next
        // chunk supplies the disqualifying byte plus a valid character.
        let chunks: [&[u8]; 2] = [b"a\xC3", b"\x28b"];
        assert_eq!(decode_all(&chunks), "a\u{FFFD}(b");
    }

    #[test]
    fn incremental_emission_does_not_wait_for_eof() {
        let mut decoder = StreamTextDecoder::new();
        assert_eq!(decoder.push(b"complete text\n"), "complete text\n");
        // First two bytes of 你 arrive alone: nothing is emitted or replaced.
        assert_eq!(decoder.push(b"\xE4\xBD"), "");
        assert_eq!(decoder.push(b"\xA0\xE5\xA5\xBD"), "你好");
        assert_eq!(decoder.finish(), "");
    }
}

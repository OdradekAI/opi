//! AWS event-stream binary frame parser (task 3.1).
//!
//! Parses the binary event-stream encoding used by Bedrock converse-stream
//! responses. No live AWS dependency.

/// A parsed event-stream frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventFrame {
    /// The `:event-type` header value (e.g. "messageStart", "contentBlockDelta").
    pub event_type: String,
    /// The `:content-type` header value (e.g. "application/json").
    pub content_type: String,
    /// The payload bytes.
    pub payload: Vec<u8>,
}

/// Parse all complete frames from a byte buffer.
///
/// Returns parsed frames and removes consumed bytes from the buffer.
/// Incomplete frames remain in the buffer for future appends.
pub fn parse_frames(buffer: &mut Vec<u8>) -> Vec<EventFrame> {
    let mut frames = Vec::new();
    while buffer.len() >= MIN_FRAME_SIZE {
        let total_len = read_u32_be(&buffer[0..4]);
        if total_len < (PRELUDE_LEN + 4) as u32 {
            // Malformed: too short even for prelude + message CRC
            buffer.drain(..4); // skip the bad length bytes
            continue;
        }
        if (total_len as usize) > buffer.len() {
            break; // incomplete frame, wait for more data
        }
        let frame_bytes: Vec<u8> = buffer.drain(..total_len as usize).collect();
        if let Some(frame) = parse_single_frame(&frame_bytes) {
            frames.push(frame);
        }
    }
    frames
}

/// Minimum frame size: prelude (12 bytes) + message CRC (4 bytes).
const MIN_FRAME_SIZE: usize = 16;
/// Prelude length: total_len (4) + headers_len (4) + prelude CRC (4).
const PRELUDE_LEN: usize = 12;

fn parse_single_frame(data: &[u8]) -> Option<EventFrame> {
    if data.len() < PRELUDE_LEN + 4 {
        return None;
    }

    let total_len = read_u32_be(&data[0..4]) as usize;
    let headers_len = read_u32_be(&data[4..8]) as usize;
    let prelude_crc = read_u32_be(&data[8..12]);
    let message_crc = read_u32_be(&data[data.len() - 4..]);

    if total_len != data.len() {
        return None;
    }
    if crc32(&data[0..8]) != prelude_crc {
        return None;
    }
    if crc32(&data[..data.len() - 4]) != message_crc {
        return None;
    }

    // Skip prelude CRC (bytes 8..12)
    let headers_start = PRELUDE_LEN;
    let headers_end = headers_start + headers_len;
    if headers_end > data.len() - 4 {
        return None; // headers overflow into CRC space
    }

    let headers_data = &data[headers_start..headers_end];
    let payload = &data[headers_end..data.len() - 4]; // exclude message CRC

    let mut event_type = String::new();
    let mut content_type = String::new();

    let mut pos = 0;
    while pos < headers_data.len() {
        // Header name length: 1 byte
        let name_len = headers_data[pos] as usize;
        pos += 1;
        if pos + name_len + 1 > headers_data.len() {
            return None;
        }
        let name = std::str::from_utf8(&headers_data[pos..pos + name_len]).unwrap_or("");
        pos += name_len;

        // Header type: 1 byte (7 = string, other types ignored)
        let header_type = headers_data[pos];
        pos += 1;

        if header_type == 7 {
            // String header: 2 bytes length + value
            if pos + 2 > headers_data.len() {
                return None;
            }
            let val_len = read_u16_be(&headers_data[pos..pos + 2]) as usize;
            pos += 2;
            if pos + val_len > headers_data.len() {
                return None;
            }
            let value = std::str::from_utf8(&headers_data[pos..pos + val_len]).unwrap_or("");
            pos += val_len;

            match name {
                ":event-type" => event_type = value.to_string(),
                ":content-type" => content_type = value.to_string(),
                _ => {}
            }
        } else {
            // Skip non-string headers based on type
            let skip_len = skip_header_value_len(header_type, &headers_data[pos..])?;
            if pos + skip_len > headers_data.len() {
                return None;
            }
            pos += skip_len;
        }
    }

    Some(EventFrame {
        event_type,
        content_type,
        payload: payload.to_vec(),
    })
}

/// Return the byte length of a non-string header value to skip.
fn skip_header_value_len(header_type: u8, rest: &[u8]) -> Option<usize> {
    match header_type {
        0 | 1 => Some(0), // bool true/false
        2 => Some(1),     // byte
        3 => Some(2),     // short
        4 => Some(4),     // int
        5 => Some(8),     // long
        6 => {
            // bytes: 2-byte length prefix + variable-length data
            if rest.len() < 2 {
                return None;
            }
            let len = read_u16_be(&rest[0..2]) as usize;
            Some(2 + len)
        }
        8 => Some(8),  // timestamp
        9 => Some(16), // uuid
        _ => Some(0),
    }
}

fn read_u32_be(bytes: &[u8]) -> u32 {
    u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn read_u16_be(bytes: &[u8]) -> u16 {
    u16::from_be_bytes([bytes[0], bytes[1]])
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in bytes {
        crc ^= byte as u32;
        for _ in 0..8 {
            if (crc & 1) == 1 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

/// Build a synthetic event-stream frame for testing.
pub fn build_test_frame(event_type: &str, content_type: &str, payload: &[u8]) -> Vec<u8> {
    let mut headers = Vec::new();
    // :event-type header (type 7 = string)
    let event_type_name = b":event-type";
    headers.push(event_type_name.len() as u8);
    headers.extend_from_slice(event_type_name);
    headers.push(7); // string type
    let et_len = event_type.len() as u16;
    headers.extend_from_slice(&et_len.to_be_bytes());
    headers.extend_from_slice(event_type.as_bytes());

    // :content-type header
    let ct_name = b":content-type";
    headers.push(ct_name.len() as u8);
    headers.extend_from_slice(ct_name);
    headers.push(7);
    let ct_len = content_type.len() as u16;
    headers.extend_from_slice(&ct_len.to_be_bytes());
    headers.extend_from_slice(content_type.as_bytes());

    let headers_len = headers.len() as u32;
    let total_len: u32 = 4 + 4 + 4 + headers_len + payload.len() as u32 + 4; // prelude + headers + payload + CRC

    let mut frame = Vec::with_capacity(total_len as usize);
    // Prelude
    frame.extend_from_slice(&total_len.to_be_bytes());
    frame.extend_from_slice(&headers_len.to_be_bytes());
    // Prelude CRC (placeholder 0 — real CRC not needed for parser tests)
    let prelude_crc = crc32(&frame);
    frame.extend_from_slice(&prelude_crc.to_be_bytes());
    // Headers
    frame.extend_from_slice(&headers);
    // Payload
    frame.extend_from_slice(payload);
    // Message CRC (placeholder 0)
    let message_crc = crc32(&frame);
    frame.extend_from_slice(&message_crc.to_be_bytes());

    frame
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_test_frame_with_prefix_headers(prefix_headers: &[u8], payload: &[u8]) -> Vec<u8> {
        let mut headers = Vec::new();
        headers.extend_from_slice(prefix_headers);

        let event_type_name = b":event-type";
        headers.push(event_type_name.len() as u8);
        headers.extend_from_slice(event_type_name);
        headers.push(7);
        let event_type = b"messageStart";
        headers.extend_from_slice(&(event_type.len() as u16).to_be_bytes());
        headers.extend_from_slice(event_type);

        let content_type_name = b":content-type";
        headers.push(content_type_name.len() as u8);
        headers.extend_from_slice(content_type_name);
        headers.push(7);
        let content_type = b"application/json";
        headers.extend_from_slice(&(content_type.len() as u16).to_be_bytes());
        headers.extend_from_slice(content_type);

        let headers_len = headers.len() as u32;
        let total_len: u32 = 4 + 4 + 4 + headers_len + payload.len() as u32 + 4;

        let mut frame = Vec::with_capacity(total_len as usize);
        frame.extend_from_slice(&total_len.to_be_bytes());
        frame.extend_from_slice(&headers_len.to_be_bytes());
        let prelude_crc = crc32(&frame);
        frame.extend_from_slice(&prelude_crc.to_be_bytes());
        frame.extend_from_slice(&headers);
        frame.extend_from_slice(payload);
        let message_crc = crc32(&frame);
        frame.extend_from_slice(&message_crc.to_be_bytes());
        frame
    }

    #[test]
    fn parse_single_message_start_frame() {
        let payload = br#"{"role":"assistant"}"#;
        let frame_bytes = build_test_frame("messageStart", "application/json", payload);

        let mut buffer = frame_bytes.clone();
        let frames = parse_frames(&mut buffer);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].event_type, "messageStart");
        assert_eq!(frames[0].content_type, "application/json");
        assert_eq!(frames[0].payload, payload);
        assert!(buffer.is_empty());
    }

    #[test]
    fn parse_multiple_frames_from_buffer() {
        let f1 = build_test_frame(
            "messageStart",
            "application/json",
            br#"{"role":"assistant"}"#,
        );
        let f2 = build_test_frame(
            "contentBlockDelta",
            "application/json",
            br#"{"delta":{"text":"Hi"}}"#,
        );
        let f3 = build_test_frame(
            "messageStop",
            "application/json",
            br#"{"stopReason":"end_turn"}"#,
        );

        let mut buffer = Vec::new();
        buffer.extend_from_slice(&f1);
        buffer.extend_from_slice(&f2);
        buffer.extend_from_slice(&f3);

        let frames = parse_frames(&mut buffer);
        assert_eq!(frames.len(), 3);
        assert_eq!(frames[0].event_type, "messageStart");
        assert_eq!(frames[1].event_type, "contentBlockDelta");
        assert_eq!(frames[2].event_type, "messageStop");
        assert!(buffer.is_empty());
    }

    #[test]
    fn incomplete_frame_stays_in_buffer() {
        let frame = build_test_frame("messageStart", "application/json", b"{}");
        let mut buffer = frame[..frame.len() - 5].to_vec(); // chop off last 5 bytes

        let frames = parse_frames(&mut buffer);
        assert!(frames.is_empty());
        assert!(!buffer.is_empty());
    }

    #[test]
    fn second_chunk_completes_frame() {
        let frame = build_test_frame("messageStart", "application/json", b"{}");
        let split = frame.len() / 2;

        let mut buffer = frame[..split].to_vec();
        let mut frames = parse_frames(&mut buffer);
        assert!(frames.is_empty());

        buffer.extend_from_slice(&frame[split..]);
        frames.extend(parse_frames(&mut buffer));
        assert_eq!(frames.len(), 1);
        assert!(buffer.is_empty());
    }

    #[test]
    fn empty_buffer_returns_no_frames() {
        let mut buffer = Vec::new();
        let frames = parse_frames(&mut buffer);
        assert!(frames.is_empty());
    }

    #[test]
    fn frame_with_empty_payload() {
        let frame = build_test_frame("contentBlockStop", "application/json", b"");
        let mut buffer = frame;
        let frames = parse_frames(&mut buffer);
        assert_eq!(frames.len(), 1);
        assert!(frames[0].payload.is_empty());
    }

    #[test]
    fn parse_frames_ignores_header_length_past_total_length_without_panic() {
        let mut frame = build_test_frame("messageStart", "application/json", b"{}");
        let total_len = read_u32_be(&frame[0..4]);
        let bad_headers_len = total_len;
        frame[4..8].copy_from_slice(&bad_headers_len.to_be_bytes());
        let prelude_crc = crc32(&frame[0..8]);
        frame[8..12].copy_from_slice(&prelude_crc.to_be_bytes());
        let message_crc_pos = frame.len() - 4;
        let message_crc = crc32(&frame[..message_crc_pos]);
        frame[message_crc_pos..].copy_from_slice(&message_crc.to_be_bytes());

        let mut buffer = frame;
        let frames = parse_frames(&mut buffer);
        assert!(frames.is_empty());
        assert!(buffer.is_empty());
    }

    #[test]
    fn parse_frames_ignores_string_header_value_past_header_region_without_panic() {
        let mut frame = build_test_frame("messageStart", "application/json", b"{}");
        let headers_start = PRELUDE_LEN;
        let event_name_len = b":event-type".len();
        let event_len_pos = headers_start + 1 + event_name_len + 1;
        frame[event_len_pos..event_len_pos + 2].copy_from_slice(&u16::MAX.to_be_bytes());
        let message_crc_pos = frame.len() - 4;
        let message_crc = crc32(&frame[..message_crc_pos]);
        frame[message_crc_pos..].copy_from_slice(&message_crc.to_be_bytes());

        let mut buffer = frame;
        let frames = parse_frames(&mut buffer);
        assert!(frames.is_empty());
        assert!(buffer.is_empty());
    }

    #[test]
    fn parse_frames_ignores_garbage_shorter_than_min_frame_without_panic() {
        let mut buffer = vec![0xAA, 0xBB, 0xCC, 0xDD, 0xEE];
        let frames = parse_frames(&mut buffer);
        assert!(frames.is_empty());
        assert_eq!(buffer, vec![0xAA, 0xBB, 0xCC, 0xDD, 0xEE]);
    }

    #[test]
    fn parse_frames_ignores_bad_crc_without_panic() {
        let mut frame = build_test_frame("messageStart", "application/json", b"{}");
        let last = frame.len() - 1;
        frame[last] ^= 0xFF;

        let mut buffer = frame;
        let frames = parse_frames(&mut buffer);
        assert!(frames.is_empty());
        assert!(buffer.is_empty());
    }

    #[test]
    fn parse_frames_skips_bool_and_uuid_headers_before_event_type() {
        let mut prefix_headers = Vec::new();

        let bool_name = b"x-bool";
        prefix_headers.push(bool_name.len() as u8);
        prefix_headers.extend_from_slice(bool_name);
        prefix_headers.push(0);

        let uuid_name = b"x-uuid";
        prefix_headers.push(uuid_name.len() as u8);
        prefix_headers.extend_from_slice(uuid_name);
        prefix_headers.push(9);
        prefix_headers.extend_from_slice(&[0xAB; 16]);

        let payload = br#"{"role":"assistant"}"#;
        let mut buffer = build_test_frame_with_prefix_headers(&prefix_headers, payload);
        let frames = parse_frames(&mut buffer);

        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].event_type, "messageStart");
        assert_eq!(frames[0].content_type, "application/json");
        assert_eq!(frames[0].payload, payload);
        assert!(buffer.is_empty());
    }
}

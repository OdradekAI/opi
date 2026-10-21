use crate::ws::frame::{Decoder, Fault, Frame, Message, Messages, checked_storage_len};

fn wire(first: u8, payload: &[u8]) -> Vec<u8> {
    let mut bytes = vec![first];
    match payload.len() {
        0..=125 => bytes.push(payload.len() as u8),
        126..=65535 => {
            bytes.push(126);
            bytes.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        }
        _ => {
            bytes.push(127);
            bytes.extend_from_slice(&(payload.len() as u64).to_be_bytes());
        }
    }
    bytes.extend_from_slice(payload);
    bytes
}

fn parse_chunks(bytes: &[u8], pmd: bool, size: usize) -> Result<Vec<Frame>, Fault> {
    let mut decoder = Decoder::new(pmd);
    let mut frames = Vec::new();
    for chunk in bytes.chunks(size) {
        frames.extend(decoder.feed(chunk)?);
    }
    decoder.eof()?;
    Ok(frames)
}

fn assert_bad(bytes: &[u8], pmd: bool) {
    for size in [1, 2, 3, bytes.len().max(1)] {
        let mut decoder = Decoder::new(pmd);
        let mut rejected = false;
        for chunk in bytes.chunks(size) {
            if decoder.feed(chunk).is_err() {
                rejected = true;
                break;
            }
        }
        assert!(rejected, "accepted {bytes:02x?} at chunk={size}");
        assert!(decoder.failed());
        assert!(matches!(decoder.feed(&[0x81, 0]), Err(Fault::Closed)));
        assert!(matches!(decoder.eof(), Err(Fault::Closed)));
    }
}

#[test]
fn rejects_high_bit_and_nonminimal_lengths_in_every_split() {
    for length in [1_u64 << 63, u64::MAX] {
        let mut bytes = vec![0x82, 127];
        bytes.extend_from_slice(&length.to_be_bytes());
        assert_bad(&bytes, false);
    }
    for length in [0_u16, 1, 125] {
        let mut bytes = vec![0x82, 126];
        bytes.extend_from_slice(&length.to_be_bytes());
        assert_bad(&bytes, false);
    }
    for length in [0_u64, 125, 126, 65535] {
        let mut bytes = vec![0x82, 127];
        bytes.extend_from_slice(&length.to_be_bytes());
        assert_bad(&bytes, false);
    }
}

#[test]
fn accepts_shortest_payload_boundaries_without_tail_loss() {
    for length in [0, 1, 125, 126, 65535, 65536] {
        let payload: Vec<u8> = (0..length).map(|n| (n % 251) as u8).collect();
        let mut bytes = wire(0x82, &payload);
        bytes.extend(wire(0x81, b"tail"));
        for size in [1, 7, bytes.len()] {
            let frames = parse_chunks(&bytes, false, size).unwrap();
            assert_eq!(frames.len(), 2);
            assert_eq!(frames[0].payload, payload);
            assert_eq!(frames[1].payload, b"tail");
        }
    }
}

#[test]
fn huge_valid_declaration_only_stores_actual_bytes() {
    let mut decoder = Decoder::new(false);
    let mut head = vec![0x82, 127];
    head.extend_from_slice(&(1_u64 << 40).to_be_bytes());
    for byte in &head {
        assert!(decoder.feed(&[*byte]).unwrap().is_empty());
    }
    assert_eq!(decoder.buffered_len(), 10);
    assert_eq!(decoder.buffered_capacity(), 10);
    assert!(decoder.feed(b"abc").unwrap().is_empty());
    assert_eq!(decoder.buffered_len(), 13);
    assert!(decoder.buffered_capacity() < 1024);
    assert!(decoder.eof().is_err());
    assert!(decoder.failed());
}

#[test]
fn representation_overflow_is_typed_without_allocating() {
    assert!(matches!(
        checked_storage_len(usize::MAX, 1),
        Err(Fault::Capacity)
    ));
    assert!(matches!(
        checked_storage_len(isize::MAX as usize, 1),
        Err(Fault::Capacity)
    ));
    assert_eq!(checked_storage_len(17, 23).unwrap(), 40);
}

#[test]
fn invalid_opcode_rsv_and_mask_bits_reject_before_body() {
    for opcode in [3, 4, 5, 6, 7, 11, 12, 13, 14, 15] {
        assert_bad(&[0x80 | opcode, 0], true);
    }
    for rsv in [0x10, 0x20, 0x30] {
        assert_bad(&[0x81 | rsv, 0], true);
    }
    assert_bad(&[0xc1, 0], false);
    assert_bad(&[0x81, 0x80], false);
    for opcode in [8, 9, 10] {
        assert_bad(&[0xc0 | opcode, 0], true);
        assert_bad(&[opcode, 0], true);
        assert_bad(&[0x80 | opcode, 126], true);
    }
}

#[test]
fn fragmentation_requires_unique_initial_and_uncompressed_continuations() {
    assert_bad(&[0x80, 0], true);
    assert_bad(&[0x01, 0, 0x81, 0], true);
    assert_bad(&[0x41, 0, 0xc0, 0], true);
    let bytes = [0x41, 1, b'a', 0x89, 1, b'p', 0x00, 1, b'b', 0x80, 1, b'c'];
    for size in [1, 2, bytes.len()] {
        let frames = parse_chunks(&bytes, true, size).unwrap();
        assert_eq!(frames.len(), 4);
        assert!(frames[0].compressed);
        assert!(!frames[1].compressed);
        assert!(!frames[2].compressed);
        assert!(!frames[3].compressed);
        assert_eq!(
            frames.iter().map(|f| f.opcode).collect::<Vec<_>>(),
            [1, 9, 0, 0]
        );
    }
}

#[test]
fn every_incomplete_prefix_fails_eof_and_eof_is_permanent() {
    let bytes = wire(0x82, &[7; 126]);
    for end in 1..bytes.len() {
        let mut decoder = Decoder::new(false);
        assert!(decoder.feed(&bytes[..end]).unwrap().is_empty());
        assert!(decoder.eof().is_err(), "prefix {end}");
        assert!(decoder.failed());
        assert!(matches!(decoder.feed(&bytes[end..]), Err(Fault::Closed)));
    }
    let mut decoder = Decoder::new(false);
    decoder.feed(&[0x01, 0]).unwrap();
    assert!(decoder.eof().is_err(), "unfinished fragmented message");
    let mut decoder = Decoder::new(false);
    assert!(decoder.eof().is_ok());
    assert!(matches!(decoder.eof(), Err(Fault::Closed)));
    assert!(matches!(decoder.feed(&[0x81, 0]), Err(Fault::Closed)));
}

fn frame(fin: bool, opcode: u8, payload: &[u8]) -> Frame {
    Frame {
        fin,
        opcode,
        compressed: false,
        payload: payload.to_vec(),
    }
}

#[test]
fn message_fragments_keep_utf8_boundary_and_control_order() {
    let mut messages = Messages::new();
    assert!(
        messages
            .receive(frame(false, 1, &[0xe2]))
            .unwrap()
            .is_none()
    );
    let ping = messages.receive(frame(true, 9, b"ping")).unwrap().unwrap();
    assert_eq!(
        ping,
        Message {
            opcode: 9,
            compressed: false,
            payload: b"ping".to_vec()
        }
    );
    assert!(
        messages
            .receive(frame(false, 0, &[0x82]))
            .unwrap()
            .is_none()
    );
    let text = messages.receive(frame(true, 0, &[0xac])).unwrap().unwrap();
    assert_eq!(text.payload, "€".as_bytes());
    assert_eq!(text.opcode, 1);
    messages.eof().unwrap();
    assert!(matches!(
        messages.receive(frame(true, 1, b"late")),
        Err(Fault::Closed)
    ));
}

#[test]
fn malformed_utf8_and_close_are_terminal() {
    for payload in [&[0xff][..], &[0xc0, 0xaf], &[0xe2, 0x82]] {
        let mut messages = Messages::new();
        assert!(matches!(
            messages.receive(frame(true, 1, payload)),
            Err(Fault::Utf8)
        ));
        assert!(messages.failed());
        assert!(matches!(
            messages.receive(frame(true, 1, b"late")),
            Err(Fault::Closed)
        ));
    }
    for payload in [
        vec![0],
        1005_u16.to_be_bytes().to_vec(),
        999_u16.to_be_bytes().to_vec(),
        5000_u16.to_be_bytes().to_vec(),
        vec![3, 232, 0xff],
    ] {
        let mut messages = Messages::new();
        assert!(
            messages.receive(frame(true, 8, &payload)).is_err(),
            "{payload:?}"
        );
        assert!(messages.failed());
    }
    for code in [1000_u16, 1001, 1003, 1007, 1014, 3000, 4999] {
        let mut messages = Messages::new();
        let mut payload = code.to_be_bytes().to_vec();
        payload.extend_from_slice("完成".as_bytes());
        assert_eq!(
            messages
                .receive(frame(true, 8, &payload))
                .unwrap()
                .unwrap()
                .opcode,
            8
        );
        assert!(matches!(
            messages.receive(frame(true, 1, b"late")),
            Err(Fault::Closed)
        ));
    }
}

#[test]
fn compressed_message_is_assembled_without_pretending_it_is_utf8() {
    let mut messages = Messages::new();
    let mut initial = frame(false, 1, &[0xff]);
    initial.compressed = true;
    assert!(messages.receive(initial).unwrap().is_none());
    assert_eq!(
        messages
            .receive(frame(true, 9, b"p"))
            .unwrap()
            .unwrap()
            .opcode,
        9
    );
    let message = messages.receive(frame(true, 0, &[0xfe])).unwrap().unwrap();
    assert!(message.compressed);
    assert_eq!(message.opcode, 1);
    assert_eq!(message.payload, [0xff, 0xfe]);
    assert!(
        messages
            .receive(frame(true, 1, b"plain"))
            .unwrap()
            .is_some()
    );
}

#[test]
fn coalesced_failure_preserves_completed_prefix_exactly_once() {
    let mut bytes = wire(0x81, b"accepted");
    bytes.extend_from_slice(&[0xc9, 0]);
    let mut decoder = Decoder::new(true);
    let mut frames = Vec::new();
    assert!(decoder.feed_into(&bytes, &mut frames).is_err());
    assert!(decoder.failed());
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].payload, b"accepted");
    assert!(matches!(
        decoder.feed_into(&wire(0x81, b"late"), &mut frames),
        Err(Fault::Closed)
    ));
    assert_eq!(frames.len(), 1);
    assert_eq!(decoder.buffered_capacity(), 10);
}

#[test]
fn real_reservation_failure_keeps_typed_source_without_oom() {
    use std::error::Error;
    let error = Vec::<u8>::new().try_reserve(usize::MAX).unwrap_err();
    let fault = Fault::from(error);
    assert!(matches!(fault, Fault::Storage(_)));
    assert!(fault.source().is_some());
    assert!(fault.to_string().starts_with("WebSocket storage:"));
}

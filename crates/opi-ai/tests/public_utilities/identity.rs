//! Public short hash and UUIDv7 format, bounds and sequencing.

use opi_ai::identity::{UuidV7Error, UuidV7Generator, short_hash};

fn uuid_v7_bytes(id: &str) -> [u8; 16] {
    let hex: String = id.chars().filter(|c| *c != '-').collect();
    let mut bytes = [0u8; 16];
    for (slot, pair) in bytes.iter_mut().zip(hex.as_bytes().chunks(2)) {
        *slot = u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap();
    }
    bytes
}

fn uuid_v7_timestamp_ms(id: &str) -> u64 {
    let bytes = uuid_v7_bytes(id);
    u64::from(bytes[0]) << 40
        | u64::from(bytes[1]) << 32
        | u64::from(bytes[2]) << 24
        | u64::from(bytes[3]) << 16
        | u64::from(bytes[4]) << 8
        | u64::from(bytes[5])
}

fn uuid_v7_sequence(id: &str) -> u64 {
    let b = uuid_v7_bytes(id);
    (u64::from(b[6]) & 0x0f) << 37
        | u64::from(b[7]) << 29
        | (u64::from(b[8]) & 0x3f) << 23
        | u64::from(b[9]) << 15
        | u64::from(b[10]) << 7
        | u64::from(b[11]) >> 1
}

#[test]
fn short_hash_matches_pinned_source_fixtures() {
    // Expected values computed from the pinned JavaScript algorithm with
    // strict ToInt32/ToUint32 discipline (UTF-16 code units, Math.imul 32-bit
    // wrapping, self>>>16 vs cross>>>13 final mix, h2-then-h1 base36).
    let fixtures = [
        ("", "k4n83c7h0j2b"),
        ("a", "m8735310ae7sx"),
        ("hello", "1h6qa0qrowduu"),
        ("pi\u{2248}3.14", "1nlj7n2m2u17f"),
        ("\u{1F600}emoji", "a5yrnht4g44f"),
        ("0123456789abcdef0123456789abcdef", "1gcesuz1crg44z"),
    ];
    for (input, expected) in fixtures {
        assert_eq!(short_hash(input), expected, "mismatch for {input:?}");
    }
}

#[test]
fn short_hash_is_deterministic_and_distinct() {
    assert_eq!(short_hash("abc"), short_hash("abc"));
    assert_ne!(short_hash("abc"), short_hash("abd"));
}

#[test]
fn uuidv7_preserves_explicit_timestamp_and_layout() {
    let mut generator = UuidV7Generator::new();
    let id = generator.generate_at(1_700_000_000_123).unwrap();
    assert_eq!(uuid_v7_timestamp_ms(&id), 1_700_000_000_123);
    let bytes = uuid_v7_bytes(&id);
    assert_eq!(bytes[6] >> 4, 0x7, "version nibble");
    assert_eq!(bytes[8] >> 6, 0b10, "variant bits");
    assert!(bytes[12..16].iter().any(|byte| *byte != 0), "random tail");
}

#[test]
fn uuidv7_rejects_out_of_range_timestamps() {
    let mut generator = UuidV7Generator::new();
    let max = 0xffff_ffff_ffff;
    assert!(generator.generate_at(max).is_ok());
    assert_eq!(
        generator.generate_at(max + 1).unwrap_err(),
        UuidV7Error::InvalidTimestamp
    );
    assert_eq!(
        generator.generate(max + 1).unwrap_err(),
        UuidV7Error::InvalidTimestamp
    );
}

#[test]
fn uuidv7_ordinary_generation_never_regresses_on_rollback() {
    let mut generator = UuidV7Generator::new();
    let first = generator.generate(5_000).unwrap();
    assert_eq!(uuid_v7_timestamp_ms(&first), 5_000);
    let rolled_back = generator.generate(4_000).unwrap();
    assert_eq!(
        uuid_v7_timestamp_ms(&rolled_back),
        5_000,
        "clock rollback must not regress the timestamp"
    );
    let advanced = generator.generate(6_000).unwrap();
    assert_eq!(uuid_v7_timestamp_ms(&advanced), 6_000);
}

#[test]
fn uuidv7_explicit_path_does_not_advance_the_rollback_floor() {
    let mut generator = UuidV7Generator::new();
    generator.generate(5_000).unwrap();
    generator.generate_at(9_000).unwrap();
    let after = generator.generate(4_500).unwrap();
    assert_eq!(uuid_v7_timestamp_ms(&after), 5_000);
}

#[test]
fn uuidv7_sequence_increments_within_a_generator() {
    let mut generator = UuidV7Generator::new();
    let first = generator.generate_at(1_000).unwrap();
    let second = generator.generate_at(1_000).unwrap();
    assert_eq!(uuid_v7_sequence(&second), uuid_v7_sequence(&first) + 1);
    let ordinary = generator.generate(2_000).unwrap();
    assert_eq!(uuid_v7_sequence(&ordinary), uuid_v7_sequence(&second) + 1);
}

#[test]
fn uuidv7_generates_unique_well_formed_ids() {
    let mut generator = UuidV7Generator::new();
    let mut seen = std::collections::HashSet::new();
    for index in 0..16u64 {
        let id = generator.generate(10_000 + index).unwrap();
        assert_eq!(id.len(), 36);
        for (slot, dash) in [8usize, 13, 18, 23].iter().enumerate() {
            let _ = slot;
            assert_eq!(id.as_bytes()[*dash], b'-');
        }
        assert!(id.bytes().all(|byte| {
            byte == b'-' || byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
        }));
        assert!(seen.insert(id), "duplicate id generated");
    }
    for timestamp in [11_000u64, 11_001] {
        let id = generator.generate_at(timestamp).unwrap();
        assert!(seen.insert(id), "duplicate id across paths");
    }
}

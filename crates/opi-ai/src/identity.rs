//! Compatibility identity utilities (pinned sources:
//! `.repo/pi-1.3.0/packages/ai/src/utils/hash.ts` and `utils/uuid.ts`).
//!
//! [`short_hash`] is a fast deterministic hash used for short display and
//! compat identifiers (session seeds, `fc_` item ids). It is a compatibility
//! algorithm, **not** a cryptographic hash and **not** a durable identity
//! promise: wire/cache-key byte identity and source-qualified crypto hashes
//! (PKCE, AWS signing) keep their exact required encodings in their owning
//! modules and must never be replaced by this utility.
//!
//! [`UuidV7Generator`] reproduces the pinned time-ordered UUIDv7 generator:
//! explicit follower timestamps are preserved exactly, ordinary generation
//! never regresses on clock rollback, and sequence exhaustion fails with a
//! typed error. A generator owns its monotonic state; there is no module
//! global and no generic identity framework.

/// Maximum UUIDv7 timestamp (48 bits, milliseconds).
const MAX_UUID_V7_TIMESTAMP: u64 = 0xffff_ffff_ffff;

/// Maximum 41-bit sequence before the generator fails with
/// [`UuidV7Error::SequenceExhausted`].
const MAX_SEQUENCE: u64 = (1 << 41) - 1;

/// Fast deterministic hash to shorten long strings (pinned `shortHash`).
///
/// Mirrors the pinned JavaScript algorithm exactly: UTF-16 code units
/// (surrogate pairs contribute two units), 32-bit wrapping multiplies, the
/// sequential final mix (the second half mixes the already-updated `h1`),
/// and the unsigned base36 rendering of `h2` followed by `h1` without
/// padding. Non-cryptographic and non-durable: see the module docs.
pub fn short_hash(input: &str) -> String {
    const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    fn base36(mut value: u32) -> String {
        if value == 0 {
            return "0".to_owned();
        }
        let mut out = Vec::new();
        while value > 0 {
            out.push(DIGITS[(value % 36) as usize]);
            value /= 36;
        }
        out.reverse();
        String::from_utf8(out).expect("base36 digits are ASCII")
    }

    let mut h1: u32 = 0xdead_beef;
    let mut h2: u32 = 0x41c6_ce57;
    for unit in input.encode_utf16() {
        let unit = u32::from(unit);
        h1 = (h1 ^ unit).wrapping_mul(2_654_435_761);
        h2 = (h2 ^ unit).wrapping_mul(1_597_334_677);
    }
    h1 = (h1 ^ (h1 >> 16)).wrapping_mul(2_246_822_507)
        ^ (h2 ^ (h2 >> 13)).wrapping_mul(3_266_489_909);
    h2 = (h2 ^ (h2 >> 16)).wrapping_mul(2_246_822_507)
        ^ (h1 ^ (h1 >> 13)).wrapping_mul(3_266_489_909);
    format!("{}{}", base36(h2), base36(h1))
}

/// Typed UUIDv7 generation failure (pinned `RangeError` throws).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum UuidV7Error {
    /// The requested timestamp is not an integer in `0..=2^48-1`.
    #[error("UUIDv7 timestamp must be an integer between 0 and {MAX_UUID_V7_TIMESTAMP}")]
    InvalidTimestamp,
    /// The generator's 41-bit sequence is exhausted; reuse requires a new
    /// generator (the pinned source throws the same way).
    #[error("UUIDv7 generator sequence exhausted")]
    SequenceExhausted,
}

/// Time-ordered UUIDv7 generator owning its monotonic timestamp and sequence
/// state (pinned `uuidv7`; task 20.53.4).
///
/// `generate` takes the caller-supplied current time (injected for fake-clock
/// determinism) and never regresses on clock rollback; `generate_at` preserves
/// an explicit follower timestamp exactly and does not advance the ordinary
/// rollback floor. The first call seeds a 41-bit sequence from random bytes;
/// later calls increment it until exhaustion, past which generation
/// fails with [`UuidV7Error::SequenceExhausted`].
#[derive(Debug)]
pub struct UuidV7Generator {
    last_ordinary_timestamp: i64,
    sequence: Option<u64>,
}

impl UuidV7Generator {
    /// Create a generator with no state (sequence seeded on first use).
    pub fn new() -> Self {
        Self {
            last_ordinary_timestamp: -1,
            sequence: None,
        }
    }

    /// Generate an ordinary UUIDv7 using the caller-supplied current time in
    /// milliseconds.
    ///
    /// The effective timestamp is `max(now_ms, last_ordinary_timestamp)`, so
    /// a clock rollback never regresses generated ids. Fails with
    /// [`UuidV7Error::InvalidTimestamp`] when `now_ms` exceeds the 48-bit
    /// UUIDv7 timestamp range.
    pub fn generate(&mut self, now_ms: u64) -> Result<String, UuidV7Error> {
        if now_ms > MAX_UUID_V7_TIMESTAMP {
            return Err(UuidV7Error::InvalidTimestamp);
        }
        let effective = (self
            .last_ordinary_timestamp
            .max(i64::try_from(now_ms).expect("u64 fits i64 after the range check")))
            as u64;
        self.last_ordinary_timestamp = effective as i64;
        self.render(effective)
    }

    /// Generate a UUIDv7 with an explicit follower timestamp, preserved
    /// exactly.
    ///
    /// Unlike [`UuidV7Generator::generate`], this path does not advance the
    /// ordinary rollback floor. Fails with
    /// [`UuidV7Error::InvalidTimestamp`] outside `0..=2^48-1`.
    pub fn generate_at(&mut self, timestamp_ms: u64) -> Result<String, UuidV7Error> {
        if timestamp_ms > MAX_UUID_V7_TIMESTAMP {
            return Err(UuidV7Error::InvalidTimestamp);
        }
        self.render(timestamp_ms)
    }

    /// Test hook: pin the sequence state directly (production exhaustion
    /// would otherwise need `2^41` calls).
    pub(crate) fn set_sequence_for_tests(&mut self, sequence: u64) {
        self.sequence = Some(sequence);
    }

    fn render(&mut self, timestamp: u64) -> Result<String, UuidV7Error> {
        let mut bytes = rand::random::<[u8; 16]>();
        match self.sequence {
            None => {
                self.sequence = Some(
                    (u64::from(bytes[1]) << 32)
                        | (u64::from(bytes[2]) << 24)
                        | (u64::from(bytes[3]) << 16)
                        | (u64::from(bytes[4]) << 8)
                        | u64::from(bytes[5]),
                );
            }
            Some(MAX_SEQUENCE) => return Err(UuidV7Error::SequenceExhausted),
            Some(sequence) => self.sequence = Some(sequence + 1),
        }
        let sequence = self.sequence.expect("sequence just seeded or incremented");

        for index in 0..=5u32 {
            bytes[index as usize] = ((timestamp >> ((5 - index) * 8)) & 0xff) as u8;
        }
        bytes[6] = 0x70 | ((sequence >> 37) & 0x0f) as u8;
        bytes[7] = ((sequence >> 29) & 0xff) as u8;
        bytes[8] = 0x80 | ((sequence >> 23) & 0x3f) as u8;
        bytes[9] = ((sequence >> 15) & 0xff) as u8;
        bytes[10] = ((sequence >> 7) & 0xff) as u8;
        bytes[11] = (((sequence & 0x7f) as u8) << 1) | (bytes[11] & 0x01);

        let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
        Ok(format!(
            "{}-{}-{}-{}-{}",
            &hex[0..8],
            &hex[8..12],
            &hex[12..16],
            &hex[16..20],
            &hex[20..32]
        ))
    }
}

impl Default for UuidV7Generator {
    fn default() -> Self {
        Self::new()
    }
}

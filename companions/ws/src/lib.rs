//! Agent-neutral owned WebSocket client substrate (RFC 6455 + RFC 7692).
//!
//! This Independent Companion owns the neutral client protocol state: strict
//! frame decoding, message assembly, permessage-deflate (PMD) compression,
//! the explicit Close exchange, and the async IO owner ([`io::Driver`]). It
//! takes a caller-completed upgraded stream (any `AsyncRead + AsyncWrite`),
//! so HTTP upgrade policy, credentials, proxies and TLS stay with the caller.
//!
//! Design contracts (see the repository's RDF-24 task record):
//!
//! - One exclusive connection state machine per [`io::Driver`]; borrowed
//!   futures keep no protocol state, so dropping and re-polling resumes the
//!   same owned operation.
//! - Reception validates RFC 6455 rules strictly: reserved opcodes, RSV bits,
//!   fragmentation, control-frame shape, shortest length encodings, and the
//!   64-bit length high bit are rejected; payload storage grows only from
//!   actually received bytes with typed, fallible reservations.
//! - PMD decoding proves the pre-tail `SyncPoint`, injects the four-byte
//!   `00 00 FF FF` tail exactly once, accepts legal `BFINAL` streams with
//!   `ResetKeep` history, and keeps context takeover across messages.
//! - The explicit client Close distinguishes queued/sent/received/complete
//!   states, requires the masked response to be queued by the caller, and
//!   finishes only after both frames, write shutdown and peer EOF; the only
//!   deadline is the caller's.
//! - Cancellation of a borrow never destroys state; explicit cancellation,
//!   abort or dropping the owner releases the transport and compression
//!   resources and suppresses late output.
//!
//! There is no HTTP layer, reconnect, retry, timer, credential or provider
//! policy here. The product bridge lives in the `opi-ai` crate.

pub mod frame;
pub mod io;
pub mod pmd;

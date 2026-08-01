//! Command-execution protocol versions.
//!
//! [`v1`] is the current and only version. Its wire identity is
//! `command-execution-jsonl-v1`. Later versions live in sibling modules (for
//! example a future `v2`) and do not change `v1`.

pub mod v1;

//! Windows restriction posture. Job Objects provide L0 supervision only (see
//! [`crate::process_tree`]); Phase 16 publishes no Windows confinement artifact
//! (design `### Windows`). Windows `run` therefore refuses before target start
//! for the whole of Phase 16, and `doctor` reports `supported == false` with the
//! permanent (not "not yet wired") limitation.

#![forbid(unsafe_code)]

use super::Posture;

/// The Windows posture: permanently unsupported for command restriction in
/// Phase 16 (Job Objects are an L0 mechanism, not a restriction mechanism).
pub(crate) fn posture() -> Posture {
    Posture {
        supported: false,
        mechanisms: Vec::new(),
        limitations: vec![
            "Job Objects provide L0 supervision only; Phase 16 publishes no \
             Windows confinement artifact"
                .to_string(),
        ],
        restriction: None,
    }
}

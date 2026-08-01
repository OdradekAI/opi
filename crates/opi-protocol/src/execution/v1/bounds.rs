//! Frame and cumulative bounds.
//!
//! See the [`v1`](super) module docs for the bound-enforcement table. Defaults
//! are checked for internal consistency at compile time.

/// Wire/size bounds enforced by the codec (per frame) and the session
/// (cumulative, across one execution).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bounds {
    /// Max wire bytes per JSONL line. The decoder's per-stream line-buffer
    /// ceiling and thus the per-connection memory cap. Must be large enough to
    /// hold a maximally base64-inflated output chunk and a maximally
    /// NativeString-amplified configuration plus JSON framing.
    pub max_line_size: usize,
    /// Max decoded bytes per stdout/stderr chunk.
    pub max_decoded_chunk_size: usize,
    /// Max native (pre-encode) bytes of an `initialize` adapter configuration.
    pub max_configuration_size: usize,
    /// Max bytes of a single `diagnostic` message.
    pub max_diagnostics_size: usize,
    /// Max decoded stdout+stderr bytes across one execution.
    pub max_cumulative_output: usize,
}

impl Default for Bounds {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Error indicating the configured [`Bounds`] are internally inconsistent.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BoundsError {
    /// `max_line_size` must be >= `ceil(max_decoded_chunk_size * 4/3) + framing`.
    #[error("max_line_size must be >= ceil(max_decoded_chunk_size * 4/3) + framing")]
    LineTooSmallForChunk,
    /// `max_line_size` must be >= `max_configuration_size * 5 + framing` (because
    /// NativeString can amplify each native byte up to 5x on the wire).
    #[error("max_line_size must be >= max_configuration_size * 5 + framing")]
    LineTooSmallForConfig,
}

impl Bounds {
    /// Defaults chosen so a maximally base64-inflated chunk and a maximally
    /// NativeString-amplified configuration both fit under `max_line_size`.
    pub const DEFAULT: Bounds = Bounds {
        max_line_size: 2 * 1024 * 1024,
        max_decoded_chunk_size: 1024 * 1024,
        max_configuration_size: 256 * 1024,
        max_diagnostics_size: 64 * 1024,
        max_cumulative_output: 256 * 1024 * 1024,
    };

    /// Check the bounds for internal consistency.
    pub const fn validate(self) -> Result<(), BoundsError> {
        if self.max_line_size < self.max_decoded_chunk_size * 4 / 3 + 64 {
            return Err(BoundsError::LineTooSmallForChunk);
        }
        if self.max_line_size < self.max_configuration_size * 5 + 256 {
            return Err(BoundsError::LineTooSmallForConfig);
        }
        let _ = self.max_diagnostics_size;
        let _ = self.max_cumulative_output;
        Ok(())
    }
}

// Compile-time guarantee that the shipped defaults are consistent.
const _DEFAULT_BOUNDS_OK: () = match Bounds::DEFAULT.validate() {
    Ok(()) => (),
    Err(_) => panic!("inconsistent Bounds::DEFAULT"),
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_consistent() {
        assert!(Bounds::DEFAULT.validate().is_ok());
    }

    #[test]
    fn inconsistent_chunk_detected() {
        let bad = Bounds {
            max_line_size: 100,
            max_decoded_chunk_size: 1024,
            max_configuration_size: 16,
            max_diagnostics_size: 64,
            max_cumulative_output: 1024,
        };
        assert_eq!(bad.validate(), Err(BoundsError::LineTooSmallForChunk));
    }
}

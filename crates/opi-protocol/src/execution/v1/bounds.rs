//! Frame and cumulative bounds.
//!
//! See the [`v1`](super) module docs for the bound-enforcement table. Defaults
//! are checked for internal consistency at compile time.

const OUTPUT_CHUNK_FRAMING_RESERVE: usize = 64;

/// Wire/size bounds enforced by the codec (per frame) and the session
/// (cumulative, across one execution).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bounds {
    /// Max JSON data bytes per line, excluding the LF or CRLF delimiter. The
    /// decoder's per-stream line-buffer ceiling and thus the per-connection
    /// memory cap. Must be large enough to hold a maximally base64-inflated
    /// output chunk or serialized adapter configuration plus framing.
    pub max_line_size: usize,
    /// Max decoded bytes per stdout/stderr chunk.
    pub max_decoded_chunk_size: usize,
    /// Max serialized JSON bytes of an `initialize` adapter configuration.
    pub max_configuration_size: usize,
    /// Max bytes of a `diagnostic` entry or optional `failed.message`.
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
    /// `max_line_size` must be >= the padded base64 chunk length plus output
    /// framing: `4 * ceil(max_decoded_chunk_size / 3) + framing`.
    #[error("max_line_size must be >= 4 * ceil(max_decoded_chunk_size / 3) + output framing")]
    LineTooSmallForChunk,
    /// `max_line_size` must be >= `max_configuration_size + framing`.
    #[error("max_line_size must be >= max_configuration_size + framing")]
    LineTooSmallForConfig,
}

impl Bounds {
    /// Defaults chosen so a maximally base64-inflated chunk and serialized
    /// adapter configuration both fit under `max_line_size`.
    pub const DEFAULT: Bounds = Bounds {
        max_line_size: 2 * 1024 * 1024,
        max_decoded_chunk_size: 1024 * 1024,
        max_configuration_size: 256 * 1024,
        max_diagnostics_size: 64 * 1024,
        max_cumulative_output: 256 * 1024 * 1024,
    };

    /// Check the bounds for internal consistency.
    pub const fn validate(self) -> Result<(), BoundsError> {
        let padded_groups = match self.max_decoded_chunk_size.checked_add(2) {
            Some(value) => value,
            None => return Err(BoundsError::LineTooSmallForChunk),
        };
        let padded_base64_len = match (padded_groups / 3).checked_mul(4) {
            Some(value) => value,
            None => return Err(BoundsError::LineTooSmallForChunk),
        };
        let chunk_required = match padded_base64_len.checked_add(OUTPUT_CHUNK_FRAMING_RESERVE) {
            Some(value) => value,
            None => return Err(BoundsError::LineTooSmallForChunk),
        };
        if self.max_line_size < chunk_required {
            return Err(BoundsError::LineTooSmallForChunk);
        }
        let config_required = match self.max_configuration_size.checked_add(256) {
            Some(value) => value,
            None => return Err(BoundsError::LineTooSmallForConfig),
        };
        if self.max_line_size < config_required {
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

    #[test]
    fn overflowing_chunk_requirement_is_rejected() {
        let bad = Bounds {
            max_line_size: usize::MAX,
            max_decoded_chunk_size: usize::MAX,
            max_configuration_size: 0,
            max_diagnostics_size: 0,
            max_cumulative_output: 0,
        };
        assert_eq!(bad.validate(), Err(BoundsError::LineTooSmallForChunk));
    }

    #[test]
    fn overflowing_configuration_requirement_is_rejected() {
        let bad = Bounds {
            max_line_size: usize::MAX,
            max_decoded_chunk_size: 0,
            max_configuration_size: usize::MAX,
            max_diagnostics_size: 0,
            max_cumulative_output: 0,
        };
        assert_eq!(bad.validate(), Err(BoundsError::LineTooSmallForConfig));
    }

    #[test]
    fn configuration_reserve_covers_serialized_bytes() {
        let exact = Bounds {
            max_line_size: 100 + 256,
            max_decoded_chunk_size: 0,
            max_configuration_size: 100,
            max_diagnostics_size: 0,
            max_cumulative_output: 0,
        };
        assert!(exact.validate().is_ok());
        assert_eq!(
            Bounds {
                max_line_size: exact.max_line_size - 1,
                ..exact
            }
            .validate(),
            Err(BoundsError::LineTooSmallForConfig)
        );
    }
}

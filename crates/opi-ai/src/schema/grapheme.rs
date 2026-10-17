//! Grapheme-cluster length bounds ported from the pinned upstream
//! `guard/string.mjs` (TypeBox 1.3.27, tree SHA `ebf39b14…`).
//!
//! The upstream algorithm is a limited cluster counter — combining marks,
//! variation selectors, zero-width joiner sequences and regional-indicator
//! pairs — not a general UAX #29 segmentation, and its string indices are
//! UTF-16 code units. This port keeps both properties: the source's exact
//! acceptance set and its UTF-16 cursor arithmetic.

/// Combining-mark range membership (source `IsCombiningMark`).
fn is_combining_mark(cp: u32) -> bool {
    matches!(cp, 0x0300..=0x036F | 0x1AB0..=0x1AFF | 0x1DC0..=0x1DFF | 0xFE20..=0xFE2F)
}

/// Variation-selector range membership (source `IsVariationSelector`).
fn is_variation_selector(cp: u32) -> bool {
    matches!(cp, 0xFE00..=0xFE0F)
}

/// Regional-indicator range membership (source `IsRegionalIndicator`).
fn is_regional_indicator(cp: u32) -> bool {
    matches!(cp, 0x1F1E6..=0x1F1FF)
}

/// UTF-16 length of a code point (source `CodePointLength`).
fn utf16_len(cp: u32) -> u64 {
    if cp > 0xFFFF { 2 } else { 1 }
}

/// Source `IsGraphemeCodePoint` over a raw UTF-16 code unit.
fn is_grapheme_code_unit(unit: u16) -> bool {
    let unit = u32::from(unit);
    (0x0300..=u32::MAX).contains(&unit)
        && (matches!(unit, 0xD800..=0xDBFF)
            || is_combining_mark(unit)
            || is_variation_selector(unit)
            || unit == 0x200D)
}

/// Cursor over a `&str` viewed as UTF-16 code units with code-point decoding,
/// mirroring the source's `codePointAt`/`charCodeAt` pairing.
struct Utf16Chars {
    units: Vec<u16>,
}

impl Utf16Chars {
    fn new(text: &str) -> Self {
        Self {
            units: text.encode_utf16().collect(),
        }
    }

    fn utf16_len(&self) -> u64 {
        self.units.len() as u64
    }

    /// `charCodeAt(index)` as `Option` (the source receives `NaN` past the
    /// end, which every range test rejects).
    fn unit(&self, index: u64) -> Option<u16> {
        self.units.get(index as usize).copied()
    }

    /// `codePointAt(index)` for a valid index (the source never calls it on
    /// an unpaired trailing surrogate position in these algorithms; a lone
    /// surrogate yields itself, which matches JavaScript).
    fn code_point(&self, index: u64) -> u32 {
        let hi = self.units[index as usize] as u32;
        if (0xD800..=0xDBFF).contains(&hi)
            && let Some(lo) = self.unit(index + 1)
            && (0xDC00..=0xDFFF).contains(&u32::from(lo))
        {
            let lo = u32::from(lo);
            return 0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00);
        }
        hi
    }

    /// Consume combining marks and variation selectors (source
    /// `ConsumeModifiers`), returning the advanced UTF-16 index.
    fn consume_modifiers(&self, mut index: u64) -> u64 {
        while index < self.utf16_len() {
            let point = self.code_point(index);
            if is_combining_mark(point) || is_variation_selector(point) {
                index += utf16_len(point);
            } else {
                break;
            }
        }
        index
    }

    /// Source `NextGraphemeClusterIndex`.
    fn next_cluster_index(&self, cluster_start: u64) -> u64 {
        let start_cp = self.code_point(cluster_start);
        let mut cluster_end = cluster_start + utf16_len(start_cp);
        cluster_end = self.consume_modifiers(cluster_end);
        // Handle multi-ZWJ sequences. The source bound is
        // `clusterEnd < value.length - 1`, a strict comparison against
        // `length - 1` (never underflowing: JavaScript number arithmetic).
        while cluster_end < self.utf16_len().saturating_sub(1)
            && self.code_point(cluster_end) == 0x200D
        {
            let next_cp = self.code_point(cluster_end + 1);
            cluster_end += 1 + utf16_len(next_cp);
            cluster_end = self.consume_modifiers(cluster_end);
        }
        // Handle regional-indicator (flag) pairs.
        if is_regional_indicator(start_cp)
            && cluster_end < self.utf16_len()
            && is_regional_indicator(self.code_point(cluster_end))
        {
            cluster_end += utf16_len(self.code_point(cluster_end));
        }
        cluster_end
    }

    /// Source `IsMinLengthSegmented` — at least `min_length` clusters.
    fn is_min_length_segmented(&self, min_length: f64) -> bool {
        let mut count: u64 = 0;
        let mut index: u64 = 0;
        while index < self.utf16_len() {
            index = self.next_cluster_index(index);
            count += 1;
            if (count as f64) >= min_length {
                return true;
            }
        }
        false
    }

    /// Source `IsMaxLengthSegmented` — at most `max_length` clusters.
    fn is_max_length_segmented(&self, max_length: f64) -> bool {
        let mut count: u64 = 0;
        let mut index: u64 = 0;
        while index < self.utf16_len() {
            index = self.next_cluster_index(index);
            count += 1;
            if (count as f64) > max_length {
                return false;
            }
        }
        true
    }
}

/// Source `IsMinLength(value, minLength)`: true when the string has at least
/// `min_length` grapheme clusters. The guard upstream only requires the limit
/// to be a finite number; fractional and negative limits flow into the same
/// binary64 comparisons.
pub(crate) fn is_min_length(text: &str, min_length: f64) -> bool {
    if min_length == 0.0 {
        return true;
    }
    let chars = Utf16Chars::new(text);
    if (chars.utf16_len() as f64) < min_length {
        return false;
    }
    let mut index: u64 = 0;
    loop {
        if chars.unit(index).is_some_and(is_grapheme_code_unit) {
            return chars.is_min_length_segmented(min_length);
        }
        index += 1;
        if index as f64 >= min_length {
            return true;
        }
    }
}

/// Source `IsMaxLength(value, maxLength)`: true when the string has at most
/// `max_length` grapheme clusters.
pub(crate) fn is_max_length(text: &str, max_length: f64) -> bool {
    let chars = Utf16Chars::new(text);
    if (chars.utf16_len() as f64) <= max_length {
        return true;
    }
    let mut index: u64 = 0;
    loop {
        if chars.unit(index).is_some_and(is_grapheme_code_unit) {
            return chars.is_max_length_segmented(max_length);
        }
        index += 1;
        if index as f64 > max_length {
            return false;
        }
    }
}

/// Source `GraphemeCount`: the number of grapheme clusters in the string.
#[cfg(test)]
pub(crate) fn grapheme_count(text: &str) -> u64 {
    let chars = Utf16Chars::new(text);
    let mut count = 0;
    let mut index: u64 = 0;
    while index < chars.utf16_len() {
        index = chars.next_cluster_index(index);
        count += 1;
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combining_marks_join_the_base_cluster() {
        assert_eq!(grapheme_count("e\u{301}"), 1);
        assert!(is_max_length("e\u{301}", 1.0));
        assert!(!is_min_length("e\u{301}", 2.0));
    }

    #[test]
    fn zwj_sequences_form_one_cluster() {
        assert_eq!(grapheme_count("👩\u{200D}💻"), 1);
        assert!(is_max_length("👩\u{200D}💻", 1.0));
        // multiple ZWJ joins
        assert_eq!(grapheme_count("👩\u{200D}💻\u{200D}🦢"), 1);
    }

    #[test]
    fn regional_indicator_pairs_form_one_cluster() {
        assert_eq!(grapheme_count("🇨🇳"), 1);
        assert_eq!(grapheme_count("🇨🇳🇫🇷"), 2);
    }

    #[test]
    fn hangul_jamo_are_not_joined_by_the_source_algorithm() {
        // The source's limited rules intentionally differ from UAX #29:
        // precomposed Hangul syllables stay separate clusters.
        assert_eq!(grapheme_count("가"), 1);
        assert_eq!(grapheme_count("\u{1100}\u{1161}"), 2);
    }

    #[test]
    fn emoji_modifier_is_not_joined_by_the_source_algorithm() {
        // The upstream's limited rules intentionally differ from UAX #29:
        // emoji modifiers (U+1F3FB..U+1F3FF) are not combining marks or
        // variation selectors, so the cluster count is 2.
        assert_eq!(grapheme_count("👍\u{1F3FD}"), 2);
    }

    #[test]
    fn plain_ascii_counts_per_scalar() {
        assert_eq!(grapheme_count("abc"), 3);
        assert!(is_min_length("abc", 3.0));
        assert!(!is_min_length("abc", 4.0));
        assert!(is_max_length("abc", 3.0));
        assert!(!is_max_length("abc", 2.0));
    }

    #[test]
    fn fast_paths_use_utf16_bounds_like_the_source() {
        // minLength fast path: length is an upper bound on clusters.
        assert!(is_min_length("😀", 1.0));
        assert!(is_min_length("", 0.0));
        assert!(!is_min_length("", 1.0));
        assert!(is_max_length("", 0.0));
        assert!(is_max_length("😀😀", 2.0));
        assert!(!is_max_length("😀😀", 1.0));
    }

    #[test]
    fn fractional_and_negative_limits_keep_source_semantics() {
        // minLength: 2.5 — two clusters give 2 >= 2.5 false, three true.
        assert!(!is_min_length("ab", 2.5));
        assert!(is_min_length("abc", 2.5));
        // negative limits accept via the fast path exactly like the source.
        assert!(is_min_length("a", -1.0));
        assert!(!is_max_length("a", -1.0));
        // maxLength: 2.5 on "ab" — length <= 2.5 -> true directly.
        assert!(is_max_length("ab", 2.5));
    }
}

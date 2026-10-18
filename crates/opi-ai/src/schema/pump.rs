//! Proof operations for a directory word inserted into a URL path.
//!
//! A growing orbit is `P W^n S`, with fixed URL fields outside the path.
//! These operations never infer a proof from a finite number of examples:
//! literal joins are classified by the locked WHATWG parser, and equality is
//! solved by the exact length difference and cancellation of the common word.
//! Failure to prove a property returns `None`; it does not reject a schema.

use url::Url;

/// The same non-empty directory word inserted after one protected prefix.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PathPump {
    prefix: String,
    word: String,
}

/// A constant URL, or the orbit obtained by repeatedly applying a `PathPump`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct UrlOrbit {
    pub(crate) base: Url,
    pub(crate) growing: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Equality {
    Full,
    Path,
}

/// Exactly which simultaneous iterations make two orbits equal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EqualitySchedule {
    Always,
    Never,
    At(usize),
}

/// Equality between a query at iteration `n` and a key inserted at `m`.
/// The caller separately applies its cache's condition on `m` and `n`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CacheEquality {
    Always,
    Never,
    QueryAt(usize),
    KeyAt(usize),
    /// Equality holds exactly when `n = m + cycles`.
    QueryAfter(usize),
    /// Equality holds exactly when `m = n + cycles`.
    KeyAfter(usize),
}

impl UrlOrbit {
    pub(crate) fn fixed(base: Url) -> Self {
        Self {
            base,
            growing: false,
        }
    }
}

impl PathPump {
    /// Find the earliest segment boundary at which a non-empty directory word
    /// was inserted. Every URL field outside the path must remain identical.
    pub(crate) fn from_pair(before: &Url, after: &Url) -> Option<Self> {
        if before.cannot_be_a_base()
            || after.cannot_be_a_base()
            || !same_outside_path(before, after)
        {
            return None;
        }
        let old = before.path();
        let new = after.path();
        let growth = new.len().checked_sub(old.len())?;
        if growth == 0 {
            return None;
        }
        for (index, byte) in old.bytes().enumerate() {
            if byte != b'/' {
                continue;
            }
            let boundary = index + 1;
            let end = boundary.checked_add(growth)?;
            let Some(word) = new.get(boundary..end) else {
                continue;
            };
            if !word.ends_with('/')
                || new.get(..boundary) != old.get(..boundary)
                || new.get(end..) != old.get(boundary..)
            {
                continue;
            }
            let pump = Self {
                prefix: old[..boundary].to_owned(),
                word: word.to_owned(),
            };
            if pump.next(before).as_ref() == Some(after) {
                return Some(pump);
            }
        }
        None
    }

    pub(crate) fn growing(&self, base: Url) -> Option<UrlOrbit> {
        self.next(&base)?;
        Some(UrlOrbit {
            base,
            growing: true,
        })
    }

    #[cfg(test)]
    pub(crate) fn fixed(&self, base: Url) -> UrlOrbit {
        UrlOrbit::fixed(base)
    }

    pub(crate) fn next(&self, base: &Url) -> Option<Url> {
        self.apply_n(base, 1)
    }

    /// Apply the insertion exactly `n` times. The checked allocation is an
    /// arithmetic/capacity check, not an admission work or depth limit.
    pub(crate) fn apply_n(&self, base: &Url, n: usize) -> Option<Url> {
        if base.cannot_be_a_base() || !base.path().starts_with(&self.prefix) {
            return None;
        }
        let extra = self.word.len().checked_mul(n)?;
        let length = base.path().len().checked_add(extra)?;
        let mut path = String::new();
        path.try_reserve_exact(length).ok()?;
        path.push_str(&self.prefix);
        for _ in 0..n {
            path.push_str(&self.word);
        }
        path.push_str(&base.path()[self.prefix.len()..]);
        let mut result = base.clone();
        result.set_path(&path);
        (result.path() == path && same_outside_path(base, &result)).then_some(result)
    }

    pub(crate) fn orbit_at(&self, orbit: &UrlOrbit, n: usize) -> Option<Url> {
        if orbit.growing {
            self.apply_n(&orbit.base, n)
        } else {
            Some(orbit.base.clone())
        }
    }

    /// Prove a literal WHATWG join commutes with every insertion, or resets to
    /// a constant URL. The proof uses a path of distinct sentinel segments.
    /// Its length is `input.len() + 2`: a fixed input cannot contain enough
    /// parent-segment operations to consume all those sentinels. Consequently
    /// no retained sentinel means a path reset; retained sentinels identify
    /// the exact fixed pop-and-append operation. The latter commutes only if
    /// its pop leaves the protected prefix intact.
    ///
    /// The parser itself handles escaped dot segments, backslashes, and the
    /// same-scheme special-URL form such as `https:child`. No textual URI
    /// prefix or `..` heuristic substitutes for those rules.
    pub(crate) fn join_literal(&self, orbit: &UrlOrbit, input: &str) -> Option<UrlOrbit> {
        let joined = orbit.base.join(input).ok()?;
        if !orbit.growing {
            return Some(UrlOrbit::fixed(joined));
        }
        let next = self.next(&orbit.base)?;
        // File joins can inherit a drive even when they reset the path. Keep
        // that parser context in the sentinel URL. Its first segment must be
        // invariant under insertion; otherwise a purported constant reset
        // could inherit a different drive in a later iteration.
        let drive = if orbit.base.scheme() == "file" {
            let drive = file_drive(orbit.base.path());
            if drive != file_drive(next.path()) {
                return None;
            }
            drive
        } else {
            None
        };
        let count = input.len().checked_add(2)?;
        let mut marker = "__opi_admission_path_marker_".to_owned();
        while input.contains(&marker) {
            marker.push('_');
        }
        let markers: Vec<String> = (0..count).map(|index| format!("{marker}{index}")).collect();
        let trailing = orbit.base.path().ends_with('/');
        let mut probe_path = match drive {
            Some(drive) => format!("/{drive}/{}", markers.join("/")),
            None => format!("/{}", markers.join("/")),
        };
        if trailing {
            probe_path.push('/');
        }
        let mut probe = orbit.base.clone();
        probe.set_path(&probe_path);
        if probe.path() != probe_path {
            return None;
        }
        let probe_joined = probe.join(input).ok()?;
        if !same_outside_path(&joined, &probe_joined) {
            return None;
        }
        let output: Vec<&str> = probe_joined.path().split('/').skip(1).collect();
        let marker_start = usize::from(drive.is_some_and(|drive| output.first() == Some(&drive)));
        let retained = markers
            .iter()
            .zip(&output[marker_start..])
            .take_while(|(marker, segment)| marker.as_str() == **segment)
            .count();
        if retained == 0 {
            // A relative input cannot exhaust the sentinel path. A reset may
            // still inherit stable scheme/authority fields, as `/child` does.
            return (joined == probe_joined).then(|| UrlOrbit::fixed(joined));
        }
        if output[marker_start + retained..]
            .iter()
            .any(|segment| segment.contains(&marker))
        {
            return None;
        }
        let popped = count.checked_sub(retained)?;
        let mut original: Vec<&str> = orbit.base.path().split('/').skip(1).collect();
        if trailing {
            original.pop();
        }
        let remaining = original.len().checked_sub(popped)?;
        let protected = self.prefix.bytes().filter(|byte| *byte == b'/').count() - 1;
        if remaining < protected {
            return None;
        }
        original.truncate(remaining);
        original.extend_from_slice(&output[marker_start + retained..]);
        let rewritten = format!("/{}", original.join("/"));
        if rewritten != joined.path() {
            return None;
        }
        self.growing(joined)
    }

    /// Solve equality at the same iteration on both sides. Insertion is
    /// injective for two growing paths. A growing and a fixed path can meet
    /// at at most one iteration, determined by their exact length difference.
    pub(crate) fn equality_schedule(
        &self,
        left: &UrlOrbit,
        right: &UrlOrbit,
        kind: Equality,
    ) -> Option<EqualitySchedule> {
        self.check_orbit(left)?;
        self.check_orbit(right)?;
        match (left.growing, right.growing) {
            (false, false) | (true, true) => Some(if equal(&left.base, &right.base, kind) {
                EqualitySchedule::Always
            } else {
                EqualitySchedule::Never
            }),
            (true, false) => self.growing_fixed_schedule(&left.base, &right.base, kind),
            (false, true) => self.growing_fixed_schedule(&right.base, &left.base, kind),
        }
    }

    /// A boolean is returned only when it is proved for every iteration.
    pub(crate) fn prove_equality(
        &self,
        left: &UrlOrbit,
        right: &UrlOrbit,
        kind: Equality,
    ) -> Option<bool> {
        match self.equality_schedule(left, right, kind)? {
            EqualitySchedule::Always => Some(true),
            EqualitySchedule::Never => Some(false),
            EqualitySchedule::At(_) => None,
        }
    }

    /// Prove equality for every simultaneous iteration when the two URLs use
    /// different insertion words or protected prefixes.
    ///
    /// Unequal growth rates permit at most one equal path length; that one
    /// candidate is checked exactly. For equal positive growth `g`, unequal
    /// starting lengths never meet. Otherwise, once the common length is at
    /// least `max(prefix lengths) + max(suffix lengths) + g`, comparison splits
    /// into a fixed prefix zone, a fixed suffix zone, and a middle containing
    /// a complete `g`-byte period. Their phases do not depend on the iteration,
    /// so equality is constant from that point onward. Checking the preceding
    /// iterations is a finite word-equation proof whose extent is derived
    /// entirely from the two strings; it is not an admission work limit.
    pub(crate) fn prove_mixed_equality(
        left_pump: &Self,
        left: &UrlOrbit,
        right_pump: &Self,
        right: &UrlOrbit,
        kind: Equality,
    ) -> Option<bool> {
        left_pump.check_orbit(left)?;
        right_pump.check_orbit(right)?;
        if !left.growing && !right.growing {
            return Some(equal(&left.base, &right.base, kind));
        }
        if kind == Equality::Full && !same_outside_path(&left.base, &right.base) {
            return Some(false);
        }
        let left_growth = if left.growing {
            left_pump.word.len()
        } else {
            0
        };
        let right_growth = if right.growing {
            right_pump.word.len()
        } else {
            0
        };
        let left_length = left.base.path().len();
        let right_length = right.base.path().len();
        if left_growth != right_growth {
            let (difference, growth) = if left_growth > right_growth {
                let Some(difference) = right_length.checked_sub(left_length) else {
                    return Some(false);
                };
                (difference, left_growth - right_growth)
            } else {
                let Some(difference) = left_length.checked_sub(right_length) else {
                    return Some(false);
                };
                (difference, right_growth - left_growth)
            };
            if difference % growth != 0 {
                return Some(false);
            }
            let iteration = difference / growth;
            return if equal(
                &left_pump.orbit_at(left, iteration)?,
                &right_pump.orbit_at(right, iteration)?,
                kind,
            ) {
                // A single equality is not a boolean invariant of the orbit.
                None
            } else {
                Some(false)
            };
        }
        if left_length != right_length {
            return Some(false);
        }
        // The fixed/fixed case above is the only zero-growth case.
        let growth = left_growth;
        let prefix_length = left_pump.prefix.len().max(right_pump.prefix.len());
        let suffix_length = left_length
            .checked_sub(left_pump.prefix.len())?
            .max(right_length.checked_sub(right_pump.prefix.len())?);
        let stable_length = prefix_length
            .checked_add(suffix_length)?
            .checked_add(growth)?;
        let stable_iteration = if let Some(difference) = stable_length.checked_sub(left_length) {
            difference / growth + usize::from(difference % growth != 0)
        } else {
            0
        };
        let expected = equal(&left.base, &right.base, kind);
        for iteration in 1..=stable_iteration {
            if equal(
                &left_pump.orbit_at(left, iteration)?,
                &right_pump.orbit_at(right, iteration)?,
                kind,
            ) != expected
            {
                return None;
            }
        }
        Some(expected)
    }

    /// Solve cross-cycle cache equality without guessing a warm-up count.
    /// For two growing keys, cancellation leaves exactly one possible lag:
    /// `n - m = (key.path.len() - query.path.len()) / word.len()`.
    /// The actual words must also match at that lag; length alone is not a
    /// proof. `QueryAfter(0)` denotes equal iteration indices.
    pub(crate) fn cache_equality(
        &self,
        query: &UrlOrbit,
        key: &UrlOrbit,
        kind: Equality,
    ) -> Option<CacheEquality> {
        self.check_orbit(query)?;
        self.check_orbit(key)?;
        match (query.growing, key.growing) {
            (false, false) => Some(if equal(&query.base, &key.base, kind) {
                CacheEquality::Always
            } else {
                CacheEquality::Never
            }),
            (true, false) => match self.growing_fixed_schedule(&query.base, &key.base, kind)? {
                EqualitySchedule::At(iteration) => Some(CacheEquality::QueryAt(iteration)),
                EqualitySchedule::Never => Some(CacheEquality::Never),
                EqualitySchedule::Always => None,
            },
            (false, true) => match self.growing_fixed_schedule(&key.base, &query.base, kind)? {
                EqualitySchedule::At(iteration) => Some(CacheEquality::KeyAt(iteration)),
                EqualitySchedule::Never => Some(CacheEquality::Never),
                EqualitySchedule::Always => None,
            },
            (true, true) => {
                if kind == Equality::Full && !same_outside_path(&query.base, &key.base) {
                    return Some(CacheEquality::Never);
                }
                if key.base.path().len() >= query.base.path().len() {
                    let difference = key.base.path().len() - query.base.path().len();
                    if !difference.is_multiple_of(self.word.len()) {
                        return Some(CacheEquality::Never);
                    }
                    let cycles = difference / self.word.len();
                    Some(
                        if equal(&self.apply_n(&query.base, cycles)?, &key.base, kind) {
                            CacheEquality::QueryAfter(cycles)
                        } else {
                            CacheEquality::Never
                        },
                    )
                } else {
                    let difference = query.base.path().len() - key.base.path().len();
                    if !difference.is_multiple_of(self.word.len()) {
                        return Some(CacheEquality::Never);
                    }
                    let cycles = difference / self.word.len();
                    Some(
                        if equal(&query.base, &self.apply_n(&key.base, cycles)?, kind) {
                            CacheEquality::KeyAfter(cycles)
                        } else {
                            CacheEquality::Never
                        },
                    )
                }
            }
        }
    }

    fn check_orbit(&self, orbit: &UrlOrbit) -> Option<()> {
        if orbit.growing {
            self.next(&orbit.base)?;
        }
        Some(())
    }

    fn growing_fixed_schedule(
        &self,
        growing: &Url,
        fixed: &Url,
        kind: Equality,
    ) -> Option<EqualitySchedule> {
        if kind == Equality::Full && !same_outside_path(growing, fixed) {
            return Some(EqualitySchedule::Never);
        }
        let Some(difference) = fixed.path().len().checked_sub(growing.path().len()) else {
            return Some(EqualitySchedule::Never);
        };
        if difference % self.word.len() != 0 {
            return Some(EqualitySchedule::Never);
        }
        let iteration = difference / self.word.len();
        Some(if equal(&self.apply_n(growing, iteration)?, fixed, kind) {
            EqualitySchedule::At(iteration)
        } else {
            EqualitySchedule::Never
        })
    }
}

fn same_outside_path(left: &Url, right: &Url) -> bool {
    let mut left = left.clone();
    let mut right = right.clone();
    left.set_path("/");
    right.set_path("/");
    left == right
}

/// The locked parser's normalized Windows drive-letter condition. Existing
/// URL paths have already normalized the alternative `|` spelling to `:`.
fn file_drive(path: &str) -> Option<&str> {
    let segment = path.split('/').nth(1)?;
    let bytes = segment.as_bytes();
    (bytes.len() == 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':').then_some(segment)
}

fn equal(left: &Url, right: &Url, kind: Equality) -> bool {
    match kind {
        Equality::Full => left == right,
        Equality::Path => left.path() == right.path(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(text: &str) -> Url {
        Url::parse(text).unwrap()
    }

    fn root_pump() -> PathPump {
        PathPump::from_pair(
            &url("https://offline.invalid/a/"),
            &url("https://offline.invalid/a/a/"),
        )
        .unwrap()
    }

    fn assert_join_commutes(pump: &PathPump, base: &str, input: &str, growing: bool) {
        let orbit = pump.growing(url(base)).unwrap();
        let joined = pump.join_literal(&orbit, input).expect("join proof");
        assert_eq!(joined.growing, growing, "{input}");
        for iteration in 0..6 {
            let expected = pump
                .orbit_at(&orbit, iteration)
                .unwrap()
                .join(input)
                .unwrap();
            assert_eq!(
                pump.orbit_at(&joined, iteration).unwrap(),
                expected,
                "{input}, {iteration}"
            );
        }
    }

    #[test]
    fn positive_directory_word_uses_the_earliest_segment_boundary() {
        let pump = root_pump();
        assert_eq!(pump.prefix, "/");
        assert_eq!(pump.word, "a/");
        assert_eq!(
            pump.apply_n(&url("https://offline.invalid/a/leaf?q=1#x"), 3),
            Some(url("https://offline.invalid/a/a/a/a/leaf?q=1#x"))
        );
        assert!(
            PathPump::from_pair(
                &url("https://offline.invalid/a"),
                &url("https://offline.invalid/ab")
            )
            .is_none()
        );
        assert!(
            PathPump::from_pair(
                &url("https://offline.invalid/a/"),
                &url("https://other.invalid/a/a/")
            )
            .is_none()
        );
    }

    #[test]
    fn fixed_resource_equality_is_delayed_and_then_impossible() {
        let pump = root_pump();
        let growing = pump.growing(url("https://offline.invalid/a/leaf")).unwrap();
        let fixed = pump.fixed(url("https://offline.invalid/a/a/a/a/a/leaf"));
        assert_eq!(
            pump.equality_schedule(&growing, &fixed, Equality::Full),
            Some(EqualitySchedule::At(4))
        );
        assert_eq!(pump.prove_equality(&growing, &fixed, Equality::Full), None);
        let later = pump.growing(pump.orbit_at(&growing, 5).unwrap()).unwrap();
        assert_eq!(
            pump.prove_equality(&later, &fixed, Equality::Full),
            Some(false)
        );
        let other_host = pump.fixed(url("https://other.invalid/a/a/a/a/a/leaf"));
        assert_eq!(
            pump.prove_equality(&growing, &other_host, Equality::Full),
            Some(false)
        );
        assert_eq!(
            pump.equality_schedule(&growing, &other_host, Equality::Path),
            Some(EqualitySchedule::At(4))
        );
        let other_word = pump.fixed(url("https://offline.invalid/a/b/leaf"));
        assert_eq!(
            pump.prove_equality(&growing, &other_word, Equality::Path),
            Some(false)
        );
    }

    #[test]
    fn parent_segments_use_whatwg_escaped_dot_and_backslash_rules() {
        let pump = PathPump::from_pair(
            &url("https://offline.invalid/a/b/c/"),
            &url("https://offline.invalid/a/p/b/c/"),
        )
        .unwrap();
        assert_eq!(pump.prefix, "/a/");
        for input in ["../leaf", "%2e%2e/leaf", ".%2E/leaf", "..\\leaf"] {
            assert_join_commutes(&pump, "https://offline.invalid/a/b/c/", input, true);
        }
        let orbit = pump.growing(url("https://offline.invalid/a/b/c/")).unwrap();
        for input in [
            "../../../leaf",
            "%2e%2e/%2E%2e/%2e%2E/leaf",
            "..\\..\\..\\leaf",
        ] {
            assert!(pump.join_literal(&orbit, input).is_none(), "{input}");
        }
    }

    #[test]
    fn same_scheme_relative_urls_are_distinguished_from_resets() {
        let pump = root_pump();
        for input in [
            "leaf",
            "https:leaf",
            "?q=2",
            "#anchor",
            "",
            "__opi_admission_path_marker_0",
            "..%2F..",
        ] {
            assert_join_commutes(&pump, "https://offline.invalid/a/file?q=1#old", input, true);
        }
        for input in [
            "https://other.invalid/reset",
            "https://offline.invalid/a/file",
            "/reset",
            "//other.invalid/reset",
            "https:/reset",
        ] {
            assert_join_commutes(
                &pump,
                "https://offline.invalid/a/file?q=1#old",
                input,
                false,
            );
        }
    }

    #[test]
    fn joins_preserve_empty_segments_and_can_consume_the_original_tail() {
        let pump = root_pump();
        assert_join_commutes(&pump, "https://offline.invalid/a/", "../leaf", true);
        assert_join_commutes(&pump, "https://offline.invalid/a//b/", "../leaf", true);
        let short = pump.growing(url("https://offline.invalid/a/")).unwrap();
        assert!(pump.join_literal(&short, "../../leaf").is_none());
    }

    #[test]
    fn constant_orbits_remain_constant_on_join() {
        let pump = root_pump();
        let fixed = pump.fixed(url("file:///C:/root/file"));
        assert_eq!(
            pump.join_literal(&fixed, "../leaf"),
            Some(pump.fixed(url("file:///C:/leaf")))
        );
        assert!(pump.join_literal(&fixed, "http://[").is_none());
    }

    #[test]
    fn hierarchical_custom_scheme_joins_have_the_same_path_rewrite_proof() {
        for base in ["custom://host/a/b/", "custom:/a/b/"] {
            let before = url(base);
            let after = before.join("b/").unwrap();
            let pump = PathPump::from_pair(&before, &after).unwrap();
            for input in ["b/", "leaf", "../leaf", "%2e%2e/leaf", "..\\leaf"] {
                assert_join_commutes(&pump, base, input, true);
            }
            for input in ["/reset", "custom:opaque", "https://offline.invalid/reset"] {
                assert_join_commutes(&pump, base, input, false);
            }
        }
    }

    #[test]
    fn file_joins_preserve_the_drive_context_of_path_resets() {
        for base in ["file:///a/b/", "file:///C:/a/b/", "file://server/a/b/"] {
            let before = url(base);
            let after = before.join("b/").unwrap();
            let pump = PathPump::from_pair(&before, &after).unwrap();
            for input in [
                "b/",
                "leaf",
                "../leaf",
                "%2e%2e/leaf",
                "..\\leaf",
                "file:leaf",
            ] {
                assert_join_commutes(&pump, base, input, true);
            }
            for input in [
                "/reset",
                "file:/reset",
                "file:///D:/reset",
                "https://offline.invalid/reset",
            ] {
                assert_join_commutes(&pump, base, input, false);
            }
        }
    }

    #[test]
    fn file_drive_context_cannot_be_consumed_or_changed() {
        let pump = PathPump {
            prefix: "/C:/".to_owned(),
            word: "b/".to_owned(),
        };
        let orbit = pump.growing(url("file:///C:/leaf/")).unwrap();
        assert!(pump.join_literal(&orbit, "../../result").is_none());
        let changing_drive = PathPump {
            prefix: "/".to_owned(),
            word: "D:/".to_owned(),
        };
        let orbit = changing_drive.growing(url("file:///C:/a/")).unwrap();
        assert!(changing_drive.join_literal(&orbit, "/reset").is_none());
    }

    #[test]
    fn cache_keys_have_exact_cross_cycle_lags() {
        let pump = root_pump();
        let short = pump.growing(url("https://offline.invalid/a/leaf")).unwrap();
        let long = pump
            .growing(url("https://offline.invalid/a/a/a/a/leaf"))
            .unwrap();
        assert_eq!(
            pump.cache_equality(&short, &long, Equality::Full),
            Some(CacheEquality::QueryAfter(3))
        );
        assert_eq!(
            pump.cache_equality(&long, &short, Equality::Full),
            Some(CacheEquality::KeyAfter(3))
        );
        assert_eq!(
            pump.cache_equality(&short, &short, Equality::Full),
            Some(CacheEquality::QueryAfter(0))
        );
        let fixed = pump.fixed(long.base.clone());
        assert_eq!(
            pump.cache_equality(&short, &fixed, Equality::Full),
            Some(CacheEquality::QueryAt(3))
        );
        assert_eq!(
            pump.cache_equality(&fixed, &short, Equality::Full),
            Some(CacheEquality::KeyAt(3))
        );
        let wrong_word = pump
            .growing(url("https://offline.invalid/a/b/a/a/leaf"))
            .unwrap();
        assert_eq!(
            pump.cache_equality(&short, &wrong_word, Equality::Full),
            Some(CacheEquality::Never)
        );
        assert_eq!(
            pump.cache_equality(&fixed, &fixed, Equality::Full),
            Some(CacheEquality::Always)
        );
    }

    #[test]
    fn growing_path_equality_is_injective() {
        let pump = root_pump();
        let left = pump.growing(url("https://offline.invalid/a/leaf")).unwrap();
        let right = pump
            .growing(url("https://offline.invalid/a/other"))
            .unwrap();
        assert_eq!(
            pump.prove_equality(&left, &left, Equality::Full),
            Some(true)
        );
        assert_eq!(
            pump.prove_equality(&left, &right, Equality::Full),
            Some(false)
        );
    }

    #[test]
    fn mixed_growth_detects_a_single_late_equality() {
        let slow = root_pump();
        let fast = PathPump::from_pair(
            &url("https://offline.invalid/a/"),
            &url("https://offline.invalid/a/a/a/"),
        )
        .unwrap();
        let left = slow
            .growing(url("https://offline.invalid/a/a/a/leaf"))
            .unwrap();
        let right = fast.growing(url("https://offline.invalid/a/leaf")).unwrap();
        assert_ne!(left.base, right.base);
        assert_eq!(slow.orbit_at(&left, 2), fast.orbit_at(&right, 2));
        assert_ne!(slow.orbit_at(&left, 3), fast.orbit_at(&right, 3));
        assert_eq!(
            PathPump::prove_mixed_equality(&slow, &left, &fast, &right, Equality::Full),
            None
        );
        let wrong_suffix = fast.growing(url("https://offline.invalid/a/path")).unwrap();
        assert_eq!(
            PathPump::prove_mixed_equality(&slow, &left, &fast, &wrong_suffix, Equality::Full),
            Some(false)
        );
    }

    #[test]
    fn mixed_pumps_preserve_conjugate_words_at_all_iterations() {
        // (ab)^n a = a(ba)^n. Use distinct protected prefixes to exercise
        // phase cancellation rather than identical pump representations.
        let left_pump = PathPump {
            prefix: "/".to_owned(),
            word: "a/b/".to_owned(),
        };
        let right_pump = PathPump {
            prefix: "/a/b/a/b/a/".to_owned(),
            word: "b/a/".to_owned(),
        };
        let base = url("https://offline.invalid/a/b/a/b/a/");
        let left = left_pump.growing(base.clone()).unwrap();
        let right = right_pump.growing(base).unwrap();
        assert_eq!(
            PathPump::prove_mixed_equality(&left_pump, &left, &right_pump, &right, Equality::Full,),
            Some(true)
        );
        let other_host = right_pump
            .growing(url("https://other.invalid/a/b/a/b/a/"))
            .unwrap();
        assert_eq!(
            PathPump::prove_mixed_equality(
                &left_pump,
                &left,
                &right_pump,
                &other_host,
                Equality::Full,
            ),
            Some(false)
        );
        assert_eq!(
            PathPump::prove_mixed_equality(
                &left_pump,
                &left,
                &right_pump,
                &other_host,
                Equality::Path,
            ),
            Some(true)
        );
    }

    #[test]
    fn mixed_equal_growth_with_different_words_can_never_match() {
        let left_pump = root_pump();
        let right_pump = PathPump::from_pair(
            &url("https://offline.invalid/b/"),
            &url("https://offline.invalid/b/b/"),
        )
        .unwrap();
        let left = left_pump
            .growing(url("https://offline.invalid/x/"))
            .unwrap();
        let right = right_pump
            .growing(url("https://offline.invalid/y/"))
            .unwrap();
        assert_eq!(
            PathPump::prove_mixed_equality(&left_pump, &left, &right_pump, &right, Equality::Path,),
            Some(false)
        );
        let same_start = right_pump.growing(left.base.clone()).unwrap();
        assert_eq!(
            PathPump::prove_mixed_equality(
                &left_pump,
                &left,
                &right_pump,
                &same_start,
                Equality::Path,
            ),
            None
        );
    }

    #[test]
    fn mixed_comparison_keeps_fixed_orbits_constant() {
        let left_pump = root_pump();
        let right_pump = PathPump::from_pair(
            &url("https://offline.invalid/b/"),
            &url("https://offline.invalid/b/b/"),
        )
        .unwrap();
        let left = left_pump
            .growing(url("https://offline.invalid/a/leaf"))
            .unwrap();
        let right = right_pump.fixed(url("https://offline.invalid/a/a/a/leaf"));
        assert_eq!(
            PathPump::prove_mixed_equality(&left_pump, &left, &right_pump, &right, Equality::Full,),
            None
        );
        let later = left_pump
            .growing(left_pump.orbit_at(&left, 3).unwrap())
            .unwrap();
        assert_eq!(
            PathPump::prove_mixed_equality(&left_pump, &later, &right_pump, &right, Equality::Full,),
            Some(false)
        );
        assert_eq!(
            PathPump::prove_mixed_equality(&left_pump, &right, &right_pump, &right, Equality::Full,),
            Some(true)
        );
    }
}

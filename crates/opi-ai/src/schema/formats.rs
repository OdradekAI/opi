//! The 21 fixed default string formats ported from the pinned TypeBox 1.3.27
//! `build/format/*.mjs` (tree SHA `ebf39b14…`). Each format keeps its
//! upstream algorithm — regex acceptance sets are the extracted verbatim
//! patterns compiled by the dialect's ECMAScript regex engine, `url`/`iri`
//! delegate to WHATWG URL parsing, and the hostname family implements the
//! upstream IDNA label rules. Unknown formats are inert (`true`) exactly like
//! the upstream default registry; there is no mutable global registry.

use std::sync::OnceLock;

use regress::Regex;
use serde_json::Value;
use unicode_normalization::UnicodeNormalization;

use super::format_patterns as p;
use super::punycode;

/// Compile-once cache for the fixed pattern set. The map values are leaked
/// on first use; the set of (pattern, flags) pairs is a compile-time-fixed
/// constant set.
fn regex(pattern: &'static str, flags: &'static str) -> &'static Regex {
    static CACHE: OnceLock<
        std::sync::Mutex<std::collections::HashMap<(&'static str, &'static str), &'static Regex>>,
    > = OnceLock::new();
    let cache = CACHE.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()));
    let mut guard = cache.lock().unwrap();
    if let Some(existing) = guard.get(&(pattern, flags)) {
        return existing;
    }
    let compiled = Regex::with_flags(pattern, flags)
        .unwrap_or_else(|e| panic!("static format pattern must compile: {pattern}: {e}"));
    let leaked: &'static Regex = Box::leak(Box::new(compiled));
    guard.insert((pattern, flags), leaked);
    leaked
}

fn test(pattern: &'static str, flags: &'static str, value: &str) -> bool {
    regex(pattern, flags).find(value).is_some()
}

/// JS UTF-16 string length.
fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

// ------------------------------------------------------------------
// date / time / date-time
// ------------------------------------------------------------------

/// Source `IsDate`: RFC 3339 full-date with real calendar days.
fn is_date(value: &str) -> bool {
    let Some(m) = regex(p::P_DATE, p::P_DATE_FLAGS).find(value) else {
        return false;
    };
    let group = |idx: usize| m.group(idx).and_then(|range| value.get(range));
    let year: i64 = group(1).and_then(|g| g.parse().ok()).unwrap_or(0);
    let month: u32 = group(2).and_then(|g| g.parse().ok()).unwrap_or(0);
    let day: u32 = group(3).and_then(|g| g.parse().ok()).unwrap_or(0);
    if !(1..=12).contains(&month) || day == 0 {
        return false;
    }
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => return false,
    };
    day <= days
}

fn is_leap_year(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

/// Source `IsTime` with the default `strictTimeZone = true`, including the
/// leap-second normalization to 23:59:60 UTC.
fn is_time(value: &str) -> bool {
    let Some(m) = regex(p::P_TIME, p::P_TIME_FLAGS).find(value) else {
        return false;
    };
    let group = |idx: usize| {
        m.group(idx)
            .and_then(|range| value.get(range))
            .filter(|g| !g.is_empty())
    };
    let zulu = group(4);
    let tz_sign = group(5);
    if zulu.is_none() && tz_sign.is_none() {
        return false;
    }
    let hr: i64 = group(1).and_then(|g| g.parse().ok()).unwrap_or(-1);
    let min: i64 = group(2).and_then(|g| g.parse().ok()).unwrap_or(-1);
    let sec: i64 = group(3).and_then(|g| g.parse().ok()).unwrap_or(-1);
    if hr > 23 || min > 59 || sec > 60 {
        return false;
    }
    let tz_h: i64 = group(6).and_then(|g| g.parse().ok()).unwrap_or(0);
    let tz_m: i64 = group(7).and_then(|g| g.parse().ok()).unwrap_or(0);
    if tz_sign.is_some() && (tz_h > 23 || tz_m > 59) {
        return false;
    }
    if sec < 60 {
        return true;
    }
    // Leap second: must normalize to 23:59:60 UTC.
    let sign: i64 = if tz_sign == Some("-") { -1 } else { 1 };
    let total_utc_min = (hr * 60 + min) - sign * (tz_h * 60 + tz_m);
    (total_utc_min % 1440 + 1440) % 1440 == 1439
}

/// Source `IsDateTime`: `value.split(/T/i)` must produce exactly two parts.
fn is_date_time(value: &str) -> bool {
    let count = value.chars().filter(|c| *c == 'T' || *c == 't').count();
    if count != 1 {
        return false;
    }
    let (date, time) = value.split_once(['T', 't']).unwrap();
    is_date(date) && is_time(time)
}

// ------------------------------------------------------------------
// hostname / idn-hostname / idn-email
// ------------------------------------------------------------------

/// Source `label/ascii.mjs`: RFC 5891 §4.2.3.1 LDH label.
fn is_ascii_label(label: &str) -> bool {
    test(
        p::IDNA_RULE_HYPHEN_PLACEMENT,
        p::IDNA_RULE_HYPHEN_PLACEMENT_FLAGS,
        label,
    ) && test(
        p::IDNA_RULE_NOT_RESERVED_ACE,
        p::IDNA_RULE_NOT_RESERVED_ACE_FLAGS,
        label,
    ) && test(p::IDNA_ASCII_LDH, p::IDNA_ASCII_LDH_FLAGS, label)
}

fn is_ace_prefixed(label: &str) -> bool {
    label.to_lowercase().starts_with("xn--")
}

/// Source `label/puny.mjs`: ACE-prefixed label with a decodable, non-ASCII,
/// rule-conforming Unicode expansion.
fn is_puny_label(label: &str) -> bool {
    if !is_ace_prefixed(label) {
        return false;
    }
    let body = label[4..].to_lowercase();
    // A payload that is only a hyphen or starts with the delimiter ("-9uc")
    // has zero basic characters and is malformed.
    if body.rfind('-') == Some(0) {
        return false;
    }
    match punycode::decode(&body) {
        Ok(decoded) => {
            test(p::IDNA_NON_ASCII, p::IDNA_NON_ASCII_FLAGS, &decoded) && is_unicode_label(&decoded)
        }
        Err(()) => false,
    }
}

fn is_valid_label_length(label: &str) -> bool {
    !label.is_empty() && utf16_len(label) <= 63
}

fn is_ascii_hostname_label(label: &str) -> bool {
    is_valid_label_length(label) && (is_puny_label(label) || is_ascii_label(label))
}

/// Source `format/hostname.mjs`.
fn is_hostname(value: &str) -> bool {
    if value.is_empty() || utf16_len(value) > 253 {
        return false;
    }
    if value.ends_with('.') {
        return false;
    }
    value.split('.').all(is_ascii_hostname_label)
}

/// Source `idn-hostname.mjs` `NormalizeHostname`: fullwidth folding, NFC,
/// ignored-mapping removal and full-stop folding, in the upstream order.
fn normalize_hostname(value: &str) -> String {
    let fullwidth: String = value
        .chars()
        .map(|c| {
            let cp = c as u32;
            if (0xFF01..=0xFF5E).contains(&cp) {
                char::from_u32(cp - 0xFEE0).unwrap_or(c)
            } else {
                c
            }
        })
        .collect();
    let nfc: String = fullwidth.nfc().collect();
    let ignored: String = nfc
        .chars()
        .filter(|c| {
            let cp = u32::from(*c);
            !matches!(cp, 0x00AD | 0x034F | 0x180B..=0x180D | 0x200B | 0xFE00..=0xFE0F | 0xE0100..=0xE01EF)
        })
        .collect();
    ignored
        .chars()
        .map(|c| match c as u32 {
            0x002E | 0x3002 | 0xFF0E | 0xFF61 => '.',
            _ => c,
        })
        .collect()
}

/// Source `label/unicode.mjs` `IsUnicodeLabel`: the upstream RFC 5891/5892
/// label rules with their documented partial contextJ handling.
fn is_unicode_label(value: &str) -> bool {
    let chars: Vec<char> = value.chars().collect();
    // RFC 5890 §2.3.2.1: encoded A-label length bound.
    let exceeds = test(p::IDNA_NON_ASCII, p::IDNA_NON_ASCII_FLAGS, value)
        && punycode::encode(value).chars().count() + 4 > 63;
    if exceeds {
        return false;
    }
    // RFC 5893 bidi rule when the label itself is RTL.
    if has_right_to_left_characters(value) && !satisfies_bidi_rule(value) {
        return false;
    }
    let length = chars.len();
    // RFC 5891 §4.2.3.2 hyphen rules (codepoint positions).
    let invalid_hyphens = chars.first() == Some(&'-')
        || chars.last() == Some(&'-')
        || (chars.get(2) == Some(&'-') && chars.get(3) == Some(&'-'));
    if invalid_hyphens {
        return false;
    }
    // Must not begin with a combining mark.
    if chars.first().is_some_and(|c| {
        test(
            p::IDNA_COMBINING_MARK,
            p::IDNA_COMBINING_MARK_FLAGS,
            &c.to_string(),
        )
    }) {
        return false;
    }
    let mut has_japanese = false;
    for (i, &c) in chars.iter().enumerate() {
        let cp = c as u32;
        let char_str = c.to_string();
        // DISALLOWED exceptions
        if test(
            p::IDNA_RFC5892_DISALLOWED,
            p::IDNA_RFC5892_DISALLOWED_FLAGS,
            &char_str,
        ) {
            return false;
        }
        // General category (PVALID) enforcement
        if !test(
            p::IDNA_PERMITTED_CATEGORY,
            p::IDNA_PERMITTED_CATEGORY_FLAGS,
            &char_str,
        ) {
            return false;
        }
        if test(
            p::IDNA_SCRIPT_JAPANESE,
            p::IDNA_SCRIPT_JAPANESE_FLAGS,
            &char_str,
        ) {
            has_japanese = true;
        }
        // CONTEXTO / CONTEXTJ neighbor rules
        let prev = if i > 0 { Some(chars[i - 1]) } else { None };
        let next = if i + 1 < length {
            Some(chars[i + 1])
        } else {
            None
        };
        match cp {
            0x00B7 => {
                // MIDDLE DOT must sit between two 'l' characters.
                if prev != Some('l') || next != Some('l') {
                    return false;
                }
            }
            0x0375 => {
                // Greek KERAIA must be followed by a Greek character.
                let Some(next_char) = next else { return false };
                if !test(
                    p::IDNA_SCRIPT_GREEK,
                    p::IDNA_SCRIPT_GREEK_FLAGS,
                    &next_char.to_string(),
                ) {
                    return false;
                }
            }
            0x05F3 | 0x05F4 => {
                // Hebrew GERESH/GERSHAYIM must follow a Hebrew character.
                let Some(prev_char) = prev else { return false };
                if !test(
                    p::IDNA_SCRIPT_HEBREW,
                    p::IDNA_SCRIPT_HEBREW_FLAGS,
                    &prev_char.to_string(),
                ) {
                    return false;
                }
            }
            0x200C => {
                // ZWNJ: the upstream only rejects when preceded by ASCII
                // (U+0000..U+007F) without a virama; full validation needs a
                // joining-type table the upstream does not carry.
                let Some(prev_char) = prev else { return false };
                let prev_is_ascii = (prev_char as u32) < 0x0080;
                if prev_is_ascii
                    && !test(p::IDNA_VIRAMA, p::IDNA_VIRAMA_FLAGS, &prev_char.to_string())
                {
                    return false;
                }
            }
            0x200D => {
                // ZWJ must follow a virama.
                let Some(prev_char) = prev else { return false };
                if !test(p::IDNA_VIRAMA, p::IDNA_VIRAMA_FLAGS, &prev_char.to_string()) {
                    return false;
                }
            }
            0x30FB => {
                // KATAKANA MIDDLE DOT: checked after the loop via hasJapanese.
            }
            _ => {}
        }
    }
    // RFC 5892 Appendix A.7: katakana middle dot requires a Japanese char.
    if chars.contains(&'\u{30FB}') && !has_japanese {
        return false;
    }
    true
}

/// Source `format/bidi.mjs` `GetBidiClass` (the upstream's partial class map).
fn bidi_class(c: char) -> &'static str {
    let s = c.to_string();
    if test(p::IDNA_EUROPEAN_NUMBER, p::IDNA_EUROPEAN_NUMBER_FLAGS, &s) {
        "EN"
    } else if test(
        p::IDNA_ARABIC_INDIC_DIGIT,
        p::IDNA_ARABIC_INDIC_DIGIT_FLAGS,
        &s,
    ) {
        "AN"
    } else if test(p::IDNA_MARK_NONSPACING, p::IDNA_MARK_NONSPACING_FLAGS, &s) {
        "NSM"
    } else if test(p::IDNA_SCRIPT_HEBREW, p::IDNA_SCRIPT_HEBREW_FLAGS, &s) {
        "R"
    } else if test(
        p::IDNA_SCRIPT_ARABIC_LETTER,
        p::IDNA_SCRIPT_ARABIC_LETTER_FLAGS,
        &s,
    ) {
        "AL"
    } else if test(p::IDNA_LETTER, p::IDNA_LETTER_FLAGS, &s) {
        "L"
    } else {
        "ON"
    }
}

fn has_right_to_left_characters(value: &str) -> bool {
    value
        .chars()
        .any(|c| matches!(bidi_class(c), "R" | "AL" | "AN"))
}

/// Source `format/bidi.mjs` `SatisfiesBidiRule` with the upstream's enabled
/// subset (conditions 1, 2, 5 and the RTL EN/AN mixing rule).
fn satisfies_bidi_rule(value: &str) -> bool {
    let mut is_rtl = false;
    let mut saw_en = false;
    let mut saw_an = false;
    let mut is_first = true;
    for c in value.chars() {
        let class = bidi_class(c);
        if is_first {
            if !matches!(class, "L" | "R" | "AL") {
                return false;
            }
            is_rtl = matches!(class, "R" | "AL");
            is_first = false;
        }
        let allowed = if is_rtl {
            matches!(
                class,
                "R" | "AL" | "AN" | "EN" | "ES" | "CS" | "ET" | "ON" | "BN" | "NSM"
            )
        } else {
            matches!(class, "L" | "EN" | "ES" | "CS" | "ET" | "ON" | "BN" | "NSM")
        };
        if !allowed {
            return false;
        }
        if class == "EN" {
            saw_en = true;
        } else if class == "AN" {
            saw_an = true;
        }
    }
    !(is_rtl && saw_en && saw_an)
}

/// Source `idn-hostname.mjs` `HasBidiChars`: decode punycode labels first.
fn has_bidi_chars(value: &str) -> bool {
    if is_ace_prefixed(value) {
        let body = value[4..].to_lowercase();
        match punycode::decode(&body) {
            Ok(decoded) => has_right_to_left_characters(&decoded),
            Err(()) => false,
        }
    } else {
        has_right_to_left_characters(value)
    }
}

fn is_unicode_hostname_label(label: &str) -> bool {
    is_valid_label_length(label) && (is_puny_label(label) || is_unicode_label(label))
}

/// Source `format/idn_hostname.mjs` `IsIdnHostname`.
fn is_idn_hostname(value: &str) -> bool {
    if value.is_empty() || value.contains(' ') {
        return false;
    }
    let normalized = normalize_hostname(value);
    if utf16_len(&normalized) > 253 {
        return false;
    }
    let labels: Vec<&str> = normalized.split('.').collect();
    let has_bidi = labels.iter().any(|label| has_bidi_chars(label));
    labels
        .iter()
        .all(|label| is_unicode_hostname_label(label) && (!has_bidi || satisfies_bidi_rule(label)))
}

// ------------------------------------------------------------------
// URL-family formats
// ------------------------------------------------------------------

/// WHATWG `URL.canParse(value)` (no base).
fn can_parse(value: &str) -> bool {
    url::Url::parse(value).is_ok()
}

/// WHATWG `URL.canParse(value, base)`.
fn can_parse_with_base(value: &str, base: &str) -> bool {
    url::Url::parse(base)
        .ok()
        .and_then(|base| url::Url::options().base_url(Some(&base)).parse(value).ok())
        .is_some()
}

/// Source `format/iri.mjs` `IsIri`: character/percent guards, IPvFuture
/// substitution bounded by the 2048 threshold, then `URL.canParse`.
fn is_iri(value: &str) -> bool {
    if test(p::P_IRI_INVALID_CHARS, p::P_IRI_INVALID_CHARS_FLAGS, value) {
        return false;
    }
    if test(
        p::P_IRI_INVALID_PERCENT,
        p::P_IRI_INVALID_PERCENT_FLAGS,
        value,
    ) {
        return false;
    }
    let narrowed = narrow_ipv_future(value);
    can_parse(&narrowed)
}

/// Source `NarrowIpvFuture`: replace the first IPvFuture literal with the
/// IPv6 loopback (only for inputs under 2048 characters).
fn narrow_ipv_future(value: &str) -> String {
    if value.chars().count() < 2048
        && let Some(m) = regex(p::P_IPV_FUTURE, p::P_IPV_FUTURE_FLAGS).find(value)
    {
        let range = m.range();
        let mut out = String::with_capacity(value.len());
        out.push_str(&value[..range.start]);
        out.push_str("[::1]");
        out.push_str(&value[range.end..]);
        return out;
    }
    value.to_string()
}

/// Source `format/iri_reference.mjs` `IsIriReference`.
fn is_iri_reference(value: &str) -> bool {
    !test(
        p::P_IRIREF_INVALID_CHARS,
        p::P_IRIREF_INVALID_CHARS_FLAGS,
        value,
    ) && !test(
        p::P_IRIREF_MALFORMED_SCHEME,
        p::P_IRIREF_MALFORMED_SCHEME_FLAGS,
        value,
    ) && can_parse_with_base(value, "http://example.com")
}

// ------------------------------------------------------------------
// Registry dispatch
// ------------------------------------------------------------------

/// Source `format/_registry.mjs` `Format.Test(name, value)` with the fixed
/// default registry: unknown formats are inert (`true`).
pub(crate) fn format_test(name: &str, value: &Value) -> bool {
    let Some(text) = value.as_str() else {
        return true;
    };
    match name {
        "date-time" => is_date_time(text),
        "date" => is_date(text),
        "duration" => test(p::P_DURATION, p::P_DURATION_FLAGS, text),
        "email" => test(p::P_EMAIL, p::P_EMAIL_FLAGS, text),
        "hostname" => is_hostname(text),
        "idn-email" => {
            let nfc: String = text.nfc().collect();
            test(p::P_IDN_EMAIL, p::P_IDN_EMAIL_FLAGS, &nfc)
        }
        "idn-hostname" => is_idn_hostname(text),
        "ipv4" => test(p::P_IPV4, p::P_IPV4_FLAGS, text),
        "ipv6" => test(p::P_IPV6, p::P_IPV6_FLAGS, text),
        "iri-reference" => is_iri_reference(text),
        "iri" => is_iri(text),
        "json-pointer-uri-fragment" => test(
            p::P_JSON_POINTER_URI_FRAGMENT,
            p::P_JSON_POINTER_URI_FRAGMENT_FLAGS,
            text,
        ),
        "json-pointer" => test(p::P_JSON_POINTER, p::P_JSON_POINTER_FLAGS, text),
        "regex" => Regex::with_flags(text, "u").is_ok(),
        "relative-json-pointer" => test(
            p::P_RELATIVE_JSON_POINTER,
            p::P_RELATIVE_JSON_POINTER_FLAGS,
            text,
        ),
        "time" => is_time(text),
        "uri-reference" => test(p::P_URI_REFERENCE, p::P_URI_REFERENCE_FLAGS, text),
        "uri-template" => test(p::P_URI_TEMPLATE, p::P_URI_TEMPLATE_FLAGS, text),
        "uri" => test(p::P_URI, p::P_URI_FLAGS, text),
        "url" => can_parse(text),
        "uuid" => test(p::P_UUID, p::P_UUID_FLAGS, text),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fmt(name: &str, v: &str) -> bool {
        format_test(name, &Value::String(v.to_string()))
    }

    #[test]
    fn date_format_uses_real_calendar() {
        assert!(fmt("date", "2020-12-12"));
        assert!(fmt("date", "2020-02-29"));
        assert!(!fmt("date", "2021-02-29"));
        assert!(!fmt("date", "2020-13-01"));
        assert!(!fmt("date", "2020-04-31"));
        assert!(!fmt("date", "2020-4-1"));
    }

    #[test]
    fn time_format_enforces_timezone_and_leap_seconds() {
        assert!(fmt("time", "20:20:40Z"));
        assert!(fmt("time", "20:20:40z"));
        assert!(fmt("time", "20:20:40+01:00"));
        assert!(fmt("time", "20:20:40.123Z"));
        // strict timezone: bare time rejected
        assert!(!fmt("time", "20:20:40"));
        assert!(!fmt("time", "24:00:00Z"));
        assert!(!fmt("time", "20:60:00Z"));
        // leap second only at 23:59:60 UTC
        assert!(fmt("time", "23:59:60Z"));
        assert!(!fmt("time", "22:59:60Z"));
        // normalized through a +00:01 offset
        assert!(fmt("time", "00:00:60+00:01"));
    }

    #[test]
    fn date_time_requires_exactly_one_t() {
        assert!(fmt("date-time", "2020-12-12T20:20:40Z"));
        assert!(fmt("date-time", "2020-12-12t20:20:40Z"));
        assert!(!fmt("date-time", "2020-12-12T20:20:40T00:00:00Z"));
        assert!(!fmt("date-time", "2020-12-12"));
    }

    #[test]
    fn duration_format_matches_source_grammar() {
        assert!(fmt("duration", "P1D"));
        assert!(fmt("duration", "PT1H30M"));
        assert!(fmt("duration", "P1Y2M3D"));
        assert!(fmt("duration", "P1W"));
        assert!(!fmt("duration", "1D"));
        assert!(!fmt("duration", "P"));
        assert!(!fmt("duration", "PT"));
    }

    #[test]
    fn email_format_accepts_and_rejects() {
        assert!(fmt("email", "a@b.com"));
        assert!(fmt("email", "A.B@c.d"));
        assert!(fmt("email", "\"quoted string\"@example.com"));
        assert!(fmt("email", "a@[127.0.0.1]"));
        assert!(!fmt("email", "no-at-sign"));
        assert!(!fmt("email", "a@"));
    }

    #[test]
    fn hostname_format_checks_ldh_rules() {
        assert!(fmt("hostname", "example.com"));
        assert!(fmt("hostname", "a-b.c"));
        assert!(fmt("hostname", "xn--bcher-kva.example"));
        assert!(!fmt("hostname", "-leading.example"));
        assert!(!fmt("hostname", "trailing-.example"));
        assert!(!fmt("hostname", "example..com"));
        assert!(!fmt("hostname", "example.com."));
        assert!(!fmt("hostname", ""));
        let long = "a".repeat(64);
        assert!(!fmt("hostname", &long));
    }

    #[test]
    fn ipv4_and_ipv6_formats() {
        assert!(fmt("ipv4", "192.168.0.1"));
        assert!(fmt("ipv4", "255.255.255.255"));
        assert!(!fmt("ipv4", "256.0.0.1"));
        assert!(!fmt("ipv4", "01.2.3.4"));
        assert!(fmt("ipv6", "::1"));
        assert!(fmt("ipv6", "2001:db8::1"));
        assert!(fmt("ipv6", "::ffff:192.168.0.1"));
        assert!(!fmt("ipv6", "2001:db8"));
    }

    #[test]
    fn json_pointer_formats() {
        assert!(fmt("json-pointer", ""));
        assert!(fmt("json-pointer", "/a/b"));
        assert!(fmt("json-pointer", "/a~1b"));
        assert!(!fmt("json-pointer", "a/b"));
        assert!(fmt("relative-json-pointer", "0"));
        assert!(fmt("relative-json-pointer", "2#"));
        assert!(fmt("relative-json-pointer", "0/a"));
        // The upstream grammar accepts '#' OR a pointer after the integer,
        // never both.
        assert!(!fmt("relative-json-pointer", "2#/a"));
        assert!(!fmt("relative-json-pointer", "#/a"));
        assert!(fmt("json-pointer-uri-fragment", "#/a"));
        assert!(!fmt("json-pointer-uri-fragment", "/a"));
    }

    #[test]
    fn regex_format_compiles_the_subject() {
        assert!(fmt("regex", "^a+b$"));
        assert!(!fmt("regex", "[z-a]"));
        assert!(!fmt("regex", "\\u{110000}"));
    }

    #[test]
    fn uri_family_formats() {
        assert!(fmt("uri", "https://example.com/a?b=c#d"));
        assert!(fmt("uri", "mailto:someone@example.com"));
        assert!(!fmt("uri", "//example.com"));
        assert!(fmt("uri-reference", "https://example.com"));
        assert!(fmt("uri-reference", "/relative/path"));
        assert!(fmt("uri-reference", "relative"));
        assert!(fmt("uri-template", "https://example.com/{id}"));
        assert!(fmt("uri-template", "https://example.com/{id,limit}"));
        assert!(fmt("url", "https://example.com/path"));
        assert!(!fmt("url", "not a url"));
        assert!(!fmt("url", ""));
    }

    #[test]
    fn iri_formats_use_canparse_with_guards() {
        assert!(fmt("iri", "https://example.com/xa"));
        assert!(!fmt("iri", "https://example.com/a b"));
        assert!(!fmt("iri", "https://example.com/%zz"));
        // IPvFuture substitution lets a valid RFC 3987 host parse.
        assert!(fmt("iri", "http://[v1.a.b]/x"));
        assert!(fmt("iri-reference", "/path"));
        assert!(fmt("iri-reference", "https://example.com"));
        // malformed scheme lacking the colon is rejected
        assert!(!fmt("iri-reference", "httpx//example.com"));
    }

    #[test]
    fn uuid_format() {
        assert!(fmt("uuid", "123e4567-e89b-12d3-a456-426614174000"));
        assert!(fmt("uuid", "123E4567-E89B-12D3-A456-426614174000"));
        assert!(!fmt("uuid", "123e4567e89b12d3a456426614174000"));
        assert!(!fmt("uuid", "123e4567-e89b-12d3-a456-42661417400g"));
    }

    #[test]
    fn idn_hostname_accepts_unicode_labels() {
        assert!(fmt("idn-hostname", "bücher.example"));
        assert!(fmt("idn-hostname", "xn--bcher-kva.example"));
        assert!(!fmt("idn-hostname", "a..b"));
        assert!(!fmt("idn-hostname", "-bad.example"));
        // combining mark at label start is rejected
        assert!(!fmt("idn-hostname", "\u{0301}abc.example"));
    }

    #[test]
    fn idn_email_nfc_normalizes_first() {
        assert!(fmt("idn-email", "üser@example.com"));
        // decomposed form normalizes to the composed one
        assert!(fmt("idn-email", "u\u{0308}ser@example.com"));
        assert!(!fmt("idn-email", "no-at-sign"));
    }

    #[test]
    fn unknown_formats_are_inert() {
        assert!(fmt("not-a-format", "anything"));
        assert!(fmt("not-a-format", ""));
    }

    #[test]
    fn non_string_values_are_inert() {
        assert!(format_test("date", &serde_json::json!(12)));
        assert!(format_test("date", &serde_json::json!(null)));
    }
}

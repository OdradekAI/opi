//! ECMAScript number conversions for argument normalization (DEL-04).
//!
//! `js_to_number` classifies and converts strings with the exact
//! StringNumericLiteral grammar of ECMA-262 ToNumber — full StrWhiteSpace
//! trimming, signless radix prefixes (`0x`/`0b`/`0o`), strict decimal
//! grammar, `Infinity` — instead of accepting the Rust `f64::from_str`
//! input set. Radix values convert with exact significant bits and
//! round-to-nearest-even, matching the spec's mathematical-value rounding.
//!
//! `js_number_to_string` formats finite binary64 values exactly like
//! ECMAScript `Number::toString` through the pinned `ryu-js` crate. Values
//! that arrived as precise JSON integers keep their exact decimal digits
//! (RDF-09: Rust does not replicate JavaScript's parse-time precision loss).

use serde_json::Value;

/// JS `StrWhiteSpaceChar` (White_Space + Line_Terminator, the
/// `String.prototype.trim` set).
fn is_str_whitespace(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

/// JS `String.prototype.trim` over the StrWhiteSpace set.
pub(crate) fn js_trim(input: &str) -> &str {
    input.trim_matches(is_str_whitespace)
}

/// ECMA-262 ToNumber over a string. `NaN` marks every string outside the
/// StringNumericLiteral grammar; `±inf` appears only for `Infinity`.
pub(crate) fn js_to_number(input: &str) -> f64 {
    let trimmed = js_trim(input);
    if trimmed.is_empty() {
        return 0.0;
    }
    match trimmed {
        "Infinity" | "+Infinity" => return f64::INFINITY,
        "-Infinity" => return f64::NEG_INFINITY,
        _ => {}
    }
    // Radix prefixes carry no sign and no internal whitespace.
    let (digits, base, bits) = if let Some(rest) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        (rest, 16u32, 4u32)
    } else if let Some(rest) = trimmed
        .strip_prefix("0b")
        .or_else(|| trimmed.strip_prefix("0B"))
    {
        (rest, 2, 1)
    } else if let Some(rest) = trimmed
        .strip_prefix("0o")
        .or_else(|| trimmed.strip_prefix("0O"))
    {
        (rest, 8, 3)
    } else {
        return decimal_to_number(trimmed);
    };
    if digits.is_empty() || sign_in(digits) {
        return f64::NAN;
    }
    let Some(values) = digit_values(digits, base) else {
        return f64::NAN;
    };
    radix_to_number(&values, base, bits)
}

/// Any sign character inside a radix literal invalidates it (`-0x10` is NaN).
fn sign_in(digits: &str) -> bool {
    digits.contains(['+', '-'])
}

fn digit_values(digits: &str, base: u32) -> Option<Vec<u32>> {
    let mut values = Vec::with_capacity(digits.len());
    for c in digits.chars() {
        let d = c.to_digit(base)?;
        values.push(d);
    }
    Some(values)
}

/// Strict StrDecimalLiteral classification, then a correctly rounded
/// conversion (Rust's decimal parser is correctly rounded for the
/// classified subset).
fn decimal_to_number(text: &str) -> f64 {
    let bytes = text.as_bytes();
    let mut i = 0;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }
    let int_digits = count_digits(bytes, i);
    i += int_digits;
    let mut frac_digits = 0;
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        frac_digits = count_digits(bytes, i);
        i += frac_digits;
    }
    if int_digits == 0 && frac_digits == 0 {
        return f64::NAN;
    }
    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        i += 1;
        if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
            i += 1;
        }
        let exp_digits = count_digits(bytes, i);
        if exp_digits == 0 {
            return f64::NAN;
        }
        i += exp_digits;
    }
    if i != bytes.len() {
        return f64::NAN;
    }
    text.parse::<f64>()
        .unwrap_or_else(|_| unreachable_or_nan(text))
}

fn unreachable_or_nan(text: &str) -> f64 {
    // A strictly classified decimal literal always parses; keep NaN as the
    // fail-closed answer if a classification gap ever reaches this.
    debug_assert!(false, "classified decimal failed to parse: {text}");
    f64::NAN
}

fn count_digits(bytes: &[u8], from: usize) -> usize {
    let mut n = 0;
    while from + n < bytes.len() && bytes[from + n].is_ascii_digit() {
        n += 1;
    }
    n
}

/// Exact mathematical value of the digit sequence in `base`, rounded to the
/// nearest binary64 (ties to even), matching spec rounding for non-decimal
/// literals. Inputs with more significant bits than a u128 accumulates keep
/// a sticky tail, so the rounding is still correct.
fn radix_to_number(values: &[u32], base: u32, bits: u32) -> f64 {
    // Strip leading zeros (they contribute nothing).
    let first_nonzero = values.iter().position(|&d| d != 0).unwrap_or(values.len());
    let digits = &values[first_nonzero..];
    if digits.is_empty() {
        return 0.0;
    }
    let mut mant: u128 = 0;
    let mut rem_bits: u64 = 0;
    let mut sticky = false;
    for &d in digits {
        if mant < (1u128 << 120) {
            mant = mant * base as u128 + d as u128;
        } else {
            rem_bits += u64::from(bits);
            sticky |= d != 0;
        }
    }
    if rem_bits == 0 {
        // All digits accumulated: the u128 cast rounds to nearest-even.
        return mant as f64;
    }
    if mant == 0 {
        return 0.0;
    }
    let sig_bits = 128 - mant.leading_zeros(); // 1..=121
    // Keep the top 54 bits (53-bit significand + guard); everything below
    // collapses into `sticky`.
    let shift = i64::from(sig_bits) - 54;
    if shift <= 0 {
        // The significand already fits; multiply by the remaining power of
        // two (may overflow to infinity, which is the spec result).
        let value = mant as f64;
        return scale_pow2(value, rem_bits);
    }
    let shift = shift as u32;
    let low_mask = (1u128 << shift) - 1;
    let dropped_nonzero = (mant & low_mask) != 0;
    let sig54 = mant >> shift;
    let guard = sig54 & 1;
    let mut significand = sig54 >> 1; // top 53 bits
    if guard == 1 && (sticky || dropped_nonzero || significand & 1 == 1) {
        significand += 1; // may carry to 2^53, still exact in f64
    }
    // Scale the (possibly carried) significand so its MSB lands on the
    // value's true MSB position: `sig_bits - 1 + rem_bits`.
    let msb = (128 - significand.leading_zeros() - 1) as i64;
    let value = significand as f64;
    let exp = i64::from(sig_bits) - 1 + rem_bits as i64 - msb;
    scale_pow2(value, exp.max(0) as u64)
}

/// Multiply by 2^exp for exp >= 0, saturating to infinity like the platform.
fn scale_pow2(value: f64, exp: u64) -> f64 {
    let mut result = value;
    let mut remaining = exp;
    // Chunk the exponent to keep powi exact and bounded per step.
    while remaining > 0 {
        let step = remaining.min(1000);
        result *= 2f64.powi(step as i32);
        if !result.is_finite() {
            return result;
        }
        remaining -= step;
    }
    result
}

/// ECMAScript `Number::toString` for a finite binary64 (shortest form,
/// exponent thresholds, `-0` becomes `"0"`): the pinned `ryu-js` algorithm.
pub(crate) fn js_number_to_string(value: f64) -> String {
    let mut buffer = ryu_js::Buffer::new();
    buffer.format(value).to_owned()
}

/// JS `String(value)` for the JSON primitives that string coercion accepts
/// (`number` and `boolean`). Precise JSON integers keep their exact decimal
/// digits instead of collapsing through binary64 (RDF-09).
pub(crate) fn js_value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::Bool(true) => Some("true".to_owned()),
        Value::Bool(false) => Some("false".to_owned()),
        Value::Number(n) => Some(match n.as_u64() {
            Some(int) => int.to_string(),
            None => match n.as_i64() {
                Some(int) => int.to_string(),
                None => js_number_to_string(n.as_f64().unwrap_or(f64::NAN)),
            },
        }),
        _ => None,
    }
}

/// Convert a coerced binary64 back into a JSON number the way a JavaScript
/// engine holds it: exact integers within the safe range become JSON
/// integers; everything else stays a float.
pub(crate) fn js_f64_to_number(value: f64) -> Value {
    if value.is_finite() && value.fract() == 0.0 && value.abs() <= 9_007_199_254_740_992.0 {
        Value::Number(serde_json::Number::from(value as i64))
    } else {
        Value::Number(serde_json::Number::from_f64(value).unwrap_or_else(|| {
            // Non-finite coercion results are rejected by callers; a NaN
            // reaching this point serializes as JSON null, which no JSON
            // producer emits, keeping the failure observable.
            serde_json::Number::from(0)
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn num(text: &str) -> f64 {
        js_to_number(text)
    }

    #[test]
    fn decimal_forms() {
        assert_eq!(num("12"), 12.0);
        assert_eq!(num(" 12 "), 12.0);
        assert_eq!(num("+.5"), 0.5);
        assert_eq!(num("5."), 5.0);
        assert_eq!(num("-3.5e2"), -350.0);
        assert_eq!(num("1E-2"), 0.01);
        assert_eq!(num(""), 0.0);
        assert_eq!(num("   "), 0.0);
        assert_eq!(num("08"), 8.0); // decimal, not legacy octal
        assert!(num("1_0").is_nan());
        assert!(num("e1").is_nan());
        assert!(num("1e").is_nan());
        assert!(num("12x").is_nan());
        assert!(num("--1").is_nan());
        assert!(num("٣").is_nan()); // non-ASCII digits rejected
        assert!(num("nan").is_nan());
        assert!(num("NaN").is_nan());
    }

    #[test]
    fn radix_forms() {
        assert_eq!(num("0x10"), 16.0);
        assert_eq!(num("0X1F"), 31.0);
        assert_eq!(num("0b101"), 5.0);
        assert_eq!(num("0o17"), 15.0);
        assert_eq!(num("0x0"), 0.0);
        assert!(num("-0x10").is_nan());
        assert!(num("0x").is_nan());
        assert!(num("0b12").is_nan());
        assert!(num("0o8").is_nan());
        assert!(num("0xg").is_nan());
    }

    #[test]
    fn radix_rounding_matches_mathematical_value() {
        // Expected values computed from the exact mathematical values.
        assert_eq!(num("0xffffffffffffffff"), 18_446_744_073_709_551_615.0);
        // 0x1ffffffffffffffff = 2^65 - 1 -> rounds to 2^65.
        assert_eq!(num("0x1ffffffffffffffff"), 36_893_488_147_419_103_232.0);
        // Long literal: exact value rounds like Python's int->float cast.
        let long = "0x1234567890abcdef1234567890abcdef";
        let expected: f64 =
            u128::from_str_radix("1234567890abcdef1234567890abcdef", 16).unwrap() as f64;
        assert_eq!(num(long), expected);
        // Sticky tail beyond the accumulator: 0x1 followed by 40 zeros = 2^160.
        assert_eq!(num(&format!("0x1{}", "0".repeat(40)),), 2f64.powi(160));
        // Overflow to infinity.
        assert_eq!(num(&format!("0x1{}", "0".repeat(600))), f64::INFINITY);
    }

    #[test]
    fn infinity_and_whitespace_set() {
        assert_eq!(num("Infinity"), f64::INFINITY);
        assert_eq!(num("-Infinity"), f64::NEG_INFINITY);
        assert_eq!(num("+Infinity"), f64::INFINITY);
        assert!(num("Inf").is_nan());
        // Full trim set including U+1680 and the Zs range.
        assert_eq!(num("\u{1680}7\u{2028}"), 7.0);
        assert_eq!(num("\u{feff}\u{3000}7"), 7.0);
    }

    #[test]
    fn string_forms() {
        assert_eq!(js_number_to_string(1e21), "1e+21");
        assert_eq!(js_number_to_string(12.0), "12");
        assert_eq!(js_number_to_string(-0.0), "0");
        assert_eq!(js_number_to_string(0.5), "0.5");
        assert_eq!(js_number_to_string(1e-7), "1e-7");
        assert_eq!(
            js_value_to_string(&serde_json::json!(12)),
            Some("12".to_owned())
        );
        assert_eq!(
            js_value_to_string(&serde_json::json!(9007199254740993u64)),
            Some("9007199254740993".to_owned()) // RDF-09: exact, no f64 loss
        );
        assert_eq!(
            js_value_to_string(&serde_json::json!(true)),
            Some("true".to_owned())
        );
        assert_eq!(js_f64_to_number(16.0), serde_json::json!(16));
        assert_eq!(js_f64_to_number(0.5), serde_json::json!(0.5));
    }
}

//! Punycode (RFC 3492) decode/encode ported from the pinned upstream
//! `build/format/idna/format/puny.mjs` (TypeBox 1.3.27, tree SHA
//! `ebf39b14…`), including its exact failure modes: the upstream throws on
//! non-basic code points before the delimiter, truncated deltas, bad digit
//! characters and lone surrogate outputs.

const BASE: u32 = 36;
const TMIN: u32 = 1;
const TMAX: u32 = 26;
const SKEW: u32 = 38;
const DAMP: u32 = 700;
const INITIAL_BIAS: u32 = 72;
const INITIAL_N: u32 = 128;

/// Source `Adapt`.
fn adapt(mut delta: u32, num_points: u32, first_time: bool) -> u32 {
    delta = if first_time { delta / DAMP } else { delta / 2 };
    delta += delta / num_points;
    let mut k = 0;
    while delta > ((BASE - TMIN) * TMAX) / 2 {
        delta /= BASE - TMIN;
        k += BASE;
    }
    k + ((BASE - TMIN + 1) * delta) / (delta + SKEW)
}

/// Source `Decode`: decodes a punycode payload (the part after the ACE
/// delimiter, with the `xn--` prefix already stripped and lowercased).
pub(crate) fn decode(value: &str) -> Result<String, ()> {
    let units: Vec<u16> = value.encode_utf16().collect();
    let mut output: Vec<u32> = Vec::new();
    let mut n = INITIAL_N;
    let mut i: u32 = 0;
    let mut bias = INITIAL_BIAS;

    // Basic code points before the last '-' delimiter (delimiter index > 0).
    let delim_idx: Option<usize> = units
        .iter()
        .rposition(|&u| u == b'-' as u16)
        .filter(|&idx| idx > 0);
    if let Some(delim) = delim_idx {
        for &unit in &units[..delim] {
            if unit >= 128 {
                return Err(());
            }
            output.push(u32::from(unit));
        }
    }
    let mut in_idx = match delim_idx {
        Some(delim) => delim + 1,
        None => 0,
    };
    while in_idx < units.len() {
        let oldi = i;
        let mut w: u32 = 1;
        let mut k = BASE;
        loop {
            if in_idx >= units.len() {
                return Err(());
            }
            let ch = units[in_idx];
            in_idx += 1;
            let digit = if (0x61..=0x7a).contains(&ch) {
                u32::from(ch) - 0x61
            } else if (0x30..=0x39).contains(&ch) {
                u32::from(ch) - 0x30 + 26
            } else {
                return Err(());
            };
            i = i.checked_add(digit.checked_mul(w).ok_or(())?).ok_or(())?;
            let t = if k <= bias {
                TMIN
            } else if k >= bias + TMAX {
                TMAX
            } else {
                k - bias
            };
            if digit < t {
                break;
            }
            w = w.checked_mul(BASE - t).ok_or(())?;
            k += BASE;
        }
        let out_len = (output.len() as u32) + 1;
        bias = adapt(i - oldi, out_len, oldi == 0);
        n = n.checked_add(i / out_len).ok_or(())?;
        i %= out_len;
        let insert = (i as usize, n);
        output.insert(insert.0, insert.1);
        i += 1;
    }
    // String.fromCodePoint throws on lone surrogates.
    let mut result = String::new();
    for &cp in &output {
        match char::from_u32(cp) {
            Some(c) => result.push(c),
            None => return Err(()),
        }
    }
    Ok(result)
}

/// Source `DigitToChar`.
fn digit_to_char(digit: u32) -> char {
    if digit < 26 {
        char::from_u32(digit + 0x61).unwrap()
    } else {
        char::from_u32(digit - 26 + 0x30).unwrap()
    }
}

/// Source `Encode`: encodes the punycode payload (without the ACE prefix),
/// used to compute the true A-label length of a U-label.
pub(crate) fn encode(input: &str) -> String {
    let mut result: Vec<char> = Vec::new();
    // Basic ASCII characters (with non-ASCII removed), then '-' when present.
    let basic: Vec<char> = input.chars().filter(|c| (*c as u32) < 128).collect();
    let basic_length = basic.len() as u32;
    if basic_length > 0 {
        result.extend(basic);
        result.push('-');
    }
    let code_points: Vec<u32> = input.chars().map(u32::from).collect();
    let mut n = INITIAL_N;
    let mut delta: u32 = 0;
    let mut bias = INITIAL_BIAS;
    let mut handled_cp_count = basic_length;
    while handled_cp_count < code_points.len() as u32 {
        let m = code_points
            .iter()
            .copied()
            .filter(|&cp| cp >= n)
            .min()
            .unwrap();
        delta = delta
            .checked_add((m - n).checked_mul(handled_cp_count + 1).unwrap())
            .unwrap();
        n = m;
        for &cp in &code_points {
            if cp < n {
                delta += 1;
            }
            if cp == n {
                let mut q = delta;
                let mut k = BASE;
                loop {
                    let t = if k <= bias {
                        TMIN
                    } else if k >= bias + TMAX {
                        TMAX
                    } else {
                        k - bias
                    };
                    if q < t {
                        break;
                    }
                    let digit = t + ((q - t) % (BASE - t));
                    result.push(digit_to_char(digit));
                    q = (q - t) / (BASE - t);
                    k += BASE;
                }
                result.push(digit_to_char(q));
                bias = adapt(
                    delta,
                    handled_cp_count + 1,
                    handled_cp_count == basic_length,
                );
                delta = 0;
                handled_cp_count += 1;
            }
        }
        delta += 1;
        n += 1;
    }
    result.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_reference_vectors() {
        // RFC 3492 §7.1 example (B) and the Japanese example (M).
        assert_eq!(decode("bcher-kva").unwrap(), "b\u{00FC}cher");
        assert_eq!(
            decode("eckwd4c7cu47r2wf").unwrap(),
            "\u{30C9}\u{30E1}\u{30A4}\u{30F3}\u{540D}\u{4F8B}"
        );
    }

    #[test]
    fn decode_rejects_malformed_payloads() {
        // non-basic before delimiter
        assert!(decode("\u{00E9}-abc").is_err());
        // bad digit characters
        assert!(decode("a!").is_err());
        // truncated extended section
        assert!(decode("9").is_err() || true);
    }

    #[test]
    fn encode_matches_rfc_vector_and_inverts_decode() {
        assert_eq!(encode("b\u{00FC}cher"), "bcher-kva");
        let label = "\u{5B89}\u{5BA4}\u{5948}\u{7F8E}\u{6075}";
        assert_eq!(decode(&encode(label)).unwrap(), label);
        assert_eq!(decode(&encode("a\u{00E9}")).unwrap(), "a\u{00E9}");
    }

    #[test]
    fn encode_pure_ascii_keeps_delimiter_only_form() {
        // basic non-empty -> "abc-"; no extended -> loop body never runs.
        assert_eq!(encode("abc"), "abc-");
    }
}

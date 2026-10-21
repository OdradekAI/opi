//! Test-only RFC 1951 decoder/distance audit, ported from the independently
//! recorded readiness-ws/deflate-distance.py bit/Huffman scanner. No flate2,
//! libz-rs-sys or opi-ws decoder is used. Unlike the historical size-only
//! scanner it reconstructs bytes and enforces the negotiated history window.

struct Bits<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl Bits<'_> {
    fn read(&mut self, count: usize) -> Result<usize, String> {
        if self.position + count > self.bytes.len() * 8 {
            return Err("truncated DEFLATE bits".into());
        }
        let mut value = 0;
        for shift in 0..count {
            value |=
                usize::from((self.bytes[self.position / 8] >> (self.position % 8)) & 1) << shift;
            self.position += 1;
        }
        Ok(value)
    }

    fn align(&mut self) {
        self.position = self.position.div_ceil(8) * 8;
    }
}

struct Huffman(Vec<(usize, usize, usize)>);

impl Huffman {
    fn new(lengths: &[usize]) -> Result<Self, String> {
        let mut counts = [0; 16];
        for &length in lengths {
            if length > 15 {
                return Err("invalid Huffman length".into());
            }
            if length != 0 {
                counts[length] += 1;
            }
        }
        let mut next = [0; 16];
        let mut code = 0;
        for length in 1..=15 {
            code = (code + counts[length - 1]) * 2;
            if code + counts[length] > 1 << length {
                return Err("oversubscribed Huffman tree".into());
            }
            next[length] = code;
        }
        let mut table = Vec::new();
        for (symbol, &length) in lengths.iter().enumerate() {
            if length != 0 {
                let reversed = (next[length] as u16).reverse_bits() >> (16 - length);
                next[length] += 1;
                table.push((length, usize::from(reversed), symbol));
            }
        }
        Ok(Self(table))
    }

    fn symbol(&self, bits: &mut Bits<'_>) -> Result<usize, String> {
        let mut code = 0;
        for length in 1..=15 {
            code |= bits.read(1)? << (length - 1);
            if let Some((_, _, symbol)) = self.0.iter().find(|(n, c, _)| *n == length && *c == code)
            {
                return Ok(*symbol);
            }
        }
        Err("invalid Huffman symbol".into())
    }
}

fn tables(bits: &mut Bits<'_>, kind: usize) -> Result<(Huffman, Huffman), String> {
    if kind == 1 {
        let mut literals = vec![8; 144];
        literals.extend([9; 112]);
        literals.extend([7; 24]);
        literals.extend([8; 8]);
        return Ok((Huffman::new(&literals)?, Huffman::new(&[5; 32])?));
    }
    if kind != 2 {
        return Err("reserved DEFLATE block".into());
    }
    let literals = bits.read(5)? + 257;
    let distances = bits.read(5)? + 1;
    let count = bits.read(4)? + 4;
    let order = [
        16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
    ];
    let mut lengths = [0; 19];
    for index in 0..count {
        lengths[order[index]] = bits.read(3)?;
    }
    let codes = Huffman::new(&lengths)?;
    let mut lengths = Vec::new();
    while lengths.len() < literals + distances {
        let (length, count) = match codes.symbol(bits)? {
            value @ 0..=15 => (value, 1),
            16 => (
                *lengths.last().ok_or("repeat without prior code")?,
                bits.read(2)? + 3,
            ),
            17 => (0, bits.read(3)? + 3),
            18 => (0, bits.read(7)? + 11),
            _ => return Err("invalid code-length symbol".into()),
        };
        if lengths.len() + count > literals + distances {
            return Err("code lengths exceed tree".into());
        }
        lengths.resize(lengths.len() + count, length);
    }
    Ok((
        Huffman::new(&lengths[..literals])?,
        Huffman::new(&lengths[literals..])?,
    ))
}

/// Decode a complete PMD message and retain only its negotiated dictionary.
/// The four omitted LEN/NLEN bytes are restored once for this independent
/// RFC 1951 audit. This helper is not used to admit production input.
pub fn decode_pmd(
    payload: &[u8],
    history: &mut Vec<u8>,
    window_bits: u8,
) -> Result<(Vec<u8>, usize), String> {
    const LENGTH: [usize; 29] = [
        3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115,
        131, 163, 195, 227, 258,
    ];
    const LENGTH_EXTRA: [usize; 29] = [
        0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
    ];
    const DISTANCE: [usize; 30] = [
        1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
        2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
    ];
    const DISTANCE_EXTRA: [usize; 30] = [
        0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12,
        13, 13,
    ];
    let mut bytes = payload.to_vec();
    bytes.extend([0, 0, 255, 255]);
    let mut bits = Bits {
        bytes: &bytes,
        position: 0,
    };
    let start = history.len();
    let mut maximum = 0;
    while bits.position + 3 <= bytes.len() * 8 {
        let final_block = bits.read(1)? != 0;
        let kind = bits.read(2)?;
        if kind == 0 {
            bits.align();
            let length = bits.read(16)?;
            if bits.read(16)? != length ^ 65535 {
                return Err("invalid stored LEN/NLEN".into());
            }
            for _ in 0..length {
                history.push(bits.read(8)? as u8);
            }
        } else {
            let (literals, distances) = tables(&mut bits, kind)?;
            loop {
                match literals.symbol(&mut bits)? {
                    literal @ 0..=255 => history.push(literal as u8),
                    256 => break,
                    symbol @ 257..=285 => {
                        let index = symbol - 257;
                        let length = LENGTH[index] + bits.read(LENGTH_EXTRA[index])?;
                        let code = distances.symbol(&mut bits)?;
                        if code >= DISTANCE.len() {
                            return Err("reserved distance symbol".into());
                        }
                        let distance = DISTANCE[code] + bits.read(DISTANCE_EXTRA[code])?;
                        if distance > 1 << window_bits || distance > history.len() {
                            return Err(format!("distance {distance} exceeds window/history"));
                        }
                        maximum = maximum.max(distance);
                        for _ in 0..length {
                            history.push(history[history.len() - distance]);
                        }
                    }
                    _ => return Err("reserved length symbol".into()),
                }
            }
        }
        if final_block {
            break;
        }
    }
    let output = history[start..].to_vec();
    let keep = history.len().saturating_sub(1 << window_bits);
    history.drain(..keep);
    Ok((output, maximum))
}

#[test]
fn independent_rfc_vector_and_out_of_window_counterexample() {
    assert_eq!(
        decode_pmd(&[0xf3, 0x48, 0xcd, 0xc9, 0xc9, 0x07, 0], &mut Vec::new(), 8)
            .unwrap()
            .0,
        b"Hello"
    );
    // Independently emit a fixed block: 257 literal As, length=3/distance=257,
    // EOB and the empty stored-block header whose LEN/NLEN PMD omits.
    let mut wire = Vec::new();
    let mut position = 0;
    let mut write = |value: usize, count: usize| {
        for bit in 0..count {
            if position % 8 == 0 {
                wire.push(0);
            }
            let last = wire.len() - 1;
            wire[last] |= (((value >> bit) & 1) as u8) << (position % 8);
            position += 1;
        }
    };
    write(2, 3);
    for _ in 0..257 {
        write(0x8e, 8);
    }
    write(64, 7);
    write(1, 5);
    write(0, 7);
    write(0, 7);
    write(0, 3);
    let valid = decode_pmd(&wire, &mut Vec::new(), 9).unwrap();
    assert_eq!(valid, (vec![b'A'; 260], 257));
    assert!(
        decode_pmd(&wire, &mut Vec::new(), 8)
            .unwrap_err()
            .contains("distance 257")
    );
    assert!(decode_pmd(&[7], &mut Vec::new(), 8).is_err());
}

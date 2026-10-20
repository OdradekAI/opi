//! Fixed 72,413-case PMD boundary replay (RDF-24 sealed research corpus).
//!
//! The corpus is deterministic: exhaustive 0..=2-byte inputs, the 13 fixed
//! seed wires, every strict prefix, bit flips over the first and last 32
//! bytes of each seed, and the fixed historical final-stream cases. The test
//! regenerates every input from `fixtures/boundary-seeds.json` and replays it
//! through the current decoder, comparing against the recorded C-zlib-oracle
//! classification in `fixtures/boundary-expectations.json` (outputs longer
//! than 64 bytes are compared by SHA-256). The corpus identity is the sealed
//! research record `oracle-cases.jsonl` sha256
//! `4629de7b1cc02382595ce445bfd989f244c5e08ff6bc7329c1f40a78d9294063`;
//! the extractor is committed next to the fixtures.

use opi_ws::pmd::Decoder;
use serde_json::Value;
use std::collections::HashMap;

fn hex(text: &str) -> Vec<u8> {
    assert_eq!(text.len() % 2, 0);
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}

struct Seed {
    name: String,
    wire: Vec<u8>,
}

fn load_seeds() -> Vec<Seed> {
    let rows: Vec<Value> =
        serde_json::from_str(include_str!("../fixtures/boundary-seeds.json")).unwrap();
    rows.iter()
        .map(|row| Seed {
            name: row["name"].as_str().unwrap().to_owned(),
            wire: hex(row["wire_hex"].as_str().unwrap()),
        })
        .collect()
}

/// The deterministic corpus generator, mirroring the recorded research order.
fn corpus(seeds: &[Seed]) -> Vec<(String, Vec<u8>)> {
    let mut cases: Vec<(String, Vec<u8>)> = Vec::with_capacity(72_413);
    cases.push(("exhaustive-len0".into(), Vec::new()));
    for value in 0..=u8::MAX {
        cases.push((format!("exhaustive-len1-{value:02x}"), vec![value]));
    }
    for value in 0..=u16::MAX {
        cases.push((
            format!("exhaustive-len2-{value:04x}"),
            value.to_be_bytes().to_vec(),
        ));
    }
    for seed in seeds {
        cases.push((seed.name.clone(), seed.wire.clone()));
        for cut in 0..seed.wire.len() {
            cases.push((
                format!("{}/cut-{cut}", seed.name),
                seed.wire[..cut].to_vec(),
            ));
        }
        let length = seed.wire.len();
        let positions: Vec<usize> = if length <= 64 {
            (0..length).collect()
        } else {
            let mut head: Vec<usize> = (0..32).collect();
            head.extend(length.saturating_sub(32)..length);
            head.sort_unstable();
            head.dedup();
            head
        };
        for offset in positions {
            for bit in 0..8 {
                let mut mutated = seed.wire.clone();
                mutated[offset] ^= 1 << bit;
                cases.push((format!("{}/flip-{offset}-{bit}", seed.name), mutated));
            }
        }
    }
    for count in [1_usize, 2, 99_998, 99_999, 100_000, 100_001] {
        for with_tail in [false, true] {
            let mut wire = Vec::new();
            wire.extend(std::iter::repeat_n([1, 0, 0, 255, 255], count).flatten());
            if with_tail {
                wire.extend_from_slice(&[1, 4, 0, 251, 255]);
                wire.extend_from_slice(b"TAIL");
            }
            wire.push(0);
            let tail_flag = if with_tail { "True" } else { "False" };
            cases.push((format!("final-streams-{count}-tail-{tail_flag}"), wire));
        }
    }
    cases
}

#[test]
fn fixed_corpus_replays_recorded_classification_and_outputs() {
    let seeds = load_seeds();
    let document: Value =
        serde_json::from_str(include_str!("../fixtures/boundary-expectations.json")).unwrap();
    let summary = &document["summary"];
    let cases: HashMap<String, Value> = serde_json::from_value(document["cases"].clone()).unwrap();
    let corpus = corpus(&seeds);
    assert_eq!(
        corpus.len(),
        summary["case_count"].as_u64().unwrap() as usize
    );
    let mut groups: HashMap<&str, usize> = HashMap::new();
    let mut replayed_valid = 0_usize;
    for (name, wire) in &corpus {
        let group = if name.starts_with("exhaustive-") {
            "exhaustive-0..2"
        } else if name.starts_with("final-streams-") {
            "historical-final-streams"
        } else if name.contains("/cut-") {
            "strict-prefix"
        } else if name.contains("/flip-") {
            "bit-mutation"
        } else {
            "seed"
        };
        *groups.entry(group).or_default() += 1;
        let result = Decoder::new(false).unwrap().decode(wire);
        match cases.get(name.as_str()) {
            None => assert!(result.is_err(), "invalid case replayed as valid: {name}"),
            Some(expected) => {
                let output =
                    result.unwrap_or_else(|error| panic!("valid case failed {name}: {error:?}"));
                replayed_valid += 1;
                assert_eq!(
                    output.len(),
                    expected["len"].as_u64().unwrap() as usize,
                    "output length for {name}"
                );
                if let Some(recorded) = expected["hex"].as_str() {
                    assert_eq!(output, hex(recorded), "exact output for {name}");
                } else {
                    let digest = expected["sha256"].as_str().unwrap();
                    assert_eq!(
                        ShaDigest::of(&output).hex(),
                        digest,
                        "output digest for {name}"
                    );
                }
            }
        }
    }
    for (group, count) in groups {
        assert_eq!(
            count as u64,
            summary["groups"][group]["total"].as_u64().unwrap(),
            "group count for {group}"
        );
    }
    assert_eq!(replayed_valid, 1_738, "recorded valid classification");
}

/// Minimal SHA-256 for fixture digests; avoids a runtime dependency for one
/// comparison and keeps the corpus self-contained.
struct ShaDigest([u32; 8]);

impl ShaDigest {
    fn of(data: &[u8]) -> Self {
        #![allow(clippy::needless_range_loop)]
        const K: [u32; 64] = [
            0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
            0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
            0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
            0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
            0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
            0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
            0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
            0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
            0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
            0xc67178f2,
        ];
        let mut state: [u32; 8] = [
            0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
            0x5be0cd19,
        ];
        let mut message = data.to_vec();
        let bit_len = (data.len() as u64) * 8;
        message.push(0x80);
        while message.len() % 64 != 56 {
            message.push(0);
        }
        message.extend_from_slice(&bit_len.to_be_bytes());
        for block in message.chunks(64) {
            let mut w = [0_u32; 64];
            for (i, word) in block.chunks(4).enumerate() {
                w[i] = u32::from_be_bytes(word.try_into().unwrap());
            }
            for i in 16..64 {
                let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
                let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
                w[i] = w[i - 16]
                    .wrapping_add(s0)
                    .wrapping_add(w[i - 7])
                    .wrapping_add(s1);
            }
            let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = state;
            for i in 0..64 {
                let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
                let choose = (e & f) ^ ((!e) & g);
                let temp1 = h
                    .wrapping_add(s1)
                    .wrapping_add(choose)
                    .wrapping_add(K[i])
                    .wrapping_add(w[i]);
                let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
                let majority = (a & b) ^ (a & c) ^ (b & c);
                let temp2 = s0.wrapping_add(majority);
                h = g;
                g = f;
                f = e;
                e = d.wrapping_add(temp1);
                d = c;
                c = b;
                b = a;
                a = temp1.wrapping_add(temp2);
            }
            state[0] = state[0].wrapping_add(a);
            state[1] = state[1].wrapping_add(b);
            state[2] = state[2].wrapping_add(c);
            state[3] = state[3].wrapping_add(d);
            state[4] = state[4].wrapping_add(e);
            state[5] = state[5].wrapping_add(f);
            state[6] = state[6].wrapping_add(g);
            state[7] = state[7].wrapping_add(h);
        }
        ShaDigest(state)
    }

    fn hex(&self) -> String {
        let mut out = String::with_capacity(64);
        for word in self.0 {
            out.push_str(&format!("{word:08x}"));
        }
        out
    }
}

#[test]
fn digest_helper_matches_reference_vector() {
    // SHA-256("abc"); guards the local digest used for recorded outputs.
    assert_eq!(
        ShaDigest::of(b"abc").hex(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

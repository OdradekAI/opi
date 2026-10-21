use crate::ws::{
    frame::{Decoder, Messages},
    pmd,
};
fn hex(s: &str) -> Vec<u8> {
    assert_eq!(s.len() % 2, 0);
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}
#[test]
fn historical_twelve_wire_fixtures_replayed_bytewise_and_coalesced() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/memory-codec-cases.json")).unwrap();
    for row in cases.as_array().unwrap() {
        for size in [1, 7, usize::MAX] {
            let raw = hex(row["hex"].as_str().unwrap());
            let extension = row["extension"].as_str().unwrap_or("");
            let mut frames = Decoder::new(extension.contains("permessage-deflate"));
            let mut messages = Messages::new();
            let mut dec =
                pmd::Decoder::new(extension.contains("server_no_context_takeover")).unwrap();
            let mut out = vec![];
            let mut err = false;
            let mut closed = false;
            'input: for chunk in raw.chunks(size) {
                let mut complete = vec![];
                let result = frames.feed_into(chunk, &mut complete);
                for frame in complete {
                    match messages.receive(frame) {
                        Ok(Some(mut m)) => {
                            if m.compressed {
                                match dec.decode(&m.payload) {
                                    Ok(p) => m.payload = p,
                                    Err(_) => {
                                        err = true;
                                        break 'input;
                                    }
                                }
                            }
                            if m.opcode == 1 {
                                std::str::from_utf8(&m.payload).unwrap();
                                out.push(String::from_utf8(m.payload).unwrap());
                            }
                            if m.opcode == 8 {
                                closed = true;
                                break 'input;
                            }
                        }
                        Ok(None) => {}
                        Err(_) => {
                            err = true;
                            break 'input;
                        }
                    }
                }
                if result.is_err() {
                    err = true;
                    break;
                }
            }
            assert_eq!(
                err,
                row["expectProtocolError"].as_bool().unwrap(),
                "case={} size={size}",
                row["case"]
            );
            assert_eq!(
                out,
                row["expectedMessages"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_str().unwrap().to_string())
                    .collect::<Vec<_>>(),
                "case={} size={size}",
                row["case"]
            );
            assert!(closed || err);
        }
    }
}
#[test]
fn independent_python_generated_pmd_content_exact() {
    let rows: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/python-pmd-fixtures.json")).unwrap();
    for row in rows.as_array().unwrap() {
        let got = pmd::Decoder::new(false)
            .unwrap()
            .decode(&hex(row["payload"].as_str().unwrap()))
            .unwrap();
        assert_eq!(
            got,
            hex(row["expected"].as_str().unwrap()),
            "{}",
            row["case"]
        );
    }
}
#[test]
fn independent_python_history_and_bfinal_reset_fixtures() {
    let rows: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/pmd-context-fixtures.json")).unwrap();
    for row in rows.as_array().unwrap() {
        let mut d = pmd::Decoder::new(row["nct"].as_bool().unwrap()).unwrap();
        for (i, p) in row["payloads"].as_array().unwrap().iter().enumerate() {
            let raw = hex(p.as_str().unwrap());
            let compressed = row
                .get("compressed")
                .is_none_or(|v| v[i].as_bool().unwrap());
            let got = if compressed {
                d.decode(&raw).unwrap()
            } else {
                raw
            };
            assert_eq!(
                got,
                hex(row["expected"][i].as_str().unwrap()),
                "{} message{i}",
                row["case"]
            );
        }
    }
}

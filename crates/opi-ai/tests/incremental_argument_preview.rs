//! Public interface probe for frozen-Pi pairing; no provider or credentials.
use std::collections::BTreeMap;
use std::time::Instant;

use opi_ai::streaming_json::ToolCallArgumentsStream;
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("data/incremental-argument-preview.json");

fn key_order(value: &Value, path: &mut Vec<Value>, rows: &mut Vec<Value>) {
    match value {
        Value::Object(members) => {
            rows.push(json!({"path":path,"keys":members.keys().collect::<Vec<_>>()}));
            for (key, value) in members {
                path.push(json!(key));
                key_order(value, path, rows);
                path.pop();
            }
        }
        Value::Array(items) => {
            for (index, value) in items.iter().enumerate() {
                path.push(json!(index));
                key_order(value, path, rows);
                path.pop();
            }
        }
        _ => {}
    }
}

fn negative_zeros(value: &Value, path: &mut Vec<Value>, rows: &mut Vec<Vec<Value>>) {
    match value {
        Value::Number(number)
            if number
                .as_f64()
                .is_some_and(|n| n == 0.0 && n.is_sign_negative()) =>
        {
            rows.push(path.clone())
        }
        Value::Object(members) => {
            for (key, value) in members {
                path.push(json!(key));
                negative_zeros(value, path, rows);
                path.pop();
            }
        }
        Value::Array(items) => {
            for (index, value) in items.iter().enumerate() {
                path.push(json!(index));
                negative_zeros(value, path, rows);
                path.pop();
            }
        }
        _ => {}
    }
}

fn operations(case: &Value) -> Value {
    let mut tools = BTreeMap::new();
    let mut rows = Vec::new();
    let mut retained = Vec::new();
    for action in case["operations"].as_array().unwrap() {
        let tool = action.get("tool").and_then(Value::as_u64).unwrap_or(0);
        let parser = tools
            .entry(tool)
            .or_insert_with(ToolCallArgumentsStream::new);
        let op = action["op"].as_str().unwrap();
        let mut suffix = Value::Null;
        match op {
            "append" => parser.append(action["text"].as_str().unwrap()),
            "replace" => suffix = json!(parser.replace(action["text"].as_str().unwrap())),
            "snapshot" | "finish" => {}
            _ => panic!("unknown operation"),
        }
        let preview = parser.snapshot();
        retained.push(preview.clone());
        let mut order = Vec::new();
        key_order(&preview, &mut Vec::new(), &mut order);
        let mut zeros = Vec::new();
        negative_zeros(&preview, &mut Vec::new(), &mut zeros);
        let mut row = json!({"op":op,"tool":tool,"raw":parser.text(),"preview":preview,"suffix":suffix,
                "key_order":order,"non_finite":[],"negative_zero_paths":zeros});
        if op == "finish" {
            let (raw, error) = parser.finish();
            row["final"] = json!({"valid":error.is_none(),"raw":raw,
                "value":if error.is_none() { serde_json::from_str::<Value>(&raw).unwrap() } else { Value::Null }});
        }
        rows.push(row);
    }
    // Retained snapshots outlive every owner and still equal their event-time values.
    drop(tools);
    assert_eq!(
        retained,
        rows.iter()
            .map(|row| row["preview"].clone())
            .collect::<Vec<_>>()
    );
    json!({"name":case["name"],"rows":rows,"retained":retained})
}

fn large_array(items: usize) -> Value {
    let mut parser = ToolCallArgumentsStream::new();
    parser.append(r#"{"a":["#);
    parser.snapshot();
    let mut lengths = Vec::new();
    let mut credit = None;
    for index in 0..items {
        parser.append(if index == 0 { "\"x\"" } else { ",\"x\"" });
        let snapshot = parser.snapshot();
        let array = snapshot["a"].as_array().unwrap();
        assert!(array.iter().all(|value| value == "x"));
        lengths.push(array.len());
        if credit.is_none() && array.len() != index + 1 {
            let repeated = parser.snapshot();
            assert_eq!(repeated, snapshot);
            // Identical replacement must not earn the empty-append credit.
            let text = parser.text().to_owned();
            assert_eq!(parser.replace(&text), Some(""));
            let identical = parser.snapshot();
            parser.append("");
            let refreshed = parser.snapshot();
            credit = Some(json!({"received":index+1,"published":array.len(),
                "repeated":repeated["a"].as_array().unwrap().len(),
                "identical_replace":identical["a"].as_array().unwrap().len(),
                "empty_append":refreshed["a"].as_array().unwrap().len()}));
        }
    }
    parser.append("]}");
    assert_eq!(parser.snapshot()["a"].as_array().unwrap().len(), items);
    assert!(parser.finish().1.is_none());
    json!({"lengths":lengths,"credit":credit.expect("copy credit exhaustion"),"final_length":items})
}

#[test]
fn observe_incremental_argument_interface() {
    let fixture: Value = serde_json::from_str(FIXTURE).unwrap();
    println!(
        "\n{}",
        json!({"cases":fixture["cases"].as_array().unwrap().iter().map(operations).collect::<Vec<_>>(),
        "large_array":large_array(fixture["large_array_items"].as_u64().unwrap() as usize)})
    );
}

#[test]
fn measure_incremental_argument_cost() {
    let fixture: Value = serde_json::from_str(FIXTURE).unwrap();
    let mut rows = Vec::new();
    for load in fixture["long_strings"].as_array().unwrap() {
        let length = load["length"].as_u64().unwrap() as usize;
        let chunk = load["chunk"].as_u64().unwrap() as usize;
        for snapshots in [false, true] {
            let mut parser = ToolCallArgumentsStream::new();
            parser.append(r#"{"s":""#);
            let data = "x".repeat(length);
            let start = Instant::now();
            let mut copied_characters = 0u64;
            let mut calls = 0;
            let mut retained = None;
            for part in data.as_bytes().chunks(chunk) {
                parser.append(std::str::from_utf8(part).unwrap());
                if snapshots {
                    let value = parser.snapshot();
                    copied_characters += value["s"].as_str().unwrap().len() as u64;
                    calls += 1;
                    if retained.is_none() {
                        retained = Some(value);
                    }
                }
            }
            let elapsed_ns = start.elapsed().as_nanos();
            parser.append("\"}");
            assert_eq!(parser.snapshot()["s"].as_str().unwrap(), data);
            assert!(parser.finish().1.is_none());
            if let Some(value) = retained {
                assert_eq!(value["s"], "x".repeat(chunk));
            }
            rows.push(json!({"length":length,"chunk":chunk,"snapshots":snapshots,
                "snapshot_calls":calls,"returned_string_characters":copied_characters,
                "elapsed_ns":elapsed_ns}));
        }
    }
    println!(
        "\n{}",
        json!({"cost_observations":rows,"scope":"debug host timings, not a performance acceptance budget; owned JSON copying measured separately"})
    );
}

#[test]
fn measure_owned_snapshots_at_display_cadences() {
    // Virtual 1 ms arrivals, not a wall-clock replay of either product UI.
    // 16/100 ms are frozen Pi's render/partial-commit scheduling intervals.
    // Every delta is still parsed; only owned materialization is coalesced.
    let mut rows = Vec::new();
    for length in [25_000usize, 250_000] {
        for cadence_ms in [1usize, 16, 100] {
            let mut parser = ToolCallArgumentsStream::new();
            parser.append(r#"{"s":""#);
            let data = "x".repeat(length);
            let mut calls = 0;
            let mut returned_characters = 0usize;
            let mut snapshot_ns = 0;
            let mut retained = None;
            let started = Instant::now();
            for (index, part) in data.as_bytes().chunks(32).enumerate() {
                parser.append(std::str::from_utf8(part).unwrap());
                if (index + 1) % cadence_ms == 0 {
                    let start = Instant::now();
                    let value = parser.snapshot();
                    snapshot_ns += start.elapsed().as_nanos();
                    assert_eq!(
                        value["s"].as_str().unwrap().len(),
                        ((index + 1) * 32).min(length)
                    );
                    returned_characters += value["s"].as_str().unwrap().len();
                    calls += 1;
                    retained.get_or_insert(value);
                }
            }
            parser.append("\"}");
            let start = Instant::now();
            let last = parser.snapshot();
            snapshot_ns += start.elapsed().as_nanos();
            let elapsed_ns = started.elapsed().as_nanos();
            assert_eq!(last["s"], data);
            assert!(parser.finish().1.is_none());
            assert_eq!(retained.unwrap()["s"], "x".repeat(32 * cadence_ms));
            calls += 1;
            returned_characters += length;
            assert_eq!(calls, length.div_ceil(32) / cadence_ms + 1);
            rows.push(
                json!({"length":length,"chunk":32,"arrival_ms":1,"cadence_ms":cadence_ms,
                "snapshot_calls":calls,"returned_string_characters":returned_characters,
                "snapshot_ns":snapshot_ns,"elapsed_ns":elapsed_ns}),
            );
        }
    }
    println!(
        "\n{}",
        json!({"cadence_observations":rows,
        "scope":"virtual arrivals and owned output volume, including final flush; no actual UI timing, allocator measurement or performance budget"})
    );
}

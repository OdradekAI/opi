//! Early ordinary external-consumer proof for the opi-vendor
//! Unicode16/ECMA/URL closure (task 20.8, DEL-04; reused at 20.57).
//!
//! Battery:
//! 1. 43 frozen leaf cases (14 ECMAScript Number::toString boundaries via
//!    ryu-js 1.0.2 safe Buffer API, 29 WHATWG URL.canParse accept/reject via
//!    the family `url`), compared against the frozen research expectations
//!    (Node 24.13.0 reference, 43/43 agreement).
//! 2. Unicode-16 ECMA regex probes: the `opi-vendor-regress` fork must
//!    accept UCD-16 property escapes and reject UCD-17-only ones, while
//!    stock regress 0.12.0 (ordinary crates.io dependency, same graph)
//!    accepts the UCD-17-only ones — a live differential between the two
//!    Unicode table generations in one dependency graph.
//! 3. Coexistence: the ordinary url 2.5.8/idna_adapter 1.2.2/ICU 2.2 chain
//!    and the family URL/IDNA/ICU 2.0 chain both parse the same inputs in
//!    this single graph.

use serde_json::{json, Value};

fn leaf_cases() -> Vec<Value> {
    let doc: Value = serde_json::from_str(include_str!("../frozen/leaf-cases.json"))
        .expect("frozen leaf cases parse");
    doc["cases"].as_array().expect("cases array").clone()
}

fn leaf_expected() -> Vec<Value> {
    serde_json::from_str::<Vec<Value>>(include_str!("../frozen/leaf-expected.json"))
        .expect("frozen leaf expectations parse")
}

fn unicode_probes() -> Value {
    serde_json::from_str(include_str!("../frozen/unicode16-probes.json"))
        .expect("frozen unicode16 probes parse")
}

fn run_leaf(value: &str, kind: &str) -> Value {
    if kind == "number-string" {
        let number: f64 = value.parse().expect("case value parses as f64");
        json!(ryu_js::Buffer::new().format(number).to_string())
    } else {
        json!(url16::Url::parse(value).is_ok())
    }
}

fn main() {
    let mut report = json!({});
    let mut pass = true;

    // 1. Leaf battery -----------------------------------------------------
    let cases = leaf_cases();
    let expected = leaf_expected();
    assert_eq!(cases.len(), 43, "frozen leaf case count");
    assert_eq!(expected.len(), 43, "frozen leaf expectation count");
    let mut matched = 0usize;
    let mut mismatches = Vec::new();
    for (case, want) in cases.iter().zip(expected.iter()) {
        let id = case["id"].as_str().expect("case id");
        let value = case["value"].as_str().expect("case value");
        let kind = case["kind"].as_str().expect("case kind");
        let got = run_leaf(value, kind);
        if got == want["result"] {
            matched += 1;
        } else {
            pass = false;
            mismatches.push(json!({"id": id, "expected": want["result"], "actual": got}));
        }
    }
    report["leaf"] = json!({"matched": matched, "total": 43, "mismatches": mismatches});
    if matched != 43 {
        pass = false;
    }

    // 2. Unicode 16 / Unicode 17 differential ------------------------------
    // Script-class match probes: classes added in UCD 17 must match their
    // code points under stock regress and not under the fork; shared
    // scripts must match under both. Compile-acceptance alone is not a
    // discriminator (unknown script value names compile to empty classes).
    let probes = unicode_probes();
    let script_probes: Vec<(String, u32, bool, bool)> = probes["script_match_probes"]
        .as_array().expect("script probes")
        .iter()
        .map(|p| (
            p["pattern"].as_str().expect("pattern").to_string(),
            p["code_point"].as_u64().expect("code point") as u32,
            p["stock_matches"].as_bool().expect("stock expectation"),
            p["fork_matches"].as_bool().expect("fork expectation"),
        ))
        .collect();
    let mut probe_failures = Vec::new();
    let mut stock_agreed = 0usize;
    let mut fork_agreed = 0usize;
    for (pattern, cp, want_stock, want_fork) in &script_probes {
        let sample = char::from_u32(*cp).expect("valid code point").to_string();
        let subject = "^\\p{Script=".to_string(); // unused; pattern is complete
        let _ = subject;
        let stock_regex = regress::Regex::with_flags(pattern.as_str(), "u")
            .expect("stock compiles script pattern");
        // A UCD-17-only name may fail to compile against the UCD-16 tables;
        // either way the fork must not match the UCD-17 code point.
        let fork_regex = regress16::Regex::with_flags(pattern.as_str(), "u");
        let fork_match = match &fork_regex {
            Ok(regex) => regex.find(&sample).is_some(),
            Err(_) => false,
        };
        let stock_match = stock_regex.find(&sample).is_some();
        if stock_match == *want_stock {
            stock_agreed += 1;
        } else {
            probe_failures.push(json!({"probe": pattern, "issue": "stock disagreed"}));
        }
        if fork_match == *want_fork {
            fork_agreed += 1;
        } else {
            probe_failures.push(json!({"probe": pattern, "issue": "fork disagreed"}));
        }
    }
    // The fork must still execute ordinary case-insensitive patterns
    // against the regenerated tables.
    let exec = regress16::Regex::with_flags("abc", "i").expect("case-insensitive pattern compiles");
    if exec.find("ABC").is_none() {
        probe_failures.push(json!({"pattern": "abc/i vs ABC", "issue": "case-insensitive fold match lost"}));
    }
    if !probe_failures.is_empty() {
        pass = false;
    }
    report["unicode16"] = json!({
        "script_probes": script_probes.len(),
        "stock_agreed": stock_agreed,
        "fork_agreed": fork_agreed,
        "failures": probe_failures,
    });

    // 3. Coexistence of the two URL/IDNA/ICU chains in one graph -----------
    let idn = "https://\u{4f8b}\u{3048}.\u{30c6}\u{30b9}\u{30c8}/\u{30d1}\u{30b9}";
    let family_ok = url16::Url::parse(idn).is_ok();
    let ordinary_ok = ordinary_url::Url::parse(idn).is_ok();
    // Touch the ordinary adapter so its presence is exercised, not just resolved.
    let _ordinary_adapter = consumer_idna_adapter::Adapter::new();
    report["coexistence"] = json!({
        "family_url_unicode_domain": family_ok,
        "ordinary_url_unicode_domain": ordinary_ok,
    });
    if !family_ok || !ordinary_ok {
        pass = false;
    }

    report["verdict"] = if pass { Value::from("PASS") } else { Value::from("FAIL") };
    println!("{}", serde_json::to_string_pretty(&report).expect("report serializes"));
    if !pass {
        std::process::exit(1);
    }
}

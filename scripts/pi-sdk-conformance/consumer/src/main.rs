//! Offline external consumer of the packaged public Schema seam. Frozen
//! observations remain intact; the current data-profile overlay is explicit.

use opi_ai::schema::{SchemaError, ToolArgumentOutcome, compile, validate_tool_arguments};
use serde_json::{Value, json};

fn check(schema: Value, value: Value, expected: bool) -> bool {
    compile(schema).and_then(|schema| schema.check(&value)) == Ok(expected)
}

fn record(checks: &mut Vec<Value>, name: &str, passed: bool) {
    checks.push(json!({"name": name, "passed": passed}));
}

fn main() {
    let frozen: Value = serde_json::from_str(include_str!("../frozen/leaf-cases.json")).unwrap();
    let expected: Vec<Value> =
        serde_json::from_str(include_str!("../frozen/leaf-expected.json")).unwrap();
    let profile: Value = serde_json::from_str(include_str!("../current-profile.json")).unwrap();
    let scripts: Value =
        serde_json::from_str(include_str!("../frozen/unicode16-probes.json")).unwrap();
    let mut checks = Vec::new();

    // Exercise the public SDK for all 43 original leaf cases, rather than only
    // proving that its leaf dependencies work in isolation.
    let string_schema = compile(json!({"type":"string"})).unwrap();
    let url_schema = compile(json!({"type":"string", "format":"url"})).unwrap();
    let cases = frozen["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 43);
    assert_eq!(expected.len(), cases.len());
    for (case, historical) in cases.iter().zip(&expected) {
        assert_eq!(case["id"], historical["id"]);
        let name = case["id"].as_str().unwrap();
        let input = case["value"].as_str().unwrap();
        let override_value = &profile["leaf_overrides"][name]["result"];
        let want = if override_value.is_null() {
            &historical["result"]
        } else {
            override_value
        };
        let actual = if case["kind"] == "number-string" {
            let raw = json!(input.parse::<f64>().unwrap());
            match validate_tool_arguments(&string_schema, &raw, None).unwrap() {
                ToolArgumentOutcome::Valid { normalized } => normalized,
                other => panic!("number conversion failed: {other:?}"),
            }
        } else {
            json!(url_schema.check(&json!(input)).unwrap())
        };
        record(&mut checks, name, &actual == want);
    }

    // The old corpus records both backend outcomes. Its stock expectation is
    // the current one; fork observations are retained as historical evidence.
    for probe in scripts["script_match_probes"].as_array().unwrap() {
        let pattern = probe["pattern"].as_str().unwrap();
        let sample = char::from_u32(probe["code_point"].as_u64().unwrap() as u32)
            .unwrap()
            .to_string();
        let want = probe["stock_matches"].as_bool().unwrap();
        let ordinary = regress::Regex::with_flags(pattern, "u")
            .unwrap()
            .find(&sample)
            .is_some();
        record(
            &mut checks,
            pattern,
            ordinary == want
                && check(
                    json!({"type":"string", "pattern":pattern}),
                    json!(sample),
                    want,
                ),
        );
    }

    let letter = "\u{16ea0}";
    record(
        &mut checks,
        "new-letter",
        check(json!({"pattern":"^\\p{Letter}$"}), json!(letter), true),
    );
    record(
        &mut checks,
        "old-letter",
        check(json!({"pattern":"^\\p{Letter}$"}), json!("a"), true),
    );
    record(
        &mut checks,
        "non-letter",
        check(json!({"pattern":"^\\p{Letter}$"}), json!("1"), false),
    );
    record(
        &mut checks,
        "named-backref",
        check(json!({"pattern":r"^(?<n>a)\k<n>$"}), json!("aa"), true),
    );
    record(
        &mut checks,
        "named-backref-reject",
        check(json!({"pattern":r"^(?<n>a)\k<n>$"}), json!("ab"), false),
    );
    record(
        &mut checks,
        "lookbehind",
        check(json!({"pattern":r"(?<=a)b"}), json!("ab"), true),
    );
    record(
        &mut checks,
        "unvisited-regex-admission",
        matches!(
            compile(json!({"anyOf":[true,{"pattern":"("}]})),
            Err(SchemaError::Regex { .. })
        ),
    );
    record(
        &mut checks,
        "inert-annotation",
        check(json!({"metadata":{"pattern":"["}}), json!({}), true),
    );
    record(
        &mut checks,
        "no-progress",
        matches!(
            compile(json!({"$id":"a/", "$ref":"#"})),
            Err(SchemaError::NoProgress)
        ),
    );

    let admitted = compile(json!({"type":"object", "properties":{"p":{"type":"string", "pattern":"^\\p{Script=Beria_Erfe}$"}}})).unwrap();
    let raw = json!({"p":null});
    record(
        &mut checks,
        "optional-null-after-successful-compile",
        matches!(validate_tool_arguments(&admitted, &raw, None).unwrap(), ToolArgumentOutcome::Valid { normalized } if normalized == json!({})),
    );
    record(&mut checks, "raw-preserved", raw == json!({"p":null}));
    record(
        &mut checks,
        "parse-failure-precedes-normalization",
        validate_tool_arguments(&admitted, &raw, Some("parse failure")).unwrap()
            == ToolArgumentOutcome::ArgumentsError,
    );
    let required = compile(json!({"type":"object", "properties":{"p":{"type":"string", "pattern":"^\\p{Script=Beria_Erfe}$"}}, "required":["p"]})).unwrap();
    record(
        &mut checks,
        "required-null-rejected",
        matches!(
            validate_tool_arguments(&required, &raw, None).unwrap(),
            ToolArgumentOutcome::Invalid { .. }
        ),
    );
    let bounded = compile(json!({"type":"integer", "minimum":5})).unwrap();
    record(
        &mut checks,
        "final-original-schema-check",
        matches!(
            validate_tool_arguments(&bounded, &json!("3"), None).unwrap(),
            ToolArgumentOutcome::Invalid { .. }
        ),
    );

    let raw_id = format!("http://{letter}.example/r");
    let ace_id = "http://xn--zs0f.example/r";
    let percent_id = "http://%F0%96%BA%A0.example/r";
    for (label, reference, target) in [
        ("raw-to-ace", raw_id.as_str(), ace_id),
        ("ace-to-raw", ace_id, raw_id.as_str()),
        ("percent-to-ace", percent_id, ace_id),
    ] {
        let schema =
            compile(json!({"$ref":reference, "metadata":{"leaf":{"$id":target,"const":3}}}))
                .unwrap();
        record(
            &mut checks,
            label,
            schema.check(&json!(3)) == Ok(true) && schema.check(&json!(4)) == Ok(false),
        );
    }
    record(
        &mut checks,
        "ordinary-url-canonical-idna",
        url::Url::parse(&raw_id).unwrap().as_str() == ace_id,
    );
    for (label, input) in [
        ("invalid-unmapped-ace", "http://xn--7r0f.example/"),
        ("invalid-url", "http://["),
        ("invalid-idna-joiner", "http://a\u{200d}.example"),
    ] {
        record(
            &mut checks,
            label,
            url_schema.check(&json!(input)) == Ok(false) && url::Url::parse(input).is_err(),
        );
    }
    for format in ["idn-hostname", "idn-email"] {
        let value = if format == "idn-email" {
            format!("a@{letter}.example")
        } else {
            format!("{letter}.example")
        };
        record(
            &mut checks,
            format,
            check(
                json!({"type":"string", "format":format}),
                json!(value),
                true,
            ),
        );
    }

    let failed = checks
        .iter()
        .filter(|check| check["passed"] != true)
        .count();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "verdict":if failed == 0 {"PASS"} else {"FAIL"},
            "total":checks.len(), "failed":failed, "checks":checks
        }))
        .unwrap()
    );
    if failed != 0 {
        std::process::exit(1);
    }
}

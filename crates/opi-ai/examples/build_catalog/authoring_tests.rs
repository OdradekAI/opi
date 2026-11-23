use super::{Bundle, authoring};
use serde_json::{Value, json};
use std::fs;

fn record(provider: &str, operation: &str, api: &str) -> Value {
    let mut row = json!({
        "id": "sample", "provider": provider, "type": operation, "api": api,
        "name": "Sample", "baseUrl": "https://example.invalid/v1", "input": ["text"],
        "cost": {"input": 0, "output": 0, "cacheRead": 0, "cacheWrite": 0}
    });
    match operation {
        "chat" => {
            row["reasoning"] = json!(false);
            row["maxTokens"] = json!(1024);
            row["contextWindow"] = json!(8192);
        }
        "image" => row["output"] = json!(["image"]),
        "classifier" => row["contextWindow"] = json!(8192),
        _ => unreachable!(),
    }
    row
}

fn build(row: Value) -> Result<Bundle, String> {
    let root = tempfile::tempdir().unwrap();
    let path = root
        .path()
        .join(format!("{}.json", row["provider"].as_str().unwrap()));
    fs::write(path, serde_json::to_vec(&[row]).unwrap()).unwrap();
    Bundle::build(root.path()).map_err(|e| e.to_string())
}

#[test]
fn authoring_reports_the_identity_and_field_of_each_bad_declaration() {
    for (field, value, path) in [
        ("api", json!("misspelled-chat"), "api"),
        ("compat", json!(false), "compat"),
        (
            "compat",
            json!({"supportsStricMode": true}),
            "compat.supportsStricMode",
        ),
        (
            "compat",
            json!({"supportsStrictMode": "yes"}),
            "compat.supportsStrictMode",
        ),
        (
            "compat",
            json!({"supportsCacheControlOnTools":true}),
            "compat.supportsCacheControlOnTools",
        ),
        (
            "compat",
            json!({"thinkingFormat": 7}),
            "compat.thinkingFormat",
        ),
        ("unsupportedMetadata", json!(true), "unsupportedMetadata"),
        (
            "compat",
            json!({"maxTokensField": "model"}),
            "compat.maxTokensField",
        ),
        ("x-opi", json!({"streaming":false}), "x-opi"),
        (
            "inputLimits",
            json!({"images":{"resize":{"jpegQualty": 80}}}),
            "inputLimits.images.resize.jpegQualty",
        ),
        ("promptCache", json!({"short": "300"}), "promptCache.short"),
        ("thinkingLevelMap", json!("high"), "thinkingLevelMap"),
    ] {
        let mut row = record("groq", "chat", "openai-completions");
        row[field] = value;
        let error = build(row).err().expect("invalid authored record");
        assert!(
            error.contains("groq/chat:sample") && error.contains(path),
            "{error}"
        );
    }
    let mut row = record("mistral", "chat", "mistral-conversations");
    row["thinkingLevelMap"] = json!({"low": "loow"});
    let error = build(row)
        .err()
        .expect("unknown effort would silently send high");
    assert!(
        error.contains("mistral/chat:sample") && error.contains("thinkingLevelMap.low"),
        "{error}"
    );
}

#[test]
fn authoring_accepts_explicit_metadata_and_routes_only_registered_operations() {
    for (provider, operation, api, allowed) in [
        ("openai", "classifier", "openai-decisions", true),
        ("groq", "classifier", "openai-decisions", false),
        ("openrouter", "image", "openrouter-images", true),
        ("groq", "image", "openrouter-images", false),
        ("typesafe", "chat", "openai-completions", false),
        ("groq", "chat", "bedrock-converse-stream", false),
        ("radius", "chat", "openai-completions", false),
        ("unknown-new-provider", "chat", "openai-completions", false),
    ] {
        let row = record(provider, operation, api);
        assert_eq!(build(row).is_ok(), allowed, "{provider}/{operation}/{api}");
    }
    let mut row = record("groq", "chat", "openai-completions");
    row["compat"] = json!({"supportsStrictMode":null, "vllmPriority":0.125});
    row["x-example"] = json!({"new": [2,null]});
    let bundle = build(row.clone()).unwrap();
    let catalog = opi_ai::catalog::Catalog::from_bytes(&bundle.catalog, &bundle.manifest).unwrap();
    assert_eq!(
        catalog
            .model("groq", opi_ai::catalog::CatalogOperation::Chat, "sample")
            .unwrap()
            .raw,
        row
    );
}

#[test]
fn openrouter_classifier_declarations_match_its_image_encoding() {
    let mut row = record("openrouter", "classifier", "typesafe-system-one");
    row["compat"] = json!({"imageInput": "state-image-urls", "sendImageOptInHeader": false});
    for input in [json!(["text"]), json!(["text", "image"])] {
        row["input"] = input;
        assert!(build(row.clone()).is_ok());
    }
    for (key, value) in [
        ("imageInput", json!("assets")),
        ("sendImageOptInHeader", json!(true)),
    ] {
        let mut invalid = row.clone();
        invalid["compat"][key] = value;
        let error = build(invalid).err().expect("unsupported image encoding");
        assert!(
            error.contains("openrouter/classifier:sample")
                && error.contains(&format!("compat.{key}")),
            "{error}"
        );
    }
}

#[test]
fn typesafe_classifier_declarations_match_its_asset_encoding() {
    let mut row = record("typesafe", "classifier", "typesafe-system-one");
    row["compat"] = json!({"imageInput": "assets", "sendImageOptInHeader": true});
    row["input"] = json!(["text", "image"]);
    assert!(build(row.clone()).is_ok());
    for (key, value) in [
        ("imageInput", json!("state-image-urls")),
        ("sendImageOptInHeader", json!(false)),
    ] {
        let mut invalid = row.clone();
        invalid["compat"][key] = value;
        let error = build(invalid).err().expect("unsupported image encoding");
        assert!(
            error.contains("typesafe/classifier:sample")
                && error.contains(&format!("compat.{key}")),
            "{error}"
        );
    }
}

#[test]
fn direct_profile_follows_removal_from_the_provider_source() {
    let mut row = record("openrouter", "chat", "openai-completions");
    row["id"] = json!("openai/gpt-4o");
    let before = build(row.clone()).unwrap().openrouter_profile.unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&before).unwrap(),
        json!([row])
    );

    row["id"] = json!("unselected-model");
    let after = build(row).unwrap().openrouter_profile.unwrap();
    assert_eq!(serde_json::from_slice::<Value>(&after).unwrap(), json!([]));
}

#[test]
fn authoring_rejects_repeated_json_keys_at_any_depth() {
    for (source, key) in [
        (r#"[{"id":"a","id":"b"}]"#, "id"),
        (
            r#"[{"compat":{"supportsStrictMode":true,"supportsStrictMode":false}}]"#,
            "supportsStrictMode",
        ),
        (r#"[{"x-proof":[{"\u0061":0,"a":1}]}]"#, "a"),
    ] {
        let error = authoring::parse(source.as_bytes()).unwrap_err().to_string();
        assert!(
            error.contains(&format!("duplicate JSON member '{key}'")),
            "{error}"
        );
    }
}

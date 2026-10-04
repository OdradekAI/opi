use std::process::Command;

#[test]
fn offline_native_projection_uses_dummy_auth_and_rejects_remote_listener() {
    let root = tempfile::tempdir().unwrap();
    let config = root.path().join("experiment.toml");
    let source = include_str!("fixtures/experiment/minimal.toml")
        .replace("reasoning = \"omitted\"", "reasoning = \"high\"")
        .replace("max_output_tokens = 4096", "max_output_tokens = 131072");
    std::fs::write(&config, source).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .args([
            "__native-control-config",
            "--product",
            "pi",
            "--base-url",
            "http://127.0.0.1:49123",
        ])
        .arg("--config")
        .arg(&config)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["thinking_level"], "high");
    let bytes = value["config"].as_str().unwrap();
    assert!(bytes.contains("dummy-scripted-credential"));
    let projected: serde_json::Value = serde_json::from_str(bytes).unwrap();
    assert!(
        projected["providers"]
            .as_object()
            .unwrap()
            .values()
            .all(|p| p["baseUrl"] == "http://127.0.0.1:49123")
    );
    let rejected = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .args([
            "__native-control-config",
            "--product",
            "pi",
            "--base-url",
            "https://remote.example",
        ])
        .arg("--config")
        .arg(&config)
        .output()
        .unwrap();
    assert!(!rejected.status.success());
    assert!(rejected.stdout.is_empty());
}

//! Trust-gating regression coverage for config-consuming early commands.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn isolated_command(root: &Path, args: &[&str]) -> Output {
    let user_home = root.join("user-home");
    let app_data = root.join("app-data");
    let xdg_config = root.join("xdg-config");
    std::fs::create_dir_all(&user_home).unwrap();
    std::fs::create_dir_all(&app_data).unwrap();
    std::fs::create_dir_all(&xdg_config).unwrap();

    Command::new(env!("CARGO_BIN_EXE_opi"))
        .args(args)
        .current_dir(root)
        .env("HOME", &user_home)
        .env("USERPROFILE", &user_home)
        .env("APPDATA", &app_data)
        .env("XDG_CONFIG_HOME", &xdg_config)
        .env("PHASE15_TEST_KEY", "test-key")
        .env_remove("OPI_MODEL")
        .env_remove("OPI_TRUST")
        .env_remove("OPI_CONFIG")
        .output()
        .unwrap()
}

fn combined(output: &Output) -> String {
    format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn project_config(root: &Path, contents: &str) -> PathBuf {
    let directory = root.join(".opi");
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("config.toml");
    std::fs::write(&path, contents).unwrap();
    path
}

fn custom_provider_config(marker: &str) -> String {
    format!(
        r#"
[providers.custom.phase15]
base_url = "https://phase15.invalid"
api_key_env = "PHASE15_TEST_KEY"
auth_scheme = "bearer"

[[providers.custom.phase15.models]]
id = "{marker}"
api = "openai-completions"
context_window = 8192
max_output_tokens = 1024
"#
    )
}

#[test]
fn doctor_trust_preflight_gates_project_config_but_not_explicit_config() {
    let root = tempfile::tempdir().unwrap();
    let malformed = project_config(root.path(), "[providers.custom.\n");

    for args in [
        &["--no-trust", "doctor", "--scope", "config", "--json"][..],
        &["doctor", "--scope", "config", "--json"][..],
    ] {
        let output = isolated_command(root.path(), args);
        assert!(
            !combined(&output).contains("config_parse_failed"),
            "untrusted project parse error leaked for {args:?}: {}",
            combined(&output)
        );
    }

    let trusted = isolated_command(
        root.path(),
        &["--trust", "doctor", "--scope", "config", "--json"],
    );
    assert!(
        !trusted.status.success() && combined(&trusted).contains("config_parse_failed"),
        "trusted doctor must consume malformed project config: {}",
        combined(&trusted)
    );

    let explicit = isolated_command(
        root.path(),
        &[
            "--no-trust",
            "--config",
            malformed.to_str().unwrap(),
            "doctor",
            "--scope",
            "config",
            "--json",
        ],
    );
    assert!(
        !explicit.status.success() && combined(&explicit).contains("config_parse_failed"),
        "explicit config must remain authorized under --no-trust: {}",
        combined(&explicit)
    );
}

#[test]
fn list_models_trust_preflight_gates_project_and_preserves_user_config() {
    let root = tempfile::tempdir().unwrap();
    let project_marker = "project-trust-marker";
    project_config(root.path(), &custom_provider_config(project_marker));

    for args in [
        &["--no-trust", "--list-models", "--json"][..],
        &["--list-models", "--json"][..],
    ] {
        let output = isolated_command(root.path(), args);
        assert!(
            output.status.success(),
            "list-models failed: {}",
            combined(&output)
        );
        assert!(
            !combined(&output).contains(project_marker),
            "untrusted project provider leaked for {args:?}: {}",
            combined(&output)
        );
    }

    let trusted = isolated_command(root.path(), &["--trust", "--list-models", "--json"]);
    assert!(
        trusted.status.success(),
        "list-models failed: {}",
        combined(&trusted)
    );
    assert!(
        combined(&trusted).contains(project_marker),
        "trusted project provider was not consumed: {}",
        combined(&trusted)
    );

    let user_marker = "user-config-marker";
    let user_config = root.path().join("app-data").join("opi").join("config.toml");
    std::fs::create_dir_all(user_config.parent().unwrap()).unwrap();
    std::fs::write(&user_config, custom_provider_config(user_marker)).unwrap();
    let user = isolated_command(root.path(), &["--no-trust", "--list-models", "--json"]);
    assert!(
        user.status.success(),
        "list-models failed: {}",
        combined(&user)
    );
    assert!(
        combined(&user).contains(user_marker),
        "global user config must remain active: {}",
        combined(&user)
    );
    assert!(!combined(&user).contains(project_marker));
}

//! Phase 15 task 15.8.1 integration tests: non-interactive headless trust
//! resolution.
//!
//! Non-interactive startup never prompts. An unresolved ask resolves to
//! untrusted, `--trust`/`--no-trust` and `[defaults] default_project_trust`
//! override it, and the resulting decision feeds the 15.7 two-stage config gate
//! (project `.opi/config.toml` skipped when untrusted). The headless
//! pre-trust UI is immediately unavailable and never blocks.

use opi_coding_agent::config::{ConfigSource, merge_project_config, resolve_pre_trust_config};
use opi_coding_agent::project_trust::{
    HeadlessPreTrustUi, PreTrustUi, PreTrustUiError, ProjectTrustCli, ProjectTrustDefault,
    ProjectTrustResolverRegistry, ProjectTrustStore, TrustDecision, prepare_project_startup,
};

fn init_git(dir: &std::path::Path) {
    std::fs::create_dir_all(dir.join(".git")).expect("create .git marker");
}

fn write_project_config(workspace: &std::path::Path) {
    std::fs::create_dir_all(workspace.join(".opi")).expect("create .opi");
    std::fs::write(
        workspace.join(".opi").join("config.toml"),
        "[providers.bedrock]\nprofile = \"proj-aws-profile\"\n",
    )
    .expect("write project config");
}

fn config_source(workspace: &std::path::Path, global: &std::path::Path) -> ConfigSource {
    ConfigSource {
        cli_model: None,
        config_path: None,
        env_model: None,
        project_dir: Some(workspace.to_path_buf()),
        user_config_path: Some(global.join("config.toml")),
    }
}

/// Replicate the production headless decision + two-stage config merge using the
/// public preflight + config symbols (the same composition `main` performs).
async fn headless_startup(
    cli: ProjectTrustCli,
    global_default: TrustDecision,
    workspace: &std::path::Path,
    user_config: &std::path::Path,
) -> (opi_coding_agent::config::OpiConfig, TrustDecision) {
    let pre = resolve_pre_trust_config(config_source(workspace, user_config)).unwrap();
    let mut registry = ProjectTrustResolverRegistry::new();
    let plan = prepare_project_startup(
        cli,
        &mut registry,
        user_config,
        workspace,
        global_default,
        &HeadlessPreTrustUi,
    )
    .await
    .unwrap();
    let decision = plan.headless_decision();
    let config = if matches!(decision, TrustDecision::Untrusted) {
        pre
    } else {
        merge_project_config(pre, workspace).unwrap()
    };
    (config, decision)
}

#[tokio::test]
async fn headless_ask_defaults_untrusted_with_overrides() {
    let user_config = tempfile::tempdir().unwrap();
    std::fs::write(user_config.path().join("config.toml"), "").unwrap();
    let workspace = tempfile::tempdir().unwrap();
    init_git(workspace.path());
    write_project_config(workspace.path());

    // Default ask + no prior decision + no override -> Untrusted, and the
    // project `.opi/config.toml` (bedrock profile) is NOT merged.
    let (config, decision) = headless_startup(
        ProjectTrustCli::default(),
        TrustDecision::Undecided,
        workspace.path(),
        user_config.path(),
    )
    .await;
    assert_eq!(decision, TrustDecision::Untrusted);
    assert_eq!(
        config.providers.bedrock.profile, None,
        "untrusted project config must be skipped on the headless path"
    );

    // --trust overrides to Trusted; the project config IS merged.
    let (config, decision) = headless_startup(
        ProjectTrustCli {
            trust: true,
            no_trust: false,
        },
        TrustDecision::Undecided,
        workspace.path(),
        user_config.path(),
    )
    .await;
    assert_eq!(decision, TrustDecision::Trusted);
    assert_eq!(
        config.providers.bedrock.profile.as_deref(),
        Some("proj-aws-profile"),
        "trusted project config must be merged"
    );

    // [defaults] default_project_trust = always -> Trusted (read from the
    // global/pre-trust config, so a project cannot self-authorize). Run before
    // the store is polluted so the global default is the decider.
    let (_config, decision) = headless_startup(
        ProjectTrustCli::default(),
        ProjectTrustDefault::Always.to_decision(),
        workspace.path(),
        user_config.path(),
    )
    .await;
    assert_eq!(decision, TrustDecision::Trusted);

    // [defaults] default_project_trust = never -> Untrusted.
    let (_config, decision) = headless_startup(
        ProjectTrustCli::default(),
        ProjectTrustDefault::Never.to_decision(),
        workspace.path(),
        user_config.path(),
    )
    .await;
    assert_eq!(decision, TrustDecision::Untrusted);

    // A stored allow is authoritative at precedence position 3; only a CLI
    // --no-trust override (position 1) beats it.
    let store = ProjectTrustStore::load(user_config.path()).unwrap();
    store.record(workspace.path(), true).unwrap();
    let (_config, decision) = headless_startup(
        ProjectTrustCli::default(),
        TrustDecision::Undecided,
        workspace.path(),
        user_config.path(),
    )
    .await;
    assert_eq!(
        decision,
        TrustDecision::Trusted,
        "stored allow decides over the ask default"
    );
    let (_config, decision) = headless_startup(
        ProjectTrustCli {
            trust: false,
            no_trust: true,
        },
        TrustDecision::Undecided,
        workspace.path(),
        user_config.path(),
    )
    .await;
    assert_eq!(
        decision,
        TrustDecision::Untrusted,
        "--no-trust beats a stored allow"
    );
}

#[tokio::test]
async fn headless_pre_trust_ui_is_immediately_unavailable() {
    // select/confirm/input return Unavailable immediately; notify is a tested
    // no-op (completes without panicking). No method ever blocks.
    let ui = HeadlessPreTrustUi;
    assert_eq!(
        ui.select("q", &["a", "b"]).await,
        Err(PreTrustUiError::Unavailable)
    );
    assert_eq!(ui.confirm("q?").await, Err(PreTrustUiError::Unavailable));
    assert_eq!(ui.input("q").await, Err(PreTrustUiError::Unavailable));
    // notify returns unit (no-op) — exercised here.
    ui.notify("informational").await;
}

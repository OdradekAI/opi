//! Integration contract tests for the opi-eval experiment seam.
//!
//! Covers the Independent Companion dependency boundary (EVAL-A01) and the
//! canonical, fail-closed resolution behavior of
//! [`opi_eval::experiment::ResolvedExperiment`].

use std::path::Path;
use std::process::Command;

use opi_eval::cli;
use opi_eval::experiment::{
    CapturePolicy, ConnectionApi, ControlValue, CredentialReference, EXPERIMENT_SCHEMA,
    ExecutionMode, ResolveError, ResolvedExperiment,
};

const MINIMAL_FIXTURE: &str = include_str!("fixtures/experiment/minimal.toml");

fn manifest_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn minimal_fixture_path() -> std::path::PathBuf {
    manifest_dir().join("tests/fixtures/experiment/minimal.toml")
}

fn generic_fixture_path() -> std::path::PathBuf {
    manifest_dir().join("tests/fixtures/experiment/generic-three-subject-fourth-benchmark.toml")
}

const THREE_SUBJECT_DOC: &str = r#"
schema = "opi-eval-experiment/2"
experiment_id = "three-subject-generic"
connection = "mock-local"

[benchmark]
name = "fourth-descriptor"
revision = "9"
dataset = "example/dataset"

[[subjects]]
id = "alpha"
product = "product-a"
version = "1.0.0"

[[subjects]]
id = "beta"
product = "product-b"
version = "2.0.0"

[[subjects]]
id = "gamma"
product = "product-c"
version = "3.0.0"

[[edges]]
id = "alpha-vs-beta"
baseline = "alpha"
candidate = "beta"

[[edges]]
id = "beta-vs-gamma"
baseline = "beta"
candidate = "gamma"

[[connections]]
id = "mock-local"
provider = "mock"
api = "openai-completions"
base_url = "http://127.0.0.1:48127/v1"
model = "mock-model"
credential_env = "OPI_EVAL_FIXTURE_KEY"

[model_controls]
temperature = "unknown"
top_p = "omitted"
max_output_tokens = "omitted"
context_window = 8192
reasoning = "high"

[storage]
cache = "cache"
results = "results"
capture_policy = "full"

[environment]
platform = "linux"
architecture = "aarch64"
cwd_policy = "workspace-root"

[[trials]]
id = "t-alpha-1"
subject = "alpha"
task = "task-1"
group = "g1"

[[trials]]
id = "t-beta-1"
subject = "beta"
task = "task-1"
group = "g1"

[[trials]]
id = "t-gamma-2"
subject = "gamma"
task = "task-2"
group = "g2"
"#;

/// EVAL-A01: with the Opi workspace present, the Companion has no Opi crate
/// dependency in any dependency table and nothing in the lock graph gives an
/// Opi product a reverse edge to `opi-eval`. The scenario's `cargo tree`
/// commands provide the full transitive proof; this test pins the local
/// manifest and lockfile boundary the package itself owns.
#[test]
fn independent_companion_dependency_boundary() {
    let manifest_text = std::fs::read_to_string(manifest_dir().join("Cargo.toml")).unwrap();
    let manifest: toml::Value = toml::from_str(&manifest_text).unwrap();

    let package = manifest.get("package").unwrap();
    assert_eq!(package.get("name").unwrap().as_str(), Some("opi-eval"));
    assert_eq!(
        package.get("publish").and_then(|v| v.as_bool()),
        Some(false),
        "the Companion must stay publish-disabled"
    );

    let mut opi_edges: Vec<String> = Vec::new();
    fn collect_opi_deps(table: &toml::Value, table_name: &str, out: &mut Vec<String>) {
        if let Some(deps) = table.as_table() {
            for (key, value) in deps {
                let package_name = value
                    .as_table()
                    .and_then(|t| t.get("package"))
                    .and_then(|p| p.as_str())
                    .unwrap_or(key.as_str());
                if package_name.starts_with("opi-") {
                    out.push(format!("{table_name}:{package_name}"));
                }
            }
        }
    }
    for table_name in ["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(table) = manifest.get(table_name) {
            collect_opi_deps(table, table_name, &mut opi_edges);
        }
    }
    if let Some(targets) = manifest.get("target").and_then(|t| t.as_table()) {
        for (target, tables) in targets {
            if let Some(tables) = tables.as_table() {
                for (kind, deps) in tables {
                    collect_opi_deps(deps, &format!("target.{target}.{kind}"), &mut opi_edges);
                }
            }
        }
    }
    assert!(
        opi_edges.is_empty(),
        "opi-eval must not depend on any Opi crate: {opi_edges:?}"
    );

    let lock_text = std::fs::read_to_string(manifest_dir().join("../../Cargo.lock")).unwrap();
    let lock: toml::Value = toml::from_str(&lock_text).unwrap();
    let packages = lock.get("package").and_then(|p| p.as_array()).unwrap();
    let entry = packages
        .iter()
        .find(|p| p.get("name").and_then(|n| n.as_str()) == Some("opi-eval"))
        .expect("Cargo.lock must contain opi-eval");
    let lock_deps: Vec<&str> = entry
        .get("dependencies")
        .and_then(|d| d.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();
    assert!(
        !lock_deps.iter().any(|d| d.starts_with("opi-")),
        "Cargo.lock gives opi-eval an Opi dependency: {lock_deps:?}"
    );

    for package in packages {
        let name = package.get("name").and_then(|n| n.as_str()).unwrap();
        if name == "opi-eval" {
            continue;
        }
        let deps: Vec<&str> = package
            .get("dependencies")
            .and_then(|d| d.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
            .unwrap_or_default();
        assert!(
            !deps.contains(&"opi-eval"),
            "reverse dependency on opi-eval from {name}"
        );
    }
}

#[test]
fn resolve_minimal_fixture_freezes_identity() {
    let resolved = ResolvedExperiment::resolve(MINIMAL_FIXTURE).unwrap();
    assert_eq!(resolved.schema(), EXPERIMENT_SCHEMA);
    assert_eq!(resolved.experiment_id(), "minimal-pairing");
    assert_eq!(resolved.manifest_digest().len(), 64);

    assert_eq!(resolved.benchmark().name, "terminal-bench");
    assert_eq!(resolved.benchmark().revision, "2.1");
    assert!(resolved.benchmark().integrity_digest.is_none());

    assert_eq!(resolved.subjects().len(), 2);
    assert_eq!(resolved.subjects()[0].id, "opi");
    assert_eq!(resolved.subjects()[1].id, "pi");

    assert_eq!(resolved.edges().len(), 1);
    assert_eq!(resolved.edges()[0].baseline, "opi");
    assert_eq!(resolved.edges()[0].candidate, "pi");

    let controls = resolved.model_controls();
    assert_eq!(controls.temperature, ControlValue::Value(0.0));
    assert_eq!(controls.top_p, ControlValue::Omitted);
    assert_eq!(controls.max_output_tokens, ControlValue::Value(4096));
    assert_eq!(controls.context_window, ControlValue::Value(8192));
    assert_eq!(controls.reasoning, ControlValue::Omitted);
    let connection = resolved.selected_connection();
    assert_eq!(connection.provider, "mock");
    assert_eq!(connection.model, "mock-model");
    assert_eq!(connection.api, ConnectionApi::OpenAiCompletions);

    assert_eq!(resolved.environment().platform, "linux");
    assert_eq!(resolved.environment().architecture, "x86_64");
    assert_eq!(resolved.environment().cwd_policy, "workspace-root");

    assert_eq!(resolved.trials().len(), 2);
    assert_eq!(resolved.trials()[0].id, "trial-opi-hello");
    assert_eq!(resolved.trials()[0].subject, "opi");
    assert_eq!(resolved.trials()[1].id, "trial-pi-hello");
}

#[test]
fn resolve_is_deterministic_and_digest_canonical() {
    let first = ResolvedExperiment::resolve(MINIMAL_FIXTURE).unwrap();
    let second = ResolvedExperiment::resolve(MINIMAL_FIXTURE).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.manifest_digest(), second.manifest_digest());

    // Formatting-only changes keep the canonical digest stable.
    let reformatted = MINIMAL_FIXTURE.replace('\n', "\n\n");
    let third = ResolvedExperiment::resolve(&reformatted).unwrap();
    assert_eq!(third.manifest_digest(), first.manifest_digest());

    // A semantic control change must change the digest.
    let mutated = MINIMAL_FIXTURE.replace("temperature = 0.0", "temperature = 0.5");
    let fourth = ResolvedExperiment::resolve(&mutated).unwrap();
    assert_ne!(fourth.manifest_digest(), first.manifest_digest());
}

#[test]
fn resolve_supports_n_subjects_and_directed_edges() {
    let resolved = ResolvedExperiment::resolve(THREE_SUBJECT_DOC).unwrap();
    assert_eq!(resolved.subjects().len(), 3);
    assert_eq!(resolved.edges().len(), 2);
    assert_eq!(resolved.trials().len(), 3);
    let controls = resolved.model_controls();
    assert_eq!(controls.temperature, ControlValue::Unknown);
    assert_eq!(controls.max_output_tokens, ControlValue::Omitted);
    assert_eq!(controls.reasoning, ControlValue::Value("high".to_owned()));
}

#[test]
fn generic_schema_resolves_without_product_hard_coding() {
    let path = generic_fixture_path();
    let summary =
        cli::validate(&path).expect("generic three-subject fourth-benchmark fixture must resolve");
    assert_eq!(summary.subject_count, 3);
    assert_eq!(summary.edge_count, 2);
    assert_eq!(summary.trial_count, 4);

    let source = std::fs::read_to_string(&path).unwrap();
    let resolved = ResolvedExperiment::resolve(&source).unwrap();
    let products: Vec<&str> = resolved
        .subjects()
        .iter()
        .map(|subject| subject.product.as_str())
        .collect();
    assert!(products.contains(&"future-agent"));
    assert_eq!(products.len(), 3);
    assert_eq!(resolved.benchmark().integrity_digest, None);

    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("validate")
        .arg("--config")
        .arg(path)
        .output()
        .expect("run opi-eval validate");
    assert!(
        output.status.success(),
        "validate CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("subjects=3"), "summary: {stdout}");
    assert!(stdout.contains("edges=2"), "summary: {stdout}");
}

fn assert_rejected(source: &str) -> ResolveError {
    ResolvedExperiment::resolve(source).expect_err("document must be rejected")
}

fn invalid_doc(tweak: fn(&mut String)) -> String {
    let mut source = MINIMAL_FIXTURE.to_owned();
    tweak(&mut source);
    source
}

#[test]
fn resolve_rejects_missing_or_unsupported_schema() {
    let error = assert_rejected(&invalid_doc(|s| {
        *s = s.replace("schema = \"opi-eval-experiment/2\"\n", "");
    }));
    assert!(matches!(error, ResolveError::UnsupportedSchema(_)));

    let error = assert_rejected(&invalid_doc(|s| {
        *s = s.replace("opi-eval-experiment/2", "opi-eval-experiment/3");
    }));
    assert!(matches!(error, ResolveError::UnsupportedSchema(_)));

    // The pre-P19-D02 shape is rejected without a compatibility alias.
    let error = assert_rejected(&invalid_doc(|s| {
        *s = s.replace("opi-eval-experiment/2", "opi-eval-experiment/1");
    }));
    assert!(matches!(error, ResolveError::UnsupportedSchema(_)));
}

#[test]
fn legacy_experiment_schema_is_rejected() {
    let legacy_schema = format!("{}{}{}", "phase", 18, "-experiment/1");
    let legacy = MINIMAL_FIXTURE.replace(EXPERIMENT_SCHEMA, &legacy_schema);
    let error = assert_rejected(&legacy);
    assert!(matches!(error, ResolveError::UnsupportedSchema(found) if found == legacy_schema));
}

#[test]
fn resolve_rejects_missing_experiment_id() {
    let error = assert_rejected(&invalid_doc(|s| {
        *s = s.replace("experiment_id = \"minimal-pairing\"", "");
    }));
    assert!(matches!(error, ResolveError::MissingField(f) if f.contains("experiment_id")));
}

#[test]
fn resolve_rejects_empty_subject_set_and_duplicate_subjects() {
    let no_subjects = THREE_SUBJECT_DOC.replace(
        &THREE_SUBJECT_DOC[THREE_SUBJECT_DOC.find("[[subjects]]").unwrap()
            ..THREE_SUBJECT_DOC.find("[[edges]]").unwrap()],
        "",
    );
    let error = assert_rejected(&no_subjects);
    assert!(matches!(error, ResolveError::MissingSubjects));

    let error = assert_rejected(&invalid_doc(|s| {
        *s = s.replace("id = \"pi\"", "id = \"opi\"");
    }));
    assert!(matches!(error, ResolveError::DuplicateSubject(id) if id == "opi"));
}

#[test]
fn resolve_rejects_edge_faults() {
    let error = assert_rejected(&invalid_doc(|s| {
        *s = s.replace("baseline = \"opi\"", "baseline = \"unknown-harness\"");
    }));
    assert!(matches!(
        error,
        ResolveError::UnknownEdgeEndpoint { edge, role: "baseline", subject }
        if edge == "opi-baseline-vs-pi-candidate" && subject == "unknown-harness"
    ));

    let error = assert_rejected(&invalid_doc(|s| {
        *s = s.replace("candidate = \"pi\"", "candidate = \"opi\"");
    }));
    assert!(matches!(error, ResolveError::SelfEdge(id) if id == "opi-baseline-vs-pi-candidate"));

    let error = assert_rejected(&invalid_doc(|s| {
        s.insert_str(0, "");
        *s = format!(
            "{s}\n\n[[edges]]\nid = \"duplicate-edge\"\nbaseline = \"opi\"\ncandidate = \"pi\"\n"
        );
    }));
    assert!(
        matches!(error, ResolveError::DuplicateEdge(pair) if pair.contains("opi") && pair.contains("pi"))
    );

    let no_edges = MINIMAL_FIXTURE.replace(
        &MINIMAL_FIXTURE[MINIMAL_FIXTURE.find("[[edges]]").unwrap()
            ..MINIMAL_FIXTURE.find("[model_controls]").unwrap()],
        "",
    );
    let error = assert_rejected(&no_edges);
    assert!(matches!(error, ResolveError::MissingEdges));
}

#[test]
fn resolve_rejects_implicit_or_malformed_model_controls() {
    let error = assert_rejected(&invalid_doc(|s| {
        *s = s.replace("temperature = 0.0", "");
    }));
    assert!(matches!(error, ResolveError::MissingModelControl(c) if c == "temperature"));

    let error = assert_rejected(&invalid_doc(|s| {
        *s = s.replace("temperature = 0.0", "temperature = \"sometimes\"");
    }));
    assert!(matches!(
        error,
        ResolveError::InvalidControlMarker { control, marker }
        if control == "temperature" && marker == "sometimes"
    ));

    let error = assert_rejected(&invalid_doc(|s| {
        *s = s.replace("max_output_tokens = 4096", "max_output_tokens = 0.5");
    }));
    assert!(matches!(
        error,
        ResolveError::InvalidControlValue { control, .. } if control == "max_output_tokens"
    ));

    let error = assert_rejected(&invalid_doc(|s| {
        *s = s.replace("provider = \"mock\"", "");
    }));
    assert!(matches!(error, ResolveError::MissingField(f) if f.contains("provider")));
}

#[test]
fn resolve_rejects_missing_environment_fields() {
    let error = assert_rejected(&invalid_doc(|s| {
        *s = s.replace("cwd_policy = \"workspace-root\"", "");
    }));
    assert!(matches!(error, ResolveError::MissingField(f) if f.contains("cwd_policy")));
}

#[test]
fn resolve_rejects_trial_faults() {
    let error = assert_rejected(&invalid_doc(|s| {
        *s = s.replace("id = \"trial-pi-hello\"", "id = \"trial-opi-hello\"");
    }));
    assert!(matches!(error, ResolveError::DuplicateTrial(id) if id == "trial-opi-hello"));

    let error = assert_rejected(&invalid_doc(|s| {
        *s = s.replace("subject = \"pi\"", "subject = \"mystery\"");
    }));
    assert!(matches!(
        error,
        ResolveError::UnknownTrialSubject { trial, subject }
        if trial == "trial-pi-hello" && subject == "mystery"
    ));

    let error = assert_rejected(&invalid_doc(|s| {
        *s = s.replace("task = \"hello\"", "task = \"\"");
    }));
    assert!(matches!(error, ResolveError::MissingField(f) if f.contains("task")));

    let no_trials = MINIMAL_FIXTURE[..MINIMAL_FIXTURE.find("[[trials]]").unwrap()].to_owned();
    let error = assert_rejected(&no_trials);
    assert!(matches!(error, ResolveError::MissingTrials));
}

#[test]
fn resolve_rejects_missing_benchmark_fields() {
    let error = assert_rejected(&invalid_doc(|s| {
        *s = s.replace("revision = \"2.1\"", "");
    }));
    assert!(matches!(error, ResolveError::MissingField(f) if f.contains("revision")));
}

#[test]
fn cli_validate_summarizes_the_minimal_fixture() {
    let summary = cli::validate(&minimal_fixture_path()).unwrap();
    assert_eq!(summary.experiment_id, "minimal-pairing");
    assert_eq!(summary.schema, EXPERIMENT_SCHEMA);
    assert_eq!(summary.subject_count, 2);
    assert_eq!(summary.edge_count, 1);
    assert_eq!(summary.trial_count, 2);
    assert_eq!(summary.manifest_digest.len(), 64);
    let rendered = summary.to_string();
    assert!(rendered.contains("minimal-pairing"));
    assert!(rendered.contains("subjects=2"));
}

#[test]
fn cli_validate_fails_closed_on_invalid_documents() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("invalid.toml");
    std::fs::write(&path, "schema = \"not-a-schema\"\n").unwrap();
    let error = cli::validate(&path).unwrap_err();
    assert!(error.to_string().contains("unsupported experiment schema"));

    let missing = dir.path().join("missing.toml");
    let error = cli::validate(&missing).unwrap_err();
    assert!(matches!(error, cli::ValidateError::Io(_)));
}

// --- P19-D02: selected named connections, execution mode, storage ------

const CONNECTION_DOC: &str = r#"
schema = "opi-eval-experiment/2"
experiment_id = "connection-projection"
connection = "primary"

[benchmark]
name = "terminal-bench"
revision = "2.1"
dataset = "laude-institute/terminal-bench-2.1"

[[subjects]]
id = "opi"
product = "opi"
version = "0.8.2"

[[subjects]]
id = "pi"
product = "pi"
version = "0.84.1"

[[edges]]
id = "opi-baseline-vs-pi-candidate"
baseline = "opi"
candidate = "pi"

[[connections]]
id = "primary"
provider = "scripted"
api = "anthropic-messages"
base_url = "http://127.0.0.1:48127"
model = "glm-5.3-flash"
credential_env = "OPI_EVAL_SCRIPTED_KEY"

[[connections]]
id = "secondary"
provider = "scripted"
api = "openai-responses"
base_url = "http://127.0.0.1:48128/v1"
model = "eval-responses"
credential_file = "/private/eval/credential.txt"

[model_controls]
temperature = 0.0
top_p = "omitted"
max_output_tokens = 4096
context_window = 128000
reasoning = "omitted"
required = ["max_output_tokens", "context_window"]

[execution]
mode = "native-fixture"
time_limit_seconds = 900

[storage]
cache = "cache"
results = "results"
capture_policy = "full"

[environment]
platform = "linux"
architecture = "x86_64"
cwd_policy = "workspace-root"

[[trials]]
id = "trial-opi-hello"
subject = "opi"
task = "hello"
group = "hello"

[[trials]]
id = "trial-pi-hello"
subject = "pi"
task = "hello"
group = "hello"
"#;

#[test]
fn connection_doc_resolves_and_freezes_the_selected_connection() {
    let resolved = ResolvedExperiment::resolve(CONNECTION_DOC).unwrap();
    assert_eq!(resolved.schema(), "opi-eval-experiment/2");
    let connection = resolved.selected_connection();
    assert_eq!(connection.id, "primary");
    assert_eq!(connection.provider, "scripted");
    assert_eq!(connection.api, ConnectionApi::AnthropicMessages);
    assert_eq!(connection.base_url, "http://127.0.0.1:48127");
    assert_eq!(connection.model, "glm-5.3-flash");
    assert_eq!(
        connection.credential,
        CredentialReference::Env {
            variable: "OPI_EVAL_SCRIPTED_KEY".to_owned()
        }
    );
    // The unselected sibling stays declared but is not selected.
    assert_eq!(resolved.connections().len(), 2);
    assert!(
        resolved
            .connections()
            .iter()
            .any(|c| c.id == "secondary" && c.api == ConnectionApi::OpenAiResponses)
    );
    // Effective execution bounds and storage identity are frozen.
    let execution = resolved.execution();
    assert_eq!(execution.mode, ExecutionMode::NativeFixture);
    assert_eq!(execution.time_limit_seconds, Some(900));
    // The bound is omitted: a positive request has no enforcement route,
    // so the fixed runtime capture protection stays in force.
    assert_eq!(execution.max_output_bytes, None);
    let storage = resolved.storage();
    assert_eq!(storage.cache, "cache");
    assert_eq!(storage.results, "results");
    assert_eq!(storage.capture_policy, CapturePolicy::Full);
    // Extended controls: every control explicit, required list recorded.
    let controls = resolved.model_controls();
    assert_eq!(controls.temperature, ControlValue::Value(0.0));
    assert_eq!(controls.top_p, ControlValue::Omitted);
    assert_eq!(controls.context_window, ControlValue::Value(128_000));
    assert_eq!(
        controls.required,
        vec!["max_output_tokens", "context_window"]
    );
    // The same document resolves deterministically to one digest.
    let again = ResolvedExperiment::resolve(CONNECTION_DOC).unwrap();
    assert_eq!(resolved.manifest_digest(), again.manifest_digest());
}

#[test]
fn connection_doc_passes_the_validate_cli() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("connection.toml");
    std::fs::write(&path, CONNECTION_DOC).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("validate")
        .arg("--config")
        .arg(&path)
        .output()
        .expect("run opi-eval validate");
    assert!(
        output.status.success(),
        "validate CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn connection_doc(tweak: fn(&mut String)) -> String {
    let mut source = CONNECTION_DOC.to_owned();
    tweak(&mut source);
    source
}

#[test]
fn resolve_rejects_unknown_or_duplicate_connection_selection() {
    let error = assert_rejected(&connection_doc(|s| {
        *s = s.replace("connection = \"primary\"", "connection = \"missing\"");
    }));
    assert!(matches!(error, ResolveError::UnknownConnection(id) if id == "missing"));

    let error = assert_rejected(&connection_doc(|s| {
        *s = s.replace(
            "[[connections]]\nid = \"secondary\"",
            "[[connections]]\nid = \"primary\"",
        );
    }));
    assert!(matches!(error, ResolveError::DuplicateConnection(id) if id == "primary"));

    let error = assert_rejected(&connection_doc(|s| {
        *s = s.replace("connection = \"primary\"\n", "");
    }));
    assert!(matches!(error, ResolveError::MissingConnectionSelection));
}

#[test]
fn resolve_rejects_invalid_credential_references() {
    // Both references at once.
    let error = assert_rejected(&connection_doc(|s| {
        *s = s.replace(
            "credential_env = \"OPI_EVAL_SCRIPTED_KEY\"",
            "credential_env = \"OPI_EVAL_SCRIPTED_KEY\"\ncredential_file = \"/private/k.txt\"",
        );
    }));
    assert!(matches!(
        error,
        ResolveError::InvalidCredentialReference { .. }
    ));
    // Neither reference.
    let error = assert_rejected(&connection_doc(|s| {
        *s = s.replace("credential_env = \"OPI_EVAL_SCRIPTED_KEY\"\n", "");
    }));
    assert!(matches!(
        error,
        ResolveError::InvalidCredentialReference { .. }
    ));
}

#[test]
fn resolve_rejects_unknown_connection_api() {
    let error = assert_rejected(&connection_doc(|s| {
        *s = s.replace("api = \"anthropic-messages\"", "api = \"grpc\"");
    }));
    assert!(matches!(error, ResolveError::InvalidConnectionApi(_)));
}

#[test]
fn resolve_rejects_remote_routes_without_explicit_live_mode() {
    for mode in ["hermetic", "native-fixture"] {
        for route in [
            "https://open.bigmodel.cn/api/anthropic",
            "http://example.invalid#@localhost",
            "http://example.invalid#@127.0.0.1",
            r"http://example.invalid\@localhost",
            "http://localhost:not-a-port",
            "http://localhost:65536",
            "http://localhost@example.invalid",
            "http://127.0.0.2",
            "http://[::2]",
            "ftp://localhost",
        ] {
            let doc = CONNECTION_DOC
                .replace(
                    "base_url = \"http://127.0.0.1:48127\"",
                    &format!("base_url = {route:?}"),
                )
                .replace("mode = \"native-fixture\"", &format!("mode = \"{mode}\""));
            let error = assert_rejected(&doc);
            assert!(
                matches!(error, ResolveError::RemoteRouteNotLive { .. }),
                "mode {mode} must reject route {route:?}: {error:?}"
            );
        }
        for route in [
            "http://localhost:48127/v1",
            "https://LOCALHOST/v1#part",
            "http://127.0.0.1/v1#part",
            "http://example.invalid@localhost",
            "http://[::1]:48127/v1",
            "https://[::1]/v1#part",
        ] {
            let doc = CONNECTION_DOC
                .replace(
                    "base_url = \"http://127.0.0.1:48127\"",
                    &format!("base_url = {route:?}"),
                )
                .replace("mode = \"native-fixture\"", &format!("mode = \"{mode}\""));
            assert!(
                ResolvedExperiment::resolve(&doc).is_ok(),
                "mode {mode} must admit parsed loopback route {route:?}"
            );
        }
    }
    // The same remote route with explicit live mode resolves: real
    // acceptance belongs only to the 19.11 operator evidence run.
    let doc = CONNECTION_DOC
        .replace(
            "base_url = \"http://127.0.0.1:48127\"",
            "base_url = \"https://open.bigmodel.cn/api/anthropic\"",
        )
        .replace("mode = \"native-fixture\"", "mode = \"live\"");
    assert!(ResolvedExperiment::resolve(&doc).is_ok());
}

#[test]
fn resolve_rejects_unknown_execution_mode_and_bad_capture_policy() {
    let error = assert_rejected(&connection_doc(|s| {
        *s = s.replace("mode = \"native-fixture\"", "mode = \"remote\"");
    }));
    assert!(matches!(error, ResolveError::InvalidExecutionMode(m) if m == "remote"));

    let error = assert_rejected(&connection_doc(|s| {
        *s = s.replace(
            "capture_policy = \"full\"",
            "capture_policy = \"sometimes\"",
        );
    }));
    assert!(matches!(error, ResolveError::InvalidCapturePolicy(p) if p == "sometimes"));
}

#[test]
fn resolve_rejects_requested_caps_the_native_route_cannot_enforce() {
    let error = assert_rejected(&connection_doc(|s| {
        *s = s.replace(
            "[storage]",
            "[execution.caps]\nmonetary_usd = 10.0\n\n[storage]",
        );
    }));
    assert!(matches!(error, ResolveError::UnenforceableCap { cap, .. } if cap == "monetary_usd"));

    let error = assert_rejected(&connection_doc(|s| {
        *s = s.replace("[storage]", "[execution.caps]\nrequests = 100\n\n[storage]");
    }));
    assert!(matches!(error, ResolveError::UnenforceableCap { cap, .. } if cap == "requests"));
}

#[test]
fn resolve_rejects_required_control_marked_unsupported() {
    for name in ["temperature", "top_p"] {
        for marker in ["omitted", "unknown"] {
            let mut source: toml::Value = CONNECTION_DOC.parse().unwrap();
            source["model_controls"][name] = marker.into();
            source["model_controls"]["required"] = toml::Value::Array(vec![name.into()]);
            let error = assert_rejected(&toml::to_string(&source).unwrap());
            assert!(
                matches!(error, ResolveError::RequiredControlUnsupported { control, .. } if control == name)
            );
            source["model_controls"]["required"] = toml::Value::Array(Vec::new());
            assert!(ResolvedExperiment::resolve(&toml::to_string(&source).unwrap()).is_ok());
        }
    }
    let error = assert_rejected(&connection_doc(|s| {
        *s = s.replace(
            "required = [\"max_output_tokens\", \"context_window\"]",
            "required = [\"reasoning\"]",
        );
    }));
    assert!(
        matches!(error, ResolveError::RequiredControlUnsupported { control, .. } if control == "reasoning")
    );

    let error = assert_rejected(&connection_doc(|s| {
        *s = s.replace(
            "required = [\"max_output_tokens\", \"context_window\"]",
            "required = [\"nonexistent\"]",
        );
    }));
    assert!(
        matches!(error, ResolveError::RequiredControlUnsupported { control, .. } if control == "nonexistent")
    );
}

#[test]
fn resolve_requires_every_extended_control_and_storage_identity() {
    let error = assert_rejected(&connection_doc(|s| {
        *s = s.replace("top_p = \"omitted\"\n", "");
    }));
    assert!(matches!(error, ResolveError::MissingModelControl(c) if c == "top_p"));

    let error = assert_rejected(&connection_doc(|s| {
        *s = s.replace("context_window = 128000\n", "");
    }));
    assert!(matches!(error, ResolveError::MissingModelControl(c) if c == "context_window"));

    let error = assert_rejected(&connection_doc(|s| {
        *s = s.replace("capture_policy = \"full\"\n", "");
    }));
    assert!(matches!(error, ResolveError::MissingField(f) if f.contains("capture_policy")));
}

// --- Task 19.6: [selection]/[execution] document contract (P19-D05) ---

/// A minimal fixture with the explicit trials replaced by a selection
/// block (plus optional execution bound overrides).
fn selection_doc(tweak: fn(&mut String)) -> String {
    let mut source = MINIMAL_FIXTURE[..MINIMAL_FIXTURE.find("[[trials]]").unwrap()].to_owned();
    source.push_str(
        r#"
[selection]
scope = "subset"
task_ids = ["hello", "world"]
reason = "feasible subset for the deterministic run"
repetitions = 2

[selection.seeds]
hello = 7

[execution]
mode = "hermetic"
concurrency = 2
max_trials = 8
agent_timeout_seconds = 600
verifier_timeout_seconds = 900
max_output_tokens = 4096
"#,
    );
    tweak(&mut source);
    source
}

#[test]
fn resolve_selection_freezes_scope_tasks_repetitions_and_bounds() {
    let resolved = ResolvedExperiment::resolve(&selection_doc(|_| {})).unwrap();
    let selection = resolved.selection().expect("selection must resolve");
    assert_eq!(selection.scope.as_str(), "subset");
    assert_eq!(selection.task_ids, ["hello", "world"]);
    assert_eq!(
        selection.reason,
        "feasible subset for the deterministic run"
    );
    assert_eq!(selection.repetitions, 2);
    assert_eq!(selection.seeds.get("hello"), Some(&7));

    let execution = resolved.execution();
    assert_eq!(execution.concurrency, 2);
    assert_eq!(execution.max_trials, Some(8));
    assert_eq!(execution.agent_timeout_seconds, Some(600));
    assert_eq!(execution.verifier_timeout_seconds, Some(900));
    assert_eq!(execution.max_output_tokens, Some(4096));

    // No explicit trials in selection mode; expansion happens at freeze.
    assert!(resolved.trials().is_empty());
    let again = ResolvedExperiment::resolve(&selection_doc(|_| {})).unwrap();
    assert_eq!(resolved.manifest_digest(), again.manifest_digest());
}

#[test]
fn resolve_rejects_selection_and_explicit_trials_ambiguity() {
    let mut source = selection_doc(|_| {});
    source.push_str(
        "\n[[trials]]\nid = \"t\"\nsubject = \"opi\"\ntask = \"hello\"\ngroup = \"hello\"\n",
    );
    let error = assert_rejected(&source);
    assert!(error.to_string().contains("mutually exclusive"), "{error}");
}

#[test]
fn resolve_rejects_selection_faults() {
    let error = assert_rejected(&selection_doc(|s| {
        *s = s.replace("scope = \"subset\"", "scope = \"half\"");
    }));
    assert!(matches!(error, ResolveError::InvalidSelectionScope(s) if s == "half"));

    let error = assert_rejected(&selection_doc(|s| {
        *s = s.replace("task_ids = [\"hello\", \"world\"]\nreason =", "reason =");
    }));
    assert!(matches!(error, ResolveError::EmptySubsetSelection));

    let error = assert_rejected(&selection_doc(|s| {
        *s = s.replace(
            "task_ids = [\"hello\", \"world\"]",
            "task_ids = [\"hello\", \"hello\"]",
        );
    }));
    assert!(matches!(error, ResolveError::DuplicateSelectionTask(t) if t == "hello"));

    let error = assert_rejected(&selection_doc(|s| {
        *s = s.replace("repetitions = 2", "repetitions = 0");
    }));
    assert!(matches!(
        error,
        ResolveError::InvalidSelectionRepetitions(0)
    ));

    let error = assert_rejected(&selection_doc(|s| {
        *s = s.replace(
            "\nreason = \"feasible subset for the deterministic run\"",
            "",
        );
    }));
    assert!(matches!(error, ResolveError::MissingField(f) if f.contains("reason")));
}

#[test]
fn resolve_full_scope_cannot_pin_task_ids() {
    let error = assert_rejected(&selection_doc(|s| {
        *s = s.replace("scope = \"subset\"", "scope = \"full\"");
    }));
    assert!(matches!(error, ResolveError::FullScopePinsTasks));
    assert_rejected(&selection_doc(|s| {
        *s = s.replace(
            "task_ids = [\"hello\", \"world\"]\nreason",
            "task_ids = []\nreason",
        )
    }));
    ResolvedExperiment::resolve(&selection_doc(|s| {
        *s = s.replace(
            "scope = \"subset\"\ntask_ids = [\"hello\", \"world\"]\n",
            "scope = \"full\"\n",
        );
    }))
    .unwrap();
}

#[test]
fn resolve_rejects_unenforceable_execution_bounds() {
    let error = assert_rejected(&selection_doc(|s| {
        *s = s.replace("max_output_tokens = 4096", "max_total_tokens = 100000");
    }));
    assert!(
        matches!(error, ResolveError::UnenforceableExecutionBound { ref bound, .. } if bound == "max_total_tokens"),
        "{error}"
    );

    let error = assert_rejected(&selection_doc(|s| {
        *s = s.replace("max_output_tokens = 4096", "max_cost = \"5 USD\"");
    }));
    assert!(
        matches!(error, ResolveError::UnenforceableExecutionBound { ref bound, .. } if bound == "max_cost"),
        "{error}"
    );

    // An output-token bound is enforceable only through the pinned model
    // control: a contradiction with the frozen control rejects.
    let error = assert_rejected(&selection_doc(|s| {
        *s = s.replace(
            "verifier_timeout_seconds = 900\nmax_output_tokens = 4096",
            "verifier_timeout_seconds = 900\nmax_output_tokens = 8192",
        );
    }));
    assert!(
        matches!(error, ResolveError::UnenforceableExecutionBound { ref bound, .. } if bound == "max_output_tokens"),
        "{error}"
    );

    // An explicit positive captured-output bound has no enforcement route
    // either: it rejects by name in every execution mode, and omission
    // keeps the fixed runtime capture protection.
    for mode in ["hermetic", "native-fixture", "live"] {
        let mut source = selection_doc(|s| {
            *s = s.replace(
                "mode = \"hermetic\"",
                "mode = \"CAPTURE-BOUND-PROBE-MODE\"\nmax_output_bytes = 1048576",
            );
        });
        source = source.replace("CAPTURE-BOUND-PROBE-MODE", mode);
        let error = assert_rejected(&source);
        assert!(
            matches!(error, ResolveError::UnenforceableExecutionBound { ref bound, .. } if bound == "max_output_bytes"),
            "{mode}: {error}"
        );
    }
}

#[test]
fn resolve_rejects_nonpositive_execution_bounds() {
    let error = assert_rejected(&selection_doc(|s| {
        *s = s.replace("concurrency = 2", "concurrency = 0");
    }));
    assert!(matches!(error, ResolveError::InvalidConcurrency(0)));

    let error = assert_rejected(&selection_doc(|s| {
        *s = s.replace("max_trials = 8", "max_trials = 0");
    }));
    assert!(
        matches!(error, ResolveError::InvalidControlValue { ref control, .. } if control.contains("max_trials")),
        "{error}"
    );
}

#[test]
fn resolve_rejects_wrongly_typed_execution_bounds() {
    // P19-AUD-001: every integer execution field rejects present
    // non-integers before effects. Quoted numbers and empty strings no
    // longer erase the bound; absence keeps its omitted meaning.
    let base = selection_doc(|_| {});
    for (field, original) in [
        ("max_trials", "max_trials = 8"),
        ("agent_timeout_seconds", "agent_timeout_seconds = 600"),
        ("verifier_timeout_seconds", "verifier_timeout_seconds = 900"),
        ("max_output_tokens", "max_output_tokens = 4096"),
    ] {
        for replacement in [
            format!("{field} = \"2\""),
            format!("{field} = \"\""),
            format!("{field} = true"),
            format!("{field} = 1.5"),
            format!("{field} = -3"),
        ] {
            let error = assert_rejected(&base.replace(original, &replacement));
            assert!(
                matches!(error, ResolveError::InvalidControlValue { ref control, .. } if control == field),
                "{field} -> {replacement}: {error}"
            );
        }
    }
    // The remaining bounds without a selection fixture are rejected
    // through the same resolver on a document that carries them.
    for field in ["time_limit_seconds", "max_total_tokens", "max_output_bytes"] {
        for replacement in [format!("{field} = \"2\""), format!("{field} = \"\"")] {
            let error = assert_rejected(&base.replace(
                "concurrency = 2",
                &format!("concurrency = 2\n{replacement}"),
            ));
            assert!(
                matches!(error, ResolveError::InvalidControlValue { ref control, .. } if control == field),
                "{field} -> {replacement}: {error}"
            );
        }
        // Numeric present values keep their existing typed/unenforceable
        // refusals instead of the wrong-type branch.
        if field != "time_limit_seconds" {
            let error = assert_rejected(
                &base.replace("concurrency = 2", &format!("concurrency = 2\n{field} = 2")),
            );
            assert!(
                matches!(error, ResolveError::UnenforceableExecutionBound { ref bound, .. } if bound == field),
                "{field}: {error}"
            );
        }
    }
    // Absence stays a valid omitted bound.
    let resolved = ResolvedExperiment::resolve(&base).unwrap();
    assert_eq!(resolved.execution().time_limit_seconds, None);
}

#[test]
fn validate_cli_summarizes_selection_documents() {
    let dir = std::env::temp_dir().join(format!("opi-eval-selection-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("selection.toml");
    std::fs::write(&path, selection_doc(|_| {})).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_opi-eval"))
        .arg("validate")
        .arg("--config")
        .arg(&path)
        .output()
        .expect("run opi-eval validate");
    assert!(
        output.status.success(),
        "validate CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("selection=subset"), "summary: {stdout}");
    assert!(stdout.contains("tasks=2"), "summary: {stdout}");
    assert!(stdout.contains("repetitions=2"), "summary: {stdout}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn single_subject_selection_needs_no_artificial_edge() {
    let mut source = selection_doc(|_| {});
    // One subject, no edges: the report stands without a fabricated pair.
    source = source
        .replace(
            "[[subjects]]\nid = \"opi\"\nproduct = \"opi\"\nversion = \"0.8.1\"\n\n[[subjects]]\nid = \"pi\"\nproduct = \"pi\"\nversion = \"0.84.1\"\n\n[[edges]]\nid = \"opi-baseline-vs-pi-candidate\"\nbaseline = \"opi\"\ncandidate = \"pi\"\n",
            "[[subjects]]\nid = \"pi\"\nproduct = \"pi\"\nversion = \"0.84.1\"\n",
        );
    let resolved = ResolvedExperiment::resolve(&source).unwrap();
    assert_eq!(resolved.subjects().len(), 1);
    assert!(resolved.edges().is_empty());
    assert!(resolved.selection().is_some());
}

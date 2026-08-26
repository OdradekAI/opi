//! Integration contract tests for the provisional Phase 18 experiment seam.
//!
//! Covers the Independent Companion dependency boundary (P18-A01) and the
//! canonical, fail-closed resolution behavior of
//! [`opi_eval::experiment::ResolvedExperiment`].

use std::path::Path;

use opi_eval::cli;
use opi_eval::experiment::{ControlValue, EXPERIMENT_SCHEMA, ResolveError, ResolvedExperiment};

const MINIMAL_FIXTURE: &str = include_str!("fixtures/experiment/minimal.toml");

fn manifest_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn minimal_fixture_path() -> std::path::PathBuf {
    manifest_dir().join("tests/fixtures/experiment/minimal.toml")
}

const THREE_SUBJECT_DOC: &str = r#"
schema = "phase18-experiment/1"
experiment_id = "three-subject-generic"

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

[model_controls]
provider = "mock"
model = "mock-model"
endpoint_class = "local"
temperature = "unknown"
max_output_tokens = "omitted"
reasoning = "high"

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

/// P18-A01: with the Opi workspace present, the Companion has no Opi crate
/// dependency in any dependency table and nothing in the lock graph gives an
/// Opi product a reverse edge to `opi-eval`. The scenario's `cargo tree`
/// commands provide the full transitive proof; this test pins the local
/// manifest and lockfile boundary the package itself owns.
#[test]
fn p18_a01_independent_companion_dependency_boundary() {
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
    assert_eq!(resolved.experiment_id(), "phase18-minimal-pairing");
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
    assert_eq!(controls.provider, "mock");
    assert_eq!(controls.model, "mock-model");
    assert_eq!(controls.endpoint_class, "local");
    assert_eq!(controls.temperature, ControlValue::Value(0.0));
    assert_eq!(controls.max_output_tokens, ControlValue::Value(4096));
    assert_eq!(controls.reasoning, ControlValue::Omitted);

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
        *s = s.replace("schema = \"phase18-experiment/1\"\n", "");
    }));
    assert!(matches!(error, ResolveError::UnsupportedSchema(_)));

    let error = assert_rejected(&invalid_doc(|s| {
        *s = s.replace("phase18-experiment/1", "phase18-experiment/2");
    }));
    assert!(matches!(error, ResolveError::UnsupportedSchema(_)));
}

#[test]
fn resolve_rejects_missing_experiment_id() {
    let error = assert_rejected(&invalid_doc(|s| {
        *s = s.replace("experiment_id = \"phase18-minimal-pairing\"", "");
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
    assert_eq!(summary.experiment_id, "phase18-minimal-pairing");
    assert_eq!(summary.schema, EXPERIMENT_SCHEMA);
    assert_eq!(summary.subject_count, 2);
    assert_eq!(summary.edge_count, 1);
    assert_eq!(summary.trial_count, 2);
    assert_eq!(summary.manifest_digest.len(), 64);
    let rendered = summary.to_string();
    assert!(rendered.contains("phase18-minimal-pairing"));
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

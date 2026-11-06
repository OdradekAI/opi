//! Product CI routing and optional native-platform verification contracts.

// opi-runtime-acceptance

use serde_yaml::{Mapping, Value};

fn workflow(source: &str) -> Mapping {
    serde_yaml::from_str::<Value>(source)
        .expect("workflow YAML parses")
        .as_mapping()
        .expect("workflow is a mapping")
        .clone()
}

fn field<'a>(mapping: &'a Mapping, key: &str) -> &'a Value {
    mapping
        .get(Value::String(key.to_owned()))
        .unwrap_or_else(|| panic!("missing {key}"))
}

fn mapping(value: &Value) -> &Mapping {
    value.as_mapping().expect("mapping")
}

fn job<'a>(workflow: &'a Mapping, name: &str) -> &'a Mapping {
    mapping(field(mapping(field(workflow, "jobs")), name))
}

fn commands(job: &Mapping) -> String {
    field(job, "steps")
        .as_sequence()
        .expect("steps")
        .iter()
        .filter_map(|step| step.get("run").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn documentation_gate_runs_its_own_regression_suite() {
    let ci = workflow(include_str!("../../../.github/workflows/ci.yml"));
    let run = commands(job(&ci, "scope"));
    assert!(run.contains("python3 scripts/opi-doc-check.py"));
    assert!(run.contains("python3 scripts/test_opi_doc_check.py"));
}

#[test]
fn development_eval_workflows_are_not_actions_entrypoints() {
    let workflows =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.github/workflows");
    for entry in std::fs::read_dir(workflows).expect("workflow directory exists") {
        let path = entry.expect("workflow entry").path();
        let name = path
            .file_name()
            .expect("workflow filename")
            .to_string_lossy();
        assert!(
            !(name.starts_with("opi-eval")
                && matches!(
                    path.extension().and_then(|value| value.to_str()),
                    Some("yml" | "yaml")
                )),
            "development-only eval workflow must remain outside Actions: {name}"
        );
    }
}

#[test]
fn routine_product_checks_are_linux_only_and_exclude_eval() {
    let ci = workflow(include_str!("../../../.github/workflows/ci.yml"));
    for name in ["product", "msrv"] {
        let selected = job(&ci, name);
        assert_eq!(field(selected, "runs-on").as_str(), Some("ubuntu-latest"));
        assert_eq!(
            field(selected, "if").as_str(),
            Some("needs.scope.outputs.rust == 'true'")
        );
        for line in commands(selected).lines() {
            if line.contains("--workspace") {
                assert!(line.contains("--exclude opi-eval"), "{line}");
            }
        }
    }
    let jobs = mapping(field(&ci, "jobs"));
    for redundant in ["phase17_acceptance", "sandbox_package", "target_check"] {
        assert!(!jobs.contains_key(Value::String(redundant.to_owned())));
    }
}

#[test]
fn required_gate_checks_selected_results_even_after_failure_or_skip() {
    let ci = workflow(include_str!("../../../.github/workflows/ci.yml"));
    let required = job(&ci, "required");
    assert_eq!(field(required, "if").as_str(), Some("always()"));
    let needs = field(required, "needs").as_sequence().expect("needs");
    for name in ["scope", "product", "msrv", "platforms"] {
        assert!(needs.iter().any(|value| value.as_str() == Some(name)));
    }
    let run = commands(required);
    for result in ["SCOPE", "PRODUCT", "MSRV", "PLATFORMS"] {
        assert!(run.contains(&format!("test \"${result}\" = success")));
    }
}

#[test]
fn automatic_platform_smoke_is_bound_to_the_current_candidate() {
    let ci = workflow(include_str!("../../../.github/workflows/ci.yml"));
    let platforms = job(&ci, "platforms");
    assert_eq!(
        field(platforms, "uses").as_str(),
        Some("./.github/workflows/platform-verify.yml")
    );
    let inputs = mapping(field(platforms, "with"));
    assert_eq!(field(inputs, "ref").as_str(), Some("${{ github.sha }}"));
    assert_eq!(field(inputs, "linux").as_bool(), Some(false));
    assert_eq!(field(inputs, "suite").as_str(), Some("smoke"));
}

#[test]
fn native_verification_exposes_platform_selection_and_has_no_eval_gate() {
    let platforms = workflow(include_str!(
        "../../../.github/workflows/platform-verify.yml"
    ));
    let events = mapping(field(&platforms, "on"));
    for event in ["workflow_dispatch", "workflow_call"] {
        let inputs = mapping(field(mapping(field(events, event)), "inputs"));
        for name in ["ref", "linux", "windows", "macos", "suite"] {
            field(inputs, name);
        }
    }
    let acceptance = job(&platforms, "execution_acceptance");
    let run = commands(acceptance);
    assert!(run.contains("--workspace --exclude opi-eval --all-targets --locked"));
    assert!(run.contains("--test auth_process_lifecycle"));
    assert!(run.contains("--test execution_protocol_host"));
    assert!(!run.contains("cargo test -p opi-eval"));
}

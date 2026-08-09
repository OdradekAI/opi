//! Host-portable compile and dependency guard for the native policy probe.

#![deny(unsafe_code)]

#[path = "support/policy_probe.rs"]
mod policy_probe;

#[test]
fn policy_probe_has_no_python_runtime_dependency() {
    let source = include_str!("support/policy_probe.rs");
    assert!(!source.contains("python3"));
    assert!(!source.contains("Command::new(\"python"));
    assert_eq!(policy_probe::TEST_NAME, "policy_probe::native_policy_probe");
}

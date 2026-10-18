//! CI selects the same offline runtime acceptance on Linux, macOS and Windows.

// opi-runtime-acceptance
#[test]
fn phase17_ci_matrix_selects_same_acceptance_on_three_platforms() {
    let ci = include_str!("../../../.github/workflows/ci.yml");
    let start = ci
        .find("  phase17_acceptance:")
        .expect("the phase17_acceptance job exists");
    let rest = &ci[start..];
    // The job block runs until the next sibling job key: a line indented by
    // exactly two spaces (deeper indentation belongs to this job's body).
    let mut block_lines: Vec<&str> = Vec::new();
    for (i, line) in rest.lines().enumerate() {
        if i > 0
            && ((line.starts_with("  ") && !line.starts_with("   "))
                || (!line.starts_with(' ') && !line.is_empty()))
        {
            break;
        }
        block_lines.push(line);
    }
    let block = block_lines.join("\n");
    for os in ["ubuntu-latest", "macos-latest", "windows-latest"] {
        assert!(
            block.contains(os),
            "the phase17 acceptance matrix includes {os}"
        );
    }
    for target in [
        "--test cross_mode_runtime",
        "--test runtime_failure_recovery",
        "--test architecture_guards",
        "--test test_hermeticity",
        "--test ci_contract",
    ] {
        assert!(
            block.contains(target),
            "the phase17 acceptance job selects {target} on every matrix OS"
        );
    }
    assert!(
        !block.contains("if: matrix.os") && !block.contains("if: runner"),
        "the phase17 acceptance job has no OS-conditional gating"
    );
}

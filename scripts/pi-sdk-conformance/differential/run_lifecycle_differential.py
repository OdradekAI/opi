"""Replay controlled source lifecycles; evidence generation is not alignment approval."""
from __future__ import annotations

import argparse
import importlib.util
import json
import subprocess
from datetime import datetime, timezone
from pathlib import Path

from common import DIFFERENTIAL_PATH, REPO_ROOT, build_rust_driver, file_hash, run_json, run_rust_trace
from compare_lifecycle import SCENARIOS, compare_traces


def material_tools(repo: Path):
    path = repo / DIFFERENTIAL_PATH / "tools/freeze_acceptance_materials.py"
    spec = importlib.util.spec_from_file_location("acceptance_materials", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def verdict(cases: list[dict], blocked: list[str]) -> tuple[str, int]:
    if len(cases) != len(SCENARIOS) or {case["scenario"] for case in cases} != set(SCENARIOS):
        return "INCOMPLETE_CASES", 1
    if any(case["comparison"]["status"] == "fail" for case in cases):
        return "FAIL", 1
    if blocked:
        return "BLOCKED_WITH_RECORDED_OBSERVATIONS", 2
    if any(case["comparison"]["status"] == "difference" for case in cases):
        return "DIFFERENCES_REQUIRE_DECISION", 0
    return "SHARED_OBSERVATIONS_PASS", 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=REPO_ROOT)
    parser.add_argument("--materials", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args(argv)
    if args.output.exists():
        parser.error(f"refusing to overwrite evidence: {args.output}")
    repo = args.repo.resolve()
    freezer = material_tools(repo)
    materials_path = args.materials.resolve()
    materials_bytes = materials_path.read_bytes()
    material = json.loads(materials_bytes)
    materials_hash = freezer.sha256_bytes(materials_bytes)

    def material_drift() -> list[str]:
        errors = freezer.verify_evidence(repo, material)
        if freezer.file_record(materials_path).get("sha256") != materials_hash:
            errors.append("material manifest bytes changed")
        return errors

    evidence = {
        "schemaVersion": 2, "generatedAtUtc": datetime.now(timezone.utc).isoformat(),
        "repository": str(repo),
        "scope": "Paired public Models source completion, EOF timing, telemetry and setup failures, and deferred fetch and cancel.",
        "materialsEvidence": {"path": str(materials_path), "sha256": materials_hash, "snapshot": material.get("snapshot")},
        "blocked_gates": list(material.get("blocked_gates", [])),
        "strict_alignment": False,
        "cases": [],
    }
    drift = material_drift()
    if drift:
        evidence.update({"verdict": "STALE_MATERIALS", "material_drift": drift})
        freezer.write_new_json(args.output, evidence)
        return 2
    try:
        build = build_rust_driver(repo)
        evidence["rust_build"] = build
        upstream_driver = repo / DIFFERENTIAL_PATH / "tools/observe_upstream.mjs"
        evidence["drivers"] = {str(path.relative_to(repo)): file_hash(path) for path in (
            upstream_driver, repo / "crates/opi-ai/examples/lifecycle_differential.rs",
            repo / "scripts/pi-sdk-conformance/differential/compare_lifecycle.py",
            repo / "scripts/pi-sdk-conformance/differential/common.py",
            repo / "scripts/pi-sdk-conformance/differential/run_lifecycle_differential.py",
        )}
        for scenario in SCENARIOS:
            command = ["node", str(upstream_driver), "--snapshot", material["snapshot"], "--case", scenario]
            upstream_runs = [run_json(command, repo) for _ in range(3)]
            rust = run_rust_trace(scenario, build, repo)
            comparison = compare_traces(upstream_runs[0]["trace"], rust["trace"], scenario)
            if any(row["trace"] != upstream_runs[0]["trace"] for row in upstream_runs[1:]):
                comparison = {"status": "fail", "reason": "upstream observations differ across repetitions"}
            evidence["cases"].append({"scenario": scenario, "upstream_runs": upstream_runs, "rust_run": rust, "comparison": comparison})
        after = material_drift()
        if after:
            evidence.update({"verdict": "MATERIALS_CHANGED_DURING_RUN", "material_drift": after})
            code = 2
        else:
            evidence["verdict"], code = verdict(evidence["cases"], evidence["blocked_gates"])
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        evidence.update({"verdict": "DRIVER_FAILURE", "error": str(error)})
        if isinstance(error, subprocess.CalledProcessError):
            as_text = lambda value: value.decode("utf-8", errors="replace") if isinstance(value, bytes) else value
            evidence["failed_command"] = {"command": error.cmd, "exit_code": error.returncode,
                                          "stdout": as_text(error.stdout), "stderr": as_text(error.stderr)}
        code = 1
    freezer.write_new_json(args.output, evidence)
    print(json.dumps({"status": "recorded", "verdict": evidence["verdict"], "output": str(args.output)}))
    return code


if __name__ == "__main__":
    raise SystemExit(main())

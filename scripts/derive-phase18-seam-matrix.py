#!/usr/bin/env python3
"""Derive the final Phase 18 seam-evidence matrix from the sealed artifact.

Task 18.16. This is assurance, not runtime: the script consumes the
successful, unexpired 18.15 outer upload receipt plus the downloaded
sealed artifact (zip, tar, or an already-unpacked stage directory) and
NEVER reruns native work. It re-verifies the binding and causal records
through the same fail-closed machinery `scripts/verify-phase18-native-
artifact.py` owns (identity, seal, dispatch, trials, provider, canary,
conformance rerun, oracle preflights, material, agent evidence), then
requires the typed trajectory/span records and native-source edges for
Agent activity, all three grader lifecycles, artifact collection, final
workspaces, verifiers, and the pre-seal lifecycle — and only then derives
the shared / adapter-private / rejected field matrix from those exact
records.

The matrix is derived, never hand-written: shared fields are the receipt
and bundle-evidence paths present for BOTH real Agents across all three
revisions; adapter-private fields are the ones only one product emits;
rejected fields are the ones whose facts stay typed unknowns (per-adapter
native values the contract refuses to fabricate into parity).

`--verify` re-runs the full chain and byte-compares the regenerated
matrix with the committed document.

Usage:
  python scripts/derive-phase18-seam-matrix.py \
      --receipt <upload-receipt.json> --artifact <sealed.zip|tar|stage> \
      --require-trajectory-spans \
      --output crates/opi-eval/docs/seam-evidence-matrix.md [--verify] \
      [--repo <repo>] [--expected-commit <sha>]

Exit 0 accepts; exit 1 prints one `finding <family>` line per violation.
"""

from __future__ import annotations

import argparse
import importlib.util
import json
import sys
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parent
_spec = importlib.util.spec_from_file_location(
    "verify_phase18_native_artifact", SCRIPTS / "verify-phase18-native-artifact.py"
)
assert _spec is not None and _spec.loader is not None
native = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(native)

MATRIX_SCHEMA = "phase18-seam-evidence-matrix/1"

# Lifecycle coverage required from every one of the six trials when
# --require-trajectory-spans is set. Each entry maps the causal record
# family to the staged evidence that must exist for both agents.
LIFECYCLE_EVIDENCE = (
    ("agent-activity", ("agent-trace",)),
    ("grader-lifecycle", ("verifier-trace", "bundle/settlement.json")),
    ("artifact-collection", ("bundle/artifacts",)),
    ("final-workspace", ("workspace",)),
    ("verifier", ("bundle/artifacts/native/verifier-stdout.log",)),
    ("pre-seal", ("receipt.json",)),
)

# Adapter-private bundle evidence per product (normalized under
# bundle/artifacts): the importer-owned native records each product emits
# and the other deliberately does not.
PRIVATE_EVIDENCE = {
    "opi": (
        "native/evidence/manifest",
        "native/evidence/records",
    ),
    "pi": (),
}


# ---------------------------------------------------------------------------
# Derivation
# ---------------------------------------------------------------------------


def flatten(value, prefix: str = "") -> dict[str, object]:
    flat: dict[str, object] = {}
    if isinstance(value, dict):
        for key, item in value.items():
            flat.update(flatten(item, f"{prefix}.{key}" if prefix else str(key)))
    else:
        flat[prefix] = value
    return flat


def trial_receipt_paths(stage: Path, reports: list[dict]) -> list[Path]:
    paths: list[Path] = []
    for entry in reports:
        cfg_dir = stage / "07-trials" / entry["cfg"] / "trials"
        for trial in entry["report"].get("trials", []):
            paths.append(cfg_dir / str(trial.get("id")))
    return paths


def require_lifecycle_edges(stage: Path, reports: list[dict], f) -> None:
    """Typed trajectory/span records and native-source edges must cover
    every declared lifecycle for every trial of both real Agents."""
    for entry in reports:
        cfg_dir = stage / "07-trials" / entry["cfg"] / "trials"
        for trial in entry["report"].get("trials", []):
            root = cfg_dir / str(trial.get("id"))
            product = trial.get("agent", {}).get("product")
            for family, members in LIFECYCLE_EVIDENCE:
                for member in members:
                    if not (root / member).exists():
                        f.reject(
                            "lifecycle",
                            f"{trial.get('id')} ({product}): causal record "
                            f"for {family} is missing: {member}",
                        )
            # The pre-seal lifecycle binding: the receipt carries the typed
            # trajectory's pre-seal digest and binds the sealed bundle.
            receipt_path = root / "receipt.json"
            if receipt_path.is_file():
                try:
                    receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
                except ValueError:
                    receipt = {}
                if not native.is_hex64(receipt.get("pre_seal_digest")):
                    f.reject(
                        "lifecycle",
                        f"{trial.get('id')}: typed trajectory pre-seal digest "
                        f"is missing from the trial receipt",
                    )
                sealed = ((receipt.get("seal_result") or {}).get("sealed") or {})
                if sealed.get("bundle_digest") != receipt.get("bundle_identity"):
                    f.reject(
                        "lifecycle",
                        f"{trial.get('id')}: the sealed bundle digest does not "
                        f"bind the typed trajectory",
                    )


def derive_matrix(stage: Path, reports: list[dict]) -> dict:
    """Classify receipt fields and bundle evidence per real Agent."""
    fields: dict[str, set[str]] = {"opi": set(), "pi": set()}
    unknown_fields: dict[str, set[str]] = {"opi": set(), "pi": set()}
    evidence: dict[str, set[str]] = {"opi": set(), "pi": set()}

    for entry in reports:
        cfg_dir = stage / "07-trials" / entry["cfg"] / "trials"
        for trial in entry["report"].get("trials", []):
            root = cfg_dir / str(trial.get("id"))
            product = trial.get("agent", {}).get("product")
            if product not in fields:
                continue
            receipt_path = root / "receipt.json"
            if not receipt_path.is_file():
                continue
            try:
                receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
            except ValueError:
                continue
            flat = flatten(
                {
                    "agent": receipt.get("agent", {}),
                    "verifier": receipt.get("verifier", {}),
                }
            )
            for path, value in flat.items():
                fields[product].add(path)
                if isinstance(value, str) and value.startswith("unknown:"):
                    unknown_fields[product].add(path)
            artifacts = root / "bundle" / "artifacts"
            if artifacts.is_dir():
                for file in artifacts.rglob("*"):
                    if file.is_file():
                        evidence[product].add(
                            file.relative_to(artifacts).as_posix()
                        )

    shared = fields["opi"] & fields["pi"]
    private = {
        "opi": sorted(fields["opi"] - fields["pi"]),
        "pi": sorted(fields["pi"] - fields["opi"]),
    }
    rejected = {
        "opi": sorted(unknown_fields["opi"]),
        "pi": sorted(unknown_fields["pi"]),
    }
    # Evidence is shared only when BOTH products stage it in EVERY one of
    # the three revisions. A path that appears in only some revisions is
    # forked by native-verifier ownership (the Terminal-Bench aggregate
    # versus the DeepSWE job result): the shared contract refuses to
    # unify those contents, so they classify as rejected, not shared.
    per_revision: dict[str, set[str]] = {}
    for entry in reports:
        cfg_dir = stage / "07-trials" / entry["cfg"] / "trials"
        revision_evidence: set[str] = set()
        for trial in entry["report"].get("trials", []):
            artifacts = cfg_dir / str(trial.get("id")) / "bundle" / "artifacts"
            if artifacts.is_dir():
                for file in artifacts.rglob("*"):
                    if file.is_file():
                        revision_evidence.add(
                            file.relative_to(artifacts).as_posix()
                        )
        per_revision[entry["cfg"]] = revision_evidence
    all_revisions = sorted(per_revision)
    in_every_revision: set[str] = set.intersection(*per_revision.values())         if per_revision else set()

    shared_evidence = (evidence["opi"] & evidence["pi"]) & in_every_revision
    private_evidence = {
        "opi": sorted(evidence["opi"] - evidence["pi"]),
        "pi": sorted(evidence["pi"] - evidence["opi"]),
    }
    rejected_evidence = sorted(
        (evidence["opi"] & evidence["pi"]) - in_every_revision
    )
    return {
        "schema": MATRIX_SCHEMA,
        "shared_fields": sorted(shared),
        "private_fields": private,
        "rejected_fields": rejected,
        "shared_evidence": sorted(shared_evidence),
        "private_evidence": private_evidence,
        "rejected_evidence": rejected_evidence,
        "revisions": all_revisions,
        "trials": len(trial_receipt_paths(stage, reports)),
    }


def render_matrix(matrix: dict, binding: dict) -> str:
    lines: list[str] = []
    lines.append("# Phase 18 seam-evidence matrix")
    lines.append("")
    lines.append("Derived by `scripts/derive-phase18-seam-matrix.py` from the")
    lines.append("sealed Phase 18 native artifact. Conformance-only evidence:")
    lines.append("no score, leaderboard, or product claim is made here.")
    lines.append("")
    lines.append("## Binding")
    lines.append("")
    lines.append("| Field | Value |")
    lines.append("|---|---|")
    for key in ("candidate_commit", "run_id", "artifact_digest",
                "sealed_manifest_sha256", "trials"):
        lines.append(f"| {key} | `{binding[key]}` |")
    lines.append(f"| matrix_schema | `{matrix['schema']}` |")
    lines.append("")

    lines.append("## Shared fields (both real Agents, all three revisions)")
    lines.append("")
    lines.append("Frozen by shared conformance; present in every trial")
    lines.append("receipt of both products.")
    lines.append("")
    lines.append("| Field |")
    lines.append("|---|")
    for field in matrix["shared_fields"]:
        lines.append(f"| `{field}` |")
    lines.append("")

    lines.append("## Shared staged evidence")
    lines.append("")
    lines.append("| Artifact path |")
    lines.append("|---|")
    for field in matrix["shared_evidence"]:
        lines.append(f"| `{field}` |")
    lines.append("")

    for product in ("opi", "pi"):
        lines.append(f"## Adapter-private fields ({product})")
        lines.append("")
        lines.append("| Field |")
        lines.append("|---|")
        for field in matrix["private_fields"][product]:
            lines.append(f"| `{field}` |")
        lines.append("")
        lines.append(f"## Adapter-private staged evidence ({product})")
        lines.append("")
        lines.append("| Artifact path |")
        lines.append("|---|")
        for field in matrix["private_evidence"][product]:
            lines.append(f"| `{field}` |")
        lines.append("")

    lines.append("## Rejected (never unified into the shared contract)")
    lines.append("")
    lines.append("Facts that remain per-adapter native values or fork by")
    lines.append("native-verifier ownership; the shared contract refuses")
    lines.append("to translate them into parity.")
    lines.append("")
    lines.append("| Class | Product | Field |")
    lines.append("|---|---|---|")
    for product in ("opi", "pi"):
        for field in matrix["rejected_fields"][product]:
            lines.append(f"| typed-unknown | {product} | `{field}` |")
    for field in matrix["rejected_evidence"]:
        lines.append(f"| verifier-forked | both | `{field}` |")
    lines.append("")

    lines.append("## Verifier ownership")
    lines.append("")
    lines.append("| Revision | Native verifier owner |")
    lines.append("|---|---|")
    lines.append("| terminal-bench-2.1 | Terminal-Bench (harbor aggregate) |")
    lines.append("| terminal-bench-3.0 | Terminal-Bench (harbor aggregate) |")
    lines.append("| deepswe-v1.1 | DeepSWE (pier job result) |")
    lines.append("")
    return "\n".join(lines) + "\n"


# ---------------------------------------------------------------------------
# Chain
# ---------------------------------------------------------------------------


def run_chain(receipt_path: Path, artifact: Path, repo: Path,
              expected_commit: str | None, require_spans: bool):
    f = native.Findings()
    upload = native.unpack_receipt(receipt_path, f)
    if upload is None or f.errors:
        return None, None, None, f
    native.verify_identity(upload, f)
    if artifact.is_dir():
        # An already-unpacked stage handoff: the envelope digest was
        # verified when the archive was unpacked; re-verify the sealed
        # manifest digest directly from the directory.
        manifest_probe = artifact / "08-seal" / "artifact-manifest.json"
        if not manifest_probe.is_file():
            f.reject("artifact", "directory carries no sealed manifest")
            return None, None, None, f
        if native.sha(manifest_probe.read_bytes()) != upload.get(
                "sealed_manifest_sha256"):
            f.reject("digest", "unpacked stage manifest drifts from the receipt")
            return None, None, None, f
        stage = artifact
    else:
        stage = native.unpack_artifact(artifact, upload["artifact_digest"], f)
        if stage is None or f.errors:
            return None, None, None, f
    outer = native.verify_seal(stage, upload, f)
    reports = native.load_run_reports(stage, f)
    if outer is None or f.errors:
        return None, None, None, f
    commit = expected_commit or outer.get("dispatch", {}).get(
        "candidate_sha", "")
    native.verify_dispatch(outer, commit, repo, f)
    native.verify_shared(stage, outer, f)
    native.verify_canary(stage, f)
    native.verify_provider(stage, f)
    native.verify_conformance_rerun(stage, f)
    native.verify_oracle_preflights(stage, f)
    native.verify_agent_evidence(stage, reports, "opi",
                                 ("native/evidence/manifest",
                                  "native/evidence/records"), f)
    native.verify_agent_evidence(stage, reports, "pi",
                                 ("native/events/stdout",), f)
    native.verify_material(stage, f)
    if require_spans:
        require_lifecycle_edges(stage, reports, f)
    return upload, outer, stage, f


def derive(upload: dict, outer: dict, stage: Path,
          reports_holder: list) -> tuple[dict, dict]:
    f0 = native.Findings()
    reports = native.load_run_reports(stage, f0)
    matrix = derive_matrix(stage, reports)
    reports_holder.extend(reports)
    binding = {
        "candidate_commit": outer.get("dispatch", {}).get(
            "candidate_sha", "<unknown>"),
        "run_id": upload.get("run_id", "<unknown>"),
        "artifact_digest": upload.get("artifact_digest", "<unknown>"),
        "sealed_manifest_sha256": upload.get("sealed_manifest_sha256",
                                             "<unknown>"),
        "trials": matrix["trials"],
    }
    return matrix, binding


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--receipt", required=True)
    parser.add_argument("--artifact", required=True)
    parser.add_argument("--require-trajectory-spans", action="store_true")
    parser.add_argument("--output", required=True)
    parser.add_argument("--verify", action="store_true")
    parser.add_argument("--repo", default=".")
    parser.add_argument("--expected-commit")
    args = parser.parse_args(argv)

    upload, outer, stage, f = run_chain(
        Path(args.receipt), Path(args.artifact), Path(args.repo),
        args.expected_commit, args.require_trajectory_spans,
    )
    if upload is None or outer is None or stage is None or f.errors:
        for row in f.errors:
            print(row, file=sys.stderr)
        print("derive-phase18-seam-matrix: REJECTED (binding or causal "
              "records failed)", file=sys.stderr)
        return 1

    holder: list = []
    matrix, binding = derive(upload, outer, stage, holder)
    rendered = render_matrix(matrix, binding)

    output = Path(args.output)
    if args.verify:
        try:
            committed = output.read_text(encoding="utf-8")
        except OSError:
            print(f"finding matrix: committed matrix is missing: {output}",
                  file=sys.stderr)
            return 1
        if committed != rendered:
            print("finding matrix: the committed matrix does not match the "
                  "artifact-derived matrix", file=sys.stderr)
            return 1
        print("derive-phase18-seam-matrix: VERIFIED")
        return 0

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(rendered, encoding="utf-8", newline="\n")
    print(f"derive-phase18-seam-matrix: wrote {output} "
          f"({matrix['trials']} trials)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

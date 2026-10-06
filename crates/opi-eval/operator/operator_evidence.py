"""Semantic comparison keys and lossless native-report derivatives."""
from __future__ import annotations
import copy
from datetime import datetime, timezone
import json
from pathlib import Path
import re
import tomllib
import math

from operator_config import BENCHMARKS
from operator_process import OperatorError, digest, pin, sha, write_json


def read_json(path):
    try:
        return json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, ValueError) as exc:
        raise OperatorError("unreadable machine evidence") from exc


def semantic_identity(profile, bench, material, plan):
    """Ignore transport paths and subject bytes, bind real shared outcome conditions."""
    adapter = BENCHMARKS[bench][2]
    benchmark = material["benchmarks"][adapter]
    experiment = plan["experiment"]
    if "source" in experiment:
        experiment = tomllib.loads(experiment["source"])
    connections = experiment.get("connections", [])
    selected = next((row for row in connections if row["id"] == experiment.get("connection")), None)
    if selected is None:
        raise OperatorError("plan has no selected frozen model connection")
    connection = {key: selected[key] for key in ("provider", "model", "api", "base_url")}
    runner = benchmark.get("runner_project") or {}
    selected_products = [row["product"] for row in experiment.get("subjects", [])]
    proofs = [material.get("agents", {}).get(product, {}).get("output_control_evidence") for product in selected_products]
    proof_admitted = bool(proofs) and all(proofs)
    observations = next((row.get("observations") for row in plan["corpus"] if row.get("selected") and row.get("observations")), {})
    environment = {key: observations.get(key) for key in ("engine_id", "engine_cpu", "engine_memory", "engine_gpus", "gpu_types")}
    for key in ("engine_cpu", "engine_memory", "engine_gpus"):
        observed = environment.get(key) or {}
        environment[key] = {"status": observed.get("status"), "value": observed.get("value")}
    verifier_env = {key: value for key, value in benchmark.get("verifier_env", {}).items()
                    if key in {"DOCKER_HOST", "DOCKER_CONTEXT", "HTTPS_PROXY", "HTTP_PROXY", "NO_PROXY", "SSL_CERT_FILE", "SSL_CERT_DIR"}}
    common = {"benchmark": {key: experiment["benchmark"][key] for key in ("name", "revision", "dataset")},
              "corpus": {key: benchmark["corpus"][key] for key in ("source_commit", "tasks_tree")},
              "connection": connection, "controls": experiment["model_controls"],
              "runner": {"source_manifest_sha256": runner.get("source_manifest_sha256"),
                         "verifier_sha256": benchmark["verifier_executable"]["sha256"]},
              "environment": {"contract": experiment.get("environment", {}), "engine": environment,
                              "verifier_connection": verifier_env},
              "controls_admission": "exact-client-loopback" if proof_admitted else "unproven",
              "execution": {key: value for key, value in experiment.get("execution", {}).items()
                            if key not in {"max_trials", "concurrency"}},
              "capture_policy": experiment["storage"]["capture_policy"]}
    tasks = {}
    corpus = {row["id"]: row for row in plan["corpus"]}
    for task, entry in benchmark["tasks"].items():
        row = corpus.get(task, {})
        images = row.get("image_material", {})
        image_entries = images.get("images", [])
        image_known = images.get("status") == "ok" and all(image.get("status") == "known" and "@sha256:" in image.get("image", "") for image in image_entries)
        tasks[task] = {"package_manifest_sha256": entry["task_package_manifest_sha256"],
                       "profile_sha256": entry["profile"]["sha256"],
                       "requirements": row.get("requirements", {}),
                       "images": sorted(image["image"] for image in image_entries),
                       "admitted": image_known and row.get("status") == "ok"}
    common["admitted"] = bool(runner.get("source_manifest_sha256") and benchmark["verifier_executable"].get("sha256")
                               and proof_admitted and observations.get("engine_id")
                               and environment["engine_cpu"].get("status") == "known"
                               and environment["engine_memory"].get("status") == "known"
                               and experiment["model_controls"].get("reasoning") not in {"omitted", "unknown"}
                               and experiment["model_controls"].get("temperature") == "omitted"
                               and experiment["model_controls"].get("top_p") == "omitted")
    return copy.deepcopy({"common": common, "tasks": tasks})


def retry_trials(planned, report, attempt):
    results = {trial["id"]: trial for trial in report.get("trials", [])}
    retries = []
    for declared in planned:
        trial = results.get(declared["id"])
        if trial is None:
            continue  # unstarted trials are reported, never invented as executed failures
        outcome = trial.get("outcome", {})
        if outcome.get("class") != "invalid":
            continue
        reason = str(outcome.get("reason", "unknown"))
        boundary = str(outcome.get("boundary", ""))
        if (any(token in reason.lower() for token in ("authority", "privacy", "canary", "credential-leak", "boundary", "unconfirmed", "credential-material-removal-failed"))
                or boundary.lower() in {"authority", "privacy", "integrity", "evidence"}):
            raise OperatorError("authority or privacy failure forbids retries")
        if reason not in {"provider-access", "verifier-timeout"} and not reason.startswith(("agent-provider-failure:", "job-exception:")):
            continue
        replacement = copy.deepcopy(declared)
        replacement["id"] = declared["id"] + f"-retry-{attempt}"
        retries.append(replacement)
    return retries


def logical_slots(plan, reports):
    originals = plan["trials"]
    results = []
    for attempt in reports:
        for trial in attempt["report"].get("trials", []):
            results.append((trial, attempt))
    slots = []
    per_task = {}
    for declared in originals:
        task = declared["task"]
        repetition = per_task.get(task, 0) + 1
        per_task[task] = repetition
        seed = plan.get("selection", {}).get("seeds", {}).get(task)
        attempts = []
        for trial, run in results:
            if (trial.get("task"), trial.get("group"), trial.get("subject")) != (task, declared["group"], declared["subject"]):
                continue
            identity = trial["id"]
            attempts.append({"trial": identity, "root": run["root"], "report": run["report_pin"],
                             "outcome": trial.get("outcome", {}), "native_facts": trial.get("native_facts", {}),
                             "headline": trial.get("headline")})
        graded = [row for row in attempts if row["outcome"].get("class") == "graded"]
        selected = graded[0] if graded else (attempts[-1] if attempts else None)
        headline = (selected or {}).get("headline") or {}
        token = headline.get("reward")
        # Companion /2 uses a fact token, with an independent grader-artifact
        # provenance citation. Unknown facts are preserved as missing measurements.
        match = re.fullmatch(r"known:([01](?:\.0+)?)(?:\(native-trial-result\))", token) if isinstance(token, str) else None
        source = headline.get("native_source", {})
        measured = bool(match and source.get("artifact") == "native/trial-result.json" and re.fullmatch(r"[0-9a-f]{64}", source.get("digest", "")))
        reward = float(match[1]) if measured else None
        slots.append({"task": task, "repetition": repetition, "seed": seed,
                      "group": declared["group"], "subject": declared["subject"], "trial": declared["id"],
                      "valid": bool(graded) and measured, "reward": reward if graded and measured else None,
                      "selected": selected, "attempts": attempts,
                      "state": "graded" if graded and measured else ("invalid" if attempts else "unexecuted")})
    return slots


def unverified_slots(slots, attempts):
    """A dispatched run lacking a report cannot establish unstarted coverage."""
    for slot in slots:
        unverified = []
        for attempt in attempts:
            if attempt.get("verification") == "verified":
                continue
            for trial in attempt["trials"]:
                if (trial["task"], trial["subject"], trial["group"]) != (slot["task"], slot["subject"], slot["group"]):
                    continue
                receipt = Path(attempt["root"]) / "trials" / trial["id"] / "receipt.json"
                unverified.append({"root": attempt["root"], "trial": trial["id"],
                                   "receipt": pin(receipt) if receipt.is_file() else None,
                                   "effect_state": "started-receipt-unverified" if receipt.is_file() else "effect_unknown"})
        if unverified:
            slot["unverified_attempts"] = unverified
            if not slot["valid"]:
                slot["state"] = "verification_unknown"
    return slots


def eligible(baseline, candidate):
    if baseline.get("product") != "pi" or not baseline.get("complete") or baseline.get("status", "complete") != "complete":
        return False
    if baseline["semantic"]["common"] != candidate["semantic"]["common"]:
        return False
    if not baseline["semantic"]["common"].get("admitted"):
        return False
    # Companion changes require a separate reviewed grading-path equivalence artifact.
    companion_sha = baseline.get("companion", {}).get("sha256", "")
    if not re.fullmatch(r"[0-9a-f]{64}", companion_sha) or companion_sha != candidate.get("companion", {}).get("sha256"):
        return False
    requested_pi = candidate.get("pi_version")
    if not requested_pi or baseline.get("version") != requested_pi:
        return False
    shared = set(baseline["semantic"]["tasks"]) & set(candidate["semantic"]["tasks"])
    return bool(shared) and any(baseline["semantic"]["tasks"][task].get("admitted") and baseline["semantic"]["tasks"][task] == candidate["semantic"]["tasks"][task] for task in shared)


def choose_baseline(snapshots, candidate):
    matches = [value for value in snapshots if eligible(value, candidate)]
    def completed(value):
        try:
            stamp = datetime.fromisoformat(value["completed_at"].replace("Z", "+00:00"))
            return stamp.replace(tzinfo=stamp.tzinfo or timezone.utc).astimezone(timezone.utc), value["id"]
        except (KeyError, ValueError, TypeError, AttributeError) as exc:
            raise OperatorError("baseline completion time is unverifiable") from exc
    return max(matches, key=completed) if matches else None


def compare_snapshots(candidate, baseline):
    if candidate.get("product") != "opi" or (baseline is not None and baseline.get("product") != "pi"):
        raise OperatorError("comparison requires an Opi candidate and a pi baseline")
    result = {"schema": "opi-eval-cross-run-comparison/1", "classification": "conformance-evidence",
              "candidate": candidate.get("self"), "baseline": baseline.get("self") if baseline else None,
              "status": "baseline_missing" if baseline is None else "comparable",
              "paired_reward_delta": None, "pairs": [],
              "efficiency": {"state": "unknown", "reason": "native reports do not provide symmetric complete timing/usage/cost measurements"}}
    if baseline is None:
        result["coverage"] = {"paired": 0, "candidate_slots": len(candidate["slots"]), "baseline_slots": 0}
        return result
    if not eligible(baseline, candidate):
        result["status"] = "incomparable"
        result["reason"] = "baseline incomplete, unproven, wrong pi pin, changed Companion or shared execution identities differ"
        return result
    def slots(snapshot):
        return {(slot["task"], slot["repetition"], slot["seed"]): slot for slot in snapshot["slots"]}
    left, right = slots(baseline), slots(candidate)
    deltas = []
    counts = {"paired": 0, "missing_baseline": 0, "missing_candidate": 0, "invalid": 0, "identity_mismatch": 0}
    for key in sorted(set(left) | set(right), key=lambda key: (key[0], key[1], str(key[2]))):
        pi, opi = left.get(key), right.get(key)
        state = "paired"
        if pi is None:
            state = "missing_baseline"
        elif opi is None:
            state = "missing_candidate"
        elif baseline["semantic"]["tasks"].get(key[0]) != candidate["semantic"]["tasks"].get(key[0]):
            state = "identity_mismatch"
        elif not pi["valid"] or not opi["valid"]:
            state = "invalid"
        counts[state] += 1
        delta = opi["reward"] - pi["reward"] if state == "paired" else None
        if delta is not None:
            deltas.append(delta)
        result["pairs"].append({"task": key[0], "repetition": key[1], "seed": key[2], "state": state,
                                "baseline_evidence": (pi or {}).get("selected"),
                                "candidate_evidence": (opi or {}).get("selected"),
                                "baseline_reward": (pi or {}).get("reward"), "candidate_reward": (opi or {}).get("reward"),
                                "reward_delta": delta})
    result["coverage"] = counts
    result["paired_reward_delta"] = sum(deltas) / len(deltas) if deltas else None
    if any(counts[key] for key in counts if key != "paired"):
        result["status"] = "partial"
    return result


def indexed_snapshots(index):
    if not Path(index).is_file():
        return []
    values = []
    for line in Path(index).read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        entry = json.loads(line)
        if entry.get("kind") != "pi-baseline-snapshot":
            continue
        snapshot_pin = entry.get("snapshot")
        if not isinstance(snapshot_pin, dict) or not Path(snapshot_pin.get("path", "")).is_file():
            continue
        if sha(snapshot_pin["path"]) != snapshot_pin.get("sha256"):
            continue
        value = read_json(snapshot_pin["path"])
        value["self"] = snapshot_pin
        values.append(value)
    return values

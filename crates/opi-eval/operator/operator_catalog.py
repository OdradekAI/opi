"""Operator-owned discovery metadata; original native evidence remains authoritative."""
from __future__ import annotations

from contextlib import contextmanager
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path, PurePosixPath, PureWindowsPath
import re
import uuid

from operator_process import OperatorError, digest, pin, sha, utc_now

MAX_JSON_BYTES = 8 * 1024 * 1024
MAX_ROWS = 10000
MAX_RECORDS = 100000
MAX_RECEIPTS = 10000
BENCHES = {"tb21", "tb30", "deepswe"}
LEGACY_BENCHES = {"terminal-bench-2.1": "tb21", "terminal-bench-3.0": "tb30", "deepswe-v1.1": "deepswe"}
ID = re.compile(r"[A-Za-z0-9][A-Za-z0-9_-]{0,127}\Z")
EVENT_ACTIONS = {"registration", "capture", "archive", "restore", "purge", "retire", "unretire", "pin", "unpin"}


def safe_path(value, *, within=None):
    """Reject lexical traversal and existing links before resolving a path."""
    try:
        path = Path(value)
    except (TypeError, ValueError) as exc:
        raise OperatorError("invalid catalogue path") from exc
    if not path.is_absolute() or ".." in path.parts:
        raise OperatorError("catalogue paths must be absolute without traversal")
    for component in (path, *path.parents):
        if component.is_symlink() or (hasattr(component, "is_junction") and component.is_junction()):
            raise OperatorError("catalogue symlink or junction refused")
    path = path.resolve()
    if within is not None and path != within and within not in path.parents:
        raise OperatorError("catalogue path escapes owned storage")
    return path


def read_json(path):
    path = safe_path(path)
    try:
        with path.open("rb") as stream:
            data = stream.read(MAX_JSON_BYTES + 1)
        if len(data) > MAX_JSON_BYTES:
            raise OperatorError("catalogue JSON exceeds byte bound")
        value = json.loads(data.decode("utf-8"), parse_constant=_invalid_number)
        if not isinstance(value, dict):
            raise OperatorError("catalogue JSON must be an object")
        return value
    except (OSError, ValueError, RecursionError) as exc:
        raise OperatorError("unreadable catalogue JSON") from exc


def _invalid_number(value):
    raise ValueError("nonfinite JSON number")


def _external_path(value, *, within=None):
    """Preserve unavailable foreign-platform references without interpreting them locally."""
    if not isinstance(value, str) or not value or any(ord(character) < 32 for character in value):
        raise OperatorError("invalid external reference path")
    if Path(value).is_absolute():
        return str(safe_path(value, within=Path(within) if within else None))
    path = PurePosixPath(value) if PurePosixPath(value).is_absolute() else PureWindowsPath(value)
    if not path.is_absolute() or ".." in path.parts:
        raise OperatorError("invalid external reference path")
    if within is not None and not path.is_relative_to(type(path)(within)):
        raise OperatorError("external snapshot escapes referenced root")
    return str(path)


def _identity(value):
    if not isinstance(value, str) or not ID.fullmatch(value):
        raise OperatorError("invalid evaluation identity")
    return value


def _metadata(profile):
    def select(value, fields):
        if not isinstance(value, dict):
            raise OperatorError("catalogue profile metadata must be tables")
        return {key: value[key] for key in fields if key in value}
    if not isinstance(profile, dict):
        raise OperatorError("catalogue profile metadata must be tables")
    model = profile.get("model", {})
    result = {"schema": profile.get("schema"), "model": select(model, ("provider", "model", "api"))}
    for key, fields in (("controls", ("context_window", "max_output_tokens", "temperature", "top_p")),
                        ("thinking", ("level", "required"))):
        if key in model:
            result["model"][key] = select(model[key], fields)
    for key, fields in (("execution", ("trial_retries", "batch_timeout_seconds", "concurrency", "capture_policy")),
                        ("baseline", ("pi_version", "lookup", "max_age_days")),
                        ("retention", ("keep_latest_per_group",))):
        if key in profile:
            result[key] = select(profile[key], fields)
    if "benchmarks" in profile:
        if not isinstance(profile["benchmarks"], dict):
            raise OperatorError("catalogue benchmark metadata must be a table")
        result["benchmarks"] = {bench: select(value, ("scope", "repetitions", "task_ids", "seeds"))
                                for bench, value in profile["benchmarks"].items() if bench in BENCHES}
    return result


def _encoded(value, *, pretty=False):
    data = bytearray()
    try:
        encoder = json.JSONEncoder(sort_keys=True, indent=2 if pretty else None, allow_nan=False)
        for chunk in encoder.iterencode(value):
            data.extend(chunk.encode())
            if len(data) > MAX_JSON_BYTES - 1:
                raise OperatorError("catalogue JSON exceeds byte bound")
    except (ValueError, TypeError, RecursionError) as exc:
        raise OperatorError("invalid catalogue JSON value") from exc
    data.extend(b"\n")
    return bytes(data)


def _write(path, value):
    data = _encoded(value, pretty=True)
    path = safe_path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    try:
        descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    except FileExistsError as exc:
        raise OperatorError("catalogue artifact already exists") from exc
    with os.fdopen(descriptor, "wb") as stream:
        stream.write(data)
        stream.flush()
        os.fsync(stream.fileno())
    if hasattr(os, "O_DIRECTORY"):
        descriptor = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY)
        try:
            os.fsync(descriptor)
        finally:
            os.close(descriptor)


def _time(value):
    try:
        result = datetime.fromisoformat(value.replace("Z", "+00:00"))
        if result.tzinfo is None:
            raise ValueError()
        return result.astimezone(timezone.utc)
    except (AttributeError, ValueError, TypeError) as exc:
        raise OperatorError("invalid catalogue timestamp") from exc


def _registered_at(value):
    if isinstance(value, str) and re.fullmatch(r"\d{4}-\d{2}-\d{2}", value):
        value += "T00:00:00+00:00"
    return _time(value).isoformat()


def _pin_state(binding):
    if not isinstance(binding, dict) or not isinstance(binding.get("sha256"), str) or not re.fullmatch(r"[0-9a-f]{64}", binding["sha256"]):
        return "unverifiable"
    try:
        path = safe_path(binding.get("path", ""))
        if not path.is_file():
            return "missing"
        return "verified" if sha(path) == binding["sha256"] else "drifted"
    except (OperatorError, OSError, ValueError, TypeError):
        return "unverifiable"


def _no_dispatch(root, families):
    for directory in (root, *families):
        for name in ("execution-authorization.json", "live-authorization.json", "runs"):
            if safe_path(directory / name, within=root).exists():
                return False
    return True


class Store:
    """Discover external immutable records and receipts without a durable query cache."""
    def __init__(self, workspace):
        self.workspace = safe_path(workspace)
        self.evaluations = self.workspace / "evaluations"
        self.imports = self.workspace / "imports"
        self.events = self.workspace / "events"
        self.leases = self.workspace / "leases"
        self.workspace.mkdir(parents=True, exist_ok=True)
        for path in (self.evaluations, self.imports, self.events, self.leases):
            safe_path(path, within=self.workspace)

    @staticmethod
    def _paths(directory, pattern, limit=MAX_RECORDS):
        paths = []
        for path in directory.glob(pattern):
            if len(paths) == limit:
                raise OperatorError("evaluation record scan exceeds entry bound")
            paths.append(path)
        return sorted(paths)

    def create(self, action, benches, profile):
        if action not in {"baseline", "candidate", "compare", "acceptance"}:
            raise OperatorError("unknown catalogue action")
        if not isinstance(benches, (list, tuple)) or not benches or any(not isinstance(bench, str) for bench in benches) or len(set(benches)) != len(benches) or set(benches) - BENCHES:
            raise OperatorError("invalid catalogue benchmark families")
        now = datetime.now(timezone.utc)
        identity = action + "-" + uuid.uuid4().hex
        root = self.evaluations / now.strftime("%Y/%m") / identity
        safe_path(root, within=self.evaluations)
        metadata = _metadata(profile)
        record = {"schema": "opi-eval-record/1", "id": identity, "root": str(root), "action": action,
                  "created_at": now.isoformat(), "families": {bench: str(root / "families" / bench) for bench in benches},
                  "profile": metadata, "retention": metadata.get("retention", {})}
        _encoded(record, pretty=True)
        root.mkdir(parents=True, mode=0o700)
        _write(root / "record.json", record)
        self._append_event(identity, "registration", {"record": record, "record_pin": pin(root / "record.json")})
        self.refresh(root)
        return root

    def _receipts(self, identity):
        directory = safe_path(self.events / _identity(identity), within=self.events)
        receipts = []
        for path in self._paths(directory, "*.json", MAX_RECEIPTS):
            value = read_json(path)
            if value.get("schema") != "opi-eval-lifecycle-event/1" or value.get("id") != identity:
                raise OperatorError("invalid catalogue lifecycle receipt")
            if value.get("action") not in EVENT_ACTIONS or not isinstance(value.get("fields"), dict):
                raise OperatorError("unknown catalogue lifecycle receipt contract")
            _time(value.get("at"))
            receipts.append(value)
        return receipts

    def _append_event(self, identity, action, fields):
        now = datetime.now(timezone.utc)
        receipt = {"schema": "opi-eval-lifecycle-event/1", "id": _identity(identity),
                   "action": action, "at": now.isoformat(), "fields": fields}
        path = self.events / identity / (now.strftime("%Y%m%dT%H%M%S%fZ") + "-" + uuid.uuid4().hex + ".json")
        _write(path, receipt)
        return receipt

    def refresh(self, root):
        root = safe_path(root, within=self.evaluations)
        receipts = self._receipts(root.name)
        registration = next((receipt["fields"] for receipt in receipts if receipt["action"] == "registration"), None)
        parts = root.relative_to(self.evaluations).parts
        if len(parts) != 3 or not re.fullmatch(r"\d{4}", parts[0]) or not re.fullmatch(r"0[1-9]|1[0-2]", parts[1]):
            raise OperatorError("evaluation record must have a dated owned root")
        record_path = safe_path(root / "record.json", within=root)
        if registration and (not isinstance(registration.get("record"), dict)
                             or not isinstance(registration.get("record_pin"), dict)
                             or safe_path(registration["record_pin"].get("path"), within=root) != record_path):
            raise OperatorError("invalid immutable registration binding")
        record_missing = not record_path.is_file()
        if record_missing and registration:
            record = registration["record"]
        else:
            record = read_json(record_path)
        if not record_missing and registration and _pin_state(registration["record_pin"]) != "verified":
            raise OperatorError("immutable evaluation identity drift")
        if record.get("schema") != "opi-eval-record/1" or record.get("root") != str(root) or record.get("id") != root.name:
            raise OperatorError("invalid evaluation record binding")
        _identity(record["id"])
        _time(record.get("created_at"))
        if record.get("action") not in {"baseline", "candidate", "compare", "acceptance"} or not isinstance(record.get("families"), dict) or not record["families"]:
            raise OperatorError("invalid evaluation record contract")
        families = {}
        bindings = {}
        final_binding = None
        failure_binding = None
        storage_state, selection_state, pinned = "active", "active", False
        lifecycle = {}
        for receipt in receipts:
            if receipt["action"] == "capture":
                for bench, captured in receipt["fields"].get("families", {}).items():
                    bindings.setdefault(bench, {}).update(captured)
                if "evaluation" in receipt["fields"]:
                    final_binding = receipt["fields"]["evaluation"]
                if "preparation_failure" in receipt["fields"]:
                    failure_binding = receipt["fields"]["preparation_failure"]
            else:
                action = receipt["action"]
                if action in {"archive", "restore", "purge"}:
                    storage_state = {"archive": "archived", "restore": "active", "purge": "purged"}[action]
                    lifecycle.update(receipt["fields"])
                    lifecycle["action"] = action
                    lifecycle["at"] = receipt["at"]
                elif action in {"retire", "unretire"}:
                    selection_state = "retired" if action == "retire" else "active"
                elif action in {"pin", "unpin"}:
                    pinned = action == "pin"
        failure_path = root / "preparation-failure.json"
        failure_capture, failure_unknown = None, []
        if failure_binding is None and failure_path.is_file():
            try:
                document = read_json(failure_path)
                results = document.get("families")
                if (document.get("schema") != "opi-eval-preparation-failure/1" or document.get("evaluation") != str(root)
                        or document.get("status") != "preparation_failed" or document.get("live_dispatch") is not False
                        or not isinstance(results, dict) or set(results) != set(record["families"])):
                    raise OperatorError("unknown grouped preparation failure contract")
                for bench, result in results.items():
                    if (not isinstance(result, dict) or result.get("root") != record["families"][bench]
                            or result.get("status") not in {"prepared", "preparation_failed", "not_started"}
                            or result.get("live_dispatch") is not False):
                        raise OperatorError("unknown family preparation failure contract")
                failure_capture = {"pin": pin(failure_path), "status": "preparation_failed", "families": results,
                                   "completed_at": _time(document.get("completed_at", record["created_at"])).isoformat()}
                failure_binding = failure_capture
            except (OperatorError, AttributeError, TypeError):
                failure_unknown.append("preparation-failure-unverifiable")
        root_pre_live = False
        if failure_binding:
            if safe_path(failure_binding["pin"]["path"], within=root) != failure_path:
                raise OperatorError("preparation failure binding escaped owned root")
            if storage_state == "active":
                root_pre_live = (_pin_state(failure_binding["pin"]) == "verified"
                                 and _no_dispatch(root, [safe_path(path, within=root) for path in record["families"].values()]))
                if not root_pre_live:
                    failure_unknown.append("preparation-failure-unverifiable")
        capture = {}
        for bench, value in record.get("families", {}).items():
            family = safe_path(value, within=root)
            if bench not in BENCHES or family != root / "families" / bench:
                raise OperatorError("invalid evaluation family binding")
            item = {"root": str(family), "status": "created", "snapshot": None, "effective": None,
                    "product": "pi" if record["action"] == "baseline" else "opi", "version": None, "completed_at": None,
                    "verification_state": "unprepared"}
            bound = bindings.get(bench, {})
            if not isinstance(bound, dict):
                raise OperatorError("invalid catalogue capture contract")
            for key, filename in (("prepared", "prepared.json"), ("snapshot", "snapshot.json"), ("effective", "inputs/effective.json"),
                                  ("preparation_failure", "preparation-failure.json"),
                                  ("comparison_input", "inputs/comparison-input.json")):
                if key in bound and (not isinstance(bound[key], dict) or safe_path(bound[key].get("path"), within=family) != family / filename):
                    raise OperatorError("catalogue capture escaped original family artifact")
            captured = {}
            for key, filename in (("prepared", "prepared.json"), ("snapshot", "snapshot.json"), ("effective", "inputs/effective.json"),
                                  ("preparation_failure", "preparation-failure.json"),
                                  ("comparison_input", "inputs/comparison-input.json")):
                path = family / filename
                if key not in bound and path.is_file():
                    safe_path(path, within=family)
                    captured[key] = pin(path)
            if captured:
                capture[bench] = captured
                bound = {**bound, **captured}
            item.update({key: bound.get(key) for key in ("prepared", "snapshot", "effective", "preparation_failure")})
            references = list(bound.get("references", []))
            unknown = []
            comparison = bound.get("comparison_input")
            family_failure = bound.get("failure_metadata", {})
            pre_live = False
            if item["preparation_failure"] and storage_state == "active":
                try:
                    if _pin_state(item["preparation_failure"]) != "verified":
                        raise OperatorError("preparation failure is not original")
                    document = read_json(item["preparation_failure"]["path"])
                    if (document.get("status") != "preparation_failed" or document.get("live_dispatch") is not False
                            or document.get("commands") != str(family / "commands")):
                        raise OperatorError("unknown preparation failure contract")
                    metadata = {"status": "preparation_failed", "completed_at": _time(document.get("at")).isoformat()}
                    if "failure_metadata" not in bound:
                        captured["failure_metadata"] = metadata
                    family_failure = metadata
                    pre_live = not item["prepared"] and not item["snapshot"] and _no_dispatch(root, [family])
                    if not pre_live and not comparison:
                        unknown.append("preparation-failure-dispatch-unknown:" + bench)
                except (OperatorError, AttributeError, TypeError):
                    unknown.append("preparation-failure-unverifiable:" + bench)
            root_family = failure_binding["families"].get(bench) if failure_binding else None
            if root_family and root_pre_live:
                if root_family["status"] == "prepared":
                    pre_live = bool(item["prepared"]) and not item["snapshot"]
                elif root_family["status"] == "not_started":
                    pre_live = not item["prepared"] and not item["snapshot"] and not comparison and not item["preparation_failure"]
                if not pre_live:
                    unknown.append("preparation-failure-family-unknown:" + bench)
            item["pre_live"] = pre_live
            if storage_state in {"archived", "purged"}:
                unknown = list(bound.get("reference_unknown", []))
            elif comparison:
                if _pin_state(comparison) != "verified":
                    unknown.append("comparison-input-unverifiable:" + bench)
                else:
                    try:
                        document = read_json(comparison["path"])
                        if document.get("schema") != "opi-eval-comparison-input/1" or "baseline" not in document:
                            raise OperatorError("unknown comparison input contract")
                        if record["action"] == "compare" and not isinstance(document.get("candidate"), dict):
                            raise OperatorError("unknown comparison candidate identity")
                        for name in ("candidate", "baseline"):
                            reference = document.get(name)
                            if reference is None:
                                continue
                            path = safe_path(reference["path"])
                            if not re.fullmatch(r"[0-9a-f]{64}", reference.get("sha256", "")):
                                raise OperatorError("unknown comparison baseline identity")
                            references = sorted(set(references + [str(path)]))
                    except (OperatorError, KeyError, TypeError, ValueError):
                        unknown.append("comparison-reference-unknown:" + bench)
            elif record["action"] != "baseline" and not pre_live:
                unknown.append("comparison-input-missing:" + bench)
            if item["prepared"] and storage_state == "active":
                if _pin_state(item["prepared"]) != "verified":
                    unknown.append("prepared-operation-unverifiable:" + bench)
                else:
                    prepared = read_json(item["prepared"]["path"])
                    if prepared.get("comparison_input") != comparison:
                        unknown.append("prepared-comparison-binding-unknown:" + bench)
            item["references"] = references
            item["reference_unknown"] = unknown
            if item["prepared"]:
                item["status"] = "prepared"
            if item["snapshot"]:
                item["verification_state"] = storage_state if storage_state in {"archived", "purged"} else _pin_state(item["snapshot"])
                if item["verification_state"] == "verified":
                    try:
                        snapshot = read_json(item["snapshot"]["path"])
                        if snapshot.get("schema") != "opi-eval-baseline-snapshot/1" or snapshot.get("bench") != bench or snapshot.get("product") not in {"pi", "opi"}:
                            raise OperatorError("unknown snapshot contract")
                        _time(snapshot.get("completed_at"))
                        item.update({key: snapshot.get(key) for key in ("status", "product", "version", "completed_at")})
                        item["complete"] = snapshot.get("complete", False)
                        item["semantic"] = snapshot.get("semantic")
                        item["companion"] = snapshot.get("companion")
                        captured["metadata"] = {key: item.get(key) for key in ("status", "product", "version", "completed_at", "complete", "semantic", "companion")}
                        if bench in capture:
                            capture[bench]["metadata"] = captured["metadata"]
                    except OperatorError:
                        item["verification_state"] = "unverifiable"
                        item["status"] = "unverifiable"
                        item["reference_unknown"].append("snapshot-contract-unknown:" + bench)
                elif "metadata" in bound:
                    item.update(bound["metadata"])
            elif (family / "handoff.json").is_file():
                item["status"] = read_json(family / "handoff.json").get("status", "unverifiable")
            if family_failure and not item["snapshot"]:
                item.update(family_failure)
            if root_family and not item["snapshot"]:
                item["status"] = root_family["status"]
            if (root_family or family_failure) and storage_state in {"archived", "purged"}:
                item["pre_live"] = bound.get("pre_live", False)
            if captured or (root_pre_live and bound.get("pre_live") != pre_live):
                capture[bench] = captured
                captured["references"] = references
                captured["reference_unknown"] = unknown
                captured["pre_live"] = pre_live
            families[bench] = item
        statuses = [item["status"] for item in families.values()]
        status = next((value for value in ("preparation_failed", "stopped", "incomplete", "unverifiable") if value in statuses),
                      "complete" if statuses and all(value == "complete" for value in statuses) else ("prepared" if "prepared" in statuses else "created"))
        completed = [item["completed_at"] for item in families.values() if item.get("completed_at")]
        if failure_binding:
            status = failure_binding["status"]
            completed.append(failure_binding["completed_at"])
        final_capture, final_unknown = None, []
        final_path = root / "result-handoff.json"
        if final_binding is None and final_path.is_file():
            try:
                final = read_json(final_path)
                results = final.get("families")
                if (final.get("schema") != "opi-eval-evaluation-handoff/1" or final.get("evaluation") != str(root)
                        or not isinstance(results, dict) or set(results) != set(families)):
                    raise OperatorError("unknown evaluation handoff contract")
                states = [result.get("status") for result in results.values()]
                if any(state not in {"complete", "incomplete", "stopped", "not_started"} for state in states):
                    raise OperatorError("unknown evaluation family result")
                final_status = "complete" if all(state == "complete" for state in states) else ("stopped" if "stopped" in states else "incomplete")
                if final.get("status") != final_status:
                    raise OperatorError("evaluation handoff summary differs from results")
                finished = _time(final.get("completed_at", record["created_at"])).isoformat()
                final_capture = {"result_handoff": pin(final_path), "status": final_status, "completed_at": finished}
                final_binding = final_capture
            except (OperatorError, AttributeError, TypeError):
                final_unknown.append("evaluation-handoff-unverifiable")
        if final_binding:
            if safe_path(final_binding["result_handoff"]["path"], within=root) != final_path:
                raise OperatorError("evaluation handoff binding escaped owned root")
            status = final_binding["status"]
            completed.append(final_binding["completed_at"])
            if storage_state == "active" and _pin_state(final_binding["result_handoff"]) != "verified":
                final_unknown.append("evaluation-handoff-unverifiable")
        if capture or final_capture or failure_capture:
            fields = {"families": capture}
            if final_capture:
                fields["evaluation"] = final_capture
            if failure_capture:
                fields["preparation_failure"] = failure_capture
            self._append_event(record["id"], "capture", fields)
        row = {"id": record["id"], "root": str(root), "ownership": "managed", "action": record["action"],
               "created_at": record["created_at"],
               "completed_at": final_binding["completed_at"] if final_binding else (failure_binding["completed_at"] if failure_binding else (max(completed, key=_time) if completed else None)), "status": status,
               "storage_state": storage_state, "selection_state": selection_state, "pinned": pinned,
               "references": sorted({path for item in families.values() for path in item["references"]}),
               "reference_unknown": sorted({gap for item in families.values() for gap in item["reference_unknown"]}
                                           | ({"evaluation-record-missing"} if record_missing else set())),
               "families": families, "record": record, "lifecycle": lifecycle,
               "preparation_failure": failure_binding["pin"] if failure_binding else None,
               "active_lease": safe_path(self.leases / (record["id"] + ".json"), within=self.leases).exists()}
        row["reference_unknown"] = sorted(set(row["reference_unknown"] + final_unknown + failure_unknown))
        if storage_state == "purged":
            if lifecycle.get("cleanup_state") == "confirmed":
                row["references"] = []
                row["reference_unknown"] = []
            else:
                row["reference_unknown"] = sorted(set(row["reference_unknown"] + ["purge-cleanup-pending"]))
        if record_missing:
            row["storage_state"] = "missing"
            row["reference_unknown"] = sorted(set(row["reference_unknown"] + ["evaluation-record-missing"]))
        _encoded(row)
        return row

    def event(self, identity, action, **fields):
        _identity(identity)
        if action not in {"archive", "restore", "purge", "retire", "unretire", "pin", "unpin"}:
            raise OperatorError("unknown catalogue lifecycle action")
        row = self.show(identity)
        if row["ownership"] != "managed" and action in {"archive", "restore", "purge"}:
            raise OperatorError("imported evidence is never maintained")
        if "archive_path" in fields:
            safe_path(fields["archive_path"], within=self.workspace / "archives")
        self._append_event(identity, action, fields)
        return self.show(identity)

    @contextmanager
    def lease(self, identity):
        """An exclusive operation/maintenance lease; stale owners require inspection."""
        row = self.show(_identity(identity))
        path = safe_path(self.leases / (identity + ".json"), within=self.leases)
        value = {"schema": "opi-eval-lease/1", "id": identity, "pid": os.getpid(),
                 "token": uuid.uuid4().hex, "created_at": utc_now()}
        _write(path, value)
        try:
            yield row
        finally:
            if read_json(path) != value:
                raise OperatorError("evaluation lease ownership changed; inspect owned resources")
            path.unlink()

    def reindex(self):
        """Validate a fresh bounded record scan; retain no derived query state."""
        managed = imported = 0
        for row in self._rows():
            if row["ownership"] == "managed":
                managed += 1
            else:
                imported += 1
        return {"schema": "opi-eval-record-scan/1", "indexed": managed + imported,
                "managed": managed, "imported": imported}

    def _rows(self):
        known = set()
        for path in self._paths(self.evaluations, "*/*/*/record.json"):
            row = self.refresh(path.parent)
            if row["id"] in known:
                raise OperatorError("duplicate evaluation identity")
            known.add(row["id"])
            yield row
        for directory in self._paths(self.events, "*"):
            identity = _identity(directory.name)
            if identity in known or (self.imports / (identity + ".json")).is_file():
                continue
            registration = self._registration(identity)
            if registration is None:
                raise OperatorError("lifecycle receipts have no immutable original identity")
            if len(known) == MAX_RECORDS:
                raise OperatorError("evaluation record scan exceeds entry bound")
            row = self.refresh(registration["record"]["root"])
            known.add(row["id"])
            yield row
        for path in self._paths(self.imports, "*.json"):
            if len(known) == MAX_RECORDS:
                raise OperatorError("evaluation record scan exceeds entry bound")
            row = self._import_row(read_json(path))
            if row["id"] != path.stem or row["id"] in known:
                raise OperatorError("invalid imported evaluation identity binding")
            known.add(row["id"])
            yield row

    def _registration(self, identity):
        registrations = [receipt["fields"] for receipt in self._receipts(identity) if receipt["action"] == "registration"]
        if len(registrations) > 1:
            raise OperatorError("duplicate immutable registration")
        if not registrations:
            return None
        registration = registrations[0]
        if not isinstance(registration.get("record"), dict) or registration["record"].get("id") != identity:
            raise OperatorError("invalid immutable registration identity")
        root = safe_path(registration["record"].get("root"), within=self.evaluations)
        if root.name != identity:
            raise OperatorError("invalid immutable registration root")
        return registration

    def import_index(self, path):
        """Freeze explicit legacy references without modifying or adopting their roots."""
        path = safe_path(path)
        try:
            if path.stat().st_size > 64 * 1024 * 1024:
                raise OperatorError("legacy registry exceeds byte bound")
            with path.open("rb") as stream:
                data = stream.read(64 * 1024 * 1024 + 1)
            if len(data) > 64 * 1024 * 1024:
                raise OperatorError("legacy registry exceeds byte bound")
            lines = data.decode("utf-8").splitlines()
        except (OSError, UnicodeError) as exc:
            raise OperatorError("unreadable legacy registry") from exc
        if len(lines) > 100000:
            raise OperatorError("legacy registry exceeds entry bound")
        source = {"path": str(path), "sha256": hashlib.sha256(data).hexdigest()}
        records = []
        for line in lines:
            if not line.strip():
                continue
            if len(line.encode()) > MAX_JSON_BYTES:
                raise OperatorError("legacy registry entry exceeds byte bound")
            try:
                entry = json.loads(line, parse_constant=_invalid_number)
                if not isinstance(entry, dict):
                    raise ValueError()
                root_value = entry.get("windows_path", entry["path"]) if os.name == "nt" else entry["path"]
                root = _external_path(root_value)
                kind = entry.get("kind", "unknown")
                if not isinstance(kind, str) or len(kind) > 128:
                    raise ValueError()
                snapshot = entry.get("snapshot") if kind in {"pi-baseline-snapshot", "opi-candidate-snapshot"} else None
                if snapshot is not None:
                    snapshot_path = _external_path(snapshot["path"], within=root)
                    if not re.fullmatch(r"[0-9a-f]{64}", snapshot.get("sha256", "")):
                        raise ValueError()
                    snapshot = {"path": snapshot_path, "sha256": snapshot["sha256"]}
                selected = {"root": root, "kind": kind, "snapshot": snapshot,
                            "families": entry.get("families", []), "registered": entry.get("registered"),
                            "status": entry.get("status", "unknown")}
                identity = "import-" + digest(selected)
                created_at = _registered_at(entry.get("registered") or utc_now())
                record = {"schema": "opi-eval-import/1", "id": identity, **selected,
                          "created_at": created_at, "source": source}
                imported_row = self._import_row(record)
                record["metadata"] = imported_row["families"]
                record["profile"] = imported_row["record"].get("profile", {})
                records.append(record)
            except (ValueError, TypeError, KeyError, RecursionError) as exc:
                raise OperatorError("invalid legacy registry reference") from exc
        imported, existing = 0, 0
        for record in records:
            destination = self.imports / (record["id"] + ".json")
            if destination.exists():
                existing += 1
                record = read_json(destination)
            else:
                _write(destination, record)
                imported += 1
            _encoded(self._import_row(record))
        return {"schema": "opi-eval-catalog-import/1", "imported": imported, "existing": existing}

    def _import_row(self, record):
        if record.get("schema") != "opi-eval-import/1":
            raise OperatorError("unknown catalogue import schema")
        identity = _identity(record.get("id"))
        root = _external_path(record.get("root", ""))
        _time(record.get("created_at"))
        kind = record.get("kind")
        binding = record.get("snapshot")
        family_names = record.get("families", [])
        if isinstance(family_names, list):
            family_names = [LEGACY_BENCHES.get(bench, bench) for bench in family_names if isinstance(bench, str)]
        if not isinstance(family_names, list) or any(bench not in BENCHES for bench in family_names):
            family_names = []
        families, profile = {}, record.get("profile", {})
        state = _pin_state(binding) if binding else "unverifiable"
        snapshot = {}
        if state == "verified":
            snapshot = read_json(binding["path"])
            if snapshot.get("schema") == "opi-eval-baseline-snapshot/1" and snapshot.get("bench") in BENCHES:
                family_names = [snapshot["bench"]]
            else:
                state = "unverifiable"
        if snapshot.get("effective") and _pin_state(snapshot["effective"]) == "verified":
            profile = _metadata(read_json(snapshot["effective"]["path"]))
        for bench in family_names:
            captured = record.get("metadata", {}).get(bench, {})
            families[bench] = {"root": root, "snapshot": binding, "effective": snapshot.get("effective"),
                               "status": snapshot.get("status", "unknown"), "complete": snapshot.get("complete", False),
                               "product": snapshot.get("product"), "version": snapshot.get("version"),
                               "completed_at": snapshot.get("completed_at"), "verification_state": state,
                               "semantic": snapshot.get("semantic"), "companion": snapshot.get("companion")}
            if state != "verified" and captured:
                families[bench].update({key: captured.get(key) for key in ("status", "complete", "product", "version", "completed_at", "semantic", "companion", "effective")})
        selection_state, pinned = "active", False
        for receipt in self._receipts(identity):
            if receipt["action"] in {"retire", "unretire"}:
                selection_state = "retired" if receipt["action"] == "retire" else "active"
            elif receipt["action"] in {"pin", "unpin"}:
                pinned = receipt["action"] == "pin"
        action = {"pi-baseline-snapshot": "baseline", "opi-candidate-snapshot": "candidate"}.get(kind, "import")
        completed = [family["completed_at"] for family in families.values() if family.get("completed_at")]
        status = next((family["status"] for family in families.values() if family.get("status") != "unknown"), record.get("status", "unknown"))
        known_baseline = action == "baseline" and bool(families) and all(family.get("product") == "pi" for family in families.values())
        native_root = Path(root)
        return {"id": identity, "root": root, "ownership": "imported", "action": action,
                "created_at": record["created_at"], "completed_at": max(completed) if completed else None,
                "status": status, "storage_state": "active" if native_root.is_absolute() and native_root.exists() else "missing",
                "selection_state": selection_state, "pinned": pinned, "references": [],
                "reference_unknown": [] if known_baseline else ["legacy-reference-contract-unknown"], "families": families,
                "record": {**record, "profile": profile}, "lifecycle": {},
                "active_lease": safe_path(self.leases / (identity + ".json"), within=self.leases).exists()}

    def snapshots(self, bench=None, pi_version=None, max_age_days=0):
        if bench is not None and bench not in BENCHES:
            raise OperatorError("unknown snapshot benchmark")
        if type(max_age_days) is not int or max_age_days < 0:
            raise OperatorError("invalid snapshot freshness bound")
        snapshots = []
        filters = {"selection_state": "active", "storage_state": "active", "action": "baseline", "product": "pi"}
        if bench is not None:
            filters["bench"] = bench
        if pi_version is not None:
            filters["version"] = pi_version
        for row in self._rows():
            if not self._matches(row, filters):
                continue
            for family_bench, family in row["families"].items():
                if bench is not None and family_bench != bench:
                    continue
                binding = family.get("snapshot")
                if _pin_state(binding) != "verified":
                    continue
                try:
                    value = read_json(binding["path"])
                    if (value.get("schema") != "opi-eval-baseline-snapshot/1" or value.get("product") != "pi"
                            or value.get("complete") is not True or value.get("status") != "complete"
                            or value.get("bench") != family_bench):
                        continue
                    if pi_version is not None and value.get("version") != pi_version:
                        continue
                    completed = _time(value.get("completed_at"))
                    if max_age_days and (datetime.now(timezone.utc) - completed).total_seconds() > max_age_days * 86400:
                        continue
                    value["self"] = binding
                    value["_evaluation_id"] = row["id"]
                    snapshots.append(value)
                except OperatorError:
                    continue
        return sorted(snapshots, key=lambda value: (_time(value["completed_at"]), value.get("id", "")), reverse=True)

    def list(self, **filters):
        limit = filters.pop("limit", 1000)
        offset = filters.pop("offset", 0)
        if type(limit) is not int or not 1 <= limit <= MAX_ROWS or type(offset) is not int or not 0 <= offset <= 2**63 - 1:
            raise OperatorError("invalid catalogue pagination")
        allowed = {"id", "root", "action", "status", "storage_state", "selection_state", "ownership", "pinned", "state", "bench", "product", "version", "model"}
        if set(filters) - allowed:
            raise OperatorError("unknown catalogue filter")
        for key, value in filters.items():
            if key == "pinned":
                if type(value) is not bool:
                    raise OperatorError("invalid pinned filter")
            elif not isinstance(value, str):
                raise OperatorError("catalogue filters must be text")
        if "id" in filters:
            row = self._lookup(_identity(filters["id"]))
            candidates = [] if row is None else [row]
        elif "root" in filters and Path(filters["root"]).is_absolute() and self.evaluations in Path(filters["root"]).parents:
            candidates = [self.refresh(filters["root"])]
        else:
            candidates = self._rows()
        rows = [row for row in candidates if self._matches(row, filters)]
        rows.sort(key=lambda row: (_time(row["created_at"]), row["id"]), reverse=True)
        return rows[offset:offset + limit]

    @staticmethod
    def _matches(row, filters):
        family_filters = {key: value for key, value in filters.items() if key in {"bench", "product", "version", "model"}}
        for key, value in filters.items():
            if key == "state":
                if value not in {row["status"], row["storage_state"], row["selection_state"]}:
                    return False
            elif key not in family_filters and row.get(key) != value:
                return False
        model = row.get("record", {}).get("profile", {}).get("model", {}).get("model")
        return not family_filters or any(all(
            (bench if key == "bench" else family.get("model", model) if key == "model" else family.get(key)) == value
            for key, value in family_filters.items()) for bench, family in row["families"].items())

    def _lookup(self, identity):
        """Resolve one immutable identity without reading unrelated evaluation records."""
        imported = safe_path(self.imports / (identity + ".json"), within=self.imports)
        if imported.is_file():
            row = self._import_row(read_json(imported))
            if row["id"] != identity:
                raise OperatorError("invalid imported evaluation identity binding")
            _encoded(row)
            return row
        registration = self._registration(identity)
        if registration is not None:
            return self.refresh(registration["record"]["root"])
        # A pre-registration native record can still be queried at its dated path.
        paths = self._paths(self.evaluations, "*/*/" + identity + "/record.json", 2)
        if len(paths) > 1:
            raise OperatorError("duplicate evaluation identity")
        return self.refresh(paths[0].parent) if paths else None

    def show(self, identity):
        _identity(identity)
        row = self._lookup(identity)
        if row is None:
            raise OperatorError("evaluation not found")
        return row

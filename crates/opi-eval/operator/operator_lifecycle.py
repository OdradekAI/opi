"""Inventory-bound, explicit maintenance of operator-owned evaluation evidence."""
from __future__ import annotations

from contextlib import ExitStack
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path, PurePosixPath, PureWindowsPath
import stat
import tarfile
import uuid

from operator_process import OperatorError


class LifecycleError(OperatorError):
    """A fail-closed maintenance refusal without evidence or credential contents."""
    def __init__(self, code, message):
        self.code = code
        super().__init__(f"{code}: {message}")


MAX_DOCUMENT_BYTES = 8 * 1024 * 1024
MAX_INVENTORY_ENTRIES = 100000


def _digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(",", ":"),
                                    ensure_ascii=False, allow_nan=False).encode()).hexdigest()


def _sha(path):
    result = hashlib.sha256()
    with Path(path).open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            result.update(block)
    return result.hexdigest()


def _read(path):
    try:
        if Path(path).stat().st_size > MAX_DOCUMENT_BYTES:
            raise ValueError()
        with Path(path).open(encoding="utf-8") as stream:
            value = json.load(stream)
        if not isinstance(value, dict):
            raise ValueError()
        return value
    except (OSError, ValueError, TypeError, RecursionError) as exc:
        raise LifecycleError("unreadable", "machine evidence is unreadable") from exc


def _write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("x", encoding="utf-8") as stream:
        json.dump(value, stream, sort_keys=True, indent=2, ensure_ascii=False, allow_nan=False)
        stream.write("\n")
        stream.flush()
        os.fsync(stream.fileno())
    _sync_directory(path.parent)


def _sync_directory(directory):
    if hasattr(os, "O_DIRECTORY"):
        descriptor = os.open(directory, os.O_RDONLY | os.O_DIRECTORY)
        try:
            os.fsync(descriptor)
        finally:
            os.close(descriptor)


def _sync_archive(path):
    try:
        with path.open("r+b") as stream:
            stream.flush()
            os.fsync(stream.fileno())
        _sync_directory(path.parent)
    except OSError as exc:
        raise LifecycleError("archive-durability", "archive bytes could not be made durable") from exc


def _inside(path, parent):
    return path == parent or parent in path.parents


def _absolute(value):
    return Path(value).is_absolute() or PureWindowsPath(value).is_absolute()


def _owned_root(store, row):
    if row.get("ownership") != "managed":
        raise LifecycleError("unmanaged", "imported evidence is never maintained")
    root = Path(row.get("root", ""))
    evaluations = Path(store.evaluations).resolve()
    if not root.is_absolute() or root.is_symlink() or root.resolve() != root:
        raise LifecycleError("ownership", "evaluation root is not canonical")
    try:
        parts = root.relative_to(evaluations).parts
    except ValueError as exc:
        raise LifecycleError("ownership", "evaluation root is outside owned storage") from exc
    if (len(parts) != 3 or len(parts[0]) != 4 or not parts[0].isdigit()
            or len(parts[1]) != 2 or not parts[1].isdigit()
            or not 1 <= int(parts[1]) <= 12 or parts[2] != row.get("id")):
        raise LifecycleError("ownership", "evaluation root is not a dated managed record")
    record = _read(root / "record.json")
    if record.get("id") != row["id"] or record.get("root") != str(root):
        raise LifecycleError("ownership", "immutable record does not own this root")
    return root


def _inventory(root, require_record=True, allow_links=False):
    entries, paths = [], []
    def unreadable(error):
        raise LifecycleError("inventory-unreadable", "evidence subtree cannot be inventoried") from error
    for directory, directories, files in os.walk(root, followlinks=False, onerror=unreadable):
        for name in list(directories) + files:
            path = Path(directory) / name
            if path.is_symlink() or (hasattr(path, "is_junction") and path.is_junction()):
                if not allow_links:
                    raise LifecycleError("link", "linked evidence cannot be archived")
                if name in directories:
                    directories.remove(name)
            paths.append(path)
            if len(paths) > MAX_INVENTORY_ENTRIES:
                raise LifecycleError("inventory-bound", "evidence inventory exceeds entry bound")
    for path in sorted(paths, key=lambda path: path.relative_to(root).as_posix()):
        info = path.lstat()
        is_junction = hasattr(path, "is_junction") and path.is_junction()
        if path.is_symlink() or is_junction:
            if not allow_links or not _inside(path.parent.resolve(), root):
                raise LifecycleError("link", "linked evidence cannot be archived")
            entry = {"path": path.relative_to(root).as_posix(), "kind": "junction" if is_junction else "symlink",
                     "bytes": 0, "target": os.readlink(path)}
        elif not _inside(path.resolve(), root):
            raise LifecycleError("ownership", "evidence path escapes the exact root")
        elif stat.S_ISDIR(info.st_mode):
            entry = {"path": path.relative_to(root).as_posix(), "kind": "directory", "bytes": 0}
        elif stat.S_ISREG(info.st_mode):
            if info.st_nlink > 1 and not allow_links:
                # Purge (deletion) may unlink staged hardlinks such as the
                # runner venv's uv-installed packages; archive still refuses
                # them because an archive bundle must own self-contained
                # bytes.
                raise LifecycleError("link", "hardlinked evidence cannot be maintained")
            entry = {"path": path.relative_to(root).as_posix(), "kind": "file",
                     "bytes": info.st_size, "sha256": _sha(path)}
        else:
            raise LifecycleError("special-file", "special evidence files cannot be maintained")
        entry["mode"] = stat.S_IMODE(info.st_mode)
        entries.append(entry)
    if require_record and not any(entry["path"] == "record.json" and entry["kind"] == "file" for entry in entries):
        raise LifecycleError("ownership", "immutable record is missing")
    return entries


def _policy(value):
    if value is None:
        from operator_config import retention_policy
        value = retention_policy({})
    if (not isinstance(value, dict) or set(value) != {"keep_latest_per_group"}
            or type(value["keep_latest_per_group"]) is not int or value["keep_latest_per_group"] < 1):
        raise LifecycleError("policy", "retention requires one positive keep_latest_per_group count")
    return dict(value)


def _time(value):
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
        if parsed.tzinfo is None:
            raise ValueError()
        return parsed.astimezone(timezone.utc)
    except (TypeError, ValueError, AttributeError) as exc:
        raise LifecycleError("completion", "completion time is unknown") from exc


def _documents(root, inventory):
    # Reference scanning covers operator-owned evidence documents. Staged
    # upstream bytes under a family's cache/ (runner venvs, registries),
    # builds/ (the repository working-tree snapshot, including test
    # fixtures with deliberately malformed JSON) and material/packages/
    # (acquired task packages) are not operator evidence: oversized or
    # intentionally corrupt upstream documents must not make the whole
    # evaluation permanently unreadable. Operator documents directly under
    # material/ stay scanned, and scanned documents may be objects or
    # arrays (e.g. collected-artifacts manifests).
    def staged_upstream(relative):
        parts = relative.split("/")
        return (len(parts) > 2 and parts[0] == "families" and parts[2] in {"cache", "builds"}) or (
            len(parts) > 3 and parts[0] == "families" and parts[2] == "material" and parts[3] == "packages"
        )
    def scan(path):
        try:
            if path.stat().st_size > MAX_DOCUMENT_BYTES:
                raise ValueError()
            with path.open(encoding="utf-8") as stream:
                value = json.load(stream)
        except (OSError, ValueError, TypeError, RecursionError) as exc:
            raise LifecycleError("unreadable", "machine evidence is unreadable") from exc
        if not isinstance(value, (dict, list)):
            raise LifecycleError("unreadable", "machine evidence is unreadable")
        return value
    return [(PurePosixPath(entry["path"]), scan(root / entry["path"])) for entry in inventory
            if entry["kind"] == "file" and entry["path"].endswith(".json")
            and not staged_upstream(entry["path"])]


def _walk(value):
    if isinstance(value, dict):
        for key, child in value.items():
            yield key, child
            yield from _walk(child)
    elif isinstance(value, list):
        for child in value:
            yield "item", child
            yield from _walk(child)


def _pins(value):
    if isinstance(value, dict):
        if "path" in value and "sha256" in value:
            yield value
        for child in value.values():
            yield from _pins(child)
    elif isinstance(value, list):
        for child in value:
            yield from _pins(child)


def _provenance_document(relative, document):
    """Exclude exact execution-authority pointers, retaining all material bindings."""
    if isinstance(document, list):
        # Array documents (e.g. collected-artifacts manifests) carry no
        # operator execution authority to exclude; their pins still scan.
        return document
    value = dict(document)
    parts, schema = relative.parts, document.get("schema")
    family = len(parts) >= 3 and parts[0] == "families"
    if parts == ("retention-handoff.json",) and schema == "opi-eval-retention-result/1":
        return {"schema": schema}
    if family and parts[2:] == ("inputs", "effective.json") and schema in {"opi-eval-operator/1", "opi-eval-operator/2"}:
        value["storage"] = {key: child for key, child in document.get("storage", {}).items() if key not in {"workspace", "cache"}}
        credential = document.get("credential", {})
        if credential.get("kind") == "credential_file":
            value["credential"] = {key: child for key, child in credential.items() if key != "reference"}
    if (parts == ("handoff.json",) or (family and parts[2:] == ("handoff.json",))) and schema == "opi-eval-handoff/1":
        value.pop("execute_command", None)
    if family and parts[2:] == ("prepared.json",) and schema == "opi-eval-prepared-operation/1":
        value.pop("profile_path", None)
    if (family and len(parts) == 4 and parts[2] == "commands" and isinstance(document.get("argv"), list)
            and isinstance(document.get("cwd"), str) and document.get("started_at")):
        value.pop("argv", None)
        value.pop("cwd", None)
    return value


def _relative_provenance(relative, document):
    parts = relative.parts
    if len(parts) < 4 or parts[0] != "families":
        return False
    if (parts[2:] == ("cache", "pins", "static", "linux-x86_64.json")
            and document.get("schema") == "opi-eval-external-lock/static/1"):
        return True
    return (len(parts) == 4 and parts[2] == "builds" and relative.name in {"source-before.json", "source-after.json"}
            and isinstance(document.get("files"), list) and "manifest_sha256" in document)


def _group_preparation_failed(row, root, inventory):
    """A frozen request proof permits cleanup of siblings never dispatched live."""
    binding = row.get("preparation_failure")
    path = root / "preparation-failure.json"
    if (row.get("status") != "preparation_failed" or not isinstance(binding, dict)
            or binding.get("path") != str(path) or not path.is_file() or _sha(path) != binding.get("sha256")):
        return False
    if any(entry["path"].endswith(("execution-authorization.json", "live-authorization.json"))
           or "runs" in PurePosixPath(entry["path"]).parts for entry in inventory):
        return False
    failure = _read(path)
    declared, families = failure.get("families"), row.get("families")
    if (failure.get("schema") != "opi-eval-preparation-failure/1" or failure.get("evaluation") != str(root)
            or failure.get("status") != "preparation_failed" or failure.get("live_dispatch") is not False
            or not isinstance(declared, dict) or set(declared) != set(families)):
        return False
    _time(failure.get("completed_at"))
    for bench, proof in declared.items():
        family = root / "families" / bench
        if (not isinstance(proof, dict) or proof.get("root") != str(family) or proof.get("live_dispatch") is not False
                or proof.get("status") not in {"prepared", "preparation_failed", "not_started"}):
            return False
        if proof["status"] == "prepared" and not (family / "prepared.json").is_file():
            return False
        if proof["status"] == "not_started" and any((family / filename).exists()
                for filename in ("prepared.json", "preparation-failure.json", "inputs/comparison-input.json")):
            return False
    return True


def _facts(row, root, inventory):
    """Derive semantic groups and portability from original pinned evidence."""
    groups, products, reasons = [], [], []
    retained_references = {Path(path).resolve() for path in row.get("references", []) if isinstance(path, str) and _absolute(path)}
    families = row.get("families")
    if not isinstance(families, dict) or not families:
        return [], [], ["identity_unknown"]
    group_failure = _group_preparation_failed(row, root, inventory)
    derivative_outputs = set()
    if row.get("action") == "compare":
        for bench in families:
            handoff = _read(root / "families" / bench / "handoff.json")
            stopped = handoff.get("status") == "stopped"
            closed = (handoff.get("status") == "complete" or
                      (stopped and handoff.get("live_dispatch") is False and handoff.get("cleanup") == "confirmed"))
            execution = any(entry["path"].endswith(("execution-authorization.json", "live-authorization.json"))
                            or "runs" in PurePosixPath(entry["path"]).parts for entry in inventory)
            comparison_input = _read(root / "families" / bench / "inputs" / "comparison-input.json")
            inputs = [comparison_input.get("candidate"), comparison_input.get("baseline")]
            bound = all(isinstance(binding, dict) and isinstance(binding.get("path"), str)
                        and _absolute(binding["path"]) and isinstance(binding.get("sha256"), str)
                        and len(binding["sha256"]) == 64 and set(binding["sha256"]) <= set("0123456789abcdef")
                        for binding in inputs if binding is not None)
            if (handoff.get("schema") != "opi-eval-handoff/1" or not closed or (stopped and execution)
                    or comparison_input.get("schema") != "opi-eval-comparison-input/1"
                    or not bound or not isinstance(comparison_input.get("candidate"), dict) or "baseline" not in comparison_input
                    or (comparison_input["baseline"] is not None and not isinstance(comparison_input["baseline"], dict))):
                reasons.append("comparison_unverifiable")
            for key in ("comparison", "comparison_manifest"):
                binding = handoff.get(key)
                if isinstance(binding, dict) and isinstance(binding.get("path"), str) and _absolute(binding["path"]):
                    derivative_outputs.add(Path(binding["path"]).resolve())
        products = ["comparison"]
    for bench, family in sorted(families.items()):
        if row.get("action") == "compare":
            continue
        pinned = family.get("snapshot")
        if not isinstance(pinned, dict) or not pinned.get("path") or not pinned.get("sha256"):
            prepared_failure = root / "families" / bench / "preparation-failure.json"
            if (group_failure or (row.get("status") == "preparation_failed" and prepared_failure.is_file()
                    and _read(prepared_failure).get("live_dispatch") is False
                    and not any(entry["path"].endswith("execution-authorization.json") for entry in inventory))):
                products.append(family.get("product", "pi" if row.get("action") == "baseline" else "opi"))
            else:
                reasons.append("snapshot_unknown")
            continue
        path = Path(pinned["path"])
        if not _inside(path.resolve(), root) or not path.is_file() or _sha(path) != pinned["sha256"]:
            reasons.append("snapshot_unverifiable")
            continue
        snapshot = _read(path)
        semantic, companion = snapshot.get("semantic"), snapshot.get("companion", {}).get("sha256")
        product, version = snapshot.get("product"), snapshot.get("version")
        if (not isinstance(semantic, dict) or not semantic.get("common") or not semantic.get("tasks")
                or not isinstance(companion, str) or len(companion) != 64
                or product not in {"opi", "pi"} or not version):
            reasons.append("identity_unknown")
            continue
        if semantic["common"].get("admitted") is not True or any(
                task.get("admitted") is not True for task in semantic["tasks"].values()):
            reasons.append("identity_unadmitted")
        groups.append(_digest({"bench": bench, "semantic": semantic,
                              "companion": companion, "product": product, "version": version}))
        products.append(product)
        if snapshot.get("status") not in {"complete", "incomplete", "stopped"}:
            reasons.append("completion_unknown")
        if snapshot.get("status") == "complete" and snapshot.get("complete") is not True:
            reasons.append("completion_unknown")
        if any(attempt.get("verification") != "verified" for attempt in snapshot.get("attempts", [])):
            reasons.append("cleanup_unknown")
        if any(slot.get("unverified_attempts") for slot in snapshot.get("slots", [])):
            reasons.append("unresolved_failure")
    for relative, original in _documents(root, inventory):
        document = _provenance_document(relative, original)
        for pinned in _pins(document):
            try:
                if not isinstance(pinned["path"], str):
                    raise ValueError()
                if ".." in PurePosixPath(pinned["path"].replace("\\", "/")).parts:
                    raise ValueError()
                if not _absolute(pinned["path"]) and _relative_provenance(relative, original):
                    continue
                path = Path(pinned["path"])
                if not path.is_absolute():
                    path = root.joinpath(*relative.parent.parts) / path
                if path.resolve() in derivative_outputs:
                    continue
                if ((not _inside(path.resolve(), root) and path.resolve() not in retained_references)
                        or not path.is_file() or _sha(path) != pinned["sha256"]):
                    reasons.append("dependency_unverifiable")
            except (TypeError, ValueError, OSError):
                reasons.append("dependency_unverifiable")
        for key, value in _walk(document):
            if isinstance(value, str) and _absolute(value):
                # Recorded environment variables (UPPERCASE_SNAKE keys) and
                # native run trees' absolute strings (container-internal
                # paths like /app by construction) are not host references;
                # structured pins remain fully protected above.
                environment_key = isinstance(key, str) and key.isupper() and key.replace("_", "").isalnum()
                in_runs = "runs" in relative.parts
                if (not environment_key and not in_runs
                        and not _inside(Path(value).resolve(), root) and Path(value).resolve() not in retained_references
                        and Path(value).resolve() not in derivative_outputs):
                    reasons.append("unportable_dependency")
            if key in {"cleanup_state", "cleanup_status"} and value not in {"confirmed", "complete"}:
                reasons.append("cleanup_unknown")
            if key == "stream_cleanup" and value != "closed":
                reasons.append("cleanup_unknown")
            if key == "cleanup" and isinstance(value, str) and "unknown" in value:
                reasons.append("cleanup_unknown")
            if key in {"hold", "held", "unresolved"} and value:
                reasons.append("unresolved_hold")
            if key in {"reason", "gap", "stopped_reason"} and isinstance(value, str) and any(
                    token in value.lower() for token in ("unconfirmed", "effect_unknown", "credential-material-removal-failed", "authority-boundary")):
                reasons.append("cleanup_unknown")
    return groups, products, sorted(set(reasons))


def _bucket(row):
    """Retention kind follows operator controls, independent of build/source identity."""
    record = row.get("record")
    if not isinstance(record, dict):
        record = _read(Path(row["root"]) / "record.json")
    profile, families = record.get("profile"), row.get("families")
    if not isinstance(profile, dict) or not isinstance(families, dict) or not families:
        raise LifecycleError("retention_identity_unknown", "retention controls are unknown")
    model, benchmarks = profile.get("model"), profile.get("benchmarks")
    if not isinstance(model, dict) or not isinstance(benchmarks, dict):
        raise LifecycleError("retention_identity_unknown", "retention controls are unknown")
    required = {"provider", "model", "api", "controls", "thinking"}
    if not required <= set(model) or not isinstance(model["controls"], dict) or not isinstance(model["thinking"], dict):
        raise LifecycleError("retention_identity_unknown", "model controls are unknown")
    if (not {"context_window", "max_output_tokens", "temperature", "top_p"} <= set(model["controls"])
            or not {"level", "required"} <= set(model["thinking"])):
        raise LifecycleError("retention_identity_unknown", "model controls are incomplete")
    selections = {}
    for bench in sorted(families):
        selection = benchmarks.get(bench)
        if not isinstance(selection, dict) or not {"scope", "task_ids", "repetitions", "seeds"} <= set(selection):
            raise LifecycleError("retention_identity_unknown", "task selection is unknown")
        selections[bench] = {key: selection[key] for key in ("scope", "task_ids", "repetitions", "seeds")}
    agent = {"baseline": "pi", "candidate": "opi", "compare": "comparison", "acceptance": "acceptance"}.get(row.get("action"))
    if agent is None:
        raise LifecycleError("retention_identity_unknown", "evaluation kind is unknown")
    identity = {"agent": agent, "families": sorted(families), "model": {key: model[key] for key in sorted(required)},
                "selection": selections}
    if agent == "pi":
        versions = {family.get("version") or profile.get("baseline", {}).get("pi_version") for family in families.values()}
        if len(versions) != 1 or None in versions:
            raise LifecycleError("retention_identity_unknown", "pi release is unknown")
        identity["pi_version"] = versions.pop()
    return _digest(identity)


def _rows(store):
    rows, offset = [], 0
    while True:
        batch = store.list(limit=10000, offset=offset)
        rows.extend(batch)
        if len(rows) > MAX_INVENTORY_ENTRIES:
            raise LifecycleError("catalogue-bound", "maintenance catalogue exceeds row bound")
        if len(batch) < 10000:
            break
        offset += len(batch)
    current = []
    for row in rows:
        if row.get("ownership") == "managed":
            try:
                row = store.refresh(row["root"])
            except (OperatorError, OSError, ValueError, TypeError):
                row = {**row, "reference_unknown": ["current-record-unverifiable"]}
        current.append(row)
    return sorted(current, key=lambda row: row["id"])


def _archived_facts(store, row):
    fields = row.get("lifecycle", {})
    inventory = fields.get("inventory")
    if not isinstance(inventory, list) or _digest(inventory) != fields.get("inventory_digest"):
        raise LifecycleError("archive-inventory", "archived inventory binding differs")
    archive = Path(fields.get("archive_path", ""))
    base = _archive_base(store, row["id"])
    if (not archive.is_file() or archive.is_symlink() or not _inside(archive.resolve(), base)
            or _sha(archive) != fields.get("archive_sha256") or archive.stat().st_size != fields.get("archive_bytes")):
        raise LifecycleError("archive-digest", "archived bytes differ")
    root = _owned_root(store, row)
    if _inventory(root) != [entry for entry in inventory if entry["path"] == "record.json"]:
        raise LifecycleError("cleanup_unknown", "archived hot evidence cleanup is incomplete")
    groups, products = [], []
    for bench, family in sorted(row.get("families", {}).items()):
        semantic, companion = family.get("semantic"), family.get("companion", {}).get("sha256")
        product, version = family.get("product"), family.get("version")
        if not semantic or not companion or product not in {"opi", "pi"} or not version:
            raise LifecycleError("identity_unknown", "archived semantic identity is unknown")
        groups.append(_digest({"bench": bench, "semantic": semantic,
                              "companion": companion, "product": product, "version": version}))
        products.append(product)
    if not groups:
        raise LifecycleError("identity_unknown", "archived semantic identity is unknown")
    binding = {"archive_path": str(archive), "archive_sha256": fields["archive_sha256"],
               "archive_bytes": fields["archive_bytes"]}
    return inventory, groups, products, binding


def _selected_binding(entry):
    """The reviewed plan's digest-bound selection shape; application
    re-derives the same shape from current evidence before comparing."""
    return {"id": entry["id"], "inventory_digest": entry["inventory_digest"],
            "bytes": entry["bytes"], "reclaimable_bytes": entry["reclaimable_bytes"]}


def _plan(store, policy, action, ids=None, ignore_leases=(), retain_ids=(), detail=False):
    if action not in {"archive", "purge"}:
        raise LifecycleError("action", "maintenance action must be archive or purge")
    policy = _policy(policy)
    rows = _rows(store)
    if ids is not None and (not isinstance(ids, (list, tuple)) or any(not isinstance(identity, str) for identity in ids)):
        raise LifecycleError("identity", "requested identities must be a list of strings")
    requested = sorted(set(ids)) if ids is not None else None
    if requested is not None and (any(not isinstance(identity, str) for identity in requested)
                                  or set(requested) - {row["id"] for row in rows}):
        raise LifecycleError("identity", "requested evaluation identity is unknown")
    live_rows = [row for row in rows if row.get("storage_state") != "purged"
                 or row.get("lifecycle", {}).get("cleanup_state") != "confirmed"]
    unknown_references = any(not isinstance(row.get("references"), list)
                             or not isinstance(row.get("reference_unknown"), list)
                             or row["reference_unknown"] for row in live_rows)
    referenced = set()
    for row in live_rows:
        references = row.get("references")
        for target in references if isinstance(references, list) else []:
            if not isinstance(target, str) or not _absolute(target):
                unknown_references = True
                continue
            target_path = Path(target).resolve()
            referenced.update(other["id"] for other in rows
                              if _inside(target_path, Path(other["root"]).resolve()))
    inventory = []
    for row in rows:
        reasons, entries, groups, products, archive_binding = [], [], [], [], None
        if unknown_references:
            reasons.append("reference_unknown")
        if row["id"] in referenced:
            reasons.append("reference_target")
        if row.get("pinned"):
            reasons.append("pinned")
        if type(row.get("pinned")) is not bool:
            reasons.append("pin_unknown")
        if row.get("storage_state") != "active" and not (row.get("storage_state") == "archived" and action == "purge"):
            reasons.append("storage_" + str(row.get("storage_state", "unknown")))
        if row.get("status") not in {"complete", "incomplete", "stopped", "preparation_failed"}:
            reasons.append("active_or_unfinished")
        if row["id"] in retain_ids:
            reasons.append("current_output")
        if row["id"] not in ignore_leases and (Path(store.leases) / (row["id"] + ".json")).exists():
            reasons.append("active_lease")
        try:
            root = _owned_root(store, row)
            if row.get("storage_state") == "active":
                entries = _inventory(root, allow_links=action == "purge")
                groups, products, gaps = _facts(row, root, entries)
                reasons.extend(gaps)
            elif row.get("storage_state") == "archived" and action == "purge":
                entries, groups, products, archive_binding = _archived_facts(store, row)
            groups = [_bucket(row)]
        except LifecycleError as exc:
            reasons.append(exc.code)
        except (OperatorError, OSError, ValueError, TypeError, AttributeError, KeyError, RecursionError):
            reasons.append("evidence_unverifiable")
        completed_utc, created_utc = None, None
        if row.get("completed_at") is not None:
            try:
                completed_utc = _time(row["completed_at"]).isoformat()
            except LifecycleError:
                reasons.append("completion_unknown")
        try:
            created = row.get("created_at")
            if created is None:
                created = row.get("record", {}).get("created_at")
            created_utc = _time(created).isoformat()
        except LifecycleError:
            reasons.append("creation_unknown")
        reclaimable = (archive_binding["archive_bytes"] if archive_binding else
                       sum(entry["bytes"] for entry in entries if entry["path"] != "record.json"))
        inventory.append({"id": row["id"], "root": row["root"], "ownership": row.get("ownership"),
                          "completed_at": row.get("completed_at"), "status": row.get("status"),
                          "completed_utc": completed_utc,
                          "created_utc": created_utc,
                          "storage_state": row.get("storage_state"), "selection_state": row.get("selection_state"),
                          "groups": groups, "products": products, "inventory": entries,
                          "inventory_digest": _digest(entries), "bytes": sum(entry["bytes"] for entry in entries),
                          "reclaimable_bytes": reclaimable, "archive": archive_binding, "protected": reasons})
    group_members = {}
    for entry in inventory:
        if entry["ownership"] != "managed" or entry["storage_state"] not in {"active", "archived"} or entry["status"] not in {"complete", "incomplete", "stopped", "preparation_failed"}:
            continue
        for group in entry["groups"]:
            group_members.setdefault(group, []).append(entry)
    for members in group_members.values():
        members.sort(key=lambda entry: (entry["completed_utc"] or entry["created_utc"] or "", entry["created_utc"] or "", entry["id"]), reverse=True)
        for number, entry in enumerate(members):
            entry["retention_rank"] = number + 1
            entry["excess"] = number >= policy["keep_latest_per_group"]
            if not entry["excess"] and (action == "purge" or "pi" in entry["products"]):
                entry["protected"].append("keep_latest_per_group")
    for entry in inventory:
        entry["protected"] = sorted(set(entry["protected"]))
    selected = [entry for entry in inventory if not entry["protected"] and (requested is None or entry["id"] in requested)]
    measured = sum(entry["bytes"] for entry in inventory if entry["storage_state"] == "active")
    # The reviewed plan binds selections by inventory digest, not by embedded
    # per-file inventories: real material-bearing trees carry tens of
    # thousands of files each, and the application re-derives and compares
    # the same digest-bound shape before anything is removed. Internal
    # detail=True keeps full entries for the executing path.
    result = {"schema": "opi-eval-maintenance-plan/1", "workspace": str(Path(store.workspace).resolve()),
              "action": action, "policy": policy, "requested_ids": requested,
              "inventory": [{key: value for key, value in entry.items() if key != "inventory"} for entry in inventory],
              "selected": selected if detail else [_selected_binding(entry) for entry in selected],
              "selected_bytes": sum(entry["bytes"] for entry in selected),
              "reclaimable_bytes": sum(entry["reclaimable_bytes"] for entry in selected),
              "hot_bytes": measured}
    result["plan_digest"] = _digest(result)
    return result


def maintenance_plan(store, policy, action, ids=None):
    """Return a deterministic preview; never archive, purge, or edit native evidence."""
    return _plan(store, policy, action, ids)


def _archive_base(store, identity):
    base = Path(store.workspace).resolve() / "archives" / identity
    if base.is_symlink() or not _inside(base.resolve(), Path(store.workspace).resolve() / "archives"):
        raise LifecycleError("ownership", "archive storage escapes the workspace")
    return base


def _safe_member(name):
    path = PurePosixPath(name)
    if (not name or "\\" in name or path.is_absolute() or PureWindowsPath(name).drive
            or any(part in {".", ".."} for part in path.parts) or path.as_posix() != name):
        raise LifecycleError("archive-path", "archive member path is unsafe")
    return path


def _extract_verified(archive, destination, inventory):
    expected = {entry["path"]: entry for entry in inventory}
    seen, directory_modes = set(), []
    try:
        with tarfile.open(archive, "r:gz") as stream:
            for member in stream:
                relative = _safe_member(member.name)
                entry = expected.get(member.name)
                if member.name in seen or entry is None:
                    raise LifecycleError("archive-inventory", "archive member is unreviewed or duplicated")
                seen.add(member.name)
                target = destination.joinpath(*relative.parts)
                if not _inside(target.resolve(), destination.resolve()) or member.issym() or member.islnk():
                    raise LifecycleError("archive-link", "archive links are forbidden")
                if entry["kind"] == "directory" and member.isdir():
                    target.mkdir(parents=True, exist_ok=True)
                    directory_modes.append((target, entry["mode"]))
                elif entry["kind"] == "file" and member.isfile() and member.size == entry["bytes"]:
                    target.parent.mkdir(parents=True, exist_ok=True)
                    source = stream.extractfile(member)
                    if source is None:
                        raise LifecycleError("archive-inventory", "archive member has no bytes")
                    with target.open("xb") as output:
                        for block in iter(lambda: source.read(1024 * 1024), b""):
                            output.write(block)
                    if _sha(target) != entry["sha256"]:
                        raise LifecycleError("archive-digest", "archived evidence bytes differ")
                else:
                    raise LifecycleError("archive-type", "archive member kind or size differs")
                if entry["kind"] == "file":
                    os.chmod(target, entry["mode"])
    except (OSError, tarfile.TarError) as exc:
        raise LifecycleError("archive-unreadable", "archive cannot be verified") from exc
    if seen != set(expected):
        raise LifecycleError("archive-inventory", "archive evidence is incomplete")
    for target, mode in reversed(directory_modes):
        os.chmod(target, mode)
    if _inventory(destination) != inventory:
        raise LifecycleError("archive-digest", "restored evidence inventory differs")


def _remove_exact(root, inventory, keep_record=True, require_record=True, allow_links=None):
    """Delete only validated descendants, with no recursive or cross-shell deletion."""
    if allow_links is None:
        allow_links = any(entry["kind"] in {"symlink", "junction"} for entry in inventory)
    if _inventory(root, require_record=require_record, allow_links=allow_links) != inventory:
        raise LifecycleError("inventory-drift", "evidence changed before removal")
    # Sealed directory modes are archived/restored verbatim. Owner write/execute
    # is needed only while deleting these exact reviewed descendants.
    for entry in inventory:
        if entry["kind"] == "directory":
            target = root / entry["path"]
            if target.is_symlink() or not _inside(target.resolve(), root):
                raise LifecycleError("ownership", "removal directory escapes the exact root")
            os.chmod(target, entry["mode"] | stat.S_IWUSR | stat.S_IXUSR)
    for entry in sorted(inventory, key=lambda entry: (entry["path"].count("/"), entry["path"]), reverse=True):
        if keep_record and entry["path"] == "record.json":
            continue
        target = root / entry["path"]
        if entry["kind"] in {"symlink", "junction"}:
            if not _inside(target.parent.resolve(), root) or os.readlink(target) != entry["target"]:
                raise LifecycleError("ownership", "owned link changed before unlinking")
            target.rmdir() if entry["kind"] == "junction" else target.unlink()
            continue
        if target.is_symlink() or not _inside(target.resolve(), root) or target == root:
            raise LifecycleError("ownership", "removal target escapes the exact evaluation root")
        if entry["kind"] == "file":
            if os.name == "nt" and not entry["mode"] & stat.S_IWRITE:
                os.chmod(target, entry["mode"] | stat.S_IWRITE)
            target.unlink()
        else:
            target.rmdir()


def _discard_stage(stage):
    if stage.exists():
        _remove_exact(stage, _inventory(stage, require_record=False), keep_record=False, require_record=False)
        stage.rmdir()


def _archive(store, row, entry):
    root = _owned_root(store, row)
    base = _archive_base(store, row["id"])
    base.mkdir(parents=True, exist_ok=True)
    archive = base / (entry["inventory_digest"] + ".tar.gz")
    manifest = base / (entry["inventory_digest"] + ".json")
    token = uuid.uuid4().hex
    temporary, stage = base / (".archive-" + token + ".tmp"), base / (".verify-" + token)
    stage.mkdir()
    try:
        if archive.exists() or manifest.exists():
            if archive.is_symlink() or manifest.is_symlink() or not archive.is_file() or not manifest.is_file():
                raise LifecycleError("archive-exists", "archive destination is incomplete or linked")
            previous = _read(manifest)
            if (previous.get("schema") != "opi-eval-archive/1" or previous.get("id") != row["id"]
                    or previous.get("root") != str(root) or previous.get("archive_path") != str(archive)
                    or previous.get("manifest_path") != str(manifest)
                    or previous.get("inventory") != entry["inventory"]
                    or previous.get("inventory_digest") != entry["inventory_digest"]
                    or previous.get("archive_bytes") != archive.stat().st_size
                    or previous.get("archive_sha256") != _sha(archive)):
                raise LifecycleError("archive-exists", "existing archive identity differs")
            _extract_verified(archive, stage, entry["inventory"])
            fields = {key: previous[key] for key in ("archive_path", "archive_sha256", "archive_bytes",
                      "inventory_digest", "inventory", "root", "manifest_path", "bytes")}
            return _finish_archive(store, row, entry, root, fields)
        with tarfile.open(temporary, "w:gz", format=tarfile.PAX_FORMAT) as stream:
            for item in entry["inventory"]:
                target = root / item["path"]
                member = tarfile.TarInfo(item["path"])
                member.mode = item["mode"]
                if item["kind"] == "directory":
                    member.type = tarfile.DIRTYPE
                    stream.addfile(member)
                else:
                    member.size = item["bytes"]
                    with target.open("rb") as source:
                        stream.addfile(member, source)
        _sync_archive(temporary)
        _extract_verified(temporary, stage, entry["inventory"])
        if _inventory(root) != entry["inventory"]:
            raise LifecycleError("inventory-drift", "evidence changed while archiving")
        temporary.rename(archive)
        _sync_directory(base)
        fields = {"archive_path": str(archive), "archive_sha256": _sha(archive),
                  "archive_bytes": archive.stat().st_size,
                  "inventory_digest": entry["inventory_digest"], "inventory": entry["inventory"],
                  "root": str(root), "manifest_path": str(manifest), "bytes": entry["bytes"]}
        _write(manifest, {"schema": "opi-eval-archive/1", "id": row["id"], **fields})
        return _finish_archive(store, row, entry, root, fields)
    finally:
        if temporary.exists():
            temporary.unlink()
        _discard_stage(stage)


def _finish_archive(store, row, entry, root, fields):
    if _inventory(root) != entry["inventory"]:
        raise LifecycleError("inventory-drift", "evidence changed before archive receipt")
    # Durable receipt precedes removal so a crash retains restore authority.
    store.event(row["id"], "archive", **fields, cleanup_state="pending")
    _remove_exact(root, entry["inventory"])
    store.event(row["id"], "archive", **fields, cleanup_state="confirmed")
    return {"id": row["id"], "inventory_digest": entry["inventory_digest"], "bytes": entry["bytes"]}


def apply_plan(store, plan_path):
    """Apply only an exact external reviewed plan after current safety checks."""
    path = Path(plan_path)
    if not path.is_file() or path.is_symlink():
        raise LifecycleError("plan", "reviewed plan must be an existing regular file")
    path = path.resolve()
    if _inside(path, Path(store.evaluations).resolve()) or _inside(path, (Path(store.workspace) / "archives").resolve()):
        raise LifecycleError("plan", "reviewed plan must be outside evidence and archives")
    plan = _read(path)
    digest = plan.get("plan_digest")
    if plan.get("schema") != "opi-eval-maintenance-plan/1" or digest != _digest({key: value for key, value in plan.items() if key != "plan_digest"}):
        raise LifecycleError("plan-binding", "reviewed plan schema or digest differs")
    if plan.get("workspace") != str(Path(store.workspace).resolve()):
        raise LifecycleError("plan-binding", "reviewed plan belongs to another workspace")
    selected = plan.get("selected")
    if not isinstance(selected, list) or any(not isinstance(entry, dict) or not isinstance(entry.get("id"), str) for entry in selected):
        raise LifecycleError("plan-binding", "reviewed candidates are invalid")
    identities = [entry["id"] for entry in selected]
    if len(identities) != len(set(identities)):
        raise LifecycleError("plan-binding", "reviewed candidates are duplicated")
    policy = _policy(plan.get("policy"))
    with ExitStack() as leases:
        for identity in sorted(identities):
            try:
                leases.enter_context(store.lease(identity))
            except Exception as exc:
                raise LifecycleError("lease", "evaluation is active or has an unresolved lease") from exc
        current = _plan(store, policy, plan.get("action"), identities, ignore_leases=identities, detail=True)
        if [_selected_binding(entry) for entry in current["selected"]] != selected:
            raise LifecycleError("plan-stale", "reviewed inventory, protection, or retention eligibility changed")
        results = []
        for entry in current["selected"]:
            row = store.show(entry["id"])
            root = _owned_root(store, row)
            if plan["action"] == "archive":
                results.append(_archive(store, row, entry))
            else:
                results.append(_purge(store, row, entry, root))
    receipt = {"schema": "opi-eval-maintenance-receipt/1", "action": plan["action"],
               "plan_digest": digest, "results": results}
    receipt_path = Path(store.workspace) / "maintenance" / (uuid.uuid4().hex + ".json")
    _write(receipt_path, receipt)
    return {**receipt, "receipt_path": str(receipt_path)}


def _purge(store, row, entry, root):
    fields = {"tombstone": True, "inventory_digest": entry["inventory_digest"],
              "inventory": entry["inventory"], "bytes": entry["bytes"], "root": str(root)}
    if entry["archive"]:
        _, _, _, binding = _archived_facts(store, row)
        if binding != entry["archive"]:
            raise LifecycleError("archive-drift", "reviewed archive binding differs")
        archive = Path(binding["archive_path"])
        store.event(row["id"], "purge", **fields, **binding, cleanup_state="pending")
        archive.unlink()
        store.event(row["id"], "purge", **fields, **binding, cleanup_state="confirmed")
    else:
        store.event(row["id"], "purge", **fields, cleanup_state="pending")
        # Purge tolerates staged symlinks and hardlinks (e.g. uv-installed
        # runner venv packages): deletion only drops this tree's links.
        _remove_exact(root, entry["inventory"], allow_links=True)
        store.event(row["id"], "purge", **fields, cleanup_state="confirmed")
    return {"id": row["id"], "inventory_digest": entry["inventory_digest"], "bytes": entry["bytes"]}


def apply_retention(store, policy=None, *, retain_ids=()):
    """Roll finished owned history down to latest N, reporting every protection gap."""
    policy = _policy(policy)
    if not isinstance(retain_ids, (list, tuple, set)) or any(not isinstance(identity, str) for identity in retain_ids):
        raise LifecycleError("identity", "retained current identities must be strings")
    purged, refusals = [], {}
    for _ in range(MAX_INVENTORY_ENTRIES):
        plan = _plan(store, policy, "purge", retain_ids=retain_ids, detail=True)
        candidates = [entry for entry in plan["selected"] if entry["id"] not in refusals]
        if not candidates:
            break
        # Purge one sourceholder, then rebuild the graph before selecting its targets.
        entry = candidates[0]
        try:
            with store.lease(entry["id"]):
                current = _plan(store, policy, "purge", ids=[entry["id"]],
                                ignore_leases=[entry["id"]], retain_ids=retain_ids, detail=True)
                if current["selected"] != [entry]:
                    raise LifecycleError("retention-stale", "retention protection or inventory changed")
                row = store.show(entry["id"])
                purged.append(_purge(store, row, entry, _owned_root(store, row)))
        except (OperatorError, OSError, ValueError, TypeError, KeyError):
            refusals[entry["id"]] = "maintenance_refused"
    else:
        raise LifecycleError("retention-bound", "retention iteration bound exceeded")
    final = _plan(store, policy, "purge", retain_ids=retain_ids)
    protected, excess = [], []
    for entry in final["inventory"]:
        if entry["storage_state"] not in {"active", "archived"}:
            continue
        reasons = sorted(set(entry["protected"] + ([refusals[entry["id"]]] if entry["id"] in refusals else [])))
        summary = {"id": entry["id"], "root": entry["root"], "protected": reasons,
                   "retention_rank": entry.get("retention_rank"), "groups": entry["groups"]}
        if reasons:
            protected.append(summary)
        if entry.get("excess"):
            excess.append(summary)
    return {"schema": "opi-eval-retention-result/1", "policy": policy, "purged": purged,
            "protected": protected, "remaining_excess": excess}


def restore(store, identity):
    """Restore verified archived bytes to the immutable original canonical root."""
    try:
        context = store.lease(identity)
        with context:
            row = store.show(identity)
            if row.get("storage_state") != "archived":
                raise LifecycleError("restore-state", "evaluation is not archived")
            root = _owned_root(store, row)
            fields = row.get("lifecycle", {})
            archive = Path(fields.get("archive_path", ""))
            base = _archive_base(store, identity)
            if (archive.is_symlink() or not archive.is_file() or not _inside(archive.resolve(), base)
                    or _sha(archive) != fields.get("archive_sha256")
                    or archive.stat().st_size != fields.get("archive_bytes")):
                raise LifecycleError("archive-digest", "archive location or digest differs")
            inventory = fields.get("inventory")
            if not isinstance(inventory, list) or _digest(inventory) != fields.get("inventory_digest"):
                raise LifecycleError("archive-inventory", "archive inventory binding differs")
            current = _inventory(root)
            expected_record = [entry for entry in inventory if entry["path"] == "record.json"]
            if current != expected_record:
                raise LifecycleError("restore-exists", "canonical destination contains existing evidence")
            stage = base / (".restore-" + uuid.uuid4().hex)
            stage.mkdir()
            try:
                _extract_verified(archive, stage, inventory)
                # Move only verified first-level children into the record-only root.
                for child in sorted(stage.iterdir(), key=lambda child: child.name):
                    if child.name == "record.json":
                        continue
                    target = root / child.name
                    if target.exists() or target.is_symlink() or not _inside(target.resolve(), root):
                        raise LifecycleError("restore-exists", "restore destination appeared concurrently")
                    child.rename(target)
                if _inventory(root) != inventory:
                    raise LifecycleError("restore-digest", "restored evidence inventory differs")
                store.event(identity, "restore", inventory_digest=fields["inventory_digest"],
                            archive_path=str(archive), archive_sha256=fields["archive_sha256"])
                return {"schema": "opi-eval-restore-receipt/1", "id": identity,
                        "root": str(root), "inventory_digest": fields["inventory_digest"]}
            finally:
                _discard_stage(stage)
    except LifecycleError:
        raise
    except Exception as exc:
        raise LifecycleError("restore-refused", "restore was refused; original archive retained") from exc

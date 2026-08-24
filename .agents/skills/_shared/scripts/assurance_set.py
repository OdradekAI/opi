#!/usr/bin/env python3
"""Install independently produced Opi audit reports into the live assurance set.

Each audit peer renders its four suffixed files in a private member directory.
`complete` validates that member, admits it against Git-clean rotation rules,
requires its audit head to be a committed ancestor of HEAD, and installs it
into `docs/snapshots/phase<N>/assurance/` under the repository assurance lock:
the four files land in place and `audit.index.json` is written last as the
membership switch. A re-run of the same reviewer/model replaces its own entry
and archives the superseded run under `history/<audit-run-id>/`. `migrate`
converts one legacy schema-1 generation set into the live schema-2 set.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import uuid
from pathlib import Path
from typing import Any


sys.dont_write_bytecode = True


FULL_SHA_RE = re.compile(r"^[0-9a-f]{40}$")
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
PHASE_RE = re.compile(r"^phase([1-9][0-9]*)$")
SLUG_RE = re.compile(r"^[a-z0-9][a-z0-9-]*$")

INDEX_MEMBER_FIELDS = (
    "reviewer_id",
    "model_id",
    "artifact_stem",
    "audit_run_id",
    "audit_head",
    "verdict",
    "digests",
)
DIGEST_SUFFIXES = {
    "meta_sha256": "meta.json",
    "requirements_sha256": "requirements.jsonl",
    "findings_sha256": "findings.jsonl",
    "report_sha256": "md",
}


class AssuranceSetError(RuntimeError):
    pass


def run_git(path: Path, *args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", "-C", str(path), *args],
        capture_output=True,
        text=True,
        check=False,
    )


def repository_root(path: Path) -> Path:
    result = run_git(path, "rev-parse", "--show-toplevel")
    if result.returncode != 0:
        raise AssuranceSetError("phase path is not inside a Git repository")
    return Path(result.stdout.strip()).resolve()


def git_common_directory(repository: Path) -> Path:
    result = run_git(repository, "rev-parse", "--git-common-dir")
    if result.returncode != 0:
        raise AssuranceSetError("cannot resolve Git common directory")
    path = Path(result.stdout.strip())
    if not path.is_absolute():
        path = repository / path
    return path.resolve()


def committed_head(repository: Path) -> str:
    result = run_git(repository, "rev-parse", "--verify", "HEAD")
    value = result.stdout.strip()
    if result.returncode != 0 or FULL_SHA_RE.fullmatch(value) is None:
        raise AssuranceSetError("repository has no valid committed HEAD")
    return value


def is_ancestor(repository: Path, commit: str) -> bool:
    result = run_git(repository, "merge-base", "--is-ancestor", commit, "HEAD")
    return result.returncode == 0


class AssuranceLock:
    def __init__(self, repository: Path, phase: int) -> None:
        lock_directory = git_common_directory(repository) / "opi-assurance-locks"
        lock_directory.mkdir(parents=True, exist_ok=True)
        self.path = lock_directory / f"phase{phase}.lock"
        self.phase = phase
        self.handle: Any = None
        self.locked = False

    def __enter__(self) -> "AssuranceLock":
        self.handle = self.path.open("a+b")
        self.handle.seek(0, os.SEEK_END)
        if self.handle.tell() == 0:
            self.handle.write(b"0")
            self.handle.flush()
        self.handle.seek(0)
        try:
            if os.name == "nt":
                import msvcrt

                msvcrt.locking(self.handle.fileno(), msvcrt.LK_NBLCK, 1)
            else:
                import fcntl

                fcntl.flock(self.handle.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
        except OSError as exc:
            self.handle.close()
            self.handle = None
            raise AssuranceSetError(f"assurance phase{self.phase} is locked") from exc
        self.locked = True
        return self

    def __exit__(self, exc_type: Any, exc: Any, traceback: Any) -> None:
        if self.locked and self.handle is not None:
            self.handle.seek(0)
            if os.name == "nt":
                import msvcrt

                msvcrt.locking(self.handle.fileno(), msvcrt.LK_UNLCK, 1)
            else:
                import fcntl

                fcntl.flock(self.handle.fileno(), fcntl.LOCK_UN)
        if self.handle is not None:
            self.handle.close()
        self.handle = None
        self.locked = False


def parse_phase(phase_directory: Path) -> int:
    match = PHASE_RE.fullmatch(phase_directory.name)
    if match is None:
        raise AssuranceSetError("phase path must end in phase<N>")
    return int(match.group(1))


def assurance_root(phase_directory: Path) -> Path:
    phase_root = phase_directory.resolve()
    resolved = (phase_root / "assurance").resolve()
    try:
        resolved.relative_to(phase_root)
    except ValueError as exc:
        raise AssuranceSetError("live assurance directory escapes its Phase root") from exc
    resolved.mkdir(parents=True, exist_ok=True)
    return resolved


def require_external_member_dir(
    phase_directory: Path, member_directory: Path
) -> None:
    assurance_directory = (phase_directory / "assurance").resolve()
    try:
        member_directory.resolve().relative_to(assurance_directory)
    except ValueError:
        return
    raise AssuranceSetError(
        "member directory must be outside the live assurance directory"
    )


def atomic_write_json(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, temporary_name = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    temporary = Path(temporary_name)
    try:
        with os.fdopen(fd, "w", encoding="utf-8", newline="\n") as handle:
            json.dump(value, handle, indent=2, sort_keys=True)
            handle.write("\n")
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, path)
    finally:
        if temporary.exists():
            temporary.unlink()


def raw_sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def ensure_within(path: Path, root: Path, label: str) -> Path:
    resolved_path = path.resolve()
    resolved_root = root.resolve()
    try:
        resolved_path.relative_to(resolved_root)
    except ValueError as exc:
        raise AssuranceSetError(f"{label} escapes its authorized root") from exc
    return resolved_path


def atomic_copy(source: Path, destination: Path) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    fd, temporary_name = tempfile.mkstemp(
        prefix=f".{destination.name}.", dir=destination.parent
    )
    temporary = Path(temporary_name)
    try:
        with source.open("rb") as source_handle, os.fdopen(fd, "wb") as output:
            shutil.copyfileobj(source_handle, output)
            output.flush()
            os.fsync(output.fileno())
        os.replace(temporary, destination)
    finally:
        if temporary.exists():
            temporary.unlink()


def transaction_base(repository: Path) -> Path:
    return git_common_directory(repository) / "opi-assurance-transactions"


def read_journal(transaction_directory: Path) -> dict[str, Any]:
    path = transaction_directory / "journal.json"
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise AssuranceSetError(f"cannot read assurance recovery journal: {exc}") from exc
    if not isinstance(value, dict):
        raise AssuranceSetError("assurance recovery journal must be a JSON object")
    return value


def root_files(assurance_directory: Path) -> list[Path]:
    assurance_directory.mkdir(parents=True, exist_ok=True)
    unexpected_directories = [
        path.name
        for path in assurance_directory.iterdir()
        if path.is_dir() and path.name != "history"
    ]
    if unexpected_directories:
        raise AssuranceSetError(
            "unexpected active assurance directories: "
            + ", ".join(sorted(unexpected_directories))
        )
    return sorted(
        (path for path in assurance_directory.iterdir() if path.is_file()),
        key=lambda path: path.name,
    )


def same_flat_files(left: Path, right: Path, names: list[str]) -> bool:
    expected = set(names)
    if not left.is_dir() or not right.is_dir():
        return False
    if {path.name for path in left.iterdir()} != expected:
        return False
    if {path.name for path in right.iterdir()} != expected:
        return False
    return all(
        (left / name).is_file()
        and (right / name).is_file()
        and (left / name).read_bytes() == (right / name).read_bytes()
        for name in names
    )


def member_files_equal(left: Path, right: Path, names: list[str]) -> bool:
    if not left.is_dir() or not right.is_dir():
        return False
    for name in names:
        source = left / name
        target = right / name
        if not source.is_file() or not target.is_file():
            return False
        if source.read_bytes() != target.read_bytes():
            return False
    return True


def remove_transaction(transaction_directory: Path, repository: Path) -> None:
    base = transaction_base(repository).resolve()
    target = ensure_within(transaction_directory, base, "transaction cleanup")
    if target == base:
        raise AssuranceSetError("transaction cleanup cannot target the transaction root")
    shutil.rmtree(target)


def aggregate_verdict(verdicts: list[str]) -> str:
    if "FAIL" in verdicts:
        return "FAIL"
    if "PASS-WITH-FINDINGS" in verdicts:
        return "PASS-WITH-FINDINGS"
    return "PASS"


def member_file_names(stem: str) -> list[str]:
    return [
        f"{stem}.meta.json",
        f"{stem}.requirements.jsonl",
        f"{stem}.findings.jsonl",
        f"{stem}.md",
    ]


def build_next_index(
    live_index: dict[str, Any] | None, entry: dict[str, Any], phase: int
) -> dict[str, Any]:
    pair = (entry["reviewer_id"], entry["model_id"])
    members = [
        dict(member)
        for member in ((live_index or {}).get("members") or [])
        if (member.get("reviewer_id"), member.get("model_id")) != pair
    ]
    members.append({field: entry[field] for field in INDEX_MEMBER_FIELDS})
    members.sort(key=lambda member: (member["reviewer_id"], member["model_id"]))
    revision = (live_index["revision"] + 1) if live_index else 1
    return {
        "schema_version": 2,
        "phase": phase,
        "revision": revision,
        "aggregate_verdict": aggregate_verdict(
            [member["verdict"] for member in members]
        ),
        "members": members,
    }


def new_transaction(repository: Path, phase: int) -> Path:
    base = transaction_base(repository)
    base.mkdir(parents=True, exist_ok=True)
    transaction = base / f"phase{phase}-{uuid.uuid4().hex}"
    transaction.mkdir()
    (transaction / "prior").mkdir()
    (transaction / "new").mkdir()
    return transaction


def prepare_member_install(
    repository: Path,
    phase_directory: Path,
    member_directory: Path,
    entry: dict[str, Any],
) -> tuple[Path, dict[str, Any]]:
    assurance_directory = assurance_root(phase_directory)
    phase = parse_phase(phase_directory)
    stem = str(entry["artifact_stem"])
    files = member_file_names(stem)
    index_path = assurance_directory / "audit.index.json"
    prior_index = index_path.is_file()
    live_index: dict[str, Any] | None = None
    replaced_run_id: str | None = None
    if prior_index:
        live_index = json.loads(index_path.read_text(encoding="utf-8"))
        replaced = next(
            (
                member
                for member in live_index.get("members", [])
                if isinstance(member, dict)
                and (member.get("reviewer_id"), member.get("model_id"))
                == (entry["reviewer_id"], entry["model_id"])
            ),
            None,
        )
        if replaced is not None:
            replaced_run_id = str(replaced["audit_run_id"])
    next_index = build_next_index(live_index, entry, phase)
    transaction = new_transaction(repository, phase)
    prior = transaction / "prior"
    new = transaction / "new"
    if prior_index:
        atomic_copy(index_path, prior / "audit.index.json")
    if replaced_run_id is not None:
        for name in files:
            source = ensure_within(
                assurance_directory / name, assurance_directory, "prior member file"
            )
            if not source.is_file():
                raise AssuranceSetError(f"replaced member file is missing: {name}")
            atomic_copy(source, prior / name)
    for name in files:
        source = ensure_within(
            member_directory / name, member_directory, "staged member file"
        )
        if not source.is_file():
            raise AssuranceSetError(f"staged member file is missing: {name}")
        atomic_copy(source, new / name)
    atomic_write_json(new / "audit.index.json", next_index)
    journal = {
        "schema_version": 2,
        "mode": "member-install",
        "state": "prepared",
        "phase_directory": str(phase_directory.resolve()),
        "member_stem": stem,
        "member_files": files,
        "replaced_run_id": replaced_run_id,
        "prior_index": prior_index,
        "next_revision": next_index["revision"],
        "aggregate_verdict": next_index["aggregate_verdict"],
        "intended_index_sha256": raw_sha256(new / "audit.index.json"),
    }
    atomic_write_json(transaction / "journal.json", journal)
    return transaction, journal


def install_member(
    assurance_directory: Path,
    transaction: Path,
    journal: dict[str, Any],
) -> None:
    journal["state"] = "installing"
    atomic_write_json(transaction / "journal.json", journal)
    files = [str(name) for name in journal["member_files"]]
    replaced_run_id = journal.get("replaced_run_id")
    if replaced_run_id:
        ready = transaction / "history-ready"
        if ready.exists():
            shutil.rmtree(ready)
        ready.mkdir()
        for name in files:
            source = ensure_within(
                assurance_directory / name, assurance_directory, "history member file"
            )
            atomic_copy(source, ready / name)
        for name in files:
            ensure_within(
                assurance_directory / name, assurance_directory, "member replace"
            ).unlink()
    for name in files:
        atomic_copy(transaction / "new" / name, assurance_directory / name)
    atomic_copy(
        transaction / "new" / "audit.index.json",
        assurance_directory / "audit.index.json",
    )


def finalize_member_history(
    assurance_directory: Path,
    transaction: Path,
    journal: dict[str, Any],
) -> None:
    files = [str(name) for name in journal["member_files"]]
    ready = transaction / "history-ready"
    replaced_run_id = journal.get("replaced_run_id")
    if not replaced_run_id:
        if ready.exists():
            shutil.rmtree(ready)
        return
    history_root = assurance_directory / "history"
    history_root.mkdir(parents=True, exist_ok=True)
    target = ensure_within(
        history_root / str(replaced_run_id), history_root, "history target"
    )
    if target.exists():
        if target.is_dir() and member_files_equal(ready, target, files):
            return
        raise AssuranceSetError("history target exists with different bytes")
    os.replace(ready, target)


def recover_member_transaction(
    repository: Path,
    assurance_directory: Path,
    transaction: Path,
    journal: dict[str, Any],
) -> None:
    files = [str(name) for name in journal["member_files"]]
    index_path = assurance_directory / "audit.index.json"
    if journal.get("state") == "switched":
        intended = journal.get("intended_index_sha256")
        index_matches = (
            isinstance(intended, str)
            and SHA256_RE.fullmatch(intended) is not None
            and index_path.is_file()
            and raw_sha256(index_path) == intended
        )
        if index_matches:
            import validate_assurance_artifact as validator

            if not validator.validate_audit_set(assurance_directory):
                finalize_member_history(assurance_directory, transaction, journal)
                remove_transaction(transaction, repository)
                return
    for name in files:
        live = ensure_within(
            assurance_directory / name, assurance_directory, "rollback delete"
        )
        if live.exists():
            live.unlink()
    if journal.get("replaced_run_id"):
        for name in files:
            atomic_copy(transaction / "prior" / name, assurance_directory / name)
    if journal.get("prior_index"):
        atomic_copy(transaction / "prior" / "audit.index.json", index_path)
    elif index_path.exists():
        index_path.unlink()
    ready = transaction / "history-ready"
    if ready.exists():
        shutil.rmtree(ready)
    replaced_run_id = journal.get("replaced_run_id")
    if isinstance(replaced_run_id, str):
        history_target = assurance_directory / "history" / replaced_run_id
        if history_target.is_dir():
            if member_files_equal(transaction / "prior", history_target, files):
                shutil.rmtree(history_target)
            else:
                raise AssuranceSetError(
                    "cannot remove non-transaction history during recovery"
                )
    remove_transaction(transaction, repository)


def strip_generation_header(data: bytes) -> bytes:
    lines = data.replace(b"\r\n", b"\n").split(b"\n")
    kept = [
        line
        for line in lines
        if not line.startswith(b"**Audit generation ID**")
    ]
    return b"\n".join(kept)


def lf_only(data: bytes) -> bytes:
    return data.replace(b"\r\n", b"\n")


def prepare_set_rewrite(
    repository: Path, phase_directory: Path
) -> tuple[Path, dict[str, Any]]:
    assurance_directory = assurance_root(phase_directory)
    phase = parse_phase(phase_directory)
    index_path = assurance_directory / "audit.index.json"
    if not index_path.is_file():
        raise AssuranceSetError("legacy index v1 is required for migration")
    legacy_index = json.loads(index_path.read_text(encoding="utf-8"))
    if legacy_index.get("schema_version") != 1:
        raise AssuranceSetError("legacy index v1 is required for migration")
    legacy_members = legacy_index.get("members")
    if not isinstance(legacy_members, list) or not legacy_members:
        raise AssuranceSetError("legacy index has no members")

    transaction = new_transaction(repository, phase)
    prior = transaction / "prior"
    new = transaction / "new"
    prior_names = [path.name for path in root_files(assurance_directory)]
    for path in root_files(assurance_directory):
        atomic_copy(path, prior / path.name)

    entries: list[dict[str, Any]] = []
    new_names: list[str] = []
    for member in legacy_members:
        stem = str(member["artifact_stem"])
        meta_path = assurance_directory / f"{stem}.meta.json"
        meta = json.loads(meta_path.read_text(encoding="utf-8"))
        digests = member["digests"]
        for field, suffix in DIGEST_SUFFIXES.items():
            live = assurance_directory / f"{stem}.{suffix}"
            if not live.is_file() or raw_sha256(live) != digests.get(field):
                raise AssuranceSetError(
                    f"legacy member digests do not match live bytes: {stem}.{suffix}"
                )
        migrated = dict(meta)
        migrated.pop("audit_generation_id", None)
        migrated["schema_version"] = 3
        write_bytes = json.dumps(migrated, indent=2, sort_keys=True).encode("utf-8") + b"\n"
        (new / f"{stem}.meta.json").write_bytes(write_bytes)
        (new / f"{stem}.requirements.jsonl").write_bytes(
            lf_only((assurance_directory / f"{stem}.requirements.jsonl").read_bytes())
        )
        (new / f"{stem}.findings.jsonl").write_bytes(
            lf_only((assurance_directory / f"{stem}.findings.jsonl").read_bytes())
        )
        report_bytes = strip_generation_header(
            (assurance_directory / f"{stem}.md").read_bytes()
        )
        (new / f"{stem}.md").write_bytes(report_bytes)
        entries.append(
            {
                "reviewer_id": member["reviewer_id"],
                "model_id": member["model_id"],
                "artifact_stem": stem,
                "audit_run_id": meta["audit_run_id"],
                "audit_head": meta["audit_head"],
                "verdict": member["verdict"],
                "digests": {
                    "meta_sha256": hashlib.sha256(write_bytes).hexdigest(),
                    "requirements_sha256": digests["requirements_sha256"],
                    "findings_sha256": digests["findings_sha256"],
                    "report_sha256": hashlib.sha256(report_bytes).hexdigest(),
                },
            }
        )
        new_names.extend(member_file_names(stem))
    entries.sort(key=lambda entry: (entry["reviewer_id"], entry["model_id"]))
    next_index = {
        "schema_version": 2,
        "phase": phase,
        "revision": int(legacy_index["revision"]) + 1,
        "aggregate_verdict": aggregate_verdict(
            [entry["verdict"] for entry in entries]
        ),
        "members": entries,
    }
    atomic_write_json(new / "audit.index.json", next_index)
    new_names.append("audit.index.json")
    journal = {
        "schema_version": 2,
        "mode": "set-rewrite",
        "state": "prepared",
        "phase_directory": str(phase_directory.resolve()),
        "prior_files": prior_names,
        "new_files": sorted(new_names),
        "next_revision": next_index["revision"],
        "member_count": len(entries),
        "aggregate_verdict": next_index["aggregate_verdict"],
        "intended_index_sha256": raw_sha256(new / "audit.index.json"),
    }
    atomic_write_json(transaction / "journal.json", journal)
    return transaction, journal


def install_set(
    assurance_directory: Path,
    transaction: Path,
    journal: dict[str, Any],
) -> None:
    journal["state"] = "installing"
    atomic_write_json(transaction / "journal.json", journal)
    for path in root_files(assurance_directory):
        ensure_within(path, assurance_directory, "set rewrite delete").unlink()
    new = transaction / "new"
    new_files = [str(name) for name in journal["new_files"]]
    if "audit.index.json" not in new_files:
        raise AssuranceSetError("transaction new-file roster is invalid")
    for name in new_files:
        if name == "audit.index.json":
            continue
        atomic_copy(new / name, assurance_directory / name)
    atomic_copy(new / "audit.index.json", assurance_directory / "audit.index.json")


def recover_set_transaction(
    repository: Path,
    assurance_directory: Path,
    transaction: Path,
    journal: dict[str, Any],
) -> None:
    index_path = assurance_directory / "audit.index.json"
    if journal.get("state") == "switched":
        intended = journal.get("intended_index_sha256")
        index_matches = (
            isinstance(intended, str)
            and SHA256_RE.fullmatch(intended) is not None
            and index_path.is_file()
            and raw_sha256(index_path) == intended
        )
        if index_matches:
            import validate_assurance_artifact as validator

            if not validator.validate_audit_set(assurance_directory):
                remove_transaction(transaction, repository)
                return
    for path in root_files(assurance_directory):
        ensure_within(path, assurance_directory, "set rollback delete").unlink()
    prior = transaction / "prior"
    for name in journal["prior_files"]:
        source = ensure_within(prior / str(name), prior, "prior backup")
        if not source.is_file():
            raise AssuranceSetError(f"prior backup is missing {name}")
        atomic_copy(source, assurance_directory / str(name))
    remove_transaction(transaction, repository)


def recover_transaction(
    repository: Path,
    phase_directory: Path,
    transaction: Path,
    journal: dict[str, Any],
) -> None:
    mode = journal.get("mode")
    assurance_directory = assurance_root(phase_directory)
    if mode == "member-install":
        recover_member_transaction(
            repository, assurance_directory, transaction, journal
        )
    elif mode == "set-rewrite":
        recover_set_transaction(repository, assurance_directory, transaction, journal)
    else:
        raise AssuranceSetError("assurance recovery journal mode is invalid")


def recover_locked(repository: Path, phase_directory: Path) -> None:
    base = transaction_base(repository)
    if not base.is_dir():
        return
    resolved_phase = phase_directory.resolve()
    phase_prefix = f"phase{parse_phase(resolved_phase)}-"
    for transaction_directory in sorted(
        (path for path in base.iterdir() if path.is_dir()), key=lambda path: path.name
    ):
        if not transaction_directory.name.startswith(phase_prefix):
            continue
        if not (transaction_directory / "journal.json").is_file():
            remove_transaction(transaction_directory, repository)
            continue
        journal = read_journal(transaction_directory)
        journal_phase = journal.get("phase_directory")
        if not isinstance(journal_phase, str):
            raise AssuranceSetError("assurance recovery journal phase path is invalid")
        if Path(journal_phase).resolve() != resolved_phase:
            continue
        recover_transaction(repository, resolved_phase, transaction_directory, journal)


def complete_member(args: argparse.Namespace) -> None:
    phase_directory = args.phase_directory.resolve()
    member_directory = args.member_directory.resolve()
    repository = repository_root(phase_directory)
    phase = parse_phase(phase_directory)
    require_external_member_dir(phase_directory, member_directory)
    assurance_directory = assurance_root(phase_directory)
    with AssuranceLock(repository, phase):
        recover_locked(repository, phase_directory)

        import validate_assurance_artifact as validator

        meta, digests, errors = validator.validate_staged_member(
            member_directory,
            phase=phase,
            reviewer_id=args.reviewer,
            model_id=args.model,
        )
        if errors:
            raise AssuranceSetError(
                "member validation failed: " + "; ".join(errors)
            )
        rotation_errors = validator.validate_rotation(phase_directory)
        if rotation_errors:
            raise AssuranceSetError(
                "admission refused: " + "; ".join(rotation_errors)
            )
        if not is_ancestor(repository, str(meta["audit_head"])):
            raise AssuranceSetError(
                "audit head is not a committed ancestor of HEAD: "
                f"{meta['audit_head']}"
            )
        index_path = assurance_directory / "audit.index.json"
        live_index: dict[str, Any] | None = None
        if index_path.is_file():
            live_index = json.loads(index_path.read_text(encoding="utf-8"))
            if live_index.get("schema_version") != 2:
                raise AssuranceSetError(
                    "legacy audit.index.json schema 1 requires migrate"
                )
            live_errors = validator.validate_audit_set(assurance_directory)
            if live_errors:
                raise AssuranceSetError(
                    "current live audit set is invalid: " + "; ".join(live_errors)
                )
            for member in live_index.get("members", []):
                if (
                    member.get("reviewer_id") == args.reviewer
                    and member.get("model_id") == args.model
                    and member.get("audit_run_id") == meta["audit_run_id"]
                ):
                    raise AssuranceSetError("audit run is already installed")
        else:
            existing = [path for path in assurance_directory.iterdir() if path.is_file()]
            if existing:
                raise AssuranceSetError(
                    "active assurance files exist without audit.index.json"
                )
        entry = {
            "reviewer_id": args.reviewer,
            "model_id": args.model,
            "artifact_stem": f"audit.{args.reviewer}.{args.model}",
            "audit_run_id": meta["audit_run_id"],
            "audit_head": meta["audit_head"],
            "verdict": meta["verdict"],
            "digests": digests,
        }
        transaction, journal = prepare_member_install(
            repository, phase_directory, member_directory, entry
        )
        try:
            install_member(assurance_directory, transaction, journal)
            live_errors = validator.validate_audit_set(assurance_directory)
            if live_errors:
                raise AssuranceSetError(
                    "installed live audit set is invalid: " + "; ".join(live_errors)
                )
            journal["state"] = "switched"
            atomic_write_json(transaction / "journal.json", journal)
            finalize_member_history(assurance_directory, transaction, journal)
            remove_transaction(transaction, repository)
        except (AssuranceSetError, OSError):
            if transaction.is_dir():
                recover_transaction(
                    repository,
                    phase_directory,
                    transaction,
                    read_journal(transaction),
                )
            raise
    print(
        "complete: PASS "
        f"reviewer={args.reviewer} model={args.model} "
        f"revision={journal['next_revision']} "
        f"audit_run_id={entry['audit_run_id']} "
        f"aggregate_verdict={journal['aggregate_verdict']}"
    )


def migrate_set(args: argparse.Namespace) -> None:
    phase_directory = args.phase_directory.resolve()
    repository = repository_root(phase_directory)
    phase = parse_phase(phase_directory)
    assurance_directory = assurance_root(phase_directory)
    with AssuranceLock(repository, phase):
        recover_locked(repository, phase_directory)

        import validate_assurance_artifact as validator

        rotation_errors = validator.validate_rotation(phase_directory)
        if rotation_errors:
            raise AssuranceSetError(
                "admission refused: " + "; ".join(rotation_errors)
            )
        transaction, journal = prepare_set_rewrite(repository, phase_directory)
        try:
            install_set(assurance_directory, transaction, journal)
            live_errors = validator.validate_audit_set(assurance_directory)
            if live_errors:
                raise AssuranceSetError(
                    "migrated live audit set is invalid: " + "; ".join(live_errors)
                )
            journal["state"] = "switched"
            atomic_write_json(transaction / "journal.json", journal)
            remove_transaction(transaction, repository)
        except (AssuranceSetError, OSError):
            if transaction.is_dir():
                recover_transaction(
                    repository,
                    phase_directory,
                    transaction,
                    read_journal(transaction),
                )
            raise
    print(
        "migrate: PASS "
        f"revision={journal['next_revision']} "
        f"members={journal['member_count']} "
        f"aggregate_verdict={journal['aggregate_verdict']}"
    )


def recover_set(args: argparse.Namespace) -> None:
    phase_directory = args.phase_directory.resolve()
    repository = repository_root(phase_directory)
    phase = parse_phase(phase_directory)
    with AssuranceLock(repository, phase):
        recover_locked(repository, phase_directory)
    print(f"recover: PASS phase={phase}")


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    complete = subparsers.add_parser("complete")
    complete.add_argument("phase_directory", type=Path)
    complete.add_argument("member_directory", type=Path)
    complete.add_argument("--reviewer", required=True)
    complete.add_argument("--model", required=True)
    complete.set_defaults(handler=complete_member)
    recover = subparsers.add_parser("recover")
    recover.add_argument("phase_directory", type=Path)
    recover.set_defaults(handler=recover_set)
    migrate = subparsers.add_parser("migrate")
    migrate.add_argument("phase_directory", type=Path)
    migrate.set_defaults(handler=migrate_set)
    return parser


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    try:
        args.handler(args)
    except (AssuranceSetError, OSError) as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

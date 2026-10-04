"""Check one Rust libtest target's inventories and observed execution offline."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path


MAX_LOG_BYTES = 16 * 1024 * 1024
LIST_ROW = re.compile(r"(.+): test")
LIST_TOTAL = re.compile(r"([0-9]{1,20}) tests?, ([0-9]{1,20}) benchmarks?")
RUN_ROW = re.compile(r"test (.+) \.\.\. (ok|FAILED|ignored)(?:, .*)?")
RUN_TOTAL = re.compile(
    r"test result: (ok|FAILED)\. ([0-9]{1,20}) passed; ([0-9]{1,20}) failed; "
    r"([0-9]{1,20}) ignored; ([0-9]{1,20}) measured; ([0-9]{1,20}) filtered out; "
    r"finished in .+"
)


class LogError(Exception):
    """Fixed diagnostics for incomplete or inconsistent input, without log payload."""


def inventory(text: str) -> set[str]:
    names: set[str] = set()
    totals = []
    for line in text.splitlines():
        match = LIST_ROW.fullmatch(line)
        if match:
            name = match[1]
            if name in names:
                raise LogError("duplicate test in inventory")
            names.add(name)
        total = LIST_TOTAL.fullmatch(line)
        if total:
            totals.append((int(total[1]), int(total[2])))
    if len(totals) != 1:
        raise LogError("inventory must contain one test target with a complete summary")
    if totals[0][0] != len(names):
        raise LogError("inventory count differs from listed tests")
    if totals[0][1] != 0:
        raise LogError("benchmark inventory is not a behavioral test inventory")
    return names


def execution(text: str) -> tuple[dict[str, set[str]], dict[str, int]]:
    cases: dict[str, set[str]] = {"ok": set(), "FAILED": set(), "ignored": set()}
    seen: set[str] = set()
    totals = []
    for line in text.splitlines():
        match = RUN_ROW.fullmatch(line)
        if match:
            name, status = match[1], match[2]
            if name in seen:
                raise LogError("duplicate test execution observation")
            seen.add(name)
            cases[status].add(name)
        total = RUN_TOTAL.fullmatch(line)
        if total:
            totals.append(total)
    if len(totals) != 1:
        raise LogError("run must contain one test target with a complete summary")
    total = totals[0]
    counts = dict(zip(
        ("passed", "failed", "ignored", "measured", "filtered"),
        (int(total[index]) for index in range(2, 7)),
    ))
    if any(counts[key] != len(cases[status]) for key, status in
           [("passed", "ok"), ("failed", "FAILED"), ("ignored", "ignored")]):
        raise LogError("summary counts differ from case observations")
    if counts["measured"] or (total[1] == "FAILED") != bool(counts["failed"]):
        raise LogError("run summary is not a consistent behavioral test result")
    counts["executed"] = counts["passed"] + counts["failed"]
    return cases, counts


def check_execution(
    all_text: str,
    ignored_text: str,
    run_text: str,
    *,
    test_filter: str | None = None,
    exact: bool = False,
    selection: str = "normal",
    expected_count: int | None = None,
) -> dict:
    if selection not in {"normal", "ignored", "include-ignored"}:
        raise LogError("unsupported test selection")
    if exact and not test_filter:
        raise LogError("exact selection requires a nonempty filter")
    if expected_count is not None and (type(expected_count) is not int or expected_count <= 0):
        raise LogError("expected count must be a positive integer")
    all_tests, ignored = inventory(all_text), inventory(ignored_text)
    if not ignored <= all_tests:
        raise LogError("ignored inventory is not a subset of the complete inventory")
    matching = {
        name for name in all_tests
        if test_filter is None or (name == test_filter if exact else test_filter in name)
    }
    scheduled = matching & ignored if selection == "ignored" else matching
    eligible = scheduled - ignored if selection == "normal" else scheduled
    expected_ignored = scheduled & ignored if selection == "normal" else set()
    cases, observed = execution(run_text)
    errors = []
    if test_filter is not None and not matching:
        errors.append("filter matches no listed test")
    if not eligible:
        errors.append("selection has no runnable tests")
    if observed["executed"] == 0:
        errors.append("zero executed tests cannot establish behavioral acceptance")
    if cases["ok"] | cases["FAILED"] != eligible:
        errors.append("executed identities differ from selected runnable tests")
    if cases["ignored"] != expected_ignored:
        errors.append("ignored identities differ from inventory selection")
    if observed["filtered"] != len(all_tests - scheduled):
        errors.append("filtered count differs from inventory selection")
    if expected_count is not None and expected_count != len(eligible):
        errors.append("expected count differs from selected runnable tests")
    if observed["failed"]:
        errors.append("executed tests failed")
    return {
        "status": "FAIL" if errors else "PASS",
        "inventory": len(all_tests),
        "matching": len(matching),
        "scheduled": len(scheduled),
        "eligible": len(eligible),
        "observed": observed,
        "errors": errors,
    }


def read_log(path: Path) -> tuple[str, dict[str, str]]:
    try:
        with path.open("rb") as stream:
            content = stream.read(MAX_LOG_BYTES + 1)
        if len(content) > MAX_LOG_BYTES:
            raise LogError("input log exceeds the 16 MiB limit")
        text = content.decode("utf-8-sig")
        if "\x00" in text:
            raise LogError("input log contains NUL bytes")
    except (OSError, UnicodeError) as error:
        raise LogError("input log cannot be read as UTF-8") from error
    return text, {"path": str(path), "sha256": hashlib.sha256(content).hexdigest()}


def positive_count(value: str) -> int:
    try:
        result = int(value)
    except ValueError as error:
        raise argparse.ArgumentTypeError("expected count must be a positive integer") from error
    if result <= 0:
        raise argparse.ArgumentTypeError("expected count must be a positive integer")
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--list-output", required=True, type=Path)
    parser.add_argument("--ignored-list-output", required=True, type=Path)
    parser.add_argument("--run-output", required=True, type=Path)
    parser.add_argument("--filter", dest="test_filter")
    parser.add_argument("--exact", action="store_true")
    parser.add_argument("--selection", choices=("normal", "ignored", "include-ignored"),
                        default="normal")
    parser.add_argument("--expected-count", type=positive_count)
    args = parser.parse_args()
    if args.exact and not args.test_filter:
        parser.error("--exact requires --filter")
    if args.test_filter == "":
        parser.error("--filter must be nonempty")
    try:
        inputs = {}
        logs = []
        for key in ("list_output", "ignored_list_output", "run_output"):
            text, identity = read_log(getattr(args, key))
            inputs[key] = identity
            logs.append(text)
        result = check_execution(*logs, test_filter=args.test_filter, exact=args.exact,
                                 selection=args.selection, expected_count=args.expected_count)
        result["inputs"] = inputs
    except LogError as error:
        result = {"status": "FAIL", "errors": [str(error)]}
    print(json.dumps(result, ensure_ascii=False))
    return 0 if result["status"] == "PASS" else 2


if __name__ == "__main__":
    raise SystemExit(main())

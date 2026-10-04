# Rust Test Execution Check

Use `../scripts/check_test_execution.py` to check one Rust libtest target's
complete inventory, ignored inventory, and execution log offline. It reads
existing UTF-8 logs, emits JSON with counts and input SHA-256, and returns
exit 0 for PASS or exit 2 for FAIL. It does not run commands or modify files.

## Capture and selection

In the sealed source export, select one `--test <target>` or `--lib` target,
using identical package/features/toolchain for all three commands. Follow
repository Cargo cache/lease rules. Capture stdout and stderr, the exact
commands and original process exits. Use the default named test output;
`--quiet`, `--skip`, benchmarks, doctests, and multiple targets are unsupported.

For example, capture these commands to separate logs:

```text
cargo test -p <package> --test <target> -- --list
cargo test -p <package> --test <target> -- --ignored --list
cargo test -p <package> --test <target> <filter> -- --exact
```

The first two inventories must be complete and unfiltered. Listing tests does
not execute them. Check the listed identities and ignored annotations before
running the third command. Choose its execution mode only within existing
authorization; `--ignored` may execute native or credentialed tests.

Then check the captured logs:

```text
python .agents/skills/opi-audit/scripts/check_test_execution.py --list-output <all.log> --ignored-list-output <ignored.log> --run-output <run.log> --filter <filter> --exact --selection normal --expected-count 1
```

Match options to the original execution, not to the desired result:

| Original libtest execution | Checker options |
|---|---|
| No filter | Omit `--filter` and `--exact` |
| Substring filter | `--filter <substring>` |
| Exact filter | `--filter <full-test-name> --exact` |
| Default ignored behavior | `--selection normal` |
| `--ignored` | `--selection ignored` |
| `--include-ignored` | `--selection include-ignored` |

`--expected-count` is optional, positive, and counts runnable tests rather than
ignored observations. A stale filter, normal selection of ignored-only tests,
or `--ignored` selection of a nonignored test fails when nothing executes.
The checker also rejects inconsistent identities/counts, failed tests,
incomplete summaries, duplicate observations, and combined targets. Each input
is limited to 16 MiB; split large targets through an appropriate runner instead
of trimming a transcript into a seemingly complete log.

## Evidence boundary

Put the command in `checks.command`; record the original exit, checker result,
input hashes, actual execution and limitations as text in `checks.observed`.
Retain original logs privately or in the supplied redacted proof directory.
A checker PASS establishes only internally consistent selection and execution.
It does not authenticate logs, prove assertion quality, bind a binary to
`audit_head`, or turn fixtures into native/operator acceptance. Validate those
separately under `audit-proof-obligations.md`. Zero execution can support a
compilation claim where appropriate, never a behavior claim.

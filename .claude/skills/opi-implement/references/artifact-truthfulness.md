# Artifact Truthfulness Gate

Use this gate whenever a task claims runtime, CLI, JSON/NDJSON, RPC, session, provider, tool, browser, or generated-artifact behavior.

## Required Saved Evidence

- Exact command line, working directory, environment overrides relevant to opi, and exit code.
- Stdout and stderr captured as files.
- Session directory or session JSONL when session behavior is claimed.
- NDJSON/RPC stream file when JSON/RPC behavior is claimed.
- Provider request capture or wiremock assertion when provider/tool availability is claimed.
- Browser console log, deterministic script, screenshot, and page-evaluation JSON when browser behavior is claimed.
- Direct curl output saved to a file when a report cites direct curl behavior.

## Classification Rules

- A claim is `verified` only when a preserved artifact proves it.
- A claim is `observed-unpreserved` when the operator saw it live but the artifact directory does not contain the raw evidence.
- A claim is `source-inferred` when source code proves the behavior but the run artifact does not.
- A claim is `not-opi` when the issue is generated output quality rather than opi runtime behavior.

## Required Checks

Run:

```sh
python scripts/opi-artifact-audit.py <artifact-dir> --workspace-root <workspace-root> --json
```

For Windows PowerShell:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\opi-artifact-audit.ps1 <artifact-dir> -WorkspaceRoot <workspace-root> -Json
```

## Blocking Findings

- Public NDJSON/session/export artifacts (message/tool records; the session-header `cwd` is allowed) contain the workspace root or user home path.
- Runtime message timestamps are all zero after timestamp support exists.
- `session_summary.provider_turns` differs from the number of `TurnStart` events after provider-turn support exists.
- Report text cites provider failures, HTTP status failures, or rate limits without preserved raw failure artifacts AND without an explicit disclosure phrase (e.g. "not preserved", "observed-unpreserved").
- Long text streams carry duplicate cumulative partial snapshots in default `--json` after compact support exists, OR a streamed run did not use `--json-compact` when the gate policy requires it.

## Reporting Format

Use this table in task evidence:

| Claim | Classification | Artifact | Result |
|---|---|---|---|
| `<specific behavior>` | `verified` | `<path>` | `<short result>` |

Never promote `observed-unpreserved` to `verified`.

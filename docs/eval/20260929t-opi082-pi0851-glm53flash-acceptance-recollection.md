# 2026-09-29: Opi 0.8.2 / Pi 0.85.1 glm-5.3-flash acceptance re-collection

([中文](20260929t-opi082-pi0851-glm53flash-acceptance-recollection.zh.md))

A user-authorized six-trial live acceptance collection with the final applied
tree, staged under the current evidence contracts (native material
`opi-eval-native-material/6`, agent terminal preflight `/2`, and the
admission-bound acceptance provenance verifier). This supersedes the
2026-09-15 acceptance root for current consumption; that historical root is
retained and registered separately.

## Evidence

- Evidence root: `C:/Users/Luiz/.local/state/opi-remediation-evidence/phase19-acceptance-20260929/evidence-root`
  (WSL: `/mnt/c/Users/Luiz/.local/state/opi-remediation-evidence/phase19-acceptance-20260929/evidence-root`),
  registered in [evidence-roots.jsonl](evidence-roots.jsonl).
  Manifest schema `opi-eval-local-acceptance/1`, SHA-256
  `9897f08fed7876b757542235e77e54735d233b1d6b0d82ec5024eab26fc6a9ce`.
- Clients: Opi 0.8.2 static musl (final applied tree, executable SHA-256
  `0ccc2b7f5be5257068c1d3bb8cb73dfeaa0e482c16b468872179a0b1720e2ad2`,
  build bound to source tree manifest `5e2f8220…`, dirty-state disclosure in
  the manifest); pi 0.85.1 runtime package from the pinned local cache.
- Materials: three `opi-eval-native-material/6` manifests produced by the
  final public `prepare` producer, with exact-client output-control proof
  (`max_output_tokens = 131072`) pinned for both clients on initial and
  tool-continuation requests.
- Connection: zhipu / anthropic-messages / glm-5.3-flash, one repetition per
  subject per family, concurrency 1, no automatic paid retry. The credential
  was read from the operator's `.env` only into the trial subprocess
  environment; zero occurrences of its value exist in the staged root
  (scanned).

## Native outcomes

| Family | Task | pi | opi | Notes |
|---|---|---|---|---|
| terminal-bench-2.1 | openssl-selfsigned-cert | reward 1 | reward 0 (agent failure, graded) | both sealed |
| terminal-bench-3.0 | batched-eval-parity | reward 0 (agent failure, graded) | reward 0 (agent failure, graded) | both sealed |
| deepswe-v1.1 | abs-module-cache-flags | reward 1 | reward 1 | both sealed |

Valid graded native outcomes including benchmark-valid agent failures; no
provider/auth/infrastructure failure is counted in the accepted set.

## Retained failed attempts

- `runs/tb30`: opi trial cancelled by an outer supervision timeout while the
  job was alive and streaming (pi sealed, reward 1).
- `runs/tb30-attempt2-cancelled`: 30-second launch probe, cancelled.
- `runs/deepswe`: pi trial hit the pinned native job deadline
  (`native-job-timeout`; opi sealed, reward 1).

The accepted tb30/deepswe trials are fresh-identity replacements
(`r1.r…`) under the documented replacement flow; every failed attempt stays
retained under the collection's run roots.

## Offline verification

- `verify-local-acceptance.py --root <E> --out <out> --companion <final
  companion d2502daf…>`: exit 0, `accepted: true`, all six trials and three
  families bound (admitted static-lock provenance, per-family material
  identities, sealed `/2` terminal preflight observations).
- `scripts/opi-artifact-audit.py <E>/opi-artifact-audit`: PASS
  (`ok: true`), non-empty, byte-bound and exhaustive for the three Opi
  trial captures.
- Per-family `verify` and `report` exit 0 on the sealed run roots.

This report records evidence, not task-completion authority: the owning
workflow's independent gates and the audit set decide phase state. C/P19-AUD-011
(original pinned offline regrade) remains a separate, unevaded exclusion.

# opi-eval native-ci verifier fixtures

Standalone negative fixtures for `crates/opi-eval/scripts/verify-native-smoke-ci.py`.
Most rejection families are exercised by mutating copies of the real
committed producer files inside `crates/opi-eval/scripts/test_verify_native_smoke_ci.py`;
the files here are independent negatives that keep rejecting even if the
real files drift, so a regression in the real contract cannot silently
mask a verifier gap.

- `workflow-floating-action.yml`: a workflow whose checkout action is a
  floating tag instead of a full commit (family `action`).
- `producer-second-listener.sh`: a producer that starts a second listener
  endpoint beside the admitted one (family `endpoint`).

The shared-preparation contract (CI materialization delegating to the
public local `prepare` path with per-family routing) is exercised by the
in-test mutations and the offline Bash-subprocess equivalence case in
`test_verify_native_smoke_ci.py`, not by standalone fixture files.

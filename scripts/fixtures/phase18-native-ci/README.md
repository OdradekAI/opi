# Phase 18 native-ci verifier fixtures (task 18.14)

Standalone negative fixtures for `scripts/verify-phase18-native-ci.py`.
Most rejection families are exercised by mutating copies of the real
committed producer files inside `scripts/test_verify_phase18_native_ci.py`;
the files here are independent negatives that keep rejecting even if the
real files drift, so a regression in the real contract cannot silently
mask a verifier gap.

- `workflow-floating-action.yml`: a workflow whose checkout action is a
  floating tag instead of a full commit (family `action`).
- `producer-second-listener.sh`: a producer that starts a second listener
  endpoint beside the admitted one (family `endpoint`).

# Frozen eval producer contracts

[简体中文](README.zh.md)

These YAML files preserve the former manually dispatched GitHub producers
byte-for-byte. They are inert development references, outside the Actions
workflow directory, used by offline contract tests and historical evidence
verification. Embedded `.github/workflows/` paths describe historical dispatch
identity; do not rewrite them or execute these files as local workflows.

Current calibration uses the [local CLI and operator](../../README.md). The
materializer's `prepare --cache --plan` mode remains the local acquisition
entrypoint. GitHub receipt modes require their original authority; no local
command may manufacture GitHub environment or upload receipts.

The current static lock points to the relocated external-lock contract with
its unchanged workflow digest. That path change gives the static lock a new
digest. Old external artifacts require their matching historical checkout,
lock and producer bytes. Native artifact verification retains the original
workflow path and reads its original Git objects. Existing evidence is not
rebound to today's lock. No live trials are authorized by these references.

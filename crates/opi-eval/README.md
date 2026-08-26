# opi-eval

> Unpublished Independent Companion for cross-agent evaluation experiments
> (provisional Phase 18 seam).

[简体中文](README.zh.md) | [opi workspace](../../README.md)

`opi-eval` is an Agent-neutral workspace member for cross-agent evaluation
experiments: it freezes a canonical, digest-addressed experiment contract
(N harness subjects with directed baseline/candidate comparison edges, fully
explicit shared model controls, environment identity, and declared trials)
before any Agent process starts. Resolution is fail-closed; there is no
implicit control default and no fallback.

## Independent Companion boundary

The crate is `publish = false` and depends on no Opi crate in any dependency
table (normal, dev, build, optional, or target-specific). No Opi product
links it, and it registers no provider, tool, package, command, extension,
startup hook, or default capture path in `opi`. Ordinary `opi` runtime
behavior is unchanged by its presence, and existing sessions, native
evidence, local Eval reports, configuration, credentials, and user artifacts
are never read, rewritten, or migrated by this crate.

## Provisional seam

Every type, module, and command here is provisional and unpublished
(P18-SEAM-001): nothing is a durable public promise until the complete
Phase 18 integration matrix proves the seam. Internal traits, envelopes,
modules, and command names may be renamed while the phase is active.

## Usage

```sh
cargo run -p opi-eval -- validate --config crates/opi-eval/tests/fixtures/experiment/minimal.toml
```

`validate` resolves the experiment document and prints a one-line summary
(experiment id, schema, canonical manifest digest, and subject, edge, and
trial counts). An invalid document fails with exit code 1 and a typed
diagnostic on stderr.

Current crate version: `0.8.1`, inherited from the workspace package version.

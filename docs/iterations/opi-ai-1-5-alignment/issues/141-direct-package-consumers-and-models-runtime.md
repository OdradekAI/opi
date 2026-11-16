# #141 Direct package consumers and models-runtime cases

Status: the three package-consumer templates have direct external-consumer
evidence, and the two previously related-only `models-runtime` cases have exact
frozen Pi 1.5.0 dynamic evidence. The historical #139 freeze, #140 binding
receipt, contract index and all prior receipts remain unchanged.

## Package consumers

The new machine-readable receipt is
[phase8-package-consumers-models-runtime-20261116-v1.json](../evidence/phase8-package-consumers-models-runtime-20261116-v1.json).
Its manifest SHA-256 is
`d9405f6e4acef5a131d0d65be4241c9f563b87f8b74f7284a60306419be0b4e8` and its
file SHA-256 is
`71b7e0578596c9909ef63fa003f5c4e9105c8d72b63de472ba9701822851f691`.

- `consumer-http` was executed from the current repository template through
  the actual `opi-ai-0.8.3` archive in ordinary and `caller-union` graphs:
  6/6 checks passed in each profile.
- `consumer-crypto` was executed from the current repository template through
  the actual archive in ordinary and `caller-union` graphs: 6/6 checks passed
  in each profile, including the public TLS/Models/Chat path.
- `consumer-ws` was executed from the current repository template and the
  actual extracted archive in ordinary, `union-default` and `union-c-zlib`
  graphs: 10/10 checks passed in each profile, including the real
  Models-to-Codex Upgrade chain, PMD/window8, tail/Close and active/idle
  cleanup.

The generic WS package runner also passed its private protocol, public Upgrade
and clippy layers in all three graphs, but stopped before its consumer phase at
the packaged `cargo doc --no-deps -D warnings` gate. The retained error is the
existing public-doc link from `tls` to private `node_trust` in `src/tls/mod.rs`.
This gate failure is recorded separately; it does not change the direct
consumer PASS and was not hidden by changing the candidate source.

All three package proofs used the marked shared target
`E:\\opi\\cargo-targets\\opi-72fa37a0fa044958`; the authoritative WS consumer
reruns used an explicit cache lease for each profile. The exact package and
telemetry artifact hashes, consumer template hashes, lock hashes, dependency
graphs, executable hashes, reports and scratch evidence paths are in the
receipt.

## Frozen Pi models-runtime cases

The exact frozen command selected only the two previously unbound cases:

1. `CASE-REVIEW-09`: persists an OAuth refresh that started before model
   refresh cancellation.
2. `CASE-REVIEW-10`: rejects late publication from a superseded
   non-cooperative provider.

Vitest reported one file passed and 2 tests passed (the remaining 41 tests in
the file were intentionally skipped by the name filter). Both cases now have
`exact-upstream-dynamic` status in the new receipt, with the frozen test source
hash and retained output-log hash. This promotes only the upstream test
execution; it does not claim that a Rust implementation is byte-for-byte
equivalent.

## Historical integrity and test impact

- #139 freeze receipt remains manifest
  `9f773dc1268633ab7c225507ff052672feadaae7bb274fe4cd993aafcffff9c7` and
  file `dba9424192863d362b8ebf1447b77e0fc3148eab4d4f6d714ca758df27413eac`.
- #140 binding receipt remains manifest
  `ec6d47f0393cd8d251dae630c804504612add72f285db6e715aa350abde3dd6f` and
  file `a204b9f0f94c9660f53cfe283f26b21a5e71584cac0c8a0956123789ffcf598a`.
- The historical contract-index hash remains
  `c863b3632c2286c9f3bb423c5cb4e906dcc9329ddb58d8cb3df98631546a3aa6`.
- Test impact: `add` the #141 receipt and issue; `update` spec history only;
  `delete` none; `retain` all earlier hashes and scratch evidence.

The next bounded decision is whether to repair the packaged public-doc link
that blocks the generic WS runner, or to record it as a separate documentation
gate gap. The direct WS consumer and both exact `models-runtime` cases are
already verified.

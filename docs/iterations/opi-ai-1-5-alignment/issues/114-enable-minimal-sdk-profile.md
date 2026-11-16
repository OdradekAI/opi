# #114 Enable the minimal SDK profile

Status: complete for the minimum-profile slice; full Pi alignment remains open.

The user requested the next priority after #113. At entry, the no-default profile
only removed the tracing carrier; its ordinary dependency graph still included
network, TLS, cloud authentication and embedded catalog implementations. That
does not satisfy the already approved core plus Faux minimum in
[the acceptance policy](../decisions/10-acceptance-policy.md). Historical package
receipts accurately prove their earlier tracing profiles and remain unchanged.

## Selected scope and acceptance

- Add a default-enabled `builtins` capability group for the existing network
  adapters, built-in authentication and bundled catalog. Preserve the default
  public API, explicit registration and request behavior. Keep
  `telemetry-tracing` independently selectable.
- With defaults disabled, retain core values, Models, custom operation and
  authentication interfaces, Store mutation, retry, schema, incremental
  arguments, neutral telemetry and the existing scripted Mock/Faux route.
  This does not establish complete parity with Pi's separate Faux provider.
- An actual archive consumer must exercise the minimum without the selected
  network/TLS/cloud/compression dependencies, execute the scripted route and
  tool continuation, preserve #113 numeric decoding, and demonstrate that custom
  authentication still reports success only after Store acknowledgement.
- Move pure retry/model policy and Store login coordination to their core
  owners as needed; preserve the default feature's old public paths.
- Verify default and explicit-builtins consumers, update English/Chinese
  feature guidance, and run the check union for affected runtime and scripts.

The source baseline is worktree HEAD
`ee18dd1323a089d536bdc7e293d0b4869777c344`; the frozen Pi reference remains
`f9a26365568127461c47a07a37dcb42c140a3ac5`. Task inputs, the pre-repair dependency
counterexample and raw checks are retained outside the worktree under
`C:/Temp/opi-minimal-profile-20261114/`. All 230 pre-existing JSON evidence
receipts are bound before edits. No live service, remote CI, publication,
staging or commit is part of this task.

## Remaining acceptance priorities

1. Implement the minimum dependency closure and preserve default delivery
   (this issue). A coarse builtins group does not freeze the complete set V of
   independent protocol/auth/catalog feature combinations.
2. Complete the remaining capability, public-entry and caller mapping and
   adapter/observer comparisons; static phase-one mappings are not runtime
   passes. Keep the approved Unicode, non-finite/depth, numeric, cancellation
   and EOF contracts and their original traces.
3. Freeze the applicable feature/platform/toolchain rows and obtain missing
   MSRV and native platform evidence after the configuration boundary is stable.
4. Calibrate resource/fuzz/performance workloads and costs before seeking
   approval of formal numeric budgets. Calibration is not acceptance.

Real Deferred integration remains blocked under [#94](94-record-real-deferred-integration-gap.md).
Neither more scripted calls nor this feature change can close that gap. The
six-gate milestone and current live-service availability remain unproven.

## Delivery and evidence

`builtins` is now a default-enabled group owning the existing protocol adapters,
HTTP/TLS/WebSocket implementations, cloud authentication and embedded catalog
API. `telemetry-tracing` remains independent. Defaults-disabled consumers keep
Models/custom operation APIs, Store/custom OAuth, core utilities and the existing
scripted MockProvider/Deferred fixtures. `Catalog::from_bytes` remains in core;
the bundled data file may still be physically present in the archive. Core still
uses Tokio and HTTP value types; this is not a zero-network-primitive or no_std
claim. Full Pi Faux equivalence and fine-grained backend feature groups are not
closed by this slice.

Pure Smithy state moved to `retry::smithy`, login Store coordination to
`auth::store_oauth`, and pure Bedrock model policy to the model owner. Default
`bedrock::retry` and `auth::oauth` login paths re-export the same definitions.
The default adapter behavior is retained. Existing no-default network consumers
must explicitly enable `builtins`; the bilingual README and Unreleased breaking
entry describe this 0.x migration. Built-in integration targets and three
onboarding examples declare their required features; mixed suites still run
their core assertions in the default profile and are not counted as minimum
proof. The core lifecycle example remains available without builtins.

The [before-repair graph](../evidence/phase5-minimal-before.json) is a workspace
dependency counterexample, not a package pass. The
[minimum archive proof](../evidence/phase5-minimal-package-verified.json) runs
20 tests with no features and 21 with tracing, plus one isolated depth child in
each profile. It verifies both SDK and companion telemetry feature nodes, denies
the selected network/TLS/cloud/compression/product dependencies, checks custom
login ACK/rejection and a scripted tool-result continuation, and confirms the
absence of `Catalog::builtin` through a typed compile-negative probe. Current
raw/preview/strict-final tests and the `1e64` bit pattern remain covered.

The [README archive proof](../evidence/phase5-minimal-readme-package.json) runs
all three exact onboarding sources and the controlled-peer regression in default
and explicit-builtins/no-tracing consumers. The
[Schema archive proof](../evidence/phase5-minimal-schema-package.json) runs all
73 checks per core/default profile. All three use real SDK and companion
telemetry archives, with ordinary third-party registry dependencies; no workspace
source patch substitutes for an artifact. [Payload bindings](../evidence/phase5-minimal-package-bindings.json)
compare every non-generated package file to the candidate workspace. The later
metadata-only guard repair changes one test file, and a final rustdoc repair
changes one OAuth documentation link; the
[final applicability receipt](../evidence/phase5-minimal-source-applicability-v2.json)
records both changes and confirms that all production Rust code excluding doc
comments, manifests and executed consumer inputs still match the package proofs.

The [check receipt](../evidence/phase5-minimal-checks.json) retains exact commands,
logs, feature/platform scope, failures and worktree identities:

- Windows host product workspace: **6512 passed, 0 failed,
  5 ignored**, plus 5 passing isolated child invocations.
- Minimum `opi-ai --all-targets`: **584 passed**, plus its depth child.
- Default workspace, core and builtins/no-tracing Clippy pass with warnings
  denied; workspace doctests, default/minimum rustdoc and formatting pass.
- **112 Python conformance tests** pass; the public CLI rejects evidence
  overwrite before creating a scratch directory.

Earlier failed package attempts remain separate FAIL receipts. They exposed
Clippy subcommand configuration forwarding, a missing copied fixture, moved
rustdoc links, and invalid/wording-sensitive negative probes; their successful
replacements do not relabel those runs. Targeted compiler/import/test repairs
also retain their original logs.

The first broad workspace run stopped at the old dependency-breadth guard:
it mistook `oauth` in a required-feature test target name for a dependency.
The guard now inspects dependency declarations, retains the same dependency
and public-module restrictions, and the complete no-fail-fast rerun passes.
The first failure log is retained separately.

A closing-script quoting error restarted a broad run and replaced its earlier
working success log. The earlier command/exit/digest record is retained in the
scratch, the repeated full run supplies the final raw log, and the original
failure plus all pre-task evidence are unchanged.

All **230** pre-existing JSON receipts retain their bytes. Prior #111–113 package
archives and #113 sealed executables are rehashed. The frozen Pi revision remains
`f9a26365568127461c47a07a37dcb42c140a3ac5`; #113's three paired runs are retained
as historical numeric/preview evidence, not relabeled as a new feature-build
differential. The JSON owner, raw/preview fixtures, strict-final numeric tests,
actual Agent/session regressions and Cargo.lock are unchanged. Those Rust
contracts run again in the current workspace/package profiles. No approved
UTF-16, non-finite/depth, integer, cancellation or EOF difference is revoked.

Test impact: `add` core profile/ACK/tracing tests, minimum archive runner and
adversarial graph/negative-diagnostic controls; `update` feature guards and
README/Schema consumers; `retain` all earlier parser, caller and lifecycle
regressions and their raw evidence; `delete=none`. The iteration issue has no
language mirror; affected published guides were updated in English and Chinese.

Worktree-only delivery: no commit, staging, branch, publication, remote CI or
live-service call. No cache deletion was performed in this slice. The next
priority is the remaining public-entry/caller map and adapter partial/observer
contracts; then freeze the full configuration set and obtain MSRV/native-platform
evidence. Real Deferred remains blocked by #94; numerical budgets remain
unapproved. This is not full six-gate or strict-equivalence acceptance.

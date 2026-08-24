# Phase 17 Remediation Plan

**Status**: READY-FOR-APPLY
**Audit index SHA-256**: `3a431c990871dd9183b31a1376daee59a3e4f2d888b7393dfb208e14a3cdea3f`
**Remediation head**: `23f5754c6e9b1f46ea3151222fc1c1289ae5b64a`
**Disposition artifact**: `remediation.plan.dispositions.jsonl`
**Dirty-worktree baseline**: staged: none; unstaged: deleted `audit.claude.glm53.{findings.jsonl,md,meta.json,requirements.jsonl}`, modified `audit.codex.gpt56.{findings.jsonl,md,meta.json,requirements.jsonl}` and `audit.index.json`, deleted four files under `history/phase17-claude-glm53-890de6b-20260824t081717z/`; untracked: none. These are carried-in changes and are outside remediation ownership.
**Unresolved decisions**: none

## Bound audit set

- Current indexed run: `phase17-codex-gpt56-23f5754-20260824t162222z`
- Current findings digest: `ab88f5d00ef08251abd18aca2b849c6e7e67dc4c7877448fca3a0bc7c8d611c0`
- Strict current finding union: `P17-AUD-001`, `P17-AUD-002`
- Plan scope is limited to those two current findings. Historical audit groups, prior remediation outputs, and evaluation artifacts are not semantic inputs.

## Current finding verification

| Finding | Verification | Final severity | Disposition |
| --- | --- | --- | --- |
| `P17-AUD-001` | Confirmed. The committed registry loop returns the first decided resolver vote. In an isolated archive of the remediation head, a new order-permutation regression observed `Trusted` for `Trust` followed by `Deny`. | Major | Batch `B1`: merge resolver votes by the most restrictive decision. |
| `P17-AUD-002` | Confirmed. The current local `phase17_api_audit` passes the static platform-neutral matrix checks, but that cannot establish Linux and macOS execution for the current head. | Info | `no-action:current-head-three-platform-evidence-requires-post-materialization-ci`; no repository edit can retroactively produce current-head CI evidence. |

## Batch B1 — monotonic project-trust resolver composition

#### Fix P17-AUD-001 — make Deny dominate conflicting resolver votes

- **Source**: `phase17-codex-gpt56-23f5754-20260824t162222z/P17-AUD-001`
- **Closure key**: `project-trust.resolver-deny-dominates`
- **Family key**: `project-trust.authority-composition`
- **Decision**: `fix:merge-resolver-votes-most-restrictive`
- **Change kind**: behavioral
- **Changed paths**: `CHANGELOG.md`; `crates/opi-coding-agent/src/project_trust.rs`; `crates/opi-coding-agent/tests/project_trust_store.rs`
- **Closure predicate**: Within the registered-resolver layer, the effective result is independent of registration order: any `Deny` vote resolves `Untrusted`; otherwise any `Trust` vote resolves `Trusted`; all-`Undecided` falls through to the persistent store/default/prompt layers. The earlier explicit CLI override remains authoritative, and an encountered `Deny` may short-circuit because no later vote can widen it.
- **Red-before**: `cargo test -p opi-coding-agent --test project_trust_store conflicting_resolver_votes_deny_independent_of_registration_order` must fail before production edits; observed failure was `left: Trusted`, `right: Untrusted` for `Trust` followed by `Deny` at the bound remediation head.
- **Green-after**: `cargo test -p opi-coding-agent --test project_trust_store conflicting_resolver_votes_deny_independent_of_registration_order` must pass after the resolver merge is implemented, covering both conflicting registration orders.

Implementation is restricted to the following minimal changes:

1. In `resolve_trust`, retain the explicit CLI override as the earlier layer. In the registered-resolver layer, stop treating `Trust` as a terminal first vote: remember it, continue scanning for a possible `Deny`, return `Untrusted` on `Deny`, and return `Trusted` after the registry is exhausted only when at least one resolver voted `Trust`. Preserve the existing all-`Undecided` fallback.
2. Update the module and API rustdoc that currently promises first-decided-wins semantics so it states the monotonic `Deny > Trust > Undecided` composition contract. Do not add a new trait, feature flag, compatibility shim, or registry abstraction.
3. Replace the test that requires a `Trust` vote to hide all later resolvers with the discriminating two-order regression. Update `explicit_embedder_resolver_precedence_and_cli_empty_registry` so `Trust` followed by `Deny` expects `Untrusted` and records that the later `Deny` was consulted. Retain the separate CLI-override short-circuit and empty-registry coverage.
4. Add one concise entry under the existing `CHANGELOG.md` `Unreleased / Breaking Changes` section because the public 0.x embedder resolver seam deliberately changes conflict semantics. No bilingual counterpart exists for this file.

No normative specification, domain-model document, manifest, dependency, schema, fixture, implementation ledger, or historical snapshot is changed by this batch.

## P17-AUD-002 — bounded no-action disposition

The current-head three-platform evidence advisory is confirmed but is not a repository defect that the apply branch can close. The materialized remediation commit does not exist at plan time, and Linux/macOS/Windows execution evidence can only be produced by CI after that commit is formed and published to the relevant CI context. Apply must therefore:

- make no production or assurance-index edit for `P17-AUD-002`;
- retain final severity `Info`, closure batch `null`, and decision `no-action:current-head-three-platform-evidence-requires-post-materialization-ci` in the result disposition;
- run `cargo test -p opi-coding-agent --test phase17_api_audit` locally to confirm the static matrix contract, while explicitly not representing that result as Linux/macOS execution evidence; and
- avoid claiming Phase 17 conformity or current-head three-platform closure from the remediation result alone.

## Apply sequence and checks

Apply is admitted only when the current `audit.index.json` digest, remediation `HEAD`, approved plan digest, and carried-in dirty baseline still match this plan. Execute the following sequence:

1. Revalidate the audit set and both plan artifacts.
2. Re-run the exact `P17-AUD-001` red-before test against the unmodified bound head and require the same discriminating failure.
3. Implement Batch `B1` only in the three declared paths.
4. Run the focused green test, then the regression and repository checks below.
5. Inventory outgoing changes against the dirty baseline; reject any undeclared path rather than absorbing it.
6. Write the fixed remediation result artifacts only after all required checks pass.

Required verification union:

```text
cargo test -p opi-coding-agent --test project_trust_store conflicting_resolver_votes_deny_independent_of_registration_order
cargo test -p opi-coding-agent --test project_trust_store
cargo test -p opi-coding-agent --test project_trust_startup
cargo test -p opi-coding-agent --test interactive_trust
cargo test -p opi-coding-agent --test non_interactive_trust
cargo test -p opi-coding-agent --test rpc_trust
cargo test -p opi-coding-agent --test phase17_api_audit
cargo clippy -p opi-coding-agent --all-targets -- -D warnings
cargo fmt --check --all
python scripts/opi-doc-check.py
git diff --check
```

Expected results are PASS for every command after Batch `B1`. The `phase17_api_audit` result is limited to its local static assertions. Test impact for apply is `update`: the existing project-trust integration test file is changed to reject the vulnerable conflict behavior; no new test binary is added.

## Apply stop conditions

Stop without production edits if the audit index digest or remediation head changes, the approved combined plan digest does not match, the carried-in worktree overlaps a declared production path, the red-before no longer fails for the stated reason, or implementation requires any path or decision outside this plan. Such a change requires a new `mode=plan` run and a new approval digest.

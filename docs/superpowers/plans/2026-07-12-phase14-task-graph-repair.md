# Phase 14 Task-Graph Repair Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Repair the reviewed Phase 14 design and six-task ledger so every task is executable, source-aligned, correctly classified as product or substrate, and mechanically verifiable before implementation starts.

**Architecture:** Preserve the existing T1 -> 14.1, T2 -> 14.2, and T3 -> 14.3-14.6 implementation decomposition, then add one final documentation/guard task because the new exit criteria require an executable owner and substrate-only 14.6 cannot close them. Keep generic Request scalars independent but make 14.3's Codex-profile evidence follow 14.2, which creates that profile. Correct the normative Phase 14 design first, synchronize both `opi-spec` languages, then atomically reconcile active and draft ledgers without changing existing runtime task status.

**Tech Stack:** Markdown specifications, schema-v2 opi-implement JSON ledger, PowerShell structured JSON serialization, SHA-256 drift guards.

---

### Task 1: Correct the normative Phase 14 design

**Files:**
- Modify: `docs/superpowers/specs/2026-07-11-phase14-provider-auth-design.md`

- [x] **Step 1: Reconcile current types with the design**

  Replace the claim that `ModelCapabilities` is new with an explicit migration of the existing `opi_ai::registry::ModelCapabilities`: make it non-exhaustive, add exact cache capability fields, embed it in `ModelInfo`, and route registry/collection capability queries through that field.

- [x] **Step 2: Pin auth and error behavior**

  Require explicit `AuthDescriptor::StoreCredential` doctor/listing arms, document the `SecretKey` versus `SecretString` boundary, name `CredentialNeeded` and `CredentialRevoked` error behavior, and identify `/login` and `/logout` branches as production call sites created by task 14.2.

- [x] **Step 3: Correct usage and refresh semantics**

  Define `cache_write_1h_tokens` as a subset of cache-write tokens and `reasoning_tokens` as a subset of output tokens; keep `CostBreakdown` copyable and avoid double-counting. Define dynamic model refresh as an error-aware, atomic catalog replacement substrate with no Phase 14 production trigger.

- [x] **Step 4: Add success and exit criteria**

  Add exact criteria covering store/probe behavior, OAuth/live re-resolution, request/session affinity, capability-gated caching, usage/cost accounting, substrate-only refresh, documentation synchronization, non-goals, and mock-only verification.

### Task 2: Synchronize the technical specification

**Files:**
- Modify: `docs/opi-spec.md`
- Modify: `docs/opi-spec.zh.md`

- [x] **Step 1: Correct the Phase 14 summary**

  Replace the concrete-provider `AuthSource` ownership shorthand with `Arc<dyn AuthResolver>` plus coding-agent-owned `AuthSource`, describe migration of the existing capabilities type, state usage subset semantics, and call dynamic refresh a substrate extension point without a production trigger.

- [x] **Step 2: Synchronize the localized counterpart**

  Run: `rg --files docs | rg 'opi-spec.*zh|zh.*opi-spec'`

  Expected: `docs/opi-spec.zh.md` exists and its Phase 14 summary is updated in the same change.

### Task 3: Reconcile the implementation tasks and add the final guard task atomically

**Files:**
- Modify: `.opi-impl-state.json`
- Modify: `.opi-impl-state.draft.json`

- [x] **Step 1: Update task definitions with a structured JSON writer**

  Preserve all runtime fields and the six implementation task IDs. Tighten 14.1/14.2 DoDs; remove the premature `ModelCapabilities` assertion from 14.3; rewrite 14.4 around the existing type migration; correct 14.5 subset/cost semantics and remove its dependency on 14.3; mark 14.6 substrate-only, remove product acceptance ownership, remove its dependency on 14.3, and pin error-aware atomic catalog replacement. Add 14.7 as the sole documentation/alignment task owning README/localized README, help, changelog, non-goal guards, and final Phase 14 source alignment.

- [x] **Step 2: Correct tiers, paths, and test ownership**

  Use paths broad enough for compile-required cross-crate migrations but separate the 14.4 and 14.5 wiring tests. Keep workspace tier where production/session code is actually touched; use library tier only for substrate-only 14.6.

- [x] **Step 3: Recompute spec hashes and record reconciliation provenance**

  Recompute SHA-256 for `docs/opi-spec.md` and the Phase 14 design, update both ledgers, append inference/session notes describing the human-approved repair, and write through `.tmp` plus atomic rename.

### Task 4: Validate the repaired graph

**Files:**
- Verify: `.opi-impl-state.json`
- Verify: `.opi-impl-state.draft.json`

- [x] **Step 1: Run schema and graph checks**

  Check schema version, unique task IDs, dependency existence, acyclicity, tier names, behavioral-test ownership, acceptance scenario completeness, and substrate/product separation.

- [x] **Step 2: Run source and hash checks**

  Confirm every `spec_files_sha256` entry matches the current file and that `opi-spec.md`, the Phase 14 design, the active ledger, and the draft agree on all seven tasks.

- [x] **Step 3: Run documentation and diff checks**

  Run `rg` checks for stale `preserving Copy`, duplicate/new `ModelCapabilities`, product claims for 14.6, and old AuthSource wording. Run `git diff --check` and inspect the final diff without modifying unrelated user changes.

# opi-ai Public README Restructure Implementation Plan

> **For agentic workers:** Execute this plan task-by-task with a verification checkpoint after each task. Do not change Rust runtime behavior or public API while carrying out the documentation migration.

**Goal:** Reorganize the bilingual `opi-ai` package documentation into a Pi-style task-oriented developer manual, moving detailed current contracts into package-local topics and keeping historical implementation records internal.

**Architecture:** Keep `README.md` and `README.zh.md` as the short public onboarding and capability index. Create four bilingual package-local topic references for transport, utilities, provider-specific behavior, and testing, while retaining the existing cost/pricing and Provider/Auth topics. Keep exact signatures in rustdoc and historical evidence under `docs/iterations/opi-ai-1-5-alignment/`.

**Tech Stack:** Markdown, Python `unittest`, `opi-doc-check.py`, Cargo package listing, and the existing offline external-consumer harness.

## Global Constraints

- Follow `docs/superpowers/specs/2026-11-08-opi-ai-readme-restructure-design.md` and `docs/iterations/opi-ai-1-5-alignment/decisions/09-readme-contract.md`.
- Keep English and Chinese manuals structurally parallel and keep the three canonical examples byte-for-byte identical to `crates/opi-ai/examples/` in both README files.
- Do not change Rust APIs, runtime behavior, Cargo dependencies, feature names, provider semantics, test ownership, or the external consumer proof protocol.
- Public manuals may describe current behavior and limits; task IDs, phase IDs, acceptance snapshots, private paths and implementation diary prose stay out of public manuals.
- Preserve every behavior statement that affects caller-visible results, timing, cancellation, errors, resource cleanup or serialization; move it to a topic document when it no longer fits the onboarding flow.
- Never stage or commit `.serena/`.

---

### Task 1: Freeze the target README contract with failing tests

**Files:**
- Modify: `scripts/pi-sdk-conformance/test_run_readme_examples.py`
- Reference: `crates/opi-ai/README.md`
- Reference: `crates/opi-ai/README.zh.md`

**Interfaces:**
- Consumes: existing `ReadmeExecutionAdmissionTests` and the design's section order.
- Produces: deterministic tests that fail against the current mixed README and describe the target public structure.

- [ ] **Step 1: Add the target heading-order test**

Add `test_readmes_follow_pi_style_public_task_order` to read both README files and assert these headings occur in this order:

```python
expected = (
    "Installation",
    "Feature profiles",
    "Quick Start",
    "Streaming and the Final Result",
    "Context, Tools, and Continuation",
    "Providers",
    "Image Support",
    "Classification",
    "Thinking/Reasoning",
    "Error Handling",
    "Telemetry: Explicit Context and Optional Tracing",
    "Custom Providers",
    "Faux Provider for Tests",
    "Deferred Responses",
    "Migration",
    "Development",
)
```

For `README.zh.md`, normalize heading text through a small `headings(source)` helper so the test compares the documented order without relying on line numbers. Use this exact translated sequence:

```python
expected_zh = (
    "安装",
    "Feature profile",
    "快速开始",
    "流式与最终结果",
    "Context、工具与续轮",
    "Provider",
    "图片支持",
    "分类器",
    "思考与推理",
    "错误处理",
    "Telemetry：显式上下文与可选 tracing",
    "自定义 Provider",
    "Faux Provider 测试",
    "Deferred 响应",
    "迁移",
    "开发",
)
```

- [ ] **Step 2: Add the private-section rejection test**

Add `test_public_readmes_do_not_expose_private_implementation_sections` and reject these exact heading fragments in either localized README:

```python
forbidden = (
    "Private protocol ownership",
    "Compression and maintenance boundaries",
    "Private Schema Dialect Engine",
    "Private Upstream Crypto Profile",
    "Scripted Multi-Operation MockProvider Support",
    "私有协议所有权",
    "压缩与维护边界",
    "私有 Schema 方言引擎",
    "私有上游 Crypto Profile",
    "MockProvider 多操作脚本测试支撑",
)
```

Also assert that the README links to `docs/transport.md`, `docs/utilities.md`, `docs/providers.md`, and `docs/testing.md` and their `.zh.md` counterparts.

- [ ] **Step 3: Run the focused tests and verify red**

Run:

```text
python -B -m unittest scripts.pi-sdk-conformance.test_run_readme_examples.ReadmeExecutionAdmissionTests.test_readmes_follow_pi_style_public_task_order -v
python -B -m unittest scripts.pi-sdk-conformance.test_run_readme_examples.ReadmeExecutionAdmissionTests.test_public_readmes_do_not_expose_private_implementation_sections -v
```

Expected: both tests fail because the current README still contains the old implementation-order sections and does not link the four new topics.

- [ ] **Step 4: Commit the contract tests**

```text
git add scripts/pi-sdk-conformance/test_run_readme_examples.py
git diff --cached --check
git commit -m "test(opi-ai): freeze public README structure"
```

---

### Task 2: Create bilingual topic references from current contracts

**Files:**
- Create: `crates/opi-ai/docs/transport.md`
- Create: `crates/opi-ai/docs/transport.zh.md`
- Create: `crates/opi-ai/docs/utilities.md`
- Create: `crates/opi-ai/docs/utilities.zh.md`
- Create: `crates/opi-ai/docs/providers.md`
- Create: `crates/opi-ai/docs/providers.zh.md`
- Create: `crates/opi-ai/docs/testing.md`
- Create: `crates/opi-ai/docs/testing.zh.md`
- Reference: `crates/opi-ai/README.md`, `crates/opi-ai/README.zh.md`
- Reference: `crates/opi-ai/docs/provider-auth.md`, `crates/opi-ai/docs/provider-auth.zh.md`
- Reference: `crates/opi-ai/docs/cost-pricing.md`, `crates/opi-ai/docs/cost-pricing.zh.md`

**Interfaces:**
- Consumes: current public behavior paragraphs and links from the README.
- Produces: package-local topic documents with stable titles, bilingual mirrors and no internal alignment labels.

- [ ] **Step 1: Create the transport topic**

Move the current public transport material into `transport.md` with these headings:

```text
# Transport, Protocols and Resource Lifecycle
## Proxy and Source-Scoped HTTP
## Source-Scoped TLS
## WebSocket Upgrade
## Resource Generation, Pool Cleanup and Awaited Release
## Deferred Transport Semantics
## Public Transport Boundaries
```

Keep only caller-visible configuration, failure, cancellation and cleanup rules. Put private frame/PMD/codec implementation and local protocol-test details in the final `Public Transport Boundaries` paragraph as maintainer links, not as public onboarding instructions. Write the Chinese mirror with the same headings and equivalent rules.

- [ ] **Step 2: Create the utilities topic**

Move current public utility contracts into `utilities.md`:

```text
# Public Utilities and Data Contracts
## Tool-Argument Normalization and Validation
## Canonical Replay Projection
## Context and Transcript Chronology
## Headers and Scoped Environment
## Identity, Overflow and Typed Diagnostics
## JSON Repair and Portable Schema Utilities
## Text/System Rendering
## Whole-Call Retry and Request Callbacks
```

Keep exact public entry points and link to rustdoc for exhaustive signatures. Do not describe private schema-engine or crypto implementation details beyond their public effect. Create the equivalent Chinese mirror.

- [ ] **Step 3: Create the provider topic**

Move the long provider-specific sections from the README into `providers.md`:

```text
# Provider Wire and Operation Reference
## Native Chat Adapters
## Image Operation
## Classification Operation
## Reasoning and Native Options
## OpenAI-Compatible Provider Profiles
## Direct Adapters
## Provider-Specific Limits
```

Reference `provider-auth.md` for registration/auth/catalog and `transport.md` for HTTP/WebSocket mechanics. Create the equivalent Chinese mirror. Keep only current behavior; remove implementation-history wording and private acceptance references.

- [ ] **Step 4: Create the testing topic**

Move maintainer-facing test information into `testing.md`:

```text
# Testing and Package Consumption
## Faux Provider for External Tests
## Controlled Provider and Protocol Fixtures
## Test Layers and Replacement Rules
## Packaged Consumer Proof
## Development Checks
```

Explain how an external user can use Faux and how a maintainer runs package proof. Keep raw evidence file paths as internal links only when they are stable package/repository paths. Create the equivalent Chinese mirror.

- [ ] **Step 5: Add topic-specific link checks**

Extend `test_run_readme_examples.py` so each new topic has a bilingual title, its required public headings, and no forbidden private/phase labels. Reuse `test_public_opi_ai_manuals_omit_internal_alignment_labels` for the new files.

- [ ] **Step 6: Validate the topic batch**

Run:

```text
python -B -m unittest scripts.pi-sdk-conformance.test_run_readme_examples.ReadmeExecutionAdmissionTests.test_public_opi_ai_manuals_omit_internal_alignment_labels -v
python -B scripts/opi-doc-check.py
 git diff --check
```

Expected: PASS for all eight new files and all links they introduce.

- [ ] **Step 7: Commit the topic batch**

```text
git add crates/opi-ai/docs scripts/pi-sdk-conformance/test_run_readme_examples.py
git diff --cached --check
git commit -m "docs(opi-ai): split public topic references"
```

---

### Task 3: Rebuild the bilingual README around the public task order

**Files:**
- Modify: `crates/opi-ai/README.md`
- Modify: `crates/opi-ai/README.zh.md`
- Reference: `crates/opi-ai/examples/quick_start.rs`
- Reference: `crates/opi-ai/examples/quick_start_stream.rs`
- Reference: `crates/opi-ai/examples/tool_loop.rs`
- Modify: `scripts/pi-sdk-conformance/test_run_readme_examples.py`

**Interfaces:**
- Consumes: the four topic documents from Task 2 and the three canonical examples.
- Produces: two task-oriented README files whose public examples remain exact and whose long details link to package-local topics.

- [ ] **Step 1: Retain the onboarding spine**

Keep and reorder the current complete external manifest, feature profile explanation, Quick Start, streaming example and tool-loop example into the exact sequence fixed by Task 1. Do not invent builders or convenience APIs. Keep `Models`, `Request`, `ChatCall`, `TelemetryContext`, `ProviderRegistry`, `AuthContext` and `Faux` names grounded in current Rust source.

- [ ] **Step 2: Condense current capability sections**

Replace duplicated long sections with short contracts and links:

```markdown
## Providers

Use `Models`, `ProviderRegistry` and `register_builtin` to select a model or register an explicit route. The complete provider/auth/catalog rules live in [Provider, Catalog, Auth and Refresh reference](docs/provider-auth.md).

## Provider wire details

Provider-specific image, classification, reasoning and direct-adapter behavior lives in [Provider Wire and Operation Reference](docs/providers.md).

## Transport and resource lifecycle

Proxy, TLS, WebSocket, deferred transport and cleanup rules live in [Transport, Protocols and Resource Lifecycle](docs/transport.md).

## Public utilities

Schema, transcript, replay, retry, identity and callback contracts live in [Public Utilities and Data Contracts](docs/utilities.md).

## Testing and package consumption

Faux usage, test ownership and packaged-consumer proof live in [Testing and Package Consumption](docs/testing.md).
```

Use equivalent Chinese links and wording. Preserve stable anchors with a short redirect sentence when existing users may link to the old headings.

- [ ] **Step 3: Remove implementation-diary content from the main README**

Remove the private protocol, compression, private schema/crypto, MockProvider, module-layout and provider-by-provider implementation chapters from both localized README files after their public behavior has a destination in Tasks 2 and 3. Remove the duplicate `Minimal Example`; the Quick Start is the only canonical first-call example.

- [ ] **Step 4: Preserve useful public contracts**

Keep concise sections for image input, classification, reasoning/stop reasons, error handling, cancellation, telemetry, deferred calls, serialization/migration and non-goals. Each section must state the observable behavior and point to the owning topic or rustdoc for depth.

- [ ] **Step 5: Re-run heading and example contract tests**

Run:

```text
python -B -m unittest scripts.pi-sdk-conformance.test_run_readme_examples.ReadmeExecutionAdmissionTests.test_readmes_follow_pi_style_public_task_order -v
python -B -m unittest scripts.pi-sdk-conformance.test_run_readme_examples.ReadmeExecutionAdmissionTests.test_public_readmes_do_not_expose_private_implementation_sections -v
python -B -m unittest scripts.pi-sdk-conformance.test_run_readme_examples.ReadmeExecutionAdmissionTests.test_public_opi_ai_manuals_omit_internal_alignment_labels -v
```

Expected: PASS, with all three canonical examples still embedded exactly in both READMEs.

- [ ] **Step 6: Commit the README batch**

```text
git add crates/opi-ai/README.md crates/opi-ai/README.zh.md crates/opi-ai/examples scripts/pi-sdk-conformance/test_run_readme_examples.py
git diff --cached --check
git commit -m "docs(opi-ai): restructure public README"
```

---

### Task 4: Update internal implementation records and documentation map

**Files:**
- Create: `docs/iterations/opi-ai-1-5-alignment/issues/35-readme-public-manual-restructure.md`
- Modify: `docs/iterations/opi-ai-1-5-alignment/spec.md`
- Modify: `docs/iterations/opi-ai-1-5-alignment/phase4-readme-package-profiles.md`
- Modify: `docs/README.md` only if the package topic links need a documentation-map entry

**Interfaces:**
- Consumes: final README/topic structure and validation evidence.
- Produces: durable traceability for what moved, why it moved, and which checks prove it.

- [ ] **Step 1: Record the implementation ticket**

Create issue 35 with `Status: done`, parent `spec.md`, scope, source-to-destination mapping, acceptance commands and limits. State explicitly that public README content is capability-oriented while historical task IDs remain internal.

- [ ] **Step 2: Update the phase report**

Add the final section map, topic files, contract test names and packaged consumer result to `phase4-readme-package-profiles.md`. Keep the earlier package-profile evidence intact.

- [ ] **Step 3: Update the spec handoff**

Append the implementation result and issue link to the current README/provider-auth/cost-pricing history in `spec.md`. Do not rewrite the original planning decisions or claim complete Pi alignment.

- [ ] **Step 4: Validate internal links and commit**

Run:

```text
python -B scripts/opi-doc-check.py
git diff --check
git add docs/iterations/opi-ai-1-5-alignment docs/README.md
git diff --cached --check
git commit -m "docs(opi-ai): record README restructure"
```

---

### Task 5: Run the complete documentation and package acceptance

**Files:**
- Reference: all modified public manuals, topics, examples, tests and iteration records.
- Update: `scripts/pi-sdk-conformance/evidence/readme-examples-proof.json` with the final proof output.

**Interfaces:**
- Consumes: the completed documentation migration.
- Produces: a reproducible final proof that package consumers can use the shipped documentation and examples.

- [ ] **Step 1: Run the full conformance suite**

```text
python -B -m unittest discover -s scripts/pi-sdk-conformance -p 'test_*.py' -q
```

Expected: all tests pass, including the heading order, topic links, private-section rejection, historical-label scan and existing provider/auth/cost contracts.

- [ ] **Step 2: Run repository documentation and formatting checks**

```text
python -B scripts/opi-doc-check.py
cargo fmt --all -- --check
git diff --check
```

Expected: all commands exit 0.

- [ ] **Step 3: Verify the packaged file set**

```text
cargo package -p opi-ai --allow-dirty --list | Select-String -Pattern 'README|docs/(cost-pricing|provider-auth|transport|utilities|providers|testing)|examples/(quick_start|tool_loop)'
```

Expected: both README files, all six topic pairs and all three onboarding examples appear in the package list.

- [ ] **Step 4: Run the actual external consumer proof**

```text
python -B scripts/pi-sdk-conformance/run_readme_examples.py --offline --scratch C:/Temp/opi-readme-proof-20261108-restructure --target-dir E:/opi/cargo-targets/opi-72fa37a0fa044958
```

Expected: exit 0, evidence verdict `PASS`, and `manuals_embed_exact_sources` reports `true` for both README files.

- [ ] **Step 5: Review the final diff and commit evidence**

```text
python - <<'PY'
import json
p = 'scripts/pi-sdk-conformance/evidence/readme-examples-proof.json'
d = json.load(open(p, encoding='utf-8'))
assert d['verdict'] == 'PASS'
assert all(d['manuals_embed_exact_sources'].values())
print('packaged README proof: PASS')
PY
git status --short
git diff --stat
```

Use PowerShell's equivalent here-string invocation on Windows. Stage only the intended evidence file and task files; leave `.serena/` untracked. Commit:

```text
git add scripts/pi-sdk-conformance/evidence/readme-examples-proof.json
 git diff --cached --check
git commit -m "test(opi-ai): verify restructured package manuals"
```

The final status may contain only `?? .serena/`. Report the commit hashes, complete check results and any intentionally retained topic content.

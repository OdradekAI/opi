---
title: OPI Knowledge SDK 与持续学习 Worker 架构规格
status: ready-for-agent
labels:
  - ready-for-agent
date: 2026-08-11
language: zh-CN
---

# OPI Knowledge SDK 与持续学习 Worker 架构规格

## Problem Statement

OPI 需要获得长期知识、组织知识镜像与历史经验学习能力，但这些能力不应进入 `opi-agent` 的核心依赖树，也不应把 Agent 主循环变成数据库、同步、密码学或模型驱动学习流程的宿主。

当前需要解决的不是单一的“外挂知识库”问题，而是四个相互关联、又必须保持职责分离的问题：

1. 本地 Agent 需要消费来自公司、Team、个人或其他 Source of Record 的可变知识，并按上游修订保留历史快照、新鲜度与撤回语义。
2. Agent 需要从自己的 Session、Trace、工具结果和外部评价中形成 Episode，跨新旧经历执行 Experience Consolidation，并生成可验证的 Lesson、Experience Rule、Skill 与 EvalCase。
3. 组织知识与个人经验具有不同的所有权、写入权限、恢复方式和生命周期，不能因为都可被检索而压平成同一类 RAG 文档。
4. 该能力应能独立于 OPI 使用，并能通过 SDK、CLI/JSONL、Wiki、Skill 或 Adapter 为其他 Agent 赋能，而不是成为 OPI 专属的内部模块。

如果把所有能力直接加入 OPI 根 workspace，会扩大核心编译、测试、发布和安全范围。如果把 `opi-knowledge` 与 `opi-learning` 设计成两个对等的 OPI Package，则需要处理 Package 间通信、多个进程争夺 Ledger 写权限、重复消费 Agent Event 和跨进程事务协调。当前 OPI Package Interface 并不提供这种分布式知识写入模型，也没有必要为此扩张核心。

系统还必须避免一种更隐蔽的失败：模型总结历史经验后，立即把自己的总结写入生产知识、提高其检索排名，再把“被频繁检索”误判为“已经被外部验证”。这会形成自我确认循环，使派生摘要逐渐冒充独立证据。

因此需要一个独立、Agent-neutral、本地优先的 Knowledge SDK 作为知识记录系统，并将语义总结和学习算法放入一个只能产生提案、不能直接写入 Ledger 的可选 Worker。

## Solution

创建独立仓库 `OdradekAI/opi-knowledge`，将其定位为 Agent-neutral、local-first、版本化、可追溯的知识与经验 SDK。OPI 是第一个官方 Adapter，但 SDK 不依赖 `opi-agent`、`opi-coding-agent` 或任何 OPI Session/Tool 类型。

独立仓库第一版只包含两个主要 crate：

- `opi-knowledge`：Rust SDK、嵌入式 KnowledgeEngine、CLI/JSONL 入口、OPI Adapter、Authority/Personal Ledger、Source Archive、Managed Knowledge Feed、检索、Wiki/Skill 投影、Proposal 校验和唯一提交权限。
- `opi-learning`：可选 Experience Consolidation Worker，负责历史 Episode 选择、Digest、聚类、Experience Pattern、知识协调、抽象压缩、EvalCase 和状态/Activation 建议，但不能打开 SQLite、同步官方知识或直接改变 Ledger。

第一版不创建独立 `opi-knowledge-protocol` crate。Knowledge/Learning 之间的交换契约是必要的，但由 `opi-knowledge` SDK 的 consolidation 领域拥有，并同时发布 JSON Schema。只有出现多个独立 Worker、非 Rust 正式实现或独立兼容周期后，才考虑提取协议 crate。

`opi-knowledge` 可以单独安装和使用。`opi-learning` 缺失、崩溃或版本不兼容时，知识同步、查询、历史快照、手工知识、Wiki 和 Skill Projection 必须继续工作。

OPI 通过现有 process-jsonl Package Interface 调用 `opi-knowledge`。其他 Rust Agent 可以进程内嵌入 SDK；其他语言或 Agent 框架可以调用 CLI/JSONL；只需要被动知识的消费者可以读取 Portable Knowledge Export、Wiki 或 Skill Snapshot。未来可以增加 MCP 或其他 Agent Adapter，但不属于第一版。

## User Stories

1. As an OPI user, I want to install `opi-knowledge` without modifying OPI core, so that I can add long-term knowledge without increasing the default Agent runtime surface.
2. As an OPI user, I want OPI to behave exactly as before when `opi-knowledge` is absent or disabled, so that optional knowledge features never become a runtime prerequisite.
3. As a Rust Agent author, I want to embed KnowledgeEngine as a library, so that my Agent can use the same knowledge model without spawning OPI.
4. As a non-Rust Agent author, I want to query and ingest knowledge through CLI/JSONL, so that I can integrate without Rust FFI.
5. As another Agent framework maintainer, I want to translate my native Session and Trace into neutral `SourceArtifact` and `EpisodeInput` objects, so that the SDK does not require OPI-specific types.
6. As a local user, I want my Personal Ledger to remain local and private by default, so that Agent experience is not automatically uploaded or shared.
7. As a local user, I want to subscribe to an official Managed Knowledge Feed explicitly, so that network access never happens merely because OPI or the SDK starts.
8. As a user working offline, I want the last verified Authority Revision to remain queryable, so that temporary network failure does not destroy local productivity.
9. As a user handling high-risk policies, I want stale or expired Authority knowledge to be clearly identified, so that Personal knowledge is never silently substituted as current official policy.
10. As an organization knowledge publisher, I want signed complete snapshots, stable object IDs, explicit retractions and monotonic revisions, so that local mirrors can verify identity, integrity and continuity.
11. As an auditor, I want every Organization Fact to trace to a verified Authoritative Publication or Authoritative Claim, so that local extraction cannot claim upstream-issued authority.
12. As an auditor, I want Authority Ledger and Personal Ledger histories to remain separate, so that official facts, local interpretations and learned rules cannot be confused.
13. As a knowledge maintainer, I want all projections and indexes to be rebuildable from Ledger and Source Archive, so that caches never become a second truth source.
14. As an Agent user, I want to search concise knowledge first and open supporting evidence only when needed, so that runtime Token usage stays bounded.
15. As an Agent user, I want Search and Open operations to share an immutable `snapshot_ref`, so that knowledge cannot change between discovery and evidence expansion.
16. As an Agent user, I want search results to preserve `official`, `local`, `derived`, `stale` and `conflict` identity, so that the model can reason about provenance instead of seeing a flat RAG list.
17. As an Agent user, I want runtime retrieval to use only eligible, current and appropriately scoped knowledge, so that high semantic similarity cannot bypass permissions, validity or Policy Constraints.
18. As a user who does not enable Learning, I want manual knowledge, official feeds, queries, Wiki and Skill Projection to keep working, so that Learning remains genuinely optional.
19. As a Learning researcher, I want `opi-learning` to receive frozen, reproducible inputs, so that a Consolidation Cycle can be replayed and compared.
20. As a Learning researcher, I want the selection phase to include both new and representative old Episodes, so that the system does not overfit to recent experience.
21. As a Learning researcher, I want successful, failed, conflicting, rare high-severity and reservoir samples represented, so that experience selection is not biased toward frequent successes.
22. As a user, I want historical experiences summarized into Episode Digest and Experience Pattern, so that useful lessons can be retrieved without loading every raw Session.
23. As an auditor, I want summaries to retain source spans and Compression Maps, so that every abstraction can be expanded back to the underlying Episode and Trace.
24. As an auditor, I want summaries and generated Rules excluded from independent evidence counts, so that repeated derivations do not inflate confidence.
25. As a storage-conscious user, I want deterministic Episode Envelopes saved for all Episodes but semantic Digests generated only when needed, so that knowledge growth remains controlled.
26. As a knowledge maintainer, I want Digests referenced by Patterns, Rules, Evaluations or Conflicts to become immutable Anchored Digests, so that later model changes cannot rewrite the reasoning behind prior decisions.
27. As a privacy reviewer, I want sensitive source deletion to invalidate or purge derived Digests as required, so that summaries cannot bypass source-retention policy.
28. As a user, I want new model-generated knowledge to enter only as Draft, so that one Consolidation Cycle cannot generate and self-approve production knowledge.
29. As a reviewer, I want Proposal Assessment separated from Commit, so that low-risk Drafts can be recorded automatically while high-risk transitions wait for review.
30. As a reviewer, I want candidate-level rejection reasons preserved, so that one bad candidate does not erase the rest of a valid Consolidation Cycle.
31. As a user, I want a stale Proposal based on old Ledger revisions rejected instead of silently rebased, so that concurrent knowledge changes are not overwritten.
32. As a safety reviewer, I want current Policy Constraints to block conflicting Experience Rules without deleting their historical evidence, so that policy and experience remain distinguishable.
33. As an Agent user, I want the current task prevented from learning from and reusing its own unfinished output, so that self-referential reinforcement cannot occur inside one run.
34. As a Learning researcher, I want model, prompt, selection policy, random seed, evaluator and budget fingerprints recorded, so that results are attributable and reproducible.
35. As an integrator, I want Learning Worker capability and schema versions checked before a cycle starts, so that incompatibility fails without Ledger changes.
36. As an integrator, I want a crashed or timed-out Learning Worker to leave the last committed knowledge view unchanged, so that semantic processing cannot produce partial writes.
37. As an OPI maintainer, I want OPI compatibility tests to run in the Knowledge repository, so that OPI core does not depend on an optional external SDK.
38. As a Knowledge SDK maintainer, I want `opi-knowledge` and `opi-learning` to version independently from OPI, so that Agent runtime and knowledge schema releases do not force each other.
39. As a third-party Learning author, I want published JSON Schemas for Inventory, Selection, Input and Proposal objects, so that I can implement a compatible Worker in another language later.
40. As a future Adapter author, I want Agent-specific conversion code kept outside KnowledgeEngine, so that MCP, Codex, Claude, Pi or custom Agent integrations can reuse the same Ledger semantics.

## Implementation Decisions

### 1. Repository and release ownership

- `opi-knowledge` MUST be managed in an independent repository rather than added to the OPI root Cargo workspace.
- The independent repository MUST contain both `opi-knowledge` and `opi-learning` during the first protocol-evolution phase.
- Sharing one repository MUST NOT imply that both artifacts are installed together or share a process.
- `opi-knowledge`, `opi-learning`, Knowledge Schema and Learning Exchange Schema MUST have versions independent from OPI.
- The OPI repository MUST NOT depend on `opi-knowledge`; compatibility runs from the Knowledge repository against supported OPI versions or fixtures.
- A separate `opi-learning` repository MAY be created only after its release cadence, maintainership or implementation ecosystem clearly diverges.

### 2. Crate organization and dependency direction

- First version MUST have only two principal crates: `opi-knowledge` and `opi-learning`.
- `opi-knowledge` MUST be both a Rust SDK and the implementation used by the CLI/OPI Adapter binary.
- `opi-learning` MUST depend only on the lightweight domain and consolidation contract surface exported by `opi-knowledge`.
- `opi-knowledge` MUST NOT link to or require the `opi-learning` crate. It discovers and starts a compatible external Worker only when an explicit Consolidation operation is requested.
- `opi-agent` and `opi-coding-agent` MUST NOT depend on either crate.
- Ledger, Feed, retrieval, projection, digest and evaluation concerns SHOULD remain internal modules until at least two real implementations justify new public seams.
- First version MUST NOT create `opi-knowledge-protocol`, `opi-knowledge-sqlite`, `opi-knowledge-feed`, `opi-learning-digest` or similar shallow crates.

### 3. SDK packaging

- The default Rust SDK MUST expose an embedded KnowledgeEngine while keeping OPI integration optional.
- The crate SHOULD use only two primary build profiles: an embedded SDK profile and an Adapter/CLI profile.
- `opi-learning` MUST be able to depend on `opi-knowledge` with storage and Adapter functionality disabled.
- The lightweight surface MUST not pull SQLite, Feed cryptography, OPI Adapter or CLI dependencies into `opi-learning`.
- The public SDK MUST expose use-case operations rather than database repositories. Required operations are query, ingest, sync, assess proposal, commit assessment and build projection.
- The SDK MUST NOT expose raw SQLite connections, unchecked inserts, direct Authority writes, direct state assignment or index writers.
- KnowledgeEngine MUST hide transactions, migrations, Feed state, index maintenance and Source Archive layout behind its public Interface.

### 4. Agent neutrality

- KnowledgeEngine MUST NOT use OPI Agent messages, events, tools, session entries, extension traits or diagnostics in its public domain model.
- Agent-native artifacts MUST enter through neutral `SourceArtifact` and `EpisodeInput` representations.
- A `SourceArtifact` MUST preserve producer identity, media type, content hash, capture time, locator, sensitivity and payload reference.
- An `EpisodeInput` MUST preserve external run identity, Agent kind/version, task, scope, environment, actions, outcome, tool observations, source references, Evidence Spans, redaction state and idempotency identity.
- Missing data from an Agent Adapter MUST remain explicitly unknown; Adapters MUST NOT invent values to satisfy a universal Session schema.
- Knowledge query results MUST be returned as neutral Knowledge Views. Each Agent Adapter translates them into its native Tool Result or context representation.

### 5. Integration modes

- Rust applications MAY embed KnowledgeEngine directly.
- Non-Rust applications and Agent frameworks MUST be able to use CLI/JSONL operations without FFI or a permanent daemon.
- Passive consumers MAY use Portable Knowledge Export, Markdown Wiki or Skill Snapshot.
- OPI MUST integrate through its existing process-jsonl Package Interface.
- First version MUST provide OPI commands for status, explicit sync, ingestion, consolidation and review, plus model-callable Search and Open tools.
- MCP, HTTP service and additional framework-specific Adapters MAY be added later as thin translation layers over the SDK.
- Multiple supported Agent frameworks MUST NOT be interpreted as multi-user collaboration or cloud tenancy.

### 6. Source Archive and dual Ledger ownership

- The system MUST maintain a shared immutable, content-addressed Source Archive.
- `authority.sqlite` MUST contain Authoritative Knowledge Mirror state, verified upstream revision history, freshness, quarantine state and authoritative temporal claims.
- `personal.sqlite` MUST contain Episode, personal Knowledge Item, Experience Rule, local analysis, annotations, proposals and Consolidation records.
- Authority Ledger and Personal Ledger MUST be physically separate in the first version because their writers, recovery semantics and ownership differ.
- Only Sync Engine may advance Authority Ledger.
- Only KnowledgeEngine may commit Personal Ledger transactions.
- Agent runtime, Agent Adapter and `opi-learning` MUST NOT write either database directly.
- Cross-Ledger references MUST preserve source namespace, upstream revision and stable object identity.
- Cross-Ledger atomic transactions MUST NOT be assumed.

### 7. Authority and personal knowledge semantics

- Organization Fact MUST remain the current projection of verified State Claim and Norm Claim from the configured Source of Record.
- Local extraction, interpretation, Experience Pattern and Rule MUST remain `local` or `derived` even when their source Artifact is official.
- Only a structured Authoritative Claim directly issued in a verified upstream publication may automatically enter the official claim layer.
- Local work may create an upstream Revision Proposal but cannot change Organization Fact.
- Personal knowledge MUST never become a temporary official substitute when Authority knowledge is absent, stale or unavailable.
- Policy Norms from Authority Ledger MUST form Policy Constraints that Personal Experience Rules cannot override.
- Authority publication lifecycle and personal Epistemic lifecycle MUST remain separate.

### 8. Managed Knowledge Feed first profile

- First version MUST implement only Minimal Feed Profile.
- A Feed subscription MUST bind a configured source identity, namespace and one pinned Ed25519 public key.
- A Feed revision MUST be a signed complete snapshot, not a delta.
- Manifest identity MUST be derived from the exact signed bytes; reparsed or reserialized JSON is not an equivalent signature input.
- Manifest MUST include schema version, source identity, namespace, sequence, parent revision, publication time, freshness, Artifact declarations, structured Claims and explicit Retractions.
- Every Artifact MUST have stable object identity, size, media type and content hash.
- Previously active objects MUST NOT disappear without an explicit Retraction.
- Each namespace MUST form one linear revision chain guarded by a persistent high-water mark.
- Snapshot activation MUST be idempotent, crash-safe and atomic.
- The following prototype-derived state machine is normative for first-version behavior:

```text
Downloaded
  -> SignatureVerified
  -> ContentVerified
  -> ContinuityVerified
  -> Staged
  -> Active

any failure -> Quarantined
```

- Agent readers MUST observe either the previous complete Active Revision or the next complete Active Revision, never a partial mixture.
- Sync MUST be explicitly invoked. Startup MAY inspect local freshness but MUST NOT contact the network.

### 9. Episode ingestion reliability

- Agent Event delivery MAY be used as a wake-up hint but MUST NOT be the only durable source of Episode ingestion.
- Finalized Session, Trace and persisted artifacts MUST be the authoritative ingestion inputs.
- Ingestion MUST use Session tip, stable entry identity, content hash and cursor/high-water mark for idempotency.
- A later reconciliation scan MUST be able to recover Episodes whose event notification was dropped.
- Unfinalized assistant streams or partial tool results MUST NOT enter an Episode.
- Ingestion MUST run outside the synchronous Agent Turn critical path.
- Raw artifacts containing secrets, private code or personal data MUST be redacted or constrained before they are exposed to a different model provider.

### 10. Learning Worker authority

- `opi-learning` MUST be an optional, on-demand Worker rather than a second OPI Adapter.
- KnowledgeEngine MUST start the Worker only for an explicit Consolidation request and MUST terminate it after the operation.
- First version MUST not require a daemon, dynamic Rust library loading or live package-to-package RPC.
- `opi-learning` MUST NOT open Authority or Personal SQLite databases.
- `opi-learning` MUST NOT synchronize official sources, build runtime indexes, publish Wiki pages, assign upstream-issued identity or commit state transitions.
- `opi-learning` MUST own Episode selection strategy, Episode Digest generation, clustering, Experience Pattern synthesis, Knowledge Reconciliation, abstraction, Compression Map creation, EvalCase generation, evaluation interpretation and transition/Activation recommendations.
- Learning Worker absence or failure MUST affect only Consolidation-related operations.

### 11. Two-stage frozen learning input

- Experience Consolidation MUST use a two-stage exchange: Learning Inventory to Selection Plan, then materialized Consolidation Input to Knowledge Proposal Bundle.
- KnowledgeEngine MUST freeze the relevant Authority and Personal Ledger revisions before producing a Learning Inventory.
- Learning Inventory MUST expose descriptors and budgets, not unrestricted raw Session content.
- KnowledgeEngine decides what the Worker is permitted to see; `opi-learning` decides what is worth processing among permitted candidates.
- Selection Plan MUST record selected Episode and Knowledge IDs, selection reasons, category coverage, estimated budget, random seed and policy fingerprint.
- KnowledgeEngine MUST reject Selection Plans that reference unavailable or unauthorized objects, exceed budget, omit mandatory safety evidence or violate configured coverage constraints.
- Consolidation Input MUST contain immutable versions of selected Episodes, Trace Spans, current Knowledge Items, relevant Authority Claims, Policy Constraints, conflicts, evaluation fixtures and redaction metadata.
- Worker MUST NOT query the live Ledger during a cycle.
- When evidence is insufficient, Worker MAY return a bounded Needs More Evidence result. It MUST NOT begin an unbounded interactive retrieval loop.

### 12. Historical experience summarization

- Experience Consolidation MUST explicitly include historical experience summarization; it is not limited to generating Rules.
- All Episodes MUST have a deterministic Episode Envelope containing stable indexing, outcome, environment, source and redaction metadata.
- Only selected Episodes require a semantic Working Digest.
- Working Digest is a rebuildable derived cache and MUST NOT count as independent evidence.
- A Working Digest MUST become an Anchored Digest when it is cited by an Experience Pattern, Lesson, Rule, Skill, EvalCase, Conflict, promotion decision or human annotation, or when high-severity audit requirements demand preservation.
- Anchored Digest MUST preserve exact content, source spans, input hash, generation fingerprint, anchoring references, System Time and supersession history.
- Experience Pattern MUST summarize shared conditions, strategies, outcomes, success cases, failure cases, exceptions, environmental variants, unresolved conflicts and supporting Episode IDs.
- Compression Map MUST connect every higher-level summary to input Episodes/Knowledge Items and record preserved exceptions, conflicts, omitted details and compression method.
- Consolidation Report MUST record trigger, selection, generated Digests, Patterns, proposals, rejections, unresolved conflicts, model/policy versions and budgets.
- Raw Episode and Evidence MUST remain auditable even when runtime retrieval prefers higher-level summaries.

### 13. Digest retention and invalidation

- Episode Envelope MUST be durable for every ingested Episode.
- Working Digest MAY be evicted when unreferenced and over cache budget.
- Anchored Digest MUST be versioned and MUST NOT be silently regenerated or overwritten when model, prompt or redaction policy changes.
- Digest cache identity MUST incorporate source content hash, digest schema, summarizer fingerprint, prompt fingerprint and redaction profile.
- Corrected sources MUST create new Digest versions rather than mutating prior anchored versions.
- Privacy purge, permission revocation or source invalidation MUST propagate to derived Digests and dependent Patterns/Rules according to policy.
- A summary MUST never be used to bypass access restrictions attached to its source.
- If source removal invalidates support for a promoted Knowledge Item, that item MUST enter revalidation or retirement processing.

### 14. Proposal and commit authority

- Learning output MUST be named Knowledge Proposal Bundle rather than ChangeSet to avoid implying write authority.
- Bundle MUST identify schema, proposal, Consolidation Cycle, producer/model/prompt/policy fingerprints, base snapshots, creation time and idempotency identity.
- Bundle MAY contain Episode Digest proposals, Experience Pattern proposals, new Knowledge Items, revision proposals, relation proposals, Conflict proposals, Compression Maps, evaluation results and state/Activation recommendations.
- Bundle MUST NOT contain raw SQL, direct delete, unconditional promote, direct official marking, Authority overwrite or Evidence purge commands.
- KnowledgeEngine MUST convert a valid Proposal Bundle into an internal Assessment before any write.
- Assessment and Commit MUST be separate operations.
- Structurally invalid, oversized, incompatible or stale-base Bundles MUST be rejected as a whole.
- Within a structurally valid Bundle, candidate-level semantic failures SHOULD be recorded as rejected results while valid candidates are recorded in the same auditable Consolidation transaction.
- No Proposal Bundle may directly create a Promoted item.

### 15. Personal knowledge state ownership

- Every newly generated Knowledge Item MUST enter as Draft.
- Personal Epistemic lifecycle MUST support Draft, Shadow, Trial, Promoted, Retired and Rejected.
- `opi-learning` may recommend transitions and provide evidence/evaluation; KnowledgeEngine alone decides whether a transition is legal and authorized.
- Promotion requires configured evidence, regression protection, no unresolved high-severity blocker and required approval.
- Model confidence or repeated generation MUST NOT satisfy evidence requirements.
- Activation state MUST remain orthogonal to Epistemic state and support Hot, Warm, Cold and Archive.
- Learning may recommend Activation changes; KnowledgeEngine applies only configured deterministic policy.
- Authority Claims MUST NOT pass through Draft/Shadow/Trial/Promoted. Their state derives from verified publication, activation, supersession and retraction.

### 16. Runtime retrieval

- Runtime retrieval MUST be owned entirely by `opi-knowledge`; `opi-learning` MUST NOT participate synchronously in Agent execution.
- First version MUST be Tool-first and provide Search then Open behavior.
- Search MUST return concise candidates, provenance/truth identity, state, conflict information, freshness, revisions and a `snapshot_ref`.
- Open MUST require or accept the originating `snapshot_ref` and expand one stable item/version with evidence, counter-evidence, history and applicable exceptions.
- Permission, Ledger revision, Valid Time, freshness, Policy Constraint, Epistemic state, Scope and environment filters MUST run before semantic or vector ranking.
- Search results MUST preserve official/local/derived/stale/conflict identity.
- Ranking MUST be explainable and versioned. It MAY include relevance, scope, environment, evidence, stability, successful use, Activation, source priority, conflict and freshness factors.
- Learning MUST NOT modify global ranking weights, hard-filter order or Authority priority.
- Retrieval frequency alone MUST NOT increase Stability or evidence strength.
- Current task results MUST become eligible for learning only after finalization, ingestion, later Consolidation, assessment and allowed state transition.
- First version MUST NOT enable default dynamic `transform_context` retrieval.

### 17. Projection and portability

- Full-text, relation adjacency, semantic vectors, Current Knowledge View, Markdown Wiki and Skill Snapshot MUST be rebuildable projections.
- Markdown Wiki MUST not become a second writable truth source.
- Official Compiled Core MUST derive only from Authority Ledger; personal annotations and analysis MUST remain separately identified.
- Projection manifests MUST record Ledger revisions, query rules, template/model fingerprints, permissions and content hashes.
- Portable Knowledge Export MUST be deterministically reproducible from specified Ledger revisions.
- SDK SHOULD support Obsidian-compatible Markdown without depending on Obsidian as the storage engine.
- Skill Snapshot is a task-oriented runtime view, not a replacement for Ledger or Wiki.

### 18. Multi-process and multi-Agent access

- Supporting multiple Agent frameworks MUST mean interoperability, not collaborative multi-user editing.
- Embedded mode SHOULD have one KnowledgeEngine owner per data directory.
- Multi-process mode MUST centralize writes through one `opi-knowledge` process or equivalent single-writer coordination.
- Other Agent processes MUST submit ingest/proposal operations through SDK/CLI/JSONL instead of opening SQLite for writes.
- File locks and SQLite transactions MAY protect mechanical concurrency, but they do not replace semantic single-writer ownership.
- Separate Agent Adapters MUST not maintain independent competing ingestion watermarks for the same native session source.

### 19. Failure semantics

- OPI core MUST continue when Knowledge Package is absent, fails startup or becomes unavailable.
- Query failure SHOULD surface as a structured Tool/SDK error and MUST NOT fabricate an empty “authoritative” answer.
- Worker unavailable, timeout, crash or incompatible schema MUST leave Ledger unchanged except for an auditable failed-cycle diagnostic where possible.
- A stale base snapshot MUST return Stale Base Snapshot and MUST NOT be silently rebased.
- Feed signature, hash, identity, continuity, rollback or undeclared disappearance failures MUST quarantine the candidate revision and retain the last Active Revision.
- Index corruption MUST trigger rebuild or degraded structured retrieval without changing Ledger truth.
- Authority mirror expiry MUST retain history and apply configured warn-or-block behavior based on risk.
- Personal Ledger MUST never be promoted to official status during Authority failure.

### 20. Versioning and compatibility

- OPI version, `opi-knowledge` version, `opi-learning` version, Knowledge Schema and Learning Exchange Schema MUST be independently visible.
- OPI Adapter package metadata MUST declare the supported OPI compatibility range and process protocol.
- Knowledge repository MUST maintain an OPI compatibility matrix.
- Learning Worker MUST expose capabilities, exchange version, supported schema versions, operations and resource limits before a cycle begins.
- Rust Worker MAY use SDK contract types directly; non-Rust Worker MUST be able to use published JSON Schemas.
- Extracting a separate protocol crate is deferred until the exchange has multiple independent implementations or a clearly separate compatibility lifecycle.

## Testing Decisions

### 1. Test philosophy and primary seams

- Tests MUST assert externally observable behavior and invariants rather than private module layout or SQL implementation details.
- The highest and primary test seam is the public KnowledgeEngine SDK Interface. Most domain, persistence, retrieval, state and projection behavior MUST be exercised through it.
- The second real seam is the on-demand Learning Worker exchange because it is a deliberate process and failure boundary.
- The third real seam is the existing OPI process-jsonl Adapter Interface because it is the compatibility point with OPI.
- Internal repository, index and feed modules SHOULD not receive parallel test suites that duplicate behavior already proven at a higher seam.
- Pure algorithms MAY have focused property tests when failure would be difficult to localize through the high seam.

### 2. SDK contract tests

- Verify `opi-knowledge` can be used by a standalone Rust application without OPI dependencies.
- Verify the lightweight contract build used by `opi-learning` excludes storage and Adapter dependencies.
- Verify Agent-neutral `SourceArtifact` and `EpisodeInput` can represent OPI and at least one non-OPI fixture without invented fields.
- Verify query, ingest, sync, assess, commit and project operations through KnowledgeEngine.
- Verify public callers cannot obtain raw database write access or bypass state transitions.
- Verify identical requests against identical Ledger revisions produce semantically equivalent deterministic projections.

### 3. Ledger and recovery tests

- Verify only Sync Engine advances Authority Ledger and only KnowledgeEngine commits Personal Ledger.
- Verify no cross-Ledger atomicity assumption leaks into behavior.
- Verify append-only events, idempotent ingest, duplicate proposal submission and crash recovery.
- Verify incomplete final line, interrupted transaction, WAL recovery, migration rollback and corrupted index handling.
- Verify Authority Ledger can be rebuilt from verified revisions and Source Archive.
- Verify Personal Ledger is not falsely claimed rebuildable from Authority sources.
- Verify Snapshot References keep Search/Open results on the same revision pair.

### 4. Managed Feed tests

- Reuse the existing throwaway Rust prototype scenarios as behavioral prior art.
- Verify first valid snapshot activation, next valid revision and idempotent repeat.
- Verify old revision replay, sequence gap, parent mismatch and high-water rollback rejection.
- Verify bad signature, object tampering and undeclared object disappearance quarantine.
- Verify explicit Retraction updates current view while preserving history.
- Verify crash after staging and restart recovery never expose a partial Active Revision.
- Verify freshness warning and expiry policy without substituting Personal knowledge.
- All default Feed tests MUST be offline and use deterministic fixtures.

### 5. Learning selection and frozen-input tests

- Verify Learning Inventory is fixed to explicit Authority and Personal revisions.
- Verify KnowledgeEngine removes unauthorized, unredacted and non-finalized candidates before Inventory creation.
- Verify Selection Plan includes new and old samples, success and failure, conflicts, reservoir and mandatory high-severity cases.
- Verify random selection is reproducible from seed and policy fingerprint.
- Verify over-budget or unauthorized Selection Plans are rejected before content materialization.
- Verify Worker cannot access live Ledger and cannot observe changes made after snapshot freeze.
- Verify Needs More Evidence cannot create an unbounded query loop.

### 6. Historical summarization tests

- Verify Episode Envelope exists for every ingested Episode without model invocation.
- Verify Working Digest is generated only for selected Episodes and remains a derived cache.
- Verify Digest points to exact source spans and does not count as independent evidence.
- Verify a Digest becomes anchored when referenced by Pattern, Rule, EvalCase, Conflict or promotion decision.
- Verify prompt/model/redaction changes create a distinct Digest identity rather than overwriting an anchored version.
- Verify Experience Pattern includes success, failure, exceptions and unresolved conflicts.
- Verify Compression Map can expand a Rule or Pattern to all underlying Episode/Evidence references.
- Verify source correction, privacy purge and permission revocation propagate invalidation to dependent knowledge.

### 7. Proposal and lifecycle tests

- Verify Learning Worker output cannot directly write Ledger or mark an item official.
- Verify every new generated item begins as Draft regardless of Worker recommendation.
- Verify structurally invalid Bundle rejects completely.
- Verify structurally valid Bundle can record accepted Drafts and candidate-level rejection reasons atomically.
- Verify stale base revisions reject instead of auto-rebase.
- Verify Draft cannot skip directly to Promoted.
- Verify promotion requires evaluation, configured approval and absence of high-severity blockers.
- Verify rollback restores the prior promoted version without deleting the failed version or evidence.
- Verify Authority Claim lifecycle never enters Personal Epistemic states.

### 8. Retrieval tests

- Verify hard filters execute before semantic ranking.
- Verify Draft, Rejected, Retired and non-experimental Trial content are excluded from default production retrieval.
- Verify results retain official/local/derived/stale/conflict identity.
- Verify Search returns concise candidates and Open expands evidence under the same Snapshot Reference.
- Verify ranking explanations identify exact, full-text, relation or optional semantic sources.
- Verify learning recommendations cannot modify global ranking weights or bypass Policy Constraints.
- Verify being retrieved or cited by the model does not increase evidence strength.
- Verify a task cannot retrieve knowledge generated from its own unfinished Session.

### 9. Worker process tests

- Use deterministic fake Model and EvalRunner Adapters for default tests.
- Verify capability handshake and schema compatibility before Plan or Run.
- Verify missing executable, incompatible version, malformed output, timeout, cancellation and crash.
- Verify every failure leaves Ledger and Active Knowledge View unchanged.
- Verify Worker resource, Token, time and retry budgets are enforced.
- Paid or live model calls MUST NOT be required by default CI.

### 10. OPI Adapter compatibility tests

- Follow existing OPI package, process Adapter, Tool, Command, Session and RPC test styles as prior art.
- Verify initialize/capabilities, Search/Open Tool registration and `/knowledge` Command dispatch.
- Verify static Skill or Prompt Fragment discovery when included.
- Verify Adapter failure produces diagnostics and does not modify OPI Session format or core behavior.
- Verify OPI without the Package behaves identically to the supported baseline.
- Verify startup does not trigger network sync or Learning Worker execution.
- Verify the Knowledge repository, not OPI core, owns the compatibility job.

### 11. Security and privacy tests

- Verify prompt injection contained in Session, Tool output or web content remains data rather than Consolidation instruction.
- Verify unredacted Episode cannot be sent to a different external model provider.
- Verify local/derived content cannot obtain upstream-issued identity.
- Verify Policy Constraints block conflicting Experience Rules while retaining history.
- Verify deleted or access-revoked sources cannot remain exposed through Digests, Wiki or Skill Projection.
- Verify secrets and private paths do not leak through diagnostics, manifests or query explanations.

### 12. Long-term and regression tests

- Simulate continued Episode growth, environment changes, contradictions and knowledge supersession.
- Compare learning-enabled treatment with baseline on target success, old-task retention, Token, latency, tool calls and regression rate.
- Verify Activation reduction controls normal retrieval volume without deleting evidence.
- Verify repeated model summaries do not accumulate as independent evidence.
- Verify old representative Episodes continue to enter Consolidation according to policy.
- Verify projection and index rebuilds preserve semantic results across migrations.

## Out of Scope

- Adding `opi-knowledge` or `opi-learning` to the OPI core Cargo workspace.
- Making OPI core depend on the Knowledge SDK.
- Creating a cloud-hosted, multi-tenant or multi-user knowledge service.
- Enterprise approval UI, collaborative editing or distributed Ledger consensus.
- Automatic startup networking, built-in synchronization daemon or file watcher.
- Default dynamic `transform_context` retrieval in the first version.
- Letting Learning Worker participate synchronously in Agent turns or control runtime ranking.
- Letting the current task learn from and reuse its own unfinished outputs.
- Direct Learning Worker access to SQLite or Source of Record synchronization.
- Mandatory vector database or semantic index dependency.
- Base-model training, fine-tuning, parameter merging or recursive self-modification.
- Feed Delta, Root/Release delegated keys, automatic key rotation/revocation and Signed Purge Directive in Minimal Feed Profile.
- MCP, HTTP server, language bindings or framework-specific Adapters beyond the first OPI Adapter.
- A separate `opi-knowledge-protocol` crate in the first version.
- A separate `opi-learning` repository before exchange contracts stabilize.
- Dynamic Rust plugin loading for Learning Worker.
- Treating Session Compaction as long-term Experience Consolidation.
- Treating Wiki, Skill Snapshot, Digest, vector index or generated summary as an independent truth source.
- Treating support for several Agent frameworks as authorization for automatic cross-Agent sharing of private experience.
- Internet crawling and automatic fact extraction from Observed Official Source; that requires a separate monitoring and change-detection specification.
- Embodied intelligence, robotics or direct physical-world learning.

## Further Notes

- This specification supersedes earlier module-placement assumptions in which `opi-learning` was an in-process library/controller owned by OPI. It does not supersede the established dual-Ledger, temporal claim, provenance, Managed Feed or Experience Consolidation domain decisions.
- The formal process name remains **Experience Consolidation**. The design intentionally avoids metaphorical alternative names.
- The most important ownership rule is: `opi-learning` produces semantic compression and proposals; `opi-knowledge` preserves provenance, validates constraints and owns every Ledger commit.
- The most important portability rule is: KnowledgeEngine is Agent-neutral; OPI is an Adapter and first consumer, not the semantic owner of the SDK.
- The most important evidence rule is: Summary, Pattern and Rule may organize evidence but never multiply it. Evidence strength must resolve to underlying Episode, external outcome, human validation, EvalCase or independent Source Artifact.
- The three confirmed test seams are the KnowledgeEngine SDK Interface, the Learning Worker exchange and the existing OPI process Adapter Interface. New lower-level public seams require evidence from a second real implementation.
- Suggested implementation order is SDK domain and Ledger invariants, Agent-neutral ingestion/query, OPI Tool-first Adapter, Minimal Feed Profile, frozen Learning exchange, Digest/Pattern/Proposal flow, then evaluation and promotion automation.
- The existing Managed Feed state-machine prototype remains useful behavioral prior art but is not production cryptography, networking, SQLite or OPI integration.
- Normative domain terms should remain synchronized with the project glossary, especially Source Artifact, Evidence Span, Knowledge Claim, Organization Fact, Source of Record, Authority Ledger, Personal Ledger, Managed Knowledge Feed, Mirror Freshness Policy, Quarantined Revision, Policy Constraint, Wiki Page and Skill Snapshot.
- External project references: [OdradekAI/opi](https://github.com/OdradekAI/opi), [OPI specification](https://github.com/OdradekAI/opi/blob/main/docs/opi-spec.md), and [Pi extension documentation](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/extensions.md).

# #267 继续拆分 Models facade 的实现所有者

Status: complete
Owner: opi-ai Models facade
Parent: [#209](209-maintain-module-test-and-catalog-boundaries.md)
Blocked by: none
Related: [#204](204-extract-models-prepared-call-module.md), [#205](205-extract-models-registration-records.md), [#206](206-extract-models-chat-call-orchestration.md)

## 证据

实施前 `src/models.rs` 有 166,987 字节、3,956 行和约 106 个函数声明。虽然
已经拆出 registration、auth、catalog、operation、prepared call 和 ChatCall 目录，
根文件仍同时包含 public contract、Models facade 的大量 method body、refresh
结果类型和共享编排 helper。结构检查能证明 owner 定义不在 facade，却不能证明
维护者能按职责定位这些实现。

2026-11-22 完成物理拆分。`models.rs` 收敛为 422 行，保留模块索引、公开转出、
`Models` 核心状态和跨 owner 的请求编译/错误映射 seam。构造、合同、registry、
auth、catalog、operation、registration、ChatCall、advanced dispatch 和 prepared
execution 各有唯一 owner 文件。字段与 API 边界表及原始 package consumer 证据见
[`267-models-facade-20261122.json`](../evidence/267-models-facade-20261122.json)。

## 目标和范围

- 将 Models public contract、registry/catalog/auth/operation facade impl 和共享
  error/result 类型放入有明确职责的 `src/models/` 文件。
- 保留 `opi_ai::Models`、`opi_ai::models::*` 和现有调用顺序，不引入兼容 shim。
- 用现有按领域划分的 integration target 验证 owner 边界，避免 facade 重新拥有
  provider wire policy。新增 owner 文件不自动创建独立 test target，只有出现
  可独立复现的行为合同时才增加目标。

## 独立验收

1. `models.rs` 只保留模块索引、公开 re-export、必要的核心状态和跨 owner seam，
   每个迁移方法在唯一 owner 文件中有可定位定义。通过，facade 中没有 owner、
   registration record 或 ChatCall orchestration 定义。
2. 结构报告中的 Models owner 指标保持全量通过，facade owner definition 为零，
   external SDK、custom registration、catalog operations、deferred 和 lifecycle
   测试通过。通过，结构报告 `violations=[]`，受影响 focused tests 为 246/246。
3. minimum、builtins、provider protocol package consumer 和 rustdoc 均通过，
   公开路径与错误分类不变。通过，minimum、bundled catalog、11 个 provider
   protocol external consumers 及 minimum/builtins rustdoc 均通过。

## 字段 owner 与 API 边界

| surface | owner | boundary |
| --- | --- | --- |
| 模块索引、`Models` 状态、跨 owner 编译 seam | `models.rs` | `opi_ai::models::Models` 与 crate-private helper |
| 构造与默认值 | `models/facade.rs` | `Models::new`、`Models::from_registry`、`Default` |
| public contract、error、refresh result | `models/contracts.rs` | `opi_ai::models::*` 与 crate-root re-export |
| registry、lookup、resource lifecycle | `models/registry_owner.rs` | registry/list/resolve/resource methods |
| auth descriptor、resolver、credential operation | `models/auth_owner.rs` | auth/status/login/store methods |
| catalog read、availability、refresh publication | `models/catalog_owner.rs` | catalog/refresh methods |
| classifier、image、deferred operation routing | `models/operation_owner.rs` | operation registration 与 deferred methods |
| typed registration records 与 handles | `models/registration.rs` | `register_*` 与 native handles |
| 普通 ChatCall state machine 与 binding | `models/chat_call.rs` | stream/complete/simple/direct 与 `ChatCall` |
| advanced request preparation | `models/dispatch.rs` | `prepare_call*` |
| prepared attempt execution | `models/prepared_call.rs` | `PreparedProviderCall` 与 `PreparedRoute` |

### 验证

- `cargo check --locked --offline -p opi-ai --no-default-features` 通过。
- `cargo check --locked --offline -p opi-ai --no-default-features --features builtins`
  通过，保留一个未触及的 Bedrock duplicate-pattern warning。
- `cargo test --locked --offline -p opi-ai --no-default-features --features builtins --test core_profile --test external_sdk --test catalog_operations --test catalog_lifecycle --test provider_collection` 通过，246/246。
- `check_architecture_boundaries.py --enforce --json` 通过，`models_lines=422`、
  facade owner/record/orchestration 均为 0、`models_protocol_branches=0`。
- minimum package、bundled catalog package、11 个 provider protocol package
  consumer 通过。
- `all-provider-owners` native feature union 与 11 个 provider profile target
  通过。
- minimum 与 builtins `cargo doc --no-deps` 通过。

Test impact: retain。现有 public facade、provider profile、catalog、deferred、
lifecycle 与 package consumer 断言继续覆盖相同 public contract，没有增加兼容路径。

# #269 明确 shared test support 的稳定边界

Status: complete for the shared-support inventory, selector and three WS archive profiles
Owner: opi-ai integration test support
Parent: [#209](209-maintain-module-test-and-catalog-boundaries.md)
Blocked by: none

## 证据

`tests/support/` 当前有 16 个 Rust helper 和 60 条实际 target consumer edge。
`pending_auth.rs` 被 3 个 target 使用，`catalog_store.rs` 被 5 个 target 使用，
`tls_fixtures.rs` 被 6 个 target 使用，`ws_peer.rs` 被 3 个 target 使用。它们有些
是稳定的本地工具，有些已经携带特定协议或平台策略。selector 对 helper 的修改会
扩大到全部实际消费者，这是正确但不够可读的默认行为。

## 目标和范围

- 为每个 helper 标注稳定工具、协议 fixture 或平台 fixture 的角色与 owner。
- 把只服务一个协议的 helper 移到对应 test family，把真正跨 target 的工具保留在
  `tests/support/`。
- selector 继续按实际 `#[path]` 消费者扩展，不通过复制 helper 或隐藏依赖缩小范围。

## 独立验收

1. helper inventory 能列出每个消费者、layer、cost 和 runner，找不到未归属或
   零消费者 helper。
2. 修改稳定工具与协议 fixture 时，selector 输出分别命中约定的消费者集合，且
   共享 helper 没有生产源码复制。
3. 受影响 target、archive package slice 和负例检查通过。

Test impact: add helper ownership metadata and selector cases; retain existing fixture
behavior and package consumers.

## 实施结果

`tests/support/OWNERS.tsv` 现在覆盖全部 16 个 helper，角色固定为
`stable-tool`、`protocol-fixture` 或 `platform-fixture`，并记录 owner、layer、cost、
runner 和实际消费者。`check_support_inventory.py` 从 Rust `#[path]` 模块图重新推导
消费者，拒绝重复、缺失、过期或零消费者记录，也拒绝生产模块复制 support 路径；`src/**/tests`
中的既有私有测试 include 保留为测试边界。selector 对清单变更给出独立 inventory
runner，并新增全 helper consumer 对照测试。没有改变生产运行时行为或增加兼容行为。

## 首轮验收结果

1. 清单检查通过，16 个 helper、60 条 consumer edge 全部与真实模块图一致。过期
   consumer 和未归属 helper 的负例均按预期失败。
2. selector 测试 22 项通过，所有 helper 都命中清单声明的 target；`ws_peer` 额外
   命中 `lib` 私有 Upgrade 测试，TLS/WS helper 保留 `ws` archive runner。共享 helper
   没有复制到生产源码。
3. 受影响的 `codex_native_options`、`service_tier_cost`、`http_acceptance`、
   `tls_contract`、`ws_upgrade` focused Cargo tests 全部通过。archive `ws` package
   slice 的 private/public test stages 通过，但 clippy 在已有生产迁移代码的四个 lint
  处停止，具体为 Bedrock 重复 pattern、OpenAI Responses 和 PreparedCall 的
  `too_many_arguments`、Models auth owner 的 `needless_borrow`。2026-11-22 首轮
  记录保留这次失败和已通过的测试阶段。

证据见 [`269-shared-test-support-20261122.json`](../evidence/269-shared-test-support-20261122.json)。

## 归档收尾验收

2026-11-23 在当前工作树复现三条严格 Clippy 诊断。`PreparedDispatch` 持有冻结的
provider、已装箱的模型记录和选项。`BackgroundRequest` 持有 Responses Background
的 HTTP 方法、路径及 body。认证后目录筛选移除重复借用。工作树的 162 项具名用例
在修改前后全部通过，另有 38 项覆盖获取重试、Codex 原生选项和最小 SDK。格式、
默认 WS、受影响目标、最小 SDK 及独立 Responses feature 的严格检查均通过。

| 依赖图 | 解包私有协议 | 解包公开 Upgrade | 严格 Clippy | 外部 `consumer-ws` |
| --- | ---: | ---: | --- | ---: |
| `ordinary` | 98 | 16 | 通过 | 10 |
| `union-default` | 98 | 16 | 通过 | 10 |
| `union-c-zlib` | 98 | 16 | 通过 | 10 |

实际 `.crate` 的严格 rustdoc 通过。外部消费者通过 Models → Codex → Upgrade → codec
及清理用例。解包后的 SDK 单独通过 Background 提交、查询和取消的具名回归。归档
源码与工作树逐字节核对，保留 AWS-LC 和 C zlib 的构建来源记录。测试影响为 `retain`
原有具名用例，`update` 三个 fixture 的格式。
[收尾证据](../evidence/269-ws-archive-closeout-20261123.json)

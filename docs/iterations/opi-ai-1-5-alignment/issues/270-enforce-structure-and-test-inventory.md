# #270 把结构与测试边界检查接入日常门禁

Status: complete for the unified read-only gate
Owner: opi-ai conformance scripts
Parent: [#209](209-maintain-module-test-and-catalog-boundaries.md)
Blocked by: none
Related: [#266](266-separate-route-context-contracts.md), [#267](267-split-models-facade-implementation.md), [#268](268-partition-oversized-protocol-fixtures.md), [#269](269-classify-shared-test-support.md)

## 目标

把当前需要人工记住的根文件归属、architecture edge、target inventory 和按路径
选测规则，收敛成一个可重复的只读报告。报告只描述当前源码，不复制进度状态或
替代 Cargo 的 feature 解析。

## 独立验收

1. 单个命令同时检查 Cargo target/source、`targets.tsv`、模块 owner、禁止依赖和
   selector 可命中性，并对缺失、重复、旧路径返回非零。
2. 报告包含稳定的摘要 hash、当前已批准例外和需要扩大检查的路径，不把 skipped
   或 zero-test 当作通过。
3. CI 与本地 README 使用同一入口，生产、测试、脚本和文档变更各自保留最小检查
   集合，完整回归入口仍显式可用。

Test impact: add checker unit and CLI negative-path tests; retain the existing focused,
feature-profile, package and full regression commands.

## 实施结果

新增 `scripts/pi-sdk-conformance/check_structure_gate.py` 作为 architecture、Cargo
target/source inventory、shared-support ownership 和 selector coverage 的统一入口。
它复用四个现有 owner 的检查器，不复制规则，也不把进度状态写入源码。报告固定包含
schema、完整结构摘要、批准的 architecture exceptions、selector 覆盖、zero-test
目标、需要扩大检查的路径和 `summary_sha256`。没有改变生产行为或增加兼容行为。

`select_tests.py` 现在把 `tests/targets.tsv` 和 `tests/support/OWNERS.tsv` 分别路由
到对应 inventory checker。日常 gate 会检查 124 个 target、124 个 source、16 个
shared helper、60 条 helper consumer edge 和 229 个 Rust test source 的 selector
覆盖。`--strict-selection` 用于让改动路径中未登记的 crate 路径直接失败。

CI product job 与中英文测试 README 使用同一个 gate 命令。完整 workspace Cargo
回归仍保持显式入口，不被结构 gate 隐式替代。

## 验收结果

1. `check_structure_gate.py --json` 通过，architecture violations 为零，target、
   support 和 selector 清单全部通过。报告包含稳定摘要 hash、77 条已批准 lower-layer
   exceptions、11 条 route-context edges 和零 zero-test target。
2. `test_check_structure_gate.py` 的 3 项单元和 CLI 负例通过，未登记的
   `crates/opi-ai/tests/missing.rs` 返回非零。`test_select_tests.py` 保留既有覆盖，
   并验证 `targets.tsv` 与 `OWNERS.tsv` 的 checker 路由。
3. `opi-doc-check.py`、target inventory、support inventory、architecture enforce、
   unified gate 和 focused selector checks 通过。CI 已接入相同命令，完整 regression
   command 继续显式保留。

证据见 [`270-structure-gate-20261122.json`](../evidence/270-structure-gate-20261122.json)。

## 结构收尾复验

2026-11-23 复验覆盖 124 个 Cargo target、402 个 Rust 测试源、16 个 helper 和
60 条共享支持边。三个格式化 fixture 路径经 `--strict-selection` 选中完整
`openai_responses_fixtures`。五个变更生产路径报告需要扩大检查，实跑范围由调用点
及工作树、实际归档检查记录给出。
[收尾证据](../evidence/269-ws-archive-closeout-20261123.json)

# #209 Maintain module, test and catalog boundaries after G0–G6

Status: done
Owner: opi-ai structure and alignment follow-up
Parent: [#208 G0–G6 closure audit](208-audit-g0-g6-closure.md)
Blocked by: none
Related: [module structure decision](../decisions/06-module-structure.md), [test boundaries decision](../decisions/08-test-boundaries.md)

## 子票与执行顺序

模块归属、测试边界、静态目录的独立维护、Radius 刷新和 OpenAI Chat 入口已逐项验收。表中阻塞列记录实际前置。每张子票给出独立交付物、验收方法和测试影响。

| 顺序 | 本地 issue | 阻塞 | 独立验收结果 |
| --- | --- | --- | --- |
| 1 · 已验收 | [#210 逐文件审查与三个分类协议改名](210-audit-owners-and-name-classifier-adapters.md) | 无 | 四个范围逐文件归属、三个公开 classifier module、五个目标及归档证明 |
| 2 · 已验收 | [#211 修复单独启用 provider feature 的构建](211-restore-single-provider-feature-builds.md) | 无 | 11 个单独 feature 的库检查与归档消费者通过 |
| 2 · 已验收 | [#215 列清每个集成测试 target 的职责和最小 feature](215-own-integration-test-targets.md) | 无 | 112 个显式目标，19 个补声明目标在 minimum/默认的 432/433 项通过 |
| 3 · 已验收 | [#212 规范 profile 对 API 的依赖](212-guard-profile-to-api-imports.md) | 无 | 18 处直接 API 引用降至 9 处，违规清零，归档消费者验证公开计费符号 |
| 3 · 已验收 | [#216 从改动路径选择有效的测试集合](216-select-tests-by-changed-owner.md) | #215 | 12 个输入、35 个目标与 feature 配置核对命中，单 wire 27 项、共享 helper 158 项实跑 |
| 4 · 已验收 | [#213 给底层语义依赖建立可回归的基线](213-measure-lower-layer-dependencies.md) | #212 | 76 处现存例外、4 处合同、0 处新增依赖，明确 Copilot 等策略归属 |
| 4 · 已验收 | [#217 以 classifier_contract 试行按协议运行测试](217-partition-classifier-contract-tests.md) | #215 | 84 个用例逐名对应，七个分组实跑，拒绝诊断负例验证 |
| 5 · 已验收 | [#214 迁出 lower Provider 中的 Copilot 请求头规则](214-move-copilot-header-policy.md) | #213 | 底层三处定义归零，三种 wire 的实际请求和 21 个拒绝输入通过 |
| 6 · 已验收 | [#218 划定公开 Faux 与私有测试探针](218-separate-faux-and-private-probes.md) | 无 | minimum 归档工具续轮和 deferred fetch 通过，旧 UUID 探针导入被拒 |
| 6 · 已验收 | [#219 清除生产代码和当前指南中的过期来源指引](219-remove-stale-source-guidance.md) | 无 | 当前指南零旧来源命中，三个断链修复，Bedrock reasoning 用例通过 |
| 7 · 已验收 | [#220 将 pack.py 收窄为只读 Pi 比较工具](220-parameterize-catalog-source.md) | 无 | 8 项 CLI 测试与两份固定参考比较通过，44 份 opi 数据和发行文件摘要保持一致 |
| 8 · 已验收 | [#221 建立 opi 自有静态目录](221-reconcile-pi-1-5-catalog.md) | 无 | 1,705 条离线复现，v1 归档回读，自有模型通过 Groq 本地请求，真实归档可消费 |
| 9 · 已验收 | [#222 让 builtin Radius 从已存和远端配置刷新目录](222-refresh-radius-builtin-catalog.md) | 无 | 8 项真实 builtin 端到端验证，离线、替换、空目录、实际请求、取消和竞争通过 |
| 10 · 已验收 | [#223 消除 OpenAI Chat 与同族 API 的命名歧义](223-disambiguate-openai-chat-module.md) | #211 | `api::openai_completions` 归档消费者运行，旧路径被拒，结构和 rustdoc 通过 |

截至 2026-11-20，14/14 张执行票均有本地验收记录。集成测试清单有 113 个显式目标、103 个源文件。协议能力进度分别由协议用例、feature 与平台验证、真实接入证据衡量。Pi 目录差异提供后续候选。#211 记录现存的工作区严格 Clippy 诊断。

## Why this issue exists

The G0–G6 receipt covers API and profile import direction, the lower contract
boundary, the Models owner split, and external consumers of selected feature
unions. This follow-up added file ownership, semantic dependency checks, test
selection, and the maintenance and publication paths for opi's catalogs.

## 拆票时的基线

- #208 的 G0–G6 验证了当时定义的依赖方向、Models 内部所有者和所选 feature 合集。当前 checker 尚未覆盖所有生产符号依赖。#210 已逐项列出文件，#212–#214 分别处理 profile 导入、底层语义依赖和首个明确的策略迁移。
- 测试树有 102 个根 Rust 文件、93 个显式 Cargo test 声明及 19 个自动发现的 target，共 112 个 target。11 个 provider profile target 共用一个源文件。原有 package runner 有按名字指向的回归案例，迁移前须保留这层映射。四层测试职责见[测试边界决定](../decisions/08-test-boundaries.md)。
- 旧注释审计曾命中 `.repo/pi-1.3.0` 和已删除的根路径，#219 从当前树复核余项。静态 catalog 的 manifest 仍标注 1.3.0，`scripts/pi-catalog/pack.py` 还依赖已不存在的 `.repo/pi-1.3.0/nix/model-catalog.json`。#220 负责参考输入的可验证比较，#221 负责产品数据的独立维护与可重建性。

## Pi 1.5.0 capability distance

The current profile inventory is complete at 42/42 provider IDs, and the 14
API IDs present in the catalog match Pi 1.5.0. Pi's wider known-API set also
contains `llama-cpp-classify`, which currently has no builtin catalog entry.
This does not establish catalog parity.

The following baseline compares the existing opi bundle with the generated provider files in the local Pi 1.5.0 snapshot (1,738 entries). The Pi release artifact pinned in the same tree contains 1,712 entries.

| Type | Pi 1.5.0 | opi bundle | Missing in opi | Extra in opi |
| --- | ---: | ---: | ---: | ---: |
| chat | 1,613 | 1,608 | 136 | 131 |
| classifier | 59 | 38 | 26 | 5 |
| image | 66 | 59 | 8 | 1 |
| provider IDs | 42 | 42 | 0 | 0 |
| `(provider,type,id)` intersection | 1,568 / 1,738 | 90.2% identity overlap | 170 | 137 |

These are the pre-migration counts. The current default follows a selected Pi
release one provider at a time, with local extensions preserved through a
three-way comparison. [#226](226-sync-provider-catalog-three-way.md)
Release checks cover reproducibility from
opi data and the behavior of the supported provider routes. Among the 1,568
shared identities, 325 have at least one
metadata difference. The observed fields include base URL, pricing, compat,
token limits, context window, thinking levels, provider aliases, prompt cache,
input capabilities, API identity, display name and reasoning flags.

Builtin Radius now restores and publishes gateway-scoped dynamic catalogs
through the Models refresh and credential owners. The acceptance exercises
the public builtin factory and real Pi Messages dispatch over a loopback
gateway. [#222](222-refresh-radius-builtin-catalog.md)

The remaining capability gates are separate from structure: complete
higher-order feature unions remain open, native macOS evidence is blocked by
the available environment, real Deferred production integration remains open
under #94/#143, and performance/fuzz evaluations remain open.
[#219](219-remove-stale-source-guidance.md) verified the Bedrock reasoning
documentation with stream and replay fixtures. [#218](218-separate-faux-and-private-probes.md)
verified scripted Faux from the minimum archive and moved the UUID state
check into the identity module tests.

## 父票关闭核对

已逐张核对子票的具名验收结果、实际 feature、非零选中用例及外部入口。#208 的 G0–G6 回执记录前一轮结构验证。本轮以 14 张执行票的通过记录关闭。高阶 feature 组合、macOS、性能/fuzz 预算及真实 Deferred 按各自能力票交付。平台见 [#126](126-msrv-and-platform-core-proof.md)，真实接入见 [#143](143-connect-real-deferred-production-route.md)。

Test impact: add Radius builtin 目标；update 目标归属、选测规则和归档消费者；retain 现有合同测试、冻结 fixtures 与 package receipts。前批逐项影响见子票。

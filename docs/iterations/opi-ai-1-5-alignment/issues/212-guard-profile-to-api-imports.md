# #212 规范 profile 对 API 的依赖

Status: done
Owner: providers 与架构检查器
Parent: [#209](209-maintain-module-test-and-catalog-boundaries.md)
Blocked by: none

mapped.rs 经 API 门面取 SimpleOptions，openai.rs 和 openai_responses.rs 经 wire helper 取 ProviderHeaders。openrouter.rs 还向外公开计费类型，collection.rs 负责实际构造适配器。

## 验收

前两类引用改为 provider::operation 与 types::headers。按「文件、符号、用途」记录装配构造和公开计费导出；拒绝未经批准的生产依赖，区分 use/pub use、限定路径、rustdoc 和测试。

## 验证

运行 `test_architecture_boundaries.py` 单测及 `check_architecture_boundaries.py --enforce`。在临时样例中允许装配/指定导出、拒绝普通 profile 读协议专属选项。跑 Cargo targets `api_mapped_provider`、`provider_collection`、`openrouter_cost`、`openai_chat_fixtures`、`openai_responses_fixtures`；包端实际导入 OpenRouter 原公开计费符号。

## 实施与复核 · 2026-11-20

用同一个检查器复核 #211 留下的 crate 和当前源码，`providers/` 下直接引用 API 的生产位置从 18 处降到 9 处。原有 9 处中性类型引用全部改回合同所有者。`SimpleOptions` 在 `collection.rs` 有 4 处、`mapped.rs` 有 2 处，归 `provider::operation`。`ProviderHeaders` 在 collection、OpenAI Chat、OpenAI Responses 各有 1 处，归 `types::headers`。[逐文件、符号、行号与归档回执](../evidence/issue-212-profile-api-edges.json)

| 允许位置 | 用途 | 当前数量 |
| --- | --- | ---: |
| `providers/collection.rs` | 构造 TypeSafe、Decisions、Cloudflare classifier 和 OpenRouter Images adapter | 4 |
| `providers/collection.rs` | 以 `dyn ClassifierApi<…>` 注册 classifier | 2 |
| `providers/openrouter.rs` | 公开转出 `OpenRouterCostDetails`、`OpenRouterUsageAccounting`、`apply_openrouter_reported_cost` | 3 |

`python scripts/pi-sdk-conformance/check_architecture_boundaries.py --enforce --json` 报告 0 处未获准的依赖。临时 profile 中加入分组导入的 OpenAI 专属选项时，同一 CLI 退出 1 并指出文件、行和符号。该门禁检查 Rust 源码中的直接路径，处理 use 树、限定路径、测试配置、嵌套模块和注释。间接转出、宏展开后的完整语义依赖仍需调用链审查。

五个默认配置的 Cargo 目标全部通过：`api_mapped_provider` 17 项、`provider_collection` 111 项、`openrouter_cost` 6 项、`openai_chat_fixtures` 116 项、`openai_responses_fixtures` 84 项。对这五个目标运行普通 Clippy 完成，严格模式仍报告 #211 已记录的三项诊断。

用 `run_provider_protocol_package.py --owner provider-openrouter` 及新的 scratch、evidence-output 和共享 target-dir 打包后，外部 crate 关闭默认 feature，从原公开路径导入三个计费符号并执行 BYOK 费用断言，报告总额 0.75 且带有 provider 标记。

Test impact: add 检查器正反例；update OpenRouter 归档消费断言；retain 五个 Rust 目标及原有业务用例。

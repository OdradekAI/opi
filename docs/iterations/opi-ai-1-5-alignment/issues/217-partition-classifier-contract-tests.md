# #217 以 classifier_contract 试行按协议运行测试

Status: done
Owner: classifier_contract 与测试工具
Parent: [#209](209-maintain-module-test-and-catalog-boundaries.md)
Blocked by: [#215](215-own-integration-test-targets.md)

classifier_contract 在一个根文件里混排 Cloudflare、Decisions、llama.cpp、System One 和跨协议行为。#210 的实跑基线为 84 个用例。

## 验收

同一集成目标下，以协议子模块和独立的跨协议用例组织业务断言；只抽取真正复用的本地 server/fixture 工具。为每个旧 case 建立新测试名映射，更新依赖名称的 runner。删除重复断言需举出同一失败模式的替代用例。

## 验证

运行改前/改后的 `cargo test -p opi-ai --test classifier_contract -- --list` 比对映射；实际执行整 target 和按协议过滤的子集，各子集非空。用 Cloudflare 信封错误或 Decisions 拒绝响应确认语义回归能被测试发现。同步 #215 索引。

## 分组结果 · 2026-11-20

保留单个 `classifier_contract` 集成目标和原有 `builtins` 准入条件。根文件登记协议、公共行为和 fixture 模块。协议载荷、mock 服务与预期结果随各自 wire 维护。classifier 专用的中性输入和多协议失败 fixture 继续在该目标内复用，已有 `tests/support/pending_auth.rs` 仍供其他目标使用。

| Libtest 模块 | 用例数 | 验证范围 |
| --- | ---: | --- |
| `typesafe_system_one` | 23 | 请求、答案、计费、响应获取与超时 |
| `cloudflare_workers_ai_system_one` | 13 | account 路由、run 信封、答案及失败 |
| `llama_cpp_classify` | 16 | tokenization、读出深度、声明顺序与标签概率 |
| `openai_decisions` | 14 | 具名问答、拒绝、图片、价格与失败计费 |
| `models_contract` | 8 | 注册、auth 取消、调用绑定与 telemetry context |
| `cross_protocol` | 6 | 多协议分派、共享 retry 边界、回调及保护头 |
| `declaration_order` | 4 | 声明修改和序列化的枚举顺序 |

Cargo 改前改后各列出 84 个用例，按原用例名逐个映射到完整新名称。84 个测试函数内容均保留，只有一处赋值的换行随 rustfmt 调整。四种 wire、三组公共用例都在关闭默认 feature 并启用 `builtins` 的配置下实跑，默认配置的整个目标也通过。

目标清单已更新。唯一按名字选择该目标用例的现行入口 `select_tests.py` 现用协议模块名和具名跨协议用例。Decisions、Cloudflare、llama.cpp 生产路径的 classifier 部分分别命中 **20、15、19** 项。用改名前后的 libtest 清单转换并实跑，三组均逐名匹配。现有归档验证器独立生成外部消费者。

临时禁用 Decisions 的拒绝类型判定后，`openai_decisions::decisions_refusal_fails_result_but_keeps_billed_usage` 精确运行并在预期诊断上失败。恢复原源码后，同一命令通过，源码摘要与实验前一致。普通 Clippy 完成，仍有前批记录的三项库警告。[84 个名称的映射、分组与回归探针](../evidence/issue-217-classifier-test-modules.json)

Test impact: update classifier 分组、目标清单与 selector 规则；add 模块名失效回归；retain 全部 84 个行为断言与原 public API。

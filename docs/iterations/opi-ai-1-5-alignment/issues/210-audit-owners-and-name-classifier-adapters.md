# #210 Audit source owners and name classifier adapters

Status: done
Owner: opi-ai API and builtin composition
Parent: [#209](209-maintain-module-test-and-catalog-boundaries.md)
Blocked by: none

## Acceptance

Account for each root Rust source and every Rust source below `api/`, `provider/`
and `providers/`. Migrate the three classifier paths in the module declarations,
builtin registration, Rust consumers, package consumer and public migration
guide. Preserve feature gates, wire IDs and request/response behavior.
Exercise every changed Rust integration target and the extracted archive.

## 逐文件归属（2026-11-20）

源码基线 `980b7524c1b52d4bc95fd0a0c56c6e2fd7db36a0` 加本次工作树改动。表内路径相对于 `crates/opi-ai/src`，记录迁移后的路径。范围为根层文件及三个指定目录的递归 Rust 源文件。Pi 对照为 `.pi/pi-1.5.0/packages/ai/src/types.ts` 的 API ID 和 `src/api/{openai-decisions,cloudflare-workers-ai-system-one,llama-cpp-classify}.ts`；快照摘要见 [spec](../spec.md)。Phase 1 的 contract-index 带冻结源码字节定位，当前路径以本表为准。

### 根目录 · 27

| 文件 | 实际职责、归属与决定 |
| --- | --- |
| `api.rs` | 操作入口、ImageApi/ClassifierApi、feature 模块声明。保留；本批改三个声明。 |
| `auth.rs` | 每次请求解析凭据、Store/OAuth 协调。保留根合同；内置实现对 profile 的引用进 #213。 |
| `callbacks.rs` | 请求回调、失败和转换约束。保留调用合同；OpenAI payload 形状验证进 #213。 |
| `cancel.rs` | 取消 token、可取消等待和竞争。保留共用生命周期原语。 |
| `classifier.rs` | 分类调用选项、预检和结果。保留领域入口；解析服务端 usage JSON 的职责进 #213。 |
| `codex_resources.rs` | 按实例登记 generation/lease、按会话清理。保留资源 owner；搬迁前需同时核对 Models 和 Codex transport 的释放边界。 |
| `constrained_sampling.rs` | 严格 schema 投影与 grammar。保留公共语义；Anthropic keyword 准入策略进 #213。 |
| `context.rs` | 消息/token 和 context 占用估计。独立预算语义，保留。 |
| `errors.rs` | Provider 错误、分类和脱敏。保留基础错误入口；底层纯度随 #213 核对。 |
| `execution.rs` | 私有 StartedChatStream 驱动与完成责任。保留，对 stream 与 telemetry 的生产边进 #213。 |
| `http.rs` | 连接池、代理、重定向及有界错误读取。保留 HTTP owner；OpenAI/Zhipu 429 到 stream failure 的映射进 #213。 |
| `identity.rs` | 短 hash、UUIDv7 生成器。保留；测试用的状态操纵归 #218。 |
| `images.rs` | ImagesCall 和 image 请求预检。保留图像操作入口。 |
| `lib.rs` | crate 门面、feature 导出、常用 re-export。保留；doc-hidden test_support 的可见性归 #218。 |
| `models.rs` | Models 注册、读目录、鉴权与调用的公共门面，私有实现已有 models/。保留；此批不以行数判定拆分。 |
| `provider.rs` | 服务身份、Provider trait、Request、attempt 和 stream 类型。保留合同；Copilot wire 规则归 #214，thinking/strict/事件依赖归 #213。 |
| `registry.rs` | provider:model 解析、注册与查找错误。保留注册表 owner。 |
| `replay.rs` | 按身份投影 chronology，关联工具结果并处理有损转换。保留回放领域模块。 |
| `retry.rs` | 重试时限、退避与 Retry-After 解析。保留共用策略；已有 smithy 子模块。 |
| `stream.rs` | 公共事件、ChatStream、结果和失败。保留；检查 feature 门控的单测及事件/driver 依赖（#211、#213）。 |
| `streaming_json.rs` | 工具参数增量预览与快照。保留独立解析合同。 |
| `telemetry.rs` | AI 请求/事件的遥测语义。保留观察入口；与 stream 的类型依赖进 #213。 |
| `test_support.rs` | 公开 MockProvider、脚本化 deferred 与私有探针混合。按真实消费者拆分（#218）。 |
| `time.rs` | 共享毫秒时间戳。单文件私有 helper 足以承载。 |
| `transcript.rs` | 系统消息和工具声明的时间线、规范化。保留；与 provider::Request 的双向耦合进 #213。 |
| `transport.rs` | Codex transport 的私有子模块入口。对子模块内 profile 常量和 Responses 请求投影做生产依赖审计（#213）。 |
| `ws.rs` | HTTP Upgrade 验证及 WebSocket 子模块。保留有独立测试的协议边界。 |

### api/（含子模块） · 25

| 文件 | 实际职责、归属与决定 |
| --- | --- |
| `api/anthropic.rs` | Anthropic Messages 请求与 SSE。保留现有家族入口；profile 在 providers/anthropic.rs。 |
| `api/azure_openai.rs` | Azure Chat 和 Responses 共用部署配置和 wire 路由。保留双协议家族名。 |
| `api/bedrock.rs` | Bedrock Converse 请求、状态和事件映射。保留家族入口。 |
| `api/bedrock/event_stream.rs` | AWS 二进制帧与 CRC。保留在 Bedrock 协议子模块。 |
| `api/cloudflare_workers_ai_system_one.rs` | 由 cloudflare.rs 改名。Workers AI /run 信封与两种回包，委派 System One 核心。 |
| `api/contract.rs` | ChatApi、handle、native options 与低层操作合同的公开入口。保留。 |
| `api/gemini.rs` | Gemini streamGenerateContent 请求及事件映射。保留现有协议家族入口。 |
| `api/llama_cpp_classify.rs` | 由 llama.rs 改名。llama-server 的标签 token、模板和单 token 概率分类。 |
| `api/mistral.rs` | 原生 Mistral Conversations 选项、请求和事件。保留；OpenAI-compatible 工厂归 providers/mistral.rs。 |
| `api/openai.rs` | Chat Completions 的请求与 SSE。同族有多个独立 API，公开路径迁移另立 #223。 |
| `api/openai_codex_responses.rs` | Codex Responses 的 HTTP/WebSocket 调用与生命周期。保留明确的协议名。 |
| `api/openai_codex_responses/native.rs` | 订阅型 Codex 的 native options，由上层协议模块导出。保留。 |
| `api/openai_decisions.rs` | 由 decisions.rs 改名。OpenAI POST /decisions，按问题名解析答案和用量。 |
| `api/openai_responses.rs` | OpenAI Responses 的 chat 与 deferred 操作。保留明确的协议名。 |
| `api/openrouter_images.rs` | OpenRouter 图像生成的请求与结果。保留操作名。 |
| `api/pi_messages.rs` | pi-messages 的请求、流与选项。保留协议名。 |
| `api/shared.rs` | 声明供多个适配器消费的 wire helper。保留；新增共享项须指明调用方。 |
| `api/shared/endpoint.rs` | 组装已解析的 base URL 和路径。保留 wire helper。 |
| `api/shared/env.rs` | 按 scope 惰性取环境值。保留；源码注释的版本和 task 指引归 #219。 |
| `api/shared/headers.rs` | 合并并验证多来源 header、保护路由拥有的字段。值类型已有 types::headers，profile 改从值 owner 导入（#212）。 |
| `api/shared/openai_responses.rs` | 多个 Responses wire 共用的消息转换和事件解码。保留；Codex transport 到此处的引用进 #213。 |
| `api/shared/openrouter_usage.rs` | 多个适配器共用的 OpenRouter 费用字段和入账规则。保留实现；公开转出路径由 #212 验证。 |
| `api/shared/text_decode.rs` | 流式字节增量 UTF-8 解码。保留共享底层实现。 |
| `api/system_one.rs` | TypeSafe 直连、Cloudflare 共用的 System One transport/答案解析，另有分类器共用的重试 helper。保留；全文件冠以 TypeSafe 会把共享逻辑误标为厂商专属。 |
| `api/vertex.rs` | Vertex 选项、请求与响应，复用 Gemini 协议组件。保留家族入口。 |

### provider/ · 2

| 文件 | 实际职责、归属与决定 |
| --- | --- |
| `provider/operation.rs` | 下层 SimpleOptions、deferred 操作和请求/事件/错误合同。保留，供 profile 和 adapter 共同使用。 |
| `provider/routes.rs` | 注入给适配器的路由状态。修复独立 feature 编译（#211）；Azure URL 与 Bedrock endpoint 校验的语义归属进 #213。 |

### providers/ · 14

| 文件 | 实际职责、归属与决定 |
| --- | --- |
| `providers/anthropic.rs` | Anthropic profile、目录与构造。保留。 |
| `providers/azure_openai.rs` | Azure 部署配置和双协议服务构造。保留 profile。 |
| `providers/bedrock.rs` | Bedrock 服务构造、代理凭据与支持的模型族。保留。 |
| `providers/collection.rs` | builtin 装配根，按 API ID 选择适配器并绑定目录和鉴权。本批迁移两个构造路径。 |
| `providers/gemini.rs` | Gemini 服务构造和模型目录。保留。 |
| `providers/mapped.rs` | 一个 provider 身份/目录下按 wire 选择路由。保留组合服务；SimpleOptions 导入归 #212。 |
| `providers/mistral.rs` | Mistral compatible 工厂及 native Conversations profile。保留服务维度。 |
| `providers/mod.rs` | 公开内置模块与静态 profile 元数据表。保留；装配实现已有 collection.rs。 |
| `providers/openai.rs` | OpenAI Chat profile、目录与兼容配置。保留；中性 header 类型归 #212。 |
| `providers/openai_codex.rs` | Codex 的身份与路由工厂。保留；底层引用 profile 常量归 #213。 |
| `providers/openai_responses.rs` | OpenAI Responses profile 和目录。保留；中性 header 类型归 #212。 |
| `providers/openrouter.rs` | OpenRouter 路由、标识 header、catalog 与公开计费导出。保留；#212 对计费导出做包端验证。 |
| `providers/pi_messages.rs` | Radius/pi-messages profile 与缺省目录。保留；builtin 动态刷新归 #222。 |
| `providers/vertex.rs` | Vertex 项目/区域和 express 模式服务构造。保留。 |

## 每项改动的测试

| 改动 | 日常开发先跑 | 本次关闭所用的额外证据 |
| --- | --- | --- |
| OpenAI Decisions | `cargo test -p opi-ai --test classifier_contract decisions_`，检查提交问题、按名读回答、拒绝/计费及 504 策略 | `builtin_auth_ownership` 验证重绑凭据，`catalog_operations` 验证按 provider/API 区分可用性，`provider_collection` 验证真实内置入口 |
| Cloudflare Workers AI System One | `cargo test -p opi-ai --test classifier_contract cloudflare_`，检查 /run 信封、account、两种结果与失败脱敏 | `provider_collection` 验证内置注册、选中的记录与已存凭据；`external_sdk` 经 Models 调用外部适配器 |
| llama.cpp 分类 | `cargo test -p opi-ai --test classifier_contract llama_`，检查 label token、模板、概率与读出深度 | `external_sdk` 使用自定义记录注册并分派；当前 builtin catalog 没有 llama-cpp-classify 条目 |
| 共同的 Rust 公开路径 | 完整 `classifier_contract` 和 `external_sdk`，包含跨协议用例 | 默认 feature 来自工作区。另从真实归档以 `default-features=false, features=["protocol-adapters"]` 编译并运行，按新路径断言三个原始 ID |
| runner、文档和规划 | `test_protocol_adapters_package.py`（含拒绝覆盖已有回执的 CLI 负例）、文档检查、格式和 diff 检查 | 对照实际源树核对下列集合，检查移除的源路径及源码引用 |

## 收敛判定与实跑记录

1. 归属表对根层及三个目录的递归文件集合精确相等，重复、遗漏和多余均为零（27 + 25 + 2 + 14）。可在仓库根运行以下只读复核。

   ```python
   from pathlib import Path
   import re
   src = Path("crates/opi-ai/src")
   files = list(src.glob("*.rs"))
   for name in ("api", "provider", "providers"):
       files.extend((src / name).rglob("*.rs"))
   ticket = next(Path("docs/iterations/opi-ai-1-5-alignment/issues").glob("210-*.md"))
   rows = re.findall(r"^\| `([\w/]+\.rs)` \|", ticket.read_text(encoding="utf-8"), re.M)
   assert len(rows) == len(set(rows)) and set(rows) == {p.relative_to(src).as_posix() for p in files}
   ```

2. 三个旧文件和 Rust 模块声明退出，新旧适配器实现对照只改了两个文件的文档标题。新模块继续使用原 feature 条件。活跃 Rust 源、测试、示例与 runner 中旧 API 路径为零，归档仅带新文件。升级者的 import 对照见 opi-ai 的双语 Provider Wire 指南和 Unreleased。
3. 实际运行以下五个完整 Cargo target，242 项通过，零失败、忽略或过滤。顺序为 `classifier_contract` 84、`builtin_auth_ownership` 9、`catalog_operations` 27、`provider_collection` 111、`external_sdk` 11。

   ```sh
   cargo test --offline --locked -p opi-ai --test classifier_contract --test builtin_auth_ownership --test catalog_operations --test provider_collection --test external_sdk
   ```

4. 实际归档外部消费通过。归档 SHA-256 为 `151de36f516b3d6ee6bfc8aec8dcc5095fef347025ef1b2d1760d863393a75c7`。重跑时传入新的外部 scratch 路径、已标记的共享 Cargo target 和新的 JSON 输出路径。[完整包回执](../evidence/issue-210-classifier-package.json)

   ```sh
   python -B scripts/pi-sdk-conformance/run_protocol_adapters_package.py --scratch <fresh-external-directory> --target-dir <marked-shared-target> --evidence-output <new-receipt.json>
   ```

5. runner 单测 6 项通过（含 CLI 负例）。`cargo fmt --all --check` 和 `check_architecture_boundaries.py --enforce` 通过，后者报告零已定义违规。严格 Clippy 对同一组目标退出 101：`api/openai_responses.rs:699`、`models/prepared_call.rs:200` 的参数个数及 `models.rs:2749` 的 needless borrow。这三个文件与 HEAD 相同；普通 Clippy 跑完 lib 和五个目标，只报告上述三处 Rust lint。Cargo 另提示 11 个 targets 复用同一文件（#215）。
6. `python scripts/opi-doc-check.py`、子票链接/归属集合核对及 `git diff --check` 通过。从独立 feature 的已复现失败进入 [#211](211-restore-single-provider-feature-builds.md)；本批回执覆盖上面的默认与协议聚合配置。父票的目录、测试布局和其余语义依赖各按子票关闭。

Test impact: update 五个 Rust 测试 target 的 import 和实际归档消费者断言；retain 原有 242 项行为断言。没有删除或合并测试用例。

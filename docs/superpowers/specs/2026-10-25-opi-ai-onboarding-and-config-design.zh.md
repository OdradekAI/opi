# Opi SDK 上手、配置与请求参数设计

[English](2026-10-25-opi-ai-onboarding-and-config-design.md) · [验收计划](../plans/2026-10-25-opi-ai-onboarding-and-config-acceptance.zh.md)

## 状态与权威

本文是 **2026-10-25 的待审设计提案**，固定 Opi 提交
`de703bb12a7602d0f128b22a7cbb4bb6c81f12ab`，账本 SHA-256 为
`b9273baa5b5751717f6e4fdea7ab9c7fbd5be11a9e1ff7150c6c04454476592a`。
本文不修改登记规范、不准入任务、不改变任务状态，不授权实施，也不证明文中的
目标 API 已存在。当前 Phase 20 合同仍固定 Pi 1.3.0。Pi 1.4.0 的本地 README
与产品配置源码用于比较和提出字段词汇；新增的 1.4 行为必须另行审查来源和任务图。

本轮只交付本文及对应验收计划。生产代码、当前 README、登记来源、schemas、
Cargo 文件及规范账本均保持不变；不提交、不发布、不调用真实 provider。
无关的 `pi-1.4.0-upgrade-assessment.md` 排除在本轮工作之外。

## 用户已确认的决定

1. 最终 `opi-ai` README 应帮助普通开发者完成接入，达到接近 Pi AI 的上手体验。
2. 应用配置默认使用 TOML；显式选择 JSON 时使用 Pi 的 settings/model 字段。
3. **旧 Opi TOML 字段无需兼容。** 新 TOML 与 JSON 直接使用相同的 Pi 字段名、
   层级和 camelCase；不保留旧别名、旧合并路径或自动转换 reader。
4. Provider 请求行为对齐已采用的 Pi 合同。Rust 可用类型、所有权和错误模型自然表达，
   不能静默改变默认值、优先级、接受值或 wire 行为。

## 现有来源与已观察到的缺口

父级[规范](../../opi-spec.md)拥有架构和权威，登记的
[Phase 20 来源](../../research/pi-ai-opi-ai-alignment/spec-draft.md)拥有当前 SDK 合同。
其中 AIR-045/046 与公开 SDK 表已要求普通直接调用、默认内存 stores、惰性来源和
显式内置组装。[账本](../../../.opi-impl-state.json)已分配 factories、原生选项、
外部包和最终证据责任。

当前 crate README 把最小示例放在内部后端细节之后，示例手工组装 resolver、route
和 attempt，还在构造已移除的 `Request.system/messages/tools` 字段。
当前 [`Request`](../../../crates/opi-ai/src/provider.rs)拥有规范化后的 `context`。
这些是文档漂移的证据，不授权在相应切片就绪前贸然改写整份手册。

产品当前通过 [`config.rs`](../../../crates/opi-coding-agent/src/config.rs) 的
`TomlConfig` 加载 `config.toml`。CLI 已有 `-c, --config <FILE>`，没有配置目录或
独立 models 配置参数。Pi 将偏好放在 `settings.json`，将 provider/model 定义放在
`models.json`；把当前 `OpiConfig` 换成 JSON 编码不能满足字段兼容目标。

## 方案选择

| 方案 | 结论 |
|---|---|
| 默认 TOML、显式 Pi JSON、单一语义 owner | 采用。保留默认发现位置，替换外部词汇，共用解析后的配置、合并和校验。 |
| 仅 JSON | 解析入口更少，但不符合默认 TOML 的决定。 |
| 两种语法继续使用旧 Opi 字段 | 不能提供 Pi 字段兼容，还会保留不需要的迁移负担。 |

工作分成 SDK 上手文档、Reference Product 配置、逐协议请求参数三条线。
可复用验收 fixture，不新增第二 SDK facade 或通用配置框架。

## SDK 上手与 README 收口

普通 API-key 路径应是：加入 crate 和文档列明的运行时依赖，配置 key，选择
provider/model，构造对话 Context，然后 await completion 或消费 stream。
调用者不需要构造 auth resolver、mutation queue、产品配置、Agent 或 prepared-attempt
生命周期；这些仍是高级扩展入口。

`Models::new` 可以继续创建空集合。单 provider factory 或显式内置组装负责目录、
惰性认证和真实 adapter。默认构造不能启动认证或目录网络请求。普通 API-key 使用
不要求自定义 store/presenter；OAuth 持久化与 UI 仍属于显式宿主责任。

最终双语 README 按以下顺序组织：

1. 用途、安装、运行时和平台要求。
2. 一份完整 Quick Start，随后介绍 streaming 与共用的最终结果入口。
3. Provider factories、显式内置集合、模型查询。
4. 默认认证、逐调用覆盖、存储凭据与可选 OAuth。
5. Context、tools、工具结果续调、切换模型和序列化。
6. Native/simple options、错误、取消、usage 和 cost。
7. 高级定制与协议/架构参考链接。
8. 贡献者说明及迁移链接。

Phase 编号、AIR/RDF 表、内部签名/codec 维护及历史验证日志保留在已有设计与证据
归属中。移动事实时保留全部适用条件，不能为了简短删除安全或生命周期合同。
不照搬 Pi 的兼容 shim、浏览器/Node 打包假设，也不让已过时的描述覆盖 Rust 或
当前源码已经明确的差异。

拟议示例为 `crates/opi-ai/examples/quick_start.rs`、`quick_start_stream.rs` 和
`tool_loop.rs`；这些路径 **当前尚不存在**，由后续 owning task 创建。
README 嵌入或提取这些确切源码。必须从实际打包 crate，在新建的 workspace 外
consumer 中使用受控 endpoint 编译、执行同一示例。另一份复杂测试 consumer
不能替代首次接入证明；不由此推断 live 成功、模型质量或性能。

## 产品配置选择与归属

### 文件及显式 JSON 选择

默认发现保留现有位置：Windows `%APPDATA%/opi`、Unix `~/.config/opi`，以及
受信任的项目 `.opi/`。默认主设置文件仍为 `config.toml`，可选的同目录
`models.toml` 承载 Pi 的 `providers` 文档。字段替换不隐含要求同时更名默认文件。

拟将现有 `--config` 扩展为选择设置 **配置束**：

| 选定设置文件 | 同目录的可选配套文件 |
|---|---|
| 默认用户 `config.toml` | `models.toml` |
| 受信项目 `.opi/config.toml` | `.opi/models.toml` |
| 显式 `--config /chosen/config.toml` | `/chosen/models.toml` |
| 显式 `--config /chosen/settings.json` | `/chosen/models.json` |

以上是目标语义，并非当前已可运行的命令。显式 JSON 主文件也可使用其他 basename，
其角色始终是 settings，不能把原始 models 文档当作主设置输入；误用应报错。
现有参数从单文件扩展为读取命名配套文件，是可见行为变更，必须进入 help 和准入审查。

按已知扩展名选择语法，不做内容嗅探；JSON 不自动发现。同一层不合并两种表示，
JSON 失败不回退 TOML。可选配套文件缺失表示空模型层；文件存在但无法读取、畸形或
不支持时，整个选定配置束在发布前失败。显式主文件缺失必须失败。
显式层为 JSON 时，低优先级用户/项目层仍可使用默认 TOML。选择配置束不等于导入
同目录 `auth.json`、extensions、skills、prompts、packages 或指令文件。

默认 user/trusted-project 发现中，settings 文件缺失代表空 settings layer，不能
因此跳过已存在的 model companion。这些选中位置允许仅配置 models，仍受项目
trust 约束；这与显式选择一个不存在的 settings 文件不同。

### 单一配置语义

两种 parser 进入 `opi-coding-agent` 内同一套私有、保留字段存在性的配置表示，
由一个 owner 合并和校验，并保留字段来源用于诊断。`opi-ai` 不增加产品文件
loader，也不依赖 `opi-coding-agent`。

保留当前层级权威：内置默认、用户配置束、受信项目配置束、显式配置束，最后是 CLI
模型覆盖。保留 `OPI_MODEL` 的当前例外：只有未传 `--config` 时才覆盖低层。
这是显式的 Opi 宿主差异，不宣称与 Pi 的优先级相同。
Provider/model 偏好使用 Pi 的分离字段，在合并后解析；CLI/env 模型选择器对该字段对
实施一次原子覆盖。

标量取最高优先级的存在值；已支持的嵌套设置按固定 Pi settings 机制逐字段合并，
逐模型 map 使用精确 provider/model key。数组按字段规则处理，资源列表不能与模型
选择规则混用。兼容清单必须逐项列出已支持数组和 model override 的替换/合并/清空
行为，不准入“所有数组统一拼接”的算法。

### 字段词汇与覆盖边界

| 新外部字段/位置 | 含义与边界 |
|---|---|
| `defaultProvider`、`defaultModel` | Pi 的 provider 和裸模型 ID；替换旧 `model` 偏好字段。 |
| `defaultThinkingLevel`、`modelThinkingLevels`、`thinkingBudgets` | Pi 的字段和 level 词汇；中性策略与各 wire 原生参数分开。 |
| `compaction.*`、`retry.*` | 使用 Pi 词汇，仅支持已有明确准入宿主行为的字段；provider retry 与 agent retry 保持各自 owner。 |
| `models.*: providers.<id>` | Pi 模型文档：`baseUrl`、`api`、`apiKey`、`headers`、`models`、`modelOverrides` 和协议限定的 `compat`。 |
| `models[].contextWindow`、`maxTokens`、`reasoning`、`input`、`cost`、`thinkingLevelMap` | 保留 Pi 元数据的真实单位和可选性，不意味着所有 metadata limit 都自动变成运行时拒绝。 |
| `opi.*` | 没有 Pi 对应项的现有 Opi execution/authority/limit 设置使用一个明确命名空间；保持原行为和归属，不留旧顶层别名。 |

拟议宿主任务应从固定 Pi schema 和真实 loader 生成逐字段兼容清单。当前 Opi 的每项
配置能力都必须映射到真实 Pi 同义字段或类型化 `opi.*` 字段，不能借改格式移除行为。
每个已知 Pi 字段标为已实现、需要明确宿主能力，或有明确诊断的暂不支持。
不能因为 JSON 可以解析就宣称实现全部 Pi tools、MCP、codemode、UI、包管理或设置。

Pi 当前生成的 schema 允许 additional properties。真正未知的扩展数据保留为无行为的
数据，并给出可见的“不生效”警告；不得透传为请求 body 或获得运行时行为。
已知但未实现的 Pi 选项应显式失败。已退役的旧 Opi 字段，包括旧 `model`、
`[defaults]`、`api_key_env` 和旧 thinking 字段，必须给出位置及替换指引，不能静默
忽略后回落默认值。`$schema` 只作本地元数据，不触发远程 schema 获取。

TOML 没有原生 `null`。共同的非 null 输入子集必须得到相同语义；Pi schema 允许的
null 专属配置，例如 `thinkingLevelMap` 中“不支持此级别”的值，使用显式 JSON。
不制造 null sentinel，不把 TOML 字段省略等同于 JSON null。此限制必须作为表示差异
公开，不能称两种格式完全等价。

后续若宿主 UI 保存设置，只写选定的 settings 或 models 文档，保留所选格式和无行为
扩展数据；未请求的情况下不重写配套文件或另一格式。原子保存和权限仍归产品 owner。

### 凭据与项目 trust

本设计及离线测试不读取或执行真实用户配置、HOME、keychain、账号或 `.pi` 扩展。
项目 trust 保持 Opi 当前规则，包含项目指令/资源，不引入 Pi 不同的 context-file
trust 行为。项目仍不能为自己授予持久化 execution permission。trust 前 staging
只读取已授权层，不执行 key command，不加载扩展。

Pi 的 `apiKey`/header 表达式是单独的拟议宿主来源切片：literal、`$NAME`/`${NAME}`
插值、`$$`/`$!` 转义，以及开头的 `!command`。解析时不执行；真实解析属于被选定的
auth/request 来源，保持身份权威和类型化错误。models 值路径须保留来源的逐请求
不缓存行为，不能误用 Pi 另一个 helper 的进程全局缓存。命令执行必须有准入后的
宿主 owner，负责 shell 选择、冻结 cwd/env、取消、进程树清理、来源限定的 timeout
及脱敏诊断；不能套用 AWS credential-process JSON 协议或无关的统一 auth timeout。
该切片通过前，command 表达式明确报未支持，不当作字面秘密发送或静默忽略。
`auth.json` 导入/导出及 native credential backend 替换不在本提案中。

## Provider 请求参数对齐

分开产品 settings/model 文档、SDK 类型化逐调用 options、实际协议请求三层。
`signal`、`fetch` 和函数回调属于运行时值，不是 JSON 配置字段。Rust 可使用
snake_case、枚举和 `Duration`；外部 Pi 文档保持其字段名及规定的毫秒单位。

现有 SDK owner 继续实现自己的 Pi 1.3 合同。参数清单逐项记录来源/版本、native
或 simple 路径、单位、省略时默认、zero/false/null 行为、有效优先级、wire 字段及
请求/失败预期。README 与代码冲突时查源码及 wire tests：Pi 1.4 Azure README
称仅 Responses，实际 provider 仍注册 Responses 和 Chat；不能据此移除已准入的
Azure 双 wire 义务。

| 家族 | 必须按来源验收的内容 |
|---|---|
| Common/simple | temperature、输出限制、中性 reasoning/tool choice、取消、timeout 范围、cache/session；native/simple 默认分别保留。 |
| OpenAI Chat/Responses/Codex | 输出字段、store 省略与 false、reasoning/tool choice、service tier、session/cache/transport、逐变体 retry/callback。 |
| Anthropic | thinking 关闭/启用/省略、来源预算默认、adaptive effort、tool choice、strict 变体；不继承统一预算。 |
| Google/Vertex | 原生动态预算 `-1`、关闭 `0`、level 和签名；Vertex Express/ADC 的 target/auth 分别核验。 |
| Azure | 两个真实 wire，以及 v1/resource/deployment/API-version 映射。 |
| Bedrock | 原生 inference/additional request 字段、SigV4/Bearer、精确 retry modes；已验收认证不等于完整生成验收。 |
| Mistral/Radius 与 typed operations | 各自 option 白名单、timeout、raw events、类型化结果，不强制套用 Chat serializer。 |
| Headers/callbacks | 设置/删除/省略、受保护认证身份、来源时序及逐变体次数，任意 callback 数据不能替换选定账号。 |

有效 options 遵循登记来源的合并规则，保留显式 false/zero/removal。先确定认证给出的
最终 endpoint，再做依赖 endpoint 的兼容推断。通过公开调用抓取真实 method、URL、
headers、body 和 retry；只比较 adapter 之前的对象不够。性能声称另需测量。

Pi 1.4 独有行为，例如 client identity 和空白 tool-result 处理变化，属于来源增量提案。
旧测试只能在 owning contract 已被替代、同等或更强 replacement 通过后退出。
不机械复制 live 模型名更新、退役 frame/global facade 断言或 JS raw-error 泄露预期。
复用已有验收矩阵和测试清单，本文不建立第二交付账本。

## 任务归属与依赖计划

| 处置 | Owner | 补充或保留的责任 |
|---|---|---|
| 保留 | 20.32 / 20.33 | 真实打包目录、动态目录原子 owner。 |
| 补验收 | 20.34 | Factories、canonical IDs、逐 profile 有效 options 到真实请求；不负责宿主文件格式迁移。 |
| 保留协议归属 | 20.45–20.49 | Typed image/classifier/Decisions 参数和请求保留在各自 operation owner。 |
| 保留前置 | 20.50、20.52–20.55 及现有依赖任务 | 本上手计划不豁免 deferred/retry/transport/telemetry 合同；20.54 输入 adapters，20.55 先于 20.56。 |
| 落实现有合同 | 20.35、20.36–20.44、20.51 | 参数清单、simple/native 映射、各真实 adapter、headers/callbacks；保持原前置关系。 |
| 补验收 | 20.56 | 双语用户手册结构、旧示例退出、深层文档链接；不追加新宿主行为。 |
| 补验收 | 20.57 | 实际外部包运行同源 Quick Start，普通 SDK 调用不依赖产品配置。 |
| 保留 | 20.58 | Native/MSRV/features/package 证据支撑发布平台声称。 |
| 补交接 | 20.59 | 最终双语手册和真实证据，保留来源漂移协调及独立 Phase exit 边界。 |
| 新增，未编号/未准入 | Reference Product 配置纵切 | 默认 TOML、显式 JSON 配置束、Pi 字段、旧字段拒绝、共用合并、trust 和真实产品请求。 |
| 新增，未编号/未准入 | Reference Product 配置值来源 | Pi key/header 表达式与自有 command 生命周期；消费配置纵切，不迁移 auth 文件。 |
| 采用前审查来源 | Pi 1.4 增量 | 精确新增字段/行为及相关测试，不改贴旧 passing 证据。 |

SDK 线按原有依赖通过参数机制、adapters、20.34、20.56、20.57、20.58、20.59，最后
独立 Phase exit。宿主配置线消费已准入 factories/options，独立验收配置到请求；
不能反过来成为 SDK 独立使用的前提。任务已有某路径不代表 DoD 自动扩展。

实施前，将获批含义写入 owning registered sources，审查来源增量和任务图，并通过
显式调用的 `opi-implement plan` 完成既有人类图确认。在该协调获准前，保留当前任务
ID、原证据和来源 pins；不单独修 hash，也不预占新 Phase 20 编号。
20.59 仍仅交接材料，F.1a 独立判断完整退出。

## 本提案的交付标准

对应验收计划规定具体正反例和未来命令。本次规划交付要求：中英对应、引用可核对、
没有未解决占位或隐藏实施声称、doc checker 和 diff 空白检查通过。
计划中的 runtime/package/platform 命令属于未来验收，不算本轮执行证据。
本次规划的测试影响为 `none`。

## 本地比较来源身份

以下 SHA-256 固定相关本地 Pi 1.4.0 比较输入，不代表上游 Git revision，也不是
新的 Phase 20 登记基线。路径相对于 `.repo/pi-1.4.0/`。

| 来源 | SHA-256 |
|---|---|
| `packages/ai/README.md` | `a3c2b8de70b396187b6e12ed679624ba1c3c1c9c028e90a9aad69cd46dd84e8f` |
| `packages/ai/src/types.ts` | `9f7346dfd52bda4f25e527d768c67a210c88fb9dc9c4a16610b693087e626319` |
| `packages/coding-agent/schemas/settings.schema.json` | `d4b19b0e492cb887a23c50034f00213c476048956fa92c5cce2f1536ef50b9b0` |
| `packages/coding-agent/schemas/models.schema.json` | `86ade9716724902bdfe91347a728980ad7242a76d0773dbc7217ce4584d696ea` |
| `packages/coding-agent/src/core/settings-manager.ts` | `aef8fea51000f291c1c51d81fd9b30f7a2bdd0e68370ad84b969e5fd15667e98` |
| `packages/coding-agent/src/core/model-registry.ts` | `f4ced2d92df9097647aba7da3ae0bb273fa8089e966be15dba880db51d5dee0b` |
| `packages/coding-agent/src/core/resolve-config-value.ts` | `0f53dad47fe5d5d8837c022b7951ccd3bd5a9b577bd662f0986272110e83bcc7` |

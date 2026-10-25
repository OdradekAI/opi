# Opi 上手体验与配置：设计验收计划

> **面向执行代理：** 来源和任务图准入后，使用 `superpowers:subagent-driven-development` 或 `superpowers:executing-plans` 推进已准入工作。本文是验收提案，不是实施准入。本次用户仅允许规划，不允许修改生产代码、规范或账本，也不允许提交。

[English](2026-10-25-opi-ai-onboarding-and-config-acceptance.md) · [设计](../specs/2026-10-25-opi-ai-onboarding-and-config-design.zh.md)

**目标：** 让开发者能够按手册独立使用 SDK；Reference Product 默认 TOML、显式选择 Pi JSON；保持已采用的 provider 请求语义。

**架构：** SDK factories 和 adapters 拥有 provider 行为；产品拥有一个保留字段存在性的配置 resolver，接收两种输入语法。文档与实际 Cargo 包的外部消费者使用同一份示例源码完成验证。

**技术：** 现有 Rust workspace、Serde/TOML/JSON、Tokio、受控本地 HTTP 服务、fake clock、隔离配置根目录及现有 Cargo cache/package-proof 工作流。本文不批准新增依赖。

## 全局约束与证据状态

- 基线：Opi `de703bb12a7602d0f128b22a7cbb4bb6c81f12ab`；账本 SHA-256 `b9273baa5b5751717f6e4fdea7ab9c7fbd5be11a9e1ff7150c6c04454476592a`。
- 当前 Phase 20 来源仍为 Pi 1.3.0。Pi 1.4.0 提供拟采用的配置词汇与比较证据；行为增量需要明确采用。
- 旧 TOML 外部字段允许破坏性更换。直接采用 Pi 名称，不保留旧别名、双语义模型或自动旧格式读取器。
- 保留 Opi 执行权威、项目信任、凭据后端及 SDK/产品依赖方向。现有能力必须通过 Pi 等价字段或明确的 Opi 扩展保留。
- 不使用真实账号、浏览器授权、云 metadata 或收费 provider。测试采用合成 key、隔离 HOME/env/配置根目录；环境修改串行化。
- 不增加统一超时/重试策略、推测性公共 trait，不导入整套 Pi runtime，不用功能测试推断性能，不发布包。
- 只有显式调用的 `opi-implement` 可协调 `.opi-impl-state.json`。本文不分配新任务 ID，不建立平行进度账本。
- 下列 case 名仅标识验收要求，不代表任务状态。所有 runtime 命令都是**计划执行，本次规划未执行**。

## 实施前的文件归属

路径均相对于仓库根目录。既有文件是准入后应检查和修改的 owner；列在此处并不授予路径权限。

| 工作线 | 既有 owner | 拟新增内容及边界 |
|---|---|---|
| SDK 上手 | `crates/opi-ai/README.md`、`README.zh.md`、`src/models.rs`、`src/provider.rs`；根 `README.md` / `README.zh.md` 的产品配置说明 | 将来的 `crates/opi-ai/examples/quick_start.rs`、`quick_start_stream.rs`、`tool_loop.rs` 在本基线不存在。API 修改仍归 factories/options 任务。 |
| 配置选择与解析 | `crates/opi-coding-agent/src/config.rs`、`src/cli.rs`、`src/main.rs`，既有 startup/provider-factory 消费者和测试 | 必要时从 `config.rs` 拆出私有语法解析模块，不增加 SDK loader。将来的产品 settings/model schemas 和双语配置指南由准入任务明确命名；目前没有产品 schema 目录。 |
| 配置值解析 | 产品 auth/header 来源接线，既有凭据和进程生命周期边界 | 独立宿主任务拥有命令解析与取消，不复制 AWS credential-process，不改变 auth-store 归属。 |
| Provider 参数 | `crates/opi-ai/src/models.rs`、实际 provider 模块、`tests/request_enrichment.rs`、协议 fixture/lifecycle 测试，以及产品 factory/config 测试 | 扩充既有参数清单和带来源的 fixtures，不增加任意请求体透传。 |
| 打包与交付 | `scripts/pi-sdk-conformance/`、既有打包证据、`CHANGELOG.md` | 将来的 `tests/external_sdk.rs`、`catalog_artifact.rs`、`platform_delivery.rs` 是任务登记目标，不是已有 passing 证据。扩展既有 harness，不再造打包框架。 |

## 依赖顺序与正式任务归属

| 顺序 / 工作线 | 现有任务 owner | 应补充的计划或验收 |
|---|---|---|
| 来源协调 | 既有 Phase 20 登记来源及图准入 | 审查获批增量，保留原证据和 pins，完成既有人类图确认，不能只替换来源 hash。 |
| SDK 机制 | 20.35、20.51 及其既有前置 | 完成 simple/native 参数清单和 headers/callbacks 语义。 |
| SDK adapters | 20.36–20.44 及其他登记前置 | 各 native 协议的真实请求/响应证明；auth 证据不能代替 generation 收口。 |
| Typed operations | 20.45–20.49 | 各自 image/classifier/Decisions 参数和 wire 合同，不用仅覆盖 Chat 的清单替代。 |
| SDK 组装 | 20.32 / 20.33 → 20.34，保留账本完整依赖 | 打包 catalog、动态 catalog 和 factory/profile 集成；向 20.34 补有效参数的公共请求场景，不添加宿主 parser 归属。 |
| 宿主配置独立线 | 新 Reference Product 任务，未编号、未准入 | bundle 发现、TOML/JSON 的 Pi 字段、字段清单、merge/trust/诊断、配置到请求证明；消费已准入 SDK factories/options。 |
| 宿主值来源 | 新 Reference Product 任务，未编号、未准入 | literal/env/转义/命令经真实 auth/headers 接线，拥有进程生命周期；消费宿主 resolver。 |
| SDK 手册与退役 | 20.56 | README 结构、准确示例、旧公共符号退出和迁移文档；文件归属本身不准入宿主迁移。 |
| 外部可用性 | 20.57 | 实际 `.crate` 的普通外部消费者运行同源 quick start，落实 V-39 和 SDK 独立使用。 |
| 平台与打包声称 | 20.58 | Native/MSRV 1.97、features、实际依赖图与支持平台证据。 |
| 文档收口 | 20.59 | 双语手册、带来源证据和剩余缺口；保留 P.0/P.2/P.4 漂移协调及人类确认。 |
| Phase exit | 独立 F.1a | 判断完整已准入任务图。20.59 和本提案均不宣布 Phase exit。 |

SDK 上手不依赖产品 TOML/JSON。宿主线不得成为独立 SDK 的前置。不能因字段改名重开 passing auth 任务；真实消费者/来源变化使证据失效时，再运行受影响回归。

本表是有限范围依赖视图，不替换任务图。保留 20.54 先于真实 adapters、20.55 先于 20.56，以及 20.50/20.52/20.53 子任务先于各自登记消费者的全部既有依赖；其 deferred/retry/transport/telemetry 义务继续生效，不在此新增范围。

## 验收包 A：来源、字段清单与准入

**输入：** 设计、Phase 20 登记来源、当前 Opi 配置类型/消费者、固定本地 Pi schema/loader 来源。**输出：** 用户请求该工作后，由 owning workflow 形成获批来源增量及正式任务归属。

1. 提取当前 Opi 外部字段及实际消费者，对比 Pi settings schema、model schema、`settings-manager.ts`、`model-registry.ts`、`resolve-config-value.ts`；schema/文档不足时追踪源码。
   -> verify: 每项旧能力都有真实 Pi 等价或 typed `opi.*` 扩展；每个支持字段都有 presence、null、单位、merge、权威和消费者规则。
2. 至少审查下列迁移处置。它们约束字段清单，不声称实现整套 Pi 功能。

| 当前能力 | 拟归宿及验收条件 |
|---|---|
| `defaults.model` | 拆为 `defaultProvider`、`defaultModel`，分层后解析；CLI/env selector 原子覆盖此 pair。 |
| 主题和项目信任默认值 | 语义匹配时采用 Pi `theme` / `defaultProjectTrust` 名称；保留 Opi pre-trust 权威和项目限制。 |
| Thinking enabled/budget | `defaultThinkingLevel`、`modelThinkingLevels`、`thinkingBudgets`；不能把旧 boolean/budget 机械投射到全部 native wires。 |
| Compaction 和 retry | 仅语义匹配时采用 Pi 嵌套字段。绝对占用阈值不是 `reserveTokens`；除非 owning contract 明确修订，必要的 Opi 行为放入 `opi.*` 保留。Agent retry 不等于 provider retry。 |
| Provider/model 定义和 compatible/custom profiles | Pi model 文档 `providers`、规范 provider/API IDs、`baseUrl`、`apiKey`、`authHeader`、`headers`、`models`、`modelOverrides`、`compat`、metadata 和 cost 单位。既有 wires/proxy 通过准确等价或 Opi 扩展保留。 |
| Iteration/output/tool/image 限制、mutation policy、execution routing/permissions、credential backend | 没有真实 Pi 等价时归 typed `opi.*`；保留省略/默认和授权语义。 |
| 资源路径及 packages | 仅对已支持行为采用 Pi list/object 形式；保留 Opi trust/优先级，不隐式执行 Pi package、MCP 或 skill。 |
| 既有 keybindings | 保留当前动作。Pi 的 keybinding 文件/协议独立，不声称属于 `settings.json`，不悄悄删除 Opi bindings；独立协议准入前采用有限 Opi 扩展。 |

3. 单列 Pi 1.4 增量，包括 client identity 和空白 tool-result 行为变化；owner 来源被替代前保留当前 Pi 1.3 断言。
   -> verify: 没有测试修改或新字段暗改当前版本，也没有保留旧 Opi 拼写作为读取别名。
4. 通过 `opi-implement plan` 协调获批来源和任务图，包括实际 source-drift gates 和人类确认。
   -> verify: 任务 DoD、pins、owned paths、依赖描述同一范围。**本次规划不执行此步骤。**

## 验收包 B：TOML/JSON 宿主配置

**将来 owner：** 新宿主配置任务。**测试归宿：** 既有 `crates/opi-coding-agent/tests/config_tests.rs`、`provider_factory.rs`、`cross_mode_runtime.rs`、`trust_resource_gating.rs`、`early_command_trust.rs`、`doctor_cli.rs`、`list_models.rs`；schema fixtures 归准入后的产品 owner。

先增加以下成对合成 fixtures。地址仅是数据；真正发请求前，测试替换为自己绑定的本地服务地址，不假定 18080 端口有服务。

Settings TOML / JSON：

```toml
defaultProvider = "fixture"
defaultModel = "chat"
defaultThinkingLevel = "medium"
```

```json
{
  "defaultProvider": "fixture",
  "defaultModel": "chat",
  "defaultThinkingLevel": "medium"
}
```

配套 model TOML / JSON：

```toml
[providers.fixture]
api = "openai-completions"
baseUrl = "http://127.0.0.1:18080/v1"
apiKey = "synthetic-test-key"
authHeader = true

[[providers.fixture.models]]
id = "chat"
name = "Fixture Chat"
input = ["text"]
contextWindow = 8192
maxTokens = 1024
```

```json
{
  "providers": {
    "fixture": {
      "api": "openai-completions",
      "baseUrl": "http://127.0.0.1:18080/v1",
      "apiKey": "synthetic-test-key",
      "authHeader": true,
      "models": [{
        "id": "chat",
        "name": "Fixture Chat",
        "input": ["text"],
        "contextWindow": 8192,
        "maxTokens": 1024
      }]
    }
  }
}
```

| Case | 输入 / 操作 | 必须观察到的结果 |
|---|---|---|
| CFG-01 | 成对文档进入同一个语义 resolver。 | 值、存在性、选中 model/auth 来源相同；诊断保留各自路径/格式；单纯解析不访问 env、命令、auth-store 或网络。 |
| CFG-02 | 默认 user/project TOML 旁放有效 JSON，不加 `--config` 启动，再显式选择 JSON。 | 默认仅采用 TOML；显式 JSON 采用可选 `models.json` companion，不发现同目录 `auth.json` 或资源。 |
| CFG-03 | 有有效 TOML 时选择畸形 JSON；显式路径不存在；`.json` 内放 TOML；主文件放原始 model 文档。 | 选中来源按路径/格式/字段失败，不 sniff、回退、重选来源或部分发布配置。 |
| CFG-04 | 先缺省 companion，再放不可读、畸形、不支持的 companion；另测默认 bundle 仅 model companion 存在。 | 缺 companion 为空 model layer；存在但无效则整个 bundle 失败，不把旧 catalog/config 说成新加载成功；默认 settings 缺失仍允许选中 model layer，受 project trust 约束。 |
| CFG-05 | User/project/explicit 层给不同值，再组合 CLI selector 与有/无 `--config` 的 `OPI_MODEL`。 | 严格遵循已记录 Opi 顺序、CLI/env 的 provider/model 原子覆盖和现有 `OPI_MODEL` 例外；显式 JSON 可覆盖较低 TOML。 |
| CFG-06 | 用清单中带来源例子检查 nested maps、arrays、空集合、modelOverrides。 | False/zero/present-empty 保留；每字段执行自己的 replace/merge/clear，不对资源数组统一拼接。 |
| CFG-07 | 旧 `[defaults]`、`model`、`api_key_env`、旧 thinking 路径和新旧混用。 | 已知退役字段报告准确位置和替代说明，不 alias、静默忽略或自动转换。 |
| CFG-08 | 已识别但未支持的 Pi 功能、真正未知扩展键、`$schema` URL。 | 未支持功能明确失败；未知值不生效、保留并可见警告，不透传请求或远程抓 schema。 |
| CFG-09 | Schema 允许的 JSON `thinkingLevelMap` null，再与省略比较。 | Null 为不支持，省略保留来源的不同默认；手册明确 TOML 无原生 null，不引入 sentinel 或省略强转。 |
| CFG-10 | Trusted/untrusted project bundle 尝试命令 key、资源激活、`opi` execution permission 授权。 | 保留 Opi trust；pre-trust 解析不启动行为；即使 trusted，项目也不能自授永久执行权限。 |
| CFG-11 | 改名后逐一检查现有 Opi-only 能力，包括省略/默认/覆盖。 | 行为和权威不变；compaction threshold/reserve、credential-backend 选择仍有区别。 |
| CFG-12 | 两种格式分别启动真实产品，选择 fixture provider/model，向本地服务发请求。 | 两次实际 method/path/model/auth/header/body 一致；discovery/list/doctor 对同一有效 model 一致，且不启动 auth exchange。 |
| CFG-13 | 若获准实现会保存配置，在有 companion 和未知数据时编辑一个 setting。 | 仅原子写选中文档、保持格式和未知数据，不改 companion/另一格式；失败不报告持久化成功。 |

准入后实施顺序：

1. 向既有 config 测试增加 CFG-01–09 fixtures 和反例。
   -> verify: 新场景因预期缺失行为失败，不因缺依赖或 fixture 设置错误失败。
2. 替换外部读取词汇，共用 selection/merge/validation；同步 owning consumers 和 schemas，不增加旧字段兼容。
   -> verify: CFG-01–09 通过，原权威场景继续有效。
3. 接通 trusted staging、真实 factories/startup、诊断，以及确有保存行为时的文档级持久化。
   -> verify: CFG-10–13 及受影响 execution、credential、trust 回归通过。
4. 同步双语指南/help/examples、真实 schema 引用，完整读取 Unreleased 后更新合适 subsection。
   -> verify: 文档符合真实行为，CLI 不宣传不存在的格式/目录选项。

## 验收包 C：配置值与 provider 参数

宿主值来源任务可独立于双语法被否决。其交付前，parser 必须将不支持的命令表达式明确拒绝。

| Case | 输入 / 操作 | 必须观察到的结果 / owner |
|---|---|---|
| VAL-01 | Literal、`$NAME`、`${NAME}`、`$$`、`$!`、缺失 env、空值、畸形值。 | 已采用 Pi 来源的准确语义和 typed selection failure，诊断脱敏；在使用时解析，不在注册时解析。归新宿主值来源任务。 |
| VAL-02 | 受控 fake shell 命令返回 key/header、失败、停顿、启动子进程；执行中取消，跨调用改变 env/output。 | 正确 shell/argv、冻结 cwd/env、准确来源超时、进程树清理、model-value 不跨请求缓存；不套 AWS JSON-process，不回退其他账号。 |
| VAL-03 | 未信任项目含命令；另一选中来源失败或取消。 | 授权前不启动命令，选中来源失败不发现另一身份；实际请求消费选中值或失败。 |
| PAR-01 | 公共调用捕获省略默认、显式 false/zero、model/provider 默认和 per-call 覆盖。 | 产品字段→native/simple option→真实 wire 的来源优先级和单位完整保留。归 20.35 / adapter / 20.34。 |
| PAR-02 | OpenAI Chat/Responses/Codex 各变体更改 output limit、reasoning、store false/省略、session/cache/transport/tool choice。 | 变体字段/省略正确，callback 次数和重试身份准确，不使用统一 serializer。归 20.36 / 20.37 / 相应 owner。 |
| PAR-03 | Anthropic native thinking 省略、关闭、开启但省略预算、显式预算，另测 simple reasoning。 | Native 来源默认（Pi 1.3：开启未给预算时相应默认 1024）不被当前通用 10000 路径或 simple policy 代替。归 20.39 / 20.35。 |
| PAR-04 | Google/Vertex native budget `-1`、`0`、正值，支持的 levels 和 tool/signature 场景。 | Typed 转换保留 dynamic/disabled 语义；Vertex auth target/quota 来源也到达实际请求。归 20.40 / 20.41。 |
| PAR-05 | Azure Chat/Responses、Bedrock、Mistral、Radius 专属请求。 | 两种 Azure wire 保留来源义务；各协议字段、auth endpoint、重试准确；auth 证据不关闭 20.41/42 generation；Radius 归已准入 owner。 |
| PAR-06 | 跨 retry/variant 的 header 省略/设置/删除和函数 callback。 | SDK 来源的 null 删除、受保护 auth 身份、逐变体时序/次数正确；callback/signal/fetch 不是 JSON 字段。归 20.51 和消费者。 |
| PAR-07 | 畸形/不支持 option、选中来源错误、取消、endpoint-sensitive compat、typed provider error。 | 用错 wire 前失败，无账号回退、静默 clamp 或通用 auth/network 超时；auth endpoint 在 compat 推断前确定；保留已有 auth 回归。 |
| PAR-08 | Typed image/classifier/Decisions 参数 fixtures，含不支持输入和 Decisions 凭据限制。 | 各 typed 公共操作消费自身准确字段、返回 typed result/error，不使用 Chat serializer；API-key-only 处不接受 OAuth。归 20.45–20.49。 |

先加 owner 范围内最小失败请求捕获场景，再实现该 conversion/adapter，再检查成功和无效输入。验证实际 URL/headers/body 及每次 attempt；来源交换请求与 generation retry 分开统计；复用有效 auth 证据，不重复全部来源链套件。

## 验收包 D：README 与独立 SDK 使用

**Owner：** 20.56 负责手册/退役，20.57 负责独立包证明，20.58 支撑支持声称，20.59 负责交接。

| Case | 必需证明 |
|---|---|
| SDK-01 | 第一个完整例子使用普通 factory/model → Context → complete/stream，采用 env 或合成显式 API key；不要求产品配置、Agent、手工 resolver、route registration 或 prepare/attempt 生命周期。 |
| SDK-02 | Factory/default 构造、注册、静态 model 查询不启动 env/store/network；显式 built-in 组装提供承诺的 catalog/adapters；不把空 `Models::new` 称为自动 populated。默认 memory store 无须宿主持久化实现。 |
| SDK-03 | Quick-start、streaming、tool-result continuation 示例从实际 `.crate` 进入全新 workspace 外消费者编译运行；README 源码与示例一致；本地服务证明经真实 adapter 的请求和最终结果。 |
| SDK-04 | Streaming/nonstreaming 共用已准入最终结果路径；取消、typed error、usage、model/context continuation 例子符合公共合同；simple/native、auth persistence 职责清楚可用。 |
| SDK-05 | 中英手册条件、平台限制、示例、advanced 链接相同；无活跃示例再用 `Request.system/messages/tools` 或退役 global/frame API；内部协议/历史细节仍有可发现 owner。 |
| SDK-06 | 外部消费者唯一 Opi 依赖为 `opi-ai`，可加已说明普通 runtime/Serde/stream 依赖；不需要产品 factory、解包目录外 unpublished path、`[patch]`、workspace 私有 feature 或真实凭据；平台/MSRV/features/性能声称由相应实际证据支撑。 |

1. 先完成已准入 factories/native options，再将其描述为可用手册。
   -> verify: 对应公共 SDK 路径测试通过，未完成 provider 诚实记录。
2. 创建拟定示例源码并用于手册；包消费者使用同一源码。
   -> verify: SDK-01–04、SDK-06 对实际 artifact 执行，不只测 workspace 源码；保留 artifact/consumer lockfile/依赖图 hash。
3. 重组双语 README，协调链接、迁移说明、根产品例子和 Unreleased。
   -> verify: SDK-05、doc checker、rustdoc、受影响 example targets 通过，不让旧手册成为正确性来源。
4. 向 20.59、再向独立退出审查交接带来源场景和结果。
   -> verify: 缺失证据仍标缺失，示例成功不代表全部 profiles/platforms 通过。

## 测试继承与计划验证命令

保留当前来源仍有效的 schema、transcript/replay、auth/rotation 测试，包括第三节点 R-02 公共 resolver 时序。旧字段 config fixtures 必须与新字段正反例一起更新；仅在合同被替代后删除旧断言。增加能揭示独立行为的测试，不写文档/结构镜像测试。

执行 Pi 测试前检查 imports 和 helpers；有些普通测试导入会读取/刷新用户凭据的 OAuth helper。优先把固定合成场景转写为 Rust fixtures，记录来源/版本和预期 wire。不执行整套 Pi 测试或本地 `.pi` 扩展；Pi 1.4 whitespace/identity 和 live model 名变化不是自动的 1.3 回归要求。

以下是按实际改动选择的未来检查。每个改动 binary/filter 都要执行；记录命令、代码身份、exit 和有效断言数量。Zero-match filter 不是证据。

```powershell
python scripts/opi-cargo-cache.py status
cargo test -p opi-coding-agent --test config_tests --test provider_factory --test cross_mode_runtime
cargo test -p opi-coding-agent --test trust_resource_gating --test early_command_trust --test doctor_cli --test list_models
cargo test -p opi-ai --test request_enrichment --test model_wire_metadata --test per_request_auth
cargo test -p opi-ai --test openai_chat_fixtures --test openai_responses_fixtures --test openai_codex_responses
cargo test -p opi-ai --test anthropic_fixtures --test gemini_fixtures --test vertex_fixtures
cargo test -p opi-ai --test azure_openai_fixtures --test bedrock_fixtures --test mistral_fixtures
```

按 cache tool 使用已有标记的共享外部 Cargo target 和 lease，保留 incremental；不 `cargo clean`，不建 per-session target。受影响 execution/credential/source 测试及新 package/example target 在准入文件存在后加入。既有 `run_early_package_proof.py` 仅覆盖 Schema seam，其历史成功不证明新 quick-start 场景。

跨 crate runtime 和最终退出按范围运行适用 workspace gates：

```powershell
cargo fmt --check --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo test --workspace --doc
$env:RUSTDOCFLAGS = "-D warnings"
cargo doc --workspace --no-deps
```

检查后恢复原 `RUSTDOCFLAGS`。通过 owner 完成真实包/MSRV/平台证据；不能说不存在的 `external_sdk` 或平台 target 已运行。无新改动、失败、证据缺口时不扩大或重复 passing 检查。

本次仅文档规划，必需检查为：

```powershell
python scripts/opi-doc-check.py
git diff --check
```

另核对成对 TOML/JSON fixtures 等价、本地文档链接、新增未跟踪 Markdown 空白。未修改 runtime 或测试，测试影响为 `none`。

## 可审查交接与停止点

未来执行证据逐 case 区分本轮执行、可复核既有证据、缺失，记录相关 source/lockfile/config/artifact 身份。Committed、staged、unstaged、untracked 分开记录，保留无关工作。

本次以双语设计和验收计划结束，不改生产文件、登记来源、schema、账本、测试或当前 README；不暂存、不提交。无关研究文件保留并排除。下一步是另行授权的来源/任务准入，再按依赖实施并由 owner 完成最终验收。

## 本次规划的验证记录

2026-10-25，`python scripts/opi-doc-check.py` exit 0，输出 `opi documentation contracts: PASS`；`git diff --check` exit 0。另用 Python 标准库解析两组成对 TOML/JSON 示例，验证各对及中英之间相等，并检查四份新文档本地 Markdown 链接和行尾空白，全部通过。仅改规划 Markdown，因此未运行 Cargo/runtime/package 测试。

Worktree-only 状态：`HEAD` 保持上述基线；`committed=none-for-this-task`、`staged=none`、`unstaged=none`；任务新增 untracked 为本计划和设计的中英四文件。带入的无关研究文件仍为 untracked 并排除，SHA-256 为 `aa18f7ec8106c34ae194d8d23e6657af6559be78e79a84e266a710bca5cb2824`；账本保持原 hash。测试影响：`none`。

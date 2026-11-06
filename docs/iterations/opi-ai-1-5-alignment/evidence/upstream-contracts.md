# Pi 1.5.0 快照、AI 入口与 telemetry 边界核验

研究日期：2026-11-06。任务：`docs/iterations/opi-ai-1-5-alignment/decisions/01-upstream-contracts.md`。本文为规划证据，未修改 Opi/Pi 仓库、未执行构建或测试、未安装依赖或调用 provider。测试影响：`none`。

## 结论

1. **用户指定的 `.pi/pi-1.5.0` 快照就是本轮目标，但它不等于旧验收文档的 `a57438e918350a2387d7518e46c74f367cc7feaf`。** 同为包版本 1.5.0 不能消除行为差异。应迁移验收条目到快照，保留旧文档作为历史来源；无需再次让用户选择基准。
2. **Pi 的通用遥测契约、AI 埋点和宿主 adapter 是三个职责。** 直接依赖 `pi-telemetry` 的生产包是 `ai`、`durable`、`chord`，不能写成“三个 agent 层”。`agent-core` 没有直接依赖；Pi coding-agent 的安装遥测是独立 HTTP ping，不是已实现的 `pi-telemetry` exporter 桥接。
3. **建议 Opi 提取独立的中立遥测接口，AI 仍拥有自己的事件语义，产品拥有出口与开关。** 这是基于真实跨包消费和依赖方向的结构建议，不是要求复制 TS 目录，也不是自动引入 OpenTelemetry SDK、全局 subscriber 或完整 Durable/Chord 功能。
4. **README 适合作为使用路径参考，不可直接作契约真值。** `telemetry/README.md` 仍声称 agent-core 导出旧遥测 schema，已被 manifest、入口和 changelog 否定。

本文路径约定：`S/...` = `D:/Luiz/Odradek/opi/.pi/pi-1.5.0/...`；`B/...` = `D:/Luiz/Odradek/pi` 中 Git commit `a57438e918350a2387d7518e46c74f367cc7feaf` 的 blob。旧验收文档为 `C:/Users/Luiz/Documents/Codex/2026-11-03/r/outputs/pi-ai-rust-acceptance.md`。

## 一、身份与验收漂移：事实

快照根目录没有自己的 `.git`。旧审计的 `docs/realign/2026-11-06-opi-0.8.3-d382688-vs-pi-1.5.0-8aefc4c-full/evidence/00-inputs.md:12–15` 也明确写明它是来源 commit 未核实的 export；其中 `8aefc4c29d420c27` 是其内容清单 SHA256 的前 16 位，**不是 Git commit**。该旧摘要的精确序列化算法未随文档保存，本次没有将不同算法的摘要差异解释为快照再次变化。

本次独立读取 Git tree，按 `SHA1("blob " + byte_length + NUL + raw_bytes)` 逐文件与目标快照比较；不使用旧审计结论代替核验。检查结果：

| 范围 | 结果 |
| --- | --- |
| 快照文件 | 排除 `.git`、`node_modules` 后 2949 个 |
| 整体与 B 对比 | 相同 blob 2134；同路径不同内容 201；B 有而快照无 1465；快照独有 614。此表只证明身份不同，不表示本任务审计了整个 monorepo |
| `packages/ai` | 同路径不同内容 13 个；快照缺 B 的 2 个测试；无额外文件 |
| AI 源码/测试计数 | 快照 205 个 `src` 文件、183 个 `*.test.ts`；旧文档基线 205/185 |
| `packages/telemetry` | 只有 `CHANGELOG.md` 内容不同；其余文件 byte-identical |
| AI 相同的重要文件 | `package.json`、`src/index.ts`、`src/models.ts`、`src/telemetry.ts` byte-identical |
| 模型运行时数据 | 快照缺 `packages/ai/src/providers/data/`；生成器及 `.models.ts` 包装器不等于实际模型 JSON |

本次冻结算法：按 POSIX 相对路径排序，对每个文件生成 `SHA256(raw bytes) + 两个空格 + 相对路径 + LF`，再对整个 UTF-8 清单计算 SHA256。结果：`959b3b486f111b53771d37da272c8951064952bd9b18596c5c244935cea45072`。复现脚本在本次聊天工作目录 `work/compare_snapshot.py`；这是新的明确算法内容身份，不替旧摘要伪造来源。

关键原始文件 SHA256：

| 文件 | SHA256 |
| --- | --- |
| `S/package-lock.json` | `802adbf0a5e093830226723f37b8971ce42eee442181a1471e0b70b1396b244a` |
| `S/packages/ai/package.json` | `9bac9281776095c8cb1597b706e6cc43c1e082d77189c666b02d068d3c6b9aca` |
| `S/packages/ai/README.md` | `53c48b087f55cd31902cc5988b081d87d99e7ec855335b4491ecf241f9779175` |
| `S/packages/ai/src/telemetry.ts` | `91c635e9e3760869faa9d26af9f0347152b11006dac18f7e9733c7b4dae3b086` |
| `S/packages/telemetry/src/index.ts` | `97839ea4755527f472303ffd512ff94c0ccd9497716f725a0039eb7fc0a9a310` |

AI 的 13 个变化文件是 `CHANGELOG.md`、`README.md`、`catalog/providers/{github-copilot,opencode}.ts`、`src/api/{anthropic-messages,openai-codex-responses,simple-options}.ts`、`src/providers/{compat-schema,github-copilot}.ts`、`src/types.ts`、`test/{anthropic-strict-tool-schema,classifier-models,openai-codex-stream}.test.ts`。缺少的两个测试为 `test/anthropic-message-threads.test.ts` 与 `test/clamp-max-tokens.test.ts`。

以下是最影响旧验收的四组差异，方向均为“旧 B 基线 → 用户指定快照”，不表示本任务实施了删除：

| 契约 | B 与快照的实质区别 | 必须重判的旧验收 |
| --- | --- | --- |
| Anthropic server-side continuation | B 有 `threadState`、create/continue 请求、`supportsMessageThreads` 与 `disableContinuation`；快照没有这套机制。快照直接发送完整 `params`。比较 `B/packages/ai/src/api/anthropic-messages.ts:239–466`、`:837–915`，`S/.../anthropic-messages.ts:608–678`；B `types.ts:257–267` 对 S `types.ts:257`；B `compat-schema.ts:496` 对 S `compat-schema.ts:493`。 | B05、P10、K13、S02。旧文档关于 Anthropic 续接缓存跨账号键的具体风险不适用于该快照；Codex 等其他共享状态仍须验，不能一并删除安全目标。 |
| 输出预算 | B `simple-options.ts:17–35` 使用 15% 输入余量以及 32768 分支；快照 `src/api/simple-options.ts:15–21` 只有固定 4096 余量，仍有 `contextWindow<=0` 和显式 cap 的边界。 | G12 的 15%/32768 预期需改为快照公式；不能直接执行旧 expected 后把合法快照行为判回归。 |
| Codex retry sleep 清理 | B `openai-codex-responses.ts:189–198` 注册 once abort handler，并在正常 timer 完成时 remove；快照 `src/api/openai-codex-responses.ts:182–194` 注册匿名 handler，不设 once、正常完成不 remove。 | R03、S04/S05 的 listener 生命周期需要新受控案例；源码显示清理差异，不等于已经实测整体泄漏。若 Rust 修复则分别报告兼容与资源门。 |
| Copilot 分类 | B `src/providers/github-copilot.ts:4–48` 合并 classifier models，并为 System One 添加 `/v1` endpoint wrapper；快照 `src/providers/github-copilot.ts:1–34` 只载入 chat models、注册三个聊天 API。 | M09 的派生分类 endpoint、provider 能力矩阵、相关 C01 分类覆盖应按快照重列。整个库仍有分类能力，不能将“Copilot 此能力不存在”写成“库无需分类”。 |

额外相关差异：B Anthropic legacy tool schema 保留 `$defs`/`definitions`（`B/.../anthropic-messages.ts:1901–1914`），快照 `S/.../anthropic-messages.ts:1636–1643` 只取 `properties`/`required`；工具 schema/strict 相关预期也要迁移。此处只定位，不扩张为全协议审计。

## 二、AI README、包导出与源码结构：事实及规划含义

`S/packages/ai/README.md:3–5` 定义统一 API 的能力背景；`:108` 从 `Models` collection 的实际调用开始，`:243–247` 区分 provider（目录、鉴权、行为的运行单元）与 API（wire protocol）。随后文档按 Models/目录、鉴权、工具、图像、分类、thinking、遥测/错误、扩展、Faux、上下文、宿主、OAuth、compat、开发展开。它是完整使用指南，不只是类型清单。

实际导出由 `S/packages/ai/package.json:13–52` 决定：根、`models`、`compat`、`providers/*`、`api/*`、`utils/*`、`oauth`、`bedrock-provider`、`bun-oauth`，以及 `pi-ai` CLI。通配要展开嵌套路径；`src` 有 export 不自动等于包级可达。根 `src/index.ts:1–47` 选择性重导出类型、Models/auth/store/Faux、流与工具函数；它没有完整重新导出所有 provider/API。`AI_TELEMETRY_SCHEMA` 经 `models.ts:73` 从根与 models 暴露，`observeModelRequest` 不是另一个公开 telemetry 子路径。

源码不是“一层 provider 列表”：`src/{api,auth,compat,providers,utils}` 五个目录，以及 `models.ts`、`models-store.ts`、`types.ts`、注册表、`telemetry.ts`、CLI/宿主入口等根文件。`catalog/README.md:3–7` 另把生成器、提交的类型包装器、gitignored 模型 JSON 分开；不能仅凭 205 源文件清点判断发布闭包已齐。

轻量边界有明确消费者意义：`README.md:1724–1734` 说明 `./models` 不载 schema 验证、内建目录、SDK 实现；根仍 eager 载 pi-schema 验证；单 provider 工厂载自身目录和 lazy API；直接 `api/<id>` eager 载该 SDK；`providers/all` 载全集，compat 带旧全局 API。`package.json:8–11` 单列有副作用入口。Rust 规划可保留这些依赖能力边界，**不必复制 JS 文件名、Node 加载技巧或 TS 目录**。

暂定文档建议：独立 `opi-ai` README 应先给“最小核心+Faux”和单 provider 的可编译外部调用，再分别讲 Models/provider/API、鉴权与存储、chat/image/classify/deferred、流终态与取消、扩展和遥测、宿主/features、compat；详细契约放 API/rustdoc。对照验收 R08，示例应从公开发布入口编译，而不是复制上游 README 后引用内部模块。

## 三、telemetry 的职责、三个消费者与文档冲突

### 中立包拥有的契约

`S/packages/telemetry/package.json:8–23` 导出根、render、testing，且没有 runtime dependencies。`src/index.ts:3–110` 定义 attributes/status、opaque carrier、links、store query、TelemetryContext/Span；`:168–176` schema identity；`:448–456` typed starter；`noop.ts:4–37` 无操作实现；`memory.ts:182–244` 参考记录实现；render/testing 是通用辅助面，不含 AI provider 语义。

接口是显式 context、callback 拥有生命周期：startSpan 同步恰好一次调用 callback；返回值/异常保持；等待 callback future 结束；录制同步；settled 后录制无效；attributes 合并且忽略 undefined；显式状态最后写入获胜。证据为 `README.md:123–139` 与 `memory.ts:193–244`，而非“引入 tracing 就自然等价”的推断。metrics 和 propagation 是可选 capability，子 span 保留 capability；不要求所有 adapter 实现 exporter。

schema 是类型与描述数据，非 runtime 验证器；`defineTelemetrySchema` 返回输入（`index.ts:174–177`），typed starter 不读取/保留 schema（`:448–456`）。Exporter buffering/flush/sampling/backend ID 属 adapter（`README.md:139`），包本身无 exporter/全局 current span（`:13`）。

### 三个直接生产消费者

| 包 | 直接依赖证据 | 实际复用 |
| --- | --- | --- |
| ai | `packages/ai/package.json:73` | `types.ts:163` 请求选项携带父 TelemetrySpan；`models.ts:901–975` 的 stream/streamSimple/streamDeferred 在已有转发路径接 observer。`telemetry.ts:64–105` 自己拥有 AI schema；`:121–209` 自己记录请求/poll、首事件、块长度、用量、费用。 |
| durable | `packages/durable/package.json:165` | `harness/types.ts:685–686` 接收 host-owned TelemetryContext；`harness/submissions.ts:89–118` 输入/continue 建 root 并 link caller，write 用 parent；`harness/scheduler.ts:2218–2281` 从 task carrier extract，再围绕 phase handler + commit 建 span、计时。`harness/telemetry.ts:49–139` 自己定义 Durable schema，非 AI schema。 |
| chord | `packages/chord/package.json:94` | `context/index.ts:10–35` Context 携 TelemetrySpan 默认 no-op；`:90–104` withTelemetry/traceIn 显式派生子 context。`next/host/host.ts:83–84` 接受宿主 adapter，`:492` 将 metrics 交 Batch。`telemetry-sqlite/index.ts:18–35` 实现 adapter+TelemetryStore 并暴露 flush/close，`:71–134` 负责队列、计时刷新、carrier 与能力；这是后端实现实际位于契约包之外的证据。 |

AI 埋点边界必须单独记录：`Models.stream/streamSimple` 用 `pi.ai.request`，`streamDeferred` 每 poll 用 `pi.ai.poll`；complete/fetch helper 复用它们。`Models.cancelDeferred/generateImages/classify`（`models.ts:977–1024`）没有调用 observeModelRequest；direct provider/API 也不自动埋点（`README.md:1128`，`api/lazy.ts:98–119` 不传 observer）。不能因 option 类型含 telemetry 就声称每个入口都有 request span。

`telemetry.ts:156–208` 的内容政策和时序属于 AI：首个 normalized event 含 setup error；只在 block end 计 UTF-16 长度，工具按 delta 累加；error 的 provider_error 文本最多 500 字符；aborted 保持 ok；pending/deferred poll 不累计 token/cost。`api/lazy.ts:37–83` 在转发既有事件时观察，不加第二个消费者。`test/telemetry.test.ts:95–176` 验证事件对象/终态和 error/aborted 的区别；本次只读测试，未运行。

### agent-core、coding-agent 不应误画为直接消费者

`S/packages/agent/package.json:25–28` 只有 ai/schema runtime 依赖；`src/index.ts:1–5` 仅 Agent、loop、proxy、stream-fn、types。`CHANGELOG.md:45` 明确移走 harness、遥测 schema 与 pi-telemetry re-export。因此 `packages/telemetry/README.md:458–462` 的 `AGENT_TELEMETRY_SCHEMAS` 示例和 `:504` 的 agent-core instrumentation 声明已漂移。

Pi coding-agent 的 `core/telemetry.ts:3–12` 只解析 PI_TELEMETRY/setting；`modes/interactive/interactive-mode.ts:1446–1462` 在非 offline 且开启时直接向 `https://pi.dev/api/report-install?version=...` fetch，超时 5 秒并忽略失败。其 manifest 无 pi-telemetry runtime dependency，源代码无该包 import；`package.json:35` 的 binary build 先 build telemetry 只是构建顺序。**此安装 ping 不桥接 TelemetryContext；不能将它称作上游已经存在的 OpenTelemetry exporter。**

## 四、暂定建议与剩余未知

建议规划三个所有者，接口深度按真实消费者收敛：

- `opi-telemetry` 或同等独立中立 crate：显式 context/span、status/event/attributes、可选 metrics/propagation、no-op 与参考/一致性测试。根据目标能力保留 store/render 的独立公开边界；不依赖 AI、agent、CLI，也不拥有 `pi.ai.*` schema。
- `opi-ai` 内的 instrumentation：只拥有 model 请求/poll 的 span 名称、字段、状态、计量、时点。保持请求选项携 context，依靠已有转发/终态路径；不能提取后顺手扩大或缩小已冻结的观测面。
- 产品/宿主 adapter：拥有启用、backend、导出目标、脱敏、缓冲、sampling、flush/close 与运行时配置。是否 OpenTelemetry、日志或 SQLite 是 adapter 选择，不能倒灌到独立 AI 库的强制依赖。Pi 的 install ping 可作为产品遥测与库遥测分别拥有的例子，不应拿它证明桥接已存在。

提取理由是：已有 AI、耐久执行和通用上下文三种真实消费者共享相同生命周期/传播协议；独立 AI 库不能反向依赖产品才能被观测；同一个宿主 adapter 应可关联跨层 trace。**不是**“上游有一个 telemetry 文件夹，所以 Rust 必须逐包照抄”，也不是“当前三个 agent 层都依赖它”。具体 Rust trait/owned handle、Send/Sync 和 drop 承诺应由本次 Opi 边界研究决定，本文不冒充已定接口。

未知/后续证据要求：快照的真正上游 Git commit 尚不明，但完整内容身份已可冻结；真实生成模型 JSON、发布 dist、Node/SDK 运行环境尚未补齐；未运行 adapter conformance、AI 遥测测试或 Codex listener 探针；未审核全部 13 个变化文件的每条行为。旧验收应首先更新 B01/B02/B05 的身份及公开清单，再局部重判上述 P10/K13/G12/M09/资源要求，保留其他未失效的验收原则。文档漂移登记为冲突，不复制进新 README。

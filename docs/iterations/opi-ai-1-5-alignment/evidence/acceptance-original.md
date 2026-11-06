**pi ai 的 Rust 完整复刻验收标准**

适用对象：`D:/Luiz/Odradek/pi/packages/ai`。源码基准为本地 Git commit `a57438e918350a2387d7518e46c74f367cc7feaf`，包名 `@earendil-works/pi-ai`，包版本 `1.5.0`。制定日期：2026 年 11 月 3 日；第二轮修订：2026 年 11 月 5 日，Asia/Shanghai。本轮保持原基准，不随上游版本移动。

验收结论必须同时满足：**原版可观察行为等价、能力覆盖完整、Rust 工程结构合格、验证证据可复现。** 四者不能相互抵消。本标准规定交付结果和判定条件，不规定实现步骤、具体 crate、异步运行时或目录照搬方式。

这里的“完整复刻”指：在固定版本、声明的平台和输入范围内，所有纳入范围的契约均有验证证据，且不存在未解决的行为差异。有限测试不能证明所有未来输入都绝对正确；因此不能用“测试全绿”替代范围清点、反例审查与风险关闭。

本地文件清点为 205 个 `src` 文件和 185 个 `*.test.ts` 文件。本轮补查 package exports、公开方法与辅助函数、运行时 schema、原测试边界及跨语言观察语义；文件清点不代表逐行审计完成。当前检出没有 `src/providers/data/` 和根目录 `node_modules/`，原版执行基线仍须补齐。本文中的条件是待执行的验收要求，不是 Rust 实现的通过记录。

**1. 验收对象必须先冻结，不能一边复刻一边移动基准。**

| 编号 | 必须满足的条件 | 拒收情形 |
|---|---|---|
| B01 | 固定完整 commit、工作树差异、包版本、lockfile、实际 SDK 版本、Node 版本及原版执行入口；源码和材料有内容摘要。 | 只写“兼容 pi 最新版”或只记录包版本。 |
| B02 | 同时固定模型生成器、上游目录输入、生成 JSON、生成配置和摘要；代码版本与模型数据版本分别记录。 | 只有 `models.generated.ts`，没有真正模型 JSON，仍声称目录和价格完全相同。 |
| B03 | 固定操作系统、架构、时区、locale、代理、环境变量集合、时钟与随机控制规则；不包含真实凭据。 | 原版和 Rust 使用不同隐式环境，差异无法重放。 |
| B04 | 固定 Rust commit、工具链、MSRV、依赖解析结果、features、目标平台、构建模式及交付包摘要。 | 仅开发目录可运行，发布产物身份不明。 |
| B05 | 以 package exports、构建产物和递归 re-export 形成清单；每个运行时值、类型、公开方法/重载、可达子路径、CLI/生成工具操作均有唯一记录及契约 ID。分类为 runtime、type-only、internal 或 tooling；公开项无空白映射才通过。 | 未实现改标“不适用”；把内部 export 强制公开；遗漏通配路径下的辅助函数或嵌套模块。 |
| B06 | 源码、原测试、文档和实测发生冲突时，必须登记冲突并确定该案例的基准；未决项阻断该契约通过。 | 默认认为文档正确，或默认所有原版偶然行为都可忽略。 |

Rust 原生接口允许采用符合 Rust 的命名、类型、错误表达和生命周期。每个原入口必须有功能及行为映射；不要求保留 TS 文件布局或语法。**旧 JS 调用方无需修改即可运行**是额外的生态兼容声明：若作出这个声明，必须验收实际 JS/Node/Bun/浏览器调用和绑定，不能以 Rust 原生测试代替。没有作出该声明，也不能因此省略原包的业务能力或数据契约。

范围应包含根入口、`models`、`compat`、`providers/*`、`api/*`、`utils/*`、`oauth`、`bedrock-provider`、`bun-oauth` 和 `pi-ai` CLI。通配子路径须按实际发布内容展开，包含可匹配的嵌套模块；源码里单独出现 export 不足以判定公开。例如 `auth/resolve.ts` 没有独立包导出，根入口只重导出指定 auth 文件；`providers/images/register-builtins` 则属于通配可达项。包外的 `pi-schema`、`pi-telemetry` 和供应商 SDK 所决定的行为纳入观察范围，不扩展为整个 Pi agent/TUI 的复刻。

公开清单至少覆盖以下能力簇；表内斜线表示同簇多项，不能只抽测其中之一。最终发布包的入口解析结果必须与清单逐项核对，缺失、额外的未声明入口或 unresolved re-export 均不通过。

| 能力簇 | 必须登记的公开观察面 |
|---|---|
| Models | getProviders/getProvider/getModels/getModel/getModelsOfType/getModelOfType/getAllModels/refresh/checkAuth/getAvailable/getAvailableOfType/getAllAvailable/getAuth/login/logout/stream/complete/streamSimple/completeSimple/streamDeferred/fetchDeferred/cancelDeferred/generateImages/classify，共 24 个方法；getAuth 的 provider-id 与 model 重载另分案例。 |
| MutableModels 与工厂 | setProvider/deleteProvider/clearProviders；createModels/createProvider；逐 provider 的工厂选项、元数据、鉴权方法、支持的模型类型和操作。 |
| 静态目录 | providers/all 查询、每个 `.models` 导出、生成时间、三类目录 getter；compat 的旧查询和图像目录。静态目录不等于已注册 provider 集合。 |
| Schema | compat-schema/model-schema/schema-builders 的运行时值、常量、构造器及类型；CompatSchemasByApi 的映射。 |
| 协议及包装器 | api 下公开 stream/simple、转换器、工具/预算/采样/缓存辅助函数、错误类、lazy 能力；Cloudflare/OpenCode 等 provider 包装器。 |
| Auth 与存储 | 根入口重导出的 AuthContext/CredentialStore/helpers/types、ModelsStore；OAuth 旧扩展类型；lazyOAuth 与专用宿主加载入口。 |
| 流与工具 | EventStream/AssistantMessageEventStream、JSON preview/finish/repair、工具验证、Transcript 及文本辅助函数。 |
| 辅助与观察 | estimate/overflow/retry、abort/abort-signals/sleep、headers/provider-env/proxy、identity/hash/UUID/version、error-body/models-error/diagnostics、StringEnum、OAuth page 与公开成本转换函数。 |
| Faux | 构造、脚本/队列、state、辅助消息构造器、流、缓存模拟、deferred 和 compat 注册/注销。 |
| 兼容、宿主与工具链 | 聊天 registry、独立图像 registry、legacy aliases、环境鉴权辅助函数、CLI、目录生成/检查模式；包级副作用。 |

**2. 每一条验收项都必须有可执行的契约记录。**

每条细化契约至少记录：`ID、原入口、Rust 对应入口、前置状态、输入、动作、输出、事件顺序、错误阶段、状态变化、副作用、清理条件、判等方式、源码证据、原测试、差分案例、执行结果、环境身份`。正文中的条目是必须展开的验收族；不能用一个普通 happy-path 用例替整行签字。

结果状态只允许明确区分：通过、失败、未执行、受阻、不适用、有意差异。不适用必须有源码范围和理由；有意差异即使有正当原因，也不能计入“严格等价通过”。每个适用契约至少覆盖正常、边界、失败；涉及状态的补充取消、竞态、迟到结果和清理。

**统一判定规则：每行中每个适用子条件都成立且证据完整，才算该行通过。** “有测试”“已记录”“符合原版”均要求：案例能定位到具体输入、原版预期、比较规则、实际执行结果和输出摘要；仅创建空表、只写测试名称或留待日后决定均不通过发布验收。预期为不完成的低层操作必须用有界观察窗口断言其 pending 状态；测试进程超时不能直接当作该行为已被证明。

每个子条件标为“兼容事实”或“新增工程要求”。两者冲突时分别报告：兼容差分不得改 expected 来迁就工程改进；安全或资源门槛也不得因原版同样存在问题而自动豁免。未解决冲突只能交付明确限定的兼容结论，不能签署“全部门槛通过”。本轮已定位的 Faux 引用、泛型流回调、Anthropic 会话隔离等差异必须保留在冲突表，不能靠统一包装消失。

判等规则必须在运行 Rust 测试前确定：

| 观察面 | 合格的比较方法 | 禁止的宽松处理 |
|---|---|---|
| 请求 | URL、方法、有效 headers、请求体、工具/schema、路由、重试次数和顺序分别比较；签名及协议要求的字段按精确字节比较。 | 因 JSON 大致相同就忽略 URL、默认参数、鉴权来源及签名。 |
| JSON 数据 | 按字段规定缺失、null、默认值、未知字段、顺序和数值语义；仅对确认无序的对象采用结构比较。 | 全局删除 null、空值或未知字段；对全部数组排序。 |
| 事件 | 比较事件种类、次序、索引、delta、终态以及读取 partial 的时刻。只有明确允许并行的事件才按偏序比较。 | 只比较最终文本，或将全部 delta 拼接后丢弃轨迹。 |
| 对象和状态 | 同时检查事件发生时的内容、保留引用后的再次读取、输入是否被改写、存储与缓存状态。 | 把所有活对象深拷贝后声称已证明原引用语义。 |
| ID 与时间 | 可控时精确比较；不可控时采用具名的关联映射，保留一致性、唯一性、先后、单位、缺失条件。 | 删除全部 ID、签名、时间字段。 |
| 错误 | 比较发生阶段、载体、类别、稳定消息/错误体、保留的 partial/usage、后续副作用。 | 全部转成一个字符串 `request failed`，或统一忽略错误正文。 |
| 成本和概率 | 字段缺失、计数及分支选择精确；确需浮点容差时，逐字段预先定义并证明误差来源。 | 统一保留两位小数掩盖阶梯、重复计费或归一化错误。 |

自然语言模型输出没有可重复的逐字承诺。严格差分使用同一受控上游响应；真实联网测试只验证身份、请求接受、协议、工具和结果结构等可观察能力。

**3. 公共能力和数据契约必须完整。**

| 编号 | 验收条件 |
|---|---|
| C01 | `stream`、`streamSimple`、`complete`、`completeSimple`、延迟结果获取/取消、图像生成、分类均有公开入口和对应能力判定。不能只交付聊天流。 |
| C02 | `system/user/assistant/toolResult`、text/thinking/image/toolCall、usage、diagnostics、deferred、嵌套工具记录及所有可选字段都有映射、序列化和反序列化用例。 |
| C03 | 聊天、图像、分类模型不能混用；缺失模型 `type` 按 chat 处理。模型身份比较、同名不同类型模型及动态覆盖行为与原版一致。 |
| C04 | 自定义 provider/API 字符串、provider 专用选项、扩展 sampling 参数及自定义适配器能力保留；不能将开放标识封闭成只容许内建值的枚举。 |
| C05 | `onPayload`、`onResponse`、`onProviderStreamEvent`、鉴权与遥测回调的调用时点、次数、等待、报错和支持范围逐入口验收。不能假定全部适配器都支持全部钩子。 |
| C06 | 聊天 compat registry 验证同 API 覆盖、sourceId 注销、reset、内建注册保留既有项和旧 API 分派。图像 registry 独立验证 register/get 及 built-in 无条件覆盖；不得套用聊天的 reset/unregister/保留已有项规则。两类初始化顺序任一不匹配即失败。 |
| C07 | 运行时 schema 值及构造器须比较枚举顺序、required、optional、union、数值界限、开放属性、description/default 注解及 CompatSchemasByApi。ProviderCompatSchema 中被清除的 API 专属 default 不得重新出现；只验证模型能反序列化不足以通过。 |
| C08 | 24 个 Models 方法、3 个修改方法、构造器及重载全部有案例。原始查询与 available 查询分别覆盖未知/空串 provider、首匹配、注册顺序、替换/删后重插、provider 抛错和过滤优先级；将所有查询异常统一吞成空列表即失败。 |

| 编号 | 跨语言数据验收条件 |
|---|---|
| D01 | 分别覆盖字段缺失、JS undefined、null、空串、0、false、空数组、空对象；按原调用点的默认规则判定，不作全局同一化。 |
| D02 | 对实际可达数值路径验证 JS Number 的精度、2^53 附近整数、负零、指数、溢出、非有限数及序列化结果；只规定原入口能到达的输入，不虚构无效场景。 |
| D03 | 对每个长度、截断、offset 指明 UTF-8 字节、UTF-16 code unit、Unicode 标量或其他单位；覆盖非 BMP、组合字符、代理对跨片和孤立代理项。 |
| D04 | 对象键、数字形字符串键、Map 顺序、替换和删后重插、重复 JSON 键分别验收；影响 prompt、schema 比较、签名或序列化的顺序不得丢失。 |
| D05 | 未知字段按入口分别确定保留、过滤、拒绝；未知 provider 字符串可扩展，不代表未知模型 type 也应被无条件接受。 |
| D06 | 可序列化 transcript、目录、凭据和 handle 至少覆盖该 TS 快照与 Rust 的互读及往返；重启只适用于声明持久性的格式/Store。历史兼容列明来源/目标版本、方向和不可逆规则；InMemory 不承担耐久性。合法数据丢失、误发裁剪字段或未声明迁移即失败。 |
| D07 | UUID、ID 归一化、错误格式、正则分类、字符串清理均以原行为为基准。UUIDv7 的版本/variant、同毫秒顺序、时钟回退、显式 timestamp 与越界错误需单列。 |
| D08 | 按入口登记顶层对象、数组、元素、arguments/schema 的别名图，测试保留引用后读取及合法修改传播。ModelsStore 的深拷贝、CredentialStore 的直接引用、模型 getter 元素复用、Transcript 的浅复制/复用和 Faux 的分层共享分别验收；统一 deep clone 或统一共享导致可观察差异即失败。 |

**4. 流式协议必须按状态机验收。**

| 编号 | 验收条件 |
|---|---|
| E01 | 正常生成先 start，再内容事件，最后 done；准备失败可直接 error。done 对应 stop/length/toolUse/deferred，error 对应 error/aborted。pending 不能被当作成功完成。 |
| E02 | 每个内容块的 start/delta/end、contentIndex、交错顺序、权威完成值与原版一致。start 的初始内容、参数和签名按协议验收；Anthropic 可从 content_block_start 带入 text/thinking，redacted 块另测，不能统一要求为空。 |
| E03 | partial 按 Adapter 验证即时及延后读取，不能全局假定单一活对象。Faux 每次浅拷贝顶层，新增块替换 content 数组，已开始块可共享增长；旧 start.partial.content 因而可仍为空。对顶层/数组/块/arguments 任一关系的错误统一化即失败。 |
| E04 | 对正常 provider 请求且用户回调不重入/抛错的路径，至多一个有效终态；缓冲事件按序排空，完成后 push 忽略，result 与对应 complete 一致且不依赖先排空。泛型流及 observer 异常按 E09 单独判定，不能强套此不变量。 |
| E05 | 底层 EventStream 多消费者为竞争 FIFO，不是广播。`end()` 无 result 会结束等待中的迭代，却不自动完成 result；此低层行为不能混同有效 provider 流的合法终态要求。 |
| E06 | 对同一字节流逐点二分、单字节、多帧同块及随机多段切分；覆盖 UTF-8、CRLF、JSON 转义、SSE 字段、EOF 尾片。相同切分需匹配事件；改变切分只断言该协议保证不变的结果。 |
| E07 | 空流、缺终态、半截工具参数、终态之后连接仍开放、重复终态、断连、malformed 帧分别有预期；区分供应商停止字段、协议终态、SDK 迭代结束。仅对有提前结束契约的协议要求不等 EOF；其他协议保留原终结时点及后续 usage。 |
| E08 | 请求创建、首次 poll、鉴权、响应头、首 token、工具参数中途、重试等待、关闭时发生取消，分别验证错误阶段、保留结果和清理；drop、停止消费、请求取消和远端取消不得视为同一动作。 |
| E09 | 泛型 EventStream 覆盖 isComplete 重入 push/end、两种回调抛错、多个 waiter 的 terminal push、随后 end、无 result 的 end 后再带 result。原版 extractResult 抛错时 done 已置位而事件/result 尚未递交，terminal push 仅交付一个 waiter；必须录制这些低层结果，不能以“所有结束均唤醒并完成”判通过。 |
| E10 | lazyStream/lazyApi 覆盖 setup 同步抛错/异步拒绝、source 迭代失败、有无 result、observer 重入/抛错、提前终态与加载失败。分别比较流结果和 observer 收到的事件/结果，不假定两者在 observer 失败时必然相同；可选 deferred 方法的存在性必须与能力位相同。 |

当前 `types.ts` 关于 text/thinking start 为空的注释与 Anthropic 接收非空初始块的实现及回归测试预期存在差异；其 shared partial 注释也不能概括 Faux 的分层引用。E02/E03 采用具体入口实现与已有测试预期，正式验收须以原版录制确认。Google 的 finishReason 仅更新停止状态，当前实现继续迭代后才产生 done；不能把它等同于所有协议都可立即关闭的终态。

Rust Future 通常需要被 poll 才推进，但独立任务可能继续运行；因此必须测量入口返回前后的可观察动作，不能机械照搬 JS Promise 的写法，也不能机械认为 drop 就结束外部副作用。[Rust Future 官方说明](https://doc.rust-lang.org/std/future/trait.Future.html)

**5. Transcript、工具参数和推理状态必须逐项兼容。**

| 编号 | 验收条件 |
|---|---|
| H01 | Context 的 `systemPrompt/tools` 正规化、空输入、合成 system 的 timestamp、后续 content 追加、section 替换/null 删除和顺序均匹配原版。 |
| H02 | 工具先移除后添加、同名重定义、none/additions/changes 三类历史表达能力均有测试；不能把对话中所有 system 和工具变化简单折叠成最终列表。 |
| H03 | 跨模型判定包含 provider/api/model；外来 thinking 删除，不转成可见正文；同模型 opaque/redacted 签名按原规则保留。textSignature、thoughtSignature、namespace 均有相应案例。 |
| H04 | 工具 ID 变换与结果关联同步；孤立调用补 `No result provided` 错误结果；夹在调用和结果之间的 system 的位置正确；error/aborted assistant 的历史回放规则匹配。 |
| H05 | 非视觉模型、错误工具结果中的图像、相邻图片占位、正文与图片顺序、base64 和 MIME 均有请求快照；不得无声丢图或擅自更换占位文本。 |
| H06 | thinking level 的缺省、off、null、不支持、预算和最大档映射与原版一致；xhigh/max 的可用条件和 clamp 寻找方向需覆盖，不得统一向下取整。 |
| J01 | partial JSON preview、最终 JSON repair、最终工具参数验证分开验收。最终非对象或损坏参数必须采用原版 `{}` 与 `argumentsError` 等行为，不能用容错 preview 冒充成功结果。 |
| J02 | preview snapshot 不随以后 append 被改写；公开 set/append 变更、深度规则、replace 的前缀与非前缀情况有证据；保留原版允许的复杂度预算导致的 preview 滞后。 |
| J03 | 工具参数验证必须匹配 pi-schema 的 coercion 和 unresolved ref 规则，而不仅符合某 JSON Schema 规范。包括 integer `"42.1"`→42、required string null→`"null"`、optional 非 nullable null→省略等原测试行为。 |
| J04 | `argumentsError` 优先失败，输入不被修改，返回值的深拷贝隔离、错误路径、条目顺序、消息和输入展示符合原版；nullable union 已匹配值不被错误转换。 |
| J05 | constrained sampling 的 strict prefer/require、可降级条件、schema 转换、grammar 选择、字符增量和重复关闭均有专项；不得修改调用者的原 schema。 |
| J06 | parser、原始累计 JSON、SDK response、传输临时状态不得泄漏到公开 content、最终消息或持久 transcript；长参数、深层对象和恶意片段无 panic、无限循环及未受控的复杂度退化。 |

**6. 协议、provider 和传输必须分开清点。**

当前类型声明中有 10 类聊天协议、1 类图像生成协议和 4 类分类协议。一个协议通过，不代表复用该协议的全部 provider 配置通过。下表是必须建立专项证据的范围，不宣称本次已完整验证这些实现。

| 协议或组合 | 必须纳入验收的差异 |
|---|---|
| OpenAI Completions | tools 为空/缺失、tool choice、reasoning 内容和签名、兼容字段、cache-control、sampling 覆盖、rawStopReason、responseModel、usage。 |
| OpenAI Responses | item/output 索引、工具与 namespace、reasoning replay、message/response ID、incomplete/failed、终态缺失、缓存亲和、实际服务模型与费用。 |
| Azure Responses 及 Azure 的 Completions 配置 | endpoint/resource/deployment/API version 优先级，Azure 与代理地址规则，鉴权、工具和 reasoning 差异。 |
| OpenAI Codex Responses | SSE/WebSocket/websocket-cached/auto、握手与 idle 超时、fallback、终态早于 EOF、增量续传、压缩、账号/会话连接隔离和清理。 |
| Anthropic Messages | SSE、签名和 redacted thinking、严格工具降级、缓存写入时长、模型 fallback/federation、continuation/message threads、重放与 effort 变化。 |
| Bedrock Converse Stream | SDK 事件流、凭据链、区域/endpoint、签名与保留头、response/error metadata、thinking、工具状态、严格工具降级和缓存费用。 |
| Google Generative AI 和 Vertex | 两种鉴权/配置路径、SDK retry、thinking level/budget/signature、工具 ID、图片工具结果、usage、原版不支持的选项行为。 |
| Mistral Conversations | wire 命名、工具 schema、thinking replay、UTF-8/SSE 分片、raw stop reason、响应头超时与已开始长流的不同处理。 |
| pi-messages | 上游 Pi 事件还原、索引验证、debug/rewrite 信息、错误与 partial、缺终态、回调顺序；不擅自套用其他协议的默认重试。 |
| OpenRouter Images | 非流式生成、输出 modalities、首 choice、文本和图像顺序、data URL 的接受规则、usage、取消与错误结果。 |
| Typesafe System One / Cloudflare System One | choice/score/bool、概率/置信度、图片格式和数量、不同图片传递格式、温度和错误路径。 |
| llama.cpp Classify / OpenAI Decisions | logits 到结果的转换、缺答案/拒答/非有限数、模型能力、图片；Decisions 的 504 不重试与已发生费用保留。 |
| Cloudflare Gateway/Binding、Copilot、OpenRouter、Radius 等 provider 配置 | 原协议之外的 endpoint、headers、路由、认证、兼容开关、目录和运行环境；不能用一个 OpenAI 兼容服务替全部品牌验收。 |

| 编号 | 跨协议验收条件 |
|---|---|
| P01 | 建立 provider × API × 模型能力 × transport × auth × 失败阶段矩阵。无效组合有拒绝/忽略的明确预期；每个有效行均有结果。 |
| P02 | 线上实际请求的默认字段、参数覆盖、删字段、路径、query、headers、压缩及签名可核对。Rust SDK 的默认行为不能未经测试就继承为兼容行为。 |
| P03 | 默认/模型/请求/鉴权 headers 的优先级、大小写不敏感覆盖、null 删除与最终转换均验证；Bedrock 的保留头例外单列。 |
| P04 | scoped env 与进程 env、代理变量、NO_PROXY 的域名/端口/IPv6/通配行为及不支持代理类型有用例；不读取测试机器真实隐式配置。 |
| P05 | HTTP 请求重试与 retryAssistantCall 分开。HTTP 验 Retry-After 优先级、抖动和服务端等待上限；assistant helper 的实际退避无随机抖动，验 retries 不含首次、quota/billing 排除优先、回调顺序/抛错传播、等待取消后保留字段及删除 errorMessage。将两层合并或额外重试即失败。 |
| P06 | 超时分别标明连接、等响应头、body idle、总请求、WebSocket 握手、OAuth 刷新的含义；不能全局变成同一个超时计时器。 |
| P07 | HTTP 非 2xx、SDK 异常、供应商错误帧、解析失败、调用者取消、设置失败分开判定；错误体保留、截断、状态提取和 diagnostics 与原版一致。 |
| P08 | `onPayload` 修改与保留原 payload、`onResponse` 在失败 HTTP 上是否调用、原始流观察钩子在哪些协议触发均逐项验证；重试不得意外重复副作用。 |
| P09 | deferred 按可选方法及 lazy 能力位登记：无能力、仅 fetch、仅 cancel、两者都有分别测试。该快照内建远端 lazy 工厂未启用 deferred，不能要求它们全部支持；成功路径以 Faux/自定义 Adapter 为基准。handle、pending、wait、取消、重复/并发 fetch 逐入口相等才通过。 |
| P10 | 按协议记录 continuation/affinity/WS 缓存实际键、前缀验证、disableContinuation、命中/失效/清理后的请求；并比较账号变化等时序。当前 Anthropic threadState 以 sessionId 为键，前缀含 provider/baseUrl/model 而不含凭据；不得把跨账号隔离写成已实现事实。该风险另过 S02，不能隐去兼容与安全冲突。 |
| P11 | direct API、direct provider、Models、compat 各自捕获请求。工厂暴露 baseUrl/headers 元数据不等于每条调用路径自动应用；是否鉴权、参数覆盖、模型归属与回调次数须逐入口相等。用一个入口通过代替其他入口即失败。 |
| P12 | 选项矩阵每格只允许“应用/忽略/拒绝”，并绑定 request/错误断言。Google/Vertex 拒绝不同于 ambient fetch 的自定义 fetch，显式传入同一个 ambient fetch 可接受；拒绝路径两者调用数均为 0。支持注入的 HTTP 路径只能调用被选中的客户端，不得改全局客户端；WS 单列。 |
| P13 | strict fallback 与普通 retry 分开计数：Anthropic/Bedrock 各按其 status/type/消息及 prefer 工具条件至多降级一次，仅 prefer 变化，require/普通工具/缓存标记/原历史不变。正常降级成功路径 payload hook 两次、response hook 一次；hook 伪错误、流内错误、取消或二次拒绝不再降级。下一独立请求仍先尝试 strict。 |
| P14 | Responses 对未知事件、无 slot delta、未见 added 即出现 done、最终正文/参数覆盖及签名回填逐项匹配。并行 output_index 独立；缺工具 output_item.done 不交出可执行 toolUse。缺失/queued/in_progress/未知 terminal status 按原映射验收，不能套用名称推断。 |
| P15 | Responses changes/additions/none 验实际 wire：additional_tools、删除/仍 unavailable/重声明提示、配对 tool_search_call/output、稳定 call_id、defer_loading、configuration_update 与历史 effort 的位置逐项比较。只比最终工具列表不得通过。 |
| P16 | OpenRouter Images 只处理首 choice；无 choice 或全部图片被过滤可为 stop+空 output；普通 URL 不下载，匹配 data URL 的 base64 文本原样保留。text 在 images 前，模型 modalities 不用于擅自过滤返回 text。无图改报错、额外解码拒绝或转码均为差异。 |
| P17 | System One/Decisions 的 wire 答案按原版有限数及必要形状验证：有限但越界的概率、和不为 1、额外答案、重复 name、prototype-sensitive key 单列；不得自动 clamp/normalize。缺必需答案失败，已读取 usage 保留。图片 assets/images/state-image-urls/Decisions/llama 分别使用原接受范围。 |
| P18 | llama.cpp：choice 数 2–62、score 数 2–10、temperature 有限且>0、图片拒绝，所有题目在联网前验证。逐问题串行；同题 tokenize 与 apply-template 按允许的并行偏序比较，completion 等待两者，token 缓存命中可省略探测。标签单 token/唯一性、缓存及失败驱逐、深度 max(256,16×标签数)→4096→32768、全下溢错误均比较。choice 同概率取首，score 为期望，confidence 为 clamp((n×peak−1)/(n−1),0,1)；不得替换为文本生成/argmax score 或虚构 usage。 |

**7. 鉴权、目录、费用和周边能力都是正式验收范围。**

| 编号 | 鉴权与凭据验收条件 |
|---|---|
| A01 | 显式凭据、持久凭据、provider auth、环境值及云原生鉴权的优先级逐入口匹配；已有 OAuth 凭据刷新失败不能偷偷回落到环境 key。 |
| A02 | CredentialStore 的读取对象身份/别名、修改、删除、同 provider 串行和不同 provider 并行符合契约；modify 返回 undefined 表示保持原值，不是删除；排队取消与执行中取消分别验收。不能假设它与 ModelsStore 一样返回深拷贝。 |
| A03 | getAuth 在 now+max(300000,minValidity)≥expires 时刷新；目录 refresh 只在已过期时刷新。剩余 1 分钟的同一 token 必须产生不同刷新次数。刷新独立超时 15000ms；默认刷新后不再强求余期 5 分钟，显式 minValidity 则重新检查。取消等待不得撤销已开始轮换所需的 Store 写入。 |
| A04 | 每个 OAuth provider 的登录/刷新、PKCE/state、回调或设备码、拒绝/过期/轮询、手工输入、URL 验证、取消和清理均有模拟服务测试。不能只验收拿到一次 access token。 |
| A05 | 凭据 key、provider/account ID、过期单位、额外字段、refresh token 和派生鉴权可往返。缺失/损坏记录按入口逐形态指定接受、过滤、回退或失败；CLI 的损坏 JSON 回退等不能统一改为严格报错。任何未登记的拒绝或静默改变均失败。 |
| A06 | 浏览器/Node/Bun、云凭据和回调服务所需宿主能力有对应清单；不支持的目标不得承诺全量等价。CLI 登录成功之外还须验收失败、取消及进程退出。 |
| A07 | checkAuth/available 不因查看 OAuth 状态而刷新 token；list 不暴露 secret 或执行 key command。API-key handler 的 check 缺省回退与按字段补齐环境配置须分别验证，不能全局禁止或全局强制读取环境。 |
| A08 | callback 的错误 path/method/state/code、重复回调、端口占用、换 token 失败有精确状态码；手工输入与回调竞争只产生一次结果。device flow 的首次等待、slow_down、轮询期限和取消逐项验证。 |
| A09 | lazyOAuth 构造不加载，元数据立即可读；login/refresh/toAuth 并发共用一次加载，加载拒绝被缓存。比较调用数、传参及失败复用；擅自自动重载失败 loader 即不等价。 |
| A10 | 每个 Store 明列内存/持久、best-effort、进程内/跨进程锁和取消提交点。InMemory 不要求耐久；CredentialStore 允许其文档规定的 best-effort Adapter。按其声明故障注入，不能把所有 Store 强加为跨进程事务存储，也不能把弱持久性宣称为强持久性。 |

| 编号 | 模型目录验收条件 |
|---|---|
| M01 | 全部模型 ID、类型、provider、API、base URL、输入/输出能力、上下文限制、价格/阶梯、thinking map、sampling、compat 和图片限制逐字段比较。 |
| M02 | 目录来源、修正、family、finalize/finalizeAll 顺序固定；相同来源、生成选项、generatedAt 与 locale 下逐字段及产物字节可重现。时间未控制时只允许单列时间字段差异，不能忽略价格/顺序等其他内容。 |
| M03 | 模型查询的顺序、provider 查询抛错的处理、空列表、同名不同类型、静态与动态 overlay 的替换位置/追加行为匹配。 |
| M04 | cache read/write 隔离、ETag 原样含引号、Last-Modified/checkedAt、304、过期与离线恢复均有用例；allowNetwork=false 的路径必须证明确实零联网。 |
| M05 | 刷新时先缓存恢复再鉴权/联网、按 provider 汇总错误、取消处理均匹配；删除、替换、clear 或较新刷新之后，旧结果不得晚到覆盖。 |
| M06 | 分别验证写入失败、写前取消、写入中取消、写完后 generation 过期及内存发布。Models 先 await Store 写入，再检查取消/generation 后更新内存；已发生的写入不承诺自动回滚。记录 Store 与内存两个最终状态，擅自要求/假装跨二者原子事务均失败。 |
| M07 | provider 专用目录规则单列：Radius 的远程目录替换 baseline，空目录不能自动回退；旧凭据内嵌目录迁移需验收。OpenRouter 个性化目录按实际入口验收过滤范围、无 key 和不同 HTTP 错误；不能假设 factory 自动调用该能力。 |
| M08 | 目录 manifest schemaVersion=6，文件集合、structureHash、各文件 hash、generatedAt 必须符合原检查器；缺/多 JSON、跨 API 重复 identity、type/id/api/provider 错配分别拒绝，同 ID 不同 type 可共存。data-only/json-only/pretty/非法参数及 keyed chat、all array 输出各有精确案例。 |
| M09 | 静态目录、builtinProviders 新建实例、Models 注册集合及图像 provider 子集分别验顺序和未知 ID；三类 getAll 顺序固定。Copilot availableModelIds 合法空数组过滤掉 chat，缺失/不合法数组保留，未声明全类型过滤时不删除 image/classifier；派生分类 endpoint 另验。 |

| 编号 | Usage、费用及时间验收条件 |
|---|---|
| U01 | input/output/cacheRead/cacheWrite/cacheWrite1h/reasoning/totalTokens 按协议转换；reasoning 是 output 子集时不能再累计，供应商 totalTokens 不能无条件重算。 |
| U02 | 阶梯边界覆盖前/等/后；原版按严格大于阈值选择阶梯，输入门槛包含 input+cacheRead+cacheWrite。所有价格单位和缓存时长分别校验。 |
| U03 | provider 真实账单总额与 catalog 估算区分，totalFromProvider、实际响应模型、服务档位及 calculateCost 的可观察修改行为保持一致。 |
| U04 | 失败前已取得的 usage 按入口保留；单次请求内转换/转发不得额外累计。pending/deferred poll 不计 token/cost，但每次返回完成结果的 fetch 按原版逐请求计量，顺序复用 final 不代表跨请求去重。普通重试按原 span 边界比较；工具执行 usage 不混入模型上下文记账。 |
| U05 | timestamp 的 wall clock 与 duration 的 monotonic clock 分开；非负、舍入、已有 duration、不属于该流开始的 deferred 结果和迟到事件的处理匹配。 |

| 编号 | 周边公共能力验收条件 |
|---|---|
| X01 | 图像与分类不支持的模型/能力、输入图片边界、错误时 usage、responseId 与结果对象全部覆盖；图像生成不能以聊天图像输入测试代替。 |
| X02 | token/context 估算、overflow 识别、retry 分类、文本/identity/UUID/诊断工具及 faux provider 均有原公共入口映射。faux 的排队、追加、消费和错误脚本可重复。 |
| X03 | 遥测 span 父子关系、开始结束、状态、计数、费用和长度单位匹配；不得把原版 aborted span 的状态擅自改变，或把 UTF-16 字符统计改成 UTF-8 字节。 |
| X04 | CLI help/list/login、provider 选择、提示、工作目录 auth.json、既有凭据合并、stdout/stderr、退出码与资源回收匹配。损坏 JSON 当前按空对象读取，安全修正应登记差异；POSIX 0600 与 Windows 权限分别验收。不纳入 coding-agent 的完整 CLI。 |
| X05 | 会话清理的注册/注销、指定会话/全局清理、继续执行其他清理并最终汇总异常的行为一致；重复关闭和失败清理不能遗留不可解释的后台资源。 |
| X06 | 类型专用入口与运行时入口分别映射：当前 oauth barrel 仅类型，bun-oauth 提供静态 flow loader 注册，bedrock-provider 是专用加载入口。不能凭子路径名称臆造原版运行能力。 |
| X07 | Cloudflare 占位符替换的 env 优先级、缺值保留、无替换返回同一 model、有替换新对象均匹配；cloudflareStreams 的能力收缩与 OpenCode 包装器的能力保留分别验证。OpenCode 任意大小写已有 session 头（含 null）阻止注入，不能统一改成自动补头。 |

X02 及通配公开入口按下列独立条目展开。它们必须有自己的输入/输出及状态断言，不能因为主推理路径调用过这些函数就算覆盖。

| 编号 | 公共辅助能力的通过条件；任一不满足即失败 |
|---|---|
| G01 | estimateTextTokens 为 ceil(UTF-16 长度/3.5)，每图按 4800 字符；system 的正文和工具增删分项计数。calculateContextTokens 使用 truthy totalTokens，否则 input+output+cacheRead+cacheWrite。空串、非 BMP、空图文和工具结果分别有精确数值样本。 |
| G02 | estimateContextTokens 只选非 error/aborted、usage>0、timestamp 不早于此前 prefix 最新时间的 assistant；之后逐消息加 trailing。tokens/usageTokens/trailingTokens/lastUsageIndex 四字段逐值相等；较新前缀令旧 usage 失效的反例必测。 |
| G03 | overflow 真值表先排除 rate-limit/throttling，再测模式；通用413前缀与Cerebras专用bodyless400/413模式分别测，前者也可对其他provider生效。silent stop 为 input+cacheRead>window；length 为 output=0 且 input+cacheRead≥99%window，不含 cacheWrite；window缺失/0不启用这两分支。recoverable length 另验 desiredMaxOutput>0 且 output 小于它。各阈值测前/等/后。 |
| G04 | getInitial 只看首条；getDeclaredTools 忽略 removals、最后定义替换且保留首次位置；相同定义重申不算 hasToolRedefinitions，却算 non-additive。声明键序、contentText 分隔、system/update framing、空/null section、ownedSlice 的 UTF-16 负索引/边界分别精确比较。 |
| G05 | withClientIdentity 覆盖所有请求操作，逐字段满足 request>wrapper>Models；undefined 跳过、attribution=null 清除；非请求方法保持委派，MutableModels 仍可修改。UA detailed/short/custom、版本、provider 匹配方式及所有 attribution/session headers 逐项比较，不能把 OpenRouter 子串匹配换成 hostname 匹配而不登记。 |
| G06 | Faux set/append 的队列顺序、stream 调用时同步消费和 callCount、随后 onResponse/工厂时点、空队列错误、工厂参数及异常相等。结果覆盖 api/provider/model 并重算 usage；Faux 的 UTF-16÷4估算及 session 前缀缓存独立于 G01，不能共用÷3.5预期。 |
| G07 | Faux deferred 提交即消费脚本但不执行工厂；有限 pendingFetches 向下取整并截到≥0，缺省为0；NaN/±Infinity 按 Math.max(0,Math.floor(value??0)) 及后续>0分支单列，不能统一拒绝或归零。ready fetch 才执行工厂；顺序重复 fetch 复用 final。handle 三元身份、计数、未知 ID、cancel 的克隆记录及工厂 options 均比较。并发 ready fetch 另录制，原实现没有 await 前的 final 占位，禁止未经证据承诺全局恰好一次。 |
| G08 | shortHash 按 UTF-16、32 位溢出及 base36 精确比较；StringEnum 保留 values 的顺序/重复/引用和 truthy 注解省略；UUID 显式 timestamp 不推进普通调用时钟，序列耗尽/失败后状态有边界证据。PI_AI_VERSION 与 Rust 产品版本分别记录，不得静默互换。 |
| G09 | diagnostics 的 Error/非 Error、空 message 回退、code 类型、stack、details 引用及 append 新数组逐项匹配；它不承诺自动脱敏。ModelsError 的 code/cause/message 与 error-body 的状态提取、去重/截断分别录制。默认采集输出的敏感值保护另按 S01 判定。 |
| G10 | operationSignal、raceWithAbortSignal、combineAbortSignals、sleep 分别测无/单/多 signal、预先取消、竞争、reason、listener/timer。停止等待不取消原任务；迟到拒绝仍被观察；合并 signal 首个生效原因及 cleanup 幂等符合原版。不得将原 helper 的时序或监听残留无记录地改成统一语义。 |
| G11 | Codex debug getter 返回副本；reset 清 stats/SSE fallback 标记，close 关 socket/idle timer；session 缺省/空串/具体值及重复调用分别验收，close 不等价于 reset。Bedrock 注入覆盖 lazy 加载的选择时点；Cloudflare binding 缺 fetch 构造即 TypeError，构造时捕获/绑定方法，后来修改 binding.fetch 不影响 wrapper，method/query/headers/非JSON与流body不重编码或预缓冲。 |
| G12 | simple-options 覆盖 contextWindow≤0、4096余量、ceil(15%×输入)、32768边界、显式 maxTokens=0；不统一规定输出≥1。thinking 默认1024/2048/8192/16384；clampThinkingBudgetToAnswerRoom 的 min(budget,max(0,ceiling−1024)) 公式，与 adjustMaxTokensForThinking 仅在 maxTokens≤thinkingBudget 时调用它，分别验收。caller cap缺省、modelMaxTokens=2000、minimal=1024时返回2000/1024，只剩976回答空间，不能强求所有路径余留1024。缺省/显式 cap与sampling按model→有效thinking level→request覆盖均验；所有公开预算helper直接调用也有样本。 |
| G13 | headers/provider-env/proxy、provider 定价/Radius config、OAuth page、公开协议转换器按具体符号建表；输出文本/HTML/schema 的字节、数字和未知输入策略有 fixture。源码导出但 package 不可达的 helper 只作为内部证据，不为凑清单增加 Rust 公共入口。 |

**8. Rust 模块结构以职责、依赖和可验证性验收，不以 crate 数量验收。**

Module 是拥有明确 Interface 和内部 Implementation 的职责单元。Interface 包括类型、错误、顺序、生命周期、配置与性能约定。Seam 是能替换行为的位置；Adapter 是满足该 Interface 的具体实现。目标是少量稳定 Interface 隐藏足够完整的职责，使变更和验证集中在负责的 Module 内。

下表是逻辑职责划分，名称及合并方式可以变化；不是要求照此创建目录或拆成同样数量的 crate。

| 逻辑 Module | 对外保证 | 结构验收要点 |
|---|---|---|
| 领域类型与数据契约 | 消息、模型、工具、usage、错误和选项语义清楚。 | 不依赖具体 SDK、网络客户端、OS 或全局注册表；封闭状态可穷尽，开放标识可扩展。 |
| Transcript 与纯语义 | 正规化、历史回放、工具变换、费用和能力判断一致。 | 不隐式联网或读凭据；原始历史与派生请求的所有权清楚；单一规则没有多份竞争实现。 |
| 流与生命周期 | 内容块、终态、迭代/result、取消及清理有完整契约。 | 每种状态和资源有明确所有者；协议临时状态不泄漏。不得用一个巨型共享对象承包所有职责。 |
| Models 运行入口 | provider 管理、模型选择、鉴权编排、刷新、操作路由。 | 调用方无需了解 SDK、刷新锁和 registry；不包含各供应商 wire 字段处理。 |
| 协议 Adapter | 一个协议的请求转换、响应解析、特例及错误转换。 | 原始 wire 变化集中在本 Adapter；共用机制可复用，真实差异不得强行统一。 |
| Provider 配置 | 模型、endpoint、鉴权来源和 compat 默认值。 | 复用已有协议的新 provider 不要求复制流状态机；协议与品牌不被绑成一一关系。 |
| 传输与宿主 | HTTP、WebSocket、代理、云 SDK 桥接、时钟等外部效果。 | 真实外部依赖和测试替身位于明确 Seam；SDK/运行时类型不无必要地进入核心公共类型。 |
| Auth 与 Store | 凭据来源、轮换、原子修改及缓存持久化。 | 不让每个 Adapter 重写刷新并发；调用者取消与必须完成的落库职责可区分。 |
| Catalog | 固定目录、动态目录、生成物与缓存版本。 | 目录数据和网络刷新不强迫进入仅使用核心/faux 的依赖图。 |
| Telemetry 与 compat/CLI | 观测、旧语义适配与终端交互。 | 兼容逻辑集中且可独立验证；新核心不反向依赖旧全局入口，遥测不开启也能工作。 |

| 编号 | Rust 工程硬性门槛 |
|---|---|
| R01 | 提交生产/构建/测试三张依赖图及允许依赖表；生产跨职责依赖无环，领域/纯语义到具体 SDK、CLI、compat、目录数据的禁止可达路径为 0。最小核心+Faux 在这些可选依赖缺席时通过构建/离线调用；缺图、禁止边或隐式加载即失败。 |
| R02 | 每类消息、模型、终态、错误和三态字段有类型映射。静态入口的正例编译成功，错误模型/内容/参数类型负例编译失败；动态入口则按原错误通道拒绝。开放 provider/API 和扩展参数有成功正例；用全局任意 JSON 绕过所有约束或关闭原扩展空间即失败。 |
| R03 | 每项任务/缓存/连接/计时器/锁均记录创建者、持有者、取消传播、提交点、关闭条件及可保留上限，空白项为0。同provider序列化时，独立provider的受控操作仍可完成；声明支持的回调重入/关闭竞争不得死锁。CredentialStore.modify回调等待同provider的modify等原版自等待场景，单列并有界观察pending；若Rust改为拒绝，须登记语义差异。异步锁跨等待不一律禁止，须通过相应时序用例。 |
| R04 | 每个 public 符号有原公开能力或真实消费者对应；每个 Trait/Seam 有已使用的变化场景或生产/测试 Adapter。仅供测试查看内部状态的 public 入口为 0。纯转发入口须有兼容/公开契约依据；没有依据的重复抽象或重复公开操作不能通过。 |
| R05 | 结构化错误记录类别、阶段、原因和是否可重试；兼容文案可单独映射。全部输入/网络故障 fixture 不触发非预期 panic/进程退出；错误来源链、取消和远端失败不混淆。unsafe 每处有必要性、不变量和适用检测结果；漏审一处即失败。 |
| R06 | 外部消费测试添加一个自定义协议 ID及两个同协议不同 provider，均可注册、调用及替换。除新增适配器、登记/feature/目录和测试外，既有领域类型、其他 Adapter 与消费者无需修改；不能满足即判扩展性失败。新增全新业务能力不用于这项探针。 |
| R07 | 对每个生产 Store/Transport Adapter 及对应测试 Adapter 运行同一 Interface 契约套件；预期值相同，只有事先声明的能力/持久性差异可按矩阵分支。生产与测试 Adapter 需要两套相互矛盾的业务 expected 即失败；测试不可绕过被验收的职责。 |
| R08 | 公开符号清单中每项具备默认、有效输入、错误阶段、取消/drop、并发和支持条件说明；至少一个外部使用正例覆盖每个方法族。文档示例从发布包编译，内部路径引用为 0；声明与编译/执行结果任一不符即失败。 |
| R09 | 核心/最小配置无隐式外网、浏览器启动、凭据刷新、文件写入、stdout/stderr或进程退出；有意副作用只能出现在清单声明的CLI/compat/实际操作入口。仅当所有相关有效字段已显式提供、且原契约不再回退环境时，空环境与相反进程env的结果必须相同。部分配置仍按原字段规则补齐；例如只给Cloudflare key而account/gateway取env时，须验证预期变化，不能强求环境无关。 |
| R10 | 列出所选运行时、线程模式及 public 类型的 Send/Sync、借用有效期和 drop 承诺；每个承诺有编译或受控运行证据。不得强制所有对象 Send/Sync，也不得在文档宣称可跨线程但实际不能使用；声明支持的任务模式不能依赖隐藏全局运行时。 |
| R11 | 公开领域 Interface 不含具体供应商 SDK/传输私有类型；原本公开的协议专用转换能力可在对应协议 Interface 表达，不能因此漏功能。SDK 版本变化的兼容探针只影响相应 Adapter/兼容面；SDK 私有类型进入通用消息/模型参数即失败。 |
| R12 | 生产包不依赖dev-dependencies才能编译；发布验证集合V的每个配置均通过T04/T11。关闭能力后其不必要依赖不进入依赖闭包；feature合并不改变无关provider请求快照。未定义/互斥组合按已声明规则拒绝，不能静默启用另一配置。最小配置不能代替覆盖原包全部能力的主验收配置。 |

发布前冻结有限的**发布验证集合V**，每行是精确的OS版本/arch/运行时/features配置。另列“组合规则允许但未逐项验证”和“不支持”的配置；前者只能标为未验证，不能宣传为已通过。V至少含一个覆盖原包全部纳入范围能力的主配置，并满足T04的最小组合覆盖；因最小profile关闭能力不代表原能力可从总验收范围删除。若声称任意有效组合均已验证，就必须提供该有限组合全集的逐行记录，pairwise不能替代它。

Cargo feature 会发生依赖图内的合并，因此只测 `all-features` 不够。最小功能、默认功能、每个provider/运行时的独立组合和选定混合组合须纳入上述矩阵；开启一个无关provider不得改变既有provider的默认行为。[Cargo features 官方说明](https://doc.rust-lang.org/cargo/reference/features.html)

**9. 测试必须按证明目标分层，而不是按源码文件逐个照抄。**

Rust 自带的单元测试、集成测试和文档测试是组织方式；下表是本项目的验收层次。私有机制可有单测，但公开兼容性必须由外部调用者使用相同 Interface 来证明。[Rust 测试组织说明](https://doc.rust-lang.org/book/ch11-03-test-organization.html)

| 层 | 证明什么 | 测试对象与替身 | 通过标准 |
|---|---|---|---|
| L0 基准与契约 | 没漏范围，预期不是 Rust 自己编造的。 | 冻结 TS、原测试、原始响应/请求录制、导出/能力清单。 | 每个契约可追踪到证据；原版 fixture 来源、环境和规范化规则可验证。 |
| L1 类型与纯语义 | 数据及确定性规则正确。 | 编译通过/失败示例；真实 transcript、schema、cost、JSON、ID 等逻辑，无网络。 | 全部正例编译/执行通过，静态负例编译失败；动态负例走已记录错误通道；每项精确 expected 相等。 |
| L2 Module 契约 | 一个完整职责对外行为正确。 | Stream、Auth、Models、Store 等真实 Module；外部依赖受控。 | 状态迁移、失败、取消、回调、输入隔离均通过；不直接操纵私有字段制造成功。 |
| L3 协议与传输 | 实际发包和流解析正确。 | 真正请求/解析逻辑；本地 HTTP/WebSocket/协议服务、录制响应及故障注入。 | 各协议分帧、SDK 默认值、超时、错误体、重试均通过。仅 mock SDK 返回值不算这一层。 |
| L4 双实现差分 | 原版与 Rust 外部行为相同。 | 同一 fixture、动作和受控时序分别驱动 TS 与 Rust 的公开入口。 | 请求、回调、事件、result、状态、副作用均按预定规则相等；零未解决差异。 |
| L5 跨 Module 与消费者 | 单独正确的职责组合起来仍正确。 | Models→auth→provider→transport→stream/result→telemetry；CLI 子进程、faux、目录服务；真实消费者的调用模式。 | 多轮工具、跨 provider、动态刷新、延迟任务和关闭流程通过；不能 mock 掉正在验证的链路。 |
| L6 性质、模糊与并发 | 常规样本之外仍守住不变量。 | 分块变形、随机 transcript、结构化 fuzz、受控竞态、故障注入及必要的内存/并发检测。 | 无crash/UB/新增死锁或非预期挂起；已登记的原版pending按E09/R03有界观察，不让测试进程挂死。关键性质成立；失败有缩减样本并加入回归，语料/种子/预算留档。 |
| L7 发布与资源 | 用户拿到的产物可用且工程指标合格。 | 干净环境中的实际包、平台/features、离线构建、文档示例、性能/耐久负载。 | 发布包通过全部适用公共契约；平台/features 按 T04/T11 执行，不以未定义抽样代替。资源与性能满足 S/Q。 |
| L8 真实服务联通 | 当前真实端点仍接受这些请求。 | 显式启用、隔离凭据/费用的 live smoke。 | 声称“当前可用”的集成须有发布前 7 日内、该发布候选版本的实际 endpoint/model/auth 成功记录；无凭据或超时算未验证。此层不能替代 L0–L7。 |

测试组织应保证：纯语义单测靠近 Module；公共契约、协议和消费者测试从外部导入；fixture、原版驱动、Rust 驱动和报告分工清楚；fuzz/性能/live 不混入默认离线测试。测试可按职责组织，不要求每个源文件有一个同名测试文件。

默认测试不得因开发机存在 API key 就自动产生联网或计费。普通 CI 无真实凭据、默认拒绝非回放网络；live 必须独立启用。既有 185 个原测试文件需按测试案例逐项映射到移植、等价替代、真正不适用或待补，不能只统计文件数量。

关键测试要求如下：

- 对每种状态迁移，至少一条可达 trace；对终态、鉴权、费用、重放、刷新发布、取消清理中的关键决策，同时覆盖允许和拒绝分支。
- 对有限枚举及阈值穷举；复杂组合先按等价类覆盖，再补已知高风险交互。pairwise 不能替代账号×会话×取消×刷新这样的已知风险组合。
- 对 JSON/文本分片：短样本覆盖每个二分切点、单字节切分；长样本覆盖固定种子的多段随机切分。公开 preview 按相同 chunk 序列差分，最终结果按适用不变量比较。
- 对非确定时序：控制屏障及顺序，覆盖取消前提交、提交前取消、双方同时就绪、晚到结果；固定随机种子并不足以固定任务调度。
- 属性测试需明确性质的前提，例如“合法分块方式不改变最终消息”“规范化幂等”“工具 ID 引用一致”“取消后没有禁止的发布”。不能对原版没有保证的性质强加断言。
- 契约判等器必须自带负例：故意删一个事件、改工具 ID、掉 usage、把 null 改缺失、重排 system，必须被判失败。原版与 Rust 共用的是输入和比较规范，不能共用被验证的语义实现。
- 关键 mutation 必须被击杀：`>` 改 `>=`、漏终态、漏签名、重复计费、错误鉴权 fallback、取消后旧刷新发布。覆盖率再高也不能放过这些缺陷。
- 代码覆盖率是发现遗漏的工具，不作为等价性证明。所有适用契约、关键状态迁移、历史回归族和关键 mutation 的覆盖必须为 100%；行覆盖率不能替代这些分母。

下列最低测试门槛是本项目新增的工程验收条件，不是对上游测试覆盖程度的描述；所有数量只约束最低执行量，不构成正确性的数学证明。

| 编号 | 可判定的测试边界 |
|---|---|
| T01 | 覆盖分母冻结为：B05 的公开符号/重载、原185文件中展开 each 参数后的案例、能力矩阵有效行、有限状态迁移表、历史回归族、具名关键 mutation。每项至少关联一个实际断言；分母缺漏、仅关联空测试、运行后删失败项均失败。 |
| T02 | 每个公开操作覆盖缺省、最小有效、一般有效、边界及非法输入等实际可达等价类；每个 threshold 测前/等/后，每个 enum 测全成员。源码没有对应类别须给可达性理由，不能把 Rust 不易表达的 JS/wire 输入直接删去。 |
| T03 | 每个协议至少覆盖 setup、头等待、首内容前、内容中、终态前/后、清理阶段故障；每个有取消的阶段覆盖预先取消、执行中取消、完成后取消。验证外发请求/回调/事件/result/Store/资源，而非只看一个返回值。 |
| T04 | V在适用平台/运行时上至少包含no-default、default、每个独立可选能力、各兼容能力组的全开配置和覆盖任意两能力有效交互的配置；已知3个以上交互另列强制配置。两两覆盖只是最低覆盖，不证明所有组合；规则允许却未执行者标未验证。不支持组合须可重现地拒绝，不能跳过后算通过。 |
| T05 | 每条协议的典型合法与 malformed 短帧（≤4096字节）覆盖全部二分切点和单字节切分；更长样本至少100个固定种子多段切分，覆盖 UTF-8/CRLF/转义/事件分隔/EOF。输入字节和切分计划相同才比较同粒度轨迹。 |
| T06 | 每个有状态职责至少覆盖两并发调用、两个独立 provider/账号/会话、同键竞争，以及操作在提交屏障两侧的所有已枚举顺序；分开正常完成、失败、取消和 drop。原版未承诺恰好一次的路径先录制，不凭 Rust 结构强推结果。 |
| T07 | 每个 parser/validator/协议帧 fuzz 目标至少累计30 CPU分钟，且相关属性族至少10000个有效生成案例；记录版本、种子、语料hash、最大输入/深度、执行数和超时预算。crash、非预期panic、越界、挂起或性质失败未关闭即失败；支持工具缺失只算受阻。 |
| T08 | 判等器负例至少包含删事件、改contentIndex/工具ID、丢usage、null变缺失、重排system、改header覆盖、丢错误体、改partial别名。每个适用负例必须导致失败，漏检任一项即不得采用该比较器签字。 |
| T09 | 关键 mutation 清单中的每个非等价变异都必须被测试击杀；等价/不可达判定需独立复核和证据。契约、状态迁移、回归族分母覆盖均为100%，不可用整体行覆盖率替代，也不可因 mutation 工具超时记作击杀。 |
| T10 | 每个差分案例先在固定输入/时序下重复 TS 至少3次，确定唯一输出或有证据的允许结果集；Rust 必须满足同一集合。失败不得靠自动重试变绿；重跑只用于诊断，保留首次失败和所有原始结果。 |
| T11 | V每一精确配置至少运行适用的公共入口、序列化、取消/资源、代理/凭据发现和CLI测试，主验收配置运行完整L1–L7。声明仅按组合规则可用的其他配置仍标未验证；不能由某OS或feature单项通过推导其笛卡尔积全部通过。V有缺测行即发布验证不通过。 |
| T12 | 原版录制、Rust驱动和判等器分别标版本；共享输入可以，共享被验证的转换/解析逻辑不可以。每个报告保留命令、环境、退出码、运行/跳过/受阻数、fixture与结果hash；结果不完整或报告与原始日志不一致即失败。 |

**10. 以下反例必须进入首批验收样本，防止标准流于形式。**

| 案例 | 输入或时序 | 必须验证的结果 |
|---|---|---|
| K01 准备失败 | 分别调用 direct API 与 Models，令鉴权缺失。 | 保留返回流前失败与流内 error 的入口差异；不伪造 start。 |
| K02 保留 partial | 对真实 Adapter 与 Faux 分别保存 start.partial、content数组、已开始块及arguments，在新增块/delta后读取。 | 按各入口别名图比较；Faux旧start可仍为空而已开始块继续增长。统一要求全部实时或全部快照均失败。 |
| K03 取消后凭据轮换 | 刷新已经开始，再取消一个等待者，服务端返回新 refresh token。 | 等待者取消结果正确，新凭据仍按原规则持久化，其他等待者不被错误破坏。 |
| K04 旧目录晚到 | 刷新 A 在网络/写Store两个阶段分别挂起，完成 B 或替换/删除 provider，再放行 A。 | A不能晚到发布内存或复活provider；分别记录已发生的Store写入，不要求原版未提供的自动回滚。 |
| K05 成本阈值 | 总输入量分别为阶梯阈值−1、阈值、阈值+1，含缓存 token。 | 严格大于条件、cacheRead/cacheWrite 门槛贡献和各费用分项正确。 |
| K06 参数 coercion | 验证 integer `"42.1"`、required string null、optional 非 nullable null、nullable union。 | 分别匹配原版 42、`"null"`、省略及保留规则；原输入不变。 |
| K07 半截工具 JSON | preview 已产生部分对象，但最终文本不能合法完成。 | 不能因为 preview 可读就把工具调用当作合法参数；argumentsError 与后续验证失败正确。 |
| K08 跨模型重放 | 含签名 thinking、工具调用和结果的历史切到不同 model/provider/API。 | 丢弃外来 thinking，按规则处理签名，工具 ID 与结果仍匹配。 |
| K09 终止时点 | Codex 等有提前结束契约的协议发送终态后保持连接；另一例无终态便 EOF；Google 在 finishReason 后还有 usage。 | 提前结束协议及时完成，缺终态按协议报错；Google 保留后续 usage 并匹配原 done 时点。 |
| K10 超时和重试差异 | Mistral 头已到但长流仍输出；Decisions 返回 504；pi-messages 返回失败。 | 分别按各入口的 timeout/retry 契约处理，不施加统一重试/超时语义。 |
| K11 签名和图像 | 空但有签名的 thinking、redacted 块、非视觉工具结果图片。 | 不用“空字符串清理”破坏签名，不因统一过滤丢失原占位和回放行为。 |
| K12 字符单位 | 非 BMP 字符、组合序列、孤立 surrogate、跨 chunk 的 Unicode 转义。 | 协议文本、工具参数、清理和 telemetry 的单位各自正确；不以 Rust String 已合法为由跳过。 |
| K13 账号和共享状态 | 两账号同sessionId/provider/baseUrl/model/历史，比较Anthropic续接、Codex socket及stats/fallback；一方取消或close/reset。 | 记录各状态实际作用域；socket隔离不证明其他状态隔离。兼容轨迹与安全判据分别判定，待验证风险不得写成已证明漏洞或已安全。 |
| K14 轻量使用 | 仅使用核心类型和 faux 完成请求；不提供模型目录、云凭据或 SDK。 | 对应最小构建可用，没有隐式网络、全局登录、目录加载或与本次调用无关的隐式后台工作；调用结束并满足清理条件后没有未声明残留任务。 |
| K15 空图像成功 | OpenRouter返回空choices，或只有普通URL图片。 | stop+空output匹配；无隐式下载/额外报错。 |
| K16 非归一化分类 | System One/Decisions返回有限但越界或和不为1的数值，另测llama同概率标签。 | 前两者保持各自接受规则；llama使用其转换并取首个并列choice，不统一归一化。 |
| K17 两种注册表 | 先注册自定义同API实现，再显式注册built-ins。 | 聊天保留已有项，图像覆盖；不借用同一预期。 |
| K18 OAuth时间窗口 | 同token还剩1分钟，分别getAuth与refresh目录；另给显式minValidity。 | 默认请求鉴权触发刷新，目录不触发；刷新后检查按是否显式minValidity区分。 |
| K19 低层流回调失败 | extractResult抛错，两个消费者正等待；随后显式end。 | 精确记录done/result/event/waiter的状态，pending用有界观察验证，不能靠测试进程挂死算通过。 |
| K20 Runtime schema | 比较单API schema与ProviderCompatSchema、开放对象及default注解。 | superset不重新注入已删除的API特定default；schema值和验证行为同时相等。 |
| K21 Strict降级 | prefer工具触发特定400拒绝；另由hook抛相似错误或第二次拒绝。 | 合法路径只降级一次且回调次数匹配，伪触发/二次拒绝不继续重试。 |
| K22 Deferred并发 | 同Faux handle两个ready fetch同时进入工厂；另测顺序重复fetch和无能力API。 | 并发以原版受控轨迹为准，顺序复用final；无能力按原通道失败，不制造通用deferred支持。 |
| K23 Context估算失效 | 旧assistant usage之前插入timestamp更新的system/压缩前缀，再追加工具结果。 | 旧usage不再代表当前prefix，四字段估算及工具结果记账匹配。 |

**11. 安全、资源、性能和交付必须有独立门槛。**

| 编号 | 验收条件 |
|---|---|
| S01 | 用唯一合成secret标记注入凭据，检查默认采集的日志/遥测/诊断、fixture及发布产物，未授权位置命中数必须为0；显式调试原文能力单列。diagnostics原始helper不自带脱敏，若默认采集链暴露标记，安全门失败并登记兼容冲突，不谎称上游已脱敏。 |
| S02 | OAuth/state、可信URL、token轮换、跨账号session、代理/重定向的具名负例全部满足安全预期：无跨账号凭据/内容复用、无不允许的回调接受或凭据转发。P10/K13已定位的状态作用域必须完成受控验证；若原版轨迹与安全预期冲突，两项不能同时标通过，修复须显示有意差异或更换明确基准。 |
| S03 | 在冻结的输入与负载预算内，不可信 wire、模型输出、schema、base64、JSON 深度和长度不会导致 panic、越界、无限循环或超预算资源消耗。新增大小上限若改变原接受范围，须作为显式兼容差异处理，不能用收窄测试输入隐藏差异。 |
| S04 | 释放调用者引用并完成所有已声明必需提交/cleanup后，任务/计时器/监听/锁/文件句柄计数回到基线；连接/缓存只允许清单列明的容量与TTL。各资源类均有计数或等价释放证据，漏一类、超上限或无法完成清理即失败。账号轮换等必需后台提交不按普通drop强杀。 |
| S05 | 正常/错误/取消/停止消费/全不消费五场景，各执行10000次请求生命周期，覆盖并发1/8/32及新session键；周期性明确清理并记录资源计数。无丢事件、串流、非预期挂起；停止输入并释放保留引用后满足S04。全不消费期间允许原版约定的排队增长，不得靠静默丢弃或缩小合法输入取得通过。 |
| Q01 | 固定OS版本/arch、Rust工具链/MSRV、运行时版本及feature组合；Windows实际目标须在V，Linux/macOS和浏览器/云宿主逐项标已验证配置、规则允许但未验证或不支持，不能留空。V满足T04/T11；原包能力被删减者只能给范围限定结论，不能借标不支持逃避完整复刻范围；缺测不能列为通过。 |
| Q02 | 在干净环境、锁定依赖且资产预置后可离线构建。生成数据、证书/宿主依赖、CLI、license/notice 和示例随发布要求完整提供，不依赖开发者私有绝对路径。 |
| Q03 | 格式、编译、静态诊断、公共文档示例及发布包测试通过；告警豁免逐条说明；没有未说明的 unsafe、panic 占位、未实现分支或生产路径测试开关。 |
| Q04 | 依赖、源码与目录数据来源和再分发材料可追踪；安全扫描结果注明日期、版本和适用范围，不能把未扫描写成无风险。 |
| Q05 | 按下表默认负载与阈值执行配对性能测试；环境、工作负载、采样和统计方法在运行前锁定。全部指标通过才通过；超预算、测量范围不一致、结果无效或数据缺失均不通过，不能仅提交“Rust更快”的描述。 |

性能门槛由下表给出默认值，属于新增工程要求，不宣称已经测得。TS与Rust使用同一台受控机器、相同原始fixture和相同输出保留/消费策略，Rust使用发布构建，TS使用锁定运行时；排除真实模型服务延迟。每项先预热5批，再交替运行至少30个配对批次；延迟分位数每批至少1000次操作，冷启动以独立进程30次为样本。保留原始样本，计时低于100个时钟分辨单位时须聚合操作后重测，不得把接近0的分母拿来计算倍率。

| 指标/负载 | 默认通过条件 |
|---|---|
| 最小核心+Faux，1000字符文本，complete与消费完整stream；并发1/8/32 | 每个组合的客户端p95/p99延迟及CPU/请求的Rust÷TS倍率，其单侧95%置信上限均≤1.10；吞吐Rust÷TS的单侧95%置信下限≥0.90。 |
| 各聊天协议的文本流、4个工具调用及错误/取消回放 | 同上；每种路径单独统计，不能以最快协议的平均值掩盖退化。两端相同轨迹的正确性须先通过。 |
| 原长工具参数fixture：25000与250000字符，32字符切片 | 累计分配 large/small<25，large分配字节/实际JSON文本长度<5000；这是原回归测试的门槛。测累计分配而非仅存活堆；跨语言计数范围须包括被测完整调用，差异须解释。 |
| 冷启动及峰值RSS | 相同最小/单provider/完整分发配置下，启动到首个本地有效结果的中位数及峰值RSS倍率的单侧95%置信上限≤1.10；各配置独立通过。 |
| 目录刷新 | 固定10个自定义provider、每个100个模型，分别全成功/一个失败/一个被屏障挂起；独立provider可完成，成功延迟与CPU倍率满足≤1.10。Store写入内容/次数另须匹配契约。 |
| 取消至清理 | 无必须继续提交的本地回放路径，Rust p99≤max(TS p99×1.10,100ms)且单次≤1000ms；OAuth等继续提交路径分别按原15秒等超时及S04验收，不混入普通取消指标。 |
| 长期资源与保留 | S05周期结束满足S04；250000字符工具流及快照释放后无存活请求对象，缓存只留允许项。RSS不立即归还OS不单独算泄漏，需区分allocator保留与仍存活对象。 |

置信区间方法、批次划分和计时范围须在首次候选性能执行前登记并固定；置信区间跨过阈值表示证据不足，不能四舍五入成通过。可以在测试前书面指定更严格的发布预算；放宽默认值属于规范版本变更，必须保留旧结果和理由，不能为已有失败追改预算。

若还声称“Rust 版本更优”，须在此前约定的代表性工作负载中，证明具体工程收益，例如依赖隔离、错误表达、维护变更局部性或资源表现；不能只因语言不同就下结论。行为兼容是前提，性能提升不能补偿行为丢失。

**12. 最终签字必须逐门通过，不能打一个加权总分。**

| 发布门 | 必须提交的证据 | 通过要求 |
|---|---|---|
| 范围门 | 固定源码/数据/环境清单，所有 exports、provider、协议、CLI 和宿主能力映射。 | 无遗漏、无未说明缩减、无未决基准冲突。 |
| 行为门 | 契约清单、TS 原版录制、Rust 输出、比较规则及差异报告。 | 全部适用契约通过；零未解决行为差异。 |
| 结构门 | 实际依赖图、公共 Interface 文档、所有权/生命周期表、扩展与最小依赖验证。 | R01–R12 全部满足。 |
| 测试门 | L0–L7及T01–T12报告、原测试映射、反例、fuzz/竞态/mutation；L8按当前可用声明另标。 | 必测零失败、零未执行/受阻；skip与“不适用”不混入通过，未关闭基准冲突为0。 |
| 运行门 | 平台/features、资源/安全、性能预算与测量报告。 | 无未解决关键安全/并发/资源问题；全部冻结预算满足。 |
| 交付门 | 干净环境中的发布包、示例、版本/变更记录、依赖与数据来源、复验说明。 | 第三方可复现同一验收，不依赖作者机器或口头说明。 |

下列任一情况直接拒收：只比较最终回答；用 live LLM 文本比对代替协议录制；把 skip 算通过；因 Rust 更严格而静默改变合法输入；批量重录 golden 掩盖回归；遗漏工具/分类/图像/OAuth/兼容入口；接口测试依赖私有字段；取消后串账号或覆盖新状态；未知来源的模型价格；无法复现的“全部测试通过”。

原版缺陷或不合理行为应登记为“原版缺陷”“移植回归”“有意改变”“证据不足”之一。可以交付包含明确改进的实现，但应精确描述兼容等级；不能把有意改变从差异报告删除后仍称严格复刻。

**13. 规范所依据的关键源码证据。**

以下定位均绑定前述 commit，路径相对 `D:/Luiz/Odradek/pi/packages/ai`；工作树升级后行号可能变化。它们证明验收范围及已指出的原行为，不证明尚未运行的 Rust 实现合格。

| 证据 | 对应依据 |
|---|---|
| `package.json`；`src/index.ts:1`；`src/types.ts:64` | 导出范围、根入口无副作用设计、聊天/图像/分类协议和开放标识。 |
| `catalog/README.md:1`；`catalog/build.ts`；`src/providers/model-schema.ts` | gitignored 目录数据、生成顺序、模型字段及价格。 |
| `src/types.ts:394`、`:814`；`src/utils/event-stream.ts:26`；`src/api/lazy.ts:59` | 错误阶段、共享 partial、事件顺序、FIFO/result/时间。 |
| `src/api/anthropic-messages.ts:981`；`test/anthropic-sse-parsing.test.ts:481`；`src/api/google-generative-ai.ts:224` | 非空初始内容与注释冲突、供应商停止字段和最终 done 时点的区别。 |
| `src/utils/transcript.ts:10`、`:58`、`:123`、`:200`；`src/api/transform-messages.ts:91`、`:181` | Context 正规化、system/工具变化、声明比较、跨模型及孤立工具处理。 |
| `src/utils/streaming-json.ts:89`、`:378`、`:452`；`src/utils/json-parse.ts:33`；`src/utils/validation.ts:54`；`test/validation.test.ts:63` | preview/finish、快照和复杂度、工具参数 coercion/错误/隔离。 |
| `src/models.ts:417`、`:498`、`:565`、`:637`、`:1094`；`src/models-store.ts:3` | 刷新发布竞态、缓存、取消与凭据、模型 overlay 和存储。 |
| `src/models.ts:1236`、`:1261`；`src/types.ts:466` | 费用阶梯、缓存/reasoning 计数、thinking 档位。 |
| `src/auth/resolve.ts`；`src/auth/credential-store.ts`；`src/auth/oauth/` | 鉴权优先级、刷新及持久化、串行修改、登录协议。 |
| `src/providers/radius.ts:30`；`src/providers/openrouter.ts:25`；`src/cli.ts:19`；`src/oauth.ts:1` | provider 专用目录替换/过滤、CLI 凭据文件和类型专用入口。 |
| `src/utils/provider-retry.ts:107`；`src/utils/retry.ts:209`；`src/utils/headers.ts:11`；`src/utils/node-http-proxy.ts:13` | 两层重试、headers 与代理。 |
| `src/api/openai-decisions.ts:131`；`src/api/google-generative-ai.ts:89`；`test/mistral-http-transport.test.ts:493` | 504 不重试、自定义 fetch 拒绝、响应头超时与长流的差异。 |
| `src/api/pi-messages.ts:184`、`:373`；`src/api/openrouter-images.ts:86`；`src/api/system-one-shared.ts:131` | Pi 事件协议、图片生成、分类和图片输入。 |
| `src/session-resources.ts:1`；`src/compat.ts:129`；`test/lazy-module-load.test.ts`；`test/models-entry.test.ts` | 清理汇总、旧 registry、轻量入口与 SDK 隔离。 |
| `test/openai-responses-terminal-event.test.ts`；`test/openai-codex-stream.test.ts`；`test/anthropic-sse-parsing.test.ts`；`test/bedrock-stream-state.test.ts` | 大型协议专项必须纳入的回归族；本次不将其清点冒充逐行完整审计或执行通过。 |
| `src/providers/faux.ts:366`、`:374`、`:459`、`:548`、`:562`、`:591`、`:626`、`:657`；`src/utils/event-stream.ts:43`；`src/telemetry.ts:199` | Faux分层引用、同步队列消费、独立usage/deferred语义；非有限计数、重复final的逐请求计量；泛型流回调重入和抛错。 |
| `src/models.ts:258`、`:369`、`:525`、`:627`、`:743`；`src/auth/types.ts:61`；`src/auth/helpers.ts:40` | Models方法分母、写入后取消、目录刷新阈值、过滤优先级、Store保证范围及lazyOAuth。 |
| `src/images-api-registry.ts:38`；`src/providers/images/register-builtins.ts:50`；`src/providers/compat-schema.ts:636`；`src/providers/schema-builders.ts:14` | 图像registry覆盖与聊天不同；运行时schema和default注解。 |
| `src/utils/estimate.ts:18`、`:71`；`src/utils/overflow.ts:144`；`src/utils/identity.ts:154`；`src/utils/text.ts:48`；`src/utils/hash.ts:2` | 估算/overflow具体阈值、identity完整委派、ownedSlice及短hash语义。 |
| `src/utils/abort.ts:15`；`src/utils/abort-signals.ts:6`；`src/utils/sleep.ts:1`；`src/utils/string-enum.ts:14` | 取消等待、合并signal、计时器和公开schema辅助函数。 |
| `src/api/anthropic-messages.ts:271`、`:318`；`src/api/openai-codex-responses.ts:931`、`:955`、`:960`、`:970` | continuation/cache/debug/fallback各自作用域；close与reset不相同。 |
| `src/api/openrouter-images.ts:86`；`src/api/system-one-shared.ts:82`；`src/api/openai-decisions.ts:131`；`src/api/llama-cpp-classify.ts:171`、`:181`、`:190`、`:197`、`:403` | 空图像成功、分类接受范围、llama标签概率/置信度/期望值与协议探测偏序。 |
| `src/api/simple-options.ts:26`、`:37`、`:85`、`:107`；`src/api/cloudflare-ai-binding.ts:80`；`src/api/bedrock-converse-stream.lazy.ts:22` | 预算/采样的具体分支、1024余量的条件调用、binding传输和宿主模块注入。 |
| `test/anthropic-strict-tool-fallback.test.ts:69`；`test/bedrock-strict-tool-fallback.test.ts:108`；`test/fetch-option.test.ts:62` | strict降级触发/回调次数/负例，以及fetch应用/拒绝边界。 |
| `scripts/model-data.ts:5`、`:214`、`:276`；`catalog/generate.ts:50`、`:144`、`:273` | manifest版本/文件hash校验、数据分组、生成模式和时间字段。 |
| `test/streaming-tool-arguments.test.ts`；`test/bedrock-stream-state.test.ts:147`；`test/memory-helpers.ts:24` | 25000/250000字符累计分配门槛和释放检查；新增性能比率/测试执行量属于本文工程要求，并非这些原测试的已有结果。 |

本文只制定验收标准，没有更改被审查仓库、执行原包测试、安装依赖、调用付费 provider，或作出 Rust 复刻已通过验收的结论。

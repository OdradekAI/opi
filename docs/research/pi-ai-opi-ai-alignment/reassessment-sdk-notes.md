# SDK 能力、compat 与新 operation 的重新评估

> **非规范性独立研究；2026-10-03。** 本文响应用户的新定位：opi 是 pi 的 Rust 实现；旧 `opi-spec` 在本轮仅作为可能过时的参考，不能据其 core/product 划分否决 pi 已发布的 SDK 能力。本文只记录重评证据和建议，不修改候选正文、现行规范、代码或配置，不授予实施权限。

本轮实际阅读 pi/opi 源码、pi 本地 CHANGELOG/README、现有测试及本目录历史证据。沿用已读取的 research/codebase-design 方法；未执行测试、未调用真实模型、未安装依赖、未查询新的网络规划。旧 Q 的确认记录存在，但若其推荐依据受这次定位纠正影响，应明确修订，不能把原确认当成充分理解新取舍后的再次批准。

路径缩写：`PI`=`.repo/pi-1.0.0/packages/ai/`；`PC`=`.repo/pi-1.0.0/packages/coding-agent/`；`OA`=`crates/opi-ai/`；`OC`=`crates/opi-coding-agent/`。精确快照身份仍以本次研究基线为准；以下 release 时间引用本地 CHANGELOG，不将目录名本身当作 tag 证明。

## 重新评估结论

- **Q8 应重新打开**：把 AWS discovery 仅移入产品会损失 SDK 自带能力；pi 明确把默认凭据链作为库能力，并持续修复其来源/region 优先级。保留 SDK 可用性优先于旧层级规则。
- **Q10 的共享多 operation 方向应保留**：它是 pi 0.99.0 已发布的设计和实际消费者路径；不再需要把图片单 adapter 当作“违反旧 core 门槛”的例外论证。
- **Q14 必须拆分**：图片默认零 retry 接近 pi；System One 默认两次 retry，统一单 attempt 会删掉行为。强制调用方提供总 deadline 也是新策略，不是 Rust 必然要求。
- **Q6 的 compat 清单不能整体删除**：`supportsStore`、`supportsDeveloperRole`、`supportsReasoningEffort`、`requiresAssistantAfterToolResult` 在 pi 当前源码均改变 wire。opi 无读点只能证明未接线，不证明对应能力该退出。允许合并表达，但先证明语义覆盖。
- **Q21 的资源防护原则可保留，固定数字和输入形状应重新审视**：安全 Rust 并不要求 8 张图、64 题或 System One 最多 10 档；这些是候选产品/SDK限制。严格解析与有界内存不是问题，混用不同 adapter 的限制才是问题。

## RS-01 · 高：AWS 凭据链是明确的 SDK 能力，不能仅以旧层级原则移出

### 来源事实

1. `PI/src/providers/amazon-bedrock.ts:7–9` 明确写出 Bedrock 接受 bearer token 或 AWS SDK default credential chain，解析 ambient credentials 且不复制到 pi credential store；`:15–54` 的 provider-owned login 选择 bearer/profile/existing chain；`:64–77` 检测 profile、key pair、ECS 以及 web identity。
2. `PI/README.md:505` 公开说明以上 SDK 行为；`:1688–1707` 说明通过 `amazonBedrockProvider()` 装配，AWS SDK 可延迟加载。这不是仅 CLI 内部的约定。
3. `PI/src/api/bedrock-converse-stream.ts:155–215` 有意让显式 `profile` 或 credential-scoped `AWS_PROFILE` 优先于 ambient access keys；通过不设置 client credentials 让 SDK 执行链。`:173–202` 区分自定义 endpoint、环境 region、profile region 与 ARN region。`:238–249` 支持 bearer auth scheme 并实例化 BedrockRuntimeClient。
4. `PI/test/bedrock-credentials.test.ts:71–110` 离线 mock client 验证显式/scoped profile 优先及仅 ambient profile 与 env keys 并存的不同规则；`bedrock-endpoint-resolution.test.ts:122–200` 覆盖 profile/ARN/bearer。这里只阅读，未运行。
5. `PI/CHANGELOG.md:525` 将 provider-owned Bedrock auth flow 列为 0.80.8 的 Added；`:650`、`:722` 记录 scoped profile 修复；`:1768` 记录 ECS/Kubernetes credential detection。反复维护与公开说明共同支持“有意能力”，不是 SDK 偶然被加载。
6. pi package manifest 固定 `@aws-sdk/client-bedrock-runtime` 3.1127.0（`:71`）；根 lock 包含 node/env/ini/process/SSO/web-identity credential providers。依赖图本身不是每个来源实际可用的运行证明，但与代码明确委托 SDK 的意图相符。

### opi 当前差异、反证与影响

`OA/src/bedrock/credentials.rs:143–307` 的公开 async resolver 是 SDK 已有可复用能力；`OC/src/provider_factory.rs:2037–2120` 产品使用它。现有实现只做 explicit/env/local profiles/credential_process，`:212–214` 明示不执行 IMDS、SSO 或 web-identity 网络链；`:536–660` 有 process timeout、输出上限和清理机制。
`OA/src/bedrock/mod.rs:223` 接受 AwsSigV4；本轮未发现相同 Bedrock 路径的 bearer 支持，因此“移动且保留当前行为”也不等于对齐 pi 的完整 auth 能力。

反证：通用库不必自动扫描所有环境或运行任意 shell；可让调用方注入 resolver/环境/文件/进程/HTTP依赖，以确保可控和离线测试。pi 也将 stored profile 选择、ambient profile 与直接 env keys 区分；不应把“default chain”当作可以随意合并身份的借口。当前 opi partial-source 拒绝、zeroizing secrets、脱敏和 cleanup 仍有价值。

**原 Q8 变更建议**：撤回“发现职责必须移到 Reference Product、删除 SDK discovery exports”的前提。目标先规定：独立 SDK 使用者能装配 Bedrock provider，并取得已声明来源的凭据解析能力；产品负责选择/注入环境与用户交互，SDK层保留可复用 resolver 机制。具体内部目录移动可以讨论，但不能使 SDK 使用者依赖 coding-agent crate 或重写整个 chain。

原有 local resolver 与 AWS 默认链不等价，应单独列清差异：explicit/scoped profile 优先级、bearer、ECS/web identity/role credentials、region/endpoint、token expiry/refresh。可选择原生 Rust resolver 或受控的官方 Rust SDK provider，不能仅凭 TS 使用 SDK 就默认新增重型依赖；也不能因当前 Rust manifest 没有 AWS SDK 就删掉目标能力。

**验证方向**：SDK public entry 直接装配；假 env/profile/process/metadata transport；不同来源优先级/取消/过期/失败均可确定；校验无秘密输出与无 fallback 身份。远端链机制需要进一步准确调查，不在本笔记宣称已掌握全部锁定 SDK 内部顺序。
**分类**：SDK 已有设计能力；Rust 表达/依赖选择可不同，不能削减用户可见能力。**置信度：高**；全 AWS chain 的完整 Rust 实现范围仍待细化。

## RS-02 · 高：多 operation 共用集合是已发布核心设计，原 placement 审核理由应退出

`PI/CHANGELOG.md:36–59` 的 0.99.0（2026-09-29）明确：移除平行 ImagesModels，image/classifier 进入 Provider/Models，operation-specific catalogs，同 upstream ID 可按 type 出现多次，chat unqualified lookup 仍只返回 chat；新增 TypeSafe、OpenRouter、Cloudflare、Vercel/OpenCode classifier，以及 llama.cpp classifier。
`PI/src/models.ts:948–980` 在同一集合中对 image/classifier 做 type check、provider resolution、auth和dispatch；`:1146–1170` 在 provider 内按 api 选择实现。`PI/README.md:792–882` 说明共享 credential 和 typed operation；`:893–977` 给出分类 SDK 使用方式。

真实产品消费者位于 `PC/src/extensions/codemode/execute.ts:612–623`：调用同一个 models 的 classify/generateImages，仅传 signal；`:50` 限制并发；`:123–182` 做脚本形状校验。它不是只有导出类型或孤立样例。此证据不表示 opi 默认终端功能已经实现，也不自动扩大用户本轮明确的 SDK 范围。

**反证**：图片只有一个具体 OpenRouter adapter；TypeSafe/OpenRouter共享System One wire。不能虚报第二种独立 image adapter，但“只有一条当前图片wire”不能据旧两-adapter门槛否决复刻已发布SDK。

**原 Q10 变更建议**：保留 `opi-ai` 共享operation-qualified身份、目录/auth和typed方法；删除“必须证明不违背旧 Agent Core 才可成立”的论证。SDK consumer 本身是真实consumer类别，不要求先为其创造默认CLI工具。Rust闭合enum/typed结果合理，但应支持pi已发布具体operation的演进，不把旧Core定义变成能力上限。

**原 Q13 建议**：OpenRouter Chat-wire image、TypeSafe/OpenRouter System One 可保留为首批明确路径；将 Cloudflare/llama.cpp 标为“已发布但本次分期未纳入”，而非未接受实验或仅未来设想。SDK完整对齐若要求所有已发布adapter，需要单独明确分期目标，不能默称已全覆盖。
**分类**：已发布架构与业务能力；不是 speculative general framework。**置信度：高**。

## RS-03 · 高：Q14 将不同操作的 retry/timeout 合并成一条，会改变 pi SDK 语义

| 位置 | pi 本地实现事实 | 与原候选的差异 |
|---|---|---|
| System One | `system-one-shared.ts:198–224` 调通用 helper，`:220` `maxRetries ?? 2`，即最多3次HTTP attempt；`:200` 每次attempt新建可选 timeout signal。 | 原候选要求一个attempt且不允许自动retry，删除默认恢复与调用方可配置重试能力。 |
| OpenRouter image | `openrouter-images.ts:67–81` 禁用OpenAI SDK内部retry，再用helper；helper默认maxRetries=0，但调用方可明确设置。 | 默认0与候选一致；“永远单attempt”仍删除显式retry选项。 |
| 通用helper | `utils/provider-retry.ts:22–34`：x-should-retry优先；有status/headers形状的异常在status缺失或408/409/429/≥500时可retry。`:49–64` 处理Retry-After/指数退避；`:96–123` 可取消sleep，默认server delay上限60s，0关闭。 | 不是同一 logical call 随机改provider。可在同一request/auth之上实现有界重试，无需破坏已有frozen binding。 |
| Timeout | `types.ts:168–184` 定义HTTP request timeout、retry、retry-delay cap。`models.ts:837–868` 透传options，未注入operation total timeout；auth先于adapter。 | 强制positive total deadline覆盖validation/auth/transport是新增策略；本轮未发现pi该层默认total deadline。 |
| 现有测试 | `test/typesafe-system-one.test.ts:170–208` 区分timeout/caller cancel，验证每attempt新timeout；`test/provider-retry.test.ts` 覆盖默认不retry/helper delay/cancel。 | 测试说明规则刻意存在，但本轮没运行，不能称“重试已测通过”。 |

**重要边界**：raw fetch rejection若没有helper要求的status/headers属性，不一定被retry；“System One所有网络错误自动重试两次”过宽。JSON parsing/answer validation在不同阶段，不能把所有失败都当可安全retry。
System One只有设置timeoutMs才建该signal；未设时没有库在此处施加的总时限。图片OpenAI client未自行覆盖默认timeout，精确第三方默认值本轮未读安装源码；类型注释说OpenAI/Anthropic默认10分钟，不能把注释当锁定 SDK 的运行结果。

**原 Q14 变更建议**：将 retry/timeout 改为逐operation协议：System One默认2 retry；image默认0；保留调用方配置，冻结同route/auth，不隐式provider/model/account fallback。支持原有per-attempt timeout和可取消backoff。总体deadline可作为独立可选上限或产品策略；不要求每个SDK调用强制提供它，除非人类在明确比较此兼容代价后仍选择有意差异。

反证与代价：retry可能再次计费，第一次请求的服务端结果可能未知；这是保留可配置次数和可观察attempt事实的理由，不能单独证明“为了安全必须删retry”。Rust需要可靠drop/cancel资源所有权，但不要求默认retry=0或必须total deadline。保留total deadline作为可选功能不等于复制pi全部缺省等待行为。

**验证方向**：fakeHTTP 500→200与429+Retry-After，默认次数、maxRetries=0/1、delay cap、sleep取消；timeout在每attempt刷新但可选整体deadline不重置；output validation failure不默重发。**分类**：业务可观察恢复策略，非语言必要差异。**置信度：高**。

## RS-04 · 高：Q6 compat 必须逐项证明“替代已覆盖”，不能从没有读点推导删除

`PI/CHANGELOG.md:2172–2186` 将 OpenAI compat overrides 列为0.17.0新增，明确服务LiteLLM/custom proxies；`:644` 记录0.80.2恢复自定义模型detectCompat fallback；`:828` 记录Responses尊重developer-role false的修复。这是长期维护的供应商差异机制。它与临时 `/compat` 全局API入口的迁移地位不是同一概念。

| 原拟退役项 | pi 行为与定位 | opi 实际替代及覆盖判断 | 重新建议 |
|---|---|---|---|
| `supports_store` | `api/openai-completions.ts:833–835` 为true发`store:false`，false省略；`:1637,1693`有默认推断/显式覆盖。 | `OA/src/openai_chat.rs:1027–1088` 无对应store输出；Responses.store是另一wire，不能作Chat替代。是否在特定服务默认结果相同不能从代码推定。 | 视为缺少wire控制，不直接删除能力。可以用更准确的可选store字段或typed compat表达，但要保留“显式false/省略”的区别并先接线。 |
| `supports_developer_role` | `api/openai-completions.ts:1225` 仅reasoning且支持时developer，否则system；`:1632–1639`考虑不同provider/model。 | `OA/src/openai_chat.rs:1317` 的`system_role_override`确实可发developer/system，`:988–1007`有有效覆盖。**表达能力可替代；当前不会自动消费supports字段，也未证明默认映射与pi一致。** | 可合并到唯一instruction-role policy；在有效模型绑定时决定role，测试reasoning/nonreasoning、supports true/false、显式role优先级。语义覆盖后旧布尔键可退，不必保留同名。 |
| `supports_reasoning_effort` | `api/openai-completions.ts:875–970` 控制是否发送reasoning_effort；若不支持此字段仍可用zai/qwen/string-thinking等其他格式；off映射可显式发送关闭值。 | `OA/src/model_info.rs:203–270` 的ThinkingLevelMap只表示Identity/Mapped/Unsupported；`openai_chat.rs:1076–1080` thinking enabled后总发reasoning_effort。Map不能单独区分“可以思考但不接受该字段”，disabled路径也不发off映射。 | 保留/补齐“请求level、模型映射、wire字段/格式能力”的分离，可合并冗余flag但不能把level unsupported当成字段unsupported。用不同格式/关闭值fixtures证明后再退旧键。 |
| `require_assistant_after_tool_result` | `api/openai-completions.ts:1231–1237` toolResult→user时插assistant bridge；`:1287`改变tool-call assistant空content表达；`:1443–1447`处理tool-image转user时的bridge。 | `OA/src/openai_chat.rs:768–773`明确只记metadata，并主张需dedicated adapter；serializer没有bridge。现有fixture `tests/openai_chat_fixtures.rs:1049–1052`只断言配置值存在。**没有替代。** | 不是已证明冗余。为需要该quirk的兼容endpoint提供同wire私有投影规则，或先明确哪些endpoint不支持。无需为一个已有wire差异强造独立adapter。 |
| Chat/Responses静态`compat.reasoning_effort`字符串 | pi当前有per-request`reasoningEffort`（Chat`:151`、Responses`:364–368`），并经model mapping；不能仅同名认为opi静态配置字段就是pi能力。 | opi实际wire由Request.thinking+selected map负责；`openai_chat.rs:762–763`明确该静态字段legacy；Responses构造虽复制它，未发现生成wire读值。 | **仍可合并/删除重复值来源**，但保留SDK每call选择effort与模型默认/映射能力。旧配置迁移指向有作用入口，不删thinking控制。 |

`require_assistant_after_tool_result` 的反证：pi默认detect值为false（`:1645`），本轮缺失生成catalog，未找到当前内建模型设true的证据；不能声称某个具体默认provider现在必需它。但custom model支持此显式quirk本身有代码和长期compat定位依据，且opi声明了同一能力。未找到preset不是“机制无意义”的证明。

`supports_developer_role` 的反证同样重要：pi有字段不意味着Rust必须照搬一个布尔字段。`system_role_override` 可成为更直接、单一的目标表达；正确目标是effect/model-default/explicit override一致，而不是字段数相等。

**原 Q6 变更建议**：旧 Config/Model/Error/ProviderKind 壳是否冗余仍按真实职责独立判断，不因这次纠偏自动恢复。compat部分从“删无行为字段”改成“补齐或合并相应wire语义，完成验证与配置迁移后淘汰重复键”。当前AIR-022的一次性删除清单不能原样作为新目标。
**分类**：前三/四项主要是漏实现或可合并表达；静态effort字符串才有较充分重复来源证据。**置信度：高**；具体内建requires-bridge消费者为证据不足。

## RS-05 · 中高：Q21 混合了资源保护与未经协议证明的业务上限

### 有一手支持、可继续保留的部分

- 输入/输出typed、known usage在答案失败时仍保留：`system-one-shared.ts:225–230` 先保存usage再parse答案；`typesafe-system-one.test.ts:210–220`对应fixture。
- 不把pi缺陷作为兼容义务：`openrouter-images.ts:91–111`只读first choice，跳过坏/非data URL并可能成功但零图片；`:101–106`只正则拆base64，无字节有效性证明。opi可修正坏媒体/空成功等问题，但要明确是正确性选择而非Rust自动保证。
- `system-one-shared.ts:123–143` 将缺失/无效token bucket变0；opi的Unknown与零分开更诚实，不需要为了Rust实现pi而复制这项信息丢失。
- 有界body/媒体解码、取消cleanup、typed错误是合理Rust工程目标；拥有bytes而非base64 string可改变表达而保留媒体内容。

### 本轮没有证据支持为统一业务限制的部分

1. **非空prompt + 最多8张reference**：pi `ImagesContext.input` 是有序text/image blocks；`openrouter-images.ts:138–163`按输入顺序映射整个数组。`PC/.../execute.ts:165–180`只要求非空blocks和类型，允许image-only，也不要求唯一文本或文本非空。远端是否接受image-only本轮未验证，但候选若只接受prompt+references，会主动缩窄SDK可表达输入和顺序。
2. **System One score最多10档**：`system-one-shared.ts:147–166`传入criteria；无10档校验。10档来自`llama-cpp-classify.ts:113–116`的单token labels机制及`PI/README.md:943`，该adapter恰被原Q13延后。将其上限套到首批System One缺少依据。
3. **64题、255labels、16/32/48MiB、1MiB/depth32**：本轮未在所选pi SDK路径找到这些精确值；codemode只做形状验证。它们是研究者选定的防护初值，不是语言或供应商强制边界。
4. **恰好一个response choice/拒绝多choice**：pi读取choice0，是已存在但可能偶然的实现策略；不证明远端保证只有一个或必须拒绝额外choice。若目标只暴露first choice，需要明确它与“所有images原序返回”的关系，不能将候选新拒绝视为对齐。
5. **严格概率和/argmax/额外ID拒绝**：pi parser只检查字段类型与finite（`:63–120`）；原候选增加的数学检查可以有契约价值，但会拒绝此前接受的响应。阈值、误差容差、分布是否存在需要协议fixture/可靠语义依据，不能从typed Rust直接推出。pi score公开结果只有score/confidence（`types.ts:665–668`），缺distribution时不能要求补一个来验证均值。

**原 Q21 变更建议**：保留“读取/解码前有界”的原则，将byte/count/structure limits明确视为SDK可说明的资源policy；重新决定是否采用默认可覆盖上限，或保留已选固定上限并将与pi不同的输入拒绝逐项列出。不要让旧“不加配置”偏好迫使所有embedder接受无法调整的任意阈值。尤其不要把llama的10档误用于System One；既有有序input blocks应优先保留。

反证：允许SDK override不意味着无限制、自动提高限制或新增一套产品配置。可以是有限的typed options。具体数值需要调用方需求、可控allocation/fixture依据；没有测量不应写性能优劣。**分类**：资源策略与业务输入限制，非Rust必然；解析防护为合理有意改进。**置信度：高（不等价事实），中（最合适的新默认值）**。

## RS-06 · 中高：Q13 的协议选择基本有证据，但“失败/选项”不能一并收窄而不说明

`PI/src/api/openrouter-images.ts:138–163` 确实用nonstream chat/completions，image/text modalities按model.output决定；`typesafe-system-one.ts`与shared实现、`typesafe-system-one.test.ts:83–108` 说明OpenRouter System One endpoint复用。原候选首批wire选择不必因为旧规范纠偏而改变。后续官方专用Image API只是不同协议线索，不是本次local基线已经迁移的证据。

但是SDK options在pi里有明确扩展点：`types.ts:152–184` 的onPayload/onResponse/headers/timeout/retry，`types.ts:324–338` 的classifier temperature与image metadata。System One忽略temperature是README`:939`及test`:60–66`明说的行为；typed Rust可选择不暴露无效选项，或显式拒绝，而不是吞选项，但这属于设计差异。原候选“无passthrough”“不支持设置均失败”不应使未来实现Agent错误声称已完整复刻所有SDK option能力。

**建议**：每个公共option只做明确处置：保留有实际wire行为的选项；无效的temperature在首批System One可不暴露/显式unsupported，但记录与pi差异；hooks/headers需要单独说明SDK使用场景与安全边界，而不是默认套用产品策略。当前Q13首批adapters可维持；Q14/Q21的收紧需独立重新确认。不要为支持一个回调自动引入无类型通用operation框架。

**分类**：选定wire为已实现能力；option/retry/限制是独立可观察SDK契约。**置信度：中高**。

## 建议主 Agent 重新记录的决策边界

| 原问题 | 新证据后的处置建议 | 不需要重新争论的部分 |
|---|---|---|
| Q8 AWS | 重开“仅产品owner”选择；保证SDK具备可复用credentials机制，再定Rust实现依赖与来源覆盖 | secrets脱敏、bounded process、取消、typed credentials、来源可追溯继续保留 |
| Q10 operations归属 | 保留opi-ai共享目录/auth；去除旧core例外裁决 | 不建平行ImagesModels；typed operation身份和chat-only旧lookup保留 |
| Q13首批wire | 保留明确首批路径；其他已发布adapter标真实分期缺口 | 无live调用、不默认CLI工具仍是本次授权范围，而非架构否决 |
| Q14 retry/deadline | classifier/image分开defaults；保留可配置retry；总deadline可选/产品policy而非唯一mandatory SDK入口 | 同route/auth冻结、cancel可见、无隐式跨provider fallback、原子answers与failure usage保留 |
| Q21数字/形状 | 移除跨adapter误套上限；区分资源policy与协议限制，保留有序input表达 | 有界读取/strict base64/实际MIME/已知性/错误脱敏可继续作为明确改进 |
| Q6退役 | 旧公开壳独立审；compat能力先补齐/合并并证明，再退重复字段/键 | 不要求TS同名字段或保留永远无效的配置 |

本轮没有查询新的上游planning，也没有把未知推断成未来承诺。AWS锁定SDK内部链、System One真实最大criteria/题目数、服务端重试幂等性/计费、远端image-only接受度及所有自定义compat真实部署，仍是明确证据缺口；可以用离线fixtures验证实现机制，但不能将其描述为live服务已验证。

本分项实际检查：本地 `rg` 与逐行读取源码/文档/测试；没有运行pi或opi测试。测试影响：**none**。旧审计/评审保留历史记录；新定位下它们的authority与Q6/Q8/Q14前提必须由主 Agent显式重新整合，本文不改写共享文档。

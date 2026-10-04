# SDK 目标契约细化提案

> **非规范性中文提案；2026-10-03；未授权实施。** 以用户更正后的“opi 是 pi 的 Rust 实现”为判断前提；Q24/Q27–30已确认具体方向，当时44 AIR已重写（历史阶段）；旧HOLD仅是复审历史。本文细化 [RA-01/02/07/10](decision-reassessment.md) 与 [RS-01–06](reassessment-sdk-notes.md)，不把建议写成人类已确认的要求，也不因旧 core/product 规则删减已发布的 SDK 能力。本文只有研究用途，无既有中英 counterpart，因而不改其他双语文件。

阅读顺序：先看目标职责及状态，再看 compat、AWS、operations 的细节，最后看三个已收敛的取舍及补证附录。SP ID 是本提案的追踪编号，不是新权威或实施账本。`保留`表示与现有回答相容的子契约；`建议`保留提案形成时的推荐措辞，当前确认映射见下方状态；`证据待补`表示不可让实施 Agent自行猜的外部事实。所有“验收”均为未来离线验证设计，本轮未运行测试。

## 目标职责与实现基线

- registry 选择唯一有效、operation-qualified 模型记录；collection 准备 owned binding、请求及鉴权；adapter 消费该值并生成 wire，不再次从另一份目录选择 compat。新的 image/classifier 方法与 chat 共用这一机制，不建立平行 Models、auth store 或 provider 集合。
- compat 是协议差异的模型数据和私有投影规则；Agent 负责 thinking 请求意图，adapter 负责 wire 表达；产品负责配置读取、诊断和人类选择。配置解析器不能成为第二个协议解释器。
- Bedrock resolver 是 SDK 可独立使用的机制；产品负责交互、存储选择、注入来源及运行权限。来源探测、凭据选择、签名、刷新和显示状态要区分，不能用一个“configured”布尔替代。
- image/classifier 返回各自的 typed 非流结果；结果不能授权工具、设置业务阈值或自动写盘。请求生命周期和恢复预算由共享 SDK 机制执行；operation adapter 解释协议差异。

本地 Rust 基线是现有代码与 `Cargo.lock`，不是假定存在的新 SDK：edition 2024、MSRV 1.97；锁定 reqwest 0.12.28（rustls/json/stream）、tokio 1.52.3、tokio-util 0.7.18、serde_json 1.0.149、secrecy 0.10.3、base64 0.22.1。当前没有 AWS 官方 Rust SDK 依赖；不以“pi 用了 Node SDK”为理由自动引入依赖，也不把当前依赖集合当作永远不能增加能力的理由。本文没有验证任何新依赖的 API、默认链、取消或平台行为。

## 证据导航

路径缩写：PI=`.repo/pi-1.0.0/packages/ai/`；PC=`.repo/pi-1.0.0/packages/coding-agent/`；OA=`crates/opi-ai/`；OC=`crates/opi-coding-agent/`。精确 revision/快照身份沿用本目录基线。

| 证据 | 一手定位及本轮证明范围 |
|---|---|
| SE-01 | PI `src/api/openai-completions.ts:829–970,1220–1287,1443–1447,1580–1720`：wire 差异、自动推断和显式覆盖；PI `CHANGELOG.md:644,2172–2186`：自定义 compat 的维护记录。 |
| SE-02 | OA `src/model_info.rs:203–270,289–343`；`openai_chat.rs:757–773,986–1007,1076–1080,1317`；OC `provider_factory.rs:1177–1200`、`config.rs:1600–1728`：当前 map、role、旧字段及配置接线。 |
| SE-03 | OA `src/lib.rs:7–59`、`config.rs:1–30`、`model.rs:1–10`；[O-003/004](opi-audit-notes.md)：旧壳与真实运行时契约分离；[route probe](route-probe.md)：第二目录选择的实际反例。 |
| SE-04 | PI `src/providers/amazon-bedrock.ts:7–78`；`src/models.ts:837–868`；`src/utils/provider-env.ts:41–51`：配置探测、鉴权入口和 scoped/ambient env 读取。 |
| SE-05 | PI `src/api/bedrock-converse-stream.ts:155–249,1176–1224`；`test/bedrock-credentials.test.ts:71–113`、`bedrock-endpoint-resolution.test.ts:122–200`：client 配置、profile/bearer/region/endpoint。测试仅阅读。 |
| SE-06 | OA `src/auth.rs:29–72,125–178`；`bedrock/credentials.rs:143–307,536–660`；`bedrock/mod.rs:222–232`；OC `provider_factory.rs:2037–2120`：SigV4、当前 local chain、取消/进程保护与产品消费者。 |
| SE-07 | PI `CHANGELOG.md:36–59`、`src/models.ts:948–980,1146–1170`；PC `src/extensions/codemode/execute.ts:123–182,612–623`：已发布多 operation、共用集合及真实调用方。 |
| SE-08 | PI `src/types.ts:145–184,324–341,612–689`；`api/openrouter-images.ts:42–164`；`test/openrouter-images.test.ts`：有序输入、first-choice 结果、请求选项及媒体行为。 |
| SE-09 | PI `api/system-one-shared.ts:63–170,175–235`、`typesafe-system-one.ts:8–21`；`test/typesafe-system-one.test.ts:170–236`；`api/llama-cpp-classify.ts:113–116`：System One、failure usage、timeout 与 llama 独有的 10 档限制。以上 API 路径均相对 PI `src/`。 |
| SE-10 | PI `src/utils/provider-retry.ts:14–66,105–123`：retry predicate、backoff、delay cap 和取消；OA `src/http.rs:17–79`、`provider.rs:210–250`：现有有界错误读取及 reserved-header 检查。 |
| SE-11 | [WEB-01/02](provider-contract-addendum.md)：主 Agent 于 2026-10-03 查询官方协议的记录；本分项未重新查询网页。它支持供应商语义主张，不是本次 live 测试或锁定版本的 SDK 源码。 |

## Compat：收敛表达，而不是删除尚未接线的能力

### 单一归一化规则

先按 registry 的现有选择规则取到最终完整模型记录（override 胜过动态/内建），再对该记录做一次 compat resolution。**不能把 shadowed 模型的字段偷偷补回最终记录。** 配置装配阶段可把 provider profile 默认值与 model 显式 patch 合成该记录；该阶段保留“未指定”与“明确 false/关闭”的区别。建议的字段级顺序为：adapter 的 pi 基线推断 → provider profile 显式默认 → 最终模型显式覆盖。现有 model-level `system_role_override` 高于 profile-level override；同一层显式 role 高于 developer-support 自动规则，新表示只允许一个 role policy。请求 thinking level 是每 call 的意图，不作为第四份 compat 值来源。

最终模型的 provider、endpoint、reasoning 能力、thinking map、已声明 compat 一起产生可检查的有效值。若 auth 合法提供 base URL，pi 是先应用 auth.baseUrl 再运行 adapter；因此 endpoint-dependent 推断必须使用最终 endpoint，不能在 pre-auth 固定一个错误默认。pre-auth 可验证已知的结构/显式冲突，auth 后只完成依赖 endpoint 的步骤，不重新选择模型；HTTP前完成最终验证。推断对已注册 provider/endpoint 的已知规则生效，不把普通 hostname substring 当作可信身份或鉴权依据。对自定义 endpoint，兼容推断只是可覆盖的协议默认值；它不能替换用户显式参数。若选择将 pi 的 substring 检测改为解析 URL 后匹配已知 host，应将误匹配反例记录为明确修正，不能声称逐字复制了 pi 检测器。

| 当前表面 | 建议唯一目标表达及默认 | 迁移的精确条件 |
|---|---|---|
| `supports_store` | Chat `store_field = Omit / SendFalse`；pi 检测到支持时 `SendFalse`，否则省略。此能力不开放 `store:true`，Responses 的 `store` 独立保留。 | 旧 true→SendFalse、false→Omit；不要把“未配置”经 serde 默认 false 变成显式覆盖。先证明 wire 差异，再退旧键。 |
| `supports_developer_role` 与 `system_role_override` | `instruction_role = Auto / System / DeveloperForReasoning / Developer`。Auto 使用 pi 支持推断；DeveloperForReasoning 仅在 reasoning 模型用developer，否则system；System/Developer为显式固定role。 | 旧 supports false→System、true→DeveloperForReasoning；未指定→Auto。同层既有显式role优先。不能简单将旧true翻译成无条件Developer；新表示只有一个policy。 |
| `supports_reasoning_effort` | 将 thinking **意图**、level→wire 值映射、**字段支持**、**格式**分开。支持 `reasoning_effort` 不等于可以思考；其 false 不禁止 zai/qwen 等原生开关。格式为 adapter 私有闭合枚举及已核实参数。 | 从 map 删除某 level 不等于禁用某 wire 字段。`off` 映射参与序列化；不能沿用 `enabled=false` 提前跳过所有 thinking 字段的旧路径。 |
| `require_assistant_after_tool_result` | 一个明确的 Chat wire 投影规则：需要时在 toolResult→user 之间插入固定 assistant bridge；tool image 投影产生 user 时同样处理；相关 assistant 空 content 用空字符串。默认 false。 | 当前没有替代实现；必须先有覆盖上述三条的 payload corpus。记录为 request-local 投影，不写回 canonical transcript，不制造真实助手推理或工具执行事实。 |
| Chat/Responses `compat.reasoning_effort` 静态字符串 | 每 call `ThinkingConfig` / model thinking map 为唯一有作用入口；不再维护静态、无 wire 读点的第二份 effort。 | 旧键报迁移错误，指出每 call/模型映射入口；不能自动把任意字符串改成用户未请求的全局默认。已有请求 level 与模型映射继续生效。 |

Auto 默认不得沿用当前 opi “所有 supports=false”的事实假装 pi 等价。SE-01 的已核实推断包括：标准 OpenAI Chat 使用 store:false；开发者角色只在 reasoning+支持条件生效；OpenRouter 的 anthropic/openai 前缀有特殊支持规则；Grok/Zai/Moonshot/Together/Cloudflare Gateway/Nvidia/AntLing 默认不发送标准 effort 字段。完整 provider 判别集集中于一个私有表并用 fixture 冻结，避免配置解析器、目录和 adapter 各维护一份。未读到的 provider 默认不能推测。

thinking 格式必须覆盖已发布的实际差异，而非只加一个 `supports_reasoning_effort` 布尔：OpenAI、Zai、Qwen、Qwen chat-template、generic chat-template、Baseten、DeepSeek、OpenRouter、AntLing、Together、string-thinking 的开关/字段规则都在 SE-01。例如 OpenRouter 使用 `reasoning.effort`，Together 使用 `reasoning.enabled`，DeepSeek 关闭时 `off:null` 表示不发送关闭指令。对 `null`、省略、映射字符串三种意义，应建立 corpus 再设计 Rust enum；不能让“Unsupported level”冒充“wire 中应省略关闭字段”。不要求复制多处实现对 null 使用 `??` 的偶然差异；这些分支的差异应在 fixture 中暴露，由模型能力归一化给出一致且有依据的结果。

| ID / 状态 | Owner / trigger | 目标行为与失败 | 机械验收 / 证据 |
|---|---|---|---|
| SP-01 / 建议；binding 方向保留 | registry + collection；准备 Chat call | 按上述顺序一次生成 owned effective compat；adapter 不重读 catalog。显式未知格式/冲突在 auth/HTTP 前失败。 | 用同 ID overlay、动态刷新和旧 prepared call 三组 fixture 捕获最终 wire、role、effort、store；旧 call 保持旧值。SE-01/02/03。 |
| SP-02 / 建议 | Chat serializer；生成 request view | 执行角色/store/bridge 表；保留消息顺序和 canonical；不以删除字段代替实现差异。 | reasoning/nonreasoning × Auto/System/DeveloperForReasoning/Developer；支持与省略；tool→user、tool-image→user、无文本 assistant；false/default/显式覆盖成对 fixture。SE-01/02。 |
| SP-03 / 建议 | thinking normalizer + 各 wire adapter；调用者指定 level/off | 将请求意图解析一次，model map 只处理值/能力，format 决定 wire。关闭值、unsupported 和省略可区分；高层级别按D-26选择可用等级；native/exact不支持在auth前失败，依最终endpoint才可判定的失败在HTTP前给出。 | 覆盖全部已声明格式的 enabled/off、映射/未映射/null 和覆盖；逐字段比较，不归一化掉字段是否存在。SE-01/02/04。 |
| SP-04 / 保留壳淘汰原则；compat 部分建议 | SDK public exports + 产品 config parser；发布有界 0.x 迁移 | 按文末退役清单同步迁移；旧键显式错误，不静默忽略或永久 alias；已有真实能力必须先有替代 proof。 | 编译当前生产消费者；每个旧键在 profile/model/合并配置位置失败；扫描旧符号排除历史研究；保留有效请求 payload tests。SE-02/03。 |

## AWS：pi前缀与精确依赖链分开

### pi 源码事实，不能混为一个优先级

| 层次 | 已核实顺序或行为 | 不可据此声称的内容 |
|---|---|---|
| provider `auth.resolve` 配置探测 | stored credential.key → ambient/scoped ctx 的 AWS_BEARER_TOKEN_BEDROCK → stored scoped AWS_PROFILE 或 ambient AWS_PROFILE → 完整环境 key pair → ECS relative URI → ECS full URI → web identity 文件；都未命中则 undefined。 | 返回 `{auth:{}}` 只表示可继续解析，不是已获得签名材料；此顺序不是 AWS SDK 最终凭据链。未探测默认 profile/IMDS 不证明 direct adapter 不支持它们。 |
| Models auth 应用 | 未配置 resolution 则报 auth；显式 options.apiKey 高于 resolved key；request env 字段高于 resolution env；adapter 随后再读取 scoped/ambient。 | provider 配置检测与 direct adapter 默认链可达性存在差别；不能一概说“Models 总能无配置探测所有 AWS 来源”。 |
| adapter Bearer | `bearerToken` → `apiKey` → scoped/ambient AWS_BEARER_TOKEN_BEDROCK；有 token 且非 SKIP_AUTH 时选择 httpBearerAuth。 | 即使同时收集到 SigV4 材料，实际 auth preference 为 Bearer；不能日志只标 profile 而称准确身份来源。 |
| adapter profile | 显式 options.profile → scoped AWS_PROFILE → ambient AWS_PROFILE。前两者构成 optionsProfile，抑制 env keys 的显式注入；只有 ambient profile 不抑制完整 env keys。 | 不能用统一“profile 最高”或现有 opi“env 永远最高”替代。 |
| adapter static pair | 未 skip-auth、没有 optionsProfile，且 getConfiguredBedrockCredentials 得到完整 pair，才将 pair 直接传 SDK；session token 可选。每个 env key 的读取先 scoped 再 ambient。 | pi 会逐字段取值，可能将 scoped access key 与 ambient secret 拼在一起；当前 opi 的完整来源拒绝策略不是同一行为。见 U-1。 |
| adapter default chain | 没有注入静态 credentials 则由锁定 Node AWS SDK解析 profile/default chain；PI manifest 的 runtime client 为 3.1127.0。 | 完整链已由附录A/B核验；本表只描述pi adapter前缀，不以它代替下游依赖行为。 |
| skip auth | `AWS_BEDROCK_SKIP_AUTH=1` 在pi层跳过显式bearer并提供dummy credentials；下游ambient bearer可能再次生效，见附录D。 | 不证明请求绝不出现签名 header；是否签名由实际 SDK 消费决定。本提案不默默新增通用 NoAuth 或照搬 dummy secrets。 |

SDK 目标推荐：保留可复用 Bedrock resolver；提供显式 auth 选择（预解析 SigV4 / Bearer / profile / default chain）以及需要兼容的 Auto 入口。**显式 typed choice 优先于 Auto 探测，多个互斥显式选择报配置错误**，避免要求调用者靠环境变量清除其他来源。Auto 复现上表已核实语义，并对来源做非秘密 provenance；不用 provider 探测时的说明文字代替实际 selected source。新显式选择是 Rust 表达，不改变同一选择之后的模型操作语义。

profile/default-chain 的 SDK 接口必须可注入 env、profile 文件访问、process runner、metadata HTTP、clock，支持独立 embedder；不必将这些实现全部变成公开 trait。既有两个真实消费者（产品和独立 SDK）共用同一机制；测试用私有 fake，不能让运行时为了测试要求全局 setenv。若暂时仅完成 local chain，明确报告 SupportedSources；不能称完整默认链已交付。未支持但已明确选中的来源报 UnsupportedAuthSource，而非悄悄换账户。

**外部链证据已补齐**：附录A/B固定pi锁定Node依赖的来源、tryNextLink、缓存及expiry事实。未来Rust实现须核验其精确锁定依赖/平台并证明同一目标corpus；依赖选择不能改变已选来源政策。不能仅凭“依赖提供default chain”验收，亦不要求Agent发明未知顺序。

当时调查导航（现已完成，结果见附录A–D）：pi 根 `package-lock.json` 锁定 `@aws-sdk/credential-provider-node` 3.972.82、env/process 3.972.70、ini 3.973.15、sso 3.973.14、web-identity 3.972.76、http 3.972.72、`@smithy/credential-provider-imds` 4.5.2。下一步核对官方 `aws/aws-sdk-js-v3` 中上述发布物对应的 `packages/credential-provider-node/src/defaultProvider.ts` 及每个被调用provider、缓存/expiry包装和错误的tryNextLink语义；不能读取main后冒充锁定实现。Bedrock client 3.1127.0并不意味着每个子provider也同版本。尚无Rust AWS锁定依赖，现阶段不需为了调查先选crate或安装。skip-auth实际签名/ambient override已核验，首批typed来源不仿造该环境开关；它不是尚未解决的首批目标问题。

### Region、endpoint 与本地生命周期

pi 已核实顺序为：model ARN region → 显式 region → scoped/ambient AWS_REGION → scoped/ambient AWS_DEFAULT_REGION → 允许使用的标准 endpoint region → 无 ambient profile 时 us-east-1，否则委托 SDK。自定义 endpoint 保留；标准 AWS catalog endpoint 仅在没有 configured region、没有 ambient profile 时固定。注意这里代码以 **ambient** profile 判断 endpoint/region fallback，和 optionsProfile 不完全同义，不能把注释“SDK default chain”过度概括。建议首先以这些既有分支的 fixture 对齐；若 profile-only 默认 region 看起来可疑，单独证明缺陷后修正，不擅改。

同一 prepared call 固定所选 auth identity/region/endpoint；支持短期凭据来源不等于可以在重试时跳到另一个 source。是否允许同一 identity 的到期刷新属于长期 credential 生命周期，须在上述外部链核验中明确，不能把 refresh 当作隐式账户 fallback。当前 opi 的 AwsSigV4Credentials 无 expiry 字段，是具体差距；不得声称已有 chain-refresh 保证。

| ID / 状态 | Owner / trigger | 目标行为与失败 | 机械验收 / 证据 |
|---|---|---|---|
| SP-05 / 建议；撤回产品独占 | SDK Bedrock resolver；SDK 或产品装配 | 独立 SDK 可装配已声明 auth sources；产品仅提供选择和依赖。保留 typed secret/provenance；缺配置、来源不支持、来源拒绝分开。 | SDK-only consumer 不依赖 coding-agent；fake sources 检查选择及非秘密 provenance；产品使用同一 resolver。SE-04/05/06。 |
| SP-06 / 建议；U-1 待定 | Bedrock Auto resolver；多来源共存 | 依据上表分别处理 bearer、explicit/scoped/ambient profile 与 pair；typed explicit selection唯一。partial/mixed source 的目标遵 U-1，未确认前不能写死。 | fixture 交叉覆盖 bearer+keys、explicit/scoped profile+keys、ambient profile+keys、缺一半pair、多个显式选项；未选中的 fake来源调用次数为0。SE-04/05/06。 |
| SP-07 / 建议 | Bedrock binding + adapter；准备 endpoint/auth | 解析并冻结 region/endpoint；SigV4 与 Bearer 各自发送真实 auth，无两种 header 叠加；401/403 不更换来源。 | 捕获 region/scope/host 与 typed bearer attach；ARN、custom URL、标准 endpoint、各 region来源矩阵；secret canary不得进入错误/Debug。SE-05/06。 |
| SP-08 / 保留保护原则；完整链证据待补 | resolver 生命周期；文件/process/metadata等待、到期 | 等待响应取消；进程超时/输出超限清理；错误不泄漏文件/进程秘密。不把来源探测当成功，不把 local-only 当全链。 | 虚拟 clock、假进程/metadata；drop/cancel 无后续发布；limit±1；过期/刷新/同源并发按后补锁定 chain 契约验证。现有10s/64KiB可作默认回归，但属opi策略。SE-06。 |

## Typed images 与 classification

### 请求与结果形状

image 输入是非空有序 `Text | InlineImage` blocks；InlineImage 为 owned bytes+MIME，base64 输入若暴露则在入口解码到同一表示。允许多个 text 及 image-only，不把内容重排成 prompt+references；空 text block 不代表必须添加占位 prompt。首批 wire 仍是 OpenRouter nonstream chat/completions，一条 user message 保留 block 顺序，model.output 决定 image 或 image+text modalities。URL 输入/输出不自动下载；既有人类选择的“不支持 URL-only”继续保留。

结果建议用有序 typed blocks 表达，而不是单独 text 字符串加 image数组；首批 adapter 的顺序为 first choice 的可用 text，随后该 choice images 的原序。pi 只读取 choices[0]，建议保留明确的 first-choice语义，不因出现其他 choice 拒绝整个结果；这是对旧“exactly one”建议的修订，待综合确认。成功仍至少需要一张通过已选校验的 owned image；坏条目、URL-only、空结果/text-only 不成功，不悄悄丢掉坏图片后返回成功。不是对远端内容正确性或完整像素解码的承诺。

classification 请求保留 object state、question ID、string instructions 和三个闭合类型：choice 的有序唯一 label→description；score 的有序 description 列表；bool 的 true/false descriptions。JSON object 的 wire 键顺序不是语义要求，但 SDK 不能用无序 map 丢掉 criteria 的声明顺序；结果按 question ID 关联。System One 把 bool 转为 `noul`；两个首批 provider 共享 `/systemone` body `{model,state,questions}` 与 parser，不冒充两种分类算法。

结果分别是 `Choice {choice, probabilities, confidence}`、`Score {score, confidence}`、`Bool {probability}`。**System One score 无必需 distribution**；不合成它，不重新计算/替代 provider confidence，不声称各模型 confidence 已校准。若特定 adapter 额外提供完整 distribution，它属于该 adapter 的有依据扩展，不能让首批所有 adapter为验证均值而伪造。

### 数值与失败验证

现有官方补核支持 choice 为最高概率选项、score 为 ordered criteria 加权值、概率/confidence 有定义域；pi parser实际只检查 finite和若干字段类型。因此“忠实表达领域”和“与当前宽松 parser 接受集完全相同”并不等价，具体强度留给 U-2。推荐保留 finite、已请求 question/type、choice属于labels、所有必需概率存在且[0,1]、confidence[0,1]、score[0,n-1]；不把缺概率补0、不归一化坏值。sum/argmax与容差仍是额外验证政策，不能从 Rust 类型直接推出。

未知额外 question ID 建议忽略以对齐 pi，但可以记录结构诊断；请求集合有任一 missing/malformed answer 时整次 typed failure、不得公开可当成功使用的部分 answer map。所有失败保留已知 usage与非秘密 response metadata。类型建议为成功值或 `OperationFailure {kind, usage, response_metadata}`；error kind 至少可区分 input/unsupported/auth/transport/http/timeout/cancel/invalid-response/resource-limit；无需为此另造一套通用 ProviderError，只在现有 typed 错误旁附 operation 结果元数据。

### 可配置参数与恢复

保留有实际 wire 作用的 SDK 能力：per-attempt timeout、retry count、retry delay cap、取消、headers、选定 endpoint/model、response metadata。image 的 metadata在 pi 公共类型存在但该 adapter不读取，System One temperature被明确忽略；不能为“参数齐全”将无效字段包装成有效功能。推荐对首批无作用的 typed option不暴露；若共享 options 已包含它，则返回 UnsupportedOption 并记录与 pi 忽略行为的差异。provider-specific image quality/size/image_config需有该 wire的实际字段证据，不从专用 Image API推导到 Chat-wire。

重试建议按已核实 operation defaults：System One maxRetries=2（最多3 attempts），image=0；调用方可以覆盖。重试相同已准备 payload、model与auth；不重复执行 payload transform，不切provider/account，不在解析到语义无效成功 body 后重发。HTTP 408/409/429/≥500及 x-should-retry 规则、可取消 backoff、默认60s服务器延迟上限由 SE-10确定；Rust enum识别错误，不能照搬 JS status/headers属性形状作为契约。

每 attempt 独立 timeout；整体 deadline 可选且不随 retry重置。System One 未设 timeout 时不强加新的总deadline。image的实际OpenAI Node 7.19.0默认600000ms已在附录C核验，SDK默认retry2但pi覆盖为0；Node计时止于headers，Rust目标覆盖body是明确正确性修正。不能误用root OpenAI 6.40.0。caller cancel、总deadline到期、attempt timeout和HTTP失败要可区分；所有wait/drop仍需按流/operation生命周期契约释放本地资源。

| ID / 状态 | Owner / trigger | 目标行为与失败 | 机械验收 / 证据 |
|---|---|---|---|
| SP-09 / operation归属和首批路径保留 | collection + model catalog；generate/classify入口 | operation-qualified model选择与auth共用；类型不匹配在auth前失败；显式注册是有效入口但不是唯一长期目录来源。 | 同provider同ID不同operation、chat默认lookup、错误类型、catalog刷新后old binding稳定；真实SDK public入口到fake transport。SE-07。 |
| SP-10 / 建议有序输入；无下载/有效媒体保留 | image typed入口 + OpenRouter adapter；generate | 执行有序blocks/first-choice语义；返回owned bytes及MIME。坏媒体/URL-only/空媒体失败；不下载/写盘。 | text→image→text、image-only、多image、多choice、URL-only、bad base64/MIME/signature、text-only与0choice；side-effect fake计数为0。SE-08；旧Q13/URL回答。 |
| SP-11 / Q28已确认 | System One parser；分类响应 | 保持三个结果类型，score不要求distribution；按请求ID原子返回，failure保留usage。数值接受规则以U-2为准，不能留给Agent随意强化。 | 两binding同corpus：score仅score/confidence成功；缺题/type错/缺label/nonfinite失败；额外ID、tie、舍入案例按选定政策。SE-09/11。 |
| SP-12 / 保留已知性及failure usage | operation result owner；解析usage或结束失败 | 先保存有效已报usage再验证业务答案；缺失/坏bucket为Unknown，零与缺失不同；费用只基于已知数据估算。 | usage缺失/0/负数/字符串/部分bucket、答案失败但usage存在；不把未见的先前retry计费当0或当总账。SE-08/09；D-10。 |
| SP-13 / Q27已确认；附录C补证 | SDK retry owner + adapter；可重试失败 | 采用逐operation默认和显式override；retry同binding/auth且可取消；attempt timeout刷新，整体deadline可选。invalid-response和bad media不retry。 | 500→200、429延迟、x-should-retry false、maxRetries0/1/2、sleep取消、timeout每attempt重置、total不重置；HTTP/auth/response-parse计数清楚。SE-08/09/10。 |
| SP-14 / Q30已确认 | SDK options + adapter；调用方提供扩展 | 支持有效typed options，明确忽略/unsupported差异。header和payload扩展的权限由U-3决定；不能暗中改binding身份后仍记录旧plan。 | headers优先级/删除、reserved header、callback顺序/调用次数/失败/取消、改model或无限body的反例；不启用真实凭据。SE-08/10。 |
| SP-15 / 建议 | SDK resource budget + reader；编码/解码/解析前 | 按下表采用有限且可覆盖预算；协议硬限制单独校验。超限返回typed ResourceLimit，终止读取且不保留部分成功。 | 每项limit−1/limit/limit+1、UTF-8 bytes非字符计数、base64膨胀、压缩后HTTP body、深度、整数溢出、取消；override后只改预算不改协议。SE-08/09；原Q21及RA-10。 |

### 预算：建议保留的初值与必须移除的错误推论

SDK预算是请求或已装配SDK实例的有限正值；调用级显式值覆盖实例默认，实例默认使用以下初值。**建议不提供 unlimited 哨兵**，高上限仍需检查整数溢出、可表示的allocation及已确认协议硬限制。可降低或升高预算，无需新增产品配置键；每call冻结有效预算。同一进程允许可信embedder设置更高预算，不意味着模型输出能自己提高它。

| 项目 | 建议初值/规则 | 与旧候选的关系及依据 |
|---|---|---|
| image 输入/输出媒体 | 每方向最多8张；每图16 MiB解码bytes，每方向合计32 MiB | 保留首版防护值但改可覆盖；不是provider上限。输入计text/image顺序，不要求非空prompt；输入blocks整体非空。 |
| image text/body | 每方向text合计64 KiB UTF-8；编码request与HTTP解压后的success JSON各48 MiB | 预算，不是token限制。base64编码膨胀计入request body；同时满足各独立预算，不承诺达到媒体最大值时任意文字仍可加入。 |
| MIME/容器 | 首批PNG/JPEG/GIF/WebP；校验base64/MIME/容器签名，不解像素 | 保留已确认媒体策略；此为已支持格式集合，不能用数字override绕过未实现格式。新增格式需实现与fixtures。 |
| image choices | 明确只处理first choice；不要求整个响应恰好一个choice | 旧exactly-one是额外拒绝，pi实际有相反证据；建议修订。first choice内坏条目仍不可跳过。 |
| classifier 输入 | 最多64题、每choice最多255 labels；state/完整request编码≤1 MiB、JSON深度≤32 | 保留初值为可覆盖SDK预算。ID/label唯一；最少1题、choice/score至少2项保留为有意义问题的构造约束，非已证实System One服务最小值。 |
| System One score | **删除固定10档上限**；受总request字节/深度预算限制 | 10来自llama单token标签实现，误套到System One；不是简单改成另一任意“协议上限”。未来llama adapter才执行自己的2–10硬限制。 |
| classifier 响应 | HTTP解压后的success JSON≤2 MiB | 可覆盖SDK预算；共享较小error-body限额继续有效。 |
| 数值容差 | 若U-2选择sum/argmax严格校验，sum容差1e-6曾为首轮备选；Q28选择不默认执行sum/argmax；**不要求System One score distribution/mean检查** | 原score-mean阈值对不存在的必需字段不可执行。不得归一化或伪造distribution来“通过验收”。 |

## 淘汰与迁移的闭合条件

| 项目 / ID | 当前职责/依赖者与处置原因 | 替代/保留与有意变化 | 影响面、过渡和退出证明 |
|---|---|---|---|
| 旧 Config/ProviderConfig/Error/Result、Model/ProviderKind；SP-16 / 保留独立淘汰评估 | 公开配置/身份壳；O-003未找到进入现生产调用链的证据，不能当所有外部消费者不存在。与真正的model/auth/error重复表达。 | typed模型目录、有效binding、ProviderError/ResolvedAuth；SDK配置能力仍可通过这些真实入口使用，不要求依赖产品OpiConfig。保留ApiKind及持久化消费者。 | root/module exports、docs/examples、构造型测试；scoped 0.x同步break，无默认永久shim。未来Unreleased说明。编译当前消费者、旧symbol生产/当前doc静态归零；不删除历史研究引用。 |
| supports/bridge旧键；SP-17 / 建议 | 旧键部分无opi wire读点，但pi有行为；贸然删除会使缺实现永久化。 | SP-01–03能力补齐，按字段表合并表达；源输入支持明确 false。旧键改报migration error，而非无声变默认。 | 所有profile/model/config merge/schema/factory、SDKstruct和文档；先新payload proof，后原子迁移调用方与删除旧reader。退出时仅一条compat解释路径；删除仅断言“字段存在”的测试，保留并扩展behavior corpus。 |
| 静态 Chat/Responses reasoning_effort；SP-18 / 建议 | 旧profile metadata，当前真正wire由请求+model map决定；重复值会误导。 | 不增加新的默认effort来源；每call请求仍有效，明确off仍有效。legacy任意字符串不能悄悄赋予新含义。 | 同上配置位置和Responses构造拷贝；错误指向ThinkingConfig/model map；扫描旧读写/导出，保留请求effort与off fixtures。无持久化格式变化证据，不扩大迁移范围。 |

SP-16的owner为SDK公共表面及当前调用方，SP-17/18的owner为SDK compat normalizer与产品配置parser；trigger都是未来有界0.x迁移，不是本轮研究。SP-16–18的失败统一为明确编译期/API迁移错误或配置解析错误，发生在auth/network前；机械验收除静态扫描外还必须经过公共新入口的payload行为验证。任何替代未完成的项不得先删字段再称后续补齐；有真实外部兼容承诺时重新讨论期限，不由实现 Agent自行留无限期双路径。

## 三个已收敛取舍（保留原候选与代价）

1. **U-1：AWS 来源完整性。** pi scoped env逐字段回退ambient，当前opi对部分来源失败关闭。推荐以完整来源为单位：typed explicit或scoped pair一旦尝试但不完整则失败，不拼接ambient；同时按pi保留 explicit/scoped profile优先、ambient profile不压过完整env pair。代价是与pi的部分scoped覆盖接受集不同；收益是避免形成未明确选择的身份。替代是精确对齐pi逐字段overlay并明确provenance。不能称推荐是Rust强制。本项不要求用户决定未知AWS SDK内部顺序。
2. **U-2：System One 数值验证接受集。** 推荐基础typed/finite/定义域/请求label完整性验证，额外question ID按pi忽略；sum/argmax作为明确选择的严格校验，而不是默认拒绝任何远端舍入差。score只接收score/confidence并验证范围，不要求distribution。替代是延续旧Q21严格sum/argmax（接受集更窄、需冻结容差fixtures），或完全复制pi当前有限检查（更宽，但可接受领域无效值）。主 Agent应把旧“完整distribution均值验证”作为无依据要求撤回，无须让用户猜一个字段。
3. **U-3：SDK高级请求扩展的权限。** pi允许onPayload替换最终body、headers覆盖和onResponse观察。推荐提供可信caller的可选payload transform/response observer；不因此新增通用框架。transform可改非身份协议参数，但最终body仍经过预算/JSON/wire检查，改model/provider/endpoint或auth身份则拒绝并要求调用者重新prepare；普通headers继续使用provider-managed auth保护。这明确比pi任意替换收窄。替代是首批仅typed options+只读观测，列出payload改写能力缺口；若完全复刻任意payload/auth-header覆盖，则需重新定义binding与actual identity，不能仅追加fact掩盖已失效的模型能力预检。这个选择实质影响binding和鉴权，不宜让实现 Agent代选。

当前状态：U-1由Q24/D-21及D-28保留完整选定来源失败关闭；U-2由Q28/D-25选择基础typed/finite/domain/label完整性，无sum/argmax；U-3由Q30/D-27选择binding约束的可信扩展。预算、有序输入/first-choice也由Q28确认。完整Node chain与image timeout已由附录A–D补核，不再列为未查明的目标事实。上述是当时44 AIR/38 V阶段；当前精确要求以53 AIR及V-01–45为准，SDK管理/资源/retry补证见 sdk-consumer-contract 和 D-35/36。

本分项实际工作：读取本地源码、测试断言、manifest/lock与既有研究；无新网络查询、无live、无安装、无runtime改动。测试影响：**none（本轮）**；未来设计为 **add** 操作/compat/来源conformance、**update** 配置迁移和有效wire断言、**delete** 无行为字段存在测试、**retain** 已有有效能力/脱敏/取消/资源保护。文档不意味着实现已通过。

实际文档检查：`python scripts/opi-doc-check.py` PASS；本文件SP-01–18唯一、无尾空白、末尾LF。全局`git diff --check`报本分项未修改的`docs/research/pi-1.0.0/source-manifest.json`换行尾空白；未修该共享改动。本文件未跟踪，因此另以UTF-8文本逐行检查，未声称Git已检查未跟踪内容。

## 补证附录 A：精确锁定发布源码（Q24 后续调查）

> 2026-10-03补证。Q24已确认SDK能力方向；本节只补可查明事实，不扩大SP范围、不替代其他尚待主Agent综合的选择。本节结论更新正文中“完整链/图片默认timeout未核验”的历史状态。没有安装、import或执行任何下载包，没有读取真实env/profile/token/cache，没有访问AWS/OpenRouter/OpenAI服务。

实际使用了 agent-reach 的 GitHub/gh 路由并读取其 `references/dev.md`/`web.md`。`gh api repos/aws/aws-sdk-js-v3/git/ref/tags/v3.972.82` 返回404，因此没有把repo main当发布源码；`gh api repos/openai/openai-node/git/ref/tags/v7.19.0` 返回commit `15f6e408a0359df50ee7a996616957cc14988659`。随后只读下载pi根lock的`resolved`官方npm tarballs，逐包计算SHA-512并与lock的integrity严格相等后才解开文本成员。未运行npm、构建/生成脚本或包内代码。

临时源码根为 `C:/Users/Luiz/AppData/Local/Temp/opi-sdk-source-6q2wc11l/`；每包目录名将`/`改为`__`。下表链接和lock足以重新取得同一发布物，不依赖临时目录长期存在。16个发布包全部integrity匹配；SHA-256为本轮下载文件指纹。

| 包 / 精确版本 / 官方发布物 | SHA-256 |
|---|---|
| [credential-provider-node 3.972.82](https://registry.npmjs.org/@aws-sdk/credential-provider-node/-/credential-provider-node-3.972.82.tgz) | `2f8732219e361edc37fdb26fec538c8037d292daab9e6f30eb7166911a6b200c` |
| [credential-provider-env 3.972.70](https://registry.npmjs.org/@aws-sdk/credential-provider-env/-/credential-provider-env-3.972.70.tgz) | `0a81fd770c8807da5854029f75712627f8ef9ab072789f10fe75bdc388affe35` |
| [credential-provider-http 3.972.72](https://registry.npmjs.org/@aws-sdk/credential-provider-http/-/credential-provider-http-3.972.72.tgz) | `2fdfd740abe32d1bf93fd12fe3bc7aecdf253b56ed00de36555178b6eb681a53` |
| [credential-provider-ini 3.973.15](https://registry.npmjs.org/@aws-sdk/credential-provider-ini/-/credential-provider-ini-3.973.15.tgz) | `35541ba7111317142d225fd619bd51060c756dca7c369f40798ebf5066230dd6` |
| [credential-provider-process 3.972.70](https://registry.npmjs.org/@aws-sdk/credential-provider-process/-/credential-provider-process-3.972.70.tgz) | `09d70d8a8a3ce2191f4cdcac64b0f509d5a59e9953f915ea5d1736f4ebffda13` |
| [credential-provider-sso 3.973.14](https://registry.npmjs.org/@aws-sdk/credential-provider-sso/-/credential-provider-sso-3.973.14.tgz) | `f6d80983c384e09007dcebc89e465941ffee15d6949ad52a782b6851369ec8de` |
| [credential-provider-web-identity 3.972.76](https://registry.npmjs.org/@aws-sdk/credential-provider-web-identity/-/credential-provider-web-identity-3.972.76.tgz) | `633b4cf4eaf7dda2eaeadf67cad5e721982fe2103169911ba7dff18455269235` |
| [credential-provider-imds 4.5.2](https://registry.npmjs.org/@smithy/credential-provider-imds/-/credential-provider-imds-4.5.2.tgz) | `01ce4798200c999ed16b932c90adfa63e83cd9b437bd8809c3ac68ced0310612` |
| [client-bedrock-runtime 3.1127.0](https://registry.npmjs.org/@aws-sdk/client-bedrock-runtime/-/client-bedrock-runtime-3.1127.0.tgz) | `cb983616e2e7a81d0b9f454ca3bbc0892a58e3997e188a82f6c69d65191ab6ed` |
| [AWS core 3.977.9](https://registry.npmjs.org/@aws-sdk/core/-/core-3.977.9.tgz) | `553bcc5d5191b751bdf56f21a793f320fa9383c2e026b6dcc3988903e50eab8b` |
| [Smithy core 3.33.3](https://registry.npmjs.org/@smithy/core/-/core-3.33.3.tgz) | `3fdfde668cab2a1cc97e674dc9e93e6f9c49641690a83b6c4abffce0560c4a87` |
| [token-providers 3.1127.0](https://registry.npmjs.org/@aws-sdk/token-providers/-/token-providers-3.1127.0.tgz) | `d5d3a58a713bbedf87cbfb6a616a0057d1718fd46438bb1cd64b5517cf3c95c0` |
| [signature-v4 5.7.3](https://registry.npmjs.org/@smithy/signature-v4/-/signature-v4-5.7.3.tgz) | `f07d2e1a06b1c2c181ae4bffefc3ed16c20938df87e3b3e08df03e1a3c533c95` |
| [credential-provider-login 3.972.77](https://registry.npmjs.org/@aws-sdk/credential-provider-login/-/credential-provider-login-3.972.77.tgz) | `c8f4051ea210213c7bf60fc0c4e757e295c5e33ea3addc1ef98b678690661234` |
| [nested-clients 3.997.44](https://registry.npmjs.org/@aws-sdk/nested-clients/-/nested-clients-3.997.44.tgz) | `bfadc489d5343dc2908b9bb8c0a462d85fbca0ed6a41f7cf73b7c67ed5303951` |
| [OpenAI 7.19.0](https://registry.npmjs.org/openai/-/openai-7.19.0.tgz) | `597532dcb9051ee8aa5edadedb57b798b42add68cc5aec1d67e994d0cfad07f1` |

下文`包名/path:line`指上表发布包解开后、去掉tar的`package/`前缀的文件。只摘录少量标识符，结论来自静态路径追踪，不声称已运行这些SDK或远端服务。

## 补证附录 B：完整默认链的选择与失败接受规则

### B-1 可以固定的顺序

`credential-provider-node/dist-cjs/index.js:96–159`明确按以下顺序尝试；末尾产生不可继续的“无来源”错误。每个阶段只在前一个阶段抛出的错误允许继续时进入。

| 次序 | 输入、选择规则 | 失败如何影响后续 |
|---|---|---|
| 1 Env | 若`init.profile ?? ambient AWS_PROFILE`为真，跳过Env；否则读取ambient完整access/secret pair、可选session/expiration/account/scope。 | 缺pair为`CredentialsProviderError`默认`tryNextLink=true`。profile跳过同样标true。**SDK自身profile压过env；pi只ambient profile时会先直接注入env pair，从而绕过这一规则。** |
| 2 显式SSO | 只有init有任一ssoStartUrl/accountId/region/roleName/session时调用；普通profile的SSO在INI阶段处理。 | 全部未给则继续；给了不完整的SSO配置为false，停止。 |
| 3 INI | `profile = init.profile ?? callerClientConfig.profile`，其后getProfileName取显式→ambient AWS_PROFILE→default。同名profile按字段合并config和credentials文件，后者覆盖前者。 | 文件读取/parse失败被loadSharedConfigFiles吞为{}；找不到可用profile/不支持credential_source/循环profile多为默认true。不得把pi此行为误写为“全部文件错误fail-closed”。 |
| 4 Process | 针对同一选定profile解析`credential_process`；即使它已在INI阶段失败，也可能再次执行。 | 缺命令、执行/JSON/Version/缺key/过期失败统一包装为默认true，可继续。不是失败就永久锁定该账户，也不意味着命令无副作用。 |
| 5 TokenFile | 显式webIdentityTokenFile/roleArn/sessionName优先ambient对应字段；读取token后STS AssumeRoleWithWebIdentity。 | 缺file或role配置为默认true；实际readFile失败、原生STS失败及无效STS结果无true标记，停止。 |
| 6 remote | ambient ECS relative/full URI存在时使用`chain(fromHttp, fromContainerMetadata)`；否则若`AWS_EC2_METADATA_DISABLED`为非空且不是字符串false则停止remote；否则IMDS。 | 不在“ECS失败后自动试IMDS”这一层继续；ECS两子provider都失败即到链末尾。禁用IMDS只约束此默认remote分支；显式INI credential_source路径直接调用它自己的provider。 |

证据细节：node `index.js:5–18,78–94,96–159`；env `dist-cjs/index.js:10–38`；Smithy core `dist-es/submodules/config/shared-ini-file-loader/{getProfileName,parseKnownFiles,mergeConfigFiles,loadSharedConfigFiles,readFile}.js`。`parseKnownFiles:5`调用`mergeConfigFiles(configFile,credentialsFile)`，不是二选一整份文件。

外层还会影响来源字段覆盖：pi `getConfiguredBedrockCredentials:1185–1196`只把access/secret/session注入client，没有传`AWS_CREDENTIAL_EXPIRATION`、account或credential scope。因此不能从SDK Env provider支持这些字段就声称pi每条env调用都保留了它们。SDK内部多数env读取是ambient `process.env`；pi options.env只影响它显式读取/转交的字段（如profile、pair、region、bearer），不等于自动注入所有STS/ECS/IMDS环境变量。目标的受控来源注入应明确映射这些字段，避免为了测试改全局env。

INI内部顺序固定于`credential-provider-ini/dist-es/resolveProfileData.js:8–35`：递归source-profile已有static pair先取static；一般入口先assume-role/source-profile，再static、web-identity、process、SSO、login_session。assume-role要求source_profile和credential_source二者恰取一种；source可为Environment、EcsContainer、Ec2InstanceMetadata。MFA缺callback是明确终止错误；role参数保留role_arn/session_name/external_id/duration_seconds/MFA，STS返回expiration/account信息。对应 `resolveAssumeRoleCredentials.js:59–75`、`resolveCredentialSource.js`和`nested-clients/dist-es/submodules/sts/defaultStsRoleAssumers.js:21–94`。不能把完整profile支持缩成access/secret两个字段。

### B-2 继续规则不是按错误文字猜测

node `internalCreateChain:78–94`仅在`err?.tryNextLink`为真时继续；普通Error立即抛出。Smithy core `dist-es/submodules/config/property-provider/ProviderError.js:5–16`默认tryNextLink=true，显式false覆盖。至少固定以下离线判别：

- **继续**：未配置env、profile无匹配凭据、缺credential_process、process执行/输出解析失败、未配置web identity、某些HTTP metadata错误。这些在调用尚未成功选定身份前属于来源发现，不等于HTTP 401/403之后跨账户重试。
- **停止**：SSO字段不完整、SSO token不可读/失效/过期、SSO服务拒绝或返回不完整临时凭据（`credential-provider-sso/dist-es/{validateSsoProfile,resolveSSOCredentials}.js`，后者`:4,26,37,44,66,73`均false）；MFA必需但无callback；web identity文件读取/STS的普通Error；后备container URL/host/protocol无效（imds `fromContainerMetadata.js:51–75`）。
- **login_session**：已发布链确实包含AWS login缓存，不是未来规划。`credential-provider-login/dist-cjs/index.js:13–179,271–290`：无login_session允许继续；已选session的坏/缺缓存、DPoP失败和特定AccessDenied为终止；其它刷新失败有未到期token则继续用它，到期后抛默认可继续错误。target source覆盖表应列此能力，不能声称SSO等于AWS login。

**SP-06/08的证据影响**：可将上述链次序和显式错误分类冻结为baseline corpus；不能把所有非成功来源一律fail-closed并称pi等价。U-1关于partial scoped/ambient pair的更严格完整来源规则是独立有意选择，不覆盖整个默认链的所有missing-source情况。完整chain并不等于每次失败都fallback，也不等于解析成功之后可以偷偷换身份。

### B-3 远端来源、过期与缓存的实际边界

`credential-provider-http/dist-es/fromHttp/fromHttp.js:13–71`：relative URI优先full URI；authorization token file优先直接token；默认连接/请求timeout 1000ms、maxRetries=3（最多4次）且retry delay默认1000ms；finally销毁handler。`checkUrl.js`允许HTTPS或指定metadata/loopback host；`requestHelpers.js`验证返回access/secret/token/expiration后构造临时凭据。其后备旧container provider默认timeout 1000ms、retries=0，且有更窄host/protocol判断。两者不是同一请求的两次同名调用。

IMDS `dist-es/fromInstanceMetadata.js:14–112`先取v2 token；token400停止该次请求，timeout/403/404/405启用后续v1模式，其他token错误也会尝试本次v1 fallback；v1可由显式ec2MetadataV1Disabled、env或profile禁止。角色列表/凭据401会清除disableFetchToken以重新取token。默认remote超时1000ms/retries0来自`remoteProvider/RemoteProviderInit.js:1–3`。endpoint可由配置/IPv4/IPv6模式决定（`utils/getInstanceMetadataEndpoint.js`及对应config文件）。这些事实用于fake transport，不能在研究或默认测试中访问metadata地址。

node `memoizeChain:21–76,161–162`的缓存有独立状态：无值时active lookup共享promise；未过期但距expiration小于300000ms时返回旧值并启动单个passive刷新；passive失败吞掉，旧值保留到过期；严格早于当前时间的过期值被丢弃后阻塞取新值。无expiration值缓存不主动刷新。forceRefresh共用另一锁并等待结果，失败传播。三把锁并不证明全局generation顺序，不可据此承诺并发刷新绝不发布旧结果。

该缓存属于**返回的provider实例**。AWS core `resolveAwsSdkSigV4Config.js:106–134`有credentials时规范化显式identity；否则构造defaultProvider；pi Bedrock每次stream重新new client，因此不能将SDK缓存描述成pi跨所有call的单例credentials cache。另一方面，Smithy `shared-ini-file-loader/readFile.js:1–12`确实有模块级filepath→promise缓存，ignoreCache才重读；新建Bedrock client并不等于profile文件一定重新读取。

SSO现代session token在5分钟窗口尝试OIDC refresh，每session最多每30秒发起一次刷新；未过期时失败可继续使用旧token，过期时失败。成功尝试写标准SSO cache，但该写失败被忽略（token-providers `dist-es/fromSso.js:48–83`）。legacy SSO没有这条现代token刷新路径，过期要求重新login。AWS login在5分钟阈值读cache/刷新并尝试写cache；此副作用属于完整链支持范围，不能将“凭据发现”宣称只读文件。

IMDS还有`utils/staticStabilityProvider.js:1–23`与`getExtendedInstanceMetadataCredentials.js:1–16`：对其provider实例中旧凭据/过期响应可延长expiration 5–10分钟并保留originalExpiration。**不要过度推导**：node默认链每次走remote分支会重新构造remote provider，因此不能据这个wrapper就承诺默认链跨refresh总持有pastCredentials；但过期响应延长确实位于实际返回路径。opi若禁止用原始过期凭据，应记录为有意更严格差异，不能假称锁定SDK本来如此。

### B-4 可以验收什么，仍不能声称什么

现证据足以固定主来源顺序、profile叠加、fallthrough分类、关键cache/expiry与现代SSO/login副作用；这些不再是要用户回答的技术事实。离线验收至少用source call log证明顺序和短路，fake INI覆盖role/static/web/process/SSO/login，fake clock覆盖五分钟/真正过期/forceRefresh，fake transport覆盖STS/ECS/IMDS及SSO/login刷新；禁止用全局真env或真实cache代替fixture。

这不要求机械复制实现偶然：INI失败后同一个credential_process可能再执行、模块级file promise长期不刷新、三类刷新锁的潜在覆盖顺序、pi漏传env expiry，均应按可观察目的与已有错误/取消/来源契约审计。验收应首先证明source接受集、明确停止/继续规则和有效凭据，而非以“执行同一命令两次”或“缓存永远不失效”作为正确性目标。

尚需目标实现验证的是：选择的Rust机制是否实现这些来源/平台、取消是否贯穿process/STS/OIDC/metadata、cache是否有唯一owner及过期结果屏障、返回结果是否泄密；当前没有被选定且锁定的Rust AWS依赖，因此不宣称已证明某crate满足这些条件。SDK现有Node实现也没有把pi的abort signal贯穿所有credential subrequests的明确证据；不能只因为外层client.send接signal就宣称取消完整。

pi自身还有入口可达性差别：`amazon-bedrock.ts:54–78`不探测default-profile文件和纯IMDS，而`Models.applyAuth:853–854`会拒绝undefined。direct adapter/显式DefaultChain能力与“自动configured列表一定显示所有可用AWS来源”不是一件事。目标可提供已确认的显式SDK DefaultChain装配，不需要复制此探测遗漏；若宣称Auto也覆盖所有来源，应增加公共入口fake链验收，而非仅验证内部resolver。

## 补证附录 C：OpenAI 7.19.0 图片timeout/retry

真实依赖由pi `packages/ai/package.json`与lock的`packages/ai/node_modules/openai`共同锁到7.19.0，不是root 6.40.0。发布包 `src/client.ts:608,618,1992`给出client默认timeout **600000ms**、maxRetries **2**；`:1797–1799`请求timeout覆盖client值，显式值须positive integer；`:1193–1195`请求maxRetries覆盖client默认。

pi `src/api/openrouter-images.ts:67–81`对每次SDK request传maxRetries=0，只有显式timeoutMs才传timeout；外层`retryProviderRequest`默认0。因此首批image目标的基线是：默认SDK fetch timeout 600000ms、默认0重试、显式maxRetries由pi外层管理；不能把SDK constructor的默认2当作pi image默认retry。每次外层retry新建SDK request，从而重新取同样timeout。

**计时范围有实质限制**：client `fetchWithTimeout:1610–1652`在fetch前setTimeout、fetch返回Response后finally clearTimeout；之后 `src/internal/parse.ts:18–88`再读取response.text并JSON.parse，`src/core/api-promise.ts:69–80`的withResponse等待解析。默认600000ms因此不是包括auth、hook、完整body下载、parse、retry/backoff在内的总deadline；对普通fetch，它连headers之后的全部body读取都不持续计时。System One的AbortSignal.timeout则位于包住fetch和next.json的attempt closure内，是不同机制。

SP-13可据此固定image默认数值与retry归属。若目标将同一timeout明确覆盖完整HTTP body，这是有意修正，建议保留但不能称精确复现锁定OpenAI timer；已有可选overall deadline也不能用600000ms冒充强制整体预算。验收用假fetch将headers立即返回但body挂起，区分header timer、body预算和caller取消；本轮没有执行该SDK测试。

## 补证附录 D：AWS_BEDROCK_SKIP_AUTH 不是无签名模式

链路已可静态闭合：pi `bedrock-converse-stream.ts:181–215`在skip标志时不主动注入bearer而给dummy access/secret；Bedrock发布包 `dist-es/auth/httpAuthSchemeProvider.js:40–46`默认先SigV4、再Bearer；`runtimeConfig.js:37–53`注册两种signer；AWS core `AwsSdkSigV4Signer.js:27–55`调用signer.sign；Smithy signature-v4 `SignatureV4.js:107–129`计算签名并写Authorization。凭据validator `SignatureV4Base.js:61–67`只要求对象及string字段，dummy strings不会关闭签名。**默认没有其他偏好时仍发送dummy SigV4 Authorization，不是NoAuth。**

更重要的反例：AWS core `dist-es/submodules/httpAuthSchemes/aws_sdk/NODE_AUTH_SCHEME_PREFERENCE_OPTIONS.js:5–19`先检测ambient `AWS_BEARER_TOKEN_BEDROCK`是否存在，并自动偏好httpBearerAuth；之后才看AWS_AUTH_SCHEME_PREFERENCE/profile设置。pi skip路径没有显式覆盖此preference。Bedrock runtimeConfig在无显式token时仍以`fromEnvSigningName`解析ambient bearer（token-providers `dist-es/fromEnvSigningName.js:4–17`），或进入token nodeProvider的SSO路径。Smithy `middleware-http-auth-scheme/resolveAuthOptions.js:1–21`排序，`httpAuthSchemeMiddleware.js:11–40`直接await选定identity且不catch其失败改试下一scheme。因此skip标志既不保证无签名，也不保证不采用ambient bearer；找不到被偏好scheme的identity还可能直接失败。

这是**实现事实与潜在缺陷**，不是已证实的设计承诺。opi目标不应把SKIP_AUTH照译成“自动放dummy但绝不触碰真实凭据”的保证。若纳入无鉴权代理能力，应以显式无鉴权选择保证resolver调用数0、Authorization/x-amz credential字段不存在；若选择兼容dummy签名代理则单独命名实际行为。Q24的SDK能力方向不要求复制ambient bearer绕过skip意图的行为。这里补事实而不自行新增NoAuth产品开关；主Agent综合时须避免再把skip-auth当未查明或无副作用。

本附录检查记录：官方gh exact tag一成功一404；16个官方npm发布包下载且lock SHA-512校验成功；静态阅读与路径追踪。测试影响仍为**none**，无真实凭据、无模型/metadata/OIDC请求、无依赖执行。仅改本文件；其他候选/决定由主Agent顺序整合。

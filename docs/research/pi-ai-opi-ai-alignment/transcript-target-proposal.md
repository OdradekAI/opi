# Transcript、动态工具、replay 与参数规范化目标提案

> **非规范性目标细化，2026-10-03，已同步第二轮回答。** [D-19/Q22、D-20/Q23、D-22/Q25](decisions.md#第二轮目标决策d-18之后)已确认动态transcript、replay/normalization和可信hook方向；D-28记录主Agent依据这些方向收敛的pi式compaction与plain-JSON语义，不冒称新用户回答。本文保留提案推理与替代方案，当前选择以上述决定为准；不授权实施、不代表运行时已实现。

读法：先看“目标条款”，发生差异时查对应精确规则与证据；最后看三项已收敛的实质取舍及其依据。已有源码/历史依据集中在 [RT01–RT08](reassessment-transcript-notes.md)，本文只补充形成可执行契约所需的局部证据。术语沿用 `docs/CONTEXT.md`；下文的 registered、active、wire-visible 是工具不同使用位置的说明，不建立新 GLOSSARY 或第二份状态账本。

## 当前事实与已选目标分界

| 已核验的当前事实 | 当前目标选择（未实施） |
|---|---|
| pi 1.0.0 用有序 system content、named sections、tool 增删重定义表达变化；不支持的模型自动 collapse。 | Rust 表示这些语义；默认自动适配，Strict 为显式策略。所有权可不同，但不因内部简化丢掉变化的身份。 |
| pi 可信宿主可在同会话更换工具；历史声明不持有 execute。 | Rust 为每次模型调用及其响应工具批次持有稳定可信 revision，后继调用原子切换；不默认因普通动态变化产生 child。 |
| pi 压缩保存**压缩时当前** system/tool checkpoint，并去掉压缩前 retained 区间中的 system 更新。 | 推荐对齐此有损压缩政策；不承诺 retained 旧调用当年的每个 system 边界原样保留。 |
| pi 合成结果缺失 error result；Length 不执行工具但生成结构化失败结果。 | 对齐中性恢复，canonical 与 request projection 分开；对无法关联的损坏明确失败。 |
| pi 参数处理包含 optional-null、TypeBox 转换、plain-JSON 转换与校验；可信 hook 之后可以改成超 schema 值且不再校验。 | 推荐 Rust 采用一个明确的 plain-JSON 规范化契约；D-22已确认trusted hook可超schema执行，最终authorizer仍审实际值。 |

## 目标契约细化

TP条目细化已选目标，仍是非规范性设计输入；下表保留“建议”措辞的工程选择不推翻D-19/20/22/28。“失败”描述目标契约，不宣称当前代码已实现；“验收”均是未执行的机械验证设计。源码导航使用 PI=`.repo/pi-1.0.0/packages/`、OPI=`crates/`。

| ID | Owner / 触发 | 建议行为、不变量与失败 | 机械验收 | 证据 |
|---|---|---|---|---|
| TP-01 | opi-ai 公共输入规范化；SDK/Agent 准备请求 | 保留初始 system/tools 便利构造，一次转换成 leading system entry；下游只有 canonical transcript 权威。明确 shorthand 先于已有消息的顺序；空初始值不凭空产生有意义的变更。adapter 不再独立读取旧 fields；冲突或非法结构在准备边界给 typed error。 | 直接 SDK 与 Agent 的同一初始输入产生相同 transcript/wire；同时提供 shorthand 和后续 system 时按既定次序求值；检索所有 adapter 不留第二解释路径。 | RT08；PI ai/types.ts:724–748，ai/utils/transcript.ts:10–34 |
| TP-02 | opi-ai transcript；初始/后续 system entry | content 在当前位置追加指令；named section 新名字插入、同名替换保留其顺序、null 删除、删除后重新加入在末尾。正文与 sections 分别保留，空 section 与删除不同；rendering 规则见下节。非法重复项的输入形状显式拒绝，不能靠 map 最后覆盖掩盖调用方错误。 | S0→追加正文→替换 section→删除→重加，逐步比较内容及顺序；空字符串/null、嵌入换行、section 名含数字的 fixture；序列化不重排 section。 | RT02；PI ai/types.ts:512–537，ai/utils/transcript.ts:73–101，ai/utils/text.ts:14–39 |
| TP-03 | Agent 可信 loadout 投影 + opi-ai transcript；工具变化 | declaration 保存 name/description/schema/constrained-sampling，不保存执行器；执行移除再增加。同名定义变化作为 remove+add，可变描述也是声明变化；未变化不重复。保留 tool 顺序和变化位置。schema 不合法或同一新集合名字歧义，在安装前失败。 | 增删、同名 schema/描述/sampling 重定义、删除重加、无变化、只换执行器的 fixtures；最后一项更新 execution revision，但不伪造模型声明变化。 | RT03；PI ai/utils/transcript.ts:122–166，agent/agent-loop.ts:323–362 |
| TP-04 | 产品 session/compaction；压缩选定 active branch | **D-28采用 pi 压缩政策**：checkpoint 是压缩提交边界的完整有效 system/tools；重建为 checkpoint+summary+保留的非 system 历史+压缩之后的条目。已纳入 checkpoint 的 system delta 不重复应用；后续变化仍有序。旧原始记录保持可检查，不能声称这是旧调用逐时点无损重建。失败不发布部分 checkpoint/leaf。 | 多次压缩、retained 区间含 section/tool 改动、sibling/fork、压缩后再次变更；比较压缩时 effective state 与压缩后起始 state，并验证后继变化；不要用所有 retained 旧 call 的原上下文相同作验收。 | 新补核：PI coding-agent/session-manager.ts:461–510,1261–1285；RT07 |
| TP-05 | opi-ai 集合/adapter；为选定模型编译请求 | 默认 native-or-collapse：仅对有证据的变化类别使用原位表达，否则采用当前 system/tools；canonical 不回写。模式/受影响范围可诊断。显式 Strict 若支持，表示拒绝无法保持**本次要求的时间位置**的投影，不等于字节级 HTTP 重放；拒绝在 auth/网络前发生。能力按 adapter+模型+变化类别，非一个万能 bool。 | 同一 transcript 用 native、仅 add、collapse 三类 fixture；remove/redefine 触发正确路径；Strict 失败零 auth/network；原始 transcript 不变。 | RT01；PI ai/utils/transcript.ts:104–119,196–232；ai/test/transcript-tool-changes.test.ts |
| TP-06 | Agent 可信更新与执行 owner；下一 call 准备 | trusted host/extension 可更换 active 集合、schema、实现和来源。候选 revision 原子安装；一个模型请求及其响应工具批次绑定同一 execution revision，直到批次结束/取消释放。更新只影响后继 call；当前 User Policy 可撤销授权，不能因冻结 revision 永久冻结 permission。失败保留前 revision，未执行项有明确结果。 | 旧 call 在新 schema/实现安装后仍使用旧绑定；下一 call 使用新绑定；并发完成/取消不混 revision；政策撤销仍拒绝；替换失败无半新半旧状态。 | RT03；PI agent/agent.ts:82–105,460–464；agent/agent-loop.ts:171–215；OPI agent/authority.rs:65–129 现有不可变装配是迁移影响，不是目标否决理由 |
| TP-07 | Agent 工具选择/授权；模型直接调用或可信嵌套调用 | 区分 registered 实现集合、当前调用上下文可执行集合、wire-visible 投影及当前授权。直接调用从其绑定 active/executable 集合解析；可信 orchestration 可传入自己的合法 callable 集合。隐藏声明本身不撤销执行权，历史声明/模型输出也不能新增执行权。unknown/上下文不可调用/policy deny 零执行。 | active 移除拒绝；hidden declaration 仍可由相应合法上下文调用；codemode callable 与 model-only 方向反例；历史伪造 schema/name 不注册；authorizer 收到正确实现/来源。 | RT04；PI agent/agent-loop.ts:707–775；coding-agent/agent-session.ts:1511–1571,1715–1737；tool-orchestration.test.ts:62–143 |
| TP-08 | Agent trusted hook、authorizer 与执行器；工具参数完成后 | raw→normalized→execution 三个阶段按 TP-12–15 记录。pi 默认参照是 prepareArguments→normalize/validate→可信 beforeToolCall→执行；hook 可以 block，后续 hook 看前序 mutation。**D-22允许可信hook改变execution而不复验模型schema，见TP-15**。无论选择哪一项，opi 最终 authorizer 审查并绑定实际 execution 值；之后若值或 registration 改变，旧授权不可复用。模型不能选择 hook、修改来源或越过 authorizer。 | prepare/normalize 后仍无效的输入未到 beforeToolCall；合法输入被 hook 修改后，各候选的允许/拒绝结果不同且明确；多 hook 顺序、block/cancel、授权参数与执行参数相等、旧授权不能覆盖新值。 | PI agent/agent-loop.ts:693–775，agent/test/agent-loop.test.ts:487–545；coding-agent/extensions/types.ts:1198–1203；OPI agent/agent_loop.rs:2235–2300、authority.rs:220–247 |
| TP-09 | opi-ai replay；前次 assistant Error/Aborted/Length | Error/Aborted 留 canonical、从模型 request view 省略，不执行其提案。Length 的全部工具提案都不执行，即使 JSON/schema 看似完整；可表达的 proposal+明确未执行 error-result 保留成对结构。产品 overflow/Length 恢复可省略整 attempt view，原始事实保留。坏 JSON 无法忠实组成 wire 时局部失败投影，不默默 `{}`。矛盾执行事实不能随失败状态一起抹掉。 | Error/Aborted/Length×有文本/有提案/缺结果/声称已执行；Length schema-valid 参数仍零执行；第二次真实 adapter request 的 pairing 和事件顺序正确；产品恢复没有 half-pair。 | RT06；PI ai/api/transform-messages.ts:191–215；agent/agent-loop.ts:263–277,471–502；coding-agent/agent-session.ts:1208–1223,2938–2999 |
| TP-10 | opi-ai request replay；关联明确但 tool result 缺失 | 为缺项生成 request-local error result，语义是 ResultUnavailable，副作用状态 UnknownEffect；这些是语义标签，不预定 Rust 类型名。只补缺项；不写成真正 canonical 结果，不声称 NotExecuted/回滚，不自动重执行。已知 NotExecuted 是更强的 Agent 事实，可另用于失败恢复。 | trailing batch、部分 result、system 插在 call/result 间、下一 user、跨模型 ID 映射；仅缺项被补；工具执行计数0；canonical 原文不变；生成文本明确缺失而非断言执行失败。 | RT05；PI ai/api/transform-messages.ts:158–186,222–232；0.69.0 CHANGELOG；官方 issue3555 |
| TP-11 | opi-ai replay/adapter；关联和内容转换 | 按批次位置+source ID 关联，一批内歧义重复、孤立/重复 result 明确失败；闭合批次后 ID 可复用。wire ID 需要合法化时使用 request-local 一一映射，call/result 同步。保留混合 text/tool 的原序；签名/opaque 仅在已知 provider+wire+requested model 匹配且 adapter 支持时回放，否则用既定可见文本/省略投影并记录。历史未知身份不猜。 | 同 ID 跨批次、碰撞候选、非法字符/截断、交错 text/tool、same/cross/unknown provenance；规范化不得掩盖顺序、关联或终止差异。 | R01–R08；RT05；PI ai/api/transform-messages.ts:95–144,205–215；原 AIR-010 可保留核心 |
| TP-12 | opi-ai 参数接收 + Agent 参数准备/证据；每个 tool proposal | 保留收到的 raw 参数及解析状态，规范化在副本执行；normalized 是按可信 schema 转换并成功校验的模型输入；execution 是可信 hook 完成后的实际执行值。三者不同可区分，raw 不被 coercion 覆写；trace/diagnostics 使用现有脱敏策略，不默认把全部原文写日志。schema/实现 revision 与最终授权值关联。 | 原对象/原始字符串 hash 不变；coercion 和 hook 两次不同变化均可辨；被拒绝时无虚构 execution；敏感 canary 不进入公开错误，受控 evidence 能正确关联。 | PI ai/utils/validation.ts:317–349；OPI agent/authority.rs:220–247；原 replay/canonical 证据要求 |
| TP-13 | opi-ai 纯参数规范化机制、Agent 调用；parsed JSON + trusted schema | 推荐唯一 plain-JSON 规则，精确转换见下节；先 optional-null，再 schema-guided coercion，再完整原 schema validation。只变换已有值；不自动填 default、不把 schema 不允许的 extra key 清掉、不用 wire strict schema 替代可信 schema。失败返回 typed、脱敏的验证错误并零执行；根参数也总经过完整验证。 | 精确 corpus 对每项检查 input/normalized/error/path/raw unchanged；包括 unions、nested arrays/objects、额外属性、边界数字、无效 root、schema 不合法；执行测试证明实际用 normalized。 | 新补核 PI ai/utils/validation.ts:59–269,317–349；ai/test/validation.test.ts；OPI agent/validation.rs:11–20 目前仅直接校验 |
| TP-14 | adapter constrained sampling；声明要求 prefer/require JSON strict | 对模型支持和可转换 schema 使用独立 wire 副本：对象全部 properties required，原 optional 且非 nullable 项加 null 分支，additionalProperties=false，递归规则见下节。原可信 schema 不变；返回的 optional null 按原 schema 规范化。prefer 不支持/不可转换则退 non-strict；require 则请求前 typed error。局部供应商限制必须参与判断。 | require/prefer×capability/schema 可转换矩阵；nested optional、nullable、refs/unions、不合法 required、provider 额外限制；wire 变化而原定义 byte/canonical equality 不变；本地校验继续生效。 | PI ai/api/constrained-sampling.ts:14–143,228–249；ai/test/constrained-sampling.test.ts:90–199；types.ts:705–719 |
| TP-15 | trusted hook 契约；TP-08 hook 返回修改后的值 | **已选 A（D-22/Q25）**：先校验模型输入，可信 hook 对 execution 值负责任，不再次按模型 schema 校验；authorizer 必审最终值。**未选替代 B**：允许 hook 改写，但重新验证；超 schema 值拒绝。B 是 opi 独立强化，会破坏 pi 已支持的 extension 行为；保留它只解释取舍，不实现双模式。raw/normalized/execution 来源可辨，模型无授权入口。 | schema string、模型值字符串、可信 hook 改为数字123：已选A进入 final authorization/execute；deny/cancel与注册来源检查保持。不得实现B的二次schema拒绝或增加模式开关。 | PI agent/test/agent-loop.test.ts:487–545；coding-agent/extensions/types.ts:1198–1203；OPI 当前 before hook 仅 Continue/Deny 且位于验证前 |
| TP-16 | schema SDK owner；选择 Rust 公共规范化契约 | **D-28采用 plain-JSON 为唯一契约**，不模拟 TypeBox 私有 `~kind` 导致的第二套转换。公开说明与 pi typed-TypeBox 路径的已知差异，使用实际序列化 JSON schema corpus；不得把 pi 两条路径说成一致。若选择 typed 路径，应另定义完整 portable 转换规则，而非接受模型提供 `~kind` 来切换行为。 | 同一 JSON schema 无论构造方式都得到同值；带未知私有标记不改变规范化；列出的 plain/typed 分歧均有固定结果；不引入“compat mode”开关来回避决定。 | 下节 TypeBox1.3.27 一手补核；PI ai/package.json:79，validation.ts:318–335 |
| TP-17 | 产品持久化/迁移；接受当前输入或继续旧会话 | 已接受输入在后续 provider/auth 失败或取消后保留；当前单事务 owner 管 context/checkpoint/leaf，遵守已有 commit ack 边界。旧历史原文缺失保持 unknown，在 continuation 位置追加当前可信状态；普通动态更新不要求 child。Q11 若仍选保源 child，应标存量迁移政策。额外双 durable append 不由本提案自动引入。 | commit前/ack后失败、取消与 compaction组合；accepted input 不丢，未接受输入不冒充已提交；旧源保真/unknown；continuation 与精确历史重建对缺口返回不同且明确结果。 | RT07；PI coding-agent/session-manager.ts:1160–1213；OPI agent/hooks.rs:211–224、coding-agent/compaction.rs:479–640 |

## TP-02/03 的顺序、渲染与变化身份

pi 当前实现：content 的 text blocks 以 `\n` 连接，各次非空追加以 `\n\n` 连接；完整 prompt 为累积 content 后接当前有序 sections，以 `\n\n` 分隔非空部分。native 后续 section 更新通过有名称的 updated/removed framing 表达。`ai/utils/text.ts:24–39` 明说该 framing 是 request-time、可以随版本改变，因此目标应固定其功能及 fixture，不把英文提示模板当永久持久化协议。

Rust 可用有序 entries 表示 sections；不依赖 JSON object 的整数样式键重排。section replace 不改变插入顺序，delete+add 改变顺序，空 section 保持身份但完整渲染不贡献文本。重复名称应在构造/解码后确定唯一性；如果采用 JSON object 输入，必须说明 parser 对重复 key 的处理，不能假装解析后还能找回已经丢掉的重复信息。

tool 声明身份涵盖 sampling config；execute/display 字段不进入模型声明。对象 key canonicalization 可以避免纯编码顺序产生无意义变化，但 declaration/array 顺序必须保留。这里的 canonicalization 是 Rust 数据表示选择，不许可把同名 schema 的实际改变抹成“同一工具”。执行器变化与声明变化分别判断，避免实现换了却继续把旧 execution revision 作为来源。

## TP-13：plain-JSON 规范化的精确参照

这是**推荐对齐的 pi plain-JSON 路径**，不是所有 TypeBox 原生对象的统一事实。前置输入是已解析的 JSON 值；字节级 partial/repair parser 是更早阶段，规范化不从缺损文本凭空构造 `{}`。pi `json-parse.ts:108–134` 的 streaming salvage 与最终转换不可混为一谈；本提案不据 JSON/schema 成功推断一次 Length 生成完整。

| 规则 | 精确行为与失败界线 | 必须有的 corpus |
|---|---|---|
| optional null | 克隆参数后，对 properties 中已有且不在 required 的 null：当 property 没有字符串 `$ref` 且独立子 schema 校验明确拒绝 null 时删该 key；否则保留。递归沿 properties 和 items/tuple items。pi 不在此阶段普遍遍历 allOf/anyOf/oneOf/additionalProperties，也不解 `$ref` 再删 null；其余由最终校验判断。 | optional number null→省略；required number null→后续转换0；optional nullable null保留；带 `$ref` null保留交给最终校验；nested object/array；无关 extra null不偷偷删 |
| number/integer | null→0，bool→0/1；非空白字符串经 JavaScript Number 语义解析，number 要 finite，integer 要整数；失败保留原值交最终校验。已有合法同型数值保持。Rust 实现需有显式兼容数字语法与有限数值检查，不能用默认 `parse` 冒充等价。 | `"42"`、`"42.0"`、`"42.1"`、空/空白、正负号、指数、`"0x10"`、`"0b10"`、`"0o10"`、`"Infinity"`、`"NaN"`；finite溢出与2^53附近 |
| boolean | null→false，字符串仅精确 `"true"`/`"false"`，数字仅1/0；其他值不变、随后校验。 | `"TRUE"`、`"1"`、`"0"`、2失败；true/false/1/0/null对应值 |
| string | null→空字符串；number/bool 用相应 JavaScript String 表示；对象/array不 stringify。 | 0/-0、指数边界、true/null、对象/数组；不能用 Rust Debug 输出替代 |
| null | 仅空字符串、数值0、false→null；其他不变。 | `"null"`、`"0"`、`"false"`不转；0/-0/false/空串转 |
| type 数组 | 若现值的 JSON 类型已匹配其中任一类型，先保留它；否则按 schema type 顺序尝试，选首次实际改变的 primitive 转换，再最终校验。 | number/string 输入`"1"`保留字符串；boolean/number 输入`"1"`→1；数组次序与最终 constraint 失败 |
| allOf、anyOf、oneOf | 先按顺序应用 allOf；再 anyOf；再 oneOf。union 先检查原值是否已通过任何完整 arm，若是则原值保留；否则逐 arm 在克隆值上转换，选首个通过该 arm 的值；最后仍按整个 schema 校验，尤其 oneOf 必须仅匹配一个 arm。 | nullable null保留；`"42"`选number；两arm都能匹配的oneOf最后拒绝；原值有效时不为了首arm改变类型 |
| 对象/数组 | 仅存在的 properties 递归；schema-valued additionalProperties 对未知 key递归；数组 items/旧式 tuple items按对应位置递归。不生成缺失 key，不删违反 additionalProperties 的 key，不包装标量为数组、不解析字符串成对象。 | nested object/array/tuple，schema-valued extra，required缺失，additionalProperties=false，scalar→array/object拒绝 |
| 最后校验 | 规范化后针对原可信 schema 完整校验；失败无执行。不能以“转换有返回值”代替校验；错误正文脱敏，raw另存。 | enum/min/max/pattern/oneOf 失败；根标量转换后仍不符合enum；对象键与required错误定位 |

上表最后一行特意不复制 `validation.ts:325–332` 根值转换分支中“转换值失败时直接返回原值”的宽松出口；该出口源码可见，但本次未执行 repro，不能称上游已确认缺陷。Rust 不应因此把不符合 schema 的模型参数当验证成功。pi 常规工具定义为 object，故需区分公开 helper 边界与实际产品路径。

数字语法/格式的差分 corpus 应固定有限值、指数和 IEEE-754 表示规则。Rust 可保留 raw JSON 的精确整数表示，但 coercion 对超出安全整数范围的字符串是额外选择；推荐先复现 pi 的可表示有限 Number 结果，同时保留 raw，**不要**偷偷改成拒绝全部大整数或将舍入说成无损。具体算法和库选型是实施判断，不能把边界结果留给 Agent 猜。

## TP-14：strict nullable 是 wire 转换，不是执行 schema

pi 的 strict 转换在副本上进行：root 必须 object；properties 递归转换；原 optional 且尚不允许 null 的 property 变为 `anyOf: [原property, {type:null}]`；required 改为全部 property names；additionalProperties 设 false。原本允许 null（type/null、type数组含null、const/enum含null、anyOf中允许null）不重复包裹。随后实际工具参数仍对原 schema 做 optional-null normalization 与校验。

当前可转换子集明确拒绝：boolean schema；`$ref/$defs/definitions/allOf/oneOf/patternProperties/dependentSchemas/dependencies/unevaluatedProperties/propertyNames/contains/prefixItems/not/if/then/else`；已有 anyOf 空或非数组；已有 anyOf 中的 object/array arm；tuple items；没有 type object 却有 properties；additionalProperties=true/schema；properties非map；required非字符串数组或引用不存在的property。provider 可再拒绝具体 keyword/value。自动对 optional property 增加的 nullable wrapper 是转换产物，不能误把它与**输入中已存在的** structured anyOf 混为同一前置拒绝条件。

require/prefer 描述 sampling 约束，不授予执行权限。prefer fallback 对模型采样约束变弱，本地校验仍存在；require 失败不能自动改 prefer。未配置/false 表示不主动要求此采样；保留各 adapter 原有默认字段语义的 fixture。Grammar 是另一条已发布契约，不能以本节 JSON strict 已完成就宣称 grammar 也交付；它由总研究目标单列。

## TP-16 的一手依赖补核：两条 coercion 路径确实不同

本次为澄清源码中的 `Value.Convert`，只读核验 pi 锁定的 TypeBox **1.3.27** 官方 tag，commit `ffe23ba614322bf0dcb846ceaf7c1bbc7ef0bba6`，访问日期2026-10-03。未安装、导入或运行该依赖。pi 本地 manifest 指定1.3.27；`validation.ts:318–335` 先调用 Convert，再根据 Symbol.for("TypeBox.Kind") 检查决定 plain fallback，不能仅按这个旧式 symbol 检查推断 Convert 的全部行为。

官方 [FromType](https://github.com/sinclairzx81/typebox/blob/ffe23ba614322bf0dcb846ceaf7c1bbc7ef0bba6/src/value/convert/from_type.ts) 通过 Type.Is* 分派；[Object](https://github.com/sinclairzx81/typebox/blob/ffe23ba614322bf0dcb846ceaf7c1bbc7ef0bba6/src/type/types/object.ts) 与 [IsKind](https://github.com/sinclairzx81/typebox/blob/ffe23ba614322bf0dcb846ceaf7c1bbc7ef0bba6/src/type/types/schema.ts) 使用 `~kind`。因此纯 JSON schema 与原生带 TypeBox 元数据的 schema 不能未经验证就视为相同转换器。例子来自官方依赖源码，当前 pi 的纯 JSON 对照来自其自身测试：

| 输入/目标类型 | pi 纯 JSON fallback | TypeBox 原生转换的源码行为 |
|---|---|---|
| `"1"`/boolean | 不转，验证失败 | 转 true |
| `"TRUE"`/boolean | 不转，验证失败 | 转 true |
| null/string（required） | 空字符串 | `"null"` |
| `"null"`/null | 不转，验证失败 | 转 null |
| 空字符串/number（required） | 不转，验证失败 | numeric conversion 可变0 |

来源：[TryBoolean](https://github.com/sinclairzx81/typebox/blob/ffe23ba614322bf0dcb846ceaf7c1bbc7ef0bba6/src/value/convert/try/try_boolean.ts)、[TryString](https://github.com/sinclairzx81/typebox/blob/ffe23ba614322bf0dcb846ceaf7c1bbc7ef0bba6/src/value/convert/try/try_string.ts)、[TryNull](https://github.com/sinclairzx81/typebox/blob/ffe23ba614322bf0dcb846ceaf7c1bbc7ef0bba6/src/value/convert/try/try_null.ts)、[TryNumber](https://github.com/sinclairzx81/typebox/blob/ffe23ba614322bf0dcb846ceaf7c1bbc7ef0bba6/src/value/convert/try/try_number.ts)。这些是静态推导，未宣称差分执行通过。Rust 当前公共 schema 是 JSON Value，没有 TypeBox 的原生类型构造生态；推荐采用 pi 明确测试过的 serialized-schema 规则，并把上述差异如实登记，而非引入私有 marker 或猜测两条路径本应相同。

## 对旧候选 spec 的替换与保留

当前 opi `before_tool_call` 仅返回 Continue/Deny，在 schema validation 前观察参数 clone；没有修改参数的返回通道。采用 TP-08/15 需要扩展这个公开 hook，并改变其观察阶段，不能写成“已有 mutation 后重校验”的小调整。迁移需同步 hook 实现者、mock 和断言；若旧调用方依赖观察无效 raw proposal，保留可辨的只读观察位置或明确迁移到既有诊断/事件，不能偷偷把该观察行为删除。最终 mandatory authorization 仍检查真正 execution 值，User Policy 的可信来源与取消/拒绝行为保持。

| 旧要求 | 处理建议及退出验收 |
|---|---|
| AIR-008 全量 replacement、无sections | 用TP-01–04替换；保留原始状态/顺序，不把无结构full string当等价；旧 snapshot helpers 可作为checkpoint/builder，不能平行决定运行时状态。 |
| AIR-009 固定registry/schema、NotDeclared、新child | 用TP-06–08替换；保留可信来源、当前授权与无模型授权；退出整个run固定registry及wire可见性授权的无条件规则。 |
| AIR-010 provenance | 保留核心，与TP-11联动；因后续 thinking 来源补核需调整的范围由总研究维护，不以本文覆盖。 |
| AIR-011 默认Strict | 用TP-05替换，自动能力适配为候选默认；显式Strict是可选策略。 |
| AIR-012/013 missing/Length | 用TP-09–11替换；NotExecuted保留为真实事实类型，退出“它是所有合成恢复唯一来源”；旧普通failure marker仅作不能表示结构化pair的局部方案，不作为所有Length默认。 |
| AIR-014/015/018/019 session/compaction/reconstruction | 保留单一owner、branch/来源/unknown和原始事实；按TP-04重写有损checkpoint保证。退出对retained旧call每个原始上下文完全相同的隐含要求。 |
| AIR-016 legacy child / AIR-017接受与提交 | 保留源事实、unknown与已接受输入；child标存量迁移政策。复用现有commit ack，不因pi对齐新增双durable append；实际格式迁移细节仍由产品owner检验。 |
| AIR-020 Request入口 | 保留唯一normalize下游；恢复有清晰转换的公开初始便利输入，不留旧adapter分支。 |
| AIR-024 schema / no coercion | 用TP-08、12–16替换；保留验证与授权，加入精确转换corpus；可信hook超schema已由D-22确认，不得偷偷加二次模型schema校验。 |

未来测试影响预计 **add/update/retain**；只有旧断言专门固化被明确替代的行为时才 delete，并以同一公开 seam 的新行为测试替代。当前文档研究的实际测试影响为 **none**。

## 三项已收敛的实质取舍

1. **Compaction保证（D-28技术收敛）**：采用pi当前checkpoint+summary+retained非system历史+后续变化，保留可审查原始事实；撤回首轮由研究者加入、未独立提问的retained旧call逐边界完全保真要求。这落实D-18/19，不伪造另一人类回答。
2. **可信hook最终参数（D-22/Q25人类确认）**：模型输入先normalize/validate，trusted beforeToolCall可改execution而不复验模型schema；最终authorizer审实际执行值。raw/normalized/execution可辨，模型不能注册hook或授予权限。当前opi仅Continue/Deny，实施需扩展公开hook并同步调用方/测试。
3. **Rust唯一coercion语义（D-28技术收敛）**：当前Rust公共schema是JSON Value，采用pi已测试serialized plain-JSON路径，承认与TypeBox私有metadata路径的差异；不添加兼容模式。精确numeric和union corpus属于实施验证，不让实现Agent另选接受集。

自动适配、动态revision、active/visible分离与ResultUnavailable等方向已由D-19/20确认，不再留作开放选择。Q11 legacy child仍是保源迁移政策，不扩张到普通动态工具更新。schema/实现revision冻结不冻结User Policy，实时撤销仍生效。section enum、Arc与diff算法等由工程实现选择。

## 本次验证与限制

已执行：本地源码/测试断言静态读取；只读 `gh api` 固定 TypeBox tag、tree 与对应 source。一次 `from_integer.ts` 请求返回 EOF，本提案未据其内容作断言；没有重试安装或扩大运行时环境。pi 与 opi 的既有运行时测试本专题均未执行，差分 corpus 是设计而非已通过结果。

剩余技术核验包括跨 Rust JSON-number 表示的精确数字 corpus、可信hook在opi API中的最小接线、最终 partial/repair parser 与raw证据的接口、所有adapter真实第二请求fixture。它们不应隐蔽改变上列已选目标；若暴露新的语义矛盾再提出具体决定，不让 Agent 猜产品策略。

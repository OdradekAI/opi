# Transcript 与会话持久化迁移影响研究

> **非规范性研究，目标已按第二轮决定同步；不构成实施准入。** 2026-10-03。当前目标见 [D-19、D-20、D-22、D-28](decisions.md#第二轮目标决策d-18之后)；TM-01–13保留本次基线的源码事实，不能当作目标已实现。

本专题只涉及transcript、可信工具revision、会话迁移与压缩。使用中文，没有既有英文counterpart；候选spec双语由主文档维护。未运行本页运行时测试、未改代码或正式规范。pi快照与opi revision见本目录README。

## 1. 首轮历史与当前目标

首轮Q12/Q15/Q20曾选择默认Strict、完整无结构snapshot、整个run固定registry及声明外拒绝；这些**已由Q22/D-19替代**。Q16/修订Q17的Length统一普通文本及未知缺result拒绝**已由Q23/D-20替代**。Q25/D-22确认可信hook可在模型输入normalize/validate之后修改execution值，不复验模型schema，但最终authorizer仍审实际执行值。不得只摘取旧研究中的“已确认”。

当前组合是：named sections和工具add/remove/redefine；默认native-or-collapse并记录，Strict显式；可信宿主在调用边界原子更新revision，当前模型请求及其响应工具批次持有稳定revision；wire可见性与执行资格分离，User Policy可实时撤销。canonical输入/历史、raw参数、normalized参数和execution参数可辨。Q11仍保留v1/v2显式legacy child迁移，保护旧源与unknown；它不是普通工具更新的运行规则。

D-28是主Agent依据已确认方向作出的技术收敛，**不是新的用户回答**：压缩采用pi当前有效checkpoint+summary+retained非system历史+后续变化；Rust以plain-JSON schema转换为唯一规范化语义。首轮逐保留旧call完全保真验收撤回。正式旧规范仅作未来协调记录，不能否决上述目标或要求先设小范围例外。

## 2. 核心结论

1. 当前opi旧持久化只有system/tool摘要，不能恢复过去原文。unknown不可伪造为空或由当前配置倒填。
2. 完整transcript跨opi-ai、Agent与产品session；仅加Message variant不足以覆盖revision、prepare/commit、branch、compaction、fork/export与失败恢复。
3. 执行权来自可信宿主注册和当前User Policy。动态声明不是授权；可信宿主动态更新也不是模型自授权。冻结调用revision不会冻结或阻止后续权限撤销。
4. canonical transcript不是逐次HTTP字节档案；native/collapse、hidden declarations和产品恢复会产生request view。投影需可辨，不反写原始事实。
5. 新session v3和required条目用于表达历史变化、checkpoint、来源与unknown。当前writer仍为v2；设计目标不等于已完成格式升级。

## 3. 当前事实与目标责任图

```text
trusted product/embedder → candidate tool/system revision
                                  ↓ atomic prepare/commit
Agent call snapshot ──────────────┼──→ opi-ai normalize/project → selected adapter
  ├─ implementation/schema        │
  └─ raw/normalized/execution     └──→ single session owner → branch/checkpoint/export
              ↓
     live User Policy / final authorizer → actual execution value
```

revision是调用材料的身份，不是另一份工具registry或持久化账本。以下事实ID和定位继续作为基线证据；设计影响已同步当前目标。

| 编号 | 当前事实与证据 | 设计影响 |
|---|---|---|
| TM-01 | `opi-ai/src/message.rs:9` 的 Message 只有 User/Assistant/ToolResult；`opi-ai/src/provider.rs:102` 的 Request 分别持有 system、messages、tools。 | 现有 API 表达当前一次请求快照，不表达 system/tools 的时间位置；这不是仅靠序列化配置能补齐的能力。 |
| TM-02 | `opi-agent/src/loop_types.rs:308` 的 NextTurnState 只有 context/model_selection/inference；`:408` 的 LoopContext 另持不可变 ToolRegistry 与 system；`agent_loop.rs:463` 每 turn 从 context.system 与 registry.definitions 组装一次请求。 | 新的 system/section/tool 变化应进入 context 的有序语义，避免再增加一套互相同步的 NextTurnState.system/tools。registry 仍拥有执行权威。 |
| TM-03 | `hooks.rs:167` preparation 返回完整 replacement；`:211` 验证不得外部提交；`:224` commit 不因取消丢弃 future，已完成 durable commit 要安装 candidate；`agent_loop.rs:1399` 验证、selected input 后缀检查、外部提交串行。 | 可复用现有事务边界扩展 transcript 与 revision，保留停止先于准备、取消、队列 suffix 与 commit-ack 规则。不能另起独立持久化 hook。 |
| TM-04 | `agent_loop.rs:442` 在 preparation 后执行 transform_context，再 convert_to_llm；产物仅用于 request。`harness.rs:3049` 按成功 execution 提交新消息，`:3121` 处理未持久化 suffix。 | canonical transcript 与 effective request 目前存在合法分层。目标需定义 material projection 的身份与降级记录，不能声称保存 canonical 就能逐字重建全部 hook/wire 输出。 |
| TM-05 | `authority.rs:83` registration 在 trusted assembly 后不可变；`:129` registry 由可信注册构建；`agent_loop.rs:2237` 先查当前注册，`:2280` 使用其 schema 校验最终参数，再授权。 | 这是现有不可变装配事实；D-19允许可信宿主按call更新revision，声明本身仍不授权。既有无模型授权负向测试保留，D-22扩展可信hook执行值契约。 |
| TM-06 | `session.rs:47` writer version=2、legacy=1；`:59` header 可绑定 Runtime Input Binding；`:148` MessageEntry 只存 Message；`:157` CompactionEntry 只有 summary、kept pointer、token/usage；`:287` entry enum 无 system/tools 历史。 | 新格式需要原始 system/section/tool 变化、checkpoint 与显式历史已知性。不能声称旧 v2 是可精确回放的新 transcript。 |
| TM-07 | `harness.rs:228` 对工具 name/description/schema 求 digest，`:239` 对 system 求 digest；`:311` material_inputs 仍是 digests；`opi-agent/src/evidence.rs:510` binding 与 `:1631` InputIdentity 不保存原始内容。 | 对旧数据而言是信息缺失；扩展状态或 trace “可能曾记录”不是普遍可恢复保证。无完整原始值且来源不确定时应保留 unknown。 |
| TM-08 | `session.rs:624` 拒绝未知版本，`:633` v2 要 binding，`:683` required/ignorable 读取；`:657` 区分未完成尾行。 | 新 context entry 必须 required；继承 fail-closed，不用 permissive serde defaults 把缺失 snapshot 变成空 system/无工具。 |
| TM-09 | `session_branch.rs:60` 只有 Message/Compaction 是 content nodes；`session_context.rs:184` 沿 active chain 重建；`:226` 元数据按 active branch/file order 归并。 | system/section/tool 变化是有因果位置的内容，不应当作附着旧 tip 的 latest-wins metadata。放入 Message variant 可复用内容链，但所有 match 仍需审计。 |
| TM-10 | `session_context.rs:350` compaction 丢弃 first_kept 之前内容，插 summary；没有 system/tools checkpoint；`session_coordinator.rs:732` 原子提交 compaction marker 与 Leaf。 | 压缩需要保存提交时当前完整有效 checkpoint，并按pi政策去掉已纳入checkpoint的压缩前system变化。压缩摘要不能代替 schema、prompt 或 provenance。 |
| TM-11 | `session_cli.rs:208` fork 不改源；`:272` 按 active chain 复制条目；`:334` entry_on_active_chain 的未来 variant fallback=false；`:475` export 选 active/full-tree；`:500` full-tree 是消息文件顺序。 | 新类型可能被 fork 静默丢弃；full-tree 文件序列不是单条可执行 transcript。机器导出必须表明 branch/tip 和缺口，不能把 siblings 接成一段上下文。 |
| TM-12 | `harness.rs:2410` 旧 model spec 唯一解析后，`:2446` 用当前 trusted assembly 生成 binding 并 fork child；`:3295` 当前 binding 与 session binding 不同会先 fork child。 | Q11保留该机制用于legacy迁移；D-19不将普通可信schema/实现更新变成child。该机制不能恢复历史system/tools，也不能用当前配置补造旧事实。 |
| TM-13 | `compaction.rs:479` ProductCompactionHooks 已拥有 preparation commit；`:546` 拒绝另一个 owner 重叠提交；`:605` bridge 驱动原子持久化并确认；`:640` 返回 ExternalStateCommitted。 | transcript/revision准备与compaction复用一个product transaction owner。不要另起第二个持久化owner或自动要求每call双durable append。 |

本表源码路径均相对 `crates/`，除明确以 `docs/` 开头者。精确内容以本次工作树为准；以上是静态事实，不是运行验证结果。

## 4. pi 契约与有效边界

| 一手证据（相对PI包根） | 当前目标与限制 |
|---|---|
| `ai/src/types.ts:512–537`、`ai/src/utils/transcript.ts:58–166` | 保留content追加、named section增删/替换和有序tool delta。snapshot可用作checkpoint/便捷初始值，不能替代变化身份而损失native更新语义。 |
| `ai/src/utils/transcript.ts:104–119,220–232`；官方PR9548 | 默认能力驱动native-or-collapse，不能称collapse无损；Strict显式选择。 |
| `agent/src/agent-loop.ts:171–215,323–362`；`coding-agent/src/core/agent-session.ts:3499–3650` | 可信宿主可在会话内更新工具；Rust持有稳定的每call revision，不因schema/实现变化强制child。 |
| `coding-agent/src/core/agent-session.ts:1511–1571,1715–1737` | registered、active/callable、wire-visible与authorized不同；hidden declaration不自动撤销合法上下文的执行资格。 |
| `coding-agent/src/core/session-manager.ts:461–510,1261–1285` | 压缩捕获当前完整state；summary后保留非system历史，再接后续变化。这是有损上下文投影，不保留retained旧调用当时的每个system边界。 |
| `coding-agent/src/core/session-manager.ts:1160–1213` | 从首个user开始落盘，使模型首轮失败后输入仍在；不证明pi拥有opi多条事务的相同失败原子性。 |

详细反证与历史见[RT01–08](reassessment-transcript-notes.md)，目标细化见[TP-01–17](transcript-target-proposal.md)。

## 5. 旧规范的未来协调记录

本表是需要同步旧文本的导航，不是本轮目标的否决或新增准入条件。本次不修改这些文件。

| 旧条款 | 需要协调的内容 |
|---|---|
| `docs/opi-spec.md:387` §7.2、INV-004 `:402` | 下一调用原子材料包含transcript/revision；保留stop-before-prepare、队列suffix及commit ack，允许可信动态更新。 |
| INV-005 `:413` | 保留非模型来源的registration/User Policy；说明模型输入校验与可信hook修改execution的责任区别，最终authorizer审实际值；不再用固定整个run schema否决D-19/22。 |
| §7.4、INV-007/008；`docs/CONTEXT.md:76` | 原始prefix/来源事实保持，不改写旧header；每call revision与projection共同识别实际材料。固定初始digest不能冒充后续全部材料，普通动态更新也不要求新child。 |
| required/ignorable规则 | system/tool变化、checkpoint与unknown是因果内容；旧reader不得静默忽略。迁移兼容和新运行时输入分开。 |
| 旧placement/第二adapter条款 | 仅记录现有JSONL owner可承接本次迁移；不以旧门槛否决pi行为，也不预建另一存储trait/crate。 |

首轮Q19“已接受输入在后续失败后保留”的核心继续成立；完整双durable append是首轮候选的额外工程设计，不是当前opi事实或pi必要条件。

## 6. 当前职责与调用边界

### 6.1 Transcript与可信revision

opi-ai负责normalize、system/section/tool状态求值及adapter投影；初始system/tools builder可以保留，经一次normalize后下游只有transcript权威。Agent负责在调用边界安装可信system/tool revision。declared schema/description/sampling与工具次序有历史位置；execute不从历史反序列化注册。

可信宿主可改变schema、实现和来源；已发起模型请求及其响应工具批次保留自己的绑定，新revision只影响后继call。失败不留下半更新。权限策略实时检查，旧revision不构成已撤销权限的豁免。模型直接调用需属于其执行上下文，可信nested调用使用其合法callable集合；wire-hidden不是单独的执行拒绝理由。

参数处理采用D-20/22/28：raw保留；prepare/optional-null/plain-JSON normalization后模型输入通过原可信schema校验；可信beforeToolCall可修改execution而不复验该schema；最终authorizer检查和绑定真实execution值。模型不能选择hook、伪造revision/来源或改变已授权值后继续用旧授权。当前opi hook仅Continue/Deny且在校验前观察clone；实现目标要同步公开hook及观察阶段，不能声称已有mutation通道。

### 6.2 接受、投影与提交

1. 保留已有停止判定与selected input suffix规则，trusted preparation形成完整候选材料。
2. 校验结构/revision/模型输入；默认按能力native-or-collapse，显式Strict不支持时提前失败。投影仅影响request view。
3. 一个产品持久化owner承接context/revision、必要checkpoint与leaf；与compaction合并事务，不增加第二权威。commit前失败保留前态和未交付输入。
4. durable ack后安装候选；后续auth/provider失败或cancel保留已接受baseline，不伪造成功dispatch。保留现有不可取消ack和partial-effect诊断。
5. 低层prepared call及其attempt复用同一材料/auth/投影。已确认的产品高层恢复可另建关联call，遵D-24，不把它与同流隐式重播混为一谈。

原始prefix、当前revision、投影模式与真实执行参数须可关联；不要求把凭据、完整HTTP字节或所有诊断原文落盘。材料在哪一步具备何种身份，应由单一owner完成，不靠重复写入“保证可追踪”。

## 7. v3迁移、分支、压缩与导出

### 7.1 Legacy迁移

Q11保留：v1/v2原文件及trace不改，显式创建legacy child，记录来源session/branch/tip/binding，复制有效active历史并标记system/tools未知区间；在continuation处追加当前可信初始state。当前值不回填过去。exact历史重建触及unknown失败；普通继续允许默认适配并记录投影，显式Strict仍遵自身保真要求。接受legacy缺口不是宣称过去已知。

v3 required条目保持unknown与known-empty区别；未知required/version在副作用前报错，中断尾行与损坏区分。旧reader/migration保留，旧格式不作为永久第二运行时。**普通可信工具/schema/source变更只更新revision，不走legacy child流程。**

### 7.2 压缩与分支

有序system/tool变化是推进tip的因果内容，sibling不互相继承。D-28压缩规则为：压缩提交时的完整有效checkpoint → summary → retained非system历史 → compaction之后的新变化；被checkpoint吸收的旧system delta不重复应用。压缩前原始条目可审查，但给模型的重建是有损的。

验收比较压缩时当前state与新checkpoint、后续变化、分支隔离及原始事实保留；**撤回“每个retained旧call的原system/tool边界完全等价”**。unknown不因摘要变known；新的可信continuation baseline不抹掉前区间缺口。fork选定tip只能复制该tip可达因果条目，不能包含未来revision或因variant fallback丢新类型。

### 7.3 导出

active export与resume/fork复用active-chain重建；full-tree保留branch/parent/tip关系，不能把siblings串成可执行历史。现有JSON/Markdown是脱敏展示，不是可逆HTTP或签名归档。保留system/schema/raw参数脱敏和opaque/signature隐藏；若未来机器可重放导出成为目标，再明确格式与导入契约，不悄悄升级展示输出。

## 8. 旧运行时路径的退出

| 项目 | 替代、迁移与完成条件 |
|---|---|
| Request.system/tools独立下游权威 | Agent、adapter、capability检查、mock、compaction统一消费normalize后的transcript；保留公开初始builder/shorthand，明确prepend次序；不留第二adapter解释路径。 |
| 无结构full-snapshot-only目标 | 由named sections/tool delta表达变化，full state只用于初始便利输入/checkpoint；序列化、session、fork/export与fixtures同步。 |
| run-fixed registry/每次更新child | trusted revision按call原子更新，旧call/batch稳定、实时权限可撤销；legacy child仍存在但普通动态更新不触发。 |
| wire-visible即授权上界 | 当前执行上下文/可信来源/实时authorizer决定资格；hidden可执行反例有真实fixture，unknown工具仍零执行。 |
| 无coercion及Continue/Deny-only hook | plain-JSON normalize/validate，扩展trusted mutation，raw/normalized/execution可辨，最终authorizer值与执行值相同。旧raw观察行为需明确迁移，不静默丢失。 |
| 旧compaction无checkpoint | checkpoint承接有效state，压缩旧system delta退出request view但原始条目保留；未来variation不因现有match遗漏。 |

退出证明包括静态旧路径检索、消费者编译、真实adapter fixture与session重建测试。旧数据reader、历史证据里的旧符号和便利builder不应被错误计作未退出runtime权威。

## 9. 离线验收与既有测试边界

以下均未执行，ID保留以便更新既有追踪关系；具体最终AIR/V编号由主spec统一。

| 验证ID | 当前目标断言 | 现有边界/变化 |
|---|---|---|
| TM-V01 | content追加、section替换/删除/重加顺序、tool增删重定义、known-empty/unknown、checkpoint；重复/坏schema拒绝 | 新增opi-ai纯transcript fixtures；保留serde与有序消息语义 |
| TM-V02 | 默认native或collapse且canonical不变；显式Strict提前失败；单初始state不误报loss | provider_collection及adapter伪传输；替换默认Strict旧断言 |
| TM-V03 | context/model/inference/revision原子安装；stop/queue suffix/cancel/ack保持 | Agent next-turn/hooks；替换fixed-registry假设 |
| TM-V04 | 模型声明不注册；可信动态schema可更新；hidden与active分离；normalized模型值先校验，hook后execution可超schema但authorizer审实际值 | authority/tool-validation，新增旧callrevision及live policy revocation |
| TM-V05 | legacy源bytes不变、child来源及unknown、continuation处初始state、exact拒绝unknown；普通变化不child | phase17_legacy_migration与migration fixtures |
| TM-V06 | sibling隔离、fork旧tip无future、active resume/fork/export一致、required与中断尾行区分 | session_contract/context/branching |
| TM-V07 | pi式checkpoint+summary+retained非system+后续更新；多次压缩、重加、unknown保留；原始历史仍可检查 | session_context、coordinator；撤回逐retained旧call完全保真断言 |
| TM-V08 | commit/leaf失败原子性；ack后provider/auth失败或取消仍保留accepted baseline；一个事务owner | phase17_failure_rollback、ProductCompactionHooks；改掉失败一律删除输入的旧预期 |
| TM-V09 | 每call prefix/revision/projection可关联，同prepared attempts相同；新call新revision；旧call稳定但User Policy实时撤销 | evidence/binding多call fixtures；不新增普通更新child断言 |
| TM-V10 | export脱敏、变化/unknown可见、full-tree不伪装单支、signature/opaque隐藏 | session_export现有边界 |
| TM-V11 | 原单次聊天语义保持；便利输入唯一normalize；旧adapter双权威退出 | 直接SDK、mock、compaction、examples编译和payload fixtures |

本次测试影响 **none**；未来实施为add/update/retain，delete只针对已替代行为的旧断言，并保持必要公共行为覆盖。

## 10. 限制与交接

目标选择已由D-19/20/22和D-28收敛，不将首轮Strict/fixed-registry/Length规则留给实施Agent选择。Q11仅作为存量迁移选择继续有效。源码未变化；这里没有声称format v3、动态revision或coercion已实现。

未执行transcript prototype/现有测试，未验证所有provider远端中途更新接受度和外部embedder规模。最终numeric corpus、raw/repair parser seam与adapter continuation需按目标离线核验；若新证据揭示语义冲突再明确处理，不能以测试未跑改写已确认方向。

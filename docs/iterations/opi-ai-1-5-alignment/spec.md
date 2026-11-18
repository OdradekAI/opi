# opi-ai 对齐 Pi 1.5.0：重构方案与实施交接
Status: active
Owner: user
Baseline: opi 90f953d5ee288c165dc295bfdfb7cb7a05d7cecd; initial worktree clean; upstream D:/Luiz/Odradek/opi/.pi/pi-1.5.0; SHA256 content manifest 959b3b486f111b53771d37da272c8951064952bd9b18596c5c244935cea45072

日期：2026-11-07。本文件承接已完成的[决策地图](map.md)，作为实施的单一方案入口。active 表示已确认的目标范围仍有效。第一阶段已完成并提交，证据见[第一阶段报告](phase1.md)。用户随后授权继续推进；第二阶段首个切片[建立中立遥测基础与可选 tracing 适配](issues/02-neutral-telemetry-foundation.md)已完成，结果及全量回归限制见[切片报告](phase2.md)。随后完成第二个切片[分离纯值、公共错误与认证目录契约](issues/03-values-errors-auth-catalog.md)，结果见[基础边界报告](phase2-foundation.md)。两票完成第二阶段基础整理。第三阶段已完成首个切片[显式启动 chat 请求并分离执行与消费](issues/04-started-chat-execution.md)，见[交付与剩余工作](phase3-started.md)；随后完成[迁移 agent 请求消费与显式中止](issues/05-agent-execution-consumers.md)，状态见[消费者迁移报告](phase3-consumers.md)；接着完成[让普通 chat 入口独立执行](issues/06-default-chat-execution.md)，交付、检查与边界见[默认入口迁移报告](phase3-default.md)；随后完成限定范围的[AI telemetry seam 与 source/result 尾部](issues/07-ai-telemetry-source-tail.md)，见[切片报告](phase3-ai-telemetry.md)；当前又完成第四阶段首个[可选 telemetry tracing feature](issues/08-optional-telemetry-feature.md)，见[feature 报告](phase4-features.md)，以及[普通 Models chat 自动安装 AI observer](issues/09-ordinary-chat-ai-telemetry.md)，见[切片报告](phase4-ordinary-ai-telemetry.md)。普通 Models chat 已自动安装 observer；随后 deferred fetch 的 `ChatStream` 与
`pi.ai.poll` 自动观测、prepared attempt 的 observed helper，以及供独立 provider
使用的公开 `ChatStream::new_with_telemetry` 均已落地；其 `ChatStream::start()` 正常
后台终态也会完成同一 observer。无观测的 raw ChatStream、
prepared raw attempt、非 chat 其他路径仍开放；prepared observed helper 的跨 attempt
聚合已明确为调用方责任，不自动合并；完整 provider
覆盖、feature 矩阵与 Pi 差分仍开放。各票独立维护检查范围，不代表完整能力对齐
通过。答案细节以各决策票的 Answer 为准，研究报告只提供证据。

最新的普通 chat telemetry 切片又把 `pi.ai.request` observer 的安装边界对齐到
`ChatCall` 构造：显式 context 在 setup/authentication 开始前即可看到未结束的
span，setup failure 使用规范 `CallFailure.message()` 结算；后台执行仍保持异步，
不会把构造时点误写成同步 provider 调用。

随后完成首个验收材料冻结与 TS/Rust 生命周期差分切片：pinned model catalog
响应已按 revision 校验并保存原始 bytes，Pi 源、lockfile、Cargo、工具链与平台身份
均有哈希证据。首次记录时上游 `node_modules` 与 `packages/ai/dist` 缺失；初版
三例只比较有限共享字段，不能证明 source 尾部结算。历史记录见
[材料与差分报告](phase5-materials-and-differential.md)及
[初版差分证据](evidence/phase5-lifecycle-differential.json)。

用户补充依赖、dist 和生成目录后，已完成[材料复核与有屏障的差分切片](issues/36-supplemented-materials-and-gated-differential.md)。
十个 AI 直接依赖匹配冻结声明/lock 摘录，dist 内容与 source-map/output 完整性已记录；
但完整 lockfile 的身份发生漂移，生成目录虽内部有效却与 pinned catalog 的九个
provider 不同（1712 vs 1738 项），这两个阻断没有清除。新增终态放行屏障和真实
result/EOF/Drop 观察后，普通 provider 的不消费、停止消费两例均暴露 source 尾部
未执行的 **FAIL**；setup failure 仍是未获批准的差异。该切片只补强工具，未修改生产
生命周期；历史失败与材料阻断见[补充报告](phase5-supplemented-materials-and-gated-differential.md)。

用户随后批准修复普通 source 尾部截断，[修复票](issues/37-ordinary-source-tail-settlement.md)
已完成。普通调用 wrapper 在首终态后只排空当前 attempt，不重新进入 retry/admission；
公共入口红绿回归覆盖终态/result/遥测等待、尾部事件吸收、显式 abort、协作取消和
消费 Drop，以及获准 retry 后的尾部结算，209 项相关 Rust 测试及 2 项实际 driver 测试通过。新录制的三例共享观察
全部通过，但整体仍是 `BLOCKED_WITH_RECORDED_OBSERVATIONS` / exit 2：两个材料
身份阻断未清除，启动时点与 setup 错误通道未获差异批准。旧 FAIL 证据原样保留，
该修复的状态见[普通调用尾部修复报告](phase5-ordinary-source-tail.md)，不代表完整 TS/Rust
差分或六门验收通过。

随后推进[隔离副本中的 pinned catalog 恢复](issues/38-isolated-pinned-catalog-restoration.md)：
使用已验证的原始响应和上游离线 hydrator，复制冻结清单中的源码，重建 telemetry、
schema、AI，并实际加载编译后的 catalog。候选的 42 个 provider / 1712 个模型、
source/dist 数据与运行时导出均匹配 pin；用户原树保持不变。该候选仍沿用发生漂移的
完整 lock，未证明所有传递依赖来源或 npm 包消费，当时也未接入生命周期 runner 的
候选材料路径。结果和验证边界见[恢复报告](phase5-isolated-catalog-restoration.md)。

随后修正 AI telemetry `provider_error` 状态消息的长度单位：错误文本现在按
UTF-16 code unit 限制为 500，非 BMP 文本不会因 Rust `char` 计数而超出 Pi 的
边界；孤立代理对无法由 Rust `String` 表示，已作为明确的 Rust-native 差异单独
记录。随后补齐 `StopReason::Pending`，将各 provider 的初始 partial 从 `Stop`
改为非终态 `Pending`；AI telemetry 对 pending/deferred poll 均不累计
token/cost，并将 `toolUse` 与完整 stop vocabulary 对齐。canonical Rust
`StopReason::as_str()` 的 `tool_use` 仍保留，telemetry 通过独立映射使用 Pi
词汇。随后修正可选的 `response.id`：provider 没有报告 response id 时保持属性缺失，
不把它改写为空字符串。request duration metric 也改为只携带 Pi 定义的
`provider`、`model`、`stop`，不复用 span 的高基数和 usage 结束属性。

当前 pricing/cost 切片已完成：`Usage.cost` 区分 catalog 分量、catalog total
和 provider-reported total；tier 选择、cache-write 1h 子集以及 OpenRouter
非 BYOK/BYOK 账单规则按 Pi 1.5.0 固定。通用 OpenAI/Anthropic 解析器保留
OpenRouter 扩展字段但不应用它们，只有对应 provider route 才进行账单覆盖。
专项测试、两个 provider fixture 集合和 `model_wire_metadata` 通过；Radius OAuth
固定回调端口的并行竞争已修复，platform delivery feature 契约也已更新，schema-engine
的 sibling resource identity 回归也已收敛。全量 `opi-ai` 包测试通过。详情见
[pricing/cost 切片报告](phase4-cost-pricing-semantics.md)
和[检查证据](evidence/phase4-cost-pricing-semantics-checks.json)。

随后完成独立消费者补充验收：实际 `opi-ai` `.crate` 在 workspace 外部消费者中
分别以 `default-features = false` 和显式 `telemetry-tracing` 编译并执行 README
三条 onboarding 流程；metadata、controlled-peer 回归、执行前启动、最终结果和
tool-result continuation 均通过。证据见[独立消费者报告](phase4-readme-package-profiles.md)
和 `scripts/pi-sdk-conformance/evidence/readme-examples-proof.json`。
随后补上双语 README 的任务导航，将独立接入路径与后续 provider/协议参考分开；
实施记录见[README 导航票](issues/27-readme-task-guide.md)。
随后补上公共 API 总览，固定中立入口与 Provider/Telemetry 的责任边界；实施记录见
[README API 总览票](issues/28-readme-public-api-map.md)。
随后补上双语高级能力参考索引，将 Provider、认证、Telemetry、cost/pricing、
WebSocket、Deferred 与测试包证明集中成入口；实施记录见
[README 参考索引票](issues/29-readme-reference-map.md)。
随后补上双语 feature profile 矩阵，固定 `telemetry-tracing` 与
`default-features = false` 的真实语义；实施记录见
[README feature profile 票](issues/30-readme-feature-profile-matrix.md)。
随后同步 `opi-telemetry` 独立 README 的 AI instrumentation 边界，修正过时的
future-migration 描述；实施记录见
[telemetry README 边界票](issues/31-telemetry-readme-ai-boundary.md)。
随后把长篇 cost/pricing 规则拆成 crate 内双语专题文档，主 README 仅保留稳定摘要、
旧标题锚点和包内链接；实施记录见
[README cost/pricing 专题票](issues/32-readme-cost-pricing-topic.md)。
随后把 Provider、Catalog、认证与 Refresh 参考拆成 crate 内双语专题文档，主 README
保留稳定锚点和 Core API 总览；实施记录见
[README Provider/Auth 专题票](issues/33-readme-provider-auth-topic.md)。
随后清理公开手册中的历史任务、阶段和验收快照标签，使 README 与包内专题回到面向
能力和行为的开发者手册；内部 issue、report、evidence 继续保留完整追溯。实施记录见
[公开手册历史标签清理票](issues/34-remove-internal-labels-from-public-manuals.md)。
随后按 `decisions/09-readme-contract.md` 的任务顺序重构双语主 README，并把传输、
公共工具、Provider wire/operation 和测试/包消费细节拆为四对包内专题；三份真实
onboarding 示例继续与中英文 README 逐字绑定。实施记录见
[README 公共手册重构票](issues/35-readme-public-manual-restructure.md)。
该切片的 105 个 conformance tests、文档/格式检查、实际包文件清单和双 feature profile
外部消费者证明均通过；证据见
`scripts/pi-sdk-conformance/evidence/readme-examples-proof.json`。这只证明文档与
独立包接入契约，不代表完整 Pi 或 pi-coding-agent 对齐完成。

2026-11-11 完成[绑定隔离候选并重放生命周期差分](issues/39-bind-isolated-candidate-lifecycle-replay.md)。
材料冻结与重放都使用 #38 的审阅后候选。runner 从同一份清单取得候选路径，在重放
前后复核清单哈希与材料身份。三例的共享观察均通过，指定目录不一致已从该候选的
阻断项中移除。整体仍为 `BLOCKED_WITH_RECORDED_OBSERVATIONS` / exit 2，候选根
lock 与冻结字节不同，且目录链接布局缺少自己的 npm 安装锁。构造时 setup 启动时点
和 setup 错误通道两项差异仍待处理。

原始 `package-lock.json` 已从相邻 Pi 仓库 `v1.5.0` 标签的 Git blob 找回，大小
185524 bytes 和完整 SHA-256 均匹配冻结记录。本次重放使用 #38 当时的实际安装
材料。原锁已可追溯，候选的传递依赖来源仍须验证。

2026-11-11 完成[用原锁重建并重放隔离候选](issues/40-rebuild-frozen-lock-candidate.md)。
在新的 Windows x64 副本中使用原锁离线 `npm ci`，根锁与全部 2949 项冻结源码
输入匹配，npm 生成了该副本自己的安装锁。实际核对 269 个 registry 包和 16 个
workspace 链接，重建的四棵生成树与 #38 字节一致，运行时目录仍匹配 pin。
新清单的 `blocked_gates` 为空，绑定它的三例重放共享观察均通过，整体为
`DIFFERENCES_REQUIRE_DECISION` / exit 0。setup 启动时点和错误通道两项差异仍待
决策，实际 npm 包消费、其他平台和完整六门验收仍开放。

2026-11-11 完成[公开 Models setup 同故障点差分](issues/41-match-public-models-setup-failures.md)。
在同一原锁候选上，以缺失 provider 和有屏障的认证失败分别对照公开入口。两侧均
在派发前交付一次错误、EOF 和同一最终失败。显式 key 路径上，Pi 在构造返回前
已进入 resolver，Rust 当前线程探针在后台进入 resolver，两侧都无需先消费。
Pi 的 `result()` 兑现为带错误的消息，Rust 返回 typed `CallFailure`，且该
setup 错误消息的 provider/model 为空。四例共享观察均通过，材料阻断为空，
整体仍为 `DIFFERENCES_REQUIRE_DECISION`。原始 `lazyStream` 同步 throw 的证据
保留为单独入口，公开入口的调度、错误通道与消息身份差异待处理。

2026-11-11 完成[准备阶段错误消息保留请求身份](issues/42-preserve-chat-setup-failure-identity.md)。
普通 Rust chat 调用在准备阶段合成失败消息时，从冻结的 `ChatModelRecord` 填入
provider/model。cause 已附带的 partial 优先。使用同一份原锁候选重放四例，两种
公开准备失败的错误消息身份均与 Pi 一致，四例共享观察通过，材料阻断为空。整体
仍为 `DIFFERENCES_REQUIRE_DECISION`。

2026-11-11 完成[公开 Models 源流完成差分](issues/43-match-public-models-completion.md)。
不消费、首事件后停止消费两例现从两侧公开 chat 入口测量同一个 provider 派发点。
调用返回时均为零次，消费前到达源流屏障时均为一次。放行后均观测到单一终态、
源流 EOF 和公开 `result()`。两例通过，准备失败两例复现 #42 的观测并继续标为差异。
同一原锁候选的四例共享观察全部通过，材料阻断为空。该次重放的原始判定为
`DIFFERENCES_REQUIRE_DECISION`。

随后确认保留 Rust 的后台调度和类型化准备失败。显式 key 用例的 resolver 进入
时点、缺失 provider 与受控认证失败两例的错误通道，均按具体观察登记为有意差异。
本组两例源流完成通过，两例准备失败的共享断言通过，差异按已确认范围验收。
[调度决策](decisions/11-telemetry-lifecycle.md#普通-models-的-resolver-进入时点) · [错误通道决策](decisions/04-public-contract.md#公开-models-的准备失败错误通道)

2026-11-11 完成[公开 Models 在源流 EOF 前的结果差分](issues/44-observe-public-result-before-source-eof.md)。
两侧在源流产出成功终态前均观测到 `result()` 待定。源流产出终态后，EOF
仍被屏障扣留时，Pi 的公开结果已兑现，Rust 的结果 future 仍为 `Pending`。
放行后均到达源流 EOF，最终结果的 `stop` 和 provider/model 一致。同一原锁
候选五例共享观察通过，材料阻断为空，原始判定为 `DIFFERENCES_REQUIRE_DECISION`。

随后确认在该普通成功用例中保留 Rust 等待源流 EOF 的语义。调用方收到成功结果时，
本次调用持有的源流已读完并释放。五例中两例通过，三例的共享断言通过且具名差异
已确认，原始轨迹的 `strict_alignment` 为 `false`。
[结果时序决策](decisions/11-telemetry-lifecycle.md#普通-models-的成功结果与源流-eof)

2026-11-11 完成[公开 Models 终态交付与请求 span 差分](issues/45-observe-public-terminal-and-span-before-source-eof.md)。
两侧读取公开 `Start` 后，保持同一次下一项读取。源流产出一个 `Done` 而 EOF 仍被屏障
扣留时，Pi 已交付公开 `Done`，Rust 下一项待定，两侧各有一个开放的 `pi.ai.request`
span。放行并观测到源流 EOF 后，两侧均已取得单一公开终态、外层 EOF 和与终态一致的
结果，并记录到一个以 `stop` 结算的请求 span。同一原锁候选六例共享断言通过，材料阻断
为空。

随后确认在该有限尾部的成功用例中，Rust 保留源流到达 EOF 并释放后再交付公开 `Done`。
该交付时点登记为有意差异。六例中两例通过，四例共享断言通过且具名差异已确认，冻结
重放的 `strict_alignment` 为 `false`。
[终态交付决策](decisions/11-telemetry-lifecycle.md#普通-models-的公开终态与源流-eof)

2026-11-11 完成[公开 Models 结果属性写入故障差分](issues/46-observe-public-result-attribute-write-failure.md)。
两侧在源流 EOF 后各遇到一次结果遥测属性写入故障，仍交付单一成功 `Done`、外层 EOF 和
与终态一致的 `result()`。两个内存请求 span 均为开放状态。Pi 的有界检查点观测到零次
`unhandledRejection`，Rust 捕获一次带故障标记的 tracing WARN。Rust 已释放 span 后端
包装，后端 `finish` 调用为零。同一原锁候选七例共享断言通过，材料阻断为空。

2026-11-11 完成[成功 chat 请求属性写入失败后的 span 收尾](issues/47-finish-request-span-after-result-attribute-write-failure.md)。
Rust 自动观察器保留写入诊断，尝试显式结束 span 一次。重放中 Rust 内存 span 已结算，
`stop` 属性缺失。Pi span 在有界检查点仍开放，两侧都交付成功终态和匹配的结果，`stop`
及 provider/model 一致。后端结束也失败的 Rust 回归保留两条独立警告和业务成功。
同一原锁候选的七例共享断言全部通过，其中五例已登记具名差异。材料阻断为空，冻结重放
的 `strict_alignment` 为 `false`。
[写入故障决策](decisions/11-telemetry-lifecycle.md#成功-chat-请求的最终遥测属性写入故障)

2026-11-11 完成[公开 Models 首个非终态事件写入故障差分](issues/48-observe-first-chunk-event-write-failure.md)。
两侧在 `Start` 上各遇到一次 `pi.ai.first_chunk` 写入失败。Pi 提前结束受控源迭代，交付
公开 `error` 及匹配的错误消息。Rust 发出 WARN、结束请求 span，并读完源流后交付成功
`Done` 和匹配的结果。两侧的 span 都在源流业务终态屏障内结算，Pi 写入 `stop: error`，
Rust 的 `stop` 属性缺失。同一原锁候选八例共享断言通过，材料阻断为空。

随后确认保留 Rust 对这一次首事件遥测写入故障的隔离，将业务结果、源流退出、span 属性
及宿主诊断登记为具名有意差异。八例中两例通过，六例共享断言通过且具名差异已确认。
原始重放判定为 `DIFFERENCES_REQUIRE_DECISION`，`strict_alignment` 为 `false`。
[首事件写入故障决策](decisions/11-telemetry-lifecycle.md#首个非终态事件的遥测写入故障)

2026-11-11 完成[公开 Models 请求时长直方图创建故障差分](issues/49-observe-request-duration-histogram-creation-failure.md)。
Pi 在创建 `pi.ai.request.duration` 时同步抛错，检查点内没有 provider 派发或请求 span。
Rust 发出 WARN、返回调用句柄并在之后派发 provider，源流 EOF 和 Drop 后取得成功
`Done` 与匹配结果。Rust 在直方图创建失败时已有请求 span，包装在调用返回前 Drop，
业务完成后后端 `finish` 仍为零，span 保持开放。同一原锁候选九例共享断言通过，
材料阻断为空。该次重放作为 #50 修复前的基线，`strict_alignment` 为 `false`。

2026-11-11 完成[先创建 AI 指标再启动请求 span](issues/50-create-ai-instruments-before-starting-span.md)。
Rust 在打开 `pi.ai.request` 或 `pi.ai.poll` 前创建四个指标。#49 用例重放中，故障、
公开调用边界和最终检查点两侧均为零个请求 span。Rust 报告一次 WARN，派发一次并交付
成功结果。Pi 继续在派发前同步抛出原始异常。新增手动 poll 回归验证最后一个指标失败时
返回原始错误并留下零 span，重试成功后结算一个 poll span。九例共享断言通过，旧八例
和全部 Pi 轨迹与 #49 一致。

随后确认保留 Rust 对这一次直方图创建故障的隔离，将公开调用边界、后续派发、业务结果和
宿主诊断登记为具名有意差异。九例中两例通过，七例共享断言通过且具名差异已确认。
原始重放判定为 `DIFFERENCES_REQUIRE_DECISION`，`strict_alignment` 为 `false`。
[#50 初始化故障决策](issues/50-create-ai-instruments-before-starting-span.md#accepted-decision)

2026-11-11 完成[公开 Models 请求 span 启动故障差分](issues/51-observe-request-span-start-failure.md)。
两侧在四个指标创建后各遇到一次 `pi.ai.request` 启动故障，均返回句柄、派发一次，
且各检查点均为零个请求 span。Pi 在转发 `Start` 后由观察器抛错，提前关闭受控源迭代，
交付 `error` 和匹配结果。宿主观测到启动 Promise 与结果观察器的两次未处理拒绝。
Rust 发出一次 WARN，源流到达 EOF 并释放后交付成功终态和匹配结果。十例共享断言通过，
旧九例与 #50 一致，材料阻断为空。

随后确认保留 Rust 对这一次请求 span 启动故障的隔离，将业务结果、源流退出及宿主诊断
登记为具名有意差异。十例中两例通过，八例共享断言通过且具名差异已确认。原始重放判定为
`DIFFERENCES_REQUIRE_DECISION`，`strict_alignment` 为 `false`。
[#51 请求 span 启动故障决策](issues/51-observe-request-span-start-failure.md#accepted-decision)

2026-11-11 完成[公开 Models 首事件为 `Done` 时的首块时长记录故障差分](issues/52-observe-first-chunk-duration-record-failure.md)。
两侧先写入 `pi.ai.first_chunk`，随后各遇到一次 `pi.ai.first_chunk.duration` 记录失败。
Pi 在观察器抛错前把 `Done` 推入公开流并锁定成功结果，随后执行源迭代器 `return()`，请求 span
和请求时长记为 `stop: error`。Rust 发出一次 WARN，提前结束 span，保留首块事件，
`stop` 缺失。源流到达自然 EOF 并释放后，Rust 交付 `Done` 和匹配的成功结果。
同一原锁候选十一例共享断言通过，旧十例一致，材料阻断为空。

随后确认保留 Rust 对这一次首块时长记录故障的隔离，将终态交付时点、源流退出、遥测和
宿主诊断登记为具名有意差异。十一例中两例通过，九例共享断言通过且具名差异已确认。
原始重放判定为 `DIFFERENCES_REQUIRE_DECISION`，`strict_alignment` 为 `false`。
[#52 首块时长记录故障决策](issues/52-observe-first-chunk-duration-record-failure.md#accepted-decision)

2026-11-11 完成[公开 Models 请求时长记录故障差分](issues/53-observe-request-duration-record-failure.md)。
两侧在源流自然 EOF 且写入最终 `stop` 和 `usage.input` 后，各遇到一次 `pi.ai.request.duration`
记录失败。调用方均收到成功 `Done` 和匹配结果，两侧内存记录器的请求 span 在最终检查点仍开放。
Rust 报告一次 WARN，后端包装已 Drop，`finish` 调用为零。两侧均保留首块时长样本，请求时长、
token 和 cost 样本缺失。同一原锁候选十二例共享断言通过，旧十一例一致，材料阻断为空。
本例记录 Rust 显式结算修复前的基线。

2026-11-11 完成[成功请求在时长记录失败后显式结束 span](issues/54-finish-request-span-after-duration-record-failure.md)。
自动观察器在 `pi.ai.request` 以 `Stop`、`Length` 或 `ToolUse` 完成且时长记录返回类型化错误时，
记录原始 WARN 并尝试结束 span 一次，保留已写入属性，跳过剩余的结果计量。#53 用例重放中，
Rust 后端 `finish` 为一次，span 带 `stop: stop` 结算。两侧源流释放和成功结果与基线一致，
Pi span 在有界检查点仍开放。结算状态及宿主诊断登记为具名有意差异。十二例共享断言通过，
两例通过、十例有已确认的差异，材料阻断为空。原始重放的 `strict_alignment` 为 `false`。

2026-11-12 完成[公开 deferred fetch 的两次零等待检查](issues/55-observe-deferred-wait0-pending-then-ready.md)。
同一 handle 的两次调用各派发一次，依次返回带原 handle 的 `deferred` 和就绪消息。
两侧的已结算 `pi.ai.poll` span 与计量一致，只有就绪结果累计 token 和 cost。
Rust 在每次结果返回时已完成计量和 span 结算，Pi 在随后事件循环检查点补齐。该时序差异待决。
十三例共享断言通过，旧十二例一致，材料阻断为空。

2026-11-12 完成[源流 EOF 暂缓时的公开 deferred fetch 差分](issues/56-observe-deferred-result-before-source-eof.md)。
两边各派发一次零等待检查。源已发出 `Done` 且 EOF 仍在屏障内时，Pi 已交付就绪结果，
Rust 的同一 fetch 仍待定，两侧 poll span 均开放。放行后两边最终结果和计量一致，
Rust 返回时已释放源。十四例共享断言通过，旧十三例一致，材料阻断为空。
新增的结果可见时点差异待决。

2026-11-12 完成[`Done` 后、EOF 暂缓时的本地 fetch 取消差分](issues/57-observe-deferred-local-cancel-after-done.md)。
两侧后端均收到本地取消状态。在 EOF 仍被屏障挡住时，Rust 已释放源流、结算 poll span 并返回
缓存的就绪结果。Pi 在同一脚本源上保留此前的就绪结果，源流与 span 到放行后才收尾。
两侧远端取消调用均为零，随后对原 handle 的显式 fetch 均成功。十五例共享断言通过，
旧十四例一致，材料阻断为空。三项 deferred 差异待决。

2026-11-12 完成[`Start` 后、`Done` 前的本地 fetch 取消差分](issues/58-observe-deferred-local-cancel-before-done.md)。
Pi 的脚本后端响应实际传入的 `AbortSignal`，公开 fetch 履约返回 `aborted` 消息。Rust SDK
取消仍待定的源流，返回带 `ProviderError::Cancelled` 原因和 `Aborted` 消息的 `Err(CallFailure)`。
两边均在结果可见时退出本地源流，最终 `pi.ai.poll` span 和计量一致。Rust 在返回前结算，
Pi 在公开结果可见后读取源结果并结算。远端取消调用均为零，原 handle 的显式重取均成功。
十六例共享断言通过，旧十五例一致，材料阻断为空。四项 deferred 差异待决。

2026-11-12 完成[显式远端取消和原 handle 重取](issues/59-verify-deferred-remote-cancel-then-fetch.md)。
冻结版 Pi Faux 与 Rust 脚本后端各执行一次待定 fetch、一次显式取消和一次失败重取。
取消携带调用方遥测，且没有额外 fetch 或 SDK AI span。重取在两边均为 `error`。
基线中 Rust 没有保留失败消息的 provider 和 model。修复后，`ChatStream` 在没有 partial
可用时用 handle 身份合成失败，重取结果的共同字段对齐。Pi 履约返回错误消息，并把首个
`error` 事件计为首块。Rust 返回 `Err(CallFailure)`，该路径当时没有首块样本。十七例共享断言通过，
旧十六例一致，材料阻断为空。当时有五项 deferred 差异待决。

2026-11-12 完成[首项为类型化失败时的首块遥测对齐](issues/60-align-first-chunk-on-typed-failure.md)。
新增零等待 deferred fetch 的鉴权失败。屏障内两侧待定，各进入 resolver 一次，后端派发和远端
取消均为零。放行后，Pi 与 #59 已派发的重取失败一样，把 `error` 计为首个规范化事件。
Rust 在两例都漏记首块。修复后，Rust 在交付首个规范化流项时记录首块，包括类型化失败和普通
chat 的准备失败。两例各新增一个首块事件和时长样本，错误内容、类型、派发次数和原有计量一致。
十八例共享断言通过，其余十六例完整一致，材料阻断为空。六项 deferred 比较仍有待决差异，
其中两例失败剩下结果通道、原因和诊断差异。

2026-11-12 完成[公开 deferred fetch 的正等待重放](issues/61-verify-deferred-positive-wait.md)。
两边每次调用各派发一次，后端收到 `50 ms`，原 handle 保留 `25 ms` 调度提示。后端首个事件
被屏障挡住时，两边的 fetch 均待定。首次放行返回带原 handle 的 `Deferred`，调用方显式发起
第二次 fetch 后取得就绪结果。两次 poll 的已结算遥测一致。十九例共享断言通过，既有十八例完整
一致，材料阻断为空。下一步对照过期 handle 的鉴权和派发边界。六项 deferred 差异待决，
真实 provider 的预算执行和六门验收仍待验证。

2026-11-12 完成[公开 deferred fetch 的 handle 过期差分](issues/62-observe-deferred-handle-expiry.md)。
同一到期时间的 handle 在到期前一毫秒及省略 Rust 时钟时，两边各鉴权、派发一次并返回就绪消息。
到期瞬间，Rust 在鉴权和派发前返回带 `Config` 原因的 `Err(CallFailure)`，冻结版 Pi 仍派发给
Faux 并取得就绪消息。提前校验源于 Opi 的 AIR-043。统一修正 `now_ms` 和 `expires_at_ms` 的
Unix epoch 毫秒约定，建议在截止时间和时钟可信时由调用方显式启用。二十例共享断言通过，
既有十九例完整一致，材料阻断为空。七项 deferred 差异待决。下一步对照 handle 路由不匹配时的
拒绝位置、鉴权和派发。真实 provider 过期语义和六门验收仍待验证。

2026-11-12 完成[公开 deferred fetch 的 handle 路由不匹配差分](issues/63-compare-deferred-handle-route-mismatch.md)。
分别改动同一 handle 的 provider、model 和 API。冻结版 Pi 每次鉴权、派发各一次，再由 Faux
拒绝。Rust 均在鉴权前拒绝。独立校验对 provider 和 API 返回 `Unsupported`，对 model 返回
`RouteMismatch`，fetch 均返回带 `Config` 原因的 `Err(CallFailure)`。原 handle 随后在两边均
成功重取。提前校验源于 Opi 的 AIR-043，能省去无效的鉴权和派发。API 不匹配时的 Rust 诊断
只指向 provider，定位仍不够明确。二十一例共享断言通过，既有二十例一致，材料阻断为空。
八项 deferred 差异待决。下一步对照路由正确但 job ID 不存在时的后端拒绝和诊断。

2026-11-12 完成[有效路由下未知 job ID 的 deferred fetch 差分](issues/64-compare-deferred-unknown-job-id.md)。
只替换 job ID 时，Rust 本地校验通过。两边各鉴权、派发一次，由后端拒绝，原 handle 随后都能
成功重取。失败消息的共同字段与已结算计量一致。Pi 履约返回错误消息，Rust 返回带脚本后端
`Config` 原因的 `Err(CallFailure)`，各保留自身诊断。二十二例共享断言通过，既有二十一例完整
一致，材料阻断为空。九项 deferred 差异待决。下一步修正 #63 的 API 查找诊断并重放两类失败。

2026-11-12 完成[deferred 注册查找诊断修正](issues/65-clarify-deferred-api-lookup-diagnostic.md)。
Rust fetch 和显式 cancel 在查找失败时报告请求的 provider 和 API，独立校验保留
`DeferredError::Unsupported { provider }`。同一原始锁材料下，#63 仅更新两条查找失败的
错误文本和对应的 poll 状态文本，鉴权和派发均仍为零。#64 的未知 job ID 仍由后端在一次
鉴权和派发后报告，完整记录一致。二十二例共享断言通过，二十一例完整一致，材料阻断为空。
九项 deferred 差异待决，下一步结合已有重放复核结果交付时序及流源清理责任。

2026-11-12 完成[deferred fetch 的结果时序与源流清理复核](issues/66-review-deferred-fetch-result-and-source-cleanup.md)。
Pi 1.3.0 与冻结版 1.5.0 都在首个终态推送时兑现结果，转发任务继续读取源流。Opi 的
AIR-043 原实现在终态处释放源流，当前的 EOF 排空和 poll observer 结算来自本迭代。
复核 #55–58 后，建议保留直接 fetch 在返回前收尾的语义。有限尾部会计入等待，取消在
`Done` 后结束待定尾部时保留已缓存结果，在 `Done` 前获胜时返回类型化取消。双语指南已
注明直接 `result()`、逐项读取和 `.start()` 的源流边界。复用 #65 的绑定材料，九项差异仍
待决。下一步用同一个 EOF 屏障验证公开 `stream_deferred` 的终态交付和清理时序。

2026-11-12 完成[公开 deferred 流式终态和源流清理差分](issues/67-compare-deferred-stream-source-cleanup.md)。
两边均在源流 EOF 前交付公开 `Done` 和迭代结束。Pi 转发任务已进入尾部，poll 在源流收尾后
结算。Rust 逐项读取在公开 EOF 时已结算 poll 并保留源流，对同一流调用 `result()` 才进入
尾部。Rust `.start()` 在交付公开 `Done` 前已释放源流并结算，未到达自然 EOF。三条路径的
业务结果、每次派发的 handle 与等待值、最终遥测一致。二十三例共享断言通过，既有二十二例
完整一致，材料阻断为空。十项 deferred 差异待决。下一步检查现有调用者，再验证启动后的
deferred 执行任务能否复用尾部排空并在消费者丢弃后独立收尾。

2026-11-12 完成[启动后的 deferred 源流排空](issues/68-drain-started-deferred-source-tail.md)。
仓库内仅差分探针启动 deferred collector。公开 `stream_deferred(...).start()` 现由原有
执行任务复用 `result()`，读到有限源流的 EOF、释放源流并结算 poll 后交付终态和结果。
在消费句柄丢弃后，执行任务仍会进入尾部并完成清理。先读到 `Done` 再启动也可排空源流
并取得同一结果。受控调用各鉴权和派发一次，远端取消为零。本地取消在终态缓存前返回
类型化失败，缓存后返回原结果。使用同一原始锁材料的二十三例共享断言通过，另外二十二例
与 #67 完整一致。十项 deferred 差异待决，下一步核对直接流式迭代的 poll 结算与源流寿命
能否对齐。

2026-11-12 完成[直接迭代停止消费的 deferred 清理差分](issues/69-observe-deferred-direct-consumer-stop.md)。
在 `Done` 和公开 EOF 两处停止消费，Pi 的转发任务均继续读到自然 EOF，再结算 poll。
Rust 直接迭代丢弃句柄时释放源流，未进入尾部。读到 `Done` 就丢弃时内存 poll 仍开放，
读过公开 EOF 后则已结算。两边交付的就绪消息字段一致，每次各鉴权和派发一次，远端取消为零。
早期 Rust collector 已在首终态后返回公开 EOF，后来接入的 poll 沿用了这个时点。下一步验证
直接迭代的最后一次 `next()` 能否复用尾部排空，在交付公开 EOF 前完成源流清理和 poll 结算，
同时测量等待时序与本地取消。二十三例共享断言通过，另外二十二例及 #68 目标用例原有检查点
一致，材料阻断为空。十项 deferred 差异待决。

2026-11-12 完成[直接迭代在公开 EOF 前排空 deferred 源流](issues/70-drain-deferred-source-before-direct-eof.md)。
Rust 先交付 `Done`，最后一次 `next()` 复用 `result()` 的排空。在源流 EOF 屏障内，这次读取
和 poll 均未完成。放行后，源流到自然 EOF 并释放，poll 结算，再返回公开 EOF。若本地取消
先结束待定尾部，则释放源流、返回公开 EOF 并保留缓存的就绪结果。各路径只鉴权、派发一次，
远端取消为零。Pi 仍在源流收尾前交付公开 EOF。二十三例共享断言通过，另外二十二例与 #69
完整一致，材料阻断为空。十项 deferred 差异待决，包含公开 EOF 的时序和在 `Done` 处提前
停止时的源流所有权。

2026-11-12 完成[放弃待定 `next()` 后接续 deferred 清理](issues/71-resume-deferred-tail-after-pending-next.md)。
收到 `Done` 后，让最后一次 `next()` 进入源流 EOF 屏障，再丢弃这次等待并保留 collector。
`result()` 和 `.start()` 都在放行前接手续读同一个源流。启动后立即丢弃消费句柄时，任务也
能接手并在放行后读到自然 EOF、释放源流、结算 poll。保留结果的两条路径取得与 `Done`
匹配的就绪字段，启动后的迭代不重复交付 `Done`。每条路径各鉴权、派发一次，本地与远端
取消均未触发。产品源码与 #70 相同。同一原始锁快照下二十三例共享断言通过，另外二十二例
及目标例的原有检查点一致，材料阻断为空。十项差异仍待决，下一步核对 Pi eager 入口的 Rust
能力映射、公开 EOF 时点及丢弃整条直接流时的清理责任。

2026-11-12 完成[Pi eager deferred 流的 Rust 入口映射](issues/72-map-pi-eager-deferred-stream.md)。
用 `Models::stream_deferred(...).start()` 迁移 Pi 的 `Models.streamDeferred`。Pi 在返回前进入
setup，Rust 显式调度 Tokio 执行任务，两边都能在消费前推进请求，并在停止消费后继续收尾。
直接 `ChatStream` 的轮询与释放由调用方负责。EOF 屏障内，Pi 已交付 `Done`、公开 EOF
和结果。Rust 直接流已交付 `Done`，启动流则仍等待尾部。两条 Rust 路径在源流释放后交付
公开 EOF 和结果，记录正常时 poll 已结算。建议保留这项收尾保证，接下来验证启动流能否
提前交付 `Done`，同时保持 EOF、结果及取消的完成语义。Pi 1.3.0 已有相同的独立转发和
提前交付边界。复用 #71 的原始锁证据，二十三例共享断言通过，十项 deferred 差异仍待决。

2026-11-12 完成[deferred 尾部被中断时的终态复现](issues/73-observe-deferred-abort-after-ready.md)。
先直接读到成功 `Done`，再用 `.start()` 接手待定尾部。强制 `abort()` 已等待源流释放，
但会把结果改为 `Err(Cancelled)`，poll 按 aborted 结算。关闭执行任务所在的 Tokio runtime
也会释放源流，随后启动句柄交付 `StreamError`，poll 仍开放。两个中断都未到自然 EOF，也未
触发请求令牌或远端取消。建议在 SDK 缓存首个成功 `Done` 时确定该次 fetch 的结果。强制
取消仍停止任务并等待释放，之后复用已确定的结果。这样可验证启动流提前交付 `Done`，而
公开 EOF 和 `result()` 继续等待收尾。两条新集成测试及其目标全部通过，#71 材料与重放
可复用，十项 deferred 差异仍待决。

2026-11-12 完成[启动后的 deferred 流在源流释放前交付成功 `Done`](issues/74-deliver-started-deferred-done-before-source-release.md)。
执行任务缓存首个规范化的成功 `Done` 后入队，继续复用 `result()` 排空同一源流。在 EOF
屏障内，Pi eager 入口和 Rust 启动入口都能交付 `Done`，Rust 的公开 EOF 和 `result()`
仍等待源流释放及 poll 结算。强制 `abort()` 或 runtime 停止发生在缓存后时，等待任务退出
可取得同一成功结果，包括未读终态。缓存前仍保留类型化中断，源流 panic 继续传播。每条
受控 fetch 各鉴权、派发一次，无远端取消。原始锁快照重放的二十三例共享断言通过，另外
二十二例、目标例的 Pi 轨迹及四十四条 Rust 检查点与 #71 一致。十项 deferred 差异待决。
下一步核对 Pi 的结果式消费与 Rust 收尾保证在尾部待定时的取舍，再决定 EOF 和结果时序。

2026-11-12 完成[deferred 结果式调用者与收尾时序复核](issues/75-review-deferred-result-consumer.md)。
冻结版 Pi Durable 仅等待 `fetchDeferred()`。带 EOF 屏障的 Faux 对照中，第一次返回 pending 后，
Durable 在首条尾流仍待定时用同一 handle 发起第二次 fetch，并在两条尾流都待定时写入就绪回答。
Rust 启动流读到 `Done` 即可把消息交给业务并让原任务继续清理，`result()` 则在屏障内待定。
建议为结果式迁移提供复用原任务的单次 `await` 就绪入口。现有 `fetch_deferred`、`result()`
及公开 EOF 提供收尾边界。下一步验证就绪入口的 pending、成功、
类型化失败和取消路径。#74 的二十三例共享断言及绑定材料可复用，十项 deferred 差异待决。

2026-11-12 完成[deferred 结果式就绪入口](issues/76-return-deferred-outcome-before-source-release.md)。
`Models::fetch_deferred_ready` 在 Tokio 上派发一次 fetch，首个规范化终态可返回带 handle 的
待定消息、就绪消息或类型化失败。同一任务随后复用 `result()` 排空源流并结算 poll。保留的
本地取消令牌能释放尾流并保留已交付结果。`fetch_deferred`、启动后的 `result()` 和公开 EOF
提供收尾边界。绑定原始锁材料的二十四例共享断言通过，原有二十三例与 #74 完全一致。新增
Pi/Rust 对照在 EOF 屏障内取得相同结果，随后观察源流释放及一次 poll 结算。十项 deferred
差异待决。下一步核对 Pi 的流式调用者是否依赖提前的公开 EOF。

2026-11-12 完成[deferred 公开 EOF 的调用者复核](issues/77-review-deferred-stream-eof-consumers.md)。
冻结版唯一逐项读完 `streamDeferred` 的用法是遥测测试，它核对事件和结果，再单独等待
span 结算。Durable 通过 `fetchDeferred()` 的结果继续业务。#76 的同一份材料显示，两侧在
尾流待定时交付相同 `Done`，Pi 此时可返回公开 EOF，Rust 的最后一次读取等待源流释放和
poll 结算。Rust 以 `Done` 作为业务继续点，保留成功终态后的公开 EOF 作为收尾边界，记录
该时序差异。二十四例共享断言可复用，十个 deferred 差分项仍保留。下一步核对 Durable
如何分类 deferred 错误和中止，验证类型化失败能否保留应用的重试、压缩和取消行为。

2026-11-12 完成[Durable deferred 失败分类复核](issues/78-review-deferred-failure-classification.md)。
冻结版应用的六例实测显示，收到 503 后重试会提交新任务，配额错误直接结束，两种溢出会先
压缩再提交。应用主动中止才在该组调用远端取消，provider 返回 `aborted` 时按模型失败结束。
Rust `CallFailure` 保留受控诊断和类型化取消。仅看类型的 retryable 标记会与 Pi 的消息策略
分歧。Rust 的 1.3 溢出规则漏掉 Pi 1.5 新增的两种形态，旧式 `aborted` 事件也会被规范化成
`error`。这些是已复现的应用语义缺口。下一步先补齐两条溢出规则及负例，再处理旧式中止和
应用重试谓词。二十四例冻结共享断言可复用，十个 deferred 差分项仍保留。

2026-11-12 完成[deferred 上下文溢出诊断对齐](issues/79-align-context-overflow-diagnostics.md)。
`is_context_overflow` 补齐冻结版 Pi 1.5 的 LiteLLM 与前置 HTTP 413 规则。两个 deferred
失败样本可供应用判定溢出，正文仅提及 413、限流及非错误终态的负例通过。整次调用重试会
先检查溢出，带 503 文本的 413 用例只调用一次。使用原始锁重新冻结并重放，二十四例共享
断言通过，结果与 #76 一致。十个 deferred 差分项仍保留。下一步保留旧式 `Aborted`
终态，再核对现有消息级重试分类与 Durable 的判据。

2026-11-12 完成[旧式 provider 中止终态保留](issues/80-preserve-legacy-provider-abort.md)。
`ChatStream` 对两个停止原因字段都为 `Aborted` 的旧式 `Error` 事件交付 `Cancelled` 与原消息。
deferred 的 503 中止样本保留 `Aborted`，现有消息级重试分类据此终止，调用方令牌保持有效。
应用主动取消由自己的令牌标识。尾流释放后 poll 以 `aborted` 结算一次。原始锁重放的二十四
例共享断言通过，逐案与 #79 一致，仍记录十个 deferred 差分项。下一步核对现有消息级重试
分类对瞬态与配额诊断的判定。

2026-11-12 完成[整次调用重试分类对齐](issues/81-align-whole-call-retry-classification.md)。
补齐冻结版 Pi 1.5 的两条配额规则、六条瞬态规则和状态码边界。23 条相同诊断的重试判定
一致，公开 deferred 入口中的 503、配额、溢出和中止结果符合 Durable 生成分支的判定。
OpenAI 兼容 429 的本地 HTTP 夹具验证配额诊断脱敏、重试提示和请求发送预算。用原始锁
重新绑定并重放二十四例，共享断言全部通过，与 #80 逐案一致，仍记录十项 deferred 差分。
下一步核对 Responses 的 `response.failed` 网关诊断在脱敏后能否保留重试事实。

2026-11-12 完成 [Responses 流失败分类保留](issues/82-preserve-responses-stream-failure-classification.md)。
同一份 SSE 夹具在冻结 Pi 1.5 和 Rust 公开入口验证了五个瞬态重试分歧，以及配额和溢出诊断
的丢失。Rust 将已识别的分类投影成固定诊断，保留部分结果和类型化流失败。响应交接后仍按
一次发送结算，只有显式的整次调用策略能为瞬态失败发起新调用。原始锁重放的二十四例共享
断言通过，仍记录十项 deferred 差分。下一步核对独立的 SSE `error` 事件。

2026-11-12 完成 [顶层 SSE 错误分类保留](issues/83-preserve-responses-sse-error-classification.md)。
按冻结 SDK 和 Codex 的实际取值规则处理命名 error、纯 data 和嵌套信封。三个原先被跳过的
错误现在返回失败，七个瞬态重试判据恢复。整帧回退中只命中无关字段或已被遮蔽字段的两例
保留终止。Codex 连接限额和续接恢复经过发送次数验证，交付部分结果后以该部分结果结算。
原始锁重放二十四例与 #82 一致。下一步核对连续连接限额后的 SSE fallback 和请求预算。

2026-11-12 完成 [Codex 连续连接限额后的 SSE 回退](issues/84-fallback-after-codex-connection-limit.md)。
冻结 Pi 1.5 和 Rust 在三个 WebSocket 模式及已缓存续接的场景中，均在两次首事件前的限额
拒绝后发送一次 SSE POST 并返回相同答案。续接被拒后，重连和 SSE 均发送全量输入。会话标记
在清理前保持生效，成功 Upgrade 后的高级 acquisition 保持封存。原始锁重放二十四例与 #83
一致，仍记录十项 deferred 差分。下一步核对 `response.failed` 内嵌恢复码的传输语义。

2026-11-12 完成 [Codex 内嵌失败码恢复](issues/85-recover-codex-response-failed-codes.md)。
`response.failed.response.error.code` 的两个精确恢复码与 `error` 帧共用现有预算，
修复三个提前失败的场景。混合帧的续接预算耗尽后保留最后一次失败的原始状态与脱敏诊断。
已交付文本时，冻结 Pi 会重发并拼接两轮结果，Opi 返回含原部分文本的失败。原始锁重放
二十四例与 #84 一致，仍记录十项 deferred 差分。下一步核对 Codex 的 `response.done`
终态和停止元数据。

2026-11-12 完成 [Codex 终态帧对齐](issues/86-align-codex-terminal-frames.md)。
SSE 与 WebSocket 共用三个终态帧的状态映射。三十二组配对的停止原因、原始状态、ID、用量、
文本或工具内容和 `end_turn` 一致。Opi 通过 `AssistantMessage::end_turn()` 读取持久化诊断，
保持现有消息构造兼容。SSE 在首个终态后释放响应体。未知的 `incomplete.unknown_503`
保留原始原因，Opi 按终态失败结算。原始锁重放二十四例与 #85 一致，仍记录十项 deferred
差分。下一步复现同一 WebSocket 连接上迟到终态对下一次调用的影响。

2026-11-12 完成 [Codex WebSocket 空闲帧隔离](issues/87-isolate-codex-websocket-idle-input.md)。
三种 WebSocket 模式共十五组配对，修复前十二组误把上一轮帧交给第二轮。Opi 在发送下一条
`response.create` 前处理可读输入，丢弃完整空闲消息并应答 Ping，十五组的两轮结果、回调和
发送次数与冻结 Pi 一致。部分帧、对端 Close 或读取预算用尽时更换连接，第二轮只发一次全量
请求。原始锁重放二十四例与 #86 一致，仍记录十项 deferred 差分。下一步对照上一轮终态在
第二次 `response.create` 发出后才到达时的响应归属。

2026-11-12 完成 [Codex WebSocket 发送后终态归属](issues/88-attribute-codex-websocket-terminals.md)。
三种模式共三十六组配对。十二组在 `response.created` 确认本轮 ID 后收到明确属于上一轮的终态，
Opi 跳过旧帧，保留本轮用量、回调和正确的下一轮续接 ID。请求次数和输入增量保持原有规则。
终态先于 `response.created` 到达或缺少嵌套 ID 时仍无法判定归属。原始锁重放二十四例与 #87
一致，仍记录十项 deferred 差分。下一步对照仅有 WebSocket 控制帧时的空闲超时和 SSE 回退。

2026-11-13 完成 [Codex WebSocket 应用消息空闲超时](issues/89-align-codex-websocket-application-idle-timeout.md)。
三种模式共二十七组对照。Ping/Pong 沿用同一应用消息等待期限，超时后尝试 `1000 / idle_timeout`
关闭。首事件前超时发送一次全量 SSE，首事件后返回部分结果，同一会话下一次调用走 SSE。
正常应用事件、零超时和慢回调对照均完成 WebSocket 调用。原始锁重放二十四例与 #88 一致，
仍记录十项 deferred 差分。下一步核对公开资源清理后，同一会话能否从 SSE 恢复 WebSocket。

2026-11-13 完成 [Codex 会话清理后的 WebSocket 恢复对照](issues/90-preserve-codex-session-cleanup-recovery.md)。
三种模式、两种清理范围共六组配对。定向清理让 Opi 的目标会话恢复 WebSocket，全局清理让
两个已回退会话都恢复。冻结 Pi 在两种清理后继续使用 SSE。健康对照连接按范围复用或关闭后
重连。九次路由差异均发送同一份全量输入，清理后的返回 ID 和用量随夹具传输路径变化。
保留 Opi 既有的清理契约。复用 #89 的原始锁材料和二十四例重放，仍记录十项 deferred 差分。
下一步核对 deferred handle 的实际过期约定与调用者，复审 #62 的拒绝派发边界。

2026-11-13 完成 [deferred handle 到期裁决复核](issues/91-defer-expiry-to-provider-by-default.md)。
冻结 Pi 的 Faux 生成 handle 时省略 `expiresAt`，Durable 保存整份 handle 并按 `pollAfterMs`
恢复查询。同一原始锁重放三种输入。Opi 在显式传入 `now_ms == expiresAt` 时零鉴权、零派发并返回
`Config`。省略时，双方各鉴权、派发一次并取得同一就绪结果。保留 provider 默认裁决以及调用方
对硬截止时间的显式检查。原始差分保留该边界，真实 provider 过期行为待验收。下一步核对 handle
路由校验对恢复后的 fetch 和 cancel 的影响。

2026-11-13 完成 [deferred handle 路由与取消复核](issues/92-validate-deferred-routes-before-dispatch.md)。
Durable 通过保存的 provider 和 model ID 查询当前模型目录，原样传入另存的 handle。
同一原始锁重放 #63，三种路由不匹配的 fetch 在 Pi 各鉴权和派发一次，Opi 均在鉴权前拒绝。
三种不匹配的 cancel 在 Pi Faux 均取消原任务，Opi 均拒绝且仍可读取原结果。路由有效但 ID
未知时，fetch 和 cancel 的结果由后端裁决。保留 Opi 的本地路由检查，原始差分继续记录。

2026-11-13 完成 [deferred 远端任务错误分类复核](issues/93-assess-deferred-job-failure-classification.md)。
同一原始锁材料重放 #64 和 #59，两个案例与 #89 的记录逐项一致。有效路由都派发到后端，
Pi 返回错误消息，Opi 返回含消息和后端 cause 的 `Err(CallFailure)`。冻结 Durable 收到四条
诊断后均记录错误，输入结为 `unanswered/model_error`，任务结为 `failed`，零重试、零额外取消。
两条 Rust 消息也判为 `Terminal`。脚本后端的 `Config` 可表达不可重试，但配置错误诊断误述了
后端任务状态。保留原始结果差分，真实 provider 的错误映射和应用验收待验证。

2026-11-13 完成 [deferred 真实接入核查](issues/94-record-real-deferred-integration-gap.md)。
冻结 Pi 的 42 个内置 provider 均未提供 deferred fetch/cancel，Faux 提供两者。Durable 有轮询和结算
实现，仓库内的 issue triage 通过 Codex 和 Anthropic 运行普通生成。Opi 的 deferred 注册均来自测试和
差分夹具。真实任务响应、生产重试与结算列为验收受阻，错误映射待有实际接入后决定。
#64 和 #59 的原始差分已核对保留。

2026-11-13 完成 [二十四例差分裁决状态核对及 #60 重放](issues/95-replay-deferred-auth-setup-failure.md)。
四例通过，十例普通调用差异已裁决。十例 deferred 原始差分中，#62、#63 已裁决，#59、#64 的
真实接入验收受阻，#55–58、#60 和 #67 的部分观察仍待裁决。复用 #89 材料重放 #60，双方各
鉴权一次、零派发、零远端取消，失败消息的共同字段与最终 poll 计量一致。Pi 兑现错误消息，
Opi 返回带 `AuthFailed` 的 `Err(CallFailure)`。结果通道和诊断待裁决。下一步可从 #55 的
返回时遥测可见性入手，逐项复核直接 fetch 的收尾契约。

2026-11-13 完成 [#55 零等待 fetch 重放与计量结算裁决](issues/96-decide-deferred-wait0-poll-settlement.md)。
四份轨迹与 #89 完全一致。两次派发的 handle、等待值、业务结果和最终 poll 计量相同。
保留直接 `fetch_deferred` 在返回前释放有限源流并结算 poll 的时序差异。Durable 受控
调用可在尾流结束前安排后续轮询并写入答案，对应 Opi 的 `fetch_deferred_ready`。
下一项是 #56，核对尾流持续待定时直接 fetch 与就绪入口的返回边界。

2026-11-13 完成 [#56 尾流挂起重放与返回边界裁决](issues/97-decide-deferred-fetch-held-eof.md)。
两组八份轨迹均与 #89 一致。源流发出 `Done` 后，Pi `fetchDeferred` 和 Opi
`fetch_deferred_ready` 已返回同一结果，直接 `fetch_deferred` 仍待定，三条路径的 poll
都在源流释放后结算。保留直接入口的收尾等待。Durable 在两条尾流挂起时仍可安排下一次轮询
并写入答案，其结果式调用对应就绪入口。下一项是 #57，核对收到 `Done` 后本地取消时的
源流释放与结果保留。

2026-11-13 完成 [#57 终态后本地取消重放与裁决](issues/98-decide-deferred-local-cancel-after-done.md)。
四份轨迹与 #89 完全一致。两边保留就绪结果，原 handle 均可再次 fetch，远端取消为零。
在 EOF 屏障仍关闭时，Opi 释放本地源流并按缓存结果结算 poll，Pi 的受控源流在放开 EOF 后
才完成清理。保留 Opi 的本地取消行为，Durable 的任务中止仍使用独立的显式远端取消。
下一项是 #58，核对终态前取消的错误通道、诊断与应用结算。

2026-11-13 完成 [#58 终态前本地取消重放与裁决](issues/99-decide-deferred-local-cancel-before-done.md)。
四份轨迹与 #89 完全一致。保留 Opi 的 `Err(CallFailure)`、`Cancelled` 原因和返回前的
源流释放及 poll 结算。双方最终计量一致，零远端取消，同一 handle 再查成功。Durable 的
受控重放确认应用中止按任务状态结算，零重试，并显式远端取消一次。下一项是 #60，核对
鉴权失败的错误通道和诊断对调用方记录与重试的影响。

2026-11-13 完成 [#60 鉴权失败重放与裁决](issues/100-decide-deferred-auth-setup-failure.md)。
四份 SDK 轨迹与 #89 完全一致。保留 Opi 的 `Err(CallFailure)`、`AuthFailed` 和现有诊断。
Durable 持久化两段各自的原文，均零重试，输入以 `unanswered/model_error` 结算，任务为
`failed`。受控 503 原因在两种诊断下都触发一次整次重试并恢复成功。原始差分照留。
下一项是 #67，核对启动后的 `result()` 与直接 collector 提前丢弃时的源流及 poll 责任。

2026-11-13 完成 [#67 结果时点与直接丢弃重放及裁决](issues/101-decide-deferred-stream-result-and-drop.md)。
四份 #67 轨迹与 #89 完全一致。Pi 在 `Done` 时兑现结果，Opi 的 `.start().result()` 在源流
释放及 poll 结算后返回。保留该收尾边界，用 `fetch_deferred_ready` 对齐 Durable 的结果就绪
时点。直接 collector 在 `Done` 后被丢弃会立即释放源流，但 poll 仍开放。保留直接所有权与
释放行为，下一步补齐该分支按缓存成功结果结算 poll 的逻辑。冻结原始差分照留。

2026-11-13 完成 [#67 直接丢弃后的 poll 结算修复](issues/102-settle-deferred-poll-on-direct-drop.md)。
收到 `Done` 即丢弃、尾流已挂起后丢弃，两种情况下均先立即释放源流，再按缓存结果结算一次。
正常 EOF 后丢弃仍只有一次结算，远端取消为零。复用 #89 的 Pi 快照和原始锁，二十四例
共享断言通过。原始差分及直接所有权和结果返回时序差异照留。

2026-11-13 完成 [#103 partial 引用重放](issues/103-compare-stream-partial-aliases.md)。用 #102
冻结材料对照 Faux 与 OpenAI Chat，保留 Opi 的事件时快照。已读取的旧事件不会随新事件改写，
调用方修改也不会进入最终结果。显式相同参数下，本地 OpenAI Chat 的请求、原始 delta 和基准终态
一致。Durable 受控提交显示用量可晚一条事件可见，进度文本和最终结算一致。工具参数预览与
OpenAI Chat 的 `text_end` 时点另有差异，已保留原始轨迹，待分别验证。

2026-11-13 完成 [#104 OpenAI Chat 交错文本修复](issues/104-align-openai-chat-interleaved-text.md)。
工具增量后的续写复用原文本索引，结束事件携带整块正文。两种冻结输入的完整 Chat 请求、
Anthropic 助手块和受控 Durable 的进度与续轮顺序均对齐。旧 Opi 会把续写拆块，并在下一轮
Chat 请求丢弃只含空白的后缀。原始轨迹照留，真实 finish 帧与 provider 迭代结束之间的块结束
时序，以及未完成参数的直接 SDK 预览仍有差异。下一步用相同模型记录配对重放未指定输出上限
的 simple、native 和 direct 入口。

2026-11-13 完成 [#105 OpenAI Chat 输出上限配对](issues/105-compare-chat-output-limits.md)。
同一模型记录、原始锁和响应材料下，native、simple、direct 共 42 对请求及结果相同。
native 和 direct 省略缺省上限，simple 按模型容量和剩余上下文生成上限。两种 Chat 字段名、
紧上下文、显式零值和 `length` 终态均已实测。#103 的请求差异来自混用了两类入口，原始
轨迹照留。修复 coding-agent 部分用量汇总的编译缺口后，输出上限和失败用量回归通过。
会话 JSONL 的 `cost` 持久化断言仍待处理。下一步验证 finish 到 `[DONE]` 之间的块结束
时序及其调用方影响。

2026-11-14 完成 [#106 会话费用持久化契约与当前候选包消费](issues/106-clarify-session-cost-and-verify-current-packages.md)。
消息原样保留响应时的 `Usage.cost`，包括目录分量、provider 总额及其来源标志；旧消息缺少
该字段时仍可读取。会话费用摘要继续按累计 token 与当前价格重算，不累加消息费用，也不
改写历史消息。#105 的失败来自早于 `Usage.cost` 的全局禁止断言，现改为验证完整快照、
显式零账单、旧格式及恢复后改价的行为；64 项会话测试通过，运行时序列化行为未改。
当前两 crate 的真实归档在 workspace 外通过双 profile README 接入及每档 73 项 Schema
检查，归档原始文件与当前源码、手册和测试全部匹配，旧包证明保持原样。该结果限定于当前
候选的接入和 Schema 消费；最小依赖配置、其他平台及六门验收仍开放。下一项仍是 Chat
finish 到 `[DONE]` 的块结束时序及调用方影响。

2026-11-14 完成 [#107 Chat finish、usage、sentinel 与调用方差分裁决](issues/107-observe-chat-finish-terminal-boundary.md)。
八组相同请求在冻结 Pi 1.5.0 与当前 Opi 上各重复三次：Opi 在 finish 提前闭块，收到 usage
即成功结算，导致之后的错误或取消被成功结果吸收；没有 usage 时又忽略 `[DONE]`，直到
HTTP EOF 才完成。真实 Opi Agent 在尾部仍阻塞时已执行工具并返回成功。冻结 Durable
Harness 的记录投影进一步验证了工具与存储后果：Pi 的失败尾部不执行工具，Opi 轨迹已
执行并提交成功。结论是需要修复，不能批准为有意差异；本轮只新增验证与证据，运行时代码
未改。下一步修复共同的协议终结边界，同时验证共享 Chat 路由、参数收尾、失败 partial
和 telemetry；不能只移动 TextEnd/ToolCallEnd。六门验收仍未闭合。

2026-11-14 按 #107 继续实施 [#108 Chat 终结边界修复](issues/108-repair-chat-terminal-boundary.md)。
finish/usage 只累积元数据；协议 sentinel 或有效 EOF 才结束块并发布 Done。
共享 Chat 路由统一保留尾部失败与取消，sentinel 后释放连接。取消不补造块结束，
与 Pi SDK 取消展开的事件差异单独保留；冻结配对、真实 Agent 与 Durable 投影复验
结果以 #108 的最终证据为准，历史 #107 记录不覆盖。

2026-11-14 [#109](issues/109-repair-baseline-checks-and-refresh-packages.md) 完成三项基线检查修复，完整工作区回归通过，并为当前候选刷新实际包消费。README、Schema 和新增 Chat 外部消费者在各自两档 feature 配置下通过；包内源文件已逐项绑定当前工作区，旧证据保留原字节。这完成了本轮基线与包消费补证，六门整体验收仍未闭合。

2026-11-14 [#110](issues/110-verify-chat-argument-previews-and-retain-literals.md) 完成直接 SDK 参数预览的十组输入、65 个分块检查点配对，并修复完整 `true/false/null` 位于未闭容器末尾时被丢弃的问题。修复前后各三轮稳定；请求、最终参数和实际 Durable 组件提交一致。Pi Messages 与原生 Mistral 的增量/最终权威控制、当前包的两档外部消费均通过。其余 18 个 Chat 预览检查点仍有差异，尚未批准为有意差异；下一步独立落实增量参数预览映射，不能把现有 whole-prefix helper 当成 Pi 新解析器。

2026-11-14 [#111](issues/111-map-incremental-argument-previews.md) 新增独立的
`streaming_json::ToolCallArgumentsStream`，映射 raw、append、replace、独立 snapshot
及严格 finish。直接 SDK 调用方按工具分别持有 owner；旧 whole-prefix helper 保留。
用户批准未配对 UTF-16 码点暂缓发布，配对后继续，作为明确时序差异。三轮冻结 Pi 配对
关闭原 18 项预览差异，65 个 Chat 检查点、20 例 / 105 项接口操作和 1,200 项复制额度
观察均有记录。调用方与本轮实际包的默认功能关闭/开启两档验证通过；153 份历史 JSON
证据字节不变。非有限数字、深度及成本边界仍单独记录，不计作完整严格对齐。


2026-11-14 [#112](issues/112-retain-complete-preview-at-representation-boundaries.md)
按用户决定保留标准 JSON、128 层上限及完整旧快照，修复非有限数字被省略导致数组移位
的问题。23 例 / 124 步接口控制和原 65 个 Chat 检查点重跑冻结 Pi 三轮，实际包消费两档
通过，历史证据保留。实际 Pi TUI/Durable 的合并节奏已核对，受控采样单独记录复制量，
不设未经测量的产品预算。最终大数重解码差异确认会进入 Agent 和会话持久化，作为下一
独立切片；不因预览接口返回 raw 文本而宣称调用方无影响。

2026-11-14 [#113](issues/113-preserve-final-numeric-decoding.md) 承接最终数字重解码修复：
SDK 启用正确舍入的 JSON 浮点解码，实际工具调用与会话保存、恢复的红绿回归通过；
三轮冻结 Pi 配对及实际包消费验证通过，原有 27 条表示边界差异继续保留。工作区
检查发现并修正旧模块清单遗漏，原始失败与复验证据分别记录，历史证据保留。

2026-11-14 [#114](issues/114-enable-minimal-sdk-profile.md) 核对 #113 后的验收缺口，
优先补齐真实最小配置：默认开启 `builtins`，关闭默认后保留 core 与现有脚本化 Faux，
排除内置网络/TLS/云认证/压缩后端和内嵌目录 API。独立 tracing、默认及内置后端无
tracing 的实际包消费均有新证据；默认和最小配置回归通过。旧 tracing-only 包证据、
全部已确认差异和历史失败保持原样。完整 Faux、完整配置集合 V、其余 adapter/observer
及逐入口调用方映射、MSRV/其他原生平台、数值预算和 #94 真实 Deferred 仍开放。

2026-11-14 [#115](issues/115-preserve-anthropic-usage-updates.md) 补录当前 `Models`
全部 64 个公开方法与实际调用方、raw/prepared/observed 所有权映射，优先修复
Anthropic 增量用量显式零值丢失、缺省 stop reason 覆盖及失败 partial 成本过期。
三轮冻结 Pi 配对差分、Agent 会话持久化/恢复和两档实际包消费通过；事件时点快照、
observer 故障隔离等既定差异保留。原始轨迹及旧检查器保留，强化检查后另存重判回执；
244 份旧 JSON 证据字节不变。下一项为 Anthropic 嵌套 1h/reasoning 字段及计费影响，
随后继续其他 adapter/observer 和公开辅助能力；完整 V、MSRV/其他原生平台、预算及
#94 真实 Deferred 仍开放，未声明完整对齐。

2026-11-14 [#116](issues/116-preserve-anthropic-nested-usage.md) 已补齐 Anthropic
`cache_creation.ephemeral_1h_input_tokens` 与 `output_tokens_details.thinking_tokens`
在 `message_start`/`message_delta` 的映射、子集校验、Agent 统计和会话恢复。冻结
Pi 配对及两档真实包消费通过；低层 mapper 无目录上下文的成本差异和后续 wire error
的 owned partial 时序已作为范围内差异保留。其他 adapter/observer、公开辅助调用方、
完整配置集合 V、MSRV/原生平台、预算及 #94 真实 Deferred 仍开放。

2026-11-15 [#117](issues/117-openai-choice-usage-and-helper-callers.md) 核对其他
adapter/observer 与公开辅助调用方映射，优先修复 OpenAI-compatible Chat 的
`choices[0].usage` 回退：冻结 Pi 在缺少顶层 `chunk.usage` 时仍消费该字段，Rust
此前会丢失 Moonshot 类端点的最终用量。现在顶层用量保持优先，choice 用量作为回退，
推理子集校验覆盖两条路径；冻结 Pi 三轮配对及当前候选实际包的默认/最小两档消费通过。
公开 `parse_sse_events`/mapper、Models simple/direct、Agent Chat 及已有
`parse_streaming_json`/`finalize_tool_arguments` 调用方映射已补录；#19 规定的图像、
分类和 deferred-cancel carrier-only observer 保持不变。其他 adapter/observer 故障阶段、
完整 V、MSRV/原生平台、预算及 #94 真实 Deferred 仍开放。

2026-11-15 [#118](issues/118-bedrock-pending-metadata-and-observer-tail.md) 修复
Bedrock Converse 的 pending 终态边界：messageStop 只记录待定 Done，所有后续
metadata（包括重复帧）先经过 observer 与 mapper，EOF 后才发布唯一 Done，最后一条
有效 metadata 用量保持权威；尾部 observer 失败在 Done 之前以 typed callback
failure 结束。共享 Bedrock HTTP 路由、重复 metadata、无 metadata EOF 和尾部 observer
故障均有回归；冻结 Pi 1.5.0 三轮配对差分及当前候选实际包默认/最小两档消费通过。
EOF 刷新保留 cache_write_1h；首轮全套回归发现的这一边界错误、缓存回收和
后续重跑均保留在 phase7 v3 历史证据中。已确认的 Rust typed error/owned partial
形态差异和 #19 非 Chat observer 边界保留。
其他 adapter/observer 阶段、完整 V、MSRV/原生平台、预算及 #94 真实 Deferred
仍开放。

2026-11-15 [#119](issues/119-responses-terminal-tail-observer.md) 修复共享
OpenAI Responses HTTP 适配器的终结时序：`response.completed` 与
`response.incomplete` 先记录 pending terminal，所有后续 provider item 继续
经过 raw observer，EOF 后才发布唯一 Done；重复 terminal usage 以最后一份
快照为准，尾部 observer/mapper/取消故障在 Done 之前以 typed failure 结束。
OpenAI 与 Azure Responses 共享路径的 adapter 回归、冻结 Pi 三轮配对及当前
候选实际包消费均通过；Codex WebSocket 的既有终端控制保持不变，并明确记录
Rust normalized fixture 与 Pi provider-event 观察面的差异。其他
adapter/observer 故障阶段、重复/pending 路径、完整 V、MSRV/原生平台、预算及
#94 真实 Deferred 仍开放。

2026-11-15 [#120](issues/120-anthropic-message-stop-tail-observer.md) 修复 Anthropic
Messages 的 `message_stop` 终结边界：适配器先记录 pending terminal，重复
`message_delta`/`message_stop` 继续经过 mapper 与 raw observer，EOF 后才发布唯一
Done；最后一份有效用量保持权威。尾部 observer 失败在 Done 之前以 typed callback
failure 结束，mid-output fallback 等 typed transport failure 仍立即终止。Anthropic
共享 HTTP 路由、冻结 Pi 三轮配对及当前候选实际包消费均通过；低层 mapper 的直接
`process` 入口保留即时终结作为明确调用方差异。其他 adapter/observer 阶段、完整 V、
MSRV/原生平台、预算及 #94 真实 Deferred 仍开放。

## 目标与范围

让第三方 Rust 开发者按 opi-ai 文档，仅依赖 opi-ai 及其声明的库依赖，就能在普通项目中使用与指定 Pi 1.5.0 对应的能力。无需 opi-agent、opi CLI、仓库私有路径或隐式全局初始化；可以保持同一 workspace，不要求独立仓库。

采用 Rust 原生接口，保持完整能力映射，可观察行为逐项验收；有意差异单独讨论并记录，不能算严格等价通过。目标不仅是聊天，也包括图像、分类、deferred、Faux、认证、目录、工具/schema、transcript、扩展、compat、公开辅助能力及包级工具的对应关系。

本方案定义重构范围、依据和迁移次序。基准与映射保持冻结；当前开始按后续阶段迁移产品源码、Cargo 配置、文档与相关测试。第二阶段按可独立验证的切片推进，中立遥测与纯值/错误、auth/catalog 分离均已实施。已增加显式 start 的独立执行者，并迁移 agent 主循环和 preparation summary 消费者；普通 chat 入口现已调度独立执行者，受影响回归与实际包消费检查通过，当时的工作区既有失败已由 #109/#113 修复并复验；随后建立了 AI telemetry 手动 seam、`ChatStream::result()` 的 source 尾部结算，在普通 Models chat 路径自动安装 observer，并把 `opi-telemetry` tracing 适配改成默认开启但可关闭的 `opi-ai` feature。deferred fetch 现在复用同一 `ChatStream` 结算并自动产生 `pi.ai.poll`；prepared attempt 保留原始 raw seam，并新增 observed ChatStream helper；调用方持有 raw `EventStream` 时可通过公开 `ChatStream::new_with_telemetry` 复用同一 observer。prepared observed helper 的跨 attempt 聚合按已确认契约由调用方拥有，不作为自动合并缺口；其他 provider、raw/prepared/non-chat 的 observer 接线与故障语义、完整 feature 矩阵和完整 Pi 差分仍需按范围核对。本次没有发布、真实 provider 调用或整个 pi-coding-agent 的实现。opi-agent/opi-coding-agent 中已有消费者要随本次公共接口调整迁移，完整 coding-agent 能力对齐另行安排。

## 已确认的设计

| 议题 | 实施方向 | 权威决策 |
| --- | --- | --- |
| 独立接入与对齐 | Rust 原生表达，完整能力与可观察行为分别举证，有意差异按具体观察确认 | [公共契约](decisions/04-public-contract.md) |
| telemetry 拆分 | 新增中立 opi-telemetry，AI 埋点留 opi-ai；宿主拥有后端初始化与输出策略 | [遥测边界](decisions/05-telemetry-boundary.md) |
| 请求生命周期 | 执行与消费分离；释放消费句柄不隐式取消；请求执行者负责资源与遥测结算 | [生命周期与失败语义](decisions/11-telemetry-lifecycle.md) |
| 工程结构 | 一个 opi-ai 加一个 opi-telemetry；按责任组织模块，保留实际高级入口并迁移调用者 | [模块结构与公开入口](decisions/06-module-structure.md) |
| 第三方依赖 | 成熟依赖、薄适配优先；必要补充与最小自研须有具体反例及同一契约验证 | [复用与自研边界](decisions/07-dependency-boundary.md) |
| 测试 | 按访问边界和契约确定主要验证位置；用替代证据去重，保留能发现不同错误的贯通验证 | [测试归属与去重](decisions/08-test-boundaries.md) |
| README | Pi 风格任务手册；常用例子在正文，长流程在专题，精确合同在 rustdoc；中英文共用受测示例 | [开发者文档](decisions/09-readme-contract.md) |
| 默认配置与验收 | 默认完整内置能力、可选最小构建、显式注册；本次建立完整基线，后续按影响补证 | [验收政策](decisions/10-acceptance-policy.md) |

### telemetry 的实际边界

opi-ai 依赖 opi-telemetry；中立 crate 不反向依赖 AI、agent 或 CLI。中立范围覆盖 AI 所需的 context/span、属性/事件/状态、metrics、schema 描述、no-op、轻量记录器与共享一致性验证。tracing 是可选适配，不成为所有用户必须接受的公共类型。

AI 专属 schema、请求/poll 的观测入口、token/cost 和结算规则仍在 opi-ai。宿主拥有 exporter、采样、脱敏、初始化和 flush/shutdown。Store/query/render、数据库和全部 pi-telemetry 能力不因拆 crate 自动纳入；传播与 links 按已映射的公开能力和实际消费者落实。

“消费者不再读取”与“请求终结”是两件事：错误、显式取消、超时和停止消费分别验证，不能在 stream 返回、消费 Drop 或最后一个 context clone 释放时一律结束 span。原版异常路径与工程资源要求发生冲突时保留两份证据，回到具体差异决策。

### 建议目录与依赖

以下是责任布局，目录名、文件粒度和逐符号签名可在实施中调整，并非每个目录都公开：

```text
crates/
  opi-telemetry/       中立观测合同、no-op、记录器、可选适配
  opi-ai/
    src/
      lib.rs          常用公共门面
      types/          消息、模型、图像、分类、deferred 等纯值
      errors.rs       通用错误合同
      models/         集合、查询、路由与认证/刷新编排
      provider.rs     服务对象与扩展合同
      providers/      内置工厂、Faux
      api/            操作合同、原生选项、协议适配
      execution/      高级控制；私有执行驱动与资源所有者
      stream/         事件、消费与结果合同
      auth/           凭据、Store mutation、OAuth 与云认证
      catalog/        合同、来源与资产分离
      transcript/
      schema/
      tools/
      transport/      必要配置与宿主注入；backend 默认私有
      telemetry.rs    AI 专属 schema 与 instrumentation
      utils/          确有公共契约的辅助能力
      compat/
    tests/            公共合同、协议、跨模块与消费验证
    examples/         文档共用的完整程序
```

纯值和通用语义不反向依赖 SDK、网络或内嵌目录。auth/catalog 合同与具体后端分离；Models 通过操作合同路由，不直接包含某个 provider 的特殊规则。execution 拥有执行期资源，stream 表达消费关系；不能把旧的巨大 Models 搬成新的巨大通用 trait。

PreparedRoute/PreparedProviderCall、attempt/retry admission、Store mutation 及必要 transport 注入均有真实消费者，迁移时保留能力。测试专用状态探针回到 crate 内，正式 Faux 继续公开。保留 Tokio 路线，本次不增加多运行时、no_std 或预先拆分 ai-core/http/ws 等 crate。

### 默认安装与运行时

默认构建包含支持平台上的完整内置模型调用能力，包括内置协议/provider、目录和必要认证。调用者可关闭默认 features，获得核心＋Faux 的最小配置，再按需启用能力。最小配置排除可选网络 backend、云认证 SDK、内嵌目录与具体遥测适配，但不等于零依赖或 Tokio-free。

普通 Models 构造为空；单 provider 注册和显式全内置构造并存。编译进某个 provider 不意味着自动注册、联网或登录。初始化时点按各入口实际合同验收。默认完整不等于开启 exporter、CLI、测试设施和所有互斥后端，也不消除 Rust 编译依赖成本。具体 feature 名称、有效组合及成本通过实现验证，参见[Pi 默认策略的源码依据](evidence/default-configuration.md)。

## 测试与 README 的组织方式

| 验证对象 | 主要位置与方式 | 去重约束 |
| --- | --- | --- |
| 私有解析、状态机与只能内部观察的不变量 | 所属模块内测试；按需要参数化、属性验证 | 不为每个私有函数机械配测试；精准联合测试可替代简单委派单测 |
| 公共值、公开小工具和 API 合同 | 外部契约测试 | 小能力也须有明确断言，不能以“贯通时调用到了”代替 |
| 请求、流事件、错误、取消、Store 与跨模块协作 | 受控 peer 的协议/贯通测试，必要时独立 TS/Rust 差分 | 保留独有的接线、时序与副作用反例，不重复完整用例矩阵 |
| 安装、features、真实包、文档程序 | 复用并扩展现有外部包消费 harness | 仓库内编译不能代替实际打包产物的独立消费 |
| 不可信输入、资源、性能、真实服务 | 专项入口，按影响及里程碑预算执行 | 已发现失败样本转成确定性回归；live 显式启用 |

用一份契约索引连接源能力、Rust 入口、主要/互补测试、原始差分记录与旧案例替代关系，不另建重复验收台账。删除测试前证明输入类别、时序、错误阶段、副作用和资源风险仍可被发现；关键风险用具名错误变异检验。先整理 target/helper 并保留断言，再逐批合并或删除，不能凭测试数量、文件大小或旧注释直接删测。

README 从“安装 → 单 provider 的完整调用 → 离线 Faux → streaming/result → 工具续轮”开始，再展开图像/分类、Models/Provider、认证与目录、原生选项、错误与取消、用量与遥测、transcript、扩展/deferred/compat、配置与迁移。常用能力保留可用短例和关键限制，复杂协议细节进入专题，工程历史离开接入正文。篇幅不设任意上限，能力入口不能丢失。

文档例子必须对应真实公开 API。中英文、examples、rustdoc 和包内代码保持一致；快速开始包含可复制的完整 manifest、imports、main 和命令。复用已有 scripts/pi-sdk-conformance 的实际外部执行路径，允许由完整示例提取正文片段，但不以片段编译替代完整接入证明。源码/示例/文档随各阶段同步调整，不把未实现目标提前写成现状。

## 实施顺序与每步完成条件

以下按阶段实施，第一阶段的状态和产物通过上方实施票与报告追溯。每个切片先确定受影响的观察与回归，再调整结构；测试和文档随接口迁移更新，最后一阶段才给完整里程碑结论。

| 阶段 | 工作及产物 | 该阶段完成的证据 |
| --- | --- | --- |
| 基准与映射 | 固定源数据/依赖/运行时身份；建立单一契约索引；清点公开入口、真实消费者和现有案例 | 旧标准与当前快照差异有归属；已知缺口被标记；关键生命周期/别名/异常有可执行观察设计；不把静态结论当原版运行记录 |
| 值类型与基础边界 | 拆纯值/通用错误，分离 auth/catalog 合同与后端；建立中立 opi-telemetry 及必要适配 | 依赖方向可验证；中立 crate 可单独消费；保留认证/Store/目录行为；测试无额外公共探针 |
| 执行与协议组织 | 分离执行者和消费者；接入 AI instrumentation；调整 Models、Provider、API、工厂；同步高级调用者 | 停止消费、全不消费、显式取消、异常、重复身份等关键路径保持合同；业务与遥测分别结算；工作区消费者迁移并通过受影响验证 |
| 功能配置与依赖适配 | 实现默认完整/最小/定制构建；冻结有效配置集合；必要时探证第三方 backend 候选 | 各配置有真实依赖图和外部消费结果；构造与首次调用副作用可观察；候选与现有实现跑同一合同。无证据不顺带替换 backend |
| 测试整理与接入文档 | 按已建立的归属整合 target/helper，补真实入口缺口，逐批去重；完成双语 README、专题、示例与迁移表 | 每次删除有替代映射；关键反例仍被检测；两 crate 的实际包可被普通外部项目使用；完整程序及受控协议执行可复现 |
| 完整对齐里程碑 | 校准并确认数值预算；固定候选/平台/配置；执行或复用仍适用的六门证据 | 给出逐项结果、证据身份、差异与缺口；符合条件才声明对应级别通过；发布动作另行安排 |

基础边界中的中立遥测提取与后续生命周期迁移分别验证，移动文件本身不算语义对齐。已缺源数据只阻断受影响的目录/价格/原版整包结论，其他可独立验证的工作可推进。每阶段的源码改动仍按仓库规则选择检查，不用本表替代具体变更的影响分析。

第三方 backend 的研究是有条件工作：先为真实不兼容点给出输入和观察，再评估复用、适配、补充或自研。当前没有批准某个 WS/AWS 包版本、移除 PMD、重写官方 SDK 或删除现有实现；也不要求为每个 npm 包建立同名 Rust 子模块。未来确有独立消费者和维护收益时，再讨论提取 crate。

## 验收频率与结论

| 时点 | 要求 |
| --- | --- |
| 日常修改 | 仓库要求的检查，加受影响的确定性回归；文档修改不触发完整协议/性能矩阵 |
| 集成与本次中间版本 | 按影响运行离线组合、协议、差分、平台与包消费验证；已有证据经适用性检查可复用 |
| 本次独立重构＋Pi 1.5.0 完整对齐里程碑 | 建立范围、行为、结构、测试、运行、交付六门的完整基线；按冻结配置与已确认预算执行 |
| 后续普通发布 | 沿用已有 CI/发布要求，补因代码、依赖、平台等变化失效的证据；不默认全量重跑 fuzz、耐久和性能 |
| 后续更换基准、重大架构变化或影响无法界定 | 重新建立完整基线；影响有界的 backend 更新只扩大到相关合同及专项 |

行为事实与工程预算分开。30 CPU 分钟 fuzz、10000 次耐久/属性案例、性能倍率等仍是[验收政策](decisions/10-acceptance-policy.md)中的候选值；先明确适用范围、负载、计数、测量环境和预计成本，校准后由用户确认，再冻结正式要求。校准不等于通过，不能在验收失败后改低 expected 并覆盖原记录。

每项区分通过、失败、未执行、受阻、不适用和有意差异。范围限定通过、行为严格对齐、完整六门通过、包含已批准差异的交付、真实服务当前可用，分别给结论。关闭某项 feature 只影响特定配置，不能从总体能力分母删除该能力；缺凭据/数据/工具及 skip 都不能计为通过。

三 OS 原生验证沿用仓库重大里程碑政策。配置集合覆盖最小、默认、单独能力、有效两两与已知高阶组合，以及完整能力；all-features 只有合法时才是一行。记录实际 OS/架构、runner 镜像、工具链、依赖、命令和产物，区分原生运行与仅交叉编译。时间敏感的服务可用声明另需相应新证据。

## 实施前置条件与重新讨论的触发点

这些事项有明确处理方式，不把“规划完成”解释为材料、预算或兼容证据已经齐全。

| 待落实项 | 下一步及阻断范围 |
| --- | --- |
| pinned catalog 已在原锁候选中重建并绑定差分 | #40 候选 source/dist 和编译后导出均为 42 provider / 1712 model，匹配原始响应。目录数据身份已有证据，目录/价格全合同仍待验收 |
| 原锁与 Windows x64 安装身份已验证 | #40 新副本的根锁字节和冻结哈希一致，安装锁及实际包身份已复核，生命周期材料阻断已清除。验证使用 `npm ci --ignore-scripts`，native hooks 和实际 npm 包消费仍须独立验证 |
| JS 共享/异常的 Rust 映射 | [#95](issues/95-replay-deferred-auth-setup-failure.md#disposition-of-the-frozen-cases) 核对二十四例裁决状态，#55–58 与 [#60](issues/100-decide-deferred-auth-setup-failure.md) 已有裁决。[#101](issues/101-decide-deferred-stream-result-and-drop.md) 裁决 #67 的结果与所有权边界，[#102](issues/102-settle-deferred-poll-on-direct-drop.md) 修复直接丢弃已交付 `Done` 的 collector 时漏掉的 poll 结算。十项 deferred 原始差分保留，#59 和 #64 的真实接入验收受 [#94](issues/94-record-real-deferred-integration-gap.md) 阻断。[#103](issues/103-compare-stream-partial-aliases.md) 已核对 Faux 与 OpenAI Chat 的 partial。[#104](issues/104-align-openai-chat-interleaved-text.md) 修复 Chat 工具增量导致的文本拆块。[#105](issues/105-compare-chat-output-limits.md) 确认配对入口的 Chat 输出上限与返回一致。块结束与终态边界按 [#108](issues/108-repair-chat-terminal-boundary.md) 修复；取消不补块结束作为明确差异保留。直接 SDK 工具参数预览已由 #111–113 在限定范围内修复并验证，未配对 UTF-16、非有限数字/128 层完整快照保留等差异按用户决定保留。其他 adapter 的引用合同和 observer 故障仍待逐项核对 |
| 逐符号与消费者迁移 | #115 追加当前 Models 的 64 个公开方法及实际 agent/coding-agent 调用方与 observer 所有权；保留原清单。其余公开辅助能力和外部使用方继续核对，不能以仓库无引用推定没有外部消费者 |
| feature、平台和工具链精确集合 | #114 实施默认开启的 `builtins` 分组与真正的 core＋脚本化 Faux 最小配置；这尚未冻结完整 V。根据实际依赖闭包冻结其余配置集合与支持条件；MSRV 先按当前声明 1.97 评估，变更需更新支持承诺；验证加法合并下的外部消费 |
| 定量预算与维护成本 | 做标注清楚的校准，提交负载/方法/成本/推荐门槛；数值正式确认前不能签署相关验收通过 |
| 独立包与发布版本 | 按届时版本状态确定适合公共接口迁移的 0.x 版本与两 crate 版本依赖；尚未发布时用仅含候选产物的受控源验证，不能依赖仓库源码 patch |
| 具体第三方替换选择 | 仅在发现真实缺口时做候选兼容探针；新增语义例外、公开宿主要求或维护负担时重新讨论，普通私有实现细节由实施者处理 |

基准身份变更、能力删减、行为例外或支持承诺改变，需要更新对应决策；一般目录名、私有函数布局和不改变合同的实现选择无需反复确认。没有出现这些冲突时按已确认方案推进，不新增一层无依据的审批流程。

本轮 #121 已核对 Google Generative AI/Vertex 共享 Gemini mapper 的重复
`finishReason`、尾部 usage、raw observer 故障和 EOF 终结边界。适配器在
finish 后保持 pending，消费重复尾帧并以最后 usage 发布单个 `Done`；尾部
observer/解析/取消错误保留 partial 且不补发延迟 `Done`。冻结 Pi 1.5.0
三轮配对差分、Gemini fixture、实际归档包的 builtins/no-tracing 与默认
档均通过。Pi 的原始 observer 记录与 Rust 规范化事件形状仍按已确认的
报告差异保留；公共离线 fixture helper 在 malformed 尾帧后也不再补发
过期 `Done`；其他 adapter/observer 故障阶段和 repeated/pending 路径继续
逐项核对。

本轮 #122 已核对原生 Mistral Conversations 的重复终结、choices 为空的
尾部 usage、raw observer 故障、malformed/pending 尾帧和失败后的响应释放。
mapper 现在以最后一次合法 `finish_reason` 为权威：`error → usage-only →
stop` 会发布一个最终 `Done`，最后 usage 为 `10/9/19`；尾部 observer、解析
和缺少 finish 的错误保留 partial 且不补发 stale `Done`，后续请求可继续
执行。冻结 Pi 1.5.0 三轮差分与实际归档包的 builtins/no-tracing、默认档均
通过；Pi 的 synthetic `start` 与保留旧 `errorMessage` 已记录为报告形状差异。
共享增量目标因默认 profile 写入时空间不足，按已审查计划回收 6,144 个
增量 payload 文件后重跑通过；计划摘要和失败 receipt 均保留。其余
adapter/observer 故障阶段、repeated/pending 路径继续逐项核对。

本轮 #123 已核对原生 Pi Messages 的终结元数据、重复终结、raw observer
故障和响应释放。`done`/`error` 作为协议自身的终结边界，首个终结只发布一次
`Done`；后续重复终结和 `[DONE]` 不再进入 adapter。mapper 现在保留终结事件的
`responseModel`，与 `responseId`、provider thinking level、usage 和 rewrite
元数据一同交付。终结前 raw observer 故障返回 typed `CallbackPhase` partial
且不补发 `Done`，同一 provider 的后续请求可以继续。冻结 Pi 1.5.0 三轮配对
差分、`chat_pi_messages`/Pi Messages 聚焦测试、clippy 和实际归档包的
builtins/no-tracing、默认档均通过；Pi 的 provider-event 记录与 Rust 规范化
事件形状、错误 AssistantMessage 表达继续按已确认差异保留。其他
adapter/observer 故障阶段、repeated/pending 路径、完整 V、MSRV/原生平台、
预算和 #94 真实 Deferred 仍开放。

本轮 #124 补齐共享 OpenAI-compatible Chat 的 response observer 故障阶段。
响应头已取得后，`onResponse` 失败现在有可执行的 typed `CallbackPhase`
partial/no-event 证据；响应释放后同一 provider 的第二次请求仍能完成，最终
 usage 为 `2/1/3`。冻结 Pi 1.5.0 三轮配对和实际归档包双 profile 均通过；Pi
 的 error AssistantMessage 与 Rust typed `CallFailure` 继续作为报告形状差异保留。
本切片没有生产源码改动，确认的是共享 Chat producer 的既有合同。其他
 adapter/observer 故障阶段、repeated/pending 路径、完整 V、MSRV/原生平台、
 预算和 #94 真实 Deferred 仍开放。

本轮 #125 冻结当前 `opi-ai` manifest 的配置候选清单。`builtins` 与
`telemetry-tracing` 是目前声明的两个独立开关，因而形成 `core`、
`core-tracing`、`builtins-no-tracing` 和 `defaults-on` 四档；四次
`cargo metadata` 均通过，且每档均可映射到已有的独立归档包消费 receipt。
这只是当前候选 V 的身份和证据映射，不把粗粒度 `builtins` 当作完整能力组合，
也不替代 MSRV、Linux/macOS 原生平台、工具链、all-features、高阶组合或预算
验收；相关缺口和原始命令哈希见 [#125](issues/125-freeze-feature-matrix-inventory.md)。

本轮 #126 首次执行了声明的 MSRV 1.97：Cargo 1.97 原先因 `opi-ai` 在
workspace 继承后覆盖 `opi-telemetry` 的 `default-features` 而无法解析，已将
该空默认 feature 的关闭移到 workspace 依赖声明，保持语义不变。修复后，1.97
的 core 与 builtins profile 均通过 Windows MSVC 检查，并在同一工具链上通过
OpenAI Chat response-observer 回归；当前 stable 1.99.1 对 Windows、Linux、macOS
目标的 core profile 交叉检查也通过。随后在 WSL2 Ubuntu 24.04.4 和 Windows
10.0.22631.6199 上补齐原生 platform delivery、auth-process、Chat observer
和四个当前声明 feature profile；两边的 all-features library/target check 也通过。
macOS 因用户确认没有环境而暂缓，完整 capability-granular V、定量预算和 #94
真实 Deferred 仍开放；原始 MSRV 失败、原生 runner 与缓存回收证据见
[#126](issues/126-msrv-and-platform-core-proof.md)。实际归档包
消费已跑通 builtins/no-tracing profile；defaults profile 在 Windows linker 写入
PDB 时因 E: 容量不足阻断；随后在带 ownership marker 的 C: target 上重跑，
两个 profile 的 metadata、clippy 和 response-observer contract test 均通过。
前两次 E: 环境失败 receipt 仍保留，不能当作契约回归。

本轮 #127 将第一阶段的历史基线与当前工作树验证明确分开。旧的
`phase1-baseline.json`、`phase1-checks.json`、契约索引和原始 source inventory
作为历史回执保留，本轮不重写；`verify_phase1.py --mode current` 现在对单独捕获的当前 manifest
和 raw source inventory 做文件集合、内容哈希、解析器身份、语法/重导出状态及
声明唯一性校验。当前回执显示 671 个源/测试文件、1188 个上游导出、3620 个
Rust 声明和 5038 个测试声明均通过，历史模式仍会报告旧映射相对于当前重构的
漂移，并保留原始失败日志。新增的 capability V 矩阵把 `core` 与
`telemetry-tracing` 标为独立通过，把 `builtins` 内的协议、HTTP、云认证、
catalog、TLS/crypto、WebSocket/compression 和 provider OAuth/store 标为
`COARSE_ONLY`；声明的 all-features 仍只是 manifest V 通过。独立 capability
feature 拆分、有效 pair/高阶组合、原生 macOS、预算及 #94 真实 Deferred 仍开放。
详见 [#127](issues/127-refresh-phase1-snapshot-and-capability-v.md) 及其当前哈希
回执和 capability 矩阵。

本轮 #128 将嵌入式 Pi catalog 从粗粒度 `builtins` 中抽出为独立的
`catalog-bundled` feature；`builtins` 继续传递选择该 feature，因此默认配置和
已有 `features = ["builtins"]` 调用方保持不变。`Catalog::from_bytes`、自定义
source 和 Store 仍属于 core，`Catalog::builtin()` 与
`Catalog::builtin_bundle_bytes()` 只在 `catalog-bundled` 下提供。catalog-only
原生 Windows profile 的 11 个 artifact 测试和 warnings-denied Clippy 均通过，
builtins 兼容 profile 的 9 个 lifecycle 测试也通过；实际归档包消费在外部
caller 中只启用 `catalog-bundled`，metadata 未带网络/TLS/cloud/crypto/compression
后端，并从归档执行了 pinned catalog 加载，观察到 1608 个 chat model。原有
粗粒度 `builtins` 及其历史回执不重写；协议、auth、TLS、WebSocket、OAuth/store
等 capability 仍待后续拆分，macOS 和 #94 阻塞保持不变。详见
[#128](issues/128-split-catalog-bundled-feature.md) 以及新的 native/package
哈希回执和更新后的 [capability V 矩阵](evidence/phase8-capability-v-20261115-v2.json)。

本轮 #129 将共享 HTTP transport 从粗粒度 `builtins` 中抽出为独立的
`http-transport` feature。该 owner 包含公开 `HttpClient`/builder、代理和
重定向策略、共享 HTTP 错误分类以及 source-scoped TLS/crypto wiring；具体
provider protocol 和内置 auth 仍由 `builtins` 持有。`builtins` 传递选择
`http-transport`，所以默认配置和已有显式 builtins 调用方保持原有 surface。
transport-only 原生 Windows profile 的三个契约测试和 warnings-denied Clippy
均通过；默认与显式 builtins profile 的同一测试也通过。实际归档包外部调用方
只启用 `http-transport`，metadata 观察到 reqwest/rustls/AWS-LC transport，且
没有 cloud-auth codec、catalog 或 compression owner；从归档执行的
`HttpClient` pool/proxy/redirect 断言通过。仓库调用方审计确认 provider、
catalog/auth 共享路径继续通过 builtins 继承该 owner，无需迁移。协议 adapter、
cloud auth/OAuth-store、TLS backend 选择、WebSocket/compression、完整 V、macOS
和 #94 仍开放；旧 #128 矩阵及回执不重写。详见 [#129](issues/129-split-http-transport-owner.md)
以及新的 native/package/caller 哈希回执和 capability V 矩阵。

本轮 #130 将 provider protocol adapters 从粗粒度 `builtins` 中抽出为独立的
`protocol-adapters` feature。该 owner 包含 Anthropic、Azure/OpenAI、Bedrock、
Gemini、Mistral、OpenRouter、Pi Messages、Vertex 与 Codex Responses 的 wire
模块，以及协议专属的 HTTP/TLS、AWS signing/XML/query、P-256/SHA-1 和
WebSocket/compression 依赖；它传递选择 `http-transport`，但不选择
`catalog-bundled`。`builtins` 继续传递选择 catalog 与 protocol adapters，默认
和显式 builtins 调用方保持不变。中立 `auth::store_oauth` 与 credential value
留在 core；provider collection、内置 OAuth/AWS/ADC/Vertex credential factory
留在 builtins，因为它们注册内嵌 catalog 或平台 credential source。protocol-only
原生 profile、warnings-denied Clippy、实际归档外部 consumer 和调用方 inventory
均通过；旧 Pi 配对、catalog/HTTP 回执保留不改写。每 provider feature、TLS backend、
WebSocket/compression、provider OAuth/store、完整 V、macOS 和 #94 仍开放。详见
[#130](issues/130-split-provider-protocol-adapters.md) 及
`phase8-protocol-adapters-*` 证据。

本轮 #131 将 provider OAuth/store 从粗粒度 `builtins` 中抽出为独立的
`oauth-store` feature。该 owner 包含 SDK 的具体 provider OAuth flow 及其
Store-backed login/refresh bridge，并传递选择 `http-transport`，但不选择
内嵌 catalog、provider protocol adapters、AWS/ADC/Vertex credential-source
chain 或 provider collection。中立的 `OAuthProvider`、`OAuthCredential`、
`CredentialStore`、mutation 和 `auth::store_oauth` 合同继续留在 core，因此
最小配置中的自定义 OAuth/store 调用方不变；provider collection、
`default_oauth_flow` 与平台 credential factory 继续要求 `builtins`。新的
Windows native profile、warnings-denied Clippy、实际归档包外部 consumer 和
154 个 Rust 文件的调用方 inventory 均通过；旧 Pi、adapter/observer、catalog、
HTTP 与 protocol 回执保留不改写。provider OAuth 各 flow、TLS backend、
WebSocket/compression、完整 V、macOS 和 #94 仍开放。详见 [#131](issues/131-split-provider-oauth-store.md)
及 `phase8-oauth-store-*` 证据。

本轮 #132 将固定的 source-scoped TLS backend 从粗粒度 `http-transport` 中抽出为
独立的 `tls-backend` feature。该 owner 提供固定 rustls/AWS-LC/native-certs 实现与
公共 `opi_ai::tls` 配置模块，但不选择 reqwest、provider protocol、catalog 或内置
认证；`http-transport` 传递选择它，并保留历史 `opi_ai::http` TLS re-export，因此
HTTP 调用方和默认/builtins 行为保持兼容。Windows native TLS-only profile、
warnings-denied Clippy、HTTP compatibility、实际归档包外部 consumer 和 caller
inventory 均通过；旧 HTTP、protocol、OAuth/store、Pi 与平台回执保留不改写。
native-tls 替代实现、WebSocket/compression、cloud auth、完整 V、macOS 和 #94
仍开放。详见 [#132](issues/132-split-tls-backend.md) 及 `phase8-tls-backend-*`
证据。

本轮 #133 将 WebSocket/compression 从粗粒度 `builtins`/`protocol-adapters`
路径中抽出为独立的 `websocket-compression` owner。该 owner 包含公开的
`opi_ai::ws` Upgrade bridge、RFC 6455 frame/IO 与 permessage-deflate 实现，
并传递选择 `http-transport`、`tls-backend`、`libz-rs-sys` 和 `flate2`；它不
选择 provider protocol、catalog 或 cloud-auth。`protocol-adapters` 传递选择
该 owner，因此现有 Codex/provider WebSocket 调用方和默认/builtins surface
保持不变。Windows standalone profile、Upgrade/private codec 测试、warnings-
denied Clippy、实际归档包外部 consumer、665 个 Rust 文件的调用方 inventory，
以及受影响的 HTTP/protocol native/package 配对验证均通过；协议归档图已记录
四项 feature 的精确 union。冻结 Pi 1.5.0 的 WebSocket differential/material
回执保持不可变，未发现 wire 行为变化；首次 harness/standalone dead-code 失败
也保留为修复历史。cloud auth、per-provider protocol/source、完整 V、macOS
和 #94 仍开放。详见 [#133](issues/133-split-websocket-compression-owner.md)
及 `phase8-websocket-compression-*` 证据。

本轮 #134 将协议无关的 cloud-auth 能力从粗粒度 `builtins`/协议依赖中抽出为
独立 `cloud-auth` owner。该 owner 包含 AWS local source chain 与 STS/SSO、GCP
ADC、共享 SigV4 以及固定 query/XML/DPoP codec，并传递选择 `http-transport`/
`tls-backend`，但不选择 provider wire、catalog、provider collection 或
WebSocket compression。`protocol-adapters` 传递选择它，因此 Bedrock/Vertex
调用方和默认/builtins surface 保持不变；#134 当时保留的历史
`bedrock::sigv4` re-export 已由后续 #156 canonical-only 清理，当前调用方
直接使用 `auth::aws::sigv4`。Windows
native profile、auth 内部 59 个测试、warnings-denied Clippy、实际归档包外部
consumer、667 个 Rust 文件的调用方 inventory，以及 protocol adapter native/
Clippy/builtins HTTP 配对验证均通过；protocol 归档图已记录五项 feature 的精确
union。首次外部 consumer harness 因未声明 `secrecy` 的失败和修复后的 v2、
protocol v8 的磁盘空间失败均保留。per-provider protocol/source、细粒度
credential-source、完整 V、macOS 和 #94 仍开放。详见
[#134](issues/134-split-cloud-auth-owner.md) 及 `phase8-cloud-auth-*` 证据。

本轮 #135 将 `protocol-adapters` 兼容性组合拆为 per-provider protocol/source owner：
`provider-anthropic`、`provider-azure-openai`、`provider-bedrock`、`provider-gemini`、
`provider-mistral`、`provider-openai-chat`、`provider-openai-codex-responses`、
`provider-openai-responses`、`provider-openrouter`、`provider-pi-messages` 与
`provider-vertex`。`provider-protocol-common` 与 `provider-openrouter-usage` 只承载
共享实现；默认与 `builtins` 继续传递选择全部 provider owner，历史公开入口和调用方
行为保持不变。十一项 owner 的 Windows native check、独立 profile test、
warnings-denied Clippy 和实际归档外部 consumer 均通过；Vertex 首次包构建因 E 盘空间
不足的失败回执保留，随后单 owner 重跑通过。1238 个仓库文件的调用方 inventory
记录了 direct module、provider collection、测试和脚本映射；protocol-adapters 与
`builtins` HTTP compatibility/acceptance 也在当前候选上复核通过。冻结 Pi 1.5.0
配对差分、旧 protocol/cloud-auth 回执和历史授权缓存回收计划保持可追溯。单 owner
之间的完整 pair/high-order V、原生 macOS 和 #94 credential-source 细分仍开放。详见
[#135](issues/135-split-provider-protocol-source-owners.md) 及
`phase8-provider-protocol-*` 证据。

本轮 #136 补齐 per-provider owner 的组合 V：组合验证器对十一项 owner 的全部
55 个无序 pair，以及“全部 provider owner”和“全部 provider owner 加
`catalog-bundled`、`oauth-store`、`telemetry-tracing`”两个高阶组合，分别执行
原生 `cargo check`/provider profile test，并从实际 `.crate` 由 workspace 外
consumer 重复执行 metadata、build 和 run。每行保存传递 feature closure、调用
输出与哈希；冻结 Pi 1.5.0 配对差分和既有 provider/adapter/observer 回执保持
不可变，验证器不改变协议源码行为。native macOS 继续按用户确认记录为待环境项，
不以交叉编译代替；#94 credential-source 细分、预算和完整六门验收仍开放。详见
[#136](issues/136-verify-provider-owner-pairs.md) 及
`phase8-provider-protocol-matrix-*` 证据。

本轮 #137 补齐 shared-owner pair 的 capability V：cloud-auth 与
catalog-bundled 的组合，以及 websocket-compression 与 oauth-store 的
组合，分别通过当前 Windows native check/profile tests，并由实际 crate 的
workspace 外 consumer 完成 metadata、build、run。回执记录精确的
transport/TLS feature closure、所需 owner 包和禁止 owner 包；冻结 Pi 1.5.0
配对差分与既有 cloud-auth、catalog、WebSocket、OAuth/store 调用方回执保持
不可变。native macOS 继续按用户确认记录为待环境项，不以交叉编译代替；
#94 credential-source 细分、预算和完整六门验收仍开放。详见
issue #137 及 phase8-shared-owner-pairs-* 证据。

本轮 #138 在 #137 当前候选上重新封存 manifest-wide all-features 行：原生
Windows MSVC 的 `cargo check -p opi-ai --locked --offline --all-features
--all-targets` 通过，解析集合覆盖 builtins/catalog/cloud-auth、HTTP/TLS、
OAuth/store、WebSocket/compression、十一项 provider owner、共享协议和 tracing。
既有多 integration-test target 复用 `provider_protocol_profile.rs` 的 Cargo 警告
仍存在但不阻断编译；运行使用持久化标记 target，未删除缓存。原生 macOS 继续按
用户确认记录为待环境项，#94 真实 Deferred、定量预算和完整六门验收仍开放；旧
Pi 1.5.0、平台、provider matrix 与 #137 shared-owner 回执保持不变。详见
[#138](issues/138-revalidate-current-all-features.md) 及
`phase8-all-features-current-20261116-v1.json`、更新后的
`phase8-capability-v-20261116-v12.json`。

2026-11-16 [#139](issues/139-freeze-worktree-and-audit-static-contracts.md) 冻结当前共享工作树并审计 contract-index 的静态行。快照记录 HEAD、origin/main、全部 tracked/untracked 路径和当前字节 SHA-256，并区分 source、test、evidence、iteration-doc、tooling、manifest、other；当前 118 个 tracked 修改、2 个删除和 616 个 untracked 文件中，没有仅由 CRLF/CR 归一化造成的 tracked 变化。

审计发现 `consumers` 161 行（52 个生产调用方、4 个包消费方、3 个示例、102 个测试调用方）、`test_files` 266 行（183 个冻结 Pi TS 清单、83 个 Rust target/helper）、`reviewed_cases` 38 行和 `models_methods` 27 行仍带 static 状态；这表示映射或静态观察，不直接等同失败。所有路径仍存在，但 43 个记录文件哈希相对当前工作树漂移（9 个 consumer、34 个 Rust integration target）。旧哈希和历史 contract-index 保持不改，当前字节哈希进入新的冻结回执；下一项是把本次完整里程碑范围内的生产调用方、包消费、示例和 Rust target/helper 绑定到现有动态证据，再按适用性刷新 current index。详见 [#139](issues/139-freeze-worktree-and-audit-static-contracts.md) 与 `phase8-worktree-freeze-20261116-v1.json`。

2026-11-16 [#140](issues/140-bind-static-rows-to-dynamic-evidence.md) 按 #139 逐项绑定本次里程碑范围内的 52 个生产调用方、4 个包消费方、3 个示例和 83 个 Rust target/helper 到既有动态回执，并单独处理此前未被证据提及的 12 个 reviewed cases。52 个生产调用方均获得当前字节哈希的工作区动态输入绑定，3 个示例获得实际包消费绑定；4 个包消费方中 `consumer` 为精确模板哈希绑定，crypto/http/ws 三个模板仅获得能力族级实际包回执，不能宣称模板源码已直接执行。83 个 Rust 行中 73 个获得当前哈希绑定，10 个通过较新的 provider/lifecycle/native 回执保留当前哈希与旧聚合哈希的双重历史。12 个 reviewed cases 中 10 个有相关动态类比但仍保留 `static-reviewed-not-run`，两个 models-runtime 场景没有等价动态回执，均不提升为严格 Pi 等价。旧 contract-index、#139 冻结回执及所有历史证据保持不改；详见 `phase8-contract-bindings-20261116-v1.json`。

2026-11-16 [#141](issues/141-direct-package-consumers-and-models-runtime.md) 在 #140 的绑定审计之后直接执行当前的 `consumer-http`、`consumer-crypto` 和 `consumer-ws` 模板。HTTP 与 crypto 各自 ordinary/caller-union 两个 profile 均 6/6 通过；WS 的 ordinary、union-default、union-c-zlib 三个 profile 均 10/10 通过，覆盖实际 Models→Codex Upgrade 链、PMD/window8、尾部 Close 和主动/空闲清理。WS 通用 package runner 的私有/公开/clippy 层均通过，但在打包 rustdoc 的 `-D warnings` 阶段因公共 `tls` 文档引用私有 `node_trust` 而停止；该文档门槛作为独立缺口保留，未用改写候选源码掩盖。冻结 Pi 1.5.0 的 `models-runtime.test.ts` 中 CASE-REVIEW-09/10 通过精确名称过滤执行，2/2 通过，其余 41 项按计划跳过；两项从 related-only 提升为 `exact-upstream-dynamic`，但仍不宣称 Rust 实现等价。旧 #139/#140 哈希、contract-index 及历史回执保持不变；详见 `phase8-package-consumers-models-runtime-20261116-v1.json`。

2026-11-16 [#142](issues/142-repair-packaged-rustdoc-and-rerun-consumers.md) 修复打包 rustdoc 中公开 `tls` 文档链接私有 `node_trust` 的门槛：保留打包 Node CA 语义，仅移除非法公开链接，运行时代码和 feature 闭包不变。当前 `consumer-http`、`consumer-crypto` 和 `consumer-ws` 模板均由实际 `opi-ai-0.8.3` 归档在 workspace 外消费；HTTP/crypto 的 ordinary 与 caller-union 各 6/6 通过，WS 的 ordinary、union-default、union-c-zlib 各 10/10 通过，WS 私有/公开/clippy、打包 rustdoc 和三档消费者全部通过。冻结 Pi 1.5.0 Chat 终结差分以 sealed snapshot 和当前探针重跑，8 组配对三轮稳定，finish/usage、`[DONE]`/EOF、尾部错误和连接释放一致，仅保留既定取消块结束差异；精确 `models-runtime` 两项也从 sealed Pi snapshot 重跑为 2/2 通过、41 项按名称过滤跳过。旧 #139/#140/#141 回执、contract-index 和全部失败证据保持不变；详见新回执及 #142。

2026-11-16 针对 #94 真实 Deferred 阻断建立可执行的[#143 接入票](issues/143-connect-real-deferred-production-route.md)。该票把缺口拆成 provider adapter、生产注册/装配、一个可恢复的生产调用方、持久化/取消/重试策略和机器可验证证据五层；要求先冻结真实 provider/API/model 与认证环境，再以实际异步 job 的 submit/fetch/cancel 闭合 #94。Faux、`ScriptedDeferredBackend` 和现有差分夹具继续作为负例与回归，不计入真实接入；若没有可授权的真实 provider 或测试账户，保留 #94 的阻断，不降低验收标准。

2026-11-16 #143 已开始第一条实现切片：冻结 OpenAI Responses Background / `o3`，加入 `OpenAiResponsesDeferredApi` 的 submit/fetch/cancel、生产 `Models` 注册和 coding-agent 显式 submit/poll/cancel 调用方，并用 wiremock 覆盖三条端点。随后把 pending handle 接入现有 assistant-message session 持久化/恢复，让 agent-loop 在 Deferred 终端停在显式 poll/cancel 边界，并补上 ready poll 的 session settlement：pending 结果更新持久化 handle，ready 结果写入最终 assistant 并在重开时清除旧 pending。当前本地调用方策略也已固定：不自动 retry/resubmit；网络、超时、限流的 poll/cancel 失败保留原 handle 供显式重试，终态 poll 失败写入 `Error`，远程确认 cancel 写入 `Aborted`，均只结算一次。真实探针因 `OPENAI_API_KEY` 缺失而未发出网络请求，授权账户上的 live receipt 仍是未完成验收项。

2026-11-17 重新验证 #143 的本地生产边界：OpenAI Responses wire submit/fetch/cancel 1 项、coding-agent Deferred session 7 项、provider factory 注册 1 项、agent-loop pending 边界 1 项均通过。真实 OpenAI probe 以同一 `openai`/`openai-responses`/`o3` 配置重跑，`OPENAI_API_KEY` 仍不可用，submit/fetch/cancel 全部记录为 blocked，未发出网络请求；receipt 见
[`phase9-deferred-openai-probe-20261117.json`](evidence/phase9-deferred-openai-probe-20261117.json)。#143 继续保持 open，当前阻断是授权凭据缺失，不是本地 wire/lifecycle 回归。

2026-11-17 #157 继续执行 canonical-only 结构清理：`ApiMappedProvider` 被确认是
provider composition，而不是 wire adapter，因此从 `src/api_mapped.rs` 移至
`src/providers/mapped.rs`。`lib.rs` 不再声明旧根模块，也不再保留根级类型重导出；
provider collection、coding-agent、测试与 provider-auth 文档统一使用
`opi_ai::providers::mapped::{ApiMappedProvider, ApiMapError}`。mapped-provider
的 route/catalog 校验和 dispatch 行为保持不变；实际 `.crate` 的
protocol-adapters 外部 consumer 额外编译并运行该 canonical 路径。详见
[#157](issues/157-migrate-api-mapped-provider-to-providers.md)。

2026-11-17 #158 继续执行 canonical-only 结构清理：内置 provider 的注册、
catalog 装配、auth resolver 与 route composition 被确认属于 provider 组合模块，
因此将 `src/provider_collection.rs` 移至 `src/providers/collection.rs`。`lib.rs`
不再声明旧根模块，也不保留兼容 shim；coding-agent、测试、examples、README、
provider-auth 文档和 conformance 脚本统一使用 `opi_ai::providers::collection`。
注册、路由、惰性认证、catalog 以及 classifier/image 行为保持不变；实际 `.crate`
外部 consumer 从归档包解析 canonical collection 路径并运行三条受控 README 流程。
详见 [#158](issues/158-migrate-provider-collection-to-providers.md)。

2026-11-17 #159 继续执行 canonical-only 结构清理：`ProviderHeaders` 被确认是
跨 wire 复用的 header 合同，不属于某个 provider profile 或单一协议 adapter，
因此将 `src/provider_headers.rs` 移至 `src/api/shared/headers.rs`。`api::shared`
对该公共合同开放，协议专属 mapper 仍保持 crate-private；内部调用方统一使用
`api::shared::headers`，`lib.rs` 只保留精选的 `ProviderHeaders` facade，不保留
`provider_headers` 模块 shim。header 校验、保留名保护、合并顺序与 callback 输出
校验行为保持不变；实际 `.crate` 的 protocol-adapters 外部 consumer 直接编译并
运行 canonical shared-header 路径。详见
[#159](issues/159-migrate-provider-headers-to-api-shared.md)。

2026-11-17 #160 继续执行 canonical-only 结构清理：`provider_env` 只负责
解析调用方作用域与 ambient process environment，不拥有 provider profile、
endpoint、鉴权、代理或 wire 规则，因此从 crate 根迁移到
`api::shared::env`。`lib.rs` 删除旧根模块声明，Pi Messages adapter、公共
utility tests、CHANGELOG 与 conformance 统一使用 canonical path；实际归档包
external consumer 直接解析并运行该环境合同。详见
[#160](issues/160-migrate-provider-env-to-api-shared.md)。

2026-11-17 #161 继续执行 canonical-only 结构清理：`text_decode` 是多个
API wire adapter 共用的增量 UTF-8 解码实现，不拥有 provider identity、endpoint、
鉴权或 transport，因此从 crate 根迁移到 crate-private 的
`api::shared::text_decode`。所有协议 adapter 统一使用该 canonical seam，旧根
模块删除且不保留 shim；现有分片 UTF-8、非法序列和 EOF 行为测试随实现迁移，
实际归档包 protocol-adapters consumer 重新验证。详见
[#161](issues/161-migrate-text-decode-to-api-shared.md)。

2026-11-17 #162 继续执行 canonical-only 结构清理：`endpoint` 只负责把已
解析的 base URL 与 API path 进行确定性拼接，不拥有 provider identity、鉴权、
catalog 或 transport，因此从 crate 根迁移到 crate-private 的
`api::shared::endpoint`。OpenAI Chat、OpenAI Responses、Codex Responses 与
Mistral adapter 统一使用该 canonical seam，旧根模块删除且不保留 shim；归档包
protocol-adapters consumer 同时检查实际 canonical 文件布局并重新运行。
详见 [#162](issues/162-migrate-endpoint-to-api-shared.md)。

2026-11-17 [#163](issues/163-audit-root-module-ownership.md) 完成的是
`lib.rs` 活跃根模块的保留审计：`time`、`codex_resources` 和私有
`execution` 等根模块均有实际生产调用方依据，未发现仅因根目录位置就应迁入
`providers`/`api::shared` 的根实现。这一结果不等于内部职责已经解耦。

随后按第一次结构决策重新核查，`ChatApi` 与 `DeferredApi` 已归属
`api::contract`，chat adapter 也已不再从 `models` 获取这些 contract；但 adapter
仍导入具体 `crate::providers::*`，provider profile 仍保存部分 adapter-native options，
且 230 KB 的 `models.rs` 仍混合 registry、
auth/catalog、注册擦除、调用执行和 provider-specific thinking 规则。因此
结构阶段不能收口；[#164](issues/164-audit-unresolved-ownership-boundaries.md)
建立剩余 ownership/依赖清单，[#165](issues/165-extract-api-operation-contracts.md)
与 [#166](issues/166-extract-deferred-api-contracts.md) 已完成 operation contract
收口，[#167](issues/167-decouple-openai-chat-profile-options.md) 与
[#168](issues/168-decouple-openai-responses-profile-options.md) 已完成两条
profile/native-options 解耦，[#169](issues/169-decouple-openai-codex-profile-options.md)
又完成 Codex profile 的 native-options 与 API 依赖解耦；[#170](issues/170-decouple-anthropic-profile-options.md)
  完成 Anthropic profile 的 native-options 与 API 依赖解耦，并将纯
  `ProviderHeaders` 值下沉到 `types::headers`；[#171](issues/171-decouple-azure-openai-profile-options.md)
  完成 Azure profile 的 native-options、selected-record state 与 API 依赖解耦，
  并将 per-call dual-wire dispatch 收拢到 canonical wire adapter；[#175](issues/175-decouple-bedrock-profile-options.md)
  完成 Bedrock profile 的 native-options 解耦并保留 SigV4/retry 行为；[#176](issues/176-decouple-gemini-profile-options.md) 完成 Gemini profile 的 native-options 解耦并将参数化 body engine 提供给 Vertex；[#177](issues/177-decouple-vertex-profile-options.md) 完成 Vertex profile 的 native-options 解耦并关闭 G1；[#178](issues/178-extract-models-registry-owner.md) 完成 Models 第一条 RegistryOwner 拆分并将 registry/resource/gate 状态移出 facade；[#179](issues/179-extract-models-auth-owner.md) 完成第二条 AuthOwner 拆分并将 resolver/OAuth/credential-store 状态移出 facade；[#180](issues/180-extract-models-catalog-owner.md) 完成第三条 CatalogOwner 拆分并将 store/source/publisher/publication 状态移出 facade；[#181](issues/181-extract-models-operation-owner.md) 完成第四条 OperationOwner 拆分，PreparedProviderCall 明确作为 prepared execution owner；剩余结构工作、
adapter/profile route context 和 Models 内部拆分仍需有证据地推进。2026-11-18 的结构复核又发现 lower provider contract 反向依赖 `api::contract`，并且 `stream` 与 `execution` 仍互相持有实现类型；这些边界在目录迁移中没有被解决，不能把 #163 的根模块保留审计当作结构收敛。新的 [#172 结构收敛票](issues/172-freeze-layer-graph-and-lower-provider-contract.md) 将工作顺序改为先冻结依赖图、抽取无环 lower contract、分离 stream/execution owner，再继续剩余 profiles 和 Models 内部拆分。2026-11-18 已完成第一条 G3 切片，将 neutral `SimpleOptions` 与 Deferred backend contracts 移入 `provider::operation`，`api::contract` 保留公开重导出，`provider.rs` 已解除对 `api::*` 的代码依赖。随后完成 #173 Mistral profile native-options 解耦、#174 Pi Messages profile native-options 解耦、#175 Bedrock profile native-options 解耦、#176 Gemini profile native-options 解耦与 #177 Vertex profile native-options 解耦，profile native-state 从 18 个引用降至 0 个并关闭 G1。进度按 G0–G6 结构门槛、契约回归和外部包消费证据衡量，而不是按新增目录或 issue 数量衡量。完成这些边界后再回到
[#143](issues/143-connect-real-deferred-production-route.md)
及平台、capability-V、资源、性能和 fuzz 验收。

2026-11-18 的 [#182 Gemini route context](issues/182-invert-gemini-route-context.md)
将 Gemini route state 抽取到 lower `provider::routes::GeminiRouteContext`，
API adapter 直接依赖中性上下文，`providers::gemini::GeminiProvider` 保留为
canonical alias。Gemini fixture 71 项通过，`api_provider_edges` 从 14 降至
13，`api_route_context_edges=1`。其余 API/profile route context 继续按
[#172](issues/172-freeze-layer-graph-and-lower-provider-contract.md)推进。

紧接着的 [#183 Vertex route context](issues/183-invert-vertex-route-context.md)
将 Vertex project/location/endpoint/Express state 抽取到
`provider::routes::VertexRouteContext`，并复用 neutral Gemini body context。
Vertex fixture 73 项通过，`api_provider_edges=12`，
`api_route_context_edges=2`，`provider_to_provider_edges=3`。

随后 [#184 Bedrock route context](issues/184-invert-bedrock-route-context.md)
将 Bedrock endpoint/client/catalog/retry state 抽取到
`provider::routes::BedrockRouteContext`。Bedrock fixture 179 项通过，当前
`api_provider_edges=9`，`api_route_context_edges=3`，
`provider_to_provider_edges=3`。

随后 [#185 OpenAI Chat route context](issues/185-invert-openai-chat-route-context.md)
将 OpenAI-compatible route state 抽取到 `provider::routes::OpenAiRouteContext`。
OpenAI Chat fixture 116 项通过，当前 `api_provider_edges=8`，
`api_route_context_edges=4`，`provider_to_provider_edges=3`。

[#186 OpenAI route context sharing](issues/186-share-openai-route-context-across-profiles.md)
随后将 Mistral 与 OpenRouter factory 接到 `OpenAiRouteContext`，profile-to-profile
边从 3 降至 1，Azure dual-wire profile edge 保留到下一条 route context 切片。

随后 [#187 Mistral Conversations route context](issues/187-invert-mistral-conversations-route-context.md)
将 native Mistral route state 抽取到 `MistralRouteContext`。Mistral fixture
55 项通过，当前 `api_provider_edges=7`，`api_route_context_edges=5`，
`provider_to_provider_edges=1`。

随后 [#191 Azure route context](issues/191-invert-azure-route-context.md)
将 Azure dual-wire route state 抽取到 `AzureRouteContext`，并复用 neutral
Chat/Responses contexts。Azure fixture 46 项通过，当前
`api_provider_edges=2`，`api_route_context_edges=9`，
`provider_to_provider_edges=0`。

随后 [#193 stream/execution owner gate](issues/193-close-stream-execution-owner-gate.md)
将兼容 re-export 与实现依赖分开计量。execution lifecycle 41 项通过，G4
关闭，当前唯一结构 violation 是 Models protocol branches。

随后 [#188 Pi Messages route context](issues/188-invert-pi-messages-route-context.md)
将 Radius route state 抽取到 `PiMessagesRouteContext`。Pi Messages chat
回归 42 项通过，当前 `api_provider_edges=6`，`api_route_context_edges=6`，
`provider_to_provider_edges=1`。

随后 [#189 Anthropic route context](issues/189-invert-anthropic-route-context.md)
将 Anthropic Messages route state 与 federation cache 抽取到
`AnthropicRouteContext`。Anthropic fixture 155 项通过，当前
`api_provider_edges=5`，`api_route_context_edges=7`，
`provider_to_provider_edges=1`。

随后 [#190 OpenAI Responses route context](issues/190-invert-openai-responses-route-context.md)
将 Responses route state 抽取到 `OpenAiResponsesRouteContext`。Responses
fixture 84 项通过，当前 `api_provider_edges=4`，`api_route_context_edges=8`，
`provider_to_provider_edges=1`。

随后 [#192 Codex route context](issues/192-invert-codex-route-context.md)
将 Codex route state 抽取到 `OpenAiCodexRouteContext`，主 wire 与 native
subadapter 共享 neutral context。Codex fixture 87 项通过，当前
`api_provider_edges=0`，`api_route_context_edges=11`，
`provider_to_provider_edges=0`。

随后 [#194 neutral thinking preparation](issues/194-move-neutral-thinking-preparation-to-provider.md)
将 Pi neutral thinking policy 下沉到 `Provider::prepare_simple_request`。
Pi chat 回归 42 项通过，`models_protocol_branches` 从 8 降至 7。

随后 [#195 authenticated model admission](issues/195-move-auth-model-admission-to-provider.md)
将 `responses_only` model admission 下沉到 Provider contract。auth contract
108 项与 OAuth store 10 项通过，`models_protocol_branches` 从 7 降至 6。

随后 [#196 Pi preparation policy](issues/196-move-pi-preparation-policy-to-provider.md)
将 Pi shared-preparation bypass 下沉到 Provider/ChatApi capability。Pi chat
回归 42 项通过，`models_protocol_branches` 从 6 降至 5。

随后 [#197 thinking budget policy](issues/197-move-thinking-budget-policy-to-provider.md)
将 Anthropic/Bedrock thinking budget policy 下沉到 Provider capability。
Anthropic 155 项与 Bedrock 179 项通过，`models_protocol_branches` 从 5 降至 2。

随后 [#198 Models wire fallback branches](issues/198-remove-models-wire-fallback-branches.md)
将最终 wire fallback identity 下沉到 Provider capability。builtins check 与
lib 374 项通过，`models_protocol_branches=0`，结构报告 violations 为空，G5
关闭。

随后 [#199 composition proof](issues/199-close-composition-proof.md) 通过实际
`.crate` 的 workspace-external provider protocol consumer，覆盖 11 个 provider
owners 与 catalog/OAuth store/telemetry support union，G6 关闭。

随后 [#200 registry owner module](issues/200-extract-models-registry-owner-module.md)
将 `RegistryOwner` 从 `models.rs` 迁移到独立的
`models/registry_owner.rs`，并把结构报告拆为 owner 总数、独立 owner 文件数和
facade 中剩余定义数。行为回归保持 374 项 lib 测试通过，开始推进 Models 的物理
模块拆分。


随后 [#201 auth owner module](issues/201-extract-models-auth-owner-module.md)
将 `AuthOwner` 从 `models.rs` 迁移到独立的 `models/auth_owner.rs`。374 项 lib
测试与结构检查通过，物理 owner 文件数达到 2，facade 中剩余 owner 定义降至 2。


随后 [#202 catalog owner module](issues/202-extract-models-catalog-owner-module.md)
将 `CatalogOwner` 从 `models.rs` 迁移到独立的 `models/catalog_owner.rs`。catalog
personalization 5 项回归通过，物理 owner 文件数达到 3，facade 中剩余 owner 定义降至 1。

## 规划交接记录（历史）

本轮已完成三项研究、八项人机决策及本方案，共十一项决策票；所有回答均保留在各自票中。研究主要为静态源码/文档核查，不能当作原版运行、包消费或性能结果。

规划交接时只改动规划文档及 glossary 中 Model Request 的执行/观察生命周期定义；当时产品测试影响为 none。没有生产源码、Cargo、产品 README 或测试的重构，没有构建、提交、推送、发布或付费调用。

规划交接检查：

- `python scripts/opi-doc-check.py`：PASS。
- `git diff --check`：PASS；新增规划文件另外检查行尾空白。
- 规划与阅读快照的 177 个本地链接：PASS；十一张决策票均有 Answer，地图各索引一次，阻塞关系无环。
- 工作树核对：HEAD 与上述基线一致；committed=none-for-this-task，staged=none，unstaged 为任务中的 docs/GLOSSARY.md，untracked 为本迭代目录。先前各轮留下的同任务文档继续保留，没有其他产品变更。

这些检查只证明规划文件的一致性。实现、TS/Rust 差分、外部包消费、协议、资源、性能和真实服务验收均未运行。

第一阶段的实际观察、检查和剩余材料缺口见[第一阶段报告](phase1.md)，不以以上历史规划检查代替实施验证。后续从“值类型与基础边界”切入，复核消费者和相关源码是否漂移，优先保留已定位的生命周期反例；仍缺的源数据只阻断其实际影响范围。

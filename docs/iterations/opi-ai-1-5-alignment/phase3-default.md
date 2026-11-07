# 第三阶段第三个切片：普通 chat 入口独立执行

日期：2026-11-07。实施票：[让普通 chat 入口独立执行](issues/06-default-chat-execution.md)。
本切片已完成：普通 chat 入口、相关消费者与实际外部包检查通过。
工作区全量回归仍有两项既有失败；第三阶段整体与 Pi 严格等价验收尚未完成。

## 行为与范围

`Models.stream / complete`、`stream_simple / complete_simple`、
`stream_direct / complete_direct` 六个普通 chat 入口现在在构造时冻结输入、
注册对象及认证计划，并在当前 Tokio runtime 调度独立执行者。消费句柄和结果
future 无须被 poll，setup 和源仍会推进；丢弃它们不隐式取消请求。

公共 `ChatCall` 包装已有 `StartedChatStream`。原状态机成为私有
`ChatCallDriver`，仍执行预认证校验、认证、路由和 canonical collector。
`start()` 取出原消费句柄，不产生第二个任务、队列或请求；`result()` 等待同一
结果；新增 `ChatCall::abort(&mut self)` 转发显式中止与 join。源 panic、运行时
关闭失败、已完成 outcome 和中止后的 partial 沿用前一切片的合同。

这是一次 0.x 行为变化：普通入口构造需要 Tokio runtime，否则 panic；
Models 构造、内置注册及静态模型查询仍不需要 runtime、不读取外部认证来源。
普通调用未读事件使用无固定上限的 FIFO，保留但不消费的句柄会累积内存。
`result` 首次 poll 或消费者 Drop 释放积压；显式请求 token 取消不撤回已转发事件。
数值预算未冻结，不能据此声称内存或吞吐验收已完成。

资源 generation 仍在执行者首次调度时、任何认证 await 之前注册。清理 future
仍在自己的首次 poll 建立 cutoff。构造后立刻调用清理，若清理先于执行者运行，
该调用尚未登记，之后可取得新 generation；消费者有没有 poll 不决定其归属。
测试用认证/异步 payload 屏障验证 cutoff 两侧，不把一次 yield 当作注册确认。
普通消费者 Drop 不代替 `Models::cleanup_session_resources` 的资源释放确认。

显式 telemetry context 继续传到执行任务，省略/no-op 不隐式捕获消费者环境。
底层 carrier 在 poll/drop 中的环境恢复与跨线程验证仍保留；它们验证执行位置
的 carrier 合同，不要求后台任务采用等待者的 ambient dispatcher。

## 测试迁移

- add：四个普通入口执行测试，验证未 poll 的 stream/result 被丢弃后继续执行、
  无 runtime 构造失败、setup 前 runtime 退出与跨线程观察，以及非空 partial
  和用量在显式 abort、start 转换、最终结果之间保留。
- update：原完成入口冻结用例扩展为六入口不消费也 dispatch；预取消、旧绑定、
  单次认证与 Send/所有权断言保留。认证绑定测试给每个请求附加测试标识，按
  标识核验旧/新/自定义凭据，不再把并发请求到达顺序当作绑定身份。
- update：清理与 WebSocket 用例从消费者 poll/Drop 的同步假设改为执行进度、
  显式取消与清理确认；无 runtime 的内置构造用例继续验证注册与静态查询，
  旧“未消费普通调用零 I/O”断言由相反的公开生命周期测试替代。
- update：原 raw carrier 跨线程及 no-op 环境恢复测试明确通过 raw adapter /
  ChatStream 验证；普通 Models 测试验证后台执行和消费者 Drop 后的 context。
- update：Azure、OpenAI Chat、Responses 六项时长用例通过 raw 终态通知确认
  未消费时执行仍推进，然后延迟消费。保留 350 ms body 延迟、至少 300 ms
  测量时长，以及认证、响应头、response callback、延迟消费的排除验证和
  至少 500 ms 差值门槛；没有通过降低阈值修复失败。raw 通知不代表任务已 join
  或 provider 资源已释放，最终仍等待 `result()`。
- update/add：README 外部消费者报告升级为 `opi-readme-execution/2`，要求
  setup 零 I/O、读取消费者前已发生认证和一条本地请求；拒绝旧惰性报告、缺少
  执行和提前访问来源。保留三个逐字示例、最终业务输出与工具第二轮的验证。
- retain：其他原有断言与原始失败记录。delete：没有删除测试 target 或用例。

## 检查记录

环境为 Windows x86_64-pc-windows-msvc、rustc/cargo 1.99.1，沿用仓库共享
Cargo target。精确命令、返回码、日志哈希、候选文件哈希及失败修复关系见
[本轮检查证据](evidence/phase3-default-checks.json)。检查范围如下：

| 检查 | 实际结果 |
| --- | --- |
| `cargo test --workspace --exclude opi-eval --all-targets --no-fail-fast --offline -j4` | 1085.33 秒，退出 101：5 个 target、8 个用例失败，分类见下文 |
| execution_lifecycle / custom_provider_registration / resource_cleanup / telemetry_operations / codex_native_options | 分别 24 / 35 / 16 / 12 / 21 项通过；全量回归也通过对应 target |
| builtin_auth_ownership / provider_collection / openai_codex_responses | 195 项通过；WebSocket 后续加强执行屏障后随 Azure target 重跑，两 target 共 118 项通过 |
| Azure / OpenAI Chat / Responses 时长用例所属 target | 修复后分别 43 / 114 / 12 项通过；保留完整 target 验证 |
| workspace Clippy，排除 opi-eval、所有 target、`-D warnings` | 通过；最后修改的四个协议测试 target 补跑 Clippy 也通过 |
| workspace doctests / rustdoc，排除 opi-eval | 11 个文档测试通过；rustdoc 在 `-D warnings` 下通过 |
| README runner 的 Python 测试 | 15 项通过，包含旧 schema 与缺失执行证据的拒绝路径 |
| 实际打包产物的外部消费者 | 三个完整 README 示例通过，包含 streaming 和工具第二轮 |

全量回归运行中发现的六项时长失败均来自旧的“未 poll 就不认证”断言，已按
新公开合同迁移，所属三个 target 的最终重跑全部通过。原始失败记录保留，
没有把那次全量命令改记为成功；修复后没有再跑一次完整工作区。

仍未解决的两项为：

- `schema_engine::encoded_pointer_targets_preserve_selected_resource_identity`：
  相同输入得到 `Ok(false)`，期望 `Ok(true)`。此前第二阶段已记录此失败。
- `spec_ledger::live_ledger_spec_hash_matches_current_spec`：缺少
  `docs/opi-spec.md`，此前消费者切片已记录。

相关 schema 实现、测试及 coding-agent 的 spec_ledger 测试与第一阶段提交
相比未变，缺失文件在该基准也不存在；本轮的静态核对仅证明该归因仍适用，
不冒充新跑过旧版本。此前出现过的 Vertex 间歇失败本轮未复现，不能据此
声称已修复。

最终[外部包证据](evidence/phase3-default-package.json)来自实际 `.crate`
产物，而非 workspace 源码 patch。opi-ai 的 SHA-256 为
`752d12014db07cfed9b792d800e262fbee1a0db25189eb68248563c380bd5305`，
opi-telemetry 为
`28d145afef940b8ea9589a1473f6736ea6a96a7497bd11ba720b66d74e4a19c6`。
消费依赖图不含 agent、coding-agent、protocol 或 tui。三个流程构造 Models
和注册时均无认证读取或请求，消费前均已发送一条受控请求，最终输出和工具
第二轮均符合预期。包内库源码、资产、示例及双语 README 与当前工作树逐文件
一致（opi-ai 176 个、opi-telemetry 11 个）；随后时长测试改动不属于外部消费
构建输入。

独立只读复核未发现阻断问题，关于进度屏障、非空 partial、raw 注释和时长
观察范围的反馈已纳入。最终格式、文档及 diff 检查结果随检查证据保存。

## 尚未完成

这里调度的是 Rust 原生执行任务，不保证 setup 在构造返回前已运行。Pi 的
`lazyStream` 在调用中立即执行 setup 函数；精确同步步骤及异常时点仍需连同
AI observer/instrumentation 逐项对齐。此切片只证明无需消费者驱动的执行能力，
不把调度时点差异判为严格等价。

raw EventStream、`ChatStream::new(raw)` 和高级 prepared attempt 仍保留原有
手动 poll 合同；高级消费者可显式 start。普通 ChatCall 已迁移，不据此宣称全部
底层入口或所有操作已迁移。AI schema/计量、first-terminal 后 source/result 的
尾部、业务与遥测分别结算、observer 故障通道继续以上游实际观察为约束。
完整 README 重构、feature 配置、跨平台/MSRV/live、预算及六门验收仍待后续阶段。

本轮工作基点仍为 `814c95f7ce575fb9be3df6b9d04ff6e2942706c1`，开始时保留
89 个未提交路径。旧基准、映射和前序检查对象不改写，仅追加当前实施关系。
未提交、推送或发布。

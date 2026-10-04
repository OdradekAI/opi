# SDK 调用方候选 spec 独立复审

> **非规范性评审，2026-10-04；候选仍为 DRAFT。** 本页记录当前文档的证据核验与发现关闭，不是实施工单、进度账本或正式准入。它记录SDK第一轮稿的检查，保留旧评审的历史性；后续管理/资源复核见文末。本中文新材料没有既有英文 counterpart；英中候选 spec 同步，其他原研究未修改。

## 评审范围与结论

围绕开发者仅依赖 opi-ai 的调用方式、执行流程、返回结果检查前稿。主 Agent 整合三个有边界的独立研究/评审：pi SDK 事实与公开语义、opi 真实消费者/状态/双语对应、OAuth/default wire 与锁定依赖。子任务只拥有各自新增证据文件；spec/决策/验收修改顺序执行。

本轮把当前候选从44 AIR/38 V扩至53 AIR/45 V。D-30来自用户的独立 SDK 请求及技术推导；Q32–35的真实回答分别固化D-31–34。没有未回答的产品目标；仍需正式文档协调和正常实施准入，运行时及所有新增验收尚未执行。

调用一致性现在明确到 Models 的 native/simple 四入口、完整自定义模型、sessionless流程、共同结果收集、工具声明/手动循环、独立OAuth和provider默认身份。Rust保留owned输入/事件、typed Result与原因、有界背压及由宿主驱动的生命周期。覆盖差距和有意差异分别标明，不承诺全pi provider覆盖、接口已编译或外部项目已成功运行。

## 实质发现及关闭依据

| 发现 | 影响 | 修正及复核依据 |
|---|---|---|
| SDK日常入口/collection职责未固定 | 实施者可能再造一层同职责wrapper | AIR-045/050将实际collection状态与职责收敛为唯一Models，advanced prepare共用机制；SDK-01–08给真实迁移消费者 |
| durable/authorizer措辞过宽 | 普通SDK可能被迫依赖Agent/产品 | AIR-046/049及scope明确sessionless无持久化/执行前提；durable ack仅显式持久宿主，执行权限仅执行工具的host |
| 公共失败双通道与被动result | 丢失partial或容量64下死锁 | Q32/AIR-047统一Result，CallFailure携完整消息；result主动drain且已取终态缓存，V-40超过64事件 |
| Ok被误当执行许可，native/simple被误当相同payload | 错执行Length或不真实地宣称等价 | V-40/41区分消息终态与执行资格；仅有效输入/options等价时比结果，工具宿主仍校验授权 |
| OAuth机制虽有trait，却产品独占且协议滞后 | 只依赖opi-ai不能完整使用已有支持 | Q33/AIR-051/OA-01–06要求注入store/presenter完整flow，真实expiry/单5min/15s/atomic metadata，commit与远端副作用区分 |
| provider同名/重命名不等于默认wire | SDK选择与pi不同，Azure改名仍Chat | Q34/AIR-052/BP-01–04固定canonical IDs/真实Responses；Azure最终URL依据锁定SDK，不采信误导注释或constructor mock |
| Google/Vertex误套common thinking预算 | 合法调用发不同预算 | 拆独立model族表/custom/Google3 level；GenAI/Vertex flash-lite当前差异如实记录，不捏造统一设计意图 |
| 无条件1024 answer-room与Claude0回填 | 小ceiling下契约矛盾/不可能预算 | 精确F/B′公式；numericClaude零预算typed BudgetInsufficient，不复制Anthropic simple0与wire `||1024` 冲突；V-45固定1/1024/1025/2048 |
| 普通Responses/Azure/Codex gating合并 | 正确原生参数被省略或多发 | 附录/V-45逐adapter固定普通supports/Sign-in、Azure独立floor16、Codex legacy省略 |
| usage anchor/公式/舍入仍不精确 | 跨模型或多短块预算需猜 | Q35/AIR-053明确reported total优先/完整已知parent fallback、prefix时序、精确base/adapter公式及message级ceil；unknown与账单不混同 |
| header callback/raw observer/frames未选范围却似乎已对齐 | 一般hook被当作完整SDK覆盖 | 明确延后这三项，保留static headers/已确认payload和response扩展，给外部需求/脱敏/terminal语料重评触发 |

独立pi复审已核源码、closed formulas与OAuth/wire边界；英中复审核对53条要求的责任/证据/D/V及数值语义；OAuth/wire作者与独立评审核对OA/BP。最终舍入粒度由主Agent按estimate.ts:35–68原文收敛：system渲染体一次ceil、两工具数组分别ceil，其余整条合计后ceil。此最后文字精确化未声称运行差分测试。

## SDK 第一轮稿的检查与共享工作区

最终验证采用文档-only的worktree-only范围；没有待发布commit、branch或PR。以下为实际执行结果，不是未来V通过：

| 实际命令/检查 | 结果及边界 |
|---|---|
| `python scripts/opi-doc-check.py` | exit0，`opi documentation contracts: PASS`；最终正文修正后重跑 |
| `git diff --check` | exit2，11,720条诊断全部来自carried-in `docs/research/pi-1.0.0/source-manifest.json`；本次未编辑它，不能顺手修复共享工作 |
| 对36个本目录文件逐个 `git diff --no-index --check -- /dev/null <file>` | 无诊断；覆盖未跟踪文件，普通git diff不会覆盖它们 |
| PowerShell here-string → `python -X utf8 -`，标准库文档审计 | exit0；36个文件UTF-8/LF/终换行/无尾空白，564个本地Markdown链接目标存在，3个JSON可解析 |
| 同一内联Python的AIR/V/双语表核对 | 53唯一顺序AIR、45唯一V、完整AIR→V索引；每AIR责任/规范词/证据/D/V齐全。317行双语稿的AIR/D/V/58个规范词/inline-code逐行一致、687个数字按对应行multiset一致；独立语义复核另见上文 |
| `git rev-parse HEAD`、`git diff --cached --name-status`、`git diff --name-status`、`git ls-files --others --exclude-standard` | HEAD仍为`f72181a7aa0b4087a3b6a6dfd158e2b4c234bf73`，staged空；当前22个tracked unstaged均为carried-in，committed=none-for-this-task。保留其他untracked eval研究及两份coding-agent `.snap.new`；任务写入仅本目录 |
| 标准库SHA-256复核旧manifest | `7ba3764ae8921274ce78c6a8e20eeb5e75b949e8a57cc72438b2344345c0210f`，与已记录基线相同 |

当时双语候选与验收矩阵的SHA-256（下文继续复核后的稿不再由此证明）：

| 文件 | SHA-256 |
|---|---|
| spec-draft.md | `57062ae9e67e72eba911909d1be2022f62f1ff77c6ce0a0b5bf8565f52427568` |
| spec-draft.zh.md | `a6f28a15149cb6c53d7d722c422de8533c59c9cf6207d6db87d2eab3bcc13f03` |
| acceptance-matrix.md | `fc1b1de5063d0dae32697bec2001ae686d7f26ea2d61492095d838606b3e2058` |

内联审计没有创建治理配置或持久验证脚本；前一遍发现英中header的Q32重复/分布不一致后已同步，并重跑通过。机械检查不替代语义复核，也不证明SDK目标可编译或真实服务可用。以上检查记录写入后再次执行文档与全目录审计，确保最终报告本身不引入问题。

## 测试与证据边界

本轮测试影响 **none**：没有add/update/delete仓库测试，没有修改runtime/依赖/正式规范，也未运行pi/runtime/live测试。此前70个mock-only测试属于历史retain记录；V-01–45均是未来验收设计，不能追认新目标已通过。

没有新联网查询；pi基线仍为核验v1.0.0，tag后窗口及官方SDK归档身份见原证据。真实OAuth注册/账号/订阅和Azure远端服务接受、性能/费用、未选Rust依赖conformance未核验。暂不含完整MCP/codemode、真实remote deferred、其余operation adapters、新OpenAI ChatGPT OAuth/Mistral原生及动态header/rawobserver/frames。

## 继续复核：管理与资源契约

本节在上表hash状态之后追加，旧hash/check统计仅证明当时稿。新的独立场景审查发现：SDK Codex默认auto被限定产品；构造cache-only与zero-I/O冲突；六管理方法返回/副作用未分别固定；默认惰性环境来源不明确；公开session清理与owned调用存活关系缺失。主Agent按UX-11–15修正这些职责与场景；不创建通用cleanup框架。

另经已缓存锁定AWS源码确认Bedrock默认3总attempt，与其他已核helper的0额外不同。Q36已实际回答推荐方案，采用真实adapter默认/唯一owner/显式override，独立SDK默认与产品高层恢复区分。当前状态及最后检查以README为入口；前面的“无未答目标”和hash不得覆盖本轮发现。

## 本次继续复核的独立发现与关闭

Q36已实际回答推荐，D-35是已确认SDK/lifecycle目标的技术精确化，D-36记录新答复。本轮不增加AIR/V数量，53/45沿用稳定追踪ID并更新语义；旧SDK第一轮的317行/hash/check统计只适用于当时稿。

| 发现／独立来源 | 可观测影响 | 关闭依据 |
|---|---|---|
| SDK Codex Auto、管理返回、默认来源、构造cache与zero-I/O冲突；pi_contracts、opi_audit | 独立caller默认或管理副作用与pi不同，或不得不借产品装配 | AIR-034/036/046、Public SDK management annex、UX-11–14；显式cache restore和lazy默认来源，分别固定Option/Vec/RefreshReport/Credential/unit与typed失败 |
| public cleanup与owned资源关系；pi_contracts/pi_evolution | 同ID实例串扰、连接退出不可await、Models drop错误取消owned call | AIR-035、UX-15、D-35、V-29/42；实例namespace/owned guard/完成屏障，四类连接覆盖、有效未匹配no-op与失败effects明确，不复制pi同步全局helper |
| SDK-R01：prepared单attempt粒度及共享预算不闭合；pi_contracts评P1、opi_audit评P2 | 每次start重获N、双重retry，或attempt error提前终止整个call | AIR-003/007、retry annex、D-36/V-05；AttemptOutcome分层，单acquisition、同call累计budget/index/quota，退避及typed准入/封存/drop/UnknownEffect逐项固定 |
| SDK-R02：cleanup缺已poll准备中的call；opi_audit | cleanup返回后auth完成迟到建立连接 | 首次poll在任何await前原子登记generation；旧matching含prepare取消，不能晚改挂fresh代。V-29/42覆盖auth/transform屏障 |
| SDK-R03：cleanup反向cancel宿主共享token；opi_audit | 另一个Models和后续call也被取消，freshreuse失效 | 内部call取消状态、caller父token单向流入；cleanup只动自身namespace/旧代，不反向cancel，V-29共享父token反例 |
| SDK-R04：Bedrockquota作用域未定；opi_audit | Rust长期providerquota让前call耗尽影响后call | retry annex明确每logical call initial500，同call不reset；V-05连续两call与高级attempt隔离 |
| Bedrock忽略公共override、不可取消SDKtimer、Codex宽catch；pi_evolution | 参数无效/退避无法中止/400或401被重新发送 | Q36/D-36与三族分类/公式/原hint准入fixture；default真实对齐、错误纠正及AWS ambient/adaptive范围差距明示，未运行fixture |

pi_contracts在修正后只读复核确认原重要发现已关闭，管理/资源/双语未发现新重要矛盾；opi_audit确认四项已闭合并建议“未poll handle”限定logical-call而非旧PreparedCall的AttemptHandle，已就地修正。追加退役行明确Agent/OpiConfig/summary真实消费者、低高config分层、legacy键迁移错误及V-05/20/33退出；不声称当前尚未加入SDKretry的源码已有乘法故障。

逐条MUST仍通过AIR owner/trigger/失败/D/V映射检验；实现正文描述契约，源码事实定位在研究附录。所有新增类型名/接口/config是候选目标，不是编译/安装/发布成功；没有未答目标代替批准，没有任何runtime/测试/正式规范编辑。本轮新增证据只有固定本地源码及此前integrity核验缓存，未新联网或执行依赖。

## 继续修订稿的实际最终检查

以下检查针对 D-35/36 修订后的稿，覆盖上文317行/hash的历史状态；仍为文档-only、worktree-only，committed=none-for-this-task。

| 实际命令／操作 | 结果及限制 |
|---|---|
| `python scripts/opi-doc-check.py` | exit0，opi documentation contracts: PASS |
| `git diff --check`（标准库subprocess原命令、捕获完整诊断后归属统计） | exit2，11,720条尾空白全来自旧source-manifest.json；无其他诊断。原manifest hash不变，不清理共享既有问题 |
| 本目录36文件逐个 `git diff --no-index --check -- /dev/null <file>` | 全部无诊断，涵盖普通git diff漏掉的untracked文档 |
| PowerShell here-string → `python -X utf8 -` 内联文档审计 | exit0；36文件UTF-8/LF/终换行/无尾空白，565本地Markdown链接目标存在，3 JSON可解析 |
| 同一审计的双语/ID追踪检查 | 53顺序唯一AIR、45顺序唯一V及完整AIR→V索引；358行英中对应AIR/D/V/58规范词/inline-code逐行一致，795数字按对应行multiset一致，每AIR有owner/MUST/证据/D/V。语义复核另见独立评审，不把数字匹配当翻译正确性 |
| HEAD/index/worktree/untracked及旧manifest SHA-256读取 | HEAD仍f72181a7aa0b4087a3b6a6dfd158e2b4c234bf73，staged空；本轮开始22个tracked carried-in，检查时24个：另两份phase19 assurance remediation结果是本轮期间的共享其他改动，均未触碰。此前untracked coding-agent目录现不再列出，本任务未删除；另一个eval研究保留。任务只写本目录36 untracked文件中的研究文档 |

当前受检 spec/matrix 的 SHA-256：

| 文件 | SHA-256 |
|---|---|
| spec-draft.md | 2105b1042ef0c4f80051008624c8f6c454b74645fb3c6f83401ca2bcd0f399a0 |
| spec-draft.zh.md | 3d8e133b9c562db86c0edcb5458a16a447f6ed9e7a318288a81590a845c757d3 |
| acceptance-matrix.md | 52209d7640bc1ac372dccf4412bae82848fbeb7d0566c6c6ab9c5e9033db02df |
| 旧source-manifest.json（只读） | 7ba3764ae8921274ce78c6a8e20eeb5e75b949e8a57cc72438b2344345c0210f |

评审配置/summary真实消费者后，opi_audit再次只读确认新增退役行与cleanup返回/no-op/失败effects一致，无新增重要矛盾。所有重要发现已在本稿关闭；当前Q22–36及修订Q31均实际回答，没有阻断目标行为选择。本检查记录落盘后再运行同一文档检查、全目录/英中审计和diff归属检查，保证记录本身没有引入问题。测试影响none；本轮未运行runtime/live/pi suite或依赖，未来45组V仍未执行。正式协调/实施准入也未完成，DRAFT不转换ready-for-agent。

共享状态补记：检查记录落盘后的复查仍doc-check/全目录审计通过，spec/matrix hash未变；同次Git只读快照发现其他任务已将23个无关tracked改动staged，unstaged仅剩旧manifest。HEAD仍为上述值，本目录36文件仍untracked。本任务没有staging/unstaging/commit，保留这次并发变化；前表“staged空/24 unstaged”是较早检查时的真实状态，不能用作当前index断言。全仓库diff-check仍只报旧manifest，未对无关staged交付做评审或更改。

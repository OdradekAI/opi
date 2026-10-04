# 第二轮候选 spec 独立评审

> 非规范性研究评审；2026-10-03。评审对象为D-18之后、Q22–31及修订Q31已回答的44 AIR候选；不授予产品实施准入。旧spec-review/spec-evidence-review仍只证明其首轮版本。
> 中文新研究没有既有英文counterpart，spec本身双语同步。本文记录发现/修正/检查，不建立产品进度账本。

## 范围与评审分工

主Agent统一目标与证据，独立Agent分两轴只读审英稿：pi_contracts审transcript、hooks、持久化、重放、公开淘汰；pi_evolution审catalog、WS、thinking、恢复、deferred、操作默认和44 AIR→38 V。后者仅拥有acceptance-matrix/evolution-target-proposal，可按已确认目标修正其文件；其他修正由主Agent顺序完成。译者仅拥有中文spec，最终语义复核与翻译分开。

核对项包括证据支持、发布/实验/提案分类、opi合理设计保留、新旧职责退出、未确认目标、MUST责任与机械验收、migration范围及新Agent是否仍需猜测。源码/测试原文属于证据，未来验收不是测试通过。

## 发现及关闭依据

| ID / 严重度 | 发现 | 修正及复核 | 状态 |
|---|---|---|---|
| R2-01 / P1 | V-34旧“unsupported grammar拒绝”与修订Q31、AIR-040相反 | 改为unsupported→普通JSON工具＋投影；支持目标缺variant/非法schema提前失败。lark优先regex、增量/关联corpus及D-29同时纳入 | 已关闭，evolution独立复核 |
| R2-02 / P1 | accepted input在auth前ack，最终endpoint/transform投影auth后才形成；实际projection持久确认没有dispatch前屏障 | 同一事务owner在final freeze后首次dispatch前durable ack实际prefix/revision/projection；失败/取消禁发但保留先前accepted input，复用适当记录，不规定固定双append。正文/AIR-017/019/042及V-18/36含crash/cancel与capture-disabled | 已关闭，contracts只读复核无新冲突 |
| R2-03 / P2 | “invalid input不进hook”误否定保留的rejected raw只读观察 | V-15/35限定不进mutable post-validation阶段；只读raw观察收到原文，但mutable hook/authorizer/execute计数0，正常路径仍按normalize→validate→mutate→final authorize | 已关闭，验收同步 |
| R2-04 / P2 | 恢复验收只有已知account变化，漏凭据实变但account未知 | V-12/33加入identity-unverified零新dispatch/零身份探测；同scheme不是同账户证据，同账户稳定binding刷新允许 | 已关闭，evolution独立复核 |
| R2-05 / P2 | AIR-036漏EP/V已确认的connection-limit未开始最多一次重连 | 正文补独立有界分支，与missing-previous一次full恢复区分，ordinary WS→SSE仍在首有效provider事件前 | 已关闭，evolution只读复核 |
| R2-06 / P2 | 404/501下一次restore撤下在pi源码受bundle generation影响，候选若总称pi同义会误导 | 工程附录/V-28明确无generation也按typed tombstone撤下是目标一致性修正；当次restored overlay保留/transient cache保留 | 已关闭，事实/目标分开 |
| R2-07 / P2 | SDK专题还保留完整AWS链/image timeout未知和U-1–3待决，README仍只记录32 AIR/HOLD | 精确锁定npm补证与Q24/Q27–30映射更新；README/decisions/reassessment/operations/placement标明历史与当前44 AIR目标，不把旧PASS沿用到当前版本 | 已修正，由主Agent机械核对 |
| R2-08 / P2，主Agent最终复读 | AIR-005“一个attempt”可能被读成整个call只准尝试一次，终止/drop作用域也未明确 | 两版限定同时至多一个可执行attempt，终止/drop只禁止该attempt继续调度/输出；与Q9/V-05一致 | 已修正，未改变目标 |

AWS selected/scoped失败关闭与DefaultChain continue-eligible发现错误经独立核对，未混成“所有失败终止”。SKIP_AUTH dummy签名/ambient bearer已核验且明确不在首批typed目标。没有为了制造重构而删掉有状态机价值的registry/collection/typed auth；真实compat先替代proof再退重复入口。没有把pi的无界FIFO、module-global cache、错误泄漏、Nodeheaders后timer或实验路由当作目标。

## 最终版本与实际检查

独立双语复核逐段比较44 AIR、范围/DRAFT/authority、工程附录及replacement/exit表，未发现实质差异。MUST/SHOULD/MAY、代码span和证据-D-V列一致；counterpart链接按语言互指。最后AIR-005澄清另经只读复核，无取消/attempt契约冲突。

| 当前评审版本 | SHA-256 |
|---|---|
| spec-draft.md | `2bf33e9be643b0226ac9709d1690cea06a5f2950e42843d545ef03ae133d7679` |
| spec-draft.zh.md | `2388c360d8d1b9b15aeefa766b226601a4746c8edf18168a80d1cdabd197512f` |

| 实际检查/命令 | 结果及限制 |
|---|---|
| `python scripts/opi-doc-check.py` | exit0，`opi documentation contracts: PASS`；不证明runtime行为。 |
| `git diff --check`，Python subprocess完整捕获/分类 | exit2；11,720个尾空白诊断全部来自carried-in `docs/research/pi-1.0.0/source-manifest.json`，无其他diagnostic/stderr。该文件hash与baseline相同，未修共享改动；全仓库检查不能写PASS。 |
| Python逐文件 `git diff --no-index --check -- nul <task-file>` | 本目录31文件无stdout/stderr诊断；新增文件exit1仅是与空文件有差异，0/1均接受，无whitespace错误。普通diff不覆盖这些未跟踪文件，故额外执行。 |
| PowerShell here-string→`python -`结构核对 | 31文件UTF-8/LF、无CR/尾空白，3JSON可解析；539个本地Markdown文件链接目标存在（不宣称验证全部fragment）。44 AIR/38 V及覆盖索引完整，每AIR有owner、MUST、D/V；双语逐行数字/规范词/inline-code计数及AIR引用列一致。只比较总量不能证明语义，另有独立逐段复核。 |
| Git HEAD/staged/track inventory＋manifest SHA-256复核 | HEAD仍`f72181a7aa0b4087a3b6a6dfd158e2b4c234bf73`；staged为空，21条tracked unstaged清单与baseline相同；manifest SHA-256仍`7ba3764ae8921274ce78c6a8e20eeb5e75b949e8a57cc72438b2344345c0210f`。本轮开始已有的untracked coding-agent snapshot.new及独立eval研究保留。 |

机械核对最初按整文数字出现顺序比较，因中文“UTF-8”与64 KiB的正常词序变化报差；改为对应段/表行的数字计数并独立语义核对后通过。不是改变预算或归一化行为差异。实际runtime测试没有重跑；此前70测试/探针仅保留其原证明边界。检查完成不等于44 AIR运行时验收通过。

## 结论边界

所有用户目标选择已实际回答；未发现需要再让用户决定的产品范围/架构取舍。候选保持DRAFT、非规范性，正式文档协调及implementation admission仍未进行。future Rust依赖conformance、38组新验收、live/性能/账单/外部embedder均未验证；真实remote deferred及完整MCP/codemode等明确不在本批。测试影响none；未修改/新增/删除仓库测试。

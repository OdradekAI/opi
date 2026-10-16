# Chord：类型化 Host、对象与热重载

分析日期：2026-10-16。本地 Pi 1.2.0 静态分析；未安装、执行插件、启动 hosts、构建或运行测试。README、CHANGELOG、261 行 spec、750 行 guide、909 行 tutorial 与 delta README 均全文阅读；关键主张与实现/测试交叉核查，没有逐行审计全部示例实现。[审读记录](../documentation-reading.json)。

## 本版定位与公开面

1.2.0 的 Chord 是重大替换：原 service/facet runtime、bundler 及 remote-service helpers 被 typed host runtime 替代；`defineFacet` 等同名符号也换了契约。旧 pi-client/pi-protocol/pi-server 只服务旧 runtime，当前已删除、不再发布。Context/delta/replicatedState 的独立机制保留，不能把旧报告的服务协议、bundle 校验或 PLANNING 工作包直接套到本版。[CHANGELOG:7](../../../../.repo/pi-1.2.0/packages/chord/CHANGELOG.md#L7)、[exports](../../../../.repo/pi-1.2.0/packages/chord/src/index.ts#L1)。

| 入口 | 主要职责 |
|---|---|
| 根 | TypeBox local/remote contracts、method/opaque、cell/derivation/listener/resource、createHost/facet/registry、links/errors、JSON/replicatedState |
| `/context` | 显式 cancellation 与 invocation values；与对象方法 Ctx 不同 |
| `/delta` | immutable tracker、draft/prepare/adopt、diff、apply 与 wire codec |
| `/node` | child host manager、fork driver/control、processLock、nodeLoader |

生产依赖仅 TypeBox 1.3.27；不依赖其他 Pi 包，原 esbuild 已移除。ESM/Node >=22.19.0，核心与 portable state 可在浏览器；Node manager/loader 有独立入口。ws 8.21.0 仅 devDependency，WebSocket adapter 依 structural interface，不替宿主完成认证或监听配置。[manifest](../../../../.repo/pi-1.2.0/packages/chord/package.json#L1)、[node.ts](../../../../.repo/pi-1.2.0/packages/chord/src/node.ts#L1)。

当前真实包调用方主要是 Durable：Context、JSON、delta tracker 与 replicatedState；env 也使用 Context。新 hosts/remote/registry 的现存展示集中在本包测试与文档；没有发现默认 coding-agent import Chord，也没有提供 Durable RegistryReader bridge。guide 的 Session host 示例说明如何装配，`durableRegistry(...)` 是应用要写的代码，不是本包已经接好的服务器。[Durable import](../../../../.repo/pi-1.2.0/packages/durable/src/session/session.ts#L1)、[spec registry 边界](../../../../.repo/pi-1.2.0/packages/chord/docs/spec.md#L208)、[guide 示例](../../../../.repo/pi-1.2.0/packages/chord/docs/guide.md#L690)。

## 对象链与权威

```text
contract(input + members/variants) → normalize/canonical identity + principalKey
  → host.get/ctx.get/self.get → hold + stable handle
  → one coalesced activation → synchronous factory declarations
  → await resources/held dependencies → live behavior at epoch
  → method(args, Ctx) / observable state → local or JSON link
```

record identity 为 type、规范化 input、principal key；record 持 cells/resources/holds/tasks/listeners，behavior 是一次 factory 得到的 per-epoch methods/state。handle 始终间接解析，loading/live/released/gone 生命周期与 remote state hydration 分开。remote contracts 校验参数、结果、snapshot/reset；local members 可以 opaque/native，local calls 不逐次 schema 校验。variants 缩小暴露面，但 factory/method 仍需按 principal 授权。[contracts/spec](../../../../.repo/pi-1.2.0/packages/chord/docs/spec.md#L25)、[record activation](../../../../.repo/pi-1.2.0/packages/chord/src/record.ts#L819)。

factory 必须同步且在 reload prepare 会重跑，slow/effect 放 resource.open、method、listener 或 task。declarations 以字段名身份跨 reload 复用：cells 保值；resources 看 rev/deps/exclusive；holds 看地址；listener 看 origin/select；tasks 以新代码 restart。activation 合并同地址请求，失败 unwind 所创建资源和 holds，下次 get 可重试；全部请求者提前释放则结果 dispose。[declarations](../../../../.repo/pi-1.2.0/packages/chord/docs/spec.md#L74)、[open/unwind](../../../../.repo/pi-1.2.0/packages/chord/src/record.ts#L905)。

最后 hold 与 invocation 结束后按 linger dispose；forced dispose 先让 handle gone、拒绝新 calls 并 abort calls，再逆序关资源、释放 holds。host.close/invalidatePrincipal 按 holders-first 波次清理，links 最后关闭，让资源 close 仍能使用依赖。进程消失后这些内存对象不恢复；资源可自己打开数据库，但 Chord 不是持久任务/会话引擎。[lifetime](../../../../.repo/pi-1.2.0/packages/chord/src/record.ts#L1226)、[host close](../../../../.repo/pi-1.2.0/packages/chord/src/host.ts#L936)。

startup readiness 是本版的显式契约：install/load 先同步安装全部 facets，再等 eager holds；ready 只等调用时快照，不是持续健康或 tasks 完成。worker 若 eager reporter 依赖尚未被父 host 接纳的 upstream，必须先连接、`awaitEager:false`、只等本地 bootstrap、宣布 control ready，再等全量 ready，避免父子互等。该两阶段 bootstrap 是文档/实现和测试一致的现行行为。[host.ts:273](../../../../.repo/pi-1.2.0/packages/chord/src/host.ts#L273)、[guide:271](../../../../.repo/pi-1.2.0/packages/chord/docs/guide.md#L271)、[readiness test](../../../../.repo/pi-1.2.0/packages/chord/test/readiness.test.ts#L197)。

## 热重载、版本与恢复

root 是短调用树，带 rootId/epoch；Durable phase、object task 等长期 code holders 每次 Chord call 开新 root。记录最多保留 current/previous behavior。compatible reload 不等旧 roots，旧树继续旧代码；state version bump 或 exclusive resource 替换才 gate：已进入的 roots 可再入，晚到旧 root 得 upgrade_conflict，默认 5 s 不排空则 abort/reopen gate。migration 在 private copy 校验；commit 同步安装新 epoch/tables/state，再启动 tasks/listeners；previous 退休后老 root 得 superseded。[gate](../../../../.repo/pi-1.2.0/packages/chord/src/reload.ts#L412)、[commit](../../../../.repo/pi-1.2.0/packages/chord/src/reload.ts#L514)。

contract version 保护调用者，state version 保护 cell 意义，resource rev 控制重开，三者不能混为一号。同版本 provider 必须 covers consumer；break bump 通过 `accepts[old]` adapter 服务旧消费者，或同批更新所有消费者。只改语义而 shape 不变仍需作者 bump，运行时无法检测。host 自己的 eager bootstrap hold 是生命周期 pin，在 commit 换到新 contract，不阻碍其 provider 升级，其他真实消费者仍做覆盖检查。[兼容规则](../../../../.repo/pi-1.2.0/packages/chord/docs/spec.md#L58)、[eager hold](../../../../.repo/pi-1.2.0/packages/chord/src/reload.ts#L393)。

多 host reload 由无 parent 的 coordinator prepare 本地及直接 children，收 votes 后本地先 commit，FIFO links 上发子 commit，再允许新 epoch callbacks 发请求。失败 vote/deadline 则 abort。decision 不落盘；server crash 后靠 fenced child reconnect/reconciliation。remote module list 只在内存记住，重启后可能空；当前不是多层递归参与，也不是分布式持久事务。[cluster](../../../../.repo/pi-1.2.0/packages/chord/src/cluster.ts#L39)、[reconcile](../../../../.repo/pi-1.2.0/packages/chord/src/cluster.ts#L113)、[规范限制](../../../../.repo/pi-1.2.0/packages/chord/docs/spec.md#L227)。

## Links、取消、状态与背压

Connection 校验每帧 schema，重复 ID/越权控制帧会关闭 link；accepting side 根据宿主 authenticator stamp principal，仅 parent 可转发 principal/control，浏览器不携 epoch/root。route authorize 在 resolve/spawn 前后各检查一次，invalidatePrincipal 关旧 relay 或重新 stamp。默认 ping 15 s、45 s silence 断连；lost 释放 holds、失败 pending calls，重新 hold 是新生命周期，没有 session resume，effect 可能已发生。[Connection](../../../../.repo/pi-1.2.0/packages/chord/src/connection.ts#L240)、[帧校验](../../../../.repo/pi-1.2.0/packages/chord/src/connection.ts#L381)、[principal 边界](../../../../.repo/pi-1.2.0/packages/chord/src/connection.ts#L490)。

replicatedState 有两条分明路径：mutable owner 通过 change/replace 创建 revision；source attachment 只发布外部已提交的 exact value/ops，snapshot+register 必须原子，cursor 连续。其公开异步 subscriber 每份独立串行，最多 100 pending complete-value deliveries，overflow 只保最新（保留未开始 hydration）；不 join/abort 已运行 callback。此计数不是 byte cap，也不同于 hosts 的 cell/readable 模型。[state contract](../../../../.repo/pi-1.2.0/packages/chord/src/state.ts#L14)、[README 订阅](../../../../.repo/pi-1.2.0/packages/chord/README.md#L124)。

Delta 的 prepare 只产生候选，adopt 才改变权威；draft placement 会 clone+检查 strict JSON，root transfer 则信任 alias-free/不可变所有权，不 freeze。immutable apply 可以共享旧 revision/op payload；mutable apply 消耗专属 batch。reserved prototype path 会拒绝，wire encoder/decoder 每条有序流独立字典，reset r 清字典；decode/apply 失败必须扔掉 replica/dictionary 再等完整 root。[delta 契约](../../../../.repo/pi-1.2.0/packages/chord/src/delta/README.md#L28)。

## 规划时效与静态风险

当前 spec 的 Known limitations 是现行约束，不是交付时间表。本次逐项核对重要方向：

| 方向/决策 | 状态与实现证据 |
|---|---|
| typed objects/handles、runtime contracts、holds、principals、two epochs | 已实现，根 exports、host/record/connection 与 reload tests 对应 |
| compatible reload、migration/exclusive gate、cluster votes | 已实现；reload.ts:412/514、cluster.ts:37；未执行行为测试 |
| 跨 host retirement | 部分：parent retire 与本地 roots 联合控制；worker 的旧 root 仍可能遇 superseded（spec:231） |
| durable commit decision、多层参与、重启后 module list 恢复 | 尚未具备；cluster 只枚举直接 downstream，remoteModules 内存态（cluster.ts:44、130） |
| link session resume | 未实现；断连释放，重连需新 holds；effect 结果未知（spec:235） |
| 拒绝认证的结构化告知 | 未实现；WebSocket 关闭后 client 仍表现 upstream lost（spec:236） |
| publication queue 隔离/异步调度 | 未实现；publish.ts 全进程同步 FIFO，慢 observer 可拖住其他 hosts |
| isolating/evicting Node loader | 明确 TODO；动态 import query 传播到插件目录，旧副本不卸载（loader.ts:3、18、55） |
| host 自动重启与跨平台 process lock | manager 提供 keepAlive/fatal/killTree/lock；不在核心自动恢复；非 Linux Unix stale socket race 仍记录为限制 |
| listener select captured-values 检测 | 只有 identity/source text 判断，捕获变量变动仍不可见（spec:241） |
| one state per object | 已否决；保留 named cells 与 derivations（spec:261） |
| Durable RegistryReader bridge | 应用职责，当前未附带；文档示例不是现成 glue（spec:208） |

Node loader 不隔离代码及缓存泄漏、全进程同步 notification、无跨 host hold cycle 检查、无全局 request/byte budget、关闭期间等待已断 remote link，都是重要资源/可用性边界。trusted local plugin 的副作用不能由 factory recording mode完全沙箱化；external effects 需宿主 policy。这些是静态判断/作者限制，不是本次实测问题。[loader](../../../../.repo/pi-1.2.0/packages/chord/src/loader.ts#L1)、[publication queue](../../../../.repo/pi-1.2.0/packages/chord/src/publish.ts#L1)、[spec limitations](../../../../.repo/pi-1.2.0/packages/chord/docs/spec.md#L227)。

## 测试预期与升级语义

测试静态覆盖 contract binding、principal/relay forgery、lifetime/resource unwind、readiness、reload gate/rollback/two-version、cluster commit ordering、replicated-state delivery 及 delta property/fuzz/retention。代表断言是 pinned root 两次旧行为、state migration/reset、gate deadline 不切换、失败 vote abort 所有 hosts、async subscribers 互不串行阻塞。测试未执行；测试影响 `none`。[reload tests](../../../../.repo/pi-1.2.0/packages/chord/test/reload.test.ts#L87)、[cluster tests](../../../../.repo/pi-1.2.0/packages/chord/test/cluster.test.ts#L78)、[delivery tests](../../../../.repo/pi-1.2.0/packages/chord/test/replicated-state-delivery.test.ts#L52)。

相对 1.0.4：新增 42、修改 18、删除 27、相同 19 文件；当前 79 文件、30 生产 TS/11195 行。主语义是替换 runtime 而保留独立 Context/delta/state 基础；新增 host/connection/cluster/reload/manager/wire 契约和现行三篇 host 文档，删除旧 bundler/service/planning 路径。delta append 优化的 1.4 s/11.3 s 是作者历史基准数字，本次未测。完整路径差异见[版本变化](../version-changes.md)；未取得完整旧新逐行 diff，旧正文不继承为当前 runtime 事实。

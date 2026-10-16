# Pi 1.3.0：telemetry 与交付输入的设计收敛

> 2026-10-16，非规范研究。按用户确认的“设计 READY”口径判断机制和合同能否确定；产品实施、公共 consumer、平台与供应商验收继续 OPEN。既有 V／AIR 身份不变，不建立实施账本。中文附件没有既有英文 counterpart；双语 spec 同步其合同。

## 1. 固定输入和实际范围

Pi 仍为 [固定 1.3.0 快照](pi-1.3.0-source-evidence.json)，实际 Node24.13.0 的文件 SHA256 为 `d14ba95cdce1ef7dc9ad3ac74949ca5db38b27378ee30f30a23cf26f9e875a11`。源文件与工具链 pin、Cargo 生成的独立 lock、可重放程序和原始输出在 [readiness-runtime](offline-probes/readiness-runtime/README.md)。Windows rustc1.99.1／Cargo1.99.1 实际编译和运行；不是用 MSRV1.97 编译的证明。

`固定输入 → verify: Node／Pi／工具链／lock SHA`；`隔离实验 → verify: build --locked --offline、两个程序真实退出码`；`负例 → verify: 错 Node／不存在的共享 target／Pi 源目录及 workspace 作 scratch 四项均在行为子进程前失败`。本实验没有网络准备，也没有更改产品 Cargo、源码、测试或 CI。独立研究 lock 不等于产品依赖图；实验采用既有 `E:/opi/cargo-targets/windows-direct`，没有新建 session target 或清理 cache。

## 2. Telemetry：保留小模块，确定私有安全包装机制

选择保持既有 `opi-ai::telemetry` 小模块、native tracing0.1.44／tracing-core0.1.36、显式 `Span + 该 Span 的 Dispatch`。不拆新 crate、不加 exporter、自有 schema DSL、隐式 ambient 捕获或 domain span。对 `Future`／`Stream` 的 poll 和整个 pinned inner 的 Drop 分别在同步 scope 执行；`Option<Pin<Box<T>>>` 支持先取出 owned box 再 scoped drop，不移动 pinned inner，不需要 unsafe。producer task 必须包住它自己的 poll／drop；只在 spawn 外进入 scope 不成立。disabled／omitted 使用 `Dispatch::none()`。

一手来源仍是 tracing [源码核查 §7](pi-1.3.0-backend-gap-audit.md#7-native-telemetry安全的polldrop方案仍需真实组装)所列公开 API，而非假设 `with_subscriber` 自动覆盖全部 Drop。新增 [main.rs](offline-probes/readiness-runtime/main.rs) 实际编译并运行七个场景：

- A 下建立 parent、在 B 下捕获／同步 scope，恢复后 marker 回 B，业务 closure 一次。
- Future 首次 Pending，跨 OS 线程继续 poll，原样返回 functional error；inner Drop 一次。
- Stream 的 Pending、带错误 item、EOF 和 Drop 不多 poll；未 poll 的 inner 也在 A 下 Drop。
- 显式 noop 与 disabled 在 B 下运行相同业务，A/B 均不新增观测。
- Tokio 跨 worker producer 保留原错误结果；已挂起 producer abort 的 inner destructor 归 A，业务工作没有重跑。

[原始输出](offline-probes/readiness-runtime/results/telemetry.stdout)中 A 的十三个 marker 全部归 parent1，B 恰有一个恢复 marker；七场景的断言均通过。`Carrier` 有编译时 Send＋Sync 检查，源码不含 unsafe。探针 recorder 对 trusted 非阻塞／不 panic 的 subscriber 提供观察，不提供任意宿主代码隔离。Noop 不序列化、Context／auth／wire 不携带 Dispatch、全入口 native/simple/advanced/image/classifier/deferred 传播仍是 V-46–48/58 的产品验收义务。

因此机制选择没有设计阻塞；**telemetry 产品覆盖 OPEN**，不把安全包装可行性当作 SDK 已接线，也不从外部 Span clone 推断业务 settle。

## 3. Catalog：确定产物合同，人工数据不补造真实目录

保留 owned complete records＋operation-qualified selector＋不可变 revision／原子 last-known publication。真实目录是可核来源的版本化输入；构造 Models 不联网、不探测账户或运行 generator。匹配输入缺失时不从测试／defaults／当前网络重建后冒称 Pi1.3.0 原目录。

固定 Pi 的 `scripts/model-data.ts`、`hydrate-model-catalog.ts`、`src/model-catalog.ts` 用 schema6、provider 集、`type:id` identity、结构摘要和逐文件摘要验证产物。新增 [catalog.mjs](offline-probes/readiness-runtime/catalog.mjs) 直接导入这些原 helper，对 TEMP 人工 42 provider ×3 操作记录实际执行七项观察：合法目录、同 id 按操作 flatten、篡改 hash、schema5、缺 provider、重复操作 id、相同输入重复 hydration。三项成功／四项拒绝均符合断言；确定性 manifest 摘要是 `adc95d5d23532a485b221f1d274855fa151c7c356be410f6f91368ba16f1ec90`。[原始结果](offline-probes/readiness-runtime/results/catalog.stdout)明确记录人工数据身份。

未来交付输入 MUST 同时固定：Pi 源身份、来源 artifact URL／本地来源与 package revision 映射、artifact digest、schema/minimum version、provider/operation identities、生成工具摘要；先在 staging 验完整再原子发布，失败保留旧 revision，不使已 prepare 的 call 改绑。来源 artifact 与本地快照无法建立关联时报告输入缺失，不接受同名版本作为替代。允许显式完整 custom record 继续使用；这不关闭 bundled catalog 交付。

产物格式／导入／校验／发布方案已确定，**真实 hydrated catalog 与公开 store／consumer 验收仍 OPEN**。取得 artifact 是未来交付输入门禁，不是要求现在实施联网 generator。

## 4. Cargo 与平台：确定可审交付合同

依赖全部由 root workspace 声明，publishable path 带版本，lockstep 元数据保持；本次研究 crate 不进入产品。准入后由 Cargo 生成产品 lock，不能拷研究 lock 或手工拼锁图。每个新选后端必须列直接／传递版本、registry checksum、features、target cfg、build.rs、native code、proc macro、license 与 MSRV；同名依赖并存、TLS provider 和默认功能合并需要反向 feature 树审查。默认禁止不必要 transport retry／schema resolver／remote discovery，不能以 default-features=false 推断整个闭包。

本实验 Cargo metadata 包括 19 个 package（含本地 root、可选／target 记录），不将 metadata 列表全部当作实际 build 路径。依赖 manifest 所声明 rust-version 不超过1.71，不等于实际 MSRV证明。Linux x86_64／macOS x86_64 对这一纯机制研究 crate 的 cargo check 已通过；未链接或运行 native 压缩／签名／TLS，不能代表产品或平台验收。

未来 V-20/42/49/56/58/59 在 Windows／macOS／Linux 分别以真实工具链、精确 graph／features、外部 consumer、打包资源和离线 conformance 记录命令；MSRV1.97 必须另跑，不能提升版本来绕失败。OpenSSL／zlib 等新增 native 选择即使设计已定，产品依赖准入、来源／编译工具／发布策略仍需原规范审查，不通过则阻止该实现合入，不能静默关闭特性或缩减能力。

交付／失败和平台义务已经确定；**产品 graph、MSRV、三平台和 package 验收 OPEN**。研究机制可行性与规范准入不互相代替。

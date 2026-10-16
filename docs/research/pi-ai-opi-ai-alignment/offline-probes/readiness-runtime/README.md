# Tracing／catalog 设计机制实验

本目录是研究资产，不加入 workspace。`main.rs` 是既有 owned Span＋源 Dispatch 方案的安全 Rust 可行性探针，不是 `opi_ai::TelemetryContext` 或产品实现。`catalog.mjs` 导入固定 Pi 原始 helper，只在显式 TEMP scratch 下处理人工记录，不修改 Pi 数据，也不证明真实目录已经取得。

固定版本与源摘要见 [pins.json](pins.json)，Cargo 图由 Cargo 离线生成并保存在 [Cargo.lock](Cargo.lock)。不安装、下载或启用外部服务。Windows 上使用既有外部 Cargo target，保留 incremental，不清理 cache。Node 使用直接执行文件 pin；没有 Bun 自动解析。

```powershell
python docs/research/pi-ai-opi-ai-alignment/offline-probes/readiness-runtime/run.py --pi .repo/pi-1.3.0 --node D:/nvm/v24.13.0/node.exe --scratch C:/Users/Luiz/AppData/Local/Temp/opi-pi130-readiness/runtime-replay --target E:/opi/cargo-targets/windows-direct
python docs/research/pi-ai-opi-ai-alignment/offline-probes/readiness-runtime/test_run.py
```

运行先核 Node、Pi 源、工具链和 lock 摘要，失败不得当作行为结果；scratch 不得是源目录或已存在的不同 crate。`--prepare-lock` 是作者准备入口，仅离线生成研究 lock，不用于固定输入重放。操作系统／编译器／Cargo 的常规环境白名单被传给编译子进程，provider secrets／proxy／外部 auth 配置不传入。本探针不读取账户或调用网络。

成功参考见 [commands.json](results/commands.json)、[tracing 原始结果](results/telemetry.stdout)、[目录原始结果](results/catalog.stdout)。七个 tracing 场景包含 Pending／跨线程／错误结果／Stream／未 poll Drop／noop／Tokio producer 与 abort Drop；A 的十三个 marker 均归原 parent，B 只有一个显式恢复 marker。这里的两个 worker 和输入大小都是探针输入，不是 SDK 默认或预算。

目录人工生成 42×3 条记录，对同一 id 的 chat／image／classifier、固定 manifest schema6、哈希篡改、错误 schema、缺 provider、重复操作 id 和确定性 hydration 作七项观察。`baseUrl` 为未访问的 loopback 地址。不能将这些模型、零 cost 或 fixture 数值拷为 Pi 目录事实。

Linux／macOS 的 `cargo check --target` 只证明这一研究 crate 的类型检查；Windows 实际运行与跨平台 native TLS／压缩／签名、产品 graph、MSRV1.97 或包消费者验收分开。实验没有不可信 subscriber 隔离、领域 span settlement 或 exporter 保证。

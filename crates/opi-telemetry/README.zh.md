# opi-telemetry

[English](README.md)

本 crate 跟随 current workspace version `0.8.3`。

面向 Rust 库的显式、中立遥测。可选择 no-op、内存记录器或可选 tracing 适配，
无需初始化全局 subscriber。默认构建不依赖 tracing、异步运行时、网络、AI、agent 或 CLI。

本 crate 随尚未发布的 workspace 引入。以下配置适用于已包含对应版本的 registry；
发布是独立步骤。也可以将实际 `.crate` 解包后作为 Cargo path 依赖使用，无需 Opi workspace。

```toml
[dependencies]
opi-telemetry = { version = "0.8.3", default-features = false }
```

## 记录一次操作

完整离线程序位于 [examples/record_request.rs](examples/record_request.rs)。
仓库内运行 `cargo run -p opi-telemetry --example record_request`；普通项目可将它复制为
`src/main.rs`，配合上方依赖配置。

```rust
use opi_telemetry::memory::Recorder;
use opi_telemetry::{Attributes, SpanOptions};

fn main() -> opi_telemetry::Result<()> {
    let recorder = Recorder::default();
    let context = recorder.context();
    let owner = context.start_span(SpanOptions::new("example.operation"))?;
    let observed = owner.span();
    observed.add_event("work.started", &Attributes::new())?;
    let answer = owner.context().in_scope(|| 42);
    observed.set_attributes(&[("answer".into(), answer.to_string().into())].into())?;
    owner.finish(None)?;
    assert_eq!(recorder.spans()[0].end_sequence, Some(0));
    Ok(())
}
```

## 所有权与失败

`start_span` 同步开始记录。执行任务保留唯一的 `SpanOwner`，操作结束时显式 `finish`。
可克隆的 `TelemetrySpan` 只负责观察；其 `context()` 提供子 span 与 metrics。
丢弃观察句柄或 context 不结算逻辑 span，也不取消业务。丢弃 instrument 包装的 future
会在捕获的作用域内丢弃 inner future；是否取消业务遵循该 future 自身的合同，包装层不结算
逻辑 span。执行层仍需将请求所有权与流消费分开。
未调用 `finish` 就丢弃 owner，会留下未结算记录。

`finish(None)` 保留最后一次显式状态，缺省为 `Ok`；`finish(Some(ErrorStatus { .. }))`
仅在没有显式状态时补充自动错误。显式状态优先。结算后的属性、事件、状态更新无效；
通过已结算 live context 创建的子 span 不记录，但保留 metrics 能力。已开始的子 span 独立结算。

后端错误通过 `TelemetryError` 返回并保留原始 source。factory、记录、结算均同步且可分别失败，
没有隐式重试、panic 捕获或业务错误转换。`finish` 失败也会消费 owner；失败不能算成功结算。
后端负责与并发记录的序列化。业务 callback/promise 的错误映射属于执行层。

`in_scope` 恰好执行一次 body，正常返回或 unwind 时恢复调用方作用域。
`instrument` 惰性包装 future，每次外层 poll 恰好 poll 一次内层，保留输出与 `Pending`，
也对 inner Drop 建立短作用域。guard 不跨 await。no-op 只关闭中立记录，不静音宿主 ambient tracing。

## Metrics 与 schema

`context.metrics()` 提供同步 counter、up/down counter 和 histogram factory。
`InstrumentDefinition` 携带名称、描述、单位及可选 histogram boundaries；factory 决定 kind。
counter 增量非负，测量值有限，histogram boundaries 有限且严格递增。这些是可信输入前提，
不是新增运行时校验。histogram 可以携带显式 context。

`schema` 保存可序列化描述：父关系、开始/结束属性、事件属性及 required、状态和 metrics。
属性元数据保留 values、examples、sensitive 与 cardinality。
`MetricDefinition::instrument(name)` 将元数据交给 factory；通用记录不强制校验 schema 名称或属性。
类型化埋点与领域 schema 归领域 crate 所有；此处没有 AI 名称或 token/cost 规则。

记录器返回脱离内部状态的快照：span 按开始顺序，metrics 按记录顺序，`end_sequence` 表示完成顺序。
记录保留至 recorder 释放，适用于测试和有界诊断，不是无限累积的生产 exporter。
当前没有 Store/query/render 或 wire propagation 接口。

## 可选 tracing 适配

启用 `features = ["tracing"]` 后使用 `TelemetryContext::from_span(host_span)`。
它冻结源 span 自身的 dispatcher，跨线程、poll 和 Drop 保持路由，不改用其他 ambient dispatcher。
disabled span 退化为中立 no-op。

中立子 span 使用静态名称 `opi.telemetry.span` 和动态 `telemetry.name` 字段；
事件与 metrics 通过 `telemetry.*` debug 字段携带元数据。
`telemetry.operation = "finish"` 事件标识逻辑完成；原生 tracing close 可能等待已进入的同步作用域结束。
这是 tracing 适配，不是完整 exporter。初始化、过滤、采样、脱敏、flush/shutdown 归宿主所有。

`opi-ai::telemetry` 保留旧载体路径的重导出。AI 操作当前传播 tracing 作用域；传入 recorder
尚不会自动生成 AI request span、schema 事件或用量 metrics；这些埋点属于后续迁移。

自定义后端实现 `backend` 合同。`tests/support` 的公共场景共同验证 no-op、内存和 tracing，
后端专项测试验证路由。没有为测试增加生产状态探针。

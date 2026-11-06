# Pi 默认能力、入口加载与 Rust 配置的区分

核查日期：2026-11-07。目标为用户指定的 D:/Luiz/Odradek/opi/.pi/pi-1.5.0；完整快照身份见 [baseline-comparison.json](baseline-comparison.json)。本次只读源码、README 与原测试；没有安装依赖、执行原版/Rust 测试或测量编译时间。本文保存核查事实与当时建议；用户随后确认的默认策略以[验收政策的 Answer](../decisions/10-acceptance-policy.md)为准。

## Pi 的实际设计

| 维度 | 已核实事实 | 目标快照中的来源 |
| --- | --- | --- |
| 安装依赖 | Anthropic、OpenAI、Google、AWS SDK 与代理等包是普通 dependencies，没有把默认安装限定成 core/Faux | packages/ai/package.json:69 |
| 根入口 | index 导出核心与 Faux，未导入真实 provider 工厂、内置 catalog 或 SDK 实现；仍导入 schema validation | packages/ai/src/index.ts:1；README.md:1724 |
| 轻量子入口 | models 入口用于只加载模型运行层，避免 schema validation、内置 catalog、SDK 实现 | packages/ai/README.md:1724；test/models-entry.test.ts:10 |
| 默认集合构造 | createModels 返回 ModelsImpl，provider Map 初始为空；没有自动注册全部 provider，也没有自动注册 Faux | packages/ai/src/models.ts:400、:410、:1028 |
| 全内置构造 | builtinModels 调用 createModels，再逐一 setProvider；导入 providers/all 会引入所有工厂与 catalog | packages/ai/src/providers/all.ts:1、:136、:184 |
| 单 provider | 具体工厂导入自身 catalog 与 lazy API 包装，不借此导入所有 provider | packages/ai/src/providers/anthropic.ts:1、:15、:78；api/anthropic-messages.lazy.ts:4 |
| 首次协议调用 | lazyApi 在 stream/streamSimple 内调用 loader；直接导入实际 API 则立即加载其实现。JS 打包是否 code splitting 影响最终 SDK 装载方式；Bedrock 还有专用模块注入路径 | packages/ai/src/api/lazy.ts:95；api/bedrock-converse-stream.lazy.ts:4；README.md:1728 |
| README 主路径 | Quick Start 使用 builtinModels；后文另讲单 provider 与较小 bundle 的用法 | packages/ai/README.md:108、:249、:267、:1712 |

上表源码路径相对 D:/Luiz/Odradek/opi/.pi/pi-1.5.0。原 test/lazy-module-load.test.ts:57 起分别断言根入口、构造所有内置 provider、compat 导入不加载 SDK，以及经 lazy wrapper 调用时加载对应 SDK；这是原测试所规定的目标，本次没有执行这些探针。目录 JSON 缺失的证据缺口仍存在。

这几层不能合成“Pi 默认只有核心”或“Pi 默认自动启用所有 provider”一句话。安装时完整依赖、导入时轻量、构造时显式选择、调用时延迟加载，是不同的设计选择。

## 对 Rust 方案的含义

Cargo features 控制条件编译和可选依赖；default-features=false 关闭默认集合，实际依赖图还会合并其他调用者启用的 features。它不是运行时注册或按需加载机制。[Cargo 官方 features 文档](https://doc.rust-lang.org/cargo/reference/features.html)。

建议分别设计和验证：

1. 编译能力：默认提供支持平台上的完整内置模型调用能力；有需要的调用者关闭默认配置，再选择最小核心或指定能力。
2. 运行时注册：普通 Models 构造保持空集合；单 provider 注册和显式全内置构造均保留，不引入隐藏全局 registry 初始化。
3. 资源创建：没有请求时不因具备编译能力就创建所有客户端、开始登录/刷新、联网或初始化 exporter。哪些副作用发生在构造、启动、首次调用等时点，按对应 Pi 入口契约分别映射。

“默认完整内置能力”指聊天、图像、分类及其内置协议/provider、目录与必要认证能力；它不等于无条件开启每个可选 feature、测试探针、具体 telemetry exporter、CLI 或所有平台后端。具体平台能力与宿主注入条件须公开，不能假装缺失能力已可调用。

默认较完整会增加 Rust 的依赖构建成本；只是不注册某 provider，并不会让已启用的依赖退出构建图。按需初始化可以减少运行时无关工作，但不能据此承诺 JS import/code splitting 的同等编译、包体积或二进制成本。实施时比较默认、单协议和最小配置的真实依赖图与成本。

## 对前一建议的修正

此前“默认核心＋Faux”是偏重最小依赖的成本取舍，既不是用户已确认的默认策略，也不是 Pi 的安装策略。已确认的是提供最小配置与独立接入能力，这不能推出默认必须最小。

基于用户希望对齐 Pi 且让第三方按文档接入的目标，推荐改为“默认完整内置能力、可选最小构建、显式运行时注册”。它与已确认的 README 单 provider 首例相容：例子只注册一个 provider，不要求安装时只能编译一个 provider。该推荐随后得到用户确认；没有修改 Cargo 或生产 API，也没有将未经测量的成本写成结论。

# 产品 CI 与平台验证

[English](ci.md)

## 日常检查

`ci.yml` 始终执行轻量范围/文档检查，并产生稳定的 `required` 结果。分支保护
应要求该结果，而不是把每个可选矩阵 job 永久设为必需。仓库规则的调整是独立的
维护者操作；修改 YAML 不会同步修改远端分支保护。

范围检查 job 执行 `scripts/opi-doc-check.py` 及其 Python 回归测试，并运行范围
选择器和 smoke wrapper 的测试。文档检查覆盖文件、本地链接、元数据和由源码
确定的数值；文字含义和运行时行为仍由对应的审查与测试验证。

范围选择器比较精确 Git 提交，重命名前后的路径均参与判断；无法读取历史时
拒绝放行。只有 Markdown/license/ignore 或开发 eval 的改动不会启动 Rust
产品构建。其他改动执行 Linux 产品 clippy、测试、feature-gated execution
验收、doctest/rustdoc，以及独立的已声明 MSRV 检查。格式检查仍覆盖 workspace，
成本很低。不再保留重复的 Phase 命名验收 job：默认 workspace 测试已执行这些程序。

产品 Cargo 检查使用 `--workspace --exclude opi-eval`。`opi-eval` 当前是未发布的
开发校准工具，不是产品运行时、CI 或发布前提。修改其逻辑仍需在本地执行专项
离线测试；真实试验需要独立范围、预算和授权。本 CI 策略不取消规范中独立评测
的长期方向。旧 eval 工作流字节保存在[静态开发契约](../../crates/opi-eval/development/workflow-contracts/README.zh.md)，
不再作为 Actions 入口。

## 单独选择原生平台证据

`platform-verify.yml` 同时支持手动 dispatch 和 reusable workflow 调用：

- `ref`：完整、小写的 40 字符 commit SHA，拒绝分支名。
- `linux`、`windows`、`macos`：独立布尔值，至少选择一个。
- `suite=smoke`：sandbox lib、auth-process 生命周期、native keyring、跨模式
  和恢复测试，加上 feature-gated execution 验收。
- `suite=full`：产品 workspace 的所有测试 target，加上 feature-gated
  execution 验收，不再重复执行 smoke 子集。

每个已选 runner 在 Actions summary 中记录 checkout 候选、平台与 suite。应
检查实际 checkout/input SHA，而不只看 workflow 定义所在分支的 SHA。缺失或
未选择的平台不算通过。Linux clippy、文档和 MSRV 保留在日常产品检查中；
full 平台运行补充原生回归，不再复制这些平台无关检查。

| 改动或交付 | 默认所需证据 |
|---|---|
| 文档/技能说明 | 文档与 diff 检查，不构建 Rust |
| 普通产品改动 | Linux 产品 CI |
| 平台敏感 patch | Linux CI 加受影响平台 smoke |
| 新 `0.N` 系列、1.0 后的新 major、重大里程碑、平台影响无法界定 | 发布候选上的 Linux/Windows/macOS 完整回归 |

选择器会为 manifests/lockfile、workflow、sandbox/TUI、execution 及已知
process/keyring/platform 路径增加 Windows/macOS smoke。路径选择只是辅助，
不是语义影响分析。共享进程、终端、路径、锁、native 依赖或构建行为在这些
路径之外变化时，拥有者仍需补选平台。版本大小决定默认档位，不构成验证豁免。

原 macOS 稳定化入口由此处选择 macOS 替代。外部架构编译移到发布构建，不再
每个 PR 增加六目标检查。原生三平台回归不代表所有外部架构都执行了测试。

最终候选上的 Linux 产品 CI 成功即提供完整 Linux 回归证据；大迭代通常只需
另派发 Windows/macOS 的 full suite，不重复跑 Linux。PowerShell 改动会选中
Windows，并在该原生环境运行开发 wrapper 的对等性测试。

## 发布与证据复用

patch 可以继续提供所有已声明平台的二进制下载，而不重复三平台完整回归。
`release.yml` 仍构建六个 Opi 目标（Windows ARM64 为 Tier 2），生成四个原生
sandbox 归档，并要求完整 sandbox 证据审计。构建、解包后 smoke 和校验和证明
交付字节；workspace 测试证明不同的行为，不能替代打包验证。

release 技能等待最终候选的必需 CI，只补缺少的原生证据，再推 tag。同一 tag
只有一个上传者；它有界等待 draft，不覆盖已有 assets。源码或本地制品策略
必须在推 tag 之前解决自动上传者冲突。公开 Git/Actions 与不可逆 registry
发布保留两个独立关卡。

本地工作使用现有[范围/检查选择契约](../../.agents/skills/_shared/references/change-scope-and-check-selection.md)。
只有源/构建输入、toolchain、features、target 和命令仍适用时才复用执行证明。
审计可以检查原始证明，但不继承实现者的结论。输入变化只使受影响证明失效。
不新增全局验证台账或缓存。

## 耗时与验证边界

历史成功 [CI 41300159581](https://github.com/OdradekAI/opi/actions/runs/41300159581)
约耗时 51 分 47 秒；Linux workspace 测试步骤耗时 51 分 29 秒，其中 eval
`native_driver` 占 38 分 39 秒。Windows/macOS 测试 job 为 32 分 38 秒/
15 分 43 秒。[v0.8.3 制品工作流](https://github.com/OdradekAI/opi/actions/runs/38840442512)
约耗时 6 分 22 秒。这些是基线观测，不是修改后的实测或延迟保证。本地发布
准备、registry 等待和消费者安装需单独计时，不能从 tag 构建时长推断。

修改 workflow 后应执行离线选择器/子进程测试、smoke wrapper 对等性测试、
文档检查及所属 Rust CI/release 拓扑测试程序。托管 runner/toolchain 行为和
端到端发布需要单独授权的远端执行；静态测试不能证明这些结果。

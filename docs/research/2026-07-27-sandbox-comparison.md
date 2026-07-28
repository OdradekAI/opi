# Opi sandbox 与 pi 0.80.6 对比

日期：2026-07-27

## 结论

当前 `opi` 已把 sandbox 做成内置的、只作用于本地 `bash` 子进程树的
defense-in-depth 机制：`strict` 默认关闭，但 L0 进程树生命周期处理始终启用；
Linux 使用 seccomp + Landlock，macOS 使用 `sandbox-exec`，Windows 只有 Job
Object L0。它不隔离 `opi` 自身、进程内文件/导航工具，也不给 adapter 加
strict 层（`docs/opi-spec.md:1850-1876`, `1940-1959`, `1995-2005`）。

`.repo/pi-0.80.6` 的核心产品明确**没有内置 sandbox**，工具和扩展都以 pi
进程的用户权限运行；强隔离交给容器、VM、OpenShell 或 Gondolin。仓库另有
一个可选示例扩展，用 `@anthropic-ai/sandbox-runtime` 0.0.26 替换 bash
backend，但这不是核心默认行为
（`.repo/pi-0.80.6/packages/coding-agent/docs/security.md:31-53`;
`.repo/pi-0.80.6/packages/coding-agent/docs/containerization.md:3-17`, `19-43`;
`.repo/pi-0.80.6/packages/coding-agent/examples/extensions/sandbox/package.json:11-18`）。

当前 `opi` 代码还有三个会改变用户理解的实现事实：

1. fail-open 是整包回退到 L0，不会保留仍可用的 strict 层；
2. `fs/network/syscalls = false` 没有传入最终 backend plan，显式 opt-out
   只影响 capability 查询，engaged 后仍可能施加完整固定策略；
3. Unix shell 正常退出时会 `l0_tree.disarm()`，后台 descendant 可以存活。

## 历史设计与当前实现

题目所指的 2026-07-11 文档是历史设计。当前规范明确说明，Linux L2 已按
后续研究收窄，后续纠正与 shipped code 优先于旧设计
（`docs/opi-spec.md:1840-1848`）。

旧设计写的是 `extrasafe`、Landlock “6.2+ net”，并声称
`socket/connect/sendto/recvfrom/accept/bind` 都可按 domain 过滤
（`docs/superpowers/specs/2026-07-11-phase15-safety-sandbox-design.md:184-204`,
`312-319`）。实际代码直接使用 `seccompiler` 与 `landlock`，classic
seccomp 只过滤标量参数可见的 `socket(domain, ...)`；TCP bind/connect
补强要求观测到 Landlock ABI 4
（`Cargo.toml:104-113`;
`crates/opi-coding-agent/src/sandbox/linux.rs:4-22`, `46-53`, `229-246`）。

旧设计还写 Windows 使用 `win32job`；实际是直接 `windows-sys` FFI
（`docs/superpowers/specs/2026-07-11-phase15-safety-sandbox-design.md:216-224`;
`crates/opi-coding-agent/src/tool/process_tree.rs:287-380`）。

## Opi 调用链

配置模型是 `mode = off|strict`、`require` 与三个可选层开关；默认
`off/require=false`，`None` 表示平台默认，`Some(false)` 表示显式 opt-out
（`crates/opi-coding-agent/src/config.rs:86-116`）。TOML 字段在
`config.rs:640-648`, `948-961` 合并，CLI 在
`crates/opi-coding-agent/src/cli.rs:124-133` 暴露 `--sandbox` 与
`--sandbox-require`。

生产调用链为：

```text
config/CLI
  -> sandbox::prepare_production
  -> CodingHarness::build_tools_with_sandbox
  -> LocalBashOperations::with_prepared
  -> BashOperations::exec
  -> L0 setup + optional strict Confinement
  -> spawn
```

对应入口见 `crates/opi-coding-agent/src/harness.rs:792-800`, `2051-2115`；
实际策略判定、Command 构造与 attach 见
`crates/opi-coding-agent/src/tool/operations.rs:497-645`。

`prepare` 把未显式关闭的层视为 requested，并归类为 `Engaged`、
`TemporarilyUnavailable` 或 `PermanentlyUnavailable`。`require=true` 遇到
任一 gap 时 spawn 前失败；否则 fail-open。永久 gap 启动时发一次
`opi.sandbox.unavailable`，临时 gap 每条命令发
`opi.sandbox.degraded`
（`crates/opi-coding-agent/src/sandbox.rs:284-347`;
`crates/opi-coding-agent/src/diagnostics.rs:1-67`）。

## Opi 平台矩阵

| 平台 | L0 | strict L1 FS | strict L2 network | strict L3 |
|---|---|---|---|---|
| Linux | process group | Landlock 写入仅 workspace + temp | seccomp 新 socket gate + ABI 4 Landlock TCP | seccomp danger blocklist |
| macOS | process group | `sandbox-exec` write deny-overlay | `sandbox-exec` `(deny network*)` | 不可用 |
| Windows | kill-on-close Job Object | 不可用 | 不可用 | 不可用 |

L0 在 Unix 调用 `process_group(0)`，Windows spawn 后创建并 attach Job Object；
timeout/cancel/drop 时 `TreeGuard` 终止整棵树
（`crates/opi-coding-agent/src/tool/process_tree.rs:56-73`, `115-274`,
`287-380`; `crates/opi-coding-agent/src/tool/operations.rs:684-725`）。

Linux seccomp 是 default-allow/match-deny：拒绝 `AF_INET/AF_INET6/AF_NETLINK`
新建 socket，保留 AF_UNIX；另拒绝固定的高风险 syscall，允许
`clone/unshare`（`crates/opi-coding-agent/src/sandbox/linux.rs:55-175`）。
Landlock 只处理写权限，允许 workspace/temp 写；ABI 4 起处理 TCP
bind/connect 且不给端口 allow rule
（`crates/opi-coding-agent/src/sandbox/linux.rs:220-246`, `299-341`）。
继承 socket、UDP/raw/NETLINK、已连接 socket 与 io_uring 均是显式 residual
（`crates/opi-coding-agent/src/sandbox/linux.rs:392-447`;
`docs/opi-spec.md:1904-1913`）。

macOS probe `sandbox-exec` 后，以
`sandbox-exec -p <profile> sh -c <command>` 重启；profile 为 allow-default、
根目录写 deny、workspace/temp 写 allow、network deny
（`crates/opi-coding-agent/src/sandbox/macos.rs:150-233`, `267-325`;
`crates/opi-coding-agent/src/tool/operations.rs:600-626`）。

Windows 把三个 strict 层都报告为永久不可用：默认降级到 L0，
`require=true` 则 spawn 前拒绝
（`crates/opi-coding-agent/src/sandbox/windows.rs:1-46`）。

## 三项源码级偏差

### 1. Fail-open 是 all-or-nothing

`prepare_production` 只有在整个 outcome 为 `StrictOutcome::Engaged` 时才
attach confinement（`crates/opi-coding-agent/src/sandbox.rs:368-397`）。
`LocalBashOperations` 对 `FailOpen` 明确返回 `confinement = None`
（`crates/opi-coding-agent/src/tool/operations.rs:556-567`）。

因此 Linux ABI 3 上只要 requested network 形成 gap，FS Landlock、seccomp
socket gate 与 L3 blocklist 都不会保留，实际回到 L0。这与规范中
“seccomp socket denial is always engaged”的表述不一致
（`docs/opi-spec.md:1887-1892`）。现有 fallback 测试也明确断言在 L0 执行，
没有 partial-retention 测试
（`crates/opi-coding-agent/tests/sandbox_strict.rs:86-193`）。

### 2. Layer toggles 没有进入 plan

resolver 会跳过 `Some(false)`，但 `StrictBackend::build_confinement` 只接收
workspace，不接收 config 或 selected layers
（`crates/opi-coding-agent/src/sandbox.rs:189-206`, `284-317`）。

Linux plan 总是构建同一套 seccomp + Landlock
（`crates/opi-coding-agent/src/sandbox/linux.rs:124-175`, `320-380`）；
macOS 更直接硬编码 `render_profile(..., true, true)`
（`crates/opi-coding-agent/src/sandbox/macos.rs:304-325`）。

结果是：显式关闭某层只会让它不参与 availability/gap 判定；只要剩余请求
使 outcome engaged，固定 plan 仍可能把关闭层也启用。三个 toggle 全为
`false` 时甚至没有任何 capability 查询，却会得到 `Engaged`，随后 Linux
或可用 macOS attach 完整 plan。测试只覆盖 config 解析和纯
`render_profile` toggle，没有覆盖 production plan 的 opt-out
（`crates/opi-coding-agent/tests/sandbox_config.rs:137-159`;
`crates/opi-coding-agent/tests/sandbox_strict.rs:607-668`, `1192-1209`）。

### 3. Unix clean-exit 会 disarm

direct child 正常 wait 成功后，执行路径调用 `l0_tree.disarm()`
（`crates/opi-coding-agent/src/tool/operations.rs:705-710`）。Unix 的 disarm
只是把保存的 process-group guard 替换为 `Disabled`，不发信号
（`crates/opi-coding-agent/src/tool/process_tree.rs:128-137`, `163-173`）。
因此 shell 正常退出而后台 descendant 仍运行时，L0 不会清理它。

现有 Unix L0 测试命令末尾带 `wait`，shell 一直存活到 timeout/cancel/drop，
所以没有覆盖 clean-exit survivor
（`crates/opi-coding-agent/tests/sandbox_l0.rs:376-470`）。若 descendant 继续
持有 stdout/stderr，外层 `tokio::join!` 还可能在 control 已完成、guard 已
disarm 后等待 drain；这是由 `operations.rs:647-723` 可推导出的风险，需
专项回归测试确认。

补充：Linux plan 构建 Landlock ruleset 时使用 `.ok()`；构建失败会静默变成
seccomp-only，而 seccomp/arch 构建返回 `None` 也不会把已计算的 `Engaged`
改成 fail-closed（`crates/opi-coding-agent/src/sandbox/linux.rs:360-380`;
`crates/opi-coding-agent/src/sandbox.rs:391-395`）。所以 `require=true` 目前
主要保证 capability preflight，不是所有 runtime attach 步骤的强保证。

## Pi 0.80.6 的处理方式

Pi 核心 `BashOperations` 在 Unix 用 detached process group，timeout/abort
调用 `killProcessTree`；Windows 用 `taskkill /F /T`
（`.repo/pi-0.80.6/packages/coding-agent/src/core/tools/bash.ts:52-148`;
`.repo/pi-0.80.6/packages/coding-agent/src/utils/shell.ts:176-225`）。这只是
生命周期处理，不是权限 sandbox。

Pi 有意允许 shell 退出后的 descendant 存活：wait helper 用 100ms
post-exit stdio idle grace，既接收后台 writer 的尾部输出，也不会被安静的
后台 sleeper 长期卡住；测试直接覆盖这两种情形
（`.repo/pi-0.80.6/packages/coding-agent/src/utils/child-process.ts:38-136`;
`.repo/pi-0.80.6/packages/coding-agent/test/suite/regressions/5303-bash-output-truncation.test.ts:34-78`）。

可选 sandbox 扩展默认启用（显式加载后），合并 global 与 project JSON，
提供 domain allow/deny、denyRead、allowWrite、denyWrite；初始化失败或平台
不是 macOS/Linux 时只是通知并回退本地 bash，没有 opi `require=true`
等价物
（`.repo/pi-0.80.6/packages/coding-agent/examples/extensions/sandbox/index.ts:55-130`,
`234-284`）。它通过自定义 `BashOperations` 同时替换 model bash 与
`user_bash`（同文件 `132-232`）。

强隔离方面，pi 推荐 whole-process Docker/OpenShell，或用 Gondolin 把全部
built-in tools 与 `!` 命令路由到 micro-VM
（`.repo/pi-0.80.6/packages/coding-agent/docs/containerization.md:9-43`,
`45-110`）。这比 opi 内置 strict 的 bash-only 边界覆盖更完整。

## 建议

若保留 per-layer 用户界面，应让 selected layers 进入 backend plan，并明确
partial fail-open 是保留可用层还是整包回退；相应增加 production-path
opt-out、all-false、ABI-3 partial fallback 与 Unix clean-exit descendant
测试。若不准备做组合策略，更简单且诚实的方案是把 strict 文档化为固定的
all-or-nothing bundle，移除当前误导性的逐层开关语义。

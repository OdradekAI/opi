# pi-env：远端文件、命令与终端

分析日期：2026-10-16。本地 Pi 1.2.0 静态分析；未安装、构建 Rust daemon、连接 SSH、运行测试或调用 provider。本包四份文本资料全文阅读，源码重点覆盖 Connection、环境适配、SSH、watch、list/terminal 及 daemon 调度和输出；没有逐行审计全部平台系统调用。[清单](../source-manifest.json)、[审读记录](../documentation-reading.json)。

## 职责、公开面与调用方

`@earendil-works/pi-env` 将 Durable 的 ExecutionEnv 放到另一台机器：文件/进程在 daemon 一侧，Harness、存储和模型凭据留在宿主。Rust `pi-env serve --token …` 使用 stdin/stdout 二进制帧；SSH 是部署和启动方式，不是模型/session 服务协议。本包不替代被删除的 Pi server/client/protocol。[README:3](../../../../.repo/pi-1.2.0/packages/env/README.md#L3)、[协议](../../../../.repo/pi-1.2.0/packages/env/docs/protocol.md#L1)。

根出口公开 `Connection`、RemoteError/lost 判定、RemoteExecutionEnv、RemoteWatcher、SSH host-key 与部署 helpers，新增 `nodePlatform`、`LocalExecutionEnv`、`openTerminal`/RemoteTerminal。LocalExecutionEnv 的普通操作继承 NodeExecutionEnv，仅 `listFiles` 通过本机 daemon，因此由调用者拥有并关闭共享 Connection。[index.ts](../../../../.repo/pi-1.2.0/packages/env/src/index.ts#L1)、[LocalExecutionEnv](../../../../.repo/pi-1.2.0/packages/env/src/list-files.ts#L74)。

npm 1.2.0/ESM、Node >=22.19.0，依赖 Chord 与 Durable `^1.2.0`。Chord 只提供 Context 类型，执行仍是本包自己的帧协议。实际生产装配搜索范围为 `packages` 与相关脚本；未发现 coding-agent 默认入口导入本包。可见调用方是包内 conformance/differential/SSH/terminal 测试和 README 的宿主装配，不能由导出存在推导出终端产品已提供 SSH 环境选择。[manifest](../../../../.repo/pi-1.2.0/packages/env/package.json#L1)、[RemoteExecutionEnv](../../../../.repo/pi-1.2.0/packages/env/src/remote-env.ts#L354)。

## 目录、原生依赖与构建发布

`src/connection.ts` 管 transport/pending/session；`remote-env.ts` 管 paths/readers/commands；`ssh.ts` 管信任及部署；`watch.ts` 管长请求重开；`list-files.ts`/`terminal.ts` 是 1.2 新面。daemon 的 `main.rs/frame.rs/output.rs` 管 framing/调度；`fs/scan/decode` 实现文件语义；`exec/window` 管进程与输出；`watch/list/pty` 管长期资源；`sys` 隔离 Unix/Windows 系统接口。

Cargo package 元数据仍为 0.1.0、edition 2024、不可发布；`build.rs` 从 npm manifest 注入真正的 daemon release version。直接依赖精确锁定 encoding_rs 0.8.42、ignore 0.4.33、notify 8.2.0、serde_json 1.0.151；Unix libc 0.2.190，Windows windows-sys 0.61.2 明确列 Win32 features。Rust/native 层是行为的一部分；没有审计这些外部库内部。[Cargo.toml](../../../../.repo/pi-1.2.0/packages/env/daemon/Cargo.toml#L1)、[build.rs](../../../../.repo/pi-1.2.0/packages/env/daemon/build.rs#L1)。

TypeScript build 与 `build:daemon` 分开，prepublishOnly 不调用 Cargo；npm files 声明 dist/bin/docs。根 CI 和 binary workflow 已显式准备 daemon，而 local-release/release 的 offline build→test.sh 链没有在这些位置显式准备它，且 test.sh 隔离环境不透传 PI_ENV_DAEMON；因此干净源码环境需另外满足 daemon 前置。此为特定路径的静态准备缺口，不是全部 CI 必然失败；本次没有运行任何路径。[ci.yml](../../../../.repo/pi-1.2.0/.github/workflows/ci.yml#L42)、[build-binaries.yml](../../../../.repo/pi-1.2.0/.github/workflows/build-binaries.yml#L370)、[local-release](../../../../.repo/pi-1.2.0/scripts/local-release.mjs#L100)、[release](../../../../.repo/pi-1.2.0/scripts/release.mjs#L256)、[test.sh](../../../../.repo/pi-1.2.0/test.sh#L40)。

## 连接与一次操作的执行链

```text
RemoteExecutionEnv → Connection lazy start → random sync token / hello(protocol=1)
  → REQUEST(id, JSON, raw payload) → Rust dispatch → RESULT/ERROR/EVENT
  → Result<T, FileError|ExecutionError> / reader handle
```

Connection 首次请求共用启动 Promise；失败清掉 Promise，下一操作可重试启动。spawn 加 `serve --token`，先跳过 shell 启动噪声并找随机 sync line，再以 16 MiB 上限解析 big-endian frames，JSON 必须 object，内容 bytes 单独复制。hello 对协议做版本检查；60 s 启动超时、5 s ping、30 s silence loss。lost 时拒绝该代全部 pending requests、结束 stdin/kill transport，下次请求新开 daemon；不自动重放已发送 mutation，结果可能未知。[Connection](../../../../.repo/pi-1.2.0/packages/env/src/connection.ts#L178)、[启动](../../../../.repo/pi-1.2.0/packages/env/src/connection.ts#L268)、[framing](../../../../.repo/pi-1.2.0/packages/env/src/connection.ts#L335)。

文件路径先按远端 home/cwd/driveCwds 和对应平台规则解析。普通 read 模拟 Node open/stat/size/UTF-8 行为；binary reader 只打开 regular file，noFollow 只拒绝最终 symlink，session 标签把 pread/fstat/close 固定到创建 handle 的 daemon，断线后不能误用新一代同号 handle。binary read 为 256 KiB×8 流水，write 为 512 KiB×8，普通 readFile 上限约 2 GiB；chunk write 在 daemon 同一 handle 上按到达顺序执行，早期失败 poison 该 handle。[分块常量](../../../../.repo/pi-1.2.0/packages/env/src/remote-env.ts#L36)、[session 限定](../../../../.repo/pi-1.2.0/packages/env/src/connection.ts#L203)、[handle/dispatch](../../../../.repo/pi-1.2.0/packages/env/daemon/src/main.rs#L43)。

shell string 与 argv 分开；argv 不通过 shell，Windows string 用 Git Bash。exec 将 timeout 秒验证并转换毫秒，再发 cwd/env/spill/window；callback 抛错优先变 callback_error 并取消命令，其次处理 timeout/abort/spill/exit。`cleanup()` kill 本 env 创建的 command、保留其 killed exit status，不关闭共享连接，也不是每次请求的清理 API。daemon EOF 或失联 30 s kill 所有启动的 process groups 并退出。[exec](../../../../.repo/pi-1.2.0/packages/env/src/remote-env.ts#L852)、[cleanup](../../../../.repo/pi-1.2.0/packages/env/src/remote-env.ts#L950)、[daemon 生命周期](../../../../.repo/pi-1.2.0/packages/env/daemon/src/main.rs#L472)。

## 并发、背压、取消与观察

daemon 有 16 个文件 worker，exec/watch/pty 各开线程；最多 4096 个 open handles。cancel 先登记 control 再执行，立即 cancel 不丢；短文件调用可正常结束，scanLines 在 chunks、listFiles 在 entries 检查，exec/pty kill process group。Connection 取消只发 CANCEL，仍等待 daemon result/error，不能假定发 cancel 就撤回已落盘文件 effect。[常量](../../../../.repo/pi-1.2.0/packages/env/daemon/src/main.rs#L32)、[请求分派](../../../../.repo/pi-1.2.0/packages/env/daemon/src/main.rs#L404)、[协议取消](../../../../.repo/pi-1.2.0/packages/env/docs/protocol.md#L45)。

输出单 writer 优先发 control，小于等于 64 KiB 的 replies 不被 bulk 排队拖住；大 payload 走 bulk。无 window command 与 PTY 在 unsent bulk 超过 4 MiB 时等 room，像满 pipe 一样反压。tail window 可只传可能进入保留区的输出并以 skipped 保持准确字节/行计数；spill 保留完整 raw output。4 MiB 是 bulk 阈值，不是所有 request/control/input 的统一内存上限：file job mpsc、control queue、PTY input mpsc 无总容量；Connection.stdin.write 不等待 drain。高并发由可信调用者约束，此判断是静态资源风险。[Output](../../../../.repo/pi-1.2.0/packages/env/daemon/src/output.rs#L13)、[job queue](../../../../.repo/pi-1.2.0/packages/env/daemon/src/main.rs#L345)、[PTY input](../../../../.repo/pi-1.2.0/packages/env/daemon/src/pty.rs#L190)、[client write](../../../../.repo/pi-1.2.0/packages/env/src/connection.ts#L264)。

RemoteWatcher 等 ready 才返回覆盖，native event 只触发 snapshot diff；Windows/network/FUSE 可 polling，覆盖失效报 overflow。lost 后指数延迟 1 s→30 s 重开并 overflow，无法恢复丢失期间逐条历史。close 停止重连、cancel 长请求并等待其结算；用户 onChange 异常被收敛。[watch.ts](../../../../.repo/pi-1.2.0/packages/env/src/watch.ts#L19)、[语义](../../../../.repo/pi-1.2.0/packages/env/docs/semantics.md#L62)。

## 1.2 新能力与兼容

`listFiles` 是 optional ExecutionEnv 能力，hello.features 协商，旧 daemon 未声明时直接 `not_supported`，宿主可退回自己的 walk。Rust ignore walker 不 follow symlinks、排除 .git，按 depth-first/字节名称排序，默认含 hidden、尊重 gitignore；tracked 但命中 ignore 的文件也会省略，与 git ls-files 不同。默认 50000 entries、4 MiB payload、10 s，可被 context 取消；越界返回 truncated，不可读 entry 跳过。LocalExecutionEnv 与 remote 使用同一 daemon walk，NodeExecutionEnv 自身没增加该方法。[feature gate](../../../../.repo/pi-1.2.0/packages/env/src/list-files.ts#L21)、[Rust walk](../../../../.repo/pi-1.2.0/packages/env/daemon/src/list.rs#L85)、[wire](../../../../.repo/pi-1.2.0/packages/env/docs/protocol.md#L101)。

`openTerminal()` 返回 pid/exited、raw bytes、write/resize/foreground/close。输入按到达顺序，resize 发 SIGWINCH，foreground 区分 shell 与前台 job；写入分 1 MiB frame，结束或 lost 的写错误被忽略，由 exited 报告。当前 Unix 支持，Windows 明确 ENOTSUP；交互终端不是持久任务恢复，连接断开即杀掉程序。[terminal.ts](../../../../.repo/pi-1.2.0/packages/env/src/terminal.ts#L26)、[openTerminal](../../../../.repo/pi-1.2.0/packages/env/src/terminal.ts#L93)、[Windows 分支](../../../../.repo/pi-1.2.0/packages/env/daemon/src/pty.rs#L145)。

SSH 强制 BatchMode、严格 host key、独立 knownHosts alias、禁 forwarding/shared connection/RemoteCommand/local command/locale forwarding。扫描展示指纹、接受 key 是显式宿主选择；已变 key 不静默替换。部署按二进制 SHA-256 命名，每次 start 前校验，缺失才临时上传、hash 验证、rename，再删其他内容版本。hash 是传输/内容一致性，不是上游签名证明。默认不 login shell，宿主可以 opt-in；target 仍是可信账户的完整文件/进程权限，sync token 也不是远端 sandbox。[sshArguments](../../../../.repo/pi-1.2.0/packages/env/src/ssh.ts#L73)、[deployDaemon](../../../../.repo/pi-1.2.0/packages/env/src/ssh.ts#L400)、[每次启动验证](../../../../.repo/pi-1.2.0/packages/env/src/ssh.ts#L490)。

## 文档状态、测试预期与升级

README、semantics、protocol 和 CHANGELOG 均为现行能力说明，没有独立路线图/handoff；四份全文与上述关键实现对照。Node parity、output window、lost watcher overflow 已有机制；listFiles 是故意没有纯 Node counterpart 的新增能力；Windows PTY 是明确尚未支持的边界，未见实现或近期交付承诺。protocol v1 的增量 feature negotiation 已兑现，不能把 npm 同版本要求写成 wire 同版本要求。[发行记录](../../../../.repo/pi-1.2.0/packages/env/CHANGELOG.md#L3)。

测试用共享 Durable conformance、随机 differential/tools parity、SSH mocks/可选真实 SSH，以及 daemon 自测。新增 tests 表达 listFiles feature fallback/cancel、Node platform names 和 PTY raw bytes/输入顺序/resize/foreground/close；terminal suite 在 Windows skip，真实 SSH 需要 PI_ENV_SSH_HOST，普通测试先有 daemon 二进制。这里没有运行测试，也没有八平台运行证明。测试影响：`none`。[remote tests](../../../../.repo/pi-1.2.0/packages/env/test/remote.test.ts#L73)、[terminal tests](../../../../.repo/pi-1.2.0/packages/env/test/terminal.test.ts#L50)、[README development](../../../../.repo/pi-1.2.0/packages/env/README.md#L61)。

相对 1.0.4 清单：新增 6、修改 17、删除 0、相同 19；当前 42 文件、8 个生产 TS/2346 行。主要是 listFiles/LocalExecutionEnv/nodePlatform、PTY 和 foreground、Rust ignore 依赖及 wire 文档。旧版“只有远端文件/exec/watch”的 API 清单已不足，但远端环境与模型/session authority 分离的边界仍成立。旧报告只作导航，当前结论均重读本快照，不声称完成完整旧新逐行 diff。

# Windows 本地 Agent benchmark 可行性核查

日期：2026-09-05。

状态：非规范性外部研究证据；不构成 Phase 19 准入、规范修订、实施计划或完成记录。

范围：仓库已固定的 Terminal-Bench 2.1、Terminal-Bench 3.0、Datacurve DeepSWE v1.1；Windows 宿主下的本机执行条件、原生评分边界和 provider 配置。

语言：本报告响应中文讨论，属于一次性 outward research，没有现存英文对应文档；本次不新增英文副本。产品文档和规范的双语同步要求不由本报告替代。

## 结论

Windows 电脑可以承载本地评测。适合本项目当前固定材料的路径是 **WSL2 中的 Linux 执行器，加本机 Docker Desktop Linux containers**。这仍然是本地运行；不需要把正式评测交给 GitHub Actions，也不需要默认使用 Modal 等远程 sandbox。Docker 官方支持 WSL2 backend 和 Linux distribution integration；Pier 官方明确支持 `docker` 与 `modal` 两种环境。[Docker WSL2 文档](https://docs.docker.com/desktop/features/wsl/)、[固定 Pier README](https://github.com/datacurve-ai/pier/blob/c1ebc6d145b40fae8425215e3fca528945065124/README.md)

但“Windows 上能够本地执行”不能直接等同于“普通 Windows PC 可以完整复现所有官方分数”。TB3 v3.0.0 的部分任务明确要求 H100 GPU 和较大内存、磁盘配置；本机只读检查发现的是 RTX 4060 Laptop GPU，不能承诺满足全量条件。适配器必须在运行前识别资源不足，并把可执行子集的分数与全量分数区分。[TB3 H100 任务配置](https://github.com/harbor-framework/terminal-bench/blob/v3.0.0/tasks/jax-speedrun-gpu/task.toml)

本机只读检查已确认 Ubuntu 与 docker-desktop 都是 WSL2，但两者处于 Stopped，Docker Linux daemon 当前不可达。因此只能确认安装基础存在，不能宣称已经可以运行任务。本次没有启动 WSL、Docker、容器或真实 provider。

## 1. 三个 benchmark 的身份与版本

| 项目 | 本仓库固定身份 | 官方事实与定位 |
|---|---|---|
| Terminal-Bench 2.1 | `harbor-framework/terminal-bench-2-1`，commit `7131e4375048a0e408a8fb404b5f499d726b695b` | 2.0 的修订版；当前 README 说明修订了 26 个任务。固定 commit 的 `tasks/` 有 89 个任务目录。|
| Terminal-Bench 3.0 | `harbor-framework/terminal-bench`，tag `v3.0.0`，commit `2b0442c3c583b710ca8da14c8e601b99f2f1f244` | 已于 2026-07-23 发布，初始 74 个任务。不是未发布占位符。该 tag 的 README 当时使用 Frontier-Bench 名称；当前 canonical 仓库重新使用 Terminal-Bench，并描述为持续更新的 benchmark。|
| DeepSWE v1.1 | `datacurve-ai/deep-swe`，版本锚点 `8cae5984d5dd0ee37445beff0e928dc10c331116`；可执行材料 `435ee89ec2f2e2289f33b0da4f992f0b7b7266b9` | Datacurve 的 113 个长周期软件工程任务 benchmark，覆盖 TypeScript、Go、Python、JavaScript、Rust。本项目不是同名的其他模型或训练项目。v1.1 用提交消息锚定；本次 tags 查询只见 v1.0.0，不能把 v1.1 称作 Git tag。|

主源：[TB2.1 README](https://github.com/harbor-framework/terminal-bench-2-1/blob/main/README.md)、[TB2.1 固定任务目录](https://github.com/harbor-framework/terminal-bench-2-1/tree/7131e4375048a0e408a8fb404b5f499d726b695b/tasks)、[TB3 v3.0.0 release](https://github.com/harbor-framework/terminal-bench/releases/tag/v3.0.0)、[TB3 tag README](https://github.com/harbor-framework/terminal-bench/blob/v3.0.0/README.md)、[当前 TB README](https://github.com/harbor-framework/terminal-bench/blob/main/README.md)、[固定 DeepSWE README](https://github.com/datacurve-ai/deep-swe/blob/435ee89ec2f2e2289f33b0da4f992f0b7b7266b9/README.md)、[DeepSWE v1.1 锚点](https://github.com/datacurve-ai/deep-swe/commit/8cae5984d5dd0ee37445beff0e928dc10c331116)。

推断：Phase 19 应继续显式固定数据版本，不能把 `latest`、TB3 与其他版本的分数混为一谈。TB3 的“已正式发布”也不代表每台机器都能满足全部任务资源，或本项目已经完成其完整适配。

## 2. 本地运行实际需要什么

仓库目前固定 Harbor 0.22.0 与 Pier 0.3.1，两者 `pyproject.toml` 都要求 Python >=3.12。运行器和 benchmark 是两个独立版本维度，不能只固定任务 Git commit 后随意安装最新 runner。[Harbor 固定 manifest](https://github.com/harbor-framework/harbor/blob/4407eb5227a2ff4f0d3f16b2eb48849382fdf276/pyproject.toml)、[Pier 固定 manifest](https://github.com/datacurve-ai/pier/blob/c1ebc6d145b40fae8425215e3fca528945065124/pyproject.toml)

本地运行需要以下材料和能力：

1. 对应版本的完整任务包、其指定镜像或可构建 Dockerfile、固定 runner 及依赖。获取 GitHub 上的源码是下载材料，不等于在 GitHub 上执行评测。
2. 能运行任务 Linux 镜像的本机 Docker engine，足够的任务资源，以及对应 Linux Opi/pi executable 和运行时。所核查的 TB2.1 Dockerfile 基于 Debian Python 镜像；TB3 smoke task 同样基于 Linux Python；DeepSWE task 明确声明 `os = "linux"`。[TB2.1 Dockerfile](https://github.com/harbor-framework/terminal-bench-2-1/blob/7131e4375048a0e408a8fb404b5f499d726b695b/tasks/openssl-selfsigned-cert/environment/Dockerfile)、[TB3 Dockerfile](https://github.com/harbor-framework/terminal-bench/blob/v3.0.0/tasks/batched-eval-parity/environment/Dockerfile)、[DeepSWE task](https://github.com/datacurve-ai/deep-swe/blob/435ee89ec2f2e2289f33b0da4f992f0b7b7266b9/tasks/abs-module-cache-flags/task.toml)
3. 真实 Agent 运行所需的 provider、model、endpoint 和本地凭据引用。provider 可以是远程推理服务；“本地 benchmark 环境”不要求模型权重也在本地。使用本地模型 endpoint 也要满足对应 Agent 的协议和配置能力。
4. 只让 Agent 操作任务环境可见的输入及文件，并按官方规则将完成材料交给 grader。仅把 Agent 的普通主机目录输出复制给测试脚本，不能自动证明其与官方任务环境等价。
5. 本机保存每次 trial 的原始 grader 结果、轨迹、错误、版本和资源配置，再形成任务集汇总。

第 3 至 5 项是结合官方 runner 接口给本项目的实施建议，不是已实现能力声明。Harbor 的 `BaseAgent.setup/run` 接收实际 `BaseEnvironment`，并分开保存 model connection、环境变量及运行上下文；因此“Agent 的工具在哪里执行”是集成正确性的必要问题。[固定 Harbor Agent 接口](https://github.com/harbor-framework/harbor/blob/4407eb5227a2ff4f0d3f16b2eb48849382fdf276/src/harbor/agents/base.py)

WSL2 中的源码、任务、cache 和 trial 工作目录宜放在 Linux 文件系统。Docker 官方说明，从 Linux 文件系统进行 bind mount 的性能和文件变更通知行为优于 `/mnt/c` 等 Windows 挂载目录；这对大量小文件、Git、依赖安装尤其相关。[Docker WSL2 最佳实践](https://docs.docker.com/desktop/features/wsl/best-practices/)

这不构成“所有 Harbor 功能不支持 Windows”的判断。框架也有 Windows task 能力；本报告针对本仓库当前固定的 Linux benchmark 材料，未验证纯 PowerShell 中直接运行整条 opi-eval native 流程的兼容性。

## 3. 原生 grader 和网络边界不能丢

| 固定 smoke task | 官方环境与交付边界 | 不能用 smoke 覆盖掉的实际行为 |
|---|---|---|
| TB2.1 `openssl-selfsigned-cert` | 共享 verifier 模式；任务 agent timeout 900 秒，1 CPU / 2 GiB；task 允许联网 | 其他 88 个任务各自的配置、环境与 verifier，需要逐项读取和运行。|
| TB3 `batched-eval-parity` | 独立 verifier；只交付声明的 `/app/evalbench/`；agent timeout 14400 秒，verifier 900 秒；1 CPU / 4 GiB | Agent 的真实解题时间不能静默缩成 smoke 的较短预算；任务成功不能代表 TB3 全量。|
| DeepSWE `abs-module-cache-flags` | Agent 提交工作；collect hook 提取相对固定 base commit 的 patch；全新 verifier 无网络；agent timeout 5400 秒，verifier 1800 秒；各 2 CPU / 8 GiB | 未提交的工作与已提交 patch 不等价；不能把修改过的 Agent 环境直接当成 pristine grader 环境。|

主源：[TB2.1 task.toml](https://github.com/harbor-framework/terminal-bench-2-1/blob/7131e4375048a0e408a8fb404b5f499d726b695b/tasks/openssl-selfsigned-cert/task.toml)、[TB3 task.toml](https://github.com/harbor-framework/terminal-bench/blob/v3.0.0/tasks/batched-eval-parity/task.toml)、[DeepSWE task.toml](https://github.com/datacurve-ai/deep-swe/blob/435ee89ec2f2e2289f33b0da4f992f0b7b7266b9/tasks/abs-module-cache-flags/task.toml)。这里列的是所固定任务，不能外推成整个 corpus 的统一资源配置。

DeepSWE v1.1 官方要求 Pier >0.3.0，以支持独立 verifier 和 collect hook。Pier 对安装于 sandbox 内的 Agent 提供安装及推理目的域名 allowlist，解决无网络任务中 Agent 仍需要安装工具和访问 LLM API 的问题；它在 Docker 和 Modal 环境都实现了该机制。配置 provider 时必须验证运行位置和可达性，不能为了让 API 工作而开放 grader 网络或把整项任务改成无限制联网。[DeepSWE 固定 README](https://github.com/datacurve-ai/deep-swe/blob/435ee89ec2f2e2289f33b0da4f992f0b7b7266b9/README.md)、[Pier 固定 README](https://github.com/datacurve-ai/pier/blob/c1ebc6d145b40fae8425215e3fca528945065124/README.md)

DeepSWE 原生结果包括 `reward.json`、`ctrf.json` 和原始日志；主结果与测试通过比例是不同字段。建议保留原始字段，由明确的 benchmark 版本规则产生汇总，不能把任意测试通过率当作任务 resolved rate。

## 4. 抽样分数、全量分数和正式榜单

DeepSWE 官方直接支持 `--n-tasks 10 --sample-seed 0` 的确定性随机抽样，也支持单任务目录；因此在本地先跑子集完全合理。该项目固定 README 同时说明，其已公布榜单分数由 mini-swe-agent 在 Modal 上生成。换成 Opi/pi 的本地结果是新的实验，不能冒充原来的测量条件。[DeepSWE 子集与榜单说明](https://github.com/datacurve-ai/deep-swe/blob/435ee89ec2f2e2289f33b0da4f992f0b7b7266b9/README.md)

TB3 的 H100 限制是具体事实：`jax-speedrun-gpu` 的 agent 和 verifier 都声明 16 CPU、32 GiB 内存、1 H100、1024000 MB storage，并包含固定 wall-clock 训练质量门槛；`fp8-rmsnorm-gemm` 也声明 H100。不能用 CPU 或任意消费级 GPU 替代后声称测得同一官方条件下的分数。[JAX task](https://github.com/harbor-framework/terminal-bench/blob/v3.0.0/tasks/jax-speedrun-gpu/task.toml)、[FP8 task](https://github.com/harbor-framework/terminal-bench/blob/v3.0.0/tasks/fp8-rmsnorm-gemm/task.toml)

建议报告至少区分：

- 单任务或少量 smoke：验证接线、执行、收集和评分是否成立。
- 固定子集实验：保留选中的 task ids、seed、选择规则、完成数、失败数和明确分母；名称注明 subset。
- 全量实验：没有因本机资源或失败而静默跳过任务，记录每项未完成及其原因；只有满足相应官方协议时才讨论与官方分数比较。
- 榜单提交：遵循当时官方 submission 协议，独立于本地评测。TB2.1 当前页面要求至少每任务 5 次并公开上传，但也明确暂时关闭社区 submission；这不构成本地实验必须上传或使用 GitHub runner 的要求。[TB2.1 submission 说明](https://github.com/harbor-framework/terminal-bench-2-1/blob/main/README.md)

上述报告分层是研究建议。子集 solved rate 不能更名为完整 benchmark 分数，也不能以全量中的失败任务为由缩小成功率分母。

## 5. provider 配置由谁拥有

官方实现把任务配置与 Agent 配置分开。Pier 用 `agent.model_name` 记录模型，`agent.env` 传运行时变量，agent-specific kwargs 传工具配置，并从相关配置提取自定义 base URL 的网络 allowlist。[固定 Pier Agent runtime configuration](https://github.com/datacurve-ai/pier/blob/c1ebc6d145b40fae8425215e3fca528945065124/README.md)

本项目建议的职责：

- 用户在本地选择 provider、model、endpoint 和 credential reference。
- Opi/pi 各自拥有其模型协议、认证语义和 native 配置；eval adapter 将本地实验配置转成各 Agent 接受的配置，并证明实际运行采用了该配置。
- `opi-eval` 拥有实验矩阵、统一预算与记录、受控配置注入、缺少凭据的运行前诊断，以及结果的版本身份；不需要为了做 benchmark 再实现一套通用 LLM 客户端。
- benchmark grader 拥有原生评分规则，不接管 provider 选择。

开发 hermetic 测试、oracle preflight、scripted-provider 接线测试时不向用户索要真实 API key 是合理的。只有实际用户配置的模型参与解题并交给原生 grader 之后，才有该模型加 Agent 组合的实际表现证据。没有索要 key 不单独证明项目偏离；把无需 key 的 smoke 视作用户本地真实评分需求已经交付，才会产生验收错位。这是基于测试类型的判断，不是关于 Phase 18 是否满足其注册设计的结论。

## 6. 本机就绪情况与可验收路径

只读观测（2026-09-05）：

| 检查 | 结果 |
|---|---|
| `wsl --status` | 默认 distribution 为 Ubuntu，默认版本 2。原生命令输出编码在 shell capture 中出现乱码，因此 distribution 状态另用 UTF-16LE 解码核对。|
| `wsl --list --verbose` | Ubuntu：Stopped、2；docker-desktop：Stopped、2。|
| `docker version` | Client 29.0.1，windows/amd64，context `desktop-linux`；连接 `dockerDesktopLinuxEngine` named pipe 失败，无法获得 Server 版本。|
| `Get-CimInstance Win32_Processor` | Intel Core i9-14900HX，24 cores，32 logical processors。|
| `Get-CimInstance Win32_ComputerSystem` | `TotalPhysicalMemory=34053414912` bytes，约 31.7 GiB 可见物理内存；不是 WSL/container 可用内存。|
| `Get-CimInstance Win32_VideoController` | NVIDIA GeForce RTX 4060 Laptop GPU 与 Intel UHD Graphics；没有发现 H100。CIM `AdapterRAM` 不用作可靠的独显显存容量证明。|
| `Get-CimInstance Win32_LogicalDisk -Filter 'DriveType=3'` | 检查时空闲空间：C: 约 67.8 GiB，D: 约 136.7 GiB，E: 约 394.5 GiB。尚未确认 Docker image store / WSL VHD 的实际位置与限制。|

这些宿主资源表明 CPU-only 单任务/有限并发值得优先验证，但还不能证明具体 task 已可运行。所发现的 GPU 不是上述固定 TB3 task 要求的 H100，宿主物理内存也不能完整提供 32 GiB 给一个 verifier；当前机器不能据此宣称能够按原条件完成 TB3 全量。不能把声明约 1 TB 的任务 storage 要求当成镜像一定实际占用 1 TB，但同样不能在验收时忽略这项资源声明。结论是适合先规划 CPU subset 的真实本地运行；完整 TB3 的资源缺口必须显式保留。

尚未验证：Ubuntu 内 Python/uv/Rust/Node 版本、Docker integration 与 engine 启动后状态、WSL/container 实际可用资源、GPU 容器传递、镜像可取性、任务 oracle、provider 可达性、真实 Opi/pi trial。WSL distribution 的 VERSION=2 不等于 WSL 软件包版本满足 Docker 文档的最低版本要求。

可作为 Phase 19 讨论的验收顺序：

1. 本机环境诊断 -> verify：显示 Linux engine、架构、可用资源、固定 runner/Agent 版本，缺项给出准确原因；不要求先提供真实 key。
2. 解析本地实验配置并准备任务 -> verify：锁定 task ids、版本和资源需求；本地 cache 可复用；不依赖 GitHub Actions artifact 才能开始。
3. 一个 CPU-only 任务的 oracle preflight -> verify：官方解答和原生 grader 在本机形成预期结果，证明环境接线成立。
4. 用户配置一个真实 provider，Opi/pi 分别解题 -> verify：任务工具确实作用于任务环境，原生输出、模型配置、成本或不可得原因、失败原因可查；成功得分不是完成门槛，真实运行及真实评分才是。
5. 固定 CPU 子集批量运行 -> verify：按每个任务自己的限制运行，支持本地结果恢复及可靠汇总；失败和资源不足不被静默丢弃。
6. 全量可行性检查 -> verify：逐任务资源和官方条件齐备后再发起全量。资源不满足时明确报告当前无法形成完整分数；不自动改到远程环境。

步骤是研究建议，待 Phase 19 规范与正式 admission 明确后再实施。现有 CI 仍适合免费、确定的回归检查和可选 native smoke；本地真实实验应有独立入口、材料与结果生命周期。

## 7. 取证与限制

- 仓库起始 HEAD：`765608a55a78db47e8b41d2639c29ef8c90e0f31`；本报告仅新增 Markdown，不修改 runtime、规范、进度账本或外部锁。
- 通过 agent-reach 的 GitHub CLI 路径使用 `gh api` 读取上述固定源码、tag/release、目录和 manifest。可变 README 的结论注明为本次查询状态；涉及现有集成 API 时优先读取固定 SHA。
- 使用 `firecrawl --status` 后抓取 Docker 官方 WSL2 与最佳实践页面；Harbor 旧 `/docs/environments` URL 返回 404，因此未把它当作有效来源。固定 Harbor 对应部分文档仍是 placeholder，环境和 Agent 的具体事实转以固定源码及 Pier README 核实。
- Firecrawl 临时抓取文件保存在系统临时目录的 `.firecrawl` 子目录，不进入仓库。未采集任何凭据、`.env` 内容或登录态。
- `agent-reach check-update` 返回 v1.5.0 已是最新版本。
- 未安装软件、启动服务、下载镜像/数据集、运行真实 provider 或提交 Git。本文不是运行成绩单。

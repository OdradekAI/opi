# DeepSWE 镜像仓库代理修复

已在独立的原生材料候选中修复 Docker Hub 连接阻塞。本轮仅产生准备与修复证据，
新增 benchmark 试验、Agent 启动、grader 启动、模型调用及真实凭据读取均为零。
此前六个槽位仍为 4/6 有效，两次 DeepSWE 失败保持原样。用户的“继续修复”授权本次基础设施修复。

## 原因与修改

冻结的 runner 环境缺少宿主机代理。Companion 按设计清空外部环境变量，Pier 再将
显式环境传给 Docker；只在外层启动脚本中导出代理无法跨过这一边界。
使用原环境构建未修改的原生代理 Dockerfile 时，获取 Docker Hub 匿名令牌在
30.977 秒后失败。仅补入 `HTTPS_PROXY=http://127.0.0.1:19828`，构建在
2.563 秒内通过，重复检查在 0.973 秒内通过。该端点没有内嵌凭据。

修复材料仅在 DeepSWE 的 `verifier_env` 和 `oracle_env` 两个现有映射中加入
这个键，其余值完全相同。没有修改 Rust、上游 runner、任务包、镜像配方、全局
代理或 DNS 设置，也没有修改已有计划。本地评测说明补充了显式宿主机代理配置，
以及使用真实 runner 环境进行检查的步骤。该说明文件没有中文对应文件；本报告
提供内容同步的中英文版本。

## 验证

全部执行记录与输出保留在修复目录的 `commands/` 中。以下使用路径缩写：

```text
N=/home/luiz/opi-eval-runs/20260910t170439z-deepswe-network-repair
P=/home/luiz/opi-eval-runs/20260910t154500z-offline-diagnosis-opi082/next-acceptance
```

| 实际检查 | 结果 |
|---|---|
| 对公开匿名令牌端点执行 `curl`，丢弃响应正文，对照原隔离环境与显式既有代理 | 直连 exit 28 / HTTP 000；代理 exit 0 / HTTP 200。未调用模型端点。 |
| `python3 N/reproduce-build.py`，在记录的环境下执行 `docker build --progress=plain --tag opi-eval-pier-egress-proxy:20260910t170439z N/proxy-context` | 原环境 exit 1；显式代理两次 exit 0。 |
| `python3 N/run-bootstrap-checks.py`，调用固定版本的 `uv run --locked --no-sync python N/bootstrap-check.py direct\|proxy` | 缓存预热后两者均 exit 0。修复环境下，原生 Pier `DockerEnvironment.start(force_build=False)` 在 5.114 秒内完成 Compose 启动及健康检查。 |
| 默认任务环境与 Agent 进程环境断言 | 通过：默认任务执行没有宿主机代理；Agent 执行仍使用 Pier 带认证的受限代理。这里只执行 shell 断言，没有启动 Agent 或 grader。 |
| 使用已记录的原生 Compose 文件执行 `docker compose ... build --pull pier-egress-proxy` | 预热后两种环境均 exit 0，不能据此认定直连稳定。 |
| 独立客户端执行 `docker buildx imagetools inspect --raw docker.io/library/ubuntu:24.04`，对照原环境与修复环境 | 直连 30.110 秒后 exit 1；显式代理 1.960 秒后 exit 0。返回清单 SHA-256 为原固定摘要 `224a1869083a311ef3f13648a154ba79832fbef6364d31493642ca03082da254`。 |
| `P/builds/opi-eval validate --config P/inputs/deepswe-v1.1.toml --native-material N/material/material.json` | exit 0；派生 native integrity 为 `0474341d58723fc8f56bf42a3fb999370dc465104610ce55490e6d21891ec82c`。没有生成新计划或启动试验。 |
| `python3 scripts/opi-doc-check.py`；`git diff --check` | 结果见保留的最终检查记录。 |

预热后的通过结果与负向证据一并保留。它们不能消除独立 Docker 请求中已经复现的
代理需求。这些检查访问了公开镜像仓库；“未调用模型”不等于完全断网。未来执行
仍依赖本地代理可用。

## 身份与证据

- 原材料 SHA-256：`a2cf5cd83a577b76f1d070785e74cd9b47170ad1a0dbf8f7128d02faf42f169f`。
- 修复材料 SHA-256：`b725588bff93c28458e09fae2feaa23920758627961a187ff6aa7659c6f9f18e`。
- Companion 可执行文件 SHA-256：`80da1b3b3fc83802035d77cf3df6e9e4e220f36e0f86ef3eacc1733c12c7c560`，未改变，构建来源为 `55fbc95009e6f47145196dbc539454ebcb1cf692`。
- 保留的可复用代理镜像：`sha256:cab836d1ae2453f3b0dc60a25423f884238c92abc8eb49759bfe23c95b4cdce4`。
- 修复原始目录为上面的 `N`。Windows 可访问的私有副本：[证据清单](E:/opi/evaluations/20260910t170439z-deepswe-network-repair/inventory.json)。
- 已准备的候选：[material.json](E:/opi/evaluations/20260910t170439z-deepswe-network-repair/material/material.json)。其中固定的 Linux 路径仍是权威路径；副本不是迁移后的可运行缓存。

四份既有暂存清单（694、276、586、124 个文件）与两份受保护的 Linux 清单
（549、39 个文件）均检查到零变化。没有创建或升级 acceptance manifest。

清理完成：两个启动检查项目的容器、卷和网络已删除；三个可证归属的重复代理镜像
在确认文件系统和启动配置与保留镜像相同后删除。基线中的全部 11 个镜像 ID 及共享
缓存仍在。最终检查为零容器、零卷、无本轮网络。由于共享镜像层，实际释放字节数
未知。停止的合成启动检查实例所生成的代理配置保留为私有证据。

工作树范围：本轮无提交、无暂存；修改一个已跟踪的评测说明文件，新增本报告两种
语言版本。进入本轮前的八个评测报告未触碰，`HEAD` 未改变。仓库测试影响：
**none**；没有修改运行时或测试源文件，不声称执行了 Rust 测试或工作区门禁。

后续付费评测需要基于修复材料生成新计划，并取得对应精确范围的授权。本次修复
不代表 DeepSWE 任务成功、新六槽位验收通过、独立审计准入或 Phase 完成。

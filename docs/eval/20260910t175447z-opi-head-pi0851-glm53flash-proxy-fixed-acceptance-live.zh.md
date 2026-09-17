# 代理修复后的验收复跑：Opi HEAD 对照 pi 0.85.1

[English](20260910t175447z-opi-head-pi0851-glm53flash-proxy-fixed-acceptance-live.md)

**执行状态：completed，6/6 次有效原生试验。证据检查：verified。正式验收交接：blocked，待完成脱敏证据交接。** 两端在 TB 2.1 的奖励均为 1，在 TB 3.0 和 DeepSWE 1.1 均为 0。四次零分均为有效任务失败，没有无效或未执行槽位。此前 DeepSWE 的镜像仓库启动故障未再出现。离线结构验收及清理后复验均返回 `accepted: true`；manifest 仍如实标明证据为私有、未脱敏。本报告不关闭 Phase 或实施任务。

## 执行与证据身份

- 原始私有 Linux 执行目录：`/home/luiz/opi-eval-runs/20260910t175447z-proxy-fixed-acceptance`。该目录与其固定缓存依赖保留，用于离线核对原生记录。
- [暂存证据](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live)与[最终 manifest](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/manifest.json)：登记 570 个保留文件，不含 manifest 自身；SHA-256 为 **`e1604c1cfad0833f55a21e35df6fda2588969205236c39fcde892dc0603cef12`**。
- [清理后验收](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live-checks/acceptance-post-cleanup.json)、[原生试验汇总](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/reports/trial-summary.json)、[用量/时间/测试明细](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/reports/metrics-detail.json)、[外部检查](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live-checks)。
- 冻结[范围](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/operator-scope.json) SHA-256：`47158dc87139b2631bb9714335efd201825046068ef7cae1ac4fa5599a9e8044`；[授权](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/authorization.json) SHA-256：`7d7f3db46f14b429ea6c31567b1eec2da657652a90b460e6c643aa4edd56de4d`。用户“复跑”授权按已审阅限制执行这六个新槽位。预览中的计数保持冻结，完成状态另行记录。

执行开始于 `2026-09-10T17:58:32.789263+00:00`，结束于 `2026-09-10T18:45:59.825502+00:00`，跨度 2847.04 秒（47 分 27 秒），包含家族之间的检查。六次原生试验耗时合计 2730.61 秒。执行顺序为 DeepSWE、TB 2.1、TB 3.0，每对先 pi 后 Opi。[历次尝试](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/attempts/prior-attempts.json)保留原发布版、HEAD 运行与[代理修复](20260910t170439z-deepswe-network-repair.zh.md)的关系；本轮没有覆盖此前字节或结果。


| Component / 组件 | Source commit / 源码提交 | Executable SHA-256 / 可执行文件摘要 |
|---|---|---|
| Opi HEAD, package 0.8.2, musl | `55fbc95009e6f47145196dbc539454ebcb1cf692` | `d7e5ab2d2636b04bfefafdeb2285bf50933247e7fc06836b6778d91ff4056e60` |
| pi 0.85.1, official Linux x64 | `d981de1229ef899957bbe968bc8dcda02a21f477` | `443bd83f30e4dbc7bac2eed9c6aa2461b9a15016fd555f48c92a0591d028c403` |
| opi-eval Companion, package 0.8.2 | `55fbc95009e6f47145196dbc539454ebcb1cf692` | `80da1b3b3fc83802035d77cf3df6e9e4e220f36e0f86ef3eacc1733c12c7c560` |


获批的 Opi 是**包版本为 0.8.2 的未发布 HEAD 候选，不是 v0.8.2 标签**。两个客户端与 Companion 均按原字节复用，没有重新构建或修改运行时源码。保留的构建观测记录当时已跟踪源码干净；当前工作区存在此前报告/README 改动及本报告对。pi 包 manifest SHA-256 为 `0701993b1f4ddd9f7b4a6b2e9ecd963b3ac9457aca0807b71d4eb98339343aae`；Cargo.lock SHA-256 为 `261a3ec501be2e045dc6c9b2b9a940adc1c56305f477008eea940f26545925f3`。构建记录使用 rustc/Cargo 1.97.1、Linux x86_64。

[按仓库绑定的源码身份](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/builds/source-identities.json)记录 Opi 来源 `https://github.com/OdradekAI/opi`，pi 来源 `https://github.com/earendil-works/pi`，Harbor 来源 `https://github.com/harbor-framework/harbor`、提交 `4407eb5227a2ff4f0d3f16b2eb48849382fdf276`，Pier 来源 `https://github.com/datacurve-ai/pier`、提交 `c1ebc6d145b40fae8425215e3fca528945065124`。原始观测日期保留，没有冒充本轮联网观测。[复用检查](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/checks/reused-build-evidence.json)和准备步骤重新核对了固定字节与 runner 提交。

环境为 WSL Ubuntu、Docker Desktop Linux/amd64。Doctor 观测到 32 CPU、16611237888 字节内存和约 950.7 GB 可用磁盘；具备 GPU 能力，但任务无需 GPU。三个任务的资源要求依次为 1 CPU/2 GiB、1 CPU/4 GiB、2 CPU/8 GiB。固定材料、原生任务锁、镜像/资源清单与清理前后镜像身份保存在[计划](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/plans)、[材料](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/materials)、[基线](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/baseline.json)和[镜像检查](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live-checks/cleanup-images.json)中。

## 冻结控制与范围

两端均请求 `zhipu:glm-5.3-flash`、`anthropic-messages`、端点 `https://open.bigmodel.cn/api/anthropic`。获授权的启动助手从用户 `.env` 选取 `OPI_BIGMODEL_API_KEY`，本报告不含文件或密钥值。pi 已完成响应的元数据报告 `glm-5.3-flash`；Opi requested/resolved 路由符合要求，实际 wire 模型身份仍为 `wire_unknown/not_reported`，不能从选择器反推服务端模型身份。

必需控制为 `max_output_tokens=8192`、客户端 `context_window=131072`，保留了绑定精确客户端的输出控制证明。上下文声明不是服务端容量实测。temperature、top_p、reasoning 均省略。每家族选择一个准入本地任务，每端各重复一次，无 seeds，并发 1、`max_trials=2`，总计六次；未枚举完整上游题库。保留完整私有捕获。

TB 2.1/TB 3.0/DeepSWE 每端原生作业上限依次为 900/900/1800 秒，六次累计 7200 秒，包含环境准备、Agent 和评分。原生 stdout/stderr 各限 1 MiB，独立 Agent 捕获可能更大。不保证总 token、账单费用或总墙钟时间上限。本轮没有追加付费试验重试、覆盖控制、替换模型或原生重新评分。未完整测量传输/SDK 重试和压缩。Opi 的既有 50 个 provider turn 限制在 DeepSWE 中触发，执行中没有新增或修改该限制。

此前修复只向 DeepSWE 显式 verifier/oracle 环境映射加入 `HTTPS_PROXY=http://127.0.0.1:19828`，不含凭据，保持 Companion 的确定性环境隔离。启动前以相同环境检查公开镜像仓库成功。初次材料校验因尚未附加输出控制证明而退出 1；重新附加经过字节核对的既有精确客户端证明后，三个最终 validate/plan 均退出 0。初次拒绝保留在 `commands/validate-derive-deepswe-v1.*`。共享已准备缓存清单保持不变，SHA-256 为 `cbba9c15a338c696f9b816390cd66d9b72de43ecb391e0736b9cfe1e74819f2c`。


| Family / 家族 | Task / 任务 | Plan digest / 计划摘要 | Material SHA-256 / 材料摘要 |
|---|---|---|---|
| terminal-bench-2.1 | `openssl-selfsigned-cert` | `98edc79c1d9dc70f2b7d8d539e2fb59cf35a48077c1b319f5c1ecb4d97cd78e0` | `0255bc9c9b6efa299b735343e37fcdfb9d8ca33bd3681c9cf5db43e1c9a705ba` |
| terminal-bench-3.0 | `batched-eval-parity` | `d753efbe02f64e77b35789dc1222251b7891e9b66a686639949fd609f7e1b16c` | `a68fc184b63a377a4abac7957a8532f97c3f33ace6eb8fec1d96cc70cdc2cbac` |
| deepswe-v1.1 | `abs-module-cache-flags` | `4cba865a6d7dd37cf888cb1150526f4cd470dd09d6c9c4a642064b0d1028bb74` | `0e173791ad908eca5fdedbff22e96367621441217582d1499b216a831c3daf59` |


## 原生结果与试验追溯


| Family / 家族 | Subject / 软件 | Scope | Native reward | Denominator | Corpus/selected | Planned/attempted | Valid/Agent failure | Invalid/unexecuted |
|---|---|---|---|---|---|---|---|---|
| terminal-bench-2.1 | pi 0.85.1 | subset | 1 | 1 | 1/1 | 1/1 | 1/0 | 0/0 |
| terminal-bench-2.1 | Opi HEAD 0.8.2 | subset | 1 | 1 | 1/1 | 1/1 | 1/0 | 0/0 |
| terminal-bench-3.0 | pi 0.85.1 | subset | 0 | 1 | 1/1 | 1/1 | 1/1 | 0/0 |
| terminal-bench-3.0 | Opi HEAD 0.8.2 | subset | 0 | 1 | 1/1 | 1/1 | 1/1 | 0/0 |
| deepswe-v1.1 | pi 0.85.1 | subset | 0 | 1 | 1/1 | 1/1 | 1/1 | 0/0 |
| deepswe-v1.1 | Opi HEAD 0.8.2 | subset | 0 | 1 | 1/1 | 1/1 | 1/1 | 0/0 |


指标为有效已评分试验上的原生 grader reward。每家族分母为 2，每行分母为 1；Corpus/selected 指准入本地任务包，不是完整上游题库。有效任务失败保留在分母中，没有排除比较端或缺失重复。这组六次子集不构成完整基准结果或统计优劣结论，不跨家族取平均。规范化报告中的 `classification: conformance-evidence` 原样保留，真实执行含义另外由 Agent 和原生交付记录核对。

下表为本轮选定的完整六行，每个试验链接直接指向封存的原生结果。[机器汇总](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/reports/trial-summary.json)进一步绑定 Agent stdout/trace、工具计数、原结果字节一致性、收集输入、时间和验证器身份。六份封存原生结果均与各自 runner 原结果字节一致，结算均为 `graded-native`。三个 Companion `run` 均退出 0，其中包含获得合法原生评分的 Opi 非零退出试验。


| Trial (repetition 1) / 试验 | Native job / 原生作业 | Agent exit / 退出码 | Bundle manifest SHA-256 / 封存摘要 |
|---|---|---|---|
| [trial-baseline-pi-proxy-fixed-openssl-selfsigned-cert-r1](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/runs/terminal-bench-2.1/trials/trial-baseline-pi-proxy-fixed-openssl-selfsigned-cert-r1/bundle/artifacts/native/trial-result.json) | `task-package__fry8gNy` | 0 | `3fbeba0483119c395a17abdca63a29e3202d3716804e18b0d1ca602c04a192fc` |
| [trial-candidate-opi-head-proxy-fixed-openssl-selfsigned-cert-r1](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/runs/terminal-bench-2.1/trials/trial-candidate-opi-head-proxy-fixed-openssl-selfsigned-cert-r1/bundle/artifacts/native/trial-result.json) | `task-package__oqvWJdN` | 0 | `cbf76c034eeed4a2f96b9ff1b22a5d0f0a55c7100637c0e22ebd730800334c9b` |
| [trial-baseline-pi-proxy-fixed-batched-eval-parity-r1](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/runs/terminal-bench-3.0/trials/trial-baseline-pi-proxy-fixed-batched-eval-parity-r1/bundle/artifacts/native/trial-result.json) | `task-package__eQkSCgA` | 0 | `62a87c881d37de6f4fb1bcc7e98db302e87d46d0a76bfe9a88e5b36ebe79b0f7` |
| [trial-candidate-opi-head-proxy-fixed-batched-eval-parity-r1](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/runs/terminal-bench-3.0/trials/trial-candidate-opi-head-proxy-fixed-batched-eval-parity-r1/bundle/artifacts/native/trial-result.json) | `task-package__DuPKUFb` | 0 | `78e50cd100df22e6678039755632a9a8338537af923f71aef815c86b0103e710` |
| [trial-baseline-pi-proxy-fixed-abs-module-cache-flags-r1](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/runs/deepswe-v1.1/trials/trial-baseline-pi-proxy-fixed-abs-module-cache-flags-r1/bundle/artifacts/native/trial-result.json) | `task-package__qqhcYZp` | 0 | `c94fea4a6d5d0ff0dd759cd0451f44b4f1dd35ebab22725f4bd180b613526388` |
| [trial-candidate-opi-head-proxy-fixed-abs-module-cache-flags-r1](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/runs/deepswe-v1.1/trials/trial-candidate-opi-head-proxy-fixed-abs-module-cache-flags-r1/bundle/artifacts/native/trial-result.json) | `task-package__WpfaU3B` | 1 | `c0156c3cf2608f60debd301bade0025e5364866d56fcc00619787b02118142d3` |


TB 2.1：两个 Agent 均退出 0，在各自真实共享任务环境中通过全部 6 项原生检查。pi 执行 9 次工具（7 bash、1 write、1 edit），Opi 执行 5 次 bash，证书与验证产物获得原生评分。pi 相比上轮的成绩变化是本轮观测，不能归因于 DeepSWE 的代理修复。

TB 3.0：两端均退出 0，均通过 0/5 项测试。声明目录 `/app/evalbench/` 收集到 `app/evalbench`，交给独立验证器；pi 收集摘要为 `07f9891814dcfbd40a178e04084d6f6fbbb5f6bb2fcb849aaac4cfae612d24fd`，Opi 为 `9fafdfb3805cde053c0b8794a9d63998aa8dc6f9656e65a14d42913b271935a7`。pi 执行 24 次工具（6 bash、18 read），最后响应仅含 thinking，`stopReason=length`、输出 8192。Opi 执行 16 次 bash；进程退出 0，但 session 记录一条 error diagnostic，本轮未进一步诊断其具体原因。目录摘要本身不能证明哪些源码编辑发生过。[pi 语义核对](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/checks/tb3-pi-semantic-review.json)和原生测试记录保留实际边界。

DeepSWE：两个 Agent 均运行并取得可归属的原生奖励 0，测试通过 3/23（`f2p=0/20`、`p2p=3/3`、原生 partial 为 0.13043478260869565）。pi 执行 33 次工具（25 bash、8 read），退出 0，最终在 8192 响应上限处结束，只有 thinking。Opi 在 50 个 provider turn 中执行 71 次工具（45 bash、8 read、2 write、16 edit），随后退出 1，记录 `max turns exceeded (50)` / `NonZeroAgentExitCodeError`。这是有效任务失败，不是基础设施故障或超时。Opi session 中字面 `turns=0` 与 `provider_turns=50` 同时存在，不代表没有模型调用。

两端官方收集的已提交补丁均为空，SHA-256 为 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`，绑定基线 `cb1b3b671d0ee9fa9da9f7b02f86967953ffd10a`。禁止联网的 pristine 独立验证器消费该实际补丁，镜像为 `public.ecr.aws/d3j8x8q7/swe-bench-202605:kh75679ajj3b8dtd7se3h7z0a1833y6r-v1.1`。没有将 Opi 未提交的任务工作重建为另一份补丁或重新评分。[pi](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/checks/deepswe-pi-semantic-review.json) 与 [Opi](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/checks/deepswe-opi-semantic-review.json) 核对记录保留来源观测；分数仍由原生 grader 决定，这些核对属于操作诊断，不是独立模型审计。

## 用量、时间与费用


| Family / 家族 | Agent | Trial seconds | Agent seconds | Provider turns | Tool calls | Input | Output | Cache read | Cache write |
|---|---|---|---|---|---|---|---|---|---|
| terminal-bench-2.1 | pi | 127.40 | 97.70 | 8 | 9 | 16455 | 4205 | 17280 | 0 |
| terminal-bench-2.1 | opi | 265.91 | 238.01 | 6 | 5 | 33914 | 9797 | 19648 | 0 |
| terminal-bench-3.0 | pi | 301.89 | 262.98 | 10 | 24 | 29875 | 9275 | 86016 | 0 |
| terminal-bench-3.0 | opi | 381.52 | 343.26 | 10 | 16 | 30309 | 10445 | 97728 | 0 |
| deepswe-v1.1 | pi | 492.71 | 456.80 | 19 | 33 | 42100 | 19182 | 208192 | 0 |
| deepswe-v1.1 | opi | 1161.18 | 1126.08 | 50 | 71 | 62452 | 37716 | 1396160 | 0 |


用量来自 pi 已完成 assistant 响应和 Opi session/trace 汇总，工具计数来自实际事件流。[明细](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/reports/metrics-detail.json)保留分阶段时间和原生测试路径/摘要。这些是报告的 token/cache 观测，不是账单或跨客户端效率结论。pi 配置中的费用零值是占位值，实际账单费用未知；传输重试、压缩计数因未完整测量而保持未知。

## 验证与隐私

以下命令展开了实际记录的目录、家族与后缀；[命令记录](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/commands)和[外部检查](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live-checks)保留完整绝对 argv、cwd、时间、退出码及 stdout/stderr。这些是历史执行记录：canary 已删除，不得再次启动这些已使用运行目录。

```sh
R=/home/luiz/opi-eval-runs/20260910t175447z-proxy-fixed-acceptance
E=/mnt/e/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live
C=/mnt/e/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live-checks
REPO=/mnt/d/Luiz/Odradek/opi
# cwd=$R; f=terminal-bench-2.1, terminal-bench-3.0, deepswe-v1.1
$R/builds/opi-eval doctor --config $R/inputs/$f.toml
$R/builds/opi-eval prepare --config $R/inputs/$f.toml --out $R/materials/$f
$R/builds/opi-eval validate --config $R/inputs/$f.toml --native-material $R/materials/$f/material.json
$R/builds/opi-eval plan --config $R/inputs/$f.toml --native-material $R/materials/$f/material.json --out $R/plans/$f.json
$R/builds/opi-eval run --plan $R/plans/$f.json --root $R/runs/$f --fixtures $R/unused-fixtures --canaries $R/private/live-canaries.txt
$R/builds/opi-eval verify --root $R/runs/$f
# s=a,b
$R/builds/opi-eval report --root $R/runs/$f --out $R/reports/$f-$s.json --canaries $R/private/live-canaries.txt
# cwd=$REPO
$R/builds/opi-eval verify --root $E/runs/$f
# s=c,d
$R/builds/opi-eval report --root $E/runs/$f --out $C/$f-$s.json
python3 $REPO/scripts/opi-artifact-audit.py $E/opi-artifact-audit --workspace-root $REPO --json
# o=acceptance.json, acceptance-rechecked.json, acceptance-post-cleanup.json
python3 $REPO/crates/opi-eval/scripts/verify-local-acceptance.py --root $E --out $C/$o --companion $R/builds/opi-eval
```

三个家族的 doctor、prepare、最终 validate 和 plan 均退出 0；前述首次 DeepSWE validate 的退出 1 保留。原目录 A/B 与暂存副本 C/D 报告重建一致：


| Family / 家族 | run | verify original / staged | report A/B/C/D | A=B=C=D SHA-256 |
|---|---|---|---|---|
| terminal-bench-2.1 | 0 | 0 / 0 | all 0 | `3f6d87f2ab6a1bf8d3ea5a08362e19a361463d4bae8ef6311c4ddf4d3354ec40` |
| terminal-bench-3.0 | 0 | 0 / 0 | all 0 | `a7661edc32b3439709c7a70dc3a3365dc2563e3e6b8d46886b9177bf2d4befef` |
| deepswe-v1.1 | 0 | 0 / 0 | all 0 | `f53972e1e7a2fafc6dbe3b23c5ccb44e14b76b80227c49425566723e5d19f8f8` |


首次验收命令退出 2、`accepted:false`，原因是派生的操作 manifest 将凭据引用元数据混入公开 connection 对象，导致与封存实验严格比较失败。核对六份封存 connection 后，只移除了该派生对象的额外字段。[修正记录](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live-checks/manifest-connection-correction.json)、`manifest-before-connection-fix.json` 和初次拒绝全部保留，没有改变任何原生封存字节。第二次与清理后最终验收均退出 0、`accepted:true`。

Opi 审计视图精确包含三个完整 native 捕获目录，每个 7 文件，共 21 文件，已与原件核对目录成员和摘要。`opi-artifact-audit.py` 退出 0、`ok:true`、issues 为空。其已识别 `ndjson` 和 `sessions` 数组为空：它检查的是不透明 native 捕获视图，不能据此证明完整 provider/tool/session 语义。实际 Agent 事件、任务效果和原生交付另行核对。该工具不审计 pi 来源，也不能代替独立 Phase 验证。

家族 canary 检查覆盖 DeepSWE 110、TB 2.1 91、TB 3.0 208 个文件，所选 provider 凭据命中均为 0，暂存选择也在清理前完成扫描。[最终隐私检查](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live/checks/privacy-final.json)未发现私钥文本标记，记录 137 个文件含操作路径引用。任务生成材料、作者元数据和运行时代理配置保持原样。因此 `operator.redacted=false`、`public_shareable=false`；结构 `accepted:true` 不表示脱敏交接已经就绪。离线验收检查未重新读取用户 `.env`，本轮未请求或执行 regrade。

## 清理与仓库验证

[清理](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live-checks/cleanup-final.json)状态为 completed：Docker 容器和卷均为 0，网络只有默认 bridge/host/none，12 个基线镜像 ID 全部保留，原生进程均已退出。已删除临时 canary、其 private 目录、未使用临时目录、一次性 operator HOME 和三份 Docker Desktop 客户端诊断日志。原任务镜像标签均已存在，无需恢复。冻结 runner HOME 依赖、精确二进制、任务材料、原证据和共享 Docker/Cargo 缓存保留；用户 `.env` 保持不变。原生清理与共享镜像层无法提供可归属的空间计量，回收字节数为 unknown。

清理后 418 个本轮保护文件及 570 个暂存保留文件摘要全部一致。[旧证据核对](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live-checks/prior-evidence-unchanged.json)确认此前发布版、诊断、HEAD、修复证据零差异。清理后离线验收仍通过。原始 `inventory.json` 是较早的 568 文件暂存快照，最终 570 条目 manifest 为最终依据，没有重写旧快照。

本次请求只新增本中英报告对，保留此前 README 修改和十份未跟踪报告。没有运行时/测试源码、实施账本、审计归档、提交或暂存操作。测试影响为 **none**。最终 `python3 scripts/opi-doc-check.py`、`git diff --check`、HEAD 和工作区清单记录于[文档检查](E:/opi/evaluations/20260910t175447z-opi-head-pi0851-proxy-fixed-acceptance-live-checks/final-doc-checks.json)。本次仅文档改动，未运行 Rust workspace 门禁。

## 验收映射与剩余工作

| 标准 | 状态 | 证据或剩余边界 |
|---|---|---|
| P19-AC-001，本地执行/报告路径 | evidenced | 三个已准备家族计划、六次原生执行与离线报告重建。 |
| P19-AC-003，真实部分 | evidenced | TB 2.1 工具效果与实际共享环境原生评分。 |
| P19-AC-004，真实部分 | evidenced | 两份 TB 3.0 声明目录收集及独立验证器归属。 |
| P19-AC-005，真实部分 | evidenced | DeepSWE 基线绑定的已提交补丁收集与 pristine 评分，保留空补丁/有效零分。 |
| P19-AC-006，真实部分 | evidenced | 两种选定客户端在所有家族使用冻结请求连接；Opi 实际 wire 身份明确保留未知。 |
| P19-AC-012，六份原生记录 | evidenced | 六份有效选定记录、原封存、命令记录与离线结构校验。 |
| 正式脱敏交接、独立任务 19.11 与 Phase 门禁 | missing | 当前目录仍私有且未脱敏，尚未完成独立审计与 workspace 验证。 |

[opi-eval 验收交接约定](../../.agents/skills/opi-eval/references/local-acceptance.md)规定实施 harness “may only consume redacted evidence offline”。下一步是保留原封存，准备并验证派生脱敏交接，再以审阅后的目录和 manifest 摘要独立继续任务 19.11。更广泛的原生见证与 workspace 门禁在此保持未验证。本报告不授权进一步付费执行。

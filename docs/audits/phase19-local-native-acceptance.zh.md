# 第 19 阶段本地原生 Agent 验收审计（P19-D08）

[English](phase19-local-native-acceptance.md)

## 当前提供证据与 apply 结果（2026-10-14）

**操作员证据：已验证。原生闭合批次 B1：Not closed。** 当前注册准则要求
TB2.1/TB3 共四条真实试跑；DeepSWE AC-005/D04 为 deferred/reserved。
当前原始证据根为 `/home/luiz/phase19-acceptance-r2`，保留的四试跑视图位于
`/mnt/c/Users/Luiz/AppData/Local/Temp/opi-phase19-evidence-continuation-lcprjvy7/operator-f2-evidence`，
manifest SHA-256 为
`936e722ae9d2c54a5940a9d83c48a5db10eeceab7bccf1a913d4f01ace787e14`。
下方六试跑历史记录仍归属于原日期与合同，不决定当前范围或原生闭合。

本次源码绑定 Companion 对应已提交源码
`9ef3768968a559e0bc0366c1f3c66752d499595a`（test-only apply 未改变生产字节），
SHA-256 为
`1406e98d638354899fab559d24a75dc313f1bf47448f08b48dd9806253f258e2`。
locked/offline musl 构建、公开验收校验器、两个家族的 verify、每家族两次 report
重建，以及对两份 Opi 捕获穷尽的制品审计，全部 exit 0；重复报告字节相同。
真实 pi/Opi reward 分别为 TB2.1 1/1、TB3 1/0，完成工具对数分别为 13/7、61/75；
两个配对均 comparable。本次 apply 未授权或执行新的真实 provider 试跑。

新命令、原始退出码与原始哈希保留于 `/home/luiz/opi-phase19-apply-c368e42d5144468ea33b93d233c25c6d/operator-verification`，
`receipts.json` 绑定各次调用。已重新核对视图声明的 117 个保留哈希。
原始尝试单独保留：28 条已封存试跑、24 条非主试跑、六条范围外 DeepSWE
试跑；源 bundle 与原始分数均是保留对象。

## 当前原生执行与具体阻塞

批准计划摘要为
`571c76317637ab7acca5661f3c381dd8e7482648a9b8a8008d40c42e2a90e237`，
绑定 live audit index
`23b39dcdc52f71dc1b17b4f87678f39aa3d86289c3753eb0b5f55373ad82873e`。
独立目录的公开 doctor/prepare、确切 Opi 0.8.2/pi 1.0.4 运行包与构建回执、
Harbor 0.22.0/uv 0.9.7、当前 capture helper 和钉定 TB2.1/TB3 输入保留于
`/home/luiz/opi-phase19-apply-c368e42d5144468ea33b93d233c25c6d`。最终所选家族 material/7 合并输入摘要为 `c29d45781e10a271df1399edc964b5baf4bb6b212c1634b6a9fc07cb3a66274a`。
任务、基础镜像、构建上下文、模式、argv 和输出镜像身份保留于
`image-preparation` 与 `image-preparation-sidecar`。

完整清单含 138 个测试、14 个 ignored 原生测试；选中了十二个 exact 原生过滤器。
串行派发在第二项失败后停止。当前所选执行为：

| 确切过滤器 | Cargo exit | 完整 transcript SHA-256 |
|---|---:|---|
| `native_tb21_artifact_witness` | 0 | `87cefd490d1ec5617b560113e1798a75421603f09bb6d5d235e5f13b813e2ee5` |
| `native_tb3_artifact_witness` | 101 | `52026101f4423d2d99fc4838676e5a642dbd2ff573a05867906862e5329cf829` |

TB2.1 实际执行一个 libtest case，以及官方 oracle/nop 两个作业，原生 reward
为 1/0。防空跑校验器确认一项 passed、零 failed、零 ignored、137 项 filtered。
每次调用保存完整独立 stdout/stderr 及其哈希；完整 transcript 逐字节拼接两条
流，不裁剪输出。更早的合并流成功运行与计数器拒绝记录也单独保留。

TB3 实际执行一个 libtest case 和两个官方作业。Oracle 评分 1，并收集任务唯一
声明的 `/app/results.txt`；测试现绑定这一确切规范声明。Nop 在上游评分 0，
但未提交声明文件，生产 reader 因而以
`native-binding-declared-artifact-missing` 拒绝该结果，case exit 101。
要求的有效错误制品观察与不同 collected digest 仍缺失。原始基线回执还保留了
更早的 runner 缓存准备拒绝，以及实际复现的旧 `/app/evalbench/` 断言失败。

当前原生 conformance 仅接受 oracle-witness/nop-witness。把缺失文件当作有效
制品会违背 AC-002 的缺失/替换制品拒绝准则。可提交确切错误制品的见证需在
批准的四路径测试/文档范围之外重新规划；生产代码、公开 schema、任务脚本、
账本、lockfile 与客户端运行包均未编辑。停止后未执行其余十个原生过滤器，
包括新加入的 native_real_client_offline_regrade；其四来源、八请求、四次 fresh
评分谓词没有绿色结论。

`docs/snapshots/phase19/assurance/` 的 result 文件对记录源 key 的确切覆盖、
最终离线 workspace 检查及落地边界。B1 仍为 Not closed；本文不授予 Phase
符合性。测试影响：add 一个 ignored 真实客户端 regrade case；update TB3
来源断言；retain 既有普通/原生/兼容测试；delete 无。

## 保留的六试跑历史记录（2026-09-15）

以下文本保留较早验收、尝试与局限的原始身份；其中 verdict 和旧门禁声明均为历史记录。

**结论：已接受。** 六条真实本地操作员试跑——terminal-bench-2.1、
terminal-bench-3.0 与 deepswe-v1.1 各一条 Opi、一条 pi——经 Zhipu
`glm-5.3-flash` 的 Anthropic-messages 连接执行，获得可归因的原生评分、
封存 bundle，并被离线本地验收校验器完整重建（`accepted: true`，exit 0）。
Opi 专属的限定制品审计非空、穷尽且逐字节绑定，通过。全部失败尝试连同
替换身份保留在操作员运行根目录下。这些是每家族一任务、一次重复的原生
结果，不是语料库级或统计性比较。

本审计只记录脱敏事实。此处与暂存证据根中均不含任何凭据值、真实线上
报文或私钥。

### 最终证据与确切身份

- 脱敏证据根 `E`：
  `/mnt/d/Luiz/Odradek/phase19-operator/evidence-root`（与本仓库检出同级的
  `phase19-operator` 操作员工作区；Windows 路径
  `D:/Luiz/Odradek/phase19-operator/evidence-root`），manifest schema
  `opi-eval-local-acceptance/1`，manifest SHA-256
  `c7c0a2556b4e5d1dbe5626165590f5e831a218499fdc1dbc379d7029f214731a`。
  已登记于历史索引 `docs/eval/evidence-roots.jsonl`（现已从检出中移除，保留于 Git 历史）。
  验收当时（2026-09-15，companion SHA-256
  `f8bf959c1428c3e10c88a7f6ebcc37166043b3d1f3f74c40809a0f30775a861b`）的校验器
  接受了该根；当前校验器会拒绝它，因为它的 material/4 manifest 与 /1 native
  terminal 早于 material/6 和 terminal-preflight /2 合同——它作为历史验收证据
  保留，不作为当前可消费输入。
- 运行根（含保留的失败尝试）：
  `/home/luiz/.cache/opi/phase19-setup-repair/runs/{tb21,tb30,deepswe}`。
- opi agent：静态 musl 可执行文件，SHA-256
  `c89ac4a103c6cf0f433469ae45dc6baea03328c3937b8c412f67e8f0457ff8de`，
  本仓库 `55fbc95009e6f47145196dbc539454ebcb1cf692` 的 locked 离线
  release 构建。
- pi agent：SHA-256
  `f4db98b5f0ec4720a9be6d8e04971076f84050d365b066edca1e3cb84c6ed6ec`，
  `@earendil-works/pi-coding-agent` 0.85.1 运行包，源码 commit
  `d981de1229ef899957bbe968bc8dcda02a21f477`，自带 node 22.19.0。
- 来源（manifest 中带仓库限定）：pi
  `https://github.com/earendil-works/pi` @ `d981de12`，harbor
  `https://github.com/harbor-framework/harbor` @ `4407eb52`，pier
  `https://github.com/datacurve-ai/pier` @ `c1ebc6d1`。
- 材料（`opi-eval-native-material/4`，origin `local`），SHA-256：
  terminal-bench-2.1 `b02ad3dd3b948bb044c1ba4ee89560a83905afb3d76e7ef85f05964a3bc9c129`、
  terminal-bench-3.0 `03cc326f8e8831f626795822712ffd3d84e299df97e14bde694f60e407bbd760`、
  deepswe-v1.1 `35ebd3dc119d7da7d72bda6615bf66d6824f0309284284c8704b3f71fb4bab7a`。
- 保留的运行报告，SHA-256：terminal-bench-2.1
  `15bb5a4bb50498f6784201b4db9054d00bf0ad75376747d8162ce8013f31ca61`、
  terminal-bench-3.0
  `7ea5abb767d3632f4bb392c80fa4f3458eff30b80279fdc7f4e56ab1772db39a`、
  deepswe-v1.1
  `674b198cb77ba7c01c723c719f4cc66cec2210f97c4ee1e7d34066ad0d08e5d2`。

### 冻结范围

| 项目 | 值 |
|---|---|
| 连接 | `zhipu-anthropic`：provider `zhipu`，api `anthropic-messages`，`https://open.bigmodel.cn/api/anthropic`，model `glm-5.3-flash`，credential env `ZHIPU_API_KEY`，mode `live` |
| 模型控制 | context_window 131072，max_output_tokens 4096，reasoning/temperature/top_p 省略，无 required 控制 |
| 执行 | 并发 1，每个原生作业 900 s，stdout/stderr 各 1 MiB 上限，每产品一次重复，共六个槽位 |
| 任务 | tb21 `openssl-selfsigned-cert`，tb30 `batched-eval-parity`，deepswe `abs-module-cache-flags`（每家族一个已准入任务包，上游已钉定） |
| 输出控制见证 | 每个最终 bundle 内封存 `control/output-control.json` |

凭据仅由操作员在自己的 shell 中导出。任何 harness 运行都没有读取
`.env` 文件；暂存的任何文件都不含凭据值。

### 原生结果

指标为 benchmark 定义的原生评分器 reward。原生零分与 benchmark 有效
的 agent 失败都保留在分母中。

| 家族 | 任务 | pi 试跑 / reward | opi 试跑 / reward | 配对 |
|---|---|---|---|---|
| terminal-bench-2.1 | openssl-selfsigned-cert | `trial-baseline-pi-openssl-selfsigned-cert-r1.r2` / 1 | `trial-candidate-opi-openssl-selfsigned-cert-r1.r2` / 1 | comparable |
| terminal-bench-3.0 | batched-eval-parity | `trial-baseline-pi-batched-eval-parity-r1.r2.r2` / 0 | `trial-candidate-opi-batched-eval-parity-r1.r2.r2` / 0 | comparable |
| deepswe-v1.1 | abs-module-cache-flags | `trial-baseline-pi-abs-module-cache-flags-r1.r2.r2` / 0 | `trial-candidate-opi-abs-module-cache-flags-r1.r2.r2` / 0 | comparable |

六条试跑的 completion 均为 `verified-native`。tb30 与 deepswe 的四条
均为原生零分——有效的已评分结果，不是认证、基础设施、工具循环或交接
失败。每条 agent 记录绑定执行产品、`zhipu:glm-5.3-flash`、可执行文件
摘要与交付/收集的制品摘要；tb30 使用声明的独立 verifier 与声明式收集；
deepswe 在纯净 verifier 中将提交补丁摘要绑定到声明基线。

### 保留的失败尝试与替换

所有失败尝试在运行根目录下保持持久；替换通过 `--replacement-for`
在新身份下执行。

1. r1（2026-09-15T09:43Z）：tb21 双 agent 都已执行；pi 因缺失主题资源
   exit 1，配对以 `unsupported-control:max_output_tokens` 结算；
   tb30/deepswe 在该批次未启动。
2. r1（2026-09-15T12:09Z）：tb30/deepswe 尝试在 `integrity` 边界死亡
   ——runner 在自身工作目录被删除后以相对 `jobs/...` 操作数调用
   `docker compose cp`；agent 从未被调用；试跑未封存。
3. r1.r2（2026-09-15T12:50Z，仅 tb30/deepswe）：双 agent 封存并获得
   原生评分，但配对可比性仍为 `unsupported-control:max_output_tokens`
   （尚无封存的输出控制见证），运行 exit 1、incomplete。
4. 最终（2026-09-15T19:56Z）：三次替换——tb21 `r1 -> r1.r2`、
   tb30 `r1.r2 -> r1.r2.r2`、deepswe `r1.r2 -> r1.r2.r2`——在输出控制
   见被证明并钉入三份材料之后执行。三个运行 exit 0、outcome
   `completed`；所有配对 comparable。

为诚实起见记录一次 manifest 修正：暂存时 `command_exits` 的 run 条目
仍携带第 3 次尝试的陈旧 exit 1；已更正为最终批次的真实值 0，离线
校验器重新接受（exit 0）。

已发布的报告视图通过每次执行只读 staging 重构：为每个家族创建新 root，
逐字节复制恰好 `run-control.json` 及其 `planned_trials` 为最终执行声明的那些
trial 目录；对每个 staging 副本运行 `verify` 与 `report`，输出写入 staging root
与原始 root 之外。整个混合运行 root 仍然 fail-closed（`report` 在其上 exit 1），
所有被取代的尝试保留在原始 root 中，原始 `run-control.json` / `run-report.json`
字节不变。

### 操作员启动修复（试跑前，无模型调用）

记录于 `phase19-operator/setup-repair/`：DrvFs 小文件上传超出 Harbor
360 秒 setup 上限（pi 运行包移至 WSL 原生 ext4；setup 降至 2.3 秒）；
Docker Compose 从已删除 cwd 启动（稳定 cwd 启动器 + 脱敏诊断）；pi
启动脚本引用宿主机绝对 node 路径（node 22.19.0 放入包内）；opi 的 GNU
二进制需要 glibc 2.39 而容器只有 2.36（locked 离线 musl 静态构建）；
`compose cp` 相对操作数通过回溯 `verifier-trace` 祖先恢复（端到端回归
测试 3/3）；pi 运行包补齐缺失的 `dist/` 资源；本地回环输出控制探针
（零凭据）证明两个钉定客户端在初始与工具延续请求上都遵守
`max_output_tokens`，该控制之后才被接受为可比性证据。

### 执行过的验证

- `python crates/opi-eval/scripts/verify-local-acceptance.py --root <E>
  --out <out> --companion phase19-operator/bin/opi-eval` →
  `accepted: true`，exit 0；列出全部六条试跑与三个家族；companion 仅在
  凭据剥离环境下对提供的家族根目录调用。
- `python scripts/opi-artifact-audit.py --workspace-root .
  <E>/opi-artifact-audit` → PASS；Opi 专属通用审计视图非空、对三条
  Opi 试跑捕获穷尽、与原件逐字节绑定。
- `cargo fmt --check --all` 与
  `cargo clippy --workspace --all-targets -- -D warnings`：绿。
- `cargo test --workspace --all-targets`：除既有的 `spec_ledger` 哈希
  检查外全绿——该检查将账本钉在 HEAD，而一个无关的并行修复会话持有
  未提交的 `docs/opi-spec.md` 修改；非本证据造成。该编辑落地后重跑
  完整 workspace 门禁。
- 操作员平台为 WSL2 Ubuntu，Docker engine 位于
  `unix:///var/run/docker.sock`，镜像按钉定 manifest 摘要拉取。

### 局限

每家族一任务、一次重复、共六条试跑；原生零分是有效的已评分结果并计入
分母。`glm-5.3-flash` 选择器背后的真实后端模型仍未知；未捕获真实线上
字节。输出控制声明由针对钉定客户端的本地回环探针证明，而非针对线上
端点。此处没有任何语料库级或统计性比较；Phase F 仍将独立重建准则
追溯，并保留其自身的人工归档门。

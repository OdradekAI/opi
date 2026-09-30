# 2026-09-29:Opi 0.8.2 / Pi 0.85.1 glm-5.3-flash 验收重采集

([English](20260929t-opi082-pi0851-glm53flash-acceptance-recollection.md))

用户授权的六试验 live 验收采集,使用最终应用树,按当前证据合同封存(native material
`opi-eval-native-material/6`、agent terminal preflight `/2`、以及准入绑定的验收
来源校验器)。对当前消费而言它取代 2026-09-15 的验收根;该历史根另行保留并登记。

## 证据

- 证据根:`C:/Users/Luiz/.local/state/opi-remediation-evidence/phase19-acceptance-20260929/evidence-root`
  (WSL:`/mnt/c/Users/Luiz/.local/state/opi-remediation-evidence/phase19-acceptance-20260929/evidence-root`),
  已登记于 [evidence-roots.jsonl](evidence-roots.jsonl)。
  Manifest schema `opi-eval-local-acceptance/1`,SHA-256
  `9897f08fed7876b757542235e77e54735d233b1d6b0d82ec5024eab26fc6a9ce`。
- 客户端:Opi 0.8.2 静态 musl(最终应用树,executable SHA-256
  `0ccc2b7f5be5257068c1d3bb8cb73dfeaa0e482c16b468872179a0b1720e2ad2`,构建绑定源码树
  manifest `5e2f8220…`,脏状态披露见 manifest);pi 0.85.1 运行包来自固定本地 cache。
- 材料:三份 `opi-eval-native-material/6` manifest,由最终公共 `prepare` 生产者产出,
  两个客户端的 exact-client 输出控制证明(`max_output_tokens = 131072`,初请求与
  工具续接)已钉入。
- 连接:zhipu / anthropic-messages / glm-5.3-flash,每家族每 subject 一次重复,并发 1,
  无自动付费重试。凭据仅从操作员 `.env` 读入 trial 子进程环境;staged 根中其值出现
  次数为零(已扫描)。

## 原生结果

| 家族 | 任务 | pi | opi | 备注 |
|---|---|---|---|---|
| terminal-bench-2.1 | openssl-selfsigned-cert | reward 1 | reward 0(agent 失败,已评分) | 双双密封 |
| terminal-bench-3.0 | batched-eval-parity | reward 0(agent 失败,已评分) | reward 0(agent 失败,已评分) | 双双密封 |
| deepswe-v1.1 | abs-module-cache-flags | reward 1 | reward 1 | 双双密封 |

均为有效的已评分原生结果(含 benchmark 有效的 agent 失败);接受集中不含任何
provider/auth/基础设施失败。

## 保留的失败尝试

- `runs/tb30`:opi trial 被外层监督超时取消时 job 存活且仍在流式输出(pi 已密封,reward 1)。
- `runs/tb30-attempt2-cancelled`:30 秒启动探针,已取消。
- `runs/deepswe`:pi trial 触及固定 native job 截止(`native-job-timeout`;opi 已密封,reward 1)。

接受的 tb30/deepswe trial 是按文档化 replacement 流程的新身份重跑(`r1.r…`);
每个失败尝试都保留在采集的 run 根下。

## 离线验证

- `verify-local-acceptance.py --root <E> --out <out> --companion <最终 companion
  d2502daf…>`:exit 0,`accepted: true`,六个 trial 与三个家族全部绑定(准入
  static-lock 来源、每家族材料身份、密封的 `/2` terminal preflight 观测)。
- `scripts/opi-artifact-audit.py <E>/opi-artifact-audit`:PASS(`ok: true`),
  对三个 Opi trial 捕获非空、逐字节绑定且穷尽。
- 每家族密封 run 根上 `verify` 与 `report` 均 exit 0。

本报告只记录证据,不构成任务完成权威:归属工作流的独立门与审计集决定 phase 状态。
C/P19-AUD-011(原始固定离线 regrade)仍是独立的、未被绕过的排除项。

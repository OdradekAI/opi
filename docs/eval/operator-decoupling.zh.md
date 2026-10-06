# Eval operator 解耦方案

[English](operator-decoupling.md)

**状态：仅修订设计，2026-10-06。** 当前已实现的 operator 仍依赖仓库。
运行包、源码迁移、新配置与交接字段，以及扩展诊断采集均为拟实施工作。
本文记录项目中的迁移归属；技能的[操作契约](../../.agents/skills/opi-eval/references/operator.md)
和[诊断清单](../../.agents/skills/opi-eval/references/diagnostics.md) 承载可复用流程。
已经完成的[上一轮实施计划](../superpowers/plans/2026-10-04-opi-eval-operator.md)
保留为历史证据，不作为本次修订的目标架构。

## 依据与权限

诊断要求依据用户提供的本地 pi 1.0.2 evals 分析，来源路径为
`docs/research/pi-1.0.2/packages/evals.md`，
特别是第 5、7、9、11 节：执行前计划全部任务，保留部分失败，测量真实提示词和用量，
保存原生结果与会话，并区分缺失指标和零值。第 9 节还区分分数有效性与会话可用性，
以及表现统计与所有已观察尝试的实际消耗。这些是设计依据，不要求复用 pi 私有
harness，也不改变各 benchmark 的原生评分规则。
调研目录迁移属于独立工作；该路径是来源记录，不是本方案或自包含清单的必需文件依赖。

当前运行时和模板准入的是 pi 1.0.0。读取 1.0.2 调研既不更新该 pin，也不证明
1.0.2 适配器可用。每个已获取版本保留独立的源码、包、可执行文件身份与原始快照。
本方案不授权新的付费运行、pin 更新、源码修复或公开发布。

[规范](../opi-spec.md) PRIN-003、CTRL-004 至 CTRL-007 要求策略位于 Agent Core
之外、原生评分保持权威、证据不可变、公开打包接口保持暂定。沿用当前未发布的
Independent Companion 和 Python/Rust 进程边界。本地运行包不新增 crate、
Agent Core 依赖、公开适配器 ABI 或发布流程。数据集、镜像和原生 grader 实现仍是
独立获取并固定身份的外部材料；运行包提供集成代码与获取身份，不打包这些外部实现。

## 耦合清单与文件归属

| 当前依赖 | 迁移归属与目标 |
|---|---|
| `.agents/skills/opi-eval/scripts/operator_flow.py` 使用 `Path(__file__).resolve().parents[4]` 和隐式 `REPO` | 编排迁至 `crates/opi-eval/operator/`；配置分别绑定运行包与受测源码 |
| 技能中的 `evaluate.py`、`operator_config.py`、`operator_process.py`、`operator_evidence.py`、`operator_flow.py` | 五个文件整体迁入运行时源码树，组成有版本的可执行闭包 |
| 技能中的 `config.example.toml` 提供默认值 | 唯一模板迁入 `crates/opi-eval/operator/`，从安装的运行包初始化显式私有配置；调用方迁移后移除技能中的副本 |
| 技能 operator 直接打开 `external-locks/static/linux-x86_64.json`、benchmark profiles、integration/producer/proof 脚本 | 运行包拥有资源位置与摘要清单；所需 producer/helper 与入口一起打包 |
| 仅评测 pi 时仍清点/冻结 Opi checkout，并构建 Companion/helper；Opi 可执行文件只在 candidate 中构建 | 单独解析并冻结传入的评测运行包；Opi adapter 拥有当前源码的 Cargo 构建和清单，pi adapter 拥有准入版本包的获取与构建 |
| Operator 调用 `scripts/opi-cargo-cache.py` | Opi build adapter 拥有缓存协调；通过构建上下文传入共享 target/lease，必要时由 adapter 使用项目 helper，技能不直接调用 |
| 隐式 `INDEX = REPO / "docs/eval/evidence-roots.jsonl"` 和执行后追加 | 解析外部 `storage.index`；项目报告交接可显式把返回快照登记到 `docs/eval/evidence-roots.jsonl` |
| `native-run.md`、`evidence-root.md`、`local-acceptance.md`、fidelity 引用/脚本和 `SKILL.md` 包含 crate、脚本或项目路径假设 | 实现配方、资源位置迁入运行时文档和项目策略；技能保留可移植操作/诊断规则；保留 acceptance、fidelity、隐私、修复和显式调用边界 |
| `scripts/test_opi_eval_operator.py` 与 fixture 导入技能中的模块 | 保留测试并把 import/CLI 改到运行时源码；增加迁址、打包、诊断行为用例，不删除现有 44 项 operator 契约 |
| 准备后执行命令指向技能中的现行脚本 | 执行和离线校验绑定已冻结运行包入口与兼容契约，后续技能/源码修改不能替换已审阅字节 |

迁移必须涵盖原生可执行文件、capture helper、静态锁、benchmark profiles、集成、
producer/获取脚本、control proof、运行时依赖和兼容契约文档。闭包缺失任何必需资源
都应拒绝准备；不能仅移动入口，安装后仍从 checkout 导入文件。

项目源码修复、实施台账归属、双语结论和验收策略仍属于调用方上下文。把它们与
benchmark 执行分开时保留既有流程关卡。调用方可传入项目报告目的地，但缺少这些
目的地不能阻止仅 pi 的准备或离线比较。

## 配置与操作契约

所有可编辑选项集中在一个私有 TOML。保留模型、端点、唯一凭据来源、思考深度及
required、重试、并发、整批截止时间、benchmark 范围和 workspace/cache。
新增有版本的配置，包含操作契约定义的 `runtime.package`、`subjects.opi.source`、
`storage.index`、`diagnostics.required`。当前 `/1` parser 会拒绝这些字段；
现在不要把它们写入真实配置。

显式传入配置，按配置所在目录解析路径，再冻结脱敏后的有效值与来源。仅在已授权
的真实受测启动阶段解析凭据。运行时在 dispatch 前检查所要求的诊断能力；可选但
不可用的测量项保留明确缺口。

保留 `init`、`resolve`、baseline/candidate 准备、冻结后执行和离线 `compare`
这组小型进程接口。包元数据声明操作、交接、配置版本、支持的 pi pin/测试家族/
控制项、可执行限额和采集能力；它是描述信息，不是任意命令或插件注册器。
实现和 generated-help/subprocess 检查完成前，不承诺新的命令语法。

新增 `score_validity`、`diagnostic_completeness`、`comparison_eligibility`
时更新受影响的持久化契约版本。已有 `/1` 快照继续可读，原生封印不变。只在请求后
显式把旧私有配置转换为新配置；不将密钥复制进报告，也不默默增加必需诊断项。
导入发现引用而不重写原始证据，历史未测量事实保持 unknown。

## 执行与结果保留

1. 解析运行包和配置 -> verify：包内容、能力、路径分离、无凭据导出和所请求的准入限额。
2. 准备固定 pi 包或构建当前 Opi -> verify：运行包与受测身份分开，相关 dirty inputs、
   构建前后输入一致，冻结的产物/材料/计划，以及真实客户端控制证明。
3. 冻结完整预期清单和所选基线 -> verify：全部逻辑槽位、诊断要求、最大尝试数、
   原始截止时间，以及合格基线身份或明确 missing/incomparable。
4. 执行精确授权计划 -> verify：有界增量尝试记录、首次有效 settlement、失败尝试
   消耗和受支持的取消/清理；不自动调用 pi，也不重试合法零分。
5. 从原始材料重建并返回产物引用 -> verify：封印、确定性报告、独立诊断状态、
   原生/配对指标分母、不可变比较 manifest 和外部索引登记。

每次操作在源码之外保存冻结的运行包与受测包、脱敏输入、protocol/预期槽位、
observations/attempts、原生运行树、诊断产物、验证输出、快照和机器交接。
采集有界；会话、实际 prompt hash、逐调用耗时和最终工作区覆盖缺失时明确披露，
直到真实 adapter 采集提供证明。控制台输出和 `full` 标记本身不提供这些事实。

Pi 基线按准入版本和实际 benchmark、模型、控制、环境、grader 身份索引，
在 candidate 执行前确定性选择。Pi 更新版本后的全测试集刷新是显式操作，
不成为 Opi candidate 的副作用。连续 candidate 保持评测运行包固定；评测/评分
路径改变时，提供经审阅的等价证据或判为不可比。历史证据离线重新校验需要旧运行包
时，一并保留。

## 迁移顺序与验收

以下阶段均为计划，不是已完成的运行时工作：

- [ ] **打包现有执行闭包。** 迁移 operator 模块与模板，按包解析本地资源，保留
  当前离线契约。Verify：运行包迁址、缺失/变更资源拒绝、不支持契约拒绝，
  CLI help 和负向 subprocess 检查。
- [ ] **去除隐式源码/索引依赖。** 加入有版本的显式配置，分开运行包解析与真实
  pi/Opi 构建。Verify：无 Opi checkout 的 pi-only 准备；两个迁址后的 dirty Opi
  tree 使用同一运行包身份、不同受测身份；失败构建不复用旧产物；共享 Cargo cache
  lease 正确结束。
- [ ] **绑定冻结执行并迁移发现索引。** 保留原始证据并导入旧索引引用。Verify：
  移除源码/技能仍不破坏已准备的合成操作；离线比较不需要凭据/网络，源码可只读；
  原始封印不变。
- [ ] **增加由 adapter 证明的诊断能力。** 只实现所选客户端和原生 runner 实际提供
  的事实。Verify：全部[诊断用例](../../.agents/skills/opi-eval/references/diagnostics.md#verification-cases)、
  必需能力前置检查、失败的部分采集、隐私和大小限额、实际首轮/工具续轮证明，
  以及原生 grader conformance。
- [ ] **完成技能/调用方迁移。** 运行包/调用方检查通过后，移除技能中的运行时模块、
  模板和 checkout 配方，同步双语技能手册和 crate README。Verify：迁址后的技能
  仅读自身资源和返回的有版本引用，不导入产品内部代码；原有 acceptance/fidelity/
  repair 关卡和父级剩余预算监督仍有效。

执行需另外确认运行时变更范围。本次设计修订不包含台账写入、付费 benchmark、
发布或提交。实施时按实际修改面选择精确检查：脚本行为和 subprocess/包负向检查、
受影响 native integration/proof 测试、Rust 改动对应的 crate 测试/clippy，以及
文档和空白检查。Fixture 证明基础设施行为，不证明 pi/Opi benchmark 表现。

## 本次修订验证

本次仅修改文档。测试影响：**none**；既有运行时测试保持不变。文档影响：**add**
诊断参考和双语迁移方案；**update** operator、技能和手册说明；**delete** 无。
私有配置、benchmark 结果和封印保持不变。运行 `python scripts/opi-doc-check.py` 和 `git diff --check`，
在用户交接中报告实际结果。不为尚未实现的迁移声称 Cargo/运行时检查通过。

通用 skill-creator 校验器的白名单未包含既有 `disable-model-invocation`
frontmatter 字段，因此拒绝该字段。保留显式调用规则，仓库文档检查器接受它。
这是校验器兼容限制，不是本次新增的运行时缺陷。

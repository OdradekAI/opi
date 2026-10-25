# Reference Product 配置——交付来源候选

[English](delivery-candidate.md)

## 状态与权威

本候选物化用户已批准的[设计](../../superpowers/specs/2026-10-25-opi-ai-onboarding-and-config-design.zh.md)和[验收计划](../../superpowers/plans/2026-10-25-opi-ai-onboarding-and-config-acceptance.zh.md)。它**尚未在 opi-implement 登记，不属于当前 spec_files，不是已准入 Phase，也不代表 runtime 证据**。HOST-CFG、HOST-VALUES 是稳定来源行标识，不预占 Phase/task 编号。后续登记须选择 Phase、明确登记本双语来源、提取草案，经 P.0/P.2/P.4 后才能安装正式任务。

父[Opi 规范](../../opi-spec.md)约束 AUTH-003/004、GOAL-001/003、PRIN-001–005、PLACE-001–005、CTRL-001/003、INV-001/002、PHASE-001–006。本产品易用性工作沿 STRAT-001 独立 runtime 及 §4.2、§9 Parallel routes 的 Reference Product 组装路线。[CONTEXT](../../CONTEXT.md)约束 Human Authority、Reference Product、User Policy、Package Trust、Supervision。无需修改父规范、增加 Agent Core 模块或反转向内依赖。

## 问题、结果与放置

产品现有 TOML 词汇及 settings/provider 混合文档使 Pi settings/model 字段无法直接复用。仅改 parser 不能对齐模型选择、provider 定义、值来源及实际请求。目标是默认 TOML、显式 Pi JSON，共用产品拥有的一套语义和真实配置到请求路径。

放置为 **reference-product**：`opi-coding-agent` 已拥有 config/CLI/startup/trust/provider 组装。复用 `src/config.rs`、`cli.rs`、`main.rs`、`provider_factory.rs`、现有 credential backend 和可注入 SDK sources。`opi-ai`、`opi-agent` 均不增加产品文件 parser、命令 policy owner 或产品依赖。两种语法不产生两个 resolver；这是直接外部格式切换，不增加永久兼容 shim 或通用配置框架。

优先级：相应 Phase 20 owner 完成后消费已准入 SDK factory/native options；独立 SDK 在此前和没有本产品工作时均可使用。HOST-CFG 用 literal 合成凭据即可独立演示；HOST-VALUES 在其上构建。两者不加入 Phase 20 的 57 AIR / 54 V SDK exit，也不成为 20.57/58/59 前置。

## 固定比较输入与版本边界

规划基线：Opi `de703bb12a7602d0f128b22a7cbb4bb6c81f12ab`。向内比较为本地 `.repo/pi-1.4.0`，仅限配置字段及真实 loader。以下 SHA-256 固定本地字节，不是上游 Git revision。

| Pi 相对路径 | SHA-256 |
|---|---|
| `packages/coding-agent/schemas/settings.schema.json` | `d4b19b0e492cb887a23c50034f00213c476048956fa92c5cce2f1536ef50b9b0` |
| `packages/coding-agent/schemas/models.schema.json` | `86ade9716724902bdfe91347a728980ad7242a76d0773dbc7217ce4584d696ea` |
| `packages/coding-agent/src/core/settings-manager.ts` | `aef8fea51000f291c1c51d81fd9b30f7a2bdd0e68370ad84b969e5fd15667e98` |
| `packages/coding-agent/src/core/model-registry.ts` | `f4ced2d92df9097647aba7da3ae0bb273fa8089e966be15dba880db51d5dee0b` |
| `packages/coding-agent/src/core/resolve-config-value.ts` | `0f53dad47fe5d5d8837c022b7951ccd3bd5a9b577bd662f0986272110e83bcc7` |

选择这些配置字段不代表采用全部 Pi 1.4 行为。SDK 协议/option 默认仍遵循登记的 Pi 1.3 合同，直到其单独 source-delta 准入。Client identity、空白 tool-result 变化和无关产品新功能均在候选之外。未来执行前复核 loader imports 和相关 helper 身份，固定 schema 字节本身不能证明行为。

## 配置合同

- 默认 user 位置仍为 Windows `%APPDATA%/opi`、Unix `~/.config/opi`，trusted project 仍为 `.opi/`；settings 默认 `config.toml`，同目录可选 `models.toml`。
- 既有 `--config <FILE>` 显式选中 settings bundle：`.toml` 配 `models.toml`，`.json` 配 `models.json`；允许不同 settings basename。主文件误用 models、不支持扩展名或语法不匹配均失败；不自动发现 JSON，也不回退使用它。
- 默认 settings 缺失为空 settings layer，不跳过已存在 companion；显式 settings 缺失失败。Companion 缺失为空 model layer；存在但不可读/畸形/不支持时，整个选中 bundle 发布前失败。Bundle 选择不导入相邻 `auth.json`、packages、extensions、skills、prompts 或指令。
- 两种语法进入一个私有、保留 presence/provenance 的 typed resolver。保留 defaults→user→trusted project→explicit→CLI 顺序和仅无 `--config` 时有效的 `OPI_MODEL` 覆盖；CLI/env selector 原子覆盖 provider/model，普通 Pi settings 保留分立 `defaultProvider`/`defaultModel`。
- 直接采用 Pi 名称、嵌套和 camelCase。已知旧 Opi 字段（`defaults`、旧 `model`、`api_key_env`、旧 thinking 路径及嵌套形式）按位置/替代字段报错；无 alias、旧 reader、自动转换。
- 切换前形成完整字段/消费者清单：model/theme/thinking/trust 及语义相同 compaction/retry 的 Pi 等价；model 文档 provider/API/baseUrl/apiKey/headers/models/modelOverrides/compat/authHeader 及 metadata/cost；没有真实 Pi 等价的既有 limits、authority/execution/permissions/backend/keybindings 等归 typed `opi.*`。保留现有能力和省略语义。绝对 compaction threshold 不是 Pi reserveTokens；不把 Pi 独立 keybindings 协议隐式导入 settings。
- 已识别但未支持 Pi 字段明确失败；真正未知附加值保留、不生效且可见警告，不进入请求体。`$schema` 是 metadata，不触发联网；nested/map/array/clear 按 adopted loader 逐字段处理，不统一拼接数组，不用默认覆盖显式 false/zero/empty。
- 非 null 共同子集解析相同。TOML 无原生 null；schema 允许的 null-only 配置采用显式 JSON。`thinkingLevelMap` null 表示不支持，省略保留来源默认；不发明 null sentinel，不把省略转为 null。
- 保留 Opi project/resource/instruction trust 和 pre-trust staging。项目即使 trusted 也不能永久自授 execution permission。保留现有 credential-backend/keychain 归属；仅实际选中来源解析凭据，parse/construct/list/配置检查不执行命令或刷新 auth。
- 既有 UI 路径若保存配置，仅原子写选中文档并保留格式和未知无行为值；不重写 companion/另一格式；失败不报告持久化成功。

## 值来源合同

HOST-CFG 解析表达式形式但不执行；HOST-VALUES 尚未提供的形式明确报 unsupported，不发送 literal 凭据、不隐式回退。已有 SDK lazy default sources 继续独立可用。

HOST-VALUES 实现选中的 Pi model-value 路径：literal、`$NAME`/`${NAME}` 插值、`$$`/`$!` 转义、前导 `!command`。每个 logical request 冻结选中值/身份；下一请求重新解析，不复制 Pi 另一 helper 的 process-global cache。按来源处理 empty/error，错误 typed/脱敏，不重新发现账号。

命令归产品 source-resolution 生命周期：使用配置 shell 和正确 argv，选择时冻结 cwd/env，采用来源 10000ms 命令超时，传播取消并 supervision 整个进程树至清理，隔离晚结果。此超时不替代 auth-network/ACK 或 generation 时序。解析/配置检查无副作用；未信任项目不能执行命令。检查真实合同后复用已有 process/credential 机制，AWS credential-process 的 JSON 输出不是此处字符串输出协议。不迁移 auth.json，不替换 credential-store 实现。

## 来源行与任务登记规划

两个可独立演示的来源行须在将来的 Phase/source/graph 确认后成为分立任务，不分配 `20.60/20.61`，不改当前图。

| 字段 | HOST-CFG | HOST-VALUES |
|---|---|---|
| 交付 | 默认 TOML＋显式 Pi JSON、直接 Pi 字段、单 resolver、真实产品请求 | Pi model-value 来源进入真实 auth/headers，并拥有命令生命周期 |
| Placement / tier / evaluator | reference-product / workspace / true | reference-product / workspace / true |
| 实际前置 | 当前 SDK owner 已准入 factories/native options/callbacks；登记时解析准确跨 Phase 引用 | HOST-CFG 及已准入可复用 auth/process 机制；登记时明确引用 |
| 既有生产边界 | `config::stage_config`/`StagedConfig::finalize_with_project`/`resolve_config`→startup→`provider_factory`→真实 SDK adapter | 产品 provider/auth/header 组装→选中来源→真实 SDK adapter 请求 |
| 候选 owned paths | 产品 `src/config.rs`、`src/cli.rs`、`src/main.rs`、`src/provider_factory.rs` 及必要 startup/诊断/trust 消费者；字段清单中的准确 config/command 消费者；根/crate 双语产品 README、CHANGELOG | 产品 `src/config.rs`、`src/provider_factory.rs`、必要 credential/header source 消费者；双语配置指南和 CHANGELOG |
| 将来文件 | `crates/opi-coding-agent/schemas/settings.schema.json`、`schemas/models.schema.json`、`tests/fixtures/pi_config/**`、双语产品配置指南；必要时才拆私有 parser | 复用搜索确认缺口后，才增加 `crates/opi-coding-agent/src/config_values.rs`、`tests/config_value_sources.rs` |
| 既有测试归宿 | `config_tests`、`provider_factory`、`cross_mode_runtime`、`trust_resource_gating`、`early_command_trust`、`doctor_cli`、`list_models` 及受影响 execution/credential tests | 既有 credential/trust/factory tests，加上述不同的公共 command-source binary |
| 替换与上限 | 一次替换旧外部词汇，仅一个 resolver，无旧 alias；来源合法 Pi 配置无法表达/诊断或旧能力失去消费者时复审 | 单选中 source-resolution owner，无通用 shell/plugin 框架或全局 token cache；真实新 supplier 需要不同 lifecycle/authority 时复审 |

登记时盘点准确路径、runtime callers 和 test dispositions，再填写既有四项 minimum-change notes 及确有触发的 shared-decision notes。HOST-CFG 拥有 HOST-VALUES 消费的 presence/provenance 表示。语法 helper 私有，不能虚构生产消费者。保留 SDK selected-identity 规则，不复制 resolver。正式 task ID、完整依赖和全部必需字段在将来图 gate 派生，不在此猜定。

## 成功、退出与可执行场景

| 判据 / owner | 公共场景与预期效果 | 将来验证 |
|---|---|---|
| HCFG-01 / HOST-CFG | 成对 Pi-field settings/provider/model fixture 解析相同，驱动相同本地真实请求；literal 合成凭据足够。 | `cargo test -p opi-coding-agent --test config_tests --test provider_factory --test cross_mode_runtime` |
| HCFG-02 / HOST-CFG | 默认仅 TOML、显式 JSON bundle、仅 model 默认、较低 TOML/显式 JSON、缺失/错误/角色不符主文件及 companion 严守选择/原子失败，不回退。 | `cargo test -p opi-coding-agent --test config_tests` |
| HCFG-03 / HOST-CFG | 逐字段 merge、false/zero/empty/null、旧字段拒绝、unknown/unsupported 诊断及保留的 Opi 能力到达消费者，解析不触发 auth/命令。 | Config/factory 及字段清单确定的 execution/credential binaries |
| HCFG-04 / HOST-CFG | Trust、discovery/list/doctor、pre-trust startup 安全，项目不能自授权；产品确有写入时保证文档级原子保存。 | `cargo test -p opi-coding-agent --test trust_resource_gating --test early_command_trust --test doctor_cli --test list_models` 及受影响持久化测试 |
| HVAL-01 / HOST-VALUES | Literal/env/转义/命令进入真实 auth/headers；缺失/空/畸形/失败的选中来源按 typed 结果处理，不换身份。 | `cargo test -p opi-coding-agent --test config_value_sources --test provider_factory` |
| HVAL-02 / HOST-VALUES | Fake shell 成功/失败/stall/child、取消、下一调用 env/output 变化、未信任项目、晚结果竞态，证明 10000ms 来源范围、冻结调用、清理和逐请求无缓存。 | `cargo test -p opi-coding-agent --test config_value_sources --test early_command_trust` |

退出须两行均有真实生产路径证明、完整字段清单、双语 help/schema/manual 一致性、保留 authority/identity，并诚实记录缺失平台。文档/parser/token/substrate 测试不能关闭 runtime 成功。运行变更 binaries 及适用 workspace fmt/clippy/all-targets/doc/rustdoc，保留共享外部 Cargo cache，不 cargo clean。使用 fake clock、合成凭据、隔离 HOME/env、本地受控服务；执行 Pi fixtures 前检查 imports，不因发现凭据启用 live。

## 非目标、平台、风险与回滚

不选择完整 Pi 产品移植、JSON 自动发现、auth.json/keychain 迁移、新 CLI config-directory flag、旧 TOML 兼容、session format 变化、tool/permission 重设计、MCP/codemode/package 执行或新 SDK option 合同。Pi `.pi` 开发资源不是 SDK 配置或可执行验收输入。Host-only 不改变当前 SDK exit。

Native Linux/macOS/Windows 跟随 workspace manifest 和支持声称；shell/process 行为在宣传支持前须有 owner 的 native 证明，缺失平台仍缺失。身份/secret 泄漏、来源不明、语义 owner 重复、丢失 Opi 能力、trust 回归、父规范冲突时停止切换，不引入额外数字资源 cap。

改动保持可逆源码纵切，同步消费者/schema/fixtures/双语文档/Unreleased。除用户动作写选中文档外保留原配置；parse/resolve/save 失败不激活部分配置。已发布回滚须显式授权 revert/release，不静默转换用户 JSON/TOML、不恢复退役 reader、不声称远端副作用已回滚。当前规划不改生产/测试文件，也不声称 runtime 结果。

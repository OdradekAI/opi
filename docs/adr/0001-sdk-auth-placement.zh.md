# ADR-0001：SDK 认证机制归属（opi-ai 与 Reference Product）

- **状态：** 已接受 —— 2026-10-17 由 Phase 20 任务 20.1（DEL-01）记录。
- **性质：** 本 ADR 记录的是注册 Phase 20 交付来源（[spec-draft.zh.md](../research/pi-ai-opi-ai-alignment/spec-draft.zh.md)）中已选定的归属。它不引入要求、不改变运行时行为、不标记任何验收条件已满足。
- **被闸门任务：** 20.18（惰性 AuthContext、默认来源与管理结果）、20.22（store 拥有的变更 ACK／rotation／barrier 与宿主桥）、20.23+（逐 flow 的 SDK OAuth 迁移，先 Anthropic，后其余家族）。
- **权威：** [opi-spec.zh.md](../opi-spec.zh.md) §8.2 “产品与独立性别录” —— 跨层移动需要 ADR，归属案例需记录所有权、依赖、消费者、接口、符合性、权限、失败行为、生命周期、删除测试与迁移。

## 背景

1. Phase 20 目标是可独立使用的 `opi-ai`，认证 SDK 可达：AIR-023（独立于 coding agent 的可复用 SigV4/Bearer/profile/default-chain 来源）、AIR-046（构造与管理合同，构造零 I/O）、AIR-051（全部九个 OAuth 家族的 login/resolve/refresh）、AIR-063（store 拥有的串行变更/ACK 与凭据 mutation barrier）；验收设计为 V-09/V-39/V-43/V-55。
2. Opi HEAD `96b9709` 时的现状归属：
   - `crates/opi-ai/src/auth.rs` 只拥有产品中立值类型与注入 trait（`ResolvedAuth`/`AuthScheme`、`AuthResolver`、`OAuthCredential`/`OAuthLoginMethod`、`OAuthProvider`、`LoginPresenter`）。
   - Reference Product（`crates/opi-coding-agent`）拥有全部具体机制：三条 OAuth flow 实现（`AnthropicOAuthProvider`、`CodexOAuthProvider`、`CopilotOAuthProvider`）及 `src/oauth.rs` 中的 `OAuthProviderRegistry` 与 `login_oauth`；OS 钥匙串存储（`KeychainCredentialStore`，基于 `keyring-core` 与平台原生 store，service `opi`、presence service `opi.presence`）；跨进程串行化策略（`<user_config_dir>/credential.lock` 上单把独占 `fs4` 建议锁包住 write/delete/refresh，锁内刷新 HTTP 有界）；`CredentialResolver`（API key 钥匙串优先、env 兜底）；终端 presenter（`src/interactive_auth.rs`，经 `opi_ai::auth::LoginPresenter` 调用 `oauth::login_oauth`）；以及 `provider_factory` 的 `ProviderBundle`（`store`、`resolver`、`OAuthProviderRegistry`）与 harness/`main` 注入。
   - 当前消费者：上述 Reference Product factory/harness//login 路径；`opi-agent` 经 `opi-ai` provider 按调用冻结使用已解析 auth；产品套件（`tests/oauth_auth.rs`、`tests/*credential*`）固定了必须迁移后保留的 secret-canary、加锁与确认行为。
3. 与固定 Pi 1.3.0 快照的差距：九个 OAuth 家族缺六；store 事务缺 SDK 合同（串行 modify 且成功前完成凭据+元数据 ACK、`credential_mutation_barrier` 的 cutoff/receipt 捕获、诚实的 pending/unknown 报告、late-write fence、无默认等待/receipt 上限）；resolver 缺与锁和 ACK 分离的 15 秒网络刷新信号作用域，以及基于真实 expiry 的单次 Rust 五分钟临期修正；构造必须零来源 I/O；已选来源失败绝不可回退到另一账号。

## 决策

1. **opi-ai（SDK 层）拥有产品中立的认证机制。** 九个家族的逐 flow OAuth 协议状态机（browser/device/copy-code/manual 路径、PKCE/state、精确的 token 交换与刷新规则、flow 元数据）；来源选择优先级（typed per-call key > stored source > ambient；不完整的已选来源失败关闭；已选凭据后的 HTTP 401/403 绝不重启身份发现）；可注入的 AuthContext/source/store/clock/transport seam；带串行变更、成功前凭据+元数据 ACK 与 `credential_mutation_barrier`（同步同 store cutoff/receipt 捕获、无默认等待期限或 receipt 数量上限、诚实报告 pending/unknown、late-write fence）的 store 合同；基于真实服务 expiry 的单次 Rust 五分钟临期窗口；以及不包含 store 锁与持久化 ACK 的 15 秒刷新网络信号作用域。构造与静态读取执行零 env/store/auth/网络工作；失败 typed 且脱敏；没有自动浏览器、登录、钥匙串或全局运行时，没有账号回退。
2. **Reference Product 保留策略与宿主适配。** 终端 `LoginPresenter` 实现；OS 钥匙串持久化（`KeychainCredentialStore`、`keyring-core` 加平台原生 store、services `opi`/`opi.presence`）；作为持久 store 适配器串行化边界的 `fs4` 跨进程建议锁策略；产品 login/logout UX；config/credential-reference 迁移；以及产品接线。这些迁移为 SDK 合同之后的注入适配器 —— 不移入 `opi-ai`。
3. **真实消费者。** 上述 Reference Product bundle/resolver/login 路径；`opi-agent` 经 `opi-ai` provider 的按调用冻结 auth；以及规划中的 workspace 外消费者（AIR-050/V-42）——必须仅用 `opi-ai`、无任何 Opi 运行时依赖地登录与刷新。既有产品套件是迁移中的消费者；其 secret-canary、加锁与确认断言保留。
4. **最小权限与依赖方向。** 本次归属不给 `opi-ai` 增加 `keyring-core`、`fs4` 或其他原生依赖；依赖保持单向 `opi-coding-agent → opi-ai`。SDK store 合同对象安全、可注入、默认内存 store。无构造期发现，无因可选行为扩大的权限/I/O/平台/供应链面。删除测试：产品认证策略在注入 presenter/store 适配器处保持可替换；SDK 机制只能与其 AIR/V 义务一起删除。
5. **依赖与原生维护成本。** SDK OAuth/SigV4/ADC 机制搭载 Phase 20 选定后端（`reqwest` 0.12.28 + `Hyper` 1.9.0 H1 修正；`rustls` 0.23.40/`ring` 0.17.14；ADC RS256/签名/KX 的私有 OpenSSL 3 `LibCtx` EVP；Smithy Query/XML 0.62.1、`xmlparser` 0.13.5、`p256` 0.13.2、`sha1` 0.10.6）。补丁出处、原生/unsafe 审查与平台负担仍由那些后端组及其验收拥有；本次归属不新增。`keyring-core`、`fs4` 与平台原生钥匙串 store 留在产品侧，因此该移动不给 `opi-ai` 增加新的原生或供应链面。不创建新 crate。
6. **可逆切换。** 闸门顺序 20.18 → 20.22 → 20.23+（随后其余 flow 家族）。每个任务是单个可编译、可回退的切片；产品桥按注册的有界顺序迁移（校验目的冲突 → 复制显式选择的凭据/引用 → 取得目的确认 → 切换当前消费者 → 成功终结前保留来源；绝不合并账号或覆盖不同身份）。回滚是在激活前 revert 该任务切片；已发布的回滚需明确发布授权。迁移后不存留永久并行 facade、重复 SDK resolver 或根消费者补丁（AIR-021/AIR-045 退出证明）。

## 后果

- 任务 20.18/20.22/20.23 按本归属实施。任何接口漂移、新宿主依赖或第二权威 seam 都按 §8.2 触发条件回到归属评审。
- `opi-ai` 公开认证面扩大（Models 管理结果、store 与 mutation-barrier 合同、flow API）；范围内 0.x 破坏性变更同步 Unreleased changelog、调用方、测试与双语 crate 文档。
- 产品现有三条 flow 被带注入 UI/钥匙串适配器的 SDK 机制取代；其重复协议代码仅在相等或更强的公共路径证明（V-43/V-55）后退场。
- 记录本决策不实施任何东西：运行时验收在 owning 任务交付证据之前保持 open。

## 参考

- 注册来源：[spec-draft.zh.md](../research/pi-ai-opi-ai-alignment/spec-draft.zh.md)（归属案例；AIR-023/046/051/063；OAuth/rotation 附件）、[acceptance-matrix.md](../research/pi-ai-opi-ai-alignment/acceptance-matrix.md)（V-09/39/43/55；R-02/R-03）、[pi-1.3.0-sdk-contracts.md](../research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md) §5（九个 flow、store 事务）。
- 历史一手证据：[sdk-auth-provider-migration.md](../research/pi-ai-opi-ai-alignment/sdk-auth-provider-migration.md)（OA-01–06、BP-01–04；Pi 1.0.0 窗口，以当前来源限定）。
- 父规范：[opi-spec.zh.md](../opi-spec.zh.md) §4、§8.1–8.2；[opi-spec.md](../opi-spec.md)。
- 当前消费者：`crates/opi-ai/src/auth.rs`；`crates/opi-coding-agent/src/oauth.rs`、`src/credential_store.rs`、`src/interactive_auth.rs`、`src/provider_factory.rs`、`src/harness.rs`。
- 交付输入基线：[phase20-delivery-inputs.zh.md](../research/pi-ai-opi-ai-alignment/phase20-delivery-inputs.zh.md)。英文对应页：[0001-sdk-auth-placement.md](0001-sdk-auth-placement.md)。

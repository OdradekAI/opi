# SDK OAuth 与内置 provider 迁移证据

**非规范性研究与已确认候选目标的实施导航；2026-10-04。不是正式规范、实施准入或已交付能力。** 本文对应 [D-32/Q33、D-33/Q34](decisions.md)，为候选 spec 的 AIR-051/052 与唯一 [验收矩阵](acceptance-matrix.md) V-43/44 提供一手证据，不另建验收权威。Q33/Q34 均已实际确认；以下“目标”不是等待批准的建议。

基线沿用 [README](README.md)：opi HEAD `f72181a7aa0b4087a3b6a6dfd158e2b4c234bf73`，共享工作区未提交行为不当作发布行为；pi-ai 本地 378 个 Git blobs 与官方 v1.0.0 commit `a13d35a742c6ef8462812a28fbe1d8c8b7431c32` 匹配。本文只重新阅读本地源码、测试和先前已核验的依赖源码；没有运行 OAuth 测试、远端登录、模型调用或新的网络查询。

路径缩写：`PI` = `.repo/pi-1.0.0/packages/ai/`，`PC` = `.repo/pi-1.0.0/packages/coding-agent/`，`OA` = `crates/opi-ai/`，`OC` = `crates/opi-coding-agent/`。行号是当前研究导航，不是目标架构约束。OA-01–06、BP-01–04 是本文件唯一证据 ID。

Q33 只覆盖 Anthropic、`openai-codex` legacy、GitHub Copilot 三条完整 SDK 登录/刷新路径。pi 的新 `openai` ChatGPT OAuth 已存在，不能与 legacy Codex 混称，也不随 Q33 自动纳入。Q34 不自动批准 Mistral 原生 Conversations、其他新 provider、全部 Google auth 方法或完整 MCP 产品范围。源码中的公开 client ID 是协议配置事实；本文不能证明以这些注册信息运行 opi 的远端许可、注册现状或服务接受结果。

## OA-01：Anthropic 浏览器与 copy-code

| 项目 | pi 1.0.0 一手事实 | opi 当前事实及已确认目标 |
|---|---|---|
| 工厂、导出和入口 | `PI/src/providers/anthropic.ts:83` 的 provider auth 经 `src/auth/oauth/load.ts:35` 延迟载入 `anthropicOAuth`；入口为同一 `Models.login/getAuth`，非产品独占 | 当前具体实现 `OC/src/oauth.rs:900`，公共 trait 在 `OA/src/auth.rs:411`。目标由 opi-ai 的 Models 和内置 provider 提供完整 flow；产品只注入 presenter/store |
| 注册与端点 | `PI/src/auth/oauth/anthropic.ts:14–25`：client ID `9d1c250a-e61b-44d9-88ed-5944d1962f5e`；authorize `https://claude.ai/oauth/authorize`；token `https://platform.claude.com/v1/oauth/token` | 当前 `OC/src/oauth.rs:791–795` 相同；目标保留这些协议值，不能把相同常量当作全 flow 等价 |
| 浏览器回调 | 默认监听 `127.0.0.1:53692/callback`，请求的 redirect URI 固定为 `http://localhost:53692/callback`；绑定失败仍可手贴，URI 不换随机端口（pi:138–188） | 当前共享 runner 绑定 `127.0.0.1:0`、URI 为 `http://127.0.0.1:{port}/`（opi:145,246–269）。目标固定回调并保留手贴降级；成功、失败、取消后关闭 listener/手贴等待 |
| authorize 请求 | `code=true`、`client_id`、`response_type=code`、上述 `redirect_uri`、完整 scope、`code_challenge`、`code_challenge_method=S256`、`state`；pi 以 PKCE verifier 作为 state（pi:139–159） | 目标复现已选协议。完整 scope 为 `org:create_api_key user:profile user:inference user:sessions:claude_code user:mcp_servers user:file_upload`，不是只保留 inference |
| copy-code | 独立登录选项 `copy_code`；redirect `https://platform.claude.com/oauth/code/callback`；无 loopback listener；浏览器选项仍为默认（pi:191–225,273–290） | 当前没有该独立模式。目标增加 presenter 可选方法并支持其完整协议，不把“粘贴浏览器 URL”误称 copy-code |
| 手贴语法及校验 | URL、`code#state`、query string、raw code；带 state 必须匹配，不带则使用本次保存的 state；空 code 失败（pi:27–54,168–185,215–225） | 当前手贴 URL/raw-code runner 不拆 `code#state`（opi:638–697）。目标覆盖四种输入；不把秘密回显到诊断或 callback HTML |
| token exchange | JSON POST，`Content-Type: application/json`、`Accept: application/json`；字段恰为 `grant_type=authorization_code, client_id, code, state, redirect_uri, code_verifier`（pi:74–114） | 当前通用 form POST 缺 `state`（opi:410–428）。目标改为该 JSON 协议，不以复制当前函数作为对齐完成 |
| refresh | 同一 token URL，JSON `{grant_type:refresh_token, client_id, refresh_token}`，不发送 scope；响应使用 `access_token,refresh_token,expires_in`（pi:231–265） | 当前 form；缺 refresh token 时沿用旧值、缺 expires 时变未知（opi:523–579）。目标使用已选 JSON 与明确完整响应；非法/缺失必要字段失败且原 store 不变，不能持久化半个 credential |

pi 将 `expires_in` 换算后先减五分钟；目标真实 expiry 与一次窗口见 OA-05。pi 部分 token 解析只有 TypeScript 类型断言，错误还可能包含响应 body；这些是实现/诊断事实，不是目标放弃运行时校验或错误脱敏的理由。

测试证据均为静态阅读：`PI/test/anthropic-oauth.test.ts:42,80,144,176,212` 分别检查固定 URI、copy-code、refresh 无 scope、手贴取消和真实 loopback callback；不等于本次已通过。`OC/tests/oauth_auth.rs:983–1021,1737–1773` 固化旧 form/refresh 接受集，需要更新；其中 canary、state、timeout 和 listener cleanup 仍是应保留的行为边界（见 OA-06）。

## OA-02：OpenAI Codex legacy 浏览器、device-code 与 refresh

| 项目 | pi 1.0.0 一手事实 | opi 当前事实及已确认目标 |
|---|---|---|
| 身份与 wire | `PI/src/providers/openai-codex.ts:7–21`：ID `openai-codex`，显示名 `OpenAI Codex (legacy)`，base `https://chatgpt.com/backend-api`，wire `openai-codex-responses`，OAuth lazy flow | 当前同 ID；Q33 对齐这一条，不把新 `openai` ChatGPT OAuth 合入或宣称已支持 |
| client 与浏览器 URL | `PI/src/auth/oauth/openai-codex.ts:22–35`：client `app_EMoamEEZ73f0CkXaXp7hrann`；authorize/token 为 `https://auth.openai.com/oauth/authorize`、`/oauth/token`；redirect 固定 `http://localhost:1455/auth/callback` | 当前 client/端点相同（opi oauth:805–813），回调仍走随机端口 runner。目标固定 1455；占用时保留同一 redirect URI，提示手贴，不改成随机 URI |
| authorize 字段 | `response_type=code, client_id, redirect_uri, scope=openid profile email offline_access, code_challenge, code_challenge_method=S256, state, id_token_add_organizations=true, codex_cli_simplified_flow=true, originator=pi`（pi:289–305） | 当前 `originator=opi`（opi:1415）。目标保留 `opi` 作为明确客户端身份差异；其余协议字段对齐。不能以 pi 名称冒充自身。state 为独立随机值 |
| 浏览器 exchange | form POST `{grant_type:authorization_code,client_id,code,code_verifier,redirect_uri}`（pi:145–164）；手贴 state 若存在必须匹配；完整网络阶段接 caller signal；finally 关闭 callback（pi:359–395） | 当前 form 基本字段相同，但 callback URI、post-code cancellation 不同。目标采用固定 URI 和 OA-05 的取消边界 |
| device-code 起始 | POST `https://auth.openai.com/api/accounts/deviceauth/usercode`，JSON `{client_id}`；读取 `device_auth_id,user_code,interval`，interval 接受可转换的数字/字符串；展示 `https://auth.openai.com/codex/device`；15 分钟期限（pi:188–228,341–355） | 当前已有实际 device 路径。目标保留；不是把 GitHub device grant 直接套在 Codex 上 |
| device-code poll 和最终 exchange | POST `/api/accounts/deviceauth/token`，JSON `{device_auth_id,user_code}`；成功需 `authorization_code,code_verifier`，再 form 交换且 redirect=`https://auth.openai.com/deviceauth/callback`；首次 poll 不要求先等一个 interval（pi:231–286,350–355） | 当前基础三段路径已存在。目标保留已识别 `denied/expired` 的终止分类；pi 在读 body 前把所有 HTTP 403/404 当 pending（pi:262–264），不机械复制这一接受集 |
| refresh 完整响应 | form `{grant_type:refresh_token,refresh_token,client_id}`；exchange 和 refresh 共用 reader，必需非空 access/refresh 与数字 expires；JWT claim `https://api.openai.com/auth.chatgpt_account_id` 必需并重新派生 `accountId`（pi:122–183,310–329,402–433） | 当前 common refresh 允许省 refresh/expiry，Codex wrapper 后补 account ID（opi:1437–1454）。目标拒绝缺失 refresh/expiry、非法 expiry 或缺 account；不沿用旧 account ID 掩盖新 token 身份变化 |

主 auth 只派生 access token；生成 wire 对 account header 等有自己的职责，不能因为 credential 包含 `accountId` 就假定全部 adapter 已发送正确 header。身份变化与自动恢复仍受 D-24/D-28 和 AIR-036 的同账户约束；本范围不增加隐式网络身份探测。

`PI/test/openai-codex-oauth.test.ts:78,181,279,324,367,488` 提供 device 协议、方法选择、取消、15 分钟、403/404 pending 与端口占用降级的静态测试证据。`OC/tests/oauth_auth.rs:1971,2011,2110` 及其后 device fixtures 证明当前离线边界存在；旧随机 redirect、宽松 refresh 和 post-code cancel 断言需要更新，明确 denied/expired、脱敏和释放行为保留。

## OA-03：Copilot enterprise、有效 endpoint、账号模型与 policy

源事实集中在 `PI/src/auth/oauth/github-copilot.ts`。这是已发布 login/refresh 的一部分，不是未来通用 catalog 提案；但 `/models` 在这里主要产生**账号可用 ID 过滤 metadata**，没有自行把未知 ID 变成完整 `Model` 记录。

| 阶段与字段 | pi 1.0.0 事实及目标协议 | 当前 opi 差距／失败界限 |
|---|---|---|
| provider 与入口 | `PI/src/providers/github-copilot.ts:9–33`；ID `github-copilot`；lazy OAuth；同 provider 按模型选择 Anthropic/Chat/Responses adapter | 当前三个 wire 的 factory 路由已有 fixture（`OC/tests/oauth_auth.rs:4407`）；不因新增 OAuth metadata 改成单 wire provider |
| enterprise | 显式 login 首先允许用户输入 domain/URL；空值为 github.com，取 URL hostname 后组成 HTTPS 端点（pi OAuth:41–61,434–445）。credential 保存 `enterpriseUrl`；后续 refresh 复用，不重新提示 | 当前仅固定 public endpoints；构造器的 mock URL 注入不是用户 enterprise 支持。目标 presenter 支持该输入并持久化正规化域；不是接受任意 remote 凭据重定向 |
| device start | client ID `Iv1.b507a08c87ecfe98`；POST `https://{domain}/login/device/code`；form `client_id,scope=read:user`；Accept JSON、UA `GitHubCopilotChat/0.35.0`。读取 `device_code,user_code,verification_uri,expires_in` 和可选 interval；展示前只接受 HTTP(S) verification URI（pi:11,206–260） | 当前 public 注册值相同。目标保留秘密 device_code 与公开 user_code 的区别；非法 URI 不交给打开浏览器的 presenter |
| device poll | POST `https://{domain}/login/oauth/access_token`；form `client_id,device_code,grant_type=urn:ietf:params:oauth:grant-type:device_code`；首次等待 interval，省略为 5 秒、最小 1 秒；slow_down 有有效 server interval 时使用，否则加 5 秒；受 expires_in 和 caller cancel 限制（pi:263–310；`device-code.ts:46–97`） | 当前首次立即 poll，slow_down 只按本地 +5；目标按真实 flow 区分 Codex/Copilot。明确 denied/expired 失败保留；不忽略 server expiry |
| Copilot token | GET `https://api.{domain}/copilot_internal/v2/token`，Authorization Bearer **GitHub token**，Accept JSON；`token,expires_at` 必需。credential `refresh` 实际保存 GitHub token，`access` 是短时 Copilot token（pi:313–347） | 当前 token 交换已有，`expires_at` 转真实时间。目标保留这两种 token 的职责，不能把 GitHub token 当 OAuth 标准 refresh grant 发回 `/oauth/token` |
| 请求 headers | token/model/policy 阶段使用 UA `GitHubCopilotChat/0.35.0`、`Editor-Version:vscode/1.107.0`、`Editor-Plugin-Version:copilot-chat/0.35.0`、`Copilot-Integration-Id:vscode-chat`（pi:13–19）；models 另有 `X-GitHub-Api-Version:2026-06-01` | 当前 token headers 相同（opi:1463–1466,1633–1640）。这些为 pinned 协议值，不代表已验证服务端将来一直接受 |
| 有效 endpoint | access token 的 `proxy-ep=...` 优先，前缀 `proxy.` 转 `api.` 并加 HTTPS；否则 `https://copilot-api.{enterpriseDomain}`；否则 `https://api.individual.githubcopilot.com`。`toAuth` 每次从**当前** credential 派生（pi:69–87,487–505） | 当前从 token JSON `endpoints.api` 或旧 base_url 取值（opi:1624–1669）。目标改为已选 token/enterprise 派生，不能永久保留旧账户 endpoint；派生与 transport 的有效 URL 检查仍执行，不把未经验证的 token 字符串当任意 URL 指令 |
| account models | GET `${effectiveBase}/models`，Bearer **Copilot token**；顶层 `data` 必须数组。条目需 string id；`capabilities.supports.tool_calls === false` 排除；`model_picker_enabled === true` 且 `policy.state !== disabled` 为 picker 可用项（pi:93–132,168–194） | 当前没有该阶段及 `availableModelIds` 字段。目标必须将 credential 与模型可用 metadata 作为同一成功结果提交；不只迁移 token HTTP |
| Individual fallback | 仅有效 base **恰为** `https://api.individual.githubcopilot.com` 且 picker 集合为空时，使用 `policy.state === enabled` 的 tool-capable 条目。其他账号不 fallback（pi:119–123,174–177） | 目标使用此有界判定，不把空 picker 普遍解释为所有 bundled 模型可用 |
| policy 候选 | `policy.state === unconfigured`，ID 位于 bundled `GITHUB_COPILOT_MODELS`，并且 pickerEnabled 或上述 Individual fallback 生效；此前已排除 tool_calls false（pi:124–131） | 不能为远端任意 ID、disabled 项或不支持工具项启用 policy |
| policy 写入 | **仅显式 login**，顺序 POST `${base}/models/{id}/policy`，JSON `{"state":"enabled"}`；Bearer Copilot token、四个客户端 headers，另加 `openai-intent:chat-policy` 与 `x-interaction-type:chat-policy`；成功 ID 与发现的 available IDs 去重并集（pi:373–484） | Q33 已确认该登录行为；本研究不执行。它是远端副作用，取消不能承诺回滚先前成功 policy。普通生成/自动 refresh 不得偷偷启用 policy |
| policy 与 discovery 重试 | login models GET 和每个 policy 使用最多 2 次额外 429 retry、5 秒局部总预算，每次 fetch 5 秒；Retry-After 数字秒或日期，否则 500ms×2^retry；延迟达到剩余预算不继续。policy 普通失败返回 false 后继续下一项；最终 429 停止剩余 policy；caller cancel 抛出（pi:135–165,373–431,462–469） | 非取消 policy 失败不必使整个 login 失败；模型发现本身失败则不能返回完整 credential。5 秒是局部 retry 预算，不得误写成整个人类登录限时 5 秒 |
| refresh | 先换 Copilot token，再 GET account models，models **不 retry**（maxRetries=0,maxElapsedMs=0），不调用 policy（pi:353–366）。token/metadata 完整后才交 store | Q33 的 15 秒是整个 refresh 默认期限，包含 token、models 和 body 读取；models 阶段仍有 5 秒单请求界限。取消贯穿这条链，不能在 token 成功后关闭取消 |

metadata 目标至少表达 `enterprise domain`、`available model IDs` 与真实 expiry，并能从当前 access token 派生有效 endpoint。缺少 legacy metadata 与已知空数组不同：pi provider 对缺失/非法 `availableModelIds` 不过滤，合法空数组过滤为零；新成功 flow 不得伪造缺失 metadata 来跳过已确认发现步骤（`PI/src/providers/github-copilot.ts:19–26`）。目标保留这种缺失与空值的区别；历史 credential 的无 metadata 状态不能靠猜测回填为另一账号。`enterpriseUrl`、endpoint、model IDs 都**不能证明 Copilot account identity**，该 flow 没有像 Codex 一样提取已验证 account ID。

`PI/test/github-copilot-oauth.test.ts:143,178,216,232,322,380,412,440,477,572` 分别覆盖 picker、Individual fallback、企业反证、refresh 不重试、policy 范围、Retry-After、普通失败继续、budget 停止、URI 和轮询时序。`PI/test/oauth-auth.test.ts:77,83,120,167` 检查 endpoint/enterprise/Models lazy chain。当前 `OC/tests/oauth_auth.rs` 的 Copilot 模拟主要覆盖 device→token、取消/超时、base_url fallback，不能证明账号发现、enterprise 或 policy 已存在。

## OA-04：SDK refresh/store 所有权与 metadata 原子性

| 边界 | 当前一手事实 | 已确认候选目标与失败行为 |
|---|---|---|
| SDK 能力归属 | pi `Models.login:756–810`、`getAuth:735–753`、`applyAuth:837–868` 共用 provider auth 与注入 store；`src/auth/resolve.ts:27–31` 明示 stored credential 失败不得静默回退 env | opi-ai 拥有三条 flow、同源 resolver 与 Models 管理入口；既能注入内存 store，也能由产品注入持久 store/presenter；普通调用方无需依赖 coding-agent、TUI/keychain 或 durable session |
| opi 当前 refresh owner | `OC/src/credential_store.rs:1076,1152–1245` 的具体 CredentialResolver 绑定 KeychainCredentialStore；快速读→获取跨进程 mutation lock→重读→refresh→write_unlocked。`OA/src/auth.rs` 仅有公共 trait/value；具体 flow/registry 在产品 | 迁移实际状态机和可复用契约，不只 re-export 产品符号。产品保留 OS store、UI 和选择政策；库不反向依赖产品。来源解析失败不换 env 账号 |
| 并发与注销 | pi `resolve.ts:122–147` 在 CredentialStore.modify 内重读；已注销或别的请求已刷新时不再次刷旧 token | 同一 store/provider 的并发 refresh 共用一次已确认成功结果；锁等待可取消。注销或替换后不得再提交过时结果。不同独立 store/Models 不引入进程级凭据全局状态 |
| login ack | pi `Models.login:770–810` 等完整 flow 后才 store.modify，并区分 mutation 开始前取消与开始后等待结果。opi `oauth.rs:2533–2550` 延迟 success 通知到 store.write 后 | token、expiry、Codex account、Copilot enterprise/model metadata 成为一次 credential 更新；持久 ack 前不能宣告登录成功。存储失败不得写出仅 token 或仅 metadata 的“半成功” |
| refresh ack | opi `credential_store.rs:1216–1243` 成功写后返回，失败保留旧 entry；pi modify 返回完整 credential 后才派生 auth | 新 token 在 commit ack 前不用于后续生成。网络刷新失败/取消不写；commit 已开始后的取消依实际 ack 返回，不谎称撤销成功写入。credential metadata 不另建一个无版本关联的全局可变缓存 |
| 持久格式 | 当前 `OA/src/credential.rs:57–69`、`auth.rs:345–356` 只有 access/refresh/expiry/base_url/account_id，没有 Copilot enterprise/model IDs；`OC/credential_store.rs:128–137` 是 provider-ID key、version 1 envelope | format 更新由既有 store owner 负责；保留旧 reader 的明确缺失含义，未知 required 格式拒绝。新增 metadata 与 token 同一版本化记录，Debug/错误不得泄漏秘密；不通过去掉 metadata 伪装可回退格式 |

若 refresh 已在远端旋转 token，之后 account-models 或本地存储失败，保留原 store **不等于**远端旧 refresh token 仍有效。若 login policy 已成功，后续取消也不表示 policy 未改变。目标保留 typed failure 与非秘密阶段信息，下一次显式操作可重试/重新登录；不制造不存在的跨远端/本地事务，不伪造 successful ack。

## OA-05：expiry、超时与取消的已确认有意差异

pi `src/auth/resolve.ts:102–119` 默认 refresh window 为五分钟、refresh timeout 为 15 秒。Anthropic 保存 `now + expires_in − 5min`，Copilot 保存 `expires_at − 5min`，resolve 再提前五分钟，因此默认生成 auth 路径可能在真实 expiry 前十分钟刷新；Codex 保存真实 expiry，因此为一次五分钟。该结论限定于这一 resolve 路径，不说 pi 每个目录过滤函数都使用相同门槛。

Q33 确认：三条目标 credential 都保存供应商真实 expiry，resolver 只应用一次五分钟安全窗口；例如真实 expiry 为 E 时，E−5min 为触发边界，E−10min 不因双扣减提前刷新。数字需有限、时间换算可表达，不能因溢出/缺字段写出可用 credential。旧记录的 unknown expiry 仍是 unknown，不能用“现在+固定TTL”假装恢复；下次实际解析按已有未知过期保守刷新路径处理。

当前 opi 默认 refresh 为 30 秒（`OC/src/credential_store.rs:139`），目标为 15 秒。15 秒从实际 refresh 工作开始覆盖该 flow 所有网络/解析阶段；不是普通 generation attempt 的 timeout，也不是登录等人输入的总时长。调用方更早取消或自身 deadline 先到时生效；Copilot 的局部 5 秒 GET/retry 限制不能扩大外层 15 秒。

当前 opi 共享 PKCE 在得到 code 后刻意退出 cancellation race（`OC/src/oauth.rs:352–354`），Copilot token 后也有对应离线旧断言（`OC/tests/oauth_auth.rs:4044`）。目标取消继续控制后续 token exchange/refresh/model discovery/policy 网络、body、重试等待，并清理 callback/输入任务。**拿到 code、持有锁、进入 modify 回调都不等于已进入不可中断的存储提交**；仅实际存储 commit 开始后依 OA-04 ack 规则处理。已完成远端副作用不被取消抹去。

Codex 对明确 device denied/expired 终止、opi 的 secret canary/脱敏、原子持久化均保留；这些是已确认差异或保护，而非“Rust 必然如此”。不复制 pi 原始 HTTP body 错误打印。可解析 body 中的闭合错误码用于分类，返回的诊断不能包含 code、verifier、device code、access/refresh 或 header。

## OA-06：真实消费者、已有证明与需要改变的测试

| 层 | 真实当前调用证据 | 验收能证明什么／未证明什么 |
|---|---|---|
| pi SDK | `PI/src/models.ts:756–810` login、735–753 auth；`PI/test/oauth-auth.test.ts:148–182` 直接 Models.getAuth | 证实独立 SDK 路径和 lazy flow 接线；源码/测试存在不等于本轮执行通过 |
| pi 产品 | `PC/src/core/model-runtime.ts:817–825` 委托 Models.login；`PC/src/modes/interactive/interactive-mode.ts:6245` 交互登录 | 三条 SDK flow 已接产品管理入口，不是仅测试/compat 入口 |
| opi 产品 | `OC/src/provider_factory.rs:1598–1599` 构造 resolver/registry；`main.rs:1341` 注入 harness；`interactive_auth.rs:244` 调 login_oauth | 当前有真实产品消费者，但不证明只依赖 opi-ai 就能完整登录/刷新 |
| opi 离线并发/存储 | `OC/tests/oauth_auth.rs:4567` 并发刷新；4815/4883 登录与存储失败；5435 timeout 保留 credential/释放锁；factory routes 4407/4505/5193/5298 | 已有公共行为测试值得迁移和保留；不能改成只验证新增类/函数存在 |
| opi 取消/安全 | `OC/tests/oauth_auth.rs:1409,1447,1491,1529,1717,1839` 覆盖 timeout、body budget、cancel、listener cleanup、canary；1596 与4044 是旧 post-code/token cancel 行为 | 更新已确认改变的断言，保留 secret/资源边界。当前测试不能作为 pi 1.0.0 protocol 相等证明 |

`OC/src/oauth.rs:2423–2425` 的 “pinned to reviewed pi-0.80.6 profiles” 是当前代码来源说明，作为历史记录有意义；作为 1.0.0 全 flow 支持主张已不足。相同 client/endpoint 常量没有覆盖 JSON/state、固定 redirect、copy-code、严格 refresh、Copilot discovery/policy 的差异。实现时应更新当前契约注释并保留必要历史来源，不能只换注释版本号。

V-43 的必要输入由以上协议表给定：只依赖 opi-ai 的外部 fixture crate、内存 store、fake presenter、受控 loopback transport/fake clock，走三条登录→持久 ack→生成 auth→同源 refresh；覆盖 token/metadata 原子更新、并发 refresh、注销 race、15 秒/cancel、一次五分钟边界、policy 部分远端成功后失败、未知账户身份及错误 canary。真实 client ID 与远端接受仍不由此证明。测试影响：**add** 缺失 SDK/协议/metadata fixtures；**update** 旧 wire、取消、expiry 断言与产品装配；**retain** 脱敏/存储/资源/denied 边界；仅在新公共行为覆盖后 **delete** 已退出路径的重复测试，不删除历史格式读取证明。

## BP-01：canonical provider ID 映射

当前内置集合在 `OC/src/provider_factory.rs:518–529`，配置结构在 `OC/src/config.rs:254–264,558–565`。目标是一个 canonical 内置身份，不是永久同时注册两套 ID。

| 当前 opi 内置 ID | Q34 确认 canonical ID | 默认 wire／主证据 |
|---|---|---|
| `gemini` | `google` | `google-generative-ai`；`PI/src/providers/google.ts:8–13` |
| `vertex` | `google-vertex` | `google-vertex`；`PI/src/providers/google-vertex.ts:94–98` |
| `bedrock` | `amazon-bedrock` | `bedrock-converse-stream`；`PI/src/providers/amazon-bedrock.ts:84–88`；AWS 来源行为仍由 SP-05/06/08 决定，不因改 ID 变更 |
| `openai` 与 `openai-responses` | 一个 `openai` | `openai-responses`；`PI/src/providers/openai.ts:9–22`；旧 Chat 意图须显式迁移，见 BP-02/04 |
| `azure` | `azure-openai-responses` | 新真实 `azure-openai-responses` adapter；`PI/src/providers/azure-openai-responses.ts:8–12` |
| `anthropic`、`openrouter`、`mistral`、`github-copilot`、`openai-codex` | 同名保留 | 只承诺各已选 wire/operation 支持集合；`mistral` 同名不能证明已支持 pi 原生 Conversations；OAuth 范围按 OA-01–03 |

provider 身份不等于 API kind、模型 bare ID 或凭据类型。Rust ApiKind enum 可以有 idiomatic 名称，但公开 identity、wire capability 和序列化来源不得互相冒名。内置 canonical lookup、模型记录、默认选择与产品装配共同迁移，不靠字符串替换隐藏缺 adapter。

## BP-02：OpenAI 默认 Responses，Chat 为明确选择

pi `openaiProvider()` 自带 Responses adapter；当前 opi factory 的 `openai` 走 OpenAiChat（`OC/src/provider_factory.rs:1848–1858`），另有 `openai-responses`（1904 起）。Q34 已选择合并 provider identity、默认 Responses，并保留可显式选用的 Chat adapter/custom profile，不删除 Chat wire 的真实行为或所有 provider 使用它的路径。

**迁移意图不能靠新默认值推断**：旧 `openai` 配置明确使用 Chat 的调用方需迁移到显式 Chat adapter/profile；新的 `openai` 默认才是 Responses。旧 Responses 配置可迁入 canonical openai，但若已有不同 openai credential/config，必须报告冲突，不能覆盖、合并账户或静默挑一个。生成的迁移结果写明 profile/wire 与非秘密来源，不能靠长期 alias 后门维持两个默认值。

历史 assistant 的 provider/API/opaque continuation 来源仍保留原值。只有已验证 request projection bridge 可解释 legacy identity；**Chat 历史不因 provider 字符串改为 openai 就变成 Responses opaque 数据**。Cross-wire 文本/工具投影与不兼容 continuation 规则继续适用。

## BP-03：Azure Responses 的最终 URL、auth、schema 与证据冲突

当前 `OA/src/azure_openai.rs:4–5,29,161–164,286` 是 Chat：`{endpoint}/openai/deployments/{deployment}/chat/completions?api-version=2024-06-01`（可显式覆盖版本），`api-key` header。改 ID 不会把它变成 Responses。

目标按 pi adapter 与其**实际锁定 OpenAI SDK**共同确定：

| 契约 | 一手证据与目标行为 |
|---|---|
| provider/auth | `PI/src/providers/azure-openai-responses.ts:6–12`；首批 API key，环境 `AZURE_OPENAI_API_KEY`。底层 openai SDK 可支持 Entra 不等于 pi 此 provider 已暴露相同 flow，也不自动纳入 Q34 |
| base URL 优先级 | `PI/src/api/azure-openai-responses.ts:226–259`：显式 azureBaseUrl→`AZURE_OPENAI_BASE_URL`；无 base 再取显式 azureResourceName→`AZURE_OPENAI_RESOURCE_NAME`，生成 `https://{resource}.openai.azure.com/openai/v1`；再缺则 model.baseUrl；全缺在 HTTP 前失败 |
| URL 归一化 | 同文件191–219：对 `.openai.azure.com`、`.cognitiveservices.azure.com`、`.ai.azure.com` host 的 root、`/openai`、`/openai/v1/responses` 归一到 `/openai/v1` 并清 query；已有 `/openai/v1` 保持；明确非 Azure proxy path/query 不擅自改写。不能在多个层重复补 `/responses` |
| API version | 同文件25,230–234：显式 azureApiVersion→`AZURE_OPENAI_API_VERSION`→`v1`；SDK `src/azure.ts:121` 放入 query `api-version` |
| 最终默认请求 | **POST `https://{resource}.openai.azure.com/openai/v1/responses?api-version=v1`**；SDK `resources/responses/responses.ts:107–119` POST `/responses`，`azure.ts:194–201,252–262` 的 deployment endpoint 集合**不含 `/responses`**，因此不插入 `/deployments/{deployment}` |
| auth/redirect | SDK `src/azure.ts:224–233` 静态 key 发 `api-key`，而不是 Bearer；204–219 将带 api-key 请求置 manual redirect。目标 transport 验证同一 header 与不把 key 随重定向发往其他位置；不把一般 OpenAI Bearer header原样复制过来 |
| deployment 与 payload | pi adapter:30–50,302–320：显式 azureDeploymentName→`AZURE_OPENAI_DEPLOYMENT_NAME_MAP` 的 modelId=deploymentName→model.id；deployment 写 JSON `model`，不用改 canonical model ID。body 用 Responses `input`、`stream:true`、`store:false`，session cache key 同 Responses clamp；不是 Chat `messages` |
| tools/thinking | 同文件286–350：复用 Responses message/tool/stream converter；strict 默认支持（明确 false 可关闭），grammar 默认不支持，additional/tool_search 按 compat；tool_choice 按请求；reasoning effort/summary 与 encrypted_content 走 Responses 规则。不能只转换 URL，却继续用 Chat tool schema/delta parser |
| native max output | 同文件26–28,310–312：提供 truthy maxTokens 时 `max_output_tokens=max(value,16)`；native 未提供不填。simple 的预算另由已确认 D-34/AIR-053 决定，不把 native clamp 偷渡为全局准备阶段规则 |
| 流和失败 | 同文件117–159：SDK 内部 maxRetries=0，外层 provider retry；流调用共享 Responses processor，缺 stop reason 为失败；目标遵 AIR-047 的统一 Result/terminal，而不复制 pi 对象返回形状 |

**文档主张冲突**：pi adapter:206–207 注释说 SDK 可追加 `/deployments/<model>/...`，但实际锁定 SDK 对 Responses 不这么做。该注释对本路径为部分过时/不准确；实现为已实现。`PI/test/azure-openai-base-url.test.ts` mock 整个 OpenAI 类，122–167/224 只证明传给 constructor 的 baseURL，不能证明最终 HTTP URL、query 和 auth header。目标以真实 SDK 调用链为依据，新 Rust loopback fixture 必须直接断言最终请求；不能把 mock constructor 测试当 wire 完整证明。

依赖证据复用 [SDK 调查附录 A](sdk-target-proposal.md)：pi lock 的 `openai@7.19.0` 官方 npm 包已逐包验证 SHA-512 integrity，archive SHA-256 `597532dcb9051ee8aa5edadedb57b798b42add68cc5aec1d67e994d0cfad07f1`；先前已核验官方 tag commit `15f6e408a0359df50ee7a996616957cc14988659`。本轮读的是已解包的 `C:/Users/Luiz/AppData/Local/Temp/opi-sdk-source-6q2wc11l/openai/src/azure.ts` 和 `src/resources/responses/responses.ts`，没有执行包代码。固定上游导航：[Azure client](https://github.com/openai/openai-node/blob/15f6e408a0359df50ee7a996616957cc14988659/src/azure.ts)、[Responses resource](https://github.com/openai/openai-node/blob/15f6e408a0359df50ee7a996616957cc14988659/src/resources/responses/responses.ts)。不宣称该依赖是今日最新或对应任意 Azure deployment 的远端可用性。

## BP-04：显式 config/credential 迁移和旧路径退出

当前 keychain 用 `(service=opi, account=provider_id)`，presence marker 在 `opi.presence`（`OC/src/credential_store.rs:126–137,163–170`）；改显示名或 catalog key 不能自动搬凭据。产品现有 `ProvidersConfig`/TOML、环境变量映射、built-in factory、model selector、默认模型、SDK 例子、schema/help/tests/docs 都是迁移消费者。常见远端环境变量 `GEMINI_API_KEY`、`OPENAI_API_KEY`、`AZURE_OPENAI_API_KEY` 不因本地 provider 改名被凭空改成另一套名字。

Q34 确认的有界迁移顺序为既有 config/store owner 执行：读取源和目的→验证 mapping/wire 意图及冲突→安全复制 credential/metadata→取得目的持久 ack→切换配置/引用并确认→成功 finalization 后退出旧默认 key/注册。不是本轮执行步骤，也不另建迁移 ledger。每个中断点必须可重试；source 在成功 finalization 前保留；失败不能声称已完成，也不能导致旧引用只剩被删除的 secret。

| 条件 | 固定处理边界 |
|---|---|
| 目的不存在 | 按明确映射迁移；credential 不需远端重签、重登录、模型调用或付费验活；secret 不进日志/迁移报告 |
| 目的已有不同 config/credential | typed conflict，保留双方；不自动合并 account、header/base URL 或选择较新值；让显式配置决定后再重新迁移 |
| 目的已有同一份可证明相同材料 | 可作为幂等重试继续；相同性由 protected store 内部判定，不能把秘密 hash/token 输出为公共证明 |
| OpenAI/Azure 旧 Chat 用户 | 迁为显式 Chat adapter/profile，保留原有 wire 行为；新 canonical 默认为 Responses。仅原 adapter 未迁完不是永久保留两个内置默认身份的理由 |
| Google/Vertex/Bedrock rename | API wire 不因 ID rename 改变；当前 credentials/config 的 auth 选择不被名字推断改写；AWS 默认链及 Google 其他 flow 范围仍单独受目标支持集合约束 |
| source history/opaque metadata | 不改旧 session entry、原 provider/API/source identity、原 opaque bytes；request-local bridge 仅在已验证兼容条件下投影；旧 reader 保留历史审计能力 |
| 退出证明 | canonical public lookup/default payload 与 product selection 一致；无旧内置 default ID、隐式 alias fallback、同职责注册分叉；旧符号只可留在版本化 reader、明确迁移器、历史证据/fixture，不用不加区分的全仓库字符串零命中删除历史 |

V-44 必须覆盖真实 SDK-only public lookup/默认 dispatch、显式 Chat route、Azure 最终请求 URL/header/Responses schema/terminal、配置与 credential 冲突、copy/ack/reference 切换各失败点、幂等重试、无网络迁移、旧 history 不变及旧默认路径静态退出。可复用现 `OC/tests/oauth_auth.rs:4407` 多 wire factory fixture 的思路，但它不覆盖新默认/Azure/migration。测试影响为 **add** 新协议与迁移故障 fixtures、**update** factory/config/schema/help/lookup 断言、**retain** 历史 reader 与显式 Chat conformance、在等价行为已覆盖后 **delete** 旧重复内置路径断言；当前测试均未在本子任务修改或重跑。

## 可信度、未核验边界与本次检查

OA-01–06、BP-01–04 的源码行为判断为高置信度；测试覆盖描述只声称所读测试的意图/断言。官方服务当前是否接受固定 client 注册、Copilot enterprise/account policy 权限、Azure resource/deployment/版本仍未 live 核验。没有“已登录成功”“全部真实账号等价”或未来发布承诺。pi 普通 token 解析缺少某些运行时校验、重复 expiry 提前量、宽泛 403/404 pending、错误 body 暴露和 Azure 注释均不自动升级为目标。

本文为新增中文证据附录，没有既有英文 counterpart；英/中文候选 spec 应同步引用这组同一证据，避免复制第二份事实权威。本子任务实际执行 `python scripts/opi-doc-check.py`，结果 PASS；针对本新增未跟踪文件执行 `git diff --no-index --check -- /dev/null docs/research/pi-ai-opi-ai-alignment/sdk-auth-provider-migration.md`，无输出、exit 0。本轮全仓库检查及共享改动限制由主交付统一报告；本子任务未执行运行时测试、依赖安装、远端查询或生产操作。

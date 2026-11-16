# Transport、协议与资源生命周期

本文说明外部 `opi-ai` 调用方需要知道的传输和生命周期行为。Provider 注册与凭据
选择见[Provider、Catalog、认证与刷新参考](provider-auth.zh.md)；Provider wire
形状见[Provider Wire 与操作参考](providers.zh.md)。

## Proxy 与源限定 HTTP

`HttpClient` 是按 source 隔离的 HTTP owner。调用方通过 `HttpClientBuilder` 按
source policy 构造 client；重定向上限、代理路由、旁路规则、header 过滤、body
上限、采集超时和类型化终止错误在 provider 发起请求前冻结。Provider 不会偷偷
创建第二个 client，也不会在调用方不知情时读取另一套 ambient proxy policy。

代理可以来自显式 Provider profile 或宿主选定的代理环境。代理 URL 中的凭据在错误
和诊断返回前会脱敏。旁路只作用于拥有该 source 的 client；外部测试使用的 loopback
peer 可以在不改变进程全局环境的情况下访问。

响应只允许被采集一次。重定向、状态处理、body 上限、畸形 header、超时和网络失败
都转换为类型化 `ProviderError` cause；部分或无 body 的失败不会变成成功 Provider
事件。重试由 logical call 或 Provider adapter 持有，transport 不会静默重发请求。

## 源限定 TLS

`HttpClientBuilder::tls(TlsOptions)` 是公开 TLS 边界。信任根、server-name policy、
ALPN 和 TLS backend 都挂在 source client 上，而不是进程级全局 client 上。宿主拥有
native trust 与平台初始化。Provider 可以选择 source profile，但不会修改其他
Provider 的 TLS policy。

固定的 rustls/AWS-LC/native-certs 实现由 `tls-backend` feature 拥有。只需要
类型化 source-scoped 配置的调用方可以启用 `tls-backend` 并使用
`opi_ai::tls::TlsOptions`；`http-transport` 会传递选择同一 owner，并为
`HttpClient` 调用方保留历史 `opi_ai::http` re-export。

TLS 配置失败在请求发布前报告。client 保持与 HTTP 相同的超时、取消和脱敏规则；
关闭调用方持有的 stream 会按正常生命周期释放 source resource，不会通过丢弃全局
pool 来处理。

## WebSocket Upgrade

WebSocket/compression 能力由可独立选择的 `websocket-compression` feature
拥有。它暴露严格的 `opi_ai::ws` Upgrade bridge，并将 RFC 6455 frame、
permessage-deflate 与异步 IO 状态保持为私有；该 feature 传递选择
`http-transport` 和 `tls-backend`。Provider adapter 也传递选择同一 owner，
只需要公共 Upgrade 边界的调用方则可以不启用 provider 模块。

`ws::upgrade` 在共享 `HttpClient` 上执行严格动态 Upgrade。公开 transport preference
可以选择 SSE、WebSocket 或自动 route。自动选择保留 Provider 的 fallback policy，
不会把 HTTP 失败变成无限重连。

Codex 从 WebSocket 的 `error` 和 `response.failed.response.error.code` 读取恢复码，
两种帧共用同一重试预算。在首个带类型的 Provider event 前收到
`websocket_connection_limit_reached` 时重连一次。同一调用第二次遇到这种拒绝时，
沿已选 route 使用原始全量输入转为 SSE。此策略适用于 `Auto`、`Websocket` 和
`WebsocketCached`。
同一启用缓存的 session 后续调用使用 SSE，直到资源清理清除回退标记。成功 Upgrade
封存高级调用的 acquisition，即使回退的 POST 失败也保持封存。在带类型的 Provider
event 后触发连接限额时，流以包含部分结果的错误结束。

续接响应缺失时，公开 stream item 交付前允许一次全量输入尝试。若该尝试失败，
终态保留最后一次失败的原始状态和脱敏诊断。公开输出交付后遇到续接缺失时，当前调用
以包含部分消息的错误结束。

Codex 在池化连接上发送下一条 `response.create` 前检查当时可立即读取的输入。
它丢弃空闲时读到的完整应用消息，并应答可读的 Ping。收到对端 Close、输入
未读完整或用尽空闲读取预算时，释放连接和该连接上的续接状态。替代连接从
全量输入开始。

复用连接收到 `response.created` 并确认本轮 ID 与上一轮不同时，Codex 丢弃嵌套
`response.id` 指向该连接上一轮成功响应的终态帧。这些帧在原始事件回调、恢复判定和
消息映射前被丢弃。

在活跃的 Codex WebSocket 上，非零 `Request::timeout` 限制等待下一条应用消息的时间。
Ping 和 Pong 沿用该期限。应用事件的回调和交付结束后开始下一次等待。期限到达时，
Opi 在有限的关闭预算内尝试发送代码 `1000`、原因为 `idle_timeout` 的 Close。首个
带类型的 Provider event 前超时，当前调用携带全量请求转为 SSE。收到该事件后超时，
当前调用返回携带部分消息的流错误。启用缓存的同一 session 后续走 SSE，直到资源清理。
超时值为零时停用此 WebSocket 空闲计时器。

握手在返回 upgraded stream 前校验 HTTP status、version、`Connection` 与 `Upgrade`
header、`Sec-WebSocket-Accept`、subprotocol、singleton extension 参数和
permessage-deflate 范围。非法 negotiation 在产生 Provider event 前失败。升级后的
stream 继续使用其 source proxy/TLS policy 和调用方 cancellation token。

frame、PMD 和 IO owner 是实现细节。消费者使用 Provider 或 `ChatCall`/`ChatStream`
契约，不依赖 codec 类型。如果 backend 没有公开 WebSocket route，应使用其文档化的
SSE 或 HTTP route。

## Chat 完成边界

OpenAI Chat、OpenRouter、旧式 Mistral Chat profile 和 Azure Chat 将
`finish_reason` 与 usage 帧作为元数据累积。文本、thinking 和工具块保持打开，
直到 `[DONE]` 或自然 HTTP EOF。有效终点按 content 顺序结束各块、最终化工具参数，
再发送一次 `Done`。适配器在 sentinel 处释放响应，不等待 HTTP EOF；后续帧不会进入
raw 回调或改写已缓存结果。

在终点之前出现 Provider、非法帧、usage、传输或回调失败，返回累计 partial，
不返回成功。显式取消优先于排队事件，返回 `Aborted`，不补造块结束。
这沿用 Opi 的取消契约；Pi 1.5.0 的 OpenAI SDK 在取消展开时可能结束块。
Agent 仅在成功终态之后执行工具。

默认缺少 finish reason 就失败。显式设置 `supports_finish_reason=false` 的路由，
在 sentinel/EOF 推断 `Stop` 或 `ToolUse`，不伪造 `raw_stop_reason`。
Azure 的 `with_compat` 设置会保留到简单调用。HTTP EOF 时未以空行结束的 SSE 片段不派发。

自定义 parser/mapper 调用方处理 `ParsedEvent::Done`，并在协议终点调用
`OpenAiChatMapper::finish(supports_finish_reason)`。它返回完整结束事件序列，
替代 `flush_pending_done()`；任何终态或失败后都应停止读取响应。

## Codex 终态响应

Codex 在 SSE 和 WebSocket 上接受 `response.done`、`response.completed` 与
`response.incomplete` 作为终态帧。适配器从该帧读取响应状态、ID、上报模型和
用量。带工具调用的正常完成返回 `ToolUse`。原因是 `max_output_tokens` 的
未完成响应返回 `Length`。其他未完成原因、原因缺失，以及 Provider 上报
`failed` 或 `cancelled` 时，返回携带部分消息的类型化流失败。

`AssistantMessage::end_turn()` 提供终态中的布尔诊断 `end_turn`。两个布尔值
都通过 `AssistantMessage::diagnostics` 保留到序列化结果。调用结果由
`stop_reason` 表示，缓存续接使用成功终态的响应 ID。原始事件回调收到未经
改写的终态帧。Codex 交付首个终态后即释放 SSE 响应体，即使对端仍保持响应体打开。

## Resource Generation、Pool 清理与 Await 释放

Provider call 在 session 生命周期内拥有一个 resource generation。acquisition、使用、
abort 和 release 按顺序执行。已捕获的 call 保留选中的 adapter/resource revision；
删除或替换 Provider 注册不会使已捕获 call 的 cleanup 失效。

`Models` 会尝试每个 owner 的 release，并返回聚合 cleanup 结果。调用方丢弃 consumer
但没有取消请求，不等于 Provider generation 已结束。显式取消、Provider terminal
status 和 resource cleanup 是不同观察。阻塞 cleanup 在 owner 报告 release 完成前被
await；迟到结果会被抑制，不会发布到新的 generation。

raw prepared attempt 的边界由调用方拥有。observed `ChatStream` helper 提供标准
result/observer 结算；需要自定义生命周期记录时仍可使用 raw seam。

## Deferred Transport 语义

Deferred 提交就是一次普通 generation，可能返回带 `StopReason::Deferred`、空 content
和类型化 handle 的 assistant message。handle 由调用方持有。后续 fetch 创建普通
`ChatStream`。`Models::fetch_deferred` 返回带 `StopReason::Deferred` 和 handle 的待定消息、
就绪消息或类型化失败。待定检查跳过 token 和 cost 累计。`poll_after_ms` 是 Provider 提供给
调用方的调度提示。

`Models::fetch_deferred` 通过 `ChatStream::result()` 驱动源流。正常完成时，缓存成功终态，
读到源流 EOF 并释放源流，再返回结果。启用 poll 遥测且记录正常时，返回前完成 observer
结算。`wait_ms` 是 Provider 的长轮询等待上限。收到终态后，源流尾部仍待定时，fetch
也会继续等待，零等待的状态检查也是如此。

在 Tokio runtime 内迁移 Pi 的 `Models.fetchDeferred` 或
`Models.streamDeferred(...).result()` 等待点时，使用
`models.fetch_deferred_ready(&handle, call).await?`。该入口沿用同一套路由、鉴权和 Provider
等待参数，派发一次 fetch。首个规范化终态返回带 handle 的待定消息、就绪消息或类型化失败。
任务先发送结果，再排空源流并结算 poll。尾流仍待定时，future 返回或开始轮询的等待者被
丢弃后，任务可以继续收尾。尾流立即结束时，任务可能在调用方恢复执行前完成结算。调用方
收到待定消息即可安排下次 fetch。保留取消令牌可释放待定的本地尾流。需要等待源流释放和
poll 结算时，使用 `fetch_deferred`，或保留启动后的流并等待其 `result()`。

迁移应用错误处理时，在应用判定并记录失败的 assistant 消息处处理 `Err(CallFailure)`。
通过 `CallFailure::message()` 或 `into_parts()` 读取规范化 partial、停止原因和可用的诊断。
将消息交给 `retry::classify_whole_call_retry` 判定整次调用重试。
`ProviderError::is_retryable()` 分类类型化原因。重试预算、退避和上下文压缩策略由应用决定。

鉴权 resolver 返回 `ProviderError::AuthFailed` 时，fetch 在 Provider 派发前失败并保留该原因。
生成的失败消息携带请求的 provider/model、`StopReason::Error` 和安全的 resolver 诊断。
应用可用该消息记录 poll 失败。即使类型化原因为 `AuthFailed`，诊断也可能描述可重试的瞬态故障。

两个停止原因字段都是 `Aborted` 的旧式 provider `Error` 终态
会保留原消息，类型化原因为 `ProviderError::Cancelled`。应用先检查任务的中止状态，再
对 fetch 结果分类。任务仍运行时，收到 `Aborted` 消息也会结束整次调用的重试。应用放弃
远端任务时，可用保留的 handle 显式调用 `cancel_deferred`。

`Models::stream_deferred` 返回的 collector 由调用方驱动。handle 有效时，轮询 collector
会推进鉴权和派发。逐项读取时可能在源流 EOF 前收到 `Done`。随后一次读取会复用同一尾部
排空路径，释放源流并结算已启用的 poll observer，再返回公开 EOF。尾部仍待定时，这次读取
也保持待定。丢弃这次
`next()` future 后，只要仍持有 collector，就可以用 `result()` 或 `.start()` 接续排空
同一个源流并复用缓存结果。读到公开 EOF 后调用 `result()` 也会复用该结果。收到成功
`Done` 后丢弃直接迭代的 collector，会在当前位置释放源流，再按缓存消息结算一次已启用的
poll observer。最后一次读取已到达待定尾流时也适用。丢弃不会发出远端取消。

从 Pi 的 `Models.streamDeferred` 迁移时，在 Tokio runtime 内调用
`Models::stream_deferred(...).start()`。启动新建的 collector 会在首次读取前调度 fetch。
任务缓存首个成功 `Done` 后即可转发，再复用同一排空路径。希望在消息可用时继续业务，
就在循环内处理 `Done`。Pi 的循环可在源流尾部仍待定时结束。Rust 读到公开 EOF 或等待
`result()`，可观察源流释放及已启用的 poll observer 结算。直接读到 `Done` 后再调用
`.start()`，任务接手保留的尾部并复用同一结果。丢弃启动后的句柄会停止消费，任务继续驱动
源流并结算 poll。

若 `abort()` 在缓存成功 `Done` 前停止任务，它会等待源流释放并返回类型化 `Cancelled`。缓存后，
`abort()` 仍等待源流释放，并返回缓存结果，即使 `Done` 尚未被读取。runtime 在缓存后停止
任务时，等到任务退出也会复用该结果，并在源流释放后结算 poll。缓存前关闭 runtime 会收到
类型化流错误。这些中断作用于本地 fetch。远端取消使用独立的 `cancel_deferred` 入口。

SDK 按 handle 的精确 API 和 provider 查找注册项，再检查 model，然后才解析鉴权。
没有匹配的 provider/API 注册项时，`Models::validate_deferred_handle` 返回
`DeferredError::Unsupported`。注册项的 model 与 handle 不符时返回
`DeferredError::RouteMismatch`。fetch 和显式取消都以 `ProviderError::Config` 报告
这两种失败。查找不到注册项时，诊断会指明请求的 provider 和 API。

handle 决定使用哪条 deferred 注册项。应用若另外保存了预期路由，可以在调用前
与 handle 核对。派发后，由后端按 job ID 裁决能否读取结果或取消任务。SDK 将后端返回的
类型化 fetch 错误保留在 `CallFailure` 的 cause 中。

`DeferredFetchCall::wait_ms` 设置 Provider 的长轮询预算，单位为毫秒。零表示检查一次状态。
到达后端的每次调用只派发一次 fetch，等待值原样透传。后端决定何时返回待定或就绪结果。
后续 fetch 由调用方发起。

省略 `now_ms` 时，SDK 将整份 handle 交给后端，包括存在时的 `expires_at_ms`。
结果能否继续读取由后端裁决。调用方重开后可用保存的 handle 继续 fetch。

`DeferredFetchCall::now_ms` 启用本地过期检查。Provider 明确给出结果读取的硬截止时间，
且调用方有可比较的 Unix epoch 毫秒时钟时，可启用此检查。到达或超过截止时间时，SDK 在解析
鉴权和派发 fetch 前返回 `Err(CallFailure)`，原因为 `ProviderError::Config`。调用
`Models::validate_deferred_handle` 可直接取得 `DeferredError::Expired`。

收到 partial 前合成的失败消息会保留 handle 中的 provider 和 model ID。

三种入口均使用 `DeferredFetchCall::cancel` 控制本地 fetch。如果取消在终态缓存前结束 fetch，
结果为 `Err(CallFailure)`，其原因是 `ProviderError::Cancelled`，
消息的停止原因为 `StopReason::Aborted`。SDK 释放待定源流。遥测记录正常时，直接
`fetch_deferred` 在返回前结算已启用的 poll observer。如果成功的 `Done` 已缓存就绪结果，
取消可以在自然 EOF 前结束仍待定的尾部读取。SDK 释放本地源流，遥测记录正常时按缓存结果
结算已启用的 poll observer。直接 fetch 返回该结果。直接迭代若正等待最后一次读取，该取消会使它返回
公开 EOF。使用独立的
`Models::cancel_deferred` 执行 best-effort 远端取消。

Deferred route 通过类型化 deferred API 注册。没有隐式 startup fetch 或全局 deferred
worker；确定性的 Faux backend 也不代表真实远端服务。

## 公开 Transport 边界

使用公开的 `Models`、`Request`、`ChatCall`、`ChatStream`、`HttpClient` 和 Provider
注册契约。内部 frame codec、压缩状态、resource owner 和协议 fixture 布局可以变化，
而不改变这些契约。精确 trait 签名和 option 字段属于 rustdoc；本文只记录外部调用方
需要依赖的行为。

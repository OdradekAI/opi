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

TLS 配置失败在请求发布前报告。client 保持与 HTTP 相同的超时、取消和脱敏规则；
关闭调用方持有的 stream 会按正常生命周期释放 source resource，不会通过丢弃全局
pool 来处理。

## WebSocket Upgrade

`ws::upgrade` 在共享 `HttpClient` 上执行严格动态 Upgrade。公开 transport preference
可以选择 SSE、WebSocket 或自动 route。自动选择保留 Provider 的 fallback policy，
不会把 HTTP 失败变成无限重连。

握手在返回 upgraded stream 前校验 HTTP status、version、`Connection` 与 `Upgrade`
header、`Sec-WebSocket-Accept`、subprotocol、singleton extension 参数和
permessage-deflate 范围。非法 negotiation 在产生 Provider event 前失败。升级后的
stream 继续使用其 source proxy/TLS policy 和调用方 cancellation token。

frame、PMD 和 IO owner 是实现细节。消费者使用 Provider 或 `ChatCall`/`ChatStream`
契约，不依赖 codec 类型。如果 backend 没有公开 WebSocket route，应使用其文档化的
SSE 或 HTTP route。

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
`ChatStream`，`Models::fetch_deferred` 将其推进到一个 ready assistant result 或一个
类型化失败。

Pending 结果对 token 和 cost 累计来说不是终态。Ready 结果携带最终消息；`poll_after_ms`
是 Provider hint，不是隐式 sleep policy。`cancel_deferred` 是 best-effort 远端取消，
与取消本地 fetch 相互独立。被取消或被取代的操作不能迟到发布结果。

Deferred route 通过类型化 deferred API 注册。没有隐式 startup fetch 或全局 deferred
worker；确定性的 Faux backend 也不代表真实远端服务。

## 公开 Transport 边界

使用公开的 `Models`、`Request`、`ChatCall`、`ChatStream`、`HttpClient` 和 Provider
注册契约。内部 frame codec、压缩状态、resource owner 和协议 fixture 布局可以变化，
而不改变这些契约。精确 trait 签名和 option 字段属于 rustdoc；本文只记录外部调用方
需要依赖的行为。

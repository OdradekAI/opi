# 费用与计价参考

本专题包含 `opi-ai` 的详细 usage 估算器、sampling、计价 tier、缓存计价、
service tier 和兼容规则。主 [README](../README.zh.md) 保留接入路径和简短的
best-effort 费用语义；完整表格与优先级规则以本文档为准。

## 估算器、Sampling 与计价元数据

`context` 估算器按每 token 3.5 个 UTF-16 码元（`ceil(chars * 2 / 7)`）与每图
4800 等效字符计算，并在消息粒度向上取整：user／toolResult 消息整块聚合取整一次，
assistant 消息对 text/thinking/tool-call 字符聚合取整一次，system 消息对正文加
sections 取整一次，added／removed 工具列表的序列化 JSON 各自单独取整。最后一个
适用的正向成功 usage 充当锚点（不限模型身份、非 Error／Aborted、时间戳不早于前
缀）；只估算锚点之后的消息，没有同模型／全文下界。独立上报的正向
`Usage::total_tokens` 即使与桶和不同也优先；显式 0 或缺失时仅当父桶已上报才回退到
桶和。未知与零处处区分，子集永不重复相加。

| 参数 | 单位／公式 | 省略 → 默认 | 零／null／未知 | 优先级 |
|---|---|---|---|---|
| 文本估算 | `ceil(utf16 / 3.5)` | 空 → 0 | — | — |
| 图片估算 | 每图 4800 字符 | — | — | — |
| usage 锚点 | total 或桶和 | 无锚 → 内容估算 | total 0 → 已上报则取桶和；未知 → 跳过 | total > 桶和 |
| simple 上限 | `clamp(M)`：窗口 ≤ 0 时 `max(1,M)`，否则 `min(M, max(1, W−E−4096))` | `None` → 模型默认输出 | 显式 `M` 不再按模型 max 二次封顶 | request < 窗口算术 |
| 数字 Claude thinking 预算 | 等级表 1024/2048/8192/16384（xhigh/max → high），再夹取留出回答空间 | 省略预算 → 等级默认 | native enabled 且省略预算用 1024；simple 的零预算按各 wire 处理 | 等级默认 < 显式预算 |
| sampling 参数 | 可选、按键合并的 JSON 映射 | `None` → 缺失 | `Some({})` 保留显式空映射；缺失键保留下层值 | 模型基座 < 生效等级 < request |
| temperature | 具名字段 | `None` → 按适配器门控省略 | NaN 省略 | sampling 映射覆盖具名字段（最后合并） |
| 计价 tier | 以 input+cacheRead+cacheWrite 选择，严格大于，取最高，相等重复取首个 | 无 tier → 基础费率 | 保留有限负数／小数阈值；仅 `> -1` 的阈值可命中 | 命中费率适用于整个请求 |
| 1h 缓存写入 | 子集按 2× 输入费率 | `None` → 全部短缓存 | 显式 0 有效，与缺失区分 | 子集，永不重复计费 |
| service tier | Responses：flex 0.5×、priority/fast 2×（`gpt-5.5`：2.5×）；Codex fast 和 Azure 所有 tier：1× | request/response 回退依选定 wire | 对 tier 敏感的 wire，未知 tier → 未知费用 | 作用于每个成本分量 |

`ModelInfo::sampling_params` 与 `Request::sampling_params` 使用
`Option<SamplingParams>`：`None` 表示缺失，`Some(Default::default())` 表示显式空映射。
`with_sampling_params(map)` 保留 `Some(map)`，包括空映射，且自有 catalog 存储及重开后
仍保留该状态。调用 `resolve_sampling_params` 时传入 `request.sampling_params.as_ref()`；
仅当模型映射、生效等级条目与 request 映射全部缺失时才返回 `None`。显式空映射使结果
保留为 `Some`，不会清除下层键；嵌套值整体替换，不做递归合并。

仅 OpenAI 兼容适配器（Chat、Responses、Azure Responses）合并解析后的 sampling 映射，且作为 body 的
最后一步，使模型与 request 的 sampling 覆盖具名请求字段；其他适配器忽略它。
`Usage::service_tier` 保留响应观察到的类别；`effective_service_tier` 保留选定
wire 的 request/response 计费规则。`calculate_cost_tiered(usage, pricing,
model_id, wire)` 在用量未知、价格无效或计费相关 tier 未知时返回 `None`。
OpenRouter 图片使用基础费率，不做 tier 选择。`PricingTier::input_tokens_above`
使用 `serde_json::Number`，保留精确整数与有限小数；Rust 整数字面量使用
`100_000_u64.into()`，小数使用 `Number::from_f64`。旧整数目录快照仍可读取，
不会舍入；阈值与 `u64` token 比较在超过 `2^53` 时仍保持整数精度。
native 保留各适配器的预算语义；
Pi Messages simple 透传自己的白名单。`ModelInfo::input_limits` 仅为元数据，
不自动开启拒绝或缩放政策。已退役的静态 `reasoning_effort` compat 键返回迁移错误。

## 公共入口

主要公共值包括 `Usage`、`CumulativeUsage`、`Pricing`、`PricingTier`、
`SamplingParams`、`ModelInfo::sampling_params` 和 `Request::sampling_params`。
费用计算通过 `calculate_cost_tiered` 暴露；估算器和 sampling resolver 位于
crate rustdoc 所描述的对应 context、request 和 model API。

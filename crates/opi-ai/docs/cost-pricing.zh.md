# 读取用量与估算费用

[English](cost-pricing.md) · [SDK 快速开始](../README.zh.md#快速开始)

## 读取响应中的用量

聊天响应的用量在 `AssistantMessage::usage`。供应商提供总数时，使用
`usage.total_tokens`，包括 `Some(0)`。总数缺失时，仅在
`usage.is_reported()` 为真时使用 `bucket_total_tokens()`。推理 token 已计入
输出 token，一小时缓存写入已计入缓存写入。各类别只计算一次。

`CumulativeUsage` 汇总对话中的用量类别，`has_unknown_usage()` 标记有用量
缺失的回合。部分响应也可能已有供应商上报的用量。图像生成和分类的
`usage` 与 `cost` 字段可为空。

## 区分账单费用、目录估算和未知价格

`usage.cost.total_from_provider` 为真时，`total_cost()` 返回供应商报告的总额，
包括零费用。分项金额仍来自目录，分项之和可能与总额不同。例如 OpenRouter 的
BYOK 调用可能同时报告网关费用和上游费用。`catalog_total_cost()` 汇总目录分项。

要显示可选的聊天费用，传入发起该次调用所用的 `ChatModelRecord`。
已知免费的模型会得到 `Some(0.0)`。用量未报告、费率缺失或无效，或者依赖服务档位的
协议拿到未知档位时，结果可为 `None`。需要保留响应当时的计费记录时，将原始用量随消息保存。

```rust
use opi_ai::message::AssistantMessage;
use opi_ai::{ChatModelRecord, calculate_cost_tiered};

pub fn chat_cost(reply: &AssistantMessage, model: &ChatModelRecord) -> Option<f64> {
    let usage = &reply.usage;
    if usage.cost.total_from_provider {
        return Some(usage.cost.total_cost());
    }
    calculate_cost_tiered(
        usage,
        model.info().pricing.as_ref()?,
        model.model_id(),
        model.wire_api(),
    )
    .map(|cost| cost.total_cost())
}
```

## 目录费率怎样应用

- 费率以百万 token 为单位。选计价档位时，将输入、缓存读取和缓存写入 token 相加，
  应用严格低于该总数的最高 `PricingTier::input_tokens_above` 档位。整个请求使用该档
  费率。总数等于阈值时适用更低的档位，没有更低档位时使用基础费率。
- 一小时缓存写入按输入费率的两倍计价。剩余写入使用短缓存费率。
- `Usage::service_tier` 保留供应商报告的档位。`effective_service_tier` 记录
  协议用于计费的档位，例如 OpenAI Responses 的档位。Codex Fast 无加价，Azure 无档位倍率。
- OpenRouter 图像生成使用基础费率。从 `ImagesResult::cost` 读取估算值。

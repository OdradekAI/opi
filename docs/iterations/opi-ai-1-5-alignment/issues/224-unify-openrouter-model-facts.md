# #224 统一 OpenRouter 共享模型的三个读取入口

Status: done

独立工厂原先手写六份模型参数。coding-agent 的列表使用这个工厂，因此同一 GPT-4o 的显示名、限制和价格会随读取入口变化。保留独立工厂六个 ID 的顺序，以 `resources/catalog/providers/openrouter.json` 为其中四个共有模型的数据来源。两个仅此入口提供的 Claude 兼容 ID 由工厂维护。

## 独立验收

- 无默认 feature 的 `build_catalog check` 重建并逐字节比较四条记录的小文件。生成器发现选中身份消失或改用另一种 wire 时，指出需要调整的工厂契约。
- `opi-coding-agent --test list_models` 调用真实列表装配，比较独立工厂和离线目录的完整类型化模型值。独立工厂使用调用方配置的端点。自定义端点和请求路径由 `openrouter_fixtures` 覆盖。
- 运行 `provider_openrouter_profile` 与 `run_provider_protocol_package.py --owner provider-openrouter`，验证仅选此 provider 的实际归档消费者能读取价格、限制和六个 ID。

以上为 #224 完成时的验收。[#228](228-sync-openrouter-catalog.md) 采用新发布目录后，独立入口同步撤下 R1 和两个目录外的 Claude ID。当前三个模型都从同一 provider 数据生成。

Test impact: add 列表、工厂和目录的跨入口回归。update 独立归档消费者。retain 代理和协议请求 fixtures。

# #225 在生成内置目录时校验模型声明

Status: done

公开读取器支持旧 bundle 并向调用方提供完整原始记录。维护者新增的内置声明还须能由 opi 使用。生成器复用 profile 身份、classifier/image 装配表及类型化记录投影检验路由与字段。chat 依端点种类核对可用 wire。添加路由时同时核对该声明与实际装配。顶层 `x-<名称>` 用于不参与行为的扩展，`x-opi` 由持久化编解码器保留。

## 独立验收

- `cargo test -p opi-ai --no-default-features --example build_catalog` 覆盖实际源树、嵌套重复键、已知字段拼写和类型、协议错配、特殊端点、请求 token 字段名、Mistral effort 与扩展字段。错误包含模型身份和字段；重复成员在解析阶段给出文件、键名和位置。
- `catalog_artifact` 用带未知字段的 v1 记录验证安装与原值回读。`catalog_operations` 和 `provider_collection` 检查实际组合根及操作入口。涉及协议语义的修改另跑对应的请求 fixture。
- 现有 OpenCode `grok-build-0.1` 的 Responses 记录保留原始 `supportsReasoningEffort: false`。此特例只匹配该身份、API 和值。

当时有 12 条 OpenRouter System One 记录声明可输入图片，适配器会拒绝。图片请求编码及新的作者声明校验已由 [#227](227-openrouter-system-one-images.md) 验收。

Test impact: add 生成阶段的正负例。update 公共 v1 原始记录回读。retain 公开 bundle 与组合根回归。

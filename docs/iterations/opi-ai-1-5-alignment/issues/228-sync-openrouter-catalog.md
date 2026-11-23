# #228 同步 OpenRouter 发布目录及各入口的模型下线

Status: done

Prerequisite [#227](227-openrouter-system-one-images.md)

用 [#226 的三方流程](226-sync-provider-catalog-three-way.md)，把 OpenRouter 从 Pi 1.3 发布目录的基线推进到固定 Pi 1.5 发布目录。参与合并的身份是 `(provider,type,id)`，沿用维护者选定参考后默认跟随新增、删除和更新的约定。

## 数据与入口

| 操作 | 旧记录 | 新记录 | 新增 | 删除 | 更新 |
| --- | ---: | ---: | ---: | ---: | ---: |
| Chat | 394 | 388 | 20 | 26 | 59 |
| Image | 59 | 64 | 5 | 0 | 0 |
| Classifier | 23 | 33 | 12 | 2 | 21 |
| 合计 | 476 | 485 | 37 | 28 | 80 |

合并无冲突。OpenRouter 的 485 条原始记录及顺序和参考相同。33 个分类模型里有 23 个声明支持图片，10 个只接受文本。四个 Claude latest 别名迁至 Anthropic Messages 并使用新的端点和 adaptive thinking。其余 41 份 provider 源文件逐字节相同。

Claude 的 simple 路径此前仅在会话中可切换 effort 的分支解析所选等级。迁移使这个问题在使用 adaptive thinking 的 OpenRouter 别名上可见。现在普通 adaptive 请求同样使用模型映射，关闭推理时则按模型的 off 映射决定是否发送 disabled。

固定的 Pi 1.5 OpenRouter 发布目录中没有 `deepseek/deepseek-r1`、`anthropic/claude-sonnet-4` 和 `anthropic/claude-haiku-4`。独立工厂与 coding-agent 列表也撤下这三个 ID。独立工厂只读取从同一 provider 源生成的 Chat Completions 选集，当前有 GPT-4o、GPT-4o mini 和 Gemini 2.5 Pro。调用方可以覆盖工厂端点。

## 独立验收

- 检查三个旧 ID 在目录与 coding-agent 列表的查询均为空，独立工厂和列表返回相同模型。逐条核对有效模型的参数。用临时源移除选中的模型，重建后的轻量资源应自动撤下它。
- 从同一 `Models` 发送已切换协议的 Claude 和新增的图像模型。本地服务器核对 Messages、`low` effort、Haiku 的关闭状态、以及 Flux 图片模型所用的模型 ID、路径和输出模式。更新十 wire 集合测试中代表 Chat Completions 的 OpenRouter 记录。
- 离线重建与 `check` 比较实际源数据和发行资源。独立 `catalog-bundled` 消费者从实际归档读取目录。

最初验收的 586 次定向 Rust 测试曾包含保留 R1 的旧预期。项目所有者随后明确，采纳 Pi 的模型下线要覆盖该 provider 的全部内置入口，增加兼容例外须事先征得同意。修正后的验收单列在同一回执的 `retirement_correction` 中。 [验收回执](../evidence/issue-227-228-openrouter-catalog.json)

## 下次上游基线

Mistral 和 OpenRouter 使用 [Pi 1.5 固定发布归档](../evidence/phase5-model-catalog.json)，SHA-256 为 `f4dbabe79cda4b38aee4a761b23fd5eaf526ac2e28fed3c9ee86e37ca0e5d71e`。本票结束时其余 40 个 provider 仍按 [#226 记录的 Pi 1.3 Git blob](226-sync-provider-catalog-three-way.md#mistral-基线) 作为上次采纳的基线。[OpenCode Zen 采纳后的基线](229-sync-opencode-zen-catalog.md#下次上游基线)

Test impact — add 源目录移除后重建选集的回归。update coding-agent 列表、独立归档消费者和注册表查询。delete 兼容记录与目录重复声明的测试。retain builtin 真实请求及旧 v1 回读。

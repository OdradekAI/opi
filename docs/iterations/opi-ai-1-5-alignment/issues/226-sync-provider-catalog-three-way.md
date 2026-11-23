# #226 按 provider 三方同步，并验收 Mistral

Status: done

0.x 维护时，选定发布参考后默认跟随上游的模型新增、删除与字段变化。按 `(provider,type,id)` 和对象字段区分上游修订与 opi 扩展。两侧以不同值新增同一身份、修改同一字段或一侧删除另一侧已改的记录，都需先解决冲突。相同新增可重复运行。数组作为有序值整体处理。

## Mistral 基线

| 上游基线 | 获取依据 | 下次使用范围 |
| --- | --- | --- |
| Pi 1.3 发布数据，`ffa3e98626230f378f80200debb736c4a2238b5f2ba246a92cfc7d8e19604678` | Git 对象 `980b7524c1b52d4bc95fd0a0c56c6e2fd7db36a0:crates/opi-ai/resources/catalog/builtin/catalog.json`；同提交 manifest 的 `artifact.revision` 是可信 pin | 此轮未迁移的 41 个 provider |
| Pi 1.5 发布数据，`f4dbabe79cda4b38aee4a761b23fd5eaf526ac2e28fed3c9ee86e37ca0e5d71e` | [固定发布归档](../evidence/phase5-model-catalog.json)，源码快照 `.pi/pi-1.5.0/nix/model-catalog.json` 提供 pin | Mistral |

上表记录 Mistral 试点完成时的基线。OpenRouter、OpenCode Zen、OpenCode Go 和 Amazon Bedrock 后续采用同一固定发布参考。[当前基线](231-sync-amazon-bedrock-catalog.md#下次上游基线)

从 Git 取旧目录时应保留 blob 原始字节。供 `sync.py` 使用的 pin 文件格式为 `{"revision":"sha256-<digest>"}`。生成器与 crate 的正常构建只使用 opi 数据。操作步骤见[目录维护指南](../../../../crates/opi-ai/docs/provider-auth.zh.md#离线内置-model-catalog)。

例如，上游删除了 opi 已经调过上下文窗口的模型，工具会返回冲突。默认采纳下线。维护者在临时副本中把冲突记录还原为上游旧值，重跑合并，检查聚合目录、独立工厂和产品列表均已撤下该身份，再记录新基线。若需要保留旧 ID 或增加其他兼容行为，应先说明具体消费者和与 Pi 的差异，取得项目所有者明确同意后实施。opi 自行新增的模型仍按本地记录维护。

## 独立验收

- 实际候选将 Mistral 从 40 条变为 34 条，新增 3、移除 9、更新 4，其他 41 份源文件字节一致。复核 Mistral 所有记录完整值与参考一致。
- 在临时 opi 源内添加一个自有模型并修改共有模型的上下文上限，连续通过真实 1.5 升级和单独固定的下一轮模拟升级。两个本地修改保留，后一轮独立的上游调价生效。
- 实际 builtin 的 `Models::complete_simple` 对 `mistral-large-4` 发出低推理强度，对新增的 `mistral-large-4-0-8` 关闭推理时省略 effort，并通过公开 lookup 确认 `mistral-nemo` 已下线。请求在本地 HTTP 服务捕获。
- 运行 Python 同步与冲突用例、Rust 生成器、`catalog_artifact`、`provider_collection`、`mistral_fixtures` 及 `catalog-bundled` 归档消费者。[本批回执](../evidence/issue-224-226-catalog.json)

Test impact: add 三方同步 CLI 的 6 项子进程测试及 Mistral 公开请求回归。retain 比较 CLI、目录读取、定价与原生协议 fixtures。

# #221 建立由 opi 独立维护的内置模型目录

Status: done
Owner: catalog/builtin、内置 provider 装配
Parent: [#209](209-maintain-module-test-and-catalog-boundaries.md)
Blocked by: none

opi 决定随 crate 提供的模型、参数和可用能力。迁移以现有 42 个 provider 的 1,705 条记录为起点，来源记录见执行证据。[#220](220-parameterize-catalog-source.md) 可提供后续对照。

## 维护与交付

- 可编辑数据按 provider 存放在 opi 版本库的 `crates/opi-ai/resources/catalog/providers/`。`build_catalog` 确定性地产出离线资源和 manifest，并用公开读取器校验。模型数据的修改通过普通代码审查采纳。
- manifest 记录 opi 格式版本和产品数据内容摘要。来源说明可记录种子数据的引用，产品包身份由实际交付内容决定。盘点公开的 `CatalogProvenance`、`CatalogGeneratorDigests` 和 `Catalog::from_bytes` 的调用方，验证既有 v1 bundle 可读，为公开字段的变化提供明确迁移示例，并更新英中发行手册。opi 自有产物必须能如实表示其来源。
- 初次迁移原样保留种子数据。后续选定发布参考后，按 [#226](226-sync-provider-catalog-three-way.md) 对每个 provider 默认跟随上游增删改，并保留 opi 自有模型与本地修订。新增模型使用已有 wire 时验证其 provider 和 operation 路由。声称支持新协议或新能力时需有对应的适配器行为证据。动态刷新由 [#222](222-refresh-radius-builtin-catalog.md) 负责。

## 验收与选测

1. 用迁移前的实际 bundle 校验一次 1,705 个身份及其完整模型值。用迁移后的 opi 源在无 Pi 源码、归档或网络的环境中生成两次，字节相同。错误或重复记录被拒绝，已有完整产物可继续使用。
2. 以临时数据新增一个 Pi 缺少的模型，并对一个共有模型修改字段。走同一打包与公开读入口，验证新身份和新字段，再通过对应 provider 的本地请求 fixture 验证操作能路由。测试数据只出现在 fixture 中。
3. 运行 `catalog_artifact`、受影响的 `catalog_operations`、`provider_collection` 和 `run_catalog_bundled_package.py` 外部归档消费。对价格、能力或请求字段的后续修改按实际影响选测。常规发版验收由 opi 源数据、公开行为和包内资源的一致性决定。

## 结果

- 42 份 provider 源文件在临时目录中两次离线生成相同的文件。1,705 条模型的完整值、列表顺序和 `catalog.json` 原始字节均与迁移前一致。目录 revision 由数据摘要确定，v2 manifest 声明 `CatalogOrigin::Opi`。
- 真实旧 v1 归档的 1,705 条记录通过公开入口回读并逐条核对。独立的临时 Groq 候选通过内置 provider 装配查询新增模型和修订后的上下文窗口，再通过本地 HTTP fixture 核对请求路径、模型 ID 和认证头。重复记录的 CLI 负例保全已有候选。
- 默认配置 152 项测试通过，独立 `catalog-bundled` 14 项通过，打包工具 3 项通过。真实 `.crate` 的独立目录消费者和 `cloud-auth+catalog-bundled` 组合均通过。Rustdoc 和定向 Clippy 通过，Clippy 保留 3 条既有诊断。详见[执行记录](../evidence/issue-221-opi-catalog-ownership.json)。
- 离线打包命令和 `CatalogProvenance::origin` 的下游迁移方式见[中文维护与迁移说明](../../../../crates/opi-ai/docs/provider-auth.zh.md#离线内置-model-catalog)。

Test impact: update 绑定 Pi 常量的产品断言及归档消费者。add 独立打包、旧格式回读与本地模型回归。retain 目录错误、操作分派与外部消费覆盖。

# 第二阶段：纯值、错误与认证目录边界

本报告承接[中立遥测切片](phase2.md)，对应[分离纯值、公共错误与认证目录契约](issues/03-values-errors-auth-catalog.md)。
工作树基点为 814c95f7ce575fb9be3df6b9d04ff6e2942706c1，开始时干净。
本票按已有方案实施，保留行为、旧公共路径和全部测试断言；未提交、推送或发布。

## 已实现的所有者

| 所有者 | 本次归属 | 保留的职责 |
| --- | --- | --- |
| `types` | 消息、模型/兼容元数据、用量/费用、图像/分类/deferred、API 身份、chat record，以及消息引用的 transcript/replay/工具声明值 | 确定性构造、序列化、验证与计费；不拥有取消、回调、客户端、执行任务或目录资产 |
| `errors` | `ProviderError` 及诊断、脱敏、原 service-fault 分类 | 分类规则、错误源链、部分消息封装保持；运行期消息快照构造仍属于 stream |
| `auth` | values、resolver、OAuth 合同、source context、ambient 实现各有所属文件；credential/mutation 移入 auth | 唯一 mutation queue、FIFO、ACK、waiter Drop、barrier 和持久化行为保持 |
| `catalog` | values、source、document、store_contract、builtin | decoder 不经 HTTP 实现；wire 不导入文件 Store 实现；文件 Store/guard/锁/提交仍为单一所有者 |

`ProviderError::is_retryable` 不再导入 Bedrock 实现。原 Smithy/Bedrock 名称与状态分类
成为错误值的纯规则，两条使用路径共用同一定义；没有把所有 service fault 改成统一
HTTP 重试策略。`safe_excerpt` 不再通过 HTTP 客户端模块回拉依赖。

模型头部验证直接依赖 `http`，版本仍为锁内的 1.4.0；reqwest 0.12.28 本身重导出的
就是同一 `http::header`。Cargo.lock 仅增加 opi-ai 的直接依赖边，没有升级或新增外部包。

认证合同不冒充纯值：`OAuthCredential::needs_refresh` 仍按需读取时钟，
`AuthContext::process_snapshot` 的默认实现仍按需读取 cwd；构造时点和副作用没有重写。
`CatalogRefreshContext` 带取消控制，留在 source 合同而非放入 types。

## 公共入口与消费者

| 既有入口 | 新所有者/推荐入口 | 兼容关系 |
| --- | --- | --- |
| `message` / `model_info` | `types::message` / `types::model` | 原模块是同一模块的重导出 |
| `stream` 中的用量、费用和终态值 | `types::usage` | 同一类型/函数；流消费仍在 stream |
| `images` / `classifier` / `deferred` 的数据 | 对应 `types` 子模块 | 调用控制和协议合同仍在原操作模块 |
| `models::{ApiId, ChatModelRecord}`、根 `ApiKind` | `types::{ApiId, ChatModelRecord, ApiKind}` | 同一类型，不复制字段或另建包装 |
| `provider` 中的公共错误、诊断；`http::safe_excerpt` | `errors` | 旧路径重导出；错误文本/分类/脱敏保持 |
| 根 `credential` / `mutation` | `auth::credential` / `auth::mutation` | 根路径和根精选导出继续有效 |
| `HttpCatalogSource::decode_document` | `catalog::decode_document` | 旧关联函数委托同一离线函数 |
| `catalog::http_source` 的 source 合同 | `catalog::{DynamicCatalogSource, CatalogRefreshContext, CatalogDocumentError}` | 旧子模块路径仍重导出同一声明 |

逐符号映射追加在[契约索引](contract-index.json)的 `IMPL-03-FOUNDATION-BOUNDARIES`，
不改写第一阶段的源码定位和观察。原 `opi-agent` / `opi-coding-agent` 高级调用者无需转换；
通过工作区 all-targets Clippy 验证其类型与路径仍可编译。仓库外未知消费者未被推定为不存在。

## 验证与测试归属

完整命令、失败尝试、源码身份和每个 target 的结果见[检查记录](evidence/phase2-foundation-checks.json)。
各行存在覆盖重叠，不能把基准、复跑、独立探针和包运行相加当作独立能力数量。

| 检查 | 已观察结果 |
| --- | --- |
| 改动前 11 个受影响合同 target | 371 tests PASS |
| 改动后同组合同 + 库内测试 | 740 tests PASS（包含 369 个库内测试） |
| 补充 28 个集成 target | 773 tests PASS |
| 独立源码隔离探针 | 仅 types/errors、5 个直接依赖；原有 8 个消息测试 PASS |
| 工作区 all-targets Clippy（排除 opi-eval） | PASS，`-D warnings` |
| 工作区 doctest / rustdoc（排除 opi-eval） | 11 doctests PASS；rustdoc 保留 `-D warnings` 通过 |
| 实际包消费 | 新打包的 ai + telemetry；3 个完整接入程序及 1 个 peer 回归 PASS |

包的普通外部消费者只使用实际解包候选，未 patch 工作区源码。
执行覆盖普通完成、streaming 和工具续轮，包含构造/未 poll 时的零 source/store/network
访问观察。具体产物身份见[包消费记录](evidence/phase2-foundation-package.json)。
fmt、文档合同、本地链接与新增文件行尾检查通过。

源码隔离探针复制真实文件并逐一校验 SHA256；其解析图没有 reqwest、Tokio、TLS、云 SDK、
opi-ai 或 opi-telemetry。它证明源码基础依赖可以独立成立，**不是一个新发布 crate**，
也不是 opi-ai 已提供最小 feature 配置的声明。记录见[隔离证据](evidence/phase2-foundation-isolation.json)。

测试影响：update 六个现有合同文件的 import/decoder 调用，原消息内测随源文件移动；
retain 全部既有断言、输入类别和消费者路径；delete none。没有新增产品测试探针，
没有为了搬文件复制另一套行为测试。新 canonical imports 与其余旧路径测试互补验证同一能力。

迁移中编译器发现的缺失 import、私有字段访问和 test-module 后的重导出已修复，未关闭 lint。
rustdoc 的重复链接诊断曾没有源码位置：独立 JSON 诊断定位到 OAuth `CredentialStore` 链接，
两个最小文件复现 outer/inner 文档合并后的错误 span；只改该链接的镜像编译通过。
该链接在基准已存在，不能把它说成本次新增的运行行为缺陷。
额外 `--document-private-items` 诊断还暴露旧 `Models::check_auth` 私有文档链接；
该诊断命令失败保留，标准公共 rustdoc gate 已通过，不宣称 private-item 文档检查通过。

## 范围限制与下一步

本票没有运行全工作区 all-targets 测试，也没有把上一切片的 schema、spec_ledger、
Vertex 间歇性失败标成已解决。相应原始证据保留在[中立遥测报告](phase2.md)。
MSRV 1.97、Linux/macOS 原生运行、真实 provider、完整 TS/Rust 差分与六门里程碑未验证。
本机证据使用 Windows x86_64、Rust/Cargo 1.99.1。

内嵌目录、网络/云认证、tracing 适配仍在当前默认依赖闭包；本次隔离所有者，后续
功能配置阶段才提供默认完整/最小/定制构建。README 仅同步本次入口和职责，整本任务式
重写仍按原计划放在文档阶段。

下一步进入第三阶段：分离请求执行者与消费句柄，接入 AI instrumentation，并迁移
Models/Provider/API 的编排。重点逐项验证“不消费/停止消费、显式取消、异常和结算”，
不会用本次结构重排的通过结果代替生命周期对齐证据。

# Rust crate 如何维护内置目录数据

2026-11-20 核对。Cargo 文档、Rust 标准库和 Serde 文档说明工具能保证什么。下列两个项目展示如何发布随库更新的数据。对 opi 的判断属于讨论建议。

## 目录与发布边界

Cargo 对 `src/`、`examples/`、`tests/`、`benches/` 有约定和目标发现规则。它没有为任意产品数据指定 `resources/`、`assets/` 或 `data/`。项目可以按谁编辑、谁消费来选位置。区别在发布清单上，`[package].include` 和 `exclude` 控制归档，`cargo package --list` 列出文件，默认的 `cargo package` 还会解包并构建。使用可选功能的消费者仍需要验证相应的功能组合。[Cargo 包布局](https://doc.rust-lang.org/cargo/guide/project-layout.html) · [Manifest 打包规则](https://doc.rust-lang.org/cargo/reference/manifest.html#the-exclude-and-include-fields) · [cargo package](https://doc.rust-lang.org/cargo/commands/cargo-package.html)

`include_bytes!` 在编译时取得字节，`include_str!` 还要求文件是 UTF-8。两者都按调用文件解析相对路径。嵌入 JSON 后，语法、字段和跨记录关系分别要由解析器和领域代码检查。如果选择让下游构建时生成数据，Cargo 提供 `build.rs`、`OUT_DIR` 和 `rerun-if-changed`。预先生成并提交快照也能满足离线消费，生成器由维护者运行。[include_bytes!](https://doc.rust-lang.org/std/macro.include_bytes.html) · [include_str!](https://doc.rust-lang.org/std/macro.include_str.html) · [Cargo 构建脚本](https://doc.rust-lang.org/cargo/reference/build-scripts.html)

## 两个实际发布方式

| 项目与固定提交 | 数据、生成入口、消费方式 |
| --- | --- |
| Jiff，`623ea08d41d4ad0c647585eed9405e6b76d2d27e` | 维护脚本调用 `jiff-cli` 将 IANA 时区数据编译、整理成 `jiff-tzdb` 根目录下的 `concatenated-zoneinfo.dat` 和 Rust 索引。库用 `include_bytes!` 嵌入数据，公开按名称查找和版本查询。包清单列入 `/*.dat` 和 `/*.rs`。[维护脚本](https://github.com/BurntSushi/jiff/blob/623ea08d41d4ad0c647585eed9405e6b76d2d27e/scripts/release/pr-tzdb-update) · [库入口](https://github.com/BurntSushi/jiff/blob/623ea08d41d4ad0c647585eed9405e6b76d2d27e/crates/jiff-tzdb/lib.rs) · [包清单](https://github.com/BurntSushi/jiff/blob/623ea08d41d4ad0c647585eed9405e6b76d2d27e/crates/jiff-tzdb/Cargo.toml) |
| rustls/webpki-roots，`b25be3db37e41e1e9f0260d6d245854683c77f82` | 维护测试从 CCADB 获取根证书，按元数据核对证书指纹、解析证书，并与已提交的 `src/lib.rs` 比较。库直接导出类型化的信任锚点常量，抓取和生成所用的 `webpki-ccadb` 属于开发依赖。另有行为测试验证名称约束确实生效。[生成测试](https://github.com/rustls/webpki-roots/blob/b25be3db37e41e1e9f0260d6d245854683c77f82/webpki-roots/tests/codegen.rs) · [包清单](https://github.com/rustls/webpki-roots/blob/b25be3db37e41e1e9f0260d6d245854683c77f82/webpki-roots/Cargo.toml) · [行为测试](https://github.com/rustls/webpki-roots/blob/b25be3db37e41e1e9f0260d6d245854683c77f82/webpki-roots/tests/verify.rs) |

这两个案例都把数据更新与使用者构建分开。它们分别交付字节和 Rust 常量，均可明确标识及审查生成结果。两者都从外部权威数据派生，不能据此决定 opi 应采纳哪些第三方模型。

opi 当前由维护者编辑分 provider 的 JSON，`build_catalog` 从这些文件离线生成快照。Pi 比较工具报告差异，由 opi 决定采纳哪些变化。若改为“默认跟随选定摘要对应的上游数据，仅记录本地例外”，应把它作为新的维护策略讨论。固定摘要可以重现输入，本地删除项是否会重新出现、冲突的价格以谁为准，仍需明确约定。[opi 生成入口](../../../../crates/opi-ai/examples/build_catalog.rs) · [数据所有权](../issues/221-reconcile-pi-1-5-catalog.md) · [Pi 对照工具](../issues/220-parameterize-catalog-source.md)

## 字段错误与行为错误

- **Rust 结构体。** 显式写全字段的结构体字面量遇到拼错或遗漏的字段会编译失败。若协议名、模型 ID 仍是字符串，拼写合法但无对应路由也能通过类型检查。[未知字段 E0560](https://doc.rust-lang.org/error_codes/E0560.html) · [缺少字段 E0063](https://doc.rust-lang.org/error_codes/E0063.html)
- **JSON 与 Serde。** 派生结构体读取 JSON 时通常忽略未知字段。`deny_unknown_fields` 可拒绝它们，`default` 可为缺失字段提供默认值，普通必填字段可报缺失。Serde 明确不支持把 `deny_unknown_fields` 与 `flatten` 结合。如果公开读取器承诺保留未知扩展，维护者输入仍可单独检查已知字段的拼写，扩展如何标识则需按契约决定。[Serde 容器属性](https://serde.rs/container-attrs.html) · [字段属性](https://serde.rs/field-attrs.html) · [结构体读取示例](https://serde.rs/deserialize-struct.html) · [opi 保留额外字段的测试](../../../../crates/opi-ai/examples/build_catalog.rs)
- **opi 的验收。** 从同一批输入重建并逐字节比对，可以发现提交的快照过期。用已发布的查询入口加载快照，可以检查消费者是否读得出来。两项即使通过，填错但类型正确的价格仍会被忠实保留，价格单位要对照提供方资料。模型若声明支持一项操作，还要把它交给相应适配器验证请求行为。针对这几类错误，应分别选择证据。[离线目录维护约定](../issues/221-reconcile-pi-1-5-catalog.md) · [opi 公开读取入口](../../../../crates/opi-ai/src/catalog/builtin.rs)

## opi 与 Pi 1.5.0 的实际维护路径

本次核对 opi 工作树基于 `980b7524c1b52d4bc95fd0a0c56c6e2fd7db36a0`，含 #209 系列的本地交付。Pi 使用 `.pi/pi-1.5.0`。Pi 的 `packages/ai/catalog/providers/groq.ts` 指定来源、协议、兼容选项和一条模型修正。生成的 `src/providers/groq.models.ts` 导入 `data/groq.json`，运行时 `src/providers/groq.ts` 将模型交给同一套认证和协议工厂。Pi 自行扩充内置模型有直接例子，`catalog/providers/openai.ts` 用 `addMissing` 加入上游缺少的记录。应用用户自己的目录另见 Pi 的 `packages/coding-agent/docs/models.md`。[Pi 目录说明](../../../../.pi/pi-1.5.0/packages/ai/catalog/README.md) · [Groq 生成包装](../../../../.pi/pi-1.5.0/packages/ai/src/providers/groq.models.ts) · [OpenAI 维护规则](../../../../.pi/pi-1.5.0/packages/ai/catalog/providers/openai.ts)

| 可复核观察 | 对维护工作的影响 |
| --- | --- |
| opi 在 9 个 profile 源文件中还提供公开的 `model_catalog()`。以 OpenRouter 为例，同一个 `deepseek/deepseek-r1`、同一个 `openai-completions` 协议，聚合目录的上下文和输出上限是 64,000 / 16,000，直接构造器手写了 131,072 / 32,768。coding-agent 的模型列表也读取这个直接入口。[目录记录](../../../../crates/opi-ai/resources/catalog/providers/openrouter.json) · [直接入口](../../../../crates/opi-ai/src/providers/openrouter.rs) · [列表装配](../../../../crates/opi-coding-agent/src/provider_factory.rs) | 价格、限制等共享事实应只维护一份。轻量入口可以继续选择自己的模型子集。对单独支持的路由或能力，要保存其区别及验证用例。例如 Mistral 直接工厂用 Chat Completions，聚合工厂可用 Mistral Conversations。[Mistral 直接工厂](../../../../crates/opi-ai/src/providers/mistral.rs) · [聚合注册](../../../../crates/opi-ai/src/providers/collection.rs) |
| 在临时 Groq 作者数据上，用既有 `build_catalog` 分别加入拼错的 `supportsStricMode`、字符串形式的 `supportsStrictMode` 和拼错的 chat 协议名，三次构建均成功。对照组把 `maxTokens` 改成字符串，构建失败。chat 可执行记录在另一步解码并读取已知选项。[构建器](../../../../crates/opi-ai/examples/build_catalog.rs) · [目录读取器](../../../../crates/opi-ai/src/catalog/mod.rs) · [协议字段解码](../../../../crates/opi-ai/src/catalog/wire.rs) | 建议在维护者生成候选时检查协议、操作及已知行为字段，对故意保留的扩展元数据有明确处理。公共读取器现有的原始字段保留和 v1 回读仍有消费者和测试。 |
| 当前目录保留 Pi 1.3.0 种子数据的完整值。本次重跑两条只读 CLI。Pi 1.5.0 的 Nix 固定发布目录有 1,712 条，与 opi 之间有 125 条仅参考有、118 条仅 opi 有、299 条共有记录字段变化。本地生成参考有 1,738 条，三类差异分别为 170、137、325。[种子与离线重建记录](issue-221-opi-catalog-ownership.json) · [比较命令、输入摘要](issue-220-readonly-catalog-comparison.json) · [Nix 固定输入](../../../../.pi/pi-1.5.0/nix/package.nix) | 建议 0.x 用 Nix 固定的发布目录作为一次同步的输入，在维护流程记录摘要。用上次上游、本次上游、当前 opi 三份数据区分上游删除与 opi 自增。两方修改同一字段、删除本地修改过的记录、上游加入已有本地身份，都要报告冲突。 |

调研时建议先验证 OpenRouter 同协议记录的两个入口及 coding-agent 列表，再选 Mistral 演练一次有增、有删、有字段变化的同步。对固定发布目录，当时 Mistral 有 3 条仅参考有、9 条仅 opi 有、4 条共有记录变化。新建、下线均按 `(provider, type, id)` 判断。本轮已采纳该顺序，实际验收见 [#224](../issues/224-unify-openrouter-model-facts.md)、[#225](../issues/225-validate-authored-catalog.md) 和 [#226](../issues/226-sync-provider-catalog-three-way.md)。

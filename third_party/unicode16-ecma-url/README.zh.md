# opi-vendor Unicode16/ECMA/URL 私有依赖分发闭包

面向 opi-ai source dialect（DEL-04，任务 20.8）的受控私有依赖分发闭包。
本目录固定精确输入；`scripts/pi-sdk-conformance/` 下的工具负责物化、打包并
证明。这只是依赖底座——尚无产品 crate 依赖它；任务 20.57 复用该证明来验证
最终组装包。

## 组成

| 身份 | 基线 | 版本 | 变更 |
|---|---|---|---|
| `opi-vendor-url` | url 2.5.8 | 2.5.8 | 仅 manifest 身份 |
| `opi-vendor-idna` | idna 1.1.0 | 1.1.0 | 仅 manifest 身份 |
| `opi-vendor-idna-adapter` | idna_adapter 1.2.1 | 1.2.1 | 仅 manifest 身份 |
| `opi-vendor-icu-collections` | icu_collections 2.0.0 | 2.0.0 | 仅 manifest 身份 |
| `opi-vendor-icu-locale-core` | icu_locale_core 2.0.0 | 2.0.0 | 仅 manifest 身份 |
| `opi-vendor-icu-normalizer` | icu_normalizer 2.0.0 | 2.0.0 | 仅 manifest 身份 |
| `opi-vendor-icu-normalizer-data` | icu_normalizer_data 2.0.0 | 2.0.0 | 仅 manifest 身份 |
| `opi-vendor-icu-properties` | icu_properties 2.0.0 | 2.0.0 | 仅 manifest 身份 |
| `opi-vendor-icu-properties-data` | icu_properties_data 2.0.0 | 2.0.0 | 仅 manifest 身份 |
| `opi-vendor-icu-provider` | icu_provider 2.0.0 | 2.0.0 | 仅 manifest 身份 |
| `opi-vendor-regress` | regress 0.12.0 | **0.12.0-ucd16.1** | 身份 + 重新生成的 UCD 16 表 |

"仅 manifest 身份" 指除 `Cargo.toml` 外每个归档成员逐字节不变；`Cargo.toml`
只增加 vendor 包名与 family 内部依赖的 `package`/`registry` 重定向（保留原始
依赖键，crate 内部 `use` 路径不动）。regress 分叉额外把
`src/unicodetables.rs` 与 `tests/unicode_property_escapes.rs` 替换为用 regress
自家上游生成器（固定 commit）在官方 UCD 16.0.0 数据上重新生成的表；分叉版本
使用 pre-release（`0.12.0-ucd16.1`），绝不标记为未修改的 crates.io
regress 0.12.0（其表为 UCD 17）。

`ryu-js 1.0.2` 经审查但**不**改名：纯 Rust、无 C、无 `build.rs`，内部
`unsafe` 隔离在安全 `Buffer` API 之后；作为普通上游依赖解析。

## 受控输入

`closure.json` 以 sha256 固定全部 18 项输入（归档 + UCD/emoji 数据），逐字
复制自冻结研究资产
`docs/research/pi-ai-opi-ai-alignment/offline-probes/readiness-schema/`。
`archives/` 保存精确字节。Unicode-16 生成由固定上游生成器在固定 UCD 16.0.0
+ emoji 16 数据上复现，每次调用运行两遍并与冻结表哈希比对——任何发布包都不
含构建期生成。

## 审查立场

- **Features**：URL/ICU family 保留上游 feature 表；family 内部
  `writeable` 依赖固定为 `=0.6.3` 且带 `alloc`（0.6.4 变更了 ICU4X 2.0.0
  代码所依赖的宏；固定方式与冻结研究图一致）。
- **原生代码 / proc macro / 构建脚本**：family 包内无新增 C、无新增 proc
  macro、无 `build.rs`。family 的普通上游依赖（如 zerovec、yoke 的 derive
  crate）保持上游属性，按上游审查。
- **MSRV**：family 包声明的 rust-version ≤ 1.82（保留上游值）；工作区 MSRV
  1.97 覆盖。生成器/准备图是独立的固定工具图，不是产品依赖。
- **许可/NOTICE**：每个包保留归档内的上游许可文件；`NOTICE` 记录分叉关系。

## 升级所有权

由 opi-ai 维护者持有。升级输入时：替换 `archives/` 中的归档、更新
`closure.json`（版本、sha256）、重跑
`scripts/pi-sdk-conformance/run_early_package_proof.py`、提交刷新后的证据。
证明对任何 pin 不匹配、生成不确定、超出既定变换的归档成员漂移或电池回归
均失败关闭。

## 发布状态 —— OPEN

`opi-vendor` 身份目前只存在于证明工具构建的**本地 sparse registry** 中。
真实 registry 发布是另行授权的步骤，在此之前保持阻塞（任务按契约停在这
里）。本地证明打包出的构件内嵌 localhost registry URL，发布时必须对目标
registry 重新打包。本地证明中 family 包的普通 crates.io 依赖携带 sparse
crates.io 索引 URL，cargo 将其视为独立（内容相同）的源——在真实的单一
registry 上所有依赖从同一源解析，不会出现该重复。

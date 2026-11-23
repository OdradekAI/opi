# 维护内置模型目录

[English](README.md)

## 编辑并检查源文件

修改[供应商 JSON 文件](providers/)。
文件名是供应商 ID，文件内用 `(type, id)` 标识记录。自定义元数据使用顶层
`x-<name>` 字段，`x-opi` 保留给持久化编码。修改模型协议能力时，在使用
该能力的 adapter 测试中补充 fixture。

使用工作区的共享 Cargo 缓存，
在仓库根目录执行。`build` 将结果写入新的候选目录，供审阅。
`check` 校验已提交的目录与源文件是否一致。

```sh
cargo run -p opi-ai --no-default-features --example build_catalog -- build crates/opi-ai/resources/catalog/providers catalog-candidate
cargo run -p opi-ai --no-default-features --example build_catalog -- check crates/opi-ai/resources/catalog/providers crates/opi-ai/resources/catalog/builtin
```

审阅生成文件后复制到 [`resources/catalog/builtin`](builtin/)。
生成结果包括 `catalog.json`、`manifest.json`，以及从独立 OpenRouter adapter 所用的
Chat Completions 记录生成的 `openrouter-profile.json`。应用候选结果后，重新运行
`check` 及受影响的目录查询与 adapter 测试。[选择相关测试](../../tests/README.zh.md)

## 比对或跟进 Pi 发布目录

用 `python scripts/pi-catalog/pack.py --help` 查看只读比对方式。可选择固定的发布归档或通过摘要
验证的本地供应商目录。比对结果应记录参考版本及 opi 目录的摘要。

更新某个供应商时，以该供应商**上次采用的发布目录**为旧输入，以**目标发布目录**
为新输入。每份 pin 文件的 `revision` 采用 `sha256-<digest>` 格式。

```sh
python scripts/pi-catalog/sync.py --provider mistral --old-artifact old-catalog.json --old-pin old-pin.json --new-artifact new-catalog.json --new-pin new-pin.json --opi-sources crates/opi-ai/resources/catalog/providers --output-dir sync-candidate
cargo run -p opi-ai --no-default-features --example build_catalog -- build sync-candidate/providers bundle-candidate
```

审阅报告后，再应用对应供应商文件和重新生成的目录。三方合并会保留本地独有的记录
和独立的元数据修改。退出码 2 会报告冲突且不写入候选结果，退出码 1 表示输入或输出
无效。将上游删除应用到该供应商和操作的全部内置入口。保留已退役的 ID 或添加回退项，
需要项目所有者明确批准。记录采用的发布目录版本，供下次更新使用。

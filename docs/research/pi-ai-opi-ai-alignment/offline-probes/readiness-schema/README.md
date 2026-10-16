# Schema 设计可表达性研究资产

本目录是 **研究代码／语料**，不进入产品 crate、测试或 CI。设计结论见[Schema 附件](../../pi-1.3.0-readiness-schema.md)。保留原始后端62/64和Unicode16适配64/64两个结果，不把成功子集标成完整等价；11 个错误传播控制为显式合成注入。43 source mechanisms／42 JSON mechanisms 与21 formats 是实施范围，未全覆盖。

`pins.json` 固定 Node、Pi utility、TypeBox完整tree、fixtures、harness Cargo和Rust源；生成器输入／上游commit／SHA见 `generator-inputs.json`，官方获取说明和准备失败见 `acquisition.json`。Node24.13.0、TypeBox1.3.27、rustc/Cargo1.99.1及现有外部共享target是本轮构建输入。所有依赖／UCD／emoji文件须先在明确的准备阶段取得并按pin核验；**脚本不下载、不安装、不启动服务、不使用凭据**。archive不随文档提交，没有整棵第三方vendor／native二进制／生成的1MiB表。

操作只使用独占TEMP scratch，保留共享Cargo target／incremental；不要指向产品或Pi源码目录。工具固定 `CARGO_NET_OFFLINE=true`、`--locked --offline`，环境白名单去掉真实token/proxy/NODE_OPTIONS等。下面使用语义占位符，实际历史命令及退出码在 `results/*/commands.json`；既有TypeBox缓存和共享target可从前轮 `schema/results/runtime-inputs.json` 定位。

```text
python generate_unicode16.py --assets ASSET_DIR --scratch GENERATOR_TEMP --target EXISTING_SHARED_TARGET
  --regress-archive regress-0.12.0.crate --codegen-archive codegen-0.3.0.crate
  --ucd-parse-archive ucd-parse-0.1.13.crate --regex-lite-archive regex-lite-0.1.8.crate
  --ucd-zip UCD-16.0.0.zip --emoji-sequences emoji-sequences.txt
  --emoji-zwj-sequences emoji-zwj-sequences.txt

python run.py --assets ASSET_DIR --scratch ORIGINAL_TEMP --target EXISTING_SHARED_TARGET
  --node EXACT_NODE_EXE --pi PINNED_PI_ROOT --typebox PINNED_TYPEBOX_PACKAGE
  --regress-archive regress-0.12.0.crate

python run.py --assets ASSET_DIR --scratch UNICODE16_TEMP --target EXISTING_SHARED_TARGET
  --node EXACT_NODE_EXE --pi PINNED_PI_ROOT --typebox PINNED_TYPEBOX_PACKAGE
  --regress-archive regress-0.12.0.crate
  --unicode16-table GENERATOR_TEMP/unicodetables-16.rs
```

每条命令各写在一行执行，行折叠仅为阅读。generator真实复制原始官方算法，只改 UCD_PATH 输入常量和build-only path manifest，运行两次并检查table/tests完全相同。`upstream-generator/*.rs` 是指定upstream commit的原文件；`Cargo.upstream.toml` 为原manifest，`.template` 为隔离path依赖wrapper，许可证一并保存。codegen/ucd-parse/regex-lite只作生成工具；没有产品 build.rs或生成器依赖。

研究Rust解释器仅覆盖所选fixture所需结构（约200行），省略formats、完整reference stack、evaluated sets、normalizer、完整guard/fractional/negative边界和预编译IR。它不是可以直接并入产品的validator。包含人工 `__probe_runtime_failure__` marker来测试Result传播，**生产设计没有这个保留pattern或注入器**。真实regress运行没有返回BacktrackLimitExceeded；Option接口不证明可硬中断或线性时间。

64语料=旧85语料中的39项＋25新增项。旧`normalize-*`在这里按JS parsed原输入验证，未执行native normalize；Pi public utility观察单独保留，RDF-09 root correctness不被撤回。每个对比child保留stdout/stderr与exit，30秒native／10秒contained-Node只是研究containment，不是SDK限制。原始输出精确字节压成 `raw-streams.jsonl` 的base64字段；可按commands.json逐条解码并核SHA。`pi.jsonl`／`native.jsonl`是方便读取的聚合视图，不能覆盖raw。

`results/generator/`保留两次生成日志；`results/original/`和`results/unicode16/`各含81条实际命令（版本/build/metadata/features、Node、native和注入控制），每条exit0。`verification.json`另记持久资产复跑／错误pin负控制；它不是产品验收。UTF-16孤立surrogate、所有ECMA语法、非默认Format全局修改、平台/MSRV和完整public consumer仍须各自验收。

另有独立Number/String与URL leaves，避免把它们混入64个validator案例：

```text
python run_leaves.py --assets ASSET_DIR --node EXACT_NODE_EXE --target EXISTING_SHARED_TARGET
  --archives PREPARED_ARCHIVE_DIRECTORY --scratch LEAF_BASELINE_TEMP --profile baseline
python run_leaves.py --assets ASSET_DIR --node EXACT_NODE_EXE --target EXISTING_SHARED_TARGET
  --archives PREPARED_ARCHIVE_DIRECTORY --scratch LEAF_ICU16_TEMP --profile icu16
```

`leaf-inputs.json`固定全部输入、ryu-js1.0.2、8项ICU16图归档；`leaves/*/Cargo.lock`是Cargo生成的两个精确图，`.template`是研究path manifest。基线43项中41同意，ICU16图43项全部同意；14个Number String和29个URL采样分别计数。`results/leaf-*`包含每条7命令的独立stdout/stderr/exit及元数据。ryu-js为safe API下的内部unsafe纯Rust实现；ICU图有其既有derive proc macros，不能从7-package regex主图外推“所有leaves无unsafe/proc macro”。初始旧ICU图因writeable未启用alloc而exit101，固定feature后才正式构建和验证。

普通ICU16图本身**不保证独立consumer的Cargo依赖统一隔离**。idna1.1.0没有可注入provider的public constructor，root lock不是SDK对外保证。下面的namespace probe另已真实执行同图冲突；它没有把最终opi-ai发布包、所有consumer组合或完整URL接受集标成PASS。

私有身份机制已另做实际冲突图构建：

```text
python namespace_probe.py --assets ASSET_DIR --archives PREPARED_ARCHIVE_DIRECTORY
  --registry-cache EXISTING_REGISTRY_ARCHIVE_CACHE --scratch NAMESPACE_TEMP
  --target EXISTING_SHARED_TARGET --node EXACT_NODE_EXE
```

使用已存在Python工具依赖`toml 0.10.2`，不自动安装。脚本只修改10个官方crate的package身份和内部依赖manifest指向，逐归档member核对所有算法源码不变；普通caller url/idna_adapter/ICU2.2同时存在，实际build、metadata、native、Node四命令exit0并43／43同意。`namespace/Cargo.lock`固定图，`namespace-inputs.json`固定源归档，`results/namespace`保留结果。它证明身份隔离可表达，不把研究的TEMP path family冒充已发布SDK包或已选定公开Opi领域crate。

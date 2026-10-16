# Schema／WS 最小离线差分：独立复审原始报告

2026-10-16。结论：**在已审输入和范围内，没有仍需修复的实质问题；本轮研究结论和文档修订可接受。** 这不是 Pi/Opi parity PASS、产品验收或实现准入；Schema、WS 和其余五组均继续 **OPEN**。

## 独立性与输入

Reviewer 未参与两组实验的作者工作，也未改仓库、产品、文档或 index；只读仓库、作者 TEMP 资产并在独立 TEMP 执行复核。固定输入及复核时 HEAD 均为 `8cb0fd68db528ae3d3c61874458d45e46bffa29c`。本轮没有提交／推送。共享 Cargo target 的编译输出是允许的研究副作用，不是产品源码变更。

已审主报告、双语 spec 本轮全部差分、现行 backend／RDF／V／tracking／README，以及两个研究 harness、固定 fixtures、原始结果、归档／锁图来源和聚合 evidence／reviewdoc。聚合文件当时状态是 `CHECKS-PASS-REVIEW-PENDING`，errors为空；待填独立报告与最终机械 hashes 的部分不冒称已经冻结。其后仅去 pending、纳入本报告／检查记录和刷新 hashes 不改变本次语义结论。

直接核对 Pi 原始 `validation.ts`／公开导出和 Codex 的 constructor 选择／调用源码。Schema 真正调用 `validateToolArguments`，仅 loader 映射 TypeBox，未复制算法、无 stub；WS 执行相同 global constructor 调用形状，未冒充完整 public provider。参考是精确 Node 24.13.0 / Bun 1.3.5+1e86cebd7 本地 exe；Bun 的 Node-compatible `process.version` 没被当成 Bun 身份。其余 Node/Bun、发布 tag 和完整发布包仍未证明。

## 实际独立复核

| 检查／命令 | 实际结果与范围 |
|---|---|
| 持久 Schema `run.py --scratch <review TEMP>/final-schema-replay --target E:/opi/cargo-targets/windows-direct`，明确 Node/Pi/TypeBox 路径 | **29 个命令全部 exit0**，实际包含 `cargo build --offline --locked`、metadata、features 和所有 Pi/native 普通／隔离进程 case。未下载、未调用供应商，复用了既有共享 target/incremental |
| 将独立重播的 `pi-all.jsonl` / `native-all.jsonl` 与持久 `results/pi.jsonl` / `results/native.jsonl` 解析后比较 | **85 Pi case 对象、276 native 对象逐字段完全相同**；输入 fixtures、loader、runner、Rust source、manifest 和 lock bytes相同。这已覆盖前期只精选样本复跑的范围 |
| 独立重播解析图对持久 graph | **74 package 的版本／有效 features 相同；73 registry dependency edges 相同；73 archive SHA256 均等于锁 checksum**。lock SHA256 `eb20a03f072f3285dba88397583cca8a4a710c72b613dbfc9240ea56832a742f`；不推成产品 feature union／MSRV／平台证明 |
| 本次重播 native exe | SHA256 `8430d137d9dd95a174712eae05fa5e7e70d1824e9e628f5ce87af2d137c8b293`，另记于 reviewer runtime-inputs。没有冒称作者历史 `baf23...` binary；共享 target 被其他相同源图 scratch build 覆盖后 hash 可不同，历史运行记录保留 |
| Schema 错 Node pin 的实际 CLI 负向 | exit1，`Node executable pin mismatch`，没有 commands.json／没有启动 child command；只在 reviewer 新 scratch 留输入副本 |
| TypeBox 1.3.27 tgz 对提取内容 | **1385 file members 全部逐字节相同**，archive SHA256 `5a288bcd3d68f44567de84b185e04d205ef7d5b53602846bc3db30fdd219dda3` |
| WS 精选重复 baseline / 125 headers / PMD compressed | 每个 Node/Bun 两种 constructor 形状，共 **12 child 观察**；另 **3 native loopback**。baseline成功，Bun125成功／native TooManyHeaders，Node125交付后异常exit3221226505，Node/Bun解压PMD与native NonZeroReservedBits均复现。它们是 reviewer 的早期 builtin-only 过滤副本诊断，独立于作者正式 --no-install 60 行，不代替正式受控结果，也未执行 provider import |
| 正式 WS 全部 raw 的结构／退出码／命令边界 | **60 行**，exit0=52、exit1=2、3221226505=6；Bun正式30行均包含 `--no-install`。两类 Node crash、Bun非法响应意外宽松和无协商的native extension接受被分开，未挑选字符串冒充通过 |
| TW／TTW完整源码 | 两个发布归档 **38+22=60 file members 全部逐字节相同**。path patch指向完整原发布源码；初次缺模块构建失败没有被当成候选行为 |
| 持久 `prepare-ws-source.py` 正／负向 CLI | 新 reviewer TEMP 正向exit0、60members再核相同；错误SHA本地fixture exit1，未提取任何source内容。无下载器，无产品改动 |
| 持久研究资产清单 | Schema24 + WS28成员的bytes/SHA全部吻合；检查时合计55研究文件／989400bytes，无exe/crate/tgz/vendor树。新增独立报告及复核JSON由父最后另纳入清单，不与旧52项混淆 |
| `python scripts/opi-doc-check.py` | 最终语义稿复核时 exit0，`opi documentation contracts: PASS` |
| `git diff --check` | exit0；index无变更；HEAD仍固定输入，仓库改动局限于本轮研究目录 |
| AIR／V／双语差分 | 67条English AIR、67条Chinese AIR、59条canonical V要求行与输入逐字相同；最新观察表不冒充新V。双语新增选型／执行边界语义一致 |

Independent scratch 位于 `C:/Users/Luiz/AppData/Local/Temp/opi-pi130-offline-independent-review/`；关键记录是 `final-schema-replay/review-comparison.json`、`schema-negative-pin.json`、`ws-prepare-tests.json`。完整重播命令／stdout/stderr在该 scratch 的 results 内。这里只陈述实际核对的范围，不将作者结果与 reviewer 重跑相加为通过率。

## 发现与修订闭环

- 同意并修正关键命令只记录失败而未传播的问题：最终 Schema runner 对 exit／case数量／pins失败关闭，正式编译使用 offline+locked；错 pin 负向已独立执行。
- 同意按 fixture原值／JS parse值／Pi normalized 分层；参考 normalized 喂native不是Rust normalizer。大整数舍入、root-conversion漏洞和递归RangeError未被归一化为成功。
- 实际 regex 反例不是猜测 fuel 阈值：40 ASCII subject 在 leaf 触发真实 BacktrackLimitExceeded；not／if／patternProperties可在 validate=Ok／iter_errors=[] 下假成功。unselected-anyOf合法分支仍成功，不能 blanket预扫。additionalProperties假失败已补入摘要。
- 同意重新打开 Schema选型：未适配引擎／切draft并不足；content knobs和简单prefix hook只证明局部机制，没有预先批准完整 evaluator、private bypass 或替代库。
- 同意重评当前无PMD的WS完整组合：参考确有offer并解压的能力，codec没有。Rust拒绝未协商压缩本身不是bug；PMD缺失由真实参考＋源码／codec机制共同支持，未把强发extension响应作为合法协商oracle。
- Node崩溃和Bun非法响应宽松不是Rust目标；稳定125header差分使用Bun exit0。Connection token-list两参考也拒绝，未继续称其为本轮已证parity差分。
- partial attachment只证明首帧及两个样本第二次next的EOF观察；完整Upgrade验证、多帧tail恰好一次、取消、PMD组装仍OPEN。
- 同意并修正 fresh key 措辞：服务按各请求携带的key计算Accept；native使用固定RFC sample key做确定性实验，不证明随机nonce生成。此项未影响已证握手／codec结果。
- Bun默认import的自动解析副作用已如实披露：十文件cache摘要保全并移至TEMP，workspace node_modules已不存在；该诊断被排除于offline/provider证明，来源／联网范围未知。两个官方crate准备获取与此未知解析分开，未宣称整轮完全无联网。
- 现行backend §2.4／缺口表、双语模块选择和执行段、RDF、V新观察、tracking已同步：重开比较，无默认保留；旧未执行语态限定历史。没有由采样125／140headers、66000／70000bytes、regex subject或harness timer引入SDK预算。

## 结论范围与未审项

本复审支持本轮“已证反例 → 重新评估候选 → 未证明项保持OPEN”的文档结论。没有替代库比较或选型证明，没有完整 Pi/Opi provider、Rust normalizer、WS PMD参数／fragment／control／context takeover、WSS／proxy／redirect、取消／pool／cleanup、外部consumer、产品有效Cargo图、其他OS／MSRV／性能／真实服务证明。其余五组未获新增运行验证。没有以 Unsupported、更多cap或SSE成功关闭缺口。

产品 test impact **none**；研究 harness／fixtures／执行证据 **add**。本报告不是实现准入或正式规范变更。

## 语义复核稿摘要

以下 hashes 固定此次独立语义复核稿；聚合evidence的自身hash、去pending后的主reviewdoc和本独立报告的复制／self引用由父最终机械刷新，避免循环hash。这不授予未读语义改动的追认。

| 文件 | SHA256 |
|---|---|
| README.md | `e25a5894d9fa9ec3a6ab71cd4513aba403752a8b04c78e2be6bf5021be297288` |
| spec-draft.md | `3ff0e0eb3690889001a9d509b4ff7b65a967612324254339880315cc6222aa71` |
| spec-draft.zh.md | `2f3deb10bfacbc06c96eafeba8e61f4697573799f414453293b0d78786f417a8` |
| acceptance-matrix.md | `1f5fdca561b790259c4d3a46a50b8126c372203b0ceb0bc0b5fd1286d27f0873` |
| pi-1.3.0-rust-backends.md | `3dde78e5f3f568c7894195a84bcbe6027c159023ae073a98d624d683cd868cbf` |
| pi-1.3.0-rust-differences.md | `93df9a3819b6a284e31056c094bad22532232f9b0250f89b9cd2c16b6c873e20` |
| upstream-tracking.md | `056b241bed8b090196e040a0e133c2962d929d849e62d5cdb116d78300913073` |
| pi-1.3.0-schema-ws-offline-validation.md | `2c2fd814c9dde3cac2ddc688d9db5f3f88e230397e8462c936b9d2d79c0867c0` |

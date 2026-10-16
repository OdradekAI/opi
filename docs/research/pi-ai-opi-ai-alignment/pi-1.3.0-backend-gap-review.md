# Pi 1.3.0 后端专项：独立复审与验证

> 2026-10-16；输入 Opi `a37ed708dfdac3528bc7dfeba18f0df3af68cadc`。[专项核查](pi-1.3.0-backend-gap-audit.md)与[冻结证据](pi-1.3.0-backend-gap-evidence.json)限定本轮范围；旧升级复审/验证保留原输入，不作为本轮通过证明。只改文档，未准入或实施。

## 结论与仍未关闭的范围

本轮补齐七组缺口的决定性源码、可证伪语料、公开边界验证方法及有条件补救。Schema、WS、HTTP/proxy/H2/redirect、TLS/trust/crypto、auth/source retry、native tracing、Cargo/platform/catalog全部 **OPEN**。源码已证限制、未知参考运行时、候选补救及明确Rust差异分别记录，未将任何一组改为已覆盖。

jsonschema继续为有条件候选；metaschema/private bypass、mixed applicators/refs、近似grapheme和fallible regex通道先过机制可表达性验证。WS的provided-stream、自有完整Upgrade→from_partially_read、patch/重选是分别有代价的候选，不是本轮已选择的实现；移除接收cap不补握手/压缩，最终SSE成功不证明WS。没有采用fork、替代库、JS runtime、全量自研validator、新public framework或新默认限制。

双语spec、现行backend/RDF、唯一验收矩阵、README和tracking同步引用专项证据。67 AIR／59 V／20 RDF身份及其原始要求行保留；不新增验收/实施账本。41 integration＋7 source-test文件的已有处置保留。旧升级/参数复审与hash不改写成当前通过。

## 独立复审范围

三位reviewer均未参与本轮前置来源审计或候选写入，分别直接核对固定源码，而非仅接受审计摘要。

| Reviewer | 范围 | 结果与限制 |
|---|---|---|
| backend_gap_schema_review | Schema public机制/私有bypass、source-derived案例、meta/keyword/ref/format/grapheme/regex、normalization与调度，相关双语/RDF/V | 无剩余必须修正项；流程顺序和root export locator已校正。未执行Pi/TypeBox/native validator或证明完整适配可行 |
| backend_gap_transport_auth_review | WS runtime/fallback/握手/codec、HTTP/proxy/H2/redirect、trust/RSA/mTLS、AWS/ADC、tracing、graph/catalog关键源码与方案 | 无必须修正项；164个当前source文件摘要匹配。没有运行loopback/credential/process/consumer/platform，也未重建所有供应商协议 |
| backend_gap_docs_review | EN/ZH语义、AIR/V/RDF及scope、表格/链接、source/归档hash、七组对应和证据层次 | 无必须修正项；handler完整路径与p256版本/MSRV两项P3清晰度建议已采用；最终整合后的有限复核按原始报告追加记录 |

原始临时报告摘要在冻结证据保留；实质修订在下表落地，不以TEMP文件作为唯一长期结论。最终证据写入的自摘要排除，避免递归hash；reviewer审过的冻结输入与后续机械刷新分别记录，不声称其提前审过最终生成字节。

## 修订与发现闭环

| 问题／来源 | 核对与处置 |
|---|---|
| 审计整合误把plain-JSON coercion放在root Compile之前；Schema reviewer指出 | 同意。Pi validation.ts在optional-null/Value.Convert之后取得root validator，随后才manual coerce；专项§2按实际阶段重写，避免混淆compile与instance失败 |
| PI alias按snapshot根定义，但root export locator省略packages/ai | 同意。改为packages/ai/src/index.ts:47；不改变公开导出结论 |
| handler缩写、p2561.65可误作包版本；docs reviewer P3 | 同意。补完整auth/pluggable-auth-handler.js路径，区分p2560.13.2与rust-version1.65；相同reqwest归档按内容摘要合并，保留物理cache路径和members |
| 源码核查：WS检查被过度简写为header/TCP分包上限 | 修正为解析前累计成功read bytes/次数/平均长度，可包含首帧tail；另列124-header槽、Connection token-list及extension验证风险。不从npm ws推断Pi global运行时 |
| 源码核查：typ=JWT/kid默认与RSA/mTLS范围混同风险 | 撤回无直接依据的JWT header默认；保留显式alg与未审jws链；区分signing/verification和client-cert算法，参考接受集未知仍OPEN |
| 实验/平台/资源未知不能用文档关闭 | 每组给真实正反例/副作用/取消及公开consumer条件；metadata1s、adaptive/2026、trust、完整feature/native/platform和hydrated catalog仍明确缺证 |

## 实际检查与未执行项

最终冻结证据记录 `python scripts/opi-doc-check.py` 与 `git diff --check` 的精确命令、退出码及结构检查。另重算固定Pi的1925个文件、164个决定性source摘要，核对28个唯一归档/134个引用文件与归档member字节；校验双语484行数值、既有AIR/V/RDF、测试处置、Markdown表格/链接及源码anchors。计数与source-derived预期均不是产品测试通过。

本轮仅改10个alignment文档，没有stage/commit/push、Cargo/测试/CI/正式spec/ledger修改。共享工作区的既有runtime测试和snapshot删除由另一工作形成0e860aa；该提交不含本轮目录，另有ledger改动均不归本轮。冻结证据分别记录input/observed HEAD、index/unstaged/untracked，不能把其他提交当成本轮交付。

产品测试影响 **none**：add/update/delete均0。所有差分runner、Pi/TypeBox/native validator执行、loopback WS/HTTP/TLS/proxy、OAuth/metadata/process fixtures、Cargo解析/编译/feature/native/MSRV/三平台/package consumer、hydration/live供应商接受与性能实验均未运行；未安装/import/执行缓存第三方源码、联网取证或启动实现。未来只有对应V的实际证据能够改变OPEN状态。

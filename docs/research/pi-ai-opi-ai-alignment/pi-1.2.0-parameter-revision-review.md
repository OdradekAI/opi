# 参数与新增限制修订：独立复审结论

> 2026-10-16，文档候选。本轮输入由[baseline](pi-1.2.0-parameter-revision-baseline.json)固定；逐项分类/出处/处置见[参数来源审计](pi-1.2.0-parameter-audit.md)，精确官方发布归档见[source evidence](pi-1.2.0-parameter-source-evidence.json)。本报告不授予实施、正式规范准入或运行等价认证。

## 当前结论

无依据的默认容量、总deadline和额外内容拒绝已从当前parity合同撤回；Pi源参数按真实消费路径/作用域保留。backend默认与不可配置限制分别记录，Schema/transport机制选择降为有条件，不能用override或typed Unsupported关闭覆盖义务。双语spec、RDF、backend、SDK附件及唯一矩阵已同步。

三路独立reviewer均未参与本轮文档写入或前置来源审计；修后有限范围重读确认无剩余必须修正项。该结论只针对本轮文档的来源、接受集与一致性，不证明全部参数测量最优、backend符合Pi、已可编译/运行或服务接受。前轮“固定默认值/后端定型/无必须修正项”结论明确收窄，旧hash/verification仍只对应其冻结输入。

67 AIR（58 SDK＋9host）、59 V（55 SDK＋4host）和20 RDF保持；41 integration＋7 source-test文件迁移索引未删除。本轮测试影响none，未执行任何Rust/Node compile/runtime/test/live/credentials/metadata/process fixture；只读下载/解包官方源码不等于安装/执行。

## 实质修订

| 范围 | 修订结果与现有验收导航 |
|---|---|
| 流／队列／retained state | 撤回SSE/WS/AWS/compact/tool/opaque/partial/terminal的猜测bytes/count/depth cap；现opi64事件为调度差异，保留背压及大合法event可进展/完整终态，不称固定RSS；V-02/04/05/25/40/41/52 |
| media／classifier | 撤回旧8图/媒体/文本/body额度和必须有图、signature/四MIME/strictbase64、统一题数/label/概率domain等额外拒绝；保留Pi空成功/跳过及各parser真实检查；V-22–25/50 |
| auth／transport | Pi15s仅网络refresh，不覆盖ACK；撤回barrier30s/256keys、ADC60s/16sends/depth4/1MiB和blanketnone/sameorigin5/全0retry。source-specificretry/redirect/真实executable值、权限与秘密安全分开；V-09/12/29/30/43/55/58 |
| Schema／custom | 撤回无依据work/shape/cache/jobs/diagnostic数量和blanketdraft/format/keyword/cycle拒绝、allpatterns guard。按TypeBox corpus保留已知约束/未知format与无隐式I/O；backend差异未关闭。custom精确ApiId/upsert/兼容options复用，不添namespace/revision准入；V-11/21/34/35/39/42/49/58 |
| 来源参数／sentinel | 保留实际4096/min1/thinking/Google/UTF16/4800/3.5、retry/jitter/quota、端口/interval/expiry、cache/UUID/overflow/cost参数，修正0/省略/注释metadata与执行scope；V-05/22/30/43/45/54/55/57/58 |

## 独立复审与修订闭环

| Reviewer | 独立范围 | 最终处置 |
|---|---|---|
| parameter_acceptance_independent_review | media/classifier/Schema/custom/大event与terminal接受集；独立追踪OpenAI非流式body链与cap0 | F-01–03关闭，无剩余must-fix |
| parameter_numeric_independent_review | 所有剩余数值族、source/default/timeout/retry/sentinel/成本与模型参数 | 6项实质发现及2项措辞残留关闭，无剩余must-fix |
| parameter_bilingual_independent_review | 双语/编号/矩阵/历史权威/导航/渲染/文档检查 | 3项P2及导航残留关闭，无剩余must-fix |

原始临时报告的最终摘要见本轮[verification](pi-1.2.0-parameter-revision-verification.json)；所有实质发现保存在下表，不把TEMP当长期唯一证据。

| 发现 | 级别／证据 | 修正 |
|---|---|---|
| PR-01 漏读OpenAI后续body timer | acceptance/numeric均独立发现；普通非流式client.ts:1008–1015→1033–1088，而非只看fetchWithTimeout | 撤回本轮初审headers-only错误，保留image600000ms剩余attempt覆盖非流式body/parse。同步双语/SDK/矩阵/来源审计及旧SDK附录C；不泛化到streaming或Mistral |
| PR-02 bytes-only图像输出丢Pi encoded结果 | Pi只按data URI regex捕获，无container/base64完整解码保证 | AIR-027明确owned原始encoded media/MIME＋可选decodebytes/status；不能用强制decode拒绝Pi已返回结果 |
| PR-03 hint cap0语义倒置 | provider-retry.ts:44–50与Codex169–175；非正cap只关闭上限检查 | 两版均明确保留hint，不禁用hint/admission本身；区分classifier0立即timer、Codex0关闭timer、imagepositiveinteger |
| PR-04 action索引旧媒体/domain拒绝 | 矩阵曾重复要求image至少一图及classifier domain | 同步V与public动作索引，保留Pi空成功/fake-png/finiteprovider值，停止旧SP/D/Q反向加门槛 |
| PR-05 idle timer与age/acquire混同 | Codex release后5min timer、busy跳过，55min age于acquire检查 | 源事实与候选next-acquire-only idle机制明确分开，后者标Rust生命周期差异；未把age做active-generationdeadline |
| PR-06 Responses/Azure max0与simple W<=0 | 两wire truthy max才发送floor16；simple W<=0为max(1,M) | 明确0省略与M0→1，去通用M正数拒绝/Claude零BudgetInsufficient |
| PR-07 cost tier sentinel/tie遗漏 | matchedThreshold=-1，仅threshold>-1参与，重复等threshold取首；schema无正数minimum | 双语/SDK/矩阵/来源审计记录该边界，不用新正数准入掩盖来源语义 |
| PR-08 矩阵/审计table列丢失 | 分隔符缺空格、raw logical-or pipe、header与row列数不同 | 恢复5列V矩阵及所有采用附件table列，literalpipe转义，保留独立证据缺口与testimpact |
| PR-09 双语/历史“本轮已定型”残留 | ZH metadata有效性、conditional选择、README旧review导航、D-39未就地收窄 | 同步来源范围，旧记录标前轮/固定输入，D-39即时链接新审计，不以底部笼统声明代替 |

## 验证与仍未证明

[最终verification](pi-1.2.0-parameter-revision-verification.json)记录当前文件hash、HEAD/工作区、检查命令/退出码、AIR/V/RDF/双语数值/表格/links/源码摘要与carry-in范围。正式opi-spec、CONTEXT、runtime/Cargo/tests/CI/implementation ledger未改；本轮无提交/staging。

仍未证明：TypeBox语义对应的完整native validator；WS不可配置handshake/compression、H2/trust roots/crypto范围的实现补救；完整Cargo/features/native/MSRV/三平台；动态差分/内存性能/取消race/ACK/transport/签名/进程树；供应商接受、账户/费用、正式能力/placement准入。没有这些证据，不能称SDK已经等价或本候选已准入实施。此类未来证据缺口不需要现在猜测一个新默认值，也没有触发立即改正式规范的人类取舍。

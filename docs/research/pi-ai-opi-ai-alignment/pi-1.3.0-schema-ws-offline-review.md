# Schema／WS 离线差分：独立复审

> 本页只评审固定研究证据与文档候选，不准入实现，不表示与Pi等价。两组及其他五组均OPEN。独立复审未发现剩余必须修复项；这只核准本轮研究证据与候选判定，不核准产品实现或对齐通过。完整[独立报告](offline-probes/independent-review.md)与[冻结证据](pi-1.3.0-schema-ws-offline-evidence.json)保留实际范围。

## 结果与范围

输入Opi `8cb0fd68db528ae3d3c61874458d45e46bffa29c`，固定Pi1.3.0／Node24.13.0／Bun1.3.5+1e86cebd7及两个隔离Cargo.lock。作者执行与reviewer精选重跑分开计数，不相加成通过率。前置源码专项报告保留其旧输入，不追认本次运行。

Schema原生选择／draft切换不足，jsonschema仅留研究比较候选，重开选择或先证明可维护适配。WS原TW／TTW组合不能原样定案，完整PMD与握手方案需补救／重选；未选替代库。实际产品代码／测试／根Cargo／CI／正式spec／ledger未改，研究脚本／固定case／证据add。

## 独立复核与修订闭环

| 发现 | 源码／运行核对与处置 |
|---|---|
| runner若只记录失败而不传播可虚报运行成功 | 同意；Schema runner强制所有关键命令exit、case行数、前后pins；最终--offline --locked，隔离scratch重放语义JSONL逐字节一致。错误Node pin负向在子命令执行前exit1 |
| JS大整数parse舍入容易冒充normalization | 同意；fixture原值／pi_input_js／pi_normalized三层分别存。Native喂Pi normalized不是Rust normalizer实现，完整utility仍缺 |
| regex不仅额外拒绝，not／if／patternProperties可假成功 | 同意；实际BacktrackLimitExceeded和嵌套validate=Ok／iter_errors空均保留。unselected合法branch通过，不以全pattern预拒绝修补 |
| PMD服务向未offer的Rust强发extension，不是合法协商下的拒绝差分 | 同意；以Node／Bun真实offer和解码＋native无PMD机制／codec拒RSV1共同判断能力缺失。其拒未协商压缩本身不是bug |
| Node多header交付消息后进程崩溃，不是稳定accept | 同意；六条异常退出与两个subprotocol未捕获异常单列。125header稳定差分只用Bun exit0与native拒绝；不复制参考崩溃／非法响应宽松 |
| partial附件不足以证明完整Upgrade／tail生命周期 | 同意；仅首帧与两个控制的下一次EOF观察，完整多帧／cancel／协议验证继续OPEN；只改握手不能补PMD |
| native request使用固定sample key，不能称fresh nonce已证明 | 同意；服务按请求携带key计算Accept，native机制实验固定RFC sample key；主报告撤回fresh措辞，随机nonce生成仍未证明 |
| Bun默认import漏--no-install，产生真实缓存 | 同意；记录10文件摘要并原封移到TEMP，废弃诊断，网络／解析来源未知。正式Bun60constructor中的30项全部--no-install重跑；不宣称整轮未联网 |
| 现行backend与矩阵旧语态会掩盖最新选型判定 | 同意；§2.4／缺口表／双语正文和最新V观察已同步重开选择，旧源码轮未执行语态限定历史，regex补additionalProperties，原AIR/V身份行保留 |

## 证据限制

Schema公开utility85cases／native276观察，WS60constructor／15loopback／16reader／15partial-codec是作者实际研究执行，不是V完成数。完整Pi／opi provider、Rustnormalizer、PMD全参数／fragment／context takeover、完整Upgrade／proxy／TLS／cancel／pool、多平台和产品图未证。其余五组没有新增运行证据。无新SDK预算、fork或替代库准入，无提交／推送。

## 实际复核与检查

独立reviewer没有参与实验作者或文档整合，直接读固定源码／原始结果并运行以下检查：

- 从持久Schema assets在自己的TEMP scratch完整重播：29命令exit0，85 Pi／276 native结构记录逐字段一致；74package版本／features、73registry依赖边与全部归档checksum匹配。新构建binary摘要单列，不冒称共享target中历史binary未变。见[独立重播比较](offline-probes/independent/schema-replay-comparison.json)。
- 另行精选Schema正反例与regex组合、WS baseline／Bun125header／PMD：12个参考child与3个native loopback复现关键结果；TypeBox1385归档文件、TW／TTW60文件member逐字节匹配。
- 错误Node executable pin在子命令前exit1；WS准备脚本正常归档exit0、错误SHA exit1且未解出源码。见[pin负向](offline-probes/independent/schema-negative-pin.json)、[准备脚本正反例](offline-probes/independent/ws-prepare-tests.json)。这些预期失败证明harness的失败路径，不是provider失败。
- 最终双语正文492行数值对应，67 AIR／59 V原要求行逐字保留，RDF20身份与41＋7处置未改变；研究asset摘要／原始结果／退出码和当前文档相符，工作区自动缓存已隔离。

`python scripts/opi-doc-check.py` 与 `git diff --check` 退出0；固定Pi1925文件tree重算一致，全部持久资产／当前文档摘要、Markdown本地链接与实际Git归属由冻结证据记录。后续只作报告落盘与摘要机械刷新；reviewer的源码／实测范围与产品缺证保持上述限制，不追认其提前审过生成后的全部新字节。

测试影响：产品 **none**，研究 **add**。评审结论是研究方案可据反例重开选型，Schema／WS及其余五组仍OPEN；没有完成V、选择替代库或授予实施准入。

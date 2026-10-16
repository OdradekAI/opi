# Pi 1.3.0 Schema：设计就绪与后端重选

> 2026-10-16。本文是非规范性设计附件；**设计 READY** 指关键机制已有可执行依据、选型和维护责任已确定，不代表 opi-ai 实施、公开 consumer、完整差分或平台验收 PASS。产品代码、根 Cargo、产品测试、CI、实施账本均未改变。中文附件无既有英文 counterpart，由双语 spec 引用。历史 [85／276 差分](pi-1.3.0-schema-ws-offline-validation.md)保留原输入，不覆盖原始结果。

## 1. 确定选择及替代项

选择 **opi-ai 私有、仅 JSON 的 TypeBox 方言 IR／解释器，加 `regress 0.12.0` 的受控 Unicode 16 数据维护分叉**。不再把 `jsonschema 0.28.3` 放在工具参数验证的默认执行路径。`jsonschema` 可继续服务其他现有用途；本附件不授权删除产品依赖或代码。这里的方言是固定 Pi 实际调用 TypeBox 的语义集合，不宣称另一个标准 JSON Schema draft，也不执行 JavaScript、TypeBox 私有 symbol／回调或任意宿主代码。

| 路线 | 决定与依据 |
|---|---|
| `jsonschema 0.28.3` + 少数公开 hooks／换 draft | 淘汰为本边界的完整后端。先前实测证明 mandatory metaschema 在 hook 前拒绝、mixed items/prefixItems 不可由单 draft 表达、ref 与长度／数值不同；fancy-regex 运行错误在 not/if/patternProperties 中可产生假成功。若更换多数 applicator／ref，实际上已经拥有第二解释器，不能再称小型 leaf 适配。 |
| 未修改的 `regress 0.12.0` + 私有方言 | 淘汰其未适配版本。实际原生执行发现 Unicode 17 属性名字／新增 Letter 的接受集超过固定 Node 的 Unicode 16；不能把“ECMAScript regex”标签当精确版本证明。较早 0.11.0 的源码仍有 Unicode 17，0.10.4 源码为 15.1，二者仅做源码比较，未声称分别完成本轮运行。 |
| 私有方言 + `regress 0.12.0` 算法 + 官方 Unicode 16 生成数据 | **选定**。官方生成器真实重建、重复一致；更换一个运行时生成表，算法不改。关键旧反例、Unicode 正反控制和显式 fallible 分支机制已运行。把版本／数据／维护责任作为后端身份，不能仍标为“未修改 crates.io 0.12.0”。 |

这是为未来获准入的实施确定内部机制；没有新公开 validator trait、插件框架、Opi 领域 crate、JS runtime、schema I/O、配额或 Unsupported 降级。完整验收未通过时，对应 V 继续 OPEN，不能从本文的 READY 推导产品准入。

## 2. 固定输入和执行结果

参考仍是实际 `D:/nvm/v24.13.0/node.exe`、Node 24.13.0／V8 13.6／ICU 77.1／Unicode 16、TypeBox 1.3.27、原始 Pi `validation.ts`。Node SHA `d14ba95cdce1ef7dc9ad3ac74949ca5db38b27378ee30f30a23cf26f9e875a11`；Pi utility SHA `e4b8029c184bd6df830e0ee2cb7034a39d49b5ecb8c068c26af9564c20246594`；TypeBox 1385 文件 tree SHA `ebf39b1441f59a084233243d5e49ec9d4fdcaa4521417a883dfe51a2118e271d`。输入校验、fixture、native source、Cargo.lock、实际 stdout/stderr、退出码及比较见[可重复资产](offline-probes/readiness-schema/README.md)。

Native 为 Windows MSVC、rustc／Cargo 1.99.1。隔离主目标图 7 packages；regress 有效 features 精确为 `std, utf16, prohibit-unsafe`，关闭 defaults／PikeVM／nightly pattern。没有 C、proc macro 或 `build.rs` 加入该主目标路径。生成工具另有固定独立图；这些不冒充产品 feature union、MSRV 1.97 或三平台验证。

| 实际执行 | 结果与可证明范围 |
|---|---|
| 原始后端与 Unicode 16 数据适配后端，各 64 cases | 原始后端 **62／64** 与固定 TypeBox 同意；Unicode 16 版本 **64／64** 同意。64 是 39 个旧语料子集加 25 个新增语料，不是覆盖率或完整等价概率。 |
| 原来的 backtrack 反例 10 种组合位置 | bare、not、if、anyOf、contains、patternProperties、additionalProperties、propertyNames、allOf 与未选 anyOf 均与 Pi 的布尔结果同意；真实 regress 返回匹配，未发生 fancy-regex 的 BacktrackLimitExceeded。 |
| metaschema／mixed／数值／refs／长度 | malformed minLength/minimum/pattern 被对应结构 guard 忽略；items[] 与 prefixItems 同时生效，另测 prefix 子项 local ref 正反例；metadata absolute-ID 与 ref sibling 正反例；0.3 multipleOf0.1；combining/ZWJ/flag及 Hangul／emoji modifier 源特有非 UAX29 行为。只证明列出的结构子集。 |
| Unicode／语法 | U+1680／U+200B、Letter、Kawi、Nag_Mundari、Todhri、Gurung_Khema、Ol_Onal、astral dot、backref/lookbehind/named backref，及非法 escape／range。Unicode 17 的 Beria_Erfe 名称和新增 Letter 在原版偏离，重建表后符合 Node 16。 |
| 11 个显式注入的 fallible leaf 控制 | 9 个实际到达的错误保持 error；未选 anyOf／if 的 2 个不执行该 leaf，保持成功。它们是**合成 Result 传播测试**；regress `find` 自身返回 Option，这不是声称原生引擎发生过运行错误。 |
| 官方 Unicode 生成器重跑两次 | 生成表 SHA `475cb03589f0deb859b9a2ccfb34e3f33026a1e89500524024ce575a135e5daf`；配套测试 SHA `ae24799bc5c8ef417b72de6cbc2808fb581c28e6412284fa00c8f034cd21497c`，两次逐字节一致。 |
| Number::toString／URL.canParse 单独叶探针 | `ryu-js 1.0.2` 在14个binary64 String边界全部同意。url2.5.8＋ICU2.2图在Unicode17字面／percent-encoded domain上有2个假成功；固定idna_adapter1.2.1＋ICU4X2.0.0及配套data／writeable0.6.3 alloc后29个URL输入全部同意，合计43／43。[叶输入和图](offline-probes/readiness-schema/leaf-inputs.json)不等于完整normalizer或所有WHATWG边界已验证。 |
| 私有依赖身份与独立consumer冲突图 | 实际把url/idna/adapter与7个ICU包共10项package identity及内部依赖引用命名空间化，runtime算法源码不改；与caller普通url2.5.8／idna_adapter1.2.2／ICU2.2同图编译exit0，43／43叶仍同意。证明不用消费方lock／全局patch也能隔离profile，未证明opi-ai最终发布包已组装。 |

本轮比较的是 TypeBox 直接 Check 对 **JS 已 parse 的输入**和 native 同一输入的判断；Pi 原始公开 utility 也运行并保留结果，但**没有实现 Rust normalizer**，没有用 Pi normalized 输出伪装 native 完成。`normalize-*` IDs 在本轮仅作为 validator 原输入案例；不能拿它们宣称 normalize 完成。保留 RDF-09：Rust 自有精确整数不主动复制 JS parse 丢精度；根 primitive 转换后必须用原 schema 最终验证，不复制 Pi 返回未验证 raw 的漏洞。**明确的Rust语言差异：**JSON String采用有效UTF-8，不能表示孤立UTF-16 surrogate；保留收到的raw text／字节与parse状态，边界返回typed解析失败，不静默替换为U+FFFD后标成相同值。undefined／symbol／非JSON数值也不映射成有效JSON；现有owned raw与脱敏错误合同适用。这些差异不暗称完全等价，真实接线／消费者负例仍OPEN。

依赖／数据准备使用官方网络：regress 0.12.0、0.11.0、0.10.4，codegen 0.3.0、ucd-parse 0.1.13、regex-lite 0.1.8、ryu-js1.0.2、idna_adapter1.2.1及7个ICU4X2.0.0 crates的元数据和归档；固定 commit 的生成器源码；Unicode 官方 UCD16 与 emoji16。详见来源摘要，不能声称整轮没有联网。运行阶段 Cargo `--locked --offline`，Node loader 仅映射固定 TypeBox，无安装／服务／凭据。首次 UCD 获取的 transport failure、生成器缺 emoji 文件的 exit101、旧ICU编译时writeable缺alloc的exit101是准备／图问题；补齐明确features后才记录结果，不计为validator行为。

## 3. 私有合同和实现分解

### 3.1 结构 admission 与 IR

保留 trusted JSON；建立私有方言 IR，按 TypeBox `schema/types/*.mjs` 的结构 guard 识别关键词。缺失或类型不符合 guard 的字段按该源语义为 inert，不能先套标准 metaschema；合法 schema 约束不能被删除／泛化。`$schema` 不选择 draft；contentEncoding／contentMediaType 只作该路径实际使用的 annotation。格式／strict wire 投影是独立处理，不复用成原 schema 的证明。

`schema/engine/schema.mjs` 导入 **43 个机制**，其中 `_refine` 对应 JS 函数机制，不是普通 JSON 输入；其余 42 含 boolean schema。这个数量是源码分解，**不是 42 项均已实测覆盖**。完整源范围为 engine 2694、types 771、resolve 328、format 892 行 `.mjs`，另有 Pi validation 353 行及 guard helpers；这是维护边界，不是承诺 Rust LOC 或预算。[源清单](offline-probes/readiness-schema/typebox-source-inventory.json)逐文件固定摘要。

实施分组必须穷尽：boolean/type；const/enum；required/additionalProperties/dependencies/dependentRequired/dependentSchemas/patternProperties/properties/propertyNames/minProperties/maxProperties/unevaluatedProperties；additionalItems/contains/items/maxContains/maxItems/minContains/minItems/prefixItems/uniqueItems/unevaluatedItems；minLength/maxLength/format/pattern；minimum/maximum/exclusiveMinimum/exclusiveMaximum/multipleOf；ref/recursiveRef/dynamicRef；if/not/allOf/anyOf/oneOf。不能把未写的 node 默认为 true 或 Unsupported。

IR admission 编译所有实际结构选中的 regex leaves，保留源码的 compile／check 阶段区别，不能把非法 regex 延迟到可能未访问的分支然后接受。不得遍历每个 annotation string 当成 regex。普通 object 枚举使用保存顺序的 JSON 映射，并显式匹配 JS integer-index key 枚举；不得先按 Rust BTreeMap 顺序重排 ref 发现／冲突解析。选定 `serde_json/preserve_order`；它的 Cargo feature 扩散和其他 crate 序列化影响要在实际实施检查，本文没有改 manifest。

### 3.2 Applicator、错误与 evaluated 集

IR evaluator 返回独立的 `Result<Valid/Invalid + evaluated facts, SchemaError>`；compile failure、invalid instance、取消／基础设施错误是不同结果。实现直接表达 visited branch 的 Result，不经过仅 bool 的第三方 applicator，错误不会在 not/if/patternProperties 中变成普通不匹配。短路顺序以固定加速路径为准：无 unevaluated 时采用源的 fast allOf/anyOf 等顺序；有 unevaluated 时采用 `_reducer.mjs`／`_context.mjs` 的逐 arm 独立状态与成功状态 merge，不能把合成控制的 fast anyOf 无条件外推。invalid 分支的 evaluated keys/indices 不泄漏，嵌套容器有自己的 frame。错误报告保留结构路径／keyword／原因，默认脱敏；无需复制 Pi 含 raw arguments 的错误文本或 JS 栈溢出。

用显式 work stack 管理遍历和 ref 返回，避免 Rust 调用栈被 schema 深度耗尽；不引入固定 schema 深度／节点／branch 拒绝。真正重复的 `(schema node, instance location, dynamic/ref frame)` 无进展递归作为 typed 非成功终止；不能拒绝全部递归 schema。productive recursive child 与 unchanged-instance self recursion 分别测试；后者 Pi 的 RangeError 不是要求 Rust 崩溃。仍须验证组合 visited/nonvisited 错误、evaluated 集、dynamic scope、诊断阶段及资源释放。

### 3.3 引用解析

按 TypeBox `_stack.mjs` 与 `resolve/resolve.mjs` 分开保存 lexical base、resource base、reference base、dynamic／recursive anchor 与 resolved-resource 返回 frame。相对 `$id`、fragment/JSON pointer 解码、metadata 内资源遍历、重复 ID 的源枚举顺序、legacy 无 `$schema` frame 都有对应算法位置；不替换为“每对象全局 id 表”。内存 resource 与 URI identity 保留；retrieval handler 永不做 HTTP/file I/O，未解析引用保持源的非成功阶段。原始 TypeBox 默认路径也不会凭 URI 名称凭空取得外部资源。

研究 prototype 仅实现 local pointer／absolute metadata-ID 的有限子集；完整 resolver、dynamic/recursive 和 encoded/duplicate targets 是明确的实施交付项，不伪装本轮全已验证。该部分是有限的私有源定义算法，选择已确定，不依赖尚未找到的第三方扩展点。

### 3.4 Leaves、formats 与 normalization

长度逐项移植 TypeBox guard/string 的 fast path 与有限组合范围／ZWJ／regional pair 规则，不用 unicode scalar count、ICU 或一般 UAX29 替代。研究代码的简化 cluster helper只覆盖本轮有效非负整数边界，不能直接作为所有 fractional/negative guards 的最终实现。multipleOf 采用固定源的 binary64／`1e-10` tolerance 及整数 shortcut；**这个数是 TypeBox 算法常量，不是 Rust 新工作预算**，不扩展到其他数值比较。

21 默认 formats 逐一按固定 `format/*.mjs` 的算法移植，unknown format 同源为 inert。`regex` 复用同一 Unicode16 ECMA parser；`url` 对应源的 WHATWG `URL.canParse`，采用 **url2.5.8／idna1.1.0／idna_adapter1.2.1／ICU4X2.0.0** 的同ICU77数据profile；包含normalizer/properties及data、collections/locale_core/provider，writeable0.6.3显式alloc。不沿用产品当前ICU2.2图。叶实测已有literal/percent-encoded Unicode17域名负例、16域名正例与Unicode17路径正例，避免泛拒Unicode17整个URL。其他 URI/IRI/pointer/date/time/hostname/ip/email/uuid/duration 保留各自源算法，不能用同名标准库替换后声称同接受集。固定默认 registry 与外部直接修改 TypeBox 全局 registry 不混称；Pi root 未导出 Format，全局依赖对象的宿主污染不自动变成 Opi 全局可变 registry API。

**固定独立consumer的依赖身份与交付物：**idna1.1.0只提供固定Adapter构造，未提供可注入DataProvider的入口；不能声称用它完成公开provider注入，也不能用workspace锁或library里的`[patch]`固定外部consumer。选择 **registry可解析的受控vendor family发布包**，作为opi-ai内部使用的第三方后端依赖：url/idna/adapter及上述7项ICU包的package身份和内部依赖引用统一命名空间化，算法文件逐archive member不变，带独立source／manifest变换／数据与许可清单；原图和新图的公开类型不交叉。每包使用精确版本和可审查registry source/checksum；发布opi-ai前，全部依赖必须已在目标registry可解析，普通consumer不需要root patch、TEMP path或单独下载源码。研究名称`pi-schema-private-*`仅为证明用package身份；最终registry名称与版本在实施／发布配置中固定，不新增Opi领域API／公共后端trait。

这个选择明确承担 **10个维护后端包**的分发、版本、许可／NOTICE、漏洞修复与升级回归责任；它不是零成本叶适配，不以未验证的source-module内联替代发布方案。本轮不发布任何包。实测同图冲突已经证明身份机制可编译，最终registry产物／crate packaging／workspace外consumer仍由V-58实际证明，保持OPEN；这属于已确定交付路线的实施／release门禁，而不是“以后再决定怎样发包”。

normalizer 是单独私有阶段，按原 Pi validation 与实际 Value.Convert 路径 clone-before-normalize，先保留 raw 和 parse provenance，再 optional-null、已有 JSON type、allOf 顺序、anyOf/oneOf 原值优先与 clone-per-arm、有限 ECMAScript Number/String coercion、properties/items 递归，最后原 schema full check。它与 evaluator 共用只读 IR 的子 schema check，不保留两个验证接受集。Number词法采用私有ECMAScript StringNumericLiteral分类，ECMA whitespace及decimal/hex/bin/oct／符号/exponent分别处理，再用成熟binary64转换，不把Rust parse接受集当成JS Number词法；拒绝非有限coercion结果。String固定采用 **ryu-js1.0.2** 的safe Buffer API处理binary64 shortest/exponent阈值/负零，不另造float算法。该crate为纯Rust，无C/build.rs，但内部包含unsafe；调用隔离在leaf，需按原版本source／锁图审查，不能宣称整个依赖树无unsafe。RDF-09 correctness fix、整数精度与字节不互换说明继续有效。

### 3.5 所有权、工作与维护

IR／regex 编译结果由工具声明 generation 的 owned Arc 持有，原 trusted schema 不改变；替换声明的新调用获得新 IR，已创建调用保留旧 generation。内部 cache 只作复用和可淘汰存储，不以 cache miss／tuning number 拒绝有效 schema。耗 CPU 阶段置于阻塞执行单元，任务并发由运行时调度；取消可以阻止后续阶段／丢弃结果，不能把 sync regex 说成可被 Future drop 硬杀。无额外固定 schema、regex fuel、queue、work 或错误数上限。

Unicode 数据维护分叉放在受控私有依赖来源，不把改动藏进 upstream version。记录上游 archive、commit、许可、数据来源与生成器 SHA；生成期工具不进入产品 build.rs／运行依赖。回补上游修复或升级时，先重新固定 Pi 运行时 Unicode 版本，再重生表／配套 tests／运行语料，审查算法 diff，禁止 Cargo semver 自动静默替换这个 backend identity。若未来 Pi runtime 数据版本变化，升级这个明确输入即可，不再发明另一个 regex engine。

## 4. 实施门禁和仍 OPEN 的验收

设计不再等待“某个通用 engine 会支持 TypeBox”的假设；采用方言所有者明确、机制有限、数据可重生的路线。代价是维护整个 JSON 方言 evaluator／resolver／formats 和 Pi normalizer，而不是只维护几个 hooks；源规模、差分回归和依赖维护责任均显式记录，不能再称小 leaf adapter。不开产品新公共框架，也不扩大 SDK 权限。

对应 V-11/34/35/39/41/58 保持实施 OPEN：完整 21-format corpus／registry边界、所有42 JSON机制、full resolver／evaluated scope、真实 Rust normalization、source malformed guards、完整 syntax／Unicode／UTF16 差分、公开 consumer、取消／cache／generation、MSRV／三平台和供应链审查。把原 85cases 与本轮新增输入转成回归起点，新增边界／生成语料；不会拿 64／64 作为这些 V 的通过记录。

`固定输入 → verify: archive/Node/TypeBox/source/lock pins`；`官方生成 → verify: 两次 table/tests 相同`；`native differential → verify: 全部 case/退出码/原始流`；`产品实施 → verify: 上述 V + 独立复审`。研究测试影响 **add**；产品测试影响 **none**。未提交或推送。

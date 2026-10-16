# Pi 1.3.0 Schema 最小离线差分

本页仅报告隔离研究 harness 的实际结果。Schema 覆盖仍为 **OPEN**；没有修改 Opi 产品代码、测试、Cargo manifest/lock、CI 或正式规范。

## 固定输入与边界

- 参考为本地 Pi 1.3.0 原始 `packages/ai/src/utils/validation.ts`，Node **24.13.0** exact executable（SHA256 `d14ba95cdce1ef7dc9ad3ac74949ca5db38b27378ee30f30a23cf26f9e875a11`），TypeBox **1.3.27**。只用 ESM loader 将 bare TypeBox import 指向缓存 package；没有复制、更改 validation 算法，也没有安装 Pi 全量依赖。没有执行公共 root barrel，工具函数本身是 root 导出的公开 utility。
- Node V8 `13.6.233.17-node.37`、Undici `7.18.2`、ICU `77.1`、Unicode `16.0`、OpenSSL `3.5.4`。这是一个确定的 Node 参考，不证明其他 Node/Bun 版本行为或发布包身份。
- Rust `1.99.1 (58cf8f9c3 2026-10-13)`，Cargo `1.99.1`，Windows MSVC。jsonschema **0.28.3**、referencing **0.28.3**、fancy-regex **0.14.0**、serde_json **1.0.149**；精确 lock 从仓库 lock 离线 prune，所有所用 registry 版本仍在该输入 lock 中。主目标解析图 **74 packages，73 registry packages**；归档 SHA256 均匹配 lock checksum。详见 `results/cargo-graph.json`、`source-pins.json`。
- TypeBox 归档 **1385 个文件**逐字节对照缓存提取结果。Pi utility、TypeBox 完整 package tree、三个决定性 Rust crate source tree、运行时、lock、fixtures 均保存摘要。运行前后再次校验 Node/Pi/TypeBox/lock/fixtures，Cargo `--offline --locked`，未联网、未安装、未 hydrate、未调用 provider。
- Native profiles 是明确配置的 **Draft7/assert-known-formats/ignore-unknown** 与 **Draft202012/assert-known-formats/ignore-unknown**，不是声称“后端全部默认行为”。全部使用拒绝式 retriever。第三 profile 在 Draft7 关闭 base64 与 application/json content 检查；两个额外 hook profiles 仅检验最小公开机制。
- **85 cases：74 普通，11 独立 containment 进程**。Pi 85 个结构记录，native **276** 个记录（含 profiles 和必要输入阶段）。所有实验命令 exit 0；运行器检查关键 exit 与每个 case 行数。5 秒 contained-process timeout 仅是实验隔离，全部 case 已在期限内实际退出，不是 SDK 新预算。

## 实际结果

| 细项／case IDs | Pi public utility | Native 与判定 |
|---|---|---|
| `normal-valid/invalid` | 正例返回、反例抛 validation error | 两个 draft 与 content-off profile 均正确，基线连通 |
| `mixed-pass/fail` | `[1,2]` 通过、`[1,3]` 拒绝 | Draft7 两个都通过；2020 两个都 metaschema build error。公开 prefixItems hook 修复这两个无 ref 子例，不能外推 shared refs/evaluated sets |
| `malformed-minlength` | `{minLength:"2"}` 对 `""` 返回 | 所有 native profiles build error；即使注册 minLength hook，factory 调用 **0**，证明该 hook 无法越过前置 metaschema。尚未证明受控 lowering 或其他 engine 方案 |
| 四个 `grapheme-*` | combining、ZWJ、flag maxLength1 通过，combining minLength2 拒绝 | native Unicode scalar 长度得到相反结论 |
| 21 formats 各一正一反、unknown format | unknown 通过；42 个名义正反样例按 TypeBox 执行 | Draft7 错过 duration/uuid 反例；两个 draft 都错过 `url`、`json-pointer-uri-fragment` 反例。部分同名格式无差异不证明算法全等；未测所有边界/registry mutation |
| `content-base64/json` | inert，两个返回 | Draft7 额外拒绝；公开 `without_content_*` knobs 可修这两个。2020 此两条已通过 |
| local refs / siblings / metadata | local 正反例正常；ref sibling minimum 生效；metadata `$id` target 可解析；未解析 ref 抛 validation error | Draft7 忽略 sibling minimum，2020 生效；metadata target 所有 profiles build error。未解析 ref native build error，阶段也不同。static retriever实际被调用但拒绝，零 I/O |
| `regex-u1680` | `^\s$` 对 U+1680 返回 | 两个 draft 都拒绝 |
| `multipleof-decimal` | `multipleOf:0.1` 对 0.3 返回 | 所有 native profiles 拒绝。数值 tolerance/JS number 语义仍需适配证明 |
| `normalize-*` | hex `"0x10"`→16、required null→0、optional null删除、number→`"1e+21"`；已匹配 union string 保留 | native raw 拒绝需要转换的输入，喂 Pi 实际 normalized 后通过；这只证明 normalizer 的必要性，**没有实现 Rust normalizer**。raw 在全部 Pi case 未被 utility 改写 |
| `normalize-large-integer` | fixture `9007199254740993` 在 JSON.parse 已成 `9007199254740992`；utility返回同值 | 原 fixture、`pi_input_js`、`pi_normalized` 分开记录，不能把 JSON parse 舍入误记为 utility normalization；未证明 Rust 数值语义对齐 |
| `arguments-error/empty` | truthy error 前置拒绝，空字符串不过滤 | 直接 native 只校 schema，不能代替 public utility 语义 |
| `root-correctness` | integer/minimum10 + args`"2"` **返回不通过 TypeBox Check 的原始字符串** | native raw 拒绝。证实已有 RDF09 correctness 差异依据，不要求 Rust 复制该 bug |
| `self-ref` | Compile 成功，Check 与 public utility 抛 `RangeError: Maximum call stack size exceeded` | native 三 profiles 全 true；固定 Node 上已执行，不再只写“是否会返回未知”。其他 recursive/dynamic/mutual refs 未证明 |

## 实际 regex 运行错误反例

先在独立 native 进程探索 subject，再冻结 **pattern `(?:(a|b|ab)*(?=c)|.*)`，subject `ab` 重复 20 次（40 ASCII chars）**。本轮不是猜测阈值，也不把 40 当公开上限。每个最终 case 均独立进程，Pi 实际完成、无需 timeout；native 确实记录 `BacktrackLimitExceeded`。

| `regex-backtrack-*` | Pi public | Native 所有三个 profiles |
|---|---|---|
| bare / anyof / contains / propertynames / allof | 返回 | `is_valid=false`，validate/iter_errors 按具体 applicator 返回 regex 或组合错误 |
| not / if | 拒绝 | `is_valid=true`、validate valid、iter_errors 空，runtime error 被 bool 分支转成合法结论 |
| patternproperties（匹配 key 的 value 为非 integer） | 拒绝 | `is_valid=true`、validate valid、iter_errors 空，错误跳过应校验的属性 |
| additionalproperties | 返回 | 拒绝，本应被 pattern 覆盖的 key 变成未覆盖 |
| unselected-anyof（合法 const 首分支） | 返回 | 通过；不能全局预运行/拒绝所有含昂贵 pattern 的 schema 修补 |

因此仅改 validate/iter_errors 入口、将 error 映射为 typed error 或全局预检 pattern 都未获得对齐证明。公开 leaf Keyword 仍有 bool `is_valid` 通道；完整补救需 branch-correct 机制，不得在 adapter 内悄悄重写 applicator/ref evaluator。

## 后端判定与未关闭项

**排除“jsonschema0.28.3 原生配置／切换 draft 即可作为已证明后端”方案；Schema 选型重新进入比较／机制验证。** 当前库可保留为研究比较基线与候选，不再把其整体适配可行视为已定事实。content knobs 与 prefix hook 只证明局部可表达性。没有证明这些 public hooks 能跨越 metaschema、metadata refs 与 fallible regex 组合；没有选择新库、fork、JS runtime 或自研 evaluator。

下一步应以同一固定 corpus 比较维护中的候选或受控上游修改；至少证明 schema admission、混合 keywords、refs、grapheme/format/numeric leaf 与 branch-correct regex 全部可维护。新增 engine、版本、特性／native／proc-macro 闭包必须单独核查；不能以 Unsupported、更多固定预算或更严 schema 接受集关闭。

仍未证明：真实 opi-ai public consumer、TypeBox factory `~kind`/codecs、全面 draft mixing 与 `$schema` legacy frames、动态/递归/remote-in-memory refs、encoded pointers、format 全边界与 registry mutation、JS 数字／String 全域、取消/worker/cache 生命周期、其他平台/Node/Bun、实时流业务组合。本轮 V-11/34/35/39/41/58 与 RDF09 仍 **OPEN**。

## 离线重放

先设置 `$ProbeNode` 为具有记录 SHA 的现有 Node binary，`$ProbePi` 为既有 Pi snapshot，`$ProbeTypeBox` 为对应缓存 package，`$ProbeTarget` 为 Opi 既有外部共享 Cargo target，`$ProbeScratch` 为隔离 TEMP source 目录。目录中的 Cargo manifest 是模板；运行器只向 scratch 复制，不写产品源码或在研究目录编译。

```powershell
python run.py --node $ProbeNode --pi $ProbePi --typebox $ProbeTypeBox --target $ProbeTarget --scratch $ProbeScratch --output rerun
```

运行器校验固定 runtime/source/lock/fixture hash、rustc/Cargo 版本输出、关键 exit 与 case 计数。依赖不在缓存时 `--offline --locked` 应失败并报告，不能解释为 case 不通过；不要联网或隐式安装。保留 incremental，共享 target，不建 per-session target、不清理 Cargo cache。

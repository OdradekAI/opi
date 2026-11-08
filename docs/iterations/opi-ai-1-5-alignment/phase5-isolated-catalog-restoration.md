# 第五阶段：在隔离副本中恢复 pinned catalog

日期：2026-11-08。工作树基点：`69670eadf84352e3583f23799e4c2c288b322d5f`。
对应[实施票](issues/38-isolated-pinned-catalog-restoration.md)。这是 source 尾部修复
之后的独立材料恢复切片，不改变生产 Rust 行为或上游冻结基准。

## 当前结果

**隔离候选的 catalog 已恢复，完整材料来源仍未建立。**

| 观察 | 结果 |
| --- | --- |
| 原始响应 SHA-256 | 与 `nix/model-catalog.json` 中的 pin 相同 |
| 恢复后的 provider / model | 42 / 1712，与 pinned 响应一致 |
| 缺失 / 多出 / 值不同的模型 | 均为 0 |
| source、dist 数据内部校验及逐文件字节一致性 | pass |
| 实际导入编译后 `models.generated.js` | chat/image/classifier 导出全部与 pinned 值一致 |
| AI / schema / telemetry 的 source-output 检查 | 204 / 43 / 8 个源码对应输出完整，source map 内容一致 |
| 原快照保护 | 冻结源码、生成数据及三个 dist 清单前后相同；直接依赖及材料复核前后相同 |
| 完整 root lock 来源 | 仍受阻，不用相同直接依赖版本替代证明 |

新的完整材料证据必须绑定此候选才能使用上述结论；原 `.pi/pi-1.5.0` 中用户提供的
1738 项目录没有被覆盖，其原有两个阻断也没有被追溯改写。没有把本报告冒充现有
schema-v2 生命周期 runner 所要求的材料清单，也未重新执行 TS/Rust 差分。

## 恢复路径和所有权

复用冻结快照自带的 `packages/ai/scripts/hydrate-model-catalog.ts`，而不是执行
会请求当前线上数据的 `catalog/generate.ts`。它只读取原始响应，按 `type:id` 分组，
使用固定 epoch 时间戳生成 manifest，并在替换候选数据前进行内部验证。

新增 [restore_pinned_catalog.py](tools/restore_pinned_catalog.py)：

1. 先拒绝已存在的候选/证据路径、与输入树重叠的路径和错误 pin；不覆盖历史记录。
   manifest 路径必须为规范 POSIX 相对路径，拒绝 Windows 反斜杠别名回绕。
2. 从冻结 manifest 列举的 2949 个文件复制当前源码，逐个验证复制 bytes。
   除已记录的 root lock 漂移外，其他源码漂移直接失败；不复制任意本地文件。
3. 建立 277 个明确依赖目录链接，复用已安装 SDK，不执行安装或 lifecycle hooks。
   保留 AI 嵌套依赖优先级；schema/telemetry workspace 链接指向候选本身，确保 AI
   编译使用新编译的本地依赖，而不是悄悄回到原目录的 dist。
4. 校验 TypeScript 安装版本与冻结声明一致，执行离线 hydrate、内部校验，并依次
   编译 telemetry、schema、AI；复制所有数据资产，包括隐藏的 `.manifest.json`。
5. 复用现有只读 inspector，另用
   [verify_catalog_runtime.mjs](tools/verify_catalog_runtime.mjs)实际加载编译后的
   catalog 模块，比对 provider/model 身份及完整模型值，不只比较总数或源码文本。
6. 保存命令、退出码、stdout/stderr、输入身份、链接目标和候选 dist/data 文件哈希；
   再检查原快照未变。失败时保留 partial candidate 和失败记录，不自动重试刷绿。

执行子进程只继承 PATH、系统目录和临时目录变量，并设置 UTC；没有继承 provider
凭据、用户配置或 proxy 变量。调用的 hydrator、校验器和编译器是离线路径；这不是
操作系统级网络沙箱。Windows 使用 CPython `_winapi.CreateJunction`，避免 `cmd`
解释 `&`、`%...%` 等路径字符；POSIX 使用目录 symlink。此次仅在 Windows 原生执行。
`_winapi` 是 CPython 的内部模块；未验证其他 Windows Python 实现。

## 证据与复现

- [首次候选记录](evidence/phase5-restored-catalog-candidate.json)：实际恢复成功，
  随后发现目录链接可以去掉 shell，未删除或覆盖该记录。
- [原生链接候选记录](evidence/phase5-restored-catalog-native-candidate.json)：改用无 shell
  的目录链接后实际重建并验证，exit 0；保留为中间证据。
- [路径受检候选记录](evidence/phase5-restored-catalog-validated-candidate.json)：补齐路径
  别名拒绝规则后的中间候选，exit 0。
- [审阅后最终候选记录](evidence/phase5-restored-catalog-reviewed-candidate.json)：加入失效
  junction 的提前拒绝和明确的原树检查范围后，用当前工具实际重新生成，exit 0。
  状态为 `catalog-restored-full-provenance-not-established`，不是完整材料验收通过。
- [检查记录](evidence/phase5-catalog-restoration-checks.json)：测试命令、日志身份、
  原始工具版本保存位置及独立审阅处置。

当前工作副本：`C:/Temp/opi-ai-pinned-catalog-reviewed-20261108`。它依赖当前已安装目录，
不是可独立搬走的 npm 发布包。**不要在此副本运行 npm install 或修改 node_modules：
这些链接并非操作系统级只读，写入会影响共享安装目录。** 隔离的是复制的源文件与
构建输出；工具只通过已检查的离线路径读取共享依赖。
复现必须换用不存在的候选目录与证据文件：

```text
python -B docs/iterations/opi-ai-1-5-alignment/tools/restore_pinned_catalog.py \
  --repo D:/Luiz/Odradek/opi \
  --candidate C:/Temp/opi-ai-pinned-catalog-new \
  --output docs/iterations/opi-ai-1-5-alignment/evidence/catalog-new.json
```

以上为参数示意，shell 换行符按宿主选择；不能重复使用已存在的输出路径。
原始响应为 `evidence/phase5-model-catalog.json`，SHA-256：
`f4dbabe79cda4b38aee4a761b23fd5eaf526ac2e28fed3c9ee86e37ca0e5d71e`。
TypeScript 为 7.0.2，Node 为 24.13.0；完整命令及平台字段在候选记录中。

## 检查与边界

- Python restoration tests：9 项通过，包含 CLI 拒绝覆盖、错误 pin 提前退出、源路径
  越界、非 lock 漂移、失败编译留下候选/证据、构建期输入漂移、嵌套依赖链接、
  含 shell 字符的真实目录链接及失去目标的 junction。
- Node runtime tests：2 项通过，验证真实 emitted JS 加载；缺失/额外身份、价格变化、
  未表示的模型类型以及无参数 CLI 均不能伪装为 pass。
- 完整离线重建和运行时验证已在四个新副本执行。后三次分别验证修改后的链接机制、
  路径准入规则和审阅修补；四个副本的 dist/data 输出字节相同。不是对失败自动重试，
  也不等于独立 clean install 的可复现构建证明。
- Windows 别名负例先实测失败：`..\\<source-dir>\\entry.ts` 可解析回源内，但原复制
  逻辑会保留含 `..` 的相对路径。已拒绝所有非规范反斜杠路径，原负例在实际 Windows
  测试中转绿；原有有效输入及证据不修改。
- 现有 freezer/inspector 没有修改；已有单测继续适用，并在实际候选执行中复用其验证。
  未改变 Rust 产品/依赖图，不重跑已有 209 项 Rust 回归或 Cargo 构建。
- 独立审阅及最终文档、JSON/哈希、diff 检查见检查记录。

## 独立审阅处置

只读 reviewer 确认了源复制、依赖选择、离线调用及运行时比对的边界；没有执行测试。
两条具体意见均已处理：

1. 失去目标的 Windows junction 不能由 `exists() || is_symlink()` 发现。先增加真实
   junction 红例，候选/证据两个位置都复现未提前拒绝；改用 `os.path.lexists()` 后，
   两个 CLI 负例与完整 9 项测试通过，不再等到构建结束才发现输出路径不可创建。
2. `supplied_unchanged` 不是全部 node_modules 的完整审计。新增明确的
   `supplied_unchanged_scope`，列出冻结源文件、生成树和成功后的直接依赖入口复核，
   同时列出未遍历全部共享依赖内容、没有 OS 级写保护的限制；没有补写不存在的保证。

审阅还指出 `_winapi` 的 CPython 依赖，已在上方说明。原始报告路径/hash 及红绿记录
在检查证据中；修补由父执行者验证，未声称进行了第二轮独立审阅。

## 剩余事项

完整锁文件预期仍为 185524 bytes、SHA-256
`802adbf0a5e093830226723f37b8971ce42eee442181a1471e0b70b1396b244a`；当前副本沿用
185020 bytes、SHA-256
`79490cd0acd238e0ad915d96c77bfddb5a07bf73fefcadc46cc13ec25785c721`。
没有猜测重建或批准替代基准。后续需恢复原 lock 来源，并把候选路径/身份显式接入
材料冻结与差分重放；npm 独立消费、真正的 core＋Faux 最小构建及六门验收仍开放。

测试影响：`add` 两组工具测试；`update` none；`retain` 历史工具、产品回归及全部旧
JSON 证据；`delete` none。内部工具与阶段报告没有既有双语镜像；公开英中产品手册
未改变，因为产品接口和行为没有变化。未提交、推送、安装依赖、访问 provider 或发布。

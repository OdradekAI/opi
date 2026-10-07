# Pi 1.0.0 历史证据与恢复说明

> 原整理日期：2026-10-05；迁入日期：2026-10-07。当前分析基准为 [Pi 1.0.4 整体报告](../../README.md)。本目录只保存版本比较与继承审读所需的历史证据，不是另一个当前报告入口。

## 保留材料

- [源码清单](source-manifest.json)：原 1.0.0 清单逐字节迁入，保留 1,848 个文件的路径、大小、行数和 SHA-256。文件 SHA-256 为 `b4c8d2260a3bd86dfd35d31a562ab5a981a713ab9f395712b640a644aac7185b`；源码树摘要为 `cc37a8fcc2c2dd97a3953c01a1e85f951984a85a0f5252111627163ce4c71173`。清单不含源码正文，不能据此重建旧源码。
- [文档审读记录](documentation-reading.json)：从旧索引提取全部 125 条记录（33,468 行），保留文档类型、审读深度、旧行号范围和原索引行号。原索引的 Git blob 与工作区字节摘要一并记录。1.0.2 审读由 [中间版本证据](../pi-1.0.2/documentation-reading.json) 承接；当前 1.0.4 口径见 [文档索引](../../documentation-index.md)。

旧 `docs/research/pi-1.0.0/` 报告目录已退出工作区。新版报告的正文、规划时效结论和包分析可独立阅读；版本差异及继承审读的证据由以上附件提供。过去执行日志中的旧路径仍表示当时对象，不应替换成 1.0.2 的执行结果。

## 旧报告全文的固定 Git 位置

旧目录的 18 个文件均保存在当前仓库的提交 `d472a50393422121c9343332dea125c7a98ada19` 中。下表记录其 Git blob；Git blob 标识与源码清单里的文件 SHA-256、源码树摘要是三种不同身份。旧 README 在本次升级过程中新增的两行新版导航不属于该提交；正文已在提交中，本次移除无需保留这份临时导航。

| 原目录内路径 | Git blob |
|---|---|
| `README.md` | `10585a48badb7c1e59da1e289ebbaef5efde2559` |
| `documentation-audit.md` | `bc0928eb410da475b402be464f299ea1666b3cb5` |
| `documentation-index.md` | `6b7d9f8cbec07092544345e5b73eb4afc812664a` |
| `packages/agent.md` | `2a0a8f7bdc5ed53153532ea825489c7e9f985e1f` |
| `packages/ai.md` | `c3462a80b8c3b3161116f23b06f82a78567a0325` |
| `packages/chord.md` | `3f461cbb6060b44131d128ed4778f1ee38bbdc16` |
| `packages/client.md` | `044e9c79d4c34c0c500c7911c1ce4754f51436f7` |
| `packages/codemode.md` | `609e96de6b6ef7848a05c9cf37eed9a1ea445cbe` |
| `packages/coding-agent.md` | `ca346d6c6dd51fdd09ee30a4f52b520a738754a3` |
| `packages/durable.md` | `e64547b053b5d5c3451615d0de29ffb5c0179581` |
| `packages/evals.md` | `151c56c47fd6b87d1a7821083e2c41d3ce0bd3ed` |
| `packages/mcp.md` | `5e64c1b59cf00211621a9e0835bfe34121f9e66d` |
| `packages/protocol.md` | `7d2cdc93f9777ea7636c7f2edc29b35837b3eaa1` |
| `packages/server.md` | `e814b8171f9519b723d890e6d010a0782d3f79ed` |
| `packages/telemetry.md` | `1cca8e4e2f841afc5205a2e06f626d61a5bb0c10` |
| `packages/tui.md` | `bf70bc44948a17ecd7dd7fd1ff738d5e376ad904` |
| `source-index.md` | `060dd65f9491cedec9abf3c1b7caed11677f581e` |
| `source-manifest.json` | `a823c64b6620da96b3b4abc2b88bad94486e364e` |

在仓库根目录可只读查看完整历史报告，例如：

```powershell
git show d472a50393422121c9343332dea125c7a98ada19:docs/research/pi-1.0.0/packages/ai.md
git show d472a50393422121c9343332dea125c7a98ada19:docs/research/pi-1.0.0/packages/telemetry.md
git show d472a50393422121c9343332dea125c7a98ada19:docs/research/pi-1.0.0/documentation-index.md
```

需要整套报告时，可用 `git archive` 从该固定提交导出 `docs/research/pi-1.0.0` 到独立目录；无须切换分支或恢复到当前工作区。新报告的正常阅读不依赖执行这些恢复操作。

旧报告中的源码路径指向当时的 `.repo` 快照；本地已经没有该快照，恢复报告本身不会恢复源码，也不应把旧行号链接替换成 1.0.2 文件来冒充历史证据。

<a id="early-reports"></a>

## 早期 ai 与 telemetry 报告

补充规划审读之前的 ai 报告为 411 行，telemetry 报告为 167 行。它们的精确文本保存在更早提交 `f039d682aec11365e381685186faad114f0b909f`，与上表的后续增补版本不同；引用这些行数与摘要的历史记录应读取下列对象：

| 报告 | 行数 | Git blob 正文的 SHA-256 |
|---|---:|---|
| `packages/ai.md` | 411 | `84b681314a9eee5df197dbf709a9204df9a4986e32a0c262b61054cfc24387af` |
| `packages/telemetry.md` | 167 | `e7183c807a73d230764df35ca7472c4898fb24dddae15c5967f62f797ad8f2b4` |

```powershell
git show f039d682aec11365e381685186faad114f0b909f:docs/research/pi-1.0.0/packages/ai.md
git show f039d682aec11365e381685186faad114f0b909f:docs/research/pi-1.0.0/packages/telemetry.md
```

清单的历史工作区记录还有一个字节摘要 `7ba3764ae8921274ce78c6a8e20eeb5e75b949e8a57cc72438b2344345c0210f`。它等于本附件 LF 清单转为 CRLF 后的摘要，JSON 内容和源码树摘要不变。迁入附件保持删除前的 LF 字节，不重写过去的检查结果或冒称两个文件的字节摘要相同。

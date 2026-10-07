# Pi 1.0.2 历史证据与继承说明

> 整理及退役日期：2026-10-07。旧报告目录 `docs/research/pi-1.0.2` 已安全移除；当前研究入口为 [Pi 1.0.4](../../README.md)。本目录保全旧版证据和精确恢复位置，新版研究可独立使用。

## 比较对象

旧源码树摘要为 `f0309990522b4c47b96e19ec684c53fd1c8fd8a5bc7918ba70ec3289bf3f13fb`，文件清单共 1,859 项。原 [source-manifest.json](source-manifest.json) 逐字节复制，附件 SHA-256：`4cf023e1a6b0bbcec9aeb540614bac35dafea343788ef86c87b2e5515cb37ed9`。清单定位文件身份，不含源码正文。

[documentation-reading.json](documentation-reading.json) 从旧 Markdown 索引逐项提取 125 条审读记录，保留分类、口径、索引行号，补绑定旧源码摘要。未明确的精确范围不推造；“继承”条目继续指向 [1.0.0 原始证据](../pi-1.0.0/README.md)。哈希相同仅证明同一内容，不证明本轮重读。

## 原报告恢复

[report-recovery.json](report-recovery.json) 记录本轮改旧导航之前 22 个报告附件的工作区 SHA-256、Git blob、Git 正文 SHA-256 和行数。固定提交 `057af397945316a1954ec7ec65270e732f052d5a` 已逐个确认对象可读。Git 与工作区可能只有 CRLF/LF 差异，具体以 same_bytes 字段为准。

例如在仓库根目录只读查看：

```powershell
git show 057af397945316a1954ec7ec65270e732f052d5a:docs/research/pi-1.0.2/README.md
git show 057af397945316a1954ec7ec65270e732f052d5a:docs/research/pi-1.0.2/documentation-index.md
git show 057af397945316a1954ec7ec65270e732f052d5a:docs/research/pi-1.0.2/packages/tui.md
```

这里的 Git revision 属研究报告所在仓库，不是 Pi 上游 revision。旧源码目录缺失，恢复报告不会恢复旧源码链接。无需恢复旧目录即可阅读新版主体。

## 退役内容保全

[retirement.json](retirement.json) 记录移除前实际22个文件的字节数、SHA-256、固定Git提交、仓库相对路径和blob身份。21个文件与提交逐字节一致；README只有上一次升级新增的基准导航不同，其完整字节另存为 [README-before-retirement.txt](README-before-retirement.txt)。该文本仅作历史恢复附件，内部原始Markdown链接保持当时路径，不是当前导航。

恢复21个原文件及原始README正文依赖此仓库保留上述Git对象；最后一版README可直接从文本附件按字节复制，不依赖Git。恢复整套原报告可从固定提交导出，再用附件覆盖README；不要将恢复结果覆盖当前基准。目录检查涵盖隐藏、未跟踪和忽略文件，未发现额外文件或链接目录。更早的18个报告对象和两份早期ai/telemetry版本也已重新读取核验，见 [1.0.0恢复说明](../pi-1.0.0/README.md)。

<a id="tui-plan"></a>

## 删除的 TUI 计划

1.0.2 清单曾记录 `tui-plan.md`（1,001 行）；1.0.4 清单已无此路径。[historical-planning.json](historical-planning.json) 绑定旧文件摘要，并保留旧文档核查和 TUI 子报告的相关分析原文及精确恢复位置。

它是历史二手分析，不能代替丢失的计划原文。本轮对虚拟化、overlay、滚动路由等仍可检查现行代码，但只能称为旧材料记录的方向或历史设计差距；文件删除本身不能证明作者继续承诺或已撤回这些方向。

# Pi 1.0.4 历史证据与恢复

整理日期：2026-10-16。当前独立研究入口为 [Pi 1.2.0](../../README.md)。本页保留 1.0.4 比较身份、退役恢复与更早历史依赖，不把历史报告重标为当前版本分析。

## 当前升级所用证据

- [source-manifest.json](source-manifest.json) 为原 1.0.4 清单逐字节复制，共 1,908 文件，树摘要 `7e74bf510f6a2ced24656f8a03f576e01067d66fbe2e89bafb9c33059593de02`。这份清单供路径与内容差异计算，不含旧源码正文。
- [documentation-reading.json](documentation-reading.json) 保留旧版实际审读声明，未经本次改写。当前研究逐份记录本轮阅读，不将历史记录冒称本轮阅读。
- [report-recovery.json](report-recovery.json) 固定原目录 32 个实际文件的大小、工作区 SHA-256、Git commit、路径、blob 与 Git 正文 SHA-256。所有文件均已通过 `git show COMMIT:PATH` 取回原字节，与删除前实际内容逐字节一致；没有需额外归档的未提交、未跟踪或忽略文件。

以上 JSON 保留原始字节；原清单、审读附件的文件字节摘要与源码树摘要不同。它们各自的文件 SHA-256 可在 recovery 清单中查到，不能与 Git blob ID 混用。

## 恢复完整 1.0.4 报告

固定报告仓库提交：`e6c2eb80b99a69f80aefc7aafee3b61140a0a6c8`。这是研究报告所在仓库的提交，不是 Pi 上游 revision。正常阅读新版无需恢复旧目录；完整旧文恢复需要此仓库保留相应 Git 对象。

在仓库根可只读查看，例如：

```powershell
git show e6c2eb80b99a69f80aefc7aafee3b61140a0a6c8:docs/research/pi-1.0.4/README.md
git show e6c2eb80b99a69f80aefc7aafee3b61140a0a6c8:docs/research/pi-1.0.4/packages/durable.md
git show e6c2eb80b99a69f80aefc7aafee3b61140a0a6c8:docs/research/pi-1.0.4/documentation-index.md
```

整套文件可从该提交用 `git archive` 导出到独立位置；不要覆盖当前报告。恢复目录中的源码链接仍指向原 `.repo/pi-1.0.4`，本地没有该旧快照，恢复报告不恢复源码。新旧清单支持路径与摘要比较，不提供完整旧新源码逐行 diff。

## 更早版本与历史消费者

本次将已有 [1.0.2 证据](../pi-1.0.2/README.md)和 [1.0.0 证据](../pi-1.0.0/README.md)一起迁出待删目录，保留 manifest、审读记录、退役时的 README 原字节和历史规划说明。旧 JSON/文本内的目录名与日期继续表示当时事实；两个历史 README 的当前导航随本次迁移更新，其原版本也包含在固定 Git 恢复清单中。

[predecessor-recovery-verification.json](predecessor-recovery-verification.json) 记录本次实际读取核对的 42 个较早 Git 对象：1.0.2 的 22 个文件、1.0.0 的 18 个文件，以及 411 行 ai / 167 行 telemetry 的两份早期精确正文。早期正文位于 `f039d682aec11365e381685186faad114f0b909f`，有别于后续增补版本；相关历史研究仍应使用 [early-reports](../pi-1.0.0/README.md#early-reports) 的摘要身份。

迁移不重新核验历史研究的运行结果、比较决策或旧源码。仓库内现行报告导航已改到新版，对历史报告的链接则仍指向对应版本恢复说明；纯历史日志和版本兼容文字不作全局版本替换。未知仓库外消费者不在本次可检索范围。

旧目录已于2026-10-16移除，32个已核验文件及空目录逐项删除；[retirement.json](retirement.json)记录范围与检查。原源码快照没有被修改或删除。最终校验命令和实际结果见[验证记录](../../verification.md)。

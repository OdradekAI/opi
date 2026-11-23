# #220 将 pack.py 收窄为只读的 Pi 目录比较工具

Status: done
Owner: scripts/pi-catalog
Parent: [#209](209-maintain-module-test-and-catalog-boundaries.md)
Blocked by: none

0.x 迁移通过固定参考数据复现目录差异。`pack.py` 将 JSON 报告写到标准输出。正式目录由 [#221](221-reconcile-pi-1-5-catalog.md) 的 opi 源数据和打包入口维护。

## 验收

- 显式选择 opi 目录和 Pi 参考输入。对于发布归档，核对 pin 与原始响应 SHA-256。对于已生成的 provider 文件，核对该快照的清单与文件摘要。报告标明输入种类、来源版本及内容身份，例如同为 Pi 1.5.0 的 1,712 条发布归档和 1,738 条本地快照分别具名。
- 按 `(provider, type, id)` 报告仅 opi 有、仅参考有及共同条目的逐字段差异，保留字段缺失与显式空值的区别。重复身份、错误的 provider 归属或摘要错误给出具体诊断。输入相同时报告可复现。
- 移除脚本向产品数据和发行资源写入的路径。比较结果写标准输出或独立证据目录。比较只在维护者选取参考时运行，opi 新增模型、打包和发版有自己的入口。

## 验证

使用含 opi 自增模型、同名模型本地修订、上游删除和上游新增的前后两份小参考数据，实跑 CLI。检查每类差异以及比较前后 opi 输入与发行资源的摘要。对损坏输入重复此检查。再用固定的真实 Pi 1.5.0 输入执行只读比较并记录来源。

## 结果

- CLI 通过 8 项子进程测试。差异报告保留显式空值、缺失字段和不同类型的值。错误的结构摘要、损坏文件、超出有效范围的数值、重复身份、错误归属和旧写入命令均被拒绝。
- 真实 Pi 1.5.0 发布归档与本地生成数据各比较两次。两组报告分别以 1,712 和 1,738 条为参考，各自重算字节一致。连续运行前后 42 份 opi 源文件和两份发行资源的摘要全部相同。证据见[执行记录](../evidence/issue-220-readonly-catalog-comparison.json)。
- `python scripts/pi-catalog/pack.py --help` 列出入口。发布归档用 `--pi-artifact` 和 `--pi-pin`，已生成数据用 `--pi-provider-data`。两种入口均显式提供 `--opi-catalog` 和 `--pi-version`。

Test impact: add 比较 CLI 正负例与产品资源写入保护。retain 历史 Pi 对照证据。

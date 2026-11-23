# 文档导航

[English](README.md)

从当前任务出发，按需打开下面的权威来源，而不是先加载整套文档。这个索引不是新的全局 spec。

| 问题 | 维护位置 |
|---|---|
| Agent 如何在仓库内工作？ | [根指导](../AGENTS.md) |
| 如何设计工程结构、编码与验证？ | [工程指导](engineering/README.md) |
| 如何对齐迭代目标、处理文档过时或语义冲突？ | [上下文生命周期](agents/context.md) |
| 领域术语是什么意思？ | [术语表](GLOSSARY.md)、[使用规则](agents/domain.md) |
| 为什么作出某个重要决定？ | [ADR 索引](adr/README.md) |
| 这次迭代要交付什么？ | 用户选定的 [迭代简报](iterations/README.md) 或明确提供的 issue |
| Matt 技能把 map 和 ticket 写到哪里？ | [本地 tracker 配置](agents/issue-tracker.md) |
| 产品目前实际如何工作？ | 根目录和 crate README、源码、schema、测试与生成的帮助 |
| 哪些平台检查需要运行？ | [CI 策略](engineering/ci.zh.md) |

## 证据：按需读取

`docs/research/` 存放研究报告和快照对比。根据问题及实际来源身份选择报告，
采用建议前用当前来源核验其结论。

在 Git 中查看 2026-11-06 的对比报告和配套证据。

```text
git ls-tree -r 8030700c207cb9dc181eb824a89cc4dc85050af9 -- docs/realign
```

## 维护方式

当前文档与对应代码、决策一起更新。已完成的简报保留为历史；长期有效的结论进入术语表、
相关 ADR 或产品文档。这套结构不创建全局路线图或第二份实施台账。

面向 agent 的工程、上下文、tracker 和术语文件只维护英文主本，以减少同步面；
本导航提供中文镜像。已有双语产品文档继续同步更新。

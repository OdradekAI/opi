# 冻结的 eval producer 契约

[English](README.md)

这些 YAML 文件逐字节保留原手动分发的 GitHub producer。它们位于 Actions
工作流目录之外，只作为开发参考，供离线契约测试和历史证据验证使用。内嵌的
`.github/workflows/` 路径描述历史 dispatch 身份；不要改写这些身份，也不要
将这些文件当成本地工作流执行。

当前校准使用[本地 CLI 和 operator](../../README.zh.md)。materializer 的
`prepare --cache --plan` 模式仍是本地获取入口。GitHub receipt 模式仍要求
原有权威；本地命令不得伪造 GitHub 环境或上传 receipt。

当前 static lock 指向迁移后的 external-lock 契约，其 workflow 摘要不变。
路径变化使 static lock 本身获得新摘要。旧 external 工件必须使用匹配的历史
checkout、lock 和 producer 字节验证。native 工件验证仍保留原 workflow
路径，并读取原始 Git 对象。已有证据不会重新绑定到当前 lock。这些参考文件
不授权任何真实试验。

# 分离纯值、公共错误与认证目录契约
Status: done
Owner: Codex for user
Blocked by: [建立中立遥测基础与可选 tracing 适配](02-neutral-telemetry-foundation.md)

## Scope

承接用户“继续推进”，实施第二阶段剩余基础边界。工作树基点
814c95f7ce575fb9be3df6b9d04ff6e2942706c1，开始时 staged/unstaged/untracked 均为空。

将消息、模型、用量、图像、分类、deferred 及其附属纯值集中到 types；
运行控制留在原操作模块。公共错误、脱敏和既有 service-fault 分类归到 errors，
消除其对 HTTP/Bedrock 实现的反向依赖。分离 auth 合同与 ambient/云实现，
catalog 合同、离线解码与 HTTP 来源/内嵌资产。已有有消费者的路径以同一类型
重导出保留，不复制行为或引入新万能 trait。

本票不改变请求执行、取消、认证选择/刷新、Store mutation 或目录发布语义；
不删除测试，不替换网络/云 backend，不实现 feature 配置矩阵。

## Acceptance

- types 和通用错误的生产实现不导入网络客户端、SDK、执行驱动或目录资产。
- 值的序列化、费用/验证、错误类别/重试信号/脱敏/因果链与既有行为一致。
- 自定义认证与动态目录的合同不定义在具体后端中；离线解码可直接调用。
- Store mutation 的并发、顺序、持久化和取消职责保持原所有者。
- 旧公共路径与新所有者指向同一类型；高级消费者编译与相关合同保持通过。
- 同步双语文档、迁移表、契约索引；冻结第一阶段证据保持原样。

## Checks planned

先运行受影响既有合同作为基准，再重构并运行相同合同；增加/调整新所有者入口
验证。范围覆盖模型/消息/费用、错误/脱敏、transcript/replay、图像/分类/deferred、
auth/Store/OAuth 与 catalog。受影响 clippy、workspace 消费者编译、doctest/rustdoc、
fmt、文档与 diff 检查；实际包消费复用 SDK harness。已知全量回归失败独立记录，
不以本票结果覆盖。依赖版本不升级。

## Test impact

add：必要的新公共入口/隔离验证。update：相关测试的 canonical imports。
retain：所有现有断言和第一阶段证据。delete：none。

## 交付与检查

完成本票所有者提取、同一类型的旧路径重导出、离线 decoder、双语入口说明及
181 项顶层声明迁移记录；详见[切片报告](../phase2-foundation.md)和
[检查记录](../evidence/phase2-foundation-checks.json)。第一阶段与前一切片索引保持冻结。

- 改动前 11 target 的 371 tests PASS；改动后同组加库内测试 740 PASS。
- 补充 28 个集成 target 的 773 tests PASS；源文件隔离的原有 8 tests PASS。
- 工作区 Clippy all-targets、11 doctests、公共 rustdoc、fmt 和文档检查 PASS。
- 实际 ai/telemetry 包的 3 个接入程序和 1 个 peer 回归 PASS。
- 没有删除测试或新增公共状态探针；六个现有测试文件调整入口，消息内测随所有者移动。

各运行之间有覆盖重叠，不合计为独立验收数量。未执行全 workspace all-targets 回归，
前一切片的已知失败继续保留；未验证其他 OS、MSRV、live 或完整严格等价。
本票完成第二阶段的剩余基础整理，不等于完整能力对齐或最小 feature 已交付。
工作树改动尚未提交；下一步是第三阶段执行者/消费者和 AI 埋点。

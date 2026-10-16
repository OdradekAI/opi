# 文档索引与实际审读范围

对象：本地 Pi 1.2.0，2026-10-16；共137份默认文本（`.md/.mdx/.rst/.txt`，含隐藏目录、提示资源和许可证）。逐项路径、SHA-256与行数来自[清单](source-manifest.json)，实际审读声明由本轮分析者记录并合并于[documentation-reading.json](documentation-reading.json)。

源码树 SHA-256：`a2c03e7c77aa5e6837ed69055a29d724eff9e65a18c1534221ef58e299cc0c10`。本轮均为新读，不将旧报告或同哈希文件的既有阅读冒称本次全文阅读；记录范围是文本阅读范围，不能当运行测试覆盖率。全文不意味着逐个示例均运行或全部主张逐行对实现验证；选段/结构筛查的范围如实列明。文档意图和当前契约是否一致另见[时效审计](documentation-audit.md)。

| 文档（当前源码） | 行数 | 类型 | 本轮实际深度 | 读取行范围（闭区间） |
|---|---:|---|---|---|
| [.pi/prompts/cl.md](../../../.repo/pi-1.2.0/.pi/prompts/cl.md) | 54 | 开发工作流 prompt；研究对象，不是本次执行指令 | 本次全文阅读；按资料性质核对 manifest、构建/发行/triage 调用方；外部链接未抓取 | 1–54 |
| [.pi/prompts/deslop.md](../../../.repo/pi-1.2.0/.pi/prompts/deslop.md) | 67 | 开发工作流 prompt；研究对象，不是本次执行指令 | 本次全文阅读；按资料性质核对 manifest、构建/发行/triage 调用方；外部链接未抓取 | 1–67 |
| [.pi/prompts/dup.md](../../../.repo/pi-1.2.0/.pi/prompts/dup.md) | 18 | 开发工作流 prompt；研究对象，不是本次执行指令 | 本次全文阅读；按资料性质核对 manifest、构建/发行/triage 调用方；外部链接未抓取 | 1–18 |
| [.pi/prompts/is.md](../../../.repo/pi-1.2.0/.pi/prompts/is.md) | 28 | 开发工作流 prompt；研究对象，不是本次执行指令 | 本次全文阅读；按资料性质核对 manifest、构建/发行/triage 调用方；外部链接未抓取 | 1–28 |
| [.pi/prompts/pr.md](../../../.repo/pi-1.2.0/.pi/prompts/pr.md) | 37 | 开发工作流 prompt；研究对象，不是本次执行指令 | 本次全文阅读；按资料性质核对 manifest、构建/发行/triage 调用方；外部链接未抓取 | 1–37 |
| [.pi/prompts/sa.md](../../../.repo/pi-1.2.0/.pi/prompts/sa.md) | 163 | 开发工作流 prompt；研究对象，不是本次执行指令 | 本次全文阅读；按资料性质核对 manifest、构建/发行/triage 调用方；外部链接未抓取 | 1–163 |
| [.pi/prompts/wr.md](../../../.repo/pi-1.2.0/.pi/prompts/wr.md) | 41 | 开发工作流 prompt；研究对象，不是本次执行指令 | 本次全文阅读；按资料性质核对 manifest、构建/发行/triage 调用方；外部链接未抓取 | 1–41 |
| [.pi/skills/add-llm-provider.md](../../../.repo/pi-1.2.0/.pi/skills/add-llm-provider.md) | 57 | 开发/发布检查清单与设计约束 | 本次全文阅读；按资料性质核对 manifest、构建/发行/triage 调用方；外部链接未抓取 | 1–57 |
| [.pi/skills/interactive-testing.md](../../../.repo/pi-1.2.0/.pi/skills/interactive-testing.md) | 19 | 开发/发布检查清单与设计约束 | 本次全文阅读；按资料性质核对 manifest、构建/发行/triage 调用方；外部链接未抓取 | 1–19 |
| [.pi/skills/release.md](../../../.repo/pi-1.2.0/.pi/skills/release.md) | 52 | 开发/发布检查清单与设计约束 | 本次全文阅读；按资料性质核对 manifest、构建/发行/triage 调用方；外部链接未抓取 | 1–52 |
| [AGENTS.md](../../../.repo/pi-1.2.0/AGENTS.md) | 125 | 开发治理、贡献原则与工作流约束 | 本次全文阅读；按资料性质核对 manifest、构建/发行/triage 调用方；外部链接未抓取 | 1–125 |
| [CONTRIBUTING.md](../../../.repo/pi-1.2.0/CONTRIBUTING.md) | 102 | 开发治理、贡献原则与工作流约束 | 本次全文阅读；按资料性质核对 manifest、构建/发行/triage 调用方；外部链接未抓取 | 1–102 |
| [README.md](../../../.repo/pi-1.2.0/README.md) | 206 | 仓库概览、包边界、安装/构建与外部 RFC 导航 | 本次全文阅读；按资料性质核对 manifest、构建/发行/triage 调用方；外部链接未抓取 | 1–206 |
| [SECURITY.md](../../../.repo/pi-1.2.0/SECURITY.md) | 87 | 本地用户信任边界与非目标 | 本次全文阅读；按资料性质核对 manifest、构建/发行/triage 调用方；外部链接未抓取 | 1–87 |
| [packages/agent/CHANGELOG.md](../../../.repo/pi-1.2.0/packages/agent/CHANGELOG.md) | 780 | 发行历史、已兑现/被替代的接口方向 | 全文语义阅读；逐项核对当前与本次升级的重要主张，早期历史条目按后续替代关系判断，未将全部历史修复逐项运行验证 | 1–780 |
| [packages/agent/README.md](../../../.repo/pi-1.2.0/packages/agent/README.md) | 564 | 现行循环/工具协议、生命周期与使用示例 | 全文阅读；Agent/agent-loop/proxy/types 与主要测试行为交叉核查 | 1–564 |
| [packages/ai/CHANGELOG.md](../../../.repo/pi-1.2.0/packages/ai/CHANGELOG.md) | 2310 | 发行历史、已兑现/被替代的接口方向 | 全文语义阅读；逐项核对当前与本次升级的重要主张，早期历史条目按后续替代关系判断，未将全部历史修复逐项运行验证；部分长历史段显示时剥离重复 Markdown 链接目标，保留完整条目文本 | 1–2310 |
| [packages/ai/README.md](../../../.repo/pi-1.2.0/packages/ai/README.md) | 1949 | 现行包说明、API/配置、边界与迁移指导 | 全文语义阅读；核心 Models/auth/transcript/streams/升级 adapter 与调用方交叉核查；目录值缺失与文档过时条目独立说明 | 1–1949 |
| [packages/chord/CHANGELOG.md](../../../.repo/pi-1.2.0/packages/chord/CHANGELOG.md) | 20 | 运行时替换及旧包移除的发行记录 | 全文新读；旧service/facet/bundler移除、typed host接替和依赖变化对照manifest/exports | 1–20 |
| [packages/chord/README.md](../../../.repo/pi-1.2.0/packages/chord/README.md) | 147 | 新Host运行时与state/delta说明 | 全文新读；公开入口、可信不可变与异步订阅边界对照当前导出/state实现 | 1–147 |
| [packages/chord/docs/guide.md](../../../.repo/pi-1.2.0/packages/chord/docs/guide.md) | 750 | 现行Host完整使用与装配指南 | 全文新读750行；activation/两阶段readiness/授权/版本/reload/Durable示例等关键主张与实现对照；未逐行审计每段示例实现 | 1–750 |
| [packages/chord/docs/spec.md](../../../.repo/pi-1.2.0/packages/chord/docs/spec.md) | 261 | 规范性Host设计、限制与已否决方案 | 全文新读261行；契约/生命周期/readiness/reload/cluster及末节12项限制逐项追踪当前实现；one-state方案明确已否决 | 1–261 |
| [packages/chord/docs/tutorial.md](../../../.repo/pi-1.2.0/packages/chord/docs/tutorial.md) | 909 | 完整分步教学与可执行示例契约 | 全文新读909行；动机、状态/资源/授权/远端/readiness/reload及末段持久化否定均复核；代表示例测试仅静态阅读，未执行 | 1–909 |
| [packages/chord/src/delta/README.md](../../../.repo/pi-1.2.0/packages/chord/src/delta/README.md) | 193 | 现行Delta所有权、重放及限制 | 全文新读；严格JSON、root transfer、draft placement、adopt、apply/codec寿命与当前调用方对照；历史性能不复测 | 1–193 |
| [packages/codemode/CHANGELOG.md](../../../.repo/pi-1.2.0/packages/codemode/CHANGELOG.md) | 79 | 发行历史、portable/deterministic及破坏性变化 | 全文新读；1.1/1.2主张及MCP专用声明删除、options/interrupt/watch修复对照当前实现 | 1–79 |
| [packages/codemode/README.md](../../../.repo/pi-1.2.0/packages/codemode/README.md) | 356 | 现行三类VM运行时、确定性与部署说明 | 全文新读356行；Node/Inline/Remote/Cloudflare、start与execute差异、limits/store/失败边界对照实现 | 1–356 |
| [packages/codemode/examples/cloudflare/README.md](../../../.repo/pi-1.2.0/packages/codemode/examples/cloudflare/README.md) | 31 | 双Worker示例操作说明 | 全文新读；与remote/cloudflare实现及两个wrangler绑定静态对照，未安装或部署 | 1–31 |
| [packages/coding-agent/CHANGELOG.md](../../../.repo/pi-1.2.0/packages/coding-agent/CHANGELOG.md) | 6263 | 版本历史 | 本次重点阅读1.2.0/1.1.0及临近旧版发布段；未通读全部历史 | 1–120 |
| [packages/coding-agent/README.md](../../../.repo/pi-1.2.0/packages/coding-agent/README.md) | 95 | 包说明、API与原生构建边界 | 本次全文阅读；关键契约与当前实现交叉核查（非运行验证） | 1–95 |
| [packages/coding-agent/docs/cli-integration.md](../../../.repo/pi-1.2.0/packages/coding-agent/docs/cli-integration.md) | 106 | 现行接口、行为与边界说明 | 本次全文阅读；关键契约与当前实现交叉核查（非运行验证） | 1–106 |
| [packages/coding-agent/docs/cli.md](../../../.repo/pi-1.2.0/packages/coding-agent/docs/cli.md) | 333 | 现行配置、CLI或模型参考 | 本次阅读开头、结尾、全部标题及默认值/信任/鉴权/重试等契约段；未逐行通读全部示例 | 1–32、55、78、105、111、117、120、124、128、151、158、160、164、176、180、182、186、192、221、227、230、234、238、240、250、258、269、285、290、293、305、313、315–333 |
| [packages/coding-agent/docs/codemode.md](../../../.repo/pi-1.2.0/packages/coding-agent/docs/codemode.md) | 228 | 现行接口、行为与边界说明 | 本次全文阅读；关键契约与当前实现交叉核查（非运行验证） | 1–228 |
| [packages/coding-agent/docs/compaction.md](../../../.repo/pi-1.2.0/packages/coding-agent/docs/compaction.md) | 463 | 现行接口、行为与边界说明 | 本次全文阅读；关键契约与当前实现交叉核查（非运行验证） | 1–463 |
| [packages/coding-agent/docs/configuration.md](../../../.repo/pi-1.2.0/packages/coding-agent/docs/configuration.md) | 47 | 现行接口、行为与边界说明 | 本次全文阅读；关键契约与当前实现交叉核查（非运行验证） | 1–47 |
| [packages/coding-agent/docs/containerization.md](../../../.repo/pi-1.2.0/packages/coding-agent/docs/containerization.md) | 183 | 协议、平台或用户操作参考 | 本次阅读开头、结尾、全部标题及生命周期/默认值/失败/协议边界段；未逐行通读全部命令示例 | 1–25、30、34、53、56、65、72、82、84、86、88、101、105、113、116、121、123、125、127、134、137、142、144、147–148、151、153、161、168–183 |
| [packages/coding-agent/docs/custom-provider.md](../../../.repo/pi-1.2.0/packages/coding-agent/docs/custom-provider.md) | 173 | 现行配置、CLI或模型参考 | 本次阅读开头、结尾、全部标题及默认值/信任/鉴权/重试等契约段；未逐行通读全部示例 | 1–32、74、85、87、91、101、110、120、126、137、139、148、152、154–173 |
| [packages/coding-agent/docs/environment-variables.md](../../../.repo/pi-1.2.0/packages/coding-agent/docs/environment-variables.md) | 100 | 现行配置、CLI或模型参考 | 本次阅读开头、结尾、全部标题及默认值/信任/鉴权/重试等契约段；未逐行通读全部示例 | 1–32、51、53、75、81–100 |
| [packages/coding-agent/docs/extensions.md](../../../.repo/pi-1.2.0/packages/coding-agent/docs/extensions.md) | 277 | 现行接口、行为与边界说明 | 本次全文阅读；关键契约与当前实现交叉核查（非运行验证） | 1–277 |
| [packages/coding-agent/docs/how-pi-works.md](../../../.repo/pi-1.2.0/packages/coding-agent/docs/how-pi-works.md) | 49 | 现行接口、行为与边界说明 | 本次全文阅读；关键契约与当前实现交叉核查（非运行验证） | 1–49 |
| [packages/coding-agent/docs/index.md](../../../.repo/pi-1.2.0/packages/coding-agent/docs/index.md) | 39 | 终端平台、主题或用户导航参考 | 本次阅读开头、结尾、全部标题及终端/默认值/兼容边界段；平台操作示例未全部逐行阅读 | 1–39 |
| [packages/coding-agent/docs/json.md](../../../.repo/pi-1.2.0/packages/coding-agent/docs/json.md) | 226 | 协议、平台或用户操作参考 | 本次阅读开头、结尾、全部标题及生命周期/默认值/失败/协议边界段；未逐行通读全部命令示例 | 1–25、31、45、48、50、56、62、70、101、117、128、161、182、198、211–226 |
| [packages/coding-agent/docs/keybindings.md](../../../.repo/pi-1.2.0/packages/coding-agent/docs/keybindings.md) | 195 | 协议、平台或用户操作参考 | 本次阅读开头、结尾、全部标题及生命周期/默认值/失败/协议边界段；未逐行通读全部命令示例 | 1–25、31、45、47、49、51、70、72、84、86、99、103、120、122、131、133、135、148、150、155、160、162、169、171、177、180–195 |
| [packages/coding-agent/docs/llama-cpp.md](../../../.repo/pi-1.2.0/packages/coding-agent/docs/llama-cpp.md) | 121 | 协议、平台或用户操作参考 | 本次阅读开头、结尾、全部标题及生命周期/默认值/失败/协议边界段；未逐行通读全部命令示例 | 1–25、46、54、68、81、89、98、106–121 |
| [packages/coding-agent/docs/mcp.md](../../../.repo/pi-1.2.0/packages/coding-agent/docs/mcp.md) | 272 | 现行接口、行为与边界说明 | 本次全文阅读；关键契约与当前实现交叉核查（非运行验证） | 1–272 |
| [packages/coding-agent/docs/message-types.md](../../../.repo/pi-1.2.0/packages/coding-agent/docs/message-types.md) | 279 | 协议、平台或用户操作参考 | 本次阅读开头、结尾、全部标题及生命周期/默认值/失败/协议边界段；未逐行通读全部命令示例 | 1–25、27、39、52、67、92、94、109、119、161、196、200、220、237、250、263–279 |
| [packages/coding-agent/docs/models.md](../../../.repo/pi-1.2.0/packages/coding-agent/docs/models.md) | 209 | 现行配置、CLI或模型参考 | 本次阅读开头、结尾、全部标题及默认值/信任/鉴权/重试等契约段；未逐行通读全部示例 | 1–32、35、37、45、51、65、69、90、102、104、126、130、145、147、170、174、182、185、189、191–209 |
| [packages/coding-agent/docs/packages.md](../../../.repo/pi-1.2.0/packages/coding-agent/docs/packages.md) | 129 | 现行配置、CLI或模型参考 | 本次阅读开头、结尾、全部标题及默认值/信任/鉴权/重试等契约段；未逐行通读全部示例 | 1–32、42、76、90、94、111–129 |
| [packages/coding-agent/docs/prompt-templates.md](../../../.repo/pi-1.2.0/packages/coding-agent/docs/prompt-templates.md) | 59 | 现行接口、行为与边界说明 | 本次全文阅读；关键契约与当前实现交叉核查（非运行验证） | 1–59 |
| [packages/coding-agent/docs/providers.md](../../../.repo/pi-1.2.0/packages/coding-agent/docs/providers.md) | 247 | 现行配置、CLI或模型参考 | 本次阅读开头、结尾、全部标题及默认值/信任/鉴权/重试等契约段；未逐行通读全部示例 | 1–32、67、71、84、86、104、116、136、147、159、176、181、184、187、190、193、195、200、214、225、229–247 |
| [packages/coding-agent/docs/quickstart.md](../../../.repo/pi-1.2.0/packages/coding-agent/docs/quickstart.md) | 136 | 现行配置、CLI或模型参考 | 本次阅读开头、结尾、全部标题及默认值/信任/鉴权/重试等契约段；未逐行通读全部示例 | 1–32、37、52、66、84、86、96、102、116、118–136 |
| [packages/coding-agent/docs/rpc-commands.md](../../../.repo/pi-1.2.0/packages/coding-agent/docs/rpc-commands.md) | 860 | 协议、平台或用户操作参考 | 本次阅读开头、结尾、全部标题及生命周期/默认值/失败/协议边界段；未逐行通读全部命令示例 | 1–25、46、70、94、107、130、153、155、188、208、210、228、252、272、274、291、309、329、331、341、348、358、365、367、406、419、421、434、447、449、511、524、526、569、592、610、638、666、689、719、747、767、786、788、836、845–860 |
| [packages/coding-agent/docs/rpc-extension-ui.md](../../../.repo/pi-1.2.0/packages/coding-agent/docs/rpc-extension-ui.md) | 200 | 协议、平台或用户操作参考 | 本次阅读开头、结尾、全部标题及生命周期/默认值/失败/协议边界段；未逐行通读全部命令示例 | 1–25、27、31、33、42、48、59、65、81、97、111、113、129、144、146、159、172、174、176、182、185–200 |
| [packages/coding-agent/docs/rpc.md](../../../.repo/pi-1.2.0/packages/coding-agent/docs/rpc.md) | 193 | 现行接口、行为与边界说明 | 本次全文阅读；关键契约与当前实现交叉核查（非运行验证） | 1–193 |
| [packages/coding-agent/docs/sdk.md](../../../.repo/pi-1.2.0/packages/coding-agent/docs/sdk.md) | 147 | 现行接口、行为与边界说明 | 本次全文阅读；关键契约与当前实现交叉核查（非运行验证） | 1–147 |
| [packages/coding-agent/docs/security.md](../../../.repo/pi-1.2.0/packages/coding-agent/docs/security.md) | 99 | 现行接口、行为与边界说明 | 本次全文阅读；关键契约与当前实现交叉核查（非运行验证） | 1–99 |
| [packages/coding-agent/docs/session-format.md](../../../.repo/pi-1.2.0/packages/coding-agent/docs/session-format.md) | 293 | 现行接口、行为与边界说明 | 本次全文阅读；关键契约与当前实现交叉核查（非运行验证） | 1–293 |
| [packages/coding-agent/docs/sessions.md](../../../.repo/pi-1.2.0/packages/coding-agent/docs/sessions.md) | 66 | 现行接口、行为与边界说明 | 本次全文阅读；关键契约与当前实现交叉核查（非运行验证） | 1–66 |
| [packages/coding-agent/docs/settings.md](../../../.repo/pi-1.2.0/packages/coding-agent/docs/settings.md) | 171 | 现行配置、CLI或模型参考 | 本次阅读开头、结尾、全部标题及默认值/信任/鉴权/重试等契约段；未逐行通读全部示例 | 1–33、35–36、38、40、42、46、48、52、58、60、62、64、68、70、79、83、85、88、90、92、102、113–114、119、121、123、125–133、135、137、139、141、147、151、153–171 |
| [packages/coding-agent/docs/shell-aliases.md](../../../.repo/pi-1.2.0/packages/coding-agent/docs/shell-aliases.md) | 93 | 协议、平台或用户操作参考 | 本次阅读开头、结尾、全部标题及生命周期/默认值/失败/协议边界段；未逐行通读全部命令示例 | 1–25、36、38、50、77–93 |
| [packages/coding-agent/docs/skills.md](../../../.repo/pi-1.2.0/packages/coding-agent/docs/skills.md) | 93 | 现行接口、行为与边界说明 | 本次全文阅读；关键契约与当前实现交叉核查（非运行验证） | 1–93 |
| [packages/coding-agent/docs/slash-commands.md](../../../.repo/pi-1.2.0/packages/coding-agent/docs/slash-commands.md) | 60 | 协议、平台或用户操作参考 | 本次阅读开头、结尾、全部标题及生命周期/默认值/失败/协议边界段；未逐行通读全部命令示例 | 1–25、33、44–60 |
| [packages/coding-agent/docs/terminal-setup.md](../../../.repo/pi-1.2.0/packages/coding-agent/docs/terminal-setup.md) | 233 | 终端平台、主题或用户导航参考 | 本次阅读开头、结尾、全部标题及终端/默认值/兼容边界段；平台操作示例未全部逐行阅读 | 1–25、29、31、41、45、47、63、65、67、69、80、82、107、109、112、116、118、131、133、152、167、169、171、184、186、190、192、196、198、200、203、207、218–233 |
| [packages/coding-agent/docs/termux.md](../../../.repo/pi-1.2.0/packages/coding-agent/docs/termux.md) | 117 | 终端平台、主题或用户导航参考 | 本次阅读开头、结尾、全部标题及终端/默认值/兼容边界段；平台操作示例未全部逐行阅读 | 1–25、46、48、58、75、77、79、82、93、95、102–117 |
| [packages/coding-agent/docs/themes.md](../../../.repo/pi-1.2.0/packages/coding-agent/docs/themes.md) | 138 | 终端平台、主题或用户导航参考 | 本次阅读开头、结尾、全部标题及终端/默认值/兼容边界段；平台操作示例未全部逐行阅读 | 1–26、48、59、61、63、70、74–75、78、92、94、98、102、104、109、118、123–138 |
| [packages/coding-agent/docs/tmux.md](../../../.repo/pi-1.2.0/packages/coding-agent/docs/tmux.md) | 55 | 终端平台、主题或用户导航参考 | 本次阅读开头、结尾、全部标题及终端/默认值/兼容边界段；平台操作示例未全部逐行阅读 | 1–25、35、40–55 |
| [packages/coding-agent/docs/tui.md](../../../.repo/pi-1.2.0/packages/coding-agent/docs/tui.md) | 125 | 现行接口、行为与边界说明 | 本次全文阅读；关键契约与当前实现交叉核查（非运行验证） | 1–125 |
| [packages/coding-agent/docs/usage.md](../../../.repo/pi-1.2.0/packages/coding-agent/docs/usage.md) | 94 | 终端平台、主题或用户导航参考 | 本次阅读开头、结尾、全部标题及终端/默认值/兼容边界段；平台操作示例未全部逐行阅读 | 1–25、29、44、55、68、78–94 |
| [packages/coding-agent/docs/virtual-models.md](../../../.repo/pi-1.2.0/packages/coding-agent/docs/virtual-models.md) | 114 | 现行配置、CLI或模型参考 | 本次阅读开头、结尾、全部标题及默认值/信任/鉴权/重试等契约段；未逐行通读全部示例 | 1–32、36、56–57、59、63、72、81、84、88、96–114 |
| [packages/coding-agent/docs/windows.md](../../../.repo/pi-1.2.0/packages/coding-agent/docs/windows.md) | 67 | 终端平台、主题或用户导航参考 | 本次阅读开头、结尾、全部标题及终端/默认值/兼容边界段；平台操作示例未全部逐行阅读 | 1–25、31、33、41、45、51–67 |
| [packages/coding-agent/examples/README.md](../../../.repo/pi-1.2.0/packages/coding-agent/examples/README.md) | 36 | 可选扩展示例索引或示例提示词 | 本次全文阅读；示例提示词只按示例资源解释，未执行 | 1–36 |
| [packages/coding-agent/examples/extensions/README.md](../../../.repo/pi-1.2.0/packages/coding-agent/examples/extensions/README.md) | 214 | 可选扩展示例索引或示例提示词 | 本次结构筛查、开头/结尾与全部章节标题阅读；示例索引未逐项通读 | 1–24、29、46、82、89、98、104、110、119、126、134、141、184、194–214 |
| [packages/coding-agent/examples/extensions/doom-overlay/README.md](../../../.repo/pi-1.2.0/packages/coding-agent/examples/extensions/doom-overlay/README.md) | 46 | 可选示例说明与示例资源（非默认产品计划） | 本次全文阅读；按示例的显式加载、资源及权限边界核对 | 1–46 |
| [packages/coding-agent/examples/extensions/dynamic-resources/SKILL.md](../../../.repo/pi-1.2.0/packages/coding-agent/examples/extensions/dynamic-resources/SKILL.md) | 8 | 可选示例说明与示例资源（非默认产品计划） | 本次全文阅读；按示例的显式加载、资源及权限边界核对 | 1–8 |
| [packages/coding-agent/examples/extensions/dynamic-resources/dynamic.md](../../../.repo/pi-1.2.0/packages/coding-agent/examples/extensions/dynamic-resources/dynamic.md) | 5 | 可选示例说明与示例资源（非默认产品计划） | 本次全文阅读；按示例的显式加载、资源及权限边界核对 | 1–5 |
| [packages/coding-agent/examples/extensions/plan-mode/README.md](../../../.repo/pi-1.2.0/packages/coding-agent/examples/extensions/plan-mode/README.md) | 66 | 可选示例说明与示例资源（非默认产品计划） | 本次全文阅读；按示例的显式加载、资源及权限边界核对 | 1–66 |
| [packages/coding-agent/examples/extensions/subagent/README.md](../../../.repo/pi-1.2.0/packages/coding-agent/examples/extensions/subagent/README.md) | 177 | 可选示例说明与示例资源（非默认产品计划） | 本次全文阅读；按示例的显式加载、资源及权限边界核对 | 1–177 |
| [packages/coding-agent/examples/extensions/subagent/agents/planner.md](../../../.repo/pi-1.2.0/packages/coding-agent/examples/extensions/subagent/agents/planner.md) | 37 | 可选扩展示例索引或示例提示词 | 本次全文阅读；示例提示词只按示例资源解释，未执行 | 1–37 |
| [packages/coding-agent/examples/extensions/subagent/agents/reviewer.md](../../../.repo/pi-1.2.0/packages/coding-agent/examples/extensions/subagent/agents/reviewer.md) | 35 | 可选扩展示例索引或示例提示词 | 本次全文阅读；示例提示词只按示例资源解释，未执行 | 1–35 |
| [packages/coding-agent/examples/extensions/subagent/agents/scout.md](../../../.repo/pi-1.2.0/packages/coding-agent/examples/extensions/subagent/agents/scout.md) | 50 | 可选扩展示例索引或示例提示词 | 本次全文阅读；示例提示词只按示例资源解释，未执行 | 1–50 |
| [packages/coding-agent/examples/extensions/subagent/agents/worker.md](../../../.repo/pi-1.2.0/packages/coding-agent/examples/extensions/subagent/agents/worker.md) | 24 | 可选扩展示例索引或示例提示词 | 本次全文阅读；示例提示词只按示例资源解释，未执行 | 1–24 |
| [packages/coding-agent/examples/extensions/subagent/prompts/implement-and-review.md](../../../.repo/pi-1.2.0/packages/coding-agent/examples/extensions/subagent/prompts/implement-and-review.md) | 10 | 可选扩展示例索引或示例提示词 | 本次全文阅读；示例提示词只按示例资源解释，未执行 | 1–10 |
| [packages/coding-agent/examples/extensions/subagent/prompts/implement.md](../../../.repo/pi-1.2.0/packages/coding-agent/examples/extensions/subagent/prompts/implement.md) | 10 | 可选扩展示例索引或示例提示词 | 本次全文阅读；示例提示词只按示例资源解释，未执行 | 1–10 |
| [packages/coding-agent/examples/extensions/subagent/prompts/scout-and-plan.md](../../../.repo/pi-1.2.0/packages/coding-agent/examples/extensions/subagent/prompts/scout-and-plan.md) | 9 | 可选扩展示例索引或示例提示词 | 本次全文阅读；示例提示词只按示例资源解释，未执行 | 1–9 |
| [packages/coding-agent/examples/sdk/README.md](../../../.repo/pi-1.2.0/packages/coding-agent/examples/sdk/README.md) | 141 | 可选扩展示例索引或示例提示词 | 本次结构筛查、开头/结尾与全部章节标题阅读；示例索引未逐项通读 | 1–24、26、33、105、120–141 |
| [packages/coding-agent/test/fixtures/skills-collision/first/calendar/SKILL.md](../../../.repo/pi-1.2.0/packages/coding-agent/test/fixtures/skills-collision/first/calendar/SKILL.md) | 8 | 测试fixture（非真实计划） | 本次全文阅读；按测试输入或测试使用说明解释，不执行其中指令 | 1–8 |
| [packages/coding-agent/test/fixtures/skills-collision/second/calendar/SKILL.md](../../../.repo/pi-1.2.0/packages/coding-agent/test/fixtures/skills-collision/second/calendar/SKILL.md) | 8 | 测试fixture（非真实计划） | 本次全文阅读；按测试输入或测试使用说明解释，不执行其中指令 | 1–8 |
| [packages/coding-agent/test/fixtures/skills/consecutive-hyphens/SKILL.md](../../../.repo/pi-1.2.0/packages/coding-agent/test/fixtures/skills/consecutive-hyphens/SKILL.md) | 8 | 测试fixture（非真实计划） | 本次全文阅读；按测试输入或测试使用说明解释，不执行其中指令 | 1–8 |
| [packages/coding-agent/test/fixtures/skills/disable-model-invocation/SKILL.md](../../../.repo/pi-1.2.0/packages/coding-agent/test/fixtures/skills/disable-model-invocation/SKILL.md) | 9 | 测试fixture（非真实计划） | 本次全文阅读；按测试输入或测试使用说明解释，不执行其中指令 | 1–9 |
| [packages/coding-agent/test/fixtures/skills/invalid-name-chars/SKILL.md](../../../.repo/pi-1.2.0/packages/coding-agent/test/fixtures/skills/invalid-name-chars/SKILL.md) | 8 | 测试fixture（非真实计划） | 本次全文阅读；按测试输入或测试使用说明解释，不执行其中指令 | 1–8 |
| [packages/coding-agent/test/fixtures/skills/invalid-yaml/SKILL.md](../../../.repo/pi-1.2.0/packages/coding-agent/test/fixtures/skills/invalid-yaml/SKILL.md) | 8 | 测试fixture（非真实计划） | 本次全文阅读；按测试输入或测试使用说明解释，不执行其中指令 | 1–8 |
| [packages/coding-agent/test/fixtures/skills/long-name/SKILL.md](../../../.repo/pi-1.2.0/packages/coding-agent/test/fixtures/skills/long-name/SKILL.md) | 8 | 测试fixture（非真实计划） | 本次全文阅读；按测试输入或测试使用说明解释，不执行其中指令 | 1–8 |
| [packages/coding-agent/test/fixtures/skills/missing-description/SKILL.md](../../../.repo/pi-1.2.0/packages/coding-agent/test/fixtures/skills/missing-description/SKILL.md) | 7 | 测试fixture（非真实计划） | 本次全文阅读；按测试输入或测试使用说明解释，不执行其中指令 | 1–7 |
| [packages/coding-agent/test/fixtures/skills/multiline-description/SKILL.md](../../../.repo/pi-1.2.0/packages/coding-agent/test/fixtures/skills/multiline-description/SKILL.md) | 11 | 测试fixture（非真实计划） | 本次全文阅读；按测试输入或测试使用说明解释，不执行其中指令 | 1–11 |
| [packages/coding-agent/test/fixtures/skills/name-mismatch/SKILL.md](../../../.repo/pi-1.2.0/packages/coding-agent/test/fixtures/skills/name-mismatch/SKILL.md) | 8 | 测试fixture（非真实计划） | 本次全文阅读；按测试输入或测试使用说明解释，不执行其中指令 | 1–8 |
| [packages/coding-agent/test/fixtures/skills/nested/child-skill/SKILL.md](../../../.repo/pi-1.2.0/packages/coding-agent/test/fixtures/skills/nested/child-skill/SKILL.md) | 8 | 测试fixture（非真实计划） | 本次全文阅读；按测试输入或测试使用说明解释，不执行其中指令 | 1–8 |
| [packages/coding-agent/test/fixtures/skills/no-frontmatter/SKILL.md](../../../.repo/pi-1.2.0/packages/coding-agent/test/fixtures/skills/no-frontmatter/SKILL.md) | 3 | 测试fixture（非真实计划） | 本次全文阅读；按测试输入或测试使用说明解释，不执行其中指令 | 1–3 |
| [packages/coding-agent/test/fixtures/skills/root-skill-preferred/SKILL.md](../../../.repo/pi-1.2.0/packages/coding-agent/test/fixtures/skills/root-skill-preferred/SKILL.md) | 3 | 测试fixture（非真实计划） | 本次全文阅读；按测试输入或测试使用说明解释，不执行其中指令 | 1–3 |
| [packages/coding-agent/test/fixtures/skills/root-skill-preferred/nested-child/SKILL.md](../../../.repo/pi-1.2.0/packages/coding-agent/test/fixtures/skills/root-skill-preferred/nested-child/SKILL.md) | 3 | 测试fixture（非真实计划） | 本次全文阅读；按测试输入或测试使用说明解释，不执行其中指令 | 1–3 |
| [packages/coding-agent/test/fixtures/skills/unknown-field/SKILL.md](../../../.repo/pi-1.2.0/packages/coding-agent/test/fixtures/skills/unknown-field/SKILL.md) | 10 | 测试fixture（非真实计划） | 本次全文阅读；按测试输入或测试使用说明解释，不执行其中指令 | 1–10 |
| [packages/coding-agent/test/fixtures/skills/valid-skill/SKILL.md](../../../.repo/pi-1.2.0/packages/coding-agent/test/fixtures/skills/valid-skill/SKILL.md) | 8 | 测试fixture（非真实计划） | 本次全文阅读；按测试输入或测试使用说明解释，不执行其中指令 | 1–8 |
| [packages/coding-agent/test/mcp-conformance/README.md](../../../.repo/pi-1.2.0/packages/coding-agent/test/mcp-conformance/README.md) | 45 | 测试套件说明 | 本次全文阅读；按测试输入或测试使用说明解释，不执行其中指令 | 1–45 |
| [packages/coding-agent/test/suite/README.md](../../../.repo/pi-1.2.0/packages/coding-agent/test/suite/README.md) | 10 | 测试套件说明 | 本次全文阅读；按测试输入或测试使用说明解释，不执行其中指令 | 1–10 |
| [packages/durable/CHANGELOG.md](../../../.repo/pi-1.2.0/packages/durable/CHANGELOG.md) | 129 | 发行历史及破坏性契约变化 | 全文新读；1.1/1.2重要新增、破坏性变化及修复与当前实现交叉核查；性能数字未复测 | 1–129 |
| [packages/durable/README.md](../../../.repo/pi-1.2.0/packages/durable/README.md) | 751 | 现行包说明、宿主装配与使用边界 | 全文新读；输入/工具/恢复/错误/缓存/存储/扩展等重要主张与当前导出、核心实现和代表测试对照 | 1–751 |
| [packages/durable/docs/spec.md](../../../.repo/pi-1.2.0/packages/durable/docs/spec.md) | 5340 | 规范性设计、状态机与非目标 | 全文新读5340行（分段；工具输出截断处补读）；关键章节与当前实现及测试核对，识别2.2 InputEntry等局部陈旧，非全实现逐行审计 | 1–5340 |
| [packages/env/CHANGELOG.md](../../../.repo/pi-1.2.0/packages/env/CHANGELOG.md) | 20 | 发行历史与新增能力 | 全文新读；1.2 listFiles/nodePlatform/PTY/foreground逐项对照实现及代表测试 | 1–20 |
| [packages/env/README.md](../../../.repo/pi-1.2.0/packages/env/README.md) | 66 | 现行包说明、平台与部署边界 | 全文新读；SSH/Connection/LocalExecutionEnv/listFiles/terminal及测试前置与实现对照 | 1–66 |
| [packages/env/docs/protocol.md](../../../.repo/pi-1.2.0/packages/env/docs/protocol.md) | 151 | 现行二进制协议与调度契约 | 全文新读；帧、取消、资源上限、output窗口、features/listFiles/PTY与TS/Rust实现对照 | 1–151 |
| [packages/env/docs/semantics.md](../../../.repo/pi-1.2.0/packages/env/docs/semantics.md) | 68 | 跨平台Node语义对照契约 | 全文新读；文件/exec/watch/listFiles职责与实现交叉核查；未逐行审计全部sys平台调用 | 1–68 |
| [packages/evals/README.md](../../../.repo/pi-1.2.0/packages/evals/README.md) | 129 | 现行评测方法、隔离与结果解释契约 | 全文新读；完整cohort/paired lift/缺失数据/隔离/网络限制/作者边界与runner、Docker、harness、report核查 | 1–129 |
| [packages/mcp/CHANGELOG.md](../../../.repo/pi-1.2.0/packages/mcp/CHANGELOG.md) | 73 | 发行历史、已兑现/被替代的接口方向 | 全文语义阅读；逐项核对当前与本次升级的重要主张，早期历史条目按后续替代关系判断，未将全部历史修复逐项运行验证 | 1–73 |
| [packages/mcp/LICENSES/modelcontextprotocol-typescript-sdk.txt](../../../.repo/pi-1.2.0/packages/mcp/LICENSES/modelcontextprotocol-typescript-sdk.txt) | 21 | 第三方 MIT 许可证 | 全文阅读；与 OAuth 文件中 SDK v1.29.0 改编声明、manifest 无 SDK 运行依赖交叉核对 | 1–21 |
| [packages/mcp/README.md](../../../.repo/pi-1.2.0/packages/mcp/README.md) | 136 | 独立客户端当前契约与非目标 | 全文阅读；client/transport/OAuth/content 主链交叉核查 | 1–136 |
| [packages/telemetry/CHANGELOG.md](../../../.repo/pi-1.2.0/packages/telemetry/CHANGELOG.md) | 47 | 发行历史、已兑现/被替代的接口方向 | 全文语义阅读；逐项核对当前与本次升级的重要主张，早期历史条目按后续替代关系判断，未将全部历史修复逐项运行验证 | 1–47 |
| [packages/telemetry/README.md](../../../.repo/pi-1.2.0/packages/telemetry/README.md) | 464 | 通用 telemetry 契约、schema/adapter 设计与集成说明 | 全文阅读；NOOP/memory/schema/conformance 与当前 agent exports 交叉核查 | 1–464 |
| [packages/tui/CHANGELOG.md](../../../.repo/pi-1.2.0/packages/tui/CHANGELOG.md) | 1285 | 版本历史 | 本次重点阅读1.2.0/1.1.0及临近旧版发布段；未通读全部历史 | 1–155 |
| [packages/tui/README.md](../../../.repo/pi-1.2.0/packages/tui/README.md) | 949 | 包说明、API与原生构建边界 | 本次全文阅读；关键契约与当前实现交叉核查（非运行验证） | 1–949 |
| [packages/tui/native/darwin/README.md](../../../.repo/pi-1.2.0/packages/tui/native/darwin/README.md) | 22 | 包说明、API与原生构建边界 | 本次全文阅读；关键契约与当前实现交叉核查（非运行验证） | 1–22 |
| [packages/tui/native/linux/README.md](../../../.repo/pi-1.2.0/packages/tui/native/linux/README.md) | 23 | 包说明、API与原生构建边界 | 本次全文阅读；关键契约与当前实现交叉核查（非运行验证） | 1–23 |
| [packages/tui/native/win32/README.md](../../../.repo/pi-1.2.0/packages/tui/native/win32/README.md) | 31 | 包说明、API与原生构建边界 | 本次全文阅读；关键契约与当前实现交叉核查（非运行验证） | 1–31 |
| [scripts/issues/README.md](../../../.repo/pi-1.2.0/scripts/issues/README.md) | 59 | 包外工具说明与 Durable 调用示例 | 本次全文阅读；按资料性质核对 manifest、构建/发行/triage 调用方；外部链接未抓取 | 1–59 |
| [scripts/issues/categories/agent-loop.md](../../../.repo/pi-1.2.0/scripts/issues/categories/agent-loop.md) | 21 | issue triage 分类提示资源；非路线图 | 本次全文阅读；按资料性质核对 manifest、构建/发行/triage 调用方；外部链接未抓取 | 1–21 |
| [scripts/issues/categories/cli.md](../../../.repo/pi-1.2.0/scripts/issues/categories/cli.md) | 18 | issue triage 分类提示资源；非路线图 | 本次全文阅读；按资料性质核对 manifest、构建/发行/triage 调用方；外部链接未抓取 | 1–18 |
| [scripts/issues/categories/codemode.md](../../../.repo/pi-1.2.0/scripts/issues/categories/codemode.md) | 10 | issue triage 分类提示资源；非路线图 | 本次全文阅读；按资料性质核对 manifest、构建/发行/triage 调用方；外部链接未抓取 | 1–10 |
| [scripts/issues/categories/compaction.md](../../../.repo/pi-1.2.0/scripts/issues/categories/compaction.md) | 17 | issue triage 分类提示资源；非路线图 | 本次全文阅读；按资料性质核对 manifest、构建/发行/triage 调用方；外部链接未抓取 | 1–17 |
| [scripts/issues/categories/durable.md](../../../.repo/pi-1.2.0/scripts/issues/categories/durable.md) | 10 | issue triage 分类提示资源；非路线图 | 本次全文阅读；按资料性质核对 manifest、构建/发行/triage 调用方；外部链接未抓取 | 1–10 |
| [scripts/issues/categories/extensions.md](../../../.repo/pi-1.2.0/scripts/issues/categories/extensions.md) | 21 | issue triage 分类提示资源；非路线图 | 本次全文阅读；按资料性质核对 manifest、构建/发行/triage 调用方；外部链接未抓取 | 1–21 |
| [scripts/issues/categories/mcp.md](../../../.repo/pi-1.2.0/scripts/issues/categories/mcp.md) | 10 | issue triage 分类提示资源；非路线图 | 本次全文阅读；按资料性质核对 manifest、构建/发行/triage 调用方；外部链接未抓取 | 1–10 |
| [scripts/issues/categories/models.md](../../../.repo/pi-1.2.0/scripts/issues/categories/models.md) | 21 | issue triage 分类提示资源；非路线图 | 本次全文阅读；按资料性质核对 manifest、构建/发行/triage 调用方；外部链接未抓取 | 1–21 |
| [scripts/issues/categories/other.md](../../../.repo/pi-1.2.0/scripts/issues/categories/other.md) | 10 | issue triage 分类提示资源；非路线图 | 本次全文阅读；按资料性质核对 manifest、构建/发行/triage 调用方；外部链接未抓取 | 1–10 |
| [scripts/issues/categories/prompt.md](../../../.repo/pi-1.2.0/scripts/issues/categories/prompt.md) | 16 | issue triage 分类提示资源；非路线图 | 本次全文阅读；按资料性质核对 manifest、构建/发行/triage 调用方；外部链接未抓取 | 1–16 |
| [scripts/issues/categories/providers.md](../../../.repo/pi-1.2.0/scripts/issues/categories/providers.md) | 23 | issue triage 分类提示资源；非路线图 | 本次全文阅读；按资料性质核对 manifest、构建/发行/triage 调用方；外部链接未抓取 | 1–23 |
| [scripts/issues/categories/sdk.md](../../../.repo/pi-1.2.0/scripts/issues/categories/sdk.md) | 16 | issue triage 分类提示资源；非路线图 | 本次全文阅读；按资料性质核对 manifest、构建/发行/triage 调用方；外部链接未抓取 | 1–16 |
| [scripts/issues/categories/sessions.md](../../../.repo/pi-1.2.0/scripts/issues/categories/sessions.md) | 15 | issue triage 分类提示资源；非路线图 | 本次全文阅读；按资料性质核对 manifest、构建/发行/triage 调用方；外部链接未抓取 | 1–15 |
| [scripts/issues/categories/tools.md](../../../.repo/pi-1.2.0/scripts/issues/categories/tools.md) | 21 | issue triage 分类提示资源；非路线图 | 本次全文阅读；按资料性质核对 manifest、构建/发行/triage 调用方；外部链接未抓取 | 1–21 |
| [scripts/issues/categories/tui.md](../../../.repo/pi-1.2.0/scripts/issues/categories/tui.md) | 19 | issue triage 分类提示资源；非路线图 | 本次全文阅读；按资料性质核对 manifest、构建/发行/triage 调用方；外部链接未抓取 | 1–19 |
| [scripts/issues/prompt.md](../../../.repo/pi-1.2.0/scripts/issues/prompt.md) | 31 | issue triage 提示策略与优先级口径；非已采纳产品计划 | 本次全文阅读；按资料性质核对 manifest、构建/发行/triage 调用方；外部链接未抓取 | 1–31 |

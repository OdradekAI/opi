# Pi 1.0.2 tui 包分析

## 1 定位与架构结论

`@earendil-works/pi-tui` 是面向 Node 终端的组件、输入、布局和渲染库。它没有内部 AI 包依赖，也不拥有消息、模型或工具执行的业务状态。它提供的稳定抽象是“给定宽度渲染为终端行”的 Component，以及可以替换为虚拟终端的 Terminal 接口。

1.0.2 的 TUI 不能只概括为一个差分打印器：它有主屏幕与全屏两个 renderer，字符和控制序列解析、焦点及 overlay 状态机、应用拥有的滚动/选择/搜索、Markdown 和图片、编辑器与异步补全，以及三个操作系统的 native helper。`TUI` 在根入口是类型，实际实例为 `TuiMainScreen` 或 `TuiAltScreen`。[index.ts](../../../../.repo/pi-1.0.2/packages/tui/src/index.ts)、[tui.ts:453](../../../../.repo/pi-1.0.2/packages/tui/src/tui.ts#L453)。

本报告为静态分析。重点阅读 renderer、terminal、输入缓冲、布局、编辑器与补全关键路径、Unicode 工具和代表性回归测试；并非对全部 native C/Objective-C 和全部 Unicode 分支逐行审计。未运行 TUI、操作系统剪贴板测试、终端兼容矩阵或性能基准。

## 1.0.2 升级核查（2026-10-05）

本包 121 个文件中 113 个哈希未变、8 个变化，无增删。变化为 package/CHANGELOG、`components/image.ts`、`index.ts`、`terminal-image.ts`、`tui-alt-screen.ts` 与两个图片相关测试文件。依据 [两版源码清单](../source-manifest.json) 和 [旧版清单](../evidence/pi-1.0.0/source-manifest.json) 复用未变路径，重读现版图片转换、Kitty placement 和滚轮路由；旧源码不在现场，修改文件只按当前实现与发布记录解释，不声称取得逐行旧文本差异。

1.0.1 新增同步 `setImageTranscoder`，修正 Kitty 非 PNG 图片静默不显示，以及 WezTerm 全屏滚动后图片塌成单行的问题；1.0.2 的 TUI 发布标题没有额外条目。README、三个 native README 与 `tui-plan.md` 全文哈希未变，仍保留其已实现/未来/非目标混合性质。第 14 节更新图片行为，第 19 节保留并重新定位仍存在的 contain 偏差。见 [CHANGELOG.md:5](../../../../.repo/pi-1.0.2/packages/tui/CHANGELOG.md#L5)。

## 2 发布边界与依赖

| 项目 | 内容 |
|---|---|
| 版本与格式 | 1.0.2，ESM |
| 入口 | `main=dist/index.js`、`types=dist/index.d.ts`，没有独立 exports 映射 |
| runtime dependencies | `get-east-asian-width 1.6.0`、`marked 18.0.11` |
| dev dependencies | `@xterm/headless 5.5.0`、`chalk 6.0.0` |
| 引擎 | Node >=22.19.0 |
| 普通 build | TypeScript 编译 |
| native build | 分 Darwin、Linux、Windows 的显式脚本 |
| test | Node 内建 `node --test test/*.test.ts`，不是 Vitest |
| 发布资产 | dist、native 源码/脚本/说明与 x64/arm64 预编译 `.node` |

依据：[package.json](../../../../.repo/pi-1.0.2/packages/tui/package.json)。根导出包含文件系统补全、native loader 和 ProcessTerminal，不能把它理解为与运行环境无关的纯字符串包；例如 [terminal.ts:1](../../../../.repo/pi-1.0.2/packages/tui/src/terminal.ts#L1) 与 [native-platform.ts:1](../../../../.repo/pi-1.0.2/packages/tui/src/native-platform.ts#L1) 直接导入 Node 能力。

## 3 模块地图

| 模块 | 所有权与核心问题 |
|---|---|
| `tui.ts` | Component/Container、TuiBase、调度、焦点、overlay、终端响应分流 |
| `tui-main-screen.ts` | 与终端原生 scrollback 协作的差分输出和光标模型 |
| `tui-alt-screen.ts` | 应用拥有的 viewport、鼠标、文本选择、搜索、图像缓存 |
| `terminal.ts` | stdin/stdout、raw mode、键盘协议、启停恢复 |
| `stdin-buffer.ts` | 将任意分块输入拼接为控制序列或粘贴事件 |
| `keys.ts / keybindings.ts` | 序列到按键语义、动作到可配置按键 |
| `layout-node.ts / layout.ts` | 栈与滚动布局树、测量、裁剪、命中路径 |
| `utils.ts` | ANSI 解析、字素列宽、截断、换行、超链接范围 |
| `components/editor.ts` | 多行输入、逻辑/视觉光标、粘贴、撤销、补全 |
| `autocomplete.ts / fuzzy.ts` | 命令、参数、路径建议与排序 |
| `components/markdown.ts / latex.ts` | Markdown token 到 ANSI 行、有限 LaTeX 到 Unicode |
| `terminal-image.ts / components/image.ts` | 终端图像协议、尺寸、缓存与退化文本 |
| `colors.ts / oklab.ts / terminal-colors.ts` | 颜色空间、ANSI 样式和终端主题响应 |
| `native-* / native/` | native helper 定位、修饰键、剪贴板和 Windows VT |

完整源文件清单见 [源码索引](../source-index.md)。

## 4 Component 和 Terminal 是两条不同边界

`Component.render(width): string[]` 是同步渲染接口；每个字符串是一条逻辑终端行。组件可选实现 handleInput、handleMouse，必须提供 invalidate。Container 纵向连接子组件的行，同时保存本帧的子组件高度，供鼠标坐标转换使用。[tui.ts:117](../../../../.repo/pi-1.0.2/packages/tui/src/tui.ts#L117)、[tui.ts:347](../../../../.repo/pi-1.0.2/packages/tui/src/tui.ts#L347)。

Terminal 则负责启动输入/resize 回调、停止并恢复环境、写入、尺寸、光标和终端进度等操作。renderer 依赖此接口而非直接订阅 stdin，所以测试可以使用 `VirtualTerminal` 验证最终屏幕，产品可以选择 ProcessTerminal。[terminal.ts:71](../../../../.repo/pi-1.0.2/packages/tui/src/terminal.ts#L71)。

组件返回的字符串允许 ANSI，因此字节长度、JavaScript string.length、字素数和终端列数都不同。返回足够窄的行是组件契约；主屏差分路径发现超宽行时会写 crash dump、停止终端再抛错，而全屏最终画面会裁剪到宽度。不能把某一路径的宽度处理推及所有 renderer 分支。[tui-main-screen.ts:263](../../../../.repo/pi-1.0.2/packages/tui/src/tui-main-screen.ts#L263)、[tui-alt-screen.ts](../../../../.repo/pi-1.0.2/packages/tui/src/tui-alt-screen.ts)。

## 5 一次按键如何到达编辑器

```text
process.stdin data
  → StdinBuffer.process：重组 ESC/CSI/OSC/DCS/APC 和 bracketed paste
  → ProcessTerminal：消费键盘协议协商响应，处理 native Shift+Enter
  → TuiBase.handleTerminalInput：消费颜色/主题响应
  → inputListeners：可替换或消费事件
  → 消费 terminal cell 尺寸响应
  → 可见 overlay 与焦点恢复
  → focusedComponent.handleInput
  → requestImmediateRender
```

ProcessTerminal 保存原 raw mode，然后启用 raw mode、UTF-8、bracketed paste、resize 回调和 Windows VT 输入。键盘协商先推 Kitty flags=7，再查询 flags 和 DA1；收到 DA1 而尚未确认 Kitty 时启用 modifyOtherKeys。不能简单认为运行在支持终端里就自动有完整按键语义，协议协商和回退是具体执行路径。[terminal.ts:174](../../../../.repo/pi-1.0.2/packages/tui/src/terminal.ts#L174)。

孤立 ESC 默认等待 10ms；检测 SSH 时默认 100ms，环境变量 `PI_TUI_ESC_TIMEOUT` 可覆盖。一般未完成序列默认等 50ms，键盘协商碎片另有 150ms 拼接等待。这些时间针对不同阶段，不能混为同一个“输入延迟”。[terminal.ts:118](../../../../.repo/pi-1.0.2/packages/tui/src/terminal.ts#L118)、[stdin-buffer.ts:22](../../../../.repo/pi-1.0.2/packages/tui/src/stdin-buffer.ts#L22)。

StdinBuffer 把完整 bracketed paste 作为一个 paste 事件输出，ProcessTerminal 再包回标记交给编辑器。它还处理 WezTerm 的双 ESC 组合和 Kitty printable 后重复出现的 raw 字符。超时残片会作为输入送出；粘贴缓存依赖结束标记完成，没有在该类中声明大小上限。[stdin-buffer.ts:200](../../../../.repo/pi-1.0.2/packages/tui/src/stdin-buffer.ts#L200)、[stdin-buffer.ts:310](../../../../.repo/pi-1.0.2/packages/tui/src/stdin-buffer.ts#L310)。

`keys.ts` 区分 press/repeat/release，支持 legacy、CSI-u Kitty、modifyOtherKeys 等编码；`keybindings.ts` 将动作 ID 与按键列表分离并检测冲突。组件默认不接收 release，除非 `wantsKeyRelease=true`。[keys.ts:505](../../../../.repo/pi-1.0.2/packages/tui/src/keys.ts#L505)、[keys.ts:820](../../../../.repo/pi-1.0.2/packages/tui/src/keys.ts#L820)、[keybindings.ts:231](../../../../.repo/pi-1.0.2/packages/tui/src/keybindings.ts#L231)、[tui.ts:1044](../../../../.repo/pi-1.0.2/packages/tui/src/tui.ts#L1044)。

## 6 渲染调度与交互延迟

普通 `requestRender` 合并重复请求，在 nextTick 后用距离上次渲染至少 16ms 的 timer 调度。键盘事件和强制渲染使用 immediate nextTick 路径，取消已有 timer，以免 Windows timer 粒度拖慢输入反馈。[tui.ts:982](../../../../.repo/pi-1.0.2/packages/tui/src/tui.ts#L982)。

这里的 16ms 是请求合并间隔，不是保证 60 FPS：组件渲染成本、stdout、终端解释速度都在包外或其他层。renderer 没有通用异步组件 await；耗时补全等工作必须先异步更新组件状态，再请求渲染。

## 7 主屏幕差分渲染

`TuiMainScreen` 保存 previousLines、前次尺寸、内容末尾 cursorRow、实际 hardwareCursorRow、最大工作区行数和前一 viewportTop。内容末尾和真实硬件光标分开，是因为编辑器的 IME 光标可能位于输出末尾上方。[tui-main-screen.ts:117](../../../../.repo/pi-1.0.2/packages/tui/src/tui-main-screen.ts#L117)。

一次 doRender 的算法：

1. 按当前宽度渲染组件并叠加 overlay。
2. 提取光标 marker，给文本行做终端输出规范化和样式复位。
3. 首帧完整输出；宽度变化、通常的高度变化和部分回退条件执行 full redraw。
4. 比较前后行，定位 firstChanged 和 lastChanged；图片会扩展变化区间。
5. 无文本变化时仍可移动硬件光标。
6. 变化在已进入 scrollback 的区域时改用 full redraw；否则在可见区移动光标，重写首末变化之间的连续区间并清理删除行。
7. 更新状态，定位并显示/隐藏硬件光标。

关键实现：[tui-main-screen.ts:263](../../../../.repo/pi-1.0.2/packages/tui/src/tui-main-screen.ts#L263)。这是行字符串比较与区间重写，不是字符级 edit script，也不是只写每一条互不相邻的变化行。

写操作包在 synchronized-output 控制序列中。`BoundedTerminalWriter` 将输出分块，阈值为 `1024*1024` 个 JavaScript 字符计数，并避免拆断 UTF-16 surrogate pair。注释称为 1 MiB chunks，但实现约束是 string.length，不是 UTF-8 字节精确上限。[tui-main-screen.ts:8](../../../../.repo/pi-1.0.2/packages/tui/src/tui-main-screen.ts#L8)。

full redraw 的 clear 路径清理 viewport 和 scrollback；宽度改变会引起重新换行，不能只更新局部。Termux 的高度变化有特别分支，避免软键盘每次出现都重放历史。`clearOnShrink` 是可配置策略，不代表关闭后删除行永远不清理。[tui-main-screen.ts:351](../../../../.repo/pi-1.0.2/packages/tui/src/tui-main-screen.ts#L351)。

## 8 全屏模式与布局树

`TuiAltScreen` 进入 alternate screen，关闭自动换行，按环境开启鼠标和 focus 通知。默认把普通 children 包在一个 follow-end 的 ScrollView，也允许 `setLayoutRoot` 提供显式布局根。[tui-alt-screen.ts](../../../../.repo/pi-1.0.2/packages/tui/src/tui-alt-screen.ts)、[tui-alt-screen.ts](../../../../.repo/pi-1.0.2/packages/tui/src/tui-alt-screen.ts)。

布局节点通过 symbol 标记表达 vstack、hstack 或 scroll，包含 basis/grow/shrink/min/max/visibility 等有限布局属性。它不是浏览器 DOM/CSS：测量依然调用 Component.render，scroll 的内容高度来自实际渲染结果。[layout-node.ts:1](../../../../.repo/pi-1.0.2/packages/tui/src/layout-node.ts#L1)、[layout.ts:67](../../../../.repo/pi-1.0.2/packages/tui/src/layout.ts#L67)。

`renderLayoutFrame` 为每帧建立 component×width 缓存，生成带 rect、clip、parent 和 scroll metadata 的 LayoutBox 树，再 paint 到固定终端高度的行数组。只绘制可见矩形并不等于只计算可见内容：当前布局仍可能先渲染完整子内容测量高度，所以不能直接宣称长会话具备完全虚拟化复杂度。[layout.ts:106](../../../../.repo/pi-1.0.2/packages/tui/src/layout.ts#L106)、[layout.ts:380](../../../../.repo/pi-1.0.2/packages/tui/src/layout.ts#L380)。

全屏 doRender 顺序是布局→搜索高亮→回到底部提示→overlay→选择高亮→flash→光标与行复位→屏幕差分。通过绝对终端行定位重绘，与主屏幕的 scrollback 相对光标模型不同。[tui-alt-screen.ts](../../../../.repo/pi-1.0.2/packages/tui/src/tui-alt-screen.ts)。

正常 stop 离开 alternate screen 后会把当前文档输出到主屏，保留可读 transcript；`preserveScreen` 则只完成屏幕交接。启动全屏时暂时禁用 iTerm2 image capability，退出时恢复；Kitty 走专用 placement 路径。[tui-alt-screen.ts](../../../../.repo/pi-1.0.2/packages/tui/src/tui-alt-screen.ts)。

## 9 滚动、鼠标与选择

ScrollView 只有一个子组件，明确拒绝 add/remove/clear。它维护 scrollTop、contentHeight、viewportHeight 和 followingEnd；手动离开底部后保持位置，返回底部可恢复跟随。`scrollBy` 返回未消费的滚动量，由 renderer 配合 overscroll chain/contain 路由；当前 contain 与 primary fallback 的组合存在第 19.2 节说明的静态偏差，不能笼统保证它会阻止一切外层滚动。[components/scroll-view.ts:24](../../../../.repo/pi-1.0.2/packages/tui/src/components/scroll-view.ts#L24)、[routeWheel](../../../../.repo/pi-1.0.2/packages/tui/src/tui-alt-screen.ts)。

鼠标坐标区分 screenX/screenY 和组件局部 x/y。dispatch 记录精确命中 target 与坐标变换；capture 将后续 drag/release 继续送给同一组件。嵌套容器可以保留键盘焦点，避免焦点落在随后被移除的子组件上。[tui.ts:25](../../../../.repo/pi-1.0.2/packages/tui/src/tui.ts#L25)、[tui.ts:81](../../../../.repo/pi-1.0.2/packages/tui/src/tui.ts#L81)。

全屏模式优先尝试 overlay，再尝试布局树命中组件，最后执行 renderer 的滚动和文本选择行为。键盘滚动在普通 overlay 获得焦点时会让路，避免背后 transcript 跟着弹窗按键滚动。[tui-alt-screen.ts](../../../../.repo/pi-1.0.2/packages/tui/src/tui-alt-screen.ts)。

搜索、鼠标文本选择、滚动条拖动、链接点击、复制回调都在全屏 renderer 内部维护。默认 copyOnSelect=true；未提供 copySelection callback 时通过 OSC52 复制。多路复用终端使用 button-motion 而非所有 pointer motion，降低事件量。[tui-alt-screen.ts](../../../../.repo/pi-1.0.2/packages/tui/src/tui-alt-screen.ts)、[tui-alt-screen.ts](../../../../.repo/pi-1.0.2/packages/tui/src/tui-alt-screen.ts)。

滚轮 auto 模式按事件间隔加速，最大每事件 6 行，间隔小于 5ms 的 burst 不加速；本地 macOS 认为系统已加速而保持一行。Alt+wheel 在全屏层另乘 5。这是显式启用 auto 的行为；TuiAltScreen 默认配置为 1 行。[wheel-scroll.ts:1](../../../../.repo/pi-1.0.2/packages/tui/src/wheel-scroll.ts#L1)、[tui-alt-screen.ts](../../../../.repo/pi-1.0.2/packages/tui/src/tui-alt-screen.ts)。

## 10 Overlay 与焦点状态机

Overlay 支持绝对/百分比尺寸、锚点、偏移、边距和依赖终端尺寸的 visible 回调。显示时通常取得焦点；nonCapturing 可保持原输入目标。每个 entry 保存 preFocus 和 focusOrder，视觉前后顺序并非仅由创建时间决定。[tui.ts:243](../../../../.repo/pi-1.0.2/packages/tui/src/tui.ts#L243)、[tui.ts:718](../../../../.repo/pi-1.0.2/packages/tui/src/tui.ts#L718)。

焦点恢复具有 inactive/eligible/blocked 三种内部状态，处理“overlay 暂时因 resize 不可见”“另一个组件正在临时占用焦点”“关闭后恢复目标已被移除”等情况。hide、setHidden、focus、unfocus 是不同操作：hide 永久移除，setHidden 可逆，unfocus 不删除内容。[tui.ts:327](../../../../.repo/pi-1.0.2/packages/tui/src/tui.ts#L327)、[tui.ts:591](../../../../.repo/pi-1.0.2/packages/tui/src/tui.ts#L591)。

叠加行使用终端列进行裁剪、填充，并在片段间清理 SGR 和 OSC8 超链接状态。overlay 从一个双宽 CJK 字符中间开始时，必须删除被切穿的整个字素再填列，不能将半个汉字留给终端导致列偏移。[tui.ts:415](../../../../.repo/pi-1.0.2/packages/tui/src/tui.ts#L415)、[regression-overlay-cjk-boundary.test.ts:6](../../../../.repo/pi-1.0.2/packages/tui/test/regression-overlay-cjk-boundary.test.ts#L6)。

## 11 Unicode、样式与光标

`visibleWidth` 对 ASCII+ANSI 走快速路径，非 ASCII 使用 Intl.Segmenter、East Asian Width、emoji/零宽/组合标记等规则，维护 512 条字符串宽度缓存。区域旗帜字符即使只流出一个 regional indicator 也按两列保守处理，以减少增量刷新产生的终端自动换行错位。[utils.ts:182](../../../../.repo/pi-1.0.2/packages/tui/src/utils.ts#L182)、[utils.ts:250](../../../../.repo/pi-1.0.2/packages/tui/src/utils.ts#L250)。

公共显示工具按 tab=3 列计算并在终端输出前展开；Editor 的文本存储规范化则按 tab=4 空格。这是两个不同层的策略，不能共用一个“整个项目 tab 宽度”假设。[utils.ts:394](../../../../.repo/pi-1.0.2/packages/tui/src/utils.ts#L394)、[components/editor.ts:1147](../../../../.repo/pi-1.0.2/packages/tui/src/components/editor.ts#L1147)、[tab-width.test.ts:37](../../../../.repo/pi-1.0.2/packages/tui/test/tab-width.test.ts#L37)。

`wrapTextWithAnsi`、`truncateToWidth` 和 `sliceByColumn` 维护样式和超链接边界。光标用零宽 APC marker 表达，renderer 计算 marker 前可见列、移除 marker 并移动硬件光标；即使选择隐藏光标，也需要正确定位以支持 IME 候选窗口。[utils.ts:900](../../../../.repo/pi-1.0.2/packages/tui/src/utils.ts#L900)、[utils.ts:1121](../../../../.repo/pi-1.0.2/packages/tui/src/utils.ts#L1121)、[tui.ts:196](../../../../.repo/pi-1.0.2/packages/tui/src/tui.ts#L196)。

这些宽度规则是为真实终端兼容所做的实现选择，不可能仅凭单元测试推断所有字体、终端 Unicode 版本和 locale 下像素表现一致。

## 12 Editor：文本、视觉行、撤销和粘贴

Editor 同时处理三套坐标：存储中的逻辑行及 UTF-16 列、按终端宽度折行后的视觉行、实际可见窗口中的行列。左右移动、单词操作、上下行移动和点击定位要在这些坐标间转换。文本字素由 Intl.Segmenter 处理，长粘贴占位标记则作为编辑器自定义的不可拆分片段处理。[components/editor.ts:29](../../../../.repo/pi-1.0.2/packages/tui/src/components/editor.ts#L29)、[components/editor.ts:298](../../../../.repo/pi-1.0.2/packages/tui/src/components/editor.ts#L298)。

这不是普通 textarea 的终端皮肤。它包含 Emacs 风格移动及 kill/yank、历史回溯、视觉换行、输入光标显示、补全菜单和提交状态。宿主在成功提交后调用 `addToHistory`；该方法去掉首尾空白、忽略空项和相邻重复项，最多保留 100 项，Editor 的 submit 本身不自动记历史。历史导航还要保留开始导航时尚未提交的草稿。[components/editor.ts:427](../../../../.repo/pi-1.0.2/packages/tui/src/components/editor.ts#L427)。

`getText()` 返回当前内部文本，可能包含大段粘贴标记；`getExpandedText()` 返回展开后的内容。`setText` 会规范化换行和 tab，取消旧补全并清理粘贴注册表；内容发生改变时建立撤销点。`insertTextAtCursor` 把一次程序化插入作为一次可撤销操作。[components/editor.ts:1085](../../../../.repo/pi-1.0.2/packages/tui/src/components/editor.ts#L1085)、[components/editor.ts:1114](../../../../.repo/pi-1.0.2/packages/tui/src/components/editor.ts#L1114)。

超过 10 行或 1,000 字符的粘贴会存入独立 Map，编辑器里显示带编号与大小信息的标记。粘贴过程去除大部分控制字符并规范化内容；提交时展开注册表中的完整清洗后文本，再 trim，并清空输入及相关状态。因此“大段内容只显示一行”是编辑交互策略，不是只把占位标记发送给模型；它也不承诺逐字节保留原始剪贴板数据。[components/editor.ts:1259](../../../../.repo/pi-1.0.2/packages/tui/src/components/editor.ts#L1259)、[components/editor.ts:1363](../../../../.repo/pi-1.0.2/packages/tui/src/components/editor.ts#L1363)。

UndoSnapshot 同时包含文本状态、paste Map 和 pasteCounter，避免撤销后只恢复标记却丢失正文。UndoStack 用 structuredClone 保存快照，没有固定容量；KillRing 也不提供持久化或容量管理。与有上限的历史项不同，这两处内存增长由实际编辑行为决定。[components/editor.ts:224](../../../../.repo/pi-1.0.2/packages/tui/src/components/editor.ts#L224)、[components/editor.ts:2115](../../../../.repo/pi-1.0.2/packages/tui/src/components/editor.ts#L2115)、[undo-stack.ts:7](../../../../.repo/pi-1.0.2/packages/tui/src/undo-stack.ts#L7)、[kill-ring.ts:8](../../../../.repo/pi-1.0.2/packages/tui/src/kill-ring.ts#L8)。

## 13 异步补全：取消与过期结果判定

AutocompleteProvider 暴露异步 suggestions 和同步 applyCompletion。CombinedAutocompleteProvider 合并命令、命令参数及文件路径补全，并不向编辑器暴露文件系统扫描细节。[autocomplete.ts:271](../../../../.repo/pi-1.0.2/packages/tui/src/autocomplete.ts#L271)、[autocomplete.ts:303](../../../../.repo/pi-1.0.2/packages/tui/src/autocomplete.ts#L303)。

编辑器每次重新请求会取消前一个 controller，使用 start token/requestId 和输入快照筛选结果。结果返回时，文本、逻辑行、列和请求身份都仍匹配，才允许展示。它还等待前一个补全任务完成，避免多个 provider 扫描无节制重叠；这意味着不响应 abort 的 provider 仍可能拖延后续补全，AbortSignal 本身不能强制终止任意异步实现。[components/editor.ts:2286](../../../../.repo/pi-1.0.2/packages/tui/src/components/editor.ts#L2286)、[components/editor.ts:2314](../../../../.repo/pi-1.0.2/packages/tui/src/components/editor.ts#L2314)、[components/editor.ts:2415](../../../../.repo/pi-1.0.2/packages/tui/src/components/editor.ts#L2415)。

文件路径补全包含两种实现：直接目录查询和由 fd 支持的模糊扫描。后者会查隐藏文件、跟随链接并排除 `.git`，使用有限结果集再评分裁剪；未配置 fd 时不凭空产生模糊扫描结果。取消通过结束 fd 子进程实现；扫描失败回退为空集合。这是实际具有子进程和文件读取权限的功能，不能因为它属于 UI 包就视为纯字符串处理。[autocomplete.ts:151](../../../../.repo/pi-1.0.2/packages/tui/src/autocomplete.ts#L151)、[autocomplete.ts:737](../../../../.repo/pi-1.0.2/packages/tui/src/autocomplete.ts#L737)。

## 14 Markdown 与图片是两条渲染路径

Markdown 先词法分析，再把 token 转成包含 ANSI 样式的行，最后按终端列折行、加 padding 和基础样式。它缓存相同文本与宽度下的结果，并通过 WeakRef 缓存词法结果，降低长会话反复渲染的成本。流式输入还会处理尚未闭合的末尾 fence，减少逐字符生成时的显示抖动。[components/markdown.ts:146](../../../../.repo/pi-1.0.2/packages/tui/src/components/markdown.ts#L146)、[components/markdown.ts:181](../../../../.repo/pi-1.0.2/packages/tui/src/components/markdown.ts#L181)、[components/markdown.ts:301](../../../../.repo/pi-1.0.2/packages/tui/src/components/markdown.ts#L301)。

组件支持主题、代码高亮和内容 transform；公式由包内 LaTeX 文本转换逻辑处理，并不调用完整 TeX 排版器。因而 Markdown/数学支持应按实现中的 token 和字符映射理解，不能等同浏览器 Markdown 或出版级公式排版。[components/markdown.ts:236](../../../../.repo/pi-1.0.2/packages/tui/src/components/markdown.ts#L236)、[latex.ts:1](../../../../.repo/pi-1.0.2/packages/tui/src/latex.ts#L1)。

图片层分别编码 Kitty 和 iTerm2 协议，读取 PNG/JPEG/GIF/WebP 的头部尺寸并换算终端占用的列、行。它使用默认 9×18 像素 cell 尺寸，同时允许终端响应更新；环境和显式 override 决定协议及 true color、hyperlink 能力。尺寸能解析不等于任意图片格式可以直接发给 Kitty：当前编码使用 `f=100`，需要 PNG 数据。见 [terminal-image.ts:70](../../../../.repo/pi-1.0.2/packages/tui/src/terminal-image.ts#L70)、[terminal-image.ts:220](../../../../.repo/pi-1.0.2/packages/tui/src/terminal-image.ts#L220)。

### 非 PNG 转换由宿主注入，同步执行

根入口公开 `ImageTranscoder` 与 `setImageTranscoder`。转换签名是 `(base64Data, mimeType) => string | null`，在同步 `Image.render` 中执行；TUI 没有增加图片解码库，也没有异步转换队列。只有 Kitty 且 MIME 不为 `image/png` 才调用转换器，PNG 和 iTerm2 保留原数据。未注册转换器或返回 null 时显示有宽度截断的文本 fallback；转换器抛异常没有被此层捕获，不能把所有失败都描述成无条件降级。见 [image.ts:15](../../../../.repo/pi-1.0.2/packages/tui/src/components/image.ts#L15)、[image.ts:101](../../../../.repo/pi-1.0.2/packages/tui/src/components/image.ts#L101)、[index.ts:46](../../../../.repo/pi-1.0.2/packages/tui/src/index.ts#L46)。

跨实例 PNG cache 按原始 base64 内容索引，最多 32 条，以最近使用顺序淘汰，null 结果也缓存；实例另保留成功 PNG，避免共享条目被淘汰或换宽重绘时重转。`setImageTranscoder` 清共享 cache，但不主动 invalidate 已有组件，也不清已有实例的成功 PNG。fallback 后注册新转换器，需要使原组件行缓存失效才重试。转换后的 PNG 头部尺寸优先，容纳宿主转换时的 EXIF 旋转；实际旋转/解码是否正确仍由宿主实现负责。见 [image.ts:20](../../../../.repo/pi-1.0.2/packages/tui/src/components/image.ts#L20)、[image.ts:91](../../../../.repo/pi-1.0.2/packages/tui/src/components/image.ts#L91)、[terminal-image.test.ts:853](../../../../.repo/pi-1.0.2/packages/tui/test/terminal-image.test.ts#L853)。

### Kitty 数据上传与 placement 生命周期

`renderImage` 注册带 transmission generation 的 image metadata；`getKittyImagePlacement` 识别完整分块上传，构造保留裁剪/位置参数的 `a=p` 指令，同时保留行两侧文字。相同 ID、相同 generation 已上传时，全屏 renderer 复用 placement，不重传 base64；重新注册同 ID 会产生新 generation。`deleteAllKittyPlacements` 使用小写 `d=a` 保留已上传数据，真正释放单张图片使用大写 `d=I`。见 [terminal-image.ts:343](../../../../.repo/pi-1.0.2/packages/tui/src/terminal-image.ts#L343)、[terminal-image.ts:416](../../../../.repo/pi-1.0.2/packages/tui/src/terminal-image.ts#L416)、[tui-alt-screen.ts:426](../../../../.repo/pi-1.0.2/packages/tui/src/tui-alt-screen.ts#L426)。

离屏缓存同时限制为 16 张、32 MiB 传输数据与 64 MiB 估算解码数据，超过任一阈值时按最近可见顺序淘汰离屏图片；正在显示的图片不计入这三个离屏上限。decoded 估算为 width×height×4，不能当成实际终端 GPU/heap 内存测量或整个图片系统的硬总预算。图片 metadata 另有 1,000 条上限，PNG 转码 cache 又是独立的 32 条机制。见 [tui-alt-screen.ts:77](../../../../.repo/pi-1.0.2/packages/tui/src/tui-alt-screen.ts#L77)、[tui-alt-screen.ts:447](../../../../.repo/pi-1.0.2/packages/tui/src/tui-alt-screen.ts#L447)、[terminal-image.ts:343](../../../../.repo/pi-1.0.2/packages/tui/src/terminal-image.ts#L343)。

1.0.1 所记录的 WezTerm 修复有两步：即使图片锚点行没变，也检查其覆盖的后续行是否被文字/滚动条改写；需要重绘时先清行、写普通文字，最后画图片 placement，避免后续行清理抹去图片。`getKittyImagePlacementRows` 优先读控制参数中的 `r`，否则用已注册 metadata，因而可按实际裁剪后高度检测。现版测试用 recording terminal 断言文字更新在 placement 前、清 placement 而非重传数据；这不是实际启动 WezTerm 的终端兼容测试。见 [terminal-image.ts:407](../../../../.repo/pi-1.0.2/packages/tui/src/terminal-image.ts#L407)、[tui-alt-screen.ts:1699](../../../../.repo/pi-1.0.2/packages/tui/src/tui-alt-screen.ts#L1699)、[tui-alt-screen.ts:1737](../../../../.repo/pi-1.0.2/packages/tui/src/tui-alt-screen.ts#L1737)、[tui-alt-screen.test.ts:1065](../../../../.repo/pi-1.0.2/packages/tui/test/tui-alt-screen.test.ts#L1065)。

图片输出因此不能当作普通可截断字符串；上传、placement、占位高度、裁剪、删除与缓存代际都参与后续帧。全屏 iTerm2 仍采用文本 fallback，主屏仍可输出 iTerm2 图片，1.0.1 的 Kitty 修复没有扩大这条边界。

## 15 原生平台支持及其权限

原生 helper 使用 Node createRequire 装载 `.node`，按平台和架构选择候选路径；首先考虑已安装 TUI 包的位置，也兼容 bundle 和可执行文件附近路径。成功与失败结果都会缓存。不能假定每个平台都一定有可加载 helper，也不能把查找路径存在视为 native 模块已成功装载。[native-platform.ts:1](../../../../.repo/pi-1.0.2/packages/tui/src/native-platform.ts#L1)、[native-module-path.ts:1](../../../../.repo/pi-1.0.2/packages/tui/src/native-module-path.ts#L1)。

| 平台 | 主要职责 | 限制和回退边界 |
|---|---|---|
| macOS | AppKit 剪贴板与修饰键检测 | 依赖相应预构建 helper；终端协议仍由 TypeScript 层管理 |
| Windows | console VT、修饰键、原生文本及图片剪贴板 | 依赖对应架构 `.node`；真实剪贴板测试需显式环境开关 |
| Linux | X11 剪贴板读取 | 依赖 DISPLAY/libxcb；不等于通用 Wayland 剪贴板接口 |

平台声明与构建方式见 [native/darwin/README.md](../../../../.repo/pi-1.0.2/packages/tui/native/darwin/README.md)、[native/win32/README.md](../../../../.repo/pi-1.0.2/packages/tui/native/win32/README.md)、[native/linux/README.md](../../../../.repo/pi-1.0.2/packages/tui/native/linux/README.md)。本报告阅读了装载与 TypeScript 调用边界及平台文档，未逐行审计所有 C/Objective-C 实现或验证预构建二进制来源。

剪贴板接口区分 unavailable、没有该类型内容和操作失败，不能把它们都折叠成空字符串。OSC52 复制、原生剪贴板和自定义 copy callback 也是三种不同传输路径；是否可读写系统剪贴板取决于终端、平台和调用方选择。[native-platform.ts:10](../../../../.repo/pi-1.0.2/packages/tui/src/native-platform.ts#L10)、[tui-alt-screen.ts](../../../../.repo/pi-1.0.2/packages/tui/src/tui-alt-screen.ts)。

## 16 生命周期、失败及可观察性

终端启动会修改进程 stdin raw mode、编码、paste/keyboard/mouse 协议和屏幕状态。停止需要反向清理；`drainInput` 先关闭增强键盘协议，在最长 1 秒、默认连续 50ms 安静窗口的条件下吸收剩余输入，防止退出后 escape response 污染 shell。[terminal.ts:174](../../../../.repo/pi-1.0.2/packages/tui/src/terminal.ts#L174)、[terminal.ts:391](../../../../.repo/pi-1.0.2/packages/tui/src/terminal.ts#L391)、[terminal.ts:423](../../../../.repo/pi-1.0.2/packages/tui/src/terminal.ts#L423)。

该包没有统一的 error/result 事件总线。不同层选择不同失败策略：可选 native 模块不可用时降级；补全扫描失败时返回空；组件 render 或宽度契约错误可能向上抛出。宿主必须负责启动失败、未捕获异常和进程信号时的最终清理，不能只在正常提交路径调用 stop。

能力缓存、默认 keybindings 和终端协议状态存在进程级共享；ProcessTerminal 直接管理标准输入输出。由此推断，该实现的自然部署单位是“一个进程中的一个主要交互终端”。Component 可重用不等于多个独立 TUI 实例可以无协调地共享同一个 stdin/stdout。

## 17 测试表达的契约

本快照有 42 个匹配 `*.test.*` 或 `*.spec.*` 的测试文件，共 18,719 行；这是文件统计，不是独立断言数。使用 node:test，并通过虚拟 terminal 与 headless xterm 检查实际屏幕内容，而不只比较写出的 escape 字符串。本报告未运行这些测试。

| 测试主题 | 能约束的行为 | 证据 |
|---|---|---|
| 渲染调度及写入分块 | 输入触发及时刷新、正常帧合并、大输出切块 | [tui-render.test.ts:116](../../../../.repo/pi-1.0.2/packages/tui/test/tui-render.test.ts#L116) |
| resize、shrink、滚动与 diff | 清屏/局部更新之后的 viewport | [tui-render.test.ts:509](../../../../.repo/pi-1.0.2/packages/tui/test/tui-render.test.ts#L509) |
| 全屏布局与鼠标 | 固定 dock、嵌套 wheel、overlay 与选择行为 | [tui-alt-screen.test.ts](../../../../.repo/pi-1.0.2/packages/tui/test/tui-alt-screen.test.ts) |
| 非 PNG 转码 | 注入转换、fallback 后 invalidate 重试、共享/实例 cache、PNG/iTerm2 跳过 | [terminal-image.test.ts:829](../../../../.repo/pi-1.0.2/packages/tui/test/terminal-image.test.ts#L829) |
| Kitty 覆盖行重绘 | 锚点不变时更新被覆盖行，重画 placement 的输出顺序 | [tui-alt-screen.test.ts:1065](../../../../.repo/pi-1.0.2/packages/tui/test/tui-alt-screen.test.ts#L1065) |
| Unicode 边界回归 | overlay 切穿 CJK 时列位置仍正确 | [regression-overlay-cjk-boundary.test.ts:6](../../../../.repo/pi-1.0.2/packages/tui/test/regression-overlay-cjk-boundary.test.ts#L6) |
| 字符宽度 | tab、ANSI、emoji、宽字符和裁剪 | [tab-width.test.ts:37](../../../../.repo/pi-1.0.2/packages/tui/test/tab-width.test.ts#L37) |
| 编辑与撤销 | 移动、删除、历史、粘贴与撤销一致性 | [editor.test.ts:2075](../../../../.repo/pi-1.0.2/packages/tui/test/editor.test.ts#L2075) |
| 补全 | 文件匹配、命令、取消及结果适用性 | [autocomplete.test.ts:117](../../../../.repo/pi-1.0.2/packages/tui/test/autocomplete.test.ts#L117) |
| 装载位置 | 打包后仍优先定位 installed TUI native helper | [native-module-path.test.ts:8](../../../../.repo/pi-1.0.2/packages/tui/test/native-module-path.test.ts#L8) |
| 平台调用 | 原生接口、X11 fixture、线程池行为 | [native-platform.test.ts:31](../../../../.repo/pi-1.0.2/packages/tui/test/native-platform.test.ts#L31)、[native-clipboard-linux.test.ts:71](../../../../.repo/pi-1.0.2/packages/tui/test/native-clipboard-linux.test.ts#L71) |

测试的 skip 条件很重要：fd 不存在会跳过部分补全测试，真实 Windows 剪贴板测试需要 `PI_TEST_NATIVE_CLIPBOARD=1`，Linux native 测试依赖系统工具。仅运行 `npm test` 并得到零失败，也不能直接证明这些可选路径全部被执行。

## 18 设计取舍、阅读顺序与验证空白

该包的主要设计收益，是把终端特有复杂性集中在 renderer、输入协议和 Unicode 工具中，让多数业务组件只返回行。代价是字符串同时承载文本、样式、链接、光标 marker 与图片协议，某些渲染契约只能在底层仔细维护；普通 JavaScript 字符串长度无法替代终端列宽。

布局每帧需要测量内容，ScrollView 主要减少最终绘制区域，不能理解为完整的惰性虚拟列表；长 transcript 的 CPU 和内存成本仍取决于组件自身缓存、布局测量和图片缓存。基准脚本提供了观察入口，但本报告没有执行基准，不能据此给出吞吐、输入延迟或长会话容量的实测数字。

推荐阅读顺序：

1. `index.ts`、`tui.ts` 的 Component/Terminal/TUI 接口，确定宿主需要实现什么。
2. `terminal.ts`、`stdin-buffer.ts`、`keys.ts`，理解字节到输入动作。
3. `tui-main-screen.ts`，跟踪首次绘制、内容变化、resize 和 viewport 回退。
4. `layout.ts`、`layout-node.ts`、`components/scroll-view.ts`，理解测量与可见区域。
5. `tui-alt-screen.ts`，理解固定屏幕、选择、搜索和 Kitty 生命周期。
6. `utils.ts`，再看 Editor/Markdown，避免以字符串索引误读视觉坐标。
7. 针对实际目标平台查看 terminal/native 回归测试和 native README。

源文件入口与完整枚举见 [源码索引](../source-index.md#tui)。与业务宿主的集成见 [coding-agent 报告](coding-agent.md)。未解决的验证空白是：真实终端矩阵、字体与 Unicode 版本差异、native 二进制可重现性、长期内存曲线、突发输入和输出背压、特定 SSH/tmux/Windows 组合下的协议降级。它们是静态分析的边界，不是已证明存在的故障。

## 19 设计文档、规划与时效核查（2026-10-05）

### 19.1 文档覆盖与用途

规划来源为根目录的 [tui-plan.md](../../../../.repo/pi-1.0.2/tui-plan.md#L1)。该文件用 1,001 行解释屏幕模式的物理约束、接口取舍、实现顺序、非目标和未来方向；仅阅读包 README 不能替代它。

该计划、947 行 [包 README](../../../../.repo/pi-1.0.2/packages/tui/README.md#L1) 和三个 native README 的完整阅读沿用 1.0.0 研究，本次重算 SHA-256 后确认全文未变；重读计划的 wheel 路由要求、README 图片章节、CHANGELOG 1.0.2/1.0.1/1.0.0 章节，并保留此前对 0.84.0 以来布局、输入、图像条目的筛查。native 文档描述现有构建、平台降级和测试条件，没有独立的后续开发路线图。[Darwin](../../../../.repo/pi-1.0.2/packages/tui/native/darwin/README.md#L1)、[Linux](../../../../.repo/pi-1.0.2/packages/tui/native/linux/README.md#L1)、[Windows](../../../../.repo/pi-1.0.2/packages/tui/native/win32/README.md#L1)。

`tui-plan.md` 应定性为**大部分主干已落实、部分未来项已提前完成、仍有后续方向的历史设计与交接文档**。文件标题中的 Plan 不表示它描述的系统尚未实现。CHANGELOG 已在 0.84.0 记录布局、滚动条与跨屏选择的引入；当前源码也有对应实现。[发布记录](../../../../.repo/pi-1.0.2/packages/tui/CHANGELOG.md)。

### 19.2 规划逐项对照

| 设计或规划原意 | 本快照状态与判断依据 | 应保留的认识 |
|---|---|---|
| alternate screen 拥有固定 viewport，main screen 保留终端 scrollback | **已实现**：两套 renderer 共用 TUI 接口，布局 root 只在 viewport 模式装配；[计划](../../../../.repo/pi-1.0.2/tui-plan.md#L26)、[装配](../../../../.repo/pi-1.0.2/packages/coding-agent/src/modes/interactive/interactive-mode.ts) | 分模式是终端滚动机制决定的，不只是视觉主题开关 |
| `VStack`、`HStack`、`ScrollView` 是最小公共布局 API | **已实现**：根 index 导出组件，内部 `LayoutNode` 未作为根 API 导出；[计划](../../../../.repo/pi-1.0.2/tui-plan.md#L114)、[导出](../../../../.repo/pi-1.0.2/packages/tui/src/index.ts)、[内部协议](../../../../.repo/pi-1.0.2/packages/tui/src/layout-node.ts#L3) | 公共组件契约与内部测量、几何表示分离 |
| 每次请求重建 frame 几何，组件状态与 leaf cache 跨帧保留 | **已实现，细节演进**：`renderLayoutFrame()` 每次新建上下文及 Map；该 Map 只消除同帧同宽度重复 render，不是计划反对的第二套长期缓存；[计划](../../../../.repo/pi-1.0.2/tui-plan.md#L321)、[实现](../../../../.repo/pi-1.0.2/packages/tui/src/layout.ts#L379) | 要区分组件缓存、同帧测量缓存和跨帧几何缓存 |
| basis/grow/shrink/min/max、可见性与整数空间分配 | **已实现**：stack 分配与受约束布局由布局层处理；[计划](../../../../.repo/pi-1.0.2/tui-plan.md#L390)、[实现入口](../../../../.repo/pi-1.0.2/packages/tui/src/layout.ts#L170)、[分配测试](../../../../.repo/pi-1.0.2/packages/tui/test/layout.test.ts#L16) | 这是有限布局系统，不应推导出完整 CSS flexbox 兼容性 |
| transcript 可滚动，编辑器、状态、pending messages、widgets、footer 固定 | **已实现**：`createChatViewport()` 声明 transcript 与 dock，交互层保留一套组件树；[计划](../../../../.repo/pi-1.0.2/tui-plan.md#L743)、[实际组合](../../../../.repo/pi-1.0.2/packages/coding-agent/src/modes/interactive/chat-viewport.ts#L22)、[挂载](../../../../.repo/pi-1.0.2/packages/coding-agent/src/modes/interactive/interactive-mode.ts) | 业务只组合组件，不自行实现屏幕坐标算法 |
| 绘制完成后再提交本帧几何，输入按当前屏幕命中 | **已实现**：`terminal.write(buffer)` 之后更新 `previousScreen` 与 `currentLayout`；[计划](../../../../.repo/pi-1.0.2/tui-plan.md#L343)、[提交点](../../../../.repo/pi-1.0.2/packages/tui/src/tui-alt-screen.ts) | 输入与绘制必须指向同一已提交 frame |
| wheel 按位置路由、嵌套滚动传递剩余量、键盘可独立操作 | **主要机制已接线，contain 有静态偏差**：命中非 primary 的 contain 滚动区且剩余 delta 非零时，循环虽停止，随后仍可能交给未访问的 primary；[计划](../../../../.repo/pi-1.0.2/tui-plan.md#L524)、[实际路由](../../../../.repo/pi-1.0.2/packages/tui/src/tui-alt-screen.ts)、[primary 操作](../../../../.repo/pi-1.0.2/packages/tui/src/tui-alt-screen.ts) | chain 能力存在，不等于完整兑现 contain 停止传播的要求；本次未运行复现 |
| 没有显式 layout root 时，保留旧 children API 的隐式滚动容器 | **已实现**：实际 root 为显式 root 或 implicit ScrollView；[计划](../../../../.repo/pi-1.0.2/tui-plan.md#L664)、[选择点](../../../../.repo/pi-1.0.2/packages/tui/src/tui-alt-screen.ts) | 新布局没有要求所有旧组件先改成另一套接口 |
| 正常退出时输出完整文档，而非最后一屏 | **已实现，有显式例外**：普通 stop 重新执行无高度限制的 `render(width)`；`preserveScreen` 走另一分支；[计划](../../../../.repo/pi-1.0.2/tui-plan.md#L677)、[实现](../../../../.repo/pi-1.0.2/packages/tui/src/tui-alt-screen.ts) | 最后一帧与最终文档是不同产物 |
| 可见滚动条先预留，未来再做 | **旧延期描述已过时**：`auto/always/hidden`、比例 thumb、hover、拖动及 track click 已实现；[旧预留注释](../../../../.repo/pi-1.0.2/tui-plan.md#L205)、[状态与配置](../../../../.repo/pi-1.0.2/packages/tui/src/components/scroll-view.ts#L1)、[交互](../../../../.repo/pi-1.0.2/packages/tui/src/tui-alt-screen.ts)、[后续样式变更](../../../../.repo/pi-1.0.2/packages/tui/CHANGELOG.md) | 不应继续列为待开发功能 |
| 首轮选择只保证可见屏幕文本 | **原最小范围已被扩展**：已有拖动到边缘自动滚动并选择屏外内容；[原范围](../../../../.repo/pi-1.0.2/tui-plan.md#L577)、[选择实现](../../../../.repo/pi-1.0.2/packages/tui/src/tui-alt-screen.ts)、[发布记录](../../../../.repo/pi-1.0.2/packages/tui/CHANGELOG.md) | 原验收下限不能当作当前能力上限 |
| Kitty 图片支持 viewport 垂直裁剪，iTerm2 在 alt 模式降级 | **主要路径已实现**：layout 对 Kitty 上下边界裁剪，启动时禁用 iTerm2 图片能力；[计划](../../../../.repo/pi-1.0.2/tui-plan.md#L599)、[裁剪](../../../../.repo/pi-1.0.2/packages/tui/src/layout.ts#L330)、[降级](../../../../.repo/pi-1.0.2/packages/tui/src/tui-alt-screen.ts)、[测试](../../../../.repo/pi-1.0.2/packages/tui/test/layout.test.ts#L130) | 计划另要求受限横向图片情形明确降级；本次证据不足以宣布所有图片验收项满足 |
| overlay 沿用既有 screen-relative 合成，以后再拥有独立受约束树 | **第一阶段已实现，后半句仍是未落地的设计方向**：overlay 依然 `component.render(width)` 后裁剪、定位、合成，没有逐 overlay 调用 `renderLayoutFrame`；[计划](../../../../.repo/pi-1.0.2/tui-plan.md#L618)、[合成实现](../../../../.repo/pi-1.0.2/packages/tui/src/tui.ts#L1339) | 有 overlay bounds 和鼠标命中，不等于 overlay 内部已接入完整受约束布局 |

contain 偏差可以具体定位：计划要求遇到 contain 即停止，即使仍有余量；现版 `routeWheel` 第 992 行只 break，第 995 行仍可能执行 primary fallback，`ScrollView.scrollBy` 也不会主动把余量清零。若外层 primary 尚有可滚动空间，它仍可能移动；现有嵌套 wheel 用例验证的是默认 chain。因此这项应保留为规划与实现的具体差距，不能仅凭“存在 overscroll 字段”宣称完成。[原要求](../../../../.repo/pi-1.0.2/tui-plan.md#L531)、[余量返回](../../../../.repo/pi-1.0.2/packages/tui/src/components/scroll-view.ts#L148)、[现版 routeWheel:986](../../../../.repo/pi-1.0.2/packages/tui/src/tui-alt-screen.ts#L986)、[现有 chain 测试:532](../../../../.repo/pi-1.0.2/packages/tui/test/tui-alt-screen.test.ts#L532)。

表中的“已实现”指静态实现证据，不包含测试执行证明。布局、renderer 和宿主集成测试都保留在仓库中；计划末尾要求运行的命令也是验收说明，不能当作已经通过的结果。[计划测试矩阵](../../../../.repo/pi-1.0.2/tui-plan.md#L839)、[验收标准](../../../../.repo/pi-1.0.2/tui-plan.md#L985)。

### 图片文档的现版遗漏与修复状态

README 第 682 行仍直接列 PNG、JPEG、GIF、WebP 支持，全文没有 `setImageTranscoder` 的注册说明。应拆成两层理解：这四类头部尺寸可解析，但 Kitty 的非 PNG 显示现在明确需要宿主注册同步转码器，否则是文本 fallback；文档的无条件措辞遗漏了新前提。iTerm2 的主屏路径无此 PNG 转换要求，全屏降级说明仍有效。见 [README.md:658](../../../../.repo/pi-1.0.2/packages/tui/README.md#L658)、[CHANGELOG.md:9](../../../../.repo/pi-1.0.2/packages/tui/CHANGELOG.md#L9)。

WezTerm 覆盖行重绘已在实现与回归断言中兑现，不能继续作为未实现修复；contain 偏差则经过当前源码重读仍存在。现版转码测试使用 stub JPEG 字符串与最小 PNG 头验证契约，不直接证明真实 JPEG/GIF/WebP 解码、动画帧选择或所有终端中的视觉效果。

### 19.3 尚有价值的后续方向与明确非目标

| 文档方向 | 当前边界 | 时效判断 |
|---|---|---|
| 宽终端 sidebar、独立 transcript/sidebar 滚动、sticky top | `HStack` 与独立 ScrollView 已提供布局基础；当前 chat viewport 仍是 transcript + 底部 dock | **基础机制已有，具体产品组合不是现成能力清单**；不能将示意用途全列为缺失 |
| 多个键盘可选择的 active scroll region | 当前 renderer 的通用滚动操作仍取 primary；计划将 active pane 和新增 keybinding 留到有多窗格需求时 | **明确留待后续的交互策略**，非仅补几个快捷键；[计划](../../../../.repo/pi-1.0.2/tui-plan.md#L548)、[primary 选择](../../../../.repo/pi-1.0.2/packages/tui/src/tui-alt-screen.ts) |
| unread-line indicator | 已有偏离底部时显示的可点击 jump-to-end indicator；未见独立未读行计数状态契约 | **相邻能力已实现，原措辞的完整语义未获证实**；[规划](../../../../.repo/pi-1.0.2/tui-plan.md#L92)、[现有 indicator 记录](../../../../.repo/pi-1.0.2/packages/tui/CHANGELOG.md) |
| transcript virtualization | ScrollView 仍渲染内容并保留完整 `scrollContentLines`；绘制阶段裁剪行数并不免除完整内容测量 | **快照内尚未实现的明确方向**；[规划](../../../../.repo/pi-1.0.2/tui-plan.md#L99)、[实现](../../../../.repo/pi-1.0.2/packages/tui/src/layout.ts#L136) |
| overlay 内独立 constrained layout tree | 现有 overlay 合成仍基于行数组 | **仍有差距的后续设计方向**，见上表 |

“非目标”不能直接转成待办。计划明确首轮不做完整 CSS flexbox、grid、wrapped rows、任意 absolute layout、公开内部布局节点、增量修改布局树，以及为了布局重写所有组件的 height API；这些决定保留了现有 `render(width)` 模型，也压低了公共接口复杂度。[非目标](../../../../.repo/pi-1.0.2/tui-plan.md#L101)。

阅读这一计划最有价值的部分是：为什么主屏不能简单模拟固定 dock；为什么组件状态长期存活而 frame 几何短期存在；为什么输入、图片、选择和退出输出都必须共享布局信息。以上理由在实现完成后依然有效。只有预留状态、草拟接口和实现顺序需要结合当前 README、源码与 CHANGELOG 解读，不能把整份文档视为失效，也不能据它承诺未来版本的发布时间。

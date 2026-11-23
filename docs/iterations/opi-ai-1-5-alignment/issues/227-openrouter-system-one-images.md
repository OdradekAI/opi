# #227 让 OpenRouter System One 接受目录声明的图片输入

Status: done

OpenRouter 聚合目录里的分类模型通过 `Models::classify` 进入 `typesafe-system-one`。旧适配器拒绝所有图片，导致此前 12 个具备图片声明的模型无法按声明调用。

## 协议与归属

OpenRouter 适配器将图片按输入顺序编码成 `image_url` data URL，并将原始 JSON state 作为最后一个成员保留。图片格式由 OpenRouter provider ID 或该服务实际 URL 的主机名识别，因而配置代理也能保留正确的格式。分类合同依据模型的 `input` 判定是否接受图片。生成器在 OpenRouter 分类路由上校验 `imageInput: "state-image-urls"` 和 `sendImageOptInHeader: false`，两项描述已实现的固定协议。

参考固定版 Pi 源码 `.pi/pi-1.5.0/packages/ai/src/api/{typesafe-system-one,system-one-shared}.ts` 与同快照的 `test/typesafe-system-one.test.ts`。接口路径和 state 图片成员格式另与 OpenRouter 文档核对。[OpenRouter System One API](https://openrouter.ai/docs/api/api-reference/systemone/submit-a-system-one-request)

## 独立验收

- 从真实 builtin 装配调用 `typesafe/jev-1.14`。本地服务器收到带两张图片的 `POST /api/v1/systemone`，验证 Bearer 认证、图片顺序、原始数组的嵌套、`noul` 请求、返回概率和 usage。请求无 `assets`、顶层 `images` 或 TypeSafe 图片 opt-in 头。
- 用同一入口对文本模型 `respan/span-01` 传入图片，得到 `UnsupportedCapability`，本地服务器只收到前一条请求。已有 TypeSafe、Cloudflare 及其它分类器用例继续验证各自的行为。
- `build_catalog` 接受匹配的作者声明并拒绝 `assets` 配置和打开 opt-in 的配置。原生 TypeSafe 的 asset 编码、Cloudflare 图片输入及自定义 provider 的逐模型兼容配置仍是独立的协议能力。

本地回归先复现错误，再通过 `provider_collection` 验证实际请求。分类合同的 84 项用例以及选测回归通过。[验收回执](../evidence/issue-227-228-openrouter-catalog.json)

Test impact — add builtin 图片分类、文本模型拒绝和作者校验用例。update System One 源码的选测映射。retain 分类协议、重试、认证和遥测回归。

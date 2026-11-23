# #242 同步 TypeSafe 目录与 System One 图片路由

Status: done

## 范围与固定基线

本票接续 [#241 Baseten](241-sync-baseten-catalog.md)，只处理
`provider=typesafe` 的 SDK 分类器入口。TypeSafe 使用现有 System One 适配器和
`https://api.typesafe.ai/v1/`，不新增独立协议适配器。coding-agent 没有 TypeSafe
内置注册，本票不扩展其产品范围。下线模型按固定发布目录核对，发布目录没有独立的
下线 ID。

三方同步使用当前工作树的 Pi 1.3 发布 bundle
`sha256-f7d9c639cd3af8ac9b561f60cfd74f7ccb921d7563bf3ee7948fb7f31b1ce8f`、固定的
Pi 1.5 发布文件
`sha256-f4dbabe79cda4b38aee4a761b23fd5eaf526ac2e28fed3c9ee86e37ca0e5d71e`，以及
`crates/opi-ai/resources/catalog/providers/typesafe.json`。

| 操作 | 旧目录 → Pi 1.5 | 新增 | 撤下 | 更新 |
| --- | ---: | ---: | ---: | ---: |
| TypeSafe System One 分类器 | 1 → 1 | 0 | 0 | 2 |

两项更新为 `jev-latest` 增加 `image` 输入声明和
`compat.imageInput=assets`、`compat.sendImageOptInHeader=true`。

## 三项验收条件

1. **目录和 SDK 入口一致。** 作者源、bundle 和固定 Pi 1.5 文件逐条同序相等。
   通过 `Catalog::builtin`、注册后的 `Models` 分类列表、可用列表、按 ID 查询和
   完整分类器记录核对 `jev-latest` 的图片能力与兼容声明。`build_catalog check`
   通过。
2. **实际图片请求与 Pi 路由一致。** 通过内置 `Models` 只把 TypeSafe origin 替换
   为本地 HTTP 服务，保留 `/v1/systemone`。带 PNG 图片的请求必须使用 Bearer
   认证，发送 `TypeSafe-Danger-Zone-Images: true`，在 `state` 前放置命名图片资产，
   并在 `assets` 中发送 `type=image`、`format` 和 base64 数据。纯文本请求保留原始
   state，不发送 assets 和 opt-in 头，公开 bool 问题仍编码为 `noul`。
3. **入口集合与下线状态收敛。** 固定 Pi 1.5 目录只声明 `jev-latest`，所以内置
   catalog、分类列表、可用列表和按 ID 查询均只暴露该记录。一个未知／旧 ID 在 HTTP
   发出前返回 `NotAClassifierModel`，回环服务收到零个请求。当前发布没有待撤下的
   TypeSafe ID。

## 实施结果

- TypeSafe 作者源 SHA-256 为
  `b4f197e6a2429ce13c81bae202ed0a05b4bcefffcef7216fa885b43f6ea8c67c`。全库 bundle
  SHA-256 为 `01490d4c04afc44845beb96679cd79c369dc4c2076358d989246929d66af8e0a`，
  manifest SHA-256 为 `e3a66771f77371b4bc06dcd6de0bb387e8fcf425aa6e1af4ef015dd6940969d4`。
- `ClassifierModelRecord` 现在保留分类器的图片传输兼容声明。TypeSafe 直连按目录
  使用 assets 和 opt-in 头，OpenRouter 继续使用目录中的 state image URL 声明。
  共享 System One core 负责按记录选择图片编码并在有图片时注入 opt-in 头。
- 固定 Pi 源文件用于行为核对的 SHA-256 如下。

  - `.pi/pi-1.5.0/packages/ai/catalog/providers/typesafe.ts`：
    `a4d269944c72c74c4deb310f06db4c58036903be1b04c5776bed221a3c437c21`
  - `.pi/pi-1.5.0/packages/ai/src/providers/typesafe.ts`：
    `24e2b1f19118486e0064e252c3860eba41fade01a2e50147bf7406d00c6f4ba8`
  - `.pi/pi-1.5.0/packages/ai/src/api/typesafe-system-one.ts`：
    `3c73e0877e7f44a9ae4a25f481e1e752f585e4d3f8078464a68cdb093d30a965`
  - `.pi/pi-1.5.0/packages/ai/src/api/system-one-shared.ts`：
    `4be07d266a37a2906adb8d6776a3e4fff4c100a199eb677e3b500b57fef7400f`

## 测试回执

- `cargo test -p opi-ai --offline --locked --test provider_collection --features builtins typesafe`：3 项通过。
- `cargo test -p opi-ai --offline --locked --test classifier_contract --features builtins typesafe_system_one`：23 项通过。
- `cargo test -p opi-ai --offline --locked --test catalog_operations --features builtins`：27 项通过。
- `cargo test -p opi-ai --offline --locked --no-default-features --example build_catalog`：9 项通过。
- `build_catalog check`、`cargo fmt --all` 和目录作者校验通过。
- `run_catalog_bundled_package.py`：从实际归档包构建并运行 `catalog-bundled` 外部消费者通过。

Test impact — `add` 三个 TypeSafe 目录、请求和入口集合回归；`update` 分类器模型
记录与 System One 图片编码测试；`retain` OpenRouter System One 图片和共享分类器
回归；`none` coding-agent 入口未变更。

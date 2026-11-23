# 接入供应商

[English](provider-auth.md) · [SDK 快速开始](../README.zh.md#快速开始)

## 选择供应商和凭据来源

用 `Models::provider_ids()` 查看已注册的供应商 ID，用
`get_models(Some(id))` 查看该供应商的聊天模型。
`opi_ai::providers::builtin_profile(id)` 可以查询内置供应商的默认端点、
常用的 `api_key_env` 环境变量和 OAuth 支持情况。使用表中定义的供应商 ID，
例如 `google-vertex` 或 `amazon-bedrock`。

| 供应商 | 凭据与端点配置 |
| --- | --- |
| `anthropic` | SDK 依次读取 `ANTHROPIC_AUTH_TOKEN`（Bearer）、`ANTHROPIC_OAUTH_TOKEN`（API key）和 `ANTHROPIC_API_KEY`。使用工作负载身份联合时，设置 `ANTHROPIC_FEDERATION_RULE_ID`、`ANTHROPIC_ORGANIZATION_ID` 和 `ANTHROPIC_IDENTITY_TOKEN_FILE`。 |
| `openai` | 设置 `OPENAI_API_KEY`，或用已保存的 ChatGPT OAuth 凭据访问 Responses 聊天。Decisions 分类需要 API key。 |
| `openai-codex` | 通过 `Models::login` 使用旧版 Codex OAuth 流程。 |
| `github-copilot` | 使用 `Models::login` 或设置 `COPILOT_GITHUB_TOKEN`。OAuth 凭据可以提供账号端点和允许使用的模型 ID。 |
| `azure` | 设置 `AZURE_OPENAI_API_KEY`，再设置 `AZURE_OPENAI_BASE_URL` 或 `AZURE_OPENAI_RESOURCE_NAME`。模型与部署名称不同时，用 `AZURE_OPENAI_DEPLOYMENT_NAME_MAP` 设置逗号分隔的 `model-id=deployment`。也可设置 `AZURE_OPENAI_API_VERSION`。 |
| `amazon-bedrock` | 使用 AWS profile、标准 AWS 凭据链或 `AWS_BEARER_TOKEN_BEDROCK`。在 AWS 配置或 `AWS_REGION` 中设置区域，并选择该区域可用的模型 ID 或推理配置。 |
| `google-vertex` | Vertex AI Express 使用 `GOOGLE_CLOUD_API_KEY`。使用 Application Default Credentials 时，设置 `GOOGLE_CLOUD_PROJECT`（或 `GCLOUD_PROJECT`）和 `GOOGLE_CLOUD_LOCATION`。 |
| `cloudflare-workers-ai` 与 `cloudflare-ai-gateway` | 设置 `CLOUDFLARE_API_KEY` 和 `CLOUDFLARE_ACCOUNT_ID`。使用 Gateway 时还需设置 `CLOUDFLARE_GATEWAY_ID`。 |
| `radius` | 设置 `RADIUS_API_KEY` 或调用 `Models::login`。刷新方式见 [Radius 网关目录](#radius-网关目录)。 |

`register_builtin` 使用应用传入的 `AuthContext` 和 `HttpClient`。
`AmbientAuthContext` 在调用时读取进程环境。注入自定义实现时，在注册前用
`Models::set_auth_context` 设置同一个 Context。要修改端点，可按供应商 ID
配置 `BuiltinOverrides::base_urls`，或使用 `BuiltinOverrides` 上的 Azure、
Vertex 和 AWS 专用字段。要添加新供应商 ID 或本地模型，可[接入兼容端点](providers.zh.md#接入兼容端点)。

## 检查、保存和解析凭据

`Models::new` 默认使用内存 `CredentialStore`。对支持 API key 或 OAuth 的内置
供应商，已保存的凭据优先于环境凭据。`ApiKeyCredential` 也可通过该供应商的
`AuthContext` 提供作用域内的配置值。

- 用 `check_auth(id).await` 检查配置而不刷新凭据。`Ok(None)` 表示缺少供应商
  或配置。来源访问失败会返回错误。过期的 OAuth 凭据仍可能被判定为已配置。
- 用 `get_available(Some(id)).await` 筛选聊天模型，或用
  `get_available_of_type` 筛选某一类操作。筛选会考虑已保存的账号限制。
  对聊天操作，空的模型 ID 列表会隐藏该账号的全部聊天模型。最终访问权限由远端服务决定。
- 用 `get_auth(id, overrides).await` 解析凭据。未配置时返回 `Ok(None)`，
  OAuth resolver 可能会刷新令牌。
  要显式传入凭据，在 `AuthResolutionOverrides::api_key`、`bearer`、
  `aws_sigv4` 中选一个。已注册类型化聊天 adapter 时，可把返回的 `ResolvedAuth`
  传给 `Models::stream_direct` 或 `complete_direct`。应用自定义请求头使用 `extra_headers`。
- 用 `set_credential_store` 接入共享或持久化存储。`Models::login(id, presenter)`
  通过你提供的 `LoginPresenter` 完成登录交互，等待写入确认后返回。
  `logout(store, id)` 从传入的 store 删除本地凭据。如需撤销远端凭据，请通过供应商处理。

放弃等待已提交的写入时，写入仍可能继续。需要确认写入结束时，等待 mutation 的
返回结果，或使用存储的 `mutation_barrier`。

## 离线内置 Model Catalog

启用 `catalog-bundled` 后，调用 `Catalog::builtin()` 可直接读取包内目录。
`Catalog::model` 接受供应商 ID、操作和模型 ID，因为聊天和图像目录可能包含
同名模型。要查看当前 `Models` 已注册和已恢复的目录，调用 `Models::get_models_of_type`。

`Catalog::from_bytes` 可读取当前目录格式和旧的 v1 归档。来源见
`CatalogProvenance::origin`，版本标识见 `CatalogProvenance::revision`。
[opi 仓库中的目录维护说明](../resources/catalog/README.zh.md)

## 刷新动态目录

用 `ModelsStore::open(cache_path)?` 打开缓存，并用 `models.set_models_store(store)`
接入。应用显式调用 `Models::refresh` 更新目录。`allow_network: false` 只恢复缓存，
无需解析凭据。默认允许为所有已注册供应商访问网络。用 `RefreshReport::status()`
查询本轮状态，用 `entries()` 查看各供应商是否失败。刷新失败会保留最近可用的目录，
成功返回空目录则会清除旧条目。

### Radius 网关目录

默认 Radius 网关从包内目录开始。自定义网关初始目录为空，只恢复同源的缓存。
注册供应商后，可以用下面的函数只刷新 Radius。启动时传 `false` 恢复缓存。
用户发起联网刷新时，传 `true`，SDK 会使用已配置的凭据读取网关目录。

```rust
use opi_ai::{Models, ModelsError, ModelsRefreshOptions, RefreshReport};

pub async fn refresh_radius(
    models: &mut Models,
    allow_network: bool,
) -> Result<RefreshReport, ModelsError> {
    models
        .refresh(ModelsRefreshOptions {
            providers: Some(["radius".to_owned()].into_iter().collect()),
            allow_network,
            ..Default::default()
        })
        .await
}
```

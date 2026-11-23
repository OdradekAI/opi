# Connect a provider

[简体中文](provider-auth.zh.md) · [SDK quick start](../README.md#quick-start)

## Choose a provider and credential source

In an existing `Models` collection, use `provider_ids()` to find registered
provider IDs and `get_models(Some(id))` to list that provider's chat models.
For the built-ins, `opi_ai::providers::builtin_profile(id)` exposes the
default endpoint, the usual `api_key_env` name, and whether OAuth is
supported. Use the provider ID from that profile, such as `google-vertex`
or `amazon-bedrock`.

| Provider | How to configure credentials and the endpoint |
| --- | --- |
| `anthropic` | Set `ANTHROPIC_AUTH_TOKEN` (Bearer), `ANTHROPIC_OAUTH_TOKEN` (API key), or `ANTHROPIC_API_KEY`. The SDK tries those variables in that order. For workload identity, supply `ANTHROPIC_FEDERATION_RULE_ID`, `ANTHROPIC_ORGANIZATION_ID` and `ANTHROPIC_IDENTITY_TOKEN_FILE`. |
| `openai` | Set `OPENAI_API_KEY` or use a stored ChatGPT OAuth credential for Responses chat. The Decisions classifier requires an API key. |
| `openai-codex` | Use the legacy Codex OAuth flow with `Models::login`. |
| `github-copilot` | Use `Models::login` or set `COPILOT_GITHUB_TOKEN`. A stored OAuth credential can supply the serving endpoint and allowed model IDs. |
| `azure` | Set `AZURE_OPENAI_API_KEY` and `AZURE_OPENAI_BASE_URL` or `AZURE_OPENAI_RESOURCE_NAME`. Set `AZURE_OPENAI_DEPLOYMENT_NAME_MAP` to comma-separated `model-id=deployment` pairs when the names differ. `AZURE_OPENAI_API_VERSION` is optional. |
| `amazon-bedrock` | Use an AWS profile, the standard AWS credential chain, or `AWS_BEARER_TOKEN_BEDROCK`. Select a region through the AWS configuration or `AWS_REGION`. Choose a model ID or inference profile available in that region. |
| `google-vertex` | Set `GOOGLE_CLOUD_API_KEY` for Vertex AI Express. For Application Default Credentials, set `GOOGLE_CLOUD_PROJECT` (or `GCLOUD_PROJECT`) and `GOOGLE_CLOUD_LOCATION`. |
| `cloudflare-workers-ai` and `cloudflare-ai-gateway` | Set `CLOUDFLARE_API_KEY` and `CLOUDFLARE_ACCOUNT_ID`. For the gateway, also set `CLOUDFLARE_GATEWAY_ID`. |
| `radius` | Set `RADIUS_API_KEY` or use `Models::login`. See [Radius gateway catalog](#radius-gateway-catalog) for refresh behavior. |

`register_builtin` takes the `AuthContext` and `HttpClient` your application
chooses. Use `AmbientAuthContext` to read from the process at call time.
If you supply a custom context, also call `Models::set_auth_context` with it
before registration. To change an endpoint, set
`BuiltinOverrides::base_urls` by provider ID or use the dedicated Azure,
Vertex and AWS fields on `BuiltinOverrides`. For a new provider ID or local
endpoint, use [a compatible endpoint](providers.md#connect-a-compatible-endpoint).

## Check, store, and resolve credentials

`Models::new` starts with an in-memory `CredentialStore`. For the built-in
key/OAuth providers, a stored credential takes precedence over environment
credentials. Scoped values on `ApiKeyCredential` can supply provider-specific
configuration through that provider's `AuthContext`.

- Call `check_auth(id).await` to check the configured source without refreshing
  it. `Ok(None)` means the provider or its configuration is missing. A source
  failure is returned as an error. An expired OAuth token can still count as
  configured.
- Call `get_available(Some(id)).await` for chat, or `get_available_of_type`
  for one operation. These methods check configuration and account access
  metadata. For chat, a stored empty list of model IDs hides all chat models
  for that account. The provider makes the final access decision.
- Call `get_auth(id, overrides).await` to resolve a credential. An unconfigured
  source returns `Ok(None)`. A live OAuth resolver can refresh it. For an
  explicit source, set exactly one of `AuthResolutionOverrides::api_key`,
  `bearer`, or `aws_sigv4`. For a typed
  chat adapter, pass the returned `ResolvedAuth` to `Models::stream_direct`
  or `complete_direct`. Use `extra_headers` for application headers.
- Call `set_credential_store` to supply your own shared or persistent store.
  `Models::login(id, presenter)` uses your `LoginPresenter` for the interaction
  and waits for the credential write. `logout(store, id)` deletes the local
  entry in the supplied store. Revoke remote tokens with the provider.

If you drop the waiter for a submitted write, the store can continue the
write. Wait for its result or use the store's `mutation_barrier` to observe
settlement before runtime shutdown.

## Offline bundled model catalog

Use `Catalog::builtin()` with `catalog-bundled` to read the packaged model data
without constructing a client. `Catalog::model` takes a provider, operation
and bare ID because the same ID can appear in both chat and image catalogs.
Use `Models::get_models_of_type` to see what the current collection has
registered or restored.

`Catalog::from_bytes` reads both the current bundle and legacy v1 bundles.
Use `CatalogProvenance::origin` and `revision` to identify the bundle.
[Catalog maintenance in the opi repository](../resources/catalog/README.md)

## Refresh a dynamic catalog

Attach `ModelsStore::open(cache_path)?` via `models.set_models_store(store)`
to persist last-known catalogs. Call `Models::refresh` explicitly. Use
`allow_network: false` to restore cached records without resolving credentials.
The default allows network work for all active providers. Inspect
`RefreshReport::status()` for the round and `entries()` for per-provider
failures. A failed refresh retains the last usable catalog. An authoritative
empty update removes the previous entries.

### Radius gateway catalog

The default Radius gateway starts with bundled models. A custom gateway
starts empty and restores a cache only for the same origin. After registration,
the following function refreshes only Radius. At startup, pass `false` for
offline restore. For a user-requested network refresh, pass `true` to fetch
the gateway configuration with its configured credentials.

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

//! Provider collection and built-in assembly test target.

#[path = "provider_collection/auth_resolution.rs"]
mod auth_resolution;
#[path = "provider_collection/azure.rs"]
mod azure;
#[path = "provider_collection/baseten.rs"]
mod baseten;
#[path = "provider_collection/builtin_auth.rs"]
mod builtin_auth;
#[path = "provider_collection/builtin_chat.rs"]
mod builtin_chat;
#[path = "provider_collection/builtin_operations.rs"]
mod builtin_operations;
#[path = "provider_collection/builtin_registration.rs"]
mod builtin_registration;
#[path = "support/catalog_store.rs"]
mod catalog_fixture;
#[path = "provider_collection/catalog_store_refresh_20_33.rs"]
mod catalog_store_refresh_20_33;
#[path = "provider_collection/cerebras.rs"]
mod cerebras;
#[path = "provider_collection/cloudflare_ai_gateway.rs"]
mod cloudflare_ai_gateway;
#[path = "provider_collection/cloudflare_workers_ai.rs"]
mod cloudflare_workers_ai;
#[path = "provider_collection/dispatch.rs"]
mod dispatch;
#[path = "provider_collection/fireworks.rs"]
mod fireworks;
#[path = "provider_collection/fixtures.rs"]
mod fixtures;
#[path = "provider_collection/groq.rs"]
mod groq;
#[path = "provider_collection/helper_retry_hints.rs"]
mod helper_retry_hints;
#[path = "provider_collection/huggingface.rs"]
mod huggingface;
#[path = "provider_collection/kimi_coding.rs"]
mod kimi_coding;
#[path = "provider_collection/minimax.rs"]
mod minimax;
#[path = "provider_collection/minimax_cn.rs"]
mod minimax_cn;
#[path = "provider_collection/moonshotai.rs"]
mod moonshotai;
#[path = "provider_collection/moonshotai_cn.rs"]
mod moonshotai_cn;
#[path = "provider_collection/nvidia.rs"]
mod nvidia;
#[path = "provider_collection/opencode.rs"]
mod opencode;
#[path = "provider_collection/phase17.rs"]
mod phase17;
#[path = "provider_collection/qwen_token_plan.rs"]
mod qwen_token_plan;
#[path = "provider_collection/qwen_token_plan_cn.rs"]
mod qwen_token_plan_cn;
#[path = "provider_collection/radius.rs"]
mod radius;
#[path = "provider_collection/registry_preflight_and_refresh.rs"]
mod registry_preflight_and_refresh;
#[path = "provider_collection/selected_owner_management.rs"]
mod selected_owner_management;
#[path = "provider_collection/together.rs"]
mod together;
#[path = "provider_collection/typesafe.rs"]
mod typesafe;
#[path = "provider_collection/vercel.rs"]
mod vercel;
#[path = "provider_collection/xai.rs"]
mod xai;
#[path = "provider_collection/xiaomi.rs"]
mod xiaomi;
#[path = "provider_collection/xiaomi_token_plan_ams.rs"]
mod xiaomi_token_plan_ams;
#[path = "provider_collection/xiaomi_token_plan_cn.rs"]
mod xiaomi_token_plan_cn;
#[path = "provider_collection/xiaomi_token_plan_sgp.rs"]
mod xiaomi_token_plan_sgp;
#[path = "provider_collection/zai_coding_cn.rs"]
mod zai_coding_cn;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use opi_ai::auth::{
    AuthProvenanceSource, AuthResolver, AuthScheme, ResolvedAuth, StaticAuthResolver,
};
use opi_ai::catalog::{Catalog, CatalogOperation};
use opi_ai::credential::{BoxAuthFuture, Credential, CredentialSource, CredentialStore};
use opi_ai::message::{AssistantContent, AssistantMessage};
use opi_ai::models::{
    AuthCheckKind, AuthDescriptor, CompatMetadata, Models, ModelsError, SecretKey,
    drain_to_completion,
};
use opi_ai::provider::{Provider, ProviderError, Request, ThinkingConfig};
use opi_ai::providers::collection::{BuiltinEnvAuthResolver, BuiltinOverrides, register_builtin};
use opi_ai::providers::{BuiltinEndpoint, builtin_profile, builtin_provider_profiles};
use opi_ai::registry::ProviderRegistry;
use opi_ai::test_support::{MockProvider, text_response};
use opi_ai::{ModelCapabilities, ModelInfo, WireApi};
use tokio_util::sync::CancellationToken;

use fixtures::*;

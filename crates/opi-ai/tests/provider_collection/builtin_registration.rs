//! Built-in composition, provider metadata and catalog projection.

use super::*;

/// The exact canonical builtin provider set, sorted (pi `builtinProviders`).
const ALL_42_CANONICAL: [&str; 42] = [
    "amazon-bedrock",
    "ant-ling",
    "anthropic",
    "azure",
    "baseten",
    "cerebras",
    "cloudflare-ai-gateway",
    "cloudflare-workers-ai",
    "deepseek",
    "fireworks",
    "github-copilot",
    "google",
    "google-vertex",
    "groq",
    "huggingface",
    "kimi-coding",
    "meta",
    "minimax",
    "minimax-cn",
    "mistral",
    "moonshotai",
    "moonshotai-cn",
    "nvidia",
    "openai",
    "openai-codex",
    "opencode",
    "opencode-go",
    "openrouter",
    "qwen-token-plan",
    "qwen-token-plan-cn",
    "qwen-token-plan-individual",
    "radius",
    "together",
    "typesafe",
    "vercel-ai-gateway",
    "xai",
    "xiaomi",
    "xiaomi-token-plan-ams",
    "xiaomi-token-plan-cn",
    "xiaomi-token-plan-sgp",
    "zai",
    "zai-coding-cn",
];

#[test]
fn builtin_profiles_cover_exactly_the_42_canonical_providers() {
    let profiles = builtin_provider_profiles();
    assert_eq!(profiles.len(), 42, "the pinned capability inventory count");
    let ids: Vec<&str> = profiles.iter().map(|profile| profile.id).collect();
    assert_eq!(ids, ALL_42_CANONICAL.to_vec(), "table is sorted canonical");
    // No permanent legacy aliases.
    for legacy in ["gemini", "vertex", "bedrock", "openai-responses"] {
        assert!(builtin_profile(legacy).is_none(), "{legacy} must not exist");
    }
}

#[test]
fn builtin_profiles_pin_source_qualified_metadata() {
    assert_eq!(
        builtin_profile("amazon-bedrock").unwrap().name,
        "Amazon Bedrock"
    );
    assert_eq!(builtin_profile("anthropic").unwrap().name, "Anthropic");
    assert_eq!(
        builtin_profile("google-vertex").unwrap().name,
        "Google Vertex AI"
    );
    let openai = builtin_profile("openai").expect("openai profile");
    assert_eq!(openai.name, "OpenAI");
    assert_eq!(openai.base_url, Some("https://api.openai.com/v1"));
    assert_eq!(openai.api_key_env, Some("OPENAI_API_KEY"));
    assert_eq!(openai.auth_scheme, AuthScheme::Bearer);
    assert!(openai.oauth, "ChatGPT subscription OAuth exists");
    assert!(
        openai.hides_classifiers_on_oauth,
        "ChatGPT tokens never reach the Decisions API"
    );

    let google = builtin_profile("google").expect("google profile");
    assert_eq!(
        google.base_url,
        Some("https://generativelanguage.googleapis.com/v1beta")
    );
    assert_eq!(google.api_key_env, Some("GEMINI_API_KEY"));
    assert_eq!(google.auth_scheme, AuthScheme::ApiKey);

    let azure = builtin_profile("azure").expect("azure profile");
    assert_eq!(azure.base_url, None, "the endpoint is deployment-derived");
    assert_eq!(azure.api_key_env, Some("AZURE_OPENAI_API_KEY"));
    assert!(matches!(azure.endpoint, BuiltinEndpoint::AzureDeployment));

    let vertex = builtin_profile("google-vertex").expect("google-vertex profile");
    assert_eq!(vertex.api_key_env, None, "Google Cloud credentials chain");
    assert!(matches!(vertex.endpoint, BuiltinEndpoint::VertexLocation));

    let bedrock = builtin_profile("amazon-bedrock").expect("amazon-bedrock profile");
    assert_eq!(bedrock.api_key_env, None, "AWS SigV4 credential chain");
    assert!(matches!(bedrock.endpoint, BuiltinEndpoint::BedrockRegion));

    let deepseek = builtin_profile("deepseek").expect("deepseek profile");
    assert_eq!(deepseek.base_url, Some("https://api.deepseek.com"));
    assert_eq!(deepseek.api_key_env, Some("DEEPSEEK_API_KEY"));
    assert_eq!(deepseek.auth_scheme, AuthScheme::Bearer);

    let copilot = builtin_profile("github-copilot").expect("github-copilot profile");
    assert_eq!(
        copilot.base_url,
        Some("https://api.individual.githubcopilot.com")
    );
    assert_eq!(copilot.api_key_env, Some("COPILOT_GITHUB_TOKEN"));
    assert!(copilot.oauth);

    let workers = builtin_profile("cloudflare-workers-ai").expect("cloudflare-workers-ai");
    assert_eq!(workers.api_key_env, Some("CLOUDFLARE_API_KEY"));

    let radius = builtin_profile("radius").expect("radius profile");
    assert_eq!(radius.api_key_env, Some("RADIUS_API_KEY"));
    assert!(matches!(radius.endpoint, BuiltinEndpoint::Dynamic));

    let typesafe = builtin_profile("typesafe").expect("typesafe profile");
    assert!(typesafe.classifier_only, "typesafe ships only classifiers");
}

#[tokio::test]
async fn register_builtin_registers_the_canonical_collection() {
    let mut models = Models::new();
    let report = register_builtin(
        &mut models,
        map_context(&[]),
        std::sync::Arc::new(opi_ai::http::HttpClient::new()),
        &BuiltinOverrides::default(),
    )
    .expect("builtin registration succeeds");

    // Vertex source selection is lazy and remains registered without ambient reads.
    for expected in [
        "google",
        "google-vertex",
        "amazon-bedrock",
        "anthropic",
        "openai",
        "deepseek",
        "openrouter",
    ] {
        assert!(
            report.registered.iter().any(|id| id == expected),
            "{expected} must register; got {:?}",
            report.registered
        );
    }
    // Endpoint and credentials are lazy; typesafe is classifier-only.
    assert!(report.registered.iter().any(|id| id == "azure"));
    assert!(!report.registered.iter().any(|id| id == "typesafe"));
    let skipped_ids: Vec<&str> = report.skipped.iter().map(|(id, _)| id.as_str()).collect();
    assert!(!skipped_ids.contains(&"azure"));
    assert!(!skipped_ids.contains(&"google-vertex"));
    assert!(skipped_ids.contains(&"typesafe"));

    // The bundled catalog backs canonical resolution.
    let google_model = first_catalog_chat_model("google");
    assert!(models.resolve(&format!("google:{google_model}")).is_ok());
    // No permanent legacy aliases in the collection.
    assert!(models.resolve("gemini:anything").is_err());
    let provider_ids = models.provider_ids();
    assert!(!provider_ids.contains(&"gemini"));
    assert!(!provider_ids.contains(&"bedrock"));
    assert!(!provider_ids.contains(&"vertex"));
}

#[tokio::test]
async fn catalog_tiered_pricing_flows_into_registered_models() {
    let mut models = Models::new();
    register_builtin(
        &mut models,
        map_context(&[]),
        std::sync::Arc::new(opi_ai::http::HttpClient::new()),
        &BuiltinOverrides::default(),
    )
    .unwrap();

    // Find one tiered chat entry in the bundled catalog.
    let catalog = Catalog::builtin().unwrap();
    let mut tiered: Option<(String, String, serde_json::Value)> = None;
    for provider in catalog.providers() {
        for entry in catalog.models(provider, CatalogOperation::Chat).unwrap() {
            if entry.raw.get("cost").and_then(|c| c.get("tiers")).is_some() {
                tiered = Some((provider.to_owned(), entry.id.clone(), entry.raw.clone()));
                break;
            }
        }
        if tiered.is_some() {
            break;
        }
    }
    let Some((provider, id, raw)) = tiered else {
        panic!("bundled catalog has no tiered pricing entry");
    };
    let (_, model) = models
        .resolve(&format!("{provider}:{id}"))
        .expect("tiered model registers");
    let pricing = model.pricing.as_ref().expect("pricing attached");
    let cost = &raw["cost"];
    assert_eq!(
        pricing.base.input_cost_per_mtok,
        cost["input"].as_f64().unwrap()
    );
    let raw_tier = &cost["tiers"][0];
    let tier = &pricing.tiers[0];
    assert_eq!(
        tier.input_tokens_above,
        raw_tier["inputTokensAbove"].as_number().unwrap().clone()
    );
    assert_eq!(
        tier.pricing.input_cost_per_mtok,
        raw_tier["input"].as_f64().unwrap()
    );
}

#[tokio::test]
async fn builtin_registration_keeps_custom_record_upsert() {
    let mut models = Models::new();
    register_builtin(
        &mut models,
        map_context(&[]),
        std::sync::Arc::new(opi_ai::http::HttpClient::new()),
        &BuiltinOverrides::default(),
    )
    .unwrap();

    // A custom registration replaces the builtin provider wholesale.
    models
        .register_route(
            text_mock("google", "custom wins"),
            Arc::new(StaticAuthResolver::new(
                AuthScheme::ApiKey,
                secret(SECRET_VALUE),
            )) as Arc<dyn AuthResolver>,
            AuthProvenanceSource::Static,
            CompatMetadata::default(),
        )
        .unwrap();
    let prepared = models
        .prepare_call("google:mock-model", minimal_request("google:mock-model"))
        .await
        .expect("custom record dispatches after builtin");
    let message = drain_to_completion(prepared.start_attempt().unwrap())
        .await
        .expect("custom mock completes");
    assert_eq!(assistant_text(&message), "custom wins");
}

#[test]
fn builtin_construction_and_static_selection_need_no_runtime_or_source_reads() {
    struct NoSources;
    impl opi_ai::auth::AuthContext for NoSources {
        fn env(&self, _: &str) -> Result<Option<String>, opi_ai::auth::AuthContextError> {
            panic!("construction read environment")
        }
        fn file_exists(&self, _: &str) -> Result<bool, opi_ai::auth::AuthContextError> {
            panic!("construction inspected a file")
        }
    }
    let mut models = Models::new();
    let report = register_builtin(
        &mut models,
        std::sync::Arc::new(NoSources),
        std::sync::Arc::new(opi_ai::http::HttpClient::new()),
        &BuiltinOverrides::default(),
    )
    .unwrap();
    assert_eq!(report.registered.len(), 41);
    let model = models
        .get_model("groq", &first_catalog_chat_model("groq"))
        .unwrap();
    assert!(!model.id.is_empty());
}

#[test]
fn selected_provider_protocol_sources_are_constructible() {
    let mut owners = Vec::new();

    #[cfg(feature = "provider-anthropic")]
    {
        let _ = opi_ai::api::anthropic::parse_sse_events("");
        let _ = std::any::type_name::<opi_ai::providers::anthropic::AnthropicProvider>();
        owners.push("anthropic");
    }
    #[cfg(feature = "provider-azure-openai")]
    {
        let _ = opi_ai::providers::azure_openai::resolve_azure_base_url(
            None,
            None,
            Some("https://example.test"),
        );
        let _ = std::any::type_name::<opi_ai::api::azure_openai::AzureOpenAIOptions>();
        let _ = std::any::type_name::<opi_ai::api::azure_openai::AzureOpenAIWireAdapter>();
        owners.push("azure-openai");
    }
    #[cfg(feature = "provider-bedrock")]
    {
        assert!(opi_ai::providers::bedrock::is_standard_bedrock_endpoint(
            "https://bedrock-runtime.us-east-1.amazonaws.com"
        ));
        owners.push("bedrock");
    }
    #[cfg(feature = "provider-gemini")]
    {
        let _ = std::any::type_name::<opi_ai::api::gemini::GoogleGenerativeAiOptions>();
        let _ = std::any::type_name::<opi_ai::providers::gemini::GeminiProvider>();
        owners.push("gemini");
    }
    #[cfg(feature = "provider-mistral")]
    {
        let _ = opi_ai::providers::mistral::mistral_provider(None);
        owners.push("mistral");
    }
    #[cfg(feature = "provider-openai-chat")]
    {
        let _ = opi_ai::api::openai::parse_sse_events("");
        let _ = std::any::type_name::<opi_ai::api::openai::OpenAiCompletionsOptions>();
        let _ = std::any::type_name::<opi_ai::providers::openai::OpenAiChatProvider>();
        owners.push("openai-chat");
    }
    #[cfg(feature = "provider-openai-codex-responses")]
    {
        let _ = std::any::type_name::<opi_ai::api::openai_codex_responses::CodexOptions>();
        let _ = std::any::type_name::<opi_ai::api::openai_codex_responses::CodexChatApi>();
        let _ =
            std::any::type_name::<opi_ai::providers::openai_codex::OpenAiCodexResponsesProvider>();
        owners.push("openai-codex-responses");
    }
    #[cfg(feature = "provider-openai-responses")]
    {
        let _ = std::any::type_name::<opi_ai::api::openai_responses::OpenAiResponsesOptions>();
        let _ = std::any::type_name::<opi_ai::api::openai_responses::OpenAiResponsesChatApi>();
        let _ = opi_ai::providers::openai_responses::OpenAiResponsesProvider::new(None);
        owners.push("openai-responses");
    }
    #[cfg(feature = "provider-openrouter")]
    {
        let _ = opi_ai::providers::openrouter::model_catalog();
        owners.push("openrouter");
    }
    #[cfg(feature = "provider-pi-messages")]
    {
        let _ = std::any::type_name::<opi_ai::api::pi_messages::PiMessagesOptions>();
        let _ = opi_ai::providers::pi_messages::PiMessagesProvider::new(None);
        owners.push("pi-messages");
    }
    #[cfg(feature = "provider-vertex")]
    {
        let _ = opi_ai::providers::vertex::VertexProvider::express(None);
        owners.push("vertex");
    }

    assert!(
        !owners.is_empty(),
        "a provider protocol feature is required"
    );
}

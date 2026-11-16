#[test]
fn selected_provider_protocol_sources_are_constructible() {
    let mut owners = Vec::new();

    #[cfg(feature = "provider-anthropic")]
    {
        let _ = opi_ai::anthropic::parse_sse_events("");
        owners.push("anthropic");
    }
    #[cfg(feature = "provider-azure-openai")]
    {
        let _ =
            opi_ai::azure_openai::resolve_azure_base_url(None, None, Some("https://example.test"));
        owners.push("azure-openai");
    }
    #[cfg(feature = "provider-bedrock")]
    {
        assert!(opi_ai::bedrock::is_standard_bedrock_endpoint(
            "https://bedrock-runtime.us-east-1.amazonaws.com"
        ));
        owners.push("bedrock");
    }
    #[cfg(feature = "provider-gemini")]
    {
        let _ = opi_ai::gemini::GeminiProvider::new(None);
        owners.push("gemini");
    }
    #[cfg(feature = "provider-mistral")]
    {
        let _ = opi_ai::mistral::mistral_provider(None);
        owners.push("mistral");
    }
    #[cfg(feature = "provider-openai-chat")]
    {
        let _ = opi_ai::openai_chat::parse_sse_events("");
        owners.push("openai-chat");
    }
    #[cfg(feature = "provider-openai-codex-responses")]
    {
        let _ = std::any::type_name::<opi_ai::openai_codex_responses::CodexOptions>();
        owners.push("openai-codex-responses");
    }
    #[cfg(feature = "provider-openai-responses")]
    {
        let _ = opi_ai::openai_responses::OpenAiResponsesProvider::new(None);
        owners.push("openai-responses");
    }
    #[cfg(feature = "provider-openrouter")]
    {
        let _ = opi_ai::openrouter::model_catalog();
        owners.push("openrouter");
    }
    #[cfg(feature = "provider-pi-messages")]
    {
        let _ = opi_ai::pi_messages::PiMessagesProvider::new(None);
        owners.push("pi-messages");
    }
    #[cfg(feature = "provider-vertex")]
    {
        let _ = opi_ai::vertex::VertexProvider::express(None);
        owners.push("vertex");
    }

    assert!(
        !owners.is_empty(),
        "a provider protocol feature is required"
    );
}

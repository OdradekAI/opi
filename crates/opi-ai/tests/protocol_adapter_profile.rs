use std::any::type_name;

#[test]
fn protocol_adapters_are_available_without_catalog_bundle() {
    let _ = opi_ai::anthropic::parse_sse_events("");
    let _ = opi_ai::openai_chat::parse_sse_events("");
    let _ = opi_ai::openai_responses::OpenAiResponsesProvider::new(None);
    let _ = opi_ai::openrouter::model_catalog();
    let _ = opi_ai::mistral::mistral_provider(None);
    let _ = opi_ai::pi_messages::PiMessagesProvider::new(Some("http://127.0.0.1:1".to_owned()));
    let _ = opi_ai::bedrock::is_standard_bedrock_endpoint(
        "https://bedrock-runtime.us-east-1.amazonaws.com",
    );
    let _ = opi_ai::vertex::VertexProvider::express(None);
    let _ = opi_ai::azure_openai::resolve_azure_base_url(None, None, Some("https://example.test"));
}

#[test]
fn auth_store_and_ws_contracts_remain_visible_at_protocol_boundary() {
    assert!(type_name::<opi_ai::auth::OAuthCredential>().contains("OAuthCredential"));
    assert!(
        type_name::<opi_ai::auth::store_oauth::StoreOAuthResolver>().contains("StoreOAuthResolver")
    );
    let nonce = opi_ai::ws::fresh_nonce();
    assert_eq!(nonce.len(), 16);
    assert!(!opi_ai::ws::accept_token(&nonce).is_empty());
}

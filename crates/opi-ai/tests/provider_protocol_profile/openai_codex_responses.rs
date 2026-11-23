#[test]
fn provider_openai_codex_responses_protocol_source_is_constructible() {
    let _ = std::any::type_name::<opi_ai::api::openai_codex_responses::CodexOptions>();
    let _ = std::any::type_name::<opi_ai::api::openai_codex_responses::CodexChatApi>();
    let _ = std::any::type_name::<opi_ai::providers::openai_codex::OpenAiCodexResponsesProvider>();
}

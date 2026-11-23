#[test]
fn provider_openai_chat_protocol_source_is_constructible() {
    let _ = opi_ai::api::openai_completions::parse_sse_events("");
    let _ = std::any::type_name::<opi_ai::api::openai_completions::OpenAiCompletionsOptions>();
    let _ = std::any::type_name::<opi_ai::providers::openai::OpenAiChatProvider>();
}

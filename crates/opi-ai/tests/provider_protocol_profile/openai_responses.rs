#[test]
fn provider_openai_responses_protocol_source_is_constructible() {
    let _ = std::any::type_name::<opi_ai::api::openai_responses::OpenAiResponsesOptions>();
    let _ = std::any::type_name::<opi_ai::api::openai_responses::OpenAiResponsesChatApi>();
    let _ = opi_ai::providers::openai_responses::OpenAiResponsesProvider::new(None);
}

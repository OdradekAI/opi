#[test]
fn provider_anthropic_protocol_source_is_constructible() {
    let _ = opi_ai::api::anthropic::parse_sse_events("");
    let _ = std::any::type_name::<opi_ai::providers::anthropic::AnthropicProvider>();
}

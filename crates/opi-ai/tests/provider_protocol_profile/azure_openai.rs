#[test]
fn provider_azure_openai_protocol_source_is_constructible() {
    let _ = opi_ai::providers::azure_openai::resolve_azure_base_url(
        None,
        None,
        Some("https://example.test"),
    );
    let _ = std::any::type_name::<opi_ai::api::azure_openai::AzureOpenAIOptions>();
    let _ = std::any::type_name::<opi_ai::api::azure_openai::AzureOpenAIWireAdapter>();
    assert_eq!(opi_ai::providers::azure_openai::model_catalog().len(), 45);
}

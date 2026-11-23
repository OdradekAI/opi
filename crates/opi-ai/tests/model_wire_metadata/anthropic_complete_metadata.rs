//! Complete Anthropic records preserve default source compatibility.

#[test]
fn absent_source_true_defaults_remain_true() {
    let catalog = opi_ai::catalog::Catalog::builtin().unwrap();
    let source = catalog
        .model(
            "anthropic",
            opi_ai::catalog::CatalogOperation::Chat,
            "claude-fable-5",
        )
        .unwrap();
    let model = source.to_chat_model_info().unwrap();
    let opi_ai::WireCompat::AnthropicMessages(compat) = model.compat else {
        panic!("Anthropic source uses its matching wire")
    };
    assert!(compat.supports_eager_tool_input_streaming);
    assert!(compat.supports_temperature);
}

#[test]
fn provider_pi_messages_protocol_source_is_constructible() {
    let _ = std::any::type_name::<opi_ai::api::pi_messages::PiMessagesOptions>();
    let _ = opi_ai::providers::pi_messages::PiMessagesProvider::new(None);
}

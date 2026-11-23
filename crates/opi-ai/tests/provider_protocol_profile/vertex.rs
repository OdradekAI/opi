#[test]
fn provider_vertex_protocol_source_is_constructible() {
    use opi_ai::provider::Provider;
    use opi_ai::providers::vertex::{VertexProvider, google_vertex_provider, model_catalog};

    let catalog = model_catalog();
    assert_eq!(catalog.len(), 13);
    assert_eq!(VertexProvider::express(None).models(), catalog);
    let provider = google_vertex_provider(
        Some(("fixture-project".into(), "us".into())),
        None,
        std::sync::Arc::new(opi_ai::http::HttpClient::new()),
    );
    assert_eq!(provider.id(), "google-vertex");
    assert_eq!(provider.models(), catalog);
}

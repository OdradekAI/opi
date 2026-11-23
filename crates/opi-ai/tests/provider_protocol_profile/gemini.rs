#[test]
fn provider_gemini_protocol_source_is_constructible() {
    use opi_ai::provider::Provider;
    use opi_ai::providers::gemini::{GeminiProvider, google_provider, model_catalog};

    let _ = std::any::type_name::<opi_ai::api::gemini::GoogleGenerativeAiOptions>();
    let direct = GeminiProvider::new(None);
    let google = google_provider(None, std::sync::Arc::new(opi_ai::http::HttpClient::new()));
    let catalog = model_catalog();
    assert_eq!(catalog.len(), 23);
    assert_eq!(direct.models(), catalog);
    assert_eq!(google.models(), catalog);
    assert_eq!(google.id(), "google");
}

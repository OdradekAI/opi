//! Shared Node 5 Pi Messages replay fixture builders.

use super::*;

pub(crate) fn node5_pi_sse(events: &[serde_json::Value]) -> String {
    events
        .iter()
        .map(|event| format!("data: {event}\n\n"))
        .collect()
}

pub(crate) fn node5_pi_terminal(kind: &str) -> serde_json::Value {
    serde_json::json!({"type":kind,"reason":if kind=="error"{"error"}else{"toolUse"},
        "responseId":"terminal-id","errorMessage":"fixture failure",
        "usage":{"input":13,"output":7,"cacheRead":0,"cacheWrite":0,"totalTokens":20,
            "cost":{"input":0,"output":0,"cacheRead":0,"cacheWrite":0,"total":0}}})
}

pub(crate) async fn node5_pi_complete(
    base: &str,
) -> Result<opi_ai::message::AssistantMessage, opi_ai::stream::CallFailure> {
    let model = model_44(MODEL_44);
    let api = PiMessagesChatApi::new(
        PiMessagesProvider::new(Some(base.into())).with_models(vec![model.clone()]),
        model.clone(),
    )
    .unwrap();
    let mut models = opi_ai::models::Models::new();
    let handle = models
        .register_chat_api(
            std::sync::Arc::new(api),
            std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                opi_ai::auth::AuthScheme::Bearer,
                SecretString::from("test-key"),
            )),
            opi_ai::AuthProvenanceSource::Static,
        )
        .unwrap();
    models
        .complete(
            opi_ai::models::ChatModelRecord::new("radius", model),
            req_44("probe"),
            handle.options(PiMessagesOptions::native(None, None, false)),
        )
        .await
}

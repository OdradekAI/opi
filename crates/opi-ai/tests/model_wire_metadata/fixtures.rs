//! Request construction and sampling values shared by topics in this target.

use opi_ai::provider::Request;
use tokio_util::sync::CancellationToken;

pub(super) fn model_request(provider_id: &str, model_id: &str) -> Request {
    Request {
        projection: Default::default(),
        transport: Default::default(),
        model: format!("{provider_id}:{model_id}"),
        context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
            system_prompt: None,
            messages: Vec::new(),
            tools: Vec::new(),
        })
        .expect("valid context declarations"),
        max_tokens: None,
        temperature: None,
        sampling_params: Default::default(),
        thinking: Default::default(),
        stop_sequences: Vec::new(),
        metadata: None,
        cancel: CancellationToken::new(),
        timeout: None,
        extra_headers: Vec::new(),
        cache_retention: Default::default(),
        session_id: None,
        telemetry_context: None,
        callbacks: Default::default(),
    }
}

pub(super) fn sampling_json(value: serde_json::Value) -> opi_ai::types::model::SamplingParams {
    value.as_object().cloned().expect("object sampling params")
}

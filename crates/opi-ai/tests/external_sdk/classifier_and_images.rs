//! Typed classifier and image dispatch through one Models instance.

use std::sync::Arc;

use opi_ai::AuthProvenanceSource;
use opi_ai::auth::{AuthResolver, AuthScheme};
use opi_ai::classifier::{ClassifierCall, ClassifierContext, ClassifierQuestion};
use opi_ai::http::HttpClient;
use opi_ai::images::{ImagesCall, ImagesContext, ImagesInput};
use opi_ai::models::Models;
use secrecy::SecretString;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn classifier_and_image_apis_answer_typed_through_one_instance() {
    let mut models = Models::new();
    let resolver = || {
        Arc::new(opi_ai::auth::StaticAuthResolver::new(
            AuthScheme::Bearer,
            SecretString::from("sk-matrix"),
        )) as Arc<dyn AuthResolver>
    };

    // System One (typesafe): typed choices with probabilities and usage.
    let system_one_peer = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/systemone"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "answers": {
                "category": {
                    "type": "choice",
                    "choice": "success",
                    "probabilities": {"success": 0.9, "failure": 0.1},
                    "confidence": 0.8
                }
            }
        })))
        .expect(1)
        .mount(&system_one_peer)
        .await;
    let mut system_one_record = opi_ai::api::system_one::default_typesafe_classifier_record();
    system_one_record.base_url = system_one_peer.uri();
    models
        .register_classifier_api(
            Arc::new(opi_ai::api::system_one::TypesafeSystemOneApi::new(
                HttpClient::new(),
                system_one_record.clone(),
            )),
            resolver(),
            AuthProvenanceSource::Static,
        )
        .unwrap();

    // Cloudflare Workers AI system one: boolean mapping.
    let cloudflare_peer = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {"answers": {
                "category": {
                    "type": "choice",
                    "choice": "success",
                    "probabilities": {"success": 0.9, "failure": 0.1},
                    "confidence": 0.8
                }
            }}
        })))
        .mount(&cloudflare_peer)
        .await;
    let mut cloudflare_record =
        opi_ai::api::cloudflare_workers_ai_system_one::default_cloudflare_classifier_records()
            .into_iter()
            .next()
            .expect("cloudflare record");
    cloudflare_record.base_url = cloudflare_peer.uri();
    models
        .register_classifier_api(
            Arc::new(
                opi_ai::api::cloudflare_workers_ai_system_one::CloudflareWorkersAiSystemOneApi::new(
                    HttpClient::new(),
                    cloudflare_record.clone(),
                ),
            ),
            resolver(),
            AuthProvenanceSource::Static,
        )
        .unwrap();

    // OpenAI Decisions API: 504 stays a typed exclusion.
    let decisions_peer = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(504))
        .mount(&decisions_peer)
        .await;
    let mut decisions_record = opi_ai::api::openai_decisions::default_decisions_classifier_record();
    decisions_record.base_url = decisions_peer.uri();
    models
        .register_classifier_api(
            Arc::new(opi_ai::api::openai_decisions::OpenAiDecisionsApi::new(
                HttpClient::new(),
                decisions_record.clone(),
            )),
            resolver(),
            AuthProvenanceSource::Static,
        )
        .unwrap();

    // Llama classify: the three-endpoint llama-server protocol (tokenize,
    // apply-template, completion with char-token top_logprobs), mirroring
    // the fake server in classifier_contract.rs.
    let llama_peer = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(|request: &wiremock::Request| {
            let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap_or_default();
            match request.url.path() {
                "/tokenize" => {
                    let tokens: Vec<u64> = body["content"]
                        .as_str()
                        .unwrap_or_default()
                        .chars()
                        .map(|c| c as u64)
                        .collect();
                    ResponseTemplate::new(200)
                        .set_body_json(serde_json::json!({ "tokens": tokens }))
                }
                "/apply-template" => {
                    let mut prompt = String::new();
                    for message in body["messages"].as_array().unwrap_or(&Vec::new()) {
                        prompt.push_str(&format!(
                            "<|{}|>\n{}\n",
                            message["role"].as_str().unwrap_or_default(),
                            message["content"].as_str().unwrap_or_default()
                        ));
                    }
                    prompt.push_str("<|assistant|>\n");
                    ResponseTemplate::new(200)
                        .set_body_json(serde_json::json!({ "prompt": prompt }))
                }
                "/completion" => {
                    // Choice answers rank the second criterion highest.
                    let top_logprobs: Vec<serde_json::Value> = [
                        (b'B' as u64, -0.3f64),
                        (b'A' as u64, -1.5),
                        (b'C' as u64, -3.0),
                    ]
                    .into_iter()
                    .map(|(id, logprob)| {
                        serde_json::json!({
                            "id": id,
                            "token": char::from_u32(id as u32).unwrap_or_default(),
                            "bytes": [],
                            "logprob": logprob,
                        })
                    })
                    .collect();
                    ResponseTemplate::new(200).set_body_json(serde_json::json!({
                        "content": "B",
                        "completion_probabilities": [
                            { "id": 66, "token": "B", "top_logprobs": top_logprobs }
                        ]
                    }))
                }
                _ => ResponseTemplate::new(404).set_body_string("not found"),
            }
        })
        .mount(&llama_peer)
        .await;
    let mut llama_record = opi_ai::api::llama_cpp_classify::default_llama_classifier_record();
    llama_record.base_url = llama_peer.uri();
    models
        .register_classifier_api(
            Arc::new(opi_ai::api::llama_cpp_classify::LlamaCppClassifyApi::new(
                HttpClient::new(),
                llama_record.clone(),
            )),
            resolver(),
            AuthProvenanceSource::Static,
        )
        .unwrap();

    // The sole published image API: openrouter-images with real outputs.
    let image_peer = MockServer::start().await;
    Mock::given(method("POST"))
        .and(wiremock::matchers::path_regex(
            "^/(api/)?(v1/)?chat/completions$",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "gen-123",
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "Done",
                    "images": [{"image_url": "data:image/png;base64,QUJD"}]
                },
                "finish_reason": "stop"
            }]
        })))
        .mount(&image_peer)
        .await;
    let mut image_record = opi_ai::api::openrouter_images::default_openrouter_image_record();
    image_record.base_url = image_peer.uri();
    models
        .register_image_api(
            Arc::new(opi_ai::api::openrouter_images::OpenRouterImagesApi::new(
                HttpClient::new(),
                image_record.clone(),
            )),
            resolver(),
            AuthProvenanceSource::Static,
        )
        .unwrap();

    let classify_state = || ClassifierContext {
        state: serde_json::json!({"build": "green"}),
        questions: [(
            "category".to_owned(),
            ClassifierQuestion::Choice {
                instructions: "Is the build healthy?".into(),
                criteria: [
                    ("success".to_owned(), "healthy".to_owned()),
                    ("failure".to_owned(), "broken".to_owned()),
                ]
                .into_iter()
                .collect(),
            },
        )]
        .into_iter()
        .collect(),
        images: None,
    };

    let system_one = models
        .classify(
            system_one_record,
            classify_state(),
            ClassifierCall::default(),
        )
        .await;
    assert!(system_one.error.is_none());
    assert_eq!(
        system_one.stop_reason,
        opi_ai::classifier::ClassifierStopReason::Stop
    );

    let cloudflare = models
        .classify(
            cloudflare_record,
            classify_state(),
            ClassifierCall::default(),
        )
        .await;
    assert!(cloudflare.error.is_none());

    let decisions = models
        .classify(
            decisions_record,
            classify_state(),
            ClassifierCall::default(),
        )
        .await;
    assert!(
        decisions.error.is_some(),
        "504 stays a typed classifier failure: {:?}",
        decisions.answers
    );

    let llama = models
        .classify(llama_record, classify_state(), ClassifierCall::default())
        .await;
    assert!(llama.error.is_none());

    let images = models
        .generate_images(
            image_record,
            ImagesContext {
                input: vec![ImagesInput::Text {
                    text: "a small red square".into(),
                }],
            },
            ImagesCall::default(),
        )
        .await;
    assert_eq!(images.stop_reason, opi_ai::images::ImagesStopReason::Stop);
    assert_eq!(images.output.len(), 2);

    system_one_peer.verify().await;
}

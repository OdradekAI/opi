//! Cloudflare Workers AI run envelopes, account routing and replies.

use std::sync::Arc;

use opi_ai::AuthProvenanceSource;
use opi_ai::api::ClassifierApi;
use opi_ai::api::cloudflare_workers_ai_system_one::{
    CLOUDFLARE_WORKERS_AI_SYSTEM_ONE_API_ID, CloudflareSystemOneOptions,
    CloudflareWorkersAiSystemOneApi, default_cloudflare_classifier_records,
};
use opi_ai::api::system_one::TYPESAFE_SYSTEM_ONE_API_ID;
use opi_ai::auth::{AuthResolver, AuthScheme, ResolvedAuth};
use opi_ai::classifier::{
    ClassifierAnswer, ClassifierCall, ClassifierContext, ClassifierImage, ClassifierModelRecord,
    ClassifierStopReason,
};
use opi_ai::http::HttpClient;
use opi_ai::model_info::ModelPricing;
use opi_ai::models::{Models, ModelsError};
use opi_ai::provider::ProviderError;
use opi_ai::types::ApiId;
use secrecy::SecretString;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::failure_fixture::assert_syntax_failure;
use super::fixtures::{assert_failure_diagnostics_private, classify_context, zero_pricing};

/// Resolver carrying the Cloudflare account identity next to the Bearer
/// secret (pi resolves `CLOUDFLARE_ACCOUNT_ID` alongside the API key).
pub(super) struct CloudflareAccountResolver {
    pub(super) account_id: Option<String>,
}

impl AuthResolver for CloudflareAccountResolver {
    fn resolve<'a>(
        &'a self,
    ) -> opi_ai::credential::BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
        let account_id = self.account_id.clone();
        Box::pin(async move {
            Ok(ResolvedAuth {
                env: Default::default(),
                scheme: AuthScheme::Bearer,
                secret: SecretString::from("cf-key"),
                base_url: None,
                account_id,
                provenance: Default::default(),
            })
        })
    }
}

fn cloudflare_models(
    record: &ClassifierModelRecord,
    account_id: Option<String>,
) -> (
    Models,
    opi_ai::models::ClassifierApiHandle<CloudflareSystemOneOptions>,
) {
    let mut models = Models::new();
    let handle = models
        .register_classifier_api(
            Arc::new(CloudflareWorkersAiSystemOneApi::new(
                HttpClient::new(),
                record.clone(),
            )),
            Arc::new(CloudflareAccountResolver { account_id }),
            AuthProvenanceSource::Static,
        )
        .expect("register cloudflare classifier api");
    (models, handle)
}

/// `{CLOUDFLARE_ACCOUNT_ID}` placeholder URL rooted at the mock server.
fn cloudflare_record(server_uri: &str, id: &str, cost: ModelPricing) -> ClassifierModelRecord {
    ClassifierModelRecord {
        headers: Vec::new(),
        input_limits: None,
        compat: None,
        provider_id: "cloudflare-workers-ai".into(),
        id: id.into(),
        name: id.into(),
        api: ApiId::new(CLOUDFLARE_WORKERS_AI_SYSTEM_ONE_API_ID),
        base_url: format!("{server_uri}/accounts/{{CLOUDFLARE_ACCOUNT_ID}}/ai"),
        input: vec![opi_ai::classifier::ClassifierInputModality::Text],
        context_window: 64000,
        cost: Some(cost),
    }
}

pub(super) fn cloudflare_jev(server_uri: &str) -> ClassifierModelRecord {
    cloudflare_record(server_uri, "typesafe/jev", zero_pricing())
}

// Jev's run-record payload from the pinned test
// (https://developers.cloudflare.com/ai/models/typesafe/jev/).
fn jev_run_answers() -> serde_json::Value {
    serde_json::json!({
        "model": "jev-1.13.0",
        "answers": {
            "category": {
                "type": "choice",
                "choice": "failure",
                "confidence": 0.8,
                "probabilities": { "success": 0.13, "failure": 0.87 }
            },
            "satisfaction": { "type": "score", "score": 1, "confidence": 0.6 },
            "approved": { "type": "noul", "noul": 0.95 }
        },
        "usage": { "input_tokens": 426, "output_tokens": 73 }
    })
}

pub(super) fn run_record_response(state: &str, result: serde_json::Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "result": { "state": state, "result": result, "gatewayMetadata": { "keySource": "Unified" } },
        "success": true,
        "errors": [],
        "messages": []
    }))
}

#[tokio::test]
async fn cloudflare_posts_run_envelope_with_account_url_and_noul() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/accounts/account-id/ai/run"))
        .respond_with(run_record_response("Completed", jev_run_answers()))
        .expect(1)
        .mount(&server)
        .await;

    let record = cloudflare_jev(&server.uri());
    let (models, _handle) = cloudflare_models(&record, Some("account-id".into()));
    // The ordinary classify driver (no native options): the simple request
    // shape is the adapter's default options.
    let result = models
        .classify(record, classify_context(), ClassifierCall::default())
        .await;

    assert_eq!(result.stop_reason, ClassifierStopReason::Stop);
    let usage = result.usage.expect("usage reported");
    assert_eq!(usage.input_tokens, 426);
    assert_eq!(usage.output_tokens, 73);
    assert_eq!(usage.total_tokens, Some(499));
    assert_eq!(
        result.answers.get("approved"),
        Some(&ClassifierAnswer::Bool { probability: 0.95 })
    );

    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent.len(), 1);
    let body: serde_json::Value = serde_json::from_slice(&sent[0].body).expect("json body");
    // The Cloudflare envelope wraps the System One request in `input` and
    // carries the full model id.
    assert_eq!(body["model"], "typesafe/jev");
    assert_eq!(
        body["input"]["state"],
        serde_json::json!({ "text": "The deployment succeeded, thank you." })
    );
    assert_eq!(body["input"]["questions"]["category"]["type"], "choice");
    assert_eq!(body["input"]["questions"]["satisfaction"]["type"], "score");
    assert_eq!(body["input"]["questions"]["approved"]["type"], "noul");
    assert_eq!(
        body["input"]["questions"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["category", "satisfaction", "approved"]
    );
    assert_eq!(
        body["input"]["questions"]["category"]["criteria"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["success", "failure"]
    );
    assert!(body.get("state").is_none());
    assert!(body.get("questions").is_none());
    assert!(body.get("temperature").is_none());
    assert!(
        sent[0]
            .headers
            .get("authorization")
            .expect("bearer header")
            .to_str()
            .unwrap()
            .starts_with("Bearer ")
    );
}

#[tokio::test]
async fn cloudflare_parses_direct_envelope_and_ignores_temperature() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/accounts/account-id/ai/run"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "model": "clef",
                "answers": {
                    "category": {
                        "type": "choice",
                        "choice": "success",
                        "probabilities": { "success": 0.8368, "failure": 0.1632 },
                        "confidence": 0.4538
                    },
                    "satisfaction": { "type": "score", "score": 2, "confidence": 0.5 },
                    "approved": { "type": "noul", "noul": 0.9912 }
                },
                "usage": { "input_tokens": 222, "output_tokens": 0 }
            },
            "success": true,
            "errors": [],
            "messages": []
        })))
        .expect(1)
        .mount(&server)
        .await;

    let record = cloudflare_jev(&server.uri());
    let (models, handle) = cloudflare_models(&record, Some("account-id".into()));
    // Native options captured on the wire: temperature stays off it.
    let options = handle.options(CloudflareSystemOneOptions {
        temperature: Some(0.7),
        ..Default::default()
    });
    let result = models
        .classify_native(
            record,
            classify_context(),
            ClassifierCall::default(),
            options,
        )
        .await;

    assert_eq!(result.stop_reason, ClassifierStopReason::Stop);
    let usage = result.usage.expect("usage reported");
    assert_eq!(usage.input_tokens, 222);
    assert_eq!(usage.output_tokens, 0);
    assert_eq!(usage.total_tokens, Some(222));
    assert_eq!(
        result.answers.get("category"),
        Some(&ClassifierAnswer::Choice {
            choice: "success".into(),
            probabilities: [
                ("success".to_owned(), 0.8368),
                ("failure".to_owned(), 0.1632)
            ]
            .into_iter()
            .collect(),
            confidence: 0.4538,
        })
    );
    assert_eq!(
        result.answers.get("approved"),
        Some(&ClassifierAnswer::Bool {
            probability: 0.9912
        })
    );

    let sent = server.received_requests().await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&sent[0].body).expect("json body");
    assert!(body.get("temperature").is_none());
    assert!(body["input"].get("temperature").is_none());
}

#[tokio::test]
async fn cloudflare_failure_diagnostics_do_not_echo_envelope_messages() {
    let canary = "synthetic-cloudflare-envelope-private-canary";
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/accounts/account-id/ai/run"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": null,
            "success": false,
            "errors": [{ "code": 5007, "message": canary }]
        })))
        .expect(1)
        .mount(&server)
        .await;
    let record = cloudflare_jev(&server.uri());
    let (models, _) = cloudflare_models(&record, Some("account-id".into()));
    let result = models
        .classify(
            record.clone(),
            classify_context(),
            ClassifierCall::default(),
        )
        .await;

    assert_eq!(server.received_requests().await.unwrap().len(), 1);
    assert_failure_diagnostics_private(&result, &record, canary);
    assert!(matches!(result.error, Some(ProviderError::StreamError(_))));
    let message = result.error_message.as_deref().unwrap();
    assert!(message.contains("Cloudflare Workers AI request failed"));
    assert!(message.contains("[REDACTED]"));
}

#[tokio::test]
async fn cloudflare_failure_diagnostics_do_not_echo_run_state() {
    let canary = "synthetic-cloudflare-state-private-canary";
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/accounts/account-id/ai/run"))
        .respond_with(run_record_response(canary, serde_json::Value::Null))
        .expect(1)
        .mount(&server)
        .await;
    let record = cloudflare_jev(&server.uri());
    let (models, _) = cloudflare_models(&record, Some("account-id".into()));
    let result = models
        .classify(
            record.clone(),
            classify_context(),
            ClassifierCall::default(),
        )
        .await;

    assert_eq!(server.received_requests().await.unwrap().len(), 1);
    assert_failure_diagnostics_private(&result, &record, canary);
    assert!(matches!(result.error, Some(ProviderError::StreamError(_))));
    let message = result.error_message.as_deref().unwrap();
    assert!(message.contains("Cloudflare Workers AI run did not complete"));
    assert!(message.contains("[REDACTED]"));
}

#[tokio::test]
async fn cloudflare_reports_incomplete_runs_and_envelope_errors() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/accounts/account-id/ai/run"))
        .respond_with(run_record_response("Queued", serde_json::Value::Null))
        .expect(1)
        .mount(&server)
        .await;

    let record = cloudflare_jev(&server.uri());
    let (models, _handle) = cloudflare_models(&record, Some("account-id".into()));
    let result = models
        .classify(record, classify_context(), ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("run did not complete"),
        "message was {:?}",
        result.error_message
    );

    // success: false reports failure without exposing its envelope messages.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/accounts/account-id/ai/run"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": null,
            "success": false,
            "errors": [ { "code": 5007, "message": "No such model" } ],
            "messages": []
        })))
        .expect(1)
        .mount(&server)
        .await;
    let record = cloudflare_jev(&server.uri());
    let (models, _handle) = cloudflare_models(&record, Some("account-id".into()));
    let result = models
        .classify(record, classify_context(), ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("Cloudflare Workers AI request failed"),
        "message was {:?}",
        result.error_message
    );

    // No usable message strings falls back to the source's generic text.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/accounts/account-id/ai/run"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": null,
            "success": false,
            "errors": [ { "code": 7003 }, "not-a-record" ],
            "messages": []
        })))
        .expect(1)
        .mount(&server)
        .await;
    let record = cloudflare_jev(&server.uri());
    let (models, _handle) = cloudflare_models(&record, Some("account-id".into()));
    let result = models
        .classify(record, classify_context(), ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("Cloudflare Workers AI request failed"),
        "message was {:?}",
        result.error_message
    );
}

#[tokio::test]
async fn cloudflare_reports_unexpected_envelope_shapes() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/accounts/account-id/ai/run"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([1, 2, 3])))
        .mount(&server)
        .await;

    let record = cloudflare_jev(&server.uri());
    let (models, _handle) = cloudflare_models(&record, Some("account-id".into()));
    let result = models
        .classify(record, classify_context(), ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("Cloudflare Workers AI returned an unexpected response"),
        "message was {:?}",
        result.error_message
    );

    // A completed run whose inner result is not a record is also unexpected.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/accounts/account-id/ai/run"))
        .respond_with(run_record_response("Completed", serde_json::json!("boom")))
        .expect(1)
        .mount(&server)
        .await;
    let record = cloudflare_jev(&server.uri());
    let (models, _handle) = cloudflare_models(&record, Some("account-id".into()));
    let result = models
        .classify(record, classify_context(), ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("Cloudflare Workers AI returned an unexpected response"),
        "message was {:?}",
        result.error_message
    );

    // A failed answer parse inside a valid envelope still keeps the billed
    // usage (shared system-one rule, pinned here through the run envelope).
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/accounts/account-id/ai/run"))
        .respond_with(run_record_response(
            "Completed",
            serde_json::json!({
                "answers": {},
                "usage": { "input_tokens": 100, "output_tokens": 5 }
            }),
        ))
        .expect(1)
        .mount(&server)
        .await;
    let record = cloudflare_jev(&server.uri());
    let (models, _handle) = cloudflare_models(&record, Some("account-id".into()));
    let result = models
        .classify(record, classify_context(), ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("did not return an answer for category"),
        "message was {:?}",
        result.error_message
    );
    let usage = result.usage.expect("billed usage retained");
    assert_eq!(usage.input_tokens, 100);
    assert_eq!(usage.output_tokens, 5);
}

#[tokio::test]
async fn cloudflare_prices_clef_through_catalog_rates() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/accounts/account-id/ai/run"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": {
                "model": "clef",
                "answers": jev_run_answers()["answers"],
                "usage": { "input_tokens": 222, "output_tokens": 0 }
            },
            "success": true
        })))
        .expect(1)
        .mount(&server)
        .await;

    // Curated clef pricing from the pinned test: $0.24 per million input
    // tokens. Register the curated records repointed at the mock server and
    // classify through the registered clef record.
    let mut records: Vec<ClassifierModelRecord> = default_cloudflare_classifier_records();
    for record in &mut records {
        record.base_url = format!("{}/accounts/{{CLOUDFLARE_ACCOUNT_ID}}/ai", server.uri());
    }
    let jev = records.remove(0);
    let (mut models, _handle) = cloudflare_models(&jev, Some("account-id".into()));
    for record in records {
        models
            .register_classifier_model_record(record)
            .expect("register curated record");
    }
    let clef = models
        .classifier_model("cloudflare-workers-ai", "@cf/cloudflare/clef")
        .expect("curated clef resolvable");
    assert_eq!(clef.id, "@cf/cloudflare/clef");

    let result = models
        .classify(clef, classify_context(), ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Stop);
    let cost = result.cost.expect("cost reported");
    let expected = 222.0 * 0.24 / 1_000_000.0;
    assert!(
        (cost.input_cost - expected).abs() < 1e-12,
        "input cost {} vs {expected}",
        cost.input_cost
    );
    assert!(cost.output_cost.abs() < 1e-12);
}

#[tokio::test]
async fn cloudflare_multi_model_registration_resolves_all_curated_records() {
    let mut models = Models::new();
    let mut records = default_cloudflare_classifier_records();
    let first = records.remove(0);
    let adapter = CloudflareWorkersAiSystemOneApi::new(HttpClient::new(), first.clone());
    models
        .register_classifier_api(
            Arc::new(adapter),
            Arc::new(CloudflareAccountResolver { account_id: None }),
            AuthProvenanceSource::Static,
        )
        .expect("register");
    for record in records {
        models
            .register_classifier_model_record(record)
            .expect("append records");
    }
    for id in [
        "typesafe/jev",
        "@cf/cloudflare/clef",
        "@cf/cloudflare/clef-flash",
    ] {
        let record = models
            .classifier_model("cloudflare-workers-ai", id)
            .unwrap_or_else(|error| panic!("resolve {id}: {error}"));
        assert_eq!(record.id, id);
    }
    // Chat models and unknown ids stay typed rejections.
    assert!(matches!(
        models.classifier_model("cloudflare-workers-ai", "unknown"),
        Err(ModelsError::NotAClassifierModel { .. })
    ));
    // Appending a record whose api has no registration is a typed error.
    let orphan = ClassifierModelRecord {
        api: ApiId::new("no-such-classifier-api"),
        ..cloudflare_jev("http://127.0.0.1:9")
    };
    assert!(
        matches!(
            models.register_classifier_model_record(orphan),
            Err(ModelsError::ClassifierApiNotRegistered { .. })
        ),
        "expected ClassifierApiNotRegistered"
    );
}

#[tokio::test]
async fn cloudflare_rejects_image_input_before_any_request() {
    let server = MockServer::start().await;
    let record = cloudflare_jev(&server.uri());
    let (models, _handle) = cloudflare_models(&record, Some("account-id".into()));
    let context = ClassifierContext {
        images: Some(vec![ClassifierImage {
            mime_type: "image/png".into(),
            data: "aGk=".into(),
        }]),
        ..classify_context()
    };
    let result = models
        .classify(record, context, ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("Cloudflare Workers AI does not support image input"),
        "message was {:?}",
        result.error_message
    );
    assert!(matches!(
        result.error,
        Some(ProviderError::UnsupportedCapability { .. })
    ));
    assert_eq!(server.received_requests().await.unwrap().len(), 0);
}

#[tokio::test]
async fn cloudflare_missing_key_is_typed_pre_dispatch_failure() {
    let server = MockServer::start().await;
    let record = cloudflare_jev(&server.uri());
    let mut models = Models::new();
    models
        .register_classifier_api(
            Arc::new(CloudflareWorkersAiSystemOneApi::new(
                HttpClient::new(),
                record.clone(),
            )),
            Arc::new(MissingCloudflareKeyResolver),
            AuthProvenanceSource::Static,
        )
        .expect("register");
    let result = models
        .classify(record, classify_context(), ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert_eq!(
        result.error_message.as_deref(),
        Some("No API key for provider: cloudflare-workers-ai")
    );
    assert!(matches!(
        result.error,
        Some(ProviderError::CredentialNeeded { .. })
    ));
    assert_eq!(server.received_requests().await.unwrap().len(), 0);
}

struct MissingCloudflareKeyResolver;

impl AuthResolver for MissingCloudflareKeyResolver {
    fn resolve<'a>(
        &'a self,
    ) -> opi_ai::credential::BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
        Box::pin(async move {
            Err(ProviderError::CredentialNeeded {
                provider_id: "cloudflare-workers-ai".into(),
            })
        })
    }
}

#[tokio::test]
async fn cloudflare_missing_account_keeps_placeholder_and_fails_typed() {
    // The placeholder stays in the host when no account identity resolved,
    // so the literal URL is unsendable (pi keeps the placeholder verbatim
    // in `resolveCloudflareModel`'s fallback).
    let mut record = cloudflare_jev("http://{CLOUDFLARE_ACCOUNT_ID}");
    record.base_url = "http://{CLOUDFLARE_ACCOUNT_ID}/accounts/ai".into();
    let (models, _handle) = cloudflare_models(&record, None);
    let result = models
        .classify(record, classify_context(), ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert!(result.error.is_some());
    assert!(result.answers.is_empty());
}

#[tokio::test]
async fn cloudflare_adapter_guards_api_and_auth_scheme() {
    // A typesafe-system-one record on the cloudflare adapter is a typed
    // pre-dispatch rejection.
    let mut record = cloudflare_jev("http://127.0.0.1:9");
    record.api = ApiId::new(TYPESAFE_SYSTEM_ONE_API_ID);
    let adapter = CloudflareWorkersAiSystemOneApi::new(HttpClient::new(), record.clone());
    let result = adapter
        .classify_prepared(
            record,
            classify_context(),
            CloudflareSystemOneOptions::default(),
            opi_ai::test_support::resolved_bearer_auth(),
            ClassifierCall::default(),
        )
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("Unsupported classifier API: typesafe-system-one"),
        "message was {:?}",
        result.error_message
    );

    // Non-Bearer auth is a typed config rejection before any request.
    let record = cloudflare_jev("http://127.0.0.1:9");
    let adapter = CloudflareWorkersAiSystemOneApi::new(HttpClient::new(), record.clone());
    let result = adapter
        .classify_prepared(
            record,
            classify_context(),
            CloudflareSystemOneOptions::default(),
            opi_ai::test_support::resolved_auth(),
            ClassifierCall::default(),
        )
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("requires Bearer auth scheme"),
        "message was {:?}",
        result.error_message
    );
}

#[tokio::test]
async fn cloudflare_json_syntax_failure_does_not_retry() {
    assert_syntax_failure("cloudflare").await;
}

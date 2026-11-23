//! Cloudflare Workers AI System One adapter.
//!
//! Serves System One classifier models through Cloudflare's REST endpoint:
//! one `POST <base>/run` with the `{model, input}` envelope where `input`
//! carries the System One wire request (`{state, questions}`, public `bool`
//! questions riding as `noul`). The account-scoped endpoint URL carries a
//! `{CLOUDFLARE_ACCOUNT_ID}` placeholder that materializes from the resolved
//! auth's account identity — when absent the placeholder stays literal (the
//! pi `resolveCloudflareModel` fallback) and the request never reaches a
//! real endpoint. The response rides Cloudflare's API envelope in two
//! shapes: third-party run records wrap the output as
//! `{success, result: {state, result: {answers, usage}}}` with `state`
//! required to be `Completed`, while Cloudflare-hosted models return
//! `{success, result: {answers, usage}}` directly; `success: false`
//! produces a closed redacted diagnostic. Everything else — Bearer auth,
//! guards, the retry policy (two extra retries by default), per-attempt
//! body-inclusive timeouts, billed-usage retention and tiered catalog
//! pricing — is the shared System One core from
//! [`crate::api::system_one`]. `temperature` is a typed option the wire
//! never carries.

use crate::api::ClassifierApi;
use crate::api::system_one::{SystemOneCoreOptions, SystemOneTransport, classify_over_system_one};
use crate::auth::ResolvedAuth;
use crate::classifier::{ClassifierCall, ClassifierContext, ClassifierModelRecord};
use crate::http::HttpClient;
use crate::provider::{ProviderError, ProviderErrorSummary};
use crate::types::ApiId;

/// The exact pi classifier API identity this adapter serves.
pub const CLOUDFLARE_WORKERS_AI_SYSTEM_ONE_API_ID: &str = "cloudflare-workers-ai-system-one";

/// Default account-scoped endpoint. The adapter posts model runs to `run`
/// after substituting the resolved Cloudflare account ID.
pub const CLOUDFLARE_WORKERS_AI_REST_BASE_URL: &str =
    "https://api.cloudflare.com/client/v4/accounts/{CLOUDFLARE_ACCOUNT_ID}/ai";

/// Endpoint placeholder materialized from the resolved account identity.
const CLOUDFLARE_ACCOUNT_ID: &str = "{CLOUDFLARE_ACCOUNT_ID}";

/// Source label used in error messages.
const LABEL: &str = "Cloudflare Workers AI";

/// Typed native options of the Cloudflare Workers AI System One adapter.
///
/// Mirrors the source `ClassifierOptions` shape: `temperature` stays typed
/// for seam parity but the System One request has no temperature field, and
/// the retry policy defaults to two extra retries.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CloudflareSystemOneOptions {
    /// Logit divisor the adapter cannot apply (kept for seam parity).
    /// Never serialized.
    pub temperature: Option<f64>,
    /// Extra retry attempts after the first failure. Defaults to `2`.
    pub max_retries: u32,
    /// Cap for server-requested retry delays in milliseconds; `None` uses
    /// the 60 s source default, `Some(0)` disables the cap.
    pub max_retry_delay_ms: Option<u64>,
}

/// The Cloudflare Workers AI System One adapter: fixed
/// `O = CloudflareSystemOneOptions`.
pub struct CloudflareWorkersAiSystemOneApi {
    http: HttpClient,
    record: ClassifierModelRecord,
    api_id: ApiId,
}

impl CloudflareWorkersAiSystemOneApi {
    /// Build the adapter over a shared [`HttpClient`] with its complete
    /// record.
    pub fn new(http: HttpClient, record: ClassifierModelRecord) -> Self {
        let api_id = record.api.clone();
        Self {
            http,
            record,
            api_id,
        }
    }
}

impl ClassifierApi<CloudflareSystemOneOptions> for CloudflareWorkersAiSystemOneApi {
    fn api_id(&self) -> &ApiId {
        &self.api_id
    }

    fn provider_id(&self) -> &str {
        &self.record.provider_id
    }

    fn complete_record(&self) -> &ClassifierModelRecord {
        &self.record
    }

    fn default_options(&self) -> CloudflareSystemOneOptions {
        CloudflareSystemOneOptions {
            temperature: None,
            max_retries: 2,
            max_retry_delay_ms: None,
        }
    }

    fn validate_request(
        &self,
        record: &ClassifierModelRecord,
        context: &ClassifierContext,
        options: &CloudflareSystemOneOptions,
        call: &ClassifierCall,
    ) -> Result<(), ProviderError> {
        self.validate_options(options)?;
        crate::api::system_one::validate_system_one_request(
            record,
            context,
            call,
            LABEL,
            image_input(record),
        )
    }

    fn classify_prepared<'a>(
        &'a self,
        record: ClassifierModelRecord,
        context: ClassifierContext,
        options: CloudflareSystemOneOptions,
        auth: ResolvedAuth,
        call: ClassifierCall,
    ) -> crate::api::ClassifierFuture<'a> {
        Box::pin(async move {
            // pi `resolveCloudflareModel`: the account placeholder
            // materializes from the resolved provider env; a missing value
            // keeps the literal placeholder.
            let base = crate::auth::builtin::effective_base_url(
                &record.provider_id,
                &record.base_url,
                &auth,
            )
            .replace(
                CLOUDFLARE_ACCOUNT_ID,
                auth.account_id.as_deref().unwrap_or(CLOUDFLARE_ACCOUNT_ID),
            );
            let transport = SystemOneTransport {
                api_id: CLOUDFLARE_WORKERS_AI_SYSTEM_ONE_API_ID,
                label: LABEL,
                url: format!("{}/run", base.trim_end_matches('/')),
                image_input: image_input(&record),
                send_image_opt_in_header: false,
                payload: cloudflare_payload,
                output: cloudflare_output,
            };
            classify_over_system_one(
                self.http.client().clone(),
                record,
                context,
                SystemOneCoreOptions {
                    max_retries: options.max_retries,
                    max_retry_delay_ms: options.max_retry_delay_ms,
                },
                auth,
                call,
                transport,
            )
            .await
        })
    }
}

fn image_input(record: &ClassifierModelRecord) -> Option<crate::classifier::ClassifierImageInput> {
    record.compat.as_ref().and_then(|compat| compat.image_input)
}

/// Wrap the System One wire request in Cloudflare's `{model, input}`
/// envelope.
fn cloudflare_payload(body: serde_json::Value) -> serde_json::Value {
    let mut input = serde_json::json!({
        "state": body["state"],
        "questions": body["questions"],
    });
    if let Some(images) = body.get("images") {
        input["images"] = images.clone();
    }
    if let Some(assets) = body.get("assets") {
        input["assets"] = assets.clone();
    }
    serde_json::json!({ "model": body["model"], "input": input })
}

/// Unwrap Cloudflare's response envelope into the System One output record
/// (`{answers, usage}`), redacting upstream failure details.
fn cloudflare_output(body: &serde_json::Value, label: &str) -> Result<serde_json::Value, String> {
    let unexpected = || format!("{label} returned an unexpected response");
    let record = || -> Option<&serde_json::Value> { body.is_object().then_some(body) };
    let body = record().ok_or_else(unexpected)?;
    // Only an exact `success: false` carries the envelope error path.
    if body.get("success") == Some(&serde_json::Value::Bool(false)) {
        return Err(cloudflare_error_message(body, label));
    }
    let result = body
        .get("result")
        .filter(|result| result.is_object())
        .ok_or_else(unexpected)?;
    // Cloudflare-hosted models return the output directly in `result`.
    if result.get("answers").is_some() {
        return Ok(result.clone());
    }
    // Third-party models add a run record gated on `state: "Completed"`.
    if result.get("state").and_then(serde_json::Value::as_str) != Some("Completed") {
        return Err(format!(
            "{label} run did not complete (state: {})",
            ProviderErrorSummary::redacted()
        ));
    }
    let inner = result
        .get("result")
        .filter(|inner| inner.is_object())
        .ok_or_else(unexpected)?;
    Ok(inner.clone())
}

/// Keep upstream envelope messages behind the closed diagnostic boundary.
fn cloudflare_error_message(body: &serde_json::Value, label: &str) -> String {
    if let Some(message) = body
        .get("errors")
        .and_then(serde_json::Value::as_array)
        .and_then(|errors| {
            errors
                .iter()
                .find_map(|error| error.get("message").and_then(serde_json::Value::as_str))
        })
    {
        return format!(
            "{label} request failed: {}",
            ProviderErrorSummary::from_untrusted(message)
        );
    }
    format!("{label} request failed")
}

/// The curated default records for the Cloudflare classifier adapter.
///
/// The fallback set contains `typesafe/jev`, `@cf/cloudflare/clef`, and
/// `@cf/cloudflare/clef-flash`. Jev uses a 64,000-token context window.
/// Hosted context limits and output prices use zero in these fallback
/// records. Register a complete classifier record to supply specific values.
pub fn default_cloudflare_classifier_records() -> Vec<ClassifierModelRecord> {
    fn record(
        id: &str,
        name: &str,
        context_window: u64,
        input_cost_per_mtok: f64,
    ) -> ClassifierModelRecord {
        ClassifierModelRecord {
            headers: Vec::new(),
            input_limits: None,
            compat: None,
            provider_id: "cloudflare-workers-ai".into(),
            id: id.into(),
            name: name.into(),
            api: ApiId::new(CLOUDFLARE_WORKERS_AI_SYSTEM_ONE_API_ID),
            base_url: CLOUDFLARE_WORKERS_AI_REST_BASE_URL.into(),
            input: vec![crate::classifier::ClassifierInputModality::Text],
            context_window,
            cost: Some(
                crate::model_info::ModelPricing::try_new(
                    crate::stream::Pricing {
                        input_cost_per_mtok,
                        ..Default::default()
                    },
                    Vec::new(),
                )
                .expect("valid pricing"),
            ),
        }
    }
    vec![
        record("typesafe/jev", "Jev", 64000, 0.0),
        record("@cf/cloudflare/clef", "Clef", 0, 0.24),
        record("@cf/cloudflare/clef-flash", "Clef Flash", 0, 0.09),
    ]
}

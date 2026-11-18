//! Azure OpenAI wire adapters.
//!
//! This module owns Azure-specific Chat/Responses request transport, native
//! options, status mapping, stream decoding and the typed Responses adapter.
//! The Provider profile and endpoint/deployment resolution live in
//! [`crate::providers::azure_openai`].

use std::fmt;
use std::sync::Arc;

use futures_util::{StreamExt, stream};
use secrecy::ExposeSecret;
use tokio_util::sync::CancellationToken;

use crate::api::openai::{OpenAiChatMapper, drain_sse_events};
use crate::api::openai_responses::{
    OpenAiResponsesOptions, ResponsesReasoningSummary, ResponsesToolChoice,
};
use crate::api::shared::headers::ProviderHeaders;
use crate::api::shared::openai_responses::{ResponsesMapper, drain_sse_frames};
use crate::model_info::{ModelInfo, WireApi};
use crate::provider::routes::{
    AzureRouteContext, OpenAiResponsesRouteContext, parse_deployment_map, resolve_azure_base_url,
    responses_url,
};
use crate::provider::{EventStream, Provider, ProviderError, ProviderErrorSummary, Request};
use crate::stream::{AssistantStreamEvent, CancelAwareReceiverStream, send_or_cancel};

/// Azure OpenAI wire adapter.
///
/// The adapter owns request-scoped native options and the selected model record.
/// The registered provider profile remains an immutable route/catalog description.
#[derive(Clone)]
pub struct AzureOpenAIWireAdapter {
    provider: AzureRouteContext,
    native_options: Option<AzureOpenAIOptions>,
    active_record: Option<ModelInfo>,
}

impl AzureOpenAIWireAdapter {
    /// Construct a neutral adapter from an Azure provider profile.
    pub fn new(provider: AzureRouteContext) -> Self {
        Self {
            provider,
            native_options: None,
            active_record: None,
        }
    }

    /// Construct a per-call adapter with explicit native options.
    pub fn with_options(provider: AzureRouteContext, options: AzureOpenAIOptions) -> Self {
        Self {
            provider,
            native_options: Some(options),
            active_record: None,
        }
    }

    fn dispatch_configured(&self, model: ModelInfo, options: AzureOpenAIOptions) -> Self {
        let mut configured = self.clone();
        configured.active_record = Some(model);
        configured.native_options = Some(options);
        configured
    }
}

impl AzureOpenAIWireAdapter {
    /// Build the Azure Chat request body through the shared OpenAI adapter.
    pub fn build_request_body(
        &self,
        request: &Request,
    ) -> Result<serde_json::Value, crate::provider::ProviderError> {
        self.provider
            .inner
            .clone()
            .with_wire(crate::WireApi::AzureOpenAiCompletions)
            .build_request_body(request)
    }

    /// Stream one responses-wire dispatch (task 20.38): the shared 20.37
    /// parameter assembly under Azure transport (deployment URL, api-key
    /// header, azure status mapping, duration and telemetry).
    fn stream_responses_prepared(
        &self,
        record: ModelInfo,
        mut request: Request,
        auth: crate::auth::ResolvedAuth,
    ) -> EventStream {
        let projection = /* frozen call */ match crate::transcript::compile_request_projection(&request, "azure", Some(&record), record.wire_api).map(|projection| projection.facts) {
            Ok(facts) => facts,
            Err(error) => return Box::pin(futures_util::stream::once(async move { Err(error) })),
        };
        let event_projection = projection.clone();
        crate::stream::project_call(event_projection, || -> EventStream {
            request.callbacks = request.callbacks.ensure_call_scope();
            let secret = auth.secret.expose_secret().to_string();
            let api_key = match auth.scheme {
                crate::auth::AuthScheme::ApiKey => secret,
                crate::auth::AuthScheme::Bearer
                | crate::auth::AuthScheme::AwsSigV4(_)
                | crate::auth::AuthScheme::AnthropicFederation => {
                    return Box::pin(stream::once(async move {
                        Err(ProviderError::Config(
                            ProviderErrorSummary::attested_static(
                                "Azure OpenAI requires prepared API-key authentication",
                            ),
                        ))
                    }));
                }
            };
            let requested_model = request
                .model
                .split_once(':')
                .map(|(_, id)| id)
                .unwrap_or(&request.model)
                .to_owned();
            let native = self.native_options.clone().unwrap_or_default();
            let deployment = native
                .azure_deployment_name
                .clone()
                .or_else(|| {
                    parse_deployment_map(self.provider.deployment_map.as_deref())
                        .get(&requested_model)
                        .cloned()
                })
                .unwrap_or_else(|| requested_model.clone());
            let endpoint = match resolve_azure_base_url(
                native.azure_base_url.as_deref(),
                native.azure_resource_name.as_deref(),
                record
                    .base_url
                    .as_deref()
                    .or(Some(self.provider.endpoint.as_str())),
            ) {
                Ok(endpoint) => endpoint,
                Err(error) => return Box::pin(stream::once(async move { Err(error) })),
            };
            let api_version = native
                .azure_api_version
                .clone()
                .unwrap_or_else(|| self.provider.api_version.clone());
            let url = responses_url(&endpoint, &api_version);

            // Shared 20.37 parameter assembly under the azure identity: the
            // record keeps its own wire so same-identity replay works
            // end-to-end; service_tier stays absent (source omission).
            let responses_options = OpenAiResponsesOptions {
                tool_choice: native.tool_choice,
                reasoning_effort: native.reasoning_effort,
                reasoning_summary: native.reasoning_summary,
                service_tier: None,
                simple: native.simple,
            };
            let inner = OpenAiResponsesRouteContext::for_route(
                None,
                "azure".into(),
                ProviderHeaders::default(),
                vec![record.clone()],
                self.provider.client.clone(),
            )
            .dispatch_configured(record.clone());
            let (body, wire_facts, effort_stamp) =
                match inner.compile_request_body(&request, Some(&responses_options)) {
                    Ok(compiled) => compiled,
                    Err(error) => return Box::pin(stream::once(async move { Err(error) })),
                };

            let body = {
                let mut body = body;
                body["model"] = serde_json::Value::String(deployment.clone());
                body
            };

            let cancel = request.cancel;
            let timeout = request.timeout;
            let extra_headers = match ProviderHeaders::default().merge_model_request(
                &record.headers,
                &[],
                &request.extra_headers,
            ) {
                Ok(headers) => headers,
                Err(error) => return Box::pin(stream::once(async move { Err(error) })),
            };
            let callbacks = request.callbacks;
            // Explicit frozen telemetry context (task 20.54/AIR-055): the
            // producer task runs under the captured parent/Dispatch when
            // provided; omission observes nothing.
            let telemetry = request.telemetry_context.clone();
            let http_client = self.provider.client.client().clone();
            let snapshot = crate::stream::ProducerSnapshot::with_projection(projection);
            let producer_snapshot = snapshot.clone();
            let (tx, rx) = tokio::sync::mpsc::channel(64);
            let tx_closed = tx.clone();
            let producer_cancel = cancel.clone();

            let producer = async move {
                let work = async {
                    if let Err(error) = stream_azure_responses_http(
                        http_client,
                        api_key,
                        &url,
                        requested_model,
                        wire_facts,
                        effort_stamp,
                        &body,
                        producer_cancel.clone(),
                        timeout,
                        extra_headers,
                        callbacks,
                        producer_snapshot,
                        &tx,
                    )
                    .await
                    {
                        let _ = send_or_cancel(&producer_cancel, &tx, Err(error)).await;
                    }
                };

                tokio::select! {
                    biased;
                    _ = producer_cancel.cancelled() => (),
                    _ = tx_closed.closed() => (),
                    _ = work => {},
                }
            };
            if let Some(context) = telemetry {
                tokio::spawn(context.instrument(producer));
            } else {
                tokio::spawn(producer);
            }

            Box::pin(CancelAwareReceiverStream::with_snapshot(
                rx, cancel, snapshot,
            ))
        })
    }
    /// Stream events from a raw SSE response body (for testing).
    pub fn stream_from_sse(&self, sse_body: &str, cancel: CancellationToken) -> EventStream {
        let mut mapper = OpenAiChatMapper::new(
            crate::ApiKind::OpenAi,
            "azure",
            crate::model_info::WireApi::AzureOpenAiCompletions,
            "",
        );
        let mut stream_events: Vec<Result<AssistantStreamEvent, ProviderError>> = Vec::new();
        let supports_finish_reason = self
            .provider
            .inner
            .resolve_compat_for("", None)
            .supports_finish_reason
            .unwrap_or(true);
        for (parsed, _raw) in crate::api::openai::parse_sse_events_with_raw(sse_body) {
            let items = mapper.process_frame(parsed, supports_finish_reason);
            let terminal = items
                .iter()
                .any(|item| item.as_ref().map_or(true, |event| event.is_terminal()));
            stream_events.extend(items);
            if terminal {
                break;
            }
        }
        stream_events.extend(mapper.finish(supports_finish_reason));

        let _cancel = cancel;
        Box::pin(stream::iter(stream_events))
    }
}

impl Provider for AzureOpenAIWireAdapter {
    fn validate_simple_options(
        &self,
        model: &ModelInfo,
        options: &crate::api::contract::SimpleOptions,
    ) -> Result<(), ProviderError> {
        if model.wire_api == WireApi::AzureOpenAiResponses {
            validate_complete_azure_responses_record(model)
        } else {
            options.validate_default()
        }
    }

    fn stream_model_resource_attempt(
        &self,
        model: ModelInfo,
        options: crate::api::contract::SimpleOptions,
        request: Request,
        auth: crate::auth::ResolvedAuth,
        attempt: crate::provider::AttemptContext,
        resources: Option<crate::codex_resources::CallResourceLease>,
    ) -> EventStream {
        let mut configured =
            self.dispatch_configured(model.clone(), AzureOpenAIOptions::from_simple(&options));
        configured.provider.models = vec![model.clone()];
        // The inner provider owns profile compatibility; replace only the
        // selected model so simple dispatch retains with_compat settings.
        if let Err(error) = configured.provider.inner.replace_model_catalog(vec![model]) {
            return Box::pin(stream::once(async move { Err(error) }));
        }
        configured.stream_resource_attempt(request, auth, attempt, resources)
    }

    fn id(&self) -> &str {
        "azure"
    }

    fn models(&self) -> &[ModelInfo] {
        &self.provider.models
    }

    fn stream_prepared(
        &self,
        mut request: Request,
        auth: crate::auth::ResolvedAuth,
    ) -> EventStream {
        let projection = /* frozen call */ match crate::provider::request_projection(self, &request, crate::WireApi::AzureOpenAiCompletions) {
            Ok(facts) => facts,
            Err(error) => return Box::pin(futures_util::stream::once(async move { Err(error) })),
        };
        let event_projection = projection.clone();
        crate::stream::project_call(event_projection, || -> EventStream {
            request.callbacks = request.callbacks.ensure_call_scope();
            let secret = auth.secret.expose_secret();
            let api_key = match auth.scheme {
                crate::auth::AuthScheme::ApiKey => secret.to_string(),
                crate::auth::AuthScheme::Bearer
                | crate::auth::AuthScheme::AwsSigV4(_)
                | crate::auth::AuthScheme::AnthropicFederation => {
                    return Box::pin(stream::iter(vec![Err(ProviderError::Config(
                        ProviderErrorSummary::attested_static(
                            "Azure OpenAI requires prepared API-key authentication",
                        ),
                    ))]));
                }
            };
            let model_id = request
                .model
                .split_once(':')
                .map(|(_, id)| id)
                .unwrap_or(&request.model)
                .to_string();

            // Typed dual-wire dispatch (task 20.38): the bound record (adapter
            // path) or the listed record for this model selects the wire; a
            // responses-wire record routes through the Responses transport,
            // every other record keeps the legacy chat transport.
            if let Some(record) = self
                .active_record
                .clone()
                .or_else(|| {
                    self.provider
                        .models
                        .iter()
                        .find(|model| model.id == model_id)
                        .cloned()
                })
                .filter(|record| record.wire_api == WireApi::AzureOpenAiResponses)
            {
                return self.stream_responses_prepared(record, request, auth);
            }

            let url = self.provider.build_azure_url(&model_id);
            let requested_model = model_id.clone();
            let supports_finish_reason = self
                .provider
                .inner
                .resolve_compat_for(
                    &model_id,
                    self.active_record.as_ref().or_else(|| {
                        self.provider
                            .models
                            .iter()
                            .find(|model| model.id == model_id)
                    }),
                )
                .supports_finish_reason
                .unwrap_or(true);
            let body = match self
                .provider
                .inner
                .clone()
                .with_wire(crate::WireApi::AzureOpenAiCompletions)
                .build_request_body(&request)
            {
                Ok(body) => body,
                Err(error) => return Box::pin(stream::once(async move { Err(error) })),
            };

            let cancel = request.cancel;
            let timeout = request.timeout;
            let model_headers = self
                .active_record
                .as_ref()
                .or_else(|| {
                    self.provider
                        .models
                        .iter()
                        .find(|model| model.id == model_id)
                })
                .map(|model| model.headers.as_slice())
                .unwrap_or_default();
            let extra_headers = match ProviderHeaders::default().merge_model_request(
                model_headers,
                &[],
                &request.extra_headers,
            ) {
                Ok(headers) => headers,
                Err(error) => return Box::pin(stream::once(async move { Err(error) })),
            };
            let callbacks = request.callbacks;
            // Explicit frozen telemetry context (task 20.54/AIR-055): the chat
            // producer runs under the captured parent/Dispatch when provided.
            let telemetry = request.telemetry_context.clone();
            let http_client = self.provider.client.client().clone();

            let snapshot = crate::stream::ProducerSnapshot::with_projection(projection);
            let producer_snapshot = snapshot.clone();
            let (tx, rx) = tokio::sync::mpsc::channel(64);
            let producer_cancel = cancel.clone();

            let tx_closed = tx.clone();
            let producer = async move {
                let run = async {
                    let result = async {
                        let body = callbacks
                            .apply_payload_async(crate::callbacks::ORDINARY_PAYLOAD_VARIANT, body)
                            .await?;
                        stream_azure_http(
                            http_client,
                            api_key,
                            &url,
                            requested_model,
                            supports_finish_reason,
                            &body,
                            producer_cancel.clone(),
                            timeout,
                            extra_headers,
                            callbacks,
                            producer_snapshot,
                            &tx,
                        )
                        .await
                    }
                    .await;
                    match result {
                        Ok(()) | Err(ProviderError::Cancelled) => {}
                        Err(error) => {
                            let _ = send_or_cancel(&producer_cancel, &tx, Err(error)).await;
                        }
                    }
                };
                tokio::select! {
                    biased;
                    _ = producer_cancel.cancelled() => (),
                    _ = tx_closed.closed() => (),
                    _ = run => (),
                }
            };
            if let Some(context) = telemetry {
                tokio::spawn(context.instrument(producer));
            } else {
                tokio::spawn(producer);
            }

            Box::pin(CancelAwareReceiverStream::with_snapshot(
                rx, cancel, snapshot,
            ))
        })
    }
}
impl AzureRouteContext {
    /// Compile a neutral Azure Chat request through the canonical wire adapter.
    pub fn build_request_body(
        &self,
        request: &Request,
    ) -> Result<serde_json::Value, crate::provider::ProviderError> {
        AzureOpenAIWireAdapter::new(self.clone()).build_request_body(request)
    }

    /// Map a raw Azure Chat SSE body through the canonical wire adapter.
    pub fn stream_from_sse(&self, sse_body: &str, cancel: CancellationToken) -> EventStream {
        AzureOpenAIWireAdapter::new(self.clone()).stream_from_sse(sse_body, cancel)
    }
}

impl Provider for AzureRouteContext {
    fn validate_simple_options(
        &self,
        model: &ModelInfo,
        options: &crate::api::contract::SimpleOptions,
    ) -> Result<(), ProviderError> {
        AzureOpenAIWireAdapter::new(self.clone()).validate_simple_options(model, options)
    }

    fn stream_model_resource_attempt(
        &self,
        model: ModelInfo,
        options: crate::api::contract::SimpleOptions,
        request: Request,
        auth: crate::auth::ResolvedAuth,
        attempt: crate::provider::AttemptContext,
        resources: Option<crate::codex_resources::CallResourceLease>,
    ) -> EventStream {
        let mut profile = self.clone();
        profile.models = vec![model.clone()];
        let mut configured = AzureOpenAIWireAdapter::with_options(
            profile,
            AzureOpenAIOptions::from_simple(&options),
        );
        // The inner provider owns profile compatibility; replace only the
        // selected model so simple dispatch retains with_compat settings.
        if let Err(error) = configured.provider.inner.replace_model_catalog(vec![model]) {
            return Box::pin(stream::once(async move { Err(error) }));
        }
        configured.stream_resource_attempt(request, auth, attempt, resources)
    }

    fn id(&self) -> &str {
        "azure"
    }

    fn models(&self) -> &[ModelInfo] {
        &self.models
    }

    fn stream_prepared(&self, request: Request, auth: crate::auth::ResolvedAuth) -> EventStream {
        AzureOpenAIWireAdapter::new(self.clone()).stream_prepared(request, auth)
    }

    fn replace_model_catalog(&mut self, models: Vec<ModelInfo>) -> Result<(), ProviderError> {
        self.models = models;
        Ok(())
    }
}

/// HTTP streaming with Azure-specific URL and `api-key` header.
#[allow(clippy::too_many_arguments)]
async fn stream_azure_http(
    http_client: reqwest::Client,
    api_key: String,
    url: &str,
    requested_model: String,
    supports_finish_reason: bool,
    body: &serde_json::Value,
    cancel: CancellationToken,
    timeout: Option<std::time::Duration>,
    extra_headers: Vec<(String, String)>,
    callbacks: crate::callbacks::RequestCallbacks,
    snapshot: crate::stream::ProducerSnapshot,
    tx: &tokio::sync::mpsc::Sender<Result<AssistantStreamEvent, ProviderError>>,
) -> Result<(), ProviderError> {
    let route_headers = vec![
        ("api-key".to_owned(), api_key),
        ("content-type".to_owned(), "application/json".to_owned()),
    ];
    let merged = ProviderHeaders::default().merge_request(&route_headers, &extra_headers)?;
    let transformable: Vec<(String, String)> = merged
        .iter()
        .filter(|(name, _)| {
            // The protected route headers (api-key identity, content-type
            // wire type) are excluded from the transformable per-call set.
            !route_headers
                .iter()
                .any(|(managed, _)| name.eq_ignore_ascii_case(managed))
        })
        .cloned()
        .collect();
    // Header transform (task 20.51) on the merged per-call set; the output is
    // revalidated against protected identity/auth/signing names.
    let transformed = callbacks
        .apply_header_transform_async(&transformable)
        .await?;
    let mut headers = route_headers.clone();
    headers.extend(transformed);
    let mut req = http_client.post(url);
    for (name, value) in headers {
        req = req.header(name, value);
    }
    if let Some(timeout) = timeout {
        req = req.timeout(timeout);
    }

    let response = tokio::select! {
        biased;
        _ = cancel.cancelled() => return Err(ProviderError::Cancelled),
        response = req
            .body(serde_json::to_string(body).unwrap_or_default())
            .send() => response,
    }
    .map_err(|error| {
        if error.is_timeout() {
            ProviderError::Timeout
        } else {
            ProviderError::Network(ProviderErrorSummary::sanitized(error.to_string()))
        }
    })?;

    // Response observer (task 20.51): status and headers exist, before any
    // body processing or error mapping.
    let observed_headers: Vec<(String, String)> = response
        .headers()
        .iter()
        .map(|(name, value)| {
            (
                name.as_str().to_ascii_lowercase(),
                value.to_str().unwrap_or_default().to_string(),
            )
        })
        .collect();

    let status = response.status();
    if !status.is_success() {
        let headers = response.headers().clone();
        return Err(map_azure_status(status, &headers));
    }
    // Response callback (V-36): fired exactly once on the successfully
    // acquired response; rejected dispatches never masquerade.
    snapshot.received_response();

    let mut byte_stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut decoder = crate::api::shared::text_decode::StreamTextDecoder::new();
    let mut mapper = OpenAiChatMapper::new(
        crate::ApiKind::OpenAi,
        "azure",
        crate::model_info::WireApi::AzureOpenAiCompletions,
        &requested_model,
    );

    mapper.set_snapshot(snapshot);
    callbacks
        .notify_response_async(status.as_u16(), &observed_headers)
        .await
        .map_err(|error| mapper.failure(error))?;
    let result: Result<(), ProviderError> = async {
        loop {
            let chunk = tokio::select! {
                biased;
                _ = cancel.cancelled() => {
                    return Err(mapper.failure(ProviderError::Cancelled));
                }
                chunk = byte_stream.next() => {
                    match chunk {
                        Some(c) => c,
                        None => break,
                    }
                }
            };

            let chunk = chunk.map_err(|error| {
                if error.is_timeout() {
                    ProviderError::Timeout
                } else {
                    ProviderError::StreamError(ProviderErrorSummary::sanitized(error.to_string()))
                }
            })?;
            buffer.push_str(&decoder.push(&chunk));

            for (parsed, raw) in drain_sse_events(&mut buffer, false) {
                // Raw observer (task 20.51): parsed wire JSON before
                // normalization; failure preserves the partial.
                if let Some(raw) = raw.as_ref()
                    && let Err(error) = callbacks.notify_raw_async(raw).await
                {
                    return Err(mapper.failure(error));
                }
                for item in mapper.process_frame(parsed, supports_finish_reason) {
                    let terminal = item.as_ref().map_or(true, |event| event.is_terminal());
                    if !send_or_cancel(&cancel, tx, item).await? || terminal {
                        return Ok(());
                    }
                }
            }
        }

        let eof_tail = decoder.finish();
        buffer.push_str(&eof_tail);
        for item in mapper.finish(supports_finish_reason) {
            if !send_or_cancel(&cancel, tx, item).await? {
                return Ok(());
            }
        }

        Ok(())
    }
    .await;
    result.map_err(|cause| mapper.failure(cause))
}

fn map_azure_status(
    status: reqwest::StatusCode,
    headers: &reqwest::header::HeaderMap,
) -> ProviderError {
    match status.as_u16() {
        401 | 403 => ProviderError::AuthFailed(ProviderErrorSummary::authentication_rejected()),
        404 => ProviderError::Config(ProviderErrorSummary::from_http_response(404)),
        429 => ProviderError::RateLimited {
            retry_after_ms: crate::retry::parse_retry_after(headers),
        },
        408 | 504 => ProviderError::Timeout,
        code => ProviderError::ProviderSide(ProviderErrorSummary::from_http_response(code)),
    }
}

/// HTTP streaming for the Azure Responses wire (task 20.38): Azure URL and
/// `api-key` header, shared Responses SSE mapping, duration stamping and
/// the ordinary payload/response/raw callbacks.
#[allow(clippy::too_many_arguments)]
async fn stream_azure_responses_http(
    http_client: reqwest::Client,
    api_key: String,
    url: &str,
    requested_model: String,
    wire_facts: Option<crate::api::shared::openai_responses::ResponsesWireTools>,
    effort_stamp: Option<String>,
    body: &serde_json::Value,
    cancel: CancellationToken,
    timeout: Option<std::time::Duration>,
    extra_headers: Vec<(String, String)>,
    callbacks: crate::callbacks::RequestCallbacks,
    snapshot: crate::stream::ProducerSnapshot,
    tx: &tokio::sync::mpsc::Sender<Result<AssistantStreamEvent, ProviderError>>,
) -> Result<(), ProviderError> {
    // Payload callback (task 20.51) then original-schema validation: the
    // transformed body must stay a valid Responses request.
    let body = callbacks
        .apply_payload_async(crate::callbacks::ORDINARY_PAYLOAD_VARIANT, body.clone())
        .await
        .and_then(|body| {
            crate::callbacks::validate_openai_payload_shape(
                &body,
                crate::callbacks::OpenAiPayloadShape::Responses,
            )?;
            Ok(body)
        })?;

    let route_headers = vec![
        ("api-key".to_owned(), api_key),
        ("content-type".to_owned(), "application/json".to_owned()),
    ];
    let merged = ProviderHeaders::default().merge_request(&route_headers, &extra_headers)?;
    let transformable: Vec<(String, String)> = merged
        .iter()
        .filter(|(name, _)| {
            !route_headers
                .iter()
                .any(|(managed, _)| name.eq_ignore_ascii_case(managed))
        })
        .cloned()
        .collect();
    let transformed = callbacks
        .apply_header_transform_async(&transformable)
        .await?;
    let mut headers = route_headers.clone();
    headers.extend(transformed);
    let mut req = http_client.post(url);
    for (name, value) in headers {
        req = req.header(name, value);
    }
    if let Some(timeout) = timeout {
        req = req.timeout(timeout);
    }

    let response = tokio::select! {
        biased;
        _ = cancel.cancelled() => return Err(ProviderError::Cancelled),
        response = req
            .body(serde_json::to_string(&body).unwrap_or_default())
            .send() => response,
    }
    .map_err(|error| {
        if error.is_timeout() {
            ProviderError::Timeout
        } else {
            ProviderError::Network(ProviderErrorSummary::sanitized(error.to_string()))
        }
    })?;

    // Response observer (task 20.51): status and headers exist, before any
    // body processing or error mapping.
    let observed_headers: Vec<(String, String)> = response
        .headers()
        .iter()
        .map(|(name, value)| {
            (
                name.as_str().to_ascii_lowercase(),
                value.to_str().unwrap_or_default().to_string(),
            )
        })
        .collect();

    let status = response.status();
    if !status.is_success() {
        let headers = response.headers().clone();
        return Err(map_azure_status(status, &headers));
    }
    // Response callback (V-36): exactly once on the acquired response.
    snapshot.received_response();

    let mut byte_stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut decoder = crate::api::shared::text_decode::StreamTextDecoder::new();
    let mut response_activated_at: Option<std::time::Instant> = None;
    let mut mapper = ResponsesMapper::new("azure", WireApi::AzureOpenAiResponses, &requested_model)
        .with_deferred_terminal();
    if let Some(wire) = wire_facts {
        mapper.set_grammar_tools(wire.grammar_input_properties);
        mapper.set_strict_tools(wire.strict_tools);
        mapper.seed_diagnostics(wire.diagnostics);
    }
    mapper.set_provider_thinking_level(effort_stamp);
    mapper.set_snapshot(snapshot);
    callbacks
        .notify_response_async(status.as_u16(), &observed_headers)
        .await
        .map_err(|error| mapper.failure(error))?;
    let result: Result<(), ProviderError> = async {
        loop {
            let chunk = tokio::select! {
                biased;
                _ = cancel.cancelled() => {
                    return Err(mapper.failure(ProviderError::Cancelled));
                }
                chunk = async {
                    response_activated_at.get_or_insert_with(std::time::Instant::now);
                    byte_stream.next().await
                } => {
                    match chunk {
                        Some(c) => c,
                        None => break,
                    }
                }
            };

            let chunk = chunk.map_err(|error| {
                if error.is_timeout() {
                    ProviderError::Timeout
                } else {
                    ProviderError::StreamError(ProviderErrorSummary::sanitized(error.to_string()))
                }
            })?;
            buffer.push_str(&decoder.push(&chunk));
            for frame in drain_sse_frames(&mut buffer) {
                // Raw observer (task 20.51): parsed wire JSON before
                // normalization; failure preserves the partial.
                if let Ok(raw) = serde_json::from_str::<serde_json::Value>(&frame.data)
                    && let Err(error) = callbacks.notify_raw_async(&raw).await
                {
                    return Err(mapper.failure(error));
                }
                match mapper.decode_frame(&frame) {
                    crate::api::shared::openai_responses::ParsedEvent::Valid(event) => {
                        for item in mapper.process(event) {
                            let item = OpenAiResponsesRouteContext::stamp_done_duration(
                                item,
                                response_activated_at,
                            );
                            let is_terminal = match &item {
                                Ok(terminal_event) => terminal_event.is_terminal(),
                                Err(_) => true,
                            };
                            if !send_or_cancel(&cancel, tx, item).await? {
                                return Ok(());
                            }
                            if is_terminal {
                                break;
                            }
                        }
                    }
                    crate::api::shared::openai_responses::ParsedEvent::UsageError(error) => {
                        return Err(mapper.failure(ProviderError::StreamError(
                            ProviderErrorSummary::sanitized(error),
                        )));
                    }
                    crate::api::shared::openai_responses::ParsedEvent::Malformed { .. } => {
                        return Err(mapper.failure(ProviderError::StreamError(
                            ProviderErrorSummary::attested_static(
                                "Azure OpenAI returned a malformed Responses SSE frame",
                            ),
                        )));
                    }
                }
            }
        }

        let eof_tail = decoder.finish();
        buffer.push_str(&eof_tail);
        if mapper.has_pending_terminal() {
            for item in mapper.flush_pending_terminal() {
                let item =
                    OpenAiResponsesRouteContext::stamp_done_duration(item, response_activated_at);
                if !send_or_cancel(&cancel, tx, item).await? {
                    return Ok(());
                }
            }
        } else if !mapper.saw_done {
            let err = ProviderError::StreamError(ProviderErrorSummary::attested_static(
                "stream ended without a terminal event",
            ));
            let _ = send_or_cancel(&cancel, tx, Err(mapper.failure(err))).await?;
        }

        Ok(())
    }
    .await;
    result.map_err(|cause| mapper.failure(cause))
}
/// Typed azure-openai-responses native parameters (task 20.38, SDK annex
/// §2 R/Azure row): the responses pair keeps `reasoning_effort`,
/// `reasoning_summary` (omission and explicit null both wire `auto`) and
/// `tool_choice`, plus the Azure endpoint mapping fields. `service_tier`
/// is deliberately absent (source omission; the plain OpenAI Responses
/// adapter owns that option).
#[derive(Debug, Clone, Default, PartialEq)]
#[non_exhaustive]
pub struct AzureOpenAIOptions {
    /// Explicit deployment name; the deployment map then the model id are
    /// the fallbacks.
    pub azure_deployment_name: Option<String>,
    /// Resource name resolving to `https://{resource}.openai.azure.com`.
    pub azure_resource_name: Option<String>,
    /// Explicit base URL override (azure hosts normalize to the resource
    /// root).
    pub azure_base_url: Option<String>,
    /// Explicit api-version override (default `v1`).
    pub azure_api_version: Option<String>,
    /// Full Responses `tool_choice` shape.
    pub tool_choice: Option<ResponsesToolChoice>,
    /// Native reasoning effort; unsupported levels fail typed preflight,
    /// the simple entry clamps instead.
    pub reasoning_effort: Option<crate::model_info::ThinkingLevel>,
    /// Reasoning summary selection; omission and explicit null both map to
    /// the wire default `auto`.
    pub reasoning_summary: Option<ResponsesReasoningSummary>,
    simple: bool,
}

impl AzureOpenAIOptions {
    /// Neutral mapping for the simple entry: only neutral `Auto`/`None`
    /// tool choice and the request's neutral thinking intent.
    fn from_simple(options: &crate::api::contract::SimpleOptions) -> Self {
        Self {
            azure_deployment_name: None,
            azure_resource_name: None,
            azure_base_url: None,
            azure_api_version: None,
            tool_choice: options.tool_choice.map(|choice| match choice {
                crate::api::contract::SimpleToolChoice::Auto => ResponsesToolChoice::Auto,
                crate::api::contract::SimpleToolChoice::None => ResponsesToolChoice::None,
            }),
            reasoning_effort: None,
            reasoning_summary: None,
            simple: true,
        }
    }

    fn is_simple(&self) -> bool {
        self.simple
    }
}

/// The public azure-openai-responses direct adapter registered through
/// [`crate::Models::register_chat_api`] (task 20.38). Each dispatch clones
/// the immutable provider configuration and routes through the Azure
/// responses wire; native options never mutate a registered provider.
pub struct AzureOpenAIResponsesChatApi {
    api_id: crate::types::ApiId,
    provider: Arc<AzureRouteContext>,
    model: ModelInfo,
}

impl std::fmt::Debug for AzureOpenAIResponsesChatApi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AzureOpenAIResponsesChatApi")
            .field("api_id", &self.api_id)
            .finish_non_exhaustive()
    }
}

impl AzureOpenAIResponsesChatApi {
    /// Bind a complete azure responses record. Wrong-wire or invalid
    /// records fail before authentication or network activity.
    pub fn new(provider: Arc<AzureRouteContext>, model: ModelInfo) -> Result<Self, ProviderError> {
        validate_complete_azure_responses_record(&model)?;
        Ok(Self {
            api_id: crate::types::ApiId::new("azure-openai-responses"),
            provider,
            model,
        })
    }
}

/// One complete, wire-valid azure responses model record or a typed
/// failure.
fn validate_complete_azure_responses_record(model: &ModelInfo) -> Result<(), ProviderError> {
    if model.wire_api != WireApi::AzureOpenAiResponses
        || model.id.trim().is_empty()
        || model.validate().is_err()
    {
        return Err(ProviderError::Config(
            ProviderErrorSummary::attested_static(
                "azure-openai-responses adapters require a valid complete azure responses model record",
            ),
        ));
    }
    Ok(())
}

impl crate::api::contract::ChatApi<AzureOpenAIOptions> for AzureOpenAIResponsesChatApi {
    fn validate_simple_options(
        &self,
        model: &ModelInfo,
        _options: &crate::api::contract::SimpleOptions,
    ) -> Result<(), ProviderError> {
        validate_complete_azure_responses_record(model)
    }

    fn api_id(&self) -> &crate::types::ApiId {
        &self.api_id
    }

    fn provider_id(&self) -> &str {
        self.provider.id()
    }

    fn complete_record(&self) -> ModelInfo {
        self.model.clone()
    }

    fn simple_options(&self) -> AzureOpenAIOptions {
        AzureOpenAIOptions::from_simple(&crate::api::contract::SimpleOptions::default())
    }

    fn simple_options_for(
        &self,
        options: &crate::api::contract::SimpleOptions,
    ) -> AzureOpenAIOptions {
        AzureOpenAIOptions::from_simple(options)
    }

    fn session_resources(&self) -> Option<&dyn crate::codex_resources::SessionResourceOwner> {
        None
    }

    fn validate_native_request(
        &self,
        model: &ModelInfo,
        _request: &Request,
        options: &AzureOpenAIOptions,
    ) -> Result<(), ProviderError> {
        validate_complete_azure_responses_record(model)?;
        resolve_azure_base_url(
            options.azure_base_url.as_deref(),
            options.azure_resource_name.as_deref(),
            model
                .base_url
                .as_deref()
                .or(Some(self.provider.endpoint.as_str())),
        )?;
        // Native/exact effort fails typed preflight on unsupported levels;
        // the neutral entry clamps instead.
        if let Some(level) = options.reasoning_effort
            && !options.is_simple()
            && model.thinking_level_map.resolve(level).is_err()
        {
            return Err(ProviderError::Config(ProviderErrorSummary::sanitized(
                format!(
                    "azure-openai-responses native options reject unsupported thinking level {level:?}"
                ),
            )));
        }
        Ok(())
    }

    fn stream_prepared(
        &self,
        request: Request,
        options: AzureOpenAIOptions,
        auth: crate::auth::ResolvedAuth,
    ) -> EventStream {
        self.stream_model_resource_prepared(self.model.clone(), request, options, auth, None)
    }

    fn stream_resource_prepared(
        &self,
        request: Request,
        options: AzureOpenAIOptions,
        auth: crate::auth::ResolvedAuth,
        resources: Option<crate::codex_resources::CallResourceLease>,
    ) -> EventStream {
        self.stream_model_resource_prepared(self.model.clone(), request, options, auth, resources)
    }

    fn stream_model_resource_prepared(
        &self,
        model: ModelInfo,
        request: Request,
        options: AzureOpenAIOptions,
        auth: crate::auth::ResolvedAuth,
        _resources: Option<crate::codex_resources::CallResourceLease>,
    ) -> EventStream {
        if let Err(error) = self.validate_native_request(&model, &request, &options) {
            return Box::pin(stream::once(async move { Err(error) }));
        }
        let adapter = AzureOpenAIWireAdapter::new(self.provider.as_ref().clone())
            .dispatch_configured(model, options);
        adapter.stream_prepared(request, auth)
    }
}

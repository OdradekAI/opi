//! Telemetry context accepted by AI operations.
//!
//! The backend-neutral carrier is owned by `opi-telemetry`. These existing
//! paths remain available for request callers. `TelemetryContext::from_span`
//! uses its optional tracing adapter; no-op preserves ambient host tracing.
//! AI-specific schema, instrumentation and request settlement belong here,
//! independently of context cloning, future polling or consumer Drop.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::types::usage::{Pricing, calculate_cost};
use opi_telemetry::schema::{
    AttributeDefinition, AttributeType, Cardinality, DefaultStatus, EventDefinition,
    MetricDefinition, MetricType, ParentDefinition, RequiredAttributeDefinition, SpanDefinition,
    StatusDefinition, TelemetrySchema,
};
use opi_telemetry::{
    AttributeValue, Attributes, Counter, ErrorStatus, Histogram, InstrumentDefinition, SpanOptions,
    SpanOwner, SpanStatus,
};

pub use opi_telemetry::{Instrumented, TelemetryContext};

/// The schema revision owned by this crate's AI instrumentation.
pub const AI_TELEMETRY_SCHEMA_VERSION: u32 = 1;
pub const AI_REQUEST_SPAN: &str = "pi.ai.request";
pub const AI_POLL_SPAN: &str = "pi.ai.poll";
pub const AI_REQUEST_DURATION: &str = "pi.ai.request.duration";
pub const AI_FIRST_CHUNK_DURATION: &str = "pi.ai.first_chunk.duration";
pub const AI_TOKENS: &str = "pi.ai.tokens";
pub const AI_COST: &str = "pi.ai.cost";
pub const AI_FIRST_CHUNK: &str = "pi.ai.first_chunk";
pub const AI_BLOCK: &str = "pi.ai.block";

/// Frozen identity used by one logical AI request or deferred poll.
#[derive(Debug, Clone, PartialEq)]
pub struct AiRequestAttributes {
    pub provider: String,
    pub model: String,
    pub api: String,
    pub reasoning: Option<String>,
    pub poll: bool,
    pub pricing: Option<Pricing>,
}

impl AiRequestAttributes {
    pub fn new(
        provider: impl Into<String>,
        model: impl Into<String>,
        api: impl Into<String>,
    ) -> Self {
        Self {
            provider: provider.into(),
            model: model.into(),
            api: api.into(),
            reasoning: None,
            poll: false,
            pricing: None,
        }
    }

    pub fn with_reasoning(mut self, reasoning: impl Into<String>) -> Self {
        self.reasoning = Some(reasoning.into());
        self
    }

    pub fn as_poll(mut self, poll: bool) -> Self {
        self.poll = poll;
        self
    }

    pub fn with_pricing(mut self, pricing: Pricing) -> Self {
        self.pricing = Some(pricing);
        self
    }
}

/// Request-scoped AI observer. It owns the completion authority for exactly one
/// request span and reports only normalized stream facts: first event, completed
/// content blocks, final stop/usage/response attributes, and low-cardinality
/// measurements. It does not own business execution or cancellation.
pub struct AiRequestTelemetry {
    owner: Option<SpanOwner>,
    span: opi_telemetry::TelemetrySpan,
    request_duration: Histogram,
    first_chunk_duration: Histogram,
    tokens: Counter,
    cost: Counter,
    attributes: AiRequestAttributes,
    started: Instant,
    first: bool,
    tool_chars: BTreeMap<usize, usize>,
    finished: bool,
}

impl std::fmt::Debug for AiRequestTelemetry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AiRequestTelemetry")
            .field("attributes", &self.attributes)
            .field("first", &self.first)
            .field("finished", &self.finished)
            .finish_non_exhaustive()
    }
}

impl AiRequestTelemetry {
    /// Start synchronously when an explicit context is supplied.
    ///
    /// `None` means no AI observation and returns `Ok(None)`. An explicit noop
    /// context still creates a no-op owner, preserving the same lifecycle and
    /// adapter error surface as a recording context.
    pub fn start(
        context: Option<&TelemetryContext>,
        attributes: AiRequestAttributes,
    ) -> opi_telemetry::Result<Option<Self>> {
        let Some(context) = context else {
            return Ok(None);
        };
        let mut span_options = SpanOptions::new(if attributes.poll {
            AI_POLL_SPAN
        } else {
            AI_REQUEST_SPAN
        });
        span_options
            .attributes
            .insert("provider".into(), string(&attributes.provider));
        span_options
            .attributes
            .insert("model".into(), string(&attributes.model));
        span_options
            .attributes
            .insert("api".into(), string(&attributes.api));
        if let Some(reasoning) = &attributes.reasoning {
            span_options
                .attributes
                .insert("reasoning".into(), string(reasoning));
        }

        let owner = context.start_span(span_options)?;
        let span = owner.span();
        let metrics = context.metrics();
        let request_duration = metrics.histogram(instrument(AI_REQUEST_DURATION, "ms"))?;
        let first_chunk_duration = metrics.histogram(instrument(AI_FIRST_CHUNK_DURATION, "ms"))?;
        let tokens = metrics.counter(instrument(AI_TOKENS, "token"))?;
        let cost = metrics.counter(instrument(AI_COST, "USD"))?;
        Ok(Some(Self {
            owner: Some(owner),
            span,
            request_duration,
            first_chunk_duration,
            tokens,
            cost,
            attributes,
            started: Instant::now(),
            first: true,
            tool_chars: BTreeMap::new(),
            finished: false,
        }))
    }

    /// Observe one normalized event in delivery order.
    pub fn on_event(
        &mut self,
        event: &crate::stream::AssistantStreamEvent,
    ) -> opi_telemetry::Result<()> {
        if self.finished {
            return Ok(());
        }
        if self.first {
            self.first = false;
            self.span.add_event(AI_FIRST_CHUNK, &Attributes::new())?;
            self.first_chunk_duration.record(
                self.started.elapsed().as_secs_f64() * 1_000.0,
                &self.base_attributes(),
                Some(&self.span.context()),
            )?;
        }
        match event {
            crate::stream::AssistantStreamEvent::TextEnd {
                content_index,
                content,
                ..
            } => self.record_block("text", *content_index, utf16_len(content), None)?,
            crate::stream::AssistantStreamEvent::ThinkingEnd {
                content_index,
                content,
                ..
            } => self.record_block("thinking", *content_index, utf16_len(content), None)?,
            crate::stream::AssistantStreamEvent::ToolCallDelta {
                content_index,
                delta,
                ..
            } => {
                *self.tool_chars.entry(*content_index).or_default() += utf16_len(delta);
            }
            crate::stream::AssistantStreamEvent::ToolCallEnd {
                content_index,
                tool_call,
                ..
            } => {
                let chars = self.tool_chars.remove(content_index).unwrap_or_default();
                self.record_block("tool_call", *content_index, chars, Some(&tool_call.name))?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Complete the request span once the business result is known.
    ///
    /// The message is optional for setup-only failures. A pending/deferred poll
    /// records duration but does not add token or cost counters.
    pub fn finish(
        &mut self,
        message: Option<&crate::message::AssistantMessage>,
    ) -> opi_telemetry::Result<()> {
        if self.finished {
            return Ok(());
        }
        if let Some(message) = message {
            let mut end = self.base_attributes();
            end.insert("stop".into(), string(message.stop_reason.as_str()));
            end.insert(
                "usage.input".into(),
                number(message.usage.input_tokens as f64),
            );
            end.insert(
                "usage.output".into(),
                number(message.usage.output_tokens as f64),
            );
            end.insert(
                "usage.cache_read".into(),
                number(message.usage.cache_read_tokens as f64),
            );
            end.insert(
                "usage.cache_write".into(),
                number(message.usage.cache_write_tokens as f64),
            );
            end.insert(
                "response.id".into(),
                string(message.response_id.as_deref().unwrap_or_default()),
            );
            let cost_total = self
                .attributes
                .pricing
                .map(|pricing| calculate_cost(&message.usage, &pricing).total_cost())
                .unwrap_or(0.0);
            end.insert("cost.total".into(), number(cost_total));
            self.span.set_attributes(&end)?;
            if matches!(message.stop_reason, crate::stream::StopReason::Error) {
                self.span.set_status(SpanStatus::Error {
                    error: Some(ErrorStatus {
                        name: "provider_error".into(),
                        message: owned_slice(message.error_message.as_deref().unwrap_or_default()),
                    }),
                })?;
            }
            self.request_duration.record(
                self.started.elapsed().as_secs_f64() * 1_000.0,
                &end,
                Some(&self.span.context()),
            )?;
            if !self.attributes.poll
                || !matches!(message.stop_reason, crate::stream::StopReason::Deferred)
            {
                self.record_tokens("input", message.usage.input_tokens)?;
                self.record_tokens("output", message.usage.output_tokens)?;
                self.record_tokens("cache_read", message.usage.cache_read_tokens)?;
                self.record_tokens("cache_write", message.usage.cache_write_tokens)?;
                self.cost.add(cost_total, &self.base_attributes())?;
            }
        }
        self.finished = true;
        let owner = self.owner.take().expect("telemetry owner is present");
        owner.finish(None)
    }

    fn base_attributes(&self) -> Attributes {
        Attributes::from([
            ("provider".into(), string(&self.attributes.provider)),
            ("model".into(), string(&self.attributes.model)),
        ])
    }

    fn record_block(
        &self,
        kind: &str,
        index: usize,
        chars: usize,
        tool: Option<&str>,
    ) -> opi_telemetry::Result<()> {
        let mut attributes = Attributes::from([
            ("type".into(), string(kind)),
            ("index".into(), number(index as f64)),
            ("chars".into(), number(chars as f64)),
        ]);
        if let Some(tool) = tool {
            attributes.insert("tool".into(), string(tool));
        }
        self.span.add_event(AI_BLOCK, &attributes)
    }

    fn record_tokens(&self, kind: &str, value: u32) -> opi_telemetry::Result<()> {
        let mut attributes = self.base_attributes();
        attributes.insert("kind".into(), string(kind));
        self.tokens.add(value as f64, &attributes)
    }
}

/// Execution-owned adapter for the ordinary `Models` chat path.
///
/// The public [`AiRequestTelemetry`] surface remains fallible so hosts can
/// validate a backend explicitly. Ordinary provider execution cannot replace
/// a provider result with an observer backend error, so this adapter reports
/// the backend failure through the host's `tracing` channel and disables only
/// the AI observer for that call. Business events and their typed outcome keep
/// the normal `ChatStream` contract.
pub(crate) struct AiRequestObserver {
    inner: Option<AiRequestTelemetry>,
}

/// Shared one-shot ownership for a request observer that may cross the
/// `ChatStream`/`StartedChatStream` execution seam.
///
/// Normal completion remains owned by `ChatStream`; the started execution can
/// use the same cell to settle an explicitly aborted task after Tokio confirms
/// that the task was cancelled. The underlying observer is idempotent, so a
/// completion/abort race cannot produce a second span or overwrite a terminal
/// already recorded by the normal path.
#[derive(Clone)]
pub(crate) struct SharedAiRequestObserver {
    inner: Arc<Mutex<AiRequestObserver>>,
}

impl SharedAiRequestObserver {
    pub(crate) fn start(
        context: Option<&TelemetryContext>,
        attributes: AiRequestAttributes,
    ) -> Self {
        Self {
            inner: Arc::new(Mutex::new(AiRequestObserver::start(context, attributes))),
        }
    }

    pub(crate) fn on_event(&self, event: &crate::stream::AssistantStreamEvent) {
        self.inner
            .lock()
            .expect("AI telemetry observer lock")
            .on_event(event);
    }

    pub(crate) fn finish(&self, message: Option<&crate::message::AssistantMessage>) {
        self.inner
            .lock()
            .expect("AI telemetry observer lock")
            .finish(message);
    }
}

impl AiRequestObserver {
    pub(crate) fn start(
        context: Option<&TelemetryContext>,
        attributes: AiRequestAttributes,
    ) -> Self {
        let inner = match AiRequestTelemetry::start(context, attributes) {
            Ok(inner) => inner,
            Err(error) => {
                tracing::warn!(%error, "AI telemetry observer failed to start");
                None
            }
        };
        Self { inner }
    }

    pub(crate) fn on_event(&mut self, event: &crate::stream::AssistantStreamEvent) {
        let Some(inner) = &mut self.inner else {
            return;
        };
        if let Err(error) = inner.on_event(event) {
            tracing::warn!(%error, "AI telemetry observer failed while recording an event");
            self.disable();
        }
    }

    pub(crate) fn finish(&mut self, message: Option<&crate::message::AssistantMessage>) {
        let Some(inner) = &mut self.inner else {
            return;
        };
        if let Err(error) = inner.finish(message) {
            tracing::warn!(%error, "AI telemetry observer failed to finish");
            self.inner = None;
        }
    }

    fn disable(&mut self) {
        if let Some(mut inner) = self.inner.take()
            && let Err(error) = inner.finish(None)
        {
            tracing::warn!(%error, "AI telemetry observer failed while closing after an error");
        }
    }
}

fn instrument(name: &str, unit: &str) -> InstrumentDefinition {
    InstrumentDefinition {
        name: name.into(),
        kind: if name == AI_REQUEST_DURATION || name == AI_FIRST_CHUNK_DURATION {
            opi_telemetry::MetricKind::Histogram
        } else {
            opi_telemetry::MetricKind::Counter
        },
        description: Some(name.into()),
        unit: Some(unit.into()),
        boundaries: None,
    }
}

fn string(value: &str) -> AttributeValue {
    AttributeValue::String(value.into())
}

fn number(value: f64) -> AttributeValue {
    AttributeValue::Number(value)
}

fn utf16_len(value: &str) -> usize {
    value.encode_utf16().count()
}

fn owned_slice(value: &str) -> String {
    // Pi's JavaScript slice cap is measured in UTF-16 code units. Rust cannot
    // retain an isolated surrogate, so stop before a scalar would cross the
    // cap rather than manufacturing an invalid string.
    let mut units = 0;
    let mut end = 0;
    for (index, character) in value.char_indices() {
        let width = character.len_utf16();
        if units + width > 500 {
            break;
        }
        units += width;
        end = index + character.len_utf8();
    }
    value[..end].to_owned()
}

/// Return the AI-owned schema metadata consumed by host telemetry adapters.
pub fn ai_telemetry_schema() -> TelemetrySchema {
    let mut spans = BTreeMap::new();
    let request = request_span_definition();
    spans.insert(AI_REQUEST_SPAN.into(), request.clone());
    spans.insert(
        AI_POLL_SPAN.into(),
        SpanDefinition {
            description: "One deferred-response poll, not the deferred request lifetime.".into(),
            ..request
        },
    );
    let mut metrics = BTreeMap::new();
    metrics.insert(
        AI_REQUEST_DURATION.into(),
        histogram_definition("Logical request or poll duration.", "ms", true),
    );
    metrics.insert(
        AI_FIRST_CHUNK_DURATION.into(),
        histogram_definition("Time to the first normalized stream event.", "ms", false),
    );
    metrics.insert(
        AI_TOKENS.into(),
        MetricDefinition {
            description: "Reported model token usage; pending and deferred polls are excluded."
                .into(),
            unit: Some("token".into()),
            attributes: BTreeMap::from([
                (
                    "provider".into(),
                    required(string_definition("Provider identifier.")),
                ),
                (
                    "model".into(),
                    required(string_definition("Requested model identifier.")),
                ),
                (
                    "kind".into(),
                    required(AttributeDefinition {
                        description: "Token usage category.".into(),
                        sensitive: None,
                        cardinality: None,
                        value_type: AttributeType::String {
                            values: Some(
                                ["input", "output", "cache_read", "cache_write"]
                                    .into_iter()
                                    .map(String::from)
                                    .collect(),
                            ),
                            examples: None,
                        },
                    }),
                ),
            ]),
            metric_type: MetricType::Counter,
        },
    );
    metrics.insert(
        AI_COST.into(),
        MetricDefinition {
            description: "Reported model cost; pending and deferred polls are excluded.".into(),
            unit: Some("USD".into()),
            attributes: BTreeMap::from([
                (
                    "provider".into(),
                    required(string_definition("Provider identifier.")),
                ),
                (
                    "model".into(),
                    required(string_definition("Requested model identifier.")),
                ),
            ]),
            metric_type: MetricType::Counter,
        },
    );
    TelemetrySchema {
        version: AI_TELEMETRY_SCHEMA_VERSION,
        spans,
        metrics: Some(metrics),
    }
}

fn request_span_definition() -> SpanDefinition {
    SpanDefinition {
        description: "One logical model request, including authentication and provider setup."
            .into(),
        parents: ParentDefinition::Any,
        start_attributes: BTreeMap::from([
            (
                "provider".into(),
                required(string_definition("Provider identifier.")),
            ),
            ("api".into(), required(string_definition("API identifier."))),
            (
                "model".into(),
                required(string_definition("Requested model identifier.")),
            ),
            (
                "reasoning".into(),
                optional(string_definition("Scalar requested reasoning level.")),
            ),
        ]),
        end_attributes: BTreeMap::from([
            ("stop".into(), string_definition("Normalized stop reason.")),
            (
                "usage.input".into(),
                number_definition("Uncached input tokens."),
            ),
            ("usage.output".into(), number_definition("Output tokens.")),
            (
                "usage.cache_read".into(),
                number_definition("Cache-read tokens."),
            ),
            (
                "usage.cache_write".into(),
                number_definition("Cache-write tokens."),
            ),
            (
                "cost.total".into(),
                number_definition("Total request cost in USD."),
            ),
            (
                "response.id".into(),
                AttributeDefinition {
                    description: "Provider response identifier.".into(),
                    sensitive: None,
                    cardinality: Some(Cardinality::High),
                    value_type: AttributeType::String {
                        values: None,
                        examples: None,
                    },
                },
            ),
        ]),
        events: Some(BTreeMap::from([
            (
                AI_FIRST_CHUNK.into(),
                EventDefinition {
                    description: "First normalized stream event delivered.".into(),
                    attributes: BTreeMap::new(),
                },
            ),
            (
                AI_BLOCK.into(),
                EventDefinition {
                    description: "A completed content block; content is never recorded.".into(),
                    attributes: BTreeMap::from([
                        (
                            "type".into(),
                            required(AttributeDefinition {
                                description: "Block kind.".into(),
                                sensitive: None,
                                cardinality: None,
                                value_type: AttributeType::String {
                                    values: Some(
                                        ["text", "thinking", "tool_call"]
                                            .into_iter()
                                            .map(String::from)
                                            .collect(),
                                    ),
                                    examples: None,
                                },
                            }),
                        ),
                        (
                            "index".into(),
                            required(number_definition("Content block index.")),
                        ),
                        (
                            "chars".into(),
                            required(number_definition("UTF-16 character count.")),
                        ),
                        (
                            "tool".into(),
                            optional(string_definition("Tool name for tool-call blocks only.")),
                        ),
                    ]),
                },
            ),
        ])),
        status: StatusDefinition {
            default: DefaultStatus::Ok,
            error_when: "Stop reason is error; aborted stays ok.".into(),
        },
    }
}

fn histogram_definition(description: &str, unit: &str, include_stop: bool) -> MetricDefinition {
    let mut attributes = BTreeMap::from([
        (
            "provider".into(),
            required(string_definition("Provider identifier.")),
        ),
        (
            "model".into(),
            required(string_definition("Requested model identifier.")),
        ),
    ]);
    if include_stop {
        attributes.insert(
            "stop".into(),
            required(string_definition("Normalized stop reason.")),
        );
    }
    MetricDefinition {
        description: description.into(),
        unit: Some(unit.into()),
        attributes,
        metric_type: MetricType::Histogram { boundaries: None },
    }
}

fn required(definition: AttributeDefinition) -> RequiredAttributeDefinition {
    attribute(definition, true)
}

fn optional(definition: AttributeDefinition) -> RequiredAttributeDefinition {
    attribute(definition, false)
}

fn attribute(definition: AttributeDefinition, required: bool) -> RequiredAttributeDefinition {
    RequiredAttributeDefinition {
        definition,
        required,
    }
}

fn string_definition(description: &str) -> AttributeDefinition {
    AttributeDefinition {
        description: description.into(),
        sensitive: None,
        cardinality: None,
        value_type: AttributeType::String {
            values: None,
            examples: None,
        },
    }
}

fn number_definition(description: &str) -> AttributeDefinition {
    AttributeDefinition {
        description: description.into(),
        sensitive: None,
        cardinality: None,
        value_type: AttributeType::Number {
            values: None,
            examples: None,
        },
    }
}

//! Explicit logical call retries, callback order, terminal results and real adapter sends.

use crate::fixtures::{Finals, logical_request, message, transient_policy};
use opi_ai::message::{AssistantContent, AssistantMessage};
use opi_ai::retry::{
    RetryCallbackFailure, WholeCallRetryCallbacks, WholeCallRetryClassification,
    WholeCallRetryPolicy,
};
use opi_ai::retry_assistant_call;
use opi_ai::stream::StopReason;

#[path = "call_and_callbacks/frozen_target_wire.rs"]
mod frozen_target_wire;
#[path = "call_and_callbacks/quota_transport.rs"]
mod quota_transport;

fn error_text(stop: StopReason) -> Option<&'static str> {
    match stop {
        StopReason::Error => Some("overloaded_error"),
        _ => None,
    }
}

#[tokio::test]
async fn omitted_or_disabled_policy_makes_exactly_one_call() {
    let mut calls = 0u32;
    let failing = || {
        calls += 1;
        std::future::ready(message(StopReason::Error, Some("HTTP 503")))
    };
    let first = retry_assistant_call(
        failing,
        None,
        tokio_util::sync::CancellationToken::new(),
        None,
    )
    .await
    .expect("helper never fails without callback failures");
    assert_eq!(calls, 1);
    assert_eq!(first.stop_reason, StopReason::Error);

    let disabled = WholeCallRetryPolicy {
        enabled: false,
        ..transient_policy(9, 1)
    };
    let second = retry_assistant_call(
        || {
            calls += 1;
            std::future::ready(message(StopReason::Error, Some("HTTP 503")))
        },
        Some(&disabled),
        tokio_util::sync::CancellationToken::new(),
        None,
    )
    .await
    .unwrap();
    assert_eq!(calls, 2, "disabled must not add calls");
    assert_eq!(second.stop_reason, StopReason::Error);
}

#[tokio::test]
async fn transient_retry_succeeds_with_exact_callback_order_and_counts() {
    let policy = transient_policy(2, 1);
    let calls = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let events: std::sync::Arc<std::sync::Mutex<Vec<String>>> = Default::default();

    let calls_for_produce = calls.clone();
    let events_for_scheduled = events.clone();
    let events_for_start = events.clone();
    let events_for_finished = events.clone();

    let mut callbacks = WholeCallRetryCallbacks::new();
    callbacks.on_retry_scheduled = Some(Box::new(
        move |attempt: u32, max: u32, delay: u64, error: &str| {
            events_for_scheduled
                .lock()
                .unwrap()
                .push(format!("scheduled({attempt},{max},{delay},{error})"));
            Ok(())
        },
    ));
    callbacks.on_retry_attempt_start = Some(Box::new(move || {
        events_for_start
            .lock()
            .unwrap()
            .push("attempt_start".into());
        Ok(())
    }));
    callbacks.on_retry_finished = Some(Box::new(move |success: bool, attempt: u32, error| {
        events_for_finished.lock().unwrap().push(format!(
            "finished({success},{attempt},{})",
            error.unwrap_or("-")
        ));
        Ok(())
    }));

    let result = retry_assistant_call(
        move || {
            let n = calls_for_produce.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let stop = if n == 0 {
                StopReason::Error
            } else {
                StopReason::Stop
            };
            std::future::ready(message(stop, error_text(stop)))
        },
        Some(&policy),
        tokio_util::sync::CancellationToken::new(),
        Some(callbacks),
    )
    .await
    .unwrap();

    assert_eq!(result.stop_reason, StopReason::Stop);
    assert_eq!(
        calls.load(std::sync::atomic::Ordering::SeqCst),
        2,
        "one extra whole call only"
    );
    assert_eq!(
        *events.lock().unwrap(),
        vec![
            "scheduled(1,2,1,overloaded_error)".to_string(),
            "attempt_start".to_string(),
            "finished(true,1,-)".to_string(),
        ],
        "scheduled -> sleep -> attemptStart -> produce, finished exactly once"
    );
}

#[tokio::test]
async fn quota_veto_is_terminal_without_any_retry_scheduling() {
    let policy = transient_policy(3, 1);
    let calls = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let finished = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let calls_for_produce = calls.clone();
    let finished_sink = finished.clone();

    let mut callbacks = WholeCallRetryCallbacks::new();
    callbacks.on_retry_scheduled = Some(Box::new(|_, _, _, _| {
        panic!("quota vetoes must not schedule a retry")
    }));
    callbacks.on_retry_finished = Some(Box::new(move |_, _, _| {
        finished_sink.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }));

    let result = retry_assistant_call(
        move || {
            calls_for_produce.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            std::future::ready(message(
                StopReason::Error,
                Some("insufficient_quota: billing limit"),
            ))
        },
        Some(&policy),
        tokio_util::sync::CancellationToken::new(),
        Some(callbacks),
    )
    .await
    .unwrap();

    assert_eq!(result.stop_reason, StopReason::Error);
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(
        finished.load(std::sync::atomic::Ordering::SeqCst),
        0,
        "finished fires only when a retry was scheduled"
    );
}

#[tokio::test]
async fn context_overflow_stops_enabled_helper_without_retry_callbacks() {
    let policy = transient_policy(2, 1);
    let mut expected = message(
        StopReason::Error,
        Some("custom-provider API error (413): upstream returned 503"),
    );
    expected.usage.reported = true;
    assert!(opi_ai::message::is_context_overflow(&expected, None));
    let calls = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let callback_counts = std::sync::Arc::new(std::sync::Mutex::new([0u32; 3]));
    let calls_for_produce = calls.clone();
    let produced = expected.clone();
    let scheduled = callback_counts.clone();
    let started = callback_counts.clone();
    let finished = callback_counts.clone();
    let mut callbacks = WholeCallRetryCallbacks::new();
    callbacks.on_retry_scheduled = Some(Box::new(move |_, _, _, _| {
        scheduled.lock().unwrap()[0] += 1;
        Ok(())
    }));
    callbacks.on_retry_attempt_start = Some(Box::new(move || {
        started.lock().unwrap()[1] += 1;
        Ok(())
    }));
    callbacks.on_retry_finished = Some(Box::new(move |_, _, _| {
        finished.lock().unwrap()[2] += 1;
        Ok(())
    }));
    let result = retry_assistant_call(
        move || {
            calls_for_produce.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            std::future::ready(produced.clone())
        },
        Some(&policy),
        tokio_util::sync::CancellationToken::new(),
        Some(callbacks),
    )
    .await
    .unwrap();
    assert_eq!(
        result, expected,
        "terminal partial and known usage are retained"
    );
    assert_eq!(
        (
            calls.load(std::sync::atomic::Ordering::SeqCst),
            *callback_counts.lock().unwrap(),
        ),
        (1, [0, 0, 0]),
        "overflow must stop before scheduling, starting, or finishing a retry"
    );
}

#[tokio::test]
async fn exhausted_budget_returns_final_error_message() {
    let policy = transient_policy(2, 1);
    let calls = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let finals: Finals = Default::default();
    let calls_for_produce = calls.clone();
    let finals_sink = finals.clone();

    let mut callbacks = WholeCallRetryCallbacks::new();
    callbacks.on_retry_finished = Some(Box::new(move |success, attempt, error| {
        finals_sink
            .lock()
            .unwrap()
            .push((success, attempt, error.map(str::to_string)));
        Ok(())
    }));

    let result = retry_assistant_call(
        move || {
            let n = calls_for_produce.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let text = if n < 2 {
                "HTTP 503"
            } else {
                "insufficient_quota after retries"
            };
            std::future::ready(message(StopReason::Error, Some(text)))
        },
        Some(&policy),
        tokio_util::sync::CancellationToken::new(),
        Some(callbacks),
    )
    .await
    .unwrap();

    assert_eq!(
        calls.load(std::sync::atomic::Ordering::SeqCst),
        3,
        "initial call + max_retries extra calls"
    );
    assert_eq!(
        result.error_message.as_deref(),
        Some("insufficient_quota after retries")
    );
    assert_eq!(
        *finals.lock().unwrap(),
        vec![(false, 2, Some("insufficient_quota after retries".into()))]
    );
}

#[tokio::test]
async fn aborted_after_a_scheduled_retry_returns_without_further_calls() {
    let policy = transient_policy(3, 1);
    let calls = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let finals: Finals = Default::default();
    let calls_for_produce = calls.clone();
    let finals_sink = finals.clone();

    let mut callbacks = WholeCallRetryCallbacks::new();
    callbacks.on_retry_finished = Some(Box::new(move |success, attempt, error| {
        finals_sink
            .lock()
            .unwrap()
            .push((success, attempt, error.map(str::to_string)));
        Ok(())
    }));

    let result = retry_assistant_call(
        move || {
            let n = calls_for_produce.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let stop = if n == 0 {
                StopReason::Error
            } else {
                StopReason::Aborted
            };
            std::future::ready(message(stop, error_text(stop)))
        },
        Some(&policy),
        tokio_util::sync::CancellationToken::new(),
        Some(callbacks),
    )
    .await
    .unwrap();

    assert_eq!(result.stop_reason, StopReason::Aborted);
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 2);
    assert_eq!(*finals.lock().unwrap(), vec![(false, 1, None)]);
}

#[tokio::test]
async fn callback_failure_returns_explicitly_and_stops_the_loop() {
    let policy = transient_policy(3, 1);
    let calls = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let calls_for_produce = calls.clone();

    let mut callbacks = WholeCallRetryCallbacks::new();
    callbacks.on_retry_scheduled = Some(Box::new(|_, _, _, _| {
        Err(RetryCallbackFailure {
            detail: "observer channel closed".into(),
        })
    }));

    let error = retry_assistant_call(
        move || {
            calls_for_produce.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            std::future::ready(message(StopReason::Error, Some("HTTP 503")))
        },
        Some(&policy),
        tokio_util::sync::CancellationToken::new(),
        Some(callbacks),
    )
    .await
    .expect_err("callback failure must surface");

    assert!(
        matches!(error, opi_ai::retry::WholeCallRetryError::Callback(ref failure)
            if failure.detail == "observer channel closed"),
        "typed callback failure, got {error:?}"
    );
    assert_eq!(
        calls.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "no further calls after a callback failure"
    );
}

#[tokio::test]
async fn responses_failed_sse_reaches_the_explicit_whole_call_policy() {
    use futures_util::StreamExt;
    use opi_ai::auth::{AuthProvenanceSource, AuthScheme, StaticAuthResolver};
    use opi_ai::provider::{Provider, ProviderError};
    use opi_ai::providers::openai_responses::OpenAiResponsesProvider;
    use opi_ai::retry::{AcquisitionPolicy, AcquisitionSeal, classify_whole_call_retry};
    use opi_ai::stream::AssistantStreamEvent;
    use opi_telemetry::{AttributeValue, SpanStatus, memory::Recorder};
    use serde_json::{Value, json};
    use std::sync::Arc;
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers};

    let fixture: Value =
        serde_json::from_str(include_str!("../data/responses-failed.json")).unwrap();
    let mut mismatches = Vec::new();
    for case in fixture["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let decision = case["decision"].as_str().unwrap();
        let failed = json!({
            "type": "response.failed",
            "response": {"status":"failed","error":case["error"],"usage":{"input_tokens":21,"output_tokens":9}}
        });
        let body: String = fixture["prefix"]
            .as_array()
            .unwrap()
            .iter()
            .chain([&failed, &fixture["afterFailure"]])
            .map(|event| {
                format!(
                    "event: {}\ndata: {event}\n\n",
                    event["type"].as_str().unwrap()
                )
            })
            .collect();
        let server = MockServer::start().await;
        Mock::given(matchers::method("POST"))
            .and(matchers::path("/v1/responses"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(body),
            )
            .mount(&server)
            .await;
        let provider = OpenAiResponsesProvider::new(Some(server.uri()));
        let model = provider.models()[0].id.clone();
        let model_ref = format!("openai-responses:{model}");
        let mut models = opi_ai::Models::new();
        models
            .register_route(
                Box::new(provider),
                Arc::new(StaticAuthResolver::new(
                    AuthScheme::Bearer,
                    secrecy::SecretString::from("fixture-key"),
                )),
                AuthProvenanceSource::Static,
                Default::default(),
            )
            .unwrap();
        let recorder = Recorder::default();
        let mut request = logical_request();
        request.model = model_ref.clone();
        request.telemetry_context = Some(recorder.context());
        let cancel = request.cancel.clone();
        let models = &models;
        let mut calls = 0;
        let message = retry_assistant_call(
            || {
                calls += 1;
                let request = request.clone();
                let model_ref = &model_ref;
                async move {
                    let call = models
                        .prepare_call_with_policy(
                            model_ref,
                            request,
                            AcquisitionPolicy::with_max_retries(1),
                        )
                        .await
                        .unwrap();
                    let mut stream = call.start_attempt_with_telemetry().unwrap();
                    let mut deltas = String::new();
                    let failure = loop {
                        match stream.next().await.expect("the attempt emits a terminal") {
                            Ok(AssistantStreamEvent::TextDelta { delta, .. }) => {
                                deltas.push_str(&delta)
                            }
                            Ok(
                                AssistantStreamEvent::Done { .. }
                                | AssistantStreamEvent::Error { .. },
                            ) => {
                                panic!("expected the typed terminal for {name}")
                            }
                            Ok(_) => {}
                            Err(failure) => break failure,
                        }
                    };
                    assert_eq!(deltas, "partial", "{name}");
                    assert!(stream.next().await.is_none(), "{name}");
                    assert_eq!(stream.result().await, Err(failure.clone()), "{name}");
                    assert!(
                        matches!(failure.cause(), ProviderError::StreamError(_)),
                        "{name}"
                    );
                    assert!(!failure.cause().is_retryable(), "{name}");
                    assert!(call.retry_admission().is_none(), "{name}");
                    assert_eq!(
                        call.acquisition_seal(),
                        Some(AcquisitionSeal::PostHandoff),
                        "{name}"
                    );
                    assert!(call.start_attempt().is_err(), "{name}");
                    assert!(!format!("{failure:?}").contains("PRIVATE_CANARY"), "{name}");
                    failure.into_parts().0
                }
            },
            Some(&transient_policy(1, 0)),
            cancel.clone(),
            None,
        )
        .await
        .unwrap();
        let sent = server.received_requests().await.unwrap();
        assert_eq!(sent.len(), calls, "{name}: one send per logical call");
        assert!(
            sent.iter().all(|request| request.body == sent[0].body),
            "{name}"
        );
        assert_eq!(message.stop_reason, StopReason::Error, "{name}");
        assert_eq!(message.raw_stop_reason.as_deref(), Some("failed"), "{name}");
        assert!(
            matches!(message.content.as_slice(), [AssistantContent::Text { text, signature: None }] if text == "partial"),
            "{name}: the terminal retains the streamed prefix"
        );
        assert_eq!(message.provider, "openai-responses", "{name}");
        assert_eq!(message.model, model, "{name}");
        assert_eq!(
            message.response_id.as_deref(),
            Some("resp_partial"),
            "{name}"
        );
        assert_eq!(
            message.response_model.as_deref(),
            Some("wire-model"),
            "{name}"
        );
        assert!(!message.usage.reported, "{name}");
        assert!(message.projection.is_some(), "{name}");
        assert!(!cancel.is_cancelled(), "{name}");
        let diagnostic = message.error_message.as_deref().unwrap();
        let serialized = serde_json::to_string(&message).unwrap();
        assert!(!serialized.contains("PRIVATE_CANARY"), "{name}");
        assert_eq!(
            serde_json::from_str::<AssistantMessage>(&serialized).unwrap(),
            message,
            "{name}"
        );
        let spans = recorder.spans();
        assert_eq!(spans.len(), calls, "{name}");
        for span in &spans {
            assert_eq!(span.name, "pi.ai.request", "{name}");
            assert!(span.end_sequence.is_some(), "{name}");
            assert_eq!(
                span.attributes.get("stop"),
                Some(&AttributeValue::String("error".into())),
                "{name}"
            );
            let SpanStatus::Error { error: Some(error) } = &span.status else {
                panic!("expected an error span for {name}");
            };
            assert_eq!(error.name, "provider_error", "{name}");
            assert_eq!(error.message, diagnostic, "{name}");
        }
        let retryable =
            classify_whole_call_retry(&message) == WholeCallRetryClassification::Transient;
        let overflow = opi_ai::message::is_context_overflow(&message, None);
        let quota = diagnostic.contains("quota exceeded");
        println!(
            "responses failed observation {}",
            json!({
                "name":name,"decision":decision,"retryable":retryable,"overflow":overflow,
                "quota":quota,"diagnostic":diagnostic,"sends":sent.len(),"spans":spans.len()
            })
        );
        if (retryable, overflow, quota, calls)
            != (
                decision == "transient",
                decision == "overflow",
                decision == "quota",
                if decision == "transient" { 2 } else { 1 },
            )
        {
            mismatches.push(name);
        }
    }
    assert!(
        mismatches.is_empty(),
        "failure decision mismatches: {mismatches:?}"
    );
}

//! Captured bindings, execution before consumption and owned calls.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use futures_util::StreamExt;
use opi_ai::Models;
use opi_ai::api::contract::SimpleOptions;
use opi_ai::auth::{AuthProvenanceSource, AuthResolver, AuthScheme, ResolvedAuth};
use opi_ai::models::ChatModelRecord;
use opi_ai::types::ApiId;
use secrecy::SecretString;

use super::fixtures::{
    EchoChatApi, EchoOptions, Observed, assistant_text, counting_resolver, echo_record_info,
    models_request,
};

#[tokio::test]
async fn models_old_call_survives_provider_mutation() {
    let mut models = Models::new();
    let observed: Observed = Arc::default();
    let record_info = echo_record_info(2_048);
    let handle = models
        .register_chat_api(
            Arc::new(EchoChatApi {
                api: ApiId::new("custom-chat-v1"),
                provider_id: "custom".into(),
                record: record_info.clone(),
                text: "before-mutation".into(),
                observed: observed.clone(),
            }),
            counting_resolver(),
            AuthProvenanceSource::Static,
        )
        .unwrap();

    // Create (but do not poll) a call; then replace the registration.
    let record = ChatModelRecord::new("custom", record_info.clone());
    let call = models.stream(
        record.clone(),
        models_request(),
        handle.options(EchoOptions {
            label: "frozen".into(),
        }),
    );

    models
        .register_chat_api(
            Arc::new(EchoChatApi {
                api: ApiId::new("custom-chat-v1"),
                provider_id: "custom".into(),
                record: record_info,
                text: "after-mutation".into(),
                observed: Arc::default(),
            }),
            counting_resolver(),
            AuthProvenanceSource::Static,
        )
        .unwrap();

    // The already-created call keeps its captured binding.
    let message = call.result().await.unwrap();
    assert_eq!(assistant_text(&message), "before-mutation");

    // A NEW call on the same handle observes the replacement.
    let message = models
        .complete(
            record,
            models_request(),
            handle.options(EchoOptions {
                label: "fresh".into(),
            }),
        )
        .await
        .unwrap();
    assert_eq!(assistant_text(&message), "after-mutation");
}

#[tokio::test]
async fn models_six_ordinary_entries_freeze_binding_and_execute_before_consumption() {
    fn owned<F: std::future::Future + Send + 'static>(future: F) -> F {
        future
    }

    let mut models = Models::new();
    let observed: Observed = Arc::default();
    let record_info = echo_record_info(2_048);
    let reads = counting_resolver();
    let handle = models
        .register_chat_api(
            Arc::new(EchoChatApi {
                api: ApiId::new("custom-chat-v1"),
                provider_id: "custom".into(),
                record: record_info.clone(),
                text: "before-mutation".into(),
                observed: observed.clone(),
            }),
            reads.clone() as Arc<dyn AuthResolver>,
            AuthProvenanceSource::Static,
        )
        .unwrap();
    let record = ChatModelRecord::new("custom", record_info.clone());
    let native = owned(models.complete(
        record.clone(),
        models_request(),
        handle.options(EchoOptions {
            label: "native".into(),
        }),
    ));
    let simple =
        owned(models.complete_simple(record.clone(), models_request(), SimpleOptions::default()));
    let direct = owned(models.complete_direct(
        record.clone(),
        models_request(),
        handle.options(EchoOptions {
            label: "direct".into(),
        }),
        ResolvedAuth {
            env: Default::default(),
            scheme: AuthScheme::Bearer,
            secret: SecretString::from("direct-secret"),
            base_url: None,
            account_id: None,
            provenance: opi_ai::AuthProvenance::default(),
        },
    ));
    let native_stream = models.stream(
        record.clone(),
        models_request(),
        handle.options(EchoOptions {
            label: "native-stream".into(),
        }),
    );
    let simple_stream =
        models.stream_simple(record.clone(), models_request(), SimpleOptions::default());
    let direct_stream = models.stream_direct(
        record.clone(),
        models_request(),
        handle.options(EchoOptions {
            label: "direct-stream".into(),
        }),
        opi_ai::test_support::resolved_bearer_auth(),
    );

    let replacement_reads = counting_resolver();
    models
        .register_chat_api(
            Arc::new(EchoChatApi {
                api: ApiId::new("custom-chat-v1"),
                provider_id: "custom".into(),
                record: record_info,
                text: "after-mutation".into(),
                observed: Arc::default(),
            }),
            replacement_reads.clone() as Arc<dyn AuthResolver>,
            AuthProvenanceSource::Static,
        )
        .unwrap();
    let fresh = owned(models.complete_simple(record, models_request(), SimpleOptions::default()));
    drop(models);
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while observed.lock().unwrap().len() < 6 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("all six entries execute without consumer polling");

    for future in [
        tokio::spawn(native),
        tokio::spawn(simple),
        tokio::spawn(direct),
        tokio::spawn(native_stream.result()),
        tokio::spawn(simple_stream.result()),
        tokio::spawn(direct_stream.result()),
    ] {
        assert_eq!(
            assistant_text(&future.await.unwrap().unwrap()),
            "before-mutation"
        );
    }
    assert_eq!(
        assistant_text(&tokio::spawn(fresh).await.unwrap().unwrap()),
        "after-mutation"
    );
    assert_eq!(reads.calls.load(Ordering::SeqCst), 4);
    assert_eq!(replacement_reads.calls.load(Ordering::SeqCst), 1);
    let calls = observed.lock().unwrap();
    assert_eq!(calls.len(), 6);
    let direct = calls
        .iter()
        .find(|call| call.options_label == "direct")
        .unwrap();
    assert_eq!(direct.auth_secret, "direct-secret");
}

#[tokio::test]
async fn models_auth_runs_without_consumption_and_call_is_send_static() {
    let mut models = Models::new();
    let observed: Observed = Arc::default();
    let record_info = echo_record_info(2_048);
    let reads = counting_resolver();
    let handle = models
        .register_chat_api(
            Arc::new(EchoChatApi {
                api: ApiId::new("custom-chat-v1"),
                provider_id: "custom".into(),
                record: record_info.clone(),
                text: "lazy auth".into(),
                observed,
            }),
            reads.clone() as Arc<dyn AuthResolver>,
            AuthProvenanceSource::Static,
        )
        .unwrap();

    let record = ChatModelRecord::new("custom", record_info);
    let call = models.stream(
        record,
        models_request(),
        handle.options(EchoOptions {
            label: "lazy".into(),
        }),
    );
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while reads.calls.load(Ordering::SeqCst) == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("auth starts without polling the consumer");

    // The call is Send + 'static: it can cross into a fresh runtime task.
    let consumed = tokio::spawn(async move {
        let mut call = call;
        let first = call.next().await;
        let saw_event = first.is_some();
        let message = call.result().await.unwrap();
        (saw_event, assistant_text(&message))
    })
    .await
    .unwrap();
    assert!(consumed.0);
    assert_eq!(consumed.1, "lazy auth");
    assert_eq!(reads.calls.load(Ordering::SeqCst), 1);
}

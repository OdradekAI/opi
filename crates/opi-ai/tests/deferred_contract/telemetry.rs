//! The explicit telemetry carrier at the Deferred backend boundary.

use opi_ai::api::contract::DeferredFetchCall;
use opi_ai::telemetry::TelemetryContext;
use opi_ai::test_support::{self, ScriptedDeferredBackend};

use super::fixtures::registered_collection;

#[tokio::test]
async fn fetch_and_cancel_carry_the_frozen_telemetry_carrier() {
    let backend = std::sync::Arc::new(ScriptedDeferredBackend::new("test", "mock-model"));
    let handle = backend.submit(test_support::text_response("ready"), 1, None);
    let (collection, _record) = registered_collection(&backend);

    let fetch_call = DeferredFetchCall {
        telemetry_context: Some(TelemetryContext::noop()),
        ..Default::default()
    };
    collection
        .fetch_deferred(&handle, fetch_call)
        .await
        .expect("pending");
    assert_eq!(backend.fetch_telemetry_present(), vec![true]);

    // Default calls carry no carrier: no automatic capture.
    collection
        .fetch_deferred(&handle, DeferredFetchCall::default())
        .await
        .expect("ready");
    assert_eq!(backend.fetch_telemetry_present(), vec![true, false]);

    let cancel_call = opi_ai::api::contract::DeferredCancelCall {
        telemetry_context: Some(TelemetryContext::noop()),
        ..Default::default()
    };
    collection
        .cancel_deferred(&handle, cancel_call)
        .await
        .expect("cancel");
    assert_eq!(backend.cancel_telemetry_present(), vec![true]);
}

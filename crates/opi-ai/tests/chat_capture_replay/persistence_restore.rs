//! Canonical AssistantMessage serialization and restoration of actual producer results.

use opi_ai::WireApi;
use serde_json::json;

use super::fixtures::{BUILTIN_CASES, BuiltinCapture, capture_builtin, reload_message};

#[tokio::test]
async fn builtin_capture_round_trips_canonical_assistant_message() {
    for case @ (provider, wire, _) in BUILTIN_CASES {
        let BuiltinCapture {
            peer,
            message: captured,
            ..
        } = capture_builtin(case, 1).await;
        let (encoded, restored) = reload_message(&captured);
        assert_eq!(restored, captured, "restore {provider} on {wire:?}");
        assert_eq!(serde_json::to_vec(&restored).unwrap(), encoded);
        assert_eq!(
            serde_json::to_value(&restored).unwrap()["raw_stop_reason"],
            if wire != WireApi::OpenAiCompletions {
                json!("completed")
            } else {
                json!("stop")
            },
            "the result retains the provider's original finish fact"
        );
        peer.verify().await;
    }
}

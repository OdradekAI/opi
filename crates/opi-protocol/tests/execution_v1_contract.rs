//! Contract tests: fixture-driven frame parsing, the request-id invariants
//! (missing, mismatched-type, empty, cross-request), NativeString losslessness
//! (incl. a proptest over arbitrary bytes), version negotiation, and the
//! stateful Session over multi-frame sequences.

use std::collections::BTreeSet;
use std::path::PathBuf;

use opi_protocol::execution::v1;
use opi_protocol::execution::v1::{
    BackendToHost, Bounds, HostToBackend, NativeString, ProtocolId, Session, SessionError,
};

use proptest::prelude::*;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn read_fixture(name: &str) -> String {
    std::fs::read_to_string(fixtures_dir().join(name))
        .unwrap_or_else(|e| panic!("reading fixture {name}: {e}"))
}

// --- fixture parsing: valid frames parse into the right direction ----------

#[test]
fn valid_frames_parse_into_their_directions() {
    let init: HostToBackend = serde_json::from_str(&read_fixture("valid_initialize.json")).unwrap();
    assert_eq!(init.kind(), "initialize");
    assert_eq!(init.request_id().as_str(), "r1");

    let ready: BackendToHost = serde_json::from_str(&read_fixture("valid_ready.json")).unwrap();
    assert_eq!(ready.kind(), "ready");

    let started: BackendToHost = serde_json::from_str(&read_fixture("valid_started.json")).unwrap();
    assert_eq!(started.kind(), "started");

    let completed: BackendToHost =
        serde_json::from_str(&read_fixture("valid_completed.json")).unwrap();
    assert_eq!(completed.kind(), "completed");

    let failed: BackendToHost = serde_json::from_str(&read_fixture("valid_failed.json")).unwrap();
    assert_eq!(failed.kind(), "failed");
}

// --- request-id invariants (DoD: fixtures reject missing/mismatched/cross) --

#[test]
fn missing_request_id_rejected() {
    assert!(
        serde_json::from_str::<BackendToHost>(&read_fixture("invalid_missing_id.json")).is_err()
    );
}

#[test]
fn mismatched_request_id_type_rejected() {
    // "mismatched" = present but wrong shape/type (here an integer).
    assert!(
        serde_json::from_str::<BackendToHost>(&read_fixture("invalid_mismatched_id.json")).is_err()
    );
}

#[test]
fn empty_request_id_rejected() {
    assert!(serde_json::from_str::<BackendToHost>(&read_fixture("invalid_empty_id.json")).is_err());
}

#[test]
fn unknown_field_rejected() {
    assert!(
        serde_json::from_str::<BackendToHost>(&read_fixture("invalid_unknown_field.json")).is_err()
    );
}

#[test]
fn unknown_tag_rejected() {
    assert!(
        serde_json::from_str::<BackendToHost>(&read_fixture("invalid_unknown_tag.json")).is_err()
    );
}

#[test]
fn malformed_json_fixture_rejected() {
    assert!(
        serde_json::from_str::<BackendToHost>(&read_fixture("invalid_malformed.json")).is_err()
    );
}

#[test]
fn invalid_base64_fixture_rejected() {
    assert!(serde_json::from_str::<BackendToHost>(&read_fixture("invalid_base64.json")).is_err());
}

#[test]
fn nested_diagnostic_unknown_field_rejected() {
    assert!(
        serde_json::from_str::<BackendToHost>(&read_fixture(
            "invalid_nested_diagnostic_unknown_field.json"
        ))
        .is_err()
    );
}

#[test]
fn ready_requires_implementation_identity() {
    assert!(
        serde_json::from_str::<BackendToHost>(&read_fixture(
            "invalid_ready_missing_implementation.json"
        ))
        .is_err()
    );
}

#[test]
fn ready_rejects_empty_implementation_identity() {
    assert!(
        serde_json::from_str::<BackendToHost>(&read_fixture(
            "invalid_ready_empty_implementation.json"
        ))
        .is_err()
    );
}

#[test]
fn ready_rejects_unknown_identity_field() {
    assert!(
        serde_json::from_str::<BackendToHost>(&read_fixture("invalid_ready_unknown_field.json"))
            .is_err()
    );
}

// --- NativeString losslessness ---------------------------------------------

#[test]
fn nativestring_nonutf8_fixture_round_trips_to_expected_bytes() {
    let exe: HostToBackend =
        serde_json::from_str(&read_fixture("valid_execute_nonutf8.json")).unwrap();
    let args = match exe {
        HostToBackend::Execute(p) => p.args,
        other => panic!("expected execute, got {:?}", other.kind()),
    };
    // args[0] is the U+E000 escape of byte 0xFF.
    assert_eq!(args[0].as_bytes(), &[0xFF], "companion expected-bytes file");
    assert_eq!(args[1].as_bytes(), b"ok");
}

proptest! {
    #[test]
    fn nativestring_round_trips_arbitrary_bytes(bytes in prop::collection::vec(any::<u8>(), 0..256)) {
        let native = NativeString::from_bytes(bytes.clone());
        let wire = native.to_wire_string();
        // serde_json must accept the wire string (no surrogate; all valid scalars).
        let json = serde_json::to_string(&serde_json::Value::String(wire.clone())).unwrap();
        let reparsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let wire_back = reparsed.as_str().unwrap();
        prop_assert_eq!(wire_back, wire.as_str());
        let bytes_back = NativeString::from_wire_string(wire_back).unwrap();
        prop_assert_eq!(bytes_back, bytes);
    }
}

// --- version negotiation ----------------------------------------------------

#[test]
fn negotiation_prefers_host_order() {
    let proto = ProtocolId::new("command-execution-jsonl-v1");
    let backend = [proto.clone()].into_iter().collect::<BTreeSet<_>>();
    let host_order = [proto.clone()];
    assert_eq!(v1::select(&host_order, &backend), Ok(proto));
}

// --- stateful Session over multi-frame sequences ---------------------------

#[test]
fn valid_sequence_observed_with_cumulative_accounting() {
    let lines = read_fixture("sequence_valid.jsonl");
    let mut session = Session::new(Bounds::DEFAULT).unwrap();
    let mut lines = lines.lines();
    // First line is the host initialize; the rest are backend frames.
    session
        .feed_host_line(lines.next().unwrap().as_bytes())
        .unwrap();
    for line in lines {
        session.feed_backend_line(line.as_bytes()).unwrap();
    }
    // The valid sequence has one stdout chunk "aGVsbG8K" which decodes to
    // "hello\n" (6 bytes); cumulative output must reflect decoded bytes.
    assert_eq!(session.cumulative_output(), 6);
}

#[test]
fn cross_request_sequence_rejected() {
    let lines = read_fixture("sequence_cross_request.jsonl");
    let mut session = Session::new(Bounds::DEFAULT).unwrap();
    let mut lines = lines.lines();
    // initialize carries id "r1" (seeds the session).
    session
        .feed_host_line(lines.next().unwrap().as_bytes())
        .unwrap();
    // A backend frame carrying a foreign id "r2" is a cross-request violation.
    let err = session
        .feed_backend_line(lines.next().unwrap().as_bytes())
        .unwrap_err();
    assert!(matches!(err, SessionError::CrossRequestId { .. }));
}

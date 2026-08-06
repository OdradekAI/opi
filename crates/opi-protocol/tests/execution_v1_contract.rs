//! Contract tests: fixture-driven frame parsing, the request-id invariants
//! (missing, mismatched-type, empty, cross-request), NativeString losslessness
//! (incl. a proptest over arbitrary bytes), version negotiation, and the
//! stateful Session over multi-frame sequences, the five declared bounds,
//! once-per-execution frames, and host cancellation.

use std::collections::BTreeSet;
use std::path::PathBuf;

use opi_protocol::execution::v1;
use opi_protocol::execution::v1::codec::{LineReader, encode_backend, encode_host};
use opi_protocol::execution::v1::frames::{AcceptedPayload, FailedPayload, StdoutPayload};
use opi_protocol::execution::v1::{
    BackendToHost, Base64Bytes, Bounds, CodecError, FailureCode, FailurePhase, HostToBackend,
    NativeString, ProtocolId, RequestId, Session, SessionError,
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
fn protocol_id_rejects_empty_construction_and_deserialization() {
    assert!(ProtocolId::new("").is_err());
    assert!(serde_json::from_str::<ProtocolId>(r#""""#).is_err());
}

#[test]
fn host_initialize_rejects_empty_protocol_and_unknown_field_fixtures() {
    for name in [
        "invalid_initialize_empty_protocol.json",
        "invalid_initialize_unknown_field.json",
    ] {
        let mut session = Session::new(Bounds::DEFAULT).unwrap();
        assert!(
            session
                .feed_host_line(read_fixture(name).as_bytes())
                .is_err(),
            "host fixture {name} must be rejected"
        );
    }
}

#[test]
fn negotiation_prefers_host_order() {
    let proto = ProtocolId::new("command-execution-jsonl-v1").unwrap();
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

#[test]
fn failed_message_enforces_diagnostics_bound_on_encode_and_decode() {
    let bounds = Bounds {
        max_line_size: 4096,
        max_decoded_chunk_size: 8,
        max_configuration_size: 16,
        max_diagnostics_size: 8,
        max_cumulative_output: 64,
    };
    let failed = |message: &str| {
        BackendToHost::Failed(FailedPayload {
            request_id: RequestId::new("r1".to_string()).unwrap(),
            code: FailureCode::Failed,
            phase: FailurePhase::Handshake,
            message: Some(message.to_string()),
            diagnostics: vec![],
        })
    };

    let exact = failed("12345678");
    assert!(encode_backend(&exact, &bounds).is_ok());
    Session::new(bounds)
        .unwrap()
        .feed_backend_line(&serde_json::to_vec(&exact).unwrap())
        .unwrap();

    let oversized = failed("123456789");
    assert!(matches!(
        encode_backend(&oversized, &bounds),
        Err(CodecError::DiagnosticsTooLarge {
            actual: 9,
            limit: 8
        })
    ));
    let err = Session::new(bounds)
        .unwrap()
        .feed_backend_line(&serde_json::to_vec(&oversized).unwrap())
        .unwrap_err();
    assert!(matches!(
        err,
        SessionError::Codec(CodecError::DiagnosticsTooLarge {
            actual: 9,
            limit: 8
        })
    ));
}

#[test]
fn cross_request_output_is_rejected_before_accounting() {
    let mut session = Session::new(Bounds::DEFAULT).unwrap();
    session
        .observe_backend(&BackendToHost::Accepted(AcceptedPayload {
            request_id: RequestId::new("r1".to_string()).unwrap(),
        }))
        .unwrap();

    let err = session
        .observe_backend(&BackendToHost::Stdout(StdoutPayload {
            request_id: RequestId::new("foreign".to_string()).unwrap(),
            data: Base64Bytes::from_bytes(b"oops"),
        }))
        .unwrap_err();
    assert!(matches!(err, SessionError::CrossRequestId { .. }));
    assert_eq!(session.cumulative_output(), 0);
}

#[test]
fn all_declared_bounds_enforce_exact_and_over_boundary() {
    for (size, accepted) in [(8usize, true), (9, false)] {
        let bounds = Bounds {
            max_line_size: 8,
            max_decoded_chunk_size: 0,
            max_configuration_size: 0,
            max_diagnostics_size: 0,
            max_cumulative_output: 0,
        };
        let mut input = vec![b'x'; size];
        input.push(b'\n');
        let result =
            LineReader::new(std::io::Cursor::new(input), bounds).read_line(&mut Vec::new());
        assert_eq!(result.is_ok(), accepted, "line size {size}");
    }

    let bounds = Bounds {
        max_line_size: 4096,
        max_decoded_chunk_size: 8,
        max_configuration_size: 5,
        max_diagnostics_size: 8,
        max_cumulative_output: 10,
    };
    for (config, accepted) in [("123", true), ("1234", false)] {
        let line = format!(
            r#"{{"type":"initialize","payload":{{"request_id":"r1","deadline_ms":1,"adapter_config":"{config}","supported_protocols":["command-execution-jsonl-v1"]}}}}"#
        );
        let result = Session::new(bounds)
            .unwrap()
            .feed_host_line(line.as_bytes());
        assert_eq!(result.is_ok(), accepted, "configuration {config}");
    }

    for (message, accepted) in [("12345678", true), ("123456789", false)] {
        let line = format!(
            r#"{{"type":"diagnostic","payload":{{"request_id":"r1","message":"{message}"}}}}"#
        );
        let result = Session::new(bounds)
            .unwrap()
            .feed_backend_line(line.as_bytes());
        assert_eq!(result.is_ok(), accepted, "diagnostic {message}");
    }

    for (size, accepted) in [(8usize, true), (9, false)] {
        let frame = BackendToHost::Stdout(StdoutPayload {
            request_id: RequestId::new("r1".to_string()).unwrap(),
            data: Base64Bytes::from_bytes(vec![0; size]),
        });
        let result = Session::new(bounds).unwrap().observe_backend(&frame);
        assert_eq!(result.is_ok(), accepted, "decoded chunk size {size}");
    }

    let mut session = Session::new(bounds).unwrap();
    for size in [6usize, 4] {
        session
            .observe_backend(&BackendToHost::Stdout(StdoutPayload {
                request_id: RequestId::new("r1".to_string()).unwrap(),
                data: Base64Bytes::from_bytes(vec![0; size]),
            }))
            .unwrap();
    }
    assert_eq!(session.cumulative_output(), 10);
    assert!(matches!(
        session.observe_backend(&BackendToHost::Stdout(StdoutPayload {
            request_id: RequestId::new("r1".to_string()).unwrap(),
            data: Base64Bytes::from_bytes([0]),
        })),
        Err(SessionError::CumulativeOutputExceeded {
            cumulative: 11,
            limit: 10
        })
    ));
}

#[test]
fn once_per_execution_frames_reject_duplicates_in_both_directions() {
    for (fixture, kind) in [
        ("valid_initialize.json", "initialize"),
        ("valid_execute.json", "execute"),
    ] {
        let line = read_fixture(fixture);
        let mut session = Session::new(Bounds::DEFAULT).unwrap();
        session.feed_host_line(line.as_bytes()).unwrap();
        assert!(matches!(
            session.feed_host_line(line.as_bytes()),
            Err(SessionError::DuplicateFrame { frame }) if frame == kind
        ));
    }

    for (fixture, kind) in [
        ("valid_ready.json", "ready"),
        ("valid_accepted.json", "accepted"),
        ("valid_started.json", "started"),
        ("valid_completed.json", "completed"),
        ("valid_failed.json", "failed"),
    ] {
        let line = read_fixture(fixture);
        let mut session = Session::new(Bounds::DEFAULT).unwrap();
        session.feed_backend_line(line.as_bytes()).unwrap();
        assert!(matches!(
            session.feed_backend_line(line.as_bytes()),
            Err(SessionError::DuplicateFrame { frame }) if frame == kind
        ));
    }
}

#[test]
fn cancel_may_repeat_but_still_obeys_host_request_identity() {
    let line = read_fixture("valid_cancel.json");
    let mut session = Session::new(Bounds::DEFAULT).unwrap();
    session.feed_host_line(line.as_bytes()).unwrap();
    session.feed_host_line(line.as_bytes()).unwrap();

    let foreign = line.replace("r1", "foreign");
    assert!(matches!(
        session.feed_host_line(foreign.as_bytes()),
        Err(SessionError::CrossRequestId { .. })
    ));
}

#[test]
fn control_character_configuration_uses_serialized_json_byte_cap() {
    let adapter_config = serde_json::json!("\0");
    let serialized_size = serde_json::to_vec(&adapter_config).unwrap().len();
    assert_eq!(
        serialized_size, 8,
        "NUL must serialize as a six-byte escape"
    );
    let bounds = Bounds {
        max_line_size: serialized_size + 256,
        max_decoded_chunk_size: 0,
        max_configuration_size: serialized_size,
        max_diagnostics_size: 0,
        max_cumulative_output: 0,
    };
    let frame = HostToBackend::Initialize(v1::frames::InitializePayload {
        request_id: RequestId::new("r1".to_string()).unwrap(),
        deadline_ms: 1,
        adapter_config,
        supported_protocols: vec![ProtocolId::new(v1::WIRE_IDENTITY).unwrap()],
    });

    let line = encode_host(&frame, &bounds).unwrap();
    Session::new(bounds)
        .unwrap()
        .feed_host_line(line.as_bytes())
        .unwrap();

    let oversized = HostToBackend::Initialize(v1::frames::InitializePayload {
        request_id: RequestId::new("r1".to_string()).unwrap(),
        deadline_ms: 1,
        adapter_config: serde_json::json!("\0\0"),
        supported_protocols: vec![ProtocolId::new(v1::WIRE_IDENTITY).unwrap()],
    });
    assert!(matches!(
        encode_host(&oversized, &bounds),
        Err(CodecError::ConfigurationTooLarge { .. })
    ));
}

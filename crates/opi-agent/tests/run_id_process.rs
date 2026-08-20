use std::path::Path;
use std::process::Command;

use opi_agent::evidence::{IdentityAllocator, RunId};

const CHILD_OUTPUT_ENV: &str = "OPI_AGENT_RUN_ID_CHILD_OUTPUT";

#[test]
#[ignore = "invoked as a subprocess helper"]
fn run_id_subprocess_child() {
    let Some(output) = std::env::var_os(CHILD_OUTPUT_ENV) else {
        return;
    };
    let run_id = IdentityAllocator::new().run_id();
    std::fs::write(
        Path::new(&output),
        serde_json::to_vec(&run_id).expect("run id serializes"),
    )
    .expect("child writes its run id");
}

#[test]
fn run_ids_are_distinct_uuid_v7_values_across_processes() {
    let temp = tempfile::tempdir().expect("temporary output directory");
    let first_path = temp.path().join("first.json");
    let second_path = temp.path().join("second.json");

    for output in [&first_path, &second_path] {
        let status = Command::new(std::env::current_exe().expect("current test executable"))
            .args([
                "--exact",
                "run_id_subprocess_child",
                "--ignored",
                "--nocapture",
            ])
            .env(CHILD_OUTPUT_ENV, output)
            .status()
            .expect("run child test process");
        assert!(status.success(), "child process failed: {status}");
    }

    let first_json = std::fs::read_to_string(first_path).expect("first child output");
    let second_json = std::fs::read_to_string(second_path).expect("second child output");
    let first: RunId = serde_json::from_str(&first_json).expect("first id deserializes");
    let second: RunId = serde_json::from_str(&second_json).expect("second id deserializes");

    assert_ne!(first, second);
    for (run_id, json) in [(first, first_json), (second, second_json)] {
        let display = run_id.to_string();
        let parsed: RunId = display.parse().expect("display form parses");
        let uuid = uuid::Uuid::parse_str(&display).expect("display form is a UUID");

        assert_eq!(parsed, run_id);
        assert_eq!(uuid.get_version(), Some(uuid::Version::SortRand));
        assert_eq!(uuid.get_variant(), uuid::Variant::RFC4122);
        assert_eq!(serde_json::to_string(&run_id).unwrap(), json);
    }
}

#[test]
fn run_id_deserialization_rejects_noncanonical_or_non_v7_values() {
    let canonical = "017f22e2-79b0-7cc3-98c4-dc0c0c07398f";
    let invalid_text = [
        "550e8400-e29b-41d4-a716-446655440000".to_owned(),
        "017f22e2-79b0-7cc3-c8c4-dc0c0c07398f".to_owned(),
        canonical.to_ascii_uppercase(),
        canonical.replace('-', ""),
        "not-a-uuid".to_owned(),
    ];

    for invalid in invalid_text {
        assert!(
            invalid.parse::<RunId>().is_err(),
            "FromStr accepted invalid persisted run id {invalid}"
        );
        let invalid_json = serde_json::to_string(&invalid).unwrap();
        assert!(
            serde_json::from_str::<RunId>(&invalid_json).is_err(),
            "JSON accepted invalid persisted run id {invalid}"
        );
    }
    assert!(serde_json::from_str::<RunId>("7").is_err());
}

#[test]
fn turn_call_and_sequence_counters_remain_run_local() {
    let mut first = IdentityAllocator::new();
    let mut second = IdentityAllocator::new();

    assert_eq!(first.next_turn(), second.next_turn());
    assert_eq!(first.next_call(), second.next_call());
    assert_eq!(first.next_sequence(), second.next_sequence());
}

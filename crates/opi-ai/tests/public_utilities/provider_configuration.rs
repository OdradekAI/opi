//! Provider header merge rules and scoped environment lookup.

use std::collections::HashMap;
use std::sync::Mutex;

use opi_ai::api::shared::env::provider_env_value;
use opi_ai::api::shared::headers::merge_provider_header_sources;

/// Serializes tests that mutate the process environment.
static ENV_LOCK_536: Mutex<()> = Mutex::new(());

fn headers(pairs: &[(&str, Option<&str>)]) -> Vec<(String, Option<String>)> {
    pairs
        .iter()
        .map(|(name, value)| ((*name).to_owned(), value.map(|value| value.to_owned())))
        .collect()
}

#[test]
fn merge_provider_header_sources_merges_case_insensitively_last_wins_in_reinsert_order() {
    let first = headers(&[("X-Beta", Some("one")), ("Keep-Alive", Some("0"))]);
    let second = headers(&[("x-beta", Some("two"))]);
    let merged = merge_provider_header_sources(&[&first, &second]).unwrap();
    assert_eq!(
        merged,
        vec![
            ("Keep-Alive".to_owned(), "0".to_owned()),
            // The override re-inserts at the end and keeps the winner's case.
            ("x-beta".to_owned(), "two".to_owned()),
        ]
    );
}

#[test]
fn merge_provider_header_sources_none_deletes_and_empty_yields_none() {
    let base = headers(&[("A", Some("1")), ("B", Some("2"))]);
    let delete = headers(&[("a", None)]);
    let merged = merge_provider_header_sources(&[&base, &delete]).unwrap();
    assert_eq!(merged, vec![("B".to_owned(), "2".to_owned())]);

    let delete_all = headers(&[("b", None)]);
    assert_eq!(
        merge_provider_header_sources(&[&base, &delete, &delete_all]),
        None
    );
    // A deletion for a name that is not present adds nothing.
    let only_delete = headers(&[("absent", None)]);
    assert_eq!(merge_provider_header_sources(&[&only_delete]), None);
    assert_eq!(merge_provider_header_sources(&[]), None);
}

#[test]
fn merge_provider_header_sources_preserves_winner_within_and_across_sources() {
    let single = headers(&[("X-Api-Version", Some("v1")), ("x-api-version", Some("v2"))]);
    assert_eq!(
        merge_provider_header_sources(&[&single]).unwrap(),
        vec![("x-api-version".to_owned(), "v2".to_owned())]
    );
    let literal = headers(&[("X-Api-Version", Some("v1"))]);
    assert_eq!(
        merge_provider_header_sources(&[&literal]).unwrap(),
        vec![("X-Api-Version".to_owned(), "v1".to_owned())]
    );
}

#[test]
fn provider_env_scoped_precedes_ambient() {
    let _guard = ENV_LOCK_536.lock().unwrap();
    let name = "OPI_TEST_PROVIDER_ENV_536_A";
    // Safety: single-threaded under ENV_LOCK_536; unique test name.
    unsafe { std::env::set_var(name, "ambient") };
    let mut scoped = HashMap::new();
    scoped.insert(name.to_owned(), "scoped".to_owned());
    assert_eq!(
        provider_env_value(name, Some(&scoped)),
        Some("scoped".to_owned())
    );
    assert_eq!(provider_env_value(name, None), Some("ambient".to_owned()));
    // Safety: see above.
    unsafe { std::env::remove_var(name) };
    assert_eq!(provider_env_value(name, None), None);
}

#[test]
fn provider_env_empty_scoped_falls_back_to_ambient() {
    let _guard = ENV_LOCK_536.lock().unwrap();
    let name = "OPI_TEST_PROVIDER_ENV_536_B";
    // Safety: single-threaded under ENV_LOCK_536; unique test name.
    unsafe { std::env::set_var(name, "ambient") };
    let mut scoped = HashMap::new();
    scoped.insert(name.to_owned(), String::new());
    // Faithful to the pinned `env?.[name] || process.env[name]`: an empty
    // scoped value is falsy and falls back to ambient.
    assert_eq!(
        provider_env_value(name, Some(&scoped)),
        Some("ambient".to_owned())
    );
    // Safety: see above.
    unsafe { std::env::set_var(name, "") };
    // The pinned || chain also treats an empty ambient value as unset.
    assert_eq!(provider_env_value(name, Some(&scoped)), None);
    // Safety: see above.
    unsafe { std::env::remove_var(name) };
    assert_eq!(provider_env_value(name, Some(&scoped)), None);
}

//! Public catalog bundle loading, format compatibility and installation failures.

use std::collections::BTreeSet;

use opi_ai::catalog::{
    BUNDLE_SCHEMA_VERSION, Catalog, CatalogError, CatalogHandle, CatalogOperation, CatalogOrigin,
    DATA_SCHEMA_VERSION,
};
use opi_ai::{ChatModelRecord, ModelCapabilities, ModelInfo, WireApi};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

#[test]
fn builtin_bundle_has_opi_origin_and_content_revision() {
    let catalog = Catalog::builtin().expect("builtin bundle must load");
    let (raw, _) = Catalog::builtin_bundle_bytes();
    let digest = sha256_hex(raw);
    let provenance = catalog.provenance();
    assert_eq!(provenance.revision, format!("sha256-{digest}"));
    assert_eq!(provenance.artifact_sha256, digest);
    assert_eq!(provenance.origin, CatalogOrigin::Opi);
    assert_eq!(provenance.data_schema_version, DATA_SCHEMA_VERSION);
    assert_eq!(provenance.bundle_schema_version, BUNDLE_SCHEMA_VERSION);
    assert!(std::sync::Arc::ptr_eq(catalog, Catalog::builtin().unwrap()));
}

#[test]
fn builtin_bundle_bytes_revalidate_through_public_constructor() {
    let (catalog_bytes, manifest_bytes) = Catalog::builtin_bundle_bytes();
    let reparsed = Catalog::from_bytes(catalog_bytes, manifest_bytes)
        .expect("embedded bytes must revalidate through the public constructor");
    assert_eq!(
        reparsed.provenance(),
        Catalog::builtin().unwrap().provenance()
    );

    assert!(
        reparsed
            .model("anthropic", CatalogOperation::Chat, "claude-fable-5")
            .is_some()
    );
}

#[test]
fn legacy_v1_bundle_retains_its_source_metadata_and_can_be_installed() {
    let record = json!({
        "provider": "example",
        "type": "chat",
        "id": "legacy-model",
        "futureVendorField": {"preserved": [null, 0]},
        "api": "openai-completions",
        "name": "Legacy model",
        "baseUrl": "https://example.invalid/v1",
        "input": ["text"],
        "cost": {"input": 0, "output": 0, "cacheRead": 0, "cacheWrite": 0},
        "reasoning": false,
        "contextWindow": 8192,
        "maxTokens": 1024,
    });
    let catalog_bytes = serde_json::to_vec(&json!({"example": [&record]})).unwrap();
    let digest = sha256_hex(&catalog_bytes);
    let manifest = json!({
        "bundle_schema_version": 1,
        "data_schema_version": 6,
        "min_reader": {"bundle_schema_version": 1},
        "artifact": {
            "kind": "pi-published-model-catalog",
            "revision": format!("sha256-{digest}"),
            "sha256": digest,
            "url": "https://example.invalid/revisions/fixture",
        },
        "source": {"pi_package": "@earendil-works/pi-ai@fixture", "local_origin": "fixture"},
        "generator": {
            "pi_model_data_ts_sha256": "a".repeat(64),
            "pi_hydrate_model_catalog_ts_sha256": "b".repeat(64),
            "packer": "scripts/pi-catalog/pack.py",
            "packer_sha256": "c".repeat(64),
        },
        "structure_sha256": sha256_hex(br#"{"example":{"chat:legacy-model":"openai-completions"}}"#),
        "providers": ["example"],
        "counts": {"providers": 1, "chat": 1, "image": 0, "classifier": 0},
    });

    let (handle, held) = handle_with_builtin();
    handle
        .try_install(&catalog_bytes, &serde_json::to_vec(&manifest).unwrap())
        .unwrap();
    let legacy = handle.current();
    let provenance = legacy.provenance();
    assert_eq!(provenance.bundle_schema_version, 1);
    assert_eq!(provenance.data_schema_version, 6);
    assert_eq!(provenance.revision, format!("sha256-{digest}"));
    assert_eq!(provenance.artifact_sha256, digest);
    let CatalogOrigin::PiPublished {
        source_url,
        pi_package,
        local_origin,
        generator,
    } = &provenance.origin
    else {
        panic!("expected legacy Pi source");
    };
    assert_eq!(source_url, "https://example.invalid/revisions/fixture");
    assert_eq!(pi_package, "@earendil-works/pi-ai@fixture");
    assert_eq!(local_origin, "fixture");
    assert_eq!(generator.pi_model_data_ts_sha256, "a".repeat(64));
    assert_eq!(generator.pi_hydrate_model_catalog_ts_sha256, "b".repeat(64));
    assert_eq!(generator.packer_sha256, "c".repeat(64));
    assert_eq!(provenance.counts.chat, 1);
    assert_eq!(
        legacy
            .model("example", CatalogOperation::Chat, "legacy-model")
            .unwrap()
            .raw,
        record
    );
    assert_eq!(held.provenance().origin, CatalogOrigin::Opi);
}

// ---------------------------------------------------------------------------
// Rejection matrix — every candidate is a mutated copy of the real bundle
// ---------------------------------------------------------------------------

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// Rebuild a manifest for a mutated catalog so earlier gates (bundle schema,
/// artifact digest) pass and the mutated dimension is isolated. `providers`
/// optionally overrides the declared provider list; the structure digest is
/// intentionally left at the authentic (now stale) value.
fn resealed_manifest(mutated_catalog: &[u8], providers: Option<Value>) -> Vec<u8> {
    let (_, manifest_bytes) = Catalog::builtin_bundle_bytes();
    let mut manifest: Value =
        serde_json::from_slice(manifest_bytes).expect("builtin manifest parses");
    manifest["artifact"]["sha256"] = Value::String(sha256_hex(mutated_catalog));
    if let Some(providers) = providers {
        manifest["providers"] = providers;
    }
    serde_json::to_vec(&manifest).expect("manifest serializes")
}

fn mutated_catalog(mutate: impl FnOnce(&mut Value)) -> Vec<u8> {
    let (catalog_bytes, _) = Catalog::builtin_bundle_bytes();
    let mut catalog: Value = serde_json::from_slice(catalog_bytes).expect("builtin catalog parses");
    mutate(&mut catalog);
    serde_json::to_vec(&catalog).expect("catalog serializes")
}

fn handle_with_builtin() -> (CatalogHandle, std::sync::Arc<Catalog>) {
    let handle = CatalogHandle::new(Catalog::builtin().expect("builtin loads").clone());
    let held = handle.current();
    (handle, held)
}

#[test]
fn tampered_artifact_rejected_last_known_retained() {
    let (handle, held) = handle_with_builtin();
    // Flip one byte of a real entry (valid JSON, different bytes).
    let tampered = mutated_catalog(|catalog| {
        catalog["anthropic"][0]["name"] = Value::String("forged name".to_string());
    });
    let (_, manifest_bytes) = Catalog::builtin_bundle_bytes();
    let error = handle
        .try_install(&tampered, manifest_bytes)
        .expect_err("tampered bytes must be rejected");
    match error {
        CatalogError::DigestMismatch { .. } => {}
        other => panic!("expected DigestMismatch, got {other:?}"),
    }
    // Last-known revision retained; held catalogs never rebound.
    assert_eq!(
        handle.current().provenance().revision,
        held.provenance().revision
    );
    assert!(std::sync::Arc::ptr_eq(&handle.current(), &held));
    assert!(
        handle
            .current()
            .model("anthropic", CatalogOperation::Chat, "claude-fable-5")
            .is_some()
    );
}

#[test]
fn wrong_bundle_schema_version_rejected_last_known_retained() {
    let (handle, held) = handle_with_builtin();
    let (catalog_bytes, manifest_bytes) = Catalog::builtin_bundle_bytes();
    let mut manifest: Value = serde_json::from_slice(manifest_bytes).unwrap();
    manifest["bundle_schema_version"] = Value::from(BUNDLE_SCHEMA_VERSION + 1);
    let manifest_bytes = serde_json::to_vec(&manifest).unwrap();
    let error = handle
        .try_install(catalog_bytes, &manifest_bytes)
        .expect_err("unsupported bundle schema must be rejected");
    match error {
        CatalogError::BundleSchemaVersion {
            supported: BUNDLE_SCHEMA_VERSION,
            found,
        } => assert_eq!(found, BUNDLE_SCHEMA_VERSION + 1),
        other => panic!("expected BundleSchemaVersion, got {other:?}"),
    }
    assert_eq!(
        handle.current().provenance().revision,
        held.provenance().revision
    );
}

#[test]
fn incompatible_data_or_minimum_reader_version_preserves_last_known() {
    let (catalog_bytes, manifest_bytes) = Catalog::builtin_bundle_bytes();
    for (field, version) in [
        ("data_schema_version", 6),
        ("data_schema_version", DATA_SCHEMA_VERSION + 1),
        ("min_reader", BUNDLE_SCHEMA_VERSION + 1),
    ] {
        let (handle, held) = handle_with_builtin();
        let mut manifest: Value = serde_json::from_slice(manifest_bytes).unwrap();
        if field == "min_reader" {
            manifest[field]["bundle_schema_version"] = Value::from(version);
        } else {
            manifest[field] = Value::from(version);
        }
        assert!(
            handle
                .try_install(catalog_bytes, &serde_json::to_vec(&manifest).unwrap())
                .is_err(),
            "incompatible {field}={version} must be rejected before publication"
        );
        assert!(std::sync::Arc::ptr_eq(&handle.current(), &held));
    }
}

#[test]
fn current_bundle_rejects_unknown_source() {
    let (catalog_bytes, manifest_bytes) = Catalog::builtin_bundle_bytes();
    let mut manifest: Value = serde_json::from_slice(manifest_bytes).unwrap();
    manifest["source"] = json!("unrecognized");
    let error =
        Catalog::from_bytes(catalog_bytes, &serde_json::to_vec(&manifest).unwrap()).unwrap_err();
    assert!(matches!(error, CatalogError::Json { detail } if detail.contains("unrecognized")));
}

#[test]
fn missing_provider_rejected_last_known_retained() {
    let (handle, held) = handle_with_builtin();
    let mutated = mutated_catalog(|catalog| {
        catalog
            .as_object_mut()
            .expect("catalog object")
            .remove("anthropic");
    });
    // Manifest retains the original provider set. The digest is resealed so the
    // missing-provider gate is reached.
    let manifest_bytes = resealed_manifest(&mutated, None);
    let error = handle
        .try_install(&mutated, &manifest_bytes)
        .expect_err("missing provider must be rejected");
    match error {
        CatalogError::MissingProvider { provider } => assert_eq!(provider, "anthropic"),
        other => panic!("expected MissingProvider, got {other:?}"),
    }
    assert_eq!(
        handle.current().provenance().revision,
        held.provenance().revision
    );
}

#[test]
fn duplicate_provider_in_manifest_preserves_last_known_revision() {
    let (handle, held) = handle_with_builtin();
    let (catalog, manifest) = Catalog::builtin_bundle_bytes();
    let mut manifest: Value = serde_json::from_slice(manifest).unwrap();
    let providers = manifest["providers"].as_array_mut().unwrap();
    let provider = providers[0].as_str().unwrap().to_owned();
    providers.push(Value::String(provider.clone()));
    let error = handle
        .try_install(catalog, &serde_json::to_vec(&manifest).unwrap())
        .unwrap_err();
    assert!(matches!(error, CatalogError::DuplicateProvider { provider: id } if id == provider));
    assert!(std::sync::Arc::ptr_eq(&handle.current(), &held));
}

#[test]
fn stale_structure_rejected_last_known_retained() {
    let (handle, held) = handle_with_builtin();
    // Remove the provider from both artifact and manifest provider list, but
    // keep the authentic structure digest: self-consistent manifest, stale
    // structure.
    let mutated = mutated_catalog(|catalog| {
        catalog
            .as_object_mut()
            .expect("catalog object")
            .remove("anthropic");
    });
    let (_, builtin_manifest) = Catalog::builtin_bundle_bytes();
    let providers: Vec<String> =
        serde_json::from_slice::<Value>(builtin_manifest).unwrap()["providers"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| p.as_str() != Some("anthropic"))
            .map(|p| p.as_str().unwrap().to_string())
            .collect();
    let manifest_bytes =
        resealed_manifest(&mutated, Some(serde_json::to_value(providers).unwrap()));
    let error = handle
        .try_install(&mutated, &manifest_bytes)
        .expect_err("stale structure digest must be rejected");
    match error {
        CatalogError::StructureMismatch { .. } => {}
        other => panic!("expected StructureMismatch, got {other:?}"),
    }
    assert_eq!(
        handle.current().provenance().revision,
        held.provenance().revision
    );
}

#[test]
fn duplicate_identity_rejected_last_known_retained() {
    let (handle, held) = handle_with_builtin();
    let mutated = mutated_catalog(|catalog| {
        let entries = catalog["anthropic"]
            .as_array_mut()
            .expect("anthropic entries");
        let first = entries[0].clone();
        entries.push(first);
    });
    let manifest_bytes = resealed_manifest(&mutated, None);
    let error = handle
        .try_install(&mutated, &manifest_bytes)
        .expect_err("duplicate type:id identity must be rejected");
    match error {
        CatalogError::DuplicateIdentity { provider, identity } => {
            assert_eq!(provider, "anthropic");
            assert!(identity.starts_with("chat:"));
        }
        other => panic!("expected DuplicateIdentity, got {other:?}"),
    }
    assert_eq!(
        handle.current().provenance().revision,
        held.provenance().revision
    );
}

#[test]
fn invalid_entry_rejected_last_known_retained() {
    let (handle, held) = handle_with_builtin();
    let mutated = mutated_catalog(|catalog| {
        // Real entry loses its cost metadata — violates the data contract.
        catalog["anthropic"][0]
            .as_object_mut()
            .expect("entry object")
            .remove("cost");
    });
    let manifest_bytes = resealed_manifest(&mutated, None);
    let error = handle
        .try_install(&mutated, &manifest_bytes)
        .expect_err("contract-violating entry must be rejected");
    match error {
        CatalogError::InvalidEntry { identity, .. } => {
            assert_eq!(identity, "chat:claude-fable-5");
        }
        other => panic!("expected InvalidEntry, got {other:?}"),
    }
    assert_eq!(
        handle.current().provenance().revision,
        held.provenance().revision
    );
}

#[test]
fn operation_qualified_getters_partition_the_real_catalog() {
    let catalog = Catalog::builtin().unwrap();
    let raw: Value = serde_json::from_slice(Catalog::builtin_bundle_bytes().0).unwrap();
    for (provider, entries) in raw.as_object().unwrap() {
        for operation in [
            CatalogOperation::Chat,
            CatalogOperation::Image,
            CatalogOperation::Classifier,
        ] {
            let expected: BTreeSet<_> = entries
                .as_array()
                .unwrap()
                .iter()
                .filter(|entry| entry["type"] == operation.as_str())
                .map(|entry| entry["id"].as_str().unwrap())
                .collect();
            let actual: BTreeSet<_> = catalog
                .models(provider, operation)
                .unwrap()
                .map(|entry| entry.id.as_str())
                .collect();
            assert_eq!(actual, expected, "{provider}/{}", operation.as_str());
        }
    }
    // Unknown providers have no models but do not disturb known ones.
    assert!(
        catalog
            .models("no-such-provider", CatalogOperation::Chat)
            .is_none()
    );
    assert!(
        catalog
            .model("anthropic", CatalogOperation::Image, "claude-fable-5")
            .is_none()
    );

    // Typed core fields of one well-known entry stay faithful to the artifact.
    let model = catalog
        .model("anthropic", CatalogOperation::Chat, "claude-fable-5")
        .unwrap();
    assert_eq!(model.api, "anthropic-messages");
    assert_eq!(model.base_url, "https://api.anthropic.com");
    assert!(model.reasoning);
    assert!(model.context_window.is_some_and(|tokens| tokens > 0));
    assert!(model.max_tokens.is_some_and(|tokens| tokens > 0));
    assert!(
        model.raw.get("cost").is_some(),
        "raw entry retains full artifact data"
    );
}

#[test]
fn explicit_complete_custom_records_remain_usable_alongside_any_catalog_state() {
    // A caller-supplied complete record — the custom-model path — is fully
    // usable before, during, and after catalog rejections.
    let record = ChatModelRecord::new(
        "my-own-gateway",
        ModelInfo::new(
            "my-model",
            "My Model",
            WireApi::AnthropicMessages,
            ModelCapabilities::new(200_000, 32_768),
        ),
    );
    assert_eq!(record.provider_id(), "my-own-gateway");
    assert_eq!(record.model_id(), "my-model");
    assert_eq!(record.wire_api(), WireApi::AnthropicMessages);

    let (handle, _) = handle_with_builtin();
    let bad = mutated_catalog(|catalog| {
        catalog["anthropic"][0]["name"] = Value::String("forged".to_string());
    });
    let (_, manifest_bytes) = Catalog::builtin_bundle_bytes();
    assert!(handle.try_install(&bad, manifest_bytes).is_err());

    // The custom record is unaffected by catalog state: it never consults the
    // builtin catalog, and its identity stays intact.
    assert_eq!(record.model_id(), "my-model");
    assert!(
        handle
            .current()
            .model("my-own-gateway", CatalogOperation::Chat, "my-model")
            .is_none()
    );
}

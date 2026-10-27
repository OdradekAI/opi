//! Behavioral tests for the offline bundled model catalog (task 20.32 / DEL-10).
//!
//! Seam: the public `opi_ai::catalog` Interface — the builtin resource loader
//! and operation-qualified model getters over the authentic, digest-pinned
//! Pi published catalog packaged by `scripts/pi-catalog/pack.py`.
//!
//! Negative cases mutate copies of the real bundled bytes; no synthetic
//! catalog data is ever constructed.

use opi_ai::catalog::{Catalog, CatalogError, CatalogHandle, CatalogOperation};
use opi_ai::{ChatModelRecord, ModelCapabilities, ModelInfo, WireApi};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// Revision pinned by `.repo/pi-1.3.0/nix/model-catalog.json` — the authentic
/// artifact identity, byte-identical to the published pi.dev catalog.
const PINNED_REVISION: &str =
    "sha256-ffa3e98626230f378f80200debb736c4a2238b5f2ba246a92cfc7d8e19604678";
const PINNED_DIGEST: &str = "ffa3e98626230f378f80200debb736c4a2238b5f2ba246a92cfc7d8e19604678";

#[test]
fn builtin_bundle_loads_with_authentic_provenance() {
    let catalog = Catalog::builtin().expect("builtin bundle must load");

    let provenance = catalog.provenance();
    assert_eq!(provenance.revision, PINNED_REVISION);
    assert_eq!(provenance.artifact_sha256, PINNED_DIGEST);
    assert_eq!(
        provenance.source_url,
        "https://pi.dev/api/models/revisions/sha256-ffa3e98626230f378f80200debb736c4a2238b5f2ba246a92cfc7d8e19604678?types=chat,image,classifier"
    );
    assert_eq!(provenance.pi_package, "@earendil-works/pi-ai@1.3.0");
    assert_eq!(provenance.local_origin, ".repo/pi-1.3.0");
    assert_eq!(provenance.data_schema_version, 6);
    assert_eq!(provenance.bundle_schema_version, 1);

    // Provider and operation identities of the packaged artifact.
    assert_eq!(provenance.providers.len(), 42);
    assert!(provenance.providers.contains(&"anthropic".to_string()));
    assert!(provenance.providers.contains(&"openrouter".to_string()));
    assert_eq!(provenance.counts.chat, 1608);
    assert_eq!(provenance.counts.image, 59);
    assert_eq!(provenance.counts.classifier, 38);

    // Generator identity: Pi's own validator/hydrator digests plus the packer.
    for digest in [
        &provenance.generator.pi_model_data_ts_sha256,
        &provenance.generator.pi_hydrate_model_catalog_ts_sha256,
        &provenance.generator.packer_sha256,
    ] {
        assert_eq!(digest.len(), 64, "digest must be sha-256 hex: {digest}");
        assert!(digest.chars().all(|c| c.is_ascii_hexdigit()));
    }

    // The loader is deterministic: a second load observes the same revision.
    let again = Catalog::builtin().expect("builtin bundle reloads");
    assert_eq!(again.provenance().revision, PINNED_REVISION);
}

#[test]
fn builtin_bundle_bytes_match_pinned_digest_identity() {
    // The embedded artifact bytes carry the pinned identity: reparsing them
    // through the public constructor yields the same provenance, and the
    // manifest's declared digest equals the pinned revision.
    let (catalog_bytes, manifest_bytes) = Catalog::builtin_bundle_bytes();
    let reparsed = Catalog::from_bytes(catalog_bytes, manifest_bytes)
        .expect("embedded bytes must revalidate through the public constructor");
    assert_eq!(
        reparsed.provenance().revision,
        Catalog::builtin().unwrap().provenance().revision
    );
    assert_eq!(reparsed.provenance().artifact_sha256, PINNED_DIGEST);

    // Operation-qualified surface exists on the reparsed catalog as well.
    assert!(
        reparsed
            .model("anthropic", CatalogOperation::Chat, "claude-fable-5")
            .is_some()
    );
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
    assert_eq!(handle.current().provenance().artifact_sha256, PINNED_DIGEST);
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
    manifest["bundle_schema_version"] = Value::from(2u32);
    let manifest_bytes = serde_json::to_vec(&manifest).unwrap();
    let error = handle
        .try_install(catalog_bytes, &manifest_bytes)
        .expect_err("unsupported bundle schema must be rejected");
    match error {
        CatalogError::BundleSchemaVersion {
            supported: 1,
            found: 2,
        } => {}
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
        ("data_schema_version", 5),
        ("data_schema_version", 7),
        ("min_reader", 2),
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
fn missing_provider_rejected_last_known_retained() {
    let (handle, held) = handle_with_builtin();
    let mutated = mutated_catalog(|catalog| {
        catalog
            .as_object_mut()
            .expect("catalog object")
            .remove("anthropic");
    });
    // Manifest keeps declaring all 42 providers; digest is resealed so the
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
    for provider in ["anthropic", "openrouter", "typesafe"] {
        let chat = catalog
            .models(provider, CatalogOperation::Chat)
            .unwrap()
            .count();
        let image = catalog
            .models(provider, CatalogOperation::Image)
            .unwrap()
            .count();
        let classifier = catalog
            .models(provider, CatalogOperation::Classifier)
            .unwrap()
            .count();
        match provider {
            "anthropic" => assert_eq!((chat, image, classifier), (17, 0, 0)),
            "openrouter" => assert_eq!((chat, image, classifier), (394, 59, 23)),
            "typesafe" => assert_eq!((chat, image, classifier), (0, 0, 1)),
            _ => unreachable!(),
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

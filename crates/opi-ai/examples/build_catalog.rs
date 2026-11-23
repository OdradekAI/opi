//! Maintainer command for preparing and checking the opi catalog bundle.

use opi_ai::auth;
#[path = "build_catalog/authoring.rs"]
mod authoring;
#[cfg(test)]
#[path = "build_catalog/authoring_tests.rs"]
mod authoring_tests;
#[path = "../src/providers/profiles.rs"]
mod profiles;

use std::{
    collections::BTreeMap, env, error::Error, ffi::OsStr, fs, io, path::Path, process::ExitCode,
};

use opi_ai::catalog::{BUNDLE_SCHEMA_VERSION, Catalog, CatalogOperation, DATA_SCHEMA_VERSION};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

const OPENROUTER_DIRECT_SHARED_IDS: [&str; 3] = [
    "openai/gpt-4o",
    "openai/gpt-4o-mini",
    "google/gemini-2.5-pro",
];

struct Bundle {
    catalog: Vec<u8>,
    manifest: Vec<u8>,
    openrouter_profile: Option<Vec<u8>>,
}

impl Bundle {
    fn build(sources: &Path) -> Result<Self> {
        let mut providers = BTreeMap::<String, Vec<Value>>::new();
        for path in fs::read_dir(sources)? {
            let path = path?.path();
            if path.extension() != Some(OsStr::new("json")) {
                continue;
            }
            let provider = path
                .file_stem()
                .and_then(OsStr::to_str)
                .filter(|name| !name.is_empty())
                .ok_or_else(|| format!("invalid provider filename {}", path.display()))?;
            let entries = authoring::parse(&fs::read(&path)?)
                .map_err(|error| format!("{}: {error}", path.display()))?;
            providers.insert(provider.to_owned(), entries);
        }
        if providers.is_empty() {
            return Err(format!("no provider JSON files in {}", sources.display()).into());
        }

        let (catalog, manifest) = encode_catalog(&providers)?;
        let validated = Catalog::from_bytes(&catalog, &manifest)?;
        authoring::validate(&validated)?;
        let openrouter_profile = if providers.contains_key("openrouter") {
            Some(openrouter_profile(&validated)?)
        } else {
            None
        };
        Ok(Self {
            catalog,
            manifest,
            openrouter_profile,
        })
    }

    fn files(&self) -> impl Iterator<Item = (&'static str, &[u8])> {
        [
            ("catalog.json", self.catalog.as_slice()),
            ("manifest.json", self.manifest.as_slice()),
        ]
        .into_iter()
        .chain(
            self.openrouter_profile
                .as_deref()
                .map(|bytes| ("openrouter-profile.json", bytes)),
        )
    }

    fn write_candidate(&self, output: &Path) -> Result<()> {
        match output.symlink_metadata() {
            Ok(_) => {
                return Err(
                    format!("candidate directory already exists: {}", output.display()).into(),
                );
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        let parent = output
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let stage = tempfile::tempdir_in(parent)?;
        for (name, bytes) in self.files() {
            fs::write(stage.path().join(name), bytes)?;
        }
        fs::rename(stage.path(), output)?;
        Ok(())
    }

    fn check(&self, output: &Path) -> Result<()> {
        let profile = output.join("openrouter-profile.json");
        if self.openrouter_profile.is_none() && profile.try_exists()? {
            return Err(format!("{} has no matching OpenRouter source", profile.display()).into());
        }
        for (name, bytes) in self.files() {
            let path = output.join(name);
            let actual = fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
            if actual != bytes {
                return Err(format!(
                    "{} differs from the bundle built from opi sources",
                    path.display()
                )
                .into());
            }
        }
        Ok(())
    }
}

fn encode_catalog(providers: &BTreeMap<String, Vec<Value>>) -> Result<(Vec<u8>, Vec<u8>)> {
    let mut structure = BTreeMap::new();
    for (provider, entries) in providers {
        let mut identities = BTreeMap::new();
        for entry in entries {
            let field = |key| {
                entry
                    .get(key)
                    .and_then(Value::as_str)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| format!("{provider}: expected a nonempty model {key}"))
            };
            identities.insert(
                format!("{}:{}", field("type")?, field("id")?),
                field("api")?,
            );
        }
        structure.insert(provider, identities);
    }

    let mut catalog = serde_json::to_vec(providers)?;
    catalog.push(b'\n');
    let mut manifest = serde_json::to_vec_pretty(&json!({
        "bundle_schema_version": BUNDLE_SCHEMA_VERSION,
        "data_schema_version": DATA_SCHEMA_VERSION,
        "min_reader": {"bundle_schema_version": BUNDLE_SCHEMA_VERSION},
        "source": "opi",
        "artifact": {"sha256": sha256(&catalog)},
        "structure_sha256": sha256(&serde_json::to_vec(&structure)?),
        "providers": providers.keys().collect::<Vec<_>>(),
    }))?;
    manifest.push(b'\n');

    Ok((catalog, manifest))
}

/// The direct OpenRouter factory exposes a small ordered Chat Completions selection.
fn openrouter_profile(catalog: &Catalog) -> Result<Vec<u8>> {
    let mut entries = Vec::new();
    for id in OPENROUTER_DIRECT_SHARED_IDS {
        let Some(model) = catalog.model("openrouter", CatalogOperation::Chat, id) else {
            continue;
        };
        if model.api != "openai-completions" {
            return Err(format!(
                "openrouter:{id}: direct profile requires openai-completions, found {}",
                model.api
            )
            .into());
        }
        entries.push(&model.raw);
    }

    let mut bytes = serde_json::to_vec(&entries)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn run() -> Result<()> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    let [command, sources, output] = args.as_slice() else {
        return Err("usage: build_catalog <build|check> <provider-data-dir> <bundle-dir>".into());
    };
    let output = Path::new(output);
    match command.to_str() {
        Some("build") => Bundle::build(Path::new(sources))?.write_candidate(output)?,
        Some("check") => Bundle::build(Path::new(sources))?.check(output)?,
        _ => return Err("expected the build or check command".into()),
    }
    println!(
        "catalog {} OK: {}",
        command.to_string_lossy(),
        output.display()
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("catalog: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opi_ai::catalog::{CatalogOperation, CatalogOrigin};

    fn chat(id: &str) -> Value {
        json!({
            "provider": "groq",
            "type": "chat",
            "id": id,
            "api": "openai-completions",
            "name": "Example",
            "baseUrl": "https://example.invalid/v1",
            "input": ["text"],
            "cost": {"input": 1, "output": 2, "cacheRead": 0, "cacheWrite": 0},
            "reasoning": false,
            "contextWindow": 8192,
            "maxTokens": 1024,
            "x-example": {"preserve": [2, null, 1]},
        })
    }

    #[test]
    fn embedded_bundle_matches_reviewed_provider_files() {
        let resources = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/catalog");
        Bundle::build(&resources.join("providers"))
            .unwrap()
            .check(&resources.join("builtin"))
            .unwrap();
    }

    #[test]
    fn authored_model_and_local_edit_roundtrip_without_upstream_data() {
        let root = tempfile::tempdir().unwrap();
        let sources = root.path().join("providers");
        fs::create_dir(&sources).unwrap();
        let mut edited = chat("shared-model");
        edited["contextWindow"] = json!(16_384);
        let new = chat("opi-local-model");
        fs::write(
            sources.join("groq.json"),
            serde_json::to_vec(&[&edited, &new]).unwrap(),
        )
        .unwrap();

        let bundle = Bundle::build(&sources).unwrap();
        let output = root.path().join("candidate");
        bundle.write_candidate(&output).unwrap();
        Bundle::build(&sources).unwrap().check(&output).unwrap();
        let catalog = Catalog::from_bytes(
            &fs::read(output.join("catalog.json")).unwrap(),
            &fs::read(output.join("manifest.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(catalog.provenance().origin, CatalogOrigin::Opi);
        let entry = |id| catalog.model("groq", CatalogOperation::Chat, id).unwrap();
        assert_eq!(entry("opi-local-model").raw, new);
        assert_eq!(entry("shared-model").raw, edited);
        assert_eq!(entry("shared-model").context_window, Some(16_384));
        fs::write(output.join("openrouter-profile.json"), b"[]").unwrap();
        assert!(bundle.check(&output).is_err());
    }

    #[test]
    fn invalid_sources_and_existing_bundle_leave_the_last_candidate_readable() {
        let root = tempfile::tempdir().unwrap();
        let sources = root.path().join("providers");
        fs::create_dir(&sources).unwrap();
        let source = sources.join("groq.json");
        let model = chat("model");
        fs::write(&source, serde_json::to_vec(&[&model]).unwrap()).unwrap();
        let bundle = Bundle::build(&sources).unwrap();
        let output = root.path().join("candidate");
        bundle.write_candidate(&output).unwrap();
        assert!(bundle.write_candidate(&output).is_err());

        let mut invalid = model.clone();
        invalid["maxTokens"] = json!("1024");
        let mut wrong_owner = model.clone();
        wrong_owner["provider"] = json!("somewhere-else");
        for (rows, expected) in [
            (vec![&model, &model], "duplicate identity"),
            (vec![&invalid], "invalid maxTokens"),
            (vec![&wrong_owner], "provider field does not match"),
        ] {
            fs::write(&source, serde_json::to_vec(&rows).unwrap()).unwrap();
            assert!(
                Bundle::build(&sources)
                    .err()
                    .unwrap()
                    .to_string()
                    .contains(expected)
            );
            bundle.check(&output).unwrap();
        }
    }
}

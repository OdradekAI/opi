#![allow(dead_code)]

use opi_ai::ModelInfo;
use opi_ai::catalog::{CatalogRecord, ModelsStoreEntry};
use std::collections::HashMap;

pub fn chat_entry(models: Vec<ModelInfo>) -> ModelsStoreEntry {
    ModelsStoreEntry {
        models: models.into_iter().map(CatalogRecord::from).collect(),
        ..Default::default()
    }
}

pub fn catalogs(entries: HashMap<String, Vec<ModelInfo>>) -> HashMap<String, ModelsStoreEntry> {
    entries
        .into_iter()
        .map(|(provider, models)| (provider, chat_entry(models)))
        .collect()
}

pub fn refresh_options(
    providers: &[&str],
    allow_network: bool,
    force: Option<bool>,
) -> opi_ai::ModelsRefreshOptions {
    opi_ai::ModelsRefreshOptions {
        allow_network,
        providers: Some(providers.iter().map(|id| (*id).to_owned()).collect()),
        force,
    }
}

pub fn chat(record: &CatalogRecord) -> &ModelInfo {
    match record {
        CatalogRecord::Chat(model) => model,
        _ => panic!("expected chat model"),
    }
}

pub fn chat_mut(record: &mut CatalogRecord) -> &mut ModelInfo {
    match record {
        CatalogRecord::Chat(model) => model,
        _ => panic!("expected chat model"),
    }
}

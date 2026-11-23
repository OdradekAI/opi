//! OpenAI Codex catalog projection for the coding agent.

use opi_ai::ModelInfo;
use opi_ai::catalog::{Catalog, CatalogModel, CatalogOperation};

pub const OPENAI_CODEX_DEFAULT_BASE_URL: &str = "https://chatgpt.com/backend-api";

/// OpenAI Codex chat models from the embedded opi catalog.
///
/// The static listing and OAuth runtime factory use these records.
pub fn openai_codex_catalog() -> Vec<ModelInfo> {
    Catalog::builtin()
        .expect("validated embedded model catalog")
        .models("openai-codex", CatalogOperation::Chat)
        .expect("OpenAI Codex exists in the embedded catalog")
        .map(CatalogModel::to_chat_model_info)
        .collect::<Result<_, _>>()
        .expect("validated OpenAI Codex chat records")
}

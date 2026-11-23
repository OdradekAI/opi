//! Catalog projection and client identity for GitHub Copilot.

use opi_ai::ModelInfo;
use opi_ai::catalog::{Catalog, CatalogModel, CatalogOperation};

pub const GITHUB_COPILOT_DEFAULT_BASE_URL: &str = "https://api.individual.githubcopilot.com";

/// Headers pinned by the reviewed GitHub Copilot Chat profile.
pub fn github_copilot_static_headers() -> Vec<(String, String)> {
    [
        ("User-Agent", "GitHubCopilotChat/0.35.0"),
        ("Editor-Version", "vscode/1.107.0"),
        ("Editor-Plugin-Version", "copilot-chat/0.35.0"),
        ("Copilot-Integration-Id", "vscode-chat"),
    ]
    .into_iter()
    .map(|(name, value)| (name.to_owned(), value.to_owned()))
    .collect()
}

/// GitHub Copilot chat models from the embedded opi catalog.
///
/// The static listing, runtime factory and OAuth login policy use these
/// records before applying any account-specific selection.
pub fn github_copilot_catalog() -> Vec<ModelInfo> {
    Catalog::builtin()
        .expect("validated embedded model catalog")
        .models("github-copilot", CatalogOperation::Chat)
        .expect("GitHub Copilot exists in the embedded catalog")
        .map(CatalogModel::to_chat_model_info)
        .collect::<Result<_, _>>()
        .expect("validated GitHub Copilot chat records")
}

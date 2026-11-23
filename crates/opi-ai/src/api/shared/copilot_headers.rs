//! Request-dependent headers shared by the three GitHub Copilot chat wires.

use crate::errors::{ProviderError, ProviderErrorSummary};
use crate::message::Message;
use crate::provider::Request;

const GITHUB_COPILOT_MANAGED_HEADERS: &[&str] = &[
    "user-agent",
    "editor-version",
    "editor-plugin-version",
    "copilot-integration-id",
    "x-initiator",
    "openai-intent",
    "copilot-vision-request",
];

/// Reject per-request overrides of managed names and derive the route headers.
///
/// The adapters merge these values before request callbacks and dispatch.
pub(in crate::api) fn github_copilot_route_headers(
    request: &Request,
) -> Result<Vec<(String, String)>, ProviderError> {
    if let Some((name, _)) = request.extra_headers.iter().find(|(name, _)| {
        GITHUB_COPILOT_MANAGED_HEADERS.contains(&name.to_ascii_lowercase().as_str())
    }) {
        return Err(ProviderError::RequestFailed(
            ProviderErrorSummary::sanitized(format!(
                "request header '{name}' is reserved for GitHub Copilot"
            )),
        ));
    }

    let mut headers = vec![("Openai-Intent".into(), "conversation-edits".into())];
    if request.contains_image_content() {
        headers.push(("Copilot-Vision-Request".into(), "true".into()));
    }
    let initiator = match request.context.messages().last() {
        Some(Message::User(_)) | None => "user",
        Some(_) => "agent",
    };
    headers.push(("X-Initiator".into(), initiator.into()));
    Ok(headers)
}

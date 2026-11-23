//! Azure OpenAI Responses adapter topic index.

use super::*;
use opi_ai::api::azure_openai::{AzureOpenAIOptions, AzureOpenAIResponsesChatApi};
use opi_ai::api::openai_responses::{ResponsesReasoningSummary, ResponsesToolChoice};
use opi_ai::model_info::{ModelCapabilities, ModelInfo, ThinkingLevel, WireApi};
use opi_ai::providers::azure_openai::{
    resolve_azure_api_version, resolve_azure_base_url, resolve_deployment_name,
};

#[path = "responses/fixtures.rs"]
mod fixtures;
#[path = "responses/lifecycle.rs"]
mod lifecycle;
#[path = "responses/options.rs"]
mod options;
#[path = "responses/wire.rs"]
mod wire;

pub(crate) use fixtures::{
    azure_api_key_auth, azure_duration_on_peer, azure_history_assistant, azure_responses_provider,
    azure_responses_record, azure_responses_request, azure_responses_sse_text,
    azure_responses_sse_tool, collect_azure_events, collect_azure_items,
    mount_azure_responses_mock, registered_azure_responses_models, sent_body,
};

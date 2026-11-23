//! Public model metadata and its effects on provider requests and response cost.

#[path = "model_wire_metadata/anthropic_complete_metadata.rs"]
mod anthropic_complete_metadata;
#[path = "model_wire_metadata/anthropic_response_cost.rs"]
mod anthropic_response_cost;
#[path = "model_wire_metadata/fixtures.rs"]
mod fixtures;
#[path = "model_wire_metadata/model_contract.rs"]
mod model_contract;
#[path = "model_wire_metadata/pricing_usage.rs"]
mod pricing_usage;
#[path = "model_wire_metadata/sampling_parameters.rs"]
mod sampling_parameters;
#[path = "model_wire_metadata/sampling_wire.rs"]
mod sampling_wire;
#[path = "model_wire_metadata/thinking_admission.rs"]
mod thinking_admission;

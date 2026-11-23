//! Replay contracts through public projection and SDK request builders.

#[path = "replay_contract/association.rs"]
mod association;
#[path = "replay_contract/content_projection.rs"]
mod content_projection;
#[path = "replay_contract/execution_effects.rs"]
mod execution_effects;
#[path = "replay_contract/fixtures.rs"]
mod fixtures;
#[path = "replay_contract/responses_grammar.rs"]
mod grammar_replay_tests;
#[path = "replay_contract/malformed_arguments.rs"]
mod malformed_arguments;
#[path = "replay_contract/object_wire.rs"]
mod object_wire;
#[path = "replay_contract/wire_ids.rs"]
mod wire_ids;

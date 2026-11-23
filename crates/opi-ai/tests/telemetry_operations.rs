//! Frozen telemetry context at the public SDK operation boundaries.

#[path = "telemetry_operations/chat.rs"]
mod chat;
#[path = "telemetry_operations/classifier.rs"]
mod classifier;
#[path = "telemetry_operations/deferred.rs"]
mod deferred;
#[path = "telemetry_operations/fixtures.rs"]
mod fixtures;
#[path = "telemetry_operations/images.rs"]
mod images;
#[path = "telemetry_operations/prepared.rs"]
mod prepared;

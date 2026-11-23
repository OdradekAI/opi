//! Public schema compile and check contracts.

#[path = "schema_engine/admission.rs"]
mod admission;
#[path = "schema_engine/applicators.rs"]
mod applicators;
#[path = "schema_engine/container_keywords.rs"]
mod container_keywords;
#[path = "schema_engine/corpus.rs"]
mod corpus;
#[path = "schema_engine/fixtures.rs"]
mod fixtures;
#[path = "schema_engine/reference_resolution.rs"]
mod reference_resolution;
#[path = "schema_engine/stack_safety.rs"]
mod stack_safety;
#[path = "schema_engine/value_keywords.rs"]
mod value_keywords;

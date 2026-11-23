//! Public classifier contracts through real adapters and Models.

#[path = "classifier_contract/cloudflare_workers_ai_system_one.rs"]
mod cloudflare_workers_ai_system_one;
#[path = "classifier_contract/cross_protocol.rs"]
mod cross_protocol;
#[path = "classifier_contract/declaration_order.rs"]
mod declaration_order;
#[path = "classifier_contract/failure_fixture.rs"]
mod failure_fixture;
#[path = "classifier_contract/fixtures.rs"]
mod fixtures;
#[path = "classifier_contract/llama_cpp_classify.rs"]
mod llama_cpp_classify;
#[path = "classifier_contract/models_contract.rs"]
mod models_contract;
#[path = "classifier_contract/openai_decisions.rs"]
mod openai_decisions;
#[path = "support/pending_auth.rs"]
mod pending_auth;
#[path = "classifier_contract/typesafe_system_one.rs"]
mod typesafe_system_one;

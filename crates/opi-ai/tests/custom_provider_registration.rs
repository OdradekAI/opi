//! Public contracts for custom providers, model overrides and typed Chat registration.

#[path = "custom_provider_registration/chat_admission.rs"]
mod chat_admission;
#[path = "custom_provider_registration/chat_dispatch.rs"]
mod chat_dispatch;
#[path = "custom_provider_registration/chat_lifecycle.rs"]
mod chat_lifecycle;
#[path = "custom_provider_registration/chat_registration.rs"]
mod chat_registration;
#[path = "custom_provider_registration/fixtures.rs"]
mod fixtures;
#[path = "custom_provider_registration/registry_models.rs"]
mod registry_models;
#[path = "custom_provider_registration/registry_providers.rs"]
mod registry_providers;

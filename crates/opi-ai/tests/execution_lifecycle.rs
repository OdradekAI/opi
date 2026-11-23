//! Public execution lifecycle tests for ordinary calls, Deferred, and source release.

#[path = "execution_lifecycle/deferred.rs"]
mod deferred;
#[path = "execution_lifecycle/fixtures.rs"]
mod fixtures;
#[path = "execution_lifecycle/ordinary.rs"]
mod ordinary;
#[path = "execution_lifecycle/terminal_release.rs"]
mod terminal_release;

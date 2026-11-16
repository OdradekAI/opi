use std::sync::Arc;

use opi_ai::auth::oauth::anthropic::AnthropicOAuthProvider;
use opi_ai::auth::store_oauth::StoreOAuthResolver;
use opi_ai::auth::{AuthResolver, Credential, OAuthCredential};
use opi_ai::mutation::InMemoryCredentialStore;
use secrecy::SecretString;

#[test]
fn provider_oauth_and_store_owner_are_constructible_without_builtins() {
    let flow = Arc::new(AnthropicOAuthProvider::production());
    let store = Arc::new(InMemoryCredentialStore::new());
    let resolver = StoreOAuthResolver::new(store, "anthropic", flow);
    let _: Arc<dyn AuthResolver> = Arc::new(resolver);

    let credential: Credential = OAuthCredential {
        access: SecretString::from("access"),
        refresh: SecretString::from("refresh"),
        expires_at: None,
        base_url: None,
        account_id: None,
        client_id: None,
        enterprise_domain: None,
        available_model_ids: None,
        scopes: None,
    }
    .into();
    assert!(matches!(credential, Credential::OAuthToken { .. }));
}

#[test]
fn pkce_primitives_remain_public_on_the_independent_owner() {
    let verifier = opi_ai::auth::oauth::generate_code_verifier();
    assert_eq!(verifier.len(), 43);
    assert_eq!(
        opi_ai::auth::oauth::code_challenge_s256(&verifier).len(),
        43
    );
    assert!(!opi_ai::auth::oauth::generate_state().is_empty());
}

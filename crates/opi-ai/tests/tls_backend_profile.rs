use opi_ai::tls::{AlpnPreference, TlsOptions, TrustRoots};

#[test]
fn tls_backend_profile_exposes_the_frozen_source_scoped_defaults() {
    let options = TlsOptions::new();

    assert!(matches!(
        options.trust_roots(),
        TrustRoots::PinnedNodeDefault
    ));
    assert!(!options.has_client_identity());
    assert_eq!(options.alpn_preference(), AlpnPreference::H2Preferred);
}

#[test]
fn tls_backend_profile_keeps_trust_and_alpn_selection_local() {
    let options = TlsOptions::new().http1_only();

    assert!(matches!(
        options.trust_roots(),
        TrustRoots::PinnedNodeDefault
    ));
    assert_eq!(options.alpn_preference(), AlpnPreference::Http1Only);
}

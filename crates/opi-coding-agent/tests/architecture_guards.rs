//! Architecture boundaries and removed-interface regression guards.

// opi-runtime-acceptance
#[path = "common/source_audit.rs"]
mod source_audit;

use source_audit::*;

#[test]
fn phase17_removed_interfaces_are_absent_from_production_source() {
    let sources = production_sources();
    assert!(
        sources.len() > 100,
        "the scan examined a real source tree ({} files)",
        sources.len()
    );

    let targets: &[(&str, &str)] = &[
        (
            "SharedProvider",
            "17.2: Agent no longer owns one Arc<dyn Provider>",
        ),
        (
            "AgentLoopTurnUpdate",
            "17.2: append-only turn updates replaced by atomic NextTurnState",
        ),
        (
            "AgentHarness",
            "17.2: the unused opi-agent state owner was removed",
        ),
        (
            "HarnessRuntimeConfig",
            "17.2: the unused opi-agent state owner was removed",
        ),
        (
            "BeforeToolCallResult::Allow",
            "17.4: the authorization-suggesting hook grant is now Continue",
        ),
        (
            "MetadataProvider",
            "17.5: renamed to ListingMetadataProvider",
        ),
        (
            "TraceSink",
            "17.7: the storage-shaped core trace contract was superseded by evidence",
        ),
        (
            "TraceReader",
            "17.8: no legacy trace reader exists without a registered workflow",
        ),
    ];
    for (symbol, why) in targets {
        for (path, source) in &sources {
            let hits = source_token_occurrences(source, symbol);
            assert_eq!(
                hits,
                0,
                "removed interface `{symbol}` still referenced in {} ({why})",
                path.display()
            );
        }
    }

    // Product policy must not live in Agent Core: the Reference Product policy
    // types appear only in opi-coding-agent source.
    for (path, source) in &sources {
        let in_core = path
            .components()
            .any(|c| c.as_os_str().to_str() == Some("opi-agent"))
            || path
                .components()
                .any(|c| c.as_os_str().to_str() == Some("opi-ai"));
        if !in_core {
            continue;
        }
        for symbol in [
            "PermissionPolicy",
            "EffectiveUserPolicy",
            "ProductToolAuthorizer",
        ] {
            let sites = token_sites(path, source, symbol);
            assert_eq!(
                sites,
                Vec::<String>::new(),
                "product policy symbol `{symbol}` must not gain a new Agent Core site"
            );
        }
    }

    // The alias-registry and compatibility-shim rows of the removal audit have
    // no single scannable symbol: their absence is proven behaviorally by the
    // provider_runtime.rs no-alias/bare-model-ambiguity tests rather than by
    // a token scan here.
}

// ===========================================================================
// P17-EVD-010 (phase-exit closure) — the core-adapter boundary is ENFORCED:
// Agent Core (opi-agent) ships only the no-op and in-memory EvidenceSink
// implementations; file capture and exporters live outside it.
// ===========================================================================

#[test]
fn phase17_core_evidence_adapters_are_limited_to_noop_and_in_memory() {
    let sources = production_sources();
    let core_sources = sources
        .iter()
        .filter(|(path, _)| {
            path.components()
                .any(|component| component.as_os_str().to_str() == Some("opi-agent"))
        })
        .map(|(_, source)| source.as_str())
        .collect::<Vec<_>>();
    for (path, source) in &sources {
        let is_opi_agent = path
            .components()
            .any(|c| c.as_os_str().to_str() == Some("opi-agent"));
        if !is_opi_agent {
            continue;
        }
        // No file capture or exporter surface may exist in Agent Core.
        for symbol in ["FileEvidenceSink", "Exporter", "exporter"] {
            assert_eq!(
                source_token_occurrences(source, symbol),
                0,
                "evidence adapter/exporter symbol `{symbol}` must not live in opi-agent: {}",
                path.display()
            );
        }
    }
    let mut core_impl_sites = evidence_sink_impl_targets_across_sources(&core_sources);
    core_impl_sites.sort();
    assert_eq!(
        core_impl_sites,
        ["InMemoryEvidenceSink", "NoopEvidenceSink"],
        "Agent Core implements EvidenceSink exactly for the admitted no-op and in-memory adapters"
    );
}

// ===========================================================================
// P17-PLT-002 — Phase 17 tests call no paid/live provider endpoints. Local
// loopback servers and URL-shaped redaction data remain hermetic fixtures.
// ===========================================================================

#[test]
fn phase17_token_guard_ignores_trivia_and_finds_real_token_sequences() {
    let trivia_only = r###"
        // SharedProvider BeforeToolCallResult::Allow
        /* outer SharedProvider /* BeforeToolCallResult::Allow */ done */
        const NORMAL: &str = "SharedProvider BeforeToolCallResult::Allow";
        const BYTES: &[u8] = b"SharedProvider BeforeToolCallResult::Allow";
        const RAW: &str = r#"SharedProvider BeforeToolCallResult::Allow"#;
        const BYTE_RAW: &[u8] = br##"SharedProvider BeforeToolCallResult::Allow"##;
        const CHARACTER: char = 'X';
        const BYTE_CHARACTER: u8 = b'X';
    "###;
    for forbidden in ["SharedProvider", "BeforeToolCallResult::Allow", "X"] {
        assert_eq!(
            source_token_occurrences(trivia_only, forbidden),
            0,
            "Rust trivia or literals must not create a `{forbidden}` token hit"
        );
    }

    let real_tokens = r#"
        fn nested() {
            BeforeToolCallResult /* path trivia */ :: /* more trivia */ Allow;
            if true { SharedProvider::new(); }
        }
    "#;
    for forbidden in ["BeforeToolCallResult::Allow", "SharedProvider"] {
        assert_eq!(
            source_token_occurrences(real_tokens, forbidden),
            1,
            "real forbidden tokens remain visible across comments and token-tree nesting"
        );
    }

    for source in [
        "use BeforeToolCallResult::{Allow};",
        "use BeforeToolCallResult /* trivia */ :: { Nested :: { Allow } };",
        "enum BeforeToolCallResult { Continue, /* restored */ Allow }",
        "enum BeforeToolCallResult<T = ()> where T: X { Continue, Allow }",
        "impl BeforeToolCallResult { pub const Allow: Self = Self::Continue; }",
        "impl r#BeforeToolCallResult { pub const r#Allow: Self = Self::Continue; }",
        "impl BeforeToolCallResult /* trivia */ { pub const /* trivia */ Allow: Self = Self::Continue; }",
    ] {
        assert_eq!(
            source_token_occurrences(source, "BeforeToolCallResult::Allow"),
            1,
            "grouped paths and enum declarations cannot hide the removed Allow variant: {source}"
        );
    }

    assert_eq!(
        source_token_occurrences(
            "impl<T: BeforeToolCallResult> Other<T> { const Allow: Self = Self; }",
            "BeforeToolCallResult::Allow",
        ),
        0,
        "an inherent impl generic bound is not the associated-item target"
    );
}

#[test]
fn phase17_evidence_impl_guard_parses_qualified_comment_separated_paths() {
    let qualified_impl = r#"
        impl opi_agent /* qualifier trivia */ :: evidence :: EvidenceSink
            for crate /* target trivia */ :: CustomSink
        {}
    "#;
    assert_eq!(
        evidence_sink_impl_targets(qualified_impl),
        vec!["crate::CustomSink"],
        "qualified and comment-separated EvidenceSink implementations are discovered"
    );

    let aliased_impl = r#"
        use opi_agent /* qualifier trivia */ :: evidence :: EvidenceSink as Sink;
        impl Sink for crate::AliasedSink {}
    "#;
    assert_eq!(
        evidence_sink_impl_targets(aliased_impl),
        vec!["crate::AliasedSink"],
        "local EvidenceSink aliases cannot hide an implementation"
    );

    let chained_alias_impl = r#"
        use opi_agent::evidence::EvidenceSink as Sink;
        use Sink as Alias;
        impl Alias for crate::ChainedAliasSink {}
    "#;
    assert_eq!(
        evidence_sink_impl_targets(chained_alias_impl),
        vec!["crate::ChainedAliasSink"],
        "local EvidenceSink aliases resolve to a fixed point"
    );

    let raw_keyword_idents = r#"
        impl crate::r#for::EvidenceSink for crate::r#where::RawKeywordSink {}
    "#;
    assert_eq!(
        evidence_sink_impl_targets(raw_keyword_idents),
        vec!["crate::r#where::RawKeywordSink"],
        "raw identifiers in trait paths and self types are not impl keywords"
    );

    let impl_trivia = r###"
        // impl EvidenceSink for CommentSink {}
        const NORMAL: &str = "impl EvidenceSink for StringSink {}";
        const RAW: &str = r#"impl EvidenceSink for RawStringSink {}"#;
    "###;
    assert!(
        evidence_sink_impl_targets(impl_trivia).is_empty(),
        "implementation text in comments and strings is ignored"
    );

    let trait_bound_only = r#"
        impl<T: EvidenceSink> OtherTrait for BoundOnly<T> {}
    "#;
    assert!(
        evidence_sink_impl_targets(trait_bound_only).is_empty(),
        "a generic EvidenceSink bound is not an EvidenceSink implementation"
    );
}

#[test]
fn phase17_evidence_impl_guard_resolves_cross_source_reexports() {
    let sources = [
        "pub use opi_agent::evidence::EvidenceSink as Sink;",
        "use crate::Sink as Alias; impl Alias for crate::CrossSourceSink {}",
    ];
    assert_eq!(
        evidence_sink_impl_targets_across_sources(&sources),
        vec!["crate::CrossSourceSink"],
        "EvidenceSink re-export aliases resolve across scanned core sources"
    );
}

#[test]
fn phase17_evidence_impl_guard_renders_complete_group_self_types() {
    let source = r#"
        impl EvidenceSink for (CustomSink, Other) {}
        impl<T> crate::evidence::EvidenceSink for crate::CustomSink<T> {}
        impl EvidenceSink for (crate::ParenthesizedSink<Model>) {}
    "#;
    assert_eq!(
        evidence_sink_impl_targets(source),
        [
            "(CustomSink,Other)",
            "crate::CustomSink<T>",
            "(crate::ParenthesizedSink<Model>)",
        ],
        "the complete legal self type is retained for the core-adapter audit"
    );
}

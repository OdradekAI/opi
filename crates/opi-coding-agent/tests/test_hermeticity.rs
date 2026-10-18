//! Offline acceptance coverage, source discovery and platform-neutral test guards.

// opi-runtime-acceptance
#[path = "common/source_audit.rs"]
mod source_audit;

use source_audit::*;

#[test]
fn phase17_tests_are_hermetic_no_network_no_paid_providers() {
    let root = workspace_root();
    for relative in RUNTIME_ACCEPTANCE_SOURCES {
        let path = root.join(relative);
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read acceptance source {relative}: {error}"));
        let violations = hermetic_source_violations(&source);
        assert!(
            violations.is_empty(),
            "Phase 17 acceptance source contains live HTTP call sites: {relative}: {violations:?}"
        );
    }
}

#[test]
fn phase17_acceptance_source_manifest_is_complete_and_platform_neutral() {
    let root = workspace_root();
    let discovered = discover_acceptance_sources_under(
        &root,
        &[
            root.join("crates/opi-agent/tests"),
            root.join("crates/opi-agent/src"),
            root.join("crates/opi-ai/tests"),
            root.join("crates/opi-ai/src"),
            root.join("crates/opi-coding-agent/tests"),
            root.join("crates/opi-coding-agent/src"),
        ],
    );
    let discovered_refs = discovered.iter().map(String::as_str).collect::<Vec<_>>();
    acceptance_manifest_difference(RUNTIME_ACCEPTANCE_SOURCES, &discovered_refs)
        .unwrap_or_else(|difference| panic!("Phase 17 source manifest drift: {difference}"));

    let mut registered = RUNTIME_ACCEPTANCE_SOURCES.to_vec();
    registered.sort_unstable();
    assert_eq!(
        registered, RUNTIME_ACCEPTANCE_SOURCES,
        "complete acceptance manifest remains sorted for review"
    );
    for relative in RUNTIME_ACCEPTANCE_SOURCES {
        let path = root.join(relative);
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read acceptance source {relative}: {error}"));
        let violations = acceptance_source_violations(relative, &source);
        assert!(
            violations.is_empty(),
            "acceptance source {relative} is not platform-neutral: {violations:?}"
        );
    }
}

#[test]
fn phase17_attribute_guard_distinguishes_attributes_from_trivia() {
    let disabling_attributes = [
        "#[ignore]\n#[test]\nfn skipped() {}",
        "#[ignore = \"slow on CI\"]\n#[test]\nfn skipped() {}",
        "#[r#ignore]\n#[test]\nfn skipped() {}",
        "#[r#ignore = \"raw reason\"]\n#[test]\nfn skipped() {}",
        "#[cfg_attr(windows, ignore)]\n#[test]\nfn skipped() {}",
        "#[cfg_attr(test, r#ignore)]\n#[test]\nfn skipped() {}",
        "#[cfg_attr(any(unix, windows), cfg_attr(test, ignore = \"platform\"))]\n#[test]\nfn skipped() {}",
        "#[cfg_attr(test, cfg_attr(test, r#ignore = \"nested raw\"))]\n#[test]\nfn skipped() {}",
        "#[cfg_attr(windows, cfg(any()))]\n#[test]\nfn skipped() {}",
        "#[cfg_attr(test, test)]\nfn conditionally_a_test() {}",
        "#[cfg_attr(windows, tokio::test)]\nfn conditionally_a_qualified_test() {}",
        "#[cfg_attr(windows, cfg_attr(unix, tokio::test))]\nfn conditionally_a_nested_qualified_test() {}",
        "#[r#cfg_attr(windows, r#cfg(any()))]\n#[test]\nfn skipped() {}",
        "#[cfg_attr(test, cfg_attr(windows, r#test))]\nfn conditionally_a_test() {}",
    ];
    for source in disabling_attributes {
        assert!(
            !acceptance_source_violations("<canary>", source).is_empty(),
            "guard accepted an ignored acceptance test: {source}"
        );
    }

    let attribute_trivia = r###"
        // #[ignore]
        // #[r#ignore]
        /* #[cfg_attr(windows, ignore)] */
        const NORMAL: &str = "#[ignore = \"not an attribute\"]";
        const RAW: &str = r#"#[cfg_attr(unix, ignore)]"#;
        const RAW_IDENT_TEXT: &str = "#[r#ignore]";
    "###;
    assert!(
        acceptance_source_violations("<canary>", attribute_trivia).is_empty(),
        "ignore/cfg text in comments and strings is not an attribute"
    );
}

#[test]
fn phase17_attribute_guard_allows_harmless_cfg_attr() {
    for harmless_conditional_attribute in [
        "#[cfg_attr(test, allow(dead_code))]\n#[test]\nfn still_runs() {}",
        "#[cfg_attr(test, cfg_attr(test, allow(dead_code)))]\n#[test]\nfn still_runs() {}",
        "#[cfg_attr(test, warn(dead_code))]\n#[test]\nfn still_runs() {}",
        "#[cfg_attr(test, deny(unused_variables))]\n#[test]\nfn still_runs() {}",
    ] {
        assert!(
            acceptance_source_violations("<canary>", harmless_conditional_attribute).is_empty(),
            "a cfg_attr that cannot disable the test is harmless"
        );
    }
}

#[test]
fn phase17_required_non_named_sources_are_discovered_from_markers() {
    let root = workspace_root();
    let discovered = discover_acceptance_sources_under(
        &root,
        &[
            root.join("crates/opi-agent/tests"),
            root.join("crates/opi-agent/src"),
            root.join("crates/opi-ai/tests"),
            root.join("crates/opi-ai/src"),
            root.join("crates/opi-coding-agent/tests"),
            root.join("crates/opi-coding-agent/src"),
        ],
    );
    for required in [
        "crates/opi-agent/tests/agent_loop_semantics.rs",
        "crates/opi-agent/tests/evidence_contract.rs",
        "crates/opi-agent/tests/evidence_runtime.rs",
        "crates/opi-agent/tests/tool_authority.rs",
        "crates/opi-ai/tests/auth_contracts.rs",
        "crates/opi-ai/tests/oauth_wire_shape.rs",
        "crates/opi-ai/tests/per_request_auth.rs",
        "crates/opi-coding-agent/src/rpc.rs",
    ] {
        assert!(
            discovered.iter().any(|source| source == required),
            "required acceptance source is independently discovered: {required}"
        );
    }
}

#[test]
fn phase17_hermetic_guard_rejects_live_url_calls_on_any_non_fixture_host() {
    for source in [
        r#"reqwest::get("https://api.mistral.ai/v1/models").await;"#,
        r##"client.post(r#"https://openrouter.ai/api/v1/chat"#);"##,
        r#"connect("https://chatgpt.com/backend-api");"#,
        r#"Provider::new(Some("https://api.individual.githubcopilot.com"));"#,
        r#"HttpClient::request("https://arbitrary-provider.vendor/v1");"#,
        r#"client.get(concat!("https://api.", "githubcopilot.com"));"#,
        r#"client.get("https:\x2f\x2fapi.mistral.ai/v1");"#,
        r#"client.get("HTTPS://UPPERCASE.vendor/v1");"#,
        r#"assert!(reqwest::get("https://api.mistral.ai/v1").await.is_ok());"#,
        r#"client.get(format!("https://{}", "api.mistral.ai"));"#,
        r#"client.get(format!("https://{0}", "api.mistral.ai"));"#,
        r#"client.get(format!("https://{host}", host = "api.mistral.ai"));"#,
        r#"client.get(format!("https://{host}", host = provider_host()));"#,
        r#"client.get(format!("{}{}", "https://", provider_host()));"#,
        r#"client.get(format!("{scheme}://{host}", scheme = provider_scheme(), host = provider_host()));"#,
        r#"client.get(concat!("http://127.0.0.1:", dynamic_authority_suffix()));"#,
        r#"client.get(format!("https://{prefix}.example.test", prefix = dynamic_prefix()));"#,
        r#"client.get(format!("{}://{:.14}", "https", "api.mistral.ai.example.test"));"#,
        r#"client.get(provider_url());"#,
        r#"fn call() { let endpoint = "https://api.mistral.ai/v1"; client.get(endpoint); }"#,
        r#"fn call() { let endpoint = provider_url(); client.get(endpoint); }"#,
        r#"client.get(format!("http://127.0.0.1:{}", port));"#,
        r#"client.get(concat!("https://", concat!("api.", "mistral.ai")));"#,
        r#"client.get("https:\u{2_f}\u{2_f}api.mistral.ai/v1");"#,
        r#"fn misuse() { let redaction_fixture = "https://api.mistral.ai"; Provider::new(redaction_fixture); }"#,
    ] {
        assert!(
            !hermetic_source_violations(source).is_empty(),
            "live/non-fixture endpoint escaped the hermetic guard: {source}"
        );
    }
}

#[test]
fn phase17_hermetic_binding_chain_uses_declaration_time() {
    let source = r#"
        let source = provider_url();
        let endpoint = source;
        let source = "http://127.0.0.1:4317";
        client.get(endpoint);
    "#;
    assert!(
        !hermetic_source_violations(source).is_empty(),
        "a later safe shadow must not rewrite an earlier binding chain"
    );

    let safe_chain = r#"
        let source = "http://127.0.0.1:4317";
        let endpoint = source;
        client.get(endpoint);
    "#;
    assert!(
        hermetic_source_violations(safe_chain).is_empty(),
        "an immutable declaration-ordered loopback chain is proven safe"
    );
}

#[test]
fn phase17_hermetic_mutable_binding_fails_closed() {
    let source = r#"
        let mut endpoint = "http://127.0.0.1:4317";
        endpoint = provider_url();
        client.get(endpoint);
    "#;
    assert!(
        !hermetic_source_violations(source).is_empty(),
        "mutable or reassigned endpoint bindings must fail closed"
    );
}

#[test]
fn phase17_hermetic_method_chain_receiver_fails_closed() {
    for unknown in [
        r#"reqwest::Client::new().get(provider_url());"#,
        r#"reqwest::Client::new().get("https://provider.example.test/v1");"#,
    ] {
        assert!(
            !hermetic_source_violations(unknown).is_empty(),
            "an HTTP client constructor chain must not hide a non-local URL: {unknown}"
        );
    }

    for fixture in [
        r#"reqwest::Client::new().get("http://127.0.0.1:4317");"#,
        r#"HashMap::new().get(key);"#,
        r#"Vec::new().get(0);"#,
    ] {
        assert!(
            hermetic_source_violations(fixture).is_empty(),
            "an HTTP client constructor chain retains proven fixture URLs: {fixture}"
        );
    }
}

#[test]
fn phase17_hermetic_fluent_http_chain_fails_closed() {
    let unknown =
        r#"reqwest::Client::builder().redirect(policy()).build().unwrap().get(provider_url());"#;
    assert!(
        !hermetic_source_violations(unknown).is_empty(),
        "an arbitrary fluent HTTP client chain must not hide an unresolved URL"
    );

    let fixture = r#"reqwest::Client::builder().redirect(policy()).build().unwrap().get("http://127.0.0.1:4317");"#;
    assert!(
        hermetic_source_violations(fixture).is_empty(),
        "an arbitrary fluent HTTP client chain retains a proven loopback URL"
    );
}

#[test]
fn phase17_hermetic_concrete_provider_constructors_fail_closed() {
    for unknown in [
        r#"let base_url = std::env::var("LIVE_URL").ok(); AnthropicProvider::new(base_url);"#,
        r#"OpenAiChatProvider::new(provider_url());"#,
        r#"OpenAiResponsesProvider::new(provider_url());"#,
        r#"GeminiProvider::new(provider_url());"#,
        r#"BedrockProvider::new(provider_url(), client);"#,
        r#"OpenAiCodexResponsesProvider::new(provider_url(), models, client);"#,
        r#"AzureOpenAIProvider::new(provider_url(), deployment, api_version);"#,
        r#"VertexProvider::new(project, location, provider_url());"#,
        r#"OpenAiChatProvider::new_for_profile(provider_url(), id, compat, headers, models);"#,
        r#"mistral_provider(provider_url());"#,
        r#"openrouter_provider(provider_url());"#,
        r#"AnthropicProvider::new(None);"#,
        r#"OpenAiChatProvider::new(None);"#,
        r#"let base_url: Option<String> = Some("https://provider.example.test/v1".to_string()); AnthropicProvider::new(base_url);"#,
        r#"fn construct(server_uri: String) { AnthropicProvider::new(Some(server_uri)); }"#,
        r#"fn construct(server_uri: String) { AnthropicProvider::new(Some(server_uri)); } fn exercise() { construct(provider_url()); }"#,
        r#"ContestProvider::new(provider_url());"#,
        r#"LatestProvider::new(provider_url());"#,
        r#"CustomProvider::from_config(provider_url());"#,
        r#"CustomProvider::with_client(client, provider_url());"#,
        r#"CustomProvider::builder();"#,
    ] {
        assert!(
            !hermetic_source_violations(unknown).is_empty(),
            "a concrete provider constructor must not hide an unresolved base URL: {unknown}"
        );
    }

    for fixture in [
        r#"let base_url: Option<String> = Some("http://127.0.0.1:4317".to_owned()); AnthropicProvider::new(base_url);"#,
        r#"VertexProvider::new(project(), location(), Some("http://[::1]:4317".to_owned()));"#,
        r#"OpenAiChatProvider::new_for_profile("http://localhost:4317".to_string(), id, compat, headers, models);"#,
        r#"AnthropicProvider::for_route(id, models, Some("http://127.0.0.1:4317".into()), headers, client, false);"#,
        r#"fn construct(server_uri: String) { AnthropicProvider::new(Some(server_uri)); } fn exercise() { let server = MockServer::start().await; construct(server.uri()); }"#,
        r#"fn construct(server: &MockServer) { AnthropicProvider::new(Some(server.uri())); }"#,
    ] {
        assert!(
            hermetic_source_violations(fixture).is_empty(),
            "a concrete provider constructor retains a proven fixture URL: {fixture}"
        );
    }

    for unrelated in [
        r#"MockProvider::new(provider_url());"#,
        r#"TestProvider::new(provider_url());"#,
        r#"RecordingProvider::new(provider_url());"#,
        r#"PendingProvider::new(provider_url());"#,
        r#"UnrelatedProviderLike::new(provider_url());"#,
        r#"Mock::mistral_provider(provider_url());"#,
        r#"Models::new();"#,
    ] {
        assert!(
            hermetic_source_violations(unrelated).is_empty(),
            "a non-network provider constructor is outside the URL-consumer surface: {unrelated}"
        );
    }
}

#[test]
fn phase17_hermetic_executable_consumers_require_local_endpoints() {
    for executable in [
        r#"reqwest::get("https://example.com").await;"#,
        r#"let provider = AnthropicProvider::new(None); provider.stream_prepared(request, auth).await;"#,
        r#"let provider = CustomProvider::new(provider_url()); provider.stream_prepared(request, auth).await;"#,
        r#"let provider = CustomProvider::new(); provider.stream_prepared(request, auth).await;"#,
        r#"let provider = CustomProvider::default(); provider.stream_prepared(request, auth).await;"#,
        r#"let mut collection = Models::new(); collection.register_route(Box::new(CustomProvider::new()), resolver, provenance, compat); collection.prepare_call(model, request).await;"#,
    ] {
        assert!(
            !hermetic_source_violations(executable).is_empty(),
            "an executable network path must resolve to a local endpoint: {executable}"
        );
    }

    for fixture in [
        r#"reqwest::get("http://127.0.0.1:4317").await;"#,
        r#"let redaction_fixture = "https://example.com/secret"; assert!(!redact(redaction_fixture).is_empty());"#,
        r#"let provider = MockProvider::new(provider_url()); provider.stream_prepared(request, auth).await;"#,
        r#"let provider = RecordingProvider::new(provider_url()); provider.stream_prepared(request, auth).await;"#,
        r#"let provider = MockProvider::default(); provider.stream_prepared(request, auth).await;"#,
        r#"ControlledProvider::new(provider_url());"#,
        r#"let kind = ProviderKind::Mock; fn accepts<T: Provider>() {}"#,
    ] {
        assert!(
            hermetic_source_violations(fixture).is_empty(),
            "observation data and explicit non-network providers remain hermetic: {fixture}"
        );
    }
}

#[test]
fn phase17_hermetic_guard_allows_loopback_reserved_and_redaction_fixtures() {
    let source = r###"
        fn redaction_contract() {
            reqwest::get("http://127.0.0.1:4317/v1");
            client.post("http://[::1]:8080/callback");
            client.get(format!("http://{0}:8080", "127.0.0.1"));
            client.get(format!("http://127.0.0.1:{}", addr.port()));
            client.get(format!("http://127.0.0.1:{}", 8080));
            client.get(concat!("http://", concat!("127.0.", "0.1:8080")));
            let local_endpoint = "http://127.0.0.1:4317/v1";
            client.get(local_endpoint);
            RegisteredTool::new(format!("test-{name}"));
            command_tx.send(RpcCommand::prompt { message: format!("p{run}") });
            let redaction_fixture = "https://api.mistral.ai/secret?token=canary";
            assert!(!redact(redaction_fixture).contains("token=canary"));
            assert!(!redact(format!("{}{}", "https://", provider_host())).contains("token"));
            assert!(!redact(concat!("https://", concat!("api.", "mistral.ai"))).contains("token"));
            // reqwest::get("https://openrouter.ai/api/v1");
            const COMMENT_LIKE: &str = r#"// https://chatgpt.com/backend-api"#;
        }
    "###;
    assert!(
        hermetic_source_violations(source).is_empty(),
        "local/reserved and nonexecuted redaction fixtures remain hermetic: {:?}",
        hermetic_source_violations(source)
    );
}

#[test]
fn phase17_acceptance_manifest_guard_rejects_an_omission() {
    let fixture = tempfile::tempdir().unwrap();
    let tests = fixture.path().join("crates/fixture/tests");
    std::fs::create_dir_all(&tests).unwrap();
    std::fs::write(
        tests.join("ordinary_name.rs"),
        "#[test]\nfn phase17_acceptance_hidden_by_an_ordinary_filename() {}\n",
    )
    .unwrap();
    std::fs::write(
        tests.join("explicit_marker.rs"),
        "// opi-phase17-acceptance\n#[test]\nfn acceptance_with_stable_name() {}\n",
    )
    .unwrap();
    std::fs::write(
        tests.join("qualified_marker.rs"),
        "// opi-phase17-acceptance\n#[tokio::test]\nasync fn qualified_acceptance_with_stable_name() {}\n",
    )
    .unwrap();
    std::fs::write(
        tests.join("conditional_qualified_marker.rs"),
        "// opi-phase17-acceptance\n#[cfg_attr(windows, tokio::test)]\nasync fn phase17_case() {}\n",
    )
    .unwrap();
    std::fs::write(
        tests.join("marker_in_string.rs"),
        "const MARKER: &str = \"// opi-phase17-acceptance\";\n#[test]\nfn ordinary_test() {}\n",
    )
    .unwrap();

    let discovered = discover_acceptance_sources_under(fixture.path(), &[tests]);
    assert_eq!(
        discovered,
        [
            "crates/fixture/tests/conditional_qualified_marker.rs",
            "crates/fixture/tests/explicit_marker.rs",
            "crates/fixture/tests/ordinary_name.rs",
            "crates/fixture/tests/qualified_marker.rs",
        ],
        "discovery parses Phase 17 test names and source-aware markers outside phase17 filenames"
    );
    let discovered_refs = discovered.iter().map(String::as_str).collect::<Vec<_>>();
    assert!(
        acceptance_manifest_difference(&[], &discovered_refs).is_err(),
        "an omitted acceptance source must fail the manifest guard"
    );
}

#[test]
fn runtime_acceptance_helpers_are_scanned_without_test_functions() {
    let fixture = tempfile::tempdir().unwrap();
    let helpers = fixture.path().join("crates/fixture/tests/common");
    std::fs::create_dir_all(&helpers).unwrap();
    let source = "// opi-runtime-acceptance\nfn helper() { reqwest::get(\"https://api.mistral.ai/v1/models\"); }\n";
    std::fs::write(helpers.join("runtime_evidence.rs"), source).unwrap();
    std::fs::write(
        helpers.join("marker_in_string.rs"),
        "const MARKER: &str = \"// opi-runtime-acceptance\";\nfn helper() {}\n",
    )
    .unwrap();

    let discovered = discover_acceptance_sources_under(fixture.path(), &[helpers]);
    assert_eq!(
        discovered,
        ["crates/fixture/tests/common/runtime_evidence.rs"],
        "a marked helper is independently discovered without a test attribute"
    );
    let discovered_refs = discovered.iter().map(String::as_str).collect::<Vec<_>>();
    assert!(acceptance_manifest_difference(&[], &discovered_refs).is_err());
    assert!(
        !hermetic_source_violations(source).is_empty(),
        "discovered helper code remains subject to the offline-call guard"
    );
}

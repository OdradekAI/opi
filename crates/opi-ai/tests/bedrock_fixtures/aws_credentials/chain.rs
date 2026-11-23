//! AWS credential-source chain tests.

use super::*;

#[tokio::test]
async fn bedrock_resolution_reports_environment_profile_config_and_process_provenance() {
    let ambient = MapAuthContext(std::collections::HashMap::from([
        ("AWS_ACCESS_KEY_ID", "ENV_ACCESS"),
        ("AWS_SECRET_ACCESS_KEY", "ENV_SECRET"),
        ("AWS_SESSION_TOKEN", "ENV_SESSION"),
        ("AWS_REGION", "us-west-2"),
    ]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    // Complete ambient environment pair is selected when no profile governs.
    let environment = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    );
    assert_eq!(
        chain_source(&environment).await,
        CredentialSource::Environment
    );

    let dir = tempfile::tempdir().unwrap();
    let credentials_file = dir.path().join("credentials");
    std::fs::write(
        &credentials_file,
        "[fixture]\naws_access_key_id=PROFILE_ACCESS\naws_secret_access_key=PROFILE_SECRET\n",
    )
    .unwrap();
    let profile = chain_input(
        &ambient,
        Some(credentials_file.as_path()),
        None,
        &runner,
        AwsAuthSelection::Profile("fixture".to_owned()),
        ScopedAwsEnv::empty(),
    );
    assert_eq!(chain_source(&profile).await, CredentialSource::ProfileFile);

    let config_file = dir.path().join("config-static");
    std::fs::write(
        &config_file,
        "[profile fixture]\naws_access_key_id=CONFIG_ACCESS\naws_secret_access_key=CONFIG_SECRET\n",
    )
    .unwrap();
    let config = chain_input(
        &ambient,
        None,
        Some(config_file.as_path()),
        &runner,
        AwsAuthSelection::Profile("fixture".to_owned()),
        ScopedAwsEnv::empty(),
    );
    assert_eq!(chain_source(&config).await, CredentialSource::ConfigFile);

    let process_config = dir.path().join("config-process");
    std::fs::write(
        &process_config,
        "[profile fixture]\ncredential_process=fixture-credential-process\n",
    )
    .unwrap();
    let process = chain_input(
        &ambient,
        None,
        Some(process_config.as_path()),
        &runner,
        AwsAuthSelection::Profile("fixture".to_owned()),
        ScopedAwsEnv::empty(),
    );
    assert_eq!(
        chain_source(&process).await,
        CredentialSource::CredentialProcess
    );
    assert_eq!(
        runner.calls.lock().expect("process log").as_slice(),
        ["fixture-credential-process"],
        "one process execution per discovery round"
    );
}

#[tokio::test]
async fn complete_ambient_pair_wins_over_ambient_profile_without_explicit_selection() {
    // Only an ambient AWS_PROFILE does not beat a complete ambient pair; the
    // verified pi adapter injects the env pair unless an explicit/scoped
    // profile governs.
    let ambient = MapAuthContext(std::collections::HashMap::from([
        ("AWS_ACCESS_KEY_ID", "ENV_ACCESS"),
        ("AWS_SECRET_ACCESS_KEY", "ENV_SECRET"),
        ("AWS_PROFILE", "fixture"),
    ]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let dir = tempfile::tempdir().unwrap();
    let credentials_file = dir.path().join("credentials");
    std::fs::write(
        &credentials_file,
        "[fixture]\naws_access_key_id=PROFILE_ACCESS\naws_secret_access_key=PROFILE_SECRET\n",
    )
    .unwrap();
    let input = chain_input(
        &ambient,
        Some(credentials_file.as_path()),
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    );
    assert_eq!(chain_source(&input).await, CredentialSource::Environment);
    assert!(
        runner.calls.lock().expect("process log").is_empty(),
        "no lower source runs once the env pair is selected"
    );
}

#[tokio::test]
async fn scoped_profile_skips_the_ambient_environment_pair() {
    let ambient = MapAuthContext(std::collections::HashMap::from([
        ("AWS_ACCESS_KEY_ID", "ENV_ACCESS"),
        ("AWS_SECRET_ACCESS_KEY", "ENV_SECRET"),
    ]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let dir = tempfile::tempdir().unwrap();
    let credentials_file = dir.path().join("credentials");
    std::fs::write(
        &credentials_file,
        "[fixture]\naws_access_key_id=PROFILE_ACCESS\naws_secret_access_key=PROFILE_SECRET\n",
    )
    .unwrap();
    let scoped = ScopedAwsEnv {
        profile: Some("fixture".to_owned()),
        ..ScopedAwsEnv::empty()
    };
    let input = chain_input(
        &ambient,
        Some(credentials_file.as_path()),
        None,
        &runner,
        AwsAuthSelection::Auto,
        scoped,
    );
    assert_eq!(chain_source(&input).await, CredentialSource::ProfileFile);
}

#[tokio::test]
async fn partial_scoped_pair_fails_closed_without_borrowing_ambient_halves() {
    let ambient = MapAuthContext(std::collections::HashMap::from([(
        "AWS_SECRET_ACCESS_KEY",
        "ENV_SECRET",
    )]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let scoped = ScopedAwsEnv {
        access_key_id: Some(SecretString::from("SCOPED_ACCESS")),
        ..ScopedAwsEnv::empty()
    };
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        scoped,
    );
    assert_eq!(
        resolve_local_aws_auth(&input).await.unwrap_err(),
        CredentialResolutionError::IncompleteSource {
            credential_source: CredentialSource::ScopedEnvironment,
        }
    );
    assert!(runner.calls.lock().expect("process log").is_empty());
}

#[tokio::test]
async fn partial_ambient_pair_continues_to_the_ini_profile() {
    let ambient = MapAuthContext(std::collections::HashMap::from([(
        "AWS_ACCESS_KEY_ID",
        "ENV_ACCESS",
    )]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let dir = tempfile::tempdir().unwrap();
    let credentials_file = dir.path().join("credentials");
    std::fs::write(
        &credentials_file,
        "[default]\naws_access_key_id=PROFILE_ACCESS\naws_secret_access_key=PROFILE_SECRET\n",
    )
    .unwrap();
    let input = chain_input(
        &ambient,
        Some(credentials_file.as_path()),
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    );
    assert_eq!(chain_source(&input).await, CredentialSource::ProfileFile);
}

#[tokio::test]
async fn static_ini_fields_overlay_config_and_credentials_files_per_field() {
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let dir = tempfile::tempdir().unwrap();
    let credentials_file = dir.path().join("credentials");
    let config_file = dir.path().join("config");
    // Split pair across files: access from credentials, secret from config,
    // session token only in config attaches after the per-field merge.
    std::fs::write(
        &credentials_file,
        "[fixture]\naws_access_key_id=CREDENTIALS_ACCESS\n",
    )
    .unwrap();
    std::fs::write(
        &config_file,
        "[profile fixture]\naws_secret_access_key=CONFIG_SECRET\naws_session_token=CONFIG_SESSION\nregion=eu-north-1\n",
    )
    .unwrap();
    let input = chain_input(
        &ambient,
        Some(credentials_file.as_path()),
        Some(config_file.as_path()),
        &runner,
        AwsAuthSelection::Profile("fixture".to_owned()),
        ScopedAwsEnv::empty(),
    );
    let auth = resolve_local_aws_auth(&input).await.expect("overlay pair");
    match auth.scheme {
        AuthScheme::AwsSigV4(credentials) => {
            assert_eq!(
                credentials.access_key_id.expose_secret(),
                "CREDENTIALS_ACCESS"
            );
            assert_eq!(
                credentials.secret_access_key.expose_secret(),
                "CONFIG_SECRET"
            );
            assert_eq!(
                credentials
                    .session_token
                    .as_ref()
                    .map(secrecy::ExposeSecret::expose_secret),
                Some("CONFIG_SESSION")
            );
            assert_eq!(credentials.region, "eu-north-1");
        }
        other => panic!("expected SigV4 credentials, got {other:?}"),
    }
    // The identity anchor (access key id) came from the credentials file.
    assert_eq!(chain_source(&input).await, CredentialSource::ProfileFile);

    // Per-field precedence: a credentials-file value overrides the config
    // file for the same key.
    std::fs::write(
        &credentials_file,
        "[fixture]\naws_access_key_id=CREDENTIALS_ACCESS\naws_secret_access_key=CREDENTIALS_SECRET\n",
    )
    .unwrap();
    let auth = resolve_local_aws_auth(&input).await.expect("override pair");
    match auth.scheme {
        AuthScheme::AwsSigV4(credentials) => assert_eq!(
            credentials.secret_access_key.expose_secret(),
            "CREDENTIALS_SECRET"
        ),
        other => panic!("expected SigV4 credentials, got {other:?}"),
    }
}

#[tokio::test]
async fn partial_static_ini_pair_advances_to_credential_process() {
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let dir = tempfile::tempdir().unwrap();
    let config_file = dir.path().join("config");
    std::fs::write(
        &config_file,
        "[profile fixture]\naws_access_key_id=PARTIAL_ACCESS\ncredential_process=fixture-process\n",
    )
    .unwrap();
    let input = chain_input(
        &ambient,
        None,
        Some(config_file.as_path()),
        &runner,
        AwsAuthSelection::Profile("fixture".to_owned()),
        ScopedAwsEnv::empty(),
    );
    assert_eq!(
        chain_source(&input).await,
        CredentialSource::CredentialProcess
    );
    assert_eq!(
        runner.calls.lock().expect("process log").as_slice(),
        ["fixture-process"]
    );
}

#[tokio::test]
async fn credential_process_failure_is_continue_eligible_and_exhausts_typed() {
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Err(opi_ai::auth::aws::process::CredentialProcessError::Timeout),
    };
    let dir = tempfile::tempdir().unwrap();
    let config_file = dir.path().join("config");
    std::fs::write(
        &config_file,
        "[profile fixture]\ncredential_process=fixture-process\n",
    )
    .unwrap();
    let input = chain_input(
        &ambient,
        None,
        Some(config_file.as_path()),
        &runner,
        AwsAuthSelection::Profile("fixture".to_owned()),
        ScopedAwsEnv::empty(),
    );
    let error = resolve_local_aws_auth(&input).await.unwrap_err();
    assert_eq!(error, CredentialResolutionError::Exhausted);
    let rendered = format!("{error:?} {error}");
    assert!(!rendered.contains("PROCESS_"));
    assert!(!rendered.contains("fixture-process"));
}

#[tokio::test]
async fn blank_credential_process_continues_without_spawning() {
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let dir = tempfile::tempdir().unwrap();
    let config_file = dir.path().join("config");
    std::fs::write(&config_file, "[profile fixture]\ncredential_process=   \n").unwrap();
    let input = chain_input(
        &ambient,
        None,
        Some(config_file.as_path()),
        &runner,
        AwsAuthSelection::Profile("fixture".to_owned()),
        ScopedAwsEnv::empty(),
    );
    assert_eq!(
        resolve_local_aws_auth(&input).await.unwrap_err(),
        CredentialResolutionError::Exhausted
    );
    assert!(runner.calls.lock().expect("process log").is_empty());
}

#[tokio::test]
async fn credential_process_expiry_is_retained_on_the_selected_credentials() {
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let dir = tempfile::tempdir().unwrap();
    let config_file = dir.path().join("config");
    std::fs::write(
        &config_file,
        "[profile fixture]\ncredential_process=fixture-process\nregion=us-west-1\n",
    )
    .unwrap();
    let input = chain_input(
        &ambient,
        None,
        Some(config_file.as_path()),
        &runner,
        AwsAuthSelection::Profile("fixture".to_owned()),
        ScopedAwsEnv::empty(),
    );
    let auth = resolve_local_aws_auth(&input)
        .await
        .expect("process source");
    match auth.scheme {
        AuthScheme::AwsSigV4(credentials) => {
            assert_eq!(
                credentials.expires_at,
                Some(
                    time::OffsetDateTime::from_unix_timestamp(4_102_444_800)
                        .expect("fixed future expiry")
                )
            );
            assert_eq!(credentials.region, "us-west-1");
        }
        other => panic!("expected SigV4 credentials, got {other:?}"),
    }
}

#[tokio::test]
async fn unreadable_profile_files_continue_to_lower_sources() {
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    // A directory in place of the credentials file fails the read probe; the
    // chain must continue (fixed Node corpus) instead of failing closed.
    let dir = tempfile::tempdir().unwrap();
    let unreadable = dir.path().join("credentials-directory");
    std::fs::create_dir(&unreadable).unwrap();
    let config_file = dir.path().join("config");
    std::fs::write(
        &config_file,
        "[profile fixture]\naws_access_key_id=CONFIG_ACCESS\naws_secret_access_key=CONFIG_SECRET\n",
    )
    .unwrap();
    let input = chain_input(
        &ambient,
        Some(unreadable.as_path()),
        Some(config_file.as_path()),
        &runner,
        AwsAuthSelection::Profile("fixture".to_owned()),
        ScopedAwsEnv::empty(),
    );
    assert_eq!(chain_source(&input).await, CredentialSource::ConfigFile);
}

#[tokio::test]
async fn explicit_selection_reads_no_source_and_wins_over_auto() {
    let ambient = MapAuthContext(std::collections::HashMap::from([
        ("AWS_ACCESS_KEY_ID", "ENV_ACCESS"),
        ("AWS_SECRET_ACCESS_KEY", "ENV_SECRET"),
        ("AWS_BEARER_TOKEN_BEDROCK", "ambient-bearer"),
    ]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let credentials = AwsCredentials {
        access_key_id: "AKIASELECTED".into(),
        secret_access_key: "selected-secret".into(),
        session_token: None,
        region: "ap-south-1".into(),
        expires_at: None,
    };
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::SigV4(credentials),
        ScopedAwsEnv::empty(),
    );
    let auth = resolve_local_aws_auth(&input)
        .await
        .expect("explicit selection resolves");
    match auth.scheme {
        AuthScheme::AwsSigV4(credentials) => {
            assert_eq!(credentials.access_key_id.expose_secret(), "AKIASELECTED");
        }
        other => panic!("expected SigV4 credentials, got {other:?}"),
    }
    assert_eq!(auth.provenance.source, AuthProvenanceSource::Explicit);
    assert!(runner.calls.lock().expect("process log").is_empty());
}

#[tokio::test]
async fn bearer_token_from_scoped_or_ambient_env_selects_bearer_scheme() {
    let ambient = MapAuthContext(std::collections::HashMap::from([
        ("AWS_ACCESS_KEY_ID", "ENV_ACCESS"),
        ("AWS_SECRET_ACCESS_KEY", "ENV_SECRET"),
        ("AWS_BEARER_TOKEN_BEDROCK", "ambient-bearer-token"),
    ]));
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let ambient_bearer = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    );
    let auth = resolve_local_aws_auth(&ambient_bearer)
        .await
        .expect("ambient bearer resolves");
    assert_eq!(auth.scheme, AuthScheme::Bearer);
    assert_eq!(auth.secret.expose_secret(), "ambient-bearer-token");
    assert_eq!(
        auth.provenance.source,
        AuthProvenanceSource::Environment {
            name: "AWS_BEARER_TOKEN_BEDROCK".to_owned(),
        }
    );
    assert_eq!(
        auth.base_url.as_deref(),
        Some("https://bedrock-runtime.us-east-1.amazonaws.com"),
        "bearer resolution wires the bedrock runtime endpoint for the region"
    );

    // Scoped bearer beats ambient bearer.
    let scoped = ScopedAwsEnv {
        bearer_token_bedrock: Some(SecretString::from("scoped-bearer-token")),
        ..ScopedAwsEnv::empty()
    };
    let scoped_bearer = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        scoped,
    );
    let auth = resolve_local_aws_auth(&scoped_bearer)
        .await
        .expect("scoped bearer resolves");
    assert_eq!(auth.secret.expose_secret(), "scoped-bearer-token");
}

#[tokio::test]
async fn region_precedence_spans_config_scoped_and_ambient_sources() {
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };

    let pair_ambient = |entries: &[(&'static str, &'static str)]| {
        let mut map = std::collections::HashMap::from([
            ("AWS_ACCESS_KEY_ID", "ENV_ACCESS"),
            ("AWS_SECRET_ACCESS_KEY", "ENV_SECRET"),
        ]);
        for (name, value) in entries {
            map.insert(name, value);
        }
        MapAuthContext(map)
    };

    // Ambient AWS_REGION beats ambient AWS_DEFAULT_REGION.
    let ambient = pair_ambient(&[
        ("AWS_REGION", "us-west-2"),
        ("AWS_DEFAULT_REGION", "us-west-3"),
    ]);
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    );
    assert_eq!(selected_region(&input).await, "us-west-2");

    // The AWS_REGION tier is read scoped-then-ambient, so ambient AWS_REGION
    // still beats scoped AWS_DEFAULT_REGION (verified adapter order).
    let scoped_default = || ScopedAwsEnv {
        default_region: Some("sa-east-1".to_owned()),
        ..ScopedAwsEnv::empty()
    };
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        scoped_default(),
    );
    assert_eq!(selected_region(&input).await, "us-west-2");

    // Scoped AWS_DEFAULT_REGION wins once the AWS_REGION tier is empty.
    let default_only = pair_ambient(&[("AWS_DEFAULT_REGION", "us-west-3")]);
    let input = chain_input(
        &default_only,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        scoped_default(),
    );
    assert_eq!(selected_region(&input).await, "sa-east-1");

    // Scoped AWS_REGION beats ambient AWS_REGION and scoped default.
    let scoped_region = ScopedAwsEnv {
        region: Some("ap-northeast-3".to_owned()),
        default_region: Some("sa-east-1".to_owned()),
        ..ScopedAwsEnv::empty()
    };
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        scoped_region,
    );
    assert_eq!(selected_region(&input).await, "ap-northeast-3");

    // An explicit region option beats every environment source.
    let mut input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    );
    input.region = Some("eu-central-1");
    assert_eq!(selected_region(&input).await, "eu-central-1");
}

#[tokio::test]
async fn no_local_source_resolves_to_typed_exhaustion() {
    let ambient = MapAuthContext(std::collections::HashMap::new());
    let runner = ScriptedProcessRunner {
        calls: std::sync::Mutex::new(Vec::new()),
        result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
    };
    let input = chain_input(
        &ambient,
        None,
        None,
        &runner,
        AwsAuthSelection::Auto,
        ScopedAwsEnv::empty(),
    );
    assert_eq!(
        resolve_local_aws_auth(&input).await.unwrap_err(),
        CredentialResolutionError::Exhausted
    );
}

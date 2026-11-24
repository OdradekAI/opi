//! Shared AWS credential-source fixtures and helper contracts.

#![allow(unused_imports)]

use super::super::*;

/// Runs the local AWS chain with injected inputs on each Models call.
pub(crate) struct LocalChainBedrockResolver {
    pub(crate) ambient: MapAuthContext,
    pub(crate) credentials_file: Option<std::path::PathBuf>,
    pub(crate) config_file: Option<std::path::PathBuf>,
}

impl AuthResolver for LocalChainBedrockResolver {
    fn resolve<'a>(&'a self) -> BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
        Box::pin(async move {
            let runner = ScriptedProcessRunner {
                calls: std::sync::Mutex::new(Vec::new()),
                result: Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET")),
            };
            let input = AwsLocalChainInput {
                selection: AwsAuthSelection::Auto,
                scoped: ScopedAwsEnv::empty(),
                ambient: &self.ambient,
                credentials_file: self.credentials_file.as_deref(),
                config_file: self.config_file.as_deref(),
                process: &runner,
                transport: &NEVER_TRANSPORT,
                mfa: None,
                sts_endpoint: None,
                source_cache: None,
                clock: &SYSTEM_SOURCE_CLOCK,
                dpop_jti: &RANDOM_DPPO_JTI,
                sso_cache_dir: None,
                login_cache_dir: None,
                sso_endpoint: None,
                oidc_endpoint: None,
                signin_endpoint: None,
                region: None,
                default_region: "us-east-1",
            };
            resolve_local_aws_auth(&input)
                .await
                .map_err(|error| match error {
                    CredentialResolutionError::Exhausted => ProviderError::CredentialNeeded {
                        provider_id: "bedrock".into(),
                    },
                    other => ProviderError::Config(other.into_safe_provider_summary()),
                })
        })
    }
}

/// Supplies chain-test environment values without reading the process environment.
pub(crate) struct MapAuthContext(pub std::collections::HashMap<&'static str, &'static str>);

impl AuthContext for MapAuthContext {
    fn env(&self, name: &str) -> Result<Option<String>, AuthContextError> {
        Ok(self.0.get(name).map(|value| (*value).to_owned()))
    }
    fn file_exists(&self, path: &str) -> Result<bool, AuthContextError> {
        Ok(std::path::Path::new(path).exists())
    }
}

pub(crate) struct ScriptedProcessRunner {
    pub(crate) calls: std::sync::Mutex<Vec<String>>,
    pub(crate) result:
        Result<CredentialProcessOutput, opi_ai::auth::aws::process::CredentialProcessError>,
}

impl CredentialProcessRunner for ScriptedProcessRunner {
    fn run<'a>(
        &'a self,
        spec: &'a CredentialProcessSpec,
    ) -> opi_ai::credential::BoxAuthFuture<
        'a,
        Result<CredentialProcessOutput, opi_ai::auth::aws::process::CredentialProcessError>,
    > {
        Box::pin(async move {
            self.calls
                .lock()
                .expect("process call log")
                .push(spec.command.clone());
            self.result.clone()
        })
    }
}

pub(crate) fn process_output(access: &str, secret: &str) -> CredentialProcessOutput {
    CredentialProcessOutput {
        version: 1,
        access_key_id: SecretString::from(access),
        secret_access_key: SecretString::from(secret),
        session_token: None,
        expiration: Some(
            time::OffsetDateTime::from_unix_timestamp(4_102_444_800).expect("fixed future expiry"),
        ),
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn chain_input<'a>(
    ambient: &'a dyn opi_ai::auth::AuthContext,
    credentials_file: Option<&'a std::path::Path>,
    config_file: Option<&'a std::path::Path>,
    process: &'a dyn CredentialProcessRunner,
    selection: AwsAuthSelection,
    scoped: ScopedAwsEnv,
) -> AwsLocalChainInput<'a> {
    AwsLocalChainInput {
        selection,
        scoped,
        ambient,
        credentials_file,
        config_file,
        process,
        transport: &NEVER_TRANSPORT,
        mfa: None,
        sts_endpoint: None,
        source_cache: None,
        clock: &SYSTEM_SOURCE_CLOCK,
        dpop_jti: &RANDOM_DPPO_JTI,
        sso_cache_dir: None,
        login_cache_dir: None,
        sso_endpoint: None,
        oidc_endpoint: None,
        signin_endpoint: None,
        region: None,
        default_region: "us-east-1",
    }
}

/// Transport for chain tests that expect no source exchange.
pub(crate) static NEVER_TRANSPORT: aws_source_transport::NeverTransport =
    aws_source_transport::NeverTransport;

pub(crate) static SYSTEM_SOURCE_CLOCK: opi_ai::auth::aws::cache::SystemSourceClock =
    opi_ai::auth::aws::cache::SystemSourceClock;

pub(crate) static RANDOM_DPPO_JTI: opi_ai::auth::aws::cache::RandomDpopJti =
    opi_ai::auth::aws::cache::RandomDpopJti;

pub(crate) async fn chain_source(input: &AwsLocalChainInput<'_>) -> CredentialSource {
    let auth = resolve_local_aws_auth(input)
        .await
        .expect("local AWS chain should resolve");
    let AuthProvenanceSource::AwsSigV4 { source } = auth.provenance.source else {
        panic!(
            "expected typed AWS provenance, got {:?}",
            auth.provenance.source
        )
    };
    match source {
        AwsCredentialSource::ExplicitConfig => CredentialSource::ExplicitConfig,
        AwsCredentialSource::ConfiguredEnvironment { .. } => {
            CredentialSource::ConfiguredEnvironment {
                secret_access_key_env: String::new(),
                session_token_env: None,
            }
        }
        AwsCredentialSource::Environment => CredentialSource::Environment,
        AwsCredentialSource::ScopedEnvironment => CredentialSource::ScopedEnvironment,
        AwsCredentialSource::ProfileFile => CredentialSource::ProfileFile,
        AwsCredentialSource::ConfigFile => CredentialSource::ConfigFile,
        AwsCredentialSource::CredentialProcess => CredentialSource::CredentialProcess,
        AwsCredentialSource::AssumeRole { profile } => CredentialSource::AssumeRole { profile },
        AwsCredentialSource::WebIdentity { route } => CredentialSource::WebIdentity { route },
        AwsCredentialSource::Sso { profile } => CredentialSource::Sso { profile },
        AwsCredentialSource::LoginSession { session } => CredentialSource::LoginSession { session },
        AwsCredentialSource::EcsContainer { legacy } => CredentialSource::EcsContainer { legacy },
        AwsCredentialSource::Ec2InstanceMetadata => CredentialSource::Ec2InstanceMetadata,
        AwsCredentialSource::ProxyPlaceholder => unreachable!("no placeholder in chain tests"),
    }
}

pub(crate) async fn selected_region(input: &AwsLocalChainInput<'_>) -> String {
    match resolve_local_aws_auth(input).await.expect("resolve").scheme {
        AuthScheme::AwsSigV4(credentials) => credentials.region,
        other => panic!("expected SigV4 credentials, got {other:?}"),
    }
}

pub(crate) struct PendingBedrockResolver {
    pub(crate) entered: Arc<Notify>,
}

impl AuthResolver for PendingBedrockResolver {
    fn resolve<'a>(&'a self) -> BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
        let entered = self.entered.clone();
        Box::pin(async move {
            entered.notify_one();
            std::future::pending().await
        })
    }
}

pub(crate) fn rfc3339(value: &str) -> time::OffsetDateTime {
    time::OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339).unwrap()
}

pub(crate) fn sigv4_of(auth: &ResolvedAuth) -> &opi_ai::auth::AwsSigV4Credentials {
    match &auth.scheme {
        AuthScheme::AwsSigV4(credentials) => credentials,
        other => panic!("expected SigV4 scheme, got {other:?}"),
    }
}

pub(crate) fn sts_success_xml(creds: &str, role_arn: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<AssumeRoleResponse xmlns="https://sts.amazonaws.com/doc/2011-06-15/">
<AssumeRoleResult>
<Credentials>
<AccessKeyId>{creds}ACCESS</AccessKeyId>
<SecretAccessKey>{creds}SECRET</SecretAccessKey>
<SessionToken>{creds}SESSION</SessionToken>
<Expiration>2027-06-01T12:00:00Z</Expiration>
</Credentials>
<AssumedRoleUser>
<Arn>{role_arn}</Arn>
<AssumedRoleId>AROAFIXTUREROLEID</AssumedRoleId>
</AssumedRoleUser>
</AssumeRoleResult>
<ResponseMetadata><RequestId>sts-request-1</RequestId></ResponseMetadata>
</AssumeRoleResponse>"#
    )
}

pub(crate) fn web_identity_success_xml(token_subject: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<AssumeRoleWithWebIdentityResponse xmlns="https://sts.amazonaws.com/doc/2011-06-15/">
<AssumeRoleWithWebIdentityResult>
<Subject>{token_subject}</Subject>
<Audience>fixture-audience</Audience>
<Provider>fixture-provider</Provider>
<Credentials>
<AccessKeyId>WEBACCESS</AccessKeyId>
<SecretAccessKey>WEBSECRET</SecretAccessKey>
<SessionToken>WEBSESSION</SessionToken>
<Expiration>2027-07-01T12:00:00Z</Expiration>
</Credentials>
<AssumedRoleUser>
<Arn>arn:aws:sts::123456789012:assumed-role/web-role/session</Arn>
<AssumedRoleId>AROAWEBROLEID</AssumedRoleId>
</AssumedRoleUser>
</AssumeRoleWithWebIdentityResult>
<ResponseMetadata><RequestId>sts-request-2</RequestId></ResponseMetadata>
</AssumeRoleWithWebIdentityResponse>"#
    )
}

pub(crate) fn sts_error_xml(code: &str) -> String {
    format!(
        r#"<ErrorResponse xmlns="https://sts.amazonaws.com/doc/2011-06-15/">
<Error><Type>Sender</Type><Code>{code}</Code><Message>denied detail</Message></Error>
<RequestId>sts-request-denied</RequestId>
</ErrorResponse>"#
    )
}

pub(crate) fn source_response(
    status: u16,
    body: impl Into<String>,
) -> Result<SourceHttpResponse, SourceHttpError> {
    Ok(SourceHttpResponse {
        status,
        body: body.into().into_bytes(),
    })
}

/// Writes a role profile and its static source profile.
pub(crate) fn write_role_profiles(
    dir: &std::path::Path,
) -> (std::path::PathBuf, std::path::PathBuf) {
    let credentials = dir.join("credentials");
    std::fs::write(
        &credentials,
        "[source-prof]\naws_access_key_id=SOURCE_ACCESS\naws_secret_access_key=SOURCE_SECRET\naws_session_token=SOURCE_SESSION\n\
         [role-prof]\naws_access_key_id=ROLE_OWN_STATIC_ACCESS\naws_secret_access_key=ROLE_OWN_STATIC_SECRET\n",
    )
    .unwrap();
    let config = dir.join("config");
    std::fs::write(
        &config,
        "[profile role-prof]\nrole_arn=arn:aws:iam::123456789012:role/fixture\nrole_session_name=fixture-session\nexternal_id=fixture-external\nduration_seconds=3600\nsource_profile=source-prof\nregion=us-west-2\n",
    )
    .unwrap();
    (credentials, config)
}

pub(crate) fn recorded_send(transport: &FakeSourceTransport, index: usize) -> SourceHttpRequest {
    transport.send_at(index)
}

pub(crate) fn body_of(send: &SourceHttpRequest) -> String {
    String::from_utf8(send.body.clone().unwrap_or_default()).unwrap()
}

pub(crate) fn header_of<'h>(send: &'h SourceHttpRequest, name: &str) -> Option<&'h str> {
    send.headers
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str())
}

pub(crate) fn assert_sigv4_signed_with(send: &SourceHttpRequest, access_key_id: &str) {
    let authorization = header_of(send, "Authorization")
        .expect("source SigV4 Authorization header")
        .to_owned();
    assert!(
        authorization.starts_with("AWS4-HMAC-SHA256 Credential="),
        "SigV4 credential header expected: {authorization}"
    );
    assert!(
        authorization.contains(&format!("Credential={access_key_id}/")),
        "source credentials must sign the exchange: {authorization}"
    );
}

pub(crate) struct FixtureMfa;

impl MfaTokenPresenter for FixtureMfa {
    fn token_code(
        &self,
        _serial: &str,
    ) -> Result<String, opi_ai::auth::aws::role::MfaPresentationError> {
        Ok("123456".to_owned())
    }
}

pub(crate) fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

pub(crate) struct FrozenProcessContext {
    pub(crate) cwd: std::path::PathBuf,
    pub(crate) fail: bool,
    pub(crate) snapshots: AtomicUsize,
}

pub(crate) struct FrozenSpecRunner(pub(crate) std::sync::Mutex<Vec<CredentialProcessSpec>>);

impl CredentialProcessRunner for FrozenSpecRunner {
    fn run<'a>(
        &'a self,
        spec: &'a CredentialProcessSpec,
    ) -> BoxAuthFuture<
        'a,
        Result<CredentialProcessOutput, opi_ai::auth::aws::process::CredentialProcessError>,
    > {
        Box::pin(async move {
            self.0.lock().unwrap().push(spec.clone());
            Ok(process_output("PROCESS_ACCESS", "PROCESS_SECRET"))
        })
    }
}

impl opi_ai::auth::AuthContext for FrozenProcessContext {
    fn env(&self, _name: &str) -> Result<Option<String>, opi_ai::auth::AuthContextError> {
        Ok(None)
    }

    fn file_exists(&self, _path: &str) -> Result<bool, opi_ai::auth::AuthContextError> {
        Ok(false)
    }

    fn process_snapshot(
        &self,
    ) -> Result<opi_ai::auth::AuthProcessSnapshot, opi_ai::auth::AuthContextError> {
        self.snapshots.fetch_add(1, Ordering::SeqCst);
        if self.fail {
            return Err(opi_ai::auth::AuthContextError::ProcessSnapshot {
                reason: "synthetic snapshot failure",
            });
        }
        Ok(opi_ai::auth::AuthProcessSnapshot {
            env: std::collections::BTreeMap::from([(
                "SOURCE_IDENTITY".to_owned(),
                "scoped-synthetic".to_owned(),
            )]),
            cwd: self.cwd.clone(),
        })
    }
}

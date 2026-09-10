//! Offline, exact-client output-limit admission. The listener's retained
//! numeric wire fields are independent of the requested control declaration.

use serde::Deserialize;
use serde_json::{Value, json};

use super::material::{AgentMaterial, MaterialError, PinnedFile, sha256_hex, verify_pinned};
use crate::experiment::{
    ControlValue, CredentialReference, NativeClient, ResolvedCredential, ResolvedExperiment,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Evidence {
    schema: String,
    product: String,
    executable_sha256: String,
    runtime_package_manifest_sha256: Option<String>,
    runtime_package_entrypoint: Option<std::path::PathBuf>,
    connection: Value,
    model_controls: Value,
    probe_base_url: String,
    exit_code: i32,
    config: PinnedFile,
    requests: PinnedFile,
}

fn reject(reason: impl std::fmt::Display) -> MaterialError {
    MaterialError::Rejected(format!("output-control: {reason}"))
}

/// Validate required controls before any native effect. Optional unproven
/// controls stay unsupported in pairing; an explicit execution bound requires
/// the same proof as a required model control.
pub(crate) fn admit(
    material: &super::material::NativeMaterial,
    experiment: &ResolvedExperiment,
) -> Result<(), MaterialError> {
    let required = experiment
        .model_controls()
        .required
        .iter()
        .any(|name| name == "max_output_tokens")
        || experiment.execution().max_output_tokens.is_some();
    for (product, agent) in &material.agents {
        let evidence = snapshot(agent, product, experiment)?;
        if required && evidence.is_none() {
            return Err(reject(format!(
                "{product} has no exact-client wire evidence for required max_output_tokens"
            )));
        }
    }
    Ok(())
}

/// A self-contained snapshot for sealing: the original evidence plus the
/// independently pinned config and listener log bytes. No live credential is
/// read, and no process is started by this verification.
pub(crate) fn snapshot(
    agent: &AgentMaterial,
    product: &str,
    experiment: &ResolvedExperiment,
) -> Result<Option<Vec<u8>>, MaterialError> {
    let Some(pin) = &agent.output_control_evidence else {
        return Ok(None);
    };
    verify_pinned(pin).map_err(reject)?;
    let bytes = std::fs::read(&pin.path).map_err(reject)?;
    let evidence: Evidence = serde_json::from_slice(&bytes).map_err(reject)?;
    let selected = experiment.selected_connection();
    let identity =
        json!({"provider": selected.provider, "api": selected.api, "model": selected.model});
    if evidence.schema != "opi-eval-output-control/1"
        || evidence.product != product
        || evidence.executable_sha256 != agent.executable.sha256
        || evidence.runtime_package_manifest_sha256.as_deref()
            != agent
                .runtime_package
                .as_ref()
                .map(|p| p.manifest_sha256.as_str())
        || evidence.runtime_package_entrypoint.as_ref()
            != agent.runtime_package.as_ref().map(|p| &p.entrypoint)
        || evidence.connection != identity
        || evidence.model_controls
            != serde_json::to_value(experiment.model_controls()).map_err(reject)?
        || evidence.exit_code != 0
    {
        return Err(reject(
            "client, package, connection, controls, or exit identity differs",
        ));
    }
    let ControlValue::Value(limit) = experiment.model_controls().max_output_tokens else {
        return Err(reject("wire evidence requires a numeric output control"));
    };
    if limit == 0 {
        return Err(reject("output limit must be positive"));
    }
    // The producer binds exactly an ephemeral IPv4 loopback listener. No
    // remote URL, path, query, or credentials are admitted in the probe URL.
    let address = evidence
        .probe_base_url
        .strip_prefix("http://")
        .and_then(|address| address.parse::<std::net::SocketAddr>().ok())
        .filter(|address| address.ip() == std::net::Ipv4Addr::LOCALHOST && address.port() != 0)
        .ok_or_else(|| reject("probe endpoint is not the loopback listener"))?;
    let mut connection = selected.clone();
    connection.base_url = format!("http://{address}");
    connection.credential = CredentialReference::Env {
        variable: "OPI_EVAL_SCRIPTED_KEY".to_owned(),
    };
    let client = match product {
        "opi" => NativeClient::Opi,
        "pi" => NativeClient::Pi,
        _ => return Err(reject("unknown client")),
    };
    let projection = crate::experiment::project_connection(
        &connection,
        experiment.model_controls(),
        client,
        ResolvedCredential::FromEnv {
            variable: "OPI_EVAL_SCRIPTED_KEY".to_owned(),
            value: "dummy-scripted-credential".to_owned(),
        },
    )
    .map_err(reject)?;
    verify_pinned(&evidence.config).map_err(reject)?;
    let config = std::fs::read(&evidence.config.path).map_err(reject)?;
    if config != projection.config_bytes {
        return Err(reject("config differs from the production projection"));
    }
    verify_pinned(&evidence.requests).map_err(reject)?;
    let requests = std::fs::read(&evidence.requests.path).map_err(reject)?;
    let text = std::str::from_utf8(&requests).map_err(reject)?;
    let rows = text
        .lines()
        .map(serde_json::from_str::<Value>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(reject)?;
    if rows.len() != 2 {
        return Err(reject("expected first request and tool continuation"));
    }
    let (paths, fields): (&[&str], &[&str]) = match selected.api {
        crate::experiment::ConnectionApi::AnthropicMessages => {
            (&["/v1/messages", "/v1/messages?beta=true"], &["max_tokens"])
        }
        crate::experiment::ConnectionApi::OpenAiCompletions => (
            &["/chat/completions", "/v1/chat/completions"],
            &["max_tokens", "max_completion_tokens"],
        ),
        crate::experiment::ConnectionApi::OpenAiResponses => {
            (&["/responses", "/v1/responses"], &["max_output_tokens"])
        }
    };
    for (index, row) in rows.iter().enumerate() {
        let limits = row["output_limits"]
            .as_object()
            .ok_or_else(|| reject("listener did not observe output limits"))?;
        if row["schema"] != "opi-eval-scripted-provider-log/1"
            || row["seq"].as_u64() != Some(index as u64 + 1)
            || row["status"] != 200
            || row["method"] != "POST"
            || row["model"] != selected.model
            || !row["path"]
                .as_str()
                .is_some_and(|path| paths.contains(&path))
            || !row["request_sha256"].as_str().is_some_and(|digest| {
                digest.len() == 64 && digest.bytes().all(|b| b.is_ascii_hexdigit())
            })
            || limits.len() != 1
            || !fields
                .iter()
                .any(|key| limits.get(*key).and_then(Value::as_u64) == Some(limit))
        {
            return Err(reject("observed request limit or wire identity differs"));
        }
    }
    let snapshot = json!({
        "evidence": std::str::from_utf8(&bytes).map_err(reject)?,
        "evidence_sha256": sha256_hex(&bytes),
        "config": String::from_utf8(config).map_err(reject)?,
        "requests": text,
    });
    serde_json::to_vec(&snapshot).map(Some).map_err(reject)
}

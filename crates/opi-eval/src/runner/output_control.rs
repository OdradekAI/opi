//! Offline, exact-client output-limit admission. The listener's retained
//! numeric wire fields are independent of the requested control declaration.

use serde::Deserialize;
use serde_json::{Value, json};

use super::material::{AgentMaterial, MaterialError, PinnedFile, sha256_hex, verify_pinned};
use crate::experiment::{
    Connection, ConnectionApi, ConnectionProjection, ControlValue, CredentialReference,
    ModelControls, NativeClient, ProjectedControl, ResolvedCredential, ResolvedExperiment,
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
    let output_required = experiment
        .model_controls()
        .required
        .iter()
        .any(|name| name == "max_output_tokens")
        || experiment.execution().max_output_tokens.is_some();
    let thinking_required = experiment
        .model_controls()
        .required
        .iter()
        .any(|name| name == "reasoning");
    for (product, agent) in &material.agents {
        let evidence = snapshot(agent, product, experiment)?;
        if (output_required || thinking_required) && evidence.is_none() {
            return Err(reject(format!(
                "{product} has no exact-client wire evidence for required {}",
                if thinking_required {
                    "reasoning"
                } else {
                    "max_output_tokens"
                }
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
    let projection = probe_projection(
        selected,
        experiment.model_controls(),
        product,
        &evidence.probe_base_url,
    )?;
    verify_pinned(&evidence.config).map_err(reject)?;
    let config = std::fs::read(&evidence.config.path).map_err(reject)?;
    if config != projection.config_bytes {
        return Err(reject("config differs from the production projection"));
    }
    verify_pinned(&evidence.requests).map_err(reject)?;
    let requests = std::fs::read(&evidence.requests.path).map_err(reject)?;
    let text = std::str::from_utf8(&requests).map_err(reject)?;
    verify_requests(
        text,
        selected.api,
        &selected.model,
        limit,
        &projection.controls.reasoning,
    )?;
    let snapshot = json!({
        "evidence": std::str::from_utf8(&bytes).map_err(reject)?,
        "evidence_sha256": sha256_hex(&bytes),
        "config": String::from_utf8(config).map_err(reject)?,
        "requests": text,
    });
    serde_json::to_vec(&snapshot).map(Some).map_err(reject)
}

fn probe_projection(
    selected: &Connection,
    controls: &ModelControls,
    product: &str,
    base_url: &str,
) -> Result<ConnectionProjection, MaterialError> {
    // Only the retained dummy-credential loopback projection is reconstructed.
    let address = base_url
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
    crate::experiment::project_connection(
        &connection,
        controls,
        client,
        ResolvedCredential::FromEnv {
            variable: "OPI_EVAL_SCRIPTED_KEY".to_owned(),
            value: "dummy-scripted-credential".to_owned(),
        },
    )
    .map_err(reject)
}

fn verify_requests(
    text: &str,
    api: ConnectionApi,
    model: &str,
    limit: u64,
    reasoning: &ProjectedControl<String>,
) -> Result<(), MaterialError> {
    let rows = text
        .lines()
        .map(serde_json::from_str::<Value>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(reject)?;
    if rows.len() != 2 {
        return Err(reject("expected first request and tool continuation"));
    }
    let (paths, fields): (&[&str], &[&str]) = match api {
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
            || row["model"] != model
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
        if let ProjectedControl::Effective { value } = reasoning {
            if row["tool_continuation"].as_bool() != Some(index == 1) {
                return Err(reject(
                    "expected first request followed by a tool result continuation",
                ));
            }
            verify_thinking(row, api, value)?;
        }
    }
    Ok(())
}

/// Verify retained snapshot bytes against the sealed trial's native client and
/// frozen controls. `Some` proves the output limit; its boolean records whether
/// explicit reasoning was projected and observed on both requests. No embedded
/// host path is followed and no credential is resolved.
pub(crate) fn sealed_snapshot(
    bytes: &[u8],
    experiment: &Value,
    subject: &str,
    trial: &str,
    terminal: &Value,
) -> Option<bool> {
    let snapshot: Value = serde_json::from_slice(bytes).ok()?;
    let evidence = snapshot["evidence"].as_str()?;
    let proof: Evidence = serde_json::from_str(evidence).ok()?;
    let config = snapshot["config"].as_str()?;
    let requests = snapshot["requests"].as_str()?;
    let product = experiment["subjects"]
        .as_array()?
        .iter()
        .find(|entry| entry["id"] == subject)?["product"]
        .as_str()?;
    let selected = experiment["connections"]
        .as_array()?
        .iter()
        .find(|entry| entry["id"] == experiment["connection_id"])?;
    let api = match selected["api"].as_str()? {
        "anthropic-messages" => ConnectionApi::AnthropicMessages,
        "openai-completions" => ConnectionApi::OpenAiCompletions,
        "openai-responses" => ConnectionApi::OpenAiResponses,
        _ => return None,
    };
    let connection = Connection {
        id: selected["id"].as_str()?.to_owned(),
        provider: selected["provider"].as_str()?.to_owned(),
        api,
        base_url: selected["base_url"].as_str()?.to_owned(),
        model: selected["model"].as_str()?.to_owned(),
        credential: CredentialReference::Env {
            variable: "OPI_EVAL_SCRIPTED_KEY".to_owned(),
        },
    };
    let frozen = &experiment["model_controls"];
    let controls = ModelControls {
        temperature: frozen_control(&frozen["temperature"])?,
        top_p: frozen_control(&frozen["top_p"])?,
        max_output_tokens: frozen_control(&frozen["max_output_tokens"])?,
        context_window: frozen_control(&frozen["context_window"])?,
        reasoning: frozen_control(&frozen["reasoning"])?,
        required: serde_json::from_value(frozen["required"].clone()).ok()?,
    };
    let ControlValue::Value(limit) = controls.max_output_tokens else {
        return None;
    };
    let preflight = &terminal["runtime_preflight"];
    if limit == 0
        || proof.schema != "opi-eval-output-control/1"
        || proof.product != product
        || proof.exit_code != 0
        || proof.connection
            != json!({"provider": connection.provider, "api": connection.api, "model": connection.model})
        || proof.model_controls != *frozen
        || snapshot["evidence_sha256"] != sha256_hex(evidence.as_bytes())
        || proof.config.sha256 != sha256_hex(config.as_bytes())
        || proof.requests.sha256 != sha256_hex(requests.as_bytes())
        || !matches!(
            terminal["schema"].as_str(),
            Some("opi-eval-native-agent-terminal/2" | "opi-eval-native-agent-terminal/3")
        )
        || terminal["product"] != product
        || terminal["trial"] != trial
        || terminal["agent_executable_sha256"] != proof.executable_sha256
        || preflight["agent_executable_sha256"] != proof.executable_sha256
        || preflight["runtime_package_manifest_sha256"]
            != json!(proof.runtime_package_manifest_sha256)
        || preflight["runtime_package_entrypoint"] != json!(proof.runtime_package_entrypoint)
    {
        return None;
    }
    let projection =
        probe_projection(&connection, &controls, product, &proof.probe_base_url).ok()?;
    if config.as_bytes() != projection.config_bytes {
        return None;
    }
    verify_requests(
        requests,
        api,
        &connection.model,
        limit,
        &projection.controls.reasoning,
    )
    .ok()?;
    Some(matches!(
        projection.controls.reasoning,
        ProjectedControl::Effective { .. }
    ))
}

fn frozen_control<T: serde::de::DeserializeOwned>(value: &Value) -> Option<ControlValue<T>> {
    match value.as_str() {
        Some("omitted") => Some(ControlValue::Omitted),
        Some("unknown") => Some(ControlValue::Unknown),
        _ => serde_json::from_value(value.clone())
            .ok()
            .map(ControlValue::Value),
    }
}

fn verify_thinking(
    row: &Value,
    api: crate::experiment::ConnectionApi,
    level: &str,
) -> Result<(), MaterialError> {
    let observed = row["reasoning_controls"]
        .as_object()
        .ok_or_else(|| reject("listener did not observe reasoning controls"))?;
    let budget = crate::experiment::native_thinking_budget(level)
        .ok_or_else(|| reject("unsupported reasoning level"))?;
    let matches = if level == "off" {
        observed.iter().all(|(key, value)| {
            matches!(
                (key.as_str(), value.as_str()),
                ("thinking_type", Some("disabled"))
                    | ("reasoning_effort", Some("none"))
                    | ("response_reasoning_effort", Some("none"))
            )
        })
    } else {
        match api {
            crate::experiment::ConnectionApi::AnthropicMessages => {
                observed.get("thinking_type").and_then(Value::as_str) == Some("enabled")
                    && observed.get("thinking_budget").and_then(Value::as_u64) == Some(budget)
                    && observed.len() == 2
            }
            crate::experiment::ConnectionApi::OpenAiCompletions => {
                observed.get("reasoning_effort").and_then(Value::as_str) == Some(level)
                    && observed.len() == 1
            }
            crate::experiment::ConnectionApi::OpenAiResponses => {
                observed
                    .get("response_reasoning_effort")
                    .and_then(Value::as_str)
                    == Some(level)
                    && observed.len() == 1
            }
        }
    };
    if !matches {
        return Err(reject(
            "observed reasoning control differs from requested level",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::experiment::ConnectionApi;

    fn sealed_proof(product: &str, api: ConnectionApi, level: &str) -> (Value, Value, Value) {
        let connection = Connection {
            id: "selected".to_owned(),
            provider: "fixture".to_owned(),
            api,
            base_url: "http://127.0.0.1:48127".to_owned(),
            model: "scripted".to_owned(),
            credential: CredentialReference::Env {
                variable: "NEVER_RESOLVED".to_owned(),
            },
        };
        let controls = ModelControls {
            temperature: ControlValue::Omitted,
            top_p: ControlValue::Omitted,
            max_output_tokens: ControlValue::Value(131_072),
            context_window: ControlValue::Value(1_000_000),
            reasoning: ControlValue::Value(level.to_owned()),
            required: vec![],
        };
        let projection =
            probe_projection(&connection, &controls, product, &connection.base_url).unwrap();
        let observed = if level == "off" {
            json!({})
        } else {
            match api {
                ConnectionApi::AnthropicMessages => {
                    json!({"thinking_type": "enabled", "thinking_budget": 16384})
                }
                ConnectionApi::OpenAiCompletions => json!({"reasoning_effort": level}),
                ConnectionApi::OpenAiResponses => json!({"response_reasoning_effort": level}),
            }
        };
        let (path, field) = match api {
            ConnectionApi::AnthropicMessages => ("/v1/messages", "max_tokens"),
            ConnectionApi::OpenAiCompletions => ("/v1/chat/completions", "max_tokens"),
            ConnectionApi::OpenAiResponses => ("/v1/responses", "max_output_tokens"),
        };
        let requests = (1..=2).map(|seq| {
            json!({"schema": "opi-eval-scripted-provider-log/1", "seq": seq, "status": 200, "method": "POST", "path": path, "model": connection.model, "request_sha256": "d".repeat(64), "output_limits": {field: 131_072}, "tool_continuation": seq == 2, "reasoning_controls": observed}).to_string() + "\n"
        }).collect::<String>();
        let config = String::from_utf8(projection.config_bytes).unwrap();
        let evidence = json!({
            "schema": "opi-eval-output-control/1", "product": product,
            "executable_sha256": "a".repeat(64), "runtime_package_manifest_sha256": "b".repeat(64),
            "runtime_package_entrypoint": product,
            "connection": {"provider": connection.provider, "api": api, "model": connection.model},
            "model_controls": controls, "probe_base_url": connection.base_url, "exit_code": 0,
            "config": {"path": "/never-followed/config", "sha256": sha256_hex(config.as_bytes())},
            "requests": {"path": "/never-followed/requests", "sha256": sha256_hex(requests.as_bytes())},
        }).to_string();
        let snapshot = json!({"evidence_sha256": sha256_hex(evidence.as_bytes()), "evidence": evidence, "config": config, "requests": requests});
        let experiment = json!({"connection_id": "selected", "connections": [connection], "model_controls": controls, "subjects": [{"id": "subject", "product": product}]});
        let terminal = json!({
            "schema": "opi-eval-native-agent-terminal/3", "product": product, "trial": "trial",
            "agent_executable_sha256": "a".repeat(64), "task_dispatched": true,
            "runtime_preflight": {"agent_executable_sha256": "a".repeat(64), "runtime_package_manifest_sha256": "b".repeat(64), "runtime_package_entrypoint": product, "status": "passed", "exit_code": 0},
        });
        (snapshot, experiment, terminal)
    }

    #[test]
    fn sealed_thinking_proof_admits_exact_clients_across_native_wires() {
        for product in ["opi", "pi"] {
            for api in [
                ConnectionApi::AnthropicMessages,
                ConnectionApi::OpenAiCompletions,
                ConnectionApi::OpenAiResponses,
            ] {
                for level in ["off", "max"] {
                    let (snapshot, experiment, terminal) = sealed_proof(product, api, level);
                    assert_eq!(
                        sealed_snapshot(
                            &serde_json::to_vec(&snapshot).unwrap(),
                            &experiment,
                            "subject",
                            "trial",
                            &terminal
                        ),
                        Some(true),
                        "{product}/{api:?}/{level}"
                    );
                }
            }
        }
    }

    #[test]
    fn sealed_thinking_proof_rejects_self_consistent_identity_and_wire_drift() {
        for case in [
            "product",
            "binary",
            "package",
            "entrypoint",
            "connection",
            "controls",
            "config",
            "missing-continuation",
            "continuation-kind",
            "downgrade",
            "missing-reasoning",
            "request-digest",
            "preflight-package",
        ] {
            let (mut snapshot, experiment, mut terminal) =
                sealed_proof("pi", ConnectionApi::OpenAiCompletions, "max");
            let mut proof: Value =
                serde_json::from_str(snapshot["evidence"].as_str().unwrap()).unwrap();
            let mut rows: Vec<Value> = snapshot["requests"]
                .as_str()
                .unwrap()
                .lines()
                .map(|row| serde_json::from_str(row).unwrap())
                .collect();
            match case {
                "product" => proof["product"] = "opi".into(),
                "binary" => proof["executable_sha256"] = "f".repeat(64).into(),
                "package" => proof["runtime_package_manifest_sha256"] = "f".repeat(64).into(),
                "entrypoint" => proof["runtime_package_entrypoint"] = "alternate/pi".into(),
                "connection" => proof["connection"]["provider"] = "other".into(),
                "controls" => proof["model_controls"]["reasoning"] = "high".into(),
                "config" => snapshot["config"] = "{}".into(),
                "missing-continuation" => {
                    rows.pop();
                }
                "continuation-kind" => rows[1]["tool_continuation"] = false.into(),
                "downgrade" => rows[1]["reasoning_controls"]["reasoning_effort"] = "high".into(),
                "missing-reasoning" => {
                    rows[1]
                        .as_object_mut()
                        .unwrap()
                        .remove("reasoning_controls");
                }
                "request-digest" => rows[0]["request_sha256"] = "missing".into(),
                "preflight-package" => {
                    terminal["runtime_preflight"]["runtime_package_manifest_sha256"] =
                        "f".repeat(64).into()
                }
                _ => unreachable!(),
            }
            // Re-pin all mutated embedded bytes: these cases distinguish
            // semantic admission from a mere snapshot hash check.
            let requests = rows
                .iter()
                .map(|row| row.to_string() + "\n")
                .collect::<String>();
            proof["requests"]["sha256"] = sha256_hex(requests.as_bytes()).into();
            proof["config"]["sha256"] =
                sha256_hex(snapshot["config"].as_str().unwrap().as_bytes()).into();
            let evidence = proof.to_string();
            snapshot["evidence_sha256"] = sha256_hex(evidence.as_bytes()).into();
            snapshot["evidence"] = evidence.into();
            snapshot["requests"] = requests.into();
            assert_eq!(
                sealed_snapshot(
                    &serde_json::to_vec(&snapshot).unwrap(),
                    &experiment,
                    "subject",
                    "trial",
                    &terminal
                ),
                None,
                "{case}"
            );
        }
    }

    #[test]
    fn extended_thinking_proof_keeps_exact_effort_and_rejects_downgrade() {
        for level in ["xhigh", "max"] {
            for (api, field) in [
                (ConnectionApi::OpenAiCompletions, "reasoning_effort"),
                (ConnectionApi::OpenAiResponses, "response_reasoning_effort"),
            ] {
                let row = json!({"reasoning_controls": {field: level}});
                assert!(verify_thinking(&row, api, level).is_ok(), "{api:?}/{level}");
                let downgraded = json!({"reasoning_controls": {field: "high"}});
                assert!(verify_thinking(&downgraded, api, level).is_err());
            }
            // Both clients use the high numeric budget for extended levels;
            // the frozen config and launch retain the selected logical level.
            let row = json!({"reasoning_controls": {
                "thinking_type": "enabled", "thinking_budget": 16384
            }});
            assert!(verify_thinking(&row, ConnectionApi::AnthropicMessages, level).is_ok());
        }
    }

    #[test]
    fn reasoning_proof_rejects_continuation_downgrade_and_missing_measurement() {
        let row = json!({"reasoning_controls":{"thinking_type":"enabled","thinking_budget":16384}});
        assert!(verify_thinking(&row, ConnectionApi::AnthropicMessages, "high").is_ok());
        assert!(verify_thinking(&row, ConnectionApi::AnthropicMessages, "low").is_err());
        assert!(verify_thinking(&json!({}), ConnectionApi::AnthropicMessages, "off").is_err());
        assert!(
            verify_thinking(
                &json!({"reasoning_controls": {}}),
                ConnectionApi::AnthropicMessages,
                "high"
            )
            .is_err()
        );
        for api in [
            ConnectionApi::AnthropicMessages,
            ConnectionApi::OpenAiCompletions,
            ConnectionApi::OpenAiResponses,
        ] {
            assert!(verify_thinking(&json!({"reasoning_controls": {}}), api, "off").is_ok());
        }
        assert!(
            verify_thinking(
                &json!({"reasoning_controls":{"reasoning_effort":"medium"}}),
                ConnectionApi::OpenAiCompletions,
                "high"
            )
            .is_err()
        );
    }
}

//! Private offline probe configuration and exact-client evidence admission.

use std::path::Path;

use crate::experiment::{
    ControlValue, CredentialReference, NativeClient, ProjectedControl, ResolvedCredential,
    ResolvedExperiment,
};

#[doc(hidden)]
pub fn config(path: &Path, product: &str, base_url: &str) -> Result<serde_json::Value, String> {
    let address = base_url
        .strip_prefix("http://")
        .and_then(|value| value.parse::<std::net::SocketAddr>().ok())
        .filter(|value| value.ip() == std::net::Ipv4Addr::LOCALHOST && value.port() != 0)
        .ok_or("probe endpoint must be an IPv4 loopback listener")?;
    let experiment =
        ResolvedExperiment::resolve(&std::fs::read_to_string(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let client = match product {
        "opi" => NativeClient::Opi,
        "pi" => NativeClient::Pi,
        _ => return Err("unknown native client".to_owned()),
    };
    if !experiment
        .subjects()
        .iter()
        .any(|subject| subject.product == product)
    {
        return Err("client is not selected".to_owned());
    }
    let mut connection = experiment.selected_connection().clone();
    connection.base_url = format!("http://{address}");
    connection.credential = CredentialReference::Env {
        variable: "OPI_EVAL_SCRIPTED_KEY".to_owned(),
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
    .map_err(|e| e.to_string())?;
    let thinking_level = match &projection.controls.reasoning {
        ProjectedControl::Effective { value } => value.as_str(),
        _ if matches!(
            experiment.model_controls().reasoning,
            ControlValue::Value(_)
        ) =>
        {
            return Err("requested reasoning level cannot be projected".to_owned());
        }
        _ => "off",
    };
    Ok(serde_json::json!({
        "config_filename": projection.config_filename, "config": String::from_utf8(projection.config_bytes).map_err(|e| e.to_string())?,
        "agent_env": projection.agent_env, "thinking_level": thinking_level,
        "connection": {"provider": connection.provider, "api": connection.api, "model": connection.model},
        "model_controls": experiment.model_controls(),
    }))
}

#[doc(hidden)]
pub fn admit(config: &Path, material: &Path) -> Result<(), String> {
    let experiment =
        ResolvedExperiment::resolve(&std::fs::read_to_string(config).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    experiment
        .admit_native_controls()
        .map_err(|e| e.to_string())?;
    let products: Vec<&str> = experiment
        .subjects()
        .iter()
        .map(|subject| subject.product.as_str())
        .collect();
    let material = crate::runner::material::NativeMaterial::load_for(material, &products)
        .map_err(|e| e.to_string())?;
    crate::runner::output_control::admit(&material, &experiment).map_err(|e| e.to_string())
}

//! Private retained evidence for opt-in, no-model native diagnostics.

use super::*;

pub(super) const IMAGE: &str =
    "python@sha256:272efc3f94bec901e2bc7a5c9984b3d113c317839edca2714c6a388b9476f8fc";

pub(super) fn retain_clients(root: &Path, clients: &serde_json::Value) {
    let mut identities = serde_json::Map::new();
    for key in ["opi", "pi", "node", "pi_bundle", "experiment"] {
        if let Some(path) = clients[key].as_str() {
            identities.insert(
                key.to_owned(),
                serde_json::json!({
                    "path": path, "sha256": sha256_hex(&fs::read(path).unwrap()),
                }),
            );
        }
    }
    for key in ["opi_package", "pi_package"] {
        if let Some(path) = clients[key].as_str() {
            identities.insert(
                key.to_owned(),
                serde_json::json!({
                    "path": path, "manifest_sha256": directory_manifest_digest(Path::new(path)),
                }),
            );
        }
    }
    fs::write(
        root.join("clients.json"),
        serde_json::to_vec_pretty(clients).unwrap(),
    )
    .unwrap();
    fs::write(
        root.join("client-identities.json"),
        serde_json::to_vec_pretty(&identities).unwrap(),
    )
    .unwrap();
    eprintln!("native client evidence: {}", root.display());
}

pub(super) fn run_client(command: &mut Command, root: &Path) -> std::process::Output {
    let executable = Path::new(command.get_program());
    let argv = command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let env = command
        .get_envs()
        .map(|(key, value)| {
            (
                key.to_string_lossy().into_owned(),
                value.map(|value| value.to_string_lossy().into_owned()),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let started = std::time::Instant::now();
    let identity = sha256_hex(&fs::read(executable).unwrap());
    let output = command.output().expect("execute the exact isolated client");
    fs::write(root.join("client.stdout"), &output.stdout).unwrap();
    fs::write(root.join("client.stderr"), &output.stderr).unwrap();
    let answer = command.get_current_dir().unwrap().join("answer.txt");
    fs::write(root.join("client-execution.json"), serde_json::to_vec_pretty(&serde_json::json!({
        "executable": command.get_program(), "executable_sha256": identity,
        "argv": argv, "environment": env, "cwd": command.get_current_dir(),
        "exit_code": output.status.code(), "elapsed_seconds": started.elapsed().as_secs_f64(),
        "stdout_sha256": sha256_hex(&output.stdout), "stderr_sha256": sha256_hex(&output.stderr),
        "tool_effect": fs::read(&answer).ok().map(|bytes| serde_json::json!({"path": answer, "sha256": sha256_hex(&bytes)})),
    })).unwrap()).unwrap();
    output
}

pub(super) struct Neighbor {
    id: String,
    image: Vec<u8>,
}

impl Neighbor {
    pub(super) fn start(root: &Path) -> Self {
        let image = Command::new("docker")
            .args(["image", "inspect", "--format", "{{.Id}}", IMAGE])
            .output()
            .unwrap();
        assert!(
            image.status.success(),
            "the admitted image must already be available"
        );
        let neighbor = Command::new("docker")
            .args([
                "run",
                "--detach",
                "--pull",
                "never",
                "--network",
                "none",
                "--label",
            ])
            .arg(format!("opi-eval-diagnostic-neighbor={}", root.display()))
            .args([IMAGE, "sleep", "3600"])
            .output()
            .unwrap();
        assert!(
            neighbor.status.success(),
            "{}",
            String::from_utf8_lossy(&neighbor.stderr)
        );
        let id = String::from_utf8(neighbor.stdout)
            .unwrap()
            .trim()
            .to_owned();
        assert!(id.len() == 64 && id.bytes().all(|byte| byte.is_ascii_hexdigit()));
        fs::write(root.join("neighbor.json"), serde_json::to_vec_pretty(&serde_json::json!({
            "container": id, "image": IMAGE, "image_id": String::from_utf8_lossy(&image.stdout).trim(),
        })).unwrap()).unwrap();
        Self {
            id,
            image: image.stdout,
        }
    }

    pub(super) fn assert_survives(&self, root: &Path) {
        let alive = Command::new("docker")
            .args(["inspect", "--format", "{{.State.Running}}", &self.id])
            .output()
            .unwrap();
        assert!(alive.status.success());
        assert_eq!(String::from_utf8_lossy(&alive.stdout).trim(), "true");
        let image = Command::new("docker")
            .args(["image", "inspect", "--format", "{{.Id}}", IMAGE])
            .output()
            .unwrap();
        assert!(image.status.success());
        assert_eq!(image.stdout, self.image);
        fs::write(
            root.join("neighbor-survived.json"),
            serde_json::to_vec(&serde_json::json!({
                "container": self.id, "running": true, "shared_image_unchanged": true,
            }))
            .unwrap(),
        )
        .unwrap();
    }
}

impl Drop for Neighbor {
    fn drop(&mut self) {
        let _ = Command::new("docker")
            .args(["rm", "--force", &self.id])
            .output();
    }
}

//! Private retained evidence for opt-in, no-model native diagnostics.

use super::*;

pub(super) const IMAGE: &str =
    "python@sha256:272efc3f94bec901e2bc7a5c9984b3d113c317839edca2714c6a388b9476f8fc";

/// Validates all required families before retained material or subprocesses
/// are created. Each family uses its first task's profile as the template.
pub(super) fn witness_material_profiles(
    material: &serde_json::Value,
) -> Result<Vec<(&'static str, PathBuf)>, String> {
    let mut profiles = Vec::new();
    for family in ["terminal-bench-2.1", "terminal-bench-3.0", "deepswe-v1.1"] {
        let entry = material
            .get("benchmarks")
            .and_then(|b| b.get(family))
            .ok_or_else(|| format!("benchmarks.{family}: required family is missing"))?;
        let tasks = entry
            .get("tasks")
            .and_then(|t| t.as_object())
            .filter(|tasks| !tasks.is_empty())
            .ok_or_else(|| format!("benchmarks.{family}.tasks: expected a nonempty task map"))?;
        let (task_id, task) = tasks.iter().next().expect("validated nonempty tasks");
        let field = format!("benchmarks.{family}.tasks.{task_id}.profile.path");
        let path = task
            .get("profile")
            .and_then(|p| p.get("path"))
            .and_then(|p| p.as_str())
            .filter(|p| !p.is_empty())
            .ok_or_else(|| format!("{field}: expected a nonempty string"))?;
        let path = PathBuf::from(path);
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| format!("{field}: cannot inspect profile file: {error}"))?;
        if !metadata.file_type().is_file() {
            return Err(format!("{field}: expected a regular profile file"));
        }
        fs::File::open(&path)
            .map_err(|error| format!("{field}: cannot read profile file: {error}"))?;
        profiles.push((family, path));
    }
    Ok(profiles)
}

/// Builds private tasks and diagnostic clients around the supplied real runners.
/// These materials exercise lifecycle mechanics, never benchmark Agent scores.
pub(super) fn controlled_material(input: &Path, deadlines: bool) -> PathBuf {
    let original = fs::read(input).unwrap();
    let mut material: serde_json::Value = serde_json::from_slice(&original).unwrap();
    let profiles = witness_material_profiles(&material)
        .unwrap_or_else(|reason| panic!("controlled material: {reason}"));
    let root = tempfile::Builder::new()
        .prefix(if deadlines {
            "opi-eval-delays-"
        } else {
            "opi-eval-regrades-"
        })
        .tempdir()
        .unwrap()
        .keep();
    fs::write(root.join("source-material.json"), &original).unwrap();
    let mut lock: serde_json::Value = serde_json::from_slice(
        &fs::read(material["static_lock"]["path"].as_str().unwrap()).unwrap(),
    )
    .unwrap();
    let git = root.join("git-runtime");
    fs::create_dir(&git).unwrap();
    fs::copy("/usr/bin/git", git.join("git")).unwrap();
    let libraries = Command::new("ldd").arg("/usr/bin/git").output().unwrap();
    assert!(libraries.status.success());
    fs::write(root.join("git-ldd.txt"), &libraries.stdout).unwrap();
    for path in String::from_utf8(libraries.stdout)
        .unwrap()
        .split_whitespace()
        .filter(|word| word.starts_with('/'))
    {
        let path = Path::new(path);
        fs::copy(path, git.join(path.file_name().unwrap())).unwrap();
    }
    fs::copy("/usr/share/doc/git/copyright", git.join("copyright")).unwrap();
    let seed = root.join("git-seed");
    fs::create_dir(&seed).unwrap();
    fs::write(seed.join("answer.txt"), "incorrect\n").unwrap();
    fixture_git(&seed, &["init", "--quiet"]);
    fixture_git(&seed, &["add", "answer.txt"]);
    fixture_git(
        &seed,
        &[
            "-c",
            "user.name=Diagnostic",
            "-c",
            "user.email=diagnostic@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--quiet",
            "-m",
            "diagnostic base",
        ],
    );
    let base = fixture_git(&seed, &["rev-parse", "HEAD"]);
    let archive = root.join("seed.tar");
    assert!(
        Command::new("tar")
            .arg("-cf")
            .arg(&archive)
            .arg("-C")
            .arg(&seed)
            .arg(".")
            .status()
            .unwrap()
            .success()
    );
    let cases: &[&str] = if deadlines {
        &[
            "positive",
            "time-cap",
            "agent-cap",
            "verifier-cap",
            "both-agent",
            "both-verifier",
            "minimum-cap",
            "strict-task",
            "unbounded-task",
        ]
    } else {
        &["positive", "negative", "timeout"]
    };
    for (family, template) in profiles {
        let pier = family == "deepswe-v1.1";
        let separate = family != "terminal-bench-2.1";
        let project = root.join("corpora").join(family);
        for case in cases {
            let task = project.join("tasks").join(case);
            fs::create_dir_all(task.join("environment")).unwrap();
            fs::create_dir_all(task.join("tests")).unwrap();
            let agent_delay = if matches!(
                *case,
                "time-cap"
                    | "agent-cap"
                    | "both-agent"
                    | "minimum-cap"
                    | "strict-task"
                    | "unbounded-task"
            ) {
                6.0
            } else {
                0.05
            };
            let verifier_delay = if matches!(*case, "verifier-cap" | "both-verifier" | "timeout") {
                if pier { 60.0 } else { 6.0 }
            } else {
                0.05
            };
            let answer = if *case == "negative" {
                "incorrect"
            } else {
                "correct"
            };
            fs::write(
                task.join("instruction.md"),
                serde_json::to_vec(&serde_json::json!({
                    "diagnostic": true, "family": family, "delay": agent_delay, "answer": answer,
                }))
                .unwrap(),
            )
            .unwrap();
            let artifacts = if pier {
                "/logs/artifacts/model.patch"
            } else if separate {
                "/app/evalbench/"
            } else {
                "/work/"
            };
            let timeout = if *case == "unbounded-task" {
                String::new()
            } else {
                format!(
                    "timeout_sec = {}\n",
                    if *case == "strict-task" { 0.5 } else { 10.0 }
                )
            };
            let collect = if pier {
                format!(
                    "\n[[verifier.collect]]\ncommand = \"cd /app && mkdir -p /logs/artifacts && git diff --binary {base} HEAD > /logs/artifacts/model.patch\"\ntimeout_sec = 10\n"
                )
            } else {
                String::new()
            };
            // Fresh verifier-only execution also requires an offline baseline,
            // in addition to the Agent/verifier phase policies.
            let regrade_network = if deadlines {
                ""
            } else if pier {
                "allow_internet = false\n"
            } else {
                "network_mode = \"no-network\"\n"
            };
            fs::write(task.join("task.toml"), format!(
                "schema_version = \"{}\"\nartifacts = [\"{artifacts}\"]\n\
                 [task]\nname = \"diagnostic/{family}-{case}\"\n\
                 [metadata]\nbase_commit_hash = \"{base}\"\n\
                 [agent]\nnetwork_mode = \"no-network\"\n{timeout}\
                 [environment]\n{regrade_network}build_timeout_sec = 60\ncpus = 1\nmemory_mb = 512\nstorage_mb = 128\ngpus = 0\n\
                 [verifier]\nnetwork_mode = \"no-network\"\ntimeout_sec = 90\n{}{collect}",
                if pier { "1.3" } else { "1.0" },
                if separate { "environment_mode = \"separate\"\n" } else { "" },
            )).unwrap();
            let mut dockerfile = format!(
                "FROM {IMAGE}\nWORKDIR {}\n",
                if pier {
                    "/app"
                } else if separate {
                    "/app/evalbench"
                } else {
                    "/work"
                }
            );
            if !pier {
                // A timed-out diagnostic leaves an incorrect, verifiable artifact.
                dockerfile.push_str("RUN printf 'incorrect\\n' > answer.txt\n");
            }
            if pier {
                dockerfile.push_str("COPY git/ /opt/diagnostic-git/\nCOPY git-command /usr/local/bin/git\nCOPY seed.tar /seed.tar\nRUN python3 -c \"import tarfile; tarfile.open('/seed.tar').extractall('/app', filter='data')\"\n");
                for directory in ["environment", "tests"] {
                    let context = task.join(directory);
                    copy_dir(&git, &context.join("git"));
                    fs::copy(&archive, context.join("seed.tar")).unwrap();
                    write_exec(
                        &context.join("git-command"),
                        "#!/bin/sh\nexec /opt/diagnostic-git/ld-linux-x86-64.so.2 --library-path /opt/diagnostic-git /opt/diagnostic-git/git -c user.name=Diagnostic -c user.email=diagnostic@example.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null -c gc.auto=0 \"$@\"\n",
                    );
                }
            }
            fs::write(task.join("environment/Dockerfile"), &dockerfile).unwrap();
            fs::write(
                task.join("tests/Dockerfile"),
                format!("{dockerfile}COPY test.sh /tests/test.sh\n"),
            )
            .unwrap();
            if !deadlines {
                // Preserve the runner's named build image for offline regrade.
                for directory in ["environment", "tests"] {
                    // Pier's filtered Agent egress excludes compose tasks.
                    if pier && directory == "environment" {
                        continue;
                    }
                    fs::write(
                        task.join(directory).join("docker-compose.yaml"),
                        "services:\n  main:\n    image: ${MAIN_IMAGE_NAME}\n",
                    )
                    .unwrap();
                }
            }
            let check = if pier {
                format!(
                    "assert subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd='/app', text=True).strip() == {base:?}\npatch = Path('/logs/artifacts/model.patch')\nif patch.read_bytes():\n    subprocess.run(['git', 'apply', '--check', str(patch)], cwd='/app', check=True)\n    subprocess.run(['git', 'apply', str(patch)], cwd='/app', check=True)\nanswer = Path('/app/answer.txt')\n"
                )
            } else {
                format!(
                    "answer = Path({:?})\n",
                    if separate {
                        "/app/evalbench/answer.txt"
                    } else {
                        "/work/answer.txt"
                    }
                )
            };
            write_exec(
                &task.join("tests/test.sh"),
                &format!(
                    "#!/usr/bin/env python3\nimport json, subprocess, time\nfrom pathlib import Path\n\
                 logs = Path('/logs/verifier')\nlogs.mkdir(parents=True, exist_ok=True)\n\
                 (logs/'stage-start').write_text('started')\n(logs/'verifier-start').write_text('started')\n\
                 start = time.time_ns() // 1000000\ntime.sleep({verifier_delay})\n{check}\
                 passed = answer.is_file() and answer.read_text().strip() == 'correct'\n\
                 (logs/'reward.txt').write_text(str(int(passed)))\n\
                 report = {{'results': {{'tool': {{'name': 'diagnostic-python'}}, 'summary': {{'tests': 1, 'passed': int(passed), 'failed': int(not passed), 'skipped': 0, 'pending': 0, 'other': 0, 'start': start, 'stop': time.time_ns() // 1000000}}, 'tests': [{{'name': 'declared-artifact-content', 'status': 'passed' if passed else 'failed', 'duration': 0}}]}}}}\n\
                 (logs/'ctrf.json').write_text(json.dumps(report))\n\
                 (logs/'stage-end').write_text('completed')\n(logs/'verifier-complete').write_text('completed')\n"
                ),
            );
        }
        let (corpus, mut pin) = fixture_corpus_pin(&project, family);
        pin["repository_url"] = project.to_string_lossy().into_owned().into();
        pin["kind"] = "benchmark-source".into();
        if pier {
            pin["version_anchor"] = corpus["source_commit"].clone();
        } else if separate {
            fixture_git(&project, &["tag", "v0-diagnostic"]);
            pin["tag"] = "v0-diagnostic".into();
        }
        let subject = lock["subjects"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|subject| subject["id"] == family)
            .unwrap();
        *subject = pin;
        let mut tasks = serde_json::Map::new();
        for case in cases {
            let profile = root
                .join("profiles")
                .join(family)
                .join(format!("{case}.toml"));
            fs::create_dir_all(profile.parent().unwrap()).unwrap();
            let mut doc: toml::Value = fs::read_to_string(&template).unwrap().parse().unwrap();
            doc["identity"]["upstream"] = project.to_string_lossy().into_owned().into();
            if pier {
                doc["identity"]["version_anchor"] =
                    corpus["source_commit"].as_str().unwrap().into();
            } else if separate {
                doc["identity"]["tag"] = "v0-diagnostic".into();
            }
            doc["limits"]["timeout_secs"] = if pier { 240 } else { 120 }.into();
            fs::write(&profile, toml::to_string(&doc).unwrap()).unwrap();
            let package = root.join("packages").join(family).join(case);
            copy_dir(&project.join("tasks").join(case), &package);
            tasks.insert(
                (*case).to_owned(),
                fixture_task_binding(&project, case, &package, &profile),
            );
        }
        material["benchmarks"][family]["corpus"] = corpus;
        material["benchmarks"][family]["tasks"] = tasks.into();
    }
    for product in ["opi", "pi"] {
        let package = root.join("agents").join(product);
        let executable = package.join(product);
        write_exec(&executable, &diagnostic_client_script(product));
        material["agents"][product]["executable"] = serde_json::json!({"path": executable, "sha256": sha256_hex(&fs::read(&executable).unwrap())});
        material["agents"][product]["runtime_package"] = serde_json::json!({"path": package, "entrypoint": product, "manifest_sha256": directory_manifest_digest(&package)});
        material["agents"][product]
            .as_object_mut()
            .unwrap()
            .remove("output_control_evidence");
    }
    let lock_path = root.join("static-lock.json");
    fs::write(&lock_path, serde_json::to_vec_pretty(&lock).unwrap()).unwrap();
    material["static_lock"] = serde_json::json!({"path": lock_path, "sha256": sha256_hex(&fs::read(&lock_path).unwrap())});
    material["provider"]["request_log"] = root
        .join("requests.jsonl")
        .to_string_lossy()
        .into_owned()
        .into();
    let path = root.join("material.json");
    fs::write(&path, serde_json::to_vec_pretty(&material).unwrap()).unwrap();
    assert_eq!(fs::read(input).unwrap(), original);
    eprintln!("native controlled material: {}", path.display());
    path
}

pub(super) fn diagnostic_client_script(product: &str) -> String {
    r#"#!/usr/bin/env python3
import json, subprocess, sys, time
from pathlib import Path
if sys.argv[1:] == ['--version']:
    print('$PRODUCT diagnostic client version 1')
    sys.exit(0)
case = json.loads(sys.argv[-1])
assert case['diagnostic'] is True
# The diagnostic client writes no stderr: the capture grammar admits
# only the validated diagnostic vocabulary (P19-AUD-006), and this
# stand-in qualifies regrade mechanics, not stderr privacy.
time.sleep(case['delay'])
if case['family'] == 'deepswe-v1.1':
    Path('/app/answer.txt').write_text(case['answer'] + '\n')
    subprocess.run(['git', 'add', 'answer.txt'], cwd='/app', check=True)
    subprocess.run(['git', 'commit', '--quiet', '--allow-empty', '-m', 'diagnostic answer'], cwd='/app', check=True, stdout=subprocess.DEVNULL)
else:
    directory = Path('/app/evalbench' if case['family'] == 'terminal-bench-3.0' else '/work')
    directory.mkdir(parents=True, exist_ok=True)
    (directory/'answer.txt').write_text(case['answer'] + '\n')
    (directory/'completed').write_text('completed')
if '$PRODUCT' == 'pi':
    message = {'id': 'diagnostic', 'role': 'assistant', 'content': [{'type': 'text', 'text': 'diagnostic task completed'}], 'api': 'openai-completions', 'provider': 'scripted', 'model': 'eval', 'usage': {'input': 0, 'output': 0, 'cacheRead': 0, 'cacheWrite': 0, 'totalTokens': 0, 'cost': {}}, 'stopReason': 'stop', 'timestamp': int(time.time()*1000)}
    for event in [{'type': 'session', 'version': 3, 'id': 'diagnostic', 'timestamp': '2026-01-01T00:00:00Z', 'cwd': str(Path.cwd())}, {'type': 'message_end', 'message': message}, {'type': 'agent_end', 'messages': [message]}]:
        print(json.dumps(event))
else:
    # The Opi stand-in emits the recognized Agent event shape (as the
    # artifact witnesses do); plain text is not an admissible capture
    # stream.
    print('{"type":"Agent","event":{"type":"AgentEnd","messages":[]}}')
"#.replace("$PRODUCT", product)
}

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

pub(super) fn run_cli(command: &mut Command, root: &Path, label: &str) -> std::process::Output {
    let executable_sha256 = sha256_hex(&fs::read(command.get_program()).unwrap());
    let started = std::time::Instant::now();
    let output = command.output().unwrap();
    assert_eq!(
        sha256_hex(&fs::read(command.get_program()).unwrap()),
        executable_sha256,
        "Companion executable changed during the command"
    );
    fs::write(root.join(format!("{label}.stdout")), &output.stdout).unwrap();
    fs::write(root.join(format!("{label}.stderr")), &output.stderr).unwrap();
    let args = command
        .get_args()
        .map(|arg| arg.to_string_lossy())
        .collect::<Vec<_>>();
    let environment = command
        .get_envs()
        .map(|(key, value)| {
            (
                key.to_string_lossy(),
                value.map(|value| value.to_string_lossy()),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    fs::write(root.join(format!("{label}.json")), serde_json::to_vec_pretty(&serde_json::json!({
        "executable": command.get_program(), "executable_sha256": executable_sha256,
        "argv": args, "explicit_environment": environment, "cwd": command.get_current_dir(),
        "exit_code": output.status.code(), "elapsed_seconds": started.elapsed().as_secs_f64(),
        "stdout_sha256": sha256_hex(&output.stdout), "stderr_sha256": sha256_hex(&output.stderr),
    })).unwrap()).unwrap();
    output
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

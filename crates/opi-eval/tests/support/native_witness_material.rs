//! Private retained evidence for opt-in, no-model native diagnostics.

use super::*;

pub(super) const PHASE_FAMILIES: &[&str] = &["terminal-bench-2.1", "terminal-bench-3.0"];
pub(super) const ALL_FAMILIES: &[&str] =
    &["terminal-bench-2.1", "terminal-bench-3.0", "deepswe-v1.1"];

pub(super) const IMAGE: &str =
    "python@sha256:272efc3f94bec901e2bc7a5c9984b3d113c317839edca2714c6a388b9476f8fc";

/// Validates all required families before retained material or subprocesses
/// are created. Each family uses its first task's profile as the template.
pub(super) fn witness_material_profiles(
    material: &serde_json::Value,
) -> Result<Vec<(&'static str, PathBuf)>, String> {
    witness_material_profiles_selected(material, ALL_FAMILIES)
}

/// Admits exactly the requested private witness families before any effects.
pub(super) fn witness_material_profiles_selected(
    material: &serde_json::Value,
    families: &[&'static str],
) -> Result<Vec<(&'static str, PathBuf)>, String> {
    let unique = families
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    if families.is_empty()
        || unique.len() != families.len()
        || families.iter().any(|family| !ALL_FAMILIES.contains(family))
    {
        return Err("selected families must be nonempty, known and unique".to_owned());
    }
    let mut profiles = Vec::new();
    for &family in families {
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
    controlled_material_selected(input, deadlines, ALL_FAMILIES)
}

pub(super) fn controlled_material_selected(
    input: &Path,
    deadlines: bool,
    families: &[&'static str],
) -> PathBuf {
    build_controlled_material(input, deadlines, families, false)
}

/// The P19-AUD-105 deadline-settlement probe material: the TB3 diagnostic
/// family with an Agent that also owns same-group and setsid descendants,
/// an unrelated task baseline process in the environment entrypoint, and a
/// slower time-cap verifier so owned termination stays discriminating.
pub(super) fn timecap_settlement_material(input: &Path) -> PathBuf {
    build_controlled_material(input, true, &["terminal-bench-3.0"], true)
}

/// The P19-AUD-103 TB3 artifact-witness material: a private copy of the
/// official foodstuff task whose agent environment ships a known incorrect
/// but PRESENT /app/results.txt plus an unrelated canary, with the corpus,
/// profile, task-package and static-lock identities rebound through the
/// existing fixture helpers. The nop witness therefore keeps a collectable
/// declared artifact (present-incorrect, reward zero) instead of failing
/// as a missing artifact, and the oracle witness replaces it with the
/// reference solution (reward one). The pinned native runner, task.toml
/// collection contract, separate verifier, tests grader and solution bytes
/// stay unchanged, and the supplied material and its immutable cache bytes
/// are never modified.
pub(super) fn tb3_artifact_witness_material(input: &Path) -> PathBuf {
    let original = fs::read(input).unwrap();
    let mut material: serde_json::Value = serde_json::from_slice(&original).unwrap();
    material["benchmarks"]
        .as_object_mut()
        .unwrap()
        .retain(|family, _| family == "terminal-bench-3.0");
    let root = tempfile::Builder::new()
        .prefix("opi-eval-tb3-witness-")
        .tempdir()
        .unwrap()
        .keep();
    fs::write(root.join("source-material.json"), &original).unwrap();
    let mut lock: serde_json::Value = serde_json::from_slice(
        &fs::read(material["static_lock"]["path"].as_str().unwrap()).unwrap(),
    )
    .unwrap();
    let family = "terminal-bench-3.0";
    let tasks = material["benchmarks"][family]["tasks"]
        .as_object()
        .unwrap()
        .clone();
    assert_eq!(tasks.len(), 1, "the official TB3 witness selects one task");
    let (task_id, task) = tasks.iter().next().unwrap();
    let project = root.join("corpora").join(family);
    let package = root.join("packages").join(family).join(task_id);
    copy_dir(
        &PathBuf::from(task["task_package"].as_str().unwrap()),
        &package,
    );
    // The incorrect-but-present declared artifact and the unrelated
    // canary live only in the copied agent environment (WORKDIR /app):
    // the nop witness never writes the declared file, so its collected
    // bytes stay these seeded bytes.
    let dockerfile_path = package.join("environment/Dockerfile");
    let mut dockerfile = fs::read_to_string(&dockerfile_path).unwrap();
    dockerfile.push_str(concat!(
        "RUN printf 'incorrect-witness-baseline\\n' > /app/results.txt \\\n",
        " && printf 'canary\\n' > /app/unrelated-canary.txt\n",
    ));
    fs::write(&dockerfile_path, dockerfile).unwrap();
    let corpus_source = project.join("tasks").join(task_id);
    copy_dir(&package, &corpus_source);
    let profile = root
        .join("profiles")
        .join(family)
        .join(format!("{task_id}.toml"));
    fs::create_dir_all(profile.parent().unwrap()).unwrap();
    fs::copy(
        PathBuf::from(task["profile"]["path"].as_str().unwrap()),
        &profile,
    )
    .unwrap();
    {
        let mut doc: toml::Value = fs::read_to_string(&profile).unwrap().parse().unwrap();
        doc["identity"]["upstream"] = project.to_string_lossy().into_owned().into();
        doc["identity"]["tag"] = "v0-diagnostic".into();
        fs::write(&profile, toml::to_string(&doc).unwrap()).unwrap();
    }
    let (corpus, mut pin) = fixture_corpus_pin(&project, family);
    pin["repository_url"] = project.to_string_lossy().into_owned().into();
    pin["kind"] = "benchmark-source".into();
    fixture_git(&project, &["tag", "v0-diagnostic"]);
    pin["tag"] = "v0-diagnostic".into();
    let subject = lock["subjects"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|subject| subject["id"] == family)
        .unwrap();
    *subject = pin;
    material["benchmarks"][family]["corpus"] = corpus;
    material["benchmarks"][family]["tasks"] = serde_json::json!({
        task_id.clone(): fixture_task_binding(&project, task_id, &package, &profile),
    });
    let lock_path = root.join("static-lock.json");
    fs::write(&lock_path, serde_json::to_vec_pretty(&lock).unwrap()).unwrap();
    material["static_lock"] = serde_json::json!({
        "path": lock_path,
        "sha256": sha256_hex(&fs::read(&lock_path).unwrap()),
    });
    let path = root.join("material.json");
    fs::write(&path, serde_json::to_vec_pretty(&material).unwrap()).unwrap();
    assert_eq!(fs::read(input).unwrap(), original);
    eprintln!("native TB3 artifact-witness material: {}", path.display());
    path
}

fn build_controlled_material(
    input: &Path,
    deadlines: bool,
    families: &[&'static str],
    timecap_probe: bool,
) -> PathBuf {
    let original = fs::read(input).unwrap();
    let mut material: serde_json::Value = serde_json::from_slice(&original).unwrap();
    let profiles = witness_material_profiles_selected(&material, families)
        .unwrap_or_else(|reason| panic!("controlled material: {reason}"));
    material["benchmarks"]
        .as_object_mut()
        .unwrap()
        .retain(|family, _| families.contains(&family.as_str()));
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
    let archive = root.join("seed.tar");
    let base = if families.contains(&"deepswe-v1.1") {
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
        base
    } else {
        String::new()
    };
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
            } else if timecap_probe && *case == "time-cap" {
                // Keep the container alive well past the Agent deadline so a
                // leaked descendant stays distinguishable from a killed one.
                6.0
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
            if timecap_probe {
                // The unrelated task baseline process: started by the
                // environment entrypoint, never owned by the Agent launch.
                // It heartbeats into the capture mount so an over-broad
                // deadline kill becomes observable on the host.
                dockerfile.push_str(concat!(
                    r#"ENTRYPOINT ["/bin/sh", "-c", ""#,
                    "mkdir -p /logs/agent 2>/dev/null; ",
                    r#"while :; do printf '%s\\n' \"$(date +%s.%N)\" "#,
                    r#">> /logs/agent/baseline-heartbeat.txt; sleep 0.2; done & "#,
                    r#"exec \"$@\"", "--"]"#,
                    "\n",
                ));
            }
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
        let script = if timecap_probe {
            timecap_probe_client_script(product)
        } else {
            diagnostic_client_script(product)
        };
        write_exec(&executable, &script);
        material["agents"][product]["executable"] = serde_json::json!({"path": executable, "sha256": sha256_hex(&fs::read(&executable).unwrap())});
        material["agents"][product]["runtime_package"] = serde_json::json!({"path": package, "entrypoint": product, "manifest_sha256": directory_manifest_digest(&package)});
        material["agents"][product]
            .as_object_mut()
            .unwrap()
            .remove("output_control_evidence");
    }
    // Green-check module derivation (P19-AUD-105): the private material
    // binds the Harbor integration module snapshot from the current
    // changed source with a recomputed manifest digest. The original
    // source material and its immutable cache bytes stay untouched.
    let harbor_module = root.join("modules").join("harbor");
    fs::create_dir_all(&harbor_module).unwrap();
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("integrations/harbor/opi_eval_harbor.py"),
        harbor_module.join("opi_eval_harbor.py"),
    )
    .unwrap();
    let harbor_manifest = directory_manifest_digest(&harbor_module);
    for product in ["opi", "pi"] {
        let integration = &mut material["agents"][product]["integrations"]["harbor"];
        *integration = serde_json::json!({
            "agent_name": integration["agent_name"].clone(),
            "import_path": integration["import_path"].clone(),
            "module_dir": harbor_module.to_string_lossy().into_owned(),
            "module_manifest_sha256": harbor_manifest,
        });
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

/// Private artifact fixtures retain the supplied actual clients and runners.
/// The listener starts only after Harbor stages an admitted Agent executable;
/// verifier-only startup therefore creates neither a listener nor Agent logs.
pub(super) fn real_client_regrade_material(input: &Path) -> PathBuf {
    let original_bytes = fs::read(input).unwrap();
    let original: serde_json::Value = serde_json::from_slice(&original_bytes).unwrap();
    let path = controlled_material_selected(input, false, PHASE_FAMILIES);
    let root = path.parent().unwrap();
    let mut material: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    material["agents"] = original["agents"].clone();
    for product in ["opi", "pi"] {
        material["agents"][product]
            .as_object_mut()
            .unwrap()
            .remove("output_control_evidence");
    }
    assert_eq!(material["capture_helper"], original["capture_helper"]);
    let provider = Path::new(material["provider"]["script"]["path"].as_str().unwrap());
    let provider_bytes = fs::read(provider).unwrap();
    assert_eq!(
        material["provider"]["script"]["sha256"],
        sha256_hex(&provider_bytes)
    );
    assert_eq!(
        provider_bytes,
        fs::read(manifest_dir().join("scripts/scripted-provider.py")).unwrap()
    );
    let provider_sha = sha256_hex(&provider_bytes);
    let bootstrap = r#"import hashlib, json, subprocess, sys, time
from pathlib import Path
command = subprocess.Popen(sys.argv[1:] or ['sleep', 'infinity'])
provider = None
deadline = time.monotonic() + 150
try:
    while command.poll() is None and time.monotonic() < deadline:
        if provider is None:
            for product in ('opi', 'pi'):
                executable = Path('/opi-eval-agent')/product/'runtime'/product
                if executable.is_file() and (executable.parent.parent/'capture-helper').is_file():
                    logs = Path('/logs/agent')
                    logs.mkdir(parents=True, exist_ok=True)
                    (logs/'provider-start.json').write_text(json.dumps({
                        'schema': 'opi-eval-real-regrade-provider-start/1',
                        'product': product, 'agent_executable_sha256': hashlib.sha256(executable.read_bytes()).hexdigest(),
                        'signal': str(executable)}))
                    with (logs/'provider-listener.log').open('wb') as output:
                        provider = subprocess.Popen([sys.executable,
                            '/opi-eval-fixture/provider-wrapper.py', '/opi-eval-fixture/scripted-provider.py',
                            '--listen', '127.0.0.1:48127', '--request-log', str(logs/'provider-requests.jsonl')],
                            stdout=output, stderr=subprocess.STDOUT)
                    break
        elif provider.poll() is not None:
            raise RuntimeError('private provider stopped before task settlement')
        time.sleep(0.01)
finally:
    for child in (provider, command):
        if child is None:
            continue
        if child.poll() is None:
            child.terminate()
        try:
            child.wait(timeout=2)
        except subprocess.TimeoutExpired:
            child.kill()
            child.wait(timeout=2)
"#;
    let observer_body = r#"import atexit, hashlib, json, os, stat, sys
from pathlib import Path
def _read(path, cap, root):
    if not path.is_relative_to(root):
        raise ValueError('outside owned job')
    for parent in (path.parent, *path.parent.parents):
        entry = parent.lstat()
        if not stat.S_ISDIR(entry.st_mode) or stat.S_ISLNK(entry.st_mode):
            raise ValueError('unsafe parent')
        if parent == root:
            break
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    try:
        entry = os.fstat(descriptor)
        if not stat.S_ISREG(entry.st_mode) or entry.st_nlink != 1 or entry.st_size > cap:
            raise ValueError('unsafe capture')
        chunks = []
        size = 0
        while size <= cap:
            part = os.read(descriptor, min(8192, cap + 1 - size))
            if not part:
                break
            chunks.append(part)
            size += len(part)
        if size > cap:
            raise ValueError('capture overflow')
        return b''.join(chunks)
    finally:
        os.close(descriptor)
def _retain():
    try:
        root = Path.cwd()
        paths = list(root.glob('jobs/*/*/agent/provider-requests.jsonl'))
        if len(paths) != 1:
            return
        path = paths[0]
        raw = _read(path, 128 * 1024, root)
        rows = raw.decode('utf-8').splitlines()
        if not 1 <= len(rows) <= 64 or not raw.endswith(b'\n'):
            return
        if any(json.loads(row).get('schema') != 'opi-eval-scripted-provider-log/1' for row in rows):
            return
        terminal = json.loads(_read(path.with_name('opi-eval-terminal.json'), 64 * 1024, root))
        relative = path.relative_to(root).parts
        observer_sha = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
        metadata = {'schema': 'opi-eval-real-regrade-provider-capture/1',
            'observer_sha256': observer_sha, 'provider_sha256': '$PROVIDER_SHA',
            'job': relative[1], 'native_trial': relative[2],
            'trial': terminal['trial'], 'product': terminal['product'],
            'bootstrap': json.loads(_read(path.with_name('provider-start.json'), 64 * 1024, root)),
            'bytes': len(raw), 'rows': len(rows), 'sha256': hashlib.sha256(raw).hexdigest()}
        output = sys.stdout.buffer
        output.write(('\nOPI-EVAL-REAL-REGRADE-PROVIDER-BEGIN ' + observer_sha + '\n').encode())
        output.write((json.dumps(metadata, sort_keys=True, separators=(',', ':')) + '\n').encode())
        output.write(raw)
        output.write(('OPI-EVAL-REAL-REGRADE-PROVIDER-END ' + observer_sha + '\n').encode())
        output.flush()
    except Exception:
        return
atexit.register(_retain)
"#
    .replace("$PROVIDER_SHA", &provider_sha);
    let observer = root.join("observer/sitecustomize.py");
    fs::create_dir_all(observer.parent().unwrap()).unwrap();
    fs::write(&observer, &observer_body).unwrap();
    let mut lock: serde_json::Value = serde_json::from_slice(
        &fs::read(material["static_lock"]["path"].as_str().unwrap()).unwrap(),
    )
    .unwrap();
    let mut fixture_identities = Vec::new();
    for (family, case, answer, work) in [
        ("terminal-bench-2.1", "positive", "correct", "/work"),
        (
            "terminal-bench-3.0",
            "negative",
            "incorrect",
            "/app/evalbench",
        ),
    ] {
        for field in ["verifier_executable", "runner_project", "verifier_env"] {
            assert_eq!(
                material["benchmarks"][family][field],
                original["benchmarks"][family][field]
            );
        }
        let project = root.join("corpora").join(family);
        let task = project.join("tasks").join(case);
        fs::write(
            task.join("instruction.md"),
            "Use bash to write the requested answer.txt artifact, then finish.\n",
        )
        .unwrap();
        let context = task.join("environment");
        let wrapper = format!(
            r#"import importlib.util, json, sys
spec = importlib.util.spec_from_file_location('scripted', sys.argv[1])
provider = importlib.util.module_from_spec(spec)
spec.loader.exec_module(provider)
provider.TOOL_ARGUMENTS = json.dumps({{'command': "printf '{answer}\\n' > {work}/answer.txt; printf 'actual-client\\n' > /tmp/real-client-uncollected-marker"}})
sys.argv = [sys.argv[1], *sys.argv[2:]]
sys.exit(provider.main())
"#
        );
        fs::write(context.join("scripted-provider.py"), &provider_bytes).unwrap();
        fs::write(context.join("provider-wrapper.py"), &wrapper).unwrap();
        fs::write(context.join("provider-bootstrap.py"), bootstrap).unwrap();
        let dockerfile = format!(
            "FROM {IMAGE}\nWORKDIR {work}\nRUN printf 'initial\\n' > answer.txt\nCOPY scripted-provider.py provider-wrapper.py provider-bootstrap.py /opi-eval-fixture/\nENTRYPOINT [\"python3\", \"/opi-eval-fixture/provider-bootstrap.py\"]\nHEALTHCHECK --interval=1s --timeout=2s --start-period=1s --retries=30 CMD [\"python3\", \"-c\", \"pass\"]\n"
        );
        fs::write(context.join("Dockerfile"), &dockerfile).unwrap();
        let verifier_dockerfile_path = task.join("tests/Dockerfile");
        let verifier_dockerfile = fs::read_to_string(&verifier_dockerfile_path).unwrap();
        let verifier_baseline = if family == "terminal-bench-3.0" {
            let old = "RUN printf 'incorrect\\n' > answer.txt\n";
            assert_eq!(verifier_dockerfile.matches(old).count(), 1);
            fs::write(
                &verifier_dockerfile_path,
                verifier_dockerfile.replace(old, "RUN printf 'correct\\n' > answer.txt\n"),
            )
            .unwrap();
            "correct\n"
        } else {
            "initial\n"
        };
        let verifier = task.join("tests/test.sh");
        let original_verifier = fs::read_to_string(&verifier).unwrap();
        let canary_sha = sha256_hex(b"synthetic-real-regrade-canary");
        let dummy_sha = sha256_hex(b"dummy-scripted-credential");
        let proof = format!(
            r#"
import errno, hashlib, os, socket
connection = socket.socket()
connection.settimeout(1)
try:
    connection_result = connection.connect_ex(('127.0.0.1', 48127))
finally:
    connection.close()
assert connection_result in (0, errno.ECONNREFUSED), connection_result
proof = {{'schema': 'opi-eval-real-regrade-grader/1',
    'answer_sha256': hashlib.sha256(answer.read_bytes()).hexdigest(),
    'undeclared_absent': not Path('/tmp/real-client-uncollected-marker').exists(),
    'agent_stage_absent': not Path('/opi-eval-agent').exists() or not any(Path('/opi-eval-agent').iterdir()),
    'provider_start_absent': not Path('/logs/agent/provider-start.json').exists(),
    'provider_connection_result': connection_result,
    'provider_connection_refused': errno.ECONNREFUSED,
    'provider_listener_absent': connection_result == errno.ECONNREFUSED,
    'credential_absent': all(hashlib.sha256(value.encode()).hexdigest() not in ['{canary_sha}', '{dummy_sha}'] for value in os.environ.values())}}
(logs/'real-client-grader.json').write_text(json.dumps(proof))
assert proof['credential_absent'] and proof['agent_stage_absent'], proof
{separate_assert}
"#,
            separate_assert = if family == "terminal-bench-3.0" {
                "assert proof['undeclared_absent'] and proof['provider_start_absent'] and proof['provider_listener_absent'], proof"
            } else {
                ""
            }
        );
        fs::write(&verifier, format!("{original_verifier}{proof}")).unwrap();
        let (corpus, pin) = fixture_corpus_pin(&project, family);
        material["benchmarks"][family]["corpus"] = corpus;
        let subject = lock["subjects"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|subject| subject["id"] == family)
            .unwrap();
        for field in ["commit", "tasks_tree"] {
            subject[field] = pin[field].clone();
        }
        if family == "terminal-bench-3.0" {
            fixture_git(&project, &["tag", "v0-real-client-regrade"]);
            subject["tag"] = "v0-real-client-regrade".into();
        }
        let existing = &material["benchmarks"][family]["tasks"][case];
        let profile = PathBuf::from(existing["profile"]["path"].as_str().unwrap());
        let package = PathBuf::from(existing["task_package"].as_str().unwrap());
        copy_dir(&task, &package);
        if family == "terminal-bench-3.0" {
            let mut doc: toml::Value = fs::read_to_string(&profile).unwrap().parse().unwrap();
            doc["identity"]["tag"] = "v0-real-client-regrade".into();
            fs::write(&profile, toml::to_string(&doc).unwrap()).unwrap();
        }
        let binding = fixture_task_binding(&project, case, &package, &profile);
        material["benchmarks"][family]["tasks"] = serde_json::json!({case: binding});
        let inherited = original["benchmarks"][family]["verifier_env"]["PYTHONPATH"]
            .as_str()
            .filter(|path| !path.is_empty());
        let pythonpath = inherited.map_or_else(
            || observer.parent().unwrap().display().to_string(),
            |path| format!("{}:{path}", observer.parent().unwrap().display()),
        );
        material["benchmarks"][family]["verifier_env"]["PYTHONPATH"] = pythonpath.into();
        fixture_identities.push(serde_json::json!({"family": family, "task": case, "expected_answer": format!("{answer}\n"), "source_environment_baseline": "initial\n", "verifier_baseline": verifier_baseline, "verifier_dockerfile_sha256": sha256_hex(&fs::read(&verifier_dockerfile_path).unwrap()), "wrapper_sha256": sha256_hex(wrapper.as_bytes()), "environment_sha256": sha256_hex(dockerfile.as_bytes()), "binding": material["benchmarks"][family]["tasks"][case]}));
    }
    let lock_path = PathBuf::from(material["static_lock"]["path"].as_str().unwrap());
    fs::write(&lock_path, serde_json::to_vec_pretty(&lock).unwrap()).unwrap();
    material["static_lock"]["sha256"] = sha256_hex(&fs::read(&lock_path).unwrap()).into();
    fs::write(&path, serde_json::to_vec_pretty(&material).unwrap()).unwrap();
    fs::write(root.join("real-client-fixture-identities.json"), serde_json::to_vec_pretty(&serde_json::json!({
        "diagnostic": true, "source_material_sha256": sha256_hex(&original_bytes),
        "material_sha256": sha256_hex(&fs::read(&path).unwrap()), "provider_sha256": provider_sha,
        "bootstrap_sha256": sha256_hex(bootstrap.as_bytes()), "observer_sha256": sha256_hex(observer_body.as_bytes()),
        "image": IMAGE, "actual_agents": material["agents"], "fixtures": fixture_identities,
    })).unwrap()).unwrap();
    assert_eq!(fs::read(input).unwrap(), original_bytes);
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
# The first-sink grammar admits these native start records; flushing
# preserves the start observation when the task is subsequently cancelled.
if '$PRODUCT' == 'pi':
    print(json.dumps({'type': 'session', 'version': 3, 'id': 'diagnostic', 'timestamp': '2026-01-01T00:00:00Z', 'cwd': str(Path.cwd())}), flush=True)
    print(json.dumps({'type': 'agent_start'}), flush=True)
else:
    print(json.dumps({'type': 'Agent', 'event': {'type': 'AgentStart'}}), flush=True)
# No stderr is emitted; this stand-in qualifies lifecycle mechanics.
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
    for event in [{'type': 'message_end', 'message': message}, {'type': 'agent_end', 'messages': [message]}]:
        print(json.dumps(event))
else:
    # The Opi stand-in emits the recognized Agent event shape (as the
    # artifact witnesses do); plain text is not an admissible capture
    # stream.
    print('{"type":"Agent","event":{"type":"AgentEnd","messages":[]}}')
"#.replace("$PRODUCT", product)
}

/// The P19-AUD-105 probe client: the diagnostic client plus two owned
/// descendants - one plain child inside the Agent's own process group and
/// one setsid child in a separate session. Both redirect their streams
/// away from the helper's pipes (the first sink must still see EOF when
/// the Agent exits) and heartbeat into the mounted capture root so owned
/// termination and any leak stay observable on the host.
pub(super) fn timecap_probe_client_script(product: &str) -> String {
    let descendants = r#"# Owned descendants (P19-AUD-105): their streams leave the helper's
# pipes; heartbeats land in the mounted capture root.
for _separate in (False, True):
    _name = 'descendant-b' if _separate else 'descendant-a'
    subprocess.Popen(['sh', '-c',
        'exec >>/logs/agent/' + _name + '-heartbeat.txt 2>&1; '
        'while :; do printf "%s\\n" "$(date +%s.%N)"; sleep 0.2; done'],
        start_new_session=_separate)
"#;
    diagnostic_client_script(product).replace(
        "time.sleep(case['delay'])",
        &format!("{descendants}time.sleep(case['delay'])"),
    )
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

/// Private admission for transported actual clients. Build receipts attribute
/// clean source builds; their deleted acquisition paths are not runtime inputs.
/// The retained snapshots bind the copied packages before and after execution.
pub(super) struct AdmittedClients {
    identities: serde_json::Value,
    receipts: Vec<(String, Vec<u8>)>,
}

impl AdmittedClients {
    pub(super) fn retain(&self, root: &Path, clients: &serde_json::Value) {
        retain_clients(root, clients);
        for (product, bytes) in &self.receipts {
            fs::write(root.join(format!("build-identity-{product}.json")), bytes).unwrap();
        }
        fs::write(
            root.join("admitted-client-identities.json"),
            serde_json::to_vec_pretty(&self.identities).unwrap(),
        )
        .unwrap();
    }

    pub(super) fn recheck(&self, clients: &serde_json::Value) {
        let current =
            admit_clients(clients).expect("client identity remains admitted after execution");
        assert_eq!(
            current.identities, self.identities,
            "transported client identity changed during execution"
        );
        assert_eq!(
            current.receipts, self.receipts,
            "client build receipt changed during execution"
        );
    }
}

pub(super) fn admit_clients(clients: &serde_json::Value) -> Result<AdmittedClients, String> {
    let static_lock: serde_json::Value = serde_json::from_slice(
        &fs::read(manifest_dir().join("external-locks/static/linux-x86_64.json"))
            .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    admit_clients_with_lock(clients, &static_lock)
}

pub(super) fn admit_clients_with_lock(
    clients: &serde_json::Value,
    static_lock: &serde_json::Value,
) -> Result<AdmittedClients, String> {
    fn text<'a>(value: &'a serde_json::Value, field: &str) -> Result<&'a str, String> {
        value[field]
            .as_str()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| format!("missing {field}"))
    }
    fn equal(
        actual: &serde_json::Value,
        expected: &serde_json::Value,
        field: &str,
    ) -> Result<(), String> {
        if actual != expected {
            return Err(format!("client identity mismatch: {field}"));
        }
        Ok(())
    }
    fn bytes(path: &str) -> Result<Vec<u8>, String> {
        let path = Path::new(path);
        if !path.is_absolute()
            || !fs::symlink_metadata(path)
                .map_err(|error| error.to_string())?
                .file_type()
                .is_file()
        {
            return Err("identity input must be an absolute regular file".to_owned());
        }
        fs::read(path).map_err(|error| error.to_string())
    }
    let pins = static_lock["subjects"]
        .as_array()
        .ok_or("missing static subjects")?
        .iter()
        .filter(|pin| pin["id"] == "pi")
        .collect::<Vec<_>>();
    if pins.len() != 1 {
        return Err("expected one static Pi source pin".to_owned());
    }
    let pin = pins[0];
    let blob = |path: &str| -> Result<&serde_json::Value, String> {
        let matches = pin["blobs"]
            .as_array()
            .ok_or("missing Pi blobs")?
            .iter()
            .filter(|blob| blob["path"] == path)
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return Err(format!("expected one static Pi blob: {path}"));
        }
        Ok(&matches[0]["sha256"])
    };
    equal(&clients["opi_version"], &"0.8.2".into(), "Opi version")?;
    equal(
        &clients["opi_source_commit"],
        &"3908fba723956a6af7daf298599cbe3d30bbbc09".into(),
        "independent Opi source",
    )?;
    equal(&clients["source_commit"], &pin["commit"], "Pi source")?;
    let mut identities = serde_json::Map::new();
    let mut receipts = Vec::new();
    for product in ["opi", "pi"] {
        let executable = Path::new(text(clients, product)?);
        let package = Path::new(text(clients, &format!("{product}_package"))?);
        // Validate containment and every tree entry before recursive hashing.
        let (manifest, entrypoint) = probe_package_identity(executable, package)?;
        if entrypoint != Path::new(product) {
            return Err(format!(
                "{product} requires its admitted product entrypoint"
            ));
        }
        let package_digest_key = format!("{product}_package_manifest_sha256");
        equal(
            &clients[&package_digest_key],
            &manifest.clone().into(),
            &package_digest_key,
        )?;
        let executable_sha256 = sha256_hex(&bytes(text(clients, product)?)?);
        let reference = &clients["build_identities"][product];
        let raw = bytes(text(reference, "path")?)?;
        equal(
            &reference["sha256"],
            &sha256_hex(&raw).into(),
            "build receipt digest",
        )?;
        let receipt: serde_json::Value =
            serde_json::from_slice(&raw).map_err(|error| error.to_string())?;
        equal(
            &receipt["schema"],
            &"opi-eval-agent-identity/1".into(),
            "build receipt schema",
        )?;
        equal(&receipt["agent"], &product.into(), "build receipt product")?;
        equal(
            &receipt["checkout_dirty"],
            &false.into(),
            "clean recorded checkout",
        )?;
        equal(
            &receipt["checkout_dirty_files"],
            &0.into(),
            "zero recorded dirty files",
        )?;
        let source = if product == "opi" {
            &clients["opi_source_commit"]
        } else {
            &pin["commit"]
        };
        equal(&receipt["checkout_commit"], source, "build receipt source")?;
        if product == "opi" {
            equal(
                &receipt["executable_sha256"],
                &executable_sha256.clone().into(),
                "Opi executable",
            )?;
            equal(
                &receipt["cargo_lock_sha256"],
                &"261a3ec501be2e045dc6c9b2b9a940adc1c56305f477008eea940f26545925f3".into(),
                "Opi source Cargo.lock",
            )?;
            equal(
                &receipt["target"],
                &"x86_64-unknown-linux-musl".into(),
                "Opi target",
            )?;
            equal(&receipt["profile"], &"release".into(), "Opi profile")?;
            equal(
                &receipt["features"],
                &"workspace-default-features".into(),
                "Opi features",
            )?;
        } else {
            for (key, relative, digest_key) in [
                ("node", "node", "node_sha256"),
                ("pi_bundle", "dist/bundle/cli.js", "bundle_sha256"),
            ] {
                let transported = Path::new(text(clients, key)?);
                probe_package_identity(transported, package)?;
                let actual = sha256_hex(&bytes(text(clients, key)?)?);
                equal(&receipt[digest_key], &actual.clone().into(), key)?;
                equal(
                    &sha256_hex(&bytes(
                        package
                            .join(relative)
                            .to_str()
                            .ok_or("non-UTF8 copied path")?,
                    )?)
                    .into(),
                    &actual.clone().into(),
                    "copied Pi runtime bytes",
                )?;
                identities.insert(
                    key.to_owned(),
                    serde_json::json!({"path": transported, "sha256": actual}),
                );
            }
            equal(
                &receipt["package_lock_sha256"],
                blob("package-lock.json")?,
                "Pi package-lock",
            )?;
            // This receipt field describes Pi's root manifest, while the
            // transported package.json is the separately pinned coding Agent.
            equal(
                &receipt["package_json_sha256"],
                &"1b862e9592e29d12112b99023dfb5124f4e9c26f8b4d9c3129a311e9ca11542d".into(),
                "Pi root manifest",
            )?;
            let package_json = bytes(
                package
                    .join("package.json")
                    .to_str()
                    .ok_or("non-UTF8 Pi manifest path")?,
            )?;
            equal(
                &sha256_hex(&package_json).into(),
                blob("packages/coding-agent/package.json")?,
                "locked Pi coding-agent manifest",
            )?;
            let package_json: serde_json::Value =
                serde_json::from_slice(&package_json).map_err(|error| error.to_string())?;
            equal(
                &package_json["version"],
                &pin["model_data_archive"]["version"],
                "current Pi package version",
            )?;
            equal(
                &clients["pi_version"],
                &package_json["version"],
                "Pi client version",
            )?;
            let nodes = static_lock["tools"]
                .as_array()
                .ok_or("missing static tools")?
                .iter()
                .filter(|tool| tool["id"] == "node")
                .collect::<Vec<_>>();
            if nodes.len() != 1 {
                return Err("expected one static Node pin".to_owned());
            }
            equal(
                &receipt["node_version"],
                &format!("v{}", text(nodes[0], "version")?).into(),
                "Pi Node version",
            )?;
        }
        identities.insert(product.to_owned(), serde_json::json!({"executable": executable, "executable_sha256": executable_sha256, "package": package, "manifest_sha256": manifest, "entrypoint": entrypoint, "source_commit": source, "build_identity_sha256": sha256_hex(&raw)}));
        receipts.push((product.to_owned(), raw));
    }
    Ok(AdmittedClients {
        identities: identities.into(),
        receipts,
    })
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

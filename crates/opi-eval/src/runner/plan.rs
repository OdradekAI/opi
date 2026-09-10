//! `opi-eval plan`: corpus-aware selection freeze (P19-D05).
//!
//! The plan path enumerates the pinned corpus task inventory from
//! metadata (never pulling or starting an image), expands the document's
//! `[selection]` against that inventory into exact trials, checks every
//! selected task's declared resource requirements against the locally
//! observed resources (unmet or unobservable requirements reject the
//! request - the set never silently shrinks), and freezes everything
//! into one digest-bound immutable run plan. [`super::experiment`]
//! consumes the same expansion on the `run --config` path, so a plan
//! and its direct run resolve and freeze identically.

use std::path::{Path, PathBuf};

use serde_json::json;

use crate::experiment::{DeclaredTrial, ResolvedExperiment};
use crate::runner::material::NativeMaterial;

/// The one admitted run-plan schema identity.
pub(crate) const RUN_PLAN_SCHEMA: &str = "opi-eval-run-plan/1";

/// The one admitted preview schema identity.
pub(crate) const PLAN_PREVIEW_SCHEMA: &str = "opi-eval-plan-preview/1";

/// Typed plan-path rejections (no partial freeze ever persists).
#[derive(Debug, thiserror::Error)]
pub(crate) enum PlanError {
    /// The underlying document or inventory could not be read.
    #[error("{0}")]
    Io(#[from] std::io::Error),
    /// The request contradicts the pinned corpus, selection, bounds, or
    /// observed resources.
    #[error("plan rejected: {0}")]
    Rejected(String),
}

/// One enumerated corpus task: its id, its package directory, and the
/// requirements parsed from its `task.toml` metadata.
#[derive(Debug, Clone)]
pub(crate) struct CorpusTask {
    pub(crate) id: String,
    pub(crate) dir: PathBuf,
    pub(crate) requirements: crate::runner::prepare::TaskRequirements,
}

impl CorpusTask {
    /// The requirement projection recorded in plans and previews.
    fn requirements_json(&self) -> serde_json::Value {
        json!({
            "cpus": self.requirements.cpus,
            "memory_bytes": self.requirements.memory_bytes,
            "gpus": self.requirements.gpus,
        })
    }
}

/// Enumerates the task inventory of one task-package root.
///
/// A package root is either a single task directory (its `task.toml`
/// sits at the root - the layout every producer stage pins today) or a
/// corpus directory holding task subdirectories. Enumeration reads
/// metadata only; no image is pulled or started.
pub(crate) fn enumerate_corpus(task_package: &Path) -> Result<Vec<CorpusTask>, PlanError> {
    fn parse(dir: &Path) -> Result<crate::runner::prepare::TaskRequirements, PlanError> {
        crate::runner::prepare::parse_task_requirements(dir)
            .map_err(|error| PlanError::Rejected(error.to_string()))
    }

    let mut tasks = Vec::new();
    if task_package.join("task.toml").is_file() {
        let requirements = parse(task_package)?;
        tasks.push(CorpusTask {
            id: requirements.task_id.clone(),
            dir: task_package.to_path_buf(),
            requirements,
        });
    } else {
        let mut children: Vec<PathBuf> = std::fs::read_dir(task_package)?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.is_dir() && path.join("task.toml").is_file())
            .collect();
        children.sort();
        for child in children {
            let requirements = parse(&child)?;
            tasks.push(CorpusTask {
                id: requirements.task_id.clone(),
                dir: child,
                requirements,
            });
        }
    }
    if tasks.is_empty() {
        return Err(PlanError::Rejected(format!(
            "task package {} holds no task with a task.toml",
            task_package.display()
        )));
    }
    let mut seen = std::collections::BTreeSet::new();
    for task in &tasks {
        if !seen.insert(task.id.clone()) {
            return Err(PlanError::Rejected(format!(
                "duplicate corpus task id {} under {}",
                task.id,
                task_package.display()
            )));
        }
    }
    Ok(tasks)
}

/// The task-package root an experiment resolves against: the pinned
/// material package (native mode) or the fixture benchmark package.
pub(crate) fn task_package_root(
    experiment: &ResolvedExperiment,
    fixtures: Option<&Path>,
    material: Option<&NativeMaterial>,
) -> Result<PathBuf, PlanError> {
    if let Some(material) = material {
        let adapter_key = crate::runner::experiment::adapter_key_for(experiment)
            .map_err(|error| PlanError::Rejected(error.to_string()))?;
        let benchmark = material
            .benchmark(&adapter_key)
            .map_err(|error| PlanError::Rejected(error.to_string()))?;
        return Ok(benchmark.task_package.clone());
    }
    let fixtures = fixtures.ok_or_else(|| {
        PlanError::Rejected(
            "a hermetic plan requires --fixtures (the pinned fixture package root)".to_owned(),
        )
    })?;
    let benchmark = experiment.benchmark();
    let revision_dir = match (benchmark.name.as_str(), benchmark.revision.as_str()) {
        ("terminal-bench", "2.1") => "terminal-bench-2.1",
        ("terminal-bench", "3.0") => "terminal-bench-3.0",
        ("deepswe", "v1.1") => "deepswe-v1.1",
        _ => {
            return Err(PlanError::Rejected(format!(
                "unsupported benchmark {} revision {}",
                benchmark.name, benchmark.revision
            )));
        }
    };
    Ok(fixtures
        .join("benchmarks")
        .join(revision_dir)
        .join("task-package"))
}

/// The per-task status of one requirement against the observations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RequirementStatus {
    Ok,
    Unmet,
    Unknown,
}

impl RequirementStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Unmet => "unmet",
            Self::Unknown => "unknown",
        }
    }
}

/// Classifies one task's requirements against the local observations.
fn classify(task: &CorpusTask) -> (RequirementStatus, Option<String>) {
    if let Some(required) = task.requirements.cpus {
        match crate::runner::prepare::logical_cpu_count() {
            Some(observed) if observed as i64 >= required => {}
            Some(observed) => {
                return (
                    RequirementStatus::Unmet,
                    Some(format!(
                        "requires cpus={required} but the host observes {observed} logical CPUs"
                    )),
                );
            }
            None => {
                return (
                    RequirementStatus::Unknown,
                    Some("requires cpus but the CPU observation is unavailable".to_owned()),
                );
            }
        }
    }
    if let Some(required) = task.requirements.memory_bytes {
        match crate::runner::prepare::total_memory_bytes() {
            Some(observed) if observed >= required => {}
            Some(observed) => {
                return (
                    RequirementStatus::Unmet,
                    Some(format!(
                        "requires {required} memory bytes but the host observes {observed}"
                    )),
                );
            }
            None => {
                return (
                    RequirementStatus::Unknown,
                    Some("requires memory but the memory observation is unavailable".to_owned()),
                );
            }
        }
    }
    if task.requirements.gpus.is_some_and(|required| required > 0)
        && crate::runner::prepare::gpu_runtime().is_none()
    {
        return (
            RequirementStatus::Unknown,
            Some(
                "requires gpus but the GPU observation is unavailable \
                 (engine unreachable or no GPU runtime registered)"
                    .to_owned(),
            ),
        );
    }
    (RequirementStatus::Ok, None)
}

/// The frozen expansion of a selection against the enumerated corpus.
pub(crate) struct Expansion {
    pub(crate) trials: Vec<DeclaredTrial>,
    /// Per-task preflight entries for the whole inventory (the full
    /// preflight stays visible even when a feasible subset freezes).
    pub(crate) preflight: Vec<serde_json::Value>,
    pub(crate) selected_ids: Vec<String>,
}

/// A canonical task id and its exact upstream name, admitted together by
/// the selected profile's complete task-package file table.
pub(crate) struct ProfileTaskBinding {
    id: String,
    native_name: String,
    dir: PathBuf,
}

impl ProfileTaskBinding {
    fn canonicalize(&self, corpus: &mut [CorpusTask]) -> Result<(), PlanError> {
        let mut matched = 0;
        for task in corpus.iter_mut().filter(|task| task.dir == self.dir) {
            if task.requirements.task_id != self.native_name {
                return Err(PlanError::Rejected(
                    "profile-bound task metadata drift".to_owned(),
                ));
            }
            task.id.clone_from(&self.id);
            matched += 1;
        }
        let mut ids = std::collections::BTreeSet::new();
        if matched != 1 || corpus.iter().any(|task| !ids.insert(&task.id)) {
            return Err(PlanError::Rejected(
                "profile-bound task has no unique corpus identity".to_owned(),
            ));
        }
        Ok(())
    }
}

/// Expands the document's `[selection]` against the corpus inventory.
///
/// Rejections name the contradiction: unknown task id, unmet or
/// unobservable selected requirement, native profile binding, or a
/// derived trial count above the declared `max_trials`.
pub(crate) fn expand_selection(
    experiment: &ResolvedExperiment,
    corpus: &[CorpusTask],
    profile_bound_task: Option<&ProfileTaskBinding>,
) -> Result<Expansion, PlanError> {
    let mut canonical_corpus = corpus.to_vec();
    if let Some(bound) = profile_bound_task {
        bound.canonicalize(&mut canonical_corpus)?;
    }
    let corpus = &canonical_corpus;
    let selection = experiment.selection().ok_or_else(|| {
        PlanError::Rejected(
            "plan requires [selection]; explicit-trial documents run directly through \
             run --config"
                .to_owned(),
        )
    })?;
    let inventory_ids: Vec<&str> = corpus.iter().map(|task| task.id.as_str()).collect();
    let selected_ids: Vec<String> = match selection.scope {
        crate::experiment::SelectionScope::Full => {
            corpus.iter().map(|task| task.id.clone()).collect()
        }
        crate::experiment::SelectionScope::Subset => selection
            .task_ids
            .iter()
            .map(|id| match profile_bound_task {
                Some(bound) if id == &bound.native_name => bound.id.clone(),
                _ => id.clone(),
            })
            .collect(),
    };
    if selected_ids
        .iter()
        .collect::<std::collections::BTreeSet<_>>()
        .len()
        != selected_ids.len()
    {
        return Err(PlanError::Rejected(
            "selection repeats the same canonical task identity".to_owned(),
        ));
    }
    for id in &selected_ids {
        if !inventory_ids.contains(&id.as_str()) {
            return Err(PlanError::Rejected(format!(
                "selection names task {id:?} outside the pinned inventory {inventory_ids:?}"
            )));
        }
    }

    let mut preflight = Vec::new();
    let mut rejections = Vec::new();
    for task in corpus {
        let selected = selected_ids.contains(&task.id);
        let (status, reason) = classify(task);
        preflight.push(json!({
            "id": task.id,
            "requirements": task.requirements_json(),
            "status": status.as_str(),
            "reason": reason,
            "selected": selected,
        }));
        if selected && status != RequirementStatus::Ok {
            rejections.push(format!(
                "task {} {}: {}",
                task.id,
                status.as_str(),
                reason.unwrap_or_else(|| "requirement contradicts the observations".to_owned())
            ));
        }
    }
    if let Some(bound) = profile_bound_task {
        for id in &selected_ids {
            if id != &bound.id {
                let bound = &bound.id;
                rejections.push(format!(
                    "task {id:?} is not bound by the pinned native profile \
                     (the profile binds {bound:?}); per-task native profiles are not \
                     materialized yet"
                ));
            }
        }
    }

    let mut trials = Vec::new();
    let mut subjects: Vec<&crate::experiment::ResolvedSubject> =
        experiment.subjects().iter().collect();
    subjects.sort_by(|a, b| a.id.cmp(&b.id));
    let mut selected_sorted = selected_ids.clone();
    selected_sorted.sort();
    for task in &selected_sorted {
        for repetition in 1..=selection.repetitions {
            for subject in &subjects {
                trials.push(DeclaredTrial {
                    id: format!("trial-{}-{}-r{}", subject.id, task, repetition),
                    subject: subject.id.clone(),
                    task: task.clone(),
                    group: format!("{}-r{}", task, repetition),
                });
            }
        }
    }
    if let Some(max_trials) = experiment.execution().max_trials
        && trials.len() as u64 > max_trials
    {
        rejections.push(format!(
            "planned trial count {} exceeds the declared max_trials {max_trials}; \
             narrow the selection",
            trials.len()
        ));
    }
    if !rejections.is_empty() {
        return Err(PlanError::Rejected(rejections.join("; ")));
    }
    Ok(Expansion {
        trials,
        preflight,
        selected_ids,
    })
}

/// The profile-bound task id for a native material's pinned benchmark
/// profile (native execution admits exactly the task the production
/// profile binds; per-task profiles are future material work).
pub(crate) fn profile_bound_task(
    experiment: &ResolvedExperiment,
    material: &NativeMaterial,
) -> Result<ProfileTaskBinding, PlanError> {
    let adapter_key = crate::runner::experiment::adapter_key_for(experiment)
        .map_err(|error| PlanError::Rejected(error.to_string()))?;
    let benchmark = material
        .benchmark(&adapter_key)
        .map_err(|error| PlanError::Rejected(error.to_string()))?;
    let profile_bytes = std::fs::read(&benchmark.profile)?;
    let package_dir = |id: &str| {
        if benchmark.task_package.join("task.toml").is_file() {
            benchmark.task_package.clone()
        } else {
            benchmark.task_package.join(id)
        }
    };
    let (id, dir) = match adapter_key.as_str() {
        "terminal-bench-2.1" => {
            let profile =
                crate::benchmark::terminal_bench_21::Tb21Profile::parse(profile_bytes.as_slice())
                    .map_err(|error| PlanError::Rejected(error.to_string()))?;
            let dir = package_dir(&profile.task_id);
            profile
                .validate_task_package(&dir)
                .map_err(|error| PlanError::Rejected(error.to_string()))?;
            (profile.task_id, dir)
        }
        "terminal-bench-3.0" => {
            let profile =
                crate::benchmark::terminal_bench_30::Tb30Profile::parse(profile_bytes.as_slice())
                    .map_err(|error| PlanError::Rejected(error.to_string()))?;
            let dir = package_dir(&profile.task_id);
            profile
                .admit_task_package(&dir)
                .map_err(|error| PlanError::Rejected(error.to_string()))?;
            (profile.task_id, dir)
        }
        _ => {
            let profile =
                crate::benchmark::deepswe::DeepSweProfile::parse(profile_bytes.as_slice())
                    .map_err(|error| PlanError::Rejected(error.to_string()))?;
            let dir = package_dir(&profile.task_id);
            profile
                .admit_task_package(&dir)
                .map_err(|error| PlanError::Rejected(error.to_string()))?;
            (profile.task_id, dir)
        }
    };
    let native_name = crate::runner::prepare::parse_task_requirements(&dir)
        .map_err(|error| PlanError::Rejected(error.to_string()))?
        .task_id;
    Ok(ProfileTaskBinding {
        id,
        native_name,
        dir,
    })
}

/// Builds the digest-bound run plan and its preview.
///
/// The plan embeds the resolved experiment source, the material
/// identity, the selection, the execution bounds, the per-task corpus
/// entries, the preflight, and the exact expanded trials; its
/// `plan_digest` addresses the canonical bytes of everything else.
pub(crate) fn build_plan(
    config_path: &Path,
    material_path: Option<&Path>,
    fixtures: Option<&Path>,
    out: &Path,
) -> Result<serde_json::Value, PlanError> {
    let source = std::fs::read_to_string(config_path)?;
    let experiment = crate::experiment::ResolvedExperiment::resolve(&source)
        .map_err(|error| PlanError::Rejected(error.to_string()))?;

    let material = match material_path {
        Some(path) => {
            let products =
                crate::cli::run::selected_products_from(&experiment).map_err(|reason| {
                    PlanError::Rejected(format!("cannot select subject products: {reason}"))
                })?;
            Some((
                path.to_path_buf(),
                crate::runner::material::NativeMaterial::load_for(path, &products)
                    .map_err(|error| PlanError::Rejected(error.to_string()))?,
            ))
        }
        None => None,
    };
    // Execution-mode gate mirrors the run path: native material pairs
    // only with the declared native modes.
    match (experiment.execution().mode, material.is_some()) {
        (crate::experiment::ExecutionMode::Hermetic, true) => {
            return Err(PlanError::Rejected(
                "native material requires execution mode native-fixture or live".to_owned(),
            ));
        }
        (
            crate::experiment::ExecutionMode::NativeFixture
            | crate::experiment::ExecutionMode::Live,
            false,
        ) => {
            return Err(PlanError::Rejected(
                "execution mode native-fixture/live requires --native-material".to_owned(),
            ));
        }
        _ => {}
    }

    if let Some((_, material)) = &material {
        super::output_control::admit(material, &experiment)
            .map_err(|error| PlanError::Rejected(error.to_string()))?;
    }
    let root = task_package_root(&experiment, fixtures, material.as_ref().map(|(_, m)| m))?;
    let mut corpus = enumerate_corpus(&root)?;
    let bound = material
        .as_ref()
        .map(|(_, m)| profile_bound_task(&experiment, m))
        .transpose()?;
    if let Some(bound) = &bound {
        bound.canonicalize(&mut corpus)?;
    }
    let expansion = expand_selection(&experiment, &corpus, bound.as_ref())?;

    let execution = experiment.execution();
    // Declared supervision bounds combine by narrowing: the per-stage
    // timeout never widens the existing per-process wall-clock limit,
    // and the model controls stay authoritative (an output-token bound
    // is admitted only as the equal pinned control value).
    let narrowed_agent_timeout = match (
        execution.agent_timeout_seconds,
        execution.time_limit_seconds,
    ) {
        (Some(agent), Some(limit)) => Some(agent.min(limit)),
        (agent, limit) => agent.or(limit),
    };
    let bounds = json!({
        "concurrency": execution.concurrency,
        "max_trials": execution.max_trials,
        "agent_timeout_seconds": narrowed_agent_timeout,
        "verifier_timeout_seconds": execution.verifier_timeout_seconds,
        "max_output_tokens": execution.max_output_tokens,
        "time_limit_seconds": execution.time_limit_seconds,
        "max_output_bytes": execution.max_output_bytes,
    });
    let selection = experiment.selection().expect("checked in expand_selection");
    let material_json = match material {
        Some((path, _)) => {
            let digest = sha256_hex(&std::fs::read(&path)?);
            json!({"path": path, "manifest_sha256": digest})
        }
        None => serde_json::Value::Null,
    };
    let corpus_json: Vec<serde_json::Value> = corpus
        .iter()
        .map(|task| {
            json!({
                "id": task.id,
                "dir": task.dir,
                "requirements": task.requirements_json(),
            })
        })
        .collect();
    let mut plan = json!({
        "schema": RUN_PLAN_SCHEMA,
        "experiment": {
            "source": source,
            "manifest_digest": experiment.manifest_digest(),
        },
        "material": material_json,
        "fixtures": fixtures.map(Path::to_path_buf),
        "selection": {
            "scope": selection.scope.as_str(),
            "task_ids": expansion.selected_ids,
            "reason": selection.reason,
            "repetitions": selection.repetitions,
            "seeds": selection.seeds,
        },
        "bounds": bounds,
        "corpus": corpus_json,
        "preflight": expansion.preflight,
        "trials": expansion.trials,
    });
    let digest = {
        let mut without_digest = plan.clone();
        without_digest
            .as_object_mut()
            .expect("plan is an object")
            .remove("plan_digest");
        sha256_hex(
            serde_json::to_string(&without_digest)
                .map_err(|error| PlanError::Rejected(error.to_string()))?
                .as_bytes(),
        )
    };
    plan["plan_digest"] = json!(digest);
    std::fs::write(out, format!("{plan}\n"))?;

    let unmet: Vec<&str> = expansion
        .preflight
        .iter()
        .filter(|entry| entry["status"] == "unmet")
        .filter_map(|entry| entry["id"].as_str())
        .collect();
    let unknown: Vec<&str> = expansion
        .preflight
        .iter()
        .filter(|entry| entry["status"] == "unknown")
        .filter_map(|entry| entry["id"].as_str())
        .collect();
    let connection = experiment.selected_connection();
    let preview = json!({
        "schema": PLAN_PREVIEW_SCHEMA,
        "experiment": experiment.experiment_id(),
        "manifest_digest": experiment.manifest_digest(),
        "subjects": experiment
            .subjects()
            .iter()
            .map(|subject| json!({"id": subject.id, "product": subject.product}))
            .collect::<Vec<_>>(),
        "connection": {
            "id": connection.id,
            "provider": connection.provider,
            "api": connection.api.as_str(),
            "base_url": connection.base_url,
            "model": connection.model,
        },
        "corpus": expansion.preflight,
        "selection": {
            "scope": selection.scope.as_str(),
            "task_ids": expansion.selected_ids,
            "reason": selection.reason,
            "repetitions": selection.repetitions,
            "seeds": selection.seeds,
        },
        "trial_count": expansion.trials.len(),
        "concurrency": execution.concurrency,
        "bounds": bounds,
        "preflight": {
            "unmet": unmet,
            "unknown": unknown,
        },
        "plan_digest": digest,
    });
    Ok(preview)
}

/// Verifies a saved plan: its digest must address its canonical bytes.
pub(crate) fn verify_plan(plan: &serde_json::Value) -> Result<(), PlanError> {
    if plan["schema"] != RUN_PLAN_SCHEMA {
        return Err(PlanError::Rejected(format!(
            "unknown run-plan schema {:?} (expected {RUN_PLAN_SCHEMA:?})",
            plan["schema"]
        )));
    }
    let recorded = plan["plan_digest"]
        .as_str()
        .ok_or_else(|| PlanError::Rejected("run plan carries no plan_digest".to_owned()))?;
    let mut without_digest = plan.clone();
    without_digest
        .as_object_mut()
        .expect("plan is an object")
        .remove("plan_digest");
    let observed = sha256_hex(
        serde_json::to_string(&without_digest)
            .map_err(|error| PlanError::Rejected(error.to_string()))?
            .as_bytes(),
    );
    if observed != recorded {
        return Err(PlanError::Rejected(format!(
            "run-plan digest drift: recorded {recorded}, observed {observed}"
        )));
    }
    Ok(())
}

/// Loads the experiment source and material identity a plan freezes.
pub(crate) fn plan_source(plan: &serde_json::Value) -> Result<String, PlanError> {
    verify_plan(plan)?;
    plan["experiment"]["source"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| PlanError::Rejected("run plan carries no experiment source".to_owned()))
}

/// Resolves a path recorded in a plan against the plan file's directory.
pub(crate) fn resolve_plan_path(plan_path: &Path, recorded: &Path) -> PathBuf {
    if recorded.is_absolute() {
        recorded.to_path_buf()
    } else {
        plan_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(recorded)
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selection_source(adapter: &str, tasks: &[&str]) -> String {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/experiment/local-paired.toml");
        let source = std::fs::read_to_string(fixture).unwrap();
        let mut source = source.split("[[trials]]").next().unwrap().to_owned();
        if adapter == "terminal-bench-3.0" {
            source = source.replace("revision = \"2.1\"", "revision = \"3.0\"");
        } else if adapter == "deepswe-v1.1" {
            source = source
                .replace("name = \"terminal-bench\"", "name = \"deepswe\"")
                .replace("revision = \"2.1\"", "revision = \"v1.1\"");
        }
        source.push_str(&format!("\n[selection]\nscope = \"subset\"\ntask_ids = {tasks:?}\nreason = \"identity regression\"\nrepetitions = 1\n\n[execution]\nmode = \"native-fixture\"\n"));
        source
    }

    fn stage_profile(root: &Path, adapter: &str) -> (NativeMaterial, String, String) {
        fn copy(source: &Path, target: &Path) {
            std::fs::create_dir_all(target).unwrap();
            for entry in std::fs::read_dir(source).unwrap() {
                let entry = entry.unwrap();
                if entry.path().is_dir() {
                    copy(&entry.path(), &target.join(entry.file_name()));
                } else {
                    std::fs::copy(entry.path(), target.join(entry.file_name())).unwrap();
                }
            }
        }
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/benchmarks")
            .join(adapter);
        let package = root.join("package");
        copy(&fixture.join("task-package"), &package);
        let text = std::fs::read_to_string(fixture.join("profile/synthetic.toml")).unwrap();
        let mut profile: toml::Value = toml::from_str(&text).unwrap();
        let id = profile["identity"]["task_id"].as_str().unwrap().to_owned();
        let namespace = if adapter == "deepswe-v1.1" {
            "datacurve"
        } else {
            "terminal-bench"
        };
        let name = format!("{namespace}/{id}");
        std::fs::write(
            package.join("task.toml"),
            format!("[task]\nname = {name:?}\n"),
        )
        .unwrap();
        let rows = profile["package"].as_array_mut().unwrap();
        for row in rows.iter_mut() {
            let bytes = std::fs::read(package.join(row["path"].as_str().unwrap())).unwrap();
            row["sha256"] = sha256_hex(&bytes).into();
            row["size"] = (bytes.len() as i64).into();
        }
        rows.sort_by(|a, b| a["path"].as_str().cmp(&b["path"].as_str()));
        let table = serde_json::to_value(&*rows).unwrap();
        profile["identity"]["package_manifest_sha256"] =
            sha256_hex(serde_json::to_string(&table).unwrap().as_bytes()).into();
        let profile_path = root.join("profile.toml");
        std::fs::write(&profile_path, toml::to_string(&profile).unwrap()).unwrap();
        let pinned = json!({"path": "/unused", "sha256": "unused"});
        let material = serde_json::from_value(json!({
            "schema": "opi-eval-native-material/4", "static_lock": pinned,
            "provider": {"script": pinned, "endpoint": "unused", "request_log": "/unused"},
            "agents": {}, "benchmarks": {adapter: {
                "profile": profile_path, "task_package": package,
                "task_package_manifest_sha256": "unused", "verifier_executable": pinned,
                "verifier_env": {}, "oracle": pinned, "oracle_env": {}
            }}
        }))
        .unwrap();
        (material, id, name)
    }

    #[test]
    fn native_selection_uses_profile_id_for_namespaced_task_name() {
        for adapter in ["terminal-bench-2.1", "terminal-bench-3.0", "deepswe-v1.1"] {
            let root = tempfile::tempdir().unwrap();
            let (material, id, name) = stage_profile(root.path(), adapter);
            for selected in [&id, &name] {
                let experiment =
                    ResolvedExperiment::resolve(&selection_source(adapter, &[selected])).unwrap();
                let package = task_package_root(&experiment, None, Some(&material)).unwrap();
                let corpus = enumerate_corpus(&package).unwrap();
                let bound = profile_bound_task(&experiment, &material).unwrap();
                let expansion = expand_selection(&experiment, &corpus, Some(&bound)).unwrap();
                assert_eq!(expansion.selected_ids, std::slice::from_ref(&id));
                assert_eq!(expansion.trials.len(), 2);
                assert!(
                    expansion
                        .trials
                        .iter()
                        .all(|trial| trial.task == id && !trial.id.contains('/'))
                );
            }
        }
    }

    #[test]
    fn native_task_aliases_require_exact_package_binding() {
        for adapter in ["terminal-bench-2.1", "terminal-bench-3.0", "deepswe-v1.1"] {
            let root = tempfile::tempdir().unwrap();
            let (mut material, id, name) = stage_profile(root.path(), adapter);
            let corpus_root = root.path().join("corpus");
            std::fs::create_dir_all(&corpus_root).unwrap();
            std::fs::rename(root.path().join("package"), corpus_root.join(&id)).unwrap();
            material.benchmarks.get_mut(adapter).unwrap().task_package = corpus_root.clone();
            let valid = ResolvedExperiment::resolve(&selection_source(adapter, &[&name])).unwrap();
            let corpus = enumerate_corpus(&corpus_root).unwrap();
            let bound = profile_bound_task(&valid, &material).unwrap();
            assert_eq!(
                expand_selection(&valid, &corpus, Some(&bound))
                    .unwrap()
                    .selected_ids,
                std::slice::from_ref(&id)
            );
            let forged = format!("unrelated/{id}");
            for tasks in [&[forged.as_str()][..], &[id.as_str(), name.as_str()][..]] {
                let invalid =
                    ResolvedExperiment::resolve(&selection_source(adapter, tasks)).unwrap();
                assert!(expand_selection(&invalid, &corpus, Some(&bound)).is_err());
            }
            std::fs::write(
                corpus_root.join(&id).join("task.toml"),
                format!("[task]\nname = {forged:?}\n"),
            )
            .unwrap();
            assert!(
                profile_bound_task(&valid, &material).is_err(),
                "alias must be bound to exact reviewed task bytes"
            );
        }
    }
}

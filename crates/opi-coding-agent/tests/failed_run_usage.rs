//! Failed-run consumption remains observable without committing failed prompts.

mod common;

use opi_agent::session::{SessionHeader, SessionReader, SessionWriter};
use opi_ai::message::InputContent;
use opi_ai::model_info::{ModelInfo, ModelPricing, WireApi};
use opi_ai::registry::ModelCapabilities;
use opi_ai::stream::{AssistantStreamEvent, Pricing, Usage};
use opi_ai::test_support::{self, MockProvider};
use opi_coding_agent::config::OpiConfig;
use opi_coding_agent::harness::ResumeInfo;
use opi_coding_agent::policy::ToolSelection;
use opi_coding_agent::project_trust::TrustDecision;
use opi_coding_agent::runner::{ExitCode, NonInteractiveResult, NonInteractiveRunner};

fn response_usage(
    mut events: Vec<AssistantStreamEvent>,
    usage: Usage,
) -> Vec<AssistantStreamEvent> {
    for event in &mut events {
        if let AssistantStreamEvent::Done { message, .. }
        | AssistantStreamEvent::Error { message, .. } = event
        {
            message.usage = usage.clone();
        }
    }
    events
}

fn read_response() -> Vec<AssistantStreamEvent> {
    response_usage(
        test_support::tool_call_response("fixture-read", "read", r#"{"path":"input.txt"}"#),
        Usage::reported(10, 20, 30, 40, Some(5), None),
    )
}

fn final_response() -> Vec<AssistantStreamEvent> {
    response_usage(
        test_support::text_response("complete"),
        Usage::reported(2, 3, 5, 7, None, None),
    )
}

fn runner(
    workspace: &std::path::Path,
    responses: Vec<Vec<AssistantStreamEvent>>,
    max_iterations: u32,
    resume: Option<ResumeInfo>,
) -> NonInteractiveRunner {
    let model = ModelInfo::new(
        "mock-model",
        "Fixture model",
        WireApi::AnthropicMessages,
        ModelCapabilities::new(100_000, 4_096).with_streaming(true),
    )
    .with_pricing(
        ModelPricing::try_new(
            Pricing {
                input_cost_per_mtok: 1.0,
                output_cost_per_mtok: 2.0,
                cache_read_cost_per_mtok: 3.0,
                cache_write_cost_per_mtok: 4.0,
            },
            vec![],
        )
        .unwrap(),
    )
    .unwrap();
    let mut config = OpiConfig::default();
    config.defaults.max_iterations = Some(max_iterations);
    let resume = resume.or_else(|| {
        let directory = workspace.join("sessions");
        std::fs::create_dir_all(&directory).unwrap();
        let session_id = "usage-fixture".to_owned();
        let path = directory.join("usage-fixture.jsonl");
        SessionWriter::create(
            &path,
            SessionHeader::new_for_test(
                session_id.clone(),
                "2026-09-10T00:00:00Z".into(),
                workspace.display().to_string(),
                None,
            ),
        )
        .unwrap();
        Some(ResumeInfo {
            path,
            session_id,
            entries: vec![],
            original_cwd: workspace.to_path_buf(),
            diagnostics: vec![],
            recorded_model: None,
            recorded_thinking: None,
        })
    });
    NonInteractiveRunner::new_with_resume(
        Box::new(MockProvider::new_with_models(
            "mock",
            vec![model],
            responses,
        )),
        "mock:mock-model".into(),
        config,
        workspace.to_path_buf(),
        false,
        None,
        vec![],
        resume,
        ToolSelection::Allowlist(vec!["read".into()]),
        TrustDecision::Trusted,
    )
    .unwrap()
}

fn summary(result: &NonInteractiveResult) -> serde_json::Value {
    result
        .stdout
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .find(|event| event["type"] == "session_summary")
        .expect("JSON run has a summary even when it fails")
}

async fn run_json(
    runner: &mut NonInteractiveRunner,
    content: bool,
    prompt: &str,
) -> NonInteractiveResult {
    if content {
        runner
            .run_json_with_content(vec![InputContent::Text {
                text: prompt.into(),
            }])
            .await
    } else {
        runner.run_json(prompt).await
    }
}

async fn check_turn_budget(content: bool) {
    let _guard = common::empty_user_config_dir();
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("input.txt"), "fixture").unwrap();
    let mut runner = runner(
        workspace.path(),
        vec![read_response(), final_response()],
        1,
        None,
    );
    let failed = run_json(&mut runner, content, "uncommitted-failure-prompt").await;
    assert_eq!(failed.exit_code, ExitCode::RuntimeFailure as i32);
    assert!(failed.stderr.contains("max turns exceeded"));
    let first = summary(&failed);
    assert_eq!(
        first["tokens"],
        serde_json::json!({"input":10,"output":20,"cache_read":30,"cache_write":40})
    );
    assert_eq!(first["turns"], 0);
    assert_eq!(first["provider_turns"], 1);
    assert!((first["cost_usd"]["total"].as_f64().unwrap() - 0.000290).abs() < 1e-12);
    let session = runner.session().unwrap();
    assert_eq!(session.usage().turn_count(), 0);
    assert_eq!(session.usage().total_input_tokens(), 0);
    let persisted = std::fs::read_to_string(session.session_path()).unwrap();
    assert!(!persisted.contains("uncommitted-failure-prompt"));

    let succeeded = run_json(&mut runner, content, "accepted-follow-up").await;
    assert_eq!(succeeded.exit_code, ExitCode::Success as i32);
    let second = summary(&succeeded);
    assert_eq!(
        second["tokens"],
        serde_json::json!({"input":12,"output":23,"cache_read":35,"cache_write":47})
    );
    assert_eq!(second["turns"], 1);
    assert_eq!(second["provider_turns"], 1);
    assert!((second["cost_usd"]["total"].as_f64().unwrap() - 0.000341).abs() < 1e-12);
    assert_eq!(runner.session().unwrap().usage().total_input_tokens(), 2);
}

#[tokio::test]
async fn failed_turn_budget_keeps_text_json_usage() {
    check_turn_budget(false).await;
}

#[tokio::test]
async fn failed_turn_budget_keeps_content_json_usage() {
    check_turn_budget(true).await;
}

#[tokio::test]
async fn unknown_error_usage_keeps_known_consumption_and_unknown_cost() {
    let _guard = common::empty_user_config_dir();
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("input.txt"), "fixture").unwrap();
    let mut runner = runner(
        workspace.path(),
        vec![
            read_response(),
            test_support::error_response("fixture error"),
        ],
        2,
        None,
    );
    let failed = runner.run_json("read then fail").await;
    assert_eq!(failed.exit_code, ExitCode::ProviderFailure as i32);
    let summary = summary(&failed);
    assert_eq!(summary["tokens"]["input"], 10);
    assert_eq!(summary["tokens"]["output"], 20);
    assert!(summary["cost_usd"].is_null());
    assert_eq!(runner.session().unwrap().usage().turn_count(), 0);
}

#[tokio::test]
async fn resumed_usage_is_counted_once_before_a_failed_prompt() {
    let _guard = common::empty_user_config_dir();
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("input.txt"), "fixture").unwrap();
    let mut prior = runner(workspace.path(), vec![final_response()], 1, None);
    assert_eq!(
        prior.run_json("accepted history").await.exit_code,
        ExitCode::Success as i32
    );
    let path = prior.session().unwrap().session_path().to_path_buf();
    let (header, entries) = SessionReader::read_all(&path).unwrap();
    let resume = ResumeInfo {
        path,
        session_id: header.id,
        entries,
        original_cwd: workspace.path().to_path_buf(),
        diagnostics: vec![],
        recorded_model: None,
        recorded_thinking: None,
    };
    let mut resumed = runner(workspace.path(), vec![read_response()], 1, Some(resume));
    let failed = resumed.run_json("uncommitted new prompt").await;
    assert_eq!(failed.exit_code, ExitCode::RuntimeFailure as i32);
    let summary = summary(&failed);
    assert_eq!(
        summary["tokens"],
        serde_json::json!({"input":12,"output":23,"cache_read":35,"cache_write":47})
    );
    assert_eq!(summary["turns"], 1);
    assert_eq!(resumed.session().unwrap().usage().total_input_tokens(), 2);
}

//! llama.cpp tokenization, completion readout and question order.

use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use opi_ai::AuthProvenanceSource;
use opi_ai::api::ClassifierApi;
use opi_ai::api::llama_cpp_classify::{
    LLAMA_CPP_CLASSIFY_API_ID, LlamaCppClassifyApi, LlamaCppClassifyOptions, label_probabilities,
    peak_confidence, render_question,
};
use opi_ai::api::system_one::TYPESAFE_SYSTEM_ONE_API_ID;
use opi_ai::auth::{AuthResolver, AuthScheme, ResolvedAuth};
use opi_ai::classifier::{
    ClassifierAnswer, ClassifierCall, ClassifierContext, ClassifierDeclarations, ClassifierImage,
    ClassifierModelRecord, ClassifierQuestion, ClassifierStopReason,
};
use opi_ai::http::HttpClient;
use opi_ai::models::Models;
use opi_ai::provider::ProviderError;
use opi_ai::types::ApiId;
use secrecy::SecretString;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::failure_fixture::assert_syntax_failure;
use super::fixtures::{bool_question, zero_pricing};

/// One token per character: the token id is the character code.
fn llama_char_tokens(content: &str) -> Vec<u64> {
    content.chars().map(|c| c as u64).collect()
}

/// Maps multi-character labels ("Yes"/"No") to single tokens, as a real
/// vocabulary would.
fn llama_word_tokens(content: &str) -> Vec<u64> {
    let words: std::collections::HashMap<&str, u64> =
        [("Yes", 89u64), ("No", 78u64)].into_iter().collect();
    let mut tokens = Vec::new();
    for part in content.split('\n') {
        // Reinsert the newline as its own token, then the text between.
        if !tokens.is_empty() {
            tokens.push(b'\n' as u64);
        }
        if let Some(id) = words.get(part) {
            tokens.push(*id);
        } else {
            tokens.extend(llama_char_tokens(part));
        }
    }
    tokens
}

/// Completion log-probabilities by prompt: bool (Yes/No), score digits or
/// the three choice letters.
fn llama_answer_by_prompt(prompt: &str) -> Vec<(u64, f64)> {
    if prompt.contains("Answer Yes or No.") {
        vec![(b'Y' as u64, -0.05), (b'N' as u64, -3.0)]
    } else if prompt.contains("Answer with one level number.") {
        vec![
            (b'2' as u64, -0.2),
            (b'1' as u64, -1.8),
            (b'0' as u64, -4.0),
        ]
    } else {
        vec![
            (b'B' as u64, -0.3),
            (b'A' as u64, -1.5),
            (b'C' as u64, -3.0),
        ]
    }
}

type LlamaTokenize = Box<dyn Fn(&str) -> Vec<u64> + Send + Sync>;
type LlamaNext = Box<dyn Fn(&str, u64) -> Vec<(u64, f64)> + Send + Sync>;

/// A fake llama-server answering the three classify endpoints. Dynamic by
/// request body, unlike the fixed system-one wiremock templates.
pub(super) struct LlamaServer {
    tokenize: LlamaTokenize,
    next: LlamaNext,
    /// Fixed prompt overriding the default template rendering.
    template_prompt: Option<String>,
    /// When set, `/completion` answers with this HTTP status instead.
    fail_completion: Option<u16>,
}

impl LlamaServer {
    pub(super) fn new() -> Self {
        Self {
            tokenize: Box::new(llama_word_tokens),
            next: Box::new(|prompt, _depth| llama_answer_by_prompt(prompt)),
            template_prompt: None,
            fail_completion: None,
        }
    }
}

impl wiremock::Respond for LlamaServer {
    fn respond(&self, request: &wiremock::Request) -> ResponseTemplate {
        let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap_or_default();
        match request.url.path() {
            "/tokenize" => {
                let content = body["content"].as_str().unwrap_or_default();
                let tokens: Vec<u64> = (self.tokenize)(content);
                ResponseTemplate::new(200).set_body_json(serde_json::json!({ "tokens": tokens }))
            }
            "/apply-template" => {
                let prompt = match &self.template_prompt {
                    Some(fixed) => fixed.clone(),
                    None => {
                        let messages = body["messages"].as_array().cloned().unwrap_or_default();
                        let mut prompt = String::new();
                        for message in &messages {
                            prompt.push_str(&format!(
                                "<|{}|>\n{}\n",
                                message["role"].as_str().unwrap_or_default(),
                                message["content"].as_str().unwrap_or_default()
                            ));
                        }
                        prompt.push_str("<|assistant|>\n");
                        prompt
                    }
                };
                ResponseTemplate::new(200).set_body_json(serde_json::json!({ "prompt": prompt }))
            }
            "/completion" => {
                if let Some(status) = self.fail_completion {
                    return ResponseTemplate::new(status)
                        .set_body_string(r#"{"error":{"message":"context overflow"}}"#);
                }
                let prompt = body["prompt"].as_str().unwrap_or_default();
                let depth = body["n_probs"].as_u64().unwrap_or(0);
                let top_logprobs: Vec<serde_json::Value> = (self.next)(prompt, depth)
                    .into_iter()
                    .map(|(id, logprob)| {
                        serde_json::json!({
                            "id": id,
                            "token": char::from_u32(id as u32).unwrap_or_default(),
                            "bytes": [],
                            "logprob": logprob,
                        })
                    })
                    .collect();
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "content": "A",
                    "completion_probabilities": [
                        { "id": 65, "token": "A", "top_logprobs": top_logprobs }
                    ]
                }))
            }
            _ => ResponseTemplate::new(404).set_body_string("not found"),
        }
    }
}

pub(super) async fn mount_llama(server: &MockServer, llama: LlamaServer) {
    Mock::given(method("POST"))
        .respond_with(llama)
        .mount(server)
        .await;
}

pub(super) struct LlamaAuthResolver;

impl AuthResolver for LlamaAuthResolver {
    fn resolve<'a>(
        &'a self,
    ) -> opi_ai::credential::BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>> {
        Box::pin(async move {
            Ok(ResolvedAuth {
                env: Default::default(),
                scheme: AuthScheme::Bearer,
                secret: SecretString::from("local"),
                base_url: None,
                account_id: None,
                provenance: Default::default(),
            })
        })
    }
}

/// A `qwen` classifier record rooted at the mock server through the
/// OpenAI-compatible `/v1` base URL (pi's llama.cpp models carry it).
pub(super) fn llama_record(server_uri: &str) -> ClassifierModelRecord {
    ClassifierModelRecord {
        headers: Vec::new(),
        input_limits: None,
        compat: None,
        provider_id: "llama.cpp".into(),
        id: "qwen".into(),
        name: "qwen".into(),
        api: ApiId::new(LLAMA_CPP_CLASSIFY_API_ID),
        base_url: format!("{server_uri}/v1"),
        input: vec![opi_ai::classifier::ClassifierInputModality::Text],
        context_window: 32768,
        cost: Some(zero_pricing()),
    }
}

pub(super) fn llama_models(
    record: &ClassifierModelRecord,
) -> (
    Models,
    opi_ai::models::ClassifierApiHandle<LlamaCppClassifyOptions>,
) {
    let mut models = Models::new();
    let handle = models
        .register_classifier_api(
            Arc::new(LlamaCppClassifyApi::new(HttpClient::new(), record.clone())),
            Arc::new(LlamaAuthResolver),
            AuthProvenanceSource::Static,
        )
        .expect("register llama classifier api");
    (models, handle)
}

pub(super) fn llama_context() -> ClassifierContext {
    let mut questions = ClassifierDeclarations::new();
    questions.insert(
        "team".to_owned(),
        ClassifierQuestion::Choice {
            instructions: "Which team should handle this?".into(),
            criteria: [
                ("billing", "Payments and refunds"),
                ("technical", "Bugs and outages"),
                ("sales", ""),
            ]
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect(),
        },
    );
    questions.insert(
        "urgent".to_owned(),
        ClassifierQuestion::Bool {
            instructions: "Does this convey urgency?".into(),
            true_criteria: "The user needs help soon".into(),
            false_criteria: "No time pressure".into(),
        },
    );
    questions.insert(
        "severity".to_owned(),
        ClassifierQuestion::Score {
            instructions: "How severe is this?".into(),
            levels: vec!["low".into(), "medium".into(), "high".into()],
        },
    );
    ClassifierContext {
        state: serde_json::json!({ "message": "Help! My payouts have been failing for 3 days." }),
        questions,
        images: None,
    }
}

pub(super) fn llama_pick_context() -> ClassifierContext {
    let mut questions = ClassifierDeclarations::new();
    questions.insert(
        "pick".to_owned(),
        ClassifierQuestion::Choice {
            instructions: "Pick one".into(),
            criteria: [("a", ""), ("b", "")]
                .into_iter()
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
                .collect(),
        },
    );
    ClassifierContext {
        state: serde_json::json!({}),
        questions,
        images: None,
    }
}

async fn llama_requests(server: &MockServer) -> Vec<(String, serde_json::Value)> {
    server
        .received_requests()
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|request| {
            let path = request.url.path().to_owned();
            let body = serde_json::from_slice(&request.body).unwrap_or_default();
            (path, body)
        })
        .collect()
}

fn choice_context(first: &str, second: &str) -> ClassifierContext {
    ClassifierContext {
        state: serde_json::json!({}),
        images: None,
        questions: [(
            "pick".into(),
            ClassifierQuestion::Choice {
                instructions: "Pick one".into(),
                criteria: [
                    (first.into(), "first".into()),
                    (second.into(), "second".into()),
                ]
                .into_iter()
                .collect(),
            },
        )]
        .into_iter()
        .collect(),
    }
}

async fn run_choice(first: &str, second: &str) -> String {
    let server = MockServer::start().await;
    let mut llama = LlamaServer::new();
    llama.next = Box::new(|_, _| vec![(b'A' as u64, -1.0), (b'B' as u64, -1.0)]);
    mount_llama(&server, llama).await;
    let record = llama_record(&server.uri());
    let (models, _) = llama_models(&record);
    let result = models
        .classify(
            record,
            choice_context(first, second),
            ClassifierCall::default(),
        )
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Stop, "{result:?}");
    let requests = server.received_requests().await.unwrap();
    for endpoint in ["/completion", "/apply-template"] {
        assert_eq!(
            requests.iter().filter(|r| r.url.path() == endpoint).count(),
            1
        );
    }
    for request in &requests {
        assert_eq!(
            request.headers.get("authorization").unwrap(),
            "Bearer local"
        );
    }
    match &result.answers["pick"] {
        ClassifierAnswer::Choice {
            choice,
            probabilities,
            ..
        } => {
            assert!((probabilities[first] - 0.5).abs() < 1e-9);
            assert!((probabilities[second] - 0.5).abs() < 1e-9);
            choice.clone()
        }
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn llama_tied_choice_preserves_declared_first() {
    assert_eq!(run_choice("zeta", "alpha").await, "zeta");
}

#[tokio::test]
async fn llama_alphabetic_choice_tie_control() {
    assert_eq!(run_choice("alpha", "zeta").await, "alpha");
}

#[tokio::test]
async fn llama_question_wire_sequence() {
    let server = MockServer::start().await;
    mount_llama(&server, LlamaServer::new()).await;
    let record = llama_record(&server.uri());
    let (models, _) = llama_models(&record);
    let context = ClassifierContext {
        state: serde_json::json!({}),
        images: None,
        questions: [
            ("z-last-lexically".into(), bool_question("z-first")),
            ("a-first-lexically".into(), bool_question("a-second")),
        ]
        .into_iter()
        .collect(),
    };
    let result = models
        .classify(record, context, ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Stop, "{result:?}");
    assert_eq!(result.answers.len(), 2);
    let requests = server.received_requests().await.unwrap();
    assert_eq!(
        requests
            .iter()
            .filter(|r| r.url.path() == "/completion")
            .count(),
        2
    );
    let order: Vec<String> = requests
        .iter()
        .filter(|r| r.url.path() == "/apply-template")
        .map(|request| {
            let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
            body["messages"][1]["content"]
                .as_str()
                .unwrap()
                .rsplit("Question: ")
                .next()
                .unwrap()
                .lines()
                .next()
                .unwrap()
                .to_owned()
        })
        .collect();
    assert_eq!(order, ["z-first", "a-second"]);
}

#[tokio::test]
async fn llama_answers_choice_bool_and_score_from_label_logprobs() {
    let server = MockServer::start().await;
    mount_llama(&server, LlamaServer::new()).await;

    let record = llama_record(&server.uri());
    let (models, _handle) = llama_models(&record);
    let result = models
        .classify(record, llama_context(), ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Stop);
    assert!(result.error_message.is_none());

    // Choice: labels A/B/C over the declared keys billing/technical/sales;
    // next-token logprobs A=-1.5, B=-0.3, C=-3.
    let expected = label_probabilities(&[-1.5, -0.3, -3.0], 1.0);
    let team = result.answers.get("team").expect("team answered");
    match team {
        ClassifierAnswer::Choice {
            choice: chosen,
            probabilities,
            confidence,
        } => {
            assert_eq!(chosen, "technical");
            assert!((probabilities["billing"] - expected[0]).abs() < 1e-9);
            assert!((probabilities["technical"] - expected[1]).abs() < 1e-9);
            assert!((probabilities["sales"] - expected[2]).abs() < 1e-9);
            assert!((confidence - peak_confidence(&expected)).abs() < 1e-9);
            // Hand-derived softmax, not the helper's own arithmetic.
            assert!((probabilities["technical"] - 0.73075).abs() < 1e-4);
            assert!((confidence - 0.59613).abs() < 1e-4);
        }
        other => panic!("team is not a choice answer: {other:?}"),
    }

    // Bool: probability of the `true` key from Yes/No logprobs.
    let urgent = result.answers.get("urgent").expect("urgent answered");
    match urgent {
        ClassifierAnswer::Bool { probability } => {
            assert!((probability - 0.95026).abs() < 1e-4);
        }
        other => panic!("urgent is not a bool answer: {other:?}"),
    }

    // Score: expected level from digit logprobs, TypeSafe confidence.
    let severity = result.answers.get("severity").expect("severity answered");
    match severity {
        ClassifierAnswer::Score { score, confidence } => {
            let levels = label_probabilities(&[-4.0, -1.8, -0.2], 1.0);
            assert!((score - (levels[1] + 2.0 * levels[2])).abs() < 1e-9);
            assert!((score - 1.79858).abs() < 1e-4);
            assert!((confidence - peak_confidence(&levels)).abs() < 1e-9);
        }
        other => panic!("severity is not a score answer: {other:?}"),
    }

    // Wire: the `/v1` base URL is stripped, every body carries the model id
    // and the Bearer key, and the completion readout carries the source shape.
    let requests = llama_requests(&server).await;
    assert!(!requests.is_empty());
    for (path, body) in &requests {
        assert!(
            ["tokenize", "apply-template", "completion"]
                .iter()
                .any(|end| path.ends_with(end)),
            "unexpected endpoint: {path}"
        );
        assert_eq!(body["model"], "qwen");
    }
    let completion = requests
        .iter()
        .find(|(path, _)| path.ends_with("/completion"))
        .expect("completion request");
    assert_eq!(completion.1["n_predict"], 1);
    assert_eq!(completion.1["n_probs"], 256);
    assert_eq!(completion.1["post_sampling_probs"], false);
    assert_eq!(completion.1["cache_prompt"], true);
    assert_eq!(completion.1["temperature"].as_f64(), Some(0.0));
    let template = requests
        .iter()
        .find(|(path, _)| path.ends_with("/apply-template"))
        .expect("template request");
    assert_eq!(
        template.1["chat_template_kwargs"],
        serde_json::json!({ "enable_thinking": false })
    );
    assert_eq!(template.1["messages"][0]["role"], "system");
    assert!(
        template.1["messages"][0]["content"]
            .as_str()
            .unwrap_or_default()
            .starts_with("You answer one question")
    );
    assert_eq!(template.1["messages"][1]["role"], "user");

    // wiremock's recorded URL carries the path only; the endpoints sit
    // directly at the server root, which proves the `/v1` base is stripped.
    let raw = server.received_requests().await.unwrap_or_default();
    for request in &raw {
        assert!(
            ["/tokenize", "/apply-template", "/completion"].contains(&request.url.path()),
            "unexpected path: {}",
            request.url.path()
        );
        assert_eq!(
            request.headers.get("authorization").unwrap(),
            "Bearer local"
        );
    }
}

#[test]
fn llama_renders_questions_with_state_repetition_and_labels() {
    let context = llama_context();
    let rendered = render_question(&context, "team").expect("team renders");
    let state = "State:\n{\n \"message\": \"Help! My payouts have been failing for 3 days.\"\n}";
    assert_eq!(rendered.labels, ["A", "B", "C"]);
    assert_eq!(rendered.keys, ["billing", "technical", "sales"]);
    assert_eq!(
        rendered.content,
        [
            state,
            "",
            "Task: answer each of the following questions about the state.",
            "",
            "Question: Which team should handle this?",
            "",
            "Options:",
            "- billing: Payments and refunds",
            "- technical: Bugs and outages",
            "- sales",
            "",
            "Question: Does this convey urgency?",
            "",
            "Yes means: The user needs help soon",
            "No means: No time pressure",
            "",
            "Question: How severe is this?",
            "",
            "Levels:",
            "0. low",
            "1. medium",
            "2. high",
            "",
            state,
            "",
            "Question: Which team should handle this?",
            "",
            "Options:",
            "A. billing: Payments and refunds",
            "B. technical: Bugs and outages",
            "C. sales",
            "",
            "Answer with one letter.",
        ]
        .join("\n")
    );

    // Everything before the final question is shared across the questions.
    let prefix = |id: &str| {
        let content = render_question(&context, id).unwrap().content;
        content[..content.rfind("Question:").unwrap()].to_owned()
    };
    assert_eq!(prefix("urgent"), prefix("team"));
    assert_eq!(prefix("severity"), prefix("team"));
    assert!(
        render_question(&context, "urgent")
            .unwrap()
            .content
            .ends_with("No means: No time pressure\n\nAnswer Yes or No.")
    );
    assert!(
        render_question(&context, "severity")
            .unwrap()
            .content
            .ends_with("2. high\n\nAnswer with one level number.")
    );
}

#[tokio::test]
async fn llama_divides_logprobs_by_temperature() {
    let server = MockServer::start().await;
    let mut llama = LlamaServer::new();
    llama.tokenize = Box::new(llama_char_tokens);
    llama.next = Box::new(|_prompt, _depth| vec![(b'A' as u64, -0.1), (b'B' as u64, -2.5)]);
    mount_llama(&server, llama).await;

    let record = llama_record(&server.uri());
    let (models, handle) = llama_models(&record);
    let result = models
        .classify_native(
            record,
            llama_pick_context(),
            ClassifierCall::default(),
            handle.options(LlamaCppClassifyOptions {
                temperature: Some(2.0),
                max_retries: 2,
                max_retry_delay_ms: None,
            }),
        )
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Stop);
    // exp(-0.05) / (exp(-0.05) + exp(-1.25)) and its complement.
    match result.answers.get("pick").expect("pick answered") {
        ClassifierAnswer::Choice { probabilities, .. } => {
            assert!((probabilities["a"] - 0.76852).abs() < 1e-4);
            assert!((probabilities["b"] - 0.23148).abs() < 1e-4);
        }
        other => panic!("pick is not a choice answer: {other:?}"),
    }
}

#[tokio::test]
async fn llama_rejects_non_positive_temperature_before_requests() {
    let server = MockServer::start().await;
    mount_llama(&server, LlamaServer::new()).await;

    let record = llama_record(&server.uri());
    let (models, handle) = llama_models(&record);
    let result = models
        .classify_native(
            record,
            llama_context(),
            ClassifierCall::default(),
            handle.options(LlamaCppClassifyOptions {
                temperature: Some(0.0),
                max_retries: 2,
                max_retry_delay_ms: None,
            }),
        )
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("Temperature must be a positive number, got 0"),
        "message was {:?}",
        result.error_message
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 0);
}

#[tokio::test]
async fn llama_escalates_readout_depth_and_fails_without_invented_zeros() {
    // A label outside the shallow top list re-reads deeper and recovers.
    let server = MockServer::start().await;
    let mut deep = LlamaServer::new();
    deep.tokenize = Box::new(llama_char_tokens);
    deep.next = Box::new(|_prompt, depth| {
        if depth < 4096 {
            vec![(b'A' as u64, -0.1)]
        } else {
            vec![(b'A' as u64, -0.1), (b'B' as u64, -9.0)]
        }
    });
    mount_llama(&server, deep).await;
    let record = llama_record(&server.uri());
    let (models, _handle) = llama_models(&record);
    let result = models
        .classify(record, llama_pick_context(), ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Stop);
    let depths: Vec<u64> = llama_requests(&server)
        .await
        .into_iter()
        .filter(|(path, _)| path.ends_with("/completion"))
        .map(|(_, body)| body["n_probs"].as_u64().unwrap_or_default())
        .collect();
    assert_eq!(depths, [256, 4096]);

    // A label never ranked within the deepest readout is an error, not a
    // zero-probability answer.
    let server = MockServer::start().await;
    let mut never = LlamaServer::new();
    never.tokenize = Box::new(llama_char_tokens);
    never.next = Box::new(|_prompt, _depth| vec![(b'A' as u64, -0.1)]);
    mount_llama(&server, never).await;
    let record = llama_record(&server.uri());
    let (models, _handle) = llama_models(&record);
    let result = models
        .classify(record, llama_pick_context(), ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert!(result.answers.is_empty());
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("did not rank labels B for pick within the top 32768 tokens"),
        "message was {:?}",
        result.error_message
    );
    let depths: Vec<u64> = llama_requests(&server)
        .await
        .into_iter()
        .filter(|(path, _)| path.ends_with("/completion"))
        .map(|(_, body)| body["n_probs"].as_u64().unwrap_or_default())
        .collect();
    assert_eq!(depths, [256, 4096, 32768]);
}

#[tokio::test]
async fn llama_closes_reasoning_blocks_and_reads_labels_in_reply_position() {
    // A template that leaves a reasoning block open gets it closed.
    let server = MockServer::start().await;
    let mut closed = LlamaServer::new();
    closed.tokenize = Box::new(llama_char_tokens);
    closed.template_prompt = Some("<|assistant|>\n<think>".into());
    mount_llama(&server, closed).await;
    let record = llama_record(&server.uri());
    let (models, _handle) = llama_models(&record);
    let result = models
        .classify(record, llama_pick_context(), ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Stop);
    let completion = llama_requests(&server)
        .await
        .into_iter()
        .find(|(path, _)| path.ends_with("/completion"))
        .expect("completion request");
    assert_eq!(completion.1["prompt"], "<|assistant|>\n<think></think>");

    // A tokenizer that merges a newline with the label falls back to the
    // label alone.
    let server = MockServer::start().await;
    let mut merging = LlamaServer::new();
    merging.tokenize = Box::new(|content| {
        if content.starts_with('\n') && content.len() > 1 {
            vec![1000]
        } else {
            llama_char_tokens(content)
        }
    });
    mount_llama(&server, merging).await;
    let record = llama_record(&server.uri());
    let (models, _handle) = llama_models(&record);
    let result = models
        .classify(record, llama_pick_context(), ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Stop);

    // A label the vocabulary splits into several tokens is a typed error.
    let server = MockServer::start().await;
    let mut split = LlamaServer::new();
    split.tokenize = Box::new(llama_char_tokens);
    mount_llama(&server, split).await;
    let record = llama_record(&server.uri());
    let (models, _handle) = llama_models(&record);
    let mut context = llama_pick_context();
    context.questions.insert(
        "ok".to_owned(),
        ClassifierQuestion::Bool {
            instructions: "OK?".into(),
            true_criteria: String::new(),
            false_criteria: String::new(),
        },
    );
    let result = models
        .classify(record, context, ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains(r#"Label "Yes" is not a single token for qwen"#),
        "message was {:?}",
        result.error_message
    );
}

#[tokio::test]
async fn llama_caches_label_tokens_per_server_and_model() {
    let server = MockServer::start().await;
    mount_llama(&server, LlamaServer::new()).await;

    let record = llama_record(&server.uri());
    let (models, _handle) = llama_models(&record);
    let first = models
        .classify(record.clone(), llama_context(), ClassifierCall::default())
        .await;
    assert_eq!(first.stop_reason, ClassifierStopReason::Stop);
    let first_tokenizations = llama_requests(&server)
        .await
        .into_iter()
        .filter(|(path, _)| path.ends_with("/tokenize"))
        .count();
    assert!(first_tokenizations > 0);

    let second = models
        .classify(record, llama_context(), ClassifierCall::default())
        .await;
    assert_eq!(second.stop_reason, ClassifierStopReason::Stop);
    let total_tokenizations = llama_requests(&server)
        .await
        .into_iter()
        .filter(|(path, _)| path.ends_with("/tokenize"))
        .count();
    assert_eq!(total_tokenizations, first_tokenizations);
}

#[tokio::test]
async fn llama_validates_option_counts_before_requests() {
    let server = MockServer::start().await;
    mount_llama(&server, LlamaServer::new()).await;

    let record = llama_record(&server.uri());
    let (models, _handle) = llama_models(&record);

    let mut too_many = llama_pick_context();
    too_many.questions.insert(
        "pick".to_owned(),
        ClassifierQuestion::Choice {
            instructions: "Pick".into(),
            criteria: (0..63)
                .map(|index| (format!("option{index}"), String::new()))
                .collect(),
        },
    );
    let result = models
        .classify(record.clone(), too_many, ClassifierCall::default())
        .await;
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("A choice question needs 2 to 62 options, got 63"),
        "message was {:?}",
        result.error_message
    );

    let mut too_few = llama_pick_context();
    too_few.questions.insert(
        "rate".to_owned(),
        ClassifierQuestion::Score {
            instructions: "Rate".into(),
            levels: vec!["only".into()],
        },
    );
    too_few.questions.remove("pick");
    let result = models
        .classify(record, too_few, ClassifierCall::default())
        .await;
    assert!(
        result
            .error_message
            .as_deref()
            .unwrap_or_default()
            .contains("A score question needs 2 to 10 levels, got 1"),
        "message was {:?}",
        result.error_message
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 0);
}

#[tokio::test]
async fn llama_observers_see_only_completion_payloads_and_responses() {
    let server = MockServer::start().await;
    mount_llama(&server, LlamaServer::new()).await;

    let record = llama_record(&server.uri());
    let (models, _handle) = llama_models(&record);
    let payloads = Arc::new(AtomicU32::new(0));
    let statuses = Arc::new(Mutex::new(Vec::<u16>::new()));
    let mut call = ClassifierCall::default();
    let payload_count = Arc::clone(&payloads);
    let status_log = Arc::clone(&statuses);
    call.callbacks = opi_ai::callbacks::RequestCallbacks::new()
        .with_on_payload(Arc::new(move |body: &serde_json::Value| {
            payload_count.fetch_add(1, AtomicOrdering::SeqCst);
            let mut replaced = body.clone();
            replaced["id_slot"] = serde_json::json!(1);
            Ok(Some(replaced))
        }))
        .with_on_response(Arc::new(
            move |status: u16, _headers: &[(String, String)]| {
                status_log.lock().unwrap().push(status);
                Ok(())
            },
        ));

    let result = models.classify(record, llama_pick_context(), call).await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Stop);
    // Only the completion is observed: one payload across the whole classify.
    assert_eq!(payloads.load(AtomicOrdering::SeqCst), 1);
    assert_eq!(*statuses.lock().unwrap(), vec![200u16]);
    let completion = llama_requests(&server)
        .await
        .into_iter()
        .find(|(path, _)| path.ends_with("/completion"))
        .expect("completion request");
    assert_eq!(completion.1["id_slot"], 1);
}

#[tokio::test]
async fn llama_reports_http_errors_api_mismatch_and_images() {
    // A server error retains the status, redacts the body and does not retry.
    let server = MockServer::start().await;
    let mut failing = LlamaServer::new();
    failing.fail_completion = Some(400);
    mount_llama(&server, failing).await;
    let record = llama_record(&server.uri());
    let (models, handle) = llama_models(&record);
    let result = models
        .classify_native(
            record,
            llama_context(),
            ClassifierCall::default(),
            handle.options(LlamaCppClassifyOptions {
                temperature: None,
                max_retries: 0,
                max_retry_delay_ms: None,
            }),
        )
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    let message = result.error_message.as_deref().unwrap_or_default();
    assert!(
        message.contains("llama.cpp HTTP 400: [REDACTED]"),
        "message was {message}"
    );
    assert!(
        !message.contains("context overflow"),
        "message was {message}"
    );

    // Adapter-level api mismatch rejects before any request.
    let server = MockServer::start().await;
    mount_llama(&server, LlamaServer::new()).await;
    let record = llama_record(&server.uri());
    let adapter = LlamaCppClassifyApi::new(HttpClient::new(), record.clone());
    let mut mismatched = record.clone();
    mismatched.api = ApiId::new(TYPESAFE_SYSTEM_ONE_API_ID);
    let result = adapter
        .classify_prepared(
            mismatched,
            llama_context(),
            adapter.default_options(),
            opi_ai::test_support::resolved_bearer_auth(),
            ClassifierCall::default(),
        )
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    let message = result.error_message.as_deref().unwrap_or_default();
    assert!(
        message.contains("Unsupported classifier API: typesafe-system-one"),
        "message was {message}"
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 0);

    // Image input rejects before any request.
    let record = llama_record(&server.uri());
    let (models, _handle) = llama_models(&record);
    let mut imaged = llama_context();
    imaged.images = Some(vec![ClassifierImage {
        mime_type: "image/png".into(),
        data: "aGk=".into(),
    }]);
    let result = models
        .classify(record, imaged, ClassifierCall::default())
        .await;
    assert_eq!(result.stop_reason, ClassifierStopReason::Error);
    let message = result.error_message.as_deref().unwrap_or_default();
    assert!(
        message.contains("llama.cpp classification does not support image input"),
        "message was {message}"
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 0);
}

#[tokio::test]
async fn llama_callbacks_keep_each_question_and_readout_depth() {
    let server = MockServer::start().await;
    let mut llama = LlamaServer::new();
    llama.tokenize = Box::new(llama_char_tokens);
    llama.next = Box::new(|_, depth| {
        if depth < 4096 {
            vec![(b'A' as u64, -0.1)]
        } else {
            vec![(b'A' as u64, -0.1), (b'B' as u64, -2.0)]
        }
    });
    mount_llama(&server, llama).await;
    let record = llama_record(&server.uri());
    let (models, _) = llama_models(&record);
    let mut context = llama_pick_context();
    let mut second = context.questions.values().next().unwrap().clone();
    if let ClassifierQuestion::Choice { instructions, .. } = &mut second {
        *instructions = "A second independent question".into();
    }
    context.questions.insert("second".into(), second);
    let observed = Arc::new(AtomicU32::new(0));
    let counter = observed.clone();
    let result = models
        .classify(
            record,
            context,
            ClassifierCall {
                callbacks: opi_ai::callbacks::RequestCallbacks::new().with_on_payload(Arc::new(
                    move |_| {
                        counter.fetch_add(1, AtomicOrdering::SeqCst);
                        Ok(None)
                    },
                )),
                ..Default::default()
            },
        )
        .await;
    assert_eq!(
        result.stop_reason,
        ClassifierStopReason::Stop,
        "{:?}",
        result.error
    );
    let completions: Vec<serde_json::Value> = llama_requests(&server)
        .await
        .into_iter()
        .filter(|(path, _)| path == "/completion")
        .map(|(_, body)| body)
        .collect();
    assert_eq!(
        completions
            .iter()
            .map(|body| body["n_probs"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        [256, 4096, 256, 4096]
    );
    assert_eq!(completions[0]["prompt"], completions[1]["prompt"]);
    assert_ne!(completions[0]["prompt"], completions[2]["prompt"]);
    assert_eq!(observed.load(AtomicOrdering::SeqCst), 4);
}

#[tokio::test]
async fn llama_response_callback_cannot_timeout_or_repeat_completion() {
    let server = MockServer::start().await;
    mount_llama(&server, LlamaServer::new()).await;
    let record = llama_record(&server.uri());
    let (models, _) = llama_models(&record);
    let result = models
        .classify(
            record,
            llama_pick_context(),
            ClassifierCall {
                timeout: Some(Duration::from_millis(300)),
                callbacks: opi_ai::callbacks::RequestCallbacks::new().with_async_on_response(
                    |_, _| async {
                        tokio::time::sleep(Duration::from_millis(600)).await;
                        Ok(())
                    },
                ),
                ..Default::default()
            },
        )
        .await;
    assert_eq!(
        result.stop_reason,
        ClassifierStopReason::Stop,
        "{:?}",
        result.error
    );
    let completions = llama_requests(&server)
        .await
        .into_iter()
        .filter(|(path, _)| path == "/completion")
        .count();
    assert_eq!(completions, 1);
}

#[tokio::test]
async fn llama_json_syntax_failure_does_not_retry() {
    assert_syntax_failure("llama").await;
}

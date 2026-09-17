//! The production non-interactive runner retains safe stream failure causes.

mod common;

use opi_ai::anthropic::AnthropicProvider;
use opi_ai::{AuthScheme, ModelCapabilities, ModelInfo, StaticAuthResolver};
use opi_coding_agent::config::OpiConfig;
use opi_coding_agent::policy::ToolSelection;
use opi_coding_agent::project_trust::TrustDecision;
use opi_coding_agent::runner::NonInteractiveRunner;
use opi_coding_agent::runtime_packages::RuntimePackageStartup;
use serde_json::Value;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::time::{Duration, timeout};

const SECRET: &str = "sk-proj-stream-diagnostic-private-canary-0123456789";
const PREFIX: &str = concat!(
    "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_fixture\",\"role\":\"assistant\",\"model\":\"fixture\",\"content\":[],\"usage\":{\"input_tokens\":1,\"output_tokens\":0}}}\n\n",
    "event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n",
    "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"partial fixture output\"}}\n\n",
);

async fn run_case(case: &str, json: bool) -> (String, String, i32) {
    let root = tempfile::tempdir().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let ending = match case {
        "eof" | "truncated" => String::new(),
        "malformed" => format!("event: content_block_delta\ndata: {{invalid-{SECRET}}}\n\n"),
        "upstream" => format!("event: error\ndata: {{\"type\":\"error\",\"error\":{{\"type\":\"overloaded_error\",\"message\":\"{SECRET}\"}}}}\n\n"),
        "normal" => concat!(
            "event: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
            "event: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":3}}\n\n",
            "event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n",
        ).to_owned(),
        _ => panic!("unknown case"),
    };
    let truncated = case == "truncated";
    let server = tokio::spawn(async move {
        timeout(Duration::from_secs(20), async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            loop {
                let byte = socket.read_u8().await.unwrap();
                request.push(byte);
                assert!(request.len() < 32_768);
                if request.ends_with(b"\r\n\r\n") { break; }
            }
            let header = String::from_utf8(request).unwrap();
            let length: usize = header.lines().find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length").then(|| value.trim().parse().unwrap())
            }).unwrap();
            assert!(length < 1_000_000);
            let mut body = vec![0; length];
            socket.read_exact(&mut body).await.unwrap();
            let mut response = PREFIX.as_bytes().to_vec();
            response.extend_from_slice(ending.as_bytes());
            let header = if truncated {
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n".to_owned()
            } else {
                format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", response.len())
            };
            socket.write_all(header.as_bytes()).await.unwrap();
            if truncated {
                socket.write_all(format!("{:x}\r\n", response.len()).as_bytes()).await.unwrap();
                socket.write_all(&response).await.unwrap();
                socket.write_all(b"\r\n").await.unwrap();
                // Deliberately close before the terminating zero-length chunk.
            } else {
                socket.write_all(&response).await.unwrap();
            }
            socket.shutdown().await.unwrap();
        }).await.expect("local server deadline")
    });
    let provider = AnthropicProvider::for_route(
        "fixture".into(),
        vec![ModelInfo::new(
            "fixture",
            "Local fixture",
            opi_ai::model_info::WireApi::AnthropicMessages,
            ModelCapabilities::new(1_000_000, 131_072),
        )],
        Some(format!("http://{address}")),
        Default::default(),
        Arc::new(
            opi_ai::http::HttpClientBuilder::new()
                // Bypass the fixture address through an explicit proxy policy,
                // disabling inherited system proxy discovery without env edits.
                .proxy(opi_ai::http::ProxyConfig {
                    url: Some("http://127.0.0.1:9".into()),
                    no_proxy: Some("127.0.0.1".into()),
                })
                .build()
                .unwrap(),
        ),
        false,
    );
    let mut config = OpiConfig::default();
    config.defaults.max_output_tokens = Some(131_072);
    let mut runner = NonInteractiveRunner::new_with_resume_runtime_packages_and_auth(
        Box::new(provider),
        "fixture:fixture".into(),
        config,
        root.path().to_path_buf(),
        false,
        None,
        Vec::new(),
        None,
        ToolSelection::Default,
        RuntimePackageStartup {
            extension_registry: opi_agent::extension::ExtensionRegistry::new(),
            installed_packages: Vec::new(),
            diagnostics: Vec::new(),
            trust_decision: TrustDecision::Untrusted,
        },
        None,
        Arc::new(StaticAuthResolver::new(AuthScheme::ApiKey, SECRET.into())),
        Vec::new(),
    )
    .unwrap()
    .with_compact_ndjson(true);
    let output = timeout(Duration::from_secs(25), async {
        if json {
            runner.run_json("Return a short completion.").await
        } else {
            runner.run("Return a short completion.").await
        }
    })
    .await
    .expect("runner deadline");
    server.await.unwrap();
    let stdout = output.stdout;
    let stderr = output.stderr;
    assert!(
        !stdout.contains(SECRET) && !stderr.contains(SECRET),
        "synthetic credential leaked"
    );
    (stdout, stderr, output.exit_code)
}

#[tokio::test]
async fn stream_failures_preserve_safe_distinct_causes() {
    let _env_guard = common::empty_user_config_dir();
    let mut causes = Vec::new();
    for case in ["eof", "malformed", "truncated", "upstream"] {
        let (stdout, stderr, exit) = run_case(case, true).await;
        assert_eq!(exit, 4, "{case}: {stderr}");
        assert_eq!(stderr.lines().next(), Some("provider stream failed"));
        let diagnostic: Value = serde_json::from_str(
            stderr
                .lines()
                .nth(1)
                .unwrap_or_else(|| panic!("{case}: missing structured failure detail: {stderr}")),
        )
        .unwrap();
        assert_eq!(diagnostic["code"], "provider_stream_error");
        assert_eq!(diagnostic["source"], "provider");
        let cause = diagnostic["details"]["cause"].as_str().unwrap().to_owned();
        match case {
            "eof" => assert!(cause.contains("without a terminal event"), "{cause}"),
            "malformed" => assert!(cause.contains("malformed Anthropic SSE event"), "{cause}"),
            "truncated" => assert_ne!(cause, "[REDACTED]"),
            "upstream" => assert_eq!(cause, "[REDACTED]"),
            _ => unreachable!(),
        }
        causes.push(cause);
        assert!(
            stdout.contains("partial fixture output"),
            "{case}: partial output lost"
        );
        let events: Vec<Value> = stdout
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(
            events
                .iter()
                .filter(|v| v["event"]["type"] == "TurnStart")
                .count(),
            1
        );
        assert!(!events.iter().any(|v| v["type"] == "AutoRetryStart"));
    }
    for (index, cause) in causes.iter().enumerate() {
        assert!(
            !causes[index + 1..].contains(cause),
            "failure causes collapsed"
        );
    }
}

#[tokio::test]
async fn text_stream_failure_keeps_structured_detail_and_success_stays_clean() {
    let _env_guard = common::empty_user_config_dir();
    let (_, stderr, exit) = run_case("eof", false).await;
    assert_eq!(exit, 4);
    let diagnostic: Value = serde_json::from_str(
        stderr
            .lines()
            .find(|line| line.starts_with('{'))
            .unwrap_or_else(|| panic!("missing stream detail: {stderr}")),
    )
    .unwrap();
    assert!(
        diagnostic["details"]["cause"]
            .as_str()
            .unwrap()
            .contains("without a terminal event")
    );
    let (stdout, stderr, exit) = run_case("normal", true).await;
    assert_eq!(exit, 0, "{stderr}");
    assert!(stderr.is_empty());
    assert!(stdout.contains("partial fixture output"));
}

//! Shared OpenRouter Chat fixtures.
//!
//! These helpers own local peers and reusable provider setup without
//! protocol assertions.

use super::*;

pub(crate) fn openrouter_provider() -> OpenAiChatProvider {
    opi_ai::providers::openrouter::openrouter_provider(None)
}

/// Helper: collect stream events asynchronously.
pub(crate) async fn collect_stream(stream: EventStream) -> Vec<AssistantStreamEvent> {
    stream.filter_map(|r| async move { r.ok() }).collect().await
}

pub(crate) async fn write_chunk(
    socket: &mut tokio::net::TcpStream,
    body: &str,
) -> std::io::Result<()> {
    let chunk = format!("{:X}\r\n{}\r\n", body.len(), body);
    tokio::io::AsyncWriteExt::write_all(socket, chunk.as_bytes()).await
}

pub(crate) async fn spawn_stalled_openrouter_server() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind stalled OpenRouter server");
    let addr = listener.local_addr().expect("stalled server addr");

    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept stalled stream");
        let mut request = Vec::new();
        let mut buf = [0_u8; 1024];

        loop {
            let read = tokio::io::AsyncReadExt::read(&mut socket, &mut buf)
                .await
                .expect("read request");
            if read == 0 {
                return;
            }
            request.extend_from_slice(&buf[..read]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }

        let request_text = String::from_utf8_lossy(&request);
        assert!(
            request_text.starts_with("POST /v1/chat/completions "),
            "unexpected request line: {request_text}"
        );

        tokio::io::AsyncWriteExt::write_all(
            &mut socket,
            b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
        )
        .await
        .expect("write response headers");
        write_chunk(&mut socket, stalled_openrouter_start_chunk())
            .await
            .expect("write initial SSE chunk");
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let _ = write_chunk(&mut socket, stalled_openrouter_terminal_chunk()).await;
        let _ = tokio::io::AsyncWriteExt::write_all(&mut socket, b"0\r\n\r\n").await;
    });

    format!("http://{addr}")
}

fn stalled_openrouter_start_chunk() -> &'static str {
    "data: {\"id\":\"chatcmpl-slow\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"openai/gpt-4o\",\"choices\":[{\"delta\":{\"role\":\"assistant\",\"content\":\"\"},\"finish_reason\":null}]}\n\n"
}

fn stalled_openrouter_terminal_chunk() -> &'static str {
    concat!(
        "data: {\"id\":\"chatcmpl-slow\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"openai/gpt-4o\",\"choices\":[{\"delta\":{\"content\":\"late\"},\"finish_reason\":null}]}\n\n",
        "data: {\"id\":\"chatcmpl-slow\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"openai/gpt-4o\",\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":1}}\n\n",
        "data: [DONE]\n\n",
    )
}

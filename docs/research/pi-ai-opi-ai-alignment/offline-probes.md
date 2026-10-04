# 隔离离线探针

> 非规范性研究证据。探针不修改仓库测试或运行时代码；只连接本机随机端口，使用固定假凭据。2026-10-03。
> 以下是针对当前库的特征探针，断言当前观察结果；不是未来正确行为的回归测试，也不能注册为产品验收。

## 目的与运行方式

核对两个公开流入口差异、网络分块对 UTF-8 的影响，以及丢弃空闲 HTTP 流能否终止后台读取。先运行 README 记录的离线 Cargo 命令构建当前库，再将下列代码提取到系统临时目录，用 `rustc --edition=2024`、`-L dependency=<既有target>/debug/deps`、`--extern opi_ai=<刚构建的rlib>`、`--extern tokio=<对应rlib>` 编译并执行。无需新 Cargo manifest、依赖下载或锁文件。

```rust
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;
use opi_ai::provider::{CacheRetention, EventStream, Provider, Request, ThinkingConfig};
use opi_ai::stream::AssistantStreamEvent;

fn request(model: &str) -> Request {
    Request {
        model: model.into(), system: None, messages: vec![], tools: vec![],
        max_tokens: Some(512), temperature: None, thinking: ThinkingConfig::default(),
        stop_sequences: vec![], metadata: None, cancel: Default::default(),
        timeout: Some(Duration::from_secs(3)), extra_headers: vec![],
        cache_retention: CacheRetention::None, session_id: None,
    }
}
async fn next(stream: &mut EventStream) -> Option<Result<AssistantStreamEvent, opi_ai::provider::ProviderError>> {
    std::future::poll_fn(|cx| stream.as_mut().poll_next(cx)).await
}
fn accept_request(listener: TcpListener) -> TcpStream {
    let (mut socket, _) = listener.accept().unwrap();
    socket.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
    let mut bytes = vec![];
    let mut one = [0; 1];
    while !bytes.ends_with(b"\r\n\r\n") {
        socket.read_exact(&mut one).unwrap(); bytes.push(one[0]);
    }
    let headers = String::from_utf8(bytes).unwrap();
    let len: usize = headers.lines().find_map(|line| {
        line.to_ascii_lowercase().strip_prefix("content-length: ").map(|v| v.parse().unwrap())
    }).unwrap_or(0);
    socket.read_exact(&mut vec![0; len]).unwrap();
    socket
}
fn chunk(socket: &mut TcpStream, bytes: &[u8]) {
    write!(socket, "{:x}\r\n", bytes.len()).unwrap();
    socket.write_all(bytes).unwrap(); socket.write_all(b"\r\n").unwrap(); socket.flush().unwrap();
}
async fn probes() {
    let mut events = opi_ai::test_support::text_response("ok");
    events.push(events[0].clone()); // adversarial event after terminal
    let collection = opi_ai::test_support::single_route_collection(Box::new(
        opi_ai::test_support::MockProvider::new("mock", vec![events])
    ));
    let call = collection.prepare_call("mock:mock-model", request("mock:mock-model")).await.unwrap();
    let mut attempt = call.start_attempt().unwrap();
    while let Some(item) = next(&mut attempt).await { if item.unwrap().is_terminal() { break; } }
    let tail = next(&mut attempt).await.is_some();
    println!("prepared_delivers_event_after_terminal={tail}"); assert!(tail);
    let empty = opi_ai::test_support::single_route_collection(Box::new(
        opi_ai::test_support::MockProvider::new("mock", vec![vec![], vec![]])
    ));
    let call = empty.prepare_call("mock:mock-model", request("mock:mock-model")).await.unwrap();
    let raw_eof = next(&mut call.start_attempt().unwrap()).await.is_none();
    let drain_error = opi_ai::provider_collection::drain_to_completion(call.start_attempt().unwrap()).await.is_err();
    println!("prepared_empty_is_none={raw_eof}; drain_empty_is_error={drain_error}");
    assert!(raw_eof && drain_error);

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        let mut socket = accept_request(listener);
        socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n").unwrap();
        let frame = "data: {\"id\":\"probe\",\"model\":\"probe\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"中\"}}]}\n\n".as_bytes();
        let cut = frame.iter().position(|b| *b == 0xe4).unwrap() + 1;
        chunk(&mut socket, &frame[..cut]);
        std::thread::sleep(Duration::from_millis(100));
        chunk(&mut socket, &frame[cut..]);
        chunk(&mut socket, b"data: {\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n");
        socket.write_all(b"0\r\n\r\n").unwrap();
    });
    let provider = opi_ai::openai_chat::OpenAiChatProvider::new(Some(url));
    let mut stream = provider.stream_prepared(request("openai:gpt-4o-mini"), opi_ai::test_support::resolved_bearer_auth());
    let mut text = String::new();
    while let Some(item) = next(&mut stream).await {
        match item.unwrap() {
            AssistantStreamEvent::TextDelta { delta, .. } => text.push_str(&delta),
            event if event.is_terminal() => break,
            _ => {}
        }
    }
    server.join().unwrap();
    println!("split_utf8_text={text:?}"); assert_ne!(text, "中");

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (dropped_tx, dropped_rx) = std::sync::mpsc::channel();
    let server = std::thread::spawn(move || {
        let mut socket = accept_request(listener);
        socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n").unwrap();
        chunk(&mut socket, b"data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"hello\"}]},\"index\":0}]}\n\n");
        dropped_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        socket.set_read_timeout(Some(Duration::from_millis(500))).unwrap();
        let started = std::time::Instant::now();
        let observation = match socket.read(&mut [0; 1]) {
            Ok(0) => "eof",
            Ok(_) => "unexpected_data",
            Err(e) => match e.kind() {
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut => "timeout",
                std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted => "closed",
                _ => panic!("unexpected socket error: {e}"),
            },
        };
        (observation, started.elapsed().as_millis())
    });
    let provider = opi_ai::gemini::GeminiProvider::new(Some(url));
    let mut stream = provider.stream_prepared(request("gemini:gemini-2.5-flash"), opi_ai::test_support::resolved_auth());
    next(&mut stream).await.unwrap().unwrap();
    drop(stream);
    dropped_tx.send(()).unwrap();
    let (observation, elapsed_ms) = tokio::task::spawn_blocking(move || server.join().unwrap()).await.unwrap();
    println!("gemini_idle_after_drop={observation}; observation_ms={elapsed_ms}"); assert_eq!(observation, "timeout");
}
fn main() {
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(probes());
}
```

## 结果

首次编译与运行均 exit 0。独立评审指出首次 socket 观察把 reset 与 timeout 混为同一 false，且没有 drop 屏障，故该版本不能单独证明 drop 后500ms未关闭。上方修订版加入 drop 屏障，区分 EOF/reset/timeout；以 [原始结果与精确编译命令](offline-probe-result.json) 中修订后运行记录为准。源码摘要明确区分 fenced source 的 LF 归一化 hash 与实际编译文件的原始字节 hash。

```text
prepared_delivers_event_after_terminal=true
prepared_empty_is_none=true; drain_empty_is_error=true
split_utf8_text="���"
gemini_idle_after_drop=timeout; observation_ms=<实际计时见结果JSON>
```

第一轮执行的 Python 输出器因 Windows GBK 无法打印替换字符而失败，未保存完整结果；将 stdout 配置成 UTF-8 后重跑以上探针并保存结果。这是探针记录器修正，未更改被测库。

500 ms 是本探针观察窗口，不是擅自确定的目标性能要求；连接未关闭证明此场景下没有及时 drop cleanup，不证明永久泄漏。UTF-8 探针通过 HTTP chunked framing 和分段发送触发实际 reqwest 读取路径，不能用它推断其他 adapter 已运行验证。

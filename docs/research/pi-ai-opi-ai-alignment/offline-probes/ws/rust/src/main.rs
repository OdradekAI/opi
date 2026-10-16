use std::{io, pin::Pin, sync::{Arc, Mutex}, task::{Context, Poll}};
use futures_util::StreamExt;
use serde_json::{json, Value};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio_tungstenite::{client_async, WebSocketStream, tungstenite::{client::IntoClientRequest, protocol::Role}};

#[derive(Clone, Default)]
struct Stats(Arc<Mutex<(usize, usize, Vec<u8>)>>);
struct SuppliedReader { input: Vec<u8>, position: usize, chunk: usize, stats: Stats }
impl AsyncRead for SuppliedReader {
    fn poll_read(mut self: Pin<&mut Self>, _: &mut Context<'_>, output: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let n = self.chunk.min(output.remaining()).min(self.input.len() - self.position);
        if n > 0 {
            output.put_slice(&self.input[self.position..self.position+n]);
            self.position += n;
            let mut s = self.stats.0.lock().unwrap(); s.0 += 1; s.1 += n;
        }
        Poll::Ready(Ok(()))
    }
}
impl AsyncWrite for SuppliedReader {
    fn poll_write(self: Pin<&mut Self>, _: &mut Context<'_>, bytes: &[u8]) -> Poll<io::Result<usize>> {
        self.stats.0.lock().unwrap().2.extend_from_slice(bytes); Poll::Ready(Ok(bytes.len()))
    }
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> { Poll::Ready(Ok(())) }
    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> { Poll::Ready(Ok(())) }
}
fn unhex(s: &str) -> Vec<u8> {
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()
}
fn request(url: &str) -> tokio_tungstenite::tungstenite::http::Request<()> {
    let mut r = url.into_client_request().unwrap();
    r.headers_mut().insert("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==".parse().unwrap());
    r.headers_mut().insert("Authorization", "Bearer offline-fake-token".parse().unwrap());
    r.headers_mut().insert("X-Opi-Offline-Canary", "pi130-canary".parse().unwrap());
    r
}
async fn message<S: AsyncRead + AsyncWrite + Unpin>(ws: &mut WebSocketStream<S>) -> Value {
    match ws.next().await {
        Some(Ok(m)) => json!({"outcome":"message", "messageType": if m.is_text() {"text"} else {"other"}, "messageLength": m.len(), "messagePreview": m.to_text().ok().map(|x| x.chars().take(100).collect::<String>())}),
        Some(Err(e)) => json!({"outcome":"frame-error", "error":e.to_string(), "debug":format!("{e:?}")}),
        None => json!({"outcome":"stream-end"}),
    }
}
async fn synthetic(path: &str) {
    let rows: Vec<Value> = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    for row in rows {
        let bytes = unhex(row["hex"].as_str().unwrap());
        let stats = Stats::default();
        let reader = SuppliedReader { input: bytes.clone(), position: 0, chunk: row["chunk"].as_u64().unwrap() as usize, stats: stats.clone() };
        let mut out = match client_async(request("ws://127.0.0.1/offline"), reader).await {
            Ok((mut ws, response)) => json!({"opened":true,"status":response.status().as_u16(),"result":message(&mut ws).await}),
            Err(e) => json!({"opened":false,"result":{"outcome":"handshake-error","error":e.to_string(),"debug":format!("{e:?}")}}),
        };
        let s = stats.0.lock().unwrap();
        out["case"] = row["id"].clone(); out["layer"] = json!("deterministic-supplied-AsyncRead-native-handshake");
        out["successfulReadCalls"] = json!(s.0); out["readBytes"] = json!(s.1);
        out["capturedRequest"] = json!(String::from_utf8_lossy(&s.2));
        out["readMeasurement"] = json!("total successful AsyncRead calls including frame reads after a successful handshake; failed-handshake cases stop before frame reads");
        println!("{out}");
        drop(s);
        // Public attachment mechanism only. No HTTP validation is performed here;
        // it is deliberately NOT an owned-handshake implementation.
        if row["id"].as_str() != Some("baseline-one-byte-read") {
            let cut = row["headerBytes"].as_u64().unwrap() as usize;
            let stats = Stats::default();
            let reader = SuppliedReader { input: vec![], position: 0, chunk: 1_000_000, stats };
            let mut ws = WebSocketStream::from_partially_read(reader, bytes[cut..].to_vec(), Role::Client, None).await;
            let r = message(&mut ws).await;
            let after = if row["id"] == "baseline-101-tail" || row["id"] == "tail-text-70000" { message(&mut ws).await } else { Value::Null };
            println!("{}",json!({"case":row["id"],"layer":"public-from_partially_read-codec-only-no-handshake-validation","result":r,"secondNext":after}));
        }
    }
}
#[tokio::main(flavor="current_thread")]
async fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--synthetic") { synthetic(&args[2]).await; return; }
    let url = &args[1];
    let parsed = url.strip_prefix("ws://127.0.0.1:").expect("Only loopback URLs allowed");
    let port: u16 = parsed.split('/').next().unwrap().parse().unwrap();
    let stream = tokio::net::TcpStream::connect((std::net::Ipv4Addr::LOCALHOST,port)).await.unwrap();
    let result = match client_async(request(url), stream).await {
        Ok((mut ws, response)) => { let r = message(&mut ws).await; json!({"layer":"direct-TTW-client_async-loopback-not-opi-provider","runtime":"tokio-tungstenite0.28.0/tungstenite0.28.0","opened":true,"status":response.status().as_u16(),"outcome":r["outcome"],"result":r}) },
        Err(e) => json!({"layer":"direct-TTW-client_async-loopback-not-opi-provider","runtime":"tokio-tungstenite0.28.0/tungstenite0.28.0","opened":false,"outcome":"handshake-error","error":e.to_string(),"debug":format!("{e:?}")}),
    };
    println!("{result}");
}

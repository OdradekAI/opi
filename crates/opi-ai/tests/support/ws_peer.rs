// Scripted loopback HTTP peer shared by public Upgrade tests and private codec supplements.
// This is test support only; it contains no production protocol implementation.

use crate::ws;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use url::Url;

pub(crate) struct Peer {
    address: SocketAddr,
    handle: Option<std::thread::JoinHandle<Result<(), String>>>,
}

impl Peer {
    /// Spawn a scripted server thread. `head_fn` receives the parsed request
    /// head and returns the raw 101 response bytes; the follow-up behavior
    /// is selected by `after`.
    pub(crate) fn spawn<F, G>(after: G, head_fn: F) -> Peer
    where
        F: FnOnce(&RequestHead) -> Vec<u8> + Send + 'static,
        G: FnOnce(TcpStream) -> Result<(), String> + Send + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let head = {
                let mut reader = BufReader::new(&stream);
                RequestHead::read(&mut reader)
            };
            let mut writer = &stream;
            writer.write_all(&head_fn(&head)).unwrap();
            after(stream)
        });
        Peer {
            address,
            handle: Some(handle),
        }
    }

    pub(crate) fn url(&self) -> Url {
        Url::parse(&format!("ws://{}/socket", self.address)).unwrap()
    }

    pub(crate) fn finish(mut self) -> Result<(), String> {
        self.handle
            .take()
            .expect("peer thread")
            .join()
            .unwrap_or_else(|_| Err("server thread panicked".into()))
    }
}

impl Drop for Peer {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

#[derive(Debug)]
pub(crate) struct RequestHead {
    lines: Vec<String>,
}

impl RequestHead {
    pub(crate) fn read<R: BufRead + Read>(reader: &mut R) -> Self {
        let mut lines = Vec::new();
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap() == 0 || line == "\r\n" {
                break;
            }
            lines.push(line.trim_end().to_owned());
        }
        Self { lines }
    }

    pub(crate) fn header(&self, name: &str) -> Option<&str> {
        self.lines
            .iter()
            .filter_map(|l| l.split_once(':'))
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.trim())
    }

    // Only the public handshake battery needs the request line.
    #[allow(dead_code)]
    pub(crate) fn request_line(&self) -> String {
        self.lines.first().cloned().unwrap_or_default()
    }
}

pub(crate) fn nonce_from_key(key: &str) -> [u8; 16] {
    use base64::Engine;
    let mut nonce = [0_u8; 16];
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(key.as_bytes())
        .unwrap();
    nonce.copy_from_slice(&bytes);
    nonce
}

/// Build a 101 response for the request head with optional extra headers.
pub(crate) fn response_101(head: &RequestHead, extra: &[(&str, &str)]) -> Vec<u8> {
    let accept = ws::accept_token(&nonce_from_key(head.header("sec-websocket-key").unwrap()));
    let mut response = format!(
        "HTTP/1.1 101 Switching Protocols\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Accept: {accept}\r\n"
    );
    for (name, value) in extra {
        response.push_str(&format!("{name}: {value}\r\n"));
    }
    response.push_str("\r\n");
    response.into_bytes()
}

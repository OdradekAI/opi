//! TLS transport termination, cancellation and owned cleanup.

use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use futures_util::StreamExt;
use hyper::Response;
use opi_ai::auth::{AuthProvenanceSource, AuthScheme, StaticAuthResolver};
use opi_ai::http::{HttpClientBuilder, TlsOptions};
use opi_ai::models::Models;
use opi_ai::provider::{EventStream, ProviderError};
use opi_ai::providers::openai::OpenAiChatProvider;
use opi_ai::stream::ChatStream;
use rustls::{
    ServerConfig,
    pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    time::timeout,
};
use tokio_rustls::TlsAcceptor;

use super::fixtures::{
    KxProfile, Trust, build_tls_client, chat_models_call, chat_request, instance_provider,
    prepare_chat_request, server_config, start_server,
};
use super::tls_fixtures::{
    CERT_CA, CERT_CLIENT_RSA2048, CERT_SERVER_RSA, KEY_CLIENT_RSA2048, KEY_SERVER_RSA,
};

async fn raw_cancel_case(
    response_phase: bool,
    abort: bool,
    accepts_or_active: Arc<AtomicUsize>,
    eof: Arc<AtomicUsize>,
    late_bytes: Arc<AtomicUsize>,
    ready: Arc<tokio::sync::Notify>,
) -> Result<String, String> {
    let listener = TcpListener::bind("127.0.0.1:0").await.map_err(|_| "bind")?;
    let addr = listener.local_addr().map_err(|_| "address")?;
    let acceptor = TlsAcceptor::from(Arc::new(
        server_config(
            CERT_SERVER_RSA,
            KEY_SERVER_RSA,
            true,
            false,
            true,
            &KxProfile::Declared,
        )
        .expect("cancel server config"),
    ));
    let r = ready.clone();
    let a = accepts_or_active.clone();
    let e = eof.clone();
    let late = late_bytes.clone();
    let mut server = tokio::spawn(async move {
        let (mut tcp, _) = listener.accept().await.map_err(|_| "accept".to_owned())?;
        a.fetch_add(1, Ordering::SeqCst);
        struct Guard(Arc<AtomicUsize>);
        impl Drop for Guard {
            fn drop(&mut self) {
                self.0.fetch_sub(1, Ordering::SeqCst);
            }
        }
        let _guard = Guard(a.clone());
        if response_phase {
            let mut tls = acceptor
                .accept(tcp)
                .await
                .map_err(|_| "handshake".to_owned())?;
            let mut data = Vec::new();
            let mut buf = [0u8; 1024];
            loop {
                let n = tls
                    .read(&mut buf)
                    .await
                    .map_err(|_| "request read".to_owned())?;
                if n == 0 {
                    return Err("cancelled before request headers".to_owned());
                }
                data.extend_from_slice(&buf[..n]);
                if let Some(position) = data.windows(4).position(|w| w == b"\r\n\r\n") {
                    let length = String::from_utf8_lossy(&data[..position])
                        .lines()
                        .find_map(|line| {
                            line.split_once(':')
                                .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                                .map(|(_, value)| value.trim().parse::<usize>().unwrap())
                        })
                        .unwrap();
                    let expected = position + 4 + length;
                    while data.len() < expected {
                        let n = tls
                            .read(&mut buf)
                            .await
                            .map_err(|_| "request body".to_owned())?;
                        if n == 0 {
                            return Err("truncated request body".to_owned());
                        }
                        data.extend_from_slice(&buf[..n]);
                    }
                    late.fetch_add(data.len() - expected, Ordering::SeqCst);
                    break;
                }
            }
            r.notify_one();
            loop {
                match tls.read(&mut buf).await {
                    Ok(0) | Err(_) => {
                        e.fetch_add(1, Ordering::SeqCst);
                        break;
                    }
                    Ok(n) => {
                        late.fetch_add(n, Ordering::SeqCst);
                    }
                }
            }
        } else {
            let mut buf = [0u8; 4096];
            let first = tcp
                .read(&mut buf)
                .await
                .map_err(|_| "hello read".to_owned())?;
            if first == 0 {
                return Err("no ClientHello".to_owned());
            }
            r.notify_one();
            loop {
                match tcp.read(&mut buf).await {
                    Ok(0) | Err(_) => {
                        e.fetch_add(1, Ordering::SeqCst);
                        break;
                    }
                    Ok(_) => {}
                }
            }
        }
        if tokio::time::timeout(Duration::from_millis(75), listener.accept())
            .await
            .is_ok()
        {
            return Err("late connection after cancellation".to_owned());
        }
        Ok::<_, String>(())
    });
    let client = Arc::new(
        build_tls_client(
            Trust::Explicit(CERT_CA),
            Some((CERT_CLIENT_RSA2048, KEY_CLIENT_RSA2048)),
            false,
        )
        .expect("client"),
    );
    let url = format!("https://{addr}/paused");
    if abort {
        // Abort path: the owning task is aborted after the server reports
        // readiness; the client and its identity stay owned by the task.
        let c = client.clone();
        let u = url.clone();
        let task = tokio::spawn(async move { chat_models_call(&u, c).await });
        if timeout(Duration::from_secs(5), ready.notified())
            .await
            .is_err()
        {
            task.abort();
            let _ = task.await;
            server.abort();
            let _ = (&mut server).await;
            return Err("cancel ready".to_owned());
        }
        task.abort();
        if !task.await.is_err_and(|e| e.is_cancelled()) {
            server.abort();
            let _ = (&mut server).await;
            return Err("request owner not cancelled".to_owned());
        }
    } else {
        // Drop path: the request future (and then the whole client) is
        // dropped after the server reports readiness.
        let c = client.clone();
        let mut request = Box::pin({
            let c = c.clone();
            let u = url.clone();
            async move { chat_models_call(&u, c).await }
        });
        tokio::select! {
            _ = ready.notified() => {}
            _ = &mut request => {
                server.abort();
                let _ = (&mut server).await;
                return Err("request ended before controlled drop".to_owned());
            }
        }
        drop(request);
        drop(client);
    }
    match timeout(Duration::from_secs(5), &mut server).await {
        Ok(joined) => joined.map_err(|_| "cancel server task".to_owned())??,
        Err(_) => {
            server.abort();
            let _ = (&mut server).await;
            return Err("cancel EOF timeout".to_owned());
        }
    }
    if accepts_or_active.load(Ordering::SeqCst) != 0
        || eof.load(Ordering::SeqCst) != 1
        || late_bytes.load(Ordering::SeqCst) != 0
    {
        return Err("cancel cleanup counters".to_owned());
    }
    Ok(format!(
        "phase={} method={}",
        if response_phase {
            "awaiting response"
        } else {
            "TLS handshake"
        },
        if abort {
            "abort+await owned task"
        } else {
            "drop future+client"
        }
    ))
}

#[tokio::test]
async fn cancellation_during_handshake_and_response_releases_transport() {
    for response_phase in [false, true] {
        for abort in [false, true] {
            let counters = Arc::new(AtomicUsize::new(0));
            let eof = Arc::new(AtomicUsize::new(0));
            let late = Arc::new(AtomicUsize::new(0));
            let ready = Arc::new(tokio::sync::Notify::new());
            let outcome = raw_cancel_case(response_phase, abort, counters, eof, late, ready).await;
            outcome.unwrap_or_else(|e| {
                panic!(
                    "cancel {} {} case: {e}",
                    if response_phase {
                        "response"
                    } else {
                        "handshake"
                    },
                    if abort { "abort" } else { "drop" }
                )
            });
        }
    }
}

async fn partial_body_case(abort: bool) -> Result<(), String> {
    let listener = TcpListener::bind("127.0.0.1:0").await.map_err(|_| "bind")?;
    let addr = listener.local_addr().map_err(|_| "address")?;
    let active = Arc::new(AtomicUsize::new(0));
    let eof = Arc::new(AtomicUsize::new(0));
    let late_bytes = Arc::new(AtomicUsize::new(0));
    let acceptor = TlsAcceptor::from(Arc::new(
        server_config(
            CERT_SERVER_RSA,
            KEY_SERVER_RSA,
            true,
            false,
            true,
            &KxProfile::Declared,
        )
        .map_err(|e| e.to_owned())?,
    ));
    let a = active.clone();
    let e = eof.clone();
    let late = late_bytes.clone();
    let mut server = tokio::spawn(async move {
        let (tcp, _) = listener.accept().await.map_err(|_| "accept".to_owned())?;
        a.fetch_add(1, Ordering::SeqCst);
        struct Guard(Arc<AtomicUsize>);
        impl Drop for Guard {
            fn drop(&mut self) {
                self.0.fetch_sub(1, Ordering::SeqCst);
            }
        }
        let _guard = Guard(a.clone());
        let mut tls = acceptor
            .accept(tcp)
            .await
            .map_err(|_| "handshake".to_owned())?;
        let mut data = Vec::new();
        let mut buf = [0u8; 1024];
        loop {
            let n = tls.read(&mut buf).await.map_err(|_| "headers".to_owned())?;
            if n == 0 {
                return Err("missing request".to_owned());
            }
            data.extend_from_slice(&buf[..n]);
            if let Some(position) = data.windows(4).position(|w| w == b"\r\n\r\n") {
                late.fetch_add(data.len() - (position + 4), Ordering::SeqCst);
                break;
            }
        }
        tls.write_all(
            b"HTTP/1.1 200 OK\r\nContent-Length: 100000\r\nConnection: close\r\n\r\npartial",
        )
        .await
        .map_err(|_| "response write".to_owned())?;
        tls.flush().await.map_err(|_| "flush".to_owned())?;
        loop {
            match tls.read(&mut buf).await {
                Ok(0) | Err(_) => {
                    e.fetch_add(1, Ordering::SeqCst);
                    break;
                }
                Ok(n) => {
                    late.fetch_add(n, Ordering::SeqCst);
                }
            }
        }
        if tokio::time::timeout(Duration::from_millis(75), listener.accept())
            .await
            .is_ok()
        {
            return Err("late body connection".to_owned());
        }
        Ok::<_, String>(())
    });
    let client = Arc::new(
        build_tls_client(
            Trust::Explicit(CERT_CA),
            Some((CERT_CLIENT_RSA2048, KEY_CLIENT_RSA2048)),
            false,
        )
        .map_err(|e| e.to_string())?,
    );
    let url = format!("https://{addr}/partial");
    let pending = Arc::new(AtomicUsize::new(0));
    let outcome = async {
        let mut response = client
            .client()
            .get(&url)
            .send()
            .await
            .map_err(|_| "partial HTTPS".to_owned())?;
        let first = response
            .chunk()
            .await
            .map_err(|_| "first chunk")?
            .ok_or("missing first chunk")?;
        if first.as_ref() != b"partial" {
            return Err("partial bytes changed".to_owned());
        }
        if abort {
            let ready = Arc::new(tokio::sync::Notify::new());
            let r = ready.clone();
            let p = pending.clone();
            let owned = client.clone();
            let task = tokio::spawn(async move {
                let _owner = owned;
                let mut next = Box::pin(response.chunk());
                if !futures_util::poll!(&mut next).is_pending() {
                    return Err("body completed before controlled abort".to_owned());
                }
                p.fetch_add(1, Ordering::SeqCst);
                r.notify_one();
                next.await
                    .map(|_| ())
                    .map_err(|_| "pending body read".to_owned())
            });
            if timeout(Duration::from_secs(5), ready.notified())
                .await
                .is_err()
            {
                task.abort();
                let _ = task.await;
                return Err("body owner ready".to_owned());
            }
            task.abort();
            if !task.await.is_err_and(|e| e.is_cancelled()) {
                return Err("body owner not cancelled".to_owned());
            }
        } else {
            {
                let mut next = Box::pin(response.chunk());
                if !futures_util::poll!(&mut next).is_pending() {
                    return Err("body completed before controlled drop".to_owned());
                }
                pending.fetch_add(1, Ordering::SeqCst);
            }
            drop(response);
        }
        Ok::<_, String>(())
    }
    .await;
    outcome.map_err(|e| e.to_string())?;
    drop(client);
    match timeout(Duration::from_secs(5), &mut server).await {
        Ok(joined) => joined.map_err(|_| "body server task".to_owned())??,
        Err(_) => {
            server.abort();
            let _ = server.await;
            return Err("body EOF timeout".to_owned());
        }
    }
    if active.load(Ordering::SeqCst) != 0
        || eof.load(Ordering::SeqCst) != 1
        || pending.load(Ordering::SeqCst) != 1
        || late_bytes.load(Ordering::SeqCst) != 0
    {
        return Err("body cleanup or pending/late counters".to_owned());
    }
    Ok(())
}

#[tokio::test]
async fn accepted_partial_body_cancellation_cleanup() {
    partial_body_case(false).await.expect("drop case");
    partial_body_case(true).await.expect("abort case");
}

#[tokio::test]
async fn public_tls_partial_cancellation_preserves_known_usage_and_eof() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!(
        "https://localhost:{}",
        listener.local_addr().unwrap().port()
    );
    let config = server_config(
        CERT_SERVER_RSA,
        KEY_SERVER_RSA,
        true,
        false,
        false,
        &KxProfile::Declared,
    )
    .unwrap();
    let acceptor = TlsAcceptor::from(Arc::new(config));
    let peer = tokio::spawn(async move {
        let (tcp, _) = listener.accept().await.unwrap();
        let mut tls = acceptor.accept(tcp).await.unwrap();
        let mut data = Vec::new();
        let mut bytes = [0; 4096];
        loop {
            let n = tls.read(&mut bytes).await.unwrap();
            assert_ne!(n, 0);
            data.extend_from_slice(&bytes[..n]);
            if let Some(at) = data.windows(4).position(|w| w == b"\r\n\r\n") {
                let length = String::from_utf8_lossy(&data[..at])
                    .lines()
                    .find_map(|line| {
                        line.split_once(':')
                            .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                            .map(|(_, value)| value.trim().parse::<usize>().unwrap())
                    })
                    .unwrap();
                if data.len() >= at + 4 + length {
                    break;
                }
            }
        }
        let event = concat!(
            "data: {\"id\":\"partial-tls\",\"model\":\"gpt-4o\",",
            "\"choices\":[{\"index\":0,\"delta\":{\"content\":\"retained\"},\"finish_reason\":null}],",
            "\"usage\":{\"prompt_tokens\":7,\"completion_tokens\":3}}\n\n"
        );
        tls.write_all(format!("HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ntransfer-encoding: chunked\r\n\r\n{:x}\r\n{event}\r\n", event.len()).as_bytes()).await.unwrap();
        tls.flush().await.unwrap();
        // Only the connection shutdown may follow cancellation; no further
        // business bytes or reconnect are allowed.
        assert!(matches!(tls.read(&mut bytes).await, Ok(0) | Err(_)));
        assert!(
            timeout(Duration::from_millis(75), listener.accept())
                .await
                .is_err()
        );
    });
    let request = chat_request();
    let cancel = request.cancel.clone();
    let client = Arc::new(build_tls_client(Trust::Explicit(CERT_CA), None, true).unwrap());
    let call = prepare_chat_request(&url, client, request).await;
    let mut stream = ChatStream::new(call.start_attempt().unwrap());
    loop {
        let event = timeout(Duration::from_secs(5), stream.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        if let opi_ai::AssistantStreamEvent::TextDelta { delta, partial, .. } = event {
            assert_eq!(delta, "retained");
            assert!(partial.usage.reported);
            assert_eq!(
                (partial.usage.input_tokens, partial.usage.output_tokens),
                (7, 3)
            );
            break;
        }
    }
    cancel.cancel();
    let failure = timeout(Duration::from_secs(5), stream.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap_err();
    assert!(matches!(failure.cause(), ProviderError::Cancelled));
    assert_eq!(
        failure.message().stop_reason,
        opi_ai::stream::StopReason::Aborted
    );
    assert!(failure.message().usage.reported);
    assert_eq!(
        (
            failure.message().usage.input_tokens,
            failure.message().usage.output_tokens
        ),
        (7, 3)
    );
    assert!(
        matches!(&failure.message().content[0], opi_ai::message::AssistantContent::Text { text, .. } if text == "retained")
    );
    assert!(stream.next().await.is_none());
    assert!(stream.next().await.is_none());
    assert!(matches!(
        call.start_attempt(),
        Err(opi_ai::ModelsError::CallCancelled)
    ));
    drop(stream);
    timeout(Duration::from_secs(5), peer)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn chat_level_cancellation_stops_the_attempt_without_extra_sends() {
    let server = start_server(
        CERT_SERVER_RSA,
        KEY_SERVER_RSA,
        true,
        false,
        false,
        KxProfile::Declared,
    )
    .await
    .expect("server start");
    let client = Arc::new(build_tls_client(Trust::Explicit(CERT_CA), None, false).expect("client"));
    let provider = OpenAiChatProvider::with_client(
        Some(server.url.clone()),
        "openai".to_owned(),
        vec![],
        client,
    );
    let mut models = Models::new();
    models
        .register_route(
            Box::new(provider),
            Arc::new(StaticAuthResolver::new(
                AuthScheme::Bearer,
                secrecy::SecretString::from("tls-contract-key"),
            )),
            AuthProvenanceSource::Static,
            opi_ai::models::CompatMetadata {
                openai_compatible: true,
                profile: None,
            },
        )
        .unwrap();
    let request = chat_request();
    let token = request.cancel.clone();
    let prepared = models
        .prepare_call("openai:gpt-4o", request)
        .await
        .expect("prepare");
    let mut stream: EventStream = prepared.start_attempt().expect("attempt");
    // Wait for the first stream event (the request has been sent and the
    // response has begun), then cancel mid-stream and drop the attempt.
    let first = timeout(Duration::from_secs(5), stream.next())
        .await
        .expect("first event timeout")
        .expect("stream must produce a first event")
        .expect("first event must not be an error");
    assert!(
        !first.is_terminal(),
        "premature terminal before cancellation"
    );
    token.cancel();
    let mut saw_terminal = false;
    while let Some(item) = timeout(Duration::from_secs(5), stream.next())
        .await
        .expect("stream must terminate after cancellation")
    {
        match item {
            Ok(event) if event.is_terminal() => saw_terminal = true,
            Err(ProviderError::Cancelled) => saw_terminal = true,
            Err(ProviderError::StreamFailure(failure))
                if matches!(failure.cause(), ProviderError::Cancelled) =>
            {
                saw_terminal = true
            }
            Err(other) => panic!("unexpected stream error: {other:?}"),
            _ => {}
        }
    }
    drop(stream);
    assert!(saw_terminal, "cancelled attempt must reach one terminal");
    let stats = server.stats.clone();
    server.finish().await.expect("server finish");
    assert_eq!(
        stats.http.load(Ordering::SeqCst),
        1,
        "exactly one request before cancellation"
    );
    assert_eq!(
        stats.accepts.load(Ordering::SeqCst),
        1,
        "cancellation must not trigger a second send"
    );
}

#[tokio::test]
async fn h2_refused_stream_does_not_trigger_backend_retry() {
    let mut config = ServerConfig::builder_with_provider(instance_provider(&KxProfile::Declared))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(
            vec![CertificateDer::from_pem_slice(CERT_SERVER_RSA.as_bytes()).unwrap()],
            PrivateKeyDer::from_pem_slice(KEY_SERVER_RSA.as_bytes()).unwrap(),
        )
        .unwrap();
    config.alpn_protocols = vec![b"h2".to_vec()];
    let acceptor = TlsAcceptor::from(Arc::new(config));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!(
        "https://localhost:{}",
        listener.local_addr().unwrap().port()
    );
    let sends = Arc::new(AtomicUsize::new(0));
    let counter = sends.clone();
    let peer = tokio::spawn(async move {
        let (tcp, _) = listener.accept().await.unwrap();
        let tls = acceptor.accept(tcp).await.unwrap();
        let mut connection = h2::server::handshake(tls).await.unwrap();
        while let Some(Ok((_request, mut respond))) = connection.accept().await {
            if counter.fetch_add(1, Ordering::SeqCst) == 0 {
                respond.send_reset(h2::Reason::REFUSED_STREAM);
            } else {
                let response = Response::builder()
                    .status(200)
                    .header("content-type", "text/event-stream")
                    .body(())
                    .unwrap();
                let mut body = respond.send_response(response, false).unwrap();
                body.send_data(hyper::body::Bytes::from_static(b"data: [DONE]\n\n"), true)
                    .unwrap();
            }
        }
    });
    let client = HttpClientBuilder::new()
        .tls(
            TlsOptions::new()
                .with_explicit_roots_pem(CERT_CA.as_bytes())
                .unwrap(),
        )
        .build()
        .unwrap();
    let result = timeout(
        Duration::from_secs(5),
        chat_models_call(&url, Arc::new(client)),
    )
    .await
    .unwrap();
    peer.abort();
    let _ = peer.await;
    assert_eq!(
        sends.load(Ordering::SeqCst),
        1,
        "only the SDK may admit another send"
    );
    assert!(
        matches!(result, Err(ProviderError::Network(_))),
        "{result:?}"
    );
}

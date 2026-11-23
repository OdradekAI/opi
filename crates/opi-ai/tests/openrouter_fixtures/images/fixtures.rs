//! Shared OpenRouter Images fixtures.
//!
//! This module owns image records, local retry/timeout peers and request
//! builders without assertions.

use super::*;

fn image_cost_rates() -> ImageCostRates {
    ImageCostRates {
        input: 3.0,
        output: 15.0,
        cache_read: 0.3,
        cache_write: 3.75,
    }
}

pub(crate) fn image_record(base_url: String, output: Vec<ImageOutputModality>) -> ImageModelRecord {
    ImageModelRecord {
        cost_tiers: Vec::new(),
        headers: Vec::new(),
        input_limits: None,
        input: vec![
            opi_ai::catalog::CatalogInputModality::Text,
            opi_ai::catalog::CatalogInputModality::Image,
        ],
        provider_id: "openrouter".into(),
        id: "google/gemini-2.5-flash-image".into(),
        name: "Gemini 2.5 Flash Image (via OpenRouter)".into(),
        api: ApiId::new("openrouter-images"),
        base_url,
        output,
        cost: Some(image_cost_rates()),
    }
}

pub(crate) fn image_models(record: &ImageModelRecord) -> Models {
    let mut models = Models::new();
    models
        .register_image_api(
            std::sync::Arc::new(OpenRouterImagesApi::new(HttpClient::new(), record.clone())),
            std::sync::Arc::new(opi_ai::auth::StaticAuthResolver::new(
                AuthScheme::Bearer,
                secrecy::SecretString::from("test-key"),
            )),
            AuthProvenanceSource::Static,
        )
        .expect("register image api");
    models
}

pub(crate) fn images_call() -> ImagesCall {
    ImagesCall {
        timeout: Some(Duration::from_secs(30)),
        ..Default::default()
    }
}

pub(crate) fn text_part_json(text: &str) -> serde_json::Value {
    serde_json::json!({ "type": "text", "text": text })
}

pub(crate) async fn spawn_retry_images_server()
-> (String, std::sync::Arc<std::sync::Mutex<Vec<Vec<u8>>>>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind retry server");
    let addr = listener.local_addr().expect("retry server addr");
    let bodies = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let captured = std::sync::Arc::clone(&bodies);
    tokio::spawn(async move {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut served = 0;
        loop {
            let Ok((mut socket, _)) = listener.accept().await else {
                return;
            };
            let mut buf = vec![0u8; 65536];
            let read = socket.read(&mut buf).await.unwrap_or(0);
            let mut body = buf[..read].to_vec();
            // Keep only the JSON body after the blank line separator.
            if let Some(index) = body.windows(4).position(|window| window == b"\r\n\r\n") {
                body = body[index + 4..].to_vec();
            }
            captured.lock().expect("bodies").push(body);
            served += 1;
            let response = if served == 1 {
                "HTTP/1.1 429 Too Many Requests\r\nretry-after-ms: 5\r\ncontent-type: application/json\r\n\r\n{\"error\":{\"message\":\"Rate limited\"}}"
            } else {
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\n\r\n{\"id\":\"gen-after-retry\",\"choices\":[{\"message\":{\"role\":\"assistant\",\"content\":\"\",\"images\":[{\"image_url\":\"data:image/png;base64,QUJD\"}]}}]}"
            };
            socket.write_all(response.as_bytes()).await.ok();
            // Close the connection so the body read ends at EOF.
            socket.shutdown().await.ok();
        }
    });
    (format!("http://{addr}"), bodies)
}

pub(crate) async fn spawn_hanging_body_images_server() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind hanging body server");
    let addr = listener.local_addr().expect("hanging server addr");
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept");
        let mut buf = vec![0u8; 8192];
        use tokio::io::AsyncReadExt;
        let _ = socket.read(&mut buf).await;
        use tokio::io::AsyncWriteExt;
        socket
            .write_all(
                b"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 1000\r\n\r\n{\"partial\":",
            )
            .await
            .expect("write partial body");
        tokio::time::sleep(Duration::from_secs(60)).await;
    });
    format!("http://{addr}")
}

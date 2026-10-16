use std::{env, fs, sync::Arc};

use serde_json::json;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args[1] == "rsa" {
        let der = fs::read(&args[3]).unwrap();
        let key = if args[2] == "pkcs8" {
            ring::rsa::KeyPair::from_pkcs8(&der)
        } else {
            ring::rsa::KeyPair::from_der(&der)
        };
        match key {
            Ok(key) => {
                let mut signature = vec![0; key.public().modulus_len()];
                let result = key.sign(
                    &ring::signature::RSA_PKCS1_SHA256,
                    &ring::rand::SystemRandom::new(),
                    b"opi-research-synthetic-rs256-input",
                    &mut signature,
                );
                println!("{}", json!({"accepted":result.is_ok(),"stage":"sign","signature_bytes":signature.len()}));
            }
            Err(error) => println!("{}", json!({"accepted":false,"stage":"key","error":format!("{error:?}")})),
        }
        return;
    }
    let profile = &args[1];
    let mut builder = reqwest::Client::builder().no_proxy().retry(reqwest::retry::never());
    if profile != "native-default-redirect" {
        builder = builder.redirect(reqwest::redirect::Policy::none());
    }
    {
        let mut roots = rustls::RootCertStore::empty();
        if let Some(ca_path) = args.get(5) {
            roots.add(rustls::pki_types::CertificateDer::from(fs::read(ca_path).unwrap())).unwrap();
        }
        let tls_builder = rustls::ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions().unwrap()
            .with_root_certificates(roots);
        let mut config = if let (Some(cert), Some(key)) = (args.get(6), args.get(7)) {
            tls_builder.with_client_auth_cert(
                vec![rustls::pki_types::CertificateDer::from(fs::read(cert).unwrap())],
                rustls::pki_types::PrivateKeyDer::Pkcs8(rustls::pki_types::PrivatePkcs8KeyDer::from(fs::read(key).unwrap()))
            ).unwrap()
        } else {
            tls_builder.with_no_client_auth()
        };
        config.alpn_protocols = if profile == "h1" {
            vec![b"http/1.1".to_vec()]
        } else {
            vec![b"h2".to_vec(), b"http/1.1".to_vec()]
        };
        builder = builder.use_preconfigured_tls(config);
    }
    if profile == "h1" { builder = builder.http1_only(); }
    let client = builder.build().unwrap();
    let mut request = client.request(args[2].parse().unwrap(), &args[3]);
    request = request.header("authorization", "Bearer SYNTHETIC-SECRET")
        .header("cookie", "synthetic-cookie=1")
        .header("x-opi-canary", "synthetic-public-canary");
    if args[2] != "GET" { request = request.body(args[4].clone()); }
    match request.send().await {
        Ok(response) => {
            let status = response.status().as_u16();
            let version = format!("{:?}", response.version());
            let final_url = response.url().to_string();
            let location = response.headers().get("location").and_then(|x| x.to_str().ok()).map(str::to_owned);
            let body = response.text().await.unwrap();
            println!("{}", json!({"ok":true,"status":status,"version":version,"final_url":final_url,"location":location,"body":body}));
        }
        Err(error) => println!("{}", json!({"ok":false,"error":format!("{error:?}")})),
    }
}

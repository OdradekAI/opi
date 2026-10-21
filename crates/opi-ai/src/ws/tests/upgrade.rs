//! Private codec supplements over a scripted, publicly upgraded transport.
//!
//! These retain protocol ownership, PMD/window and Close regressions. They do
//! not exercise a production provider path or establish external SDK conformance.

use crate::http::{HttpClient, HttpClientBuilder, RedirectPolicy};
use crate::ws::{self, PmdOffer, WsUpgradeRequest};
use std::io::{BufReader, Read, Write};
use std::net::TcpStream;
use url::Url;

use super::deflate_audit;

mod peer {
    include!("../../../tests/support/ws_peer.rs");
}
use peer::{Peer, response_101};

fn independent_messages(mut stream: TcpStream, expected: &[u8]) -> Result<(), String> {
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .unwrap();
    for message in 0..2 {
        let mut header = [0; 2];
        stream.read_exact(&mut header).map_err(|e| e.to_string())?;
        assert_eq!(header[0], 0xc2);
        assert_ne!(header[1] & 0x80, 0);
        let length = match header[1] & 0x7f {
            126 => {
                let mut length = [0; 2];
                stream.read_exact(&mut length).map_err(|e| e.to_string())?;
                u16::from_be_bytes(length) as usize
            }
            127 => return Err("unexpected large frame".into()),
            length => length as usize,
        };
        let mut mask = [0; 4];
        stream.read_exact(&mut mask).map_err(|e| e.to_string())?;
        let mut payload = vec![0; length];
        stream.read_exact(&mut payload).map_err(|e| e.to_string())?;
        for (i, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask[i % 4];
        }
        let decoded = crate::ws::pmd::Decoder::new(true).unwrap().decode(&payload);
        assert_eq!(
            decoded.unwrap_or_else(|e| panic!("message {message} needs forbidden history: {e}")),
            expected
        );
    }
    Ok(())
}

fn window8_messages(
    mut stream: TcpStream,
    expected: &[u8],
    no_context: bool,
) -> Result<(), String> {
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .unwrap();
    let mut history = Vec::new();
    for message in 0..2 {
        let mut head = [0; 2];
        stream.read_exact(&mut head).map_err(|e| e.to_string())?;
        assert_eq!(head[0], 0xc2);
        assert_ne!(head[1] & 0x80, 0);
        let length = match head[1] & 127 {
            126 => {
                let mut n = [0; 2];
                stream.read_exact(&mut n).unwrap();
                usize::from(u16::from_be_bytes(n))
            }
            127 => return Err("unexpected large frame".into()),
            n => usize::from(n),
        };
        let mut mask = [0; 4];
        stream.read_exact(&mut mask).unwrap();
        let mut payload = vec![0; length];
        stream.read_exact(&mut payload).unwrap();
        for (i, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask[i % 4];
        }
        if no_context {
            history.clear();
        }
        let (decoded, maximum) = deflate_audit::decode_pmd(&payload, &mut history, 8)?;
        assert_eq!(decoded, expected);
        assert!(
            (1..=256).contains(&maximum),
            "wire must exercise actual backreferences"
        );
        if !no_context && message == 1 {
            assert!(
                deflate_audit::decode_pmd(&payload, &mut Vec::new(), 8).is_err(),
                "takeover fixture must really depend on prior history"
            );
        }
    }
    Ok(())
}

#[tokio::test]
async fn upgrade_window8_wire_has_independent_decode_and_distance_proof() {
    let mut seed = 7_u32;
    let pattern: Vec<_> = (0..200)
        .map(|_| {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            seed as u8
        })
        .collect();
    let expected = pattern.repeat(257);
    for no_context in [false, true] {
        let inbound = expected.clone();
        let peer = Peer::spawn(
            {
                let expected = expected.clone();
                move |stream| window8_messages(stream, &expected, no_context)
            },
            move |head| {
                let flags = if no_context {
                    "; server_no_context_takeover; client_no_context_takeover"
                } else {
                    ""
                };
                let extension = format!(
                    "permessage-deflate; server_max_window_bits=8; client_max_window_bits=8{flags}"
                );
                let mut response = response_101(head, &[("Sec-WebSocket-Extensions", &extension)]);
                let mut encoder = crate::ws::pmd::Encoder::new(8, no_context).unwrap();
                let mut history = Vec::new();
                for _ in 0..2 {
                    let payload = encoder.encode(&inbound).unwrap();
                    if no_context {
                        history.clear();
                    }
                    let (decoded, maximum) =
                        deflate_audit::decode_pmd(&payload, &mut history, 8).unwrap();
                    assert_eq!(decoded, inbound);
                    assert!((1..=256).contains(&maximum));
                    if payload.len() < 126 {
                        response.extend_from_slice(&[0xc2, payload.len() as u8]);
                    } else {
                        response.extend_from_slice(&[0xc2, 126]);
                        response.extend_from_slice(
                            &u16::try_from(payload.len()).unwrap().to_be_bytes(),
                        );
                    }
                    response.extend_from_slice(&payload);
                }
                response
            },
        );
        let request = WsUpgradeRequest::new(peer.url()).permessage_deflate(PmdOffer {
            client_no_context_takeover: no_context,
            client_max_window_bits: Some(Some(8)),
        });
        let upgraded = ws::upgrade(&upgrade_client(), request).await.unwrap();
        let parameters = driver_parameters(&upgraded.negotiation).unwrap();
        assert_eq!(parameters.server_max_window_bits, 8);
        assert_eq!(parameters.client_max_window_bits, 8);
        let mut driver = crate::ws::io::Driver::new(upgraded.stream, Some(parameters)).unwrap();
        for _ in 0..2 {
            assert_eq!(driver.next().await.unwrap().unwrap().payload, expected);
            driver.queue(2, &expected, true, ws::fresh_mask()).unwrap();
            driver.flush().await.unwrap();
        }
        peer.finish().unwrap();
    }
}

#[tokio::test]
async fn upgrade_preserves_directional_context_and_delivers_coalesced_tail_once() {
    let expected = b"retained-history-window:".repeat(1000);
    let server_messages = expected.clone();
    let peer = Peer::spawn(
        {
            let expected = expected.clone();
            move |stream| independent_messages(stream, &expected)
        },
        move |head| {
            let mut response = response_101(
                head,
                &[(
                    "Sec-WebSocket-Extensions",
                    "permessage-deflate; client_no_context_takeover",
                )],
            );
            response.extend_from_slice(&[0x82, 4, b't', b'a', b'i', b'l']);
            let mut encoder = crate::ws::pmd::Encoder::new(15, false).unwrap();
            for _ in 0..2 {
                let payload = encoder.encode(&server_messages).unwrap();
                if payload.len() < 126 {
                    response.extend_from_slice(&[0xc2, payload.len() as u8]);
                } else {
                    response.extend_from_slice(&[0xc2, 126]);
                    response
                        .extend_from_slice(&u16::try_from(payload.len()).unwrap().to_be_bytes());
                }
                response.extend_from_slice(&payload);
            }
            response
        },
    );
    let client = upgrade_client();
    let upgraded = ws::upgrade(&client, bridge_request(peer.url(), true))
        .await
        .unwrap();
    let pmd = upgraded.negotiation.pmd.unwrap();
    assert!(pmd.client_no_context_takeover);
    assert!(!pmd.server_no_context_takeover);
    let parameters = driver_parameters(&upgraded.negotiation);
    let mut driver = crate::ws::io::Driver::new(upgraded.stream, parameters).unwrap();
    assert_eq!(driver.next().await.unwrap().unwrap().payload, b"tail");
    for _ in 0..2 {
        assert_eq!(driver.next().await.unwrap().unwrap().payload, expected);
    }
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(10), driver.next())
            .await
            .is_err()
    );
    for _ in 0..2 {
        driver.queue(2, &expected, true, ws::fresh_mask()).unwrap();
        driver.flush().await.unwrap();
    }
    peer.finish().unwrap();
}

/// Read one small client frame (< 126 payload bytes), assert the mask bit
/// and optional RSV1, then run the explicit Close exchange.
fn exchange(stream: TcpStream, expect_rsv1: bool) -> Result<(), String> {
    let mut stream = stream;
    let mut reader = BufReader::new(&mut stream);
    let mut header = [0_u8; 2];
    reader.read_exact(&mut header).map_err(|e| e.to_string())?;
    if header[1] & 0x80 == 0 {
        return Err("client frame was not masked".into());
    }
    if expect_rsv1 && header[0] & 0x40 == 0 {
        return Err("negotiated PMD frame missing RSV1".into());
    }
    if !expect_rsv1 && header[0] & 0x40 != 0 {
        return Err("unnegotiated PMD frame set RSV1".into());
    }
    let length = (header[1] & 0x7f) as usize;
    let mut rest = vec![0_u8; 4 + length];
    reader.read_exact(&mut rest).map_err(|e| e.to_string())?;
    let mask: [u8; 4] = rest[..4].try_into().unwrap();
    let payload = rest[4..]
        .iter()
        .enumerate()
        .map(|(i, b)| b ^ mask[i % 4])
        .collect::<Vec<_>>();
    if !payload.is_empty() && payload[0] == 0x00 {
        return Err("payload looks unmasked".into());
    }
    // Peer Close, then the masked client response, then write shutdown so
    // the client's finish_close observes a clean transport EOF.
    stream.write_all(&[0x88, 0]).map_err(|e| e.to_string())?;
    let mut response = [0_u8; 6];
    stream
        .read_exact(&mut response)
        .map_err(|e| e.to_string())?;
    if response[0] != 0x88 || response[1] != 0x80 {
        return Err("expected masked empty Close response".into());
    }
    stream
        .shutdown(std::net::Shutdown::Write)
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn upgrade_client() -> HttpClient {
    HttpClientBuilder::new()
        .redirects(RedirectPolicy::None)
        .build()
        .unwrap()
}

fn bridge_request(url: Url, offer: bool) -> WsUpgradeRequest {
    let mut request = WsUpgradeRequest::new(url).subprotocols(["chat.proto".to_owned()]);
    if offer {
        request = request.permessage_deflate(PmdOffer {
            client_no_context_takeover: true,
            client_max_window_bits: Some(Some(12)),
        });
    }
    request
}

fn driver_parameters(negotiation: &ws::Negotiation) -> Option<crate::ws::io::PmdParameters> {
    negotiation
        .driver_parameters()
        .map(|pmd| crate::ws::io::PmdParameters {
            server_no_context_takeover: pmd.server_no_context_takeover,
            client_no_context_takeover: pmd.client_no_context_takeover,
            server_max_window_bits: pmd.server_max_window_bits,
            client_max_window_bits: pmd.client_max_window_bits.unwrap_or(15),
        })
}

async fn drive_bridged_exchange(extensions: &str, expect_rsv1: bool) -> Result<(), String> {
    let extensions = extensions.to_owned();
    let peer = Peer::spawn(
        move |stream| exchange(stream, expect_rsv1),
        move |head| response_101(head, &[("Sec-WebSocket-Extensions", extensions.as_str())]),
    );
    let url = peer.url();
    let client = upgrade_client();
    let upgraded = ws::upgrade(&client, bridge_request(url, expect_rsv1))
        .await
        .map_err(|error| format!("upgrade failed: {error}"))?;
    let parameters = driver_parameters(&upgraded.negotiation);
    assert_eq!(parameters.is_some(), expect_rsv1);
    assert!(!parameters.is_some_and(|pmd| pmd.server_no_context_takeover));
    let mut driver = crate::ws::io::Driver::new(upgraded.stream, parameters)
        .map_err(|error| format!("driver failed: {error}"))?;
    driver
        .queue(1, "桥接 canary".as_bytes(), expect_rsv1, ws::fresh_mask())
        .map_err(|error| format!("queue failed: {error}"))?;
    driver
        .flush()
        .await
        .map_err(|error| format!("flush failed: {error}"))?;
    let event = driver
        .next()
        .await
        .map_err(|error| format!("next failed: {error}"))?
        .ok_or("connection ended before Close")?;
    assert_eq!(event.opcode, 8);
    driver
        .queue(8, &event.payload, false, ws::fresh_mask())
        .map_err(|error| format!("close queue failed: {error}"))?;
    driver
        .finish_close(None)
        .await
        .map_err(|error| format!("close failed: {error}"))?;
    peer.finish()
}

#[tokio::test]
async fn strict_upgrade_bridges_negotiated_pmd_into_the_neutral_connection() {
    drive_bridged_exchange(
        "permessage-deflate; client_no_context_takeover; client_max_window_bits=10",
        true,
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn strict_upgrade_bridges_plain_connections_without_pmd() {
    drive_bridged_exchange("", false).await.unwrap();
}

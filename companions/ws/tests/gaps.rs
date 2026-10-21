use opi_ws::{io::Driver, pmd};
use std::{
    future::{Future, poll_fn},
    task::Poll,
};
use tokio::io::AsyncWriteExt;
fn frame(first: u8, payload: &[u8]) -> Vec<u8> {
    let mut v = vec![first];
    if payload.len() < 126 {
        v.push(payload.len() as u8)
    } else {
        v.push(126);
        v.extend((payload.len() as u16).to_be_bytes())
    }
    v.extend(payload);
    v
}
#[tokio::test]
async fn peer_close_keeps_transport_for_explicit_response() {
    let (client, mut peer) = tokio::io::duplex(16);
    let mut d = Driver::new(client, None).unwrap();
    peer.write_all(&[0x88, 0]).await.unwrap();
    assert_eq!(d.next().await.unwrap().unwrap().opcode, 8);
    assert!(
        d.queue(8, &[], false, [1, 2, 3, 4]).is_ok(),
        "peer Close must leave response path available"
    );
}
#[tokio::test]
async fn large_inflate_yields_before_returning_whole_message() {
    let compressed = pmd::Encoder::new(15, false)
        .unwrap()
        .encode(&vec![b'Z'; 1 << 20])
        .unwrap();
    let wire = frame(0xc2, &compressed);
    let (client, mut peer) = tokio::io::duplex(wire.len() + 1);
    let mut d = Driver::new(client, Some(Default::default())).unwrap();
    peer.write_all(&wire).await.unwrap();
    let mut future = Box::pin(d.next());
    let pending = poll_fn(|cx| Poll::Ready(future.as_mut().poll(cx).is_pending())).await;
    assert!(
        pending,
        "a large fully readable compressed message must yield scheduling before completion"
    );
}

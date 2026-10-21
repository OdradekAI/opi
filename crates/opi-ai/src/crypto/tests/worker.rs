//! Blocking-worker ownership and cancellation between signing operations.

use super::super::{OwnedSigningKey, fixtures, verify, worker::spawn_sign_worker};
use super::load_pem;
use rustls::{SignatureAlgorithm, SignatureScheme};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::task::Context;
use std::time::Duration;
use tokio::sync::oneshot;

#[derive(Debug, Default)]
struct SigningGate {
    entered: tokio::sync::Notify,
    calls: AtomicUsize,
    released: std::sync::Mutex<bool>,
    ready: std::sync::Condvar,
}

// Release on unwind as well, so a failed assertion cannot strand a
// blocking signer while the Tokio runtime shuts down.
struct SigningControl(Arc<SigningGate>);

impl SigningControl {
    async fn wait_for_entry(&self) {
        tokio::time::timeout(Duration::from_secs(5), self.0.entered.notified())
            .await
            .expect("worker entered the synchronous signing call");
    }

    fn release(&self) {
        *self.0.released.lock().unwrap() = true;
        self.0.ready.notify_all();
    }
}

impl Drop for SigningControl {
    fn drop(&mut self) {
        self.release();
    }
}

#[derive(Debug)]
struct ControlledKey {
    key: Arc<dyn rustls::sign::SigningKey>,
    gate: Arc<SigningGate>,
}

impl rustls::sign::SigningKey for ControlledKey {
    fn choose_scheme(&self, offered: &[SignatureScheme]) -> Option<Box<dyn rustls::sign::Signer>> {
        self.key.choose_scheme(offered).map(|signer| {
            Box::new(ControlledSigner {
                signer,
                gate: self.gate.clone(),
            }) as Box<dyn rustls::sign::Signer>
        })
    }

    fn algorithm(&self) -> SignatureAlgorithm {
        self.key.algorithm()
    }
}

#[derive(Debug)]
struct ControlledSigner {
    signer: Box<dyn rustls::sign::Signer>,
    gate: Arc<SigningGate>,
}

impl rustls::sign::Signer for ControlledSigner {
    fn sign(&self, message: &[u8]) -> Result<Vec<u8>, rustls::Error> {
        self.gate.calls.fetch_add(1, Ordering::SeqCst);
        self.gate.entered.notify_one();
        let (released, _) = self
            .gate
            .ready
            .wait_timeout_while(
                self.gate.released.lock().unwrap(),
                Duration::from_secs(20),
                |released| !*released,
            )
            .unwrap();
        let was_released = *released;
        drop(released);
        assert!(
            was_released,
            "test must release the synchronous signing call"
        );
        self.signer.sign(message)
    }

    fn scheme(&self) -> SignatureScheme {
        self.signer.scheme()
    }
}

fn controlled_key() -> (OwnedSigningKey, SigningControl) {
    let mut key = load_pem(fixtures::RSA_2048_E65537_PKCS8_PEM).expect("rsa admitted");
    let gate = Arc::new(SigningGate::default());
    key.key = Arc::new(ControlledKey {
        key: key.key.clone(),
        gate: gate.clone(),
    });
    (key, SigningControl(gate))
}

#[tokio::test]
async fn worker_signing_publishes_a_verifiable_signature() {
    let key = load_pem(fixtures::RSA_2048_E65537_PKCS8_PEM).expect("rsa admitted");
    let signature = key
        .sign_on_worker(
            SignatureScheme::RSA_PKCS1_SHA256,
            fixtures::MESSAGE_BIN.to_vec(),
        )
        .await
        .expect("worker signing publishes after native completion");
    verify::verify_rsa_pkcs1_sha256(
        fixtures::RSA_2048_E65537_PUBLIC_PKCS1_DER,
        fixtures::MESSAGE_BIN,
        &signature,
    )
    .expect("published signature verifies independently");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn caller_drop_after_signer_entry_retains_key_until_worker_finishes() {
    let (key, control) = controlled_key();
    let weak = key.weak_key();
    let mut op = key.sign_on_worker(
        SignatureScheme::RSA_PKCS1_SHA256,
        fixtures::MESSAGE_BIN.to_vec(),
    );
    drop(key);
    let waker = futures_util::task::noop_waker();
    let mut cx = Context::from_waker(&waker);
    assert!(std::pin::Pin::new(&mut op).poll(&mut cx).is_pending());
    control.wait_for_entry().await;
    drop(op);
    assert!(
        weak.upgrade().is_some(),
        "caller drop must retain the key while the synchronous call is blocked"
    );
    control.release();
    tokio::time::timeout(Duration::from_secs(5), async {
        while weak.upgrade().is_some() {
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
    })
    .await
    .expect("worker joins after caller cancellation");
    assert_eq!(control.0.calls.load(Ordering::SeqCst), 1);
    // The live-receiver case below separately distinguishes suppression
    // from a send that merely fails because the caller dropped its receiver.
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancellation_before_the_native_call_skips_it_entirely() {
    let (key, control) = controlled_key();
    let weak = key.weak_key();
    let cancelled = std::sync::Arc::new(AtomicBool::new(true));
    let (tx, mut rx) = oneshot::channel();
    let worker = spawn_sign_worker(
        key.key.clone(),
        SignatureScheme::RSA_PKCS1_SHA256,
        fixtures::MESSAGE_BIN.to_vec(),
        cancelled,
        tx,
    );
    drop(key);
    tokio::time::timeout(Duration::from_secs(5), worker)
        .await
        .expect("cancelled worker finishes without entering the signer")
        .expect("worker joins");
    assert_eq!(control.0.calls.load(Ordering::SeqCst), 0);
    assert!(
        matches!(rx.try_recv(), Err(oneshot::error::TryRecvError::Closed)),
        "cancelled worker sends nothing"
    );
    assert!(weak.upgrade().is_none(), "owned key released after join");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancellation_after_signer_entry_suppresses_result_for_a_live_receiver() {
    let (key, control) = controlled_key();
    let weak = key.weak_key();
    let cancelled = Arc::new(AtomicBool::new(false));
    let (tx, mut rx) = oneshot::channel();
    let worker = spawn_sign_worker(
        key.key.clone(),
        SignatureScheme::RSA_PKCS1_SHA256,
        fixtures::MESSAGE_BIN.to_vec(),
        cancelled.clone(),
        tx,
    );
    drop(key);
    control.wait_for_entry().await;
    cancelled.store(true, Ordering::SeqCst);
    assert!(weak.upgrade().is_some(), "in-flight worker owns the key");
    assert!(matches!(
        rx.try_recv(),
        Err(oneshot::error::TryRecvError::Empty)
    ));
    control.release();
    tokio::time::timeout(Duration::from_secs(5), worker)
        .await
        .expect("synchronous call completes after release")
        .expect("worker joins");
    assert_eq!(control.0.calls.load(Ordering::SeqCst), 1);
    assert!(
        matches!(rx.try_recv(), Err(oneshot::error::TryRecvError::Closed)),
        "a completed signature must not be published after cancellation"
    );
    assert!(weak.upgrade().is_none(), "joined worker releases its key");
}

#[test]
fn dropping_an_unpolled_signing_future_never_enters_the_signer() {
    let (key, control) = controlled_key();
    let weak = key.weak_key();
    let op = key.sign_on_worker(
        SignatureScheme::RSA_PKCS1_SHA256,
        fixtures::MESSAGE_BIN.to_vec(),
    );
    drop(key);
    drop(op);
    assert_eq!(control.0.calls.load(Ordering::SeqCst), 0);
    assert!(weak.upgrade().is_none());
}

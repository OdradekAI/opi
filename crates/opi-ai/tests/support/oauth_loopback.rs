use std::net::TcpListener;
use std::time::Duration;

pub static LOOPBACK_1455_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub async fn loopback_1455_guard() -> tokio::sync::MutexGuard<'static, ()> {
    LOOPBACK_1455_LOCK.lock().await
}

pub async fn occupy_1455_retrying() -> TcpListener {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            match TcpListener::bind("127.0.0.1:1455") {
                Ok(listener) => return listener,
                Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
                Err(error) => panic!("occupy 1455 fixture: {error}"),
            }
        }
    })
    .await
    .expect("parallel fixtures must release port 1455")
}

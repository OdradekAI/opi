//! The `opi-sandbox` standalone binary entry point (Phase 16 task 16.11.2).
//!
//! It reads native `argv`, owns the process-only stdin bridge for `backend
//! --stdio`, dispatches other commands through the library [`cli`] module, and
//! exits with the mapped code. The backend branch exits directly after its
//! flushed terminal result so non-abortable blocking workers cannot extend the
//! process lifetime through Tokio runtime shutdown. The binary is
//! dependency-neutral (no `opi` access, no durable state); the standalone smoke
//! suite proves that in isolation.

use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, ReadBuf};

const PROCESS_INPUT_CHANNEL_CAPACITY: usize = 8;

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let args: Vec<std::ffi::OsString> = std::env::args_os().collect();
    let code = if is_backend_stdio(&args) {
        let code = match process_stdin_reader() {
            Ok(stdin) => opi_sandbox::backend::run(Box::pin(stdin)).await,
            Err(_) => 1,
        };
        exit_backend_process(code)
    } else {
        opi_sandbox::cli::run(args).await
    };
    // Every CLI code (target exits 0-255, the reserved 2/124/125/130, and the
    // Unix 128+signal mapping up to 159) fits a byte; truncation matches the OS
    // exit-code convention.
    std::process::ExitCode::from(code as u8)
}

/// The process-only backend may leave non-abortable blocking validation or
/// restriction workers behind after its hard request deadline. Its terminal
/// frame has already been flushed when `backend::run` returns, so exit without
/// dropping the Tokio runtime (whose normal shutdown waits for those workers).
fn exit_backend_process(code: i32) -> ! {
    std::process::exit(code)
}

fn is_backend_stdio(args: &[std::ffi::OsString]) -> bool {
    args.len() == 3 && args[1] == "backend" && args[2] == "--stdio"
}

fn process_stdin_reader() -> std::io::Result<ProcessStdinReader> {
    let (tx, rx) = tokio::sync::mpsc::channel(PROCESS_INPUT_CHANNEL_CAPACITY);
    std::thread::Builder::new()
        .name("opi-sandbox-stdin".to_string())
        .spawn(move || {
            use std::io::Read as _;

            let stdin = std::io::stdin();
            let mut stdin = stdin.lock();
            let mut chunk = [0u8; 8192];
            loop {
                match stdin.read(&mut chunk) {
                    Ok(0) | Err(_) => return,
                    Ok(read) => {
                        if tx.blocking_send(chunk[..read].to_vec()).is_err() {
                            return;
                        }
                    }
                }
            }
        })?;
    Ok(ProcessStdinReader {
        rx,
        current: Vec::new(),
        offset: 0,
    })
}

struct ProcessStdinReader {
    rx: tokio::sync::mpsc::Receiver<Vec<u8>>,
    current: Vec<u8>,
    offset: usize,
}

impl AsyncRead for ProcessStdinReader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        loop {
            if self.offset < self.current.len() {
                let available = &self.current[self.offset..];
                let read = available.len().min(output.remaining());
                output.put_slice(&available[..read]);
                self.offset += read;
                return Poll::Ready(Ok(()));
            }
            match self.rx.poll_recv(cx) {
                Poll::Ready(Some(chunk)) => {
                    self.current = chunk;
                    self.offset = 0;
                }
                Poll::Ready(None) => return Poll::Ready(Ok(())),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn backend_stdio_path_bypasses_runtime_shutdown_waits() {
        let source = include_str!("main.rs");
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("production source");
        assert!(production.contains("exit_backend_process(code)"));
        assert!(production.contains("fn exit_backend_process(code: i32) -> !"));
        assert!(production.contains("std::process::exit(code)"));
    }

    #[test]
    fn process_exit_is_bounded_with_a_non_abortable_blocking_worker() {
        const SENTINEL: &str = "opi-sandbox-process-exit-child";
        if std::path::Path::new(SENTINEL).is_file() {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("child runtime");
            runtime.spawn_blocking(|| {
                loop {
                    std::thread::park();
                }
            });
            runtime.block_on(tokio::task::yield_now());
            super::exit_backend_process(23);
        }

        let child_dir = tempfile::tempdir().expect("child cwd");
        std::fs::write(child_dir.path().join(SENTINEL), b"child").expect("write sentinel");
        let mut child = std::process::Command::new(std::env::current_exe().expect("test binary"))
            .args([
                "--exact",
                "tests::process_exit_is_bounded_with_a_non_abortable_blocking_worker",
                "--nocapture",
            ])
            .current_dir(child_dir.path())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("spawn child test");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        let status = loop {
            if let Some(status) = child.try_wait().expect("poll child") {
                break status;
            }
            if std::time::Instant::now() >= deadline {
                child.kill().expect("kill hung child");
                let _ = child.wait();
                panic!("process exit waited for a non-abortable blocking worker");
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        assert_eq!(status.code(), Some(23));
    }
}

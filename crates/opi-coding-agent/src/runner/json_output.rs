//! Run-scoped NDJSON output with acknowledged writes and cancellable backpressure.

use std::io::{self, Write};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use tokio_util::sync::CancellationToken;

use super::{ExitCode, NonInteractiveResult};

const CANCEL_POLL: Duration = Duration::from_millis(10);

#[derive(Clone, Copy)]
enum Failure {
    Io(io::ErrorKind),
    Cancelled,
}

struct Line {
    bytes: String,
    acknowledged: SyncSender<Result<(), io::ErrorKind>>,
}

struct Writer {
    sender: Option<SyncSender<Line>>,
    worker: Option<JoinHandle<()>>,
}

impl Writer {
    fn new(mut output: impl Write + Send + 'static) -> io::Result<Self> {
        // Every producer waits for its flush acknowledgement before sending
        // again. One slot avoids scheduling latency at the rendezvous without
        // allowing more than a single outstanding line.
        let (sender, receiver) = mpsc::sync_channel::<Line>(1);
        let worker = thread::Builder::new()
            .name("opi-json-output".into())
            .spawn(move || {
                while let Ok(line) = receiver.recv() {
                    let result = output
                        .write_all(line.bytes.as_bytes())
                        .and_then(|()| output.flush())
                        .map_err(|error| error.kind());
                    // Capacity one lets the writer finish even if cancellation
                    // has already caused the producer to drop its receiver.
                    let _ = line.acknowledged.send(result);
                    if result.is_err() {
                        break;
                    }
                }
            })?;
        Ok(Self {
            sender: Some(sender),
            worker: Some(worker),
        })
    }

    fn line(&self, bytes: String, cancel: &CancellationToken) -> Result<(), Failure> {
        let (acknowledged, receiver) = mpsc::sync_channel(1);
        let mut line = Line {
            bytes,
            acknowledged,
        };
        loop {
            match self.sender.as_ref().expect("active output").try_send(line) {
                Ok(()) => break,
                Err(TrySendError::Disconnected(_)) => {
                    return Err(Failure::Io(io::ErrorKind::BrokenPipe));
                }
                Err(TrySendError::Full(pending)) => line = pending,
            }
            if cancel.is_cancelled() {
                return Err(Failure::Cancelled);
            }
            thread::sleep(CANCEL_POLL);
        }
        Self::wait_for_ack(receiver, cancel)
    }

    fn wait_for_ack(
        receiver: Receiver<Result<(), io::ErrorKind>>,
        cancel: &CancellationToken,
    ) -> Result<(), Failure> {
        loop {
            // Prefer a reported write failure to the cancellation it causes.
            match receiver.recv_timeout(CANCEL_POLL) {
                Ok(result) => return result.map_err(Failure::Io),
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(Failure::Io(io::ErrorKind::BrokenPipe));
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
            if cancel.is_cancelled() {
                return Err(Failure::Cancelled);
            }
        }
    }

    fn finish(&mut self, interrupted: bool) {
        self.sender.take();
        if let Some(worker) = self.worker.take() {
            // On interruption an OS write may still be blocked. The CLI exits
            // after Agent cleanup; do not turn that interruption into a join hang.
            if !interrupted {
                let _ = worker.join();
            }
        }
    }
}

enum Target {
    Capture(String),
    Stream(Writer),
}

pub(super) struct JsonOutput {
    target: Target,
    failure: Option<Failure>,
    cancel: CancellationToken,
    finished: bool,
}

impl JsonOutput {
    pub(super) fn capture(cancel: CancellationToken) -> Self {
        Self {
            target: Target::Capture(String::new()),
            failure: None,
            cancel,
            finished: false,
        }
    }

    pub(super) fn stream(writer: impl Write + Send + 'static, cancel: CancellationToken) -> Self {
        match Writer::new(writer) {
            Ok(writer) => Self {
                target: Target::Stream(writer),
                failure: None,
                cancel,
                finished: false,
            },
            Err(error) => {
                cancel.cancel();
                Self {
                    target: Target::Capture(String::new()),
                    failure: Some(Failure::Io(error.kind())),
                    cancel,
                    finished: false,
                }
            }
        }
    }

    pub(super) fn line(&mut self, mut json: String) {
        if self.finished || self.failure.is_some() {
            return;
        }
        json.push('\n');
        let result = match &mut self.target {
            Target::Capture(buffer) => {
                buffer.push_str(&json);
                Ok(())
            }
            Target::Stream(writer) => writer.line(json, &self.cancel),
        };
        if let Err(error) = result {
            self.failure = Some(error);
            self.cancel.cancel();
        }
    }

    pub(super) fn complete(&mut self, mut result: NonInteractiveResult) -> NonInteractiveResult {
        // A callback that upgraded its Weak reference just before completion
        // can still arrive after the writer has closed.
        self.finished = true;
        match &mut self.target {
            Target::Capture(buffer) => result.stdout = std::mem::take(buffer),
            Target::Stream(writer) => {
                writer.finish(matches!(self.failure, Some(Failure::Cancelled)))
            }
        }
        if let Some(failure) = self.failure {
            let (code, message) = match failure {
                Failure::Io(kind) => (
                    ExitCode::RuntimeFailure,
                    format!("opi: JSON output failed ({kind:?})"),
                ),
                Failure::Cancelled => {
                    (ExitCode::Interrupted, "opi: JSON output interrupted".into())
                }
            };
            if !result.stderr.is_empty() {
                result.stderr.push('\n');
            }
            result.stderr.push_str(&message);
            result.exit_code = code as i32;
        }
        result
    }
}

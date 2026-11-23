//! Scripted AWS source transport for Bedrock credential fixtures.

use std::collections::VecDeque;
use std::sync::Mutex;

use opi_ai::auth::aws::source_http::{
    AwsSourceTransport, SourceHttpError, SourceHttpRequest, SourceHttpResponse,
};

/// Scriptable fake for the AWS auth-source HTTP transport.
///
/// Records sends and replays scripted responses in order. An exhausted script
/// returns a transport error. Debug output contains only the send count.
#[derive(Default)]
pub(super) struct FakeSourceTransport {
    sends: Mutex<Vec<SourceHttpRequest>>,
    responses: Mutex<VecDeque<Result<SourceHttpResponse, SourceHttpError>>>,
}

impl FakeSourceTransport {
    /// Script the responses replayed in send order.
    pub(super) fn new(responses: Vec<Result<SourceHttpResponse, SourceHttpError>>) -> Self {
        Self {
            responses: Mutex::new(responses.into_iter().collect()),
            ..Self::default()
        }
    }

    /// A transport that fails every send with a transport error.
    pub(super) fn default_never() -> Self {
        Self::default()
    }

    /// Number of sends observed so far.
    pub(super) fn send_count(&self) -> usize {
        self.sends.lock().expect("send log").len()
    }

    /// Return the recorded send at an existing index.
    pub(super) fn send_at(&self, index: usize) -> SourceHttpRequest {
        self.sends.lock().expect("send log")[index].clone()
    }

    /// A snapshot of all recorded sends.
    pub(super) fn sends_snapshot(&self) -> Vec<SourceHttpRequest> {
        self.sends.lock().expect("send log").clone()
    }
}

impl AwsSourceTransport for FakeSourceTransport {
    fn send(
        &self,
        request: SourceHttpRequest,
    ) -> futures_util::future::BoxFuture<'_, Result<SourceHttpResponse, SourceHttpError>> {
        use futures_util::FutureExt as _;
        self.sends.lock().expect("send log").push(request);
        let response = self
            .responses
            .lock()
            .expect("response queue")
            .pop_front()
            .unwrap_or(Err(SourceHttpError::Transport));
        async move { response }.boxed()
    }
}

impl std::fmt::Debug for FakeSourceTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FakeSourceTransport")
            .field("sends", &self.send_count())
            .finish()
    }
}

/// A source transport that always fails (for fixtures that must never
/// perform a real exchange).
pub(super) struct NeverTransport;

impl AwsSourceTransport for NeverTransport {
    fn send(
        &self,
        _request: SourceHttpRequest,
    ) -> futures_util::future::BoxFuture<'_, Result<SourceHttpResponse, SourceHttpError>> {
        use futures_util::FutureExt as _;
        async { Err(SourceHttpError::Transport) }.boxed()
    }
}

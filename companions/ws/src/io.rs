//! Borrowed operations retain all partial frame, PMD, write and Close state in this owner.
//! Caller queues every masked Close; only `finish_close` drives transport shutdown.
use crate::{frame, pmd};
use std::{
    collections::VecDeque,
    fmt,
    future::poll_fn,
    io,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

#[derive(Debug)]
pub enum Fault {
    Frame(frame::Fault),
    Pmd(pmd::Fault),
    Io(io::Error),
    Closed,
    Busy,
    CloseNotStarted,
    CloseResponseRequired,
    CloseReceivePending,
    CloseTimeout,
    PeerEofBeforeCloseSent,
}

impl fmt::Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Frame(e) => fmt::Display::fmt(e, f),
            Self::Pmd(e) => fmt::Display::fmt(e, f),
            Self::Io(e) => fmt::Display::fmt(e, f),
            Self::Closed => f.write_str("WebSocket owner is terminal or closing"),
            Self::Busy => f.write_str("outgoing frame is pending"),
            Self::CloseNotStarted => f.write_str("caller has not queued a Close"),
            Self::CloseResponseRequired => f.write_str("caller must queue a masked Close response"),
            Self::CloseReceivePending => {
                f.write_str("drain the retained receive event before finishing Close")
            }
            Self::CloseTimeout => f.write_str("caller Close deadline elapsed"),
            Self::PeerEofBeforeCloseSent => f.write_str("peer EOF before our Close was sent"),
        }
    }
}
impl std::error::Error for Fault {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Frame(e) => Some(e),
            Self::Pmd(e) => Some(e),
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}
impl From<frame::Fault> for Fault {
    fn from(e: frame::Fault) -> Self {
        Self::Frame(e)
    }
}
impl From<pmd::Fault> for Fault {
    fn from(e: pmd::Fault) -> Self {
        Self::Pmd(e)
    }
}
impl From<io::Error> for Fault {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

enum Pending {
    Inflate(frame::Message),
    Text {
        message: frame::Message,
        offset: usize,
    },
}

pub struct Core {
    frame: Option<frame::Decoder>,
    messages: Option<frame::Messages>,
    pmd: Option<pmd::Decoder>,
    pending: Option<Pending>,
    terminal: bool,
    closed: bool,
}
impl Core {
    pub fn new(pmd: bool, no_context: bool) -> Result<Self, Fault> {
        Self::with_parameters(pmd.then_some(PmdParameters {
            server_no_context_takeover: no_context,
            ..PmdParameters::default()
        }))
    }
    fn with_parameters(pmd: Option<PmdParameters>) -> Result<Self, Fault> {
        Ok(Self {
            frame: Some(frame::Decoder::new(pmd.is_some())),
            messages: Some(frame::Messages::new_deferred_text()),
            pmd: pmd
                .map(|parameters| {
                    pmd::Decoder::with_window_bits(
                        parameters.server_max_window_bits,
                        parameters.server_no_context_takeover,
                    )
                })
                .transpose()?,
            pending: None,
            terminal: false,
            closed: false,
        })
    }
    /// Historical synchronous mechanism API. The async owner uses `feed_byte`/`poll_pending`.
    pub fn ingest(&mut self, bytes: &[u8], out: &mut Vec<frame::Message>) -> Result<(), Fault> {
        if self.terminal {
            return Err(Fault::Closed);
        }
        let result = self.ingest_inner(bytes, out);
        if result.is_err() {
            self.release();
        }
        result
    }
    fn ingest_inner(&mut self, bytes: &[u8], out: &mut Vec<frame::Message>) -> Result<(), Fault> {
        for byte in bytes {
            if let Some(mut message) = self.receive_byte(*byte, false)? {
                if message.compressed {
                    message.payload = self
                        .pmd
                        .as_mut()
                        .ok_or(Fault::Closed)?
                        .decode(&message.payload)?;
                    message.compressed = false;
                }
                if message.opcode == 1 {
                    std::str::from_utf8(&message.payload).map_err(|_| frame::Fault::Utf8)?;
                }
                out.try_reserve(1).map_err(frame::Fault::from)?;
                let close = message.opcode == 8;
                out.push(message);
                if close {
                    self.closed = true;
                    self.release();
                    return Ok(());
                }
            }
        }
        Ok(())
    }
    fn receive_byte(
        &mut self,
        byte: u8,
        defer_append: bool,
    ) -> Result<Option<frame::Message>, Fault> {
        let mut frames = Vec::new();
        self.frame
            .as_mut()
            .ok_or(Fault::Closed)?
            .feed_into(&[byte], &mut frames)?;
        match frames.pop() {
            Some(frame) => {
                let messages = self.messages.as_mut().ok_or(Fault::Closed)?;
                Ok(if defer_append {
                    messages.receive_deferred(frame)?
                } else {
                    messages.receive(frame)?
                })
            }
            None => Ok(None),
        }
    }
    fn feed_byte(&mut self, byte: u8) -> Result<Option<frame::Message>, Fault> {
        let Some(message) = self.receive_byte(byte, true)? else {
            return Ok(None);
        };
        self.begin_message(message)
    }
    fn begin_message(
        &mut self,
        mut message: frame::Message,
    ) -> Result<Option<frame::Message>, Fault> {
        if message.compressed {
            let payload = std::mem::take(&mut message.payload);
            self.pmd
                .as_mut()
                .ok_or(Fault::Closed)?
                .start_decode(payload, None)?;
            self.pending = Some(Pending::Inflate(message));
            Ok(None)
        } else if message.opcode == 1 {
            self.pending = Some(Pending::Text { message, offset: 0 });
            Ok(None)
        } else {
            if message.opcode == 8 {
                self.closed = true;
            }
            Ok(Some(message))
        }
    }
    fn poll_pending(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<Option<frame::Message>, Fault>> {
        if self.pending.is_none() {
            let appended = match self.messages.as_mut().ok_or(Fault::Closed)?.poll_append(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(result) => result?,
            };
            if let Some(message) = appended {
                if let Some(message) = self.begin_message(message)? {
                    return Poll::Ready(Ok(Some(message)));
                }
            } else {
                return Poll::Ready(Ok(None));
            }
        }
        if matches!(self.pending, Some(Pending::Inflate(_))) {
            let payload = match self
                .pmd
                .as_mut()
                .expect("pending PMD decoder")
                .poll_decode(cx)
            {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(result) => result?,
            };
            let Some(Pending::Inflate(mut message)) = self.pending.take() else {
                unreachable!()
            };
            message.payload = payload;
            message.compressed = false;
            if message.opcode != 1 {
                return Poll::Ready(Ok(Some(message)));
            }
            self.pending = Some(Pending::Text { message, offset: 0 });
        }
        let Some(Pending::Text { message, offset }) = self.pending.as_mut() else {
            unreachable!()
        };
        let end = offset.saturating_add(8192).min(message.payload.len());
        match std::str::from_utf8(&message.payload[*offset..end]) {
            Ok(_) => *offset = end,
            // The incomplete prefix (at most three bytes) stays at the cursor
            // and is retried together with the next quantum. Invalid complete
            // sequences or an incomplete final sequence fail before publication.
            Err(error) if error.error_len().is_none() && end < message.payload.len() => {
                *offset += error.valid_up_to();
            }
            Err(_) => return Poll::Ready(Err(frame::Fault::Utf8.into())),
        }
        if *offset < message.payload.len() {
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        let Some(Pending::Text { message, .. }) = self.pending.take() else {
            unreachable!()
        };
        Poll::Ready(Ok(Some(message)))
    }

    pub fn eof(&mut self) -> Result<(), Fault> {
        if self.closed {
            return Ok(());
        }
        if self.terminal {
            return Err(Fault::Closed);
        }
        let result = (|| {
            self.frame.as_mut().ok_or(Fault::Closed)?.eof()?;
            self.messages.as_mut().ok_or(Fault::Closed)?.eof()?;
            Err(Fault::Frame(frame::Fault::Protocol(
                "transport EOF before Close",
            )))
        })();
        self.release();
        result
    }
    fn release(&mut self) {
        self.terminal = true;
        self.frame = None;
        self.messages = None;
        self.pending = None;
        if let Some(decoder) = self.pmd.as_mut() {
            decoder.cancel();
        }
        self.pmd = None;
    }
    pub fn is_closed(&self) -> bool {
        self.closed
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseState {
    Open,
    LocalCloseQueued,
    LocalCloseSent,
    PeerCloseReceived,
    ResponseQueued,
    HandshakeComplete,
    Closed,
    Failed,
}

/// Validated client-side PMD negotiation facts, named by their RFC direction.
/// The server fields configure reception; the client fields configure sending.
/// Both windows must be in 8..=15; omitted RFC window parameters mean 15.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PmdParameters {
    pub server_no_context_takeover: bool,
    pub client_no_context_takeover: bool,
    pub server_max_window_bits: u8,
    pub client_max_window_bits: u8,
}

impl Default for PmdParameters {
    fn default() -> Self {
        Self {
            server_no_context_takeover: false,
            client_no_context_takeover: false,
            server_max_window_bits: 15,
            client_max_window_bits: 15,
        }
    }
}

pub struct Driver<S> {
    stream: Option<S>,
    core: Option<Core>,
    encoder: Option<pmd::Encoder>,
    pmd_enabled: bool,
    read: [u8; 4096],
    read_count: usize,
    read_offset: usize,
    ready: VecDeque<frame::Message>,
    write: Vec<u8>,
    write_offset: usize,
    close_queued: bool,
    close_sent: bool,
    close_bytes_written: bool,
    peer_close: bool,
    write_shutdown: bool,
    terminal: Option<bool>,
}
impl<S: AsyncRead + AsyncWrite + Unpin> Driver<S> {
    /// Take an upgraded stream and direction-specific PMD facts (`None` disables PMD).
    pub fn new(stream: S, pmd: Option<PmdParameters>) -> Result<Self, Fault> {
        Ok(Self {
            stream: Some(stream),
            core: Some(Core::with_parameters(pmd)?),
            encoder: pmd
                .map(|parameters| {
                    pmd::Encoder::new(
                        parameters.client_max_window_bits,
                        parameters.client_no_context_takeover,
                    )
                })
                .transpose()?,
            pmd_enabled: pmd.is_some(),
            read: [0; 4096],
            read_count: 0,
            read_offset: 0,
            ready: VecDeque::new(),
            write: Vec::new(),
            write_offset: 0,
            close_queued: false,
            close_sent: false,
            close_bytes_written: false,
            peer_close: false,
            write_shutdown: false,
            terminal: None,
        })
    }
    pub fn close_state(&self) -> CloseState {
        match (
            self.terminal,
            self.peer_close,
            self.close_queued,
            self.close_sent,
        ) {
            (Some(true), _, _, _) => CloseState::Closed,
            (Some(false), _, _, _) => CloseState::Failed,
            (_, true, _, true) => CloseState::HandshakeComplete,
            (_, true, true, false) => CloseState::ResponseQueued,
            (_, true, false, _) => CloseState::PeerCloseReceived,
            (_, false, _, true) => CloseState::LocalCloseSent,
            (_, false, true, false) => CloseState::LocalCloseQueued,
            _ => CloseState::Open,
        }
    }
    /// Cancellation drops only the borrow; read offsets and PMD job stay owned.
    /// Peer Close ends receive delivery but keeps the transport until `finish_close`.
    pub async fn next(&mut self) -> Result<Option<frame::Message>, Fault> {
        if let Some(message) = self.ready.pop_front() {
            return Ok(Some(message));
        }
        poll_fn(|cx| self.poll_event(cx)).await
    }
    fn poll_event(&mut self, cx: &mut Context<'_>) -> Poll<Result<Option<frame::Message>, Fault>> {
        if self.terminal.is_some() || self.peer_close {
            return Poll::Ready(Ok(None));
        }
        let mut spent = 0;
        loop {
            let decoded = self.core.as_mut().expect("live core").poll_pending(cx);
            match decoded {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Err(error)) => {
                    self.release(false);
                    return Poll::Ready(Err(error));
                }
                Poll::Ready(Ok(Some(message))) => return Poll::Ready(Ok(Some(message))),
                Poll::Ready(Ok(None)) => {}
            }
            if spent >= 2048 {
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
            if self.read_offset == self.read_count {
                let mut read = ReadBuf::new(&mut self.read);
                match Pin::new(self.stream.as_mut().expect("live stream")).poll_read(cx, &mut read)
                {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(Err(error)) => {
                        self.release(false);
                        return Poll::Ready(Err(error.into()));
                    }
                    Poll::Ready(Ok(())) => {
                        self.read_count = read.filled().len();
                        self.read_offset = 0;
                        if self.read_count == 0 {
                            let result = self.core.as_mut().expect("live core").eof();
                            self.release(false);
                            return Poll::Ready(result.map(|()| None));
                        }
                    }
                }
            }
            let byte = self.read[self.read_offset];
            self.read_offset += 1;
            spent += 1;
            match self.core.as_mut().expect("live core").feed_byte(byte) {
                Err(error) => {
                    self.release(false);
                    return Poll::Ready(Err(error));
                }
                Ok(Some(message)) => {
                    if message.opcode == 8 {
                        self.peer_close = true;
                        // Unsent application bytes may be abandoned; a partially written frame
                        // must finish before a caller-queued Close to preserve wire framing.
                        if !self.close_queued && self.write_offset == 0 {
                            self.write.clear();
                        }
                    }
                    return Poll::Ready(Ok(Some(message)));
                }
                Ok(None) => {}
            }
        }
    }
    /// Caller supplies a fresh unpredictable mask. A peer response is admitted exactly once.
    pub fn queue(
        &mut self,
        opcode: u8,
        payload: &[u8],
        compress: bool,
        mask: [u8; 4],
    ) -> Result<(), Fault> {
        if self.terminal.is_some() || self.close_queued || (self.peer_close && opcode != 8) {
            return Err(Fault::Closed);
        }
        if !self.write.is_empty() && !(self.peer_close && opcode == 8) {
            return Err(Fault::Busy);
        }
        let result = self.encode(opcode, payload, compress, mask);
        match result {
            Ok(bytes) => {
                if let Err(error) = frame::checked_storage_len(self.write.len(), bytes.len())
                    .and_then(|_| {
                        self.write
                            .try_reserve(bytes.len())
                            .map_err(frame::Fault::from)
                    })
                {
                    self.release(false);
                    return Err(error.into());
                }
                self.write.extend_from_slice(&bytes);
                if opcode == 8 {
                    self.close_queued = true;
                }
                Ok(())
            }
            Err(error) => {
                self.release(false);
                Err(error)
            }
        }
    }
    fn encode(
        &mut self,
        opcode: u8,
        payload: &[u8],
        compress: bool,
        mask: [u8; 4],
    ) -> Result<Vec<u8>, Fault> {
        if !matches!(opcode, 1 | 2 | 8 | 9 | 10)
            || (opcode >= 8 && (compress || payload.len() > 125))
        {
            return Err(frame::Fault::Protocol("invalid queued message").into());
        }
        if opcode == 1 {
            std::str::from_utf8(payload).map_err(|_| frame::Fault::Utf8)?;
        }
        if opcode == 8 {
            frame::Messages::new().receive(frame::Frame {
                fin: true,
                opcode,
                compressed: false,
                payload: payload.to_vec(),
            })?;
        }
        let encoded;
        let body = if compress {
            if !self.pmd_enabled {
                return Err(frame::Fault::Protocol("PMD was not negotiated").into());
            }
            encoded = self
                .encoder
                .as_mut()
                .ok_or(Fault::Closed)?
                .encode(payload)?;
            encoded.as_slice()
        } else {
            payload
        };
        let extra = frame::checked_storage_len(body.len(), 14)?;
        let mut bytes = Vec::new();
        bytes.try_reserve(extra).map_err(frame::Fault::from)?;
        bytes.push(0x80 | if compress { 0x40 } else { 0 } | opcode);
        match body.len() {
            0..=125 => bytes.push(0x80 | body.len() as u8),
            126..=65535 => {
                bytes.push(0xfe);
                bytes.extend_from_slice(&(body.len() as u16).to_be_bytes());
            }
            _ => {
                bytes.push(0xff);
                bytes.extend_from_slice(&(body.len() as u64).to_be_bytes());
            }
        }
        bytes.extend_from_slice(&mask);
        bytes.extend(body.iter().enumerate().map(|(i, b)| *b ^ mask[i % 4]));
        Ok(bytes)
    }
    pub async fn flush(&mut self) -> Result<(), Fault> {
        poll_fn(|cx| self.poll_flush(cx)).await
    }
    fn poll_flush(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Fault>> {
        if self.terminal.is_some() {
            return Poll::Ready(Err(Fault::Closed));
        }
        let mut spent = 0;
        while self.write_offset < self.write.len() {
            if spent >= 2048 {
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
            let end = (self.write_offset + 2048).min(self.write.len());
            match Pin::new(self.stream.as_mut().expect("live stream"))
                .poll_write(cx, &self.write[self.write_offset..end])
            {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Ok(0)) => {
                    self.release(false);
                    return Poll::Ready(Err(io::Error::from(io::ErrorKind::WriteZero).into()));
                }
                Poll::Ready(Ok(n)) => {
                    self.write_offset += n;
                    spent += n;
                }
                Poll::Ready(Err(error)) => {
                    self.release(false);
                    return Poll::Ready(Err(error.into()));
                }
            }
        }
        if self.close_queued {
            self.close_bytes_written = true;
        }
        match Pin::new(self.stream.as_mut().expect("live stream")).poll_flush(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(error)) => {
                self.release(false);
                Poll::Ready(Err(error.into()))
            }
            Poll::Ready(Ok(())) => {
                self.write = Vec::new();
                self.write_offset = 0;
                if self.close_queued {
                    self.close_sent = true;
                }
                Poll::Ready(Ok(()))
            }
        }
    }
    /// Complete the explicitly initiated Close. No automatic response/mask/deadline.
    /// Both frames, successful write shutdown and peer transport EOF are required.
    /// At most one receive event is retained: CloseReceivePending is nonterminal
    /// backpressure; drain it via next() and resume with the same absolute deadline.
    /// Timeout is an absorbing failure. Dropping this borrowed future permits resume.
    pub async fn finish_close(
        &mut self,
        deadline: Option<tokio::time::Instant>,
    ) -> Result<(), Fault> {
        if self.terminal == Some(true) {
            return Ok(());
        }
        if self.terminal == Some(false) {
            return Err(Fault::Closed);
        }
        if !self.close_queued {
            return Err(if self.peer_close {
                Fault::CloseResponseRequired
            } else {
                Fault::CloseNotStarted
            });
        }
        if deadline.is_some_and(|at| at <= tokio::time::Instant::now()) {
            self.release(false);
            return Err(Fault::CloseTimeout);
        }
        let result = match deadline {
            Some(at) => match tokio::time::timeout_at(at, self.finish_close_inner()).await {
                Ok(result) => result,
                Err(_) => Err(Fault::CloseTimeout),
            },
            None => self.finish_close_inner().await,
        };
        if result
            .as_ref()
            .is_err_and(|error| !matches!(error, Fault::CloseReceivePending))
        {
            self.release(false);
        }
        result
    }
    async fn finish_close_inner(&mut self) -> Result<(), Fault> {
        // Detect an already-observable half-close before sending our response.
        if self.peer_close && !self.close_bytes_written {
            poll_fn(|cx| {
                let mut byte = [0];
                let mut read = ReadBuf::new(&mut byte);
                match Pin::new(self.stream.as_mut().expect("live stream")).poll_read(cx, &mut read)
                {
                    Poll::Pending => Poll::Ready(Ok(())),
                    Poll::Ready(Err(e)) => Poll::Ready(Err(Fault::Io(e))),
                    Poll::Ready(Ok(())) if read.filled().is_empty() => {
                        Poll::Ready(Err(Fault::PeerEofBeforeCloseSent))
                    }
                    Poll::Ready(Ok(())) => Poll::Ready(Err(Fault::Frame(frame::Fault::Protocol(
                        "bytes after peer Close",
                    )))),
                }
            })
            .await?;
        }
        self.flush().await?;
        while !self.peer_close {
            if !self.ready.is_empty() {
                return Err(Fault::CloseReceivePending);
            }
            if let Some(message) = poll_fn(|cx| self.poll_event(cx)).await? {
                self.ready.try_reserve(1).map_err(frame::Fault::from)?;
                self.ready.push_back(message);
                if !self.peer_close {
                    return Err(Fault::CloseReceivePending);
                }
            }
        }
        if !self.write_shutdown {
            poll_fn(|cx| Pin::new(self.stream.as_mut().expect("live stream")).poll_shutdown(cx))
                .await?;
            self.write_shutdown = true;
        }
        if self.read_offset < self.read_count {
            return Err(frame::Fault::Protocol("bytes after peer Close").into());
        }
        poll_fn(|cx| {
            let mut byte = [0];
            let mut read = ReadBuf::new(&mut byte);
            match Pin::new(self.stream.as_mut().expect("live stream")).poll_read(cx, &mut read) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(Err(error)) => Poll::Ready(Err(Fault::Io(error))),
                Poll::Ready(Ok(())) if read.filled().is_empty() => Poll::Ready(Ok(())),
                Poll::Ready(Ok(())) => Poll::Ready(Err(Fault::Frame(frame::Fault::Protocol(
                    "bytes after peer Close",
                )))),
            }
        })
        .await?;
        self.release(true);
        Ok(())
    }
    fn release(&mut self, success: bool) {
        self.terminal = Some(success);
        if !success {
            self.ready.clear();
        }
        self.stream = None;
        if let Some(core) = self.core.as_mut() {
            core.release();
        }
        self.core = None;
        self.encoder = None;
        self.write = Vec::new();
        self.write_offset = 0;
    }
    pub fn is_terminal(&self) -> bool {
        self.terminal.is_some()
    }
    pub fn pending_write_len(&self) -> usize {
        self.write.len()
    }
    pub fn write_offset(&self) -> usize {
        self.write_offset
    }
    /// Isolated probe observation of owner-held bounded continuation work.
    pub fn append_progress(&self) -> Option<(usize, usize)> {
        self.core.as_ref()?.messages.as_ref()?.append_progress()
    }
    /// Isolated probe observation; no additional buffered payload or publication.
    pub fn text_validation_progress(&self) -> Option<(usize, usize)> {
        match self.core.as_ref()?.pending.as_ref()? {
            Pending::Text { message, offset } => Some((*offset, message.payload.len())),
            Pending::Inflate(_) => None,
        }
    }
}

#[cfg(test)]
mod cancellation_tests {
    use super::*;
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    };
    use tokio::io::{AsyncReadExt, DuplexStream};
    use tokio::sync::{Notify, oneshot};

    #[derive(Default)]
    struct Trace {
        pending: Notify,
        registrations: AtomicUsize,
        drops: AtomicUsize,
        last_waker: Mutex<Option<std::task::Waker>>,
    }

    struct ObservedIo {
        inner: DuplexStream,
        trace: Arc<Trace>,
    }

    impl AsyncRead for ObservedIo {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            let result = Pin::new(&mut self.inner).poll_read(cx, buf);
            if result.is_pending() {
                self.trace.registrations.fetch_add(1, Ordering::SeqCst);
                *self.trace.last_waker.lock().unwrap() = Some(cx.waker().clone());
                self.trace.pending.notify_one();
            }
            result
        }
    }

    impl AsyncWrite for ObservedIo {
        fn poll_write(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<io::Result<usize>> {
            Pin::new(&mut self.inner).poll_write(cx, buf)
        }
        fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            Pin::new(&mut self.inner).poll_flush(cx)
        }
        fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            Pin::new(&mut self.inner).poll_shutdown(cx)
        }
    }

    impl Drop for ObservedIo {
        fn drop(&mut self) {
            self.trace.drops.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn logical_cancel_wakes_idle_read_and_drops_transport_and_pmd_before_ack() {
        let (client, mut peer) = tokio::io::duplex(64);
        let trace = Arc::new(Trace::default());
        let mut owner = Driver::new(
            ObservedIo {
                inner: client,
                trace: trace.clone(),
            },
            Some(PmdParameters::default()),
        )
        .unwrap();
        let decoder = owner.core.as_ref().unwrap().pmd.as_ref().unwrap();
        assert!(!decoder.released());
        let decoder_lifetime = decoder.cancellation_flag();
        assert_eq!(Arc::strong_count(&decoder_lifetime), 2);
        let (cancel, cancelled) = oneshot::channel();
        let publications = Arc::new(AtomicUsize::new(0));
        let observed = publications.clone();
        // This is the connection owner's cancellation branch, separate from
        // cancelling a borrowed next() future (which remains resumable).
        let task = tokio::spawn(async move {
            tokio::select! {
                biased;
                _ = cancelled => {}
                result = owner.next() => {
                    observed.fetch_add(1, Ordering::SeqCst);
                    panic!("idle receive published before cancellation: {result:?}");
                }
            }
            drop(owner);
        });
        trace.pending.notified().await;
        assert!(trace.registrations.load(Ordering::SeqCst) > 0);
        cancel.send(()).unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(1), task)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(trace.drops.load(Ordering::SeqCst), 1);
        // No compression owner remains; its unique Decoder Drop ran before
        // local completion, which invokes inflateEnd exactly once.
        assert_eq!(Arc::strong_count(&decoder_lifetime), 1);
        assert_eq!(peer.read(&mut [0]).await.unwrap(), 0);
        trace.last_waker.lock().unwrap().take().unwrap().wake();
        tokio::task::yield_now().await;
        assert_eq!(publications.load(Ordering::SeqCst), 0);
        assert_eq!(trace.drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn driver_and_decoder_are_not_sync() {
        // This type-inference assertion becomes ambiguous if either owner
        // accidentally acquires Sync. It adds no dependency or unsafe code.
        trait AmbiguousIfSync<Marker> {
            fn check() {}
        }
        impl<T: ?Sized> AmbiguousIfSync<()> for T {}
        struct IsSync;
        impl<T: ?Sized + Sync> AmbiguousIfSync<IsSync> for T {}
        let _ = <pmd::Decoder as AmbiguousIfSync<_>>::check;
        let _ = <Driver<DuplexStream> as AmbiguousIfSync<_>>::check;
    }
}

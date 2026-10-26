//! Owned permessage-deflate state using the pinned upstream compression backend.
use libz_rs_sys as z;
use std::{
    collections::TryReserveError,
    error::Error,
    fmt, ptr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll},
};
#[derive(Debug)]
pub(crate) enum Fault {
    // The backend errno is diagnostic only; it surfaces through the Debug
    // formatting in `Display` and is not branched on.
    #[allow(dead_code)]
    Backend(i32),
    Boundary,
    Padding,
    Capacity,
    Allocation(TryReserveError),
    Terminal,
    Window,
    Limit,
    Busy,
    Idle,
    Cancelled,
}
impl fmt::Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl Error for Fault {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Allocation(e) => Some(e),
            _ => None,
        }
    }
}
#[cfg(test)]
thread_local! {static FAIL_APPEND:std::cell::Cell<bool>=const{std::cell::Cell::new(false)};}
fn append(out: &mut Vec<u8>, bytes: &[u8]) -> Result<(), Fault> {
    #[cfg(test)]
    if FAIL_APPEND.with(|v| v.replace(false)) {
        return Err(Fault::Allocation(
            Vec::<u8>::new().try_reserve_exact(usize::MAX).unwrap_err(),
        ));
    }
    let n = out
        .len()
        .checked_add(bytes.len())
        .filter(|n| *n <= isize::MAX as usize)
        .ok_or(Fault::Capacity)?;
    if n > out.capacity() {
        out.try_reserve_exact(bytes.len())
            .map_err(Fault::Allocation)?;
    }
    out.extend_from_slice(bytes);
    Ok(())
}
/// Unique owner: never Clone the upstream Copy z_stream; its state is one allocation.
pub(super) struct Decoder {
    stream: Box<z::z_stream>,
    initialized: bool,
    failed: bool,
    no_context: bool,
    job: Option<Job>,
    cancellation: Arc<AtomicBool>,
    pub(super) tail_injections: usize,
    pub(super) resets: usize,
    pub(super) poll_steps: usize,
    pub(super) yields: usize,
    pub(super) max_steps_per_poll: usize,
}
// SAFETY: This wrapper constructs only a boxed raw stream with the pinned upstream Rust
// allocator, null opaque pointer and no caller callbacks. The box and upstream allocations
// keep their addresses when this unique owner moves. Initialization, mutation and end always
// have exclusive access, and no handle or pointer escapes. step() clears public slice pointers
// before returning; upstream's inert bitreader/writer pointers are rebound before dispatch
// (zlib-rs 0.6.8 src/inflate.rs:2390-2396). SyncPoint reads only mode/bits; ResetKeep rebinds
// its reader, and End releases owned allocations without reading those old slices.
// Upstream itself marks InflateStream Send (src/inflate.rs:55-56); its static Rust allocator
// is Sync (src/allocate.rs:207-214). The latter is corroboration, not a Send implementation.
// This justification excludes custom
// allocators/callbacks, inflating concurrently, or exposing the raw stream; those changes
// require a new audit. Deliberately no Sync implementation.
unsafe impl Send for Decoder {}

const INPUT_QUANTUM: usize = 256;
const STEPS_PER_POLL: usize = 16;
enum Stage {
    Payload,
    Tail,
    Finalize,
}
struct Job {
    payload: Vec<u8>,
    output: Vec<u8>,
    offset: usize,
    limit: Option<usize>,
    stage: Stage,
    last_boundary: Option<(usize, u8)>,
}
enum Progress {
    Continue,
    Complete,
}
impl Decoder {
    #[cfg(test)]
    pub(super) fn new(no_context: bool) -> Result<Self, Fault> {
        Self::with_window_bits(15, no_context)
    }
    /// Construct a raw inflater with the negotiated server window (8..=15).
    pub(super) fn with_window_bits(bits: u8, no_context: bool) -> Result<Self, Fault> {
        if !(8..=15).contains(&bits) {
            return Err(Fault::Window);
        }
        let mut owner = Self {
            stream: Box::new(z::z_stream::default()),
            initialized: false,
            failed: false,
            no_context,
            job: None,
            cancellation: Arc::new(AtomicBool::new(false)),
            tail_injections: 0,
            resets: 0,
            poll_steps: 0,
            yields: 0,
            max_steps_per_poll: 0,
        };
        // SAFETY: Box is a valid unique initialized z_stream with upstream allocator callbacks.
        // Negative windowBits selects raw DEFLATE; version and size are supplied by this same crate.
        let code = unsafe {
            z::inflateInit2_(
                &mut *owner.stream,
                -i32::from(bits),
                z::zlibVersion(),
                std::mem::size_of::<z::z_stream>() as i32,
            )
        };
        if code != z::Z_OK {
            return Err(Fault::Backend(code));
        }
        owner.initialized = true;
        Ok(owner)
    }
    fn reset(&mut self, keep: bool) -> Result<(), Fault> {
        // SAFETY: stream is initialized and exclusively owned; reset preserves its allocation.
        let code = unsafe {
            if keep {
                z::inflateResetKeep(&mut *self.stream)
            } else {
                z::inflateReset(&mut *self.stream)
            }
        };
        self.resets += 1;
        if code != z::Z_OK {
            Err(Fault::Backend(code))
        } else {
            Ok(())
        }
    }
    fn point(&mut self) -> bool {
        // SAFETY: a live initialized stream under unique access; this only queries state.
        unsafe { z::inflateSyncPoint(&mut *self.stream) == 1 }
    }
    fn step(&mut self, input: &[u8], output: &mut [u8], flush: i32) -> (i32, usize, usize, i32) {
        let take = input.len().min(u32::MAX as usize);
        let room = output.len().min(u32::MAX as usize);
        self.stream.next_in = input.as_ptr();
        self.stream.avail_in = take as u32;
        self.stream.next_out = output.as_mut_ptr();
        self.stream.avail_out = room as u32;
        // SAFETY: input and nonempty output are live, disjoint slices for exactly avail lengths;
        // the unique initialized stream cannot escape. The synchronous call ends before slices expire.
        let code = unsafe { z::inflate(&mut *self.stream, flush) };
        let result = (
            code,
            take - self.stream.avail_in as usize,
            room - self.stream.avail_out as usize,
            self.stream.data_type,
        );
        // Clear public borrowed pointers. Upstream inert internal pointer fields are rebound on
        // the next call; they are never dereferenced by our wrapper between calls.
        self.stream.next_in = ptr::null();
        self.stream.avail_in = 0;
        self.stream.next_out = ptr::null_mut();
        self.stream.avail_out = 0;
        result
    }
    #[cfg(test)]
    pub(super) fn decode(&mut self, payload: &[u8]) -> Result<Vec<u8>, Fault> {
        self.decode_sync(payload, None)
    }
    #[cfg(test)]
    pub(super) fn decode_limit(&mut self, payload: &[u8], limit: usize) -> Result<Vec<u8>, Fault> {
        self.decode_sync(payload, Some(limit))
    }
    // Synchronous compatibility entry drives the exact same state machine to completion.
    // This API intentionally makes no cooperative scheduling guarantee.
    #[cfg(test)]
    fn decode_sync(&mut self, payload: &[u8], limit: Option<usize>) -> Result<Vec<u8>, Fault> {
        if self.failed {
            return Err(Fault::Terminal);
        }
        if self.job.is_some() {
            return Err(Fault::Busy);
        }
        let mut owned = Vec::new();
        if let Err(error) = append(&mut owned, payload) {
            self.cancel();
            return Err(error);
        }
        self.start_decode(owned, limit)?;
        let mut cx = Context::from_waker(std::task::Waker::noop());
        loop {
            if let Poll::Ready(result) = self.poll_decode(&mut cx) {
                return result;
            }
        }
    }
    /// Starts an owned decode. Busy rejects a second message without damaging the first job.
    /// There is no input-size or iteration ceiling. `limit` is an explicit caller output limit.
    pub(super) fn start_decode(
        &mut self,
        payload: Vec<u8>,
        limit: Option<usize>,
    ) -> Result<(), Fault> {
        if self.failed {
            return Err(Fault::Terminal);
        }
        if self.cancellation.load(Ordering::Acquire) {
            self.cancel();
            return Err(Fault::Cancelled);
        }
        if self.job.is_some() {
            return Err(Fault::Busy);
        }
        if self.no_context
            && let Err(error) = self.reset(false)
        {
            self.cancel();
            return Err(error);
        }
        self.job = Some(Job {
            payload,
            output: Vec::new(),
            offset: 0,
            limit,
            stage: Stage::Payload,
            last_boundary: None,
        });
        Ok(())
    }
    /// Executes a bounded scheduling quantum. All progress is owned by `self`, so dropping a
    /// borrowed poll future leaves a resumable job. Explicit cancellation is absorbing.
    pub(super) fn poll_decode(&mut self, cx: &mut Context<'_>) -> Poll<Result<Vec<u8>, Fault>> {
        if self.failed {
            return Poll::Ready(Err(Fault::Terminal));
        }
        if self.cancellation.load(Ordering::Acquire) {
            self.cancel();
            return Poll::Ready(Err(Fault::Cancelled));
        }
        let Some(mut job) = self.job.take() else {
            return Poll::Ready(Err(Fault::Idle));
        };
        for steps in 1..=STEPS_PER_POLL {
            if self.cancellation.load(Ordering::Acquire) {
                self.cancel();
                return Poll::Ready(Err(Fault::Cancelled));
            }
            self.poll_steps += 1;
            self.max_steps_per_poll = self.max_steps_per_poll.max(steps);
            match self.advance(&mut job) {
                Ok(Progress::Complete) => {
                    // Publication linearizes at this final cancellation observation.
                    if self.cancellation.load(Ordering::Acquire) {
                        self.cancel();
                        return Poll::Ready(Err(Fault::Cancelled));
                    }
                    return Poll::Ready(Ok(job.output));
                }
                Ok(Progress::Continue) => {}
                Err(error) => {
                    self.cancel();
                    return Poll::Ready(Err(error));
                }
            }
        }
        self.job = Some(job);
        self.yields += 1;
        cx.waker().wake_by_ref();
        Poll::Pending
    }
    fn advance(&mut self, job: &mut Job) -> Result<Progress, Fault> {
        let mut chunk = [0u8; 8192];
        match job.stage {
            Stage::Payload => {
                let room = job.limit.map_or(chunk.len(), |n| {
                    n.saturating_sub(job.output.len())
                        .saturating_add(1)
                        .min(chunk.len())
                });
                let end = job
                    .offset
                    .saturating_add(INPUT_QUANTUM)
                    .min(job.payload.len());
                let (code, used, made, state) = self.step(
                    &job.payload[job.offset..end],
                    &mut chunk[..room],
                    z::Z_BLOCK,
                );
                if job
                    .limit
                    .is_some_and(|n| made > n.saturating_sub(job.output.len()))
                {
                    return Err(Fault::Limit);
                }
                job.offset = job.offset.checked_add(used).ok_or(Fault::Capacity)?;
                append(&mut job.output, &chunk[..made])?;
                if ![z::Z_OK, z::Z_BUF_ERROR, z::Z_STREAM_END].contains(&code) {
                    return Err(Fault::Backend(code));
                }
                if state & 128 != 0 && state & 64 != 0 {
                    let padding = (state & 63) as usize;
                    if padding >= 8
                        || (padding > 0
                            && (job.offset == 0
                                || job.payload[job.offset - 1] >> (8 - padding) != 0))
                    {
                        return Err(Fault::Padding);
                    }
                }
                if code == z::Z_STREAM_END {
                    self.reset(true)?;
                    job.last_boundary = None;
                    return Ok(Progress::Continue);
                }
                if job.offset == job.payload.len() && self.point() {
                    job.stage = Stage::Tail;
                    return Ok(Progress::Continue);
                }
                // Z_BLOCK may consume EOB entirely from its buffered bits. A distinct bit
                // boundary is progress even with unchanged total_in. At an input chunk edge,
                // the next step supplies the next chunk; only full-message exhaustion can
                // establish the pre-tail sync point.
                let boundary = (job.offset, (state & 63) as u8);
                if used == 0 && made == 0 {
                    if state & 128 != 0 && job.last_boundary != Some(boundary) {
                        job.last_boundary = Some(boundary);
                        return Ok(Progress::Continue);
                    }
                    return Err(Fault::Boundary);
                }
                job.last_boundary = if state & 128 != 0 {
                    Some(boundary)
                } else {
                    None
                };
                Ok(Progress::Continue)
            }
            Stage::Tail => {
                // SyncPoint was proven before synthetic bytes: only omitted LEN/NLEN remain.
                let (code, used, made, state) =
                    self.step(&[0, 0, 255, 255], &mut chunk, z::Z_BLOCK);
                self.tail_injections += 1;
                if code != z::Z_OK || used != 4 || made != 0 || state & 128 == 0 || state & 63 != 0
                {
                    return Err(Fault::Boundary);
                }
                if state & 64 == 0 {
                    return Ok(Progress::Complete);
                }
                job.stage = Stage::Finalize;
                Ok(Progress::Continue)
            }
            Stage::Finalize => {
                // A legal final empty stored block is normalized with dictionary preserved.
                let (code, used, made, _) = self.step(&[], &mut chunk, z::Z_BLOCK);
                if code != z::Z_STREAM_END || used != 0 || made != 0 {
                    return Err(Fault::Boundary);
                }
                self.reset(true)?;
                Ok(Progress::Complete)
            }
        }
    }
    pub(super) fn cancel(&mut self) {
        self.cancellation.store(true, Ordering::Release);
        self.failed = true;
        self.job = None;
        self.release();
    }
    #[cfg(test)]
    pub(super) fn job_pending(&self) -> bool {
        self.job.is_some()
    }
    #[cfg(test)]
    pub(super) fn buffered_output(&self) -> usize {
        self.job.as_ref().map_or(0, |job| job.output.len())
    }
    #[cfg(test)]
    pub(super) fn released(&self) -> bool {
        !self.initialized
    }
    /// Cooperative signal only: a pending decode self-wakes, but this flag cannot wake an
    /// unrelated idle socket read. The transport owner must supply that cancellation wakeup.
    #[cfg(test)]
    pub(super) fn cancellation_flag(&self) -> Arc<AtomicBool> {
        self.cancellation.clone()
    }
    fn release(&mut self) {
        if self.initialized {
            // SAFETY: exactly once for the initialized state, never cloned or used after end.
            unsafe { z::inflateEnd(&mut *self.stream) };
            self.initialized = false;
        }
    }
    #[cfg(test)]
    pub(super) fn failed(&self) -> bool {
        self.failed
    }
}
impl Drop for Decoder {
    fn drop(&mut self) {
        self.release()
    }
}
/// Encoder remains an ordinary safe upstream flate2 client, including caller feature union.
pub(super) struct Encoder {
    inner: Option<flate2::Compress>,
    no_context: bool,
}
impl Encoder {
    pub(super) fn new(bits: u8, no_context: bool) -> Result<Self, Fault> {
        if !(8..=15).contains(&bits) {
            return Err(Fault::Window);
        }
        Ok(Self {
            inner: Some(flate2::Compress::new_with_window_bits(
                flate2::Compression::default(),
                false,
                bits.max(9),
            )),
            no_context,
        })
    }
    pub(super) fn encode(&mut self, input: &[u8]) -> Result<Vec<u8>, Fault> {
        let result = self.encode_inner(input);
        if result.is_err() {
            self.inner = None
        }
        result
    }
    fn encode_inner(&mut self, input: &[u8]) -> Result<Vec<u8>, Fault> {
        let inner = self.inner.as_mut().ok_or(Fault::Terminal)?;
        if self.no_context {
            inner.reset()
        }
        let mut out = Vec::new();
        let mut offset = 0usize;
        let mut chunk = [0; 8192];
        loop {
            let (a, b) = (inner.total_in(), inner.total_out());
            inner
                .compress(&input[offset..], &mut chunk, flate2::FlushCompress::Sync)
                .map_err(|_| Fault::Backend(-3))?;
            let used = (inner.total_in() - a) as usize;
            let made = (inner.total_out() - b) as usize;
            offset += used;
            append(&mut out, &chunk[..made])?;
            if offset == input.len() && made < chunk.len() {
                break;
            }
            if used == 0 && made == 0 {
                return Err(Fault::Boundary);
            }
        }
        if !out.ends_with(&[0, 0, 255, 255]) {
            return Err(Fault::Boundary);
        }
        out.truncate(out.len() - 4);
        Ok(out)
    }
}

#[cfg(test)]
mod local_growth_tests {
    use super::*;
    #[test]
    fn encoder_storage_failure_poisoning() {
        let mut e = Encoder::new(15, false).unwrap();
        FAIL_APPEND.with(|v| v.set(true));
        assert!(matches!(
            e.encode(b"advanced-compression-state"),
            Err(Fault::Allocation(_))
        ));
        assert!(matches!(e.encode(b"later"), Err(Fault::Terminal)));
    }
    #[test]
    fn decoder_storage_failure_poisoning() {
        let input = Encoder::new(15, false).unwrap().encode(b"payload").unwrap();
        let mut d = Decoder::new(false).unwrap();
        FAIL_APPEND.with(|v| v.set(true));
        assert!(matches!(d.decode(&input), Err(Fault::Allocation(_))));
        assert!(d.failed());
        assert!(matches!(d.decode(&[0]), Err(Fault::Terminal)));
    }
    #[test]
    fn pending_decoder_output_growth_failure_releases_advanced_state() {
        let input = Encoder::new(15, false).unwrap().encode(b"payload").unwrap();
        let mut d = Decoder::new(false).unwrap();
        d.start_decode(input, None).unwrap();
        FAIL_APPEND.with(|v| v.set(true));
        let mut cx = Context::from_waker(std::task::Waker::noop());
        assert!(matches!(
            d.poll_decode(&mut cx),
            Poll::Ready(Err(Fault::Allocation(_)))
        ));
        assert!(d.failed());
        assert!(d.released());
        assert!(!d.job_pending());
        assert_eq!(d.buffered_output(), 0);
        assert!(matches!(d.decode(&[0]), Err(Fault::Terminal)));
    }
}

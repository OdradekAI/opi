//! Isolated client receive codec. No HTTP, IO, PMD inflater, or provider policy.
//! Header storage is fixed; payload allocation follows actual supplied bytes.

use std::collections::TryReserveError;
use std::fmt;

#[derive(Debug)]
pub enum Fault {
    Protocol(&'static str),
    Storage(TryReserveError),
    Capacity,
    Utf8,
    Closed,
}

impl fmt::Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Protocol(reason) => write!(f, "WebSocket protocol: {reason}"),
            Self::Storage(error) => write!(f, "WebSocket storage: {error}"),
            Self::Capacity => f.write_str("WebSocket storage length is not representable"),
            Self::Utf8 => f.write_str("WebSocket text is not UTF-8"),
            Self::Closed => f.write_str("WebSocket receive state is terminal"),
        }
    }
}

impl std::error::Error for Fault {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Storage(error) => Some(error),
            _ => None,
        }
    }
}

impl From<TryReserveError> for Fault {
    fn from(error: TryReserveError) -> Self {
        Self::Storage(error)
    }
}

pub fn checked_storage_len(current: usize, additional: usize) -> Result<usize, Fault> {
    current
        .checked_add(additional)
        .filter(|length| *length <= isize::MAX as usize)
        .ok_or(Fault::Capacity)
}

#[cfg(test)]
thread_local! {
    static FAIL_PREFERRED:std::cell::Cell<bool>=const{std::cell::Cell::new(false)};
    static FAIL_REQUIRED:std::cell::Cell<bool>=const{std::cell::Cell::new(false)};
}
fn append_actual(buffer: &mut Vec<u8>, bytes: &[u8]) -> Result<(), Fault> {
    if bytes.is_empty() {
        return Ok(());
    }
    checked_storage_len(buffer.len(), bytes.len())?;
    #[cfg(test)]
    let preferred = if FAIL_PREFERRED.with(|v| v.replace(false)) {
        Vec::<u8>::new().try_reserve(usize::MAX)
    } else {
        buffer.try_reserve(bytes.len())
    };
    #[cfg(not(test))]
    let preferred = buffer.try_reserve(bytes.len());
    if preferred.is_err() {
        #[cfg(test)]
        let required = if FAIL_REQUIRED.with(|v| v.replace(false)) {
            Vec::<u8>::new().try_reserve_exact(usize::MAX)
        } else {
            buffer.try_reserve_exact(bytes.len())
        };
        #[cfg(not(test))]
        let required = buffer.try_reserve_exact(bytes.len());
        required.map_err(Fault::from)?;
    }
    buffer.extend_from_slice(bytes);
    Ok(())
}

#[derive(Debug, PartialEq, Eq)]
pub struct Frame {
    pub fin: bool,
    pub opcode: u8,
    pub compressed: bool,
    pub payload: Vec<u8>,
}

#[derive(Clone, Copy)]
struct Header {
    fin: bool,
    opcode: u8,
    compressed: bool,
    length: usize,
}

pub struct Decoder {
    pmd: bool,
    header: [u8; 10],
    header_len: usize,
    header_need: usize,
    current: Option<Header>,
    body: Vec<u8>,
    fragmented: bool,
    terminal: bool,
    failed: bool,
}

impl Decoder {
    pub fn new(pmd: bool) -> Self {
        Self {
            pmd,
            header: [0; 10],
            header_len: 0,
            header_need: 2,
            current: None,
            body: Vec::new(),
            fragmented: false,
            terminal: false,
            failed: false,
        }
    }

    /// Convenience all-or-error batch API. Use `feed_into` for prefix delivery
    /// when a later frame in this same input chunk is malformed.
    pub fn feed(&mut self, bytes: &[u8]) -> Result<Vec<Frame>, Fault> {
        let mut complete = Vec::new();
        self.feed_into(bytes, &mut complete)?;
        Ok(complete)
    }

    /// Completed frames are appended exactly once, even if a later header in
    /// this chunk fails. Errors absorb future input and release payload storage.
    pub fn feed_into(&mut self, bytes: &[u8], complete: &mut Vec<Frame>) -> Result<(), Fault> {
        if self.terminal {
            return Err(Fault::Closed);
        }
        let result = self.feed_inner(bytes, complete);
        if result.is_err() {
            self.terminal = true;
            self.failed = true;
            self.body = Vec::new();
        }
        result
    }

    fn feed_inner(&mut self, mut bytes: &[u8], complete: &mut Vec<Frame>) -> Result<(), Fault> {
        loop {
            if self.current.is_none() {
                if bytes.is_empty() {
                    break;
                }
                let count = (self.header_need - self.header_len).min(bytes.len());
                self.header[self.header_len..self.header_len + count]
                    .copy_from_slice(&bytes[..count]);
                self.header_len += count;
                bytes = &bytes[count..];
                if self.header_len < self.header_need {
                    continue;
                }
                if self.header_need == 2 {
                    self.header_need = self.validate_prefix()?;
                    if self.header_len < self.header_need {
                        continue;
                    }
                }
                self.current = Some(self.parse_header()?);
            }

            let header = self.current.expect("header parsed before payload");
            let remaining = header.length - self.body.len();
            let count = remaining.min(bytes.len());
            append_actual(&mut self.body, &bytes[..count])?;
            bytes = &bytes[count..];
            if self.body.len() != header.length {
                break;
            }

            complete.try_reserve(1).map_err(Fault::from)?;
            complete.push(Frame {
                fin: header.fin,
                opcode: header.opcode,
                compressed: header.compressed,
                payload: std::mem::take(&mut self.body),
            });
            match header.opcode {
                1 | 2 if !header.fin => self.fragmented = true,
                0 if header.fin => self.fragmented = false,
                _ => {}
            }
            self.current = None;
            self.header_len = 0;
            self.header_need = 2;
        }
        Ok(())
    }

    fn validate_prefix(&self) -> Result<usize, Fault> {
        let first = self.header[0];
        let second = self.header[1];
        let opcode = first & 0x0f;
        if !matches!(opcode, 0 | 1 | 2 | 8 | 9 | 10) {
            return Err(Fault::Protocol("reserved opcode"));
        }
        if first & 0x30 != 0 {
            return Err(Fault::Protocol("unnegotiated RSV2/RSV3"));
        }
        if first & 0x40 != 0 && (!self.pmd || !matches!(opcode, 1 | 2)) {
            return Err(Fault::Protocol("invalid RSV1"));
        }
        if second & 0x80 != 0 {
            return Err(Fault::Protocol("masked server frame"));
        }
        if opcode >= 8 && (first & 0x80 == 0 || second & 0x7f > 125) {
            return Err(Fault::Protocol("fragmented or oversized control frame"));
        }
        if opcode == 0 && !self.fragmented {
            return Err(Fault::Protocol("continuation without initial frame"));
        }
        if matches!(opcode, 1 | 2) && self.fragmented {
            return Err(Fault::Protocol(
                "new data message during fragmented message",
            ));
        }
        Ok(match second & 0x7f {
            126 => 4,
            127 => 10,
            _ => 2,
        })
    }

    fn parse_header(&self) -> Result<Header, Fault> {
        let length = match self.header[1] & 0x7f {
            126 => {
                let length = u16::from_be_bytes([self.header[2], self.header[3]]);
                if length < 126 {
                    return Err(Fault::Protocol("nonminimal 126 length"));
                }
                u64::from(length)
            }
            127 => {
                let length = u64::from_be_bytes(self.header[2..10].try_into().unwrap());
                if length & (1_u64 << 63) != 0 || length < 65536 {
                    return Err(Fault::Protocol("invalid or nonminimal 127 length"));
                }
                length
            }
            length => u64::from(length),
        };
        Ok(Header {
            fin: self.header[0] & 0x80 != 0,
            opcode: self.header[0] & 0x0f,
            compressed: self.header[0] & 0x40 != 0,
            length: usize::try_from(length).map_err(|_| Fault::Capacity)?,
        })
    }

    /// Permanent EOF. It cannot reset this owner for a different connection.
    pub fn eof(&mut self) -> Result<(), Fault> {
        if self.terminal {
            return Err(Fault::Closed);
        }
        self.terminal = true;
        if self.header_len != 0 || self.current.is_some() || self.fragmented {
            self.failed = true;
            self.body = Vec::new();
            return Err(Fault::Protocol("EOF during frame or fragmented message"));
        }
        Ok(())
    }

    pub fn buffered_len(&self) -> usize {
        self.header_len + self.body.len()
    }

    /// Includes the fixed ten-byte header, even when no bytes are buffered.
    pub fn buffered_capacity(&self) -> usize {
        self.header.len() + self.body.capacity()
    }

    pub fn failed(&self) -> bool {
        self.failed
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Message {
    pub opcode: u8,
    pub compressed: bool,
    pub payload: Vec<u8>,
}

/// Complete-message assembler. Compressed payloads remain opaque and require
/// the separate PMD boundary validator before decoded text can be delivered.
struct Append {
    frame: Frame,
    offset: usize,
}

#[derive(Default)]
pub struct Messages {
    // Only the IO owner may defer text validation; it cannot publish until its
    // bounded incremental validation finishes. Public constructors stay strict.
    defer_text: bool,
    pending: Option<Message>,
    append: Option<Append>,
    terminal: bool,
    failed: bool,
}

impl Messages {
    pub fn new() -> Self {
        Self::default()
    }

    /// Internal ownership transfer: the IO owner must validate all text before
    /// publication, including uncompressed messages, using its owned cursor.
    pub(crate) fn new_deferred_text() -> Self {
        Self {
            defer_text: true,
            ..Self::default()
        }
    }

    pub fn receive(&mut self, frame: Frame) -> Result<Option<Message>, Fault> {
        if self.terminal {
            return Err(Fault::Closed);
        }
        let result = self.receive_inner(frame);
        if result.is_err() {
            self.poison();
        }
        result
    }

    /// Only the IO owner uses this deferred append operation. A completed
    /// continuation stays owned until poll_append has copied every byte, and
    /// the owner polls that operation before accepting another frame.
    pub(crate) fn receive_deferred(&mut self, frame: Frame) -> Result<Option<Message>, Fault> {
        if self.terminal {
            return Err(Fault::Closed);
        }
        if self.append.is_some() {
            return Err(Fault::Protocol("continuation copy is pending"));
        }
        if frame.opcode != 0 {
            return self.receive(frame);
        }
        let result = (|| {
            if frame.compressed {
                return Err(Fault::Protocol("RSV1 on continuation"));
            }
            if self.pending.is_none() {
                return Err(Fault::Protocol("orphan continuation"));
            }
            self.append = Some(Append { frame, offset: 0 });
            Ok(None)
        })();
        if result.is_err() {
            self.poison();
        }
        result
    }

    pub(crate) fn poll_append(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<Option<Message>, Fault>> {
        use std::task::Poll;
        if self.append.is_none() {
            return Poll::Ready(Ok(None));
        }
        let result = (|| {
            let job = self.append.as_mut().expect("owned continuation copy");
            let end = job.offset.saturating_add(8192).min(job.frame.payload.len());
            append_actual(
                &mut self.pending.as_mut().expect("continuation message").payload,
                &job.frame.payload[job.offset..end],
            )?;
            job.offset = end;
            Ok::<_, Fault>(end == job.frame.payload.len())
        })();
        match result {
            Err(error) => {
                self.poison();
                Poll::Ready(Err(error))
            }
            Ok(false) => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Ok(true) => {
                let job = self.append.take().expect("completed continuation copy");
                if job.frame.fin {
                    // This private mode transfers Text validation to the IO
                    // owner's bounded validator; no public unchecked constructor.
                    debug_assert!(self.defer_text);
                    Poll::Ready(Ok(self.pending.take()))
                } else {
                    Poll::Ready(Ok(None))
                }
            }
        }
    }

    pub(crate) fn append_progress(&self) -> Option<(usize, usize)> {
        let job = self.append.as_ref()?;
        Some((job.offset, job.frame.payload.len()))
    }

    fn poison(&mut self) {
        self.terminal = true;
        self.failed = true;
        self.pending = None;
        self.append = None;
    }

    fn receive_inner(&mut self, frame: Frame) -> Result<Option<Message>, Fault> {
        if matches!(frame.opcode, 8..=10) {
            if !frame.fin || frame.compressed || frame.payload.len() > 125 {
                return Err(Fault::Protocol("invalid control message"));
            }
            if frame.opcode == 8 {
                validate_close(&frame.payload)?;
                self.terminal = true;
                self.pending = None;
                self.append = None;
            }
            return Ok(Some(Message {
                opcode: frame.opcode,
                compressed: false,
                payload: frame.payload,
            }));
        }

        match frame.opcode {
            1 | 2 => {
                if self.pending.is_some() {
                    return Err(Fault::Protocol("overlapping data messages"));
                }
                self.pending = Some(Message {
                    opcode: frame.opcode,
                    compressed: frame.compressed,
                    payload: frame.payload,
                });
            }
            0 => {
                if frame.compressed {
                    return Err(Fault::Protocol("RSV1 on continuation"));
                }
                let pending = self
                    .pending
                    .as_mut()
                    .ok_or(Fault::Protocol("orphan continuation"))?;
                append_actual(&mut pending.payload, &frame.payload)?;
            }
            _ => return Err(Fault::Protocol("reserved message opcode")),
        }
        if !frame.fin {
            return Ok(None);
        }
        let message = self.pending.take().expect("data message initialized");
        if message.opcode == 1 && !message.compressed && !self.defer_text {
            std::str::from_utf8(&message.payload).map_err(|_| Fault::Utf8)?;
        }
        Ok(Some(message))
    }

    pub fn eof(&mut self) -> Result<(), Fault> {
        if self.terminal {
            return Err(Fault::Closed);
        }
        self.terminal = true;
        let incomplete = self.pending.take().is_some();
        let copying = self.append.take().is_some();
        if incomplete || copying {
            self.failed = true;
            return Err(Fault::Protocol("EOF during message"));
        }
        Ok(())
    }

    pub fn failed(&self) -> bool {
        self.failed
    }
}

fn validate_close(payload: &[u8]) -> Result<(), Fault> {
    if payload.is_empty() {
        return Ok(());
    }
    if payload.len() == 1 {
        return Err(Fault::Protocol("one-byte Close payload"));
    }
    let code = u16::from_be_bytes([payload[0], payload[1]]);
    if !matches!(code, 1000..=1003 | 1007..=1014 | 3000..=4999) {
        return Err(Fault::Protocol("reserved or invalid Close code"));
    }
    std::str::from_utf8(&payload[2..]).map_err(|_| Fault::Utf8)?;
    Ok(())
}

#[cfg(test)]
mod growth_controls {
    use super::*;
    #[test]
    fn preferred_reserve_failure_falls_back_without_losing_bytes() {
        let mut d = Decoder::new(false);
        FAIL_PREFERRED.with(|v| v.set(true));
        let frames = d.feed(&[0x82, 3, b'a', b'b', b'c']).unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].payload, b"abc");
        assert!(!d.failed());
    }
    #[test]
    fn required_reserve_failure_is_typed_and_absorbing() {
        let mut d = Decoder::new(false);
        let mut out = vec![];
        d.feed_into(&[0x81, 1, b'p'], &mut out).unwrap();
        FAIL_PREFERRED.with(|v| v.set(true));
        FAIL_REQUIRED.with(|v| v.set(true));
        let error = d.feed_into(&[0x82, 3, b'a'], &mut out).unwrap_err();
        assert!(std::error::Error::source(&error).is_some());
        assert!(d.failed());
        assert_eq!(out[0].payload, b"p");
        assert_eq!(d.buffered_capacity(), 10);
        assert!(d.feed(&[0x81, 0]).is_err());
    }
}

# Independent source audit: PMD lifecycle, Send, hidden API and feature boundaries

Scope: read-only source inspection of the isolated prototype and exact cached registry sources. No Cargo, network, product/ledger changes, or cache mutation were performed by this reviewer. This report is not product admission or proof of all supported platforms. The PMD/Close source owners are still implementing this iteration; final source/log review is pending below.

## Exact source identity

Registry root: `C:/Users/Luiz/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/`.

- `libz-rs-sys-0.6.8`, `zlib-rs-0.6.8`, `flate2-1.1.10`.
- Prototype Cargo.toml selects `libz-rs-sys` exact 0.6.8, default features off, std + rust-allocator; flate2 exact 1.1.10, defaults off, zlib-rs; selected unions are flate2/default and flate2/zlib.
- A direct exact libz-rs-sys pin does not by itself pin its semver `zlib-rs = 0.6.8` transitive requirement against every future compatible release. The lock/metadata and source checksum must identify the actual zlib-rs; an upgrade requires repeating this audit, not inheriting its conclusion.

## Send: narrow justification, not a blanket assertion

A local `unsafe impl Send for Decoder` is supportable under ALL of these conditions:

1. The stream is private and uniquely owned, its allocated state is never copied (`z_stream` itself is upstream Copy/Clone), no raw references/pointers or allocator customization escape the safe wrapper, no concurrent access exists, and there is no `unsafe impl Sync`.
2. Construction uses only the default Rust global allocator (thread-safe allocation/deallocation), null opaque value, raw DEFLATE configuration, no custom callbacks, no gzip header pointer or other externally borrowed state. No public constructor accepting an arbitrary z_stream/allocator is added.
3. Every inflate call is synchronous and receives live disjoint input/output slices with checked u32 lengths. A move/yield may occur only after it returns. The wrapper clears public next_in/next_out + avail fields after each call, including errors; the only allowed subsequent state operations are the audited ones below.
4. `inflateEnd` executes exactly once on a live initialized state, on whichever thread finally owns it; constructor failure and later fault do not double end. Storage remains owned until the last synchronous operation completes. Cancellation does not manufacture a reference to a dead stream.

The internal pointer facts are more precise than “no pointers retained”: zlib-rs State stores BitReader ptr/end and Writer WeakSliceMut even after inflate returns (`inflate.rs:407–410`, `inflate/bitreader.rs:5–12`, `inflate/writer.rs:8–11`, `weak_slice.rs:3–20`). These can contain stale addresses after the caller slices expire. This alone is not dereference/UB: they are raw representations, not retained Rust slice references. Before any dispatch that reads/writes caller data, the next inflate call unconditionally rebinds BOTH reader and writer (`inflate.rs:2390–2401`). `sync_point` reads only mode and bit-buffer count (`2537–2539`), `reset_keep` replaces the bitreader and resets scalars without reading old IO (`2338–2369`), `reset` additionally clears owned window state (`2329–2335`), and `end` frees the single owned allocation without reading caller IO (`2648–2661`). Raw mode never calls get_header; window memory lives inside the owned allocation (`2264–2285`).

Upstream explicitly implements Send and Sync on its InflateStream (`inflate.rs:55–56`), and its safe Inflate owns that stream with lifetime static (`stable.rs:69–74`). These corroborate the model but do not prove this custom wrapper sound by themselves. Correction to an earlier handoff: `allocate.rs:207` is `unsafe impl Sync for Allocator<static>`, NOT Send. The default RUST allocator’s opaque is null (`210–215`), and free reconstructs the allocation size inside the upstream allocator (`allocate.rs:388–393`). CPU dispatch uses an AtomicU32 cache, not thread-affine stream ownership (`cpu_features.rs:42–44`).

Required differentiating tests: actual create/decode on one thread, move partially advanced/context-bearing decoder through another thread, resume after prior input/output buffers have gone out of scope, then drop on a third thread; error + drop migration; full Driver carried by spawned Send future, not only an assert_send helper. A negative compile check should keep Decoder non-Sync. Tests corroborate source proof and cannot prove unsafe soundness generally. Miri/sanitizer coverage remains separate unless actually executed.

## Hidden APIs and maintenance ownership

`inflateSyncPoint` (`libz-rs-sys/src/lib.rs:892–907`) and `inflateResetKeep` (`1226–1239`) are public callable Rust exports marked doc(hidden); they are not private symbol scraping, source patches, or calls through external dynamic lookup. Both have explicit unsafe initialized-stream preconditions. Their documented material does not establish a broad stable Rust PMD contract. Avoid claiming doc(hidden) automatically means a formal semver exemption; the concrete evidence is that callers own an upgrade-sensitive boundary absent from the ordinary safe flate2 interface. libz-rs-sys itself enables zlib-rs `__internal-api` (`Cargo.toml:59–62`), whose public exposure is feature-selected (`zlib-rs/src/lib.rs:14–33`).

The mechanism is not a compressed-bytes suffix guess. Upstream sync_point tests Mode::Stored + zero bits BEFORE inserting the missing LEN/NLEN bytes (`inflate.rs:2529–2539`). Z_BLOCK exposes exact block boundaries/data_type, allowing legal BFINAL/reset_keep and context-history handling. The wrapper still must replay legal empty messages/BFINAL/reset/history, invalid missing block header, truncation at every relevant prefix, retained dictionaries and exactly-once synthetic tail after every dependency upgrade. A hidden API removal/semantic change is a build/behavior failure requiring reassessment; never substitute “append tail then accept success.”

Baseline libz-rs-sys and zlib-rs manifests have `build = false`, edition 2021 and declared rust-version 1.75. They compile Rust with internal unsafe/SIMD/runtime CPU dispatch; C ABI spelling does not imply a linked native zlib for this decoder. This does not prove the entire prototype’s compiler/MSRV or platforms. C-zlib union changes the flate2 encoder to a C backend (`flate2/src/ffi/mod.rs:62–78`) and introduces native build/link/provenance requirements. Its internal wrapper’s allocator and Send proof are owned by flate2 (`ffi/c.rs:21–68,72–131`), distinct from this local Decoder unsafe proof.

## Feature union acceptance must remain finite and explicit

The three selected graphs do NOT establish all-feature compatibility:

- `libz-rs-sys/rust-allocator + libz-rs-sys/c-allocator` deliberately compile_errors (`lib.rs:104–106`). This is distinct from zlib-rs internally permitting both defaults and preferring Rust (`c_api.rs:93–101`, `inflate.rs:2239–2247`). A caller may cause an unsupported unification; this must be reported, not advertised as a successful arbitrary feature union.
- `libz-rs-sys/gz` adds optional libc (`Cargo.toml:44,55–57`) and unrelated gzip/file APIs; `gzprintf` adds the nightly `c_variadic` language feature (`lib.rs:1`). No stable all-features support follows from the selected three graphs.
- `export-symbols` without prefix exports zlib C names; paired with a C-zlib graph, symbol collision/interposition becomes a new linking/safety boundary. `semver-prefix`, `testing-prefix`, or `custom-prefix` alter the symbol identity (`lib.rs:54–99`); custom-prefix also depends on compile-time LIBZ_RS_SYS_PREFIX. The baseline leaves them disabled. Do not enable these to “fix” arbitrary caller unions without separate linker/consumer evidence.
- zlib-rs optional test/fuzz/debug/SIMD features and flate2 other native backends are not covered. Default Rust encoder + miniz default union continues to select zlib-rs; C union selects C. Decoder remains the direct libz-rs-sys path in all three, so it is not independently retested through three decoder implementations.

## Close and scheduling proof obligations

The starting iteration destroyed the driver as soon as Close was decoded, which could not complete a peer-initiated reply. A complete explicit owner must retain IO long enough to flush one valid response, not accept new data after own Close, handle simultaneous/local/peer initiation, retain partially written masked bytes across cancelled borrows, and distinguish abnormal EOF from exchanged Close. Server and client transport shutdown roles differ; the prototype is a CLIENT codec, so no server-role claim should be inferred from an outbound peer helper.

Explicit caller-supplied response masks/deadlines and explicit abort operations fit mechanism-below-policy. A hardcoded timeout, hidden task, hidden reconnect, ambient RNG, or an assertion that cancellation itself authorizes protocol success would not. Close timeout/transport abort must be distinguishable from clean closure. Successful EOF is not a substitute for actually writing and flushing the Close response. Cancelled futures need bounded resumable state, and dropping the owner must release stream plus pending compression without a detached worker.

A cooperative decoder needs work budgets based on both consumed compressed input and generated output, plus bounded per-call backend work; counting only output misses empty blocks and malformed/header-heavy streams. Each borrowed async poll must yield before consuming unbounded immediate input or backend loops. Task cancellation occurs at yield points, not preemption inside a single upstream inflate call. Test a highly compressed expansion, many empty blocks, ready transport, cancellation before final tail, resumed no-context/context streams, and post-failure permanent EOF; compare accepted outputs independent of budget to prove scheduling does not reject source-valid messages. A scheduling quantum must not become a message/input size limit.

## Adoption conclusion

This source audit finds a viable small unsafe boundary for a Send, owned, patch-free upstream PMD decoder under the stated exact profile. It does not require Ratchet or modifying upstream compression sources. “No upstream patch” still entails local unsafe ownership, hidden API upgrade review, full WS protocol maintenance, and declared unsupported feature combinations. Product adoption still needs admitted placement/contract updates, full Upgrade/provider/package/platform evidence and independent verification, with no premature 20.11/20.34/20.57 acceptance.

## Actual iteration-2 source findings (before final test logs)

PMD owner reviewed at `src/pmd.rs` and `tests/pmd_async.rs` after implementation handoff:

- The actual unsafe Send comment preserves the required distinctions, with no Sync implementation. Job owns both input Vec and output Vec; local 8192-byte output arrays cannot survive as dereferenced upstream IO after a call. Ending while a local Job still exists is safe because end does not read those IO pointers, and input/output storage remains alive through the synchronous advance call.
- poll_decode takes an owned Job out only for one poll, puts it back before Pending, and self-wakes. A borrowed future drop resumes the same Job. Busy rejects a second start without resetting the current stream; cancellation is checked before Job work, each advance, and final publication, and any observed cancellation ends the stream and discards the Job. A store occurring after the final cancellation observation is after publication's stated linearization point, not proof of suppressed already-published output. The Arc<AtomicBool> itself does not register/wake an idle transport; source says so.
- The 16-step, 256-compressed-byte and 8192-output-byte quanta bound each poll's upstream work, including many empty blocks. Existing synchronous decode_sync intentionally spins the same state machine without yielding and advertises no cooperative guarantee. Output Vec reallocation/copy is proportional to already produced data, so hard wall-clock preemption/bounded poll latency is NOT established. Encoder remains synchronous and is not part of a decompression scheduling claim.
- Tests contain real partially advanced dictionary-bearing thread transfer, later decoding and final drop migration; a second test transfers across >100000 BFINAL resets without an iteration ceiling; incompressible quantum edges/truncated prefixes, scheduling ticker, external-task cancellation, cancelled borrowed future resumption, Busy preservation, advanced-state allocation failure and limit-vs-boundary distinction. These are meaningful discriminators. Actual execution and non-Sync compile-negative still require parent evidence.

Two actual Close hazards were sent to the source owner and parent; the following text records their original triggers before repair:

1. `finish_close_inner` currently loops over immediately Ready `poll_event` messages. Every call resets poll_event's byte budget. Infinite/sufficiently many small ready messages can therefore monopolize one finish_close future poll; it also appends all such messages to a ready queue without consumer backpressure. Fix requires outer operation quantum and explicit handling of accumulated application events, not a message-size rejection threshold.
2. The pre-response EOF check runs whenever peer_close && !close_sent. close_sent becomes true only after poll_flush returns success. If all Close bytes were already written but poll_flush was Pending and the borrowed future was cancelled, a normal peer can receive Close, send reply and EOF, yet resumed finish_close reads EOF first and incorrectly reports PeerEofBeforeCloseSent. Test write Ready(all), flush Pending, receive peer Close+EOF, cancel/resume; complete existing flush before classifying handshake. This is distinct from rejecting a peer that disconnects before any actual response bytes.

Cached RFC6455 text found at `C:/Users/Luiz/AppData/Local/Temp/opi-http-ws-upstream-assess-4ebxmapq/rfc6455.txt` is zero bytes and was not used as normative proof. Closing checklist above is a review obligation against registered requirements, not a claim this reviewer read that empty artifact. No new RFC/network fetch was performed.

### Independent repair verification

The source owner repaired both hazards; this reviewer re-read actual `src/io.rs` and all 14 tests in `tests/close.rs`:

- New `close_bytes_written` becomes true after the full queued wire is accepted and before poll_flush can return Pending. The pre-response EOF test now runs only before those bytes were accepted. `cancelled_pending_flush_with_all_close_bytes_written_accepts_peer_reply_eof` exercises an AsyncWrite gate with actual Close bytes visible to the peer, Pending flush cancellation, peer reply + EOF, then resume. This distinguishes the original false failure.
- `finish_close_inner` retains at most one receive event; for a non-Close event it returns nonterminal `CloseReceivePending`. Retrying while it remains queued does not read more. Caller drains through next() and reuses the same absolute deadline. CloseReceivePending is specifically exempted from failure cleanup; timeout and transport/protocol errors still release. `finish_close_retains_inflight_application_and_close_events` checks repeat-without-drain and then correct data/Close order. Source state no longer has an unbounded immediately Ready event loop or hidden accumulating event queue.
- Source models Open, LocalCloseQueued/Sent, PeerCloseReceived, ResponseQueued, HandshakeComplete, Closed and Failed; received Close keeps the connection for explicit caller response. Close masking/reason/empty status, simultaneous close, partial application write then Close, late queue rejection, EOF/timeout/protocol failure and Drop are covered by distinguishing tests. Already exchanged Close does not become Closed until shutdown and peer EOF finish.

No additional actual-source blocker was identified in the reviewed private Send/PMD/Close state. All claimed passing results remain contingent on parent execution logs; this reviewer executed no Cargo. Strict walltime/preemption, arbitrary feature unions, complete server role, public product/package/platform acceptance remain unproved. Final test log review is pending parent handoff.

## Audited cached source SHA-256

| Registry-relative path | SHA-256 |
|---|---|
| `libz-rs-sys-0.6.8/src/lib.rs` | `a3e8baef4d10df7b7469bfeb4f0d66e288711d6974c8611b19577dfd743720ee` |
| `libz-rs-sys-0.6.8/Cargo.toml` | `699fa159ed7f380fa42a6987bc76ca7c73263005f0d67d6430b4935e174ee5b9` |
| `zlib-rs-0.6.8/src/inflate.rs` | `dc52b71c986b2b121af2fd678049e6eaac92b30971f39cf41e13702b45a1bc29` |
| `zlib-rs-0.6.8/src/inflate/bitreader.rs` | `0623e37efc4f44e62bd22491eddd6b2a0024ceece867cbcf842aa4568cbb42af` |
| `zlib-rs-0.6.8/src/inflate/writer.rs` | `0140238dd1b000e36fec096f08410cb6d8f036190601669956b6cf0fd3e47253` |
| `zlib-rs-0.6.8/src/weak_slice.rs` | `9cbcb1b44c0d5406c5c713bd2e4c04802b63964707d6b8a6680bb3922e2cf17e` |
| `zlib-rs-0.6.8/src/allocate.rs` | `46c66ec430c4389f7d3f2a84419d296ed14b13fa136791933a27b7191879411e` |
| `zlib-rs-0.6.8/src/c_api.rs` | `7b7baace3688e0c7d715caef6468119dd26f29b6468d8373d5494f86e7c2198f` |
| `zlib-rs-0.6.8/src/stable.rs` | `ba6f0d5ce10aaa274b30103bc86c0766068b2dc62d7faafced1df8c782c0c703` |
| `zlib-rs-0.6.8/Cargo.toml` | `85e89ef7d52508403f921caba72bb16545815e928083b6ab4ffecb8bf65c96b6` |
| `flate2-1.1.10/src/ffi/mod.rs` | `5b82d9100a238356200886d986ecc9dd3d50994010e1dc3367c85ddad8072caa` |
| `flate2-1.1.10/src/ffi/c.rs` | `33eccd974551c3d03d8f100fd729ad5dc1a5265c7556c53bb895f6b66e1daf2f` |

## Additional bounded Text review

Parent identified another algorithmic gap during this iteration: validating an entire plain FIN message in Messages or an entire inflated Text in Core could scan arbitrarily large output in one async poll despite bounded inflate calls. The source owner added private `Messages::new_deferred_text`, plus Core-owned `Pending::Inflate` and `Pending::Text { message, offset }` states. This reviewer inspected the actual revision:

- Public Messages::new/default remain strict, the deferred constructor is pub(crate), and synchronous Core::ingest still validates every Text before output. The changed construction does not publicly expose unchecked text.
- Async processing holds the whole message and validates at most 8192 bytes per poll. A nonfinal incomplete suffix advances by valid_up_to, keeping at most three bytes for retry with new bytes. Cursor remains at a character boundary; invalid complete sequences or a final incomplete sequence fail before any message publication.
- Every incomplete validation self-wakes and leaves the cursor/message owned. Borrowed next cancellation preserves it; following read-buffer bytes remain untouched until this pending message completes. Full output is validated exactly once along the async path.
- Added tests use 16 MiB plain and compressed Text, cancel every Pending borrow and assert observed cursor advances <=8192. They cover 2/3/4-byte scalar crossings at the quantum edge and late invalid/incomplete tails in both forms; the >=255 observed validation-poll check also discriminates restoring eager plain FIN rejection. The public strict-constructor negative is retained.

No additional source blocker identified in this repair. Allocation/deallocation/copy costs remain the disclosed hard-latency limitation; the fix removes an algorithmically unbounded UTF8 scan without claiming real-time preemption. Parent test logs still govern actual pass/fail outcomes.

## Additional continuation-copy review

Parent also identified a complete continuation payload append as algorithmically unbounded work in one async poll. This reviewer read the actual private `Messages::receive_deferred` / `poll_append` implementation and Core integration:

- An owned Append stores the entire completed continuation Frame plus offset. Each poll copies <=8192 bytes using existing fallible append_actual; incomplete work self-wakes. Core polls the append before consuming any more transport bytes. Final assembled messages then enter the same owned PMD/Text path; an unfinished append cannot overlap the next frame.
- Public Messages::receive retains its ordinary synchronous strict behavior. Error poisoning/EOF/release drop append and pending message. Partial copied progress survives cancelled borrowed next futures. A scheduling quantum has not become an accepted message-length limit.
- Per-poll work is a finite composition of at most the append-completion quantum, inflate quantum, UTF8 quantum and transport byte quantum. Many tiny continuations remain subject to transport-byte accounting. Allocation/reallocation/free walltime remains excluded from hard real-time claims.
- Regressions observe append progress across cancellation on 4 MiB binary and ~2 MiB incompressible compressed fragmented Text, preserving prefix and Ping-before/after event order, then exact decoded payload. They distinguish the former eager whole-continuation copy.

No additional actual-source blocker identified. Final runtime logs, three selected dependency graphs and negative compile evidence remain parent-owned verification.

## Final evidence audit (read-only, independently inspected)

Final source identity matches all 31 recorded source/manifests entries in each final three-graph test receipt. Current core SHA-256:

| File | SHA-256 |
|---|---|
| src/frame.rs | 4b46cca1e99462bf81125427eef443ea56030c7a56ea2ba84407611b5415836e |
| src/io.rs | 73437d47d30bccf82ed0aa085d3db85e5d382bcd1035d59ba3098559d5b1c516 |
| src/pmd.rs | 6851e91fdde0c0f76671e90db0af4659a8cbe67499046e66fedd2804cb3fdf3b |

Inspected receipt outcomes and complete test-result summaries:

| Graph | Final test receipt | Passed / failed / ignored |
|---|---|---|
| base | evidence/1792412323942172100-base-test | 76 / 0 / 0 |
| union-default | evidence/1792412348906934700-union-default-test | 76 / 0 / 0 |
| union-c-zlib | evidence/1792412376844897800-union-c-zlib-test | 76 / 0 / 0 |

Total 228 actual test-function executions. The five unit tests plus integration binary counts 14, 12, 2, 20, 7, 10, 3, 3 sum to 76 per graph; zero-test binaries/doc-test results are not added to that count. Latest Clippy, fmt, metadata, resource, cancel-resource and wire command receipts all record successful exits in each of these three profiles.

The final structural comparisons were independently inspected at evidence/1792412422833531300-base-compare, evidence/1792412373635974200-union-default-compare and evidence/1792412400343961100-union-c-zlib-compare. Each archived rust.stdout has 72413 rows, with 1738 valid and 70675 invalid cases, zero mismatches. These are 72413 DISTINCT input cases repeated in three graphs (217239 observations), not 217239 independent inputs. All three archived output hashes equal bf949a121759c3cda75c2458efab0989a62981e3a93b59aa5f9f639b72a31f5f. The oracle hash is 4629de7b1cc02382595ce445bfd989f244c5e08ff6bc7329c1f40a78d9294063. Both implementations share the selected structural boundary rule, so this is implementation differential evidence, not an independent formal proof of all DEFLATE streams. Note that the compare command receipt's binary_sha256 identifies its Python launcher; the actual boundary binary hash is in compare.stdout, so those different hashes are not a mismatch.

Inspected all five mutant receipts: each compiled and reached a runtime assertion failure (one failed test, exit 101), not an unrelated build failure. Mutants remove inflate yielding, ignore active cancellation, forget fully written Close before flush, eagerly validate whole Text, and eagerly copy a whole continuation. Each modified only its intended core file; original/restored hashes match and the final current hashes match those originals. The final three passing full suites ran after restoration.

The non-Sync consumer produces the expected E0277 on Decoder, and the allocator feature-union consumer produces the exact upstream rust-allocator/c-allocator compile_error. These are intended compile negatives, not failed supported graphs. Read final metadata: base has 19 packages, default union 24, C-zlib union 27. All selected compression packages are registry sources at reviewed versions; direct Decoder uses libz-rs-sys rust-allocator/std in all three. Baseline/default union encoder selects zlib-rs; C union enables any_c_zlib and libz-sys 1.1.30 with its native build script. No Ratchet package is in these graphs.

Finite resource logs show the 1 TiB and isize::MAX header declarations make zero payload allocations; invalid PMD/cancellation free owned backend/output, and after Drop Rust allocator observations return to baseline. The small live owner residue before Drop is not claimed as a leak. These observations do not establish native/all-process OOM recovery, all leaks, or hard walltime bounds.

Beyond reading the parent's 29-package/2545-member registry-verification record, this reviewer independently opened the three critical compression crate archives WITHOUT extraction and compared all 131 published regular files byte-for-byte with cache: libz-rs-sys 15, zlib-rs 53, flate2 63, zero mismatches. Archive SHA-256 values match the lock/provenance record. The auditor did not modify the registry/cache or run Cargo.

Final conclusion: Standards/source-safety review PASS for the declared isolated client prototype and exact selected dependency profile; tested lifecycle/PMD specification scope PASS. No unresolved blocker remains in this audited mechanism scope. Product adoption remains INCOMPLETE pending admitted contract/placement, genuine Upgrade/provider/package consumers and platform/MSRV evidence. Hidden API upgrades/local unsafe ownership, full WS protocol maintenance and the documented unsupported feature combinations are continuing obligations. No full server-role, Miri/sanitizer, hard preemption, arbitrary feature-union or 20.11/20.34/20.57 acceptance is inferred.

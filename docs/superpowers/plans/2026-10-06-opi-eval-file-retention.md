# File records and latest-three evaluation retention

The user rejected SQLite and requested only the latest three evaluations of each
kind. This supersedes the database and age/quota retention parts of the previous
storage plan; the existing native evidence and source-ownership boundaries remain.

**Completion:** queries and baseline lookup use external JSON records without a
database or durable query cache; finished evaluations apply the configured
latest-three rolling policy; active/pinned/referenced/unknown-cleanup evidence
is protected and any exception is reported. No paid run, existing-user cleanup,
commit, Cargo change or ledger write is included in implementation/testing.

The user confirmed grouping: action, benchmark-family combination,
model/API/controls/thinking and task scope, with a separate pi release bucket;
Opi commits, Opi release versions and Companion build hashes do not reset history.
Rank ended history by completion time, with creation time/identity as fallback;
score never changes the order. The just-finished result remains protected while
its retention summary is published.

- [x] Replace SQLite Store persistence/query with bounded JSON-record discovery
  and direct identity lookup -> verify: catalogue test binary plus subprocess with
  sqlite3 unavailable, missing/corrupt former database ignored and never rewritten.
- [x] Replace age/count/quota retention with latest-three grouping and explicit
  rolling cleanup of owned finished evidence -> verify: lifecycle negatives,
  four-run pruning, changing Opi source identity, separate pi/model buckets,
  retained references and active leases.
- [x] Simplify sole template/config, wire post-result retention handoff, remove
  database/reindex CLI claims -> verify: operator/storage test binaries, offline
  negative CLI and Windows/WSL affected filesystem checks.
- [x] Synchronize EN/ZH manuals, skill references, crate docs and Unreleased;
  record independent review, test impact and carried-in changes -> verify:
  doc checker, doc-check tests and git diff --check.

Tests stay isolated in disposable fixtures. Native scoring, immutable retained
seals, precise retry selection and one grouped live deadline remain unchanged.

## Verification and review

The independent review reproduced missing-completion discovery-time drift,
mixed-offset baseline ordering, closed failed-comparison over-retention and
single-family result redirection through a foreign record. Regression checks
first failed, then passed after fixes. The reviewer reran nine focused cases
and reported no remaining consequential findings in scope. The current
evaluation's global lease now remains held through retention-summary publication;
input leases can settle into immutable reference protection before maintenance.
Closed pre-live preparation failures and offline comparison failures count as
finished history. Unknown process/stream settlement remains protected.

| Offline binary | Windows | WSL |
| --- | --- | --- |
| Operator | 44 passed | 44 passed |
| JSON catalogue | 24 passed | 24 passed |
| Lifecycle | 52 passed, one POSIX-only skip | 52 passed |
| Storage/config/flow | 21 passed | 21 passed |

Compilation of the changed Python modules passed. The flow suite executes actual
offline subprocess fixtures: five finished pi evaluations retain the latest
three native payloads, pins report retained excess and release after unpinning,
closed comparisons roll their owned wrappers while caller output survives, and
unsettled verification commands remain protected. No provider/Docker/build run
or cleanup of existing user evidence was performed.
Both Windows and WSL documentation checkers passed; all 26 documentation-check
tests passed on both systems. `git diff --check` passed after the final updates.

Test impact: **add** the catalogue/lifecycle/storage binaries and their isolated
cases; **update** the operator runtime-location and UTC selection checks plus
fixture material pins; **retain** existing operator/native validity and retry
coverage; **delete** none. Rust/Cargo and the implementation ledger are unchanged.
Existing research relocation/deletions, deleted decoupling docs, `.gitignore`
changes and other carried-in work remain preserved. No staging or commit is part
of this task; HEAD remains `c0f04089001db25342d405d707826ff014603577`.

# pi-sdk-conformance — early external-consumer package proof

Harness for task 20.8 (DEL-04 dependency substrate) and reused at
task 20.57 (final assembled package behavior). It proves that a fresh,
workspace-external ordinary Cargo consumer can resolve and actually use
the opi-vendor Unicode16/ECMA/URL namespace graph from distributable
package identities, alongside ordinary upstream dependencies, with no
root patch, no TEMP path dependency, no library lockfile and no
build-time generator.

## Layout

- `sparse_registry.py` — minimal cargo sparse-index registry server
  (localhost HTTP) driven by `manifest.json`.
- `prepare_closure.py` — verifies every pinned archive in
  `third_party/unicode16-ecma-url/`, reproduces the Unicode-16 table
  generation (twice, determinism-checked, hash-pinned), materializes
  the namespace family and regress fork (per-member byte-integrity
  checks), packages all crates and publishes them into the local
  registry in dependency order.
- `run_early_package_proof.py` — orchestrates the full proof: prepare,
  fresh scratch consumer with fresh `CARGO_HOME` + its own target dir,
  `cargo build` + battery run, verdict and evidence.
- `consumer/` — the ordinary consumer crate template (leaf battery
  sources are embedded from `frozen/`).
- `frozen/` — research-frozen leaf cases/expectations (43/43 Node-agreed)
  and the derived Unicode-16/17 script probes (committed; every run
  verifies the committed copy matches regeneration).
- `evidence/` — committed evidence bundles from the authorizing runs,
  including the red run against an empty registry.

## Run

```bash
python3 scripts/pi-sdk-conformance/run_early_package_proof.py \
    --scratch /tmp/pi-sdk-proof --port 8173
```

Preparation fetches ordinary crates.io dependencies (official network);
the behavioral commands run against the local registry only. Exit 0 +
`"verdict": "PASS"` is the passing condition.

## Battery

1. 43 frozen leaf cases: 14 ECMAScript `Number::toString` binary64
   boundaries via ryu-js 1.0.2 (safe Buffer API), 29 WHATWG
   `URL.canParse` accept/reject via the family `url`; compared against
   the frozen research expectations.
2. Unicode 16/17 script-class differential: classes added in UCD 17
   (Beria_Erfe, Sidetic, Tai_Yo, Tolong_Siki) must match their code
   points under stock regress 0.12.0 and not under the
   `opi-vendor-regress` fork; shared scripts must match under both.
3. Coexistence: the ordinary url 2.5.8 / idna_adapter 1.2.2 / ICU 2.2
   chain and the family URL/IDNA/ICU 2.0 chain both parse a Unicode
   domain in the same graph.

## Known proof-registry artifact

Family packages' ordinary crates.io dependencies carry the sparse
crates.io index URL in the local registry index; cargo 1.97
canonicalizes its default crates.io source differently, so the proof
graph contains same-version duplicates of those ordinary crates. On a
real single registry all dependencies resolve from one source; the
artifact does not affect any battery result.

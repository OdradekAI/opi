# opi-vendor Unicode16/ECMA/URL dependency closure

Controlled private dependency distribution closure for the opi-ai source
dialect (DEL-04, task 20.8). This directory pins the exact inputs; the
harness under `scripts/pi-sdk-conformance/` materializes, packages and
proves them. This is dependency substrate only — no product crate code
depends on it yet; task 20.57 reuses the proof for the final assembled
package.

## Composition

| Identity | Base | Version | Change |
|---|---|---|---|
| `opi-vendor-url` | url 2.5.8 | 2.5.8 | manifest identity only |
| `opi-vendor-idna` | idna 1.1.0 | 1.1.0 | manifest identity only |
| `opi-vendor-idna-adapter` | idna_adapter 1.2.1 | 1.2.1 | manifest identity only |
| `opi-vendor-icu-collections` | icu_collections 2.0.0 | 2.0.0 | manifest identity only |
| `opi-vendor-icu-locale-core` | icu_locale_core 2.0.0 | 2.0.0 | manifest identity only |
| `opi-vendor-icu-normalizer` | icu_normalizer 2.0.0 | 2.0.0 | manifest identity only |
| `opi-vendor-icu-normalizer-data` | icu_normalizer_data 2.0.0 | 2.0.0 | manifest identity only |
| `opi-vendor-icu-properties` | icu_properties 2.0.0 | 2.0.0 | manifest identity only |
| `opi-vendor-icu-properties-data` | icu_properties_data 2.0.0 | 2.0.0 | manifest identity only |
| `opi-vendor-icu-provider` | icu_provider 2.0.0 | 2.0.0 | manifest identity only |
| `opi-vendor-regress` | regress 0.12.0 | **0.12.0-ucd16.1** | identity + regenerated UCD 16 tables |

"Manifest identity only" means every archive member stays byte-identical
except `Cargo.toml`, which gains the vendor package name plus
`package`/`registry` redirects on family-internal dependencies (keeping
the original dependency keys, so crate-internal `use` paths are
untouched). The regress fork additionally swaps
`src/unicodetables.rs` and `tests/unicode_property_escapes.rs` for
tables regenerated from official UCD 16.0.0 with regress's own upstream
generator at the pinned commit; the fork version is a pre-release
(`0.12.0-ucd16.1`) and is never labeled as unmodified crates.io
regress 0.12.0 (whose tables are UCD 17).

`ryu-js 1.0.2` is reviewed but **not** renamed: pure Rust, no C and no
`build.rs`, internal `unsafe` isolated behind the safe `Buffer` API; it
resolves as an ordinary upstream dependency.

## Controlled inputs

`closure.json` pins all 18 inputs (archives + UCD/emoji data) with
sha256, copied verbatim from the frozen research assets under
`docs/research/pi-ai-opi-ai-alignment/offline-probes/readiness-schema/`.
`archives/` holds the exact bytes. The Unicode-16 generation is
reproduced from the pinned upstream generator over the pinned UCD
16.0.0 + emoji 16 data, run twice per invocation and compared to the
frozen table hash — no build-time generation ever enters a published
package.

## Review posture

- **Features**: the URL/ICU family keeps upstream feature tables; the
  family-internal `writeable` dependency is pinned to `=0.6.3` with
  `alloc` (0.6.4 changed macros that ICU4X 2.0.0 code expects; the pin
  mirrors the frozen research graph).
- **Native code / proc macros / build scripts**: no new C, no new
  proc macros, no `build.rs` enters any family package. The family's
  ordinary upstream dependencies (e.g. zerovec, yoke derive crates)
  keep their upstream properties and are reviewed as upstream.
- **MSRV**: family packages declare rust-version ≤ 1.82 (upstream
  values preserved); the workspace MSRV 1.97 covers them. The
  generator/preparation graph is a separate pinned tool graph, not a
  product dependency.
- **License/NOTICE**: every package keeps its upstream license files
  inside the archive; `NOTICE` records the fork relationship.

## Upgrade ownership

Owned by the opi-ai maintainers. To upgrade an input: replace the
archive in `archives/`, update `closure.json` (version, sha256), rerun
`scripts/pi-sdk-conformance/run_early_package_proof.py`, and commit the
refreshed evidence. The proof fails closed on any pin mismatch,
nondeterministic generation, archive-member drift beyond the documented
transform, or battery regression.

## Publication status — OPEN

The `opi-vendor` identities exist today only in a **local sparse
registry** built by the proof harness. Real registry publication is a
separately authorized step and remains blocked until then (the task
stops here by contract). Packaged artifacts from the local proof embed
the localhost registry URL and must be repackaged against the target
registry at publication time. In the local proof, ordinary crates.io
dependencies of family packages carry the sparse crates.io index URL,
which cargo treats as a distinct (identical-content) source — on a real
single registry all dependencies resolve from one source and this
duplication does not arise.

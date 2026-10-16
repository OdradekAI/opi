# Transport/auth readiness experiments

These are disposable research harnesses, not opi-ai code or a second acceptance matrix. The [report](../../pi-1.3.0-readiness-transport-auth.md) distinguishes design decisions from OPEN implementation/acceptance work. All values controlling child containment, fixtures, synthetic keys, response sizes and loopback ports are test inputs, not SDK budgets.

`prepare-npm.py PI_PACKAGE_LOCK TEMP_DIRECTORY` is the only npm preparation step. It explicitly retrieves the 23 locked production archives and verifies every SHA-512 integrity before safe regular-file extraction. No npm install, package scripts or Bun auto-resolution. Keep the emitted `preparation.json`; its archive/member digests are the replay input. Preparation uses the network; experimental children do not need it. Rust archives must already be present in Cargo's cache.

Main replay on the recorded Windows host:

```powershell
python run.py --node D:/nvm/v24.13.0/node.exe --npm C:/Users/Luiz/AppData/Local/Temp/opi-pi130-ready-transport-auth/npm --pi D:/Luiz/Odradek/opi/.repo/pi-1.3.0 --scratch C:/Users/Luiz/AppData/Local/Temp/opi-pi130-ready-transport-auth/new-replay
```

This copies the isolated manifest, original lock and Rust source into scratch, builds `--locked --offline`, records metadata/features, generates disposable synthetic inputs with Python cryptography43.0.0 and runs Node transport/proxy probes. It uses the configured existing shared Cargo target; no target-dir override or clean. Node executable hash, Pi lock and all prepared member hashes must match before commands run. A wrong `--expect-node-sha256` was actually verified to exit1 before creating scratch. `raw/summary.json`, `raw/commands.json`, `raw/results.jsonl`, `raw/proxy-results.jsonl`, `dependency-graph.json` and the original `Cargo.lock` record the final six-command replay.

`header-probe.mjs NATIVE_EXE` runs under fixed Node or `bun --no-install`; it compares legal99/100/101/125/200-field ordinary200 responses. Original native accepts99/100 and rejects101/125/200; the temporary Hyper response-slot patch accepts all five. `hyper-response-slots.patch` and `patch-hyper.py` preserve the exact isolated source change; `Cargo.patched.lock` records that graph. The production design additionally requires trailer storage and head-buffer policy fixes; the prototype does **not** claim to implement them. Replay copies exact Hyper1.9.0 into TEMP, applies the script, uses a separate manifest with `[patch.crates-io] hyper={path="TEMP_COPY"}`, renames package/bin to `opi-research-transport-auth-patched`, copies the patched lock, then builds `--locked --offline` using the same existing target. The root workspace is unchanged. Original Hyper archive checksum/member equality and source hashes are in `source-evidence.json`.

`openssl-libctx-probe.py DLL NPM_ROOT FIXTURES NODE` runs in a fresh stdlib-only Python child. Inputs are generated synthetic keys, with an extra2048 three-prime key made by the recorded OpenSSL CLI; none are committed. Set `OPENSSL_CONF` to a synthetic file with:

```ini
openssl_conf = openssl_init
config_diagnostics = 1
[openssl_init]
alg_section = algorithm_section
[algorithm_section]
default_properties = provider=OPI_SYNTHETIC_MISSING_PROVIDER
```

Set `OPENSSL_MODULES` to a nonexistent scratch directory. The private-context decoder and signing calls must all succeed before a deliberate default-context SHA256 fetch fails; private signing must still succeed afterwards. `openssl-reference.cjs` independently uses the locked actual Google gtoken/JWS implementation and verifies signatures under fixed Node. The three negative-input cases use malformed PEM, encrypted PKCS8 without passphrase and a non-RSA EC key; both first two fail in the reference, while the last exposes the reference's wrong-algorithm behavior. The probe sets a callback that always refuses passwords, invokes only memory decoding and does not call global NO_LOAD_CONFIG/context setters. C API success is not a Rust wrapper compilation/safety proof, native delivery proof or production OpenSSL version recommendation.

`source-evidence.json` binds exact vendor files and manifests; `npm-preparation.json`, `native-source-acquisition.json` and `candidate-acquisition.json` record preparation URLs/bytes/digests. Runtime root fingerprints are observations of the pinned clean Node process, not an implementation of a package trust-store updater. Formal source locks, distributable bundle, native build and all-platform validation stay OPEN.

`mtls-fixtures.py SCRATCH` and `mtls-supplement.mjs SCRATCH NATIVE_EXE` add a fresh synthetic CA and client P521/2048e3/1024 identities. Under fixed Node, the first two really authenticate over H2 while native ring's key loader rejects;1024 is rejected by Node itself and is not a parity-positive input. These three records expose why the selected design needs the public rustls SigningKey/algorithm/KX extension mechanisms backed by mature private OpenSSL EVP. They do not execute that future Rust TLS adaptation. Native helper unwrap exit101 on unsupported keys is preserved as a research observation, not a desired product failure mode.

`harness-diagnostics.json` preserves the scope of rejected investigation runs; `raw/initial-diagnostic.jsonl` is the earlier builder/ALPN diagnostic, excluded from final counts. Some setup/summary failures are transcript summaries rather than complete raw subprocess archives and are labelled accordingly. The final raw records and exact source pin inventory are complete. No product files, real accounts, paid services or live metadata endpoints are used.

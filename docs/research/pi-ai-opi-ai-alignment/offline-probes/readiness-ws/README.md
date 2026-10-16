# WS readiness research assets

Current selected length/storage patch and the **only current replay route** are in [memory-only/README.md](memory-only/README.md). Under the latest authorization, execute no socket/connection/request/download code; the commands below describe the earlier six-line-patch experiment and must not be used to claim a rerun of the larger current patch. Its 80 observations remain historical.

Research only. The decision and remaining implementation coverage are in [the WS report](../../pi-1.3.0-readiness-ws.md). `results.json` is the final 80-observation run; `results-unpatched80.json` preserves the same protocol cases without the six-line frame patch, and `results-unadapted-mixed-failure.json` preserves the mixed-message decoder failure before its public adapter. The private patch is explicitly PMD-only.

Versions and exact hashes are in `retrieval.json`, `source-pins.json`, `patch-pins.json`, `dependency-pins.json`, and `rust/Cargo.lock`. No archives, extracted third-party source, executable or target tree are stored here. Initial dependency preparation used official network sources; the experiments subsequently used loopback and offline Cargo builds. Bun always uses `--no-install` in `run.py`. Runtime child environments omit real auth/proxy injection.

Prepare a new absolute TEMP workspace, from cached archives by default:

```powershell
python docs/research/pi-ai-opi-ai-alignment/offline-probes/readiness-ws/prepare-replay.py --workspace C:/Users/Luiz/AppData/Local/Temp/opi-ws-independent-replay --build
```

The workspace must not exist. Missing cached source archives fail; `--allow-download` explicitly permits only the five pinned official archives. Other locked Cargo dependencies must already be prepared in the shared cache; the build itself remains `--locked --offline`. `prepare-replay.py` checks the source hash before/after the patch and propagates build failure. It uses the existing global Cargo target setting; never clean or create a per-session target.

After successful preparation, run `run.py`, then `summarize.py`, `deflate-distance.py`, and `drop.py` in that TEMP workspace with the fixed Python runtime. The runner requires exact local Node/Bun paths and verifies both executable SHA values. It writes new raw evidence into TEMP. `summarize.py` checks 80 rows and 12 successful owned-handshake positive cases; the distance scanner checks actual window8 streams. `drop.py` destroys whole connections after incomplete-frame read containment; it does not demonstrate cancellation-and-reuse safety.

Negative preparation check:

```powershell
python docs/research/pi-ai-opi-ai-alignment/offline-probes/readiness-ws/prepare-replay.py --workspace C:/Users/Luiz/AppData/Local/Temp/opi-ws-pin-negative --expected-before wrong
```

Expected: nonzero exit and no workspace creation. All deadlines, byte counts and header counts in the fixtures are experiment controls or inputs, never SDK defaults. `excluded-exploration.json` documents a discarded stale-binary run following an encoding-related build failure; only `results.json` is the final correctly rebuilt run.

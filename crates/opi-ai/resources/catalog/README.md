# Maintain the bundled model catalog

[简体中文](README.zh.md)

## Edit and check the source

Edit the [provider JSON files](providers/).
Each file is named for a provider ID. Within that file, `(type, id)` identifies
the record. Use a top-level `x-<name>` for opaque metadata. The persistence
codec reserves `x-opi`. A change to a model's wire capabilities also needs
a fixture at the adapter that uses it.

Run these commands from the repository root with the shared workspace Cargo cache.
`build` writes a new candidate directory for review. `check` compares the
checked-in bundle with its source files.

```sh
cargo run -p opi-ai --no-default-features --example build_catalog -- build crates/opi-ai/resources/catalog/providers catalog-candidate
cargo run -p opi-ai --no-default-features --example build_catalog -- check crates/opi-ai/resources/catalog/providers crates/opi-ai/resources/catalog/builtin
```

Review and copy the generated files to
[`resources/catalog/builtin`](builtin/).
The builder generates `catalog.json`, `manifest.json` and an
`openrouter-profile.json` from the Chat Completions entries used by the
standalone OpenRouter adapter. After applying the candidate, rerun `check`
and exercise the affected listing and adapter tests.
[Select the relevant tests](../../tests/README.md)

## Compare or follow a Pi release

Use `python scripts/pi-catalog/pack.py --help` for a read-only comparison with a pinned
publication artifact or a verified local provider export. Record the
reference revision and the opi bundle digest with the result.

For a provider update, use the **last adopted** publication for that provider
as the old input and the **target release** as the new input. Each pin
file contains the `revision` of its publication as `sha256-<digest>`.

```sh
python scripts/pi-catalog/sync.py --provider mistral --old-artifact old-catalog.json --old-pin old-pin.json --new-artifact new-catalog.json --new-pin new-pin.json --opi-sources crates/opi-ai/resources/catalog/providers --output-dir sync-candidate
cargo run -p opi-ai --no-default-features --example build_catalog -- build sync-candidate/providers bundle-candidate
```

Review the report before applying the selected provider file and rebuilt
bundle. Local-only records and independent metadata edits survive the
three-way merge. Exit 2 reports conflicts and writes no candidate. Exit 1
reports invalid input or output. Apply upstream removals consistently across
built-in entry points for that provider and operation. Keeping a retired
ID or adding a fallback requires explicit approval from the project owner.
Record the adopted publication revision for the next update.

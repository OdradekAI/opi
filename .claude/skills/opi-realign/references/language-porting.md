# Language-Native Alignment Guide

Use target-project behavior as a semantic reference, not as a file or package
blueprint. A port or reimplementation should preserve user-visible and
integration-critical semantics while adopting the current language's strengths.

## Cross-Language Principles

- Preserve concepts before APIs: event order, lifecycle, error semantics, data
  durability, and user workflows matter more than class or file names.
- Keep ownership native: types should live in the crate/package/module that
  owns their semantics; avoid dumping shared types into a central hub unless
  there is a real cyclic-dependency problem.
- Match dependency direction to the current ecosystem's norms.
- Prefer explicit typed boundaries in compiled languages; prefer runtime
  adapters or schemas only where dynamic extension is required.
- Preserve compatibility only when it is a stated goal. Otherwise call the
  relationship semantic alignment.

## Common Translation Checks

| Source pattern | Current-language question |
|---|---|
| Dynamic plugin runtime | Should this become subprocess RPC, static traits, WASM, dynamic loading, or package metadata? |
| Declaration merging / structural typing | Should this become enums, traits, sealed types, or versioned protocol structs? |
| JSON schema at runtime | Should the current language generate schemas from typed inputs or validate untyped boundaries only? |
| Monorepo package split | Should package boundaries stay conceptual, collapse into modules, or split into crates/libraries? |
| UI renderer | Should the implementation use the current ecosystem's terminal/web/native UI stack instead of copying the target renderer? |
| Async/cancellation model | How does the current language express cancellation, backpressure, retries, and cleanup safely? |
| Config/session formats | Should the current project preserve target files, define a native format, or provide import/export only? |

## Red Flags

- One-to-one package mapping is treated as a success metric without checking
  dependency direction and ownership.
- A product crate starts owning generic runtime semantics because it is easier
  to wire quickly.
- Core runtime absorbs workflow features that the target project keeps in
  extensions, examples, or packages.
- Broad ecosystem parity lands before the seams it depends on are stable.
- Compatibility wording implies users can share files/config/plugins between
  projects when only semantic inspiration exists.

## Good Outcomes

- The current project keeps target-aligned behavior at user and protocol
  boundaries.
- Internal architecture is idiomatic for the current language.
- Load-bearing seams deepen before ecosystem breadth expands.
- Non-goals are explicit and test/docs guard against accidental parity claims.

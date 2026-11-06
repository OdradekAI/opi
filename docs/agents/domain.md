# Domain documentation

Single context, stored under `docs/`:

- Canonical glossary: [docs/GLOSSARY.md](../GLOSSARY.md).
- Decisions: [docs/adr/](../adr/README.md).

For Matt skills, resolve references to root `GLOSSARY.md` to
`docs/GLOSSARY.md`. Do not create a second root glossary or a glossary map
just because this Rust workspace has several crates. This project configuration
overrides default file-location conventions, not the skill's user gates.

Read relevant terms when domain language matters. Challenge an overloaded or
conflicting term with a concrete example; after the user resolves the meaning,
update it immediately. Definitions describe meaning, not implementation,
roadmaps, acceptance matrices or permission to build a feature. New terms are
added only when needed by current work.

ADRs record why. Use the next unused number, a short decision and rationale,
and a status (`proposed`, `accepted`, `deprecated`, or `superseded` with
successor). Read the relevant decision, not all ADRs. A changed accepted decision
needs the user's resolution and an explicit replacement link; it does not force
an update to a global spec. Keep an existing bilingual pair synchronized.

# Documentation map

[中文](README.zh.md)

Start with the task, not a reading list. Open the relevant owner below, then
follow only the links needed for the decision. This index is not a product spec.

| Question | Owner |
|---|---|
| How should an agent work here? | [Root guidance](../AGENTS.md) |
| How do we design, code and verify? | [Engineering](engineering/README.md) |
| How do we align an iteration and resolve drift? | [Context lifecycle](agents/context.md) |
| What does a domain term mean? | [Glossary](GLOSSARY.md), [consumer rules](agents/domain.md) |
| Why was a consequential choice made? | [ADR index](adr/README.md) |
| What is this iteration trying to deliver? | Its user-selected brief under [iterations](iterations/README.md) or an explicitly supplied issue |
| Where do Matt flows write maps and tickets? | [Local tracker](agents/issue-tracker.md) |
| What does the product do now? | Root/crate READMEs, source, schemas, tests and generated help |
| Which platform checks apply? | [CI policy](engineering/ci.md) ([中文](engineering/ci.zh.md)) |

## Evidence — load on demand

`docs/research/` holds research reports and snapshot comparisons. Select a
report by question and source identity, and verify its claims against current
sources before adopting a recommendation.

List the archived 2026-11-06 comparison and its supporting evidence in Git.

```text
git ls-tree -r 8030700c207cb9dc181eb824a89cc4dc85050af9 -- docs/realign
```

## Maintenance

Current docs change with the owning code/decision. Completed briefs remain
history; their durable conclusions move to the glossary, relevant ADR or owning
product documentation. No global roadmap or second implementation ledger is
created by this layout.

Agent-facing engineering/context/tracker/glossary files use one English canonical
copy to avoid another synchronization surface. This map has a Chinese counterpart;
existing bilingual product documents remain bilingual.

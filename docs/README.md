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

`docs/realign/` and `docs/research/` hold observations and proposals, not
implicit approval. Select a report by question and actual source identity, not
merely by latest filename. Read only the relevant report and verify its claims
against current sources before adopting a recommendation.

## Maintenance

Current docs change with the owning code/decision. Completed briefs remain
history; their durable conclusions move to the glossary, relevant ADR or owning
product documentation. No global roadmap or second implementation ledger is
created by this layout. See [ADR-0002](adr/0002-task-scoped-context.md) for the
migration trade-off.

Agent-facing engineering/context/tracker/glossary files use one English canonical
copy to avoid another synchronization surface. This map and governance ADR have
Chinese counterparts; existing bilingual product documents remain bilingual.

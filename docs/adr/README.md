# Decision index

Read only decisions relevant to the current change. An accepted decision records
an intended trade-off, not proof that all planned behavior shipped. If code,
current requirements and a decision disagree, use [conflict resolution](../agents/context.md#resolve-a-conflict-where-it-occurs).

| Decision | Applicability |
|---|---|
| [0001 — SDK authentication placement](0001-sdk-auth-placement.md) ([中文](0001-sdk-auth-placement.zh.md)) | Accepted historical design; Phase 20 scope, backend/version pins and delivery claims must be revalidated before reuse. Not a new-work admission gate. |
| [0002 — Task-scoped context](0002-task-scoped-context.md) ([中文](0002-task-scoped-context.zh.md)) | Accepted documentation/workflow authority model; supersedes the global spec admission hierarchy. |

Add a decision only for a costly-to-reverse, non-obvious choice with real
alternatives. Use the next unused number and a short rationale. Mark proposed,
accepted, deprecated or superseded (link successor); preserve old rationale.
A routine refactor needs no ADR. [Domain rules](../agents/domain.md) own usage.

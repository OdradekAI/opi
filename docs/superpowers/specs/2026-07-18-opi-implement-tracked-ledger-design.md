# Tracked opi-implement Ledger Design

**Date:** 2026-07-18
**Status:** Approved for implementation
**Supersedes:** The untracked-ledger policy in
`2026-05-20-opi-implement-skill-design.md`

## Problem

`.opi-impl-state.json` was treated as gitignored runtime state. A newer ledger
existed only in a feature worktree, so merging the feature branch did not move
the ledger into `main`. Removing the merged worktree then removed the only
current copy.

The last validated lost ledger is independently identified by the task record:

- validated at `2026-07-16T19:52:31.848Z`;
- 286,320 bytes;
- SHA-256
  `021299403f0ef7698df425aaeaa313db13472b521f5b41338902f4fb3ec10c24`;
- Phase 14 tasks `14.1` through `14.13`;
- 13 task-summary entries in `phase_exit.14`;
- Phase 14 exit incomplete, with `SC1`, `SC2`, and `SC3` not met, 15 criteria
  met, and residual `R1` deferred by updated design.

The original bytes are no longer available as a standalone file. During
implementation, the final atomic candidate was found byte-for-byte in the Codex
task record's `patch_apply_end` metadata, so recovery can restore the identical
286,320-byte payload and independently confirm its recorded hash.

## Decision

Track the canonical `.opi-impl-state.json` in Git.

Continue to ignore these transient or audit-only files:

- `.opi-impl-state.json.tmp`;
- `.opi-impl-state.draft.json`;
- `.opi-impl-state.corrupt-*.json`;
- recovery candidates and replacement backups.

The canonical ledger remains writable only through the opi-implement atomic
write protocol. Git tracking changes durability and handoff behavior; it does
not authorize manual JSON edits.

## Recovery

Extract the final recovery candidate from the Codex task record, then
cross-check it against four evidence classes:

1. the current valid main-worktree ledger;
2. task definitions and acceptance ownership from the reviewed Phase 14
   remediation design and plan;
3. durable `Opi-*` footers from task commits `14.8` through `14.13`;
4. the recorded atomic-update chain and final phase-exit observations from the
   Codex task record.

The recovered ledger must preserve the current ledger's Phase 1-13 archive
index and restore:

- Phase 14 tasks `14.8` through `14.13`;
- each task's passing commit, evidence, acceptance status, and relevant runtime
  notes;
- the final Phase 14 verify-run entries;
- the incomplete Phase 14 exit record and criteria trace.

The extracted candidate must have SHA-256
`021299403f0ef7698df425aaeaa313db13472b521f5b41338902f4fb3ec10c24`,
then pass BOM-less strict UTF-8 and `ledger-guard.ps1` validation. It is
installed with optimistic concurrency against the observed SHA-256 of the
current target. The pre-recovery target is retained as an ignored audit backup
until the recovered file passes all checks.

The later `14.14` through `14.21` alignment graph is not fabricated during
recovery. After restoration, opi-implement detects the changed registered
design and performs its normal drift-reconciliation and human review gate.

## Commit Model

The task implementation commit and ledger checkpoint are separate:

1. Commit task-owned source, tests, or documentation with the required
   `Opi-*` footers.
2. Read the resulting task commit SHA.
3. Atomically update the ledger to record the task as passing and attach that
   SHA.
4. Commit only `.opi-impl-state.json` in a dedicated checkpoint commit.

The checkpoint message is:

```text
chore(opi-implement): checkpoint task <id> ledger
```

This avoids a circular dependency in which a ledger inside the task commit
would need to contain that commit's not-yet-known SHA.

Phase-B and failed-attempt writes may leave the tracked ledger dirty. They are
not committed as standalone progress noise. A successful task creates one
checkpoint commit after its task commit. Blocked-task handoffs, phase-exit
updates, and reviewed graph reconciliations create a checkpoint commit because
they are durable coordination boundaries.

## Worktree and Merge Safety

Before a worktree can be removed, the finishing workflow must verify:

- `.opi-impl-state.json` is not modified, staged, or untracked in that
  worktree;
- every ledger checkpoint commit needed by the branch is contained in the
  destination branch;
- no `.opi-impl-state.json.tmp` candidate remains.

If any check fails, cleanup stops and prints the exact unresolved ledger state.
Forced cleanup is not part of the supported workflow.

Parallel branches may conflict in the canonical ledger. Such conflicts are not
resolved by choosing one side. The destination branch's ledger and both
branches' `Opi-*` commit evidence are reconciled through the opi-implement plan
path, then validated and checkpointed.

## Sensitive-Content Policy

The tracked ledger may contain task metadata, repository-relative paths,
redacted summaries, verification commands, and commit identifiers. It must not
contain:

- API keys, access or refresh tokens, authorization headers, passwords, or
  private-key material;
- raw provider request or response bodies;
- unredacted tool output, user session content, or credential-store payloads;
- secrets copied from environment variables or local configuration.

The ledger guard must reject known sensitive key names and credential patterns
before a tracked ledger can be installed or staged. Tests cover positive and
negative examples. Existing UTF-8, schema, size, string-length, and mojibake
checks remain mandatory.

## Rule Synchronization

Implementation updates the active policy surfaces:

- `AGENTS.md` and `CLAUDE.md` in lockstep;
- `.gitignore`;
- `.claude/skills/opi-implement/skill.md`;
- `references/initializer.md`;
- `references/ledger-schema.md`;
- `references/anti-patterns.md`;
- `.claude/skills/README.md` and `.claude/skills/README.zh.md`.

Historical plans remain historical. The original opi-implement design receives
only a short supersession note pointing to this decision; its original
rationale is not rewritten.

## Verification

Recovery succeeds only when:

- the guard validates the recovered canonical ledger;
- the task set contains exactly `14.1` through `14.13` before drift
  reconciliation;
- tasks `14.8` through `14.13` point to their durable Git commits and have
  matching `Opi-DoD-SHA256` evidence;
- all restored acceptance scenarios have the recorded final status;
- `phase_exit.14` remains incomplete with the recorded criterion counts and
  dispositions;
- Phase 1-13 compact archive summaries are unchanged;
- `.opi-impl-state.json` is no longer ignored;
- transient ledger files remain ignored;
- the sensitive-content guard tests pass;
- English and Chinese skill documentation agree;
- unrelated user changes remain untouched.

No commit or push is created as part of this recovery unless the user requests
it separately.

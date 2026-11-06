# Batch RUN — tools, execution env, context assembly, trust, compaction (Dims 8,10,14)

pi side: `packages/coding-agent/src/core/{tools/ (all built-in tools),
bash-executor.ts, exec.ts, output-guard.ts, nested-tool-calls.ts,
system-prompt.ts, resource-loader.ts, skills.ts, prompt-templates.ts,
slash-commands.ts, keybindings.ts, settings-manager.ts, settings-schema.ts,
settings-defaults.ts, settings-diagnostics.ts, trust-manager.ts,
project-trust.ts, compaction/ (all files), cache-warmer.ts, usage-totals.ts,
defaults.ts, messages.ts, event-bus.ts}`.

Opi side: `crates/opi-coding-agent/src/{tool/ (bash.rs, edit.rs, find.rs,
glob.rs, grep.rs, ls.rs, operations.rs, process_tree.rs, read.rs,
supervision.rs, write.rs, mod.rs), harness.rs, config.rs, context_files.rs,
skill.rs, prompt.rs, prompt_fragment.rs, resource.rs, policy.rs,
project_trust.rs, tool_authority.rs, compaction.rs, image.rs}` +
`crates/opi-sandbox/src/*` and `crates/opi-protocol/src/*` (check product
consumption vs standalone; note opi-only surface).

Measure:
1. Built-in tools: pi tool set vs opi tool set — names, schemas
   (params/defaults/offset semantics), output budgets/truncation,
   image/binary modes, permission prompts.
2. Bash execution: `bash-executor.ts` (shell, timeouts, process groups,
   output limits) vs `tool/bash.rs` + opi-sandbox (is the sandbox used by the
   bash tool? restriction policy).
3. `nested-tool-calls.ts` vs `tool/supervision.rs` + `operations.rs`.
4. System prompt assembly, context files (AGENTS.md/CLAUDE.md discovery,
   precedence, trust gating), skills, prompt templates, slash commands vs
   `context_files.rs`/`skill.rs`/`prompt*.rs`.
5. Settings: pi settings.json schema/defaults/diagnostics vs opi `config.rs`
   format/schema.
6. Compaction: pi algorithm (cut points, summaries, branch summaries, token
   estimation) vs `compaction.rs` + `opi-agent/compaction.rs`.
7. Trust: `trust-manager.ts`/`project-trust.ts` vs `project_trust.rs`/
   `policy.rs`/`tool_authority.rs`; stronger opi isolation is opi-only but
   flag accepted-behavior differences.

Output `evidence/product-runtime.md`, prefix `RUN`. Tallies + coverage
honesty; exact absence-search commands.

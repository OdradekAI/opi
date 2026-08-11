# opi-sandbox

[![Crates.io](https://img.shields.io/crates/v/opi-sandbox.svg)](https://crates.io/crates/opi-sandbox)
[![Docs.rs](https://docs.rs/opi-sandbox/badge.svg)](https://docs.rs/opi-sandbox)

> Standalone command-execution restriction SDK, human CLI, and protocol
> backend.

[Simplified Chinese](README.zh.md) | [opi workspace](../../README.md)

`opi-sandbox` is an independent package for supervising and restricting a
single command process tree. It can be used directly, embedded as a Rust
library, or launched as a
[`command-execution-jsonl-v1`](../opi-protocol/README.md) backend. It does not
require or read Opi configuration, sessions, package storage, trust state, or
credentials.

```sh
cargo install opi-sandbox
# Library use:
cargo add opi-sandbox
```

Requires Rust 1.97+ (workspace MSRV; edition 2024).

## Status

Current crate version: `0.7.3`, inherited from the workspace package version.

Official release archives target Linux and macOS. There is no official Windows
`opi-sandbox` artifact; a Windows build provides L0 Job Object supervision but
the production `run` and protocol backend refuse the requested restriction
before starting the target.

## Start with `doctor`

Inspect the actual host posture before relying on a restriction:

```sh
opi-sandbox doctor
opi-sandbox doctor --json
```

`doctor --json` emits a stable schema-version-1 object with `supported`,
`target`, `mechanisms`, `profiles`, and `limitations`. A completed diagnostic
exits `0` even when `supported` is `false`.

## Human CLI

Run an explicit program and argument vector:

```sh
opi-sandbox run \
  --workspace /path/to/workspace \
  --profile workspace-write \
  --network deny \
  -- /bin/sh -lc 'printf "hello\n" > result.txt'
```

The exact grammar is:

```text
opi-sandbox run --workspace <PATH> --profile workspace-write \
  --network <deny|allow> -- <PROGRAM> [ARGUMENTS...]
```

All three flags are required. `--` ends option parsing; the remaining values
are passed as a native program and argument vector, not as an implicit shell
string. The human CLI uses the workspace as the working directory and inherits
terminal stdin and the host environment. Each invocation receives a private
temporary root through `TMPDIR`, `TMP`, and `TEMP`.

The target's stdout and stderr are streamed byte-for-byte. Exit mapping:

| Outcome | Exit code |
|---------|-----------|
| Target exits normally | Target exit code, unchanged. |
| Unix signal | `128 + signal`. |
| Timeout | `124`. |
| Ctrl-C / cooperative cancellation | `130`. |
| Unsupported platform or pre-start setup failure | `125`. |
| Invalid CLI input, workspace, or working directory | `2`. |

## Platform Contract

`workspace-write` allows host reads and execution while restricting filesystem
mutation to the canonical workspace and invocation temporary root.
`network = deny` requests the platform's network deny layer.

| Platform | Production posture |
|----------|--------------------|
| Linux | Supported only when filesystem-capable Landlock and the audited seccomp architecture are available. Uses Landlock for filesystem mutation, a fixed seccomp danger-syscall blocklist, and additional new-socket/TCP restrictions for `network = deny`; AF_UNIX remains available. |
| macOS | Supported only when canonical `/usr/bin/sandbox-exec` passes its runtime probe. Uses a Seatbelt deny overlay: writes outside the workspace/temp roots are denied, and `network = deny` denies network operations. There is no syscall filter; `sandbox-exec` is a legacy/experimental surface. |
| Windows | Unsupported for restriction. Job Objects provide L0 process-tree supervision only; production execution is refused before target start. No official artifact is published. |
| Other platforms | Unsupported; production execution is refused before target start. |

Supported native setup is fail-closed. If the requested contract cannot be
established, the target is not released.

## Protocol Backend

```sh
opi-sandbox backend --stdio
```

This process speaks `command-execution-jsonl-v1`: host frames arrive on stdin,
backend frames leave on stdout, and stderr remains bounded out-of-band crash
evidence. One backend process accepts at most one execution. Command and policy
inputs travel in protocol frames rather than process arguments. See
[`opi-protocol`](../opi-protocol/README.md) for frame, bound, and compatibility
rules.

## Library SDK

The public SDK uses explicit inputs and keeps no cross-invocation state:

| Item | Purpose |
|------|---------|
| `SandboxPolicy` / `Profile` / `NetworkPolicy` | Requested `workspace-write` and network contract. |
| `SandboxRequest` | Explicit program, arguments, workspace, cwd, timeout, environment, stdin, and cancellation inputs. |
| `Restriction` | Platform-neutral pre-spawn restriction seam supplied by the caller. |
| `SandboxRunner` / `SandboxRun` | Synchronous setup plus an owned async event stream that supervises one process tree. |
| `SandboxEvent` | `Started`, incremental `Output`, redacted `Diagnostic`, and one terminal `Completed` event. |
| `SandboxResult` / `SandboxOutcome` | Structured exit/signal/timeout/cancellation result, cleanup state, and bounded output previews. |
| `NoRestriction` | Explicit L0-only implementation: process-tree supervision with `ContractStatus::Unrestricted`, not native confinement. |

The shipped CLI and protocol backend select the package's native Linux/macOS
restriction after probing the host. Direct SDK callers choose a `Restriction`
explicitly; constructing a runner with `NoRestriction` provides L0 supervision
only and must not be described as sandboxed or restricted.

Complete stdout/stderr is delivered through incremental `Output` events with
bounded backpressure. The terminal result keeps at most a 1 MiB preview per
stream and reports truncation separately. Dropping an in-flight `SandboxRun`
owns cleanup: it terminates the child tree and removes the invocation temporary
root on every terminal path, although `CleanupState::Unconfirmed` remains
possible when the operating system cannot confirm a cleanup step.

## Security Boundaries

- The effective contract is `restricted`, never `isolated`.
- Host reads and program execution remain available; this is not a host-file,
  environment-variable, credential, or inherited-file-descriptor
  confidentiality boundary.
- Restrictions apply to the target process tree, not to the embedding process
  or an adapter host around it.
- The package is not a container, VM, remote executor, or multi-tenant security
  boundary. Target code runs with the launching user's OS identity.
- `NoRestriction` is deliberately available for custom SDK composition and
  reports `unrestricted`; it never silently upgrades to a native guarantee.

## License

MIT. See the workspace [LICENSE](../../LICENSE).

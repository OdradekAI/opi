# Phase 15 Linux L2 feasibility

Date: 2026-07-24

## Conclusion

The Phase 15 design's current Linux L2 wording is not implementable as written.
Classic seccomp BPF can inspect the six raw syscall argument values but cannot
dereference pointers. Consequently it can filter `socket(domain, type,
protocol)` by `domain`, but it cannot recover a socket family from the
`sockaddr *` passed to `connect`, `bind`, or `sendto`; `recvfrom` and `accept`
likewise expose only a socket fd plus output pointers, not a domain. The kernel
documents the no-pointer-dereference restriction explicitly, and the syscall
signatures confirm where the family actually lives
([seccomp filter documentation](https://docs.kernel.org/userspace-api/seccomp_filter.html),
[socket(2)](https://man7.org/linux/man-pages/man2/socket.2.html),
[connect(2)](https://man7.org/linux/man-pages/man2/connect.2.html),
[bind(2)](https://man7.org/linux/man-pages/man2/bind.2.html),
[sendto(2)](https://man7.org/linux/man-pages/man2/sendto.2.html),
[recvfrom(2)](https://man7.org/linux/man-pages/man2/recvfrom.2.html),
[accept(2)](https://man7.org/linux/man-pages/man2/accept.2.html)).

The minimum implementable deny-overlay is therefore:

1. seccomp-deny new `socket()` calls whose scalar `domain` is `AF_INET`,
   `AF_INET6`, or `AF_NETLINK`, while allowing `AF_UNIX`;
2. allow the generic `connect`/`bind`/send/receive/accept operations required
   by AF_UNIX, without claiming that seccomp domain-filters them;
3. on Landlock ABI 4 or newer, additionally handle `BIND_TCP` and
   `CONNECT_TCP` with no allow-port rules, denying TCP bind/connect;
4. report the remaining inherited-fd and non-TCP gaps truthfully.

This is a useful creation-time network reduction, not a complete network
boundary.

## Seccomp feasibility by operation

| Operation | Domain-filterable by classic seccomp? | Reason |
|---|---:|---|
| `socket` | Yes | `domain` is scalar argument 0. |
| `connect` | No | The family is inside the pointed-to `struct sockaddr`. |
| `bind` | No | The family is inside the pointed-to `struct sockaddr`. |
| `sendto` | No | The destination family is inside the pointed-to `struct sockaddr`; connected sockets may also use `send`/`write`. |
| `recvfrom` | No | The source address is an output buffer; the syscall's scalar arguments do not identify the socket domain. |
| `accept` / `accept4` | No | They operate on a listening fd and return a peer address through output pointers. |

The kernel's seccomp data structure contains the raw `args[6]`, and its
documentation states that BPF programs may not dereference pointers. This also
means an fd number is not a stable proxy for a socket's family
([kernel seccomp documentation](https://docs.kernel.org/userspace-api/seccomp_filter.html)).
Seccomp user notification can move pointer inspection to a supervisor, but that
is a different, stateful architecture with pointer-lifetime/TOCTOU concerns,
not the proposed in-child filter
([seccomp_unotify(2)](https://man7.org/linux/man-pages/man2/seccomp_unotify.2.html)).

The exact `extrasafe` 0.5.1 source independently acknowledges this limitation:
its server profiles allow `bind` unconditionally because the address structure
cannot be inspected
([source permalink](https://github.com/boustrophedon/extrasafe/blob/5ad0ecea00d375a267b93bb36cabb0316d91228f/src/builtins/network.rs#L99-L122)).
Its Unix-server profile filters only `socket`'s scalar domain/type arguments,
then permits generic bind and network I/O
([source permalink](https://github.com/boustrophedon/extrasafe/blob/5ad0ecea00d375a267b93bb36cabb0316d91228f/src/builtins/network.rs#L232-L261)).

## Exact Landlock network capability

Landlock filesystem support begins at ABI 1 / Linux 5.13. Network support does
**not** begin at Linux 6.2: ABI 3 is Linux 6.2, while network access rights
begin at ABI 4 / Linux 6.7
([`landlock` 0.4.5 ABI mapping](https://github.com/landlock-lsm/rust-landlock/blob/6b13cc4f2fb452096cf0c4b6e74341437df8b630/src/compat.rs#L53-L67)).
The runtime gate must query the Landlock ABI rather than infer it from the
kernel release, because downstream kernels may backport features
([landlock(7)](https://man7.org/linux/man-pages/man7/landlock.7.html)).

In `landlock` 0.4.5, the complete network API is only:

- `AccessNet::BindTcp`;
- `AccessNet::ConnectTcp`;
- `NetPort`, whose object is a 16-bit TCP port.

`AccessNet::from_all()` is empty through ABI 3 and contains exactly those two
rights from ABI 4 onward
([`landlock` 0.4.5 network source](https://github.com/landlock-lsm/rust-landlock/blob/6b13cc4f2fb452096cf0c4b6e74341437df8b630/src/net.rs#L41-L61)).
The kernel documentation agrees that ABI 4 restricts TCP bind/connect by port
([Landlock compatibility documentation](https://docs.kernel.org/6.14/userspace-api/landlock.html#tcp-bind-and-connect-abi-4)).

Therefore `landlock` 0.4.5 does not restrict UDP, raw sockets, NETLINK, socket
creation, data transfer on already-connected sockets, or network address/IP
ranges. It can supplement the seccomp creation gate for TCP; it cannot
implement the stated general INET/INET6/NETLINK policy by itself.

## Exact `extrasafe` 0.5.1 capability

`extrasafe` 0.5.1 is not a suitable implementation of the proposed L2/L3
deny-overlay:

- `SafetyContext` is a default-deny syscall **allowlist**. Its generated
  `SeccompFilter` uses `Errno` for unmatched calls and `Allow` for matched
  rules, rather than allowing everything except a short deny set
  ([source permalink](https://github.com/boustrophedon/extrasafe/blob/5ad0ecea00d375a267b93bb36cabb0316d91228f/src/lib.rs#L511-L550)).
- Its networking rules can conditionally allow `socket()` by scalar family and
  type, but `connect` and `bind` are permitted only unconditionally
  ([network source](https://github.com/boustrophedon/extrasafe/blob/5ad0ecea00d375a267b93bb36cabb0316d91228f/src/builtins/network.rs#L99-L122),
  [connect source](https://github.com/boustrophedon/extrasafe/blob/5ad0ecea00d375a267b93bb36cabb0316d91228f/src/builtins/network.rs#L169-L181)).
- Its bundled Landlock integration is filesystem-only, hard-codes ABI 2, and
  the crate's own guide says Landlock networking is unavailable in extrasafe
  ([source permalink](https://github.com/boustrophedon/extrasafe/blob/5ad0ecea00d375a267b93bb36cabb0316d91228f/src/lib.rs#L552-L571),
  [user guide](https://github.com/boustrophedon/extrasafe/blob/5ad0ecea00d375a267b93bb36cabb0316d91228f/user-guide.md#L7-L11)).
  Its optional dependency is `landlock ^0.3`, not the separately proposed
  `landlock` 0.4.5 backend
  ([manifest](https://github.com/boustrophedon/extrasafe/blob/5ad0ecea00d375a267b93bb36cabb0316d91228f/Cargo.toml#L32-L34)).
- Its seccomp application path has an explicit compile error outside
  `linux/x86_64`, conflicting with opi's supported Linux ARM64 release target
  ([source permalink](https://github.com/boustrophedon/extrasafe/blob/5ad0ecea00d375a267b93bb36cabb0316d91228f/src/lib.rs#L511-L540)).
- Its public `SafetyContext::apply*` path constructs the filter during
  application; it does not expose the design's claimed parent-build/child-only
  raw-apply split
  ([source permalink](https://github.com/boustrophedon/extrasafe/blob/5ad0ecea00d375a267b93bb36cabb0316d91228f/src/lib.rs#L470-L550)).

Direct use of a deny-capable seccomp builder (for example `seccompiler`,
subject to verified target support) is structurally closer to Phase 15 than
`extrasafe::SafetyContext`. The design should not claim `extrasafe` 0.5.1
provides a danger-blocklist or portable Linux backend.

## Recommended implementable contract

Define Linux L2 narrowly as a **new-socket creation gate**:

> On supported Linux architectures, strict network mode installs a seccomp
> deny-overlay that returns a stable errno for `socket(AF_INET, ...)`,
> `socket(AF_INET6, ...)`, and `socket(AF_NETLINK, ...)`, while allowing
> `socket(AF_UNIX, ...)` and the generic socket operations needed for Unix-domain
> IPC. On Landlock ABI 4+ (Linux 6.7+), it also denies TCP bind/connect by
> handling both TCP access rights without allow-port rules.

Tests should prove:

- AF_UNIX stream and datagram create/bind/connect/send/receive still work;
- new AF_INET, AF_INET6, and AF_NETLINK sockets fail with the selected errno;
- on an engaged ABI-4+ host, TCP bind/connect are denied by Landlock;
- the capability report distinguishes `seccomp_socket_creation` from
  `landlock_tcp_bind_connect`;
- Linux x86-64 and Linux ARM64 are separately compiled, and only architectures
  with a verified backend claim L2 engagement.

## Residuals that must remain explicit

- Generic seccomp cannot distinguish the family of `connect`, `bind`,
  `sendto`, `recvfrom`, or `accept`; the current six-syscall
  “domain-filtered” claim must be removed.
- Already-open or inherited INET/INET6/NETLINK socket fds can still be used
  (including via `read`/`write`, `sendmsg`, or `recvmsg`). Either close/sanitize
  such descriptors before enforcement and test that invariant, or retain this
  residual.
- Landlock 0.4.5 covers TCP ports only. UDP, raw sockets, NETLINK, address
  ranges, and traffic on already-connected sockets are outside its policy.
- `socketpair` and newer alternate creation/dispatch surfaces must be audited;
  the acceptance claim should enumerate the covered syscall/architecture
  matrix rather than infer completeness from `socket()` alone.
- Kernels below Linux 6.7 cannot engage Landlock network controls. Linux 6.2
  is only ABI 3 and must not be reported as network-capable.
- A true “no external network while preserving AF_UNIX” boundary requires a
  stronger architecture such as a sanitized-fd launcher plus network namespace
  or a supervising policy mechanism. That is outside the current
  `pre_exec`-filter design and should remain a follow-up.

## Task-graph/design corrections

1. Replace “Landlock (6.2+ net)” with “Landlock ABI 4 / Linux 6.7+ TCP
   bind/connect-by-port only.”
2. Replace “`socket`/`connect`/`sendto`/`recvfrom`/`accept`/`bind` are
   arg-filtered on domain” with the implementable contract above. Name the
   inherited-fd and non-TCP residuals in the task DoD.
3. Do not assign L2/L3 deny-overlay implementation to
   `extrasafe::SafetyContext` 0.5.1. Use a backend that can express
   match-deny/default-allow and compile for each supported Linux release
   architecture, or explicitly degrade unsupported architectures.
4. Add a task dependency/order boundary: first select and cross-compile the
   seccomp backend and prove parent-build/child-apply feasibility; then wire
   L2/L3 runtime policy. The current design assumes both properties that
   `extrasafe` 0.5.1 does not provide.
5. Acceptance wording must call this defense-in-depth/new-socket reduction,
   not complete denial of INET/INET6/NETLINK activity, unless inherited-fd and
   alternate-surface closure is added and verified.

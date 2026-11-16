# #142 Repair packaged rustdoc and rerun package consumers

Status: the packaged public rustdoc gate is repaired and the three current
package-consumer templates now pass their full offline package runners. The
frozen Pi Chat terminal pairing and the two exact frozen `models-runtime` cases
were rerun against the current candidate; all prior receipts remain immutable.

## Rustdoc repair

`crates/opi-ai/src/tls/mod.rs` previously linked public module documentation to
the private `node_trust` item. The doc text now describes the packaged pinned
Node CA bundle as defined by the module, without exposing or linking the private
module. The source SHA-256 changed from
`0310f1ff17e9e3cd458ac9f9efafff79e97bd50df0c2aaef1cd2611266f16fdb` to
`aec5d3715d1130b349e928f41cc670a585ea731c28e27a7efba8de1d773c16fd`.
No runtime implementation, feature closure or dependency was changed.

## Package consumers

The machine-readable receipt is
[phase8-packaged-rustdoc-and-consumer-rerun-20261116-v1.json](../evidence/phase8-packaged-rustdoc-and-consumer-rerun-20261116-v1.json).
Its manifest SHA-256 is
`901e343bd7c6e4ad3eeb6c21a3daf2a84d44b4a5df3e8acb6943497437f9a756` and its
file SHA-256 is `b9df889d7df2ab0557b4315ab1e69f3c6c322cd14ea1e2c413e2af1148f1a43c`.

- `consumer-http`: ordinary and `caller-union` each passed 6/6; package
  evidence SHA `d5ec4fe2a45766c5edfcbd46806e2642556c78657b66b968ed3f82539a5299dc`.
- `consumer-crypto`: ordinary and `caller-union` each passed 6/6; package
  evidence SHA `b1eb78a886ff2ce2ae5489ead4110ba0223480ca52ed4bbdaf2aa8acfc927376`.
- `consumer-ws`: ordinary, `union-default` and `union-c-zlib` each passed
  10/10. The generic runner's private protocol, public Upgrade, clippy,
  packaged rustdoc and all three external consumer phases passed; package
  evidence SHA `5405a346e54e941d5e86240b936dd4eed0b5812ce8b0c0712e25fde4b85f3ba3`.

The WS rustdoc log retains the existing Cargo warning about the shared provider integration-test target; its exit code is 0. All four HTTP/crypto and three WS consumer profiles consumed the same current
`opi-ai-0.8.3` archive (`ff9a0e89650201748a4c13c273fd7cd139ce32362612886b04690d62e7fd92ca`)
and `opi-telemetry-0.8.3` archive
(`f88309c41de52a4de8964d53ddd0c65b09df5fbfad3ff8cec48f47dccc95e0aa`) from
workspace-external scratch projects using the marked target
`E:\\opi\\cargo-targets\\opi-72fa37a0fa044958`.

## Frozen Pi verification

The Chat terminal differential receipt is
[phase8-chat-terminal-differential-20261116-v1.json](../evidence/phase8-chat-terminal-differential-20261116-v1.json)
(SHA-256 `85516ccbc5c4165de2f37c1f0b7de204aa8de0a890ec40da045959d316e30ea7`).
Frozen Pi revision `f9a26365568127461c47a07a37dcb42c140a3ac5` and the current Rust
probe agree for eight request pairs across three stable runs. Finish/usage,
`[DONE]`/EOF, tail error and response release match; the previously approved
cancellation difference remains: Pi closes blocks during abort unwind, while
Opi cancellation preempts buffered events without synthetic ends.

An initial material capture against the drifted repository `.pi` tree was retained as a failed attempt; the authoritative rerun binds the sealed Pi snapshot. The exact frozen `models-runtime` command was also rerun from the sealed Pi
snapshot: one file passed, 2 selected cases passed and 41 cases were skipped by
the name filter. The source SHA is `8c6a89b2e05919cd0e355e1a6f35b4fa3a0d4953e2c4bf22e5c79c274aa6fe06` and the retained log SHA is
`f08ffa9bc162ad06ce242aed5ae559ed6c834fc5510c40923feee03658cbfa71`.

## Historical integrity and test impact

#139, #140 and #141 receipts, the frozen worktree receipt and the historical
`contract-index.json` hash remain unchanged. Test impact: `add` the #142 receipt
and issue, `update` spec history, `delete` none, `retain` all previous package,
Pi and failed-attempt evidence. This slice does not close macOS, quantitative
budgets, #94 real Deferred integration or the full six-gate milestone.
# Controlled external source packet

This is synthetic, first-party-style material for offline skill evaluation.
The names, mechanisms, results, and proposals below are fictional. They are not
Karpathy's work, a real framework, or evidence of actual benchmark performance.
Cite this packet as a fixture, never as a real publication. Use current Opi and
an identified local pi snapshot for the other sides of the comparison. Do not
use the network or run providers, installers, or the fictional implementations.

## Study M1: durable mailboxes for Agent coordination

The authors describe two local workers that exchange messages through one
SQLite-backed mailbox. Each conversation has a monotonic message sequence.
A receiver acknowledges a message after its handler returns. Unacknowledged
messages are redelivered after restart; duplicate message IDs can be rejected by
a mailbox table. Handler tool side effects and mailbox acknowledgement are not
in the same transaction. A trace records a file append, a crash before ack, and
another append after redelivery. The conclusion nevertheless calls the mechanism
"exactly-once Agent execution."

The study reports 100 repeated two-worker tasks on one host. A non-persistent
baseline loses in-flight messages on restart; the mailbox variant recovers them.
One repetition per task is reported, with no randomization, confidence intervals,
released implementation, raw traces, distributed workload, or provider-cost data.
The mechanism requires durable storage and a recovery owner. Multi-host scaling,
malicious participants, and context-quality improvements are explicitly untested.

## Framework F1: MeshAgent

The feature page lists named workers, multi-Agent messaging, shared memory,
remote execution, cancellation, and replay. Its supplied implementation description
contains one CLI dispatcher that starts local processes and routes newline-
delimited JSON by worker name. Requests and replies have IDs; dispatch is round
robin. There is no persisted mailbox, bounded-queue policy, replay implementation,
or remote transport in the described shipping code.

The cancellation option is parsed but not forwarded to workers. Shared memory
is a mutable in-process dictionary that disappears on dispatcher exit. A three-node
remote example is a design sketch, not an executable integration. No runtime
artifact or end-to-end test results are supplied. The advertised feature list
therefore has a different evidence basis from the described implemented surface.

## Proposal R1: two routes for connected Agents

Route A makes a central coordinator and shared mutable memory mandatory for all
Agent invocations, including standalone coding. It proposes ownership and setup
convenience, but has no implementation or measurements.

Route B uses an optional process adapter with explicit participants, versioned
messages, cancellation, bounded queues, and independently owned durable state.
It hypothesizes lower coupling but leaves recovery ownership and migration costs
unresolved. Both routes are future proposals, not adopted plans or proven gains.

## Claim B1: performance headline

The authors call F1 "twice as fast as ordinary Agents" from two chosen successful
tasks with one run each. Timeouts are omitted from the reported denominator;
model settings, task equivalence, total resource use, and costs are unavailable.
No comparison to Opi or pi was performed.

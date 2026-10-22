# Logger effect contracts

`LOGGER-EFFECT-CONTRACTS` creates a bounded, proposed effect contract for the
actual pinned `rd_kafka_log0` / `rd_kafka_log_buf` logging chain. It makes the
ownership, callback and recursive dependencies explicit and checkable. It
**does not unblock a production Safe result**. All fifteen universal obligations
remain Open, and existing provider contracts, pins and verdict consumers are
unchanged.

The prediction was committed first at `302baf0a8`, on base
`31c5ed500d9d4a1448a623b839e1644adb19efd1`. Focused source is `b460762b4`.
The model's exact independently pinned digest is
`1c1ffedb049c363f1003c3eaa1e3110c08828375ab85e12e647578a3f423ed18`.

## Concrete source findings

The reviewed source revision is
`66910c10a4d7eafa810b84229cf2dcf7b0f26f97`. The replay authenticates three
complete raw files and twelve explicitly bounded line ranges against
[source-observations.json](source-observations.json) and their Git blobs:

- `rdkafka.c`: `rd_kafka_log_buf` creates a `RD_KAFKA_OP_LOG`, initializes its
  priority/fields, duplicates `buf` into `log.str` and enqueues it; the other
  destination invokes `log_cb` synchronously with borrowed text. `rd_kafka_log0`
  formats a 2,048-byte stack buffer and calls this helper. The deferred LOG
  dispatch also invokes the configured callback.
- `rdkafka_queue.h`: enqueue may retain the operation, recursively forward it,
  or delegate a disabled queue to `rd_kafka_op_reply`. Forwarding takes/releases
  a queue reference, may install the original queue's serve callback, and may
  trigger IO notification. The recursive branch ignores the inner return and
  returns one: **return one does not establish queue retention**.
- `rdkafka_op.c`: reply without a reply queue destroys the operation; reply
  with a queue can recursively enqueue. LOG destruction releases `log.str`,
  then generic cleanup releases the operation. Generic callback, error/toppar,
  reply-queue, refcount and build-dependent effects are separate obligations.

The source inventory is a reviewed raw-text boundary, not a preprocessed build,
whole-function extraction proof, callback registration proof or exhaustive C
path enumeration. No reviewed-source identity is added to an official lock.
The allocator wrappers, macro semantics and all other external effects are
explicit Open leaves rather than silently treated as covered by these sites.

## Checked model and notation

[model.json](model.json) is versioned `logger-effects-v1` and retains ten effect
nodes and ten alternatives. The worklist visits every node, including the
queue/reply cycle and a possible callback reentry into logging. It collects all
fifteen leaf obligations; it never discharges them. Missing/duplicated callees,
unknown obligations, unused nodes/leaves and unsupported document shapes refuse.
A finite dependency closure supplies no C termination or recursion rank.

Paired custody tracks the newly allocated operation and separately duplicated
log text. Creation is permitted once. Both transfer to queue custody together;
a consumer must take them before local cleanup. String destruction precedes
operation destruction. Wrong terminals, leaks, duplicate creation/release,
release/use after transfer and excessive forwarding traces refuse. Callback
steps represent **conditional proposed returning traces**. They do not declare
callbacks neutral or assert that callbacks have no external allocations/effects.
Their actual behavior, retention and lifetimes remain Open obligations.

[decomposition.md](decomposition.md) renders the environment/prerequisite,
forbidden, alternative, terminal and creation/retirement roles using the user's
notation. It is explicitly readable proposed data; it does not silently extend
an older `.qv` loader. The executable contract is the bounded JSON model and
`qcore_contract::logger_effects::check`.

The checker requires an independently held expected document digest and the
exact caller-supplied source domain. It checks complete source and site hashes
before constructing an opaque `CheckedPlan`. No deserialization route or
conversion to production safety/truth evidence exists. Bounds: document 128 KiB,
64 nodes, 64 paths, 64 events per path, 16 forward/reply steps per path,
128 obligations, 16 files, 2 MiB per file and 64 sites per file. These are
refusals, not claims that deployed executions meet the bounds.

## Validation

[focused-receipts.json](focused-receipts.json) records the nine actual commands,
exits, timestamps, PIDs, log digests and full private paths. At `b460762b4`:

| Check | Actual result |
|---|---|
| `cargo fmt --all -- --check` | exit 0 |
| `cargo test -p qcore-contract --test logger_effects --no-fail-fast` | exit 0; 9 passed, 0 failed/ignored |
| `cargo clippy -p qcore-contract --all-targets -- -D warnings` | exit 0 |
| `lake env lean --version` | exit 0; pinned Lean 4.35.0 |
| `lake build QCore.LoggerEffects` | exit 0 |
| `lake env lean AuditLoggerEffects.lean` | exit 0; 12 theorems; no project axioms |
| Exact pinned raw-source replay example | exit 0; 10 nodes, 10 terminals, 15 Open leaves |
| Wrong independently held document digest | refused, exit 1 |
| Altered private source bytes | refused, exit 1 |

The Rust suite checks the reviewed document's independent digest and uses
synthetic source bytes to test source/domain/site refusal logic hermetically.
The separate positive example invocation uses the actual external source
checkout, compares every source to its exact Git blob, and retains before/after
source hashes. This separates checker controls from source observations.

The Lean model proves step/trace paired-custody invariants, freshness,
string-before-operation cleanup, transfer preservation, refusal of local cleanup
of queued resources, and the forwarded-disabled example. Its closure theorem
retains every reachable effect under explicit local-collection/closed-domain
premises, including mutually recursive calls. Faithful C extraction and
Rust–Lean correspondence remain premise 28; no C refinement is claimed.

All heavy checks serialize on `/tmp/qcore-c-card-validation-20261022.lock`.
The complete private successful run is
`private/logger-effect-contracts/final-1/`, with `commands.json`,
`completion.json`, every stdout/stderr and the altered source negative.
[measure.py](measure.py) reproduces it with `--source-root CHECKOUT --out FRESH_PRIVATE_DIR`.
It refuses an existing output directory and a source revision/digest mismatch.

[validation-attempts.json](validation-attempts.json) retains all earlier exits
and logs: initial Lean keyword parsing failure, dependent induction-context
failure, absent direct output-directory/import failure, and the successful
Lake repair. They are not omitted or counted as accepted final checks.
The initial task-context invocation also rejected an invalid topic `evidence`
with exit 2; the corrected union uses `contracts/effects/providers/source`.

Full standing acceptance is **pending the root integrator's combined gate and
post-committed-matrix Python check**. Focused evidence is not a substitute.
Provider gate is inapplicable: no provider, manifest, schema, source-lock or
clause-count table changed. No native BASE/TIP gain is claimed or inferred from
the earlier logger priority score.

## Integrator list and remaining scope

Apply the exact increment, existing residual append and current-status wording
from [integration-amendments.json](integration-amendments.json), regenerate the
report index, and supply the shared gate receipt. No follow-up is retired.
Likely shared-file merge points are only `crates/qcore-contract/src/lib.rs` and
`proof/QCore.lean`; retain every parallel card's registration.

`W48-BOTTOM-UP-RESIDUAL-PLAN` remains Open. A future production logger contract
requires formatting/TLS/configuration lifetime, allocator/copy extents, queue
synchronization and actual reference lifetimes, callback/IO/serve/destruction
closure, finite forwarding/reply execution, generic cleanup, later consumer
cleanup, and complete source/build/model correspondence. That extension needs
an explicit checked semantic consumer, matching Lean, official reviewed pins
and count updates if provider clauses change, and committed coverage prediction
plus full native attribution for any verdict movement. This finite proposed
model establishes none of those universal source truths by itself.

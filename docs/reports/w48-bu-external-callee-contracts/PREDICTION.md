# W48-BU-EXTERNAL-CALLEE-CONTRACTS prediction

Committed before contract, pin, compiler-expectation or test changes. The own
fresh corpus audit and planner both exited 0. [BASE receipt](BASE-receipt.json)
binds their exact bytes, held binary and actual build source; code/data equality
to launch BASE `8991b7b58fcbd33da082bbb44e819d47a0fb2561` is recorded separately.
The earlier path-selection failure is retained and supplies no verdict baseline.

BASE has 185 selected units: 166 Unknown, 8 Safe, 7 Unsafe and 4 data-only.
Its 4,834 analyzed function rows are 999 Safe, 3,826 Unknown and 9 Unsafe;
87 raw inventory entries are not projected. All units and unprojected entries
remain visible. Raw names, including compiled-out names, are not active-body
coverage. No whole-source safety percentage is inferred.

## Proposed data and direct prediction

| Body | Own BASE blocker | Proposed change | Expected custody result |
|---|---|---|---|
| `mk_iov_send` | one `writev` call | libc/unistd: borrowed fd, vector and count; scalar result/no custody result | Unknown → Safe |
| `thrd_current_id` | one `thrd_current` call | libc/threads: configured u64 ABI, no custody result | Unknown → Safe |
| `rd_kafka_sasl_cyrus_global_term` | one `mtx_destroy` call | decline an inexact lifecycle contract | Unknown remains |

The primary point prediction is two direct function promotions: 999 → 1,001
Safe and 3,826 → 3,824 Unknown, with 9 Unsafe unchanged. Each target unit has
other Unknown bodies, so these two promotions alone do not change unit verdicts.
No unsupported construct is made admissible, no analysis route is deleted and
no consumer semantics or Lean definitions change.

The fresh planner has 2,200 candidate bodies and a conditional ceiling of 383.
It assigns each target one own call leaf and one possible cross-unit caller.
For the two proposed contracts those callers are `net_plain_writev` and
`rd_kafka_transport_ssl_threadid_cb`. Their promotion would additionally require
existing body-summary admission and caller obligations; this card adds no body
lifting rule and promises neither gain. Any actual caller promotion, other status
change or diagnostic trade will be itemized against the held BASE. The declined
mutex target's conditional caller gain is not claimed.

## Resolution, assumptions and proof boundary

[Configured resolution](RESOLUTION.md) and the twelve refreshed probes include
the selected standalone Monkey configuration. They resolve POSIX `writev` and
system C11 thread declarations. `WITH_C11THREADS=1`; the bundled TinyC bodies are
compiled out. The own native `tinycthread.c` row has zero analyzed functions and
30 raw source names; none of the three primitives has an analyzed body in the
corpus. The system and tail-pause selections exclude the target source units.

Both new declarations are revision-flexible libc assumptions. `writev` neither
transfers nor retains the caller's vector or buffers on normal return. Custody
borrowing supplies no buffer-validity/bounds, I/O-success, full-write, errno
preservation or cancellation guarantee. `thrd_current` supplies an identifier,
not an owning thread handle or a synchronization/lifetime proof. Its u64
projection requires the observed LP64 unsigned-long ABI; incompatible pointer
or scalar declarations must withdraw it. Standard-library binding and behavior
remain declared assumptions, not a proof of libc bodies or C extraction.

`mtx_destroy` destroys mutex resources in caller-owned storage, not that storage.
The current in-place consumer does not admit this global owner, and no mutex
kind describes the lifecycle. Borrowed or heap-released would obscure the
missing obligation. Keep Unknown under `CG25-IN-PLACE-RESIDUAL-PAIRS`. The
configured C11 success value is 0; the historical bundled value 1 is not this
configuration. That changes no missing-kind/global-owner obligation.

Each reported target retains its full `(artifact, property, semantic profile,
scope, bounds, assumptions, evidence version)` in [BASE targets](BASE-targets.json).
The prediction concerns `resource-custody.v1` under the existing bounded
`legacy-pinned-resource-profile`. Matching existing declared-ABI, borrow-liveness,
call-instantiation and kinded-custody Lean models are reused; all audits remain
required. They do not establish library behavior or Rust–Lean/source refinement.

## Pre-registered validation and identities

Expect exactly two ordinary function entries and two flexible revision entries
in the complete typed environment, with the exact two appended source texts.
The 495 declaration dispositions become 497; selected modules stay 66, rejected
331; libc flexible entries become 130 → 132. The corpus pins add two declarations
and nine clauses, with no new module or include/inherit/published parent changes.

Six production-data controls cover ABI/header agreement, removal refusal,
borrowed-vector leakage and released-vector refusal, incompatible thread ABI,
global-mutex refusal and closed-descriptor refusal under the existing strict
policy. Run the standing and provider gates, pin checks and all five package
builds/verifies. No gate pass is claimed yet.

Run all three BASE/TIP routes with the same prepared paths, frozen packages,
profiles and link map. Retain raw and normalized five-artifact slots, every
unit/function status and diagnostic trade, full null records and inner dictionaries,
and outer/runtime fields. Expect no null inner-dictionary semantic change;
contract/function-table and compile-context identities change with their exact
preimages. Outer joins must be compared explicitly. Timings are observations,
not identity equivalences. No blanket digest/path masking or erased field is
permitted; every unexplained difference remains open until reviewed.

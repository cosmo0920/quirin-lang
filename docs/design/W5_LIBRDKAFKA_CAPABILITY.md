# W5 — the first librdkafka provider capability

Design record for card **P07**. Status: **design only**. No production Rust,
schema, profile, contract corpus or provider package changed in the increment
that produced this document. Nothing here promotes a verdict, and no raw
Cycle 19 unit changes class.

This document does three things, in the order the card requires:

1. records the **actual imported-operation inventory** of the librdkafka
   adapter, measured rather than assumed (§1–§2);
2. selects **one capability contract** from that inventory and states its
   source, build and ownership assumptions and its bounded claim (§3);
3. states the **precise rejection boundary** — why no existing core checker can
   discharge it, what process transport and independent semantic rejection
   would require, and the follow-on implementation units (§4–§6).

The measurements are reproduced by
`docs/reports/remaining-p07/inventory.py`; its output is
`docs/reports/remaining-p07/inventory.json`. The report at
`docs/reports/remaining-p07/README.md` records the run and the gate.

---

## 1. What the librdkafka adapter imports today

There is no librdkafka analysis code. `librdkafka` is a **profile-declared
library** plus a **built-in v1 provider stub**:

| Surface | Location | Contents |
| --- | --- | --- |
| W2 source profile | `contract/profiles/fluent-bit/profile.json` → `libraries.librdkafka` | 1 source root, 3 audited files, 3 source witnesses, 7 permitted mechanisms |
| Provider manifest | `contract/profiles/fluent-bit/providers/qcore.librdkafka.json` | `entrypoint.kind: built_in`, protocol v1, evidence v1, `certificate_checker: qcore.unsupported_assertions.v1` |
| Provider package | `providers/qcore-provider-librdkafka/` | `main.rs` delegating to the shared `quirin_provider_shared::runtime` (`providers/quirin-provider-shared/src/runtime.rs`); no librdkafka-specific computation |
| Declared contracts | `providers/qcore-provider-librdkafka/contracts/librdkafka/*.qcore` | 4 modules, 12 declared operations, 4 declared resource-type destructors |
| Loaded contracts | `cycle19-resource-contracts.json` | **1** of those 4 modules |

The profile fields that can carry an importable operation are **all empty**:

```
operation_summaries   0        typed_import          null
api_dispositions      0        callback_summaries    0
accepted_builds       0        boolean_field_models  0
required_evidence     0
```

So the adapter imports **no typed operation, no build, no callback and no
required evidence** for librdkafka. The three `source_witnesses` are prose
`digest.construct` strings checked against a file hash; they are declared
trust, not imported operations.

**The complete imported-operation inventory that reaches any checker is the six
allocator wrappers in `librdkafka/memory.qcore`.** The provider itself imports
nothing: it is a v1 built-in that returns `Unknown`, and the core refuses every
v1 `Safe` proposal.

## 2. The declared operations, measured against the audited build

`refcount.qcore`, `typed_queue.qcore` and `queue_destructor.qcore` are in the
contract corpus and are pinned by `compiler/tests/qcore_contract_corpus.rs`, but
they are **not** in `cycle19-resource-contracts.json`, so they reach no
interpreter. That absence is load-bearing, because when they are measured
against the audited configuration most of them do not describe an operation that
exists.

The audited configuration is Fluent Bit's own generated
`build/lib/librdkafka-2.15.0/generated/config.h`, which sets
`HAVE_ATOMICS_32 1` and `ENABLE_REFCNT_DEBUG 0`. `RD_REFCNT_USE_LOCKS` is
therefore **not** defined.

| Declared operation | Module | Loaded? | Form in the audited build | Declared → actual arity |
| --- | --- | --- | --- | --- |
| `rd_malloc` | memory | **yes** | `static inline` | 1 → 1 |
| `rd_calloc` | memory | **yes** | `static inline` | 2 → 2 |
| `rd_strdup` | memory | **yes** | `static inline` | 1 → 1 |
| `rd_strndup` | memory | **yes** | `static inline` | 2 → 2 |
| `rd_free` | memory | **yes** | `static inline` | 1 → 1 |
| `rd_realloc` | memory | **yes** | `static inline` | 2 → 2 |
| `rd_refcnt_add0` | refcount | no | **function-like macro** — `rd_atomic32_add(R, 1)` | 1 → *n/a* |
| `rd_refcnt_sub0` | refcount | no | `static inline` | 1 → 1 |
| `rd_refcnt_destroy_final` | refcount | no | **absent from the whole tree** | 1 → *n/a* |
| `rd_kafka_q_enq0` | typed_queue | no | `static inline` | 2 → **3** |
| `rd_kafka_q_deq0` | typed_queue | no | `static inline` | 1 → **2** |
| `rd_kafka_q_destroy_final` | queue_destructor | no | **external function** | 1 → 1 |

The four declared resource-type destructors resolve the same way:
`KafkaQueue` and `KafkaQueueOwner` both name `rd_kafka_q_destroy_final`
(external function); `KafkaQueueEntry` names `rd_kafka_op_destroy`, which this
translation unit sees as a prototype because its body lives in `rdkafka_op.c` —
a translation-unit scope artefact, not an absence; and `KafkaReference` names
`rd_refcnt_destroy_final`, which does not exist anywhere.

Four findings follow, and they are the reason this card selects its capability
the way it does.

**F1 — `rd_refcnt_add0` is not a function.** Under `HAVE_ATOMICS_32` it is
`#define rd_refcnt_add0(R) rd_atomic32_add(R, 1)`. `refcount.qcore` declares it
`extern fn … -> i32`. There is no symbol, no signature and no body to import.

**F2 — `rd_refcnt_destroy_final` does not exist.** The name appears nowhere in
librdkafka 2.15.0. It is declared both as an `extern fn` and as the `destructor:`
of the `KafkaReference` resource type. Reference release in librdkafka goes
through the `rd_refcnt_destroywrapper` macro, which invokes a *caller-supplied*
destroy expression when `rd_refcnt_sub` returns `<= 0` — there is no single
final-release function to name.

**F3 — the typed-queue arities are wrong, and one direction is inverted.**
`rd_kafka_q_enq0` actually takes `(rd_kafka_q_t *, rd_kafka_op_t *, int at_head)`;
the declaration omits the third parameter. `rd_kafka_q_deq0` actually is
`void rd_kafka_q_deq0(rd_kafka_q_t *rkq, rd_kafka_op_t *rko)` — it takes the
entry as an *input* and returns `void`. The declaration models it as
`(queue) -> result<own<KafkaQueueEntry>, QueueEmpty>`, i.e. as a dequeue that
*produces* an owned entry and can fail. The body performs no emptiness test at
all (only `rd_dassert`), so the declared `QueueEmpty` error has no source
counterpart.

**F4 — the one faithful declaration still has an unsound postcondition.**
`rd_kafka_q_destroy_final` matches in linkage, arity and result. But its body
ends with

```c
if (rkq->rkq_flags & RD_KAFKA_Q_F_ALLOCATED)
        rd_free(rkq);
```

so the containing allocation is released on *some* paths only, while
`queue_destructor.qcore` asserts `ensures normal queue_is_discharged` and
`quant normal live_queues == old(live_queues) - 1` unconditionally. An embedded
queue — the common case for queues that live inside a larger object — is
destroyed but not freed.

These four are recorded as defects in *declared, unloaded* contract text. They
are **not** upstream librdkafka defects and they change no verdict: because
these three modules reach no interpreter, no analysis result was ever derived
from them. Repairing or withdrawing them is follow-on unit **P07-U5** (§6).

## 3. The selected capability contract

### 3.1 Why this one

Selection is forced by §2. Discard every operation that has no importable
definition (F1, F2) or whose declaration does not describe the operation
(F3). Exactly one candidate survives with a real, externally linked,
single-definition body whose declared arity and result match the source:
**`rd_kafka_q_destroy_final`**.

Its declared postcondition is unsound (F4), so the capability below does **not**
adopt the declared clause. It states the conditional property the body actually
has.

### 3.2 `qcore.librdkafka.queue_final_destroy.v1`

**Artifact.** The single function `rd_kafka_q_destroy_final`, defined at
`lib/librdkafka-2.15.0/src/rdkafka_queue.c:57`, in the one translation unit
`rdkafka_queue.c`, at the profile-pinned Fluent Bit revision, with the audited
file digest already recorded in
`profile.json → libraries.librdkafka.audited_files`.

**Property — conditional allocation discharge with unconditional
synchronization-object destruction.** On every complete path through the body:

* `mtx_destroy(&rkq->rkq_lock)` and `cnd_destroy(&rkq->rkq_cond)` each execute
  **exactly once**, after the matching `mtx_unlock`;
* `rd_free(rkq)` executes **iff** `RD_KAFKA_Q_F_ALLOCATED` is set in
  `rkq->rkq_flags` as observed at that point, and at most once;
* `rd_free(rkq->rkq_qio)` executes iff `rkq->rkq_qio != NULL` at entry, and
  `rkq_qio` is set to `NULL` before any later read;
* no other release of `rkq` or of any object reachable from it occurs in this
  body.

The verdict shape is therefore **two** disjoint discharge outcomes, not one:
`ALLOCATED` set ⇒ the queue allocation is discharged; `ALLOCATED` clear ⇒ the
queue allocation is **explicitly retained** and its custody remains with
whatever object embeds it. A capability that reported a single unconditional
`custody_discharged` here would be reporting F4 as truth.

**Declared premises (assumptions, not proofs).** Each must appear verbatim in
the certificate's assumption set, and each is a caller obligation this
capability does not discharge:

| Key | Assumption |
| --- | --- |
| `rdkafka.queue.last_reference` | the caller holds the last reference; `rkq_refcnt` reached 0 before the call |
| `rdkafka.queue.exclusive_access` | no other thread reads or writes `*rkq` for the duration of the call |
| `rdkafka.queue.disabled_before_final` | `RD_KAFKA_Q_F_READY` is clear at entry (the body only `rd_dassert`s this) |
| `rdkafka.queue.not_forwarded` | `rkq_fwdq` is `NULL` after `rd_kafka_q_fwd_set0`; the body `assert`s it |
| `rdkafka.c11.threads` | `mtx_destroy`/`cnd_destroy` release their object exactly once and are not called on a locked mutex |
| `rdkafka.build.identity` | the audited `config.h` values, dialect, target and complete option vector recorded in the compile-task identity |
| `rdkafka.abi.layout` | `rd_kafka_q_t` field offsets and `int` width in the selected ABI |

**Bounded out — these stay `Unknown`, by construction.** The card requires
asynchronous, callback and reclamation claims to be bounded; they are bounded
by *excluding them from the property*, not by assuming them away:

* **Reclamation of queued ops.** `rd_kafka_q_purge0` releases the ops still on
  `rkq_q`. This capability makes **no** claim about them: not that they are
  released, not that they are leaked, not how many there are. Whether the purge
  is complete is a separate obligation with no checker.
* **Callbacks.** `rkq_serve`/`rkq_opaque` may hold and invoke caller code. No
  claim is made about callback quiescence at teardown. The reserved
  `ObligationProperty::CallbackQuiescentAtTeardown` exists precisely for that
  and has no derivation and no witness kind.
* **Asynchrony.** The property is stated over one thread's complete paths under
  `rdkafka.queue.exclusive_access`. It says nothing about concurrent
  `rd_kafka_q_enq`/`q_deq`, about the wakeup `rkq_qio` file descriptor, or about
  any happens-before edge. `MutexGuardReleased` and `JoinBeforeExit` likewise
  remain reserved with no derivation.
* **Forwarded queues.** Excluded by `rdkafka.queue.not_forwarded`; a forwarded
  queue is refused, not analysed.
* **The four other librdkafka mechanisms** in `permitted_mechanisms`
  (`reference_count`, `intrusive_list`, `typed_queue`, `atomic_publication`,
  `thread_handoff`, `reclamation`) are untouched by this capability.

## 4. The rejection boundary — why nothing can check this today

Three independent checkers were evaluated against the inventory. Each refuses,
for a measured structural reason. **No adequate core checker exists, so this
card does not implement.**

### 4.1 The Boolean-field capability refuses every audited function

`qcore.c_boolean_field_contract.v2`/`v3` is the one librdkafka-reachable
capability that already crosses the process boundary. Its importer
(`crates/qcore-c-gateway/src/boolean_source.rs`) admits a function with exactly
one record-pointer parameter, an `int` or `void` result, and a body built only
from `CompoundStmt`, `IfStmt`, `param->field = e`, `return e`, one
`void (struct R *)` frame helper call, integer literals, and the operators
`!`, unary `-`, `==`, `!=` — with every accessed field declared exactly `int`.

A census over the **96** functions with bodies in the three audited librdkafka
files — 11 in `rd.h`, 43 in `rdkafka_queue.h`, 42 in `rdkafka_queue.c` — finds
**0 admissible**. The dominant refusals (a function may fail on several):

| Refusal | Functions |
| --- | --- |
| calls that are not a `void (struct R *)` frame helper | 86 |
| unmodelled node kinds | 73 |
| member accesses outside the `int`-field model | 72 |
| unmodelled operators | 63 |
| body never accesses an `int` record field | 60 |
| assignments to a place other than the modelled field | 58 |

There is exactly **one single-reason near miss**, and it is the whole argument
for unit P07-U1. `rd_kafka_q_ready`

```c
static RD_INLINE RD_UNUSED int rd_kafka_q_ready(rd_kafka_q_t *rkq) {
        return rkq->rkq_flags & RD_KAFKA_Q_F_READY;
}
```

satisfies every structural condition — one `rd_kafka_q_t *` parameter, `int`
result, one `param->int_field` access, no calls, no foreign assignment — and is
refused on **one** ground: the operator `&`. Its sibling
`rd_kafka_q_check_yield` is refused three times over, returning `rd_bool_t`,
reading `rkq_flags & RD_KAFKA_Q_F_YIELD` and writing
`rkq->rkq_flags &= ~RD_KAFKA_Q_F_YIELD` (a `CompoundAssignOperator` with `~`).

The pattern is systematic: librdkafka packs its Boolean state into `int`
bitmasks, while the capability models a whole `int` field as one Boolean. That
is a modelling gap with a precisely identified edge, not a vague shortfall.

### 4.2 The counter capability refuses the reference counts

`check_counter_contract` (`crates/qcore-verification/src/counter_contract.rs`)
requires a single-path body of exactly
`[Read place, Write place value, Read place, ReturnValue]`, a field whose C type
is one of `unsigned int | unsigned long | unsigned long long`, and the exact
signature `"{counter type} ({root C type})"`.

* `rd_refcnt_add0` is a macro (F1) — there is no function to project.
* `rd_refcnt_sub0` stores `int32_t` (**signed**), reaches its value through
  `rd_atomic32_sub`, and branches on `r < 0` into `rd_assert`. Signed type,
  opaque call and a second path: three refusals.
* `rkq_refcnt` and `rkq_qlen` are `int`, `rkq_qsize` is `int64_t` — all signed —
  and none is updated by a standalone one-parameter function.

### 4.3 The list-unlink capability refuses the queue operations

`check_list_unlink_contract` requires `void (T *)` plus a helper
`void (T *, T *)` over `prev`/`next` cells, exactly one helper call, exactly
four writes, and the unlinked entry's own links set to null.

`rd_kafka_q_deq0` is `void (rd_kafka_q_t *, rd_kafka_op_t *)` — two parameters
of two different types. Its body is BSD `TAILQ_REMOVE`, which:

* writes through a **pointer to pointer**, `*(elm)->field.tqe_prev = …`, an
  indirect store the typed-source vocabulary does not model and W4's typed
  scalar-field whitelist explicitly refuses;
* writes `(head)->tqh_last` — a *third* object — on the tail branch;
* leaves the removed element's own links **unchanged**;

and `rd_kafka_q_deq0` then additionally updates `rkq_qlen` and `rkq_qsize` on
the queue. Signature, field model, write count, branch structure and frame all
differ. Widening the unlink checker to cover TAILQ is its own unit, not a
parameter change.

### 4.4 The schema constraint, recorded separately

Independently of §4.1–§4.3, admitting a new witness form needs a new
`SemanticWitness` variant *and* a new `ObligationProperty` variant
(`crates/qcore-schemas/src/witness.rs`, `.../obligation.rs`). `ObligationProperty`
is a public non-`non_exhaustive` enum, so adding a variant is a
source-incompatible change; under the policy that made `qcore-schemas` 4.0.0 for
P04 that is a major bump. P07 is scoped to forbid a schema version bump, so even
a capability with an adequate checker could not land under this card. Any
implementation unit in §6 must budget that bump explicitly.

## 5. What process transport and independent semantic rejection require

The card's third acceptance item asks, for a design-only outcome, exactly what a
real process capability would have to demonstrate. The standard is the one P04
set for `qcore.chunkio` 0.5.0 and E02's 52-row matrix; **envelope conformance is
not evidence**. Concretely:

1. **The provider computes, the core decides.** The request must carry no
   derived result. The provider receives the typed program and computes its own
   path projection; the core independently evaluates the pinned contract against
   its **own sealed import** and only then requires agreement. Agreement alone
   must discharge nothing — a provider that echoes the core's own input must not
   be able to buy a verdict.
2. **Sealed import.** The witness must be replayable only through a context that
   a raw wire record cannot construct (the `BooleanFieldSourceTask::derive`
   pattern), and compile-fail tests must pin that the constructor and
   `Deserialize` route stay closed.
3. **Task identity binding.** The witness must bind the G01/G02
   `CompileTaskIdentity`: Clang program, version and target, driver, working
   directory, `source_input`, the complete ordered argument vector, dialect,
   ordered preprocessing options, analysed scope, raw and preprocessed source
   bytes, and the contract/profile/database digests. A changed header, a changed
   `config.h` — in particular a flip of `HAVE_ATOMICS_32`, which changes what
   `rd_refcnt_add0` *is* — must retire the task and refuse the witness in both
   directions.
4. **A mutation matrix, not a happy path.** At minimum the E02 row families:
   tampered path sets; a well-formed projection belonging to a different
   function that happens to satisfy the same contract; permuted, foreign and
   duplicated obligation identifiers; one witness claiming two obligations; a
   forged record rebound to a real donor task digest; proposals that echo the
   assertion text, the Lean theorem family or the verdict label; oversized and
   degenerate proposals failing bounded without panicking. Every row rejects.
5. **Missing and failing evidence.** Provider absence, non-zero exit, timeout,
   limit kill and rejected evidence each yield a scoped `Unknown` under `QP006`
   with no certificate and no fallback — never a program `Unsafe`, never a
   downgrade path, and never a promotion of the raw resource unit.
6. **Attribution.** Diagnostics must carry the exact selected plugin ID plus one
   1–128-byte ASCII local code, refused with `QP007` before verdict admission.
7. **Lean parity.** New model theorems in a librdkafka-scoped namespace, audited
   by `scripts/check_lean_audits.py`, with **no new axiom** and no placeholder.
   The conditional-discharge shape of §3.2 means the theorem must be a
   case split, not a single discharge lemma.
8. **Execution profile.** The package should declare limits and run under
   P02's `linux-rlimit-v1` or P03's `linux-confined-v1`; the default remains
   `unenforced-legacy` and confinement is not an OS sandbox.

## 6. Follow-on implementation units

Ordered by dependency. None is started by this card.

| Unit | Work | Blocks |
| --- | --- | --- |
| **P07-U1** | Extend the typed-source vocabulary with a *bitmask Boolean* field model: `field & MASK`, `field \|= MASK`, `field &= ~MASK` over a single-bit constant mask, with the mask bound to a declared enumeration constant. The census shows this is the smallest change that admits anything: `rd_kafka_q_ready` is refused on `&` alone. | U2, and the `rd_kafka_q_ready` / `rd_kafka_q_check_yield` family |
| **P07-U2** | A `conditional_discharge` core checker: two disjoint outcome classes over a body with one guard on a modelled field, `mtx_destroy`/`cnd_destroy` counted exactly once, and an explicit "retained, custody elsewhere" outcome that is **not** `Safe`. Needed for §3.2. | U3 |
| **P07-U3** | Schema: one `SemanticWitness` variant and one `ObligationProperty` variant (`ConditionalCustodyDischarged`). Source-incompatible → a deliberate `qcore-schemas` major bump with the documented policy note. | U4 |
| **P07-U4** | `providers/qcore-provider-librdkafka` gains the capability: provider-side typed program execution, no derived result in the request, the §5 mutation matrix, missing/failing-evidence rows, and package version bump. Profile gains `typed_import` and the librdkafka Boolean/mask models. | — |
| **P07-U5** | Repair or withdraw `refcount.qcore`, `typed_queue.qcore` and `queue_destructor.qcore` (F1–F4). Withdrawal changes the pinned per-file tables in `compiler/tests/qcore_contract_corpus.rs` and the 59-module corpus count, so it is a deliberate re-pin, recorded as such. | — |
| **P07-U6** | Widen `check_list_unlink_contract` to BSD `TAILQ`: pointer-to-pointer `tqe_prev`, the `tqh_last` tail branch, and no null-out of the removed element. Only then is `rd_kafka_q_deq0` expressible. | — |
| **P07-U7** | A reclamation obligation for `rd_kafka_q_purge0`, so the ops bounded out of §3.2 stop being silently absent and become a named `Unknown`. | — |

U5 is independently worth doing before U1–U4: leaving contract text in the
corpus that names a function which does not exist is a standing hazard, even
while it reaches no interpreter.

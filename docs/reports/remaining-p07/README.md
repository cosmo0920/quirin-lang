# P07 — the first librdkafka provider capability

**Outcome: design only.** No adequate core checker exists for any librdkafka
operation that survives the imported-operation inventory, so this card delivers
the inventory, one capability contract, a measured rejection boundary and seven
follow-on units instead of an implementation. No production Rust, schema,
profile, contract corpus or provider package changed.

Design record: [`docs/design/W5_LIBRDKAFKA_CAPABILITY.md`](../../design/W5_LIBRDKAFKA_CAPABILITY.md).
Measurement script: [`inventory.py`](inventory.py). Its output: [`inventory.json`](inventory.json).

Nothing is promoted. Raw Cycle 19 units remain 2 Safe / 42 provisional Unsafe /
137 Unknown plus four data-only units; CT coverage remains 11/50 mapped; the
contract corpus remains 59 modules; `qcore-schemas` remains 4.0.0. The four
pinned 185-unit artifacts were not re-measured and were not touched.

---

## 1. What the card asked, and what was done

| Acceptance | Result |
| --- | --- |
| 1. Inventory actual imported operations **before** choosing a witness form | Done, and it changed the answer. The librdkafka adapter imports **zero** typed operations; the only operations reaching a checker are six allocator wrappers. §2–§3 below. |
| 2. Bound asynchronous / callback / reclamation claims; unmodelled interactions stay `Unknown` | Done by construction: the selected capability's property **excludes** queued-op reclamation, callback quiescence and every concurrency edge, rather than assuming them. Design §3.2 "Bounded out". |
| 3. Establish process transport and independent semantic rejection — **or** specify exactly what that requires if design-only | Design-only, so specified: design §5, eight requirements, held to the P04/E02 standard. |

## 2. The inventory (acceptance 1)

`librdkafka` has no analysis code. It is a profile-declared library plus a
built-in v1 provider stub whose `main.rs` delegates to the shared runtime and
computes nothing. Every profile field that can carry an importable operation is
empty:

```
operation_summaries 0   api_dispositions 0   accepted_builds 0
required_evidence   0   callback_summaries 0  boolean_field_models 0
typed_import        null
```

The contract corpus declares 12 librdkafka operations across four `.qcore`
modules, but `contract/cycle19-resource-contracts.json` selects only
`librdkafka/memory.qcore`. **Six allocator wrappers are the entire imported
inventory.** The other six operations reach no interpreter.

Measuring those six unloaded operations against Fluent Bit's own generated
`config.h` (`HAVE_ATOMICS_32 1`, so `RD_REFCNT_USE_LOCKS` is undefined) shows
that most of them do not describe an operation that exists:

| Declared operation | Form in the audited build | Declared → actual arity |
| --- | --- | --- |
| `rd_refcnt_add0` | **function-like macro** `rd_atomic32_add(R, 1)` | 1 → *n/a* |
| `rd_refcnt_sub0` | `static inline` | 1 → 1 |
| `rd_refcnt_destroy_final` | **absent from the entire tree** | 1 → *n/a* |
| `rd_kafka_q_enq0` | `static inline` | 2 → **3** |
| `rd_kafka_q_deq0` | `static inline`, returns `void`, takes the entry as input | 1 → **2** |
| `rd_kafka_q_destroy_final` | **external function** | 1 → 1 |

All six loaded allocator wrappers are `static inline` with matching arity; they
are faithful.

Four findings, recorded in design §2 as F1–F4. The one that matters most for
future work is **F4**: `rd_kafka_q_destroy_final` is the only faithfully
declared mechanism operation, and its declared contract is still unsound —
`queue_destructor.qcore` asserts `queue_is_discharged` and
`live_queues == old(live_queues) - 1` unconditionally, but the body frees the
queue only under `if (rkq->rkq_flags & RD_KAFKA_Q_F_ALLOCATED)`.

**These are defects in declared, unloaded contract text — not upstream
librdkafka defects, and not a verdict change.** Because the three modules reach
no interpreter, no analysis result was ever derived from them, so nothing to
re-measure and nothing to withdraw. Repair or withdrawal is unit P07-U5, and it
would change the pinned per-file tables in
`compiler/tests/qcore_contract_corpus.rs`, so it must be a deliberate re-pin.

## 3. The rejection boundary (acceptance 3, negative half)

Three existing checkers were evaluated. Each refuses for a measured structural
reason, so the card's "implement only if an adequate core checker already
exists" condition is not met.

**Boolean-field capability** (`qcore.c_boolean_field_contract.v2`/`v3`) — the one
librdkafka-reachable capability that already crosses the process boundary. A
census over the **96 functions with bodies** in the three audited librdkafka
files (11 in `rd.h`, 43 in `rdkafka_queue.h`, 42 in `rdkafka_queue.c`) finds
**0 structurally admissible**. Refusal histogram (functions may fail on several
grounds):

| Refusal | Functions |
| --- | --- |
| calls that are not a `void (struct R *)` frame helper | 86 |
| unmodelled node kinds | 73 |
| member accesses outside the `int`-field model | 72 |
| unmodelled operators | 63 |
| body never accesses an `int` record field | 60 |
| assignments to a place other than the modelled field | 58 |

The census applies the importer's necessary *and* the cheaply checkable
sufficient conditions (assignment target must be `param->int_field`; the only
admitted call is a `void (struct R *)` frame helper). It does not run the
gateway importer, so it is a census, not an admission decision.

There is exactly **one single-reason near miss**, and it is the sharpest result
in this card. `rd_kafka_q_ready` — `return rkq->rkq_flags & RD_KAFKA_Q_F_READY;`
— satisfies every structural condition and is refused on the single ground that
`&` is outside the expression grammar. Its sibling `rd_kafka_q_check_yield` is
refused three times over (`rd_bool_t` result, `&` read, `&= ~` write). The root
cause is systematic: librdkafka packs Boolean state into `int` bitmasks while
the capability models a whole `int` field as one Boolean. That single edge is
what follow-on unit P07-U1 targets.

**Counter capability** — requires an `unsigned int|long|long long` field and a
single four-operation path. `rd_refcnt_add0` is a macro; `rd_refcnt_sub0` is
signed `int32_t`, reaches its value through `rd_atomic32_sub` and branches into
`rd_assert` on `r < 0`; `rkq_refcnt`/`rkq_qlen` are `int` and `rkq_qsize` is
`int64_t`, all signed, and none is updated by a standalone function.

**List-unlink capability** — requires `void (T *)` plus a `void (T *, T *)`
helper over `prev`/`next`, four writes, and the entry's links nulled.
`rd_kafka_q_deq0` is `void (rd_kafka_q_t *, rd_kafka_op_t *)` over BSD `TAILQ`,
which writes through a pointer-to-pointer (`*(elm)->field.tqe_prev = …`), writes
`(head)->tqh_last` on the tail branch, leaves the removed element's links
unchanged, and is followed by two counter updates on the queue.

Separately, admitting any new witness form needs a new `SemanticWitness` variant
**and** a new `ObligationProperty` variant. `ObligationProperty` is a public
non-`non_exhaustive` enum, so that is a source-incompatible change and a major
`qcore-schemas` bump under the policy that produced 4.0.0 for P04. This card is
scoped to forbid a schema version bump, so even a capability with an adequate
checker could not have landed here. Recorded in design §4.4.

## 4. The selected capability contract (deliverable)

`qcore.librdkafka.queue_final_destroy.v1`, over the single function
`rd_kafka_q_destroy_final` at `rdkafka_queue.c:57`. Full statement in design
§3.2. The shape worth repeating here: because of F4 the property has **two
disjoint outcomes**, not one —

* `RD_KAFKA_Q_F_ALLOCATED` set ⇒ the queue allocation is discharged;
* `RD_KAFKA_Q_F_ALLOCATED` clear ⇒ the allocation is **explicitly retained**,
  custody staying with the embedding object;

with `mtx_destroy` and `cnd_destroy` executing exactly once on every path. A
capability reporting a single unconditional `custody_discharged` here would be
restating F4 as truth, so `Safe` is not the shape of this verdict.

Seven declared premises (last reference, exclusive access, disabled-before-final,
not-forwarded, C11 thread semantics, build identity, ABI layout) are caller
obligations this capability does not discharge and that must appear verbatim in
any certificate's assumption set.

**Bounded out, staying `Unknown` (acceptance 2).** Reclamation of the ops that
`rd_kafka_q_purge0` releases; callback quiescence at teardown via
`rkq_serve`/`rkq_opaque`; every concurrency edge including the `rkq_qio` wakeup
descriptor; forwarded queues, which are refused rather than analysed; and the
other six declared `permitted_mechanisms`. These are excluded from the property,
not assumed away. The reserved `CallbackQuiescentAtTeardown`,
`MutexGuardReleased` and `JoinBeforeExit` obligation properties remain with no
derivation and no witness kind.

## 5. What an implementation would have to show (acceptance 3)

Design §5, held to the P04/E02 standard, in eight items: provider computes and
core decides with no derived result in the request, so agreement alone
discharges nothing; sealed import with compile-fail pins on the constructor and
`Deserialize` route; `CompileTaskIdentity` binding, where a `HAVE_ATOMICS_32`
flip must retire the task because it changes what `rd_refcnt_add0` *is*; the
E02 mutation-matrix row families, all rejecting; missing and failing evidence as
scoped `Unknown` under `QP006` with no fallback; `QP007` attribution; Lean
parity with no new axiom, as a case split rather than a single discharge lemma;
and a declared execution profile.

Follow-on units P07-U1 … P07-U7 are tabulated in design §6. U5 (repair or
withdraw the three unfaithful contract modules) is worth doing independently of
the rest.

## 6. Reproducing the measurement

```sh
python3 -B docs/reports/remaining-p07/inventory.py \
    --fluent-bit ../fluent-bit \
    --output docs/reports/remaining-p07/inventory.json
```

The script reads committed profile/manifest/contract inputs directly. The two
source-level measurements need Clang and a Fluent Bit checkout with a configured
build; when either is absent they are recorded as `null` and the script still
terminates with a report.

Clang is run over the recorded `-D`/`-I` options of the real build entry for
`rdkafka_queue.c`, which transitively includes both other audited files. **This
is a census, not a `qcore-audit` run**: the recorded entry's own driver is
`/usr/bin/cc`, and no compiler is substituted for any verification purpose. The
audited digests of all three librdkafka files still match the profile at the
time of this run, so the census is over the pinned bytes:

```
00ab63a6…  lib/librdkafka-2.15.0/src/rd.h
107e2292…  lib/librdkafka-2.15.0/src/rdkafka_queue.h
a1d55367…  lib/librdkafka-2.15.0/src/rdkafka_queue.c
```

Environment for the recorded run: Ubuntu Clang 21.1.8,
`x86_64-pc-linux-gnu`, Fluent Bit checkout at
`90a010910e5b3328f496cc20fadc937547133439`, recorded in the artifact itself. The
Clang-dependent numbers are configuration-sensitive by design — that sensitivity
is the point of F1.

**Attribution method.** Clang's JSON AST omits `loc.file` for nearly every node
in this translation unit, and its `includedFrom` names the *includer* rather
than the defining file, so carrying a file forward across siblings silently
misattributes functions. An early draft of this script did exactly that and
placed `rd_kafka_q_enq0` in `rdkafka_op.h`; it also under-counted the census at
53 functions. The script now attributes a function to an audited file only when
that file's own text, at the line the AST reports, actually spells the
function's name — so every attribution is confirmed against the source rather
than inherited. Functions that resolve to no audited file are excluded from the
census rather than guessed at.

## 7. Gate

### 7.1 Where the gate was run, and why

The worktree this card was assigned acquired **another lane's uncommitted,
in-progress changes** while P07 was being written: modifications to
`compiler/tests/ct_corpus.rs`, `docs/reevaluation/ct_manifest.json`,
`crates/qcore-c-gateway/src/{legacy_c.rs,lib.rs,typed_field_access.rs}`, a new
untracked `crates/qcore-c-gateway/src/borrowed_actual.rs`, and new
`compiler/tests/fixtures/ct/CT{26,27,29,30}/` directories — the T03 CT-mapping
and C02-P borrowed-field follow-ups. In that state
`cargo test --workspace` **fails to compile**, with three
`cannot find type UntrackedBases` errors in `legacy_c.rs`.

That failure is not caused by P07 and was not repaired by P07: those files
belong to another card and are mid-edit, so they were left untouched and were
**not** staged. This card's commit contains only its own three files. In
particular `docs/reevaluation/ct_manifest.json`, which P07 is forbidden to edit,
was modified by that other lane and is not part of this commit.

To gate honestly without disturbing that work, a separate detached worktree was
created at `9478b94` (the branch head), the submodule initialised, and only the
three P07 files copied in. The gate below is that tree: **HEAD plus this card's
files, and nothing else.** It confirms both that HEAD is sound and that P07 adds
no failure.

### 7.2 Results

| Gate | Result |
| --- | --- |
| `cargo test --workspace` | **exit 0** — 1080 tests across 19 binaries, 0 failures |
| `cargo fmt --all -- --check` | **exit 0** |
| `cargo clippy --all-targets -- -D warnings` | **exit 0** |
| `cargo run -p qcore-compiler --example reevaluation_probe` | **exit 0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | **exit 0** |
| `python3 -B -m unittest discover -s scripts/tests` | **exit 0** — 43 tests |
| `cd proof && lake build` | **exit 0** |
| `python3 -B scripts/check_lean_audits.py` | **exit 0** |

1080 workspace tests matches the merged-increment baseline recorded in
AGENTS.md exactly, which is the intended result: this card adds no test and
changes no compiled code. `lake` is not on the default `PATH` on this host and
had to be taken from `~/.elan/bin`; with that on `PATH` both Lean gates pass.

The `providers/` workspace gate was not run: `providers/` is untouched.

The pre-existing P02 `RLIMIT_NPROC` conformance flakiness recorded in
`docs/reports/remaining-p03/README.md` §9.1 is unchanged and was not weakened.

## 8. Boundaries

* **Design only.** No capability is implemented, no witness is admitted, no
  provider computes anything new, and no verdict moves. There is no stub, and
  in particular nothing asserts `Safe`.
* The census is a **necessary-plus-partial-sufficient** structural filter over
  three audited files in one translation unit. It is not a gateway admission
  decision and not a statement about librdkafka as a whole: the profile audits
  one `.c` file and two headers, so 89 of the 90 `.c` files under
  `lib/librdkafka-2.15.0/src` are outside the audited set entirely. A function
  the census marks admissible would still have to survive the real importer;
  none did, so that direction was never exercised.
* `rd_kafka_q_ready` failing on `&` alone means the distance to a first
  admissible librdkafka function is **one operator**, not a redesign. That is a
  measured estimate of P07-U1's size, not a promise that U1 succeeds: adding
  `&` to the grammar also requires the mask to be a single-bit declared
  constant, and that condition was not measured here.
* F1–F4 are defects in **declared, unloaded** contract text. They are not
  upstream librdkafka defects, and no analysis result depended on them.
* The selected capability's property is stated but **not proved**: there is no
  checker, no Lean theorem and no witness for it. Its seven premises are
  assumptions.
* The Clang-dependent rows of `inventory.json` describe **one build
  configuration**. A different `config.h` changes F1 specifically, which is
  itself the argument for binding `config.h` into the compile-task identity.
* No claim is made about librdkafka concurrency, reclamation, durability or
  callback behaviour, and none about the ops a queue purge releases.

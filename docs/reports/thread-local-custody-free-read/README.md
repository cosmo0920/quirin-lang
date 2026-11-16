# THREAD-LOCAL-CUSTODY-FREE-READ

User decision, 2026-11-16, quoted exactly: "Queue both (Recommended)". This
card is option D1 of CURRENT-CONTEXT-NEW-TRIAGE's proposal
(`docs/reports/current-context-new-triage/PROPOSAL.md` sections 3-4 on that
card's branch, tip `3d735b88c`): read a `__thread` / `_Thread_local` pointer
variable's value as a custody-free value, the reading `pthread_getspecific`'s
result already gets. Base `0f6e17118`.

## 1. Census (before any semantic change)

Every reported function that names a thread-local object, on the three live
routes at `baseline:497bdeb90` and on the user's `current --monkey-linked`
report at binary `89fa31d21` (the integrator's evidence tree, read only).
Script: [`scripts/tls_census.py`](scripts/tls_census.py) (preprocesses each
unit with its own compile command, finds the thread-local declarations and
the main-file bodies naming one, reads each such function's Clang AST and
classifies every reference with the card's admission); table:
[`scripts/tls_census_table.py`](scripts/tls_census_table.py). Output:
[`census/tls_census.json`](census/tls_census.json), rendered in
[`census/table.md`](census/table.md). "Only thread-local aliases" says every
diagnostic of the row is an unresolved identity of a thread-local object or
of one of its copies, the only diagnostics this reading can remove.

| Route | Rows naming a thread-local object | Reading admitted | Rows whose every blocker is the read |
|---|---|---|---|
| corpus (`baseline:497bdeb90`) | 23 (librdkafka only) | 0 | 0 |
| system | 12 (6 functions x 2 configurations) | 4 (`flb_engine_evl_get`, `flb_sched_ctx_get`) | 4 |
| slice | 8 (4 x 2) | 4 (`flb_context_get`, `flb_cf_context_get`) | 4 |
| current (`--monkey-linked`, `89fa31d21`) | 23 (the corpus's librdkafka rows) | 0 | 0 |

What blocks each today, and what the reading does:

* **The four getters** (`flb_engine_evl_get`, `flb_sched_ctx_get` on system;
  `flb_context_get`, `flb_cf_context_get` on slice; two configurations each)
  are `Unknown` for exactly two diagnostics, the unresolved identity of the
  thread-local object at the copy (`evl = flb_engine_evl;`) and of the local
  at the `return`. Every reference is a copy into an automatic local and a
  return. The reading is admitted and predicted `Unknown -> Safe`: 8 rows.
* **The setters** (`flb_engine_evl_set`, `flb_sched_ctx_set`,
  `flb_context_set`, `flb_cf_context_set`) write the object: the reading is
  withheld. Their one diagnostic (the stored formal's unresolved identity,
  rule 10's publication) is untouched.
* **`flb_engine_shutdown`, `flb_output_exit`** (system) write the object and
  hand the copy to calls and dereferences: withheld; 17 and 10 independent
  diagnostics.
* **librdkafka** (corpus, current): `rd_kafka_yield_thread` is an `int`,
  `rd_kafka_last_error_code` an enumeration, `rd_kafka_thread_name` /
  `rd_kafka_thread_sysname` arrays, all outside the object-pointer
  condition; `rd_kafka_curr_transport` is written in five transport functions
  and dereferenced through its copy in `rd_kafka_transport_ssl_cert_verify_cb`;
  `rd_qsort_r_arg` is written in `rd_qsort_r` and passed to a call in
  `rd_qsort_r_trampoline`. All withheld. The three Safe rows
  (`rd_kafka_last_error`, `rd_kafka_set_thread_name`, `rd_kafka_yield`) are
  already Safe and stay so.
* **Monkey** reads no `__thread` object on any route: neither the corpus's
  Monkey build nor Fluent Bit's `lib/monkey` defines `MK_HAVE_C_TLS`, so
  `MK_TLS_GET` is `pthread_getspecific` there (0 of 1,419 current compile
  commands, 0 of 142 corpus Monkey commands). Fluent Bit's own units define
  `FLB_HAVE_C_TLS` (841 of 1,419), so `FLB_TLS_GET(x)` is a plain read.
  `lib/monkey/tls/tls_stub.c` has no compile command in the current report,
  so the script could not preprocess it; its source names no `__thread`,
  `_Thread_local` or `*_TLS_*` macro.

No other function on any route reads a thread-local object.

## 2. The reading (gateway rule 54)

[Rule 54](../../agents/rules/54.md) is the authority. In short:

* **Objects** are names every reference of which is to a `VarDecl` Clang
  marks thread-local (`tls`): a file-scope one, collected once per unit into
  the unit index (`StatementIndex::thread_local_declarations`, beside rule
  11o's no-return ids), or a block-scope one; no other declaration of the
  function may spell the name. **Copies** are automatic locals (no
  storage-class specifier, not thread-local, not a parameter) receiving an
  object's value by `=` or an initializer, closed under copying.
* **Admission, per function** (`thread_local_read::admitted`): every object
  is an object pointer at every reference (not an array, a record, a function
  pointer, `volatile`, `_Atomic`); every reference to an object is a whole
  read consumed as a copy, an initializer, a `return` operand or a null test;
  every reference to a copy is a rebinding or a whole read in those four
  positions.
* **Transfer** (one `Alias` arm in `legacy_transfer::transfer_base`): where
  admitted, `COPY = OBJECT` and `COPY = COPY'` from a source with no binding
  drop the destination's binding. This is byte for byte the transfer of a
  `ResourceReturnMode::None` call result (`pthread_getspecific`). Withheld, or
  with a bound source, the old alias transfer runs.
* The admitted sets travel in the CFG (`LegacyCfg::thread_local_reads`,
  omitted when empty, so every other function's certificate is unchanged);
  the certificate recheck re-derives them from the source.

Refusals kept (each withholds the reading for the whole function):

| Shape | Why | Fixture |
|---|---|---|
| `t = p;`, `t = 0;`, `t++` beside the read | a take-from-thread-local would hide a transfer | `a_write_to_the_thread_local_object_is_refused` |
| `&t` anywhere in the body | the object's address escapes the reading | `the_address_of_the_thread_local_object_is_refused` |
| `t_rec.p`, `t_rec->p`, `char t_buf[16]` | not a whole read of a pointer object | `a_member_of_a_thread_local_record_is_refused` |
| `c = t; q_free(c);`, `q_free(t)`, `q_transfer(c)` | a release or transfer through it | `a_getter_that_also_frees_through_it_is_refused` |
| `c = t; c->x = 1;`, `c = t; q_observe(c);` | a store through it, a call actual | same |
| static (not thread-local) object, formal destination | outside the card | `a_static_object_and_a_formal_destination_keep_the_old_answer` |
| a local shadowing the object's name | the name is not only the object | `a_shadowed_name_is_not_read_as_thread_local` |

A caller that releases a getter's result still answers for it: the getter
derives as custody-free (`Safe`), and the caller's release of its result finds
no binding and is refused (`a_caller_that_releases_the_result_still_answers_for_it`).
No ownership, teardown or retained-result claim is made. The admitted getters
(`a_plain_thread_local_getter_is_safe`: `=` and initializer forms,
`_Thread_local`, a typedef'd record pointer, `extern` plus definition, a null
test with a local-to-local copy, a block-scope thread-local) are `Safe` and
their certificates recheck.

### Choice of rule id

No rule reads `pthread_getspecific`'s result: it is a provider contract
(`libc/threads.qv`, `resource_return_none()`), so per the brief the reading
takes the new id 54 (51 and 52 are reserved for CURRENT-CONTEXT-NEW-TRIAGE
phase C, 53 for OPENSSL-PROVIDER-PLUGIN; every numeric and lettered id was
checked: 1-50 with 45 and 47 unused, none at 54). Rule 54 names itself the
completion of rule 11h (3) for a copy into a local; 11h's text is unchanged.

## 3. Lean

`proof/QCore/ThreadLocalRead.lean`, audited by
`proof/AuditThreadLocalRead.lean` (13 theorems, no `sorry`, no project
axiom): the decision table (`decision_vectors_hold`, bound to
`thread_local_read::DECISION_VECTORS` by the shared-vector test
`the_lean_decision_vectors_match_the_shared_table`), the exact admission
(`admitted_exactly_on_a_custody_free_use`), each refusal family
(`a_write_to_the_object_withholds`, `a_use_through_it_withholds`,
`a_non_pointer_object_withholds`), and the transfer over bindings and
obligation states: the admitted copy is the `return_none` reading
(`the_copy_is_the_return_none_reading`), creates and discharges no obligation
(`the_copy_changes_no_obligation`), leaves the copy unbound and every other
binding unchanged, a release of the copy refuses
(`a_release_of_the_copy_refuses`), returning it escapes nothing
(`returning_the_copy_escapes_nothing`), and withheld or with a bound source
the old answer is unchanged (`the_withheld_answer_is_unchanged`,
`a_bound_source_keeps_the_old_answer`).

## 4. Prediction and measurement

[PREDICTION.md](PREDICTION.md), committed before the semantic commit: system
`flb_engine_evl_get`, `flb_sched_ctx_get` and slice `flb_context_get`,
`flb_cf_context_get` (two configurations each) `Unknown -> Safe`, Safe +4 on
each route; corpus and current no move. The measurement is the integrator's
(see the hand-back for the command, base `rev:0f6e17118`).

## 5. Concurrent work

The only shared file with CURRENT-CONTEXT-NEW-TRIAGE phase C is
`legacy_transfer.rs`: this card adds one `Alias` arm before the generic one
in `transfer_base` and one `TransferContext` field, not the refused-store
code. No function is touched that the `pthread_getspecific` reading touches
(that reading is a contract; this one is a source read). Nothing in
`providers/` changes.

## 6. Follow-ups

Registered through [integration-amendments.json](integration-amendments.json):
`THREAD-LOCAL-READ-BORROWED-USES` (a copy handed to a borrowed actual or
read through is withheld) and `STATIC-POINTER-OBJECT-READ` (the same reading
for a non-thread-local static object). Appended to
`W46-FLUENT-TLS-RETAINED-RESULT`: `flb_output_thread_instance_get`'s own body
is this rule's shape; its retained-result relation stays open.

## 7. Gate

Attempt 1 (head `de1070a20`, `private/gate-1/`): fmt, clippy, both probes,
Lean (435 audits) and Python (761 tests) exit 0; the Rust suite exited 101
with 9,522 passed, 1 failed, 17 ignored: `qcore-audit::test::configured_cli`
`one_configured_unit_runs_each_preparation_stage_once` counted two
`-fsyntax-only` Clang launches where it pins one. Rule 54 consulted the unit
index for every function, which launches the unit AST even in a unit with no
thread-local object. Repaired in `1ac1fabbd`: the reading is asked only where
the unit spells `__thread` or `_Thread_local` before the body's end (as rule
11h asks only where it spells `volatile`); the test, the card's fixtures,
`storage_duration_volatile` and `libc_gap_declarations` then passed
(targeted run). Attempt 2 (head `b2773869a`, `private/gate-2/`, full mode):
submodules, fmt, clippy, both probes exit 0; Rust suite exit 0 (639 units,
9,523 passed, 0 failed, 17 ignored, 148 filtered); Lean exit 0 (435 audits);
Python exit 0 (761 tests). The provider gate is not assigned (no
`providers/` change).

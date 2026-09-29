# W47-PREMISES-INSTANCE: the list-head instance's premises, derived

Card W47-PREMISES-INSTANCE (wave 47, card-high), amending gateway rule
**10p**. Base: local `main` at `fe603d49` (the worktree's HEAD was already
there). The frozen prediction is [PREDICTION.md](PREDICTION.md), commit
`210f48e0`, made after BEFORE was measured and before any code of this card.

Nothing here is a verified race, a verified bug, a verified fix, an `Unsafe`
claim or an upstream defect claim. Every old-vs-fixed statement has the form
*discharged / not discharged / unresolved at this path under these derived
premises*. The Fluent Bit implementation and test are unchanged, and no
provider contract changes.

## 1. Result

This card's premises were five of those the fixed test's discharge stated
(rule 10p): P-HEAD-INSTANCE, P-FRESH-CONTEXT, P-REGISTERED-DESCRIPTORS,
P-INSTANCE and P-INSTANCE-FIELDS-BY-MEMBER. A new census mode,
`--derive-premises`, derives each from the pinned source, or narrows it to a
named residual:

| Premise | At the `90978ac6` export, configured |
|---|---|
| P-FRESH-CONTEXT | **derived** |
| P-REGISTERED-DESCRIPTORS | **derived** from the generated `flb_plugins_register`: 50 descriptors, `"tail"` matches 1 (`in_tail_plugin`) |
| P-HEAD-INSTANCE | **narrowed** to P-CREATIONS-SUCCEED: `flb_input_new` returns non-null only after the append (`src/flb_input.c:677`), and a registered descriptor matches |
| P-INSTANCE | **derived** at the fixed test's 3 sites (and `in_docker_events` `:159`); stated and named per site elsewhere |
| P-INSTANCE-FIELDS-BY-MEMBER | **narrowed**: 0 violations; residual of 80 `void *`/integer escapes and 1 other-type view |

With these derivations in place, the fixed test's three sites
(`cb_tail_pause_test`, `tests/runtime/in_tail.c:2483`, `:2492`, `:2494`) are
still **discharged at this path under these derived premises** on
`(flb_lib_worker, NonThreaded)` and `(input_thread, Threaded)`. None of the
five premises is false for this program.

At `90978ac6~1` the old test (the negative control) stays **not
discharged**: its 8 sites keep the test-thread context `T Either`.

No site verdict moves in any of the four premised readings. The six
readings without the flag are byte-identical to BEFORE. Edges, functions and
seeds are unchanged. Corpus, system and slice results are in §5.

## 2. Mechanism

The logic is in `loop_owner_premises_instance.rs`, a child module of
`thread_entry_census.rs`. Thin hooks connect it to the existing code:

- `read_program_premised` and `census_premised`: the old entry points pass
  `None`;
- the list-head end of `loop_owner_instance.rs` and `descriptors_flag`;
- one line in `hazards`;
- the site verdict in `loop_owner_census.rs`.

When a derivation fails, Step B's flag stays underived and the coroutine
route is kept.

- **P-FRESH-CONTEXT.** Both functions are read from the source:
  - `flb_create` stores the one `config = flb_config_init()` result in the
    context it returns, at its top level.
  - `flb_config_init` returns its one `flb_calloc` allocation. `flb_calloc`
    returns only `NULL` or `calloc(…)`.
  - `mk_list_init(&config->inputs)` and `mk_list_init(&config->in_plugins)`
    each run exactly once, at the top level, before every return of
    `config`. Nothing else there names those lists, and `mk_list_init`'s
    body is `list->next = list; list->prev = list;`.
- **P-REGISTERED-DESCRIPTORS.** The example reads the generated root's files.
  A file counts as included by a read file only when every compile command
  of that file resolves an `#include <…>` to it through its `-I`/`-isystem`
  order.
  - `generated/include/fluent-bit/flb_plugins.h` is resolved by both
    commands of `src/flb_config.c`. Its `flb_plugins_register` is parsed as
    `flb_malloc` / `if (!in) {…return -1;}` / `memcpy(in, &OBJ, sizeof…)` /
    `mk_list_add(&in->_head, &config->in_plugins)` blocks, then
    `return 0;`.
  - `flb_config_init` calls it once, at its top level, on `config`, after
    initialising `in_plugins`.
  - Every registered object's name is read from the typed reading, and the
    creation's literal must match one registered name, ignoring ASCII case.
  - Three new hazards:
    - a call of a generated function that names an input list;
    - an unlink or re-link of an input instance: any use of `struct
      flb_input_instance::_head` other than the constructor's append,
      today `flb_input_instance_destroy`'s `mk_list_del`, which no region
      call reaches;
    - a listed-member mutation in a function the census does not read
      (none today).
- **P-HEAD-INSTANCE.** In `flb_input_new`:
  - every return except the final `return instance;` returns `NULL`;
  - `instance` is `NULL`-initialised and assigned once, from `flb_calloc`;
  - the one `mk_list_add(&instance->_head, &config->inputs)` sits in the
    allocation's block, after it, with no `break`, `continue`, `goto` or
    non-null return between them.

  So a non-null result is on the list.
- **P-INSTANCE.** This is checked per obligated call:
  - Derived when the instance argument is the unassigned first parameter of
    its function, every edge into that function is a loop dispatch
    `C->cb_collect(C->instance, …)`, and a collector's `instance` is
    written only by `collector_create`, from a parameter. The collector
    must have no record-view violation.
  - Refuted when:
    - a dispatch passes another value;
    - the parameter is reassigned or its address is taken;
    - a collector is re-pointed.

    A refuted discharged site becomes unresolved.
  - Anything else stays stated and is named:
    - `ctx->ins`: P-INSTANCE-CONTEXT;
    - a dispatch from a coroutine entry: P-CORO-HANDLE's;
    - a function that is also called directly;
    - another argument form.
- **P-INSTANCE-FIELDS-BY-MEMBER.** New typed facts (`record_views` in
  `typed_dispatch_census.rs`) cover `struct flb_input_instance` and `struct
  flb_input_collector`: whole assignments, copy-initialised and by-value
  objects, initializer lists, and every conversion of a pointer to one,
  with its destination type and use.
  - Violations: an assignment, a copy, an initializer list, a byte
    function (`memcpy`, `memset`, …) or a character view.
  - Residual: conversions to `void *` or an integer, and conversions to
    another object type (covered by C's effective-type rule, a premise
    stated since W47-LOOP-OWNER-TYPED-DISPATCH).
  - Harmless: releases and comparisons.

The facts example now records `_head` accesses and the record views. Every
earlier fact kind is byte-identical to BEFORE: `member_accesses` differs only
by the new `_head` key, and `units` differs only by the previous test's
copied path.

## 3. Negative controls (fixtures first)

`crates/quirin-source-analysis/tests/loop_owner_premises_instance.rs` has 19
tests over the instance-flow fixtures, reshaped to the export's
`flb_config_init`, `flb_create`, `flb_calloc` and `flb_input_new`, with a
generated registration. The module has one unit test of its own. The existing
loop-owner suites (instance flow, typed dispatch, property keys, predefined
macros, coroutine, census) pass unchanged.

| Control | Outcome |
|---|---|
| no generated registration | route kept: "no generated file defines `flb_plugins_register`" |
| the includer's commands do not resolve the generated file | route kept |
| no registered descriptor matches `"tail"` | route kept |
| a descriptor added at run time (a generated function appending to `in_plugins`, called on the path) | route kept: a second hazard-reaching call |
| an internal input created in `flb_config_init` before the test's | route kept |
| a second `flb_input` call | route kept |
| `mk_list_init(&config->inputs)` on one branch | route kept: P-FRESH-CONTEXT not derived |
| an instance unlinked on the path (`mk_list_del(&ins->_head)`) | route kept |
| a non-null return before the append | route kept: P-HEAD-INSTANCE not derived |
| `memcpy` over an instance; `*a = *b`; an `unsigned char *` view | route kept: P-INSTANCE-FIELDS-BY-MEMBER refuted |
| a `void *` escape, a release, a cast to another struct | stay residual; **discharged** |
| a collector re-pointed (`c->instance = other`) | P-INSTANCE refuted; not discharged |
| both loop dispatchers passing another instance | P-INSTANCE refuted; **unresolved** "P-INSTANCE is refuted" |
| the callback reassigning its instance parameter | P-INSTANCE refuted; not discharged |
| the old test's shape | **not discharged** (T); P-INSTANCE stated |
| the fixed shape | **discharged** on both owners, every derivation named, P-INSTANCE derived at both operations |
| the unpremised census on the fixed shape | as before: premises stated, no report |

## 4. Census: `90978ac6~1` against `90978ac6`

The census reads the same 1,685 files. It runs
[`census.py`](scripts/census.py): 10 readings, the six earlier ones plus
raw-premised and configured-premised per test. Inputs:

- AFTER binaries: `loop_owner_census` `2b91f444…`, `typed_dispatch_facts`
  `2cfef100…`.
- Typed facts regenerated: library `e4282cb4…`, fixed `7a982e58…`.
- Predefined-macro facts regenerated by the AFTER binary:
  byte-identical, `9dd8bb73…`.
- [`compare.py`](scripts/compare.py) prints the attribution, archived in
  [artifacts/census/compare.txt.gz](artifacts/census/).

**BEFORE**, on the base binaries, reproduced the archive byte for byte on
all six readings (PREDICTION.md). **AFTER**:

| Reading | d / n / u | edges | functions | seeds | SHA-256 |
|---|---|---|---|---|---|
| six readings without `--derive-premises` | as BEFORE | as BEFORE | as BEFORE | as BEFORE | **byte-identical** to BEFORE |
| `90978ac6` configured, premised | 11 / 130 / 7 (= configured) | 33,529 | 11,932 | 2,852 | `492f47b2…` |
| `90978ac6` raw, premised | 8 / 126 / 10 (= raw) | 33,493 | 11,964 | 2,902 | `f17a339d…` |
| `~1` configured, premised | 8 / 136 / 7 (= configured) | 33,503 | 11,930 | 2,852 | `0d729fb9…` |
| `~1` raw, premised | 8 / 132 / 7 (= raw) | 33,466 | 11,962 | 2,902 | `5cf59027…` |

**Attribution.** No site's verdict or first reason moves between a
premised reading and its unpremised counterpart.

- At `90978ac6` configured, the only changed field is the fixed test's
  instance-flag reason. It is still derived as no coroutine, and it now
  names:
  - P-FRESH-CONTEXT derived;
  - P-HEAD-INSTANCE narrowed, with P-CREATIONS-SUCCEED stated;
  - P-INSTANCE-FIELDS-BY-MEMBER narrowed, with its residual;
  - P-REGISTERED-DESCRIPTORS derived;
  - the creation's literal matching `in_tail_plugin`.

  It no longer marks the three premises as stated.
- In the raw readings the list-head derivation stops earlier (`flb_config.c`
  unstructured), so the reason is unchanged.
- The new `premises` object reports P-INSTANCE for all 148 obligated sites
  of each reading. At `90978ac6` configured:

  | P-INSTANCE form | Sites | Verdicts |
  |---|---|---|
  | derived | 4 | all discharged: the fixed test's 3 and `in_docker_events` `:159` |
  | stated, `ctx->ins` (P-INSTANCE-CONTEXT) | 128 | 2 discharged, `in_fluentbit_metrics` `:82` and `in_stdin` `:272` |
  | stated, through the coroutine entry (P-CORO-HANDLE) | 7 | 2 discharged, `in_event_test` `:124`, `:163` |
  | stated, first parameter of a function also called directly | 4 | 3 discharged, `in_docker_events` `:87`, `:125`, `:197` |
  | stated, another form | 5 | none discharged: `src/flb_input.c:3205`, `src/flb_plugin_proxy.c:177`, `:193`, and the threaded test's helper `tests/runtime/in_tail.c:2449`, `:2450` (`ins` its second parameter) |
  | refuted | 0 | — |

  At `~1` the old test's 8 sites are "another form" (`ins` a local of the
  test): not discharged.

Stated as *discharged / not discharged / unresolved at this path under
these derived premises*:

- **`90978ac6`, fixed test, 3 sites: discharged.** The instance-premise
  derivations of this card, P-FRESH-CONTEXT, P-REGISTERED-DESCRIPTORS and
  P-INSTANCE, all hold. P-HEAD-INSTANCE and P-INSTANCE-FIELDS-BY-MEMBER are
  narrowed to the residuals of §6.
- **`90978ac6~1`, old test, 8 sites: not discharged** (T). There is no plugin
  copy, so Step B does not apply, and the Lean theorem
  `the_old_test_path_is_never_discharged_under_the_premises` covers the
  premised verdict.
- **Other sites: unchanged**, the premise each rests on named per site.

No site moves from Safe to non-Safe (the census publishes no `Safe`), so no
correction is named. No leak or violation is added.

**Condvar/join and lifecycle census.** `w47-flb-lib-lifecycle`'s
`census.sh` was compared with the archive using `condvar_identity.py`; the
exits are in §5. The old test's C5 status does not depend on this card.

## 5. Corpus, system and tail-pause slice

ARBITERS

## 6. Premises of the three discharged sites

Stated as: **discharged at `tests/runtime/in_tail.c:2483`, `:2492`, `:2494`
(the `90978ac6` export, configured and premised reading) under these derived
premises**, never a verified fix.

**Derived, and newly derived by this card:**

- P-FRESH-CONTEXT;
- P-REGISTERED-DESCRIPTORS;
- P-INSTANCE at each of the three sites;
- the appended creation, which is P-HEAD-INSTANCE's conditional part;
- the absence of any non-member write, copy, byte function or character
  view of an instance or collector.

**Derived by earlier cards:**

- the owners, the coroutine hop, the typed dispatch;
- the one creation, the holder escape and P-CALLBACK-INSTANCE;
- the property keys and the predefined-macro decisions.

**Stated, not proved (trust boundary 28), and narrowed by this card:**

- **P-CREATIONS-SUCCEED.** `flb_create()` at `:217` and the one creation's
  `flb_input_new` (through `flb_input` at `:226`) return non-null. The
  test checks neither before its read at `:2551-2552`.
  - A null context goes on to `flb_service_set`.
  - `TEST_CHECK(i_ffd >= 0)` does not return.
  - A failed creation leaves the fresh list empty, so its head is not an
    instance; the census says nothing of that read.

  This is held as `flb_input`'s return clause and `flb_create`'s nullable
  result in `fluent_bit/input_instance.qv`.
- **The residual of P-INSTANCE-FIELDS-BY-MEMBER**:
  - the 80 instance pointers converted to `void *` or an integer are not
    written through bytewise;
  - the one conversion to another object type is not used to access an
    instance (C's effective-type rule). This is `plugins/in_blob/blob.c:761`,
    which casts `in_blob_notification`'s first parameter, declared
    `struct flb_input_instance *`, to `struct blob_ctx *`. Its one
    dispatch, `src/flb_notification.c:318`, passes
    `instance.input->context` there, so the parameter never points to an
    instance. This is a candidate type pun in the `cb_notification`
    signature, not a defect claim, and there is no evidence of an access
    reaching an instance;
  - no offset from another member's address reaches `p`,
    `runs_in_coroutine` or a collector's `instance`.
- **That the generated root's file is the one the build compiled.**

**Stated, carried from rule 10p's other cards:**

- P-CORO-HANDLE, P-PLUGIN-DIR (including that the registered heap copy's
  flags are the static descriptor's at registration), the immutable
  threaded flag and P-HOLDER-OPAQUE;
- the region-macro-call premise, P-KEY-FOLD, P-RECORDED-COMPILER and
  P-DD-COMPLETE;
- acutest's `TEST_CHECK`;
- the lexical and typed readings' faithfulness, and C's effective-type rule.

## 7. Lean

`proof/QCore/LoopOwner.lean`, a new section "The list-head instance's
premises", audited in `proof/AuditLoopOwner.lean`. The theorem count goes
81 → 104. There is no new audit module, no placeholder and no project axiom.
The new theorems:

- **P-FRESH-CONTEXT**: `a_derived_fresh_context_is_empty`,
  `an_initializer_without_its_list_init_is_not_fresh`,
  `a_fresh_context_starts_the_head_derivation`.
- **P-REGISTERED-DESCRIPTORS**: `every_created_instance_runs_no_coroutine`,
  `a_derived_registration_has_a_match`,
  `an_unread_registration_derives_nothing`,
  `an_unread_registration_keeps_the_route`.
- **P-HEAD-INSTANCE**: `a_straight_block_appends_or_returns_null`,
  `a_non_null_creation_is_on_the_list`,
  `a_failed_creation_leaves_the_fresh_list_empty`.
- **P-INSTANCE**: `a_constructor_only_collector_names_its_registration`,
  `a_repointed_collector_may_name_another`,
  `a_refuted_instance_is_never_discharged`,
  `the_premises_never_add_a_discharge`,
  `a_derived_premise_keeps_the_verdict`,
  `the_old_test_path_is_never_discharged_under_the_premises`.
- **P-INSTANCE-FIELDS-BY-MEMBER**: `a_non_member_write_keeps_the_route`,
  `allowed_member_writes_keep_the_derivation`.
- Six lemmas:
  - `initStep_initList`, `initStep_other`, `initStep_others`;
  - `initStep_none_without_init`, `listStep_others`.

## 8. Prediction against outcome

| Measure | Predicted | Measured |
|---|---|---|
| P-FRESH-CONTEXT | derived (85%) | **as predicted** |
| P-REGISTERED-DESCRIPTORS | derived, no generated or unlink hazard reached (65%) | **as predicted** (50 registered, 1 match) |
| P-HEAD-INSTANCE | narrowed to the creations' success (85%) | **as predicted** |
| P-INSTANCE, fixed ×3 | derived (80%) | **as predicted** |
| P-INSTANCE, other 8 discharged | stated under the `ctx->ins` residual (70%) | **partly**: 1 derived (`:159`), 2 `ctx->ins`, 3 first-parameter-also-called, 2 coroutine-entry |
| P-INSTANCE-FIELDS-BY-MEMBER | no violation, escapes > 0 (55%) | **as predicted after one classification**: the first exploratory reading counted `in_blob`'s other-type conversion as a violation; it is an effective-type case, now residual (§6) |
| fixed ×3 | discharged (55%) | **as predicted** |
| old test ×8 | not discharged (99%) | **as predicted** |
| other sites / raw | no change (80% / 95%) | **as predicted** |
| edges, functions, seeds | unchanged (90%) | **as predicted** |
| control readings | byte-identical (90%) | **as predicted** (all six) |
| typed facts | earlier kinds identical (85%) | **as predicted** |
| corpus / system / slice | unchanged (95%) | see §5 |
| condvar/lifecycle | untouched (95%) | see §4 |

The exploratory reading of the fifth row was run before the final binary,
with the violation classification as first written. It is disclosed here
and was not archived as a result.

## 9. Follow-ups

[`followups.json`](followups.json) is applied by the idempotent
[`scripts/update_ledger.py`](scripts/update_ledger.py): 568 → 572.

- **Retired:** none.
- **Registered:**
  - `W47-PREMISES-CREATIONS-SUCCEED`: 1 residual; 3 sites rest on it.
  - `W47-P-INSTANCE-CONTEXT`: 144 stated sites, 7 of them discharged.
  - `W47-FIELDS-BY-MEMBER-VOID-ESCAPES`: 80 escapes, 1 other-type view,
    0 violations.
  - `W47-INSTANCE-CONTRACT-REASONS-AFTER-PREMISES`: 2 held reasons and 1
    comment are stale; no contract is changed here.
- **Amended:** `W47-LOOP-OWNER-INSTANCE-FLOW` and
  `W47-INSTANCE-FLOW-CONTRACT-REVIEW`.

## 10. Gate

GATE

**Files likely to conflict:**

- `docs/agents/gateway.md` (the 10p amendment, appended after the
  predefined-macro one) and `docs/agents/README.md` (the 10p row);
- `docs/STATUS.md`, `docs/development/increments/wave-47.md`,
  `docs/reports/INDEX.md` and `docs/reviews/r0_3/acceptance_matrix.json`;
- `crates/qcore-c-gateway/src/thread_entry_census.rs`:
  - the module declaration;
  - `premise_options`;
  - `CensusReading::premises`;
  - `read_program_premised` (the wrapper, with `read_program_typed`'s body
    moved into it);
  - `StepB::premises`;
  - the P-INSTANCE view after `program.property_keys`;
- `crates/qcore-c-gateway/src/loop_owner_instance.rs`:
  - `hazards` made `pub(super)`, with one hook line;
  - `descriptors_flag`;
  - the end of `list_head_instance`;
- `crates/qcore-c-gateway/src/typed_dispatch_census.rs`
  (`record_views`, `TypedOptions::record_views`, the `accesses` and
  `InitListExpr` hooks);
- `crates/quirin-source-analysis/src/loop_owner_census.rs`
  (`census_premised`) and the examples `loop_owner_census.rs` and
  `typed_dispatch_facts.rs`;
- `proof/QCore/LoopOwner.lean` and `proof/AuditLoopOwner.lean` (a new
  section at the end).

## Reproduce

```sh
python3 docs/reports/w47-loop-owner-typed-dispatch/scripts/facts.py OUT FACTS_BINARY GENERATED_DIR
python3 docs/reports/w47-loop-owner-predefined-macro-arms/scripts/predefined_facts.py OUT PREDEFINED_BINARY GENERATED_DIR
python3 docs/reports/w47-premises-instance/scripts/census.py OUT CENSUS_BINARY TYPED_FACTS PREDEFINED_FACTS
python3 docs/reports/w47-premises-instance/scripts/compare.py BEFORE_CENSUS AFTER_CENSUS
python3 docs/reports/w47-premises-instance/scripts/archive.py AFTER_CENSUS AFTER_FACTS COMPARE_TXT
python3 docs/reports/w47-loop-owner-typed-dispatch/scripts/arbiters.py OUT COMMIT SLICE_RUN_NAME
python3 docs/reports/w47-loop-owner-predefined-macro-arms/scripts/identity.py RUN OUT
python3 docs/reports/w47-loop-owner-predefined-macro-arms/scripts/condvar_identity.py RUN
python3 docs/reports/w47-premises-instance/scripts/gate.py LOGS rust|records
```

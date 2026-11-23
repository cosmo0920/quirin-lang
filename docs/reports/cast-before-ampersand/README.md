# CAST-BEFORE-AMPERSAND-READ-AS-BINARY: a cast close makes the next `&` unary

Card CAST-BEFORE-AMPERSAND-READ-AS-BINARY (USER DECISION 2026-11-23: "launch
the cast-before-ampersand card"). Base `d7540d746` (main).
[PREDICTION.md](PREDICTION.md) was committed (`0cf580aa7`) before the
implementation (`d357d98d8`, `5f0d4b7b7`). Gateway rule
[74f](../../agents/gateway.md#constraint-74f), extending rule 74e;
`QCore.CastBeforeAmpersand` with its audit.

## 1. Census

The question for every reader in the r59 census
([AMPERSAND-RUN-PARITY-OTHER-READERS](../ampersand-run-parity/README.md) §1):
does it decide a single `&` by the token before it, counting `)` as an operand
end? I re-read these at base `d7540d746`:

* the 28 `ampersand_runs` call sites in 19 files;
* the readers that census left unchanged;
* every gateway line where a `"&"` or `'&'` match has a `")"` within six
  lines. A script found 32 such hits, and each was read.

| reader | decides a single `&` by the token before? | class |
|---|---|---|
| `legacy_refused_operands::handed_on` (`ends_operand`, line 398 at the base) | yes. `)` counts as an operand end, so `(T *) &x->f` reads as binary and hands on the value `x->f` alone | **unsound, fixed by this card** |
| `legacy_call_instantiation::residual_takes_an_address` (rule 5j) | yes. A `)` before `&` is unary after `is_cast_close` | **cast-aware**. It now shares the detector, which also catches two casts its own detector missed: the function-pointer cast `(void (*)(int)) &f` and the pointer-to-array cast `(char (*)[4]) &b`. Both fail at the base (below) |
| the other 26 `ampersand_runs` call sites: `loop_owner_instance` ×2, `loop_owner_holder_opaque` ×2, `thread_entry_census` ×3, `legacy_retained_by` ×2, `legacy_callback_route` ×2, `legacy_list_iteration` ×2, `legacy_reference_result`, `indexed_collection`, `aggregate_store`, `registration_by_id` ×2, `typed_field_access`, `borrowed_actual` ×2, `legacy_retention_guard`, `legacy_reference_count`, `legacy_slot_move_borrow`, `legacy_in_place_custody`, `legacy_c::unary_address_runs` | no. Every odd run is an address whatever precedes it (`ends_in_address` alone; for `flow_of`, its even-run test) | **correct**: a cast cannot hide an address from them |
| `logical_and_operand::logical_and_before` (rule 71), with `lexical_nested_assignment.rs:239` | only for a run of exactly two `&` before a group (`X && (p)`). A single `&` there is always an address | **correct, because the context cannot hold a cast**: `(T) && (p)` is not conforming C, since `&&label` takes a name |
| `thread_entry_census.rs:1764` `address_of` | no. Any single `&` before `f (` is an address | correct (fail-closed, as r59 found) |
| `condvar_census/stop_paths.rs:696-697` and r59's other unchanged readers: fused streams, exact shapes, connective detectors, readers that refuse any `&`, binary-operator tables | no | correct |

**The same defect for `*` (out of scope, registered).** `ends_operand` also
decides a `*` before a name:

* `legacy_refused_operands`' `dereference` reads a `*` after `)` as binary,
  so `(char *) *p` hands on `p` instead of `*p`;
* `aggregate_store`'s premise 4 reads a `*` after `)` as no dereference.

Both are `CAST-BEFORE-STAR-READ-AS-BINARY`.

### The ambiguous group

`(a) & b` is a parenthesised value when `a` is an object, and a cast when `a`
is a typedef name.

* **Refused hand-on:** binary only when `a` is one of the function's declared
  objects and no `typedef` declaration in the unit spells it. Otherwise
  unary.
  * The objects are the formals, `body_declarations` and
    `file_scope_objects`, from `publication::value_names`.
  * Every identifier inside any `typedef … ;` is excluded. This removes more
    names than it strictly needs to, which errs toward unary.
* **Rule 5j:** this reader has no declaration knowledge, so `(a) & b` is
  unary. It already was: the old `is_cast_close` read any run of names and
  `*` as a type.

## 2. Changes

**`ampersand_runs::may_close_a_cast(texts, close, objects)`** is the one cast
detector (new). It reads the group a `)` closes as follows.

A group is **not a cast**, so the `&` is binary, when:

* it is an argument list: its `(` follows a name that is not one of `return`,
  `case`, `if`, `while`, `switch`, `for`, `else` or `do`, or it follows a `)`
  or `]`;
* it holds a token at its top level that no scalar cast's type name has
  there (C11 6.5.4p2): an operator other than `*`, a literal, `[`, `.` or
  `,`. Brackets and numbers are allowed once a nested declarator group has
  closed, as in `(char (*)[4])`. Inside a nested group, the allowed tokens
  are names, `*`, brackets, numbers, `,` and `...`;
* it is a single name in `objects`.

Every other group **may be a cast**, so the `&` is unary. That includes an
unmatched `)`.

The changes that use it:

* **`legacy_refused_operands`**:
  * reads a single `&` after a `)` as unary when the detector says the group
    may close a cast;
  * `line_reads` takes `objects` and passes it on to `handed_on_among`;
  * `handed_on` is now a test-only wrapper with no objects.
* **`publication::facts`** computes `value_names` and passes the result in.
  `body_declarations` and `file_scope_objects` are now computed once and
  shared with `destinations_of`, and `destinations` is test-only.
* **`legacy_call_instantiation`** calls the detector, and its `is_cast_close`
  is deleted.
* **`READERS`** now counts 2 call sites in each of these two files (30 in
  total).

## 3. Tests and pins

The fixtures:

* **`ampersand_runs` unit tests.** `CAST_VECTORS` holds 16 vectors bound to
  `QCore.CastBeforeAmpersand.castVectors`. They cover `(T *) &x`,
  `(struct s *) &v`, `(void) &x`, a typedef-name cast, `(a) & b` undecided
  and as an object, `f(x) & mask`, `(*fp)(x) & m`, expression groups, both
  nested type names, `return (T *) &x`, `sizeof (T) & x`, and an unmatched
  `)`.
* **`legacy_refused_operands` unit tests (6):**
  * `a_cast_before_an_address_hands_on_the_chain`;
  * `every_cast_spelling_hands_on_the_chain`;
  * `a_typedef_name_cast_hands_on_the_chain`;
  * `a_parenthesised_object_before_an_ampersand_is_binary`;
  * `a_call_result_or_an_expression_before_an_ampersand_is_binary`;
  * `the_value_names_exclude_every_typedef_spelling`.
* **`legacy_call_instantiation` unit test:**
  `a_return_takes_an_address_after_any_cast`.
* **End-to-end, in `tests/leak_pin_audit.rs`:**
  * `a_cast_interior_address_hand_on_withdraws_the_accusation`:
    `k_link((void *) &b->item)`, `(struct k_box *)` and a typedef `(k_any)`.
    Each is `Unknown` with no leak;
  * `a_call_result_before_an_ampersand_keeps_the_accusation`: the control,
    `k_mask(q->n) & b->n`, stays `Unsafe` with the leak.

**Checked at the base.** I ran the new fixtures against the base's gateway
sources (`git checkout d7540d746 -- crates/qcore-c-gateway/src`). The unit
helper was adapted to the base's `handed_on`, and the test only the tip has
was dropped. Logs are in the worktree's `private/cba/base-fixtures/`.

* `unit.log`: 4 of the refused-operands tests **fail**: the three cast tests,
  and the undecided half of the parenthesised-object test. The call/expression
  control passes.
* `leak_pin_audit.log`: `a_cast_interior_address_hand_on_withdraws_the_accusation`
  **fails**. The base gives `Unsafe` with `["leak", "unsupported:indirect
  memory access is outside the resource projection"]`, and the control passes.
* Rule 5j's test, grafted onto the base source, **fails** on
  `( void ( * ) ( int ) ) & f`.

**Pins:** none moved. All 9,948 Rust tests in the gate pass unchanged.

## 4. Measurement

Opt-in `current --monkey-linked` over Fluent Bit 046accf6. Each run used its
own `git archive` tree and release binary ([evidence/runs.md](evidence/runs.md)).

| run | Safe / Pending / Unknown / Unsafe | access rows / alone / leaves |
|---|---|---|
| base `d7540d746` | 1,172 / 2 / 3,622 / 8 | 2,118 / 215 / 8,992 |
| **tip `5f0d4b7b7`** | **1,172 / 2 / 3,622 / 8** | 2,118 / 215 / 8,992 |

No row moves status, and no row changes its leaf multiset. No `Safe` row is
lost. This is within the prediction, whose `measure-prediction` block was
`none`.

Why the refused hand-on moved nothing here: it acts only at a refused
statement that hands on a live tracked identity which a later exit would
accuse. None of the 337 `* ) & name ->` spellings in the corpus meets those
conditions on this route. The 8 `Unsafe` rows are unchanged.

**Pinned routes** (`quirin-audit measure --declared`) cannot run on this host,
which has no `build/cycle19` inputs or baselines. My prediction for the
integrator:

* **Corpus routes:** no move expected. Any move would be an `Unsafe` →
  `Unknown` leak withdrawal, or a leak leaf gone, at a refused statement that
  spells `(T *) &x->f`.
* **System and slice routes:** no move expected. A `Safe` → `Unknown` move
  could only come from rule 5j's wider detector, on a return that calls a
  function and casts an address through a function-pointer or array type.
  Any move should be shown with its cast in the source.

## 5. Gate

Scoped card gate: `python3 scripts/gate_scope.py --base d7540d746 --out
private/cba/gate-1`, run through `private/toolchain/env.sh` (Clang 21, Lean
4.35) at head `5f0d4b7b7`. Later commits change only this report and its
evidence. Record: [gate/gate-1.json](gate/gate-1.json); logs are in the
worktree's `private/cba/gate-1/`.

| step | exit | result |
|---|---:|---|
| submodules | 0 | 1 submodule |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| `run_test_binaries.py` | 0 | 679 units, 724 summaries; 9,948 passed, 0 failed, 21 ignored, 148 filtered |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `check_lean_audits.py` | 0 | 488 audits |
| `python3 -m unittest discover -s scripts/tests` | 0 | 807 tests |

No provider file changed, so the provider gate was not selected.

## 6. Ledger and follow-ups

**CAST-BEFORE-AMPERSAND-READ-AS-BINARY is retired.**

* Its DISCHARGE asked for `) &` to read as unary when the group is a cast.
  Both readers that decide a single `&` by the preceding `)` now use one
  shared detector, and an undecided group resolves to unary.
* It asked for `(T *) &x->f` and `(a) & b` to be pinned. They are, with the
  other required spellings, at unit level and end to end.
* It asked for the corpus to be measured. It was: no move.

Registered in [integration-amendments.json](integration-amendments.json):

* **CAST-BEFORE-STAR-READ-AS-BINARY**: the `*` half of the same
  `ends_operand` reading (§1).

**Files likely to conflict:**

* `crates/qcore-c-gateway/src/ampersand_runs.rs`, `legacy_refused_operands.rs`,
  `legacy_call_instantiation.rs` and `publication.rs`;
* `crates/qcore-c-gateway/tests/leak_pin_audit.rs` and
  `tests/unit/legacy_refused_operands.rs` (tests appended);
* `docs/agents/rules/74f.md` (new);
* generated files: `docs/agents/gateway.md`, `docs/agents/README.md`,
  `proof/QCore.lean` and `proof/AuditAll.lean`. Rerun their generators on a
  conflict.

None of these overlaps OPENSSL-RELEASED-PARAMETER-SLOTS' files.

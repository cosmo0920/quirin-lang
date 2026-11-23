# CAST-BEFORE-STAR-READ-AS-BINARY: a cast close lets the next `*` be a dereference

Card CAST-BEFORE-STAR-READ-AS-BINARY (USER DECISION 2026-11-23: "launch the
cast-before-star card"). Base `7450d69af` (main, r61).
[PREDICTION.md](PREDICTION.md) was committed (`2dcfb97b2`) before the
implementation (`269dfee4a`, `404136f21`). Gateway rule
[74g](../../agents/gateway.md#constraint-74g), extending rule 74f;
`QCore.CastBeforeStar` with its audit.

## 1. Census

The question for each gateway reader: does it decide whether a single `*` is
a unary dereference or a binary multiplication by the token before it, and
does it count `)` as an operand end? I re-read these at base `7450d69af`:

* every `"*"` match in `crates/qcore-c-gateway/src` that has a `")"` within
  six lines (100 hits in 50 files, the r61 method);
* every `"*"` match that has a previous-token test (`- 1]`, `sub(1)`,
  `previous`, `before`, `left`) within four lines, which adds readers that
  hold their `)` test in a helper. That adds the lexical access gate and
  `legacy_return_family`;
* the `gateway-rules` crates (`rule-35`'s `literal_read` is the only `*`
  reader there).

Each hit was read. Most decide a declarator (`T *name`), compare whole token
shapes (`( void * ) 0`), or list `*` in an operator table, and say nothing
about `) *`.

| reader | decides a single `*` by the token before? | class |
|---|---|---|
| `legacy_refused_operands::handed_on_among`, `dereference` (`ends_operand`) | yes. `)` ends an operand, so `(char *) *p` handed on `p`, never the slot `*p` | **unsound, fixed by this card** |
| `aggregate_store::store_bases`, premise 4 `dereferenced` | yes. A `*` after `)` was no dereference, so `(void) *b` left `b` an admissible store base | **admitting, fixed by this card** |
| `legacy_return_family::unary_position` (family S: `stars_before`, the `starred` base, `scalar_read`'s `&`) | yes. A `)` is unary only when its own `is_cast_group` resolves the group as a type the unit declares | **cast-aware, own detector**. An unresolved type name reads as binary, so for `return (T) *p;` with `p` untracked, `p` is not made a base (by reading, not tested). Registered as `RETURN-FAMILY-STAR-AFTER-UNRESOLVED-CAST` (§6) |
| `legacy_token_shapes::contains_unmodeled_access` (the lexical access gate) | yes. A `*` after anything but a name or a number (except `sizeof` and its kin) is an access | **correct, fail-closed**. Every `) *` refuses, `f(x) * n` included |
| `loop_owner_holder_opaque.rs:628` | yes. A `*` before the copy is a dereference unless a name (a declaration's type) precedes it | correct, fail-closed: `) *` refuses |
| `indirect_census::chains` | yes. `)` reads as unary | inert: a measurement behind the off-by-default `indirect-census` feature, consulted by no analysis |
| `legacy_slot_move_borrow.rs:1000`, `:1123`, `legacy_out_alias.rs:134`, `condvar_census/stop_paths.rs:678` | only for a statement-initial store `* name =`. A `)` before the `*` is unary | correct: a multiplication's result is no lvalue, so `) * name =` can only be a store through `name` |
| `condvar_census/stop_paths.rs:440`, `:729` | any `*` before the base counts (or refuses) whatever precedes it | correct: a cast cannot hide the dereference |
| `rule-35-literal-decay::literal_read` | no. `* "literal"` whatever precedes it | correct: a binary `*` with a literal operand is not conforming |
| declarator readers (`typed_field_access.rs:718`, `legacy_declared_abi.rs:642`, `:1121`, `legacy_literal_premise.rs:555`, `legacy_container_publication.rs:289`, `legacy_enumeration.rs:957`, `loop_owner_instance.rs:565`, `loop_owner_coro_plugin.rs:843`, `condvar_census/lifecycle.rs:255`, `loop_owner_property_keys.rs:597`, `registration_by_id.rs:489`, `thread_entry_census.rs:1014`, `legacy_relational_forwarding.rs:237`) | no. They tell a declarator `*` from an expression and do not read `) *` | not this question |

### The direction for each reader

**The refused hand-on.** Rule 74f chose the unary reading for `&` because a
unary `&` hands on every prefix of the chain, which contains the binary
reading's one place. For `*` that is not so. A unary `*p` hands on the slot
`*p`, a binary `a * p` hands on `p`, and neither set contains the other.
After a `)` that may close a cast, the reader therefore hands on **both**.
The union contains the places of each reading, so it is the fail-closed
answer for a certain cast and for an undecided `(a) * p` alike. The brief
expected the unary reading itself to hand on more, as for `&`; it does not,
and the union is what keeps the fix one-directional. A group the detector
rules out stays binary: a single declared object (rule 74f's
`publication::value_names`, less every typedef-spelled name), an argument
list, or a value group.

**Premise 4.** A dereferenced base is refused, and a refused base is never
admitted. So reading a dereference refuses more, which is the fail-closed
direction. The detector is asked with no declared objects, because a
candidate base is a record pointer and C11 6.5.5p2 forbids a pointer
operand of a binary `*`. Even `(a) * b` with `a` an object can only be a
dereference there. An argument list stays binary, as at the base. That
keeps `sizeof (T) * b` and the declarator in `__typeof__ (x) * b` from
refusing.

## 2. Changes

* **`legacy_refused_operands::handed_on_among`**: `cast_star` is a `*` before
  a lone name whose preceding `)` `may_close_a_cast(statement, close,
  objects)`. Such a `*` hands on `*p` and `p`. The old `dereference` (a `*`
  after no operand end) is unchanged.
* **`aggregate_store::store_bases`**: premise 4's `dereferenced` also holds
  after a `)` that `may_close_a_cast(body, close, {})`.
* **`ampersand_runs`**: the detector's documentation names the `*` readers,
  `READERS` counts 3 call sites in `legacy_refused_operands.rs` and 2 in
  `aggregate_store.rs` (32 in all, class `fail-closed+cast`), and a new
  `STAR_VECTORS` table holds the `*` spellings. The detector itself is
  unchanged: its name still fits, since it decides whether a `)` may close a
  cast, whatever operator follows.

## 3. Tests and pins

* **`ampersand_runs`**: `rule_74g_star_vectors_hold` checks 10 vectors bound
  to `QCore.CastBeforeStar.starVectors`. They are `(char *) *p`,
  `(unsigned char) *p`, `(a) * b` undecided and as an object, `f(x) * n`,
  both nested type names, `sizeof (T *) * n`, `__typeof__ (x) * b` and
  `(a + 1) * n`.
* **`legacy_refused_operands`** (3 unit tests):
  * `a_cast_before_a_dereference_hands_on_the_slot` covers `(char *)`,
    `(struct s *)`, the function-pointer cast and the pointer-to-array cast,
    as a call actual, an assigned value and a return. Each hands on `*p`
    and `p`.
  * `a_parenthesised_object_before_a_star_is_binary`: `(a) * b` with `a`
    an object hands on `a` and `b`; with `a` undecided, or a typedef-like
    `my_t`, it also hands on `*b`.
  * `a_call_result_or_an_expression_before_a_star_is_binary`: `f(x) * n`,
    `(a + 1) * n`, `(*fp)(x) * n`, `sizeof (T *) * n` and `(s.n) * k`.
* **`aggregate_store`** (2 unit tests):
  * `a_cast_before_a_dereferenced_base_is_refused` covers `(void) *b`,
    `(struct box) *b`, `(my_t) *b`, `(a) * b` with `a` declared, and both
    nested type names.
  * `an_argument_list_before_a_star_is_no_dereference` covers
    `sizeof (int) * b` and `f (x) * b`.
* **End to end:**
  * `tests/alias_aggregate.rs`:
    * `a_cast_before_a_dereferenced_base_refuses_its_stores`: `V_CREATE`
      with `(void) *b;` before its return. The opaque store `b->any = it`
      now refuses, because `b` is no admissible base.
    * The control `an_argument_list_before_a_star_keeps_the_base`
      (`n = sizeof (int) * n;`) refuses only its own line.
  * `tests/leak_pin_audit.rs`:
    * `a_cast_before_a_dereference_keeps_the_root_hand_on`: the refused
      statement `*q = k_mask(1) * (long) *b;` is `Unknown` with no leak.
      This pins the union: a unary-only reading would hand on `*b` alone
      and keep the leak.
    * `a_dereference_after_an_operator_keeps_the_accusation` checks
      `+ *b`, `* *b` and `k_mask(b[0]) * n`. Each stays `Unsafe` with the
      leak.

**Checked at the base.** On the WIP commit `269dfee4a`, I replaced the two
logic hunks with the base's reading: `cast_star` forced false, and premise 4's
cast arm removed. I then ran the new tests. Logs are in the worktree's
`private/cbs/base-fixtures/`. The source was restored from the commit
afterwards.

* `unit.log`: 3 tests fail.
  * `a_cast_before_a_dereference_hands_on_the_slot`: at the base the
    reader handed on `["char", "p"]`, with no `*p`.
  * The undecided half of `a_parenthesised_object_before_a_star_is_binary`:
    `["a", "b"]` instead of `["*b", "a", "b"]`.
  * `a_cast_before_a_dereferenced_base_is_refused`: `( void ) * b ;` left
    `b` admissible.

  The binary controls and `rule_74g_star_vectors_hold` pass, since the
  detector is unchanged.
* `e2e.log`: `a_cast_before_a_dereferenced_base_refuses_its_stores` **fails**.
  At the base only line 11, the `(void) *b` statement itself, refuses; the
  control passes.
* `leak_pin_audit.log`: all 16 pass. Both new leak pins hold at the base
  too. The union keeps the base's `p` hand-on, and a `*p` hand-on changes a
  verdict only where `*p` is a bound place at a refused node, which the
  gateway has only for a sanctioned out slot.

**No end-to-end pin moves the refused hand-on.** I tried out-slot fixtures
for that (`*out = p; k_note((char *) *out); *out = q;`), with the body's
return-code relation and with a declared `resource_out` contract on the
analysed function. In each, the analysis read the store as unsanctioned
("store through `*out` without an out-parameter contract"), so `*out` was
never bound when the refused statement ran. The change to the refused
hand-on is therefore pinned at unit level (`handed_on_among`, which feeds
`line_reads`).

**Pins:** none moved. The gate's Rust suite passes unchanged apart from the
new tests (§5).

## 4. Measurement

Opt-in `current --monkey-linked` over Fluent Bit 046accf6. Each run used its
own `git archive` tree and release binary ([evidence/runs.md](evidence/runs.md)).

| run | Safe / Pending / Unknown / Unsafe | access rows / alone / leaves |
|---|---|---|
| base `7450d69af` | 1,172 / 2 / 3,622 / 8 | 2,118 / 215 / 8,992 |
| **tip `404136f21`** | **1,172 / 2 / 3,622 / 8** | 2,118 / 215 / 8,992 |

No row moves status, and no row changes its leaf multiset. No `Safe` row is
lost. This matches the prediction, whose `measure-prediction` block was `none`
on every route.

The refused hand-on adds only `*p` places, and these matter only where `*p` is
a bound place holding a live tracked identity at a refused statement. The
corpus's `(T) *p` spellings are character reads (`(unsigned char) *p`). Its
`* ) * name` spellings are `sizeof (T *) * n`, which the detector reads as
binary, as before. Premise 4 would refuse only a record-pointer local
dereferenced whole under a cast, and no measured row has one.

**Pinned routes** (`quirin-audit measure --declared`) cannot run on this host,
which has no `build/cycle19` inputs or baselines. No move is predicted
(PREDICTION.md). A move on a corpus route would be an `Unsafe` → `Unknown`
leak withdrawal at a refused `(T) *slot`. A move on the system or slice
routes would be a `Safe` → `Unknown` move from premise 4 at `(void) *b`. Any
move should be shown with its cast in the source.

## 5. Gate

Scoped card gate: `python3 scripts/gate_scope.py --base 7450d69af --out
private/cbs/gate-1`, run through `private/toolchain/env.sh` (Clang 21, Lean
4.35) at head `404136f21`, with exit 0 and all 8 steps passed. Later commits
change only this report and its evidence. Record:
[gate/gate-1.json](gate/gate-1.json); logs are in the worktree's
`private/cbs/gate-1/`.

| step | exit | result |
|---|---:|---|
| submodules | 0 | 1 submodule |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| `run_test_binaries.py` | 0 | 679 units, 724 summaries; 9,963 passed, 0 failed, 21 ignored, 148 filtered |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `check_lean_audits.py` | 0 | 490 audits |
| `python3 -m unittest discover -s scripts/tests` | 0 | 807 tests |

No provider file changed, so the provider gate was not selected.

## 6. Ledger and follow-ups

**CAST-BEFORE-STAR-READ-AS-BINARY is retired.**

* Its DISCHARGE asked for `) *` to be read through `may_close_a_cast`. Both
  named readers now do, and no second detector was added.
* It asked for `(char *) *p` and `f(x) * n` to be pinned. They are, with the
  other required spellings.
* It asked for the corpus to be measured. It was (§4): no move, as predicted.

Registered in [integration-amendments.json](integration-amendments.json):

* **RETURN-FAMILY-STAR-AFTER-UNRESOLVED-CAST**: `legacy_return_family`'s
  `unary_position` reads a `*` or `&` after a group its own detector cannot
  resolve as binary (§1). The shared detector is not a drop-in replacement
  there. Its unary reading would turn `stars_before`'s refusal for a
  may-bind root into an admitted dereference base, which moves verdicts away
  from `Unknown`, so it needs its own direction assessment.

**Files likely to conflict:**

* `crates/qcore-c-gateway/src/ampersand_runs.rs`,
  `legacy_refused_operands.rs` and `aggregate_store.rs`;
* `crates/qcore-c-gateway/tests/leak_pin_audit.rs`,
  `tests/alias_aggregate.rs` and `tests/unit/legacy_refused_operands.rs`
  (tests appended);
* `docs/agents/rules/74g.md` (new);
* the generated files `docs/agents/gateway.md`, `docs/agents/README.md` and
  `proof/QCore.lean`. Rerun their generators on a conflict.

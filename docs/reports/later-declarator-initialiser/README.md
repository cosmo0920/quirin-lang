# LATER-DECLARATOR-INITIALISER-UNREAD: each init-declarator is its own declaration

Card LATER-DECLARATOR-INITIALISER-UNREAD (USER DECISION 2026-11-23, one of the
"soundness first" four). Base `2c60986b0` (main). [PREDICTION.md](PREDICTION.md)
was committed (`ca2a04981`) before the implementation (`f4f5a11a1`). Gateway
rule [74k](../../agents/gateway.md#constraint-74k); `QCore.LaterDeclarator`
with `AuditLaterDeclarator.lean`.

## 1. Census: who read `, *b = Y` as a store through `*b`

| reader | what it did with `T *a = X, *b = Y;` | after the card |
|---|---|---|
| statement classifier (`legacy_classify.rs`, `classify_simple_base`), the one statement reader of the CFG | first top-level `=`: destination `a`, value `X , * b = Y`; the lexical gate reads `* b` as a dereference and refuses the whole declaration (`indirect memory access`), binding neither name | asked only for the pieces when rule 74k splits; otherwise unchanged |
| lexical closure, rule 10o (`lexical_nested_assignment::assignments`) | reads every simple `=`, the later declarator's included, as an assignment to `b` (a name, not a store through it) | reused: the splitter accepts an initialiser only when its `=` is the one rule 10o reads, with the declarator's name as destination |
| rule 38h (`typed_declaration.rs`) | admits a declaration only when **no** declarator is initialised | reused per piece for an uninitialised declarator (its facts for the declaration alone) |
| rule 41 (`legacy_scalar_statement_forms.rs`) | reads each `VarDecl` of an all-scalar declaration from the AST; a pointer declarator is not its | unchanged: a declaration of scalars alone is not split (rules 41 and O04 keep it) |
| rule 10s elaborator | one-declarator declarations only | unchanged |
| statement fallbacks reading from a statement's first `=` (`legacy_call_argument_access`, `legacy_counted_users`, `legacy_layout_extents`, `legacy_list_iteration`, `index_traversal`, `legacy_cast_representation`) | asked on the gate's refusal of the whole declaration | asked per piece when it is split, as for the declaration written alone |
| census-only readers (`destination_suffix_census`, `legacy_parser_census`) | measurement only | unchanged |

Only the statement classifier read the later declarator as a store through a
pointer; the others either read it as a name or decline the declaration.

## 2. Change

`later_declarator.rs` (pure) splits a declaration's top-level commas when the
specifiers are words, every later declarator is `*`-and-qualifier prefixes then
a name, every initialiser is non-empty and is rule 10o's assignment to that
name, there are no braces and no repeated name. `legacy_later_declarator.rs`,
asked by the statement walker before the statement's own lowering, then needs
one Clang `DeclStmt` at the line with the exact tokens whose `VarDecl`s are the
split's names in order, automatic, initialised exactly when spelled so, and at
least one pointer declarator. Per declarator (the decision table bound to Lean
by 128 shared vectors): uninitialised and rule 38h's facts hold alone → no
event; pointer → the declaration written alone through the unchanged statement
lowering, unless the initialiser spells `* )` and is not the null constant
(rule 12h's cast repair reads only the statement's own Clang node, which a
piece lacks); scalar → only a numeric literal (no pointer truth-value
conversion for rule 11b's repair to see). One unread declarator keeps the whole
declaration on its old reading. The statement is lowered as the block of its
pieces.

Lean (`proof/QCore/LaterDeclarator.lean`): C's ordered meaning of a declaration
is the sequence of its one-per-statement declarations; the lowered block runs as
the sequence of the pieces' lowerings, so a reading faithful for one declarator
alone is faithful for the declaration; top-level comma splitting gives the
declarators back; the decision table and its fail-closed theorems. Premises not
proved: the Rust splitter and the Clang match (trust boundary 28), and C's
meaning as an ordered fold (C11 6.7.6p3, 6.2.1p7).

## 3. Pins

New: [`tests/later_declarator_initialiser.rs`](../../../crates/qcore-c-gateway/tests/later_declarator_initialiser.rs),
8 fixtures. The acceptance test compares the declaration with the same
declarations written one per statement on the same line (status and every
diagnostic): the Heartbeat shape (`list_t *sent = NULL, *existing = NULL, *next
= NULL;` with `RD_IF_FREE`, `Safe`), `void *a = 0, *r = 0;` (`Safe`), a leak
behind a later declarator (`Unsafe`), mixed declarators (`int n = 0, *p = q;`,
`void *a, *b = q_alloc();`, the typedef spelling, an alias of an earlier
declarator), an uncontracted initialiser (`Unknown`), rule 38h's uninitialised
pair. Refused, no leak: `char *a = 0, b[4];`, `void *a = 0, *(*fp)(void) = g;`,
`void *a = 0, *(r) = 0;`; the old access refusal kept for `void *a = 0, *r =
(void *) s;` and `int n = k, *r = 0;`; `static` keeps rule 11h's reading.

Deliberate pin updates, each to the one-per-statement verdict:

| test | was | now |
|---|---|---|
| `nullable_lazy_constructor.rs` `the_other_shapes_stay_refused_without_a_leak` (`void *a = 0, *r = 0;`) | `Unknown` (access, nullable, identity) | `Safe` |
| `ag1_indirect_member_access.rs` `two_declarators` | gate refusal | `Safe`, as its one-per-statement spelling |
| `member_access_projection.rs` `initialized` (`char *p = NULL, *q = NULL;`) | gate refusal | `Safe` |
| `scalar_statement_forms.rs` `int a = 0, *p = 0;` | `Unknown` (access) | `Safe` |
| `lexical_closure_nested_assignment.rs` `struct c_box *a = c_make(1), *b = a; … c_drop(a); b->item` | `Unknown` (access) | `Unsafe`: the use after release it is, as one per statement |
| `chained_assignment.rs` scram shape (`char *safe = 0, *d = 0;`) | `Unknown`, no leak | `Unsafe` on a provisional loop re-acquisition leak, equal to the one-per-statement spelling's verdict (asserted); not a C leak (PASS-CORRELATED-LOOP-ALLOCATION) |

Every new or changed pin fails at the base: the tip's seven test files run
against the base sources ([evidence/pins-at-base.txt](evidence/pins-at-base.txt))
fail 5 of the 8 new fixtures (the three that pass are the stays-refused and
stays-admitted ones) and each updated test.

## 4. Measurement

Opt-in `current --monkey-linked` over Fluent Bit 046accf6, base and tip each
from its own `git archive` tree and release binary
([evidence/runs.md](evidence/runs.md)).

| run | Safe / Pending / Unknown / Unsafe | access rows / alone / leaves |
|---|---|---|
| base `2c60986b0` | 1,172 / 2 / 3,622 / 8 | 2,118 / 215 / 8,992 |
| **tip `f4f5a11a1`** | **1,173 / 2 / 3,621 / 8** | 2,115 / 214 / 8,980 |

**One status move**, Unknown → Safe: `rd_kafka_sw_str_sanitize_inplace`
(`rdkafka_conf.c`), whose only leaf was its declaration `char *s = str, *d =
str;`. Read one per statement, every statement after it is admitted; the row
rests on no declaration and on seven null-dereference premises on `s` and `d`
(the console's "only on a null premise" count rises 16 → 17), the kind of
premise the other 16 such rows rest on. It touches no resource. No row moved to
or from `Unsafe`.

13 rows change ([evidence/changed-rows-base-tip.txt](evidence/changed-rows-base-tip.txt)):
access leaves −12; `rd_kafka_conf_finalize`'s two `call to uncontracted
function rd_kafka_sw_str_sanitize_inplace` leaves become access leaves at the
same lines (the callee now has a validated body, so the call reaches the access
gate on its actual); `toppar_sort_by_list_cnt` trades the access leaf for
identity leaves on the formals `_a`, `_b` its pieces alias; four
`*_ResponseParse`/`mk_string_trim` rows lose identity leaves the refused
declaration caused.

**ConsumerGroupHeartbeat** (`rd_kafka_mock_handle_ConsumerGroupHeartbeat`):
`Unknown`, 99 → 96 leaves. Gone: the declaration's access leaf (preprocessed
line 23917), the three `existing_assignment` leaves (identity 24072 and 24078,
`nullable … used by a non-null contract` 24072; the `RD_IF_FREE` guard is now
decided) and `next_assignment`'s identity leaf at 24079. New: identity leaves
on `sent_assignment` (23959) and `next_assignment` (24061), names now bound
that reach other refused statements. The row's remaining obstacles are its 18
other access refusals, 14 refused conditions and uncontracted mock-cgrp calls.

**Against the prediction.** Wrong by name. The four predicted Unknown → Safe
rows did not move: their access leaf was not at the declaration (the rough
regex census matched the declaration but the leaf's origin is a later
statement: `a->value` member reads, an indirect `cmp(...)` call, a `?:`
member read), so their rows are unchanged. The row predicted not to move,
`rd_kafka_sw_str_sanitize_inplace`, is the one that moved (its pointer
increments and stores through `d` were already admitted; the declaration was
its only obstacle). The Heartbeat prediction held (stays Unknown; the
declaration and `existing_assignment` leaves go). No unpredicted Unsafe.
Measured delta Safe +1 against the predicted +4.

**Pinned routes** (`quirin-audit measure --declared`) cannot run on this host
(no `build/cycle19` inputs or baselines); not run, nothing recorded for them.
The committed `measure-prediction` block's corpus lines are known wrong from
the opt-in route: the integrator should expect the corpus route to show
`rd_kafka_sw_str_sanitize_inplace Unknown -> Safe` as unpredicted and the four
named moves as missing.

## 5. Gate

Scoped card gate `python3 scripts/gate_scope.py --base 2c60986b0 --out
private/ldi/gate-1` through `private/toolchain/env.sh` (Clang 21, Lean 4.35),
head `f4f5a11a1`; record [gate/gate-1.json](gate/gate-1.json), logs in this
worktree's `private/ldi/gate-1/`. Later commits change this report, its
evidence and scripts only.

| step | exit | result |
|---|---:|---|
| submodules | 0 | 1 submodule |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| `run_test_binaries.py` | 0 | 681 units, 726 summaries; 9,984 passed, 0 failed, 22 ignored, 148 filtered |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `check_lean_audits.py` | 0 | 492 audits |
| `python3 -m unittest discover -s scripts/tests` | 0 | 807 tests |

No provider file changed, so the provider gate was not selected.

## 6. Files likely to conflict

`crates/qcore-c-gateway/src/legacy_c.rs` (one `mod` block beside rule 74d's,
and three lines at the statement walker's expression-statement arm, which
OWNED-CONSTRUCTOR-OUT-PARAMETER-DELIVERY may also touch), `src/lib.rs` (one
`mod`), the generated `docs/agents/gateway.md`, `docs/agents/README.md`,
`proof/QCore.lean`, `proof/AuditAll.lean` (rerun the generators), and the six
test files above if another card edits the same fixtures.

## 7. Ledger and follow-ups

**LATER-DECLARATOR-INITIALISER-UNREAD is retired**: each init-declarator is
read as its own declaration (rule 74k), the corpus measured (one move, 13 rows
changed), the Heartbeat row re-measured (its declaration and
`existing_assignment` leaves gone, still Unknown on other obstacles).

Registered in [integration-amendments.json](integration-amendments.json):

* **PASS-CORRELATED-LOOP-ALLOCATION**: the scram shape's provisional leak.
* **LATER-DECLARATOR-PIECE-CLANG-NODE**: pieces are lowered without a Clang
  node, so the split declines casts other than the null constant and
  non-literal scalar initialisers (`rd_kafka_topic_cmp_rkt`'s `(void *)_a`),
  and `for`-clause and statement-expression declarations are not split.

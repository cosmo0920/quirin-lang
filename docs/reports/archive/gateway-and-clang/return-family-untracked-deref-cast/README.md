# RETURN-FAMILY-UNTRACKED-DEREF-THROUGH-CAST: an untracked root dereferenced through a cast group is a base

Card RETURN-FAMILY-UNTRACKED-DEREF-THROUGH-CAST (USER DECISION 2026-11-24:
"launch the two return-family follow-up cards"). Base `7678fe5f1`, the tip of
RETURN-FAMILY-DEREF-THROUGH-RESOLVED-CAST (rule 74m), approved to land as r68
on main `3ca85c7a6`. [PREDICTION.md](PREDICTION.md) was committed
(`b3b6a6f79`) before the implementation (`45133777b`). This adds gateway rule
[74r](../../../../agents/gateway.md#constraint-74r), which extends rules 74h and
74m, and `QCore.ReturnFamilyUntrackedDerefCast` with its audit.

## 1. Reachability

**It is reachable.** A scratch end-to-end probe at the base (the new test
file plus a probe test, not committed; log in the worktree's
`private/rfu/probe-base.log`) gave, with `k_alloc`/`k_free` declared:

| body | base | tip |
|---|---|---|
| `long w(void) { const long *p = 0; return *(long *)p; }` | **`Safe`** | `Unknown`, `unknown_alias:p` |
| `… return *(p); }` | **`Safe`** | `Unknown`, `unknown_alias:p` |
| `… return *(p + 1); }` | **`Safe`** | `Unknown`, `unknown_alias:p` |
| `… return *p; }` / `(long) *p` (controls) | `Unknown`, `unknown_alias:p` | same |
| `long w(const long *p) { return *(long *)p; }` | `Safe` | same |
| `long *p = k_alloc(8); if (!p) return 0; return *(long *)p;` | `Unsafe`, `leak` | same |
| same with `k_free(p);` before the return | `Unsafe`, `UseAfterRelease` + `unknown_alias:p` | same |
| `… return *(long * const)p; }` | **`Safe`** | **`Safe`** (follow-up) |
| `… return ((const long *)p)[0]; }`, `((const struct s *)p)->n` | **`Safe`** | **`Safe`** (follow-up) |

At the base `scalar_read` made an untracked root a starred base only when
`texts[index - 1] == "*"`. In `* ( long * ) p` the token before `p` is `)`,
so `bases` was empty and the flow check never saw the dereference of the null
`p`. `return *p;` gets `unknown_alias:p`, because there `p` is a base and its
binding may be null.

## 2. Change and direction

* **`legacy_return_family.rs`.** The starred test keeps its old clause and
  adds `dereferenced_through_groups`, a new function. It reads back from the
  root through `(` and through every `)` whose group
  `cast_semantics::type_group_open` reads as a type (rule 12h's
  transparency, the one existing detector) to the first `*`, which counts
  when `operator_at` says it may be unary (rule 74h's union, so a `*` after
  an undecided group counts). Any other token stops it. The root is a base
  when either clause says so. `stars_before` and the may-bind admission are
  not touched, and the module header gains a rule 74r paragraph.
* **Type words.** Walking through `(` would reach the words of a cast's own
  type name: `long` in `* ( long * ) p` is a name the reader sees, and
  `never_bound` answers `true` for it. `cast_type_word` keeps such a word out
  of the new clause. A word counts as a type word when it stands in a
  parenthesised group of names and `*`s with only names before it, and the
  group holds more than one token or its one name is followed by what may
  start an operand. `( p )` before the end, a `)` or a binary-only operator
  is a value group, so `*(long *)(p)` and `*(p)` make `p` a base. A type word
  keeps the old clause exactly (`a_type_word_keeps_the_old_reading`).
* **Direction.** The old clause is kept, so every base the base read is
  still read (`old_starred_kept`, `old_bases_kept`). No refusal changes. A
  base only adds the typed access route's obligation, so a result the tip
  admits the base admitted (`the_fix_only_refuses_more`). Under every
  reading C may take of rule 74h's undecided groups, a root that the reading
  dereferences through the groups is a base
  (`starred_keeps_every_reading_base`, via `through_mono` and `cast_above`).
* **The readings the brief keeps.** `(T) *p`, `( T ) * p` (undecided) and
  `sizeof ( long ) * p` were bases and still are. `*(T *)*q` keeps rule 74m's
  refusal, and `*(T *)q` keeps its refusal, because a may-bind root never
  reaches the new clause. `n * ( long ) p` and `( n ) * ( long ) p` (a `*`
  binary under every reading) make no base (`a_multiplication_is_no_base`).
* **Scope of the new clause.** `*(p + 1)` makes `p` a base too, since the walk
  reaches the `*` through `(`. The access dereferences a pointer computed
  from `p`, so the obligation on `p` is fail-closed. A root is never removed
  from the bases.

## 3. Pins

* **Unit** (`tests/unit/legacy_return_family_untracked_deref_cast.rs`, a new
  module of `legacy_return_family`):
  * `rule_74r_untracked_vectors_hold` reads `untrackedVectors` (18 rows) from
    `proof/QCore/ReturnFamilyUntrackedDerefCast.lean` and checks each
    spelling's bases. `untracked_vectors_hold` and `old_vectors_hold` prove
    the same rows, and their base readings, in Lean.
  * `an_untracked_root_through_a_cast_group_is_a_base`: one, two and three
    nested groups, `unsigned long *`, a parenthesised cast (twice), a
    parenthesised root, arithmetic contexts, a cast of the dereference and
    an undecided `( a ) *`.
  * `the_base_readings_are_kept`: the old bases, the multiplications, a
    may-bind root (rule 74m's refusals, `(T) *q`, `sizeof`), and
    `* ( long * const ) p` held at its current reading (follow-up).
  * Rule 74m's `the_base_readings_are_kept` held `* ( long * ) p` at
    `bases: []` as the registered gap. That line now expects `["p"]`.
* **End to end** (`tests/return_family_untracked_deref_cast.rs`):
  `a_null_untracked_root_through_a_cast_is_a_base` (10 rows, all
  `Unknown`, `unknown_alias:p`) and `the_base_readings_are_kept` (the null
  `*p` and `(long) *p`, an unbound formal through casts `Safe`, a tracked
  root `Unsafe` with `leak`, or with `UseAfterRelease` + `unknown_alias:p`
  after a release, through a cast as without one).

**Checked at the base.** With the implementation in place, the new clause was
disabled (`if true || cast_type_word(…)`), which is exactly the base's
reading because the old clause is kept. The new and changed tests were run,
with logs in the worktree's `private/rfu/base-fixtures/`. The source was then
restored byte for byte from a saved copy (`cmp` clean, then `touch`).

* `unit.log`: `rule_74r_untracked_vectors_hold`,
  `an_untracked_root_through_a_cast_group_is_a_base` and rule 74m's
  amended `the_base_readings_are_kept` fail. This card's
  `the_base_readings_are_kept` passes, and so do the 18 other
  `return_family` unit tests.
* `e2e.log`: `a_null_untracked_root_through_a_cast_is_a_base` fails at its
  first row with `("Safe", [])`. `the_base_readings_are_kept` passes, and
  so do rule 74m's three end-to-end tests. The real base's probe
  (`probe-base.log`) ran the same file before the implementation existed
  and failed at the same row with the same value.

**Pins:** none moved. No contract, lock, submodule or CT pin is touched.

## 4. Lean

`QCore.ReturnFamilyUntrackedDerefCast` (imports `QCore.ReturnFamilyStarCast`
for rule 74h's readings) and `proof/AuditReturnFamilyUntrackedDerefCast.lean`,
with no placeholders and no project axioms (`propext` and `Quot.sound`
only). Each item before the root (a `*` with its token class, a `(`, a type
group, anything else) and whether the root is a type word are the Rust
reader's output (trust boundary 28), bound by `untrackedVectors`. A base's
obligation is abstracted as a predicate the flow check decides.

## 5. Measurement against the prediction

This is the opt-in `current --monkey-linked` route over Fluent Bit 046accf6.
Each run used its own `git archive` tree and release binary
([evidence/runs.md](evidence/runs.md)).

| run | Safe / Pending / Unknown / Unsafe | access rows / alone / leaves |
|---|---|---|
| base `7678fe5f1` | 1,171 / 2 / 3,623 / 8 | 2,118 / 215 / 8,992 |
| **tip `45133777b`** | **1,171 / 2 / 3,623 / 8** | 2,118 / 215 / 8,992 |

No row changes status, and no row changes its leaf multiset. There is no
`Safe` -> `Unknown` move to account for. That matches the prediction, whose
`measure-prediction` block was `none` on every route. The corpus has no
`return` that dereferences an untracked root through a cast group
(PREDICTION.md's `grep`). The base figures equal the ones rule 74m's card
measured at its base and tip.

**Pinned routes.** I attempted the system and slice routes with the tip side
tree's release `quirin-audit` and `quirin-audit-measure`
(`--side base=rev:7678fe5f1 --side tip=rev:45133777b --routes system,slice
--declared docs/reports/return-family-untracked-deref-cast/PREDICTION.md`,
over `crates/quirin-audit-plugin-measure/inputs/live.json`; script
`private/rfu/declared.sh`). The command exited 1 before it measured anything,
with verdict `Unknown`: `plugin-exit: the plugin ended with exit 1:
…/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-production/compile_commands.json:
No such file or directory (os error 2)`. The log is the worktree's
`private/rfu/measure-declared.log`. **The pinned corpus, system and slice
routes were not run.** No move is predicted on them. A move there would be
`Safe` -> `Unknown` at a `return` that dereferences an untracked root through
a cast group or a parenthesis.

## 6. Findings outside scope (registered)

The base probe found two neighbouring admitting gaps that this card's clause
does not reach. They are registered rather than fixed:

* **RETURN-FAMILY-UNTRACKED-MEMBER-THROUGH-CAST.**
  `((const struct s *)p)->n` and `((const long *)p)[0]` through a null
  untracked `p` are `Safe`. The member and subscript bases need `->` or `[`
  directly after the name. The change is outside the starred test, and
  `((T *)ctx)->f` is common in the corpus, so it needs its own measurement.
* **TYPE-GROUP-QUALIFIED-POINTER.** `type_group_open` does not read
  `( long * const )`, so `*(long * const)p` stays `Safe`. Rule 74m's count
  and rule 12h(4)'s transparency stop at the same group. Rule 74m forbids a
  second detector, so the fix belongs in `type_group_open` itself.

## 7. Gate

Scoped card gate: `python3 scripts/gate_scope.py --base 7678fe5f1 --out
private/rfu/gate-1`, run through `private/toolchain/env.sh` (Clang 21, Lean
4.35) at head `45133777b` with a clean tree. It exited 0 with all 8 steps
passed. Later commits change only this report and its evidence. The record
is [gate/gate-1.json](gate/gate-1.json), and the logs are in the worktree's
`private/rfu/gate-1/`. The two measurement runs ran alongside it.

| step | exit | result |
|---|---:|---|
| submodules | 0 | 1 submodule |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| `run_test_binaries.py` | 0 | 686 units, 731 summaries; 10,020 passed, 0 failed, 24 ignored, 148 filtered |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `check_lean_audits.py` | 0 | 498 audits |
| `python3 -m unittest discover -s scripts/tests` | 0 | 807 tests |

No provider file changed, so the provider gate was not selected.

## 8. Ledger, conflicts and follow-ups

[integration-amendments.json](integration-amendments.json) (schema
`quirin-amendments/2`, not applied) retires
RETURN-FAMILY-UNTRACKED-DEREF-THROUGH-CAST and registers the two follow-ups in
§6. `status_edits` is empty, because STATUS states no fact this card changes.
The id it retires is registered by RETURN-FAMILY-DEREF-THROUGH-RESOLVED-CAST's
amendments, which r68 applies. `--check` against this tree is therefore
refused (`no registered follow-up has this id`). Against a `git archive` of
this card's head with those amendments applied (`--root
private/rfu/amend-root`), it exits 0 (`private/rfu/amend-check.log`).

**Files likely to conflict:**

* `crates/qcore-c-gateway/src/legacy_return_family.rs`: the untracked
  starred test in `scalar_read`, two new functions before `scalar_read`,
  the module header and one `mod` line at the end. The sibling card
  RETURN-FAMILY-STAR-OUTSIDE-ROOT-PARENTHESES edits `stars_before` and the
  may-bind starred admission in the same file. This card does not touch
  those.
* `crates/qcore-c-gateway/tests/unit/legacy_return_family_deref_cast.rs`
  (one pin line).
* The generated `docs/agents/gateway.md`, `docs/agents/README.md` and
  `proof/QCore.lean`. Rerun `gen_rule_docs.py` and `gen_lean_roots.py` on a
  conflict.

New files: `docs/agents/rules/74r.md`,
`tests/return_family_untracked_deref_cast.rs`,
`tests/unit/legacy_return_family_untracked_deref_cast.rs`,
`proof/QCore/ReturnFamilyUntrackedDerefCast.lean`,
`proof/AuditReturnFamilyUntrackedDerefCast.lean`.

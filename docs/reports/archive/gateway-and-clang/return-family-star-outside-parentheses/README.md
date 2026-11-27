# RETURN-FAMILY-STAR-OUTSIDE-ROOT-PARENTHESES: a dereference outside a starred root's parentheses counts

Card RETURN-FAMILY-STAR-OUTSIDE-ROOT-PARENTHESES (USER DECISION 2026-11-24:
"launch the two return-family follow-up cards"). The base is `7678fe5f1`, the
tip of RETURN-FAMILY-DEREF-THROUGH-RESOLVED-CAST (rule 74m).
[PREDICTION.md](PREDICTION.md) was committed (`facfeb8c3`) before the
implementation (`12ca1c33c`). The card adds gateway rule
[74s](../../../../agents/gateway.md#constraint-74s), which extends rule 74m, and
`QCore.ReturnFamilyStarOutside` with its audit.

## 1. Reachability

**It is reachable.** The follow-up accepted either a fix or a proof that no
such return reaches family S. The fix is below.

A scratch end-to-end probe at the base and at the tip (not committed; logs in
the worktree's `private/rso/probe-rows-base.log` and
`private/rso/probe-rows-tip.log`) shows:

| body (`k_alloc`/`k_free` declarations + …) | base | tip |
|---|---|---|
| `long w(long **q) { return *(*q + 1); }` | **`Safe`** | `Unknown`, return refusal |
| `long w(long **q) { return *((long *)*q + 1); }` | **`Safe`** | `Unknown`, return refusal |
| `long w(long **q) { return *(1 + *q); }` | **`Safe`** | `Unknown`, return refusal |
| `long w(long **q, long n) { return *(n + *q); }` | **`Safe`** | `Unknown`, return refusal |
| `long w(long **q) { return *(*q - 1); }` | **`Safe`** | `Unknown`, return refusal |
| `long w(long **q) { return *(long *)(*q + 1); }` | **`Safe`** | `Unknown`, return refusal |
| `long w(long **q) { return (*q + 1)[0]; }` | **`Safe`** | `Unknown`, return refusal |
| `struct s { long f; }; long w(struct s **q) { return (*q + 1)->f; }` | **`Safe`** | `Unknown`, return refusal |
| `return **q;`, `return (*q)[0];` (controls) | `Unknown`, return refusal | same |
| `long **q = k_alloc(8); … k_free(q); return *(*q + 1);` | `Unsafe`, `UseAfterRelease` + `unknown_alias:q` | `Unknown`, return refusal |
| `long w(long *q) { return` `*q` / `(*q)` / `((*q))` / `*q + 1` / `(long) *q` / `2 * (*q + 1)` `; }` | `Safe` | same |
| `long w(long **q, long n) { return n[*q]; }` | `Safe` | same (follow-up, §6) |

"Return refusal" is `unsupported:unmodeled call, aggregate or indirect access
in return expression`. At the base, family S's `stars_before` walked back from
a starred root through `(` and `*` and stopped at anything else. It stopped at
a binary operator (`* ( 1 + * q )`) or at a `*` behind more opening
parentheses than close right after the root (`* ( * q + 1 )`).
`ends_one_dereference` read only up to the root's own parentheses, so
`( * q + 1 ) [ 0 ]` passed. Each counted one `*`, family S admitted `q` as one
dereference base, and the outer `*`, `[` or `->`, which dereferences a pointer
computed from the value `*q` reads, carried no obligation.

## 2. Change and direction

* **`legacy_return_family.rs`.** There are two new functions, and
  `stars_before` is not rewritten. `outside_dereference` walks forward from
  the root to every `)` that closes a group enclosing it, groups outside a
  subscript included. `group_dereferenced` says such a group is dereferenced
  when a `[` or `->` follows its `)`, or when the token before its `(` is a
  `*` that rule 74h's `operator_at` reads as unary under either reading
  (`may_be_unary`). That token is read back through any parenthesised group:
  a cast, an undecided group, or a call group the expression already refuses.
  An unbalanced group refuses. The starred-root admission gains
  `&& !outside_dereference(…)`. The module header gains a rule 74s paragraph.
* **Fail-closed reading.** The reader cannot tell whether a dereferenced
  group's value is the pointer computed from the root. With a pointer `p`,
  `*(p + *q)` dereferences `p`'s object. So every dereferenced enclosing
  group refuses, whatever it contains.
* **Direction.** The new admission is rule 74m's admission and the new
  condition, so every admission at the tip was one at the base
  (`the_fix_only_refuses_more`). Under every reading C may take of rule 74h's
  undecided groups, an admitted root has no dereferenced enclosing group, and
  its whole count (rule 74m's `through` plus the dereferenced enclosing
  groups) is exactly one (`admitted_has_no_outside_dereference`,
  `admitted_is_one_dereference`).
* **What is kept.** `*q`, `(*q)`, `((*q))`, `*q + 1`, `(*q + 1)`,
  `n * (*q + 1)` (a binary `*`), `(T) *q`, `(long) (*q)`, `(long) (*q + 1)`
  and `(*q).f` keep their admission. `*(T *)*q` keeps rule 74m's refusal, and
  `*(T *)q` its refusal. Member, subscript and untracked roots are not
  touched.
* **The one movement away from an accusation.** After `k_free(q)`,
  `return *(*q + 1);` was `Unsafe`. It is now the return refusal, which is
  what `return **q;` gets. That is `Unsafe` -> `Unknown` on a test fixture.
  Nothing moves toward `Safe`.

## 3. Pins

* **Unit** (`tests/unit/legacy_return_family_star_outside.rs`, a new module of
  `legacy_return_family`):
  * `rule_74s_outside_vectors_hold` reads `outsideVectors` (20 rows) from
    `proof/QCore/ReturnFamilyStarOutside.lean` and checks each spelling's
    admission;
  * `a_dereferenced_enclosing_group_is_a_second_dereference` (18 spellings,
    including nested groups, casts inside and outside the group, a subscript
    or `->` after it, an undecided `( a ) *` and a group outside a subscript);
  * `one_dereference_and_an_arithmetic_read_are_kept` (the controls, rule
    74m's refusal, `* * q`, and the untracked and scalar readings).
* **End to end** (`tests/return_family_star_outside_parentheses.rs`):
  * `a_star_outside_the_root_parentheses_is_a_second_dereference` pins
    `return *(*q + 1);` and `return *((long *)*q + 1);` as refused, with five
    more spellings and the `**q` control;
  * `a_released_root_under_an_outside_star_is_refused` covers the same after
    `k_free(q)`;
  * `one_dereference_and_an_arithmetic_read_are_unchanged` covers `*q`,
    `(*q)`, `*q + 1` and `(long) *q`, both `Safe` as formals and `Unsafe`
    after a release, plus rule 74m's `*(long *)*q` refusal. It passes at the
    base too.

**Checked at the base.** With the implementation in place, the new condition
was disabled (`&& (true || !outside_dereference(…))`), which gives exactly the
base's admission. The new tests were then run, with logs in the worktree's
`private/rso/base-fixtures/`. The source was restored byte for byte from a
saved copy (`cmp` clean, then `touch`).

* `unit.log` (exit 101): `rule_74s_outside_vectors_hold` and
  `a_dereferenced_enclosing_group_is_a_second_dereference` fail. The kept
  readings test and the 20 other `return_family` unit tests pass.
* `e2e.log` (exit 101): `a_star_outside_the_root_parentheses_is_a_second_dereference`
  fails with `("Safe", [])`, and `a_released_root_under_an_outside_star_is_refused`
  fails with `("Unsafe", ["UseAfterRelease", "unknown_alias:q"])`. The control
  test passes.

**Pins:** none moved. No contract, lock, submodule or CT pin is touched.

## 4. Lean

`QCore.ReturnFamilyStarOutside` imports `QCore.ReturnFamilyDerefCast` for rule
74m's items and admission. Its audit is `proof/AuditReturnFamilyStarOutside.lean`.
There are no placeholders and no project axioms (`check_lean_audits.py`, 498
audits). Each item, enclosing group, the `*` reached back from the group's
`(` and what follows its `)` are the Rust reader's output (trust boundary 28),
bound by `outsideVectors`.

## 5. Measurement against the prediction

This is the opt-in `current --monkey-linked` route over Fluent Bit 046accf6.
Each run used its own `git archive` tree and release binary
([evidence/runs.md](evidence/runs.md)).

| run | Safe / Pending / Unknown / Unsafe | access rows / alone / leaves |
|---|---|---|
| base `7678fe5f1` | 1,171 / 2 / 3,623 / 8 | 2,118 / 215 / 8,992 |
| **tip `12ca1c33c`** | **1,171 / 2 / 3,623 / 8** | 2,118 / 215 / 8,992 |

No row changes its status or its leaf multiset, so there is no Safe→Unknown
move to account for. That matches the prediction, whose `measure-prediction`
block was `none` on every route. The corpus has no such spelling
(PREDICTION.md's `grep`).

**Pinned routes.** I attempted the system and slice routes with the tip side
tree's release `quirin-audit` and `quirin-audit-measure`
(`--side base=rev:7678fe5f1 --side tip=rev:12ca1c33c --routes system,slice
--declared docs/reports/return-family-star-outside-parentheses/PREDICTION.md`,
over `crates/quirin-audit-plugin-measure/inputs/live.json`). The command exited
1 before it measured anything, with verdict `Unknown`: `plugin-exit: the plugin
ended with exit 1: …/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-production/compile_commands.json:
No such file or directory (os error 2)`. The log is the worktree's
`private/rso/measure-declared.log`. **The pinned corpus, system and slice
routes were not run.** No move is predicted on them.

## 6. Findings outside scope (registered)

* **RETURN-FAMILY-SUBSCRIPT-ENCLOSING-STARRED-ROOT.**
  `long w(long **q, long n) { return n[*q]; }` is `Safe` at both the base and
  the tip. C11 6.5.2.1p2 makes it `*(n + *q)`, which rule 74s refuses. Rule
  74s reads only parenthesised enclosing groups, and refusing every bracket
  would cost `a[*q]` with an untracked pointer `a`, where the access is
  `a`'s and `a` is a base. The same probe shows `a[*q]` `Safe` at both.

## 7. Gate

The scoped card gate was `python3 scripts/gate_scope.py --base 7678fe5f1 --out
private/rso/gate-1`, run through `private/toolchain/env.sh` (Clang 21, Lean
4.35) at head `12ca1c33c`. It exited 0, with all 8 steps passed. Later
commits change only this report, its evidence and the amendments. The record
is [gate/gate-1.json](gate/gate-1.json), and the logs are in the worktree's
`private/rso/gate-1/`. The two measurement runs ran alongside it.

| step | exit | result |
|---|---:|---|
| submodules | 0 | 1 submodule |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| `run_test_binaries.py` | 0 | 686 units, 731 summaries; 10,021 passed, 0 failed, 24 ignored, 148 filtered |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `check_lean_audits.py` | 0 | 498 audits |
| `python3 -m unittest discover -s scripts/tests` | 0 | 807 tests |

No provider file changed, so the provider gate was not selected.

## 8. Ledger, conflicts and follow-ups

[integration-amendments.json](integration-amendments.json) (schema
`quirin-amendments/2`, not applied) retires
RETURN-FAMILY-STAR-OUTSIDE-ROOT-PARENTHESES and registers the follow-up in §6.
The id it retires is registered by RETURN-FAMILY-DEREF-THROUGH-RESOLVED-CAST's
amendments, which r68 applies. In this tree those amendments are committed but
not applied, so `apply_amendments.py <file> --check` against the checkout
exits 1 ("no registered follow-up has this id"). Against a scratch root (an
archive of `docs`, `scripts`, `contract`, `proof` and `providers` at this
head) with the r68 amendments applied first (exit 0), `--check` exits 0. The
scripts and logs are in the worktree's `private/rso/amend.sh`,
`amend-prior.log` and `amend-check.log`. `status_edits` is empty, because STATUS states no fact this
card changes.

**Files likely to conflict:**

* `crates/qcore-c-gateway/src/legacy_return_family.rs`: the module header,
  the two new functions after `stars_before`, one line in the starred-root
  admission, and one `mod` line at the end. The sibling card
  RETURN-FAMILY-UNTRACKED-DEREF-THROUGH-CAST edits the untracked-root test
  in the same file. This card does not touch it.
* the generated `docs/agents/gateway.md`, `docs/agents/README.md` and
  `proof/QCore.lean`. On a conflict, rerun `gen_rule_docs.py` and
  `gen_lean_roots.py`.

New files: `docs/agents/rules/74s.md`,
`crates/qcore-c-gateway/tests/return_family_star_outside_parentheses.rs`,
`crates/qcore-c-gateway/tests/unit/legacy_return_family_star_outside.rs`,
`proof/QCore/ReturnFamilyStarOutside.lean`,
`proof/AuditReturnFamilyStarOutside.lean`.

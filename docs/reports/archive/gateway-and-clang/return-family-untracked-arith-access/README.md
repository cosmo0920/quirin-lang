# RETURN-FAMILY-UNTRACKED-ACCESS-THROUGH-ARITHMETIC: an untracked root accessed through pointer arithmetic is a base

Card RETURN-FAMILY-UNTRACKED-ACCESS-THROUGH-ARITHMETIC (USER DECISION
2026-11-24: "launch both cards"). The base is `497c859fc`, the tip of
RETURN-FAMILY-UNTRACKED-MEMBER-THROUGH-CAST (rule 74t, approved as r74), on
rule 74r (r72) and rule 74m (r68), on main `3ca85c7a6`.
[PREDICTION.md](PREDICTION.md) was committed (`090c10039`) before the
implementation (`f2a2f314c`). The card adds gateway rule
[74w](../../../../agents/gateway.md#constraint-74w), which extends rules 74r and
74t, and `QCore.ReturnFamilyUntrackedArithAccess` with its audit.

## 1. Reachability

**The gap is reachable.** A scratch end-to-end probe (not committed; the
file is kept as `private/rfa/zz_probe_rfa.rs`) ran every row below with the
new clause disabled, which is exactly the base's reading because the old
tests are kept (`private/rfa/probe-base.log`), and at the tip
(`private/rfa/probe-tip.log`). The committed end-to-end test fails at the
base reading at its first row with `("Safe", [])`
(`private/rfa/base-fixtures/e2e.log`). The declarations are `k_alloc`/`k_free`
and `struct s { long n; long a[2]; struct s *next; }`; `k` is a `long`
formal.

| body (`return …;`, `p` a null local) | base | tip |
|---|---|---|
| `(p + 1)->n`, `(k + p)->n`, `(p - k)->n` | **`Safe`** | `Unknown`, `unknown_alias:p` |
| `((const struct s *)p + 1)->n`, `((const struct s *)(p + k))->n`, `((const struct s *)(p + sizeof(long)))->n` | **`Safe`** | `Unknown`, `unknown_alias:p` |
| `(p + 1)[0]`, `(1 + p)[k]`, `((p + 1) + k)[0]`, `(p + (k * 2))[0] + 1` | **`Safe`** | `Unknown`, `unknown_alias:p` |
| `*(k + p)`, `*(const long *)(k + p)` (rule 74r's walk meets the `+` first) | **`Safe`** | `Unknown`, `unknown_alias:p` |
| `long k = 1; … (p + k)->n + 1` | **`Safe`** | `Unknown`, `unknown_alias:p` |
| `(p - q)[a]`, `(a + (p - q))[0]` (`q` a pointer, `a` an array) | `Safe` | `Unknown`, return refusal (fail-closed) |
| `p->n + 1`, `*(p + 1)` (controls) | `Unknown`, `unknown_alias:p` | same |
| `(p + 1) != 0`; `return p + 1;` in a pointer-result function; `sizeof((p + 1)->n)` | `Safe` | same |
| `&(p + 1)->n != 0` | `Unknown`, return refusal | same |
| `long w(const struct s *p, long k) { return (p + 1)->n; }` (unbound formal) | `Safe` | same |
| tracked `p` (`k_alloc`), `(p + 1)->n`, `(1 + p)->n + 1` | `Unknown`, return refusal | same |
| `(0 ? p : q)->n`, `1[p]` | **`Safe`** | **`Safe`** (not arithmetic: follow-ups, §7) |

At the base, `scalar_read`'s untracked branch made a root a base through the
direct tests (`->`, `[` or a unary `*` adjacent), rule 74r's
`dereferenced_through_groups` (back through `(` and type groups to a `*`) and
rule 74t's `accessed_through_groups` (back through `(` and type groups, then
forward through `)` to `->` or `[`). A `+` or `-` between the root and the
access stopped all three, except that 74r's walk reads `*(p + 1)` (the `+`
is after the root) but not `*(k + p)`.

## 2. Change and direction

* **`legacy_return_family.rs`.** The untracked branch keeps every old test
  and adds `accessed_through_arithmetic`, a new function with three helpers
  (`ArithmeticAccess`, `integer_operands`, `operand_end`, `enclosing_group`).
  It keeps a span of what it has read, starting at the root, and widens it
  through a type group before it (`cast_semantics::type_group_open`, the one
  detector), a `(` and `)` right around it, and a parenthesised arithmetic
  group: a binary `+`, `-`, `/` or `%` next to the span (never `- >`), the
  other neighbour such an operator or the group's own parenthesis, inside
  the innermost `( … )` around the span (`enclosing_group`; a `[` stops it,
  so a sum inside a subscript is an index). When at least one arithmetic
  group was crossed and `->` or `[` follows, or a `*` that may be unary under
  either of rule 74h's readings precedes (`operator_at`), the root is a base,
  and a `->` root for `->`. `stars_before`, `dereferenced_through_groups`,
  `accessed_through_groups`, `cast_type_word`, `operator_at` and the may-bind
  admission are not touched. The module header gains a rule 74w paragraph,
  and one `mod` line is added at the end.
* **Fail closed.** A group is classified when the span is a term of a sum
  (`+`, `-` or the group's parenthesis on each side) and its other operands
  are numeric literals, declared scalars (`DeclaredType::Scalar`, no pointer
  level), `sizeof` of a resolved type, parentheses and `+ - * / %`. An access
  through any crossed group that is not classified returns `None` from
  `scalar_read`: another pointer name (`(p - q)`, `(a + p)`), a cast
  (`(p + (long) n)`), a member (`(p + a->k)`), `(n % p)`, `(n / p + 1)`. A
  declared-scalar root is an index and is not read, so `(p + i)` with a
  scalar `i` does not refuse through `i`'s own walk.
* **What is not read.** A `*` next to the root is never a product operator:
  `( long * const ) p` would otherwise read `long` and `const` as factors
  (the rule 74r and 74t pins with `( long * const )` caught this during
  development). A product is no pointer, so `(p * 2 + 1)->m` and
  `(n * p)->m`, invalid C for a pointer `p`, keep the base's reading, and a
  pointer operand beside such a product refuses through its own walk. A
  unary operand (`(*p + 1)->m`, `(-p + 1)->m`) is not a term of the sum: the
  walk stops, and the direct starred test still makes `*p`'s root a base.
* **Rule 74t's guard.** Only the matching `)` of a `(` the walk crossed is
  read. `( n + p ) - > m`, which rule 74t's walk correctly does not read
  through, is now read by rule 74w as a classified sum, and `p` is a base.
* **Direction.** Every old base and `->` root is kept (`old_base_kept`,
  `old_arrow_kept`, `names_kept`). The one new refusal is the expression's.
  So a result the tip admits, the base admitted
  (`the_fix_only_refuses_more`).
* **The readings the brief keeps.** Tracked and may-bind roots (refused
  through arithmetic by the may-bind test, as before); an unbound formal
  (`Safe`); `&(p + 1)->n` (family S's `&` refusal, the return refusal end to
  end); `sizeof((p + 1)->n)` (`Safe`); a comparison of a sum and a pointer
  result `return p + 1;` (`Safe`, not family S's) are unchanged end to end.

## 3. Pins

* **Unit** (`tests/unit/legacy_return_family_untracked_arith_access.rs`, a
  new module of `legacy_return_family`):
  * `rule_74w_arith_vectors_hold` reads `arithVectors` (23 rows) from
    `proof/QCore/ReturnFamilyUntrackedArithAccess.lean`. It checks each
    row's old fields against the base's own functions (the direct tests,
    `dereferenced_through_groups`, `accessed_through_groups`) and the new
    fields against `scalar_read` (a refused row is `None`).
    `arith_vectors_hold` proves the rows in Lean.
  * `an_untracked_root_accessed_through_pointer_arithmetic_is_a_base`
    (21 spellings plus the `->` root and scalar-index checks),
    `an_access_through_an_unclassified_sum_refuses` (10 spellings) and
    `the_base_readings_are_kept` (values, comparisons, subscripts, direct and
    cast spellings, the `&` and `sizeof` refusals, may-bind roots, a scalar).
  * **Amended rule 74t pins** (`legacy_return_family_untracked_member_cast.rs`):
    `rule_74t_member_vectors_hold` and `the_base_readings_are_kept` held
    `( n + p ) - > m`, `( ( long * ) p + n ) - > m` and `( n + ( p ) ) - > m`
    at `bases: []`, which is this card's gap. They now expect `["p"]` (and a
    `->` root) as the union; rule 74t's Lean `memberVectors` are unchanged,
    since `accessed` still reads those rows as no access.
* **End to end** (`tests/return_family_untracked_arith_access.rs`):
  `a_null_untracked_root_accessed_through_pointer_arithmetic_is_a_base` (12
  rows, `Unknown`, `unknown_alias:p`),
  `an_access_through_an_unclassified_sum_refuses` (2 rows, the return
  refusal), `the_base_readings_are_kept` (3 null controls, 3 unbound-formal
  rows, the comparison, the pointer result, `&`, `sizeof`, 3 tracked rows).

**Checked at the base reading.** With the implementation in place,
`accessed_through_arithmetic` was made to return `ArithmeticAccess::None` at
entry, exactly the base's reading since the old tests are kept. The new and
changed tests were run (`private/rfa/base-fixtures/unit.log`, `e2e.log`), and
the source was restored byte for byte from a saved copy (`cmp` clean, then
`touch`).

* `unit.log`: `rule_74w_arith_vectors_hold`,
  `an_untracked_root_accessed_through_pointer_arithmetic_is_a_base`,
  `an_access_through_an_unclassified_sum_refuses` and both amended rule 74t
  pins fail (5 failed, 24 passed: this card's `the_base_readings_are_kept`
  and the other 23 `return_family` unit tests).
* `e2e.log`: the null-root test fails at its first row with `("Safe", [])`,
  the unclassified test with `("Safe", [])`; `the_base_readings_are_kept`
  passes.

**Pins:** no contract, lock, submodule or CT pin is touched.

## 4. Lean

`QCore.ReturnFamilyUntrackedArithAccess` and
`proof/AuditReturnFamilyUntrackedArithAccess.lean` have no placeholders and
no project axioms. Each layer (`cast`, `paren`, `sum classified`) and each
context (`arrow`, `element`, `none`) is the Rust reader's output (trust
boundary 28), bound by `arithVectors`. Obligations are predicates the flow
check decides; family S's result is modelled as `oldRead` and `newRead`
over the old reading plus `arith`.

## 5. Measurement against the prediction

This is the opt-in `current --monkey-linked` route over Fluent Bit 046accf6.
Each run used its own `git archive` tree and release binary
([evidence/runs.md](evidence/runs.md)).

| run | Safe / Pending / Unknown / Unsafe | access rows / alone / leaves |
|---|---|---|
| base `497c859fc` | 1,171 / 2 / 3,623 / 8 | 2,118 / 215 / 8,992 |
| **tip `f2a2f314c`** | **1,171 / 2 / 3,623 / 8** | 2,118 / 215 / 8,992 |

**No row changes status, and no row changes its leaf multiset**
(`moved-rows-base-tip.jsonl`: `{"moves": {}}`; `changed-rows-base-tip.txt`
is empty). There is no moved row to account for. That matches the
prediction (`none` on every route): every census hit sits inside a call,
which family S refuses before any base is read. The new refusal (an access
through an unclassified group) also moved nothing, so no precision was lost
on the route.

**Pinned routes.** These were not run. The inputs they need are still absent
on this host, as for rules 74r and 74t:
`private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-production/compile_commands.json`
and `build/cycle19` do not exist. No move is predicted there. A move would be
`Safe` -> `Unknown` at a `return` family S admits that accesses an untracked,
possibly-null root through pointer arithmetic, or a new return refusal for
an access through an unclassified group.

## 6. Gate

The scoped card gate was run as `python3 scripts/gate_scope.py --base
497c859fc --out private/rfa/gate-1`, through `private/toolchain/env.sh`
(Clang 21, Lean 4.35), at head `f2a2f314c` with a clean tree. It exited 0,
with all 8 steps passed. Later commits change only this report and its
evidence. The record is [gate/gate-1.json](gate/gate-1.json), and the logs
are in this worktree's `private/rfa/gate-1/`. Both measurement runs ran
alongside the gate.

| step | exit | result |
|---|---:|---|
| submodules | 0 | 1 submodule |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| `run_test_binaries.py` | 0 | 688 units, 733 summaries; 10,032 passed, 0 failed, 24 ignored, 148 filtered |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `check_lean_audits.py` | 0 | 500 audits |
| `python3 -m unittest discover -s scripts/tests` | 0 | 807 tests |

No provider file changed, so the provider gate was not selected.

## 7. Ledger, conflicts and follow-ups

[integration-amendments.json](integration-amendments.json) is in schema
`quirin-amendments/2` and is not applied. It retires
RETURN-FAMILY-UNTRACKED-ACCESS-THROUGH-ARITHMETIC and registers two
follow-ups found by the probe (§1), both admitting and both outside the
arithmetic shape:

* **RETURN-FAMILY-UNTRACKED-CONDITIONAL-ACCESS.** `(0 ? p : q)->n` through
  null untracked roots is `Safe`: no test reads a root that is an operand of
  `?:` inside the accessed group.
* **RETURN-FAMILY-UNTRACKED-REVERSED-SUBSCRIPT.** `1[p]` through a null
  untracked `p` is `Safe`: the subscript tests read only `p [`.

`status_edits` is empty: STATUS states no fact this card changes.
The id this card retires is registered by rule 74t's amendments (r74),
which sit on r68's and r72's. So `--check` against this tree alone is
refused. I extracted a `git archive` of this card's head
(`private/rfa/amend-root`) and applied r68's, r72's and then r74's
amendments there (all exit 0: `private/rfa/amend-apply-r68.log`,
`amend-apply-r72.log`, `amend-apply-r74.log`). Against that root, `--check`
on this card's file exits 0 (`private/rfa/amend-check.log`).

**Files likely to conflict:**

* `crates/qcore-c-gateway/src/legacy_return_family.rs`: the untracked branch
  of `scalar_read` (the base test line and a `match` before it), new
  functions after `accessed_through_groups`, the module header, and one
  `mod` line at the end. RETURN-FAMILY-STAR-OUTSIDE-ROOT-PARENTHESES (r71)
  adds `outside_dereference` and `group_dereferenced` to the same file;
  RETURN-STRICT-MEMBER-NULL-UNTRACKED-ROOT (same base) works in
  `legacy_c.rs`. TYPE-GROUP-QUALIFIED-POINTER changes
  `cast_semantics::type_group_open`, which this card calls but does not
  edit; if it starts reading `( long * const )` as a type group, the walk
  absorbs that cast too (only more bases).
* `crates/qcore-c-gateway/tests/unit/legacy_return_family_untracked_member_cast.rs`
  (two amended pins).
* The generated `docs/agents/gateway.md`, `docs/agents/README.md` and
  `proof/QCore.lean`. Rerun `gen_rule_docs.py` and `gen_lean_roots.py` on a
  conflict.

New files: `docs/agents/rules/74w.md`,
`tests/return_family_untracked_arith_access.rs`,
`tests/unit/legacy_return_family_untracked_arith_access.rs`,
`proof/QCore/ReturnFamilyUntrackedArithAccess.lean`,
`proof/AuditReturnFamilyUntrackedArithAccess.lean`, and this report with its
scripts and evidence.

## 8. Increment paragraph

The paragraph `apply_amendments.py` prepends to the current increment record
is the `increment_paragraph` of
[integration-amendments.json](integration-amendments.json):

> **RETURN-FAMILY-UNTRACKED-ACCESS-THROUGH-ARITHMETIC** (USER DECISION 2026-11-24: "launch both cards"; gateway rule 74w, extending rules 74r and 74t; `QCore.ReturnFamilyUntrackedArithAccess`; [report](README.md)). Family S of `legacy_return_family` stopped every base test at a `+` or `-` between an untracked root and its access. So `long w(long k) { const struct s *p = 0; return (p + 1)->n; }`, `(k + p)->n`, `((const struct s *)p + 1)->n`, `(p + 1)[0]` and `*(k + p)` were `Safe`, while `return p->n + 1;` and `*(p + 1)` are `Unknown` (`unknown_alias:p`). The old tests are kept, and `accessed_through_arithmetic` adds one. It widens a span outward from the root through type groups (`cast_semantics::type_group_open`), parentheses and parenthesised arithmetic groups, reading only the matching `)` of a `(` it crossed. When it crossed an arithmetic group and `->`, `[` or a `*` that may be unary accesses what it reached, the root is a base, and a `->` root for `->`. An access through a group whose other operands are not literals, declared scalars or `sizeof` of a resolved type (`(p - q)[a]`, `(a + p)[0]`) refuses the expression. A `*` is never read as a product, so a cast's type words are never roots. Bases only grow and the one new refusal is the expression's, so nothing admits more than at the base. Tracked and may-bind roots, an unbound formal, `&(p + 1)->n`, `sizeof` and `return p + 1;` read as before. Pinned at unit level (`arithVectors`, 23 rows, old fields checked against the base's functions) and end to end (12 null rows, 2 refusals, controls); rule 74t's two pins holding `( n + p ) - > m` at `bases: []` now expect the union. The change pins fail with the new reading disabled. Opt-in `current --monkey-linked` over Fluent Bit 046accf6: no row moves (1,171 / 2 / 3,623 / 8 at both), as predicted; every census hit sits inside a call family S refuses. The pinned routes need inputs this host lacks and were not run. Registered: RETURN-FAMILY-UNTRACKED-CONDITIONAL-ACCESS and RETURN-FAMILY-UNTRACKED-REVERSED-SUBSCRIPT.

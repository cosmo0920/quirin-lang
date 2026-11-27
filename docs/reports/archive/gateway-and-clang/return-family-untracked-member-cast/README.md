# RETURN-FAMILY-UNTRACKED-MEMBER-THROUGH-CAST: an untracked member or subscript root through a cast group is a base

Card RETURN-FAMILY-UNTRACKED-MEMBER-THROUGH-CAST (USER DECISION 2026-11-24:
"launch the RETURN-FAMILY-UNTRACKED-MEMBER-THROUGH-CAST card"). The base is
`7bfa0414b`, the tip of RETURN-FAMILY-UNTRACKED-DEREF-THROUGH-CAST (rule 74r,
approved as r72). That card sits on rule 74m's `7678fe5f1` (r68), which sits
on main `3ca85c7a6`. [PREDICTION.md](PREDICTION.md) was committed
(`41a32f2e9`) before the implementation (`c3228a355`). The card adds gateway
rule [74t](../../../../agents/gateway.md#constraint-74t), which extends rule 74r,
and `QCore.ReturnFamilyUntrackedMemberCast` with its audit.

## 1. Reachability

**The gap is reachable.** The card's end-to-end test file
(`tests/return_family_untracked_member_cast.rs`) ran at the real base before
the implementation existed (`private/rmc/reach/probe-base.log`). Its first
row failed with `("Safe", [])`. Scratch probes, which are not committed,
gave the rest (`private/rmc/reach/probe1.log` at the base,
`probe2-tip.log`, `probe3-tip.log` and `probe4-tip.log` at the tip). The
declarations are `k_alloc`/`k_free` and
`struct s { long n; long a[2]; struct s *next; }`.

| body (`long w(void) { … }`) | base | tip |
|---|---|---|
| `const void *p = 0; return ((const struct s *)p)->n;` | **`Safe`** | `Unknown`, `unknown_alias:p` |
| `const void *p = 0; return ((const long *)p)[0];` | **`Safe`** | `Unknown`, `unknown_alias:p` |
| `… ((const struct s *)p)->n + 1`, `(int) ((const struct s *)p)->n` | **`Safe`** | `Unknown`, `unknown_alias:p` |
| `const struct s *p = 0; return (p)->n;` / `const long *p = 0; return (p)[0];` | **`Safe`** | `Unknown`, `unknown_alias:p` |
| `const struct s *p = 0; return p->n + 1;` / `const long *p = 0; return p[0];` (controls) | `Unknown`, `unknown_alias:p` | same |
| `long w(const void *p) { return ((const struct s *)p)->n; }` | `Safe` | same |
| tracked `p` (`k_alloc`), `((struct s *)p)->n` | `Unsafe`, `leak` | same |
| tracked `p` released, `((struct s *)p)->n` | `Unsafe`, `UseAfterRelease` + `unknown_alias:p` | same |
| `const struct s *p = 0; return p->n;` | **`Safe`** | **`Safe`** (not family S, follow-up) |
| `(p + 1)->n`, `((const struct s *)p + 1)->n`, `(p + 1)[0]` | **`Safe`** | **`Safe`** (follow-up) |

At the base, `scalar_read` made an untracked root a member or subscript base
only when `arrow_at(texts, index + 1)` or `next == Some("[")` held. In
`( ( T * ) p ) - > n` the token after `p` is `)`, so `bases` was empty and
the flow check never saw the access through the null `p`.

**`return p->n;` alone is a different gap.** The bare `return p->n;` is
`Safe` too, but family S never sees it. `return_statement`
(`legacy_c.rs`) admits it through the `supported_expression` routes that run
before family S, the strict `Returned` rule or its pointer-leaf widening.
Meanwhile `return p->n + 1;` (family S) and `long x = p->n; return x;` are
`Unknown`. This card does not touch those routes. The case is registered
(§7). The cast spelling `((T *)p)->n` alone does reach family S, as the first
row shows.

## 2. Change and direction

* **`legacy_return_family.rs`.** The untracked branch of `scalar_read` keeps
  its direct member and subscript tests. It adds `accessed_through_groups`, a
  new function. The function reads back from the root through `(` and through
  every `)` whose group `cast_semantics::type_group_open` reads as a type
  (rule 12h's transparency, the one existing detector), counting the
  opening parentheses it crosses. It then reads forward through at least one
  and at most that many `)`. `Some(true)` means `->` follows and
  `Some(false)` means `[` follows. The root is a base when either test says
  so, and it is pushed to `arrow_roots` (its C02-P-ALIAS reading) when
  either says `->`, exactly as `p->f` is. `stars_before`,
  `dereferenced_through_groups`, `cast_type_word`, `operator_at` and the
  may-bind admission are not touched. The module header gains a rule 74t
  paragraph, and one `mod` line is added at the end.
* **The brief's reuse items.** `type_group_open` is the only detector.
  `operator_at` (rule 74h, `Undecided` included) has nothing to decide here.
  `->` and `[` are postfix operators with no unary reading, so no operator
  stands between the walk and the root. A group rule 74h would call
  undecided is read by `type_group_open`, which asks for no resolution, as a
  cast. The operand reading of that group stops the walk, and the cast
  reading accesses every root the operand reading accesses
  (`every_group_reading_keeps_the_base`, via `accessed_mono`).
  `cast_type_word` is not needed. A cast's type word is followed, inside
  its group, by another word, a `*`, or the `)` before an operand, never by
  `)` and then `->` or `[` (`a_type_word_is_no_root`). The unit vectors check
  that `long`, `char` and `T`, untracked names in the oracle, never become
  bases.
* **Bound.** Closing parentheses beyond the ones the walk opened are not
  read, so an operand before the root (`( n + p ) - > m`,
  `( n + ( p ) ) - > m`) is no access of it
  (`an_operand_before_the_root_is_no_access`). `f ( p ) - > m` reads as an
  access of `p`, one more base in an expression that family S refuses as a
  call.
* **Direction.** The direct tests are kept, so every base and `->` root the
  base read is still read (`old_member_kept`, `old_arrow_kept`,
  `old_bases_kept`). No refusal changes. A base or a `->` root only adds an
  obligation, so a result the tip admits, the base admitted
  (`the_fix_only_refuses_more`, which also holds rule 74r's starred bases
  fixed).
* **The readings the brief keeps.**
  * A tracked root through a cast is a may-bind root. It is refused by
    family S as before, and the typed return route decides it. The member
    cases stay `Unsafe` (`leak`, or `UseAfterRelease` +
    `unknown_alias:p`). `((long *)p)[0]` keeps the return refusal.
  * `&((T *)p)->f` hits family S's `&` refusal. End to end it keeps the
    return refusal (`unsupported:unmodeled call, aggregate or indirect
    access in return expression`).
  * `sizeof(((T *)p)->f)` is not a type operator family S admits, and it
    stays `Safe` end to end, as at the base. Its operand is not evaluated.
  * `( ( long * const ) p ) - > n` keeps the base's reading
    (TYPE-GROUP-QUALIFIED-POINTER, still open).

## 3. Pins

* **Unit** (`tests/unit/legacy_return_family_untracked_member_cast.rs`, a new
  module of `legacy_return_family`):
  * `rule_74t_member_vectors_hold` reads `memberVectors` (17 rows) from
    `proof/QCore/ReturnFamilyUntrackedMemberCast.lean`. It checks each
    spelling's bases and `->` roots. `member_vectors_hold` and
    `old_vectors_hold` prove the same rows, and their base readings, in
    Lean.
  * `an_untracked_member_or_subscript_root_through_a_cast_group_is_a_base`
    covers one, two and three nested groups, a parenthesised root (once and
    twice), `. m` after the member, a sum of two subscripts, a conditional,
    a cast of the member, `* ( ( long * ) p ) - > n`, and no type word as a
    root.
  * `the_base_readings_are_kept` covers the direct spellings, a converted
    operand, a comparison of two converted values, arithmetic inside the
    group, an operand before a parenthesised root, the `&` and `sizeof`
    refusals, may-bind roots through a cast or a parenthesis (refused) and
    directly (admitted), and `( long * const )`.
  * The W45 pin in `tests/unit/legacy_return_family.rs`
    (`a_directly_dereferenced_untracked_root_is_a_base`) held
    `( ( const char * ) p ) - > u16` at `bases: []`, which is the gap this
    card fixes. That line now expects `["p"]`.
* **End to end** (`tests/return_family_untracked_member_cast.rs`):
  * `a_null_untracked_member_or_subscript_root_through_a_cast_is_a_base` has
    11 rows, all `Unknown`, `unknown_alias:p`.
  * `the_base_readings_are_kept` covers the null `p->n + 1` and
    `p->a[0] + 1` (`Unknown`, `unknown_alias:p`), an unbound formal through
    three cast spellings (`Safe`), the address (the return refusal), the
    `sizeof` (`Safe`), and a tracked root through five member spellings and
    two subscript spellings, live and released, with the verdicts in §2.

**Checked at the base.** With the implementation in place, the new clause
was disabled (`accessed_through_groups(…).filter(|_| false)`). That is
exactly the base's reading, because the direct tests are kept. The new and
changed tests were then run, with logs in `private/rmc/base-fixtures/`.
Afterwards the source was restored byte for byte from a saved copy (`cmp`
clean, then `touch`).

* `unit.log`: `rule_74t_member_vectors_hold`,
  `an_untracked_member_or_subscript_root_through_a_cast_group_is_a_base` and
  the amended W45 pin fail. This card's `the_base_readings_are_kept` passes,
  and so do the other 21 `return_family` unit tests.
* `e2e.log`: `a_null_untracked_member_or_subscript_root_through_a_cast_is_a_base`
  fails at its first row with `("Safe", [])`. `the_base_readings_are_kept`
  passes. The real base's probe (`reach/probe-base.log`) failed at the same
  row with the same value.

**Pins:** none moved. No contract, lock, submodule or CT pin is touched.

## 4. Lean

`QCore.ReturnFamilyUntrackedMemberCast` and
`proof/AuditReturnFamilyUntrackedMemberCast.lean` have no placeholders and
no project axioms. The audit gate ran 499 audits. Each item before and after
the root (a `(`, a type group or anything else back; a `)`, `->`, `[` or
anything else forward) is the Rust reader's output (trust boundary 28),
bound by `memberVectors`. Base and `->` obligations are abstracted as
predicates that the flow check decides.

## 5. Measurement against the prediction

This is the opt-in `current --monkey-linked` route over Fluent Bit 046accf6.
Each run used its own `git archive` tree and release binary
([evidence/runs.md](evidence/runs.md)).

| run | Safe / Pending / Unknown / Unsafe | access rows / alone / leaves |
|---|---|---|
| base `7bfa0414b` | 1,171 / 2 / 3,623 / 8 | 2,118 / 215 / 8,992 |
| **tip `c3228a355`** | **1,171 / 2 / 3,623 / 8** | 2,118 / 215 / 8,992 |

**No row changes status, and no row changes its leaf multiset**
(`moved-rows-base-tip.jsonl`: `{"moves": {}}`; `changed-rows-base-tip.txt`
is empty). There is no Safe -> Unknown move to account for and no
Unknown -> Safe move. That matches the prediction: its block was `none` on
every route, and the only local-root candidate was at most one row.

Why the brief's expectation did not happen:

* **The census overcounted the route.** The census read every unit in the
  two compile databases, but the route reports only 181 units: librdkafka
  88, cmetrics 31, Monkey 30, ctraces 17, chunkio 13, and two `src/` files.
  The one local-root candidate, `lj_debug_line` (luajit), is not on the
  route.
* **The on-route census hits are formals.** These are librdkafka's
  `LZ4_read16/32`, `LZ4_read_ARCH` and `rdxxhash.c`'s readers. An unbound
  formal discharges the new base obligation. `LZ4_read16` is `Safe` at both
  sides.
* **Callback-context spellings are refused for another reason.** At a
  `return`, `((T *)ctx)->f` in the corpus sits inside a call
  (`ntohs (…)`, `strcmp (…)`), which family S refuses. Otherwise it is a
  pointer-valued result, which is not family S's.

The tip makes the reading sound, and the corpus has no row that depended on
the gap. No row had to be accounted for, and none is a precision loss.

**Pinned routes.** These were not run. The inputs they need are still absent
on this host. Rule 74r's attempt was refused for
`private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-production/compile_commands.json`,
which still does not exist, and `build/cycle19` does not exist either. No
move is predicted there. A move would be `Safe` -> `Unknown` at a `return`
that family S admits and that accesses an untracked, possibly-null root
through a cast group or a parenthesis.

## 6. Gate

The scoped card gate was run as `python3 scripts/gate_scope.py --base
7bfa0414b --out private/rmc/gate-1`, through `private/toolchain/env.sh`
(Clang 21, Lean 4.35), at head `c3228a355` with a clean tree. It exited 0,
with all 8 steps passed. Later commits change only this report and its
evidence. The record is [gate/gate-1.json](gate/gate-1.json), and the logs
are in this worktree's `private/rmc/gate-1/`. Both measurement runs ran
alongside the gate.

| step | exit | result |
|---|---:|---|
| submodules | 0 | 1 submodule |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| `run_test_binaries.py` | 0 | 687 units, 732 summaries; 10,025 passed, 0 failed, 24 ignored, 148 filtered |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `check_lean_audits.py` | 0 | 499 audits |
| `python3 -m unittest discover -s scripts/tests` | 0 | 807 tests |

No provider file changed, so the provider gate was not selected.

## 7. Ledger, conflicts and follow-ups

[integration-amendments.json](integration-amendments.json) is in schema
`quirin-amendments/2` and is not applied. It retires
RETURN-FAMILY-UNTRACKED-MEMBER-THROUGH-CAST and registers two follow-ups:

* **RETURN-STRICT-MEMBER-NULL-UNTRACKED-ROOT.** `return p->n;` through a
  null untracked `p` is `Safe` through the routes that run before family S
  (§1).
* **RETURN-FAMILY-UNTRACKED-ACCESS-THROUGH-ARITHMETIC.** `(p + 1)->n`,
  `((T *)p + 1)->n` and `(p + 1)[0]` through a null untracked `p` stay
  `Safe`. Rule 74r already made `*(p + 1)` a base, but the member and
  subscript tests stop at the `+`.

`status_edits` is empty, because STATUS states no fact this card changes.
The id this card retires is registered by rule 74r's amendments, which r72
applies. So `--check` against this tree alone is refused. I extracted a
`git archive` of this card's head (`private/rmc/amend-root`) and applied
r68's and then r72's amendments there (both exit 0:
`private/rmc/amend-apply-r68.log`, `amend-apply-r72.log`). Against that
root, `--check` exits 0 (`private/rmc/amend-check.log`).

**Files likely to conflict:**

* `crates/qcore-c-gateway/src/legacy_return_family.rs`: the untracked
  branch of `scalar_read` (the base test line), a new function before
  `scalar_read`, the module header, and one `mod` line at the end. The
  sibling RETURN-FAMILY-STAR-OUTSIDE-ROOT-PARENTHESES (rule 74s, r71) adds
  `outside_dereference` and `group_dereferenced` and changes the may-bind
  admission in the same file. This card touches neither.
* `crates/qcore-c-gateway/tests/unit/legacy_return_family.rs` (one pin).
* The generated `docs/agents/gateway.md`, `docs/agents/README.md` and
  `proof/QCore.lean`. Rerun `gen_rule_docs.py` and `gen_lean_roots.py` on a
  conflict.

New files: `docs/agents/rules/74t.md`,
`tests/return_family_untracked_member_cast.rs`,
`tests/unit/legacy_return_family_untracked_member_cast.rs`,
`proof/QCore/ReturnFamilyUntrackedMemberCast.lean`,
`proof/AuditReturnFamilyUntrackedMemberCast.lean`, and this report with its
scripts and evidence.

# RETURN-FAMILY-STAR-AFTER-UNRESOLVED-CAST: family S keeps both readings of an undecided cast

Card RETURN-FAMILY-STAR-AFTER-UNRESOLVED-CAST (USER DECISION 2026-11-23:
"RETURN-FAMILY-STAR-AFTER-UNRESOLVED-CAST -> Let's get started."). Base
`5cfca8170` (`integrate/r62`). [PREDICTION.md](PREDICTION.md) was committed
(`6c729894e`) before the implementation (`9de28d1ec`). Gateway rule
[74h](../../../../agents/gateway.md#constraint-74h), extending rule 74g;
`QCore.ReturnFamilyStarCast` with its audit.

## 1. Reachability

**An unresolved type name reaches family S.** The follow-up allowed either a
proof that it cannot or a fix. It can, and the fix is below.

`is_cast_group` resolves a group through
`TypedAccessModel::never_variably_modified`. That function answers `false` for
a group with a rejecting qualifier (`volatile`, `_Atomic`), for a pointer,
array, function or disagreeing typedef, and for any name the reader does not
resolve. None of these is refused earlier in the `return` route: the lexical
access gate refuses the expression, `return_expression_instruction` tries its
older routes, and family S is asked last but one (before the typed return
route, rule 10k).

The end-to-end pin `an_unresolved_cast_before_a_dereference_keeps_the_base`
(`tests/return_family_star_cast.rs`) shows the gap at the base:

| body (`DECLARATIONS` + …) | base | tip |
|---|---|---|
| `long w(void) { const long *p = 0; return (volatile long) *p; }` | **`Safe`** | `Unknown`, `unknown_alias:p` |
| `long w(void) { const long *p = 0; return (k_lp) *p != 0; }` (`typedef long *k_lp;`) | **`Safe`** (by reading; the run stops at the first row) | `Unknown`, `unknown_alias:p` |
| `… return (long) *p; }` (resolved cast) | `Unknown`, `unknown_alias:p` | same |
| `… return *p; }` | `Unknown`, `unknown_alias:p` | same |

At the base family S read the `*` after `(volatile long)` as a multiplication,
so the untracked `p` was no base of the read node. The flow check never saw
the dereference of the null `p`, and the function was certified. A scratch
probe (not committed) printed family S's node at the base: `bases: []` for
`(volatile long) *p`, `bases: ["p"]` for `(long) *p`.

## 2. Direction assessment

Family S has three tests on the token before a single `*` or `&`. An
undecided group has two readings: an operand (the operator is binary) and a
cast (the operator is unary). Neither reading's result contains the other's
for every test, so the tip keeps whatever either reading raises:

| test | operand reading | cast reading | tip (undecided) | direction |
|---|---|---|---|---|
| `&` refusal | admits `(a) & n` | refuses (an address) | refuses | refuses more |
| starred base of an untracked `p` | no base | base `p` | base `p` (union) | adds a base, so only adds obligations |
| `stars_before` for a may-bind `q`, starred root | `(T) *q`: 0 `*`, refuses | 1 `*`, admits a base | refuses unless both count exactly 1 | refuses more |
| `stars_before`, member or subscript root | `(T) * q->f`: 0, admits | 1, refuses | refuses unless the cast reading counts 0 | refuses more |

* **The starred base.** A base adds the typed route's obligation in transfer.
  For an untracked name that is only ever a refusal (`unknown_alias` on a
  null or unknown binding), never an accusation, since this body cannot
  release it. Taking the union is the fail-closed answer, and the brief's
  admitting-direction gap is closed by it.
* **`stars_before`.** The follow-up warned that the shared detector alone
  would turn `stars_before`'s refusal of `(T) *q` into an admitted
  dereference base, an Unknown-to-admission move. The tip does not take that
  reading alone. It counts the `*`s under both readings and admits a starred
  root only when both count exactly one. The count is monotone in the
  reading (`count_mono`): the operand reading gives the least count and the
  cast reading the greatest, so the two bound every mixed reading of several
  undecided groups. A member or subscript root needs zero, so the cast
  reading alone decides it. The refused `(T) *q` falls through to the typed
  return route, which decides it from Clang's AST, exactly as at the base.
* **Resolved knowledge is kept.** A group `is_cast_group` resolves is a cast,
  as before. The shared detector is asked only about every other `)`. A group
  it rules out (an argument list, a value group, a single declared object) is
  an operand, as before. Only the remaining groups change.

The objects the detector reads are rule 74f's (`publication::value_names`:
formals, the body's declarations and the unit's file-scope objects, less every
name a `typedef` spells). They are computed on first use from the unit's token
stream, at most once per `return`. With no body located the set is empty, so
every `( name )` is undecided, which keeps both readings' refusals.

No verdict improves. Every test refuses wherever the base refused, and every
base the base read the tip still reads (`the_fix_only_refuses_more_starred`,
`the_fix_only_refuses_more_member`, `address_old_refusal_kept`,
`old_starred_kept`).

## 3. Changes

* **`legacy_return_family.rs`.** `unary_position` is replaced by
  `operator_at`, which returns `Unary`, `Binary` or `Undecided`. Its `)` arm
  keeps `is_cast_group` and asks `may_close_a_cast` otherwise. The `&` refusal
  and the starred base use `may_be_unary`. `stars_before` takes a `Reading`.
  The member and subscript roots count under `Reading::Cast`, and a starred
  root needs exactly one under both readings. `NameFacts` gains `objects()`,
  and `ParserNames` computes it lazily (`OnceCell`).
* **`ampersand_runs.rs`.** The detector's documentation names family S, and
  `READERS` gains `("legacy_return_family.rs", 1, "fail-closed+cast")`. The
  detector itself is unchanged.
* **Rule 74h** (new); rule 74g's census sentence now points to it.
  `QCore.ReturnFamilyStarCast` and `AuditReturnFamilyStarCast`, with no
  placeholders and no project axioms. The generated `gateway.md`, router and
  `proof/QCore.lean` were regenerated.

## 4. Tests and pins

* **Unit** (`tests/unit/legacy_return_family_star_cast.rs`, a new module of
  `legacy_return_family`):
  * `rule_74h_family_vectors_hold` reads `starredVectors`, `addressVectors`
    and `rootVectors` from the Lean source and checks every spelling
    (`family_vectors_hold` proves the same classes in Lean);
  * `an_unresolved_cast_keeps_both_readings`: `( T ) * p` makes `p` a base;
    `( T ) * q`, `( T ) * * q`, `( T ) * q - > len`, `( T ) * q [ 0 ]` and
    `( T ) & n` refuse;
  * `a_resolved_cast_is_unchanged`: `( long ) * p`, `( long ) * q`,
    `( int ) ( long ) * q` admit with their base, and `( long ) * * q` and
    `( long ) & n` refuse;
  * `an_operand_group_before_a_star_is_binary`: `f ( x ) * n` (a call,
    refused by family S whatever follows), `( a ) * p` and `( a ) * n` with
    `a` declared, `( a ) * * q`, `( a ) * q - > len`, `( n + 1 ) * p`,
    `( a ) & n`;
  * `without_objects_a_single_name_is_undecided`.
  * The existing oracle in `tests/unit/legacy_return_family.rs` gains an
    empty `objects()`. All its tests pass unchanged.
* **End to end** (`tests/return_family_star_cast.rs`):
  * `an_unresolved_cast_before_a_dereference_keeps_the_base` (§1, with the
    opaque-field-read control that stays `Safe`);
  * `a_may_bind_root_after_an_unresolved_cast_keeps_the_refusal`:
    `(volatile long) *q`, `(long) *q` and `(K_READ) * *q` after `k_free(q)`
    stay `Unsafe` (`UseAfterRelease`, `unknown_alias:q`), and the released
    formal stays `Unknown`;
  * `an_operand_group_before_a_star_stays_binary`: `(a) * b` and `(a) & b`
    with `a` a formal are `Safe`, and `z_unknown(1) * n` keeps the return
    refusal.

**Checked at the base.** With the implementation in place, `operator_at`'s
undecided arm was set to `Binary`, which is exactly the base's reading, and
the new tests were run. Logs are in the worktree's
`private/rfs/base-fixtures/`. The source was then restored byte for byte from
a saved copy (`cmp` clean).

* `unit.log`: 3 of the 5 new unit tests fail.
  `an_unresolved_cast_keeps_both_readings` and `rule_74h_family_vectors_hold`
  fail because `( T ) * p` gave `Some([])`, and
  `without_objects_a_single_name_is_undecided` fails on `( a ) * p`. The two
  control tests pass, and so do all 11 existing tests.
* `e2e.log`: `an_unresolved_cast_before_a_dereference_keeps_the_base`
  **fails** with `("Safe", [])` against `("Unknown", ["unknown_alias:p"])`.
  The may-bind and binary controls pass at the base too: they pin that
  nothing else moves.

**Pins:** none moved.

## 5. Measurement against the prediction

Opt-in `current --monkey-linked` over Fluent Bit 046accf6. Each run used its
own `git archive` tree and release binary ([evidence/runs.md](evidence/runs.md)).

| run | Safe / Pending / Unknown / Unsafe | access rows / alone / leaves |
|---|---|---|
| base `5cfca8170` | 1,172 / 2 / 3,622 / 8 | 2,118 / 215 / 8,992 |
| **tip `9de28d1ec`** | **1,172 / 2 / 3,622 / 8** | 2,118 / 215 / 8,992 |

No row moves status, and no row changes its leaf multiset. This matches the
prediction, whose `measure-prediction` block was `none` on every route. The
corpus's `return` expressions with `) *` or `) &` are calls, address
returns into pointer results (not family S) and arithmetic groups, which the
detector reads as operands. No measured `return` reaches family S with an
undecided group.

**Pinned routes.** I attempted the system and slice routes:

```sh
target/release/quirin-audit --plugin-path target/release --output private/rfs/measure-declared \
  --timeout-ms 36000000 measure --inputs crates/quirin-audit-plugin-measure/inputs/live.json \
  --inputs-base /home/cosmo/Documents/GitHub/quirin-lang \
  --side base=rev:5cfca8170 --side tip=rev:9de28d1ec --routes system,slice \
  --declared docs/reports/return-family-star-cast/PREDICTION.md
```

It exited 1 before measuring anything, verdict `Unknown`: `plugin-exit: the
plugin ended with exit 1:
/home/cosmo/Documents/GitHub/quirin-lang/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-production/compile_commands.json:
No such file or directory (os error 2)`. The host has no `build/cycle19` or
the prepared inputs those routes read, so **the pinned corpus, system and
slice routes were not run**. No move is predicted. A move on them would be
`Safe` -> `Unknown` at a `return` with an unresolved parenthesised name
before `*` or `&`, and should be shown with its group in the source.

## 6. Gate

Scoped card gate: `python3 scripts/gate_scope.py --base 5cfca8170 --out
private/rfs/gate-1`, run through `private/toolchain/env.sh` (Clang 21, Lean
4.35) at head `9de28d1ec`, with exit 0 and all 8 steps passed. Later commits
change only this report, its evidence, rule 74g's pointer sentence and the
regenerated `gateway.md`. Record: [gate/gate-1.json](gate/gate-1.json); logs
are in the worktree's `private/rfs/gate-1/`.

| step | exit | result |
|---|---:|---|
| submodules | 0 | 1 submodule |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| `run_test_binaries.py` | 0 | 681 units, 726 summaries; 9,989 passed, 0 failed, 22 ignored, 148 filtered |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `check_lean_audits.py` | 0 | 493 audits |
| `python3 -m unittest discover -s scripts/tests` | 0 | 807 tests |

No provider file changed, so the provider gate was not selected.

## 7. Ledger and follow-ups

**RETURN-FAMILY-STAR-AFTER-UNRESOLVED-CAST is retired**
([integration-amendments.json](integration-amendments.json)):

* Its DISCHARGE asked for the `)` arm to be decided with the shared detector,
  or for proof that an unresolved type name cannot reach family S. It can
  (§1), and the arm now asks `may_close_a_cast`. No second detector was
  added.
* It asked for both directions to be assessed. They are (§2), and neither
  test admits more than at the base.
* It asked for `return (T) *p;` with `T` unresolved to be pinned. It is, for
  `p` untracked and may-bind (§4).
* It asked for the system and slice routes to be measured. They could not run
  on this host (§5). The opt-in corpus route shows no move.

**Registered: RETURN-FAMILY-DEREF-THROUGH-RESOLVED-CAST.** A scratch unit
probe at the tip (not committed; the base reads resolved casts the same way)
showed that family S admits `* ( long * ) * q` with `q` may-bind as one
dereference (`bases: ["q"]`). `stars_before` with a finite `closing` stops at
the resolved cast's `)`, so the outer `*`, which dereferences the converted
value `*q`, is not counted. That is the admitting direction, and it is about a
resolved cast, so it is outside this card's undecided reading. The same
reading counts `sizeof ( long ) * q` as one dereference, but a conforming body
cannot spell that with a pointer `q` (C11 6.5.5p2), and a declared scalar never
reaches the may-bind test. End-to-end reachability was not tested.

**Files likely to conflict:**

* `crates/qcore-c-gateway/src/legacy_return_family.rs` and
  `crates/qcore-c-gateway/src/ampersand_runs.rs` (one `READERS` row and the
  detector's documentation);
* `crates/qcore-c-gateway/tests/unit/legacy_return_family.rs` (the oracle's
  new `objects()`);
* `docs/agents/rules/74g.md` (one sentence) and `docs/agents/rules/74h.md`
  (new);
* the generated files `docs/agents/gateway.md`, `docs/agents/README.md` and
  `proof/QCore.lean`. Rerun their generators on a conflict.

New files: `tests/return_family_star_cast.rs`,
`tests/unit/legacy_return_family_star_cast.rs`,
`proof/QCore/ReturnFamilyStarCast.lean`, `proof/AuditReturnFamilyStarCast.lean`.

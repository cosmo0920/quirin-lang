# W45-PARENTHESISED-CALLEE: a callee that is not spelled `NAME (`

Card W45-PARENTHESISED-CALLEE, 2026-09-26. A soundness fix in the legacy-C
gateway; gateway constraint 10c.

**Base.** The brief named local `main` `2e6ae8f9`; `main` moved to `1db9a569`
(W45-CONTRACT-DEFERRED-REVIEW integrated) before any gateway code changed, and
the worktree was reset onto it. BEFORE runs at both bases reproduce the
baseline ([§3.1](#31-before-and-identity)).

**Commits.** Prediction and failing pins `a549afb3`
([PREDICTION.md](PREDICTION.md), frozen before any gateway code); rules
`56829d8e`; typedef declarator reader, Lean and rule text `d43c7234`;
file-scope typedefs only `12411375`; faithful `for` clause stripping
`776131eb` (the measured tree); evidence and records after it.

## Headline

| Measure | BEFORE (`1db9a569`) | AFTER (`776131eb`) | Delta |
|---|---:|---:|---:|
| Functions Safe / provisional Unsafe / Unknown | 390 / 12 / 4,432 | 390 / 12 / 4,432 | 0 |
| Units Safe / provisional Unsafe / Unknown / data-only | 4 / 10 / 167 / 4 | 4 / 10 / 167 / 4 | 0 |
| `leak` / `summary_conflict` / violations | 37 / 1 / 0 | 37 / 1 / 0 | 0 (every leaf identical) |
| `unknown_alias` | 3,568 | **3,567** | −1 |
| `unsupported` | 42,694 | **42,699** | +5 |
| rule-1 rewrites in pinned-corpus bodies | — | **0** | |

* **Root cause** ([§1](#1-root-cause-and-the-class)): every lexical call reader
  keys on the token pair `NAME (`, so a callee spelled any other way was
  invisible to all of them, and the classifier's last arm lowered the
  statement to `NoOp`. The same arm swallowed a `for` init or increment
  clause, whose grouping tokens are stripped before classification.
* **Fix** ([§2](#2-the-rule-positions-normalised-and-refused)): `(NAME)(…)` is
  rewritten to `NAME(…)` in every function body before any reader runs, so
  every position normalises; every other callee expression refuses; an
  unfaithfully stripped `for` clause refuses.
* **Corpus** ([§3](#3-corpus)): no pinned-corpus body calls through
  `(f)(x)`; the four measured rows are synthetic. 5 `for` clauses refuse
  in 5 functions already Unknown, so no function leaves `Safe` and there is
  no correction to name. No leak or violation is added.
* **Lean**: `QCore.ParenthesisedCallee`, 22 theorems, audited by
  `AuditParenthesisedCallee.lean`, no placeholders, no project axioms.
* **Ledger**: 414 → 417 follow-ups.

## 1. Root cause and the class

### 1.1 The mechanism

`find_call` is `is_identifier(t[i]) && t[i+1] == "("`; `contains_call`,
`nested_call`, the condition-call span reader, the neutral-position and
return-family readers, and the LOADER-ABI, LOADER-ABI-MODES and review-hold
call-site rewrites all use the same pair. `(q_free)(p)` tokenizes as
`( q_free ) ( p )`, which has no such pair. In `classify_simple_base` such a
statement has no call, no top-level `=`, no increment, and no `*`, `.`, `->` or
`[` for the lexical access gate, so it reaches the final arm,
`LegacyInstruction::NoOp`. It is read neither as a cast nor as a grouped
expression: it is an empty effect. The four rows of the brief, reproduced on
this tree by `tests/parenthesised_callee.rs` before any gateway change (19 of
its 22 tests failed there):

| Body | Before | After |
|---|---|---|
| `void *p = q_alloc(); if (p == 0) return; (q_free)(p);` | provisional Unsafe (leak) | **Safe**, the release modeled |
| `… q_free(p); (q_free)(p);` | **Safe** | **provisional Unsafe**, one double release |
| `void zz(void *); … (zz)(p); q_free(p);` | **Safe** | **Unknown**, `call to uncontracted function zz` |
| `… (*fp)(p); q_free(p);` | Unknown, access gate | Unknown, same refusal byte for byte |

### 1.2 The class

The same path skips, or silently drops, every call whose callee is not a bare
name, and one statement position whose tokens lose their calls:

1. **`(NAME)(…)`**, at any depth of parentheses (`((f))(x)`), and after a cast
   (`(void)(f)(x)`, which C reads as `(void)((f)(x))`).
2. **A callee expression that is not a name**: a conditional
   `(c ? f : g)(x)`, a cast `((rel_t) f)(x)`, an address `(&f)(x)`, a comma
   `(c, f)(x)`, a call's result `g()(x)`. Each reached `NoOp`, or, after a
   modeled first call (`(void) r_borrow_code(p), q_pick()(p);`), was dropped
   from the modeled call's tail, because `nested_call` also keys on
   `NAME (`. A subscript `a[i](x)`, a dereference `(*fp)(x)` and a member
   `s.f(x)`/`p->f(x)` were already refused by the lexical access gate.
3. **Every expression position, not only statements.** A condition
   `if ((q_free)(p))` passed `supported_neutral_expression` (no call seen), a
   return `return (f)(x);` the strict return rule; an assignment's right-hand
   side was refused (`unsupported assignment expression`) but then offered to
   the typed routes, which are consulted only when `contains_call` is false;
   a call argument `g((f)(x))` hid a nested call; the review hold, both
   contract withdrawals and W4's derivation scope never saw the call site.
4. **A `for` header's init and increment clauses.** `split_for_clauses`
   drops `(`, `)`, `[` and `]` (`W4-EXPR-POS-FOR-CLAUSES`, recorded in
   [remaining-w4-expr-pos](../remaining-w4-expr-pos/README.md) and never
   registered), so `for (;; q_free(p))` classified `q_free p`, the same
   final `NoOp`, and `for (p = q_alloc(); …)` classified `p = q_alloc`, an
   alias of the name `q_alloc`.

Any expression statement the gateway cannot classify now refuses; `NoOp` is
reached only by a statement with no call operator of any spelling.

## 2. The rule: positions normalised and refused

`crates/qcore-c-gateway/src/legacy_parenthesised_callee.rs`, with hooks marked
with the card's name in `legacy_c.rs` (three entry points, one `Parser` field,
`classify_simple`, seven expression-position readers, the `for` arm),
`legacy_condition_desugar.rs` (`planned_condition`) and
`legacy_declared_modes.rs` (`declared_mode_census`).

**Rule 1, normalise.** In every function body, `(`ᵏ`NAME`)ᵏ followed by `(` is
rewritten to `NAME`, right after tokenizing, in
`analyze_legacy_c_translation_unit_selected_partial`,
`recursive_component_census`, `analyze_legacy_c_tokens` and
`declared_mode_census`, ahead of the review hold, both withdrawals and
everything else. Placing it later (in `build_function_cfg_with`) would have
let `(f)(x)` escape LOADER-ABI-MODES, whose rewrite keys on `f (` while the
environment keeps the withdrawn contract. A group is rewritten only when it is
a callee: `NAME` is not a keyword and not a type name the function trusts, and
the token before the group is punctuation, `return`/`else`/`do`/`__extension__`,
or a `)` closing a cast or a control header (after a name, `]` or any other
`)` the group is an argument list). Conditional marks move with the tokens.

A type name is trusted when a **file-scope** `typedef` declares it in a
declarator position (`typedef_declarators`: before `;`, `,`, `[`, `(` or an
attribute, or alone after `*`s in a declarator group that is not a parameter
list) and nothing in the function's parameters or body could be an object of
the same name (`object_use`, over-approximating). A block-scope typedef is not
trusted outside its block, where the name may be a function.

**Rule 2, refuse.** After rule 1, a `(` applies a call to a callee that is not
a name when it follows `]`, or a `)` closing an argument list or a
parenthesised expression (not a cast, a control header or an operand group of
`__typeof__`/`__attribute__`/`__asm__`). A declaration's declarators are
exempt outside its initialisers and array bounds (`void (*cb)(void *);`).
Where such a call appears, a position that would otherwise be admitted refuses;
a position already refused keeps its refusal byte for byte, and a replaced
admission hands back the sites and element lowerings it consumed.

**Rule 3, `for` clauses.** A clause whose stripped spelling is not faithful to
its original is refused (`for-loop clause with a call or grouping is not
classified from its stripped spelling`) unless its stripped classification
already was. Stripping is faithful when there is no bracket and every group
wraps one name that is not a type, not an argument list and not a callee
(`TAILQ_FOREACH_SAFE`'s `(elm) = (tmpelm)`; C11 6.5.1p5).

| Position | Decision | Why |
|---|---|---|
| statement, initialiser, assignment right-hand side, call argument | normalised | rule 1 runs before the classifier, which then checks `NAME(…)` fully |
| condition (5h, desugared), `if`/`while`/`for`/`do`/`switch` | normalised | same stream; the desugaring and B02/B03 read `NAME (` |
| return (5j, W4 return call) | normalised | same |
| member release (5k), sub-custody guard | normalised | their lexical premises (P1–P4) read the same stream |
| review hold, LOADER-ABI and LOADER-ABI-MODES withdrawals | normalised | rule 1 runs ahead of the three rewrites |
| W4 derivation scope, summaries, certificate recheck | normalised | they re-tokenize through the same entry points |
| every other callee expression, in each position above | refused (rule 2) | no single designator the projection can name |
| `for` init and increment | refused unless faithful (rule 3) | the stripped spelling loses calls and accesses |
| file-scope declarations | untouched | `int (f)(int);` means `int f(int);`, but rewriting would move the ABI readers for no statement |

**Both semantic options.** Option (a) would refuse every `(NAME)(…)` callee;
option (b), taken, normalises it. On the fixtures they differ exactly on rows
1 and 2 (a: both Unknown; b: Safe and provisional Unsafe). On the pinned
corpus they are indistinguishable: the rule-1 census ([§3.2](#32-the-rule-1-census))
rewrites no site, so the sites either option decides do not exist there, and
option (a)'s run would reproduce this AFTER run byte for byte. The normalised
form is fully checked in every position, which is the brief's condition for
preferring (b).

## 3. Corpus

### 3.1 BEFORE and identity

`docs/reports/w43-return-expression/scripts/run_audit.py … --jobs 8`, each into
a fresh `measure/` directory; `baseline_identity.py … --root-alias
<this worktree>=<hungry-thompson> --additive-field contracts:pending_human_review`
against `docs/reports/w45-sub-object-stores/integration/artifacts/`:

| Run | Tree | diagnostics | chunk-contracts, source | contracts | source-effects |
|---|---|---|---|---|---|
| [BEFORE](before-2e6ae8f9-run.json) ([identity](before-2e6ae8f9-identity.json)) | `2e6ae8f9` | byte-identical | byte-identical | identical after the root alias | checkout-path digests only |
| [BEFORE](before-run.json) ([identity](before-identity.json)) | `1db9a569` | byte-identical | byte-identical | identical ignoring exactly `pending_human_review` | checkout-path digests only |
| [AFTER](artifacts/run.json) ([identity](after-identity.json)) | `776131eb` | differs, §3.3 | byte-identical | identical ignoring exactly `pending_human_review` | checkout-path digests only |

The AFTER artifacts are archived in [artifacts/](artifacts/) (gzip, mtime 0,
[manifest](artifacts/manifest.json)).

### 3.2 The rule-1 census

`QCORE_PARENTHESISED_CALLEE_CENSUS` (census feature only, listed in
`census_sink_variables`) streams one record per rewritten callee. Both census
runs reproduce their tree's AFTER run's five artifacts byte for byte
([run 1](census/census-1-run.json) against [AFTER](census/after-first-rule-3-run.json),
[run 2](census/census-2-run.json) against [AFTER](census/after-12411375-run.json)).

* **First census**, tree `56829d8e`
  ([summary](census/census-1-summary.json), [stream](census/census-1-rewrites.jsonl.gz)):
  6,931 distinct rewrites in 46 units, **every one a cast**: 6,930 through the
  compiler's vector types (`(__m128i)(x)`, `(__v2di)(x)`, … in the intrinsic
  headers' inline bodies) and one `(ssize_t)(…)` in `rd_string_render`. The
  model's typedef reader (`TypedAccessModel::declares_typedef`) reads only a
  typedef's last token, so `typedef long long __v2di __attribute__ ((…));` is
  not a typedef to it, and `rd_string_render`'s parameter `ssize_t (*…)(…)`
  read as an object use. A cast read as a call is the refusing direction, and
  no diagnostic moved (the AFTER diagnostics of `56829d8e` and `12411375` are
  byte-identical); it is fixed in `d43c7234` and `12411375` (the declarator
  reader, file-scope typedefs only, `T (*` read as a declarator).
* **Second census**, tree `12411375` ([summary](census/census-2-summary.json)):
  **0 rewrites**; the sink was never opened. No function body of the pinned
  corpus calls through a parenthesised name, and the rule-1 code is unchanged
  in `776131eb`.

### 3.3 Movement and attribution

[delta.json](delta.json) (`scripts/delta.py BEFORE AFTER`); the source line of
every added leaf is in [added-leaves.tsv](added-leaves.tsv)
(`scripts/source_lines.py`, reading the pinned read-only checkout).

| Function | Unit | Leaves | Source | Attribution |
|---|---|---|---|---|
| `balance` | `rdkafka_sticky_assignor.c` | +1 `for-loop clause …` | `RD_MAP_FOREACH_ELEM(elem, &fixedAssignments.rmap)` | rule 3: `rd_map_iter_begin((RMAP), &(ELEM))` and `rd_map_iter_next(&(ELEM))` were classified as `rd_map_iter_begin RMAP & ELEM`, calls dropped |
| `rd_kafka_sticky_assignor_assign_cb` | `rdkafka_sticky_assignor.c` | +1 | `RD_MAP_FOREACH_ELEM(elem, &currentAssignment.rmap)` | the same |
| `rd_map_copy` | `rdmap.c` | +1 | `RD_MAP_FOREACH_ELEM(elem, src)` | the same |
| `unittest_untyped_map` | `rdmap.c` | +1 | `RD_MAP_FOREACH_ELEM(elem, &rmap)` | the same |
| `ut_tq_find_prev_pos` | `rdunittest.c` | +1, −1 `unresolved resource identity e` | `TAILQ_FOREACH(e, head, link)` | rule 3: the increment `(e) = ((e)->link.tqe_next)` wraps a member chain in a group; its stripped reading reported the alias it bound, and the refusal replaces that report |

All five functions were Unknown before and stay Unknown, so no function moves
and **no function leaves `Safe`**: there is no correction to name, because no
`Safe` function of the pinned corpus rested on a skipped statement of this
class. The four `RD_MAP_FOREACH_ELEM` refusals are corrections of skipped calls
inside functions already Unknown; the `TAILQ_FOREACH` refusal is a precision
cost of rule 3 (its stripped spelling was faithful up to the member chain), and
it is the follow-on's to recover. No leak, double release, invalid release or
use-after-release is added or removed. Units: 0 moves.

Rule 3's first version (`56829d8e`) refused every grouped clause: 44 added and
31 removed leaves in 40 Unknown functions, 39 of them `TAILQ_FOREACH_SAFE`'s
`(elm) = (tmpelm)`, whose stripped spelling is the alias it is
([delta](census/delta-first-rule-3.json)). `776131eb` keeps the stripped
classification exactly when stripping is faithful.

### 3.4 Prediction against outcome

| Frozen forecast | Outcome |
|---|---|
| functions leaving `Safe`: 0–5, each a correction | **0** |
| functions entering `Safe`: at most 2 | **0** |
| provisional `Unsafe`: at most +2 | **0** |
| leak pin 37 ± 3 | **37**, every leaf identical |
| `unsupported` moves by fewer than 200 | **+5** (`unknown_alias` −1) |
| no unit moves | **0** |

The forecast was directional; the class it described is absent from the
corpus bodies, which the census, not the forecast, established.

## 4. Tests

* `crates/qcore-c-gateway/tests/parenthesised_callee.rs` (22): the four rows;
  `(f)(a)` and `f(a)` giving an identical whole-program summary in the
  statement, nested-parentheses, `(void)` cast, initialiser, assignment,
  condition (`if`, `while`, `&&`, `switch`, `do`), return, call argument,
  descriptor, member-release, W4 summary (both directions) and LOADER-ABI-MODES
  withdrawal positions; every other callee expression refusing, with the rule's
  own reason where nothing else refused, in every expression position; the
  `for` clause (a dropped release refused, a single-name group kept); casts,
  a shadowed typedef name, an argument list and a declarator.
* `crates/qcore-c-gateway/tests/contract_deferred_review.rs`:
  `a_parenthesised_call_of_a_pending_callee_is_held` — with `k_close`'s
  contract deferred, `(k_close)(h, 0)` is held `Unknown` under
  `awaiting-human-review` and reports no leak; under the bare contracts it is
  `Safe` exactly as `k_close(h, 0)`. Before the card it was a provisional
  `Unsafe` leak.
* `crates/qcore-c-gateway/tests/unit/legacy_parenthesised_callee.rs` (12): the
  rewrite and its exclusions, file-scope declarations, conditional marks,
  typedef declarators, block-scope typedefs, vector-type casts, the
  opaque-callee class and the faithful-stripping predicate.

## 5. Lean

`proof/QCore/ParenthesisedCallee.lean`, audited by
`proof/AuditParenthesisedCallee.lean` (22 `#print axioms`, only the three
allowed axioms), registered in `QCore.lean`. It models a callee expression
(`name`, `paren`, `other`), what C designates through it, rule 1 (`norm`), the
classifier after the card (`lower`) and before it (`lowerBefore`), rule 2's
hook (`guard`) and rule 3 (`forClause`). Proved: rule 1 preserves the
designated function and is idempotent (`norm_designates`, `norm_parens`,
`norm_is_name_iff`); `(f)(a)` and `f(a)` have the same lowering
(`a_parenthesised_name_is_the_name`, `parentheses_never_change_a_lowering`);
an admitted call is a call of exactly the designated function
(`an_admitted_call_is_the_designated_call`); **a call is never a `NoOp`, and a
`NoOp` is only a call-free statement** (`a_call_is_never_a_no_op`,
`a_no_op_is_call_free`); a callee designating no name is refused and keeps its
body out of `Safe`; rules 2 and 3 never admit what the base refused; and the
non-vacuity statements that the hole existed. Faithful token extraction and
the Rust correspondence are trust boundary 28's premises.

## 6. Validation

In this worktree with `CARGO_INCREMENTAL=0` and `QCORE_CLANG=/usr/bin/clang`;
every step's output and exit in the private `private/gate/`, nothing piped.
Measured at `ecedc357` (code as `776131eb`); figures in [gate.json](gate.json).

| Step | Command | Exit | Count |
|---|---|---:|---|
| submodules | `git submodule update --init third_party/libevent` | 0 | the `--recursive` form was refused by the session's permission classifier; the one submodule has none of its own |
| clang | `clang --version` | 0 | Ubuntu clang 21.1.8 ([recording](clang-version.txt)) |
| test | `python3 scripts/run_test_binaries.py` | 0 | **5,781 passed / 0 failed / 11 ignored**, 307 summaries over 297 units (`private/test-binaries/20260925T191907Z-1030790/`) |
| clean | `cargo clean --profile dev` | 0 | |
| fmt | `cargo fmt --all -- --check` | 0 | |
| clippy | `cargo clippy --all-targets -- -D warnings` | 0 | |
| probe / followup | `cargo run -p qcore-compiler --example reevaluation_probe` / `reevaluation_followup` | 0, 0 | |
| lean | `python3 scripts/check_lean_audits.py` | 0 | **195** audits (+1, `AuditParenthesisedCallee.lean`) |
| census feature | `cargo clippy -p qcore-c-gateway --features indirect-census --all-targets -- -D warnings`; `cargo test -p qcore-c-gateway --features indirect-census --lib` | 0, 0 | 578 passed |
| clean | `cargo clean --profile dev` | 0 | |
| matrix, python | `build_matrix.py .`, then `python3 -m unittest discover -s scripts/tests` | 0, 0 | **358** tests; `named_followups` 414 → 417 |
| docs index | `python3 scripts/build_docs_index.py`, then `--check` | 0, 0 | |
| arbiter | `run_audit.py measure/after-776131eb 776131eb --jobs 8`; identity as §3.1 | 0; 1 | diagnostics differ by the §3.3 delta only |

`providers/` is untouched, so the provider gate was not run; `provider_tests`
in [gate.json](gate.json) is carried. Constraint 16's load-sensitive rows all
passed in the full run; no isolated re-verification was needed.

## 7. Ledger

Registered in `named_followups` (414 → 417) from [followups.json](followups.json)
by [scripts/update_ledger.py](scripts/update_ledger.py), which also repoints
`corpus.cycle19_raw_units` at [artifacts/](artifacts/) (previous row in
[previous-corpus-row.json](previous-corpus-row.json)); `build_matrix.py .`
regenerated the derived sections.

* `W45-FOR-CLAUSE-ORIGINAL-SPELLING`: classify `for` init and increment from
  their original spelling and retire rule 3; residual 5 refused clauses in 5
  Unknown functions (4 of them with calls the stripped spelling dropped).
* `W45-CALLEE-EXPRESSION-FORMS`: model `(&f)(x)`, a cast of a function
  designator and a conditional callee; residual 0 corpus leaves.
* `W45-CALLEE-CAST-LEXICAL-PREMISE`: the cast/callee distinction is lexical
  (file-scope typedef declarators and an object-use over-approximation); the
  first census measured the reader's gap on 6,930 vector-type casts, all in the
  refusing direction.

## 8. For the integrator

* **Likely conflicts**: `crates/qcore-c-gateway/src/legacy_c.rs` (a module
  declaration after `review_hold`, one line before each of the three
  `hold_call_sites` hooks, one `Parser` field and its construction, the
  `classify_simple` wrapper, six one-line hooks in the condition and return
  readers, the `for` arm); `legacy_condition_desugar.rs` (one line in
  `planned_condition`); `legacy_declared_modes.rs` (one line in
  `declared_mode_census`); `lib.rs` (one census sink); `proof/QCore.lean` (one
  import); `docs/agents/gateway.md` (10c after 10b); `docs/agents/README.md`
  (one row); `docs/STATUS.md`; `docs/reviews/r0_3/acceptance_matrix.json`;
  `docs/development/increments/wave-45.md`; `docs/reports/INDEX.md`.
* The rule-1 rewrite must stay **ahead of** the review hold and both
  withdrawals at every entry point; a new entry point that tokenizes a unit
  needs the same line.
* The corpus row and STATUS now point at this card's AFTER artifacts, on base
  `1db9a569`.

# CG27-LOOPS-RECURSION-BUDGETS: checked loop summaries, a signed threshold rank, and budgets as coverage

Card `CG27-LOOPS-RECURSION-BUDGETS` of the
[Clang gap card queue](../../plans/clang-gap-cards/README.md) (wave 48, round 10;
entries C06, C07, C08; route G). Base `b6a3dc42` (local `main` = `origin/main`),
rebased onto main `8e8ed41f` (CG17-LAYOUT-DERIVED-EXTENTS, rule 11u, itself
byte-identical on every route) and then `06ad81cb` (CG20-NUMERIC-DOMAIN, rule
11q, a separate numeric property beside custody) and `b934c560`
(CG25-RESOURCE-KINDS, rule 11v: two resource kinds and the
`status_witnesses` state field; corpus 630 / 10 / 4,194 `5ade5aab…` there)
and `f0bbc0eb` (CG26-VARIADICS, rule 11w: its own per-`va_list` CFG and fixed
point). The BEFORE/AFTER measurements were taken on `b6a3dc42`; those cards
touch disjoint code (CG17: `layout_extents.rs` and its hooks; CG20:
`c_numeric*.rs` and a declared-ABI hook; CG25: resource kinds, the custody
state's witnesses and libc contracts; CG26: `va_list_protocol.rs`; this card:
the counted-user pass and the ranked recursion route, constructing no
`AbstractState`), so the arbiter was not repeated on the rebased tree, and the
gate (§8) was. An integrator re-measure on the merged tree would compare
against main's current corpus figures.
The user's instruction to launch every remaining CG card is the Phase 4 choice
of guarantee; the property this card publishes is stated in §1.

- **Rule:** [gateway 11x](../../agents/gateway.md#constraint-11x), amending rule
  10e's loop refusal only.
- **Lean:** `QCore.LoopInvariants`, `proof/AuditLoopInvariants.lean` (32
  theorems), over `QCore.UnsignedCountdown`, `QCore.CountedUsers` and
  `QCore.VerdictSemigroup`.
- **Prediction:** [PREDICTION.md](PREDICTION.md), commit `0526f9da` after the rebases onto `f0bbc0eb` (`745be99f` before them), before any
  AFTER run. The implementation had been drafted in the working tree first; the
  file says so.

## 1. The published property and its scope

Route G, source profile `fluent-bit` / Cycle 19 pinned corpus, custody
(leak, double release, release after escape) and counted-user clauses, under
the synthetic or declared contracts in force:

1. **A counted-user loop** is summarized only by the invariant "every count,
   pending release check and possibly-destroyed mark equals its value at the
   loop's head", checked over every token of the loop (condition, step, body,
   nested statements). Loops are otherwise still refused.
2. **A recursive component** none of the existing routes publishes is
   published when it carries a *signed threshold rank*: base `n <= c` checked
   with no component summary, step `n > c` checked with the base contracts,
   actuals `n - k` (`1 <= k <= 1024`, several at once) or `0`, one
   `int`/`long`/`long long` control type for every member.
3. **A budget** (custody alias widening at 8 targets, the counted-user path
   bound of 512, route T's 64 paths / 512 operations / depth 64 / 2,048 nodes)
   is reported when exhausted and its function is never `Safe`.

Nothing else is claimed: no numeric invariant, no heap or structural
recursion, no termination of a C loop (the counted-user invariant holds on
every prefix of every run, so it needs none).

## 2. What changed

**Counted-user loops** (`legacy_counted_loops.rs`, a child of
`legacy_counted_users.rs`). `parse_statement` reads a loop's extent
(`statement_extent`, through nested `if`/`while`/`for`/`switch`/`do`/blocks)
into `Stmt::Loop`; an unreadable extent keeps `Stmt::Unsupported("a loop")`.
The walker asks `neutral_loop`: no counter write, no call whose contract
declares a counted-user operation or exit clause, no call through an
expression, no `return`, no `goto`, and every other named call neutral by
`Unit::call_effect`, exactly as a callee anywhere else is decided. On success
every path state keeps its counts and forgets the value of each name the loop
spells; otherwise the function is abandoned with the pre-existing wording,
so a refused loop changes no artifact.

**Signed threshold rank** (`legacy_signed_rank.rs`, a child of
`legacy_recursion.rs`). `prepare` reads the component as described in rule
11x(2) into the same `ContextFunction` descriptors the zero/nonzero route
uses, with one new field, `guards`: each checked sign guard's token range and
whether it is true in the base context. `contextual_cfg_under` respells such a
guard as `n == 0` (true in base) or `n != 0`, so the existing context
selection decides it; `QCore.LoopInvariants.SignedThreshold.a_read_guard_is_decided_by_its_context`
proves the respelling agrees with the guard at every control value.
`legacy_summary.rs` gains `RankedRoute` and consults the signed route **last**,
only when the neutral-frame route also published nothing: it replaces no
contract, and every component another route decided is decided as before.
The zero/nonzero `prepare` now shares `member_header` (signature, single
integer control, flat pointer formals, prototype agreement, no loop or jump
keyword); its behaviour is unchanged.

**Budgets.** No analyzer change; tests and Lean (§5).

**Shared items added** (for the integrator): `legacy_recursion.rs`
`MemberHeader`, `member_header`, `ContextFunction::guards`, `mod signed_rank`;
`legacy_summary.rs` `RankedRoute`; `legacy_counted_users.rs` `Stmt::Loop`,
`loop_statement`, `mod loops`. No shared foundation (`elaboration.rs`,
`instantiate`, the typed place) is touched.

## 3. BEFORE: the cited claims, reproduced

| entry | claim (analysis §5) | reproduced? |
|---|---|---|
| C06 | indexed collections and limited scans are not invariants for arbitrary loops (Q12, Q25) | **Yes**, CG00's `c06_collection_loops_have_no_invariant` (a constant-bound fill/release pair refuses). **Partial correction:** route G's work list already computes a custody invariant for an arbitrary loop (the settled loop-head state; CG04's generations make the lattice finite): a per-iteration acquire/release in `for`, `while`, `do` and `for (;;)` with `break` is `Safe` at base, and a body that breaks it is refused (`loops_recursion_budgets.rs`). The gap is numeric and heap invariants, the counted-user pass, and route T. |
| C07 | countdown partitions and neutral frames exist; not general multiple ranks or heap-structural recursion (Q26, Q27) | **Yes.** At base a signed control has no rank (`n - 1` is admitted only on an unsigned component), `n - 2` on no route, and an effectful `if (n > 0) … f(p, n - 1)` is refused by every route (the neutral pins of `neutral_frame_recursion.rs`, `c03_neutral_frame_recursion_probe.rs`, `neutral_reach_census.rs`). CG00 already recorded the partial correction that neutral two-call and mutual recursion are admitted. |
| C08 | T: 64 paths, 512 operations, depth 64, 2,048 nodes; G: `MAX_ALIAS_TARGETS` 8 with widening (Q08–Q10, Q13) | **Yes**, constants read in `typed_source.rs` and `legacy_c.rs`; CG00's `c08_budgets_are_reported_never_safe`. **Addition:** the counted-user pass has its own path bound (512, `PATH_BOUND`), not named by the analysis; it too is reported (`counted users undecided: more than 512 paths`) and never `Safe`. "Widening" on G is not a precision-preserving widening: it drops the targets to unknown and reports `alias set exceeded 8 targets and was widened`. |

Baseline reproduced on this worktree's BEFORE (`private/cg27lrb/before*`,
release binary `191798bb…` built at base): corpus **629 / 10 / 4,195**,
`resource-diagnostics.json` `3d53a5d0…` (identical to the brief's); system
**44 / 2 / 188** and tail-pause slice **29 / 1 / 239 / 1 Pending** (de-duplicated
totals identical to the brief's). The system and slice digests (`4e145193…`,
`1bdf3a9c…`) differ from the brief's (`79404fed…`, `93cc660c…`) only because
those reports embed the measuring worktree's absolute path (8 and 14
occurrences of the contract path under `contract/profiles/fluent-bit/../../../providers/`);
they compare within one worktree only.

## 4. Census (BEFORE) and prediction against outcome

`scripts/census.py` over the BEFORE runs (non-`Safe` functions with a leaf of
each family, de-duplicated by unit and function as the totals are; "alone":
every leaf in that family):

| run | loop-shape functions (alone) | recursion functions (alone) | budget functions |
|---|---|---|---|
| corpus | 341 (7) | 22 (1) | **0** |
| system | 6 (0) | 2 (1) | 0 |
| slice | 12 (0) | 0 | 0 |

- **Loop shapes** in the corpus are 258 `for`-loop, 121 `while` and 24
  `do`-while condition leaves (expression elaboration, CG02/CG03's), 41 rule-11a
  re-acquisition refusals (CG04's boundary) and 6 `for` clauses not classified
  from their stripped spelling. None is a missing invariant: the 7 loop-only
  functions (`map_lock`, `cmt_metric_exp_hist_lock`, `buffer_skip_whitespace`,
  `rtrim`, `rd_kafka_broker_ops_serve`, `_rd_strcasestr`, `mk_utils_hex2int`)
  each refuse a call or member access in the loop's condition.
- **Counted-user loops:** **0** functions in any of the three runs carry the
  counted-user loop refusal (`the body contains a loop`). The retry-route
  functions W45-COUNTED-USERS-LOOPS named are blocked earlier, by unattributed
  operations (`handle_output_event`: `flb_task_users_dec` on `task`, which is
  not an unrebound formal; `flb_output_task_queue_flush_one`: on
  `queued_task -> task`), so their bodies are never walked.
- **Recursion:** 22 corpus functions name themselves or a mutual partner as an
  uncontracted callee; the one blocked by nothing else is
  `cio_os_mkpath`, structural recursion through `dirname(dup_dir)` with no
  integer control (a residual, §7). On the system route the pair
  `flb_output_task_singleplex_flush_next` ↔ `flb_output_task_queue_flush_one`
  is mutual recursion whose rank would be the length of `queue->pending`, a
  heap measure; `flush_next`'s only non-`Safe` leaf is that call (a residual).
- **Budgets:** no function in any run exhausts a budget.

**Outcome against the prediction: met exactly.** AFTER (binary `95c13b1f…`,
built at `2d6306a1`, measured with `SKIP_BUILD=1 BINARY=…`):

| route | BEFORE | AFTER | moves |
|---|---|---|---|
| corpus `66910c10` | 629 / 10 / 4,195, `3d53a5d0…` | **byte-identical** (`3d53a5d0…`; the contracts, chunk-contracts, source-effects and Monkey-source artifacts too) | 0 |
| system | 44 / 2 / 188, `4e145193…` | **byte-identical** | 0 |
| tail-pause slice | 29 / 1 / 239 / 1 P, `1bdf3a9c…` | **byte-identical** | 0 |

No function moved and no same-status function changed a leaf, so there is no
correction, no added leak and no added violation to classify. The corpus
typed-path parity streams (466,022 rows each) are identical as multisets once
Clang's AST node addresses, which differ between any two runs, are normalised
(`scripts/parity_diff.py`). Why nothing moved, by the census above: no function
in any route carries the counted-user loop refusal, and every recursive
component that reaches the derivation refuses at an earlier gate than the rank
(§7, C03-NFR-REACH). The card's reach is its tests: what it admits is a
capability for code the corpus does not contain in an otherwise admissible
form. Artifacts: [`artifacts/`](artifacts/) (`digests.txt`, the three
`*-comparison.json`, the censuses, `corpus-parity-diff.txt`).

## 5. Tests and Lean

`crates/qcore-c-gateway/tests/loops_recursion_budgets.rs` (17 tests):

- *Loop invariants, route G:* a per-iteration acquire/release loop is `Safe`
  in four loop forms; a body that breaks the invariant (an unreleased
  acquisition, a release of a resource held only on the first iteration) is not.
- *Counted-user loops:* a retry body with three summarized loops holds all three
  clause records and is `Safe`; an addition, a drop, a release check, a counter
  write, an opaque call, a `return`, a `goto` and a call through an expression
  in a loop are each refused with the pre-existing wording; a summarized loop
  does not hide the C2 refutation; a loop forgets a literal it may overwrite
  (the stale literal would have pruned the refuting arm).
- *Recursion:* single-rank over four control spellings and six guard spellings,
  multi-rank (two resources at two ranks; an owned result at two ranks), mutual
  recursion with a valid summary (both members, mixed steps), caller leaks
  kept; 20 non-ranked shapes never publish (C07's signed `n == 0`, `!n`, no
  decrease, increase, `1 - n`, a nonzero literal, a negative threshold, two
  thresholds, recursion in the base, an assigned, addressed or shadowed control,
  a compound guard, `signed char` and `short` controls, a step above 1,024, a
  double release, inconsistent contexts, one resource released by two ranks, a
  loop), nor mixed widths or mixed thresholds across a mutual pair; the
  certificate recheck refuses `n + 1`, `n`, `n == 0`, `n < 0` and `short n`
  substitutions and a forged summary; the zero/nonzero route's own shapes keep
  their summaries and unsigned `n - 2` stays refused; the effectful neutral pins
  now carry a rank (census `PublishedByRankedRecursion`) and a release after the
  recursive release is never `Safe`.
- *Budgets:* alias widening at nine targets (not at eight) is `Unknown`; the
  counted-user path bound at 1,024 paths (not at 256) is `Unknown`; route T's
  path limit at 128 paths (not at 64) is refused.

Unit tests: `sign_guard` and `step_actual` spellings, `signed_wide`, the loop
extent reader, and `lean_vectors_match_the_rust_reader`, which reads
`guardVectors` and `stepVectors` from `LoopInvariants.lean` (proved there by
`guard_vectors_hold` and `step_vectors_hold`) and checks the Rust readers on
both the fused and the split operator spelling.

**Re-pinned tests.** Four pins that asserted an effectful `if (n > 0) … f(p, n -
1)` stays refused now meet a route that publishes it. Their guard is changed to
`if (n)` on the signed control (no rank on any route), so each keeps testing the
neutral refusal it was written for: `neutral_frame_recursion.rs`
(`a_body_that_carries_an_effect_refuses_the_whole_component`, four bodies and
the acquiring `make`), `c03_neutral_frame_recursion_probe.rs`
(`undeclared_neutral_recursion_leaves_the_component_and_its_callers_unknown`)
and `neutral_reach_census.rs`
(`an_effectful_member_refuses_at_v5_and_names_its_blocking_instruction`). The
`n > 0` forms themselves are asserted under the new route in
`the_effectful_neutral_pins_now_carry_a_rank`. `legacy_counted_users.rs`'s
`statements_parse_into_branches_and_exits` now expects `Stmt::Loop` for a
readable loop and keeps `Unsupported` for an unreadable one.

**Lean** (`QCore.LoopInvariants`, 32 theorems, no `sorry`, no project axiom):
`Induction` (invariant induction over iterations and over any sequence of body
paths; the decidable check refuses a broken transition; a post-fixed point of a
monotone transfer bounds every iterate), `CountedLoop` (an admitted loop keeps
`delta` and `pendingFrom` for any number of iterations, spliced anywhere; a
counting or opaque body is refused; one counting iteration changes the count),
`Widening` (strict growth lowers a potential; the widened value is absorbing and
never `Safe`), `SignedThreshold` (admitted actuals decrease, several at once;
the relation is well founded; base contexts make no call; publication by strong
induction over the rank via `UnsignedCountdown.publish_by_mutual_rank_induction`;
the guard respelling; `int` representability; the unsigned `n - 2` and signed
`n == 0` counterexamples; shared vectors), `Budget` (an exhausted leaf makes
the function's fold at least `Unknown`; non-vacuity).

## 6. Axes (analysis §8)

- **Control:** `while`, `for`, `do` and nested loops with `break`/`continue`
  in the counted-user pass; `return`/`goto` in a loop stay refused; route T's
  back-edges stay refused (rule 11l).
- **Recursion:** self, mutual, multi-rank; signed `int`/`long`/`long long`;
  narrower, unsigned multi-rank, mixed, structural recursion refused.
- **Evidence/verdict:** budgets are coverage (`Unknown`), never `Safe`.
- **Evaluation order:** no order is introduced; the loop check is
  flow-insensitive and the rank reads only standalone calls and assignment
  right-hand sides, as the zero/nonzero route does.

## 7. Follow-ups

- **Retired:** `W45-COUNTED-USERS-LOOPS` (the bounded loop rule with Lean
  parity, measured on the retry route: 0 functions carry the loop refusal at
  base, so the rule frees none there; the retry functions' blocker is
  unattributed operations, which the rule does not touch).
- **Amended:** `CG27-LOOPS-RECURSION-BUDGETS` (what landed, residual sized),
  `W43-COLLECTION-LOOP-SHAPES` (unchanged; the collection pass's loop reading
  is not this card's), `JOINED-PROVENANCE-CYCLE-SITES` (unchanged; CG04's
  generations supply the per-iteration identity half of its DISCHARGE, the
  pre-pass is not changed here), `C03-NFR-REACH` (re-read below).
- **C03-NFR-REACH, re-read.** C03-REACH's census (`scripts/c03_census.sh`,
  release build; the debug build timed out at 7,200 s on the loaded host) at
  the card's tip: **86** recursive components (63 self-recursive, 155 members)
  over 185 units, against C03-REACH's 46 / 96 at `695f0e1`; **3** admitted by
  the neutral route, 1 kept by a declared contract, **82** refused, **0**
  published by the signed threshold rank. The first refusing blockers are still
  the containment gate's retained instructions (indirect memory access 718,
  branch-condition access 472, uncontracted call 393 occurrences), so the
  entry's conclusion stands: further reach needs model capability, not a rank.
  Because the signed route is consulted last and published nothing, the
  derivation, and hence this census, is the base's.
  [`artifacts/c03-census-after.json.gz`](artifacts/c03-census-after.json.gz)
  (uncompressed SHA-256 `2a8c1722…`).
- **Registered:** `CG27-LOOP-RECURSION-RESIDUALS` (stated loop invariants,
  route-T back-edges, counting loops, recursion without a literal signed rank
  including `cio_os_mkpath`, budget improvements by shared DAGs).

## 8. Gate

Standing gate at tip `26d105af` (on main `f0bbc0eb`), `scripts/gate.sh` under
the shared heavy-step lock; exits in [`gate/`](gate/), record in
[`gate.json`](gate.json), logs in this worktree's `private/cg27lrb-gate/`:

| step | exit | count |
|---|---|---|
| `git submodule update --init --recursive` | 0 | |
| `cargo check --workspace --all-targets` | 0 | |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --locked --all-targets -- -D warnings` | 0 | |
| `scripts/run_test_binaries.py --jobs 4` | 0 | 418 units, 431 summaries, **7,280 passed**, 0 failed, 11 ignored (`private/test-binaries/20261001T193910Z-255152`) |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `scripts/check_lean_audits.py` | 0 | **268** audits (267 on main + `AuditLoopInvariants.lean`) |
| `python3 -m unittest discover -s scripts/tests` | 1, then 0 | 365; the one expected failure (derived `lean_audits` 267 → 268) cleared by `build_matrix.py . --gate gate.json` |
| `scripts/build_docs_index.py --check` | 0 | |

`providers/` is unchanged, so the provider gate is not due. Three earlier
test-runner attempts on base `b6a3dc42` are discarded as evidence: two met
`ENOSPC` on the shared disk (rustc and `ld` I/O failures), the third ran on test
binaries the disk exhaustion had damaged (many units exited 101 with no output).

## 9. Reproduction

`scripts/measure.sh NAME` (corpus, system, slice; `SKIP_BUILD=1 BINARY=…` for a
prebuilt binary), `scripts/build_after.sh` (the AFTER binary in a private
target directory while the BEFORE read `target/release`), `scripts/census.py`,
`scripts/compare.py`, `scripts/changed.py`, `scripts/c03_census.sh`,
`scripts/gate.sh`, `scripts/update_ledger.py`. Artifacts in
[`artifacts/`](artifacts/).

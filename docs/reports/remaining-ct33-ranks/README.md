# CT33-RANKS — a lock-rank source form onto the represented rank refusal

Wave 10. Card CT33-RANKS: give lock ranks a bounded checked existence in
`CheckedMutexInvariantV1`, so that CT33's scenario — a lock-order cycle — can
be written down in QCore source and is refused at check time.

Base: `main` at `0f6d66c`. All work is on this card's branch.

## 1. What CT33 was blocked on, and what was actually in the way

The recorded blocker
(`docs/reports/wave5-ct-integration/ct_stage_evidence.json`, and the same
sentence in `docs/reviews/r0_3/acceptance_matrix.json`) reads:

> Lock-order cycle. Rank mode is a represented refusal in
> `CheckedMutexInvariantV1`, not a feature; there is no source form that
> declares a lock rank, so a cycle cannot be written down.

That is true and was the first blocker. Measuring the route turned up a
**second** one the blocker does not mention: `source.mutex.guard.v1` refused a
nested `with lock` outright, lowering it to
`UnsupportedConstruct { NestedProtectingMutex }` (`Q622`). A lock-order cycle
*is* two nestings in opposite directions, so even with a rank form the shape
would still have been inexpressible. Both had to move, and the way they were
made to move together is the design decision this card turns on (§3).

The represented refusal itself carries a code today, so no §L21 process was
needed for it: `UnsupportedSynchronization::LockRankDeclaration` reaches
`UnsupportedSynchronizationConstruct`, which is **`Q622`**. That variant is
kept, unchanged, and now denotes a rank construct *outside* this bounded
discipline (a dynamic rank, a rank propagated through a call, a rank relation
the model cannot decide). Reusing `Q622` for a genuine ordering violation was
rejected: `Q622` means "this profile does not model that construct", and a
program whose declared order is violated is a different, real finding. Under
the §L21 process two codes were taken from the free `Q624`–`Q629` band
immediately above the profile's own, and the spec was updated in the same
change (§7).

## 2. The grammar

One production is added to `source.mutex.guard.v1`, and nothing else:

```ebnf
outer = ...
      | "let" , NAME , "=" , "mutex" , "guards" , NAME , ";"
      | "let" , NAME , "=" , "mutex" , "rank" , NAT , "guards" , NAME , ";"
```

`NAT` is one decimal literal in `0 ..= 4294967295` (the reader is `u32`-valued,
so a sign, an overflow and a non-literal all land on the same refusal rather
than being truncated or defaulted). `with lock` is untouched: the acquisition
order a nested section already produces is what the discipline decides.

Both route-detection sites learned the ranked spelling, because otherwise
`mutex rank` would have been the one mutex form that escaped them:
`source_mutex::has_source_mutex` (route selection) and
`module_imports` (the `import std.concurrency;` requirement). A ranked
declaration with no import refuses at `Q015`, exactly as `mutex guards` does.

### Reserved-word collision audit

`rank` is the one new reserved word. Measured, not assumed:

| Check | Result |
|---|---|
| `rank` as an identifier in the 326 `.qc`/`.qcore` files in the tree | **0 occurrences** |
| `rank` as an identifier in QCore source embedded in Rust string fixtures | 0; every `rank` in `crates/`+`compiler/` is Rust (`LockRank`, `held_rank`, `release_rank`, prose) |
| Is the reservation load-bearing? | **Yes, measured.** Removing `"rank"` from `is_reserved_source_word` and re-running the accepted ranked control fails with `["Q013", "Q013"]`: without it the declaration's own keyword is collected as a referenced name. The removal was performed, measured and reverted. |
| Does the reservation cost an existing spelling? | **No, measured.** `rank` is still usable as a resource name (`let rank = protected 4; ... write rank with g;` checks), exactly as the already-reserved `guards` is. Both halves are asserted in `reserving_rank_is_load_bearing_and_costs_no_existing_spelling` so a later edit cannot quietly drop either. |

The reservation list's effect is to keep a word out of a statement's
*referenced-name* set. That makes a reserved word invisible to name resolution
rather than illegal, which is the standing compatibility hazard T03-F recorded
for its five words; `rank` inherits it, with the zero-occurrence census above
as the evidence that nothing in the tree is affected.

## 3. The discipline

`MutexInvariantProgram` gains `lock_ranks: BTreeMap<MutexId, LockRank>`
(`LockRank = u32`, reused from `crate::concurrency`, not redefined). The
discipline is **opt-in and module-wide**: empty map, discipline off, checker
byte-identical to before. That is what leaves every already-written trace
unmoved, and it is asserted rather than argued
(`an_empty_rank_map_checks_no_order_at_all`, plus the whole pre-existing suite
passing unchanged).

With a non-empty map:

**R1 — no mixed modules (`Q625`, `MixedLockRankDeclaration`).** Every
generation the trace creates must have a declared rank, and every declared rank
must name a generation the trace creates. A module that ranks some of its
mutexes and not others has no lock order at all, so it is refused rather than
checked over its ranked part; and a rank declared for a generation that does
not exist orders nothing, so it is refused too. Both directions are tested
(`a_created_generation_with_no_declared_rank_is_refused`,
`a_declared_rank_for_an_uncreated_generation_is_refused`). The first carries
`event: Some(index)` and points at the offending `CreateMutex`; the second
carries `event: None` and falls back to the scope span, like the end-of-trace
sweep.

**R2 — strictly increasing acquisition (`Q624`, `LockRankViolation`).** A
`Lock` of a generation at rank `r` by a task already holding a live guard over
a generation at rank `held >= r` is refused. Equality is refused, not admitted:
two generations at one rank may never be nested in either direction, which is
what makes the order strict. The diagnostic names both sides —
`{event, mutex, rank, held_guard, held_mutex, held_rank}` — so a reader sees
the comparison that failed rather than being told an order exists.

**Placement.** R2 is the **last** precondition of `Lock`, after the existing
dead-generation, unknown-mutex, already-held and guard-freshness checks. The
discipline is additive: it never displaces a refusal the profile already
issues. The case that makes this visible is a self-relock, where the equal rank
*also* violates R2: it keeps `Q600 DuplicateLockCapability`, the stronger
rank-free fact that the generation is held
(`a_self_relock_reports_the_held_generation_not_the_equal_rank` asserts both
that `Q600` is present and that `Q624` is absent).

**Fail-closed on the unrankable.** Both `None` arms of the comparison refuse.
An unranked generation on either side has no position, and an acquisition with
no position is not admitted on the strength of the other side having one. In
the current checker R1 makes that unreachable — an unranked generation is never
minted — but the rule is written closed anyway, and both directions are proved
in Lean (`an_unranked_acquisition_is_refused`,
`an_unranked_held_generation_refuses_every_acquisition`).

### The nesting admission, and why it is gated on ranks

Admitting a nested `with lock` is what makes CT33 writable, and it is admitted
**only when the module declares a rank**. The rule is: *you may nest exactly
where you have declared in which order.* Without a declared rank there is no
order to check the nesting against, so the existing `Q622` represented refusal
stands untouched — which is precisely what leaves every unranked module
byte-identical, including its `Q622`.

Whether a module is ranked is decided **once, syntactically, over the whole
source** (`declares_lock_rank`), before any statement is lowered. Reading it off
the declarations seen so far would have made a nested section's admission
depend on where a declaration happened to sit in the file.

Admitting nesting does not widen guard authority. Measured and pinned:

| Interaction | Result | Why |
|---|---|---|
| Naming the enclosing guard from inside the nested section | refused `Q600` | the inner body's frame carries only the inner section; nesting buys two guards, not their conflation |
| Enclosing invariant open across the nesting | admitted | nothing in the inner section can reach the outer footprint, and the outer `close` is still required before its `unlock` |
| Enclosing guard-derived borrow live across the nesting | admitted | still keyed to the outer guard, and leaving it live at the outer section's end is still `Q615` (full cascade pinned) |
| `close g unchanged;` after a nesting | refused `Q600` | a nested acquisition is a statement, so it invalidates the decidable read-only region; the checked witness is never silently downgraded |
| T03-GUARD-JOIN's `thread scope c under g` inside a nested section | refused `Q600` | that card's condition G3 wants the joining guard to be the *only* one held, and it was argued for one held guard; two guards keep the existing `JoinWhileGuardHeld` refusal. The same join in an unnested section is still admitted, so nesting narrowed nothing and widened nothing |

## 4. Exact diagnostic vectors

Measured through the full source route (parse → elaborate → lower → core
check), diagnostics in the order the core reports them, mapped to spans.

`RANKED` below is
`let x = protected 4; let y = protected 4; let a = mutex rank 1 guards x; let b = mutex rank 2 guards y;`

| Case | Source (inside `thread scope s`) | Vector |
|---|---|---|
| **CT33 cycle** | `RANKED` + `with lock a as ga { with lock b as gb { } } with lock b as gc { with lock a as gd { } }` | `["Q624", "Q612"]` |
| **CT33 accepted control** | `RANKED` + `with lock a as ga { with lock b as gb { open gb; write y with gb; close gb; } }` | accepted; certificate records `{1: 1, 2: 2}` |
| Three-mutex cycle | ranks 1/2/3, edges `a<b`, `b<c`, then `c` outside `a` | `["Q624", "Q612"]` |
| Three-mutex, all edges ascending | same three sections, closing edge replaced by `a` outside `c` | accepted |
| Equal ranks, either direction | both at rank 7, nested either way | `["Q624", "Q612"]` each |
| Equal ranks, not nested | both sections separately | accepted |
| Self-relock, ranked | `with lock a as g1 { with lock a as g2 { } }` | contains `Q600`, and **no** `Q624` |
| Self-relock, unranked | same, `mutex guards x` | `["Q622"]` (unchanged) |
| Mixed declared/undeclared | one ranked, one unranked mutex | leads with `Q625`, then `Q600` fallout |
| Mixed, repaired | rank the second mutex too | accepted |
| Redeclared mutex name | `let m = mutex rank 1 …; let m = mutex rank 2 …;` | `["Q012"]` — a duplicate binding is a name-resolution refusal reached before any rank question |
| Malformed rank (6 spellings: missing, non-literal, negative, overflow, missing `guards`, missing resource) | — | `["Q600"]` each |
| Ranked declaration with no import | — | `["Q015"]` |

The trailing `Q612` on every refused cycle is honest fallout, not a second
opinion: the refused `Lock` mints no guard, so the section's own `Unlock` names
a dead one. It is pinned rather than filtered, so a change that started
silently releasing the guard would be caught.

At the model level the refusal is pinned structurally, not just by code:

```
LockRankViolation { event: 3, mutex: 1, rank: 1, held_guard: 10, held_mutex: 2, held_rank: 2 }
MixedLockRankDeclaration { event: Some(1), mutex: 2 }
MixedLockRankDeclaration { event: None, mutex: 3 }
```

## 5. What a passing rank check does and does not establish

It establishes exactly R2: no acquisition in the trace nests in non-increasing
declared rank order.

It is **not** a deadlock-freedom proof, and the reason is structural rather
than a caveat. This model has no blocking at all — an acquisition of a held
generation is *refused*, not parked — so the wait-for graph a deadlock argument
needs is not represented here; and waits on anything other than these mutexes
(I/O, joins, condition variables) are outside the profile entirely.

Accordingly the discipline adds **no assumption** to the certificate. A rank is
a name for a position in a declared order, not a claim about memory, layout or
the scheduler, so there is nothing to take on trust. This is a deliberate
deviation from the sketch in `docs/reports/remaining-t01/profile.md`, which
proposed a `LockRankDeclared` assumption: adding one would have suggested an
unverified premise where there is none, and would have meant editing the shared
`ConcurrencyAssumption` enum. Instead the declared ranks are recorded verbatim
on `MutexInvariantCertificate::declared_lock_ranks` and re-derived by
`recheck_mutex_invariant_certificate`; three tampered records (emptied, permuted,
extended) each refuse with `FieldMismatch { field: "declared_lock_ranks" }`.

An empty map on a certificate means the discipline was **off** — no order was
checked. A reader must not read it as "ranks were satisfied", and the test
`an_empty_rank_map_checks_no_order_at_all` pins that the very trace R2 refuses
is admitted when no rank is declared.

## 6. Lean

`QCore.Concurrency.MutexInvariant` gains a §12 layered over an **unedited**
`step`/`run`; no existing definition or theorem was touched, and **no theorem
changed by name** — all twelve are new. No new axioms: every one depends only
on `propext` (and `Quot.sound`).

The model layer mirrors the Rust: `RankMap`, `rankOf`, `heldMutexes`,
`ranksTotal` (R1), `rankAdmits` (R2), `lockEdges`, `stepRanked`, `runRanked`.
`lockEdges` is the would-block relation the model can express: an acquisition
made while holding `h` contributes the edge `(h, acquired)`.

| Theorem | Statement |
|---|---|
| `admitted_edges_increase_rank` | every edge an admitted acquisition contributes strictly increases rank |
| `a_non_increasing_acquisition_is_refused` | contrapositive: an acquisition whose edge would not increase rank is refused |
| `equal_ranks_are_never_nested` | equal ranks are refused, stated on its own so strictness is not left to be read out of `<` |
| `an_unranked_acquisition_is_refused` | fail-closed, acquired side |
| `an_unranked_held_generation_refuses_every_acquisition` | fail-closed, held side |
| `no_generation_is_acquired_while_held` | the self-loop case |
| `runRanked_edges_increase_rank` | a whole run collects only increasing edges |
| `path_rank_strictly_increases` | along any path through increasing edges, rank strictly increases from the head to every later element |
| **`a_rank_respecting_run_admits_no_cycle`** | **the soundness claim**: no path through a rank-respecting run's collected edges returns to where it began |
| `stepRanked_agrees_with_step_off_lock` | off `.lock`, the layer is `step` |
| `stepRanked_refines_step` | the layer never admits what `step` refuses |
| `runRanked_refines_run` | the same, over a whole run |

Twelve decidable scenarios mirror the Rust cases: increasing nesting admitted
with its one collected edge `[(1, 2)]`, the CT33 reverse nesting refused, equal
ranks refused both ways, a released guard imposing no order, R1's partial map
refusing, a three-generation chain admitted and its closing edge refused, and
the `Q622` construct still refusing under the discipline.

All registered in `proof/AuditMutexInvariant.lean`; `check_lean_audits.py`
passes 48 audits.

## 7. Spec and grammar updates, in the same change

* `docs/QCORE_SPEC_r0_3.md` §L21: the mutex sub-band moves `Q610`–`Q623` →
  **`Q610`–`Q625`**, naming `Q624` and `Q625` and their owner.
* `docs/QCORE_SPEC_r0_3.md` §concurrency profiles: "lock ranks" is removed from
  the represented-refusal list and replaced by a normative paragraph stating
  R1, R2, the last-precondition placement, the no-assumption result, the
  explicit non-claim of deadlock freedom with its structural reason, and the
  retention of `Q622` for rank constructs outside the discipline.
* `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` G02c: the ranked production is added
  to the EBNF, the unconditional `Q622` nesting sentence is qualified, a
  CT33-RANKS bullet states the rule and its four measured nesting interactions,
  and the standing "no lock ranking in this profile" sentence is corrected.

## 8. Proposed CT33 mapping (not applied — the integrator's CT lane maps)

No CT file was edited. Proposed schema-2 rows, following CT28's `mutex_guard`
reject-mode shape exactly:

```
id: CT33
mapping_status: mapped
route: mutex_guard
harness_verdict_mode: reject
source_fixture: compiler/tests/fixtures/ct/CT33/case.qcore
evidence_target: linux
```

`semantic_profile` (proposed text):

> `source.mutex.guard.v1` with card CT33-RANKS's opt-in lock-rank discipline.
> Two generations over disjoint resources are declared at ranks 1 and 2, and the
> case writes both nestings: `b` inside `a`, then `a` inside `b`. Only the
> descending acquisition is refused, at `Q624 LockRankViolation`, and the exact
> ordered cascade `[Q624, Q612]` is pinned — the trailing `Q612` is the refused
> `Lock` minting no guard, so the refusal is shown not to have silently released
> or created one. The accepted control differs by one statement: the second
> section nests `b` inside `a` again, and checks, with the certificate recording
> the declared ranks `{1: 1, 2: 2}` verbatim. `cfg` is unassessed: this route
> emits from a checked event list and has no CFG stage. Nesting is admitted only
> because the module declares a rank; unranked, the same shape keeps its `Q622`
> represented refusal.

Stage rows (every other stage `not_applicable`, per the route's plan):

| Stage | Status | Executor | Observations |
|---|---|---|---|
| `parse` | executed | `compiler/tests/ct_corpus.rs::run_mapped_case` | 1 |
| `elaborate` | executed | same | 1 |
| `mutex_lowering` | executed | same | 1 |
| `mutex_invariant_check` | executed | same | 1 |
| `native_sanitized` | **unassessed** | — | — |
| `proof` | **unassessed** | — | — |

`native_sanitized` is proposed as `unassessed`, **not** `executed`, even though
this card does carry a sanitized native run of the equivalent nested trace
(§9): that run is at the core level in `mutex_c_emit`, not through the harness,
and the harness gate lists exactly which cases may read `executed`. Claiming it
would be exactly the overclaim the schema-2 gate exists to prevent. If the
integrator wants it, the ingredient is there — a `checked_mutex_sanitized`
variant over the accepted control would need a harness fixture and the gate
list edited deliberately.

Case source measured for the cascade above (as `entry(...)`):

```qcore
import std.concurrency;
fn main() -> unit {
    thread scope s {
        let x = protected 4;
        let y = protected 4;
        let a = mutex rank 1 guards x;
        let b = mutex rank 2 guards y;
        with lock a as ga { with lock b as gb { } }
        with lock b as gc { with lock a as gd { } }   // refused: Q624
    }
}
```

CT34 and CT46 are **out of scope** and their recorded reasons are unchanged:
CT34 needs a protection-growth form (`CreateMutex` fixes declared footprints at
creation), CT46 needs `Send`/`Sync` derivation.

## 9. Native evidence

`ranked_nested_sections_agree_between_the_reference_and_a_sanitized_native_run`
(`crates/qcore-core/tests/unit/mutex_c_emit.rs`) compiles and runs the nested
ranked trace once under ASan/UBSan/LSan and compares it line for line with the
independently computed reference execution. It asserts two generations are
initialised (`mtx_init(&m` twice) and that the nesting unwinds in reverse
acquisition order (`mtx_unlock(&m2` before `mtx_unlock(&m1`). A companion
records that the reversed nesting never reaches the backend at all: the checker
refuses it with `Q624` before emission.

This is a translation check for the selected model and inputs on one Linux
target. It establishes nothing about contention, other targets, or the
invariant predicate.

## 10. Gate

Run in this worktree at the final commit:

| Gate | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **1916 passed, 0 failed, 4 ignored** (wave-9 checkpoint: 1888 / 4 ignored; +28 from this card) |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 scripts/check_lean_audits.py` | **48 audits pass** (file count unchanged; `AuditMutexInvariant.lean` grows by 12 theorems) |
| `providers/` `cargo test --no-fail-fast` | **75 passed, 0 failed** |
| `python3 -m unittest discover -s scripts/tests` | **125 tests, OK** |

Load note: this host ran at load average 36–49 during the wave. Three tests
failed under that load and passed in isolation, none of them touched by this
card: `vp04_vp07_vp08_process_lifecycle_and_payload_budgets` (the recorded
load-flaky wall-clock budget, `elapsed < 500ms`), and the providers
conformance tests
`configured_helper_is_imported_from_its_real_pinned_compile_entry`,
`pinned_second_codebase_runs_configured_process_checks_without_a_rust_import_adapter`
and `configured_second_codebase_has_real_process_checked_source_evidence`
(Clang-heavy source imports). The numbers above are from runs where each suite
had the machine to itself; no assertion was weakened.

New tests by file: 21 in `crates/qcore-frontend/tests/source_mutex.rs`
(42 → 63), 11 in `crates/qcore-core/tests/unit/mutex_invariant.rs`, 2 in
`crates/qcore-core/tests/unit/mutex_c_emit.rs`.

The four pinned 185-unit artifacts are untouched **by construction, not
re-measured**: no C-gateway, audit, contract or schema file is in this card's
diff, and `MutexInvariantCertificate` carries no serde derives, so the new
field moves no serialized byte. The contract corpus stays 60 modules and
`qcore-schemas` is unchanged.

## 11. Boundaries, recorded not repaired

1. **A passing rank check is not deadlock freedom.** Stated in §5, in the
   module header, and normatively in the spec. The model has no blocking, so
   the claim it can make is about acquisition order only.
2. **Ranks are declared, never derived.** Nothing checks that a declared order
   is the *right* order for the program's intent; the discipline checks the
   program against the order it declared.
3. **Nesting is per-task and lexical.** The inner body cannot name the
   enclosing guard (§3). A nested section over a *third* task's guard is not
   expressible, because guards are body-local.
4. **The nesting admission is gated on rank declaration, not on rank
   necessity.** A module could declare ranks purely to unlock nesting. That is
   the intended trade — it still has to satisfy R1 and R2 — but it means "this
   module declares ranks" is not evidence that its author reasoned about lock
   order.
5. **`Q622` still covers every other rank construct.** Dynamic ranks, ranks
   propagated through calls, rank relations the model cannot decide: all
   unchanged, and pinned
   (`the_unsupported_lock_rank_construct_still_refuses_under_the_discipline`).
6. **Cross-task order is not analysed.** R2 is keyed to the acting task
   (`one_tasks_held_guard_does_not_order_another_tasks_acquisition`). Two tasks
   acquiring in opposite orders is the classic deadlock and this profile does
   not see it, because it cannot represent the wait.
7. **`LockRankDeclared` was deliberately not added** as a `ConcurrencyAssumption`
   variant, against `remaining-t01/profile.md`'s sketch. Reasoning in §5.
8. **The old `TrustedExplicitTrace` profile already had this rule.**
   `crates/qcore-core/src/concurrency.rs` `check_concurrent` refuses
   `held_rank >= rank` for its own `LockRank`. This card reproduces that rule
   inside the checked profile, where footprints, guards and share conservation
   are modelled; the two remain separate checkers and neither was edited into
   the other.
9. **Nothing is promoted.** No Cycle 19 unit, no CT mapping (this card proposes;
   the integrator maps), no contract corpus change, no `qcore-schemas` change.

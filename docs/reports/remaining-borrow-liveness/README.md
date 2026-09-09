# ALIAS-LOCALS-BORROW-LIVENESS — a `Borrowed` position gets a real precondition — 2026-09-09

Card ALIAS-LOCALS closed its record with one named follow-on
([§7](../remaining-alias-locals/README.md)):

> **`ALIAS-LOCALS-BORROW-LIVENESS`** — a `Borrowed` position has no implemented
> liveness precondition, so a read of a released identity is reported only when
> this rule's relation reaches it. Giving `Borrowed` a real precondition would
> report every such read, would need its own reachability argument before any of
> them could be an `Unsafe` claim, and would move the pinned artifacts. Not
> taken here.

This card takes it. The measured result is sharper than the follow-on predicted.

---

## 1. The headline: a false `Safe` with no aliasing in it at all

ALIAS-LOCALS §2 recorded a false `Safe` that needed *two* names and its new
definite-alias relation to surface. Measured at this card's base `bf210eb`, the
one-name form answers the same way:

```c
void w(int n) {
    struct b_handle *one;
    one = b_open(0, 0, n); if (!one) { return; }
    b_release(one);
    b_probe(one);            /* reads the identity it just destroyed */
}
```

**`Safe`, no diagnostics.**

No copy. No second declared name. No alias question of any kind — `one` is bound
to exactly one identity and the projection records it `RELEASED`. This is the
plainest use-after-free shape this projection is able to express, and it
certified. The cause is exactly the one ALIAS-LOCALS §2 named: the transfer
function's parameter loop is

```rust
ResourceParameterMode::Borrowed | ResourceParameterMode::Out { .. } => {}
```

so a `Borrowed` position asks nothing, ever. ALIAS-LOCALS could not reach this
body because its relation holds between two **distinct** declared names and
there is only one here.

It is now refused
(`a_released_identity_borrowed_through_its_own_name_no_longer_certifies`).

---

## 2. The precondition

Implemented as a decision in the new
[`crates/qcore-c-gateway/src/borrow_liveness.rs`](../../../crates/qcore-c-gateway/src/borrow_liveness.rs),
located and published by `legacy_c::require_live_borrowed_actuals`.

A `Borrowed` actual is refused **only** under a conjunction of *must* facts read
from the settled projection state:

| # | Requirement | What each refusal preserves |
|---|---|---|
| 1 | the name has a **settled denotation** — exactly one target, not `unknown`, not `may_be_null` — via ALIAS-LOCALS's own `Denotation::settled` | every scalar actual (§C04.2 admits scalars in exactly this position), every unresolved, ambiguous or possibly-null binding |
| 2 | that identity carries **no unresolved effect** | every site the projection already declined to decide |
| 3 | its joined status is **exactly `RELEASED`** — no `LIVE`, no `ESCAPED`, no `CONTAINED` bit | live identities, consumed ones, contained ones, and every mixed status |

Requirement 3 is the *must* half. `AbstractState::join` unions the status bits,
so the absence of every other bit is the statement that **no** reaching path
leaves the identity in another state. Released on one branch and live on the
other joins to `LIVE | RELEASED` and is not refused.

Three deliberate bounds, each with a reason rather than an omission:

* **Settled states only.** The pass runs *after* the custody work list is
  stable and reads `incoming[node]`. A must fact read on an intermediate iterate
  would depend on the work-list schedule — the hazard C02-P-ALIAS measured and
  recorded as `C02-P-ALIAS-LEAK-ORDER`. Reading the state *entering* the call
  also means the precondition is evaluated in the call's pre-state, so a call
  that releases one argument and borrows the same name at another position is
  not refused by this rule (`BORROW-LIVENESS-PRESTATE`, §7).
* **Plain names only.** A `BASE->FIELD` bound read and a member-address actual
  are governed by O04-READ-BOUND's `bound_read_resolves` and O04-B09B10's
  `resolved_aggregate_owner`, both of which already require `Some(&LIVE)` at
  exactly those positions. Restating it here would double-report against another
  card's channel without adding a refusal.
* **Read-only.** Nothing here writes to any `AbstractState`. `AbstractState`,
  `AbstractState::join`, `alias_facts` and `NodeAliasFacts` are **byte-identical
  to the base**, so the custody fixed point, the leak inventory and every
  per-exit residual are undisturbed by construction — `C02-P-ALIAS-LEAK-ORDER`
  is untouched, and measured so (§5).

### 2.1 It is not the may-alias relation, and cannot become it

The card's standing requirement was that the definite and may relations must not
blur. They are separate code, separate modules and separate facts: this rule
never reads a peer, a declared record tag or an alias fact, and ALIAS-LOCALS's
`alias_reads` obligations are untouched. The measured pin is C02-P's original
shape —

```c
void w(struct b_handle *a, struct b_handle *b) { b_release(a); b_probe(b); }
```

— where `b` is an uncontracted formal, so its own binding is unknown and
possibly null, requirement 1 fails, and the whole answer is byte-identical to
the base: `Unknown`, `[unknown_alias:a, unknown_alias:b/a]`
(`a_may_alias_pair_keeps_exactly_its_existing_obligation`). A *may* pair gains
nothing from this card. The Lean statement is
`a_name_the_projection_will_not_settle_is_never_refused`, with
`the_decision_reads_only_its_own_actual` for the closure.

---

## 3. The verdict class — decided first, on three measured grounds

The card required the verdict class to be decided **before** the implementation,
against hard invariant 2 (`Unsafe` only for a validated reachable violation) and
against the bar `DoubleRelease`/`ReleaseAfterEscape` actually meet. **The
decision is the `Unknown` class, published as the existing `UnknownAlias`.** It
is a decision, not a default, and the grounds are stronger than ALIAS-LOCALS §3
could state.

### 3.1 Every `Unsafe` kind names a contradiction against a *declared callee obligation*; `resource_borrowed` declares none

`classify_diagnostics` puts `Leak`, `DoubleRelease`, `ReleaseAfterEscape` and
`CustodyConflict` — and only those — into `AnalysisStatus::Unsafe`. Each names a
contradiction against something a contract asserted:

| Kind | The declared obligation it contradicts |
|---|---|
| `DoubleRelease` | `resource_released(p, n)` — "the trusted call contract **discharges the resource obligation**". Discharging a discharged obligation contradicts the declaration. |
| `ReleaseAfterEscape` | the same declaration, against a `resource_consumed` transfer that already moved the obligation away. |
| `CustodyConflict` | `resource_consumed`/containment — publishing custody that is not held. |
| `Leak` | the body's own exit obligation. |

`docs/QCORE_CONTRACT_SPEC_r0_3.md` §C04.2 states the borrowed mode in exactly
these terms:

> `resource_borrowed(p)` — The call **imposes no custody precondition** on `p`
> and does not transfer a tracked obligation through it. This is not the full
> QCore lifetime-checked borrow type.

There is no declared obligation to contradict. Publishing a violation here would
be asserting a rule the contract vocabulary does not contain, against upstream
code, across the whole corpus. The same clause also admits **scalars** in
`Borrowed` positions and nowhere else, which is independent evidence that the
mode is not a claim about a pointee at all.

### 3.2 The one mode that *does* declare a liveness precondition already answers `Unknown`

This is the finding that settles it, and ALIAS-LOCALS did not have it.
`ResourceParameterMode::Owned` is documented as "the caller must retain a known
live resource across the call", and §C04.2 as "the caller supplies the required
live custody". It is the one mode with an explicit liveness precondition, it
**is** implemented, and its checker is `legacy_c::require_live_resource`:

```rust
|| pointer.targets.iter().any(|site| {
    state.resources.get(site) != Some(&LIVE) || state.unresolved_effect_sites.contains(site)
});
if invalid {
    diagnostics.insert(LegacyDiagnostic { …, kind: LegacyDiagnosticKind::UnknownAlias { … } });
}
```

A *definitely released* `Owned` actual — a strictly stronger declared violation
than anything a `Borrowed` position can be — is reported today as
`UnknownAlias`, i.e. `Unknown`. Matching that for `Borrowed` is **consistency
with the implemented precondition of the neighbouring mode**, not a compromise.
ALIAS-LOCALS §3.1 wrote that "the vocabulary has no term for the event"; the
measured correction is that it does — `UnknownAlias` is this checker's
established term for a violated parameter liveness precondition — and this card
uses it rather than inventing a code.

### 3.3 The reachability bar is not met, and this rule requires *more* state than the bar does

Two independent measurements:

* **Reachability.** Neither this rule nor ALIAS-LOCALS establishes that the
  callee dereferences its `Borrowed` actual, and no path-feasibility argument is
  available in this projection. Hard invariant 2 requires a *validated reachable
  violation*; a released pointer passed to a function that only null-checks it is
  defined behaviour. Publishing `Unsafe` would manufacture upstream defect
  allegations, and the corpus measurement in §5 shows how many.
* **The bar itself.** `DoubleRelease` fires on `status & RELEASED != 0` — and
  since `AbstractState::join` is a bitwise union, that is a **may** fact over
  reaching paths. This rule requires the **must** fact (§2, requirement 3), so
  in the state dimension it demands strictly more than the kinds published as
  provisional `Unsafe` do. That asymmetry is proved in both directions
  (`a_refusal_satisfies_the_existing_bar`,
  `the_existing_violation_bar_is_strictly_weaker`), and it is why publishing the
  weaker class here is not a weaker measurement.

### 3.4 What the decision costs, stated plainly

Nothing available is lost. `UnknownAlias` already produces the whole consequence
this rule is entitled to: the function is never `Safe` again, its W4 custody
summary is not published, and its callers stop certifying too
(`a_refused_callee_withdraws_its_callers_certificate`). What is *not* claimed is
that any upstream body contains a defect, and that is the point.

The spec is reconciled with the implementation in the same change:
`docs/QCORE_CONTRACT_SPEC_r0_3.md` §C04.2 gains one paragraph stating the
caller-side refusal explicitly, marking it as a refusal to decide rather than a
violation claim, and naming every case it leaves alone. No diagnostic code was
added, changed or downgraded, so `docs/QCORE_SPEC_r0_3.md` §L21 is untouched.

---

## 4. Adversarial evidence

`crates/qcore-c-gateway/tests/borrow_liveness_boundary.rs` — **16** held-out
acceptance tests over C02-P's generic `b_*` API, deliberately unrelated to
ChunkIO, Fluent Bit and O04's `g_*` family. Every body was measured at the base
`bf210eb` with the rule disabled, and each test's `BASE:` line records that
answer, so every assertion is a measured movement or a measured preservation.

Each test asserts the **complete** diagnostic vector, not a filtered subset: a
rule that silently swallowed a pre-existing leak or double release while adding
its own refusal would pass a subset assertion.

### 4.1 Refused — measured movements

| Shape | BASE | Now |
|---|---|---|
| **release, then borrow the same name** (§1) | **`Safe`, no diagnostics** | `Unknown`, `[unknown_alias:one]` |
| release, then borrow at the first of two borrowed positions, the other a scalar | `Safe`, none | `Unknown`, `[unknown_alias:one]` |
| two independent released identities, each borrowed | `Unknown`, `[one/two, two/one]` | `+ [unknown_alias:one, unknown_alias:two]` |
| ALIAS-LOCALS's definite-copy fixture | `Unknown`, `[two/one]` | `Unknown`, `[unknown_alias:two, unknown_alias:two/one]` |

### 4.2 Preserved — measured non-movements

| Shape | BASE and now |
|---|---|
| **the may-alias pair** `b_release(a); b_probe(b)` on two formals | `Unknown`, `[unknown_alias:a, unknown_alias:b/a]` — the definite/may pin |
| **release on one branch only** | `Unsafe`, `[leak]` — the joined status carries `LIVE`, so not definite |
| release on one branch, **consume** on the other | `Safe`, none — dead on neither path is not dead on both |
| **re-acquisition** between the release and the borrow | `Safe`, none, certificate rechecks |
| a **consumed** identity, then borrowed (`C02-P-ALIAS-CONSUME`) | `Safe`, none |
| a **scalar** borrowed actual | `Safe`, none |
| a live identity borrowed | `Safe`, none |
| a **pointer-field** actual whose base was released | `Unknown`, `[unknown_alias:storage, unsupported]` |
| **double release** | `Unsafe`, `[double_release]` — the `Unsafe` class is untouched |
| borrow and release in one call (pre-state) | `Safe`, none |

### 4.3 Unit-level

`crates/qcore-c-gateway/src/borrow_liveness.rs` — **16** module tests covering
the positive direction, all four `Status` bits and their mixes, all five ways a
binding fails to settle, the absent-site and unresolved-effect refusals, the
non-vacuity of the `unresolved` guard, and the identity of the module's local
`RELEASED_ONLY` constant with the analyzer's flag.

The 32 ALIAS-LOCALS acceptance tests and the 15 `may_alias.rs` module tests pass
unchanged; one ALIAS-LOCALS test's *documentation* was strengthened deliberately
and its assertion, the complete obligation vector, is byte-identical.

---

## 5. Corpus — all five artifacts byte-identical, and a census that says why

### 5.1 What was run

The canonical pinned route on the shared prepared pinned clone (Fluent Bit
`66910c10a4d7eafa810b84229cf2dcf7b0f26f97`, explicit `/usr/bin/clang` 21.1.8),
following [`docs/reports/corpus-repin/provenance.json`](../corpus-repin/provenance.json):

```sh
python3 -B providers/package.py build --output build/cycle19/provider-packages-borrow
QCORE_CLANG=/usr/bin/clang \
QCORE_CHUNKIO_PACKAGE=build/cycle19/provider-packages-borrow/qcore.chunkio \
  ./target/debug/examples/cycle19_fluent_bit_libraries_audit \
  --pinned-evidence <clone>/fluent-bit-clang-source <clone>/monkey/monkey-build/compile_commands.json \
  <out>/monkey-source.json <out>/resource-diagnostics.json
```

BEFORE and AFTER were run from **one identical binary**, with the rule toggled
by a temporary switch removed before this card's commits — a stricter control
than reverting files between runs, because nothing else in the build can differ.
Both exit 0 with zero gateway failures. Transcripts:
[`logs/corpus-before.log.gz`](logs/corpus-before.log.gz),
[`logs/corpus-after.log.gz`](logs/corpus-after.log.gz).

### 5.2 Harness validation, then the result

BEFORE against the current baseline
[`docs/reports/wave12-corpus/artifacts/`](../wave12-corpus/artifacts), by the
exhaustive `diff_artifacts.py` document archived at
[`artifacts/diff-baseline-to-before.json.gz`](artifacts/diff-baseline-to-before.json.gz):

| Artifact | BEFORE vs baseline | AFTER vs BEFORE |
|---|---|---|
| `diagnostics.json` | **0 units added / changed / removed** | **byte-identical** |
| `chunk-contracts.json` | **0 results and 0 teardown rows changed** | **byte-identical** |
| `source.json` | **0 wrappers and 0 callbacks changed** | **byte-identical** |
| `contracts.json` | not compared by the tool | **byte-identical** |
| `source-effects.json` | differs — **package-identity residue only** | **byte-identical** |

Unit statuses **2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only**
before and after, and reason codes 136 `unsupported-syntax` / 1
`incomplete-obligation-coverage` / 48 null before and after. The one
differing row is the known package-identity residue class waves 9–12 already
record, and it is identical between BEFORE and AFTER, so it is not attributable
to this card.

**AFTER is byte-identical to BEFORE on all five artifacts** — measured directly,
not through the diff tool:

| Artifact | SHA-256 (both runs) | Bytes |
|---|---|---|
| `resource-diagnostics.json` | `a807b4714340509b…` | 11,148,247 |
| `monkey-source.json` | `53d55b7e1fb174de…` | 1,664,550 |
| `resource-diagnostics.chunk-contracts.json` | `c4f438e4ed3dab3c…` | 4,186,178 |
| `resource-diagnostics.source-effects.json` | `d63074fb49145ea6…` | 118,502 |
| `resource-diagnostics.contracts.json` | `d4e3d4a00032a96c…` | 8,168 |

**Because AFTER is byte-identical to BEFORE and BEFORE reproduces the wave-12
baseline, this card does not move the pin.**
`docs/reports/wave12-corpus/artifacts/` remains the current 185-unit baseline
and the AFTER set is not re-archived — it would be a byte-for-byte copy.
`C02-P-ALIAS-LEAK-ORDER` is undisturbed, which the unchanged `diagnostics.json`
digest confirms directly.

### 5.3 Measured reach: zero — and the census names the blocking conjunct

A null result is worth only its explanation, so the reach was **measured**. A
third instrumented run of the same pinned route emitted one scratch stderr line
per `Borrowed` position the rule considered, recording whether the actual was
bound at all, whether its denotation settled, its identity's status, its
unresolved flag and, in a second pass, the three fields that decide settling.
The instrumentation was removed before this card's commits, no production route
emits anything, and its own run produced artifacts **byte-identical to BEFORE**,
so it is proved inert. Aggregate: [`artifacts/census.txt`](artifacts/census.txt);
transcript: [`logs/corpus-census.log.gz`](logs/corpus-census.log.gz).

| Figure, over all 185 units | Count |
|---|---|
| `Borrowed` positions the rule considered | **267**, across **100** distinct functions |
| of those, **unbound** — every scalar actual, and every name this projection never tracks | **233** |
| of those, **bound** to something | **34**, across 15 functions |
| of the 34, **settling** on a single non-null identity | **0** |
| refusals published | **0** |

**The blocking conjunct is requirement 1, measured, not requirement 3.** All 34
bound considerations fail to settle, and every one of them fails on `unknown`
*and* `may_be_null` together; 27 of the 34 additionally have no target at all.
Not one borrowed actual anywhere in this corpus reaches a state where the status
test is even evaluated.

That is a sharper statement than "no released identity happened to be borrowed
here". It says the corpus's borrowed actuals are, without exception, values this
projection has not committed to — which is the same structural fact that keeps
137 of its 185 units `Unknown`, recorded as `C02-P-ALIAS-USES`. Widening what
the projection *binds* is what would move this rule's reach; widening the rule
would not.

The 15 functions carrying a bound borrowed actual are named in the census file;
the largest groups are `ctr_span_create`, `LZ4_decompress_generic` and five
`rd_kafka_txn_handle_*` bodies, each with two or three.

So the honest summary is: **this card closes a real and demonstrated hole (§1,
§4.1) and costs the corpus nothing measurable.** It is not evidence that the
hole is rare in C generally; it is evidence about these 185 units under this
projection's current, small committed-binding surface.

---

## 6. Lean inventory

[`proof/QCore/BorrowLiveness.lean`](../../../proof/QCore/BorrowLiveness.lean),
namespace `QCore.BorrowLiveness`, audited from the **new**
[`proof/AuditBorrowLiveness.lean`](../../../proof/AuditBorrowLiveness.lean).
**31 audited theorems, no new axiom, no placeholder, no `sorry`** — every one
depends only on `propext` and `Quot.sound`.

The model states `Status` as four independent bits with `join` a pointwise
union, which is `AbstractState::join`'s status clause with the bit arithmetic
removed, and reuses `MayAlias.settled`'s condition verbatim so the two rules
cannot drift apart on what "the projection committed" means.

* **the positive direction and its publication** —
  `a_settled_released_identity_is_refused`,
  `a_refusal_withdraws_a_certificate`;
* **every fail-closed direction** — `a_live_identity_is_never_refused`,
  `an_escaped_identity_is_never_refused`,
  `a_contained_identity_is_never_refused`,
  `an_identity_absent_from_the_state_is_never_refused`,
  `a_release_on_one_branch_only_is_never_refused`,
  `a_release_joined_with_an_escape_is_never_refused`,
  `an_unbound_name_is_never_refused`, `an_unknown_binding_is_never_refused`,
  `a_possibly_null_binding_is_never_refused`,
  `an_ambiguous_binding_is_never_refused`, `an_empty_binding_is_never_refused`,
  `an_unresolved_effect_site_is_never_refused`;
* **the definite/may separation** —
  `a_name_the_projection_will_not_settle_is_never_refused` (the formal-parameter
  shape) and `the_decision_reads_only_its_own_actual` (the closure);
* **the converse** — `a_refusal_names_a_settled_released_identity`: every
  refusal is backed by a settled binding, a clean unresolved flag and an
  exactly-released status, so the rule invents nothing;
* **the join and the must half** — `the_join_is_commutative`,
  `the_join_is_idempotent`, `a_live_incoming_path_prevents_the_refusal`,
  `a_refusal_implies_every_incoming_path_released`;
* **against the existing `Unsafe` bar** — `a_refusal_satisfies_the_existing_bar`
  and `the_existing_violation_bar_is_strictly_weaker`, the two directions of
  §3.3;
* **the published class** — `the_rule_never_certifies`,
  `a_refusal_is_never_the_violation_class`, `an_existing_violation_survives`,
  `not_firing_changes_nothing`;
* **monotonicity and non-vacuity** —
  `recording_an_unresolved_effect_only_drops_refusals`,
  `the_unresolved_guard_is_load_bearing`, `the_settled_guard_is_load_bearing`,
  `a_reacquired_identity_is_admitted_again`.

The Lean model is a model of the **decision**. That the Rust reader extracts the
bindings, the release events and the contract modes faithfully from C source is
a premise, not a theorem, exactly as in `QCore.MayAlias` and
`QCore.BorrowedActual`.

**For the integrator:** this card adds one audit file, so
`scripts/check_lean_audits.py` moves 57 → **58** audits and the acceptance
matrix's derived `validation.lean_audits` row must be regenerated. That is by
design and is the only thing this card leaves for the integrator.

---

## 7. Open boundaries

Each is a *retained* pre-existing answer or a measured bound, never a new
admission.

* **`BORROW-LIVENESS-MIXED`** (new) — an identity whose joined status is mixed
  is not decided, and the sharpest case is measured: released on one branch and
  **consumed** on the other, then borrowed, is still `Safe`
  (`a_release_joined_with_a_consumption_is_not_definite`). The rule declines
  rather than guessing, because `resource_consumed` "is not a proof of
  destruction" (§C04.2) and the projection therefore knows the identity is dead
  on one path only. Deciding it needs a path-sensitive or relational
  refinement this projection does not have.
* **`BORROW-LIVENESS-PRESTATE`** (new) — the precondition is read in the call's
  pre-state, so a single call that releases one argument and borrows the same
  name at another position is not refused here. That is the standard reading of
  a precondition and it keeps the rule independent of the parameter loop's
  order; the release side keeps its own diagnostics.
* **`BORROW-LIVENESS-PLACES`** (new) — the rule decides plain names only.
  Pointer-field bound reads and member-address actuals keep the `LIVE`
  requirements O04-READ-BOUND and O04-B09B10 already impose, measured unchanged.
  Widening this rule to those places would duplicate another card's channel.
* **`BORROW-LIVENESS-OUT`** (new) — `ResourceParameterMode::Out` shares the
  same fall-through arm and is deliberately untouched. An `Out` actual is the
  *address of a slot*, not a resource pointer, so "the identity is released" is
  not even the right question there; it needs its own analysis and its own card.
* **`BORROW-LIVENESS-ASSUMPTION`** (new, found by measurement) — the
  Boolean-field source-effect route publishes
  `assumptions_used.borrowed_parameter_is_live: true` on every certificate it
  issues (`crates/qcore-verification/src/source_effect.rs`), i.e. it *declares*
  as a premise the same-named fact this card now *decides*. It is **not
  discharged** by this card and the two are deliberately not conflated: that
  premise is about the analyzed function's own **entry** parameter, supplied by
  an unmodeled caller, whereas this rule decides the actuals an analyzed **body
  passes onward**. Recorded so the naming coincidence cannot later be read as a
  discharge.
* **`ALIAS-LOCALS-EXIT-CUSTODY`** — unchanged, and this card lands on the same
  side of it. The refusal does not set `state.unmodeled`, so
  `LegacyExitResidual::custody` can still read `Discharged` on an exit whose
  path carried one. The reasoning ALIAS-LOCALS recorded applies verbatim: the
  exit residual accounts for *sites*, and a read changes no site's disposition.
  **Cross-card:** the per-exit channel is O04-B09B10's; this card neither
  widened nor narrowed it.
* **`C02-P-ALIAS-CONSUME`** — unchanged and explicitly not closed through the
  back door: an escaped identity is never refused, in the Rust
  (`a_consumed_identity_is_not_a_released_one`) and in the Lean
  (`an_escaped_identity_is_never_refused`).
* **`C02-P-ALIAS-LEAK-ORDER`** — unchanged and undisturbed. `AbstractState`,
  `AbstractState::join`, `alias_facts` and `NodeAliasFacts` are byte-identical
  to the base and the leak inventory is measured unchanged (§5).
* **`C02-P-ALIAS-USES`** — unchanged. A position is decided only where the
  projection models a contracted call at all, which is why 137 of 185 corpus
  units are `Unknown` and why the reach of any such rule is small here.
* **Reachability is not established** (§3.3), and this is the reason for the
  published class rather than a caveat on it. No claim is made that a refused
  body contains a defect.
* **The declared premises are inherited unchanged and unproved** — declaration
  completeness and C effective types, exactly as C02, C02-P and C02-P-ALIAS
  record them.
* **`docs/reviews/r0_3/acceptance_matrix.json`, `acceptance.json`,
  `ct_manifest.json`, `AGENTS.md`, `CLAUDE.md`, `README.md` and
  `DEVELOPMENT.md` were deliberately not touched** — the integrator
  consolidates those.

---

## 8. Files changed

| Path | Change |
|---|---|
| `crates/qcore-c-gateway/src/borrow_liveness.rs` | **new** — the decision, its three-conjunct fail-closed rule and 16 module tests |
| `crates/qcore-c-gateway/src/legacy_c.rs` | `require_live_borrowed_actuals`, one read-only settled pass after the work list, and its call site. `AbstractState`, `AbstractState::join`, `alias_facts`, `NodeAliasFacts`, `transfer`, `report_leaks` and `apply_resource_action` are **byte-identical to the base** |
| `crates/qcore-c-gateway/src/may_alias.rs` | `Denotation::settled` becomes `pub(crate)` so both rules read one commitment; its doc records why. No other line changed |
| `crates/qcore-c-gateway/src/lib.rs` | the new private module |
| `crates/qcore-c-gateway/tests/borrow_liveness_boundary.rs` | **new** — 16 held-out acceptance tests |
| `crates/qcore-c-gateway/tests/may_alias_boundary.rs` | the strengthened documentation of one fixture and the module header; **no assertion changed** |
| `proof/QCore/BorrowLiveness.lean`, `proof/AuditBorrowLiveness.lean`, `proof/QCore.lean` | 31 audited theorems |
| `docs/QCORE_CONTRACT_SPEC_r0_3.md` | §C04.2 gains one paragraph stating the caller-side refusal |
| `docs/reports/remaining-borrow-liveness/` | this record, the diffs and the run transcripts |

`crates/qcore-c-gateway/src/borrowed_actual.rs`,
`crates/qcore-c-gateway/src/aggregate_store.rs` and
`crates/qcore-c-gateway/src/typed_field_access.rs` are **untouched**, so
O04-READ-BOUND's `untracked_bases` premise, O04-B09B10's endpoint identities and
C02's whitelist are unchanged by construction.

---

## 9. Gate results

Branch `claude/borrow-liveness` in an isolated worktree on base `bf210eb`
(the wave-12 consolidation), submodules initialised.

| Gate | Command | Result | Exit |
|---|---|---|---|
| Workspace tests | `cargo test --workspace --no-fail-fast` | **2,348 passed / 0 failed / 5 ignored** | 0 |
| Format | `cargo fmt --all -- --check` | clean | 0 |
| Clippy | `cargo clippy --all-targets -- -D warnings` | clean | 0 |
| Probe | `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 | 0 |
| Follow-up | `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 | 0 |
| Provider tests | `cargo test --locked --offline --manifest-path providers/Cargo.toml --workspace` | **87 passed / 0 failed** | 0 |
| Provider packages | `providers/package.py build` then `verify` on all five | built and verified | 0 |
| Lean audits | `python3 scripts/check_lean_audits.py` | **58 audits passed** | 0 |
| Python tests | `python3 -m unittest discover -s scripts/tests` | 125 run, **1 failed by design** (below) | 1 |
| Pinned 185-unit corpus | canonical pinned route, §5 | re-measured, **all five artifacts byte-identical** | 0 |

The +32 workspace tests are 16 module tests in `borrow_liveness.rs` and 16
held-out acceptance tests in `borrow_liveness_boundary.rs`.

### 9.1 The one deliberate failure left for the integrator

`test_the_derived_facts_are_recomputed_rather_than_carried` fails with

```
AssertionError: Lists differ: ['9.0.0', 57] != ['9.0.0', 58]
```

This card adds one Lean audit file, so the acceptance matrix's **derived**
`validation.lean_audits` row moves 57 → 58 and must be regenerated. The test is
doing exactly its job — refusing a carried number that the tree no longer
supports — and was deliberately not weakened. This is the only thing this card
leaves failing.

### 9.2 Load-flaky target, measured rather than waved away

One full-workspace run recorded a single failure in
`the_evidence_records_the_achieved_confinement_and_its_boundary`
(`qcore-verifier-host`, P03's confined-write conformance), while this host's
1-minute load average was above 19 with several delegated corpus audits running
in parallel. Re-run in isolation on the committed tree it passes, together with
all twelve tests of `confined_process_execution`, and the authoritative
full-workspace run recorded in the table above records **0 failed**. This card's
diff cannot reach that crate. **The assertion was deliberately not weakened**,
per the standing rule and `docs/reports/remaining-p03/README.md` §9.1.
P02 `RLIMIT_NPROC` passed in every run.

### 9.1 Parallel cards in the same crate

O04-TRACE, REACH-GATE and HASH-CONTRACTS run in parallel in
`crates/qcore-c-gateway`. Recorded overlap, **not resolved here**:

* `legacy_c.rs` is touched by several of them. This card's edits are two
  localised sites — one new free function placed immediately before
  `classify_diagnostics`, and one call immediately before the settled exit pass
  — and add no new instruction, state field or CFG shape, so a textual conflict
  is unlikely outside those two anchors.
* `may_alias.rs` carries a one-word visibility change on `Denotation::settled`.
* `borrow_liveness.rs`, `borrow_liveness_boundary.rs`,
  `QCore/BorrowLiveness.lean` and `AuditBorrowLiveness.lean` are this card's
  alone.
* The §7 `ALIAS-LOCALS-EXIT-CUSTODY` interaction with O04-B09B10's per-exit
  residual channel is observed, not changed.

---

## 10. Scope and non-claims

* **No raw unit is promoted**, and no rule in this card can promote one: the
  refusal is `Unknown`-class and `the_rule_never_certifies` proves it in the
  model.
* `Safe` in the `b_*` family is `Safe` for a synthetic declared custody
  projection over trusted test assumptions. The fixtures are declared test
  assumptions, not verified facts about any library.
* The §1 false `Safe` is a defect of **this projection**, demonstrated on a
  synthetic fixture. It is not a defect allegation about any upstream code.
* This is not a use-after-free analysis for C. It refuses one shape the
  projection had already decided and had been failing to ask about.
* Projection coverage is not safety coverage; unknowns stay in the denominator.

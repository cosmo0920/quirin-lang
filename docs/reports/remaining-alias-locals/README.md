# ALIAS-LOCALS — two locals, and the definite-alias relation kind — 2026-09-09

Card C02-P-ALIAS landed the bounded may-alias refinement with one deliberate
blast-radius bound, recorded as its own first open item
([C02-P-ALIAS §7](../remaining-c02-p-alias/README.md)):

> **`C02-P-ALIAS-LOCALS`** — a pair of two **locals** is related by nothing in
> v1. […] Pinned by `two_locals_keep_the_pre_existing_answer_in_v1`, which
> asserts the complete diagnostic vector rather than a subset, so lifting the
> bound moves that test.

This card lifts it. Two same-tagged **locals** are now related exactly like any
other pair, and a third relation kind — **definite alias** — is added where the
projection's own bindings show two names denote one object.

**Nothing is promoted.** All five 185-unit artifacts are **byte-identical**
before and after; no unit and no function changed verdict; the leak count is
unchanged at 271; no diagnostic code was added, changed or downgraded; no
fail-closed gate was weakened.

The measured corpus reach of the lift is **zero**, and §4 says exactly why,
from a census rather than from an argument.

---

## 1. What moved

Implemented in [`crates/qcore-c-gateway/src/may_alias.rs`](../../../crates/qcore-c-gateway/src/may_alias.rs)
and four lines of [`legacy_c.rs`](../../../crates/qcore-c-gateway/src/legacy_c.rs).
The pre-pass architecture is untouched: `alias_facts` is byte-identical,
`AbstractState` and `AbstractState::join` are byte-identical, and nothing this
card added writes to the custody state. `C02-P-ALIAS-LEAK-ORDER` is therefore
undisturbed by construction, and the leak count is measured unchanged (§4.2).

### 1.1 The relation

| | v1 | now |
|---|---|---|
| distinct declared names | required | unchanged |
| both exactly `struct TAG *`, same `TAG` | required | unchanged |
| **at least one a formal parameter** | **required** | **removed** |

That third premise was a blast-radius choice, never a soundness argument, and
it was *launderable*. The card's own measured shape

```c
void w(struct b_handle *p, struct b_handle *q) { b_release(p); b_probe(q); }
```

is refused by v1. Written through two locals —

```c
void w(struct b_handle *p, struct b_handle *q) {
    struct b_handle *one; struct b_handle *two;
    one = p; two = q; b_release(one); b_probe(two);
}
```

— v1 related `one` to `p` and `two` to `q` but never `one` to `two`, nothing
fired, and the body **certified `Safe`**. Two assignments bought back the
answer the card exists to withdraw. Measured at `67961f1`; now refused with
`two/one` (`two_locals_laundering_the_measured_shape_no_longer_certify`).

`MayAliasClasses` no longer reads the formal set at all. The caller-held half of
the question is answered where it belongs — by the provenance discharge, against
the entry sites the projection itself records.

### 1.2 The local-specific discharges, as the card required

| local-specific case | how it is decided | new machinery |
|---|---|---|
| a local bound from a **body acquisition** | the existing provenance discharge: one live identity this body acquired | **none needed** |
| a local **copied from a formal** | inherits the formal's class by declared type; the *binding* is the formal's, so provenance sees the caller-held entry site and refuses | **none needed** |
| a local from an **untyped source** | refuses as before: an unbound/unknown/possibly-null binding is never provenance (`C02-P-ALIAS-UNTYPED` untouched) | **none needed** |

The first is load-bearing rather than decorative: without it the lift would
refuse every two-acquisition body in the corpus. It is pinned by
`a_local_bound_from_its_own_acquisition_is_discharged` and by
`a_local_rebound_after_the_copy_is_discharged_by_provenance`.

### 1.3 The third relation kind — definite alias

`may_alias::definitely_aliased` decides, from the projection's own bindings at
the read, whether the read place and the peer denote **one** object: both bound,
neither `unknown`, neither `may_be_null`, and the same single target site
(`Denotation::settled`). Every way the projection declines to commit fails
closed.

It is **not a discharge**; it is a strengthening, and it **refines the peer
relation rather than widening it** — two names the declared types do not relate
stay unrelated however definite their bindings are, so `C02-P-ALIAS-UNTYPED` is
not quietly lifted through the back door.

Its one effect is that the pointer-inequality guard cannot discharge such a
pair. A source-level `one != two` asserts the two names are distinct while the
projection has bound them to one identity; refusing on that contradiction is
strictly the more refusing of the two answers, and it is the only one that does
not let a source comparison overrule the projection's own bindings. Pinned by
`a_pointer_inequality_guard_does_not_discharge_a_definite_alias`, with its
measured control `a_guard_still_discharges_two_locals_that_are_not_a_definite_alias`
— the identical guarded shape over a non-definite pair still certifies, so the
refusal is the definite alias and not the shape.

---

## 2. The false `Safe` this surfaces

The sharpest measured movement of the card is not an aliasing question at all:

```c
void w(int n) {
    struct b_handle *one; struct b_handle *two;
    one = b_open(0, 0, n); if (!one) { return; }
    two = one;
    b_release(one);
    b_probe(two);            /* reads an identity the projection records RELEASED */
}
```

Measured at `67961f1`: **`Safe`, no diagnostics.** Two reasons compose. `one` and
`two` are both locals, so v1 related them not at all; and a `Borrowed` position
of a released identity is a **no-op** in this model — `apply_resource_action`
runs only at `Released`/`Escaped`/`Consumed` positions, and `Borrowed`/`Out`
fall through untouched (`legacy_c.rs`, the `ResourceParameterMode::Borrowed |
ResourceParameterMode::Out { .. } => {}` arm). So the projection *knew* `two`
pointed at a released site and asked nothing.

It is now refused (`a_definite_copy_read_after_release_no_longer_certifies`).

---

## 3. The verdict for a definite alias — the decision, and why

The card asked whether the existing `UnknownAlias` or a stronger existing
diagnostic is the honest verdict for the §2 shape. **The decision is
`UnknownAlias`**, on four grounds, and it is a decision rather than an omission.

**3.1 The vocabulary has no term for the event.** `LegacyDiagnosticKind` offers
`Leak`, `DoubleRelease`, `ReleaseAfterEscape`, `CustodyConflict`, `UnknownAlias`,
`Unsupported`, `SummaryConflict`. A *read* of a released identity is not a
release (`DoubleRelease`), not a release after an escape
(`ReleaseAfterEscape`), and not a residual (`Leak`). `CustodyConflict` is the
closest, but it names a conflict between a *custody action's* precondition and a
site's state, and `Borrowed` is not a custody action in this model: it has no
implemented precondition at all. Reporting it would be asserting a rule this
checker does not implement, under a code that already means something else.
`Unsupported` is the unmodeled-construct channel, and this construct is modeled.

**3.2 Hard invariant 2 forbids the class the stronger kinds belong to.**
`classify_diagnostics` puts `Leak`, `DoubleRelease`, `ReleaseAfterEscape` and
`CustodyConflict` — and only those — into `AnalysisStatus::Unsafe`, which this
project publishes as *provisional Unsafe*, i.e. a validated reachable violation.
This rule establishes neither that the callee dereferences its `Borrowed` actual
nor that the path is feasible. Publishing it as a violation would manufacture
upstream defect allegations across the corpus from a rule whose reachability is
unestablished — exactly what the invariant forbids and what every prior card in
this family refused to do.

**3.3 The refusal loses nothing available.** `UnknownAlias` already produces the
whole consequence this rule is entitled to: the function is never `Safe` again,
and the reason string names the exact `place/peer`.

**3.4 The stronger claim has an honest home, and it is a different card.**
Giving `Borrowed` positions an implemented liveness precondition — so that
*every* read of a released identity is reported, aliasing or not — is a much
larger change with its own corpus measurement and its own reachability argument.
It is recorded as the named follow-on **`ALIAS-LOCALS-BORROW-LIVENESS`** (§7)
and is deliberately not taken here.

Note what this decision does *not* soften: the definite alias still changes the
**decision** (it defeats the guard, §1.3). Only the published *claim* stays at
the same code.

---

## 4. Corpus — five artifacts byte-identical, and a census that says why

### 4.1 What was run

The canonical pinned route on the shared prepared pinned clone (Fluent Bit
`66910c10a4d7eafa810b84229cf2dcf7b0f26f97`, explicit `/usr/bin/clang`), twice
from this worktree with only `crates/qcore-c-gateway/src/` and
`crates/qcore-c-gateway/tests/` moved between the runs:

```sh
python3 -B providers/package.py build --output build/cycle19/provider-packages-alias-locals
QCORE_CLANG=/usr/bin/clang \
QCORE_CHUNKIO_PACKAGE=build/cycle19/provider-packages-alias-locals/qcore.chunkio \
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  --pinned-evidence <clone> <monkey>/compile_commands.json \
  <out>/monkey-source.json <out>/resource-diagnostics.json
```

Both exit 0 with zero gateway failures. Transcripts:
[`logs/corpus-before.log.gz`](logs/corpus-before.log.gz),
[`logs/corpus-after.log.gz`](logs/corpus-after.log.gz).

### 4.2 Harness validation, then the result

BEFORE against the current baseline
[`docs/reports/wave11-corpus/artifacts/`](../wave11-corpus/artifacts):

| Artifact | BEFORE vs baseline | AFTER vs BEFORE |
|---|---|---|
| `diagnostics.json` | **byte-identical** (`a606285bbeb11b2b…`) | **byte-identical** |
| `chunk-contracts.json` | **byte-identical** (`c4f438e4ed3dab3c…`) | **byte-identical** |
| `source.json` | **byte-identical** (`53d55b7e1fb174de…`) | **byte-identical** |
| `contracts.json` | differs — path-literal provenance residue | **byte-identical** |
| `source-effects.json` | differs — package-identity residue | **byte-identical** |

The two differing rows are the two residue classes waves 9–11 already record;
both are byte-identical between BEFORE and AFTER, so neither is attributable to
this card. The exhaustive diff against the baseline is archived at
[`artifacts/diff-baseline-to-after.json.gz`](artifacts/diff-baseline-to-after.json.gz):
0 units added, 0 changed, 0 removed; unit statuses 2 Safe / 42 Unsafe /
137 Unknown / 4 data-only before and after; reason codes 136/1/48 before and
after; the only `changed` entry anywhere is the `source-effects` package-identity
digest.

| Figure | BEFORE | AFTER |
|---|---|---|
| Units | 185 | 185 |
| Unit verdicts | 2 Safe / 42 Unsafe / 137 Unknown / 4 data-only | **identical** |
| Unit and function verdict movements | — | **0** |
| `leak` diagnostics | 271 | **271** |
| `unknown_alias` diagnostics | 4,387 | **4,387** |

`C02-P-ALIAS-LEAK-ORDER` is undisturbed: the leak inventory is unchanged, which
is the observable consequence of the pre-pass architecture being untouched.

**Because AFTER is byte-identical to BEFORE and BEFORE reproduces the wave-11
baseline on all three load-bearing artifacts, this card does not move the pin.**
`docs/reports/wave11-corpus/artifacts/` remains the current 185-unit baseline,
and the AFTER set is not re-archived — it would be a byte-for-byte copy of it.

### 4.3 Measured reach: zero, and the census that explains it

A null result is only worth as much as its explanation, so the reach was
measured rather than argued. C02-P-ALIAS §4.6 left exactly this open:

> No census of how many bodies the *relation* reaches without firing was taken;
> that is recorded as an open item.

**This card closes that open item.** A third, instrumented run of the same
pinned route emitted two scratch lines — `ALIASREL` at CFG construction (every
may-alias pair a modeled read carries, classified `v1` if either name is a
formal and `locals` if neither) and `ALIASDEC` at `require_unaliased_reads` (the
per-consideration decision state). Both were removed before this card's commits;
no production route emits anything. The aggregate is archived at
[`artifacts/census.txt`](artifacts/census.txt), the transcript at
[`logs/corpus-census.log.gz`](logs/corpus-census.log.gz).

| Figure, over all 185 units | Count |
|---|---|
| may-alias pairs carried at a modeled read (emissions) | 93 — **75 `v1`, 18 `locals`** |
| distinct `(place, peer)` name pairs carried | 21 — 15 `v1`, **6 `locals`** |
| distinct `(function, place, peer)` considered at a read | 26 — 20 `v1`, **6 `locals`** |
| considerations whose peer **was released** on the reaching path | **1** — `rd_kafka_new`, `app_conf/conf` |
| guard facts present at a considered read | **0** |
| definite aliases present at a considered read | **0** |
| obligations published | **1** — the v1 one, unchanged |

The six pairs the lift adds are, exactly:

| Function | Pairs added by the lift |
|---|---|
| `decode_exponential_histogram_data_point` | `positive/negative`, `negative/positive` |
| `ut_tq_test` | `e/_aft`, `e/_elm`, `e/insert_after`, `e/_last` |

**Why none fires:** in neither function is any of those local names released on a
path reaching a modeled read. The rule is triggered by a release, not by a
declaration, so a related pair with no release between it is silent by design.
Only one consideration in the whole corpus has a released peer, and it is the
`v1` pair C02-P-ALIAS already reported.

Two further measured facts bound what this corpus can say:

* **the guard is never present at a considered read (0 of 26)**, so the
  definite-alias exception has nothing to bite on here. Its evidence is the
  held-out fixtures of §6.1, not the corpus, and that is stated rather than
  implied.
* the relation's reach is small for a structural reason this census makes
  concrete: `alias_reads` is emitted only at a **modeled contracted-call
  position** or an admitted opaque field read (`C02-P-ALIAS-USES`), and this
  corpus has few contracted calls — which is also why 137 of its 185 units are
  `Unknown`. Widening the *uses* is what would move the reach, not widening the
  relation. That is recorded, not attempted.

So the honest summary is: **the lift closes a real and demonstrated hole (§1.1,
§2) and costs the corpus nothing measurable.** It is not evidence that the hole
is rare in C generally; it is evidence about these 185 units under this
projection's current, small modeled-read surface.

---

## 5. Lean inventory

[`proof/QCore/MayAlias.lean`](../../../proof/QCore/MayAlias.lean), namespace
`QCore.MayAlias`, audited from the **existing**
[`proof/AuditMayAlias.lean`](../../../proof/AuditMayAlias.lean). **41 → 51
audited theorems, no new axiom, no placeholder, no `sorry`** — every one depends
only on `propext`, `Classical.choice` and `Quot.sound`.

**No audit file was added**, so the audit count stays 53 and the acceptance
matrix's derived `validation.lean_audits` row does **not** move. Nothing is left
failing for the integrator on that account.

### 5.1 Retired by name

Three v1 theorems are retired because the model they described no longer exists
— `Classes` has no `formal` field, matching the Rust:

| Retired | Why | Replaced by |
|---|---|---|
| `two_locals_are_never_peers` | the bound this card lifts; its exact inverse now holds | `two_locals_of_one_class_are_peers` |
| `two_same_tagged_formals_are_peers` | an instance of the general positive direction | `two_same_tagged_names_are_peers` |
| `a_formal_and_a_local_are_peers` | likewise | `two_same_tagged_names_are_peers` |

### 5.2 Added

* **the lifted relation** — `two_same_tagged_names_are_peers` (the whole
  positive direction), `two_locals_of_one_class_are_peers` (both directions);
* **the definite-alias kind** —
  `two_names_bound_to_one_identity_are_definitely_aliased`,
  `two_names_bound_to_different_identities_are_not`,
  `the_definite_relation_is_symmetric`,
  `an_uncommitted_binding_is_never_a_definite_alias`,
  `an_unbound_name_is_never_a_definite_alias`,
  `a_definite_alias_defeats_the_guard`,
  `the_guard_still_discharges_a_non_definite_pair` (the direction that proves no
  existing discharge was weakened),
  `a_released_definite_alias_refuses_under_a_guard` (the composite shape §1.3
  measures),
  `the_definite_alias_exception_is_load_bearing` (non-vacuity: without it the
  exception theorem would hold of a rule that never found a definite alias),
  `a_definite_alias_of_untyped_names_still_refuses_nothing` (the kind refines,
  never widens);
* **monotonicity of the new discharge** —
  `rebinding_only_drops_the_guard_discharge`, needed because `guardDischarges`
  now has two conjuncts and both the rebinding and the join proofs consume it.

Everything else in the module is unchanged in statement, including the whole
provenance family, the join family, `the_rule_never_promotes` and the two
existing load-bearing witnesses.

The Lean model is a model of the **decision**. That the Rust reader extracts the
declared types, the release events and the bindings faithfully from C source is
a premise, not a theorem, exactly as in `QCore.BorrowedActual`.

---

## 6. Adversarial evidence

`crates/qcore-c-gateway/tests/may_alias_boundary.rs` — 22 → **32** held-out
acceptance tests over the generic `b_*` API, deliberately unrelated to ChunkIO,
Fluent Bit and O04's `g_*` family. Every new body was measured at `67961f1`
first and its `BASE:` line records that answer, so every assertion is a measured
movement or a measured preservation.

### 6.1 Refused — each moved `Safe` → `Unknown`

| Shape | BASE | Obligation now |
|---|---|---|
| the measured shape laundered through two locals (`one = p; two = q;`) | `Safe` | `two/one` |
| two local copies of **one** formal (`one = a; two = a;`) | `Safe` | `two/one` |
| a **definite copy read after release** (§2) | `Safe` | `two/one` |
| a two-link chain of copies (`mid = one; two = mid;`) | `Safe` | `two/one` |
| a guarded definite alias — the guard does **not** discharge | `Safe` | `two/one` |
| **the moved v1 pin**: two locals bound from one field place | `Unknown`, `[unknown_alias one]` | `[unknown_alias one, unknown_alias two/one]`, complete vector asserted |

### 6.2 Untouched — each keeps its exact answer

| Shape | BASE | Now |
|---|---|---|
| two locals of **different** declared classes | `Safe` | `Safe`, no obligation |
| two locals, two distinct body-made acquisitions (provenance) | `Safe` | `Safe` |
| copy-then-rebind to a fresh acquisition before the read | `Safe` | `Safe` |
| **consumed** then the copy read (`C02-P-ALIAS-CONSUME`, retained) | `Safe` | `Safe` |
| a guarded pair of locals that is **not** a definite alias | `Safe` | `Safe` |
| every v1 fixture — all 22 | as recorded | **unchanged** |

The moved pin keeps its load-bearing half: the *complete* diagnostic vector is
still asserted, so the ordinary `UnknownAlias one` the unresolvable release
already produced is still required to be there and nothing can quietly vanish.

### 6.3 Unit-level

`crates/qcore-c-gateway/src/may_alias.rs` — 10 → **15** module tests: the class
relation's symmetry, irreflexivity and distinct-tag/untyped refusals unchanged;
`two_locals_of_one_class_are_peers` as the exact inverse of the retired v1
assertion; `two_locals_of_different_classes_are_still_unrelated` as the negative
half that survives the lift; and four definite-alias tests covering both
directions, symmetry/reflexivity, and all four ways an uncommitted binding fails
closed (unknown, ambiguous, unbound, possibly-null).

`crates/qcore-c-gateway/tests/unit/legacy_c.rs`'s seven pre-pass tests are
unchanged and pass: `alias_facts` was not touched.

### 6.4 Independent rederivation

`MayAliasClasses::read` has exactly one call site, `build_function_cfg`, which is
what W4 derivation, W4 validation and `check_legacy_safety_certificate` all use.
The lifted relation is therefore reproduced from the certificate's own
preprocessed source by construction, and C02-P-ALIAS's two substitution tests
(record-tag substitution, guard-spelling substitution) pass unchanged.

---

## 7. Open boundaries

Each is a *retained* pre-existing answer, never a new admission.

* **`ALIAS-LOCALS-BORROW-LIVENESS`** (new, §3.4) — a `Borrowed` position has no
  implemented liveness precondition, so a read of a released identity is
  reported only when this rule's relation reaches it. Giving `Borrowed` a real
  precondition would report every such read, would need its own reachability
  argument before any of them could be an `Unsafe` claim, and would move the
  pinned artifacts. Not taken here.
* **`ALIAS-LOCALS-EXIT-CUSTODY`** (new, recorded not repaired) — an
  `UnknownAlias` obligation makes the *function* `Unknown` but does not set
  `state.unmodeled`, so `LegacyExitResidual::custody` can still read
  `Discharged` on an exit whose path carries an outstanding obligation. Measured
  as pre-existing and left alone, on the argument that the two channels state
  different properties: the exit residual accounts for *sites*, and a read
  changes no site, so the custody accounting is not what the obligation
  withdraws. Nothing is promoted either way, because the function status is
  `Unknown`. **Cross-card:** the per-exit channel is O04-B09B10's; this card
  neither widened nor narrowed it, and the interaction is recorded rather than
  resolved.
* **`C02-P-ALIAS-UNTYPED`** — unchanged, and explicitly *not* lifted through the
  definite-alias kind (§1.3, `a_definite_alias_of_untyped_names_still_refuses_nothing`).
* **`C02-P-ALIAS-CONSUME`** — unchanged. A `Consumed` argument transfers custody
  away and a later read of a definite copy is exposed in much the same way;
  `released_names` still records only `Released`. Measured retained
  (`a_consumed_argument_still_relates_nothing`).
* **`C02-P-ALIAS-USES`** — unchanged. A read is related only at a modeled
  contracted-call position or an admitted opaque field read.
* **`C02-P-ALIAS-GUARD-FORMS`** — unchanged. The guard is recognised only on an
  `if`, only as `X != Y` / `X == Y` between two plain related names.
* **`C02-P-ALIAS-LEAK-ORDER`** — unchanged, and undisturbed: `alias_facts`,
  `AbstractState` and `AbstractState::join` are byte-identical and the leak
  count is measured unchanged at 271.
* **The definite-alias kind is not an alias analysis.** It reads the
  projection's own bindings; it discovers no equality the projection did not
  already record, and it establishes no *inequality* anywhere.
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
| `crates/qcore-c-gateway/src/may_alias.rs` | `peers` drops the formal premise; `MayAliasClasses` drops the `formals` field and `read` its parameter; new `Denotation` + `definitely_aliased`; 15 module tests |
| `crates/qcore-c-gateway/src/legacy_c.rs` | the single `MayAliasClasses::read` call site; new `definitely_aliased` and `distinct_by_guard` beside `distinct_by_provenance`; `require_unaliased_reads` consults the guard through `distinct_by_guard`. `alias_facts`, `NodeAliasFacts`, `released_names`, `rebound_names`, `AbstractState` and `AbstractState::join` are **byte-identical to the base** |
| `crates/qcore-c-gateway/tests/may_alias_boundary.rs` | the moved v1 pin plus 10 new held-out tests |
| `proof/QCore/MayAlias.lean` | the lifted relation, the definite-alias kind, 51 theorems |
| `proof/AuditMayAlias.lean` | the new names; three retirements recorded in its own header |
| `docs/reports/remaining-alias-locals/` | this record, the census, the diff and the run transcripts |

`crates/qcore-c-gateway/src/borrowed_actual.rs`,
`crates/qcore-c-gateway/src/aggregate_store.rs` and
`crates/qcore-c-gateway/src/typed_field_access.rs` are **untouched**, so
O04-READ-BOUND's `untracked_bases` disjointness premise and C02's whitelist are
unchanged by construction.

---

## 9. Gate results

Branch `claude/alias-locals` in an isolated worktree on base `67961f1`
(the wave-11 consolidation), submodules initialised.

| Gate | Command | Base | After | Exit |
|---|---|---|---|---|
| Workspace tests | `cargo test --workspace --no-fail-fast` | 2,119 passed / 4 ignored | **2,134 passed / 0 failed / 4 ignored** | 0 |
| Format | `cargo fmt --all -- --check` | — | clean | 0 |
| Clippy | `cargo clippy --all-targets -- -D warnings` | — | clean | 0 |
| Probe | `cargo run -p qcore-compiler --example reevaluation_probe` | — | exit 0 | 0 |
| Follow-up | `cargo run -p qcore-compiler --example reevaluation_followup` | — | exit 0 | 0 |
| Provider tests | `cargo test --locked --offline --manifest-path providers/Cargo.toml --workspace` | 76 | **76 passed / 0 failed** | 0 |
| Lean audits | `python3 scripts/check_lean_audits.py` | 53 | **53 audits passed** | 0 |
| Python tests | `python3 -m unittest discover -s scripts/tests` | 125 | **125 passed, 0 failed** | 0 |
| Pinned 185-unit corpus | canonical pinned route, §4 | reproduced byte-identically | re-measured, **byte-identical** | 0 |

The +15 workspace tests are 5 module tests in `may_alias.rs` (10 → 15) and 10
held-out acceptance tests in `may_alias_boundary.rs` (22 → 32).

**Nothing is left failing for the integrator.** No Lean audit file was added, so
`test_the_derived_facts_are_recomputed_rather_than_carried` passes — the
acceptance matrix's derived `validation.lean_audits` row stays at
`['8.0.0', 53]` and needs no regeneration on this card's account. The
integrator-owned files listed in §7 are untouched.

The load-flaky targets recorded in
[`docs/reports/remaining-p03/README.md`](../remaining-p03/README.md) §9.1 (P02
`RLIMIT_NPROC`, P03 confined-write, `vp04`) all passed in the runs above; no
re-run in isolation was needed.

### 9.1 Parallel cards in the same crate

W4-EXPR-POS and O04-B09B10 ran in parallel in `crates/qcore-c-gateway`. Recorded
overlap, **not resolved here**:

* `legacy_c.rs` is touched by all three. This card's edits are four small,
  localised sites (§8) and add no new instruction, state field or CFG shape, so
  a textual conflict is likely only in `require_unaliased_reads` and around
  `distinct_by_provenance`.
* O04-B09B10 owns the per-exit residual channel, which §7's
  `ALIAS-LOCALS-EXIT-CUSTODY` observes but does not change.
* `may_alias.rs`, `AuditMayAlias.lean` and `QCore/MayAlias.lean` are this card's
  alone.

---

## 10. Scope and non-claims

* **No raw unit is promoted**; no unit and no function verdict moved in either
  direction, anywhere in the 185-unit corpus, and all five artifacts are
  byte-identical.
* `Safe` in the `b_*` family is `Safe` for a synthetic declared custody
  projection over trusted test assumptions. The fixtures are declared test
  assumptions, not verified facts about any library.
* The §2 false `Safe` is a defect of **this projection**, demonstrated on a
  synthetic fixture. It is not a defect allegation about any upstream code, and
  the corpus measurement found no instance of it.
* Projection coverage is not safety coverage; unknowns stay in the denominator.
* CT coverage is unchanged at 31/50 mapped; the contract corpus remains 60
  modules; `qcore-schemas` is unchanged at 8.0.0.

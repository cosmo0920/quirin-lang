# CT34-GROWTH — a protection-growth source form, and the rule it is sound under

Wave 12. Card CT34-GROWTH: decide whether growing a mutex's protected footprint
is sound in `CheckedMutexInvariantV1`, and if so give it a checked existence and
a source form, so CT34's scenario can be written down.

Base: `main` at `67961f1`. All work is on this card's branch.

## 1. The model argument, first

### 1.1 What the ledger actually conserves

`CheckedMutexInvariantV1` carries one arithmetic invariant, checked before the
first event and after every event, refused events included
(`ShareLedger::unconserved`). For every live atom `a` — an indivisible span
between two coordinates the program itself mentions —

```
sum over tasks of task_shares[a]  +  (FULL_ACCESS if a is parked else 0)  ==  initial_total[a]
```

and one structural invariant, `parked: Atom -> MutexId`, a *map*: an atom has at
most one parking generation. Two further facts hold by construction and are the
reason the arithmetic is enough:

* `overcommitted()` refuses an entry state where any atom's total exceeds
  `FULL_ACCESS`. So `authority(t, f) == FULL_ACCESS` implies **no other task
  holds any share of `f`** — full authority is exclusive, not merely maximal.
* an atom is parked **iff** some live generation's `protects` covers it: parking
  happens only at `CreateMutex` and unparking only at `DestroyMutex`.

`CreateMutex` is sound against both because it performs a *move*: it checks
`authority(task, f) == FULL_ACCESS` and `!parked(a)` for every atom of every
declared interval, then `park` removes exactly the creator's `FULL_ACCESS` and
installs exactly one parked `FULL_ACCESS`. The sum is unchanged; the map gains
one entry it did not have.

### 1.2 The growth question

Growth performs the identical move on a further interval. So the conservation
equation and the single-protector map hold **for exactly the same reason** —
nothing about that argument depended on the move happening at creation time.
That is the whole of the arithmetic case, and it closes.

What does *not* transfer for free is everything `CreateMutex` gets from
happening before the generation exists. A mid-trace shape change can disturb
four things a creation cannot, and each is what one condition of the rule is
for. Derived, not assumed:

**W1 — exclusivity and publication.** At creation nobody can be inside the
mutex, because it does not exist. Mid-trace, another task can be. The model has
one holder (`MutexState::holder: Option<GuardId>`, and `Lock` refuses a held
generation), so **requiring the growing task to hold a live guard over the
generation** buys exactly the same exclusivity: no other task is inside the
section while the shape changes. It also buys *publication*: `Unlock` stores the
releasing clock and `Lock` merges it, so a change made inside the section is
ordered before every later acquisition by the mutex's own release/acquire edge.
Without W1 a concurrent holder's guard would silently widen mid-section, and the
data-race check would be left to catch it after the fact.

The event therefore names only the **guard**; the generation is read off it
(`guards[guard].mutex`). A growth cannot name a mutex its holder does not hold,
so W1 is structural rather than a comparison that could be got wrong.

**W2 — the invariant window.** `InvariantPhase::Closed` on a section means the
declared predicate holds of the protected region. Growing under a closed
invariant would extend that claim to memory nothing established it for, silently.
`Open` is precisely the window in which the predicate need not hold, so the
shape change belongs there — and requiring it *forces* the trace through a
`CloseInvariant` before it can `Unlock` or `ErrorExit`, both of which already
demand a closed invariant. This is load-bearing, not hygiene:
`grow_requires_an_open_invariant` and `a_grown_section_cannot_be_released_before_closing`
are the two halves.

**W3 — the predicate identity.** A wider region is a different proposition. If
the generation kept its `InvariantId`, a post-growth
`Declared { invariant: I }` would look identical to the pre-growth one on the
certificate, and the `InvariantPredicateDeclared` premise would have been
widened without a reader being able to see it. So a growth **supplies a new
identity, which must be one the generation has never carried** (a per-generation
`invariant_history`, so a grow-and-revert is refused too). And because growth
changes what the region *is*, it counts as a write while the invariant is open:
the **checked** `NoWriteOccurred` witness — whose content is "the invariant is
unchanged" — becomes unavailable to a section that grew, which must close by
declaration.

That last point reuses `writes_while_open` rather than adding state. It is
accurate for both purposes that field serves: the checked witness, and the
invalidation of guard-scoped facts overlapping the change.

**W4 — one protector.** The new interval must overlap no live generation's
declared shape, this generation's included, and none of its atoms may already be
parked. Those are the same fact from the interval side and the ledger side (an
atom is parked iff a live generation covers it), so the second is redundant
today; both are checked so a later change to either representation cannot open a
hole on its own — the same discipline T03-GUARD-JOIN's G9/G10 follow.

**W5 — authority provenance.** Full authority over the whole interval, because
parking is all-or-nothing exactly as at creation. *And* the grower must be the
generation's creator. That second half is the one condition the card's sketch
did not name, and it is `DestroyMutex`'s own rule read forwards: destruction
unparks everything to the **creator** (`ledger.unpark(*task, *mutex)` guarded by
`state.creator != *task`). A non-creator grower would park share that arrives,
at destruction, at a task which never gave it up — a transfer of authority
between tasks with no synchronization witness, which is exactly what this
profile refuses everywhere else. Per-atom parker provenance would be the general
repair; requiring the creator is the narrow one, and it is what landed.

### 1.3 Permanence

There is no shrink and no replacement event, so growth is permanent for the
generation by construction, and the grown intervals are released with the
declared ones at destruction. **Replacing** a protected identity — the other
half of CT34's scenario — is therefore not admitted at all: it can only be
attempted as a growth onto memory something already protects, which W4 refuses.

### 1.4 What growth does not establish

That the newly protected memory satisfies the new predicate (that is the same
`InvariantPredicateDeclared` premise, now attached to a named new identity);
that the C layout of the grown interval is what was declared
(`DeclaredFootprintShape`, unchanged); anything about shrinking, replacing or
re-pointing a protected identity; and nothing at all about deadlock, contention
or the scheduler. The rule adds **no new assumption** to the certificate: every
condition is checked.

## 2. The admission

`MutexInvariantEvent::ExtendProtection` is untouched: it still refuses
unconditionally at `Q618` with the generation live and unchanged. Beside it,
`GrowProtection { task, guard, footprint, invariant }` is the opt-in admitted
growth. Zero checker lines were deleted. A trace using neither event is
byte-identical to before, and a module that never writes `grow` lowers exactly
as it did, its declared predicate identities included (pinned by
`the_fixed_shape_refusal_is_untouched_by_the_growth_form`).

Refusal grounds, and the code each carries:

| Ground | Diagnostic | Code |
|---|---|---|
| acting task inactive / unknown | `TerminalTask` / `UnknownTask` | `Q600` |
| guard not live | `DeadGuard` | `Q612` |
| guard not held by the acting task (W1) | `WrongGuardAffinity` | `Q612` |
| generation absent (unreachable; blocks rather than admits) | `UnknownMutex` | `Q600` |
| invariant not open (W2) | **`ProtectionGrowthOutsideOpenInvariant`** | **`Q626`** |
| malformed interval | `InvalidFootprint` | `Q600` |
| destroyed resource | `DeadResource` | `Q600` |
| interval already protected or parked (W4) | `FootprintAlreadyProtected` | `Q617` |
| grower is not the creator (W5) | `MutexAuthorityNotConserved` | `Q621` |
| grower lacks full authority (W5) | `MutexAuthorityNotConserved` | `Q621` |
| predicate identity already carried (W3) | **`ProtectionGrowthInvariantUnchanged`** | **`Q627`** |
| a grown section closing `NoWriteOccurred` (W3) | `InvalidInvariantWitness` | `Q613` |

Two new codes were taken from the free band immediately above the profile's own,
under the §L21 process, with the spec updated in the same change (§6). The other
nine grounds deliberately reuse existing codes, because they are existing
obligations: `Q617` already means "this interval has a protector", and `Q621`
already means "parked authority is not conserved or would be misrouted", which
is precisely the non-creator argument. Reusing `Q621` for both W5 halves means
the two are indistinguishable in a diagnostic vector; that is recorded as a
boundary (§9.1) rather than papered over with a third code.

On success the checker parks the atoms, appends the interval to `protects`,
moves `invariant` and its history, records the growth, and marks the section as
having changed the region.

## 3. The source form

```ebnf
inner = ... | "grow" , NAME , "guards" , NAME , ";"
```

`grow g guards y;`, admitted only inside a critical section, naming that
section's own guard. One new reserved word, `grow`; `guards` was already
reserved by the mutex declaration.

### Reserved-word collision audit

| Check | Result |
|---|---|
| `grow` as an identifier in the 332 `.qc`/`.qcore` files in the tree | **0 occurrences** |
| `grow` in QCore source embedded in Rust string fixtures | one — `extern fn grow() -> unit contract {…}` in `crates/qcore-verification/tests/composition_relation.rs`, a **declaration name**, never referenced from a statement. The reserved list filters a *statement's* referenced-name set only, so it is unaffected; the whole target passes unchanged. |
| every other `grow` in `crates/`+`compiler/` | C-fixture function names inside C source strings, and Rust prose |
| Does the reservation cost an existing spelling? | **No, measured.** `grow` is still usable as a resource name (`let grow = protected 4; … write grow with g;` checks), asserted in `reserving_grow_costs_no_existing_spelling`. |

The standing hazard T03-F recorded for its five words applies: a reserved word
is invisible to name resolution rather than illegal, so a future call site
spelled `grow(...)` would resolve vacuously. The zero-occurrence census above is
the evidence that nothing in the tree is affected today.

### The invariant identity, from source

`close g;` used to emit `Declared { invariant: <the mutex id> }`. It now emits
whatever identity the generation carries *at that point*, tracked in the
lowering (`invariant_of`), and a `grow` mints the next one from a counter that
advances at each declaration and each growth. In a module with no growth the
counter reproduces the historical `invariant == mutex` numbering exactly, which
is why every pre-existing fixture is byte-identical; that is asserted, not
argued.

## 4. Adversarial matrix

Measured through the full source route (parse → elaborate → lower → core check),
diagnostics in the order the core reports them. `GROWABLE` is
`let x = protected 4; let y = protected 4; let m = mutex guards x;`.

| Row | Body (inside `thread scope s`) | Vector |
|---|---|---|
| **accepted control** | `GROWABLE with lock m as g { open g; grow g guards y; close g; }` | accepted |
| grown interval is reachable under the guard | `… } with lock m as g2 { read x with g2; read y with g2; }` | accepted |
| **reader relying on the pre-growth footprint** | `… } read y;` | `["Q611"]` |
| unguarded read *before* the growth (control) | `GROWABLE read y;` | accepted |
| **growth without the guard** — outside any section | `GROWABLE grow x guards y;` | `["Q600"]` |
| **growth by a child of the holder** | `… with lock m as g { open g; thread scope c { grow g guards y; } close g; }` | contains `Q612` |
| **growth outside an open invariant** | `GROWABLE with lock m as g { grow g guards y; }` | `["Q626"]` |
| **growth over another generation's interval** | `GROWABLE let n = mutex guards y; with lock m as g { open g; grow g guards y; close g; }` | `["Q613","Q617","Q619","Q613","Q600","Q600","Q600"]` |
| **double growth of the same interval** | `… { open g; grow g guards y; grow g guards y; close g; }` | the same, plus one `Q600` |
| **growth over its own declared interval** | `… { open g; grow g guards x; close g; }` | the same seven |
| **checked witness after a growth** | `… { open g; grow g guards y; close g unchanged; }` | `["Q600"]` |
| checked witness with no growth (control) | `… { open g; read x with g; close g unchanged; }` | accepted |
| **rank interaction** — ranked module, growth, ascending nesting | ranks 1/2, `grow` then `a` outside `b` | accepted |
| **rank interaction** — same with the descending nesting | ranks 1/2, `grow` then `b` outside `a` | `["Q624","Q612"]` |
| growth with no `import std.concurrency;` | — | `["Q015"]` |
| malformed spellings (missing `guards`, missing resource, outside a section) | — | `["Q600"]` each |
| undeclared guard or resource name | — | `["Q013"]` each |

Model-level rows the source lowering cannot reach — it only ever produces
whole-resource growths by the root task — are in
`crates/qcore-core/tests/unit/mutex_invariant.rs`:

| Row | Result |
|---|---|
| **non-creator grower**, holding the section *and* full authority over the interval | `MutexAuthorityNotConserved { event: 4, resource: OTHER }` |
| the same growth performed by the creator (control) | accepted |
| **partial share** over the new interval (0, 1, `FULL_ACCESS - 1`) | `MutexAuthorityNotConserved` each |
| **partial-interval growth**, then a guarded access to a wider one | `UnprotectedFootprint { event: 5, guard: 10 }` |
| the same access confined to the grown interval (control) | accepted |
| a second growth re-using the creation identity, or the first growth's | `ProtectionGrowthInvariantUnchanged` each |
| a second growth with a third identity (control) | accepted; two growths recorded, shape `[x, y[0,4), y[4,8)]` |
| growth on a released guard | `DeadGuard { event: 3, guard: 10 }` |
| destroying the grown resource while the generation lives | `NonquiescentDestruction { event: 6, resource: OTHER }` |
| destroying the generation, then unguarded writes to both resources | accepted — every parked interval came back to the creator |
| a fact and a borrow over the pre-growth region, across a growth | accepted — growth adds memory and moves none |
| a fact over the interval *before* it was grown onto | `UnprotectedFootprint` |
| `ExtendProtection` in the same state | `["Q618"]`, and nothing else moves |
| refused growth (`Q626`), then `close` on the *original* identity | `["Q626"]` only — the refusal installed nothing |
| certificate tampering: emptied, widened, rehomed identity, duplicated | `FieldMismatch { field: "admitted_protection_growths" }` each |
| certificate hiding the growth from `protected_shape` | `FieldMismatch { field: "protected_shape" }` |

### 4.1 The cascade, and why it is pinned rather than filtered

Every refused growth in a section that closes carries the same fallout, and each
entry says something. The lowering mints the new predicate identity before the
core decides; when the core refuses the growth, the generation keeps its old
identity, so the section's `close` names one it never took
(`Q613 InvalidInvariantWitness`), the section therefore unlocks with its
invariant still open (`Q613 InvariantOpenAtUnlock`), and the generation and its
guard are left outstanding at scope exit (`Q619`, then `Q600`). Structurally,
for the "already protected" row:

```
InvariantOpenAtUnlock  { event: 6, guard: 1 }
FootprintAlreadyProtected { event: 4, mutex: 1 }     <- the growth's own refusal
NonquiescentMutexDestruction { event: 8, mutex: 1 }
InvalidInvariantWitness { event: 5, guard: 1 }
NonquiescentDestruction { event: 11, resource: 1 }
OutstandingGuard { guard: 1 }
OutstandingMutex { mutex: 1 }
```

A change that started mutating a generation on a refused growth would break this
vector. This is the same shape as CT33's trailing `Q612`, longer only because a
growth sits mid-section rather than at its head.

## 5. Lean

`QCore.Concurrency.MutexInvariant` gains a §16 layered over an **unedited**
`step`/`run`, exactly as §12's rank discipline and §15's guard-local join are.
No existing definition was touched and **no theorem changed by name**; all 18
are new. `MutexRecord` gains one field (`invariants`, the identity history) and
`protected_shape_is_fixed` keeps its statement and its proof — only its doc
comment now points at the separate layer. No new axioms: every theorem depends
only on `propext` (and `Quot.sound`).

| Theorem | Statement |
|---|---|
| **`growProtectionEffect_preserves_authority`** | the conserved total is untouched — growth re-custodies a holding, it does not create or destroy share (the same proof as creation's) |
| `grow_step_preserves_authority` | the same over the whole guarded step |
| **`admitted_growth_had_no_protector`** | an admitted growth's interval had no protector at all, so no footprint acquires a second one |
| `grow_preserves_every_other_declared_shape` | every other generation's record survives verbatim |
| `grow_appends_the_interval_and_moves_the_predicate` | permanence and W3's effect in one: the declared intervals survive with the new one appended, the identity moves, and the history records it |
| `grow_refuses_a_dead_guard` | W1 |
| `grow_requires_the_holder` | W1 |
| `grow_requires_an_open_invariant` | W2 |
| `grow_requires_the_creator` | W5 |
| `grow_requires_an_unused_predicate_identity` | W3 |
| `grow_refuses_an_already_protected_interval` | W4, interval side |
| `grow_refuses_a_parked_interval` | W4, ledger side |
| `grow_requires_full_authority` | W5 |
| `grow_refuses_a_destroyed_resource` | liveness |
| `refused_growth_produces_no_state` | a refusal produces no state, so nothing can move on one |
| `grow_marks_the_section_it_ran_under` | the section keeps its identity, holder and phase, and is marked as having changed the region |
| `a_grown_section_cannot_close_with_the_checked_witness` | W3, witness side |
| `a_grown_section_cannot_be_released_before_closing` | W2's consequence: growth cannot leave a section without an explicit close |

Twelve decidable scenarios mirror the Rust rows: the admitted control, W2/W3/W4/W5
refusals, an unminted guard, the conserved total across an admitted growth, the
unguarded write admitted before the growth and refused after it, and
`extendProtection` still refusing in the same state.

All registered in `proof/AuditMutexInvariant.lean`; `check_lean_audits.py`
passes **53 audits** (file count unchanged).

## 6. Spec and grammar updates, in the same change

* `docs/QCORE_SPEC_r0_3.md` §L21: the mutex sub-band moves `Q610`–`Q625` →
  **`Q610`–`Q627`**, naming `Q626` and `Q627` and their owner.
* `docs/QCORE_SPEC_r0_3.md` §concurrency profiles: the fixed-footprint sentence
  is qualified (a generation may grow under the bounded rule, and may never
  shrink or replace), and a normative paragraph states the unconditional `Q618`
  refusal, W1–W5 with their codes, the checked-witness consequence, permanence,
  the certificate obligations and the no-new-assumption result.
* `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` G02c: the `grow` production is added
  to the EBNF, the "fixes the protected footprint syntactically" sentence is
  qualified, and a CT34-GROWTH bullet states the rule, the refused shapes, the
  permanence, the refusal fallout and the reservation.

## 7. Proposed CT34 mapping (not applied — the integrator's CT lane maps)

No CT file was edited and no fixture was added under `compiler/tests/fixtures/ct/`.
Proposed schema-2 rows, following CT33's `mutex_guard` reject-mode shape exactly:

```
id: CT34
mapping_status: mapped
route: mutex_guard
harness_verdict_mode: reject
source_fixture: compiler/tests/fixtures/ct/CT34/case.qcore
evidence_target: linux
```

`semantic_profile` (proposed text):

> `source.mutex.guard.v1` with card CT34-GROWTH's opt-in protection growth.
> CT34's scenario has two halves and this case separates them: a generation may
> **grow** its protected footprint over memory nothing protects, and may never
> **replace** an identity another generation already protects. Three resources
> are declared with generations over the first two; the case grows `m` onto `y`,
> which `n` already protects, and is refused at `Q617 FootprintAlreadyProtected`.
> The accepted control differs by exactly one token — the growth names the
> unprotected `z` instead — so the refusal is attributable to the interval
> having a protector rather than to growth being written at all. The exact
> ordered cascade `[Q613, Q617, Q619, Q613, Q600, Q600, Q600]` is pinned: the
> refused growth installed nothing, so the section's `close` names a declared
> predicate identity the generation never took and the section unlocks with its
> invariant still open, leaving the generation and its guard outstanding at
> scope exit. Every entry after `Q617` shows the refusal left the generation's
> shape, its declared identity and the share ledger exactly as they were. There
> is no shrink or replacement form in this profile at all, and the unconditional
> `Q618` fixed-shape refusal beside the growth is unchanged. Only parse,
> elaborate, `mutex_lowering` and `mutex_invariant_check` execute; this route
> emits from a checked event list and has no CFG stage.

Stage rows (every other stage `not_applicable`, per the route's plan):

| Stage | Status | Executor | Observations |
|---|---|---|---|
| `parse` | executed | `compiler/tests/ct_corpus.rs::run_mapped_case` | 1 |
| `elaborate` | executed | same | 1 |
| `mutex_lowering` | executed | same | 1 |
| `mutex_invariant_check` | executed | same | 1 |
| `native_sanitized` | **unassessed** | — | — |
| `proof` | **unassessed** | — | — |

`native_sanitized` is proposed **unassessed**, not `executed`: this card carries
no sanitized native run at all (§9.3), and the schema-2 gate lists exactly which
cases may read `executed`. `proof` is `unassessed` for the same reason every
other `mutex_guard` case is: the Lean work is a model-level layer, not a
refinement of this fixture.

Case source, measured for the cascade above:

```qcore
import std.concurrency;
fn main() -> unit {
    thread scope s {
        let x = protected 4;
        let y = protected 4;
        let z = protected 4;
        let m = mutex guards x;
        let n = mutex guards y;
        with lock m as g { open g; grow g guards y; close g; }  // refused: Q617
    }
}
```

Accepted control (`case.qcore` with `y` replaced by `z` on the growth line):

```qcore
        with lock m as g { open g; grow g guards z; close g; }  // accepted
```

CT46 is out of scope and its recorded reason is unchanged: it needs `Send`/`Sync`
derivation.

## 8. Gate

Run in this worktree at the final commit:

| Gate | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **GATE_WORKSPACE** |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | GATE_PROBE1 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | GATE_PROBE2 |
| `python3 scripts/check_lean_audits.py` | **53 audits pass** (file count unchanged; `AuditMutexInvariant.lean` grows by 18 theorems) |
| `providers/` `cargo test --no-fail-fast` | GATE_PROVIDERS |
| `python3 -m unittest discover -s scripts/tests` | GATE_PYTHON |

New tests by file: 12 in `crates/qcore-frontend/tests/source_mutex.rs`
(63 → 75), 14 in `crates/qcore-core/tests/unit/mutex_invariant.rs`.

The pinned 185-unit artifacts are untouched **by construction, not
re-measured**: no C-gateway, audit, contract or schema file is in this card's
diff, and `MutexInvariantCertificate` carries no serde derives, so the new field
moves no serialized byte. The contract corpus stays 60 modules, `qcore-schemas`
is unchanged, and CT coverage stays at 31/50 — this card proposes a mapping, it
does not apply one.

## 9. Boundaries, recorded not repaired

1. **Growth by a non-creator is refused, not analysed.** The general repair is
   per-atom parker provenance in the share ledger, so destruction could return
   each parked interval to whoever parked it. The narrow rule is what landed.
   Named **CT34-GROWTH-CREATOR**.
2. **Both W5 halves share `Q621`.** "Not the creator" and "not full authority"
   are indistinguishable in a diagnostic vector, though the structured
   diagnostics differ only in nothing at all (both are
   `MutexAuthorityNotConserved { event, resource }`). A third code was considered
   and rejected as over-fragmentation; the two are distinguished by test, not by
   code. Named **CT34-GROWTH-Q621**.
3. **No native evidence.** Growth is a change to the model's bookkeeping, not to
   emitted C: the emitted `mtx_t` protects whatever the program says it does, and
   nothing in the backend reads `protects`. No sanitized native run was added,
   and none is claimed. Whether a real C program's protected region can be
   widened the way this model says is the same `DeclaredFootprintShape` premise
   as before, now over a larger region.
4. **A refused growth produces a seven-entry cascade.** The lowering mints the
   new predicate identity before the core decides, so a refused growth leaves the
   section closing on an identity the generation never took. Every entry is
   honest and pinned (§4.1), but the leading diagnostic is not the growth's own —
   it sits second in the enum's order. Named **CT34-GROWTH-CASCADE**.
5. **Shrinking and replacement remain outside the profile entirely.** There is no
   event for either, so CT34's "replaces identities" half is answered by refusal
   (`Q617`), not by a rule about replacement.
6. **The rule is per-generation and lexical.** A growth names its own section's
   guard, so growing a generation from outside its critical section, or across
   tasks, is not expressible. Composition with the rank discipline was measured
   and is deliberately nil: a rank is a declared position and growth orders
   nothing (§4).
7. **`grow` is reserved and therefore invisible to name resolution.** The
   standing T03-F hazard, with a zero-occurrence census as today's evidence.

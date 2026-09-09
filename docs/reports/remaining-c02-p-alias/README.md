# C02-P-ALIAS — the bounded may-alias refinement — 2026-09-09

Card C02-P closed its own admission and recorded one boundary it deliberately
did **not** repair, because the boundary belonged to the whole resource
projection rather than to that admission
([C02-P §6](../remaining-c02-p/README.md)):

> Two *distinct declared names* may denote the same object, and this projection
> has never related them.

The measured shape was

```c
void w(struct b_handle *a, struct b_handle *b) { b_release(a); b_probe(b); }
```

which answered **`Safe`**. Nothing in that body establishes `a != b`, so the
borrow of `b` may read an object `b_release` already destroyed. The projection
said nothing about `b` because it had bound nothing to `b` — *silence about a
name was what made the answer `Safe`.* C02-P pinned the plain-name form and the
pointer-field form to answer **identically**, so that a later increment would
have to move both together. This card is that increment, and both rows moved
together, from `Safe == Safe` to `Unknown == Unknown`.

**Nothing is promoted.** No unit and no function changed verdict anywhere in the
185-unit corpus. No diagnostic code was added, changed or downgraded. No pinned
expectation was weakened.

---

## 1. The rule

Implemented as [`crates/qcore-c-gateway/src/may_alias.rs`](../../../crates/qcore-c-gateway/src/may_alias.rs)
(the class relation) plus four pieces of
[`legacy_c.rs`](../../../crates/qcore-c-gateway/src/legacy_c.rs):
`alias_facts` (the pre-pass), `require_unaliased_reads` (the refusal),
`distinct_by_provenance` (one discharge) and `LegacyInstruction::BranchOnDistinct`
(the other).

A **modeled read of a name `Y`** is *unresolved* — never `Safe` — when this body
released some other name `X` on the path reaching it and every one of these
holds:

| # | Premise | Refused otherwise |
|---|---|---|
| 1 | `X` and `Y` are **distinct declared names** of the same function | a name against itself, which the existing double-release model already judges |
| 2 | both are declared **exactly `struct TAG *`** for the same `TAG`, read from the same preprocessed token stream every other typed rule reads (`FunctionTypes::record_pointer_names`) | `void *`, multi-star, an unresolved typedef, a record value, a scalar, and *every* name in a function whose declarations the reader refused (**`C02-P-ALIAS-UNTYPED`**) |
| 3 | **at least one of the two is a formal parameter** | a pair of two locals (**`C02-P-ALIAS-LOCALS`**) |
| 4 | the release of `X` is a **`Released` effect on a plain name** — an explicit `Release` instruction or a declared `Released` position of a contracted call | a `Consumed` argument (**`C02-P-ALIAS-CONSUME`**); a release through a member place (`free(p->field)` releases a member, not the name `p`) |
| 5 | `Y` is a **root name of a modeled read**: the root of a whole actual at any declared position of a contracted call that *reads* it, or the base of an admitted C02-P opaque field read | an `Out` position, whose actual is the address of a slot the callee writes, not a pointer this call dereferences (**`C02-P-ALIAS-USES`**); a scalar-field no-op; a branch condition; a return expression |
| 6 | neither **discharge** (§2) applies | — |

The **root name** of a canonical place is the identifier before its first `->`,
`.` or `[`: `storage` for the actual `storage->cio`, `b` for the actual `b`. An
indirect place (`*out`) has no root and is related to nothing.

### 1.1 What it can and cannot do to a verdict

The rule only ever **adds** an `UnknownAlias` diagnostic, before the
instruction's own effects, which still apply exactly as before. It removes no
diagnostic, suppresses no leak, and converts no `Unsafe` into anything else. The
only movement it can cause is therefore `Safe → Unknown`; this is proved in Lean
as `the_rule_never_promotes` and measured on the corpus in §4.

The diagnostic is the existing `UnknownAlias`, spelled `place/peer` in the
composite convention `attach_contained_resource` and `detach_contained_resource`
already use (`storage/dup`, `b/a`). **No diagnostic code was added or changed.**

---

## 2. The two discharges

### 2.1 Provenance

`Y` is bound, at the read, to exactly one **live identity this body acquired** —
not a caller-held entry site, not `unknown`, not possibly null, and carrying no
unresolved effect (`distinct_by_provenance`). A body-made acquisition returns an
object distinct from every object live when it ran, so the projection's own
identity for `Y` answers the question.

Three refusals make this precise, each with its own test:

* an **unbound** name is never discharged — silence is exactly what this card
  refuses to read as safety;
* a **caller-held entry site** is never provenance: two formals carrying
  declared entry custody are the pair this card exists for;
* a **dead** binding is never provenance. `a_released_body_acquisition_no_longer_discharges`
  measures a body that answered `Safe` before this card *because* a `Borrowed`
  position of an already-released identity was a no-op. It is now refused.

### 2.2 Pointer inequality

A branch on `X != Y` or `X == Y`, where both operands are plain names this rule
relates to each other, becomes `LegacyInstruction::BranchOnDistinct`. Only the
edge on which the two names are provably distinct carries the fact; **nothing at
all is assumed on the other edge** and no path is claimed infeasible. The guard
is dropped as soon as either of its names is rebound, because an inequality
proved of the old value says nothing about the new one.

`null_condition` still wins: a null test keeps `BranchOnNull` exactly as before,
so no existing branch changed shape. A guard is refused unless both operands
are related names, so `if (key != stack_key)` over a `char *` and a `char[256]`
is untouched.

### 2.3 What is *not* a discharge

Rebinding the **releasing** name is not one. Reassigning `X` does not resurrect
the object it released, so `Y` stays refused; `released` is monotone for that
reason, and `rebinding_the_releasing_name_keeps_the_refusal` is the pin.

---

## 3. Why the facts live outside the custody state — a measured finding

The first implementation carried `released_names` and `distinct_names` inside
`AbstractState`. That is the obvious design and it is **wrong here**, for a
reason only measurement surfaced.

Adding components to `AbstractState` changes how often `AbstractState::join`
reports a change, which changes how often successors are re-pushed, which
changes the work list's order. And `report_leaks` runs **inside** the work-list
loop, on *intermediate* iterates of an exit node, not on its settled state;
diagnostics accumulate in a `BTreeSet` and are never withdrawn. So a reordering
alone can add a leak.

It did. In the first AFTER measurement, `flb_input_chunk.c :: update_logs_tag_records_metrics`
moved `Unknown → Unsafe` with two `leak` diagnostics — a *false* allegation
about upstream code, caused by scheduling and not by this rule. Two bisecting
audit runs isolated it:

| Run | rule's refusal | `BranchOnDistinct` | facts in `AbstractState` | `diagnostics.json` |
|---|---|---|---|---|
| BEFORE | — | — | — | `cc6d8dc0…` (the wave-10 baseline) |
| AFTER (first) | on | on | yes | `21d4ffc4…` — +2 leaks, +1 obligation |
| bisect 1 | **off** | on | yes | `a23e59ac…` — +2 leaks, no obligation |
| bisect 2 | **off** | **off** | yes | `a23e59ac…` — identical to bisect 1 |
| AFTER (final) | on | on | **no** | `cd68228b…` — +1 obligation, **no leak change** |

Bisect 1 shows the leaks survive with the refusal disabled; bisect 2 shows
`BranchOnDistinct` is not the cause either. The facts in the custody state were.

The repair is `alias_facts`: a **separate forward fixed point** over the same
CFG, run before the custody analysis and only read by it. Which releases have
run and which pointer inequalities a branch proved depend on the CFG's shape and
its instructions alone, never on the abstract state, so the separation is exact
rather than an approximation. `AbstractState` and its `join` are byte-identical
to the branch base, so the custody work list is scheduled exactly as it was, and
the card is **scheduling-neutral**.

> **`C02-P-ALIAS-LEAK-ORDER` (open, not repaired).** The underlying
> order-sensitivity is pre-existing and unrelated to this card: `report_leaks`
> runs on intermediate iterates of an exit node, so the leak inventory a
> function publishes can depend on work-list order. Moving it to the settled
> exit states — which the code already computes, in the observation loop — is a
> principled repair that would move the pinned leak count (271) and could
> withdraw `Unsafe` unit verdicts. It belongs to its own pin-moving card.

---

## 4. Corpus impact — one added diagnostic, zero verdict movement

### 4.1 What was run

The canonical pinned route on the shared prepared pinned clone (Fluent Bit
`66910c10a4d7eafa810b84229cf2dcf7b0f26f97`, explicit `/usr/bin/clang`), the
same one [`docs/reports/corpus-repin/provenance.json`](../corpus-repin/provenance.json)
records, run twice from this worktree with only `crates/qcore-c-gateway/src/`
moved between runs, so the comparison carries no path provenance at all:

```sh
python3 -B providers/package.py build --output build/cycle19/provider-packages-alias
QCORE_CLANG=/usr/bin/clang \
QCORE_CHUNKIO_PACKAGE=build/cycle19/provider-packages-alias/qcore.chunkio \
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  --pinned-evidence <clone> <monkey>/compile_commands.json \
  <out>/monkey-source.json <out>/resource-diagnostics.json
```

Both exit 0 with zero gateway failures. `build/cycle19/` is gitignored, so the
runner script is not committed; the command it runs is complete above.
Transcripts: [`logs/corpus-before.log.gz`](logs/corpus-before.log.gz),
[`logs/corpus-after.log.gz`](logs/corpus-after.log.gz), plus the two bisecting
runs of §3.

### 4.2 Harness validation — BEFORE against the archived baseline

| Artifact | BEFORE vs [`docs/reports/wave10-corpus/artifacts/`](../wave10-corpus/artifacts) |
|---|---|
| `diagnostics.json` | **byte-identical** (SHA-256 `cc6d8dc087f20922…`) |
| `chunk-contracts.json` | **byte-identical** (`c4f438e4ed3dab3c…`) |
| `source.json` | **byte-identical** (`53d55b7e1fb174de…`) |
| `contracts.json` | differs — path-literal provenance residue only |
| `source-effects.json` | differs — package-identity residue only |

The two differing artifacts are the two residue classes wave 9 and wave 10
already record; both are **byte-identical between BEFORE and AFTER here**, so
neither is attributable to this card.

### 4.3 The complete AFTER diff

Exhaustive, produced by
[`docs/reports/corpus-repin/diff_artifacts.py`](../corpus-repin/diff_artifacts.py)
and archived at
[`artifacts/diff-before-to-after.json.gz`](artifacts/diff-before-to-after.json.gz).

**Four of the five artifacts are byte-identical between BEFORE and AFTER.** Only
`diagnostics.json` moved, and within it exactly one leaf:

| Unit | Function | Change | Rule |
|---|---|---|---|
| `librdkafka` / `rdkafka.c` | `rd_kafka_new` | **+1** `unknown_alias` `unresolved resource identity` **`app_conf/conf`** at preprocessed line 22660 | §1, premises 1–6 with no discharge |

Everything else is unchanged, and this is measured rather than argued:

| Figure | BEFORE | AFTER |
|---|---|---|
| Units | 185 | 185 |
| Unit verdicts | 2 Safe / 42 Unsafe / 137 Unknown / 4 data-only | **identical** |
| Unit verdict movements | — | **0** |
| Function statuses | 119 Safe / 84 Unsafe / 4,631 Unknown | **identical** |
| Function status movements | — | **0** |
| `leak` diagnostics | 271 | **271** |
| `unsupported` diagnostics | 49,936 | **49,936** |
| `unknown_alias` diagnostics | 4,386 | **4,387** |

**No unit and no function left `Safe`.** The one obligation lands on a function
that was already `Unknown` and stays `Unknown`, so it is a coverage refusal on
an already-uncovered body — the narrowest possible movement in the fail-closed
direction.

### 4.4 The one movement, named

`rd_kafka_new` (librdkafka 2.15.0, `src/rdkafka.c`) declares the formal
`rd_kafka_conf_t *app_conf` and the local `rd_kafka_conf_t *conf`, and on one
path assigns `conf = app_conf`, so the two names genuinely may denote the same
object. On the `!app_conf` path `rd_kafka_conf_destroy(conf)` releases `conf`;
`app_conf` is then read at a modeled position. The body's own guard is
`if (!app_conf)` — a **null** test, not a pointer inequality — so neither
discharge applies, and the projection has no correlation between `!app_conf` and
`conf != app_conf` to offer. The upstream source comments the same hazard in its
own words ("to avoid double-freeing … and the user's eventual call to
`rd_kafka_conf_destroy()`"). **This is a withdrawn coverage claim, not a defect
allegation about librdkafka**: the function was `Unknown` before and after.

### 4.5 The archived set, and the pin

[`artifacts/`](artifacts) holds the **AFTER** measurement — all five produced
reports gzipped with a `manifest.json` carrying each one's uncompressed
SHA-256, in the layout `diff_artifacts.py --archived` reads back — together with
the complete BEFORE→AFTER diff and, for §3's record, the diff of the discarded
first AFTER run.

This card **does not move the baseline pin**. `docs/reports/wave10-corpus/artifacts/`
remains the current 185-unit baseline; nothing here supersedes it, and since no
verdict moved and only one diagnostic leaf was added, re-pinning is the
integrator's consolidation to make at the merged tree, not this card's.

### 4.6 Measured reach, honestly

**One** obligation, over 185 units and the 4,834 functions the reports carry a
per-function status for (the enumeration denominator E01-F1 measures is 4,921;
the difference is unprojected definitions). That is a low
reach and it is the measured number, not an estimate. Two reasons, both visible
in the fixtures: a body must (a) release through a plain name and (b) have
another *typed* `struct TAG *` name of the same tag with a formal involved, and
most such bodies in this corpus are already `Unknown` for other reasons, so the
refusal is invisible in the published inventory. No census of how many bodies
the *relation* reaches without firing was taken; that is recorded as an open
item (§7).

---

## 5. Adversarial evidence

`crates/qcore-c-gateway/tests/may_alias_boundary.rs` — 22 held-out acceptance
tests over the generic `b_*` API C02-P introduced, deliberately unrelated to
ChunkIO, Fluent Bit and O04's `g_*` family. **Every body was measured at the
branch base `695f0e1` first**, and each test's doc comment records that `BASE:`
answer, so every assertion is a measured movement or a measured preservation.

### 5.1 Refused — each moved `Safe → Unknown`, with its exact obligations

| Shape | BASE | Obligations now |
|---|---|---|
| the measured plain-name control | `Safe` | `b/a` |
| the measured field-read form (`b_release(dup); storage = in->storage; b_open(storage->cio, …)`) | `Safe` | `storage/dup` |
| three formals of one class | `Safe` | `b/a`, `c/a` |
| a local copy of a formal, released formal → read local | `Safe` | `held/a` |
| the same pair reversed, released local → read formal | `Safe` | `a/held` |
| a release on one arm only, reaching the join | `Safe` | `b/one` |
| the releasing name reassigned afterwards | `Safe` | `b/one` |
| work on the arm where the guard leaves the names possibly equal | `Safe` | `b/a` |
| the guarded name reassigned inside the guard | `Safe` | `b/a` |
| both names carry caller-held entry custody | `Safe` | `b/a` |
| the read place was acquired here **and then released** | `Safe` | `fresh/a` |

### 5.2 Untouched — each keeps its exact pre-card answer

| Shape | BASE | Now |
|---|---|---|
| distinct declared record tags | `Safe` | `Safe`, no obligation |
| same class, nothing released | `Safe` | `Safe`, no obligation |
| C02-P's own admitted constructor shape | `Safe`, no diagnostics | **unchanged** |
| two locals of one class, no formal (`C02-P-ALIAS-LOCALS`) | `Unknown`, `[unknown_alias one]` | **exactly that**, asserted as the complete diagnostic vector |
| a leak behind a refused read | `Unsafe`, `[leak, unknown_alias a]` | `Unsafe`, leak retained, obligation added |

### 5.3 Discharged — each keeps `Safe`

| Shape | BASE | Now |
|---|---|---|
| `if (a != b) { b_release(a); b_probe(b); } else { b_release(a); }` | `Safe` | `Safe` |
| the `a == b` spelling with the work in the `else` arm | `Safe` | `Safe` |
| `b_release(a); fresh = b_open(…); b_probe(fresh); b_close(fresh);` | `Safe` | `Safe` |

Each discharge has its measured control beside it: the identical body guarded by
an unrelated condition (`if (n)`) is refused, and the arm on which the names may
be equal is refused.

### 5.4 Independent rederivation

The refusal rests on declarations and on the guard, both carried in the
certificate's own preprocessed source. Substituting only those, leaving every
statement byte-identical, is refused by `check_legacy_safety_certificate`:

| Substitution | Result |
|---|---|
| the second formal's record tag becomes the first's | recheck **refuses** |
| the first formal's record tag becomes the second's | recheck **refuses** |
| `if (a != b)` becomes `if (a == b)` | recheck **refuses** |

The unmodified certificate is re-checked at the end of both tests, so each
refusal is caused by its substitution and not by an unrelated failure.

### 5.5 Unit-level

* `crates/qcore-c-gateway/src/may_alias.rs` — 10 module tests of the class
  relation: symmetry, irreflexivity, distinct tags, the untyped and
  two-locals bounds, both positive directions, and the place-root computation.
* `crates/qcore-c-gateway/tests/unit/legacy_c.rs` — 7 tests driving `alias_facts`
  directly over hand-built CFGs: forward propagation, the union at a join, the
  **intersection** of guards at a join, the guard dropped by a rebinding of
  either name, the release *kept* across a rebinding, only `Released` positions
  recording a release, and a member place recording none.

---

## 6. Lean inventory

New module [`proof/QCore/MayAlias.lean`](../../../proof/QCore/MayAlias.lean),
namespace `QCore.MayAlias`, imported by `proof/QCore.lean` and audited from the
new `proof/AuditMayAlias.lean`. **41 theorems, no new axiom, no placeholder, no
`sorry`** — every one depends only on `propext`, `Classical.choice` and
`Quot.sound`, the three the project's closure allows.

* **the class relation** — `sameTag_is_symmetric`, `peer_is_symmetric`,
  `a_name_is_never_its_own_peer`, `a_different_tag_is_never_a_peer`,
  `an_untyped_name_is_never_a_peer`, `two_locals_are_never_peers`, and the two
  positive directions `two_same_tagged_formals_are_peers`,
  `a_formal_and_a_local_are_peers`;
* **the refusal, both ways** — `a_released_peer_refuses_the_read` and its
  converse `a_refusal_names_a_released_undischarged_peer` (the rule invents
  nothing), `no_release_refuses_nothing`,
  `releasing_a_name_does_not_refuse_itself`,
  `releasing_a_non_peer_refuses_nothing`;
* **provenance** — `a_live_body_made_binding_discharges` and the four ways it
  fails closed: `an_unbound_name_is_never_discharged`,
  `a_dead_binding_never_discharges`,
  `a_caller_held_binding_never_discharges`,
  `an_ambiguous_binding_never_discharges`;
* **the guard** — `the_guard_discharges_its_pair`, `the_guard_is_symmetric`,
  `a_guard_discharges_no_other_peer`, `rebinding_drops_the_guard`,
  `rebinding_the_other_name_drops_the_guard`,
  `rebinding_an_unrelated_name_keeps_the_guard`;
* **rebinding never discharges a release** — `rebinding_only_drops_guards`,
  `rebinding_preserves_every_release`,
  `rebinding_the_releasing_name_keeps_the_refusal`;
* **monotonicity and the join** — `adding_a_release_only_refuses_more`,
  `the_join_unions_the_releases`, `the_join_only_keeps_shared_guards`,
  `the_join_preserves_a_refusal`, `the_join_intersects_the_guards`,
  `the_join_keeps_a_guard_both_paths_proved`, and the two length lemmas
  `the_join_never_shortens_the_releases` /
  `the_join_never_lengthens_the_guards` that are the pre-pass's termination
  argument;
* **the verdict** — `a_refusal_is_unresolved`, `no_refusal_changes_nothing`,
  and the fail-closed direction that matters, `the_rule_never_promotes`;
* **non-vacuity** — `the_rule_is_not_vacuous`,
  `the_guard_discharge_is_load_bearing`,
  `the_provenance_discharge_is_load_bearing`. Without these, every refusal
  theorem above would hold of the empty rule and of a rule that never
  discharges.

The Lean model is a model of the **decision**. That the Rust reader extracts the
declared types and the release events faithfully from C source is a premise, not
a theorem, exactly as in `QCore.BorrowedActual`; nothing here is a claim about
arbitrary C memory safety, and nothing here *decides* aliasing — the rule only
refuses to decide where the projection previously answered by silence.

### 6.1 `BoundFieldRead`'s disjointness premise, re-verified

Card O04-READ-BOUND's report records a cross-card note: *"if you widen
`untracked_bases` so a call-bound base becomes admissible, its disjointness
theorem's premise moves and its §5 must be re-measured."*

**This card does not widen `untracked_bases` at all.** The whole diff of
[`borrowed_actual.rs`](../../../crates/qcore-c-gateway/src/borrowed_actual.rs) is
empty: not one line changed, so the set of admissible access bases is
byte-identical and O04-READ-BOUND's premise is untouched by construction. This
card moves in the *opposite* direction — it withdraws admissions, it never adds
one.

Re-verified rather than asserted:

| Check | Result |
|---|---|
| `git diff 695f0e1 -- crates/qcore-c-gateway/src/borrowed_actual.rs` | **empty** |
| `QCore.BoundFieldRead` audited theorems | 44, unchanged, all pass |
| O04-READ-BOUND's own test target (`tests/o04_read_bound.rs`) | passes unchanged |
| O04-B08 / O04-ARMS / C02 / C02-P test targets | pass unchanged (13 + 16 + 18 + 11 assertions) |
| the corpus figures O04-READ-BOUND §5 measures | `chunk-contracts.json`, `source.json`, `source-effects.json` and `contracts.json` are **byte-identical between BEFORE and AFTER**, and `diagnostics.json` moves by the single `rdkafka.c` leaf of §4.3, which touches no aggregate read |

The interaction that *would* matter runs the other way and is exercised: a
bound-read base that this rule compromises is refused before the bound-read gate
sees it, because `require_unaliased_reads` runs on every `ContractCall`'s
`alias_reads` before `bound_read_resolves`. On the pinned corpus no such case
exists, which is why the artifacts do not move.

---

## 7. Open boundaries

Each is a *retained* pre-existing answer, never a new admission.

* **`C02-P-ALIAS-LOCALS`** — a pair of two **locals** is related by nothing in
  v1. `one = in->storage; two = in->storage; b_release(one); b_probe(two);` may
  well denote one object and keeps its pre-card answer exactly. Pinned by
  `two_locals_keep_the_pre_existing_answer_in_v1`, which asserts the complete
  diagnostic vector rather than a subset, so lifting the bound moves that test.
* **`C02-P-ALIAS-UNTYPED`** — a function whose declarations the reader refused,
  and every name that is not exactly `struct TAG *`, establish no type sharing,
  so the rule's precondition never holds there. This is deliberate: making an
  unreadable declaration compromise every release would be a very large,
  differently-motivated refusal.
* **`C02-P-ALIAS-CONSUME`** — a `Consumed` argument transfers custody away and a
  later read of a peer is exposed in much the same way, but v1 relates only
  `Released`.
* **`C02-P-ALIAS-USES`** — a read is related only at a modeled contracted-call
  position or an admitted opaque field read. A C02 scalar-field no-op, a branch
  condition, a return expression and an `Out` slot address are not.
* **`C02-P-ALIAS-GUARD-FORMS`** — the guard is recognised only on an `if`, and
  only as `X != Y` / `X == Y` (optionally negated and parenthesized) between two
  plain related names. A `while`, a `for`, a ternary, a conjunction and a
  comparison written through a member all keep the pre-card answer.
* **`C02-P-ALIAS-LEAK-ORDER`** — §3: `report_leaks` runs on intermediate
  iterates of an exit node, so a published leak inventory can depend on
  work-list order. Measured, bisected, recorded, **not repaired**; this card is
  scheduling-neutral instead.
* **The declared premises are inherited unchanged and unproved** — declaration
  completeness and C effective types, exactly as C02 and C02-P record them. The
  provenance discharge additionally rests on the projection's own identity model
  being right about which sites this body acquired.
* **This is not an alias analysis.** It relates names by *declared type* and
  refuses; it decides nothing about whether two pointers are in fact equal, and
  it establishes no upper bound on aliasing anywhere else in the projection.
* **`docs/reviews/r0_3/acceptance_matrix.json`, `acceptance.json`,
  `ct_manifest.json`, `AGENTS.md`, `CLAUDE.md`, `README.md` and
  `DEVELOPMENT.md` were deliberately not touched** — the integrator
  consolidates those. See §9 for the one test this leaves failing by design.

---

## 8. Files changed

| Path | Change |
|---|---|
| `crates/qcore-c-gateway/src/may_alias.rs` | **new** — the class relation and the place-root computation, plus 10 module tests |
| `crates/qcore-c-gateway/src/legacy_c.rs` | `AliasPeers`; `alias_reads` on `ContractCall` and `alias_read` on `OpaqueValue`; `BranchOnDistinct` and its `If` condition reader; `NodeAliasFacts` + `alias_facts` (the separate pre-pass) with `released_names`/`rebound_names`; `require_unaliased_reads` and `distinct_by_provenance`; `transfer` takes the node's facts. `AbstractState` and `AbstractState::join` are **byte-identical to the base** |
| `crates/qcore-c-gateway/src/typed_field_access.rs` | `FunctionTypes::record_pointer_names`, additive |
| `crates/qcore-c-gateway/src/lib.rs` | `mod may_alias;` |
| `crates/qcore-c-gateway/tests/may_alias_boundary.rs` | **new** — 22 held-out acceptance tests |
| `crates/qcore-c-gateway/tests/unit/legacy_c.rs` | 7 pre-pass tests |
| `crates/qcore-c-gateway/tests/borrowed_field_actuals.rs` | C02-P's agreement pin flips per its own design: the shared value is now asserted explicitly so the pin cannot go vacuous |
| `proof/QCore/MayAlias.lean` | **new** — 41 theorems |
| `proof/AuditMayAlias.lean`, `proof/QCore.lean` | the audit and the import |
| `docs/reports/remaining-c02-p-alias/` | this record, the AFTER artifacts, the complete diffs and the run transcripts |

`crates/qcore-c-gateway/src/borrowed_actual.rs` and
`crates/qcore-c-gateway/src/aggregate_store.rs` are **untouched**.

---

## 9. Gate results

Branch `claude/c02-p-alias` in an isolated worktree on base `695f0e1`.

| Gate | Command | Base | After | Exit |
|---|---|---|---|---|
| Workspace tests | `cargo test --workspace --no-fail-fast` | 2,034 passed / 4 ignored | **2,073 passed / 0 failed / 4 ignored** | 0 |
| Format | `cargo fmt --all -- --check` | — | clean | 0 |
| Clippy | `cargo clippy --all-targets -- -D warnings` | — | clean | 0 |
| Probe | `cargo run -p qcore-compiler --example reevaluation_probe` | — | exit 0 | 0 |
| Follow-up | `cargo run -p qcore-compiler --example reevaluation_followup` | — | exit 0 | 0 |
| Provider tests | `cargo test --locked --offline --manifest-path providers/Cargo.toml --workspace` | 76 | **76 passed / 0 failed** | 0 |
| Lean audits | `python3 scripts/check_lean_audits.py` | 51 | **52 audits passed** | 0 |
| Python tests | `python3 -m unittest discover -s scripts/tests` | 125 | **124 passed, 1 failed by design** | 1 |
| Pinned 185-unit corpus | canonical pinned route, §4 | reproduced byte-identically | re-measured, §4.3 | 0 |

**The one Python failure is by design and is for the integrator, not a defect
of this card.** `test_build_matrix_sections.BuildMatrixSections.test_the_derived_facts_are_recomputed_rather_than_carried`
compares `docs/reviews/r0_3/acceptance_matrix.json`'s derived
`validation.lean_audits` against `proof/Audit*.lean` on disk, and this card adds
one audit file: `['8.0.0', 51] != ['8.0.0', 52]`. The matrix is integrator-owned
and was deliberately left untouched; regenerating it at the merged tree resolves
this row.

One test failed transiently in an earlier full-workspace run while a corpus
audit was executing on the same host; a clean re-run reports 0 failed. The
load-flaky targets (`P02` `RLIMIT_NPROC`, `P03` confined-write, `vp04`) are
recorded in [`docs/reports/remaining-p03/README.md`](../remaining-p03/README.md) §9.1.

---

## 10. Scope and non-claims

* **No raw unit is promoted**; no unit and no function verdict moved in either
  direction, anywhere in the 185-unit corpus.
* `Safe` in the `b_*` family is `Safe` for a synthetic declared custody
  projection over trusted test assumptions. The fixtures are declared test
  assumptions, not verified facts about any library.
* The single corpus obligation is a **withdrawn coverage claim** about
  `rd_kafka_new`, not an upstream defect allegation; that function was `Unknown`
  before this card and is `Unknown` after it.
* Projection coverage is not safety coverage; unknowns stay in the denominator.
* CT coverage is unchanged at 30/50 mapped; the contract corpus remains 60
  modules; `qcore-schemas` is unchanged at 8.0.0.

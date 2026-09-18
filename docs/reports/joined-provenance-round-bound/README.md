# JOINED-PROVENANCE-ROUND-BOUND (wave 37)

**The card started as a measurement and became an arbiter, and the arbiter moved
nothing.** The measurement said facts *would* be derived if the iteration bound
rose — 9 analysis visits across 4 functions — so by the card's own rule the
bound had to move and the corpus had to be re-pinned. It was, and **zero leaves
moved**: the verdict-bearing artifacts reproduce the baseline byte for byte. The
reason is stated in §5 and is the whole finding.

The two functions the wave-36 residual named need **148** and **101** rounds,
and on convergence they derive **nothing at all**.

Base: `477b733d` (wave 37 open). Baseline reproduced:
`docs/reports/joined-provenance-call-frames/artifacts/`. `origin/main` was
fetched before the final BEFORE and had not moved, so no sibling arbiter
intervened.

## 1. What this card was asked, and what it answers

| Residual | Status |
|---|---|
| `JOINED-PROVENANCE-ROUND-BOUND` (wave 36) | **Discharged.** §3.4 |
| `JOINED-PROVENANCE-DECLINE-INDISTINGUISHABLE` (STABILITY-REEVAL #2) | **Discharged.** §2 |
| `JOINED-PROVENANCE-FIXED-POINT-BOUND-UNMEASURED` (STABILITY-REEVAL #2) | **Discharged.** §3 |
| `JOINED-PROVENANCE-RELEASE-NULLABLE` | **Not retired** — it is a decision and not this card's. Re-measured in §8 |

**One correction to the brief, on the archive's own evidence.** The cyclic
population is **1,273**, not 1,272. `docs/reports/joined-provenance-cycles/logs/census.txt`
reads `with a cyclic CFG: 1273` and the wave-35 report states 1,273 throughout;
this card's own independent census, taken by a different mechanism, reaches
1,273 as well. The figure is not edited anywhere — it was only mis-transcribed
into the brief, and is recorded here so the next reader is not left reconciling
two numbers.

## 2. What landed: the decline became a value

Before this card `fixed_point` returned an all-empty `Vec<Bindings>` in **two**
different situations — a graph that settled with nothing to say, and a graph
whose bound was exhausted — so the decline rate was unobservable without a code
change. That is exactly what `JOINED-PROVENANCE-DECLINE-INDISTINGUISHABLE`
recorded, and this is the smallest change that ends it:

* `iterate(cfg, reachable, sites, cap) -> Settled` is the sweep, split out of
  `fixed_point` unchanged. `Settled::Converged { rounds, solution }` carries the
  round it settled at; `Settled::Declined` says the cap was reached.
* `fixed_point` maps `Declined` to **precisely the vector it always returned**.
  No diagnostic, no certificate field, no census channel in production: the
  outcome is internal, and the byte-identity measurement in §4 is the evidence
  that it perturbs nothing.
* Three tests drive both arms of one graph — a cap that suffices, and one round
  less. The reported round is pinned **exactly**, as the least cap under which
  the graph settles, so an off-by-one in the count is visible (§7).

The rounds census lives behind the off-by-default cargo feature
`provenance-rounds-census`, on the `C-GATEWAY-INDIRECT-CENSUS-2` precedent: with
the feature off none of it exists in the compiled crate. It re-runs **the
analyzer's own `iterate`** under a larger cap — never a second implementation
that could drift — and drops the result. It is a *child module* of
`legacy_joined_provenance`, so `legacy_c.rs`, which another wave-37 card owns,
is not touched at all.

## 3. The measurement

One pinned Cycle 19 corpus audit, census collecting, probe cap 1,024.
`measure/joined-provenance-round-bound/logs/rounds-census.jsonl`, 2,704 records.

### 3.1 Population

| | visits | distinct functions |
|---|---:|---:|
| Cyclic CFGs reaching the fixed point | **2,704** | **1,273** |
| — no off-cycle site, so no sweep runs at all | 2,545 | 1,195 |
| — a sweep runs | **159** | 78 |

The 1,195-function majority is the headline nobody had: **94% of the cyclic
population never reaches the iteration at all**, because every site it creates
is created on a cycle and is therefore ineligible by the wave-35 rule. Whatever
the bound is, it decides nothing for them.

### 3.2 Rounds to convergence, at the old cap of 64

Of the 159 sweeping visits, **137 converged and 22 declined**. The converging
137 settle between **7 and 59** rounds — so the old constant of 64 was not
merely adequate for them, it had five rounds of headroom and no more:

```
  7:2   11:1  12:2  13:5  14:2  15:2  16:10 18:6  19:5  20:2  21:3
 23:2   24:4  25:2  26:4  27:6  28:6  29:4  30:6  31:8  32:2  37:8
 38:2   39:2  43:8  45:5  46:3  47:7  48:2  49:4  50:2  51:2  53:2
 56:2   57:2  59:2
```

### 3.3 The 22 declines, and what they actually need

Every one of the 22 declining visits converges under the probe, at:

| rounds | 65 | 67 | 76 | 95 | 98 | 101 | 105 | 148 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| visits | 2 | 2 | 6 | 4 | 2 | 2 | 2 | 2 |

**Nothing needs more than 148 rounds.** Ten distinct functions decline; the
spread 65–148 says the old cap was not wildly wrong, merely a little short —
which is the least comfortable place for a magic number to sit, because it makes
the cut arbitrary rather than principled.

### 3.4 The two functions the residual named — discharged

| function | reachable nodes | edges | sites | names | rounds needed | proved bound | facts derived on convergence |
|---|---:|---:|---:|---:|---:|---:|---:|
| `rd_kafka_mock_handle_ConsumerGroupHeartbeat` | 719 | 864 | 1 | 13 | **148** | 10,785 | **0** |
| `rd_kafka_mock_handle_ShareGroupHeartbeat` | 469 | 565 | 1 | 5 | **101** | 3,283 | **0** |

This is the discharge, and it has two halves that must be read together. The
wave-36 residual asked *how many rounds these two need*: 148 and 101, so the
64-round cap was indeed what stopped them, and the off-cycle restriction was not
— wave 35 had never separated the two causes and this separates them. But the
second half is that **on convergence they derive no fact whatever**. Their
`SubscribedTopicNames` site is eligible, the sweep settles, and the settled
solution claims no name for it. Raising the bound was therefore never going to
help those two sites, and the premise gate they visit stays blocked for the
reason §8 names.

### 3.5 Would anything be derived?

Yes — which is what made this an arbiter. Nine of the 22 declining visits, across
**4 distinct functions**, publish facts once the sweep is allowed to settle:

| function | rounds | sites newly deriving |
|---|---:|---:|
| `cat_exp_histogram_values` | 76 | 2 |
| `unittest_msgset_all_error_types_with_valid` | 95 | 1 |
| `flb_input_chunk_get_direct_routes` | 98 | 4 |
| `flb_input_chunk_write_header_v2` | 105 | 3 |

`flb_input_chunk_get_direct_routes` is the function constraint 7's own history
turns on — the six leaves wave 23 withdrew and the seventh, L19544, that still
stands. It deriving four new conditional-provenance facts is exactly the shape
that could have moved the leak pin. It did not (§5).

## 4. Byte-identity of the observability change alone

The census run's binary carries the `Settled` refactor **and** the census
feature compiled in, at the unchanged cap of 64:

```
*diagnostics      raw IDENTICAL  canonical IDENTICAL   a7aa3b5f9ab193a0
 source           raw IDENTICAL  canonical IDENTICAL   ae38f6c93dfbe341
*chunk-contracts  raw IDENTICAL  canonical IDENTICAL   c4f438e4ed3dab3c
 source-effects   raw differs                          (checkout root)
 contracts        raw differs                          (checkout root)
```

`source-effects` and `contracts` carry this worktree's checkout root and
identity digests and differ from the archive in **both** BEFORE and AFTER, as
they did for the two preceding cards; `byte_identity.py` gates only on the two
verdict-bearing artifacts. **RESULT: the verdict-bearing artifacts reproduce the
baseline byte for byte** — so the observability change moves nothing, and the
BEFORE side of the arbiter is established against the archive itself rather than
against a run of my own.

## 5. The arbiter: the bound moved, and zero leaves moved

`FIXED_POINT_ROUNDS = 64` is replaced by `round_bound(cfg, reachable)` — this
graph's own proved bound, §6. The AFTER run is the production binary, feature
off.

```
== diagnostic_counts ==            == leaf three-way split ==
  leak                 272 -> 272     added (only in AFTER):    0
  summary_conflict       1 ->   1     removed (only in BEFORE): 0
  unknown_alias       4308 -> 4308    unmoved:              54503
  unsupported        49922 -> 49922
== unit status moves: 0 ==   == function status moves: 0 ==
```

and the artifacts are byte-identical to the baseline on both verdict-bearing
documents (`a7aa3b5f9ab193a0`, `c4f438e4ed3dab3c`). **The leak pin of 272 is
unmoved**, as are `unknown_alias` 4,308 and `unsupported` 49,922; no unit and no
function changes status. There is nothing to attribute leaf by leaf, because no
leaf moved.

**Why the zero, stated as a mechanism and not as reassurance.** The derived
relation has exactly one consumer in the whole analyzer: `release_site`, reached
from `legacy_c.rs` at the single call site of `derive`. A newly derived fact can
therefore change a verdict only by meeting that premise gate. Card
`JOINED-PROVENANCE-CALL-FRAMES` measured the gate's whole reachable surface in
this corpus — **14 visits over 6 distinct `(function, site, source)` triples** —
and not one of them is in any of the four functions that newly derive. The new
facts are real, they are sound, and they are published where nothing asks for
them. That is a statement about the *narrowness of the consumer*, not about the
quality of the derivation, and it is the reason a 33.8% increase in sweep work
buys no verdict at all today.

The AFTER census confirms the new regime: all **159** sweeping visits converge,
**0** decline, maximum **148** rounds.

## 6. The proved bound

`proof/QCore/JoinedProvenanceRoundBound.lean`, audited by
`proof/AuditJoinedProvenanceRoundBound.lean` (14 theorems, axiom surface
`propext` / `Classical.choice` / `Quot.sound`, three of them axiom-free).

**The bound.** One node's value is `top`, or a set of present sites each
carrying a set of claimed names. The state space is finite, the sweep only ever
descends in it, so the whole vector descends and settles within

> `nodes × (1 + sites + sites × names)`

`rounds_are_bounded_by_the_graph_budget` assembles it; `a_cap_at_the_measure_settles`
is the operative half — **a cap at or above the measure never declines**. That
is what makes this a *bound* rather than a budget: reaching it falsifies a
theorem rather than exhausting a guess. `stopping_early_is_declining` records
the other direction, that a cap *below* it costs derivation and never soundness,
by `JoinedProvenanceCycles.claiming_nothing_is_sound`.

**The measure had to be corrected, and the correction is the interesting part.**
The obvious measure — count the absent sites, count the names literally stored —
**is wrong**, and is wrong precisely at the step that matters. A site *arriving*
(absent, with nothing stored, becoming present with a claim) would make that
count go **up**, so the descent argument would be false as stated. The repair is
to read absence as what it means: a site not present makes a vacuous claim, so
**absence is the top of that site's own lattice** and an absent site counts as
claiming *every* name. `liveAt` is that reading, and with it arrival is a
decrease, `Facts::join`'s `(false, true)` arm is literally "top meets anything to
give anything", and the tight budget comes back. I record this because a measure
that is merely plausible is how a termination proof becomes decoration.

**How loose is it?** Over the 159 sweeping visits the proved bound runs
min 28 / median 378 / max 17,091, against an observed maximum of 148 rounds. For
**11 of 159** visits the proved bound is at or below 64 — for those the old
constant was already provably sufficient, and nobody knew. No visit comes near
its own bound: the closest approach in the whole corpus is `set_events_from_ctr`
at 13 of 28 (46%), and the two big functions of §3.4 use 1.4% and 3.1% of theirs.

## 7. Falsification

`measure/joined-provenance-round-bound/mutations.py`, seven mutations, restoring
with `write_text` so the mtime advances (`MUTATION-HARNESS-MTIME-RESTORE`).
**Six of seven caught.** The first pass caught only four; the two gaps that
found are closed and are recorded here rather than quietly fixed, because a
green suite that never drove the arm is the thing `STABILITY-REEVAL` exists to
catch:

| mutation | caught | by |
|---|---|---|
| the bound is the flat 64 again | yes | `a_graph_needing_more_than_sixty_four_rounds_now_derives_its_fact` |
| the budget forgets the name universe | yes | `the_bound_is_the_proved_formula_and_grows_with_the_graph` |
| the budget is per node, not per vector | yes | seven tests |
| an exhausted cap claims to have settled | yes | four tests |
| the settling round is one too low | yes *(gap closed)* | `the_iteration_reports_convergence_and_the_round_it_settled_at` |
| a produced site is outside the site universe | yes *(gap closed)* | `a_produced_site_counts_toward_the_bound` |
| a decline publishes a fact | **no — expected** | `JOINED-PROVENANCE-DECLINE-ARM-UNREACHABLE` |

The survivor is a genuine boundary, not a missing test. Since the cap is now the
graph's *proved* bound, `fixed_point`'s declining arm is unreachable from any
graph unless `rounds_are_bounded_by_the_graph_budget` is false — so no test can
drive it through `derive` without first falsifying the theorem. The arm is
retained because it stays the only fail-closed answer if one ever did, and it
*is* driven directly at `iterate`, where a deliberately short cap reaches it.

## 8. The nullable counterfactual, re-measured

Card `JOINED-PROVENANCE-CALL-FRAMES`' premise instrumentation, re-anchored for
the refactored module (four of its seventeen anchors moved; the copy lives at
`measure/joined-provenance-round-bound/instrument.py` and the frozen card's
script is **not** edited), run over the whole corpus on this card's **final
tree** with the raised bound active. The module is swapped in and restored under
a shell trap, and `git status` was checked clean afterwards.

**The one sentence, for the user:** if the nullable-site premise were relaxed and
nothing else changed, **exactly one** of the fourteen `release_site` gate visits
would pass every premise — `rd_kafka_jwt_b64_decode_payload`, site 1, source
`payload` — unchanged by the raised bound.

The surrounding measurement, which is what makes that one sentence checkable:
14 visits over the same **6** distinct `(function, site, source)` triples as
wave 36 found; `nullable_site` blocks **14 of 14**; one visit is blocked by
`nullable_site` alone, four more by `nullable_site + unresolved_effect`, and nine
by `not_named_by_facts + nullable_site`.

It is worth recording that the two `rd_kafka_mock_handle_*GroupHeartbeat` visits
still report `not_named_by_facts` **even though their sweep now converges**. That
is an independent instrumentation reaching §3.4's conclusion by another route:
the bound was what stopped those two functions iterating, and iterating to the
end gives them nothing to name their site with.

## 9. Gate

The standing per-change gate from `AGENTS.md`, run detached with every exit
captured to its own file and no pipe in the command
(`measure/joined-provenance-round-bound/logs/gate/`):

| step | exit | note |
|---|---:|---|
| `cargo test --workspace --no-fail-fast` | **0** | **4,617 passed, 0 failed, 8 ignored**, 231 result summaries |
| `cargo fmt --all -- --check` | **0** | |
| `cargo clippy --all-targets -- -D warnings` | **0** | |
| `cargo clippy -p qcore-c-gateway --all-targets --features provenance-rounds-census` | **0** | the feature-on build is linted too, since nothing else in the workspace enables it |
| `cargo run -p qcore-compiler --example reevaluation_probe` | **0** | |
| `cargo run -p qcore-compiler --example reevaluation_followup` | **0** | |
| `python3 scripts/check_lean_audits.py` | **0** | **136 Lean audits passed** (135 → 136, this card's `AuditJoinedProvenanceRoundBound`) |

`git submodule update --init --recursive` was run first and exited 0.

**One honest note about the machine, because a gate figure is a measurement and
not a property.** The first attempt at this gate **died on a full disk** — the
workspace debug build reached 38 GB with the volume at zero bytes free, and
`ld` aborted with a bus error inside `qcore-compiler`'s `nullable_removed` test
binary. That run is discarded, not reported: its `test.exit` of 101 is a build
failure and the later steps' exit files were written empty by the full volume.
The table above is a clean re-run at 88 GB free. The only casualty of the full
disk was the untracked, gitignored `instrument.py`, truncated to zero bytes
mid-write and regenerated; `git status` and `git diff --stat HEAD` confirmed no
tracked file was touched. Free space at the card's start was 257 GB and fell to
36 GB at the first gate launch, so the brief's 60 GB threshold **was** crossed —
it is recorded here rather than worked around, and the gate above is a full
workspace run and not a crate-scoped substitute.

## 10. Residuals and retirements

### Registered in `acceptance_matrix.json`

* **`JOINED-PROVENANCE-BOUND-DESCENT-PREMISE`** — the Lean theorem's hypothesis
  is that every non-fixed sweep strictly decreases the measure. That is
  **stated of, not proved of, the Rust transfer**: this is an abstract rule
  module like every other gateway proof (`GATEWAY-LEAN-ABSTRACT-NOT-EXTRACTION`),
  and `Facts::step`, `outgoing` and the null-edge exclusion are premises. The
  premise is not idle — §6 records that the *naive* reading of it is false — so
  it is named rather than assumed. Discharge: an extraction or a per-arm
  monotonicity proof of the four transfer arms and the null-edge exclusion.
* **`JOINED-PROVENANCE-DECLINE-ARM-UNREACHABLE`** — §7. Discharge: a graph that
  provably exceeds its own bound (which would falsify the theorem), or a
  recorded decision that the arm stays untestable through `derive` by
  construction.
* **`JOINED-PROVENANCE-BOUND-COST-UNMEASURED`** — the bound buys sufficiency,
  not speed. Measured sweep work over the corpus rises 803,004 → 1,074,540
  round-edge steps (**+33.8%**, and immeasurably small against a ~7-minute
  audit). What is *not* measured is the cost of a graph that actually
  approached its bound: `rd_kafka_mock_handle_ConsumerGroupHeartbeat` would cost
  9,318,240 round-edge steps at its bound of 10,785 against the 127,872 it
  actually costs, and a larger body's bound grows without a ceiling. Discharge:
  a measured worst case, or a stated secondary ceiling with its own ground.
* **`JOINED-PROVENANCE-BOUND-NOT-TIGHT`** — the proved bound exceeds the
  observed maximum by one to two orders of magnitude (median 378 against a
  corpus maximum of 148 rounds). No tighter bound is proved; the classical
  candidates (loop-connectedness for a round-robin sweep in reverse postorder)
  do not apply to this Jacobi schedule unchanged. Discharge: a tighter proved
  bound, or a recorded decision that tightness buys nothing while nothing
  approaches the loose one.

### Retired

* `JOINED-PROVENANCE-ROUND-BOUND` — §3.4, with both halves.
* `JOINED-PROVENANCE-DECLINE-INDISTINGUISHABLE` — §2.
* `JOINED-PROVENANCE-FIXED-POINT-BOUND-UNMEASURED` — §3.

### Not retired

* `JOINED-PROVENANCE-RELEASE-NULLABLE` stands. It is a **decision** about a
  fail-closed premise, invariant 1 constrains its direction, and it is not this
  card's to take. §8 re-measures its consequence on the final tree.

## 11. Reproduction

```sh
git submodule update --init --recursive
cargo build --release -p qcore-compiler --example cycle19_fluent_bit_libraries_audit \
    --features qcore-c-gateway/provenance-rounds-census
QCORE_PROVENANCE_ROUNDS=<out>.jsonl QCORE_PROVENANCE_ROUNDS_CAP=1024 \
    python3 -B measure/joined-provenance-round-bound/run_audit.py census <binary>
python3 -B measure/joined-provenance-round-bound/census.py <out>.jsonl
python3 -B measure/joined-provenance-round-bound/byte_identity.py \
    measure/joined-provenance-round-bound/runs/census \
    docs/reports/joined-provenance-call-frames/artifacts
python3 -B measure/joined-provenance-round-bound/leaves.py census after
python3 -B measure/joined-provenance-round-bound/mutations.py
```

Raw logs stay worktree-private under `measure/` per `CLAUDE.md`'s hygiene rule.

## 12. Integrator reconciliation list

1. **Ledger headline numbers: nothing to change.** The corpus row is unmoved on
   every axis — leak **272**, `unknown_alias` **4,308**, `unsupported` **49,922**,
   `summary_conflict` **1**, zero unit and zero function status moves. The
   baseline pointer stays `docs/reports/joined-provenance-call-frames/artifacts/`;
   this card publishes no new arbiter archive because it reproduces that one byte
   for byte. **Please do not advance the baseline pointer to this card.**
2. **Standing constraint 10b — proposed amendment text.** After the sentence
   *"No caller custody is seeded, calls forget prior relations, and cyclic CFGs
   derive no facts."* — which the wave-35 card already superseded — append:

   > The cyclic iteration's round cap is **this graph's own proved bound**,
   > `nodes * (1 + sites + sites * names)`
   > (`QCore.JoinedProvenanceRoundBound.rounds_are_bounded_by_the_graph_budget`),
   > and not a constant: card JOINED-PROVENANCE-ROUND-BOUND (wave 37) measured
   > the flat 64 losing facts at 9 of 159 sweeping visits and replaced it.
   > Reaching the cap would falsify that theorem rather than exhaust a guess, so
   > the declining arm is unreachable from `derive`
   > (`JOINED-PROVENANCE-DECLINE-ARM-UNREACHABLE`) and is retained only as the
   > fail-closed answer. **Do not restore a constant cap**, and do not weaken the
   > declining arm: it is what makes the bound safe to raise. The theorem's
   > descent hypothesis is stated of, not proved of, the Rust transfer
   > (`JOINED-PROVENANCE-BOUND-DESCENT-PREMISE`).

   Land it **in these words** or record it `declined` with its ground.
3. **The 1,272 → 1,273 correction** (§1) is a transcription repair, not a count
   move. If any tracker or brief carries 1,272, please correct it there and say
   so; no archived record is edited by this card.
4. **Four residuals to register** and **three to retire** — §10, exact names.
   `JOINED-PROVENANCE-RELEASE-NULLABLE` is explicitly **not** retired.
5. **A note for whoever next widens the consumer.** §5's zero is a fact about
   `release_site` being the relation's only reader. The moment a second consumer
   lands, the 10 newly derived sites across 4 functions become live inputs
   without any further change to this module — so that card inherits an
   un-exercised surface this one created. It is worth a line in its brief.
6. **Diagnostics namespace: nothing allocated, nothing widened.** This card adds
   no code and reaches no new condition; it is not an instance of the
   condition-extension rule. It reserves no keyword and moves no lexer rule, so
   it does not engage standing constraint 14 either.
7. **Standing constraint 10b is amended *in this card's tree*,** in the words of
   item 2, rather than only proposed — per `AGENTS.md`'s sync rule, an increment
   that lands a standing rule adds it to the constraints section itself. Please
   verify the wording survived the merge rather than re-deriving it; the card
   also appends `joined-provenance-round-bound` to that constraint's evidence
   pointer list.
8. **Two ledger numbers this card moves, neither of which it edits.** The Merged
   gate row's **Lean audits 135 → 136** (this card adds
   `AuditJoinedProvenanceRoundBound`) and its workspace test count are inside a
   sentence explicitly provenanced to the **wave-35 close's own measurement on
   its own base**, so this card does not overwrite them — a card's gate is not
   the integration tip's. Please fold them in at the close. The matrix's derived
   `lean audits` field already reads 136 because the generator measures it.
9. **Matrix regeneration provenance — caught and repaired, worth a line.** The
   first regeneration was `build_matrix.py .` with no `--gate`, and it silently
   **downgraded** the gate block's `measurement_provenance` from
   `integrator_supplied (docs/reports/wave35-close/gate.json)` to a generic
   "these numbers were NOT measured" warning — a whole-file regeneration
   clobbering an earlier reconciliation, which is exactly what constraint 25
   forbids. It was re-run as
   `build_matrix.py . --gate docs/reports/wave35-close/gate.json` and the
   attribution is restored; the committed diff touches only `named_followups`,
   `lean_audits` 135 → 136 and the audit-file list. **Any card that regenerates
   this matrix should pass the standing close's `--gate` or check that line
   afterwards** — the generator's default is lossy and says so only on stdout.
10. **Nothing is archived under this card's `artifacts/`,** deliberately: an
    arbiter that reproduces the baseline byte for byte has no new archive to
    publish, and adding one would create a second copy of the same bytes for a
    later card to reconcile. The reproduction commands in §11 regenerate it.

## Published beside by `WAVE-38-CLOSE` (2026-09-18) — the §7 harness

Appended, not edited (constraint 21): every line above the rule is the frozen
card text. §7's `measure/joined-provenance-round-bound/mutations.py` (seven
mutations, six caught, one expected survivor) lives on a gitignored,
worktree-private path and is not in the tree; no per-mutation log was
published (`ls` at `4f663934`: `README.md` alone, the absence `DOCS-COMPACT-7`
also noticed). The six-of-seven count is therefore **measured once by the
card, harness not shipped**. Recorded under
`W36-W37-MUTATION-HARNESSES-UNSHIPPED` (card `STABILITY-REEVAL-3`, wave 38);
the card-report convention in `CLAUDE.md` now says a mutation count is cited
only with its script and its per-mutation log in the report directory. This
note changes no code and re-runs nothing.

# CG04-LOOP-SITE-GENERATIONS: generation-indexed acquisition sites

Soundness repair, wave 48 round 5, found by
[CG04-CONDITION-ACQUISITION](../cg04-condition-acquisition/README.md) §3 and
approved by the user with priority. Base: local `main` = `origin/main`
`ca634dc1` (later than the brief's `3e69322c`, which it contains).

- **Rule:** [gateway 11a](../../agents/gateway.md#constraint-11a), new;
  [10x](../../agents/gateway.md#constraint-10x) amended (placement guard
  lifted).
- **Lean:** `QCore.LoopSiteGenerations`,
  `proof/AuditLoopSiteGenerations.lean` (36 theorems);
  `QCore.ConditionAcquisition.placement` gains its `generations` input
  (`with_generations_every_split_is_placed`).
- **Prediction:** [PREDICTION.md](PREDICTION.md), frozen at `ebe68a53`
  (deviation note inside: the repair was written and compiled, not run,
  before it was frozen).

## 1. The defect, reproduced

The flow names a resource by its acquisition node. `create_resource`
(`legacy_c.rs`) wrote the site `LIVE` when a loop re-ran the acquisition,
over a possibly live earlier generation, and the refuted nullable
acquisition's failure edge then removed the site with the earlier
generation in it. On the base, with the shift disabled:

| Shape | Base | Now |
|---|---|---|
| `for (;;) { p = q_maybe_alloc(); if (p == 0) break; q_observe(p); }` | **Safe** | Unsafe (leak) |
| `do { p = q_maybe_alloc(); if (p) q_observe(p); } while (p != 0);` | **Safe** | Unsafe (leak) |
| `while (flag) { p = q_maybe_alloc(); if (p == 0) return; q_observe(p); }` | Unsafe | Unsafe |

Every earlier generation leaks in the first two; both are real leaks of the
source. The claim of the ledger entry reproduced exactly.

## 2. Exposure (measured before any fix)

A behaviour-neutral census build (`030e90a5`) recorded, from the settled
states of every flow run (W4 derivation runs included), each acquisition node
whose own site the state it runs in already mentions (parity
`position: "loop-site-generation"`, [exposure.py](scripts/exposure.py)).

| Route | Functions with a re-acquisition | Earlier generation live without an unresolved effect | Of them Safe |
|---|---|---|---|
| corpus | 31 (29 Unknown, 2 Unsafe) | 0 | **0** |
| system | 5 (Unknown) | 0 | **0** |
| tail-pause slice | 3 (2 Unknown, 1 Safe) | 0 | **0** |

In each of the 39 the earlier generation was already unresolved (a refused
effect reached it) or released. No W4 summary was validated on an exposed
body. So no current Safe depended on the defect: the corrections this card
exists for are **none** in the measured populations, and the repair closes
the hole for code the widening cards will admit.

## 3. The repair (gateway rule 11a)

`loop_generations.rs` (decision table, identity encoding) and the route-G
adapter `legacy_loop_generations.rs`, with thin hooks in `legacy_c.rs` and
`legacy_leak_witness.rs`.

- **Three identities per site:** *current* (the site), *previous*
  (`site | 2^29`) and *older* (`site | 2^30`). Current and previous are each
  one object per execution, so every precise update applies to them; older
  is a summary that may stand for many.
- **The shift**, before a node runs and before the driver's per-node reads
  (joined provenance, refused hand-on): for every site the node produces
  that the state already mentions, previous merges into older (statuses,
  nullability, unresolved effects, aggregate edges and pending replacements
  joined; every binding that named it becomes unknown, so any custody effect
  through it refuses; two generations' element ranges become `Unknown`),
  current is renamed previous, and the node's own transfer creates a fresh
  current. The failure edge of the fresh acquisition removes only it.
- **The refusal** (`decide`): the merge refuses when the summary may be live
  and a place that can update it without a binding holds it (field link,
  containment, pending reallocation, publication, an out slot the node does
  not overwrite), or when one side may be live and the other's unresolved
  effect would hide it. Every input only grows with the state, so the
  decision is monotone (`the_decision_is_monotone`): the settled state
  decides as every iterate did. A first version used "owes" (live and not
  unresolved) as the input; the census showed its own refusal flipping it
  back on the settled state (a decision not monotone in the state), so it
  was replaced before the final measurement (`1f9103e7`).
- **A refused re-acquiring node** leaves a shifted site it did not create
  present, live and unresolved (`settle_refused`): the call C ran may have
  produced it. Without this, a joined state paired another iteration's owed
  generation with an unknown binding and reported a false leak (the growing
  buffer `q = realloc(p, n); …; p = q;`).
- **Every route** that creates a site goes through the shift, because it
  runs on `instruction_produced_sites`: statement acquisitions (`Acquire`,
  a contract call's `Owned` result), `Out`-parameter acquisitions,
  `Reallocated` results, descriptors, a foreign unlink's element, an element
  store's and a sub-object store's inner call, and rule 10x's condition
  split. `kind_of` and `acquisition_line` read every generation through its
  base site.
- **Rule 10x's guard**, as its own step: `placement` takes
  `SITES_ARE_GENERATION_INDEXED` and refuses only for a flow without
  generations; the production value is `true`.

Shared foundations (`elaboration.rs`, `typed_place.rs`,
`call_instantiation.rs`): **untouched**. `condition_acquisition.rs` (CG04's
module): `placement` gains a third input, `PLACEMENT_VECTORS` four rows;
no variant changes meaning.

## 4. Measurements

Runs `private/cg04lsg/{before,step1,final}*` of this worktree and
`/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/cg04lsg-*`,
every exit 0, `--jobs 8`, [measure.sh](scripts/measure.sh); artifacts by
[artifacts.sh](scripts/artifacts.sh). Binaries
([binaries.txt](artifacts/binaries.txt)): BEFORE at `030e90a5`, step 1 at
`266fcf9c` (a measurement-only commit setting the guard's input `false`,
reverted by `99b0701a`), final at `99b0701a` (tree identical to `1f9103e7`).
The intermediate runs `after1`/`after2` measured the first, non-monotone
decision and are superseded (kept in `private/`, not evidence).

| Route | BEFORE | Step 1 (generations, guard kept) | Final (guard lifted) |
|---|---|---|---|
| corpus (`66910c10`) | 586 / 10 / 4,238, `c012e785…` (byte-identical to CG06's AFTER) | 586 / 10 / 4,238, `7cdcba4e…` | **586 / 10 / 4,238**, `454ecd00…` |
| system (`046accf6`) | 34 / 2 / 198 | 34 / 2 / 198 | **34 / 2 / 198** |
| tail-pause slice (`90978ac6`) | 26 / 1 / 242 / 1 Pending | the same | **the same** |

**No function changes status anywhere. No Safe → non-Safe move: there is
no correction to name.**

**Step 1, leaf changes** ([changed-step1.txt](artifacts/changed-step1.txt)):
28 corpus functions, all same-status.

- 26 Unknown functions gain rule-11a refusals (+37 leaves in total, with the
  step-2 one 38): each re-acquires a site whose earlier generation already
  carries an unresolved effect (a refusal reached it on the base) while
  another generation may be live, the case the decision refuses so that the
  merge cannot hide it. `mk_vhost_destroy` also loses one `unknown_alias`
  (`ep`: the foreign unlink's element is now the fresh generation).
- `rd_kafka_sasl_safe_string` (Unsafe on the base): +1 refusal at
  `d = safe = rd_malloc(len + 1)`, taken only when `pass == 0`.
- `rd_kafka_snappy_java_uncompress` (Unsafe on the base, with real pass-2
  leaks at `snappy.c:1648` and `1660`: `return NULL` without freeing
  `outbuf`): +1 refusal and +4 leak leaves at 1680, 1694, 1712 and 1721 on
  `outbuf = rd_malloc(uof)` (1705), which only `pass == 1` runs. The flow
  does not relate the guard to the loop counter, so it re-acquires on an
  abstract second pass and reports the earlier generation. **Classified by
  reading as the flow's imprecision, not source leaks**; registered as
  `LOOP-GENERATION-COUNTER-CORRELATION`, never presented as upstream
  defects.
- System: 4 Unknown functions (`sb_allocate_backlogs`, `sb_destroy_backlogs`,
  `flb_output_plugin_property_check`, `flb_task_create`: +6 refusals, −10
  `unknown_alias`, the route per iteration now resolving to the current
  generation). Slice: 2 Unknown functions (`flb_input_new`,
  `flb_input_plugin_property_check`: +3 refusals).

**Step 2, lifting the guard** ([changed-step2.txt](artifacts/changed-step2.txt)):
the two guarded consultations are admitted and both functions stay Unknown.
`rd_gz_decompress`: the branch refusal and 3 `unknown_alias` leaves go, a
rule-11a refusal (the same `pass == 1` pattern) and a nullable-use leaf on
the failure path come. `rd_kafka_lz4_decompress`: the branch refusal goes,
and the reallocation refuses on `out`, which the loop joins over two
identities. System and slice: byte-identical to step 1.

## 5. Prediction against outcome

| Frozen | Outcome |
|---|---|
| Corpus: no status move | **held** |
| No Safe → non-Safe | **held** (0 exposed) |
| Same-status changes at most in the 31 functions with a re-acquisition; the 2 Unsafe stay Unsafe | **held in substance**: 28 of the 31 at step 1, the 2 Unsafe stay; **missed** that `rd_kafka_snappy_java_uncompress` gains 4 imprecise leak leaves |
| System and slice: no status move | **held**; 4 + 2 Unknown functions change leaves (not predicted) |
| No new provisional Unsafe | **held** at function level |
| Step 2: the 2 guarded consultations admitted, both functions stay Unknown, nothing else moves | **held** |

## 6. Tests and controls

- `tests/loop_site_generations.rs` (10 tests): the pinned shapes against the
  Lean model's verdicts (`SHAPE_VECTORS`, bound to `shapeVectors`); controls
  for a release in each iteration and acquire/release/re-acquire (Safe),
  release only after the loop (a leak), reallocation in a loop (a
  per-iteration buffer Safe; the growing buffer Unknown with no leak claimed),
  an `Out`-parameter acquisition (Safe, and an overwritten one leaks), a
  descriptor (Safe, and a leak), the previous generation released precisely
  (`prev`/`cur`: Safe; **the base accused this correct program of a double
  release**, a false Unsafe the one site produced), the condition form on a
  cycle after the lift, and the sibling pin (§7).
- `tests/unit/legacy_loop_generations.rs`: one fixture per arm of the shift
  (fresh, precise rename, merge with unknown bindings, refusal by a field
  link, by an out slot, by an unresolved summary, collections, an
  unindexable site, the failure edge, the refused node, the census helper).
- `tests/unit/loop_generations.rs`: all 128 decision inputs, monotonicity,
  and the Lean bindings of `STEP_VECTORS` and `IDENTITY_VECTORS`.
- **Flipped deliberately:**
  - `condition_acquisition::the_statement_form_loop_site_defect_is_pinned`
    → `the_statement_form_loop_site_defect_stays_fixed`: both forms (and the
    `while (flag)` one) Unsafe with a leak.
  - `condition_acquisition::an_acquisition_on_a_cycle_keeps_the_refusal` →
    `an_acquisition_on_a_cycle_is_split_with_generations` (step 2): the five
    forms admitted, decided as their statement pairs, the call applied once.
  - `clang_gap_phase1::e09_an_owned_acquisition_in_a_condition_is_refused_and_not_hoisted`:
    the loop form admitted; with a non-null allocator its exit edge is
    retained (rule SCALAR-ROUTE-EXIT-EDGE) and it is not Safe, as its
    statement pair.
  - `indexed_collection::an_array_reallocated_in_a_loop_refuses`,
    `a_break_keeps_the_exit_equation_from_holding`,
    `a_narrower_unsigned_index_is_not_a_counted_loop`: still Unknown, other
    refusal leaves (reasons in the tests).
  - `condition_acquisition` unit table: `PLACEMENT_VECTORS` gains the
    `generations` column.

## 7. Siblings searched

- **Derived aliases:** a derived binding names its base's site, so it
  follows the generation through the rename (and becomes unknown with the
  summary). No separate defect.
- **Collections:** element slots carry sites for diagnostics only; the array
  owner's ranges are keyed by the owner site and are now generation-indexed,
  two generations' ranges never joined. No separate defect.
- **Joined provenance:** derives facts only for sites created off every
  cycle (`JOINED-PROVENANCE-CYCLES`), so no per-site fact stands for two
  generations. No defect.
- **Found, not a generation defect:** the B08 field-link join
  (`join_field_links`) marks a child linked on one side only as unresolved
  **without a diagnostic**, so its leak is withheld from a `Safe` result;
  its own comment claims the exit becomes `Unknown`, which nothing does.
  `cascade_field_links_on_escape` withholds the same way. A synthetic branch
  form (`if (n) b->item = it; else k_release(it); k_drop(b);`) is `Safe`
  while the straight-line form reports the leak P-B08-SHALLOW implies
  (pinned as observed). Its loop form, also Safe on the base, is now refused
  by rule 11a. Sized by a new census of Safe results whose settled exit
  withholds a possibly-live, unresolved identity no diagnostic reported
  (`position: "withheld-obligation"`, [withheld.py](scripts/withheld.py)):
  **0** reported functions in corpus, system and slice. Registered as
  `B08-FIELD-LINK-JOIN-SILENT-UNRESOLVED`.
- **Boundary, stated:** leaks are reported at exits, so a loop with no
  reachable exit (`for (;;) { p = malloc(n); }`) reports nothing; that is
  the property's definition, unchanged by this card.

## 8. Lean

`QCore.LoopSiteGenerations` (36 theorems, audited, no placeholders, no
project axioms): the identity encoding (`identity_vectors_hold`,
`generations_name_their_site`); the decision (`step_vectors_hold`,
`refused_iff_live_and_exposed`, `the_decision_is_monotone`); re-acquisition
cannot discharge the earlier obligation
(`reacquisition_keeps_the_earlier_obligation`,
`an_owed_generation_stays_owed_or_refused`); the failure edge removes only
the new generation (`the_failure_edge_removes_only_the_new_generation`,
`a_live_earlier_generation_survives_the_failure_edge`); the widened state
never discharges (`the_summary_never_discharges`,
`the_summary_never_discharges_along_a_run`, `a_live_summary_is_never_safe`,
`operations_keep_unresolved_reported`, `a_refused_acquisition_is_never_safe`);
the pinned shapes (`the_one_site_flow_hides_the_leak`,
`for_break_reports_exactly_the_concrete_leak`,
`the_joined_exit_reports_the_leak`, `release_each_iteration_is_safe`,
`release_after_the_loop_reports_the_earlier_generations`,
`link_each_iteration_is_refused_from_the_third`, `shape_vectors_hold`).
Bound to Rust by `STEP_VECTORS`, `IDENTITY_VECTORS` and `SHAPE_VECTORS`
(the gateway's verdict on each shape's fixture). Premises stated, not
proved: that the shift renames every mention of an identity, and that every
other rule reports a refusal wherever it leaves an identity unresolved (the
sibling above is where one does not); faithful extraction (trust boundary 28).

## 9. Gate

See [gate.json](gate.json) and [gate/](gate/).

## 10. Ledger

- **Retired:** `CG04-LOOP-SITE-GENERATIONS`. Its DISCHARGE: the flow tells a
  live earlier generation apart at its re-acquisition, reported or refused,
  with Lean parity (rule 11a, `QCore.LoopSiteGenerations`); a measured
  corpus re-pin names every Safe → non-Safe move (there are none, measured
  on corpus, system and slice); the pinned test flips; rule 10x's placement
  guard is lifted with its own re-pin (step 2). Every clause holds.
- **Amended:** `W44-CONDITION-CALL-REFUSED-SHAPES` (its acquisition-on-cycle
  class is gone) and `W44-CONDITION-CALL-ASSIGNMENT` (its off-every-cycle
  premise is lifted).
- **Registered:** `B08-FIELD-LINK-JOIN-SILENT-UNRESOLVED` (the sibling, 0
  measured exposure) and `LOOP-GENERATION-COUNTER-CORRELATION` (4 imprecise
  leak leaves in one already-Unsafe function; refusals in two more).

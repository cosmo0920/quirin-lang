# JOINED-PROVENANCE-CALL-FRAMES — conditional provenance across a call

Wave 36. Card base `60958bf0`, rebased onto `d47e2bb2`. This card owns a corpus
re-pin and merges **after** the wave's declarative arbiter
`W36-CORPUS-CONTRACTS-3`, whose AFTER its BEFORE reproduces byte for byte.

> **Ordinal conflict for the integrator to settle.** The card brief called this
> card wave 36's *first* arbiter and `W36-CORPUS-CONTRACTS-3` the second; that
> card's own increment paragraph also calls itself the *second*; and by merge
> order it is in fact first and this card second. This report therefore
> identifies both cards by **name and merge order** and claims no ordinal for
> itself. Nothing measured here depends on the label.

## 1. Headline, including the part that does not move

**This card moves no leaf, and it was never going to.** Wave 35's
`JOINED-PROVENANCE-CYCLES` measured two independent blockers holding at every
one of the 14 sites that reach `release_site`'s premise gate: the derived facts
do not name the released pointer, *and* the site is in `nullable_sites`. This
card is chartered against the first blocker only. The second —
`JOINED-PROVENANCE-RELEASE-NULLABLE` — is a decision that is not this card's to
take, invariant 1 constrains its direction, and it still stands at every site.
So a perfect answer here yields zero corpus movement by construction, and the
value delivered is a **named cause** and a **measured counterfactual**, not a
count.

What did move is the measurement:

| | before this card | after |
|---|---:|---:|
| sites reaching `release_site`'s premise gate | 14 | 14 |
| of those, the facts **name** the released pointer | **0** | **6** |
| distinct `(function, site)` pairs behind those visits | 6 | 6 |
| of those distinct sites, named | **0** | **3** |
| leaves added / removed | — | **0 / 0** |

The naming premise now passes at **6 of 14** visits (3 of 6 distinct sites),
where wave 35 measured 0 of 14. Every site whose relation had *ever* named the
released pointer is now named at the release. The remaining 8 visits are not a
call-frame problem at all, and §2 says what they are.

## 2. Attribution first — which rule loses the name

The brief asked for the 14 to be attributed individually to the rule that loses
the name, and for the card's design to follow that measurement rather than the
first suspect. The analyzer was instrumented (not a source-text classifier):
every removal of a name from a site's binding set is charged to the rule that
performed it, and one JSON line per reached premise gate carries the causes.
`scripts/instrument.py` builds the instrumented module from the shipped one by
17 uniquely-anchored edits; `logs/rb-premise-before.jsonl` and
`logs/rb-premise-after.jsonl` are its output on `d47e2bb2`'s tree. (The same
measurement was taken before the rebase and is identical in every row; §6
records that check.)

The 14 visits are **6 distinct `(function, site, source)` triples** — the
worklist reaches some of them more than once. Attribution over the base tree:

| function | site | released name | visits | cause the name is absent |
|---|---:|---|---:|---|
| `rd_kafka_oidc_token_metadata_azure_imds_refresh_cb` | 0 | `token_endpoint_url` | 4 | **`call_forget`** (+ a join that intersected the loss away) |
| `rd_kafka_jwt_b64_decode_payload` | 1 | `payload` | 1 | **`call_forget`** ×8 |
| `rd_kafka_jwt_b64_decode_payload` | 0 | `converted_src` | 1 | **`call_forget`** ×20 (+ null edge) |
| `rd_kafka_mock_handle_ConsumerGroupHeartbeat` | 0 | `SubscribedTopicNames` | 2 | fixed point **declined: round bound** |
| `rd_kafka_mock_handle_ShareGroupHeartbeat` | 0 | `SubscribedTopicNames` | 4 | fixed point **declined: round bound** |
| `rd_kafka_snappy_java_uncompress` | 0 | `outbuf` | 2 | fixed point **declined: no off-cycle site** |

Read as a cause histogram over visits: `call_forget` **6 of 14**, declined at
the round bound **6 of 14**, declined for having no off-cycle site **2 of 14**.

**Calls-forget is the cause at every site whose relation ever named the
released pointer** — all three, 6 of 6 such visits — so the card's premise
holds and its design stands. The other three sites derive *no facts at all*:
their whole function declined, so there is no name to lose. Attributing them to
"calls forget" would have been wrong, and the instrumentation is what
distinguishes the two.

Two of the three declining sites decline at the **iteration round bound**
(`FIXED_POINT_ROUNDS = 64`), which wave 35 did not separate from the
off-cycle-site restriction and which is a **new finding**, recorded below as
`JOINED-PROVENANCE-ROUND-BOUND`. The third is
`JOINED-PROVENANCE-CYCLE-SITES`, already named and already a measured boundary.

Where the `join_intersect` and `null_edge` causes appear beside `call_forget`,
they are downstream of it: the name was lost at a call on one path, and the
join then intersected it away. After the change those rows show only the null
edge — which is the relation correctly declining to claim a site on the branch
where the site does not exist — and the name survives to the release.

## 3. What landed

One arm, in `crates/qcore-c-gateway/src/legacy_joined_provenance.rs`. The
`ContractCall` arm of `Facts::step` no longer clears the binding map. It now:

* forgets the **`Reallocated` parameter**'s name, the result having replaced
  the value there — added by this card;
* keeps a name only when it is in the **frame**, the closed list of names whose
  slot no lvalue but the name itself designates; everything else is forgotten
  exactly as before.

`Facts::step` takes the frame as a parameter, and `outgoing` passes
`cfg.conditional_names`. That set is pre-existing and this card only reads it:
`unique_local_names ∩ frame_protected − formals` — an unshadowed object of
automatic storage duration this body declared, with no `&` anywhere in the body
taking its address, and not a formal.

**Why a call cannot write such a name.** The premise is the same C object model
constraint 10a already rests on under the no-undefined-behaviour reading
(`AOM-POISON-NO-UB`): a callee reaches only what its arguments and file-scope
objects give it. A name in the frame has no `&` anywhere in the body, so no
pointer to its slot exists for a callee to hold; it has automatic storage
duration, so it has no linkage either. Passing the name as an argument hands
over its *value*, never its slot. This is constraint 10a's premise 2 applied to
a call instead of to a refused statement.

**The places the call does write are already gone** before the frame test runs:
`rebound_names` forgot the call's destination and every `Out` slot. That
ordering is load-bearing and is what two of §5's mutations falsify.

Nothing else moved: `release_site`'s other premises are untouched, no caller
custody is seeded, the `UnknownAlias` identity refusal is retained, and the
`Unknown`/`AggregateStore` arms are unchanged (§7 records why `AggregateStore`
was deliberately not taken).

## 4. The Lean

`proof/QCore/JoinedProvenanceCallFrames.lean`, audited by
`proof/AuditJoinedProvenanceCallFrames.lean` (registered in `proof/QCore.lean`).
Abstract rule theorems, **not extraction**
(`GATEWAY-LEAN-ABSTRACT-NOT-EXTRACTION`). All nine depend on **no axioms at
all** — not even `propext`.

| theorem | what it settles |
|---|---|
| `call_preserves_sound_claims` | the rule itself: a sound claim restricted to the frame is sound after the call |
| `a_call_never_adds_a_name` | the transfer is a restriction, so no call can manufacture a name — the direction invariant 1 governs |
| `a_smaller_frame_claims_less` | monotone in the refusing direction: an unrecognised place only weakens |
| `a_written_name_is_outside_every_frame` | the fail-closed half — a name the call writes is in no frame the call respects, which is why the destination, `Out` slots and a `Reallocated` parameter must be forgotten first |
| `the_empty_frame_is_the_old_blanket_forget` | **conservative extension, exactly**: the pre-card analyzer is this rule at `F = ∅` |
| `the_empty_frame_stays_sound` | declining at a call stays sound |
| `the_frame_keeps_what_the_blanket_forget_dropped` | non-vacuity: the generalisation is a real one, not a restatement |
| `a_call_edge_is_transfer_sound` | the composition — the new arm discharges precisely the `TransferSound` hypothesis `JoinedProvenanceCycles.fixed_point_is_sound` already carries |
| `a_graph_with_call_edges_is_sound` | the same, stated end to end, so one citation covers it |

`a_call_edge_is_transfer_sound` is the conservative-extension claim in its
operative form: `QCore.JoinedProvenanceCycles`' acyclic-uniqueness and cyclic-
soundness theorems are **reused unedited**, not reproved. This card supplies a
new way to discharge their existing hypothesis.

Two things are **premises, not conclusions**, and both are named residuals in §7:
that the token reader's frame really is address-never-taken and automatic
(`CALL-FRAMES-FRAME-READER-PREMISE`), and the C object model itself
(`AOM-POISON-NO-UB`).

## 5. Falsification — every new arm mutated

`scripts/mutations.py`, output in `logs/mutations.log`. Each mutation breaks one
rule and names the test that must go red; the source is restored with
`write_text`, which advances the mtime, so cargo cannot skip the rebuild and
report a stale binary (`MUTATION-HARNESS-MTIME-RESTORE`). **6 of 6 caught, tree
restored green.**

| mutation | caught by |
|---|---|
| the call clears every relation again (pre-card behaviour) | `a_call_no_longer_forgets_a_relation_it_cannot_write` |
| the same, through `derive` rather than `step` | `an_acquisition_survives_an_intervening_call_end_to_end` |
| the frame test dropped: every name preserved unconditionally | `a_call_forgets_a_name_the_frame_list_does_not_carry` |
| the `Reallocated` parameter no longer forgotten | `a_call_still_forgets_a_reallocated_parameter` |
| the `rebound_names` forget skipped | `a_call_still_forgets_its_own_destination` |
| `rebound_names` stops claiming `Out` slots | `a_call_still_forgets_an_out_slot` |

**A harness defect found and fixed rather than reported as coverage.** The
`Out`-slot mutation first came back MISSED. The cause was not a missing test:
the bare anchor `matches!(mode, ResourceParameterMode::Out { .. })` occurs
**eight** times in `legacy_c.rs`, and `replace(old, new, 1)` mutated an
unrelated rule at line 3160 instead of `rebound_names` at line 5461. With the
anchor widened to `rebound_names`' whole `filter_map` — verified unique before
re-running — the mutation is caught. A non-unique anchor reports a green suite
as a gap, which is the same error in the other direction; this is recorded
because the harness is archived and will be re-run.

**One arm was removed rather than shipped unfalsifiable.** A
`bindings.retain(|_, names| !names.is_empty())` line kept the map tidy after the
frame test. No test could distinguish it from its absence *semantically* —
`release_site` asks only `facts.get(&site)?.contains(source)`, so a missing
entry and an empty entry are the same fact — and the four tests that appeared to
catch it were asserting the shape of the map. The assertions were rewritten to
read the site's **names** (`names_of_site`), and the line was deleted. The
pre-existing code already leaves empty name sets around after `outgoing`'s own
retain, so the line was an inconsistency rather than hygiene.

## 6. The re-pin

Rebased onto `d47e2bb2`, which carries wave 36's declarative arbiter
`W36-CORPUS-CONTRACTS-3` (merge `de3a914a`) and `STABILITY-REEVAL #2`. BEFORE is
that tree with this card's module reverted; AFTER is the card tip.

**BEFORE reproduces `docs/reports/w36-corpus-contracts-3/artifacts/` byte for
byte** on both verdict-bearing artifacts — `diagnostics a7aa3b5f9ab193a0`,
`chunk-contracts c4f438e4ed3dab3c` — so the chain is attributable through this
wave's declarative arbiter to wave 35's. (`source-effects` and `contracts`
differ by checkout root and identity digests, identically in BEFORE and AFTER,
exactly as every preceding arbiter recorded.)

The three-way leaf split over the pinned 185-unit corpus, leaf identity
`(unit file, function, preprocessed line, category, description)` — the
**internal** coordinate per constraint 12a:

| | |
|---|---:|
| added (only in AFTER) | **0** |
| removed (only in BEFORE) | **0** |
| unmoved | **54,503** |
| unit status moves | **0** |
| function status moves | **0** |

Every count stands exactly as the declarative arbiter left it: `leak` **272**
(constraint 7's chain untouched, no leaf added or withdrawn), `unknown_alias`
**4,308**, `unsupported` **49,922**, `summary_conflict` **1**, functions
**205 Safe / 84 provisional Unsafe / 4,545 Unknown**. AFTER's verdict-bearing
artifacts are byte-identical to BEFORE's, so this is not a histogram agreeing by
coincidence — the bytes are the same. No archive was edited (constraint 21).

**The attribution was re-taken on this tree rather than carried.** The
declarative arbiter's six declarations change which callees are contracted,
which could have moved this card's population. Re-running the instrumentation on
`d47e2bb2`'s own tree gives the **same 14 visits, the same 6 distinct sites, the
same causes and the same 0 → 6 naming move** as the pre-rebase measurement: none
of `cfl_sds_destroy`, `rd_kafka_mem_free`, `cmt_destroy`, `cJSON_Delete`,
`mk_print` or `ctr_destroy` sits behind any of these six sites. Measured
(`logs/rb-premise-before.jsonl`, `logs/rb-premise-after.jsonl`), not assumed.

**Why zero movement is the expected result and not a disappointment.** The
naming premise is one of seven at `release_site`. It now passes at 6 of 14
visits, but `nullable_site` holds at **all 14** and `unresolved_effect` at 5,
so no visit has every premise pass. A leaf can only move when all seven do. The
card's yield is therefore the measurement in §2 and the counterfactual in §7 —
and the fact that a precision improvement of this size moved nothing is itself
evidence about where the remaining blocker lies.

**The rebase was verified, not assumed.** The four audit runs were produced by
binaries built at `4e8e5bb3` (the declarative arbiter's tip before its merge
commit). `d47e2bb2` is **source-identical** to `4e8e5bb3` under `crates/`,
`contract/`, `compiler/` and `proof/`, and the baseline artifacts are unchanged
between them, so the runs measure main's tree. That containment was checked with
`git diff --stat` over those paths rather than inferred from the merge subjects.

## 7. Boundaries and named follow-ons

* **`JOINED-PROVENANCE-RELEASE-NULLABLE`** — carried forward from wave 35,
  unchanged and still universal: all 14 visits are in `nullable_sites`. This
  card's contribution is the **counterfactual**, so the decision has a measured
  consequence. If the nullable premise were relaxed and nothing else changed,
  exactly **one** of the 14 visits would have every premise pass and the
  conditional release would settle: `rd_kafka_jwt_b64_decode_payload` site 1,
  released name `payload`, whose blocking set after this card is
  `nullable_site` alone. The other five named visits also carry
  `unresolved_effect`; the eight unnamed ones derive no facts. Discharge
  unchanged: decide whether a nullable site may be settled when the release is a
  declared `nullable` contract, and re-measure.
* **`JOINED-PROVENANCE-FACTS-NAME-NOTHING`** — **discharged by this card.** Its
  discharge condition was "attribute those 14 individually to the rule that
  loses the name"; §2 is that attribution, and the naming premise now passes at
  6 of 14 rather than 0 of 14. The residual is **retired** and must be removed
  from `named_followups`.
* **`JOINED-PROVENANCE-CALL-FRAMES`** — wave 31's own follow-on ("prove
  preservation across calls") is **discharged**: §3 lands it and §4 proves it.
  **Retire** it from `named_followups`.
* **`JOINED-PROVENANCE-ROUND-BOUND`** — **new.** Two of the six distinct sites
  (6 of 14 visits) derive nothing because the greatest fixed point exhausts
  `FIXED_POINT_ROUNDS = 64` and declines the whole function —
  `rd_kafka_mock_handle_ConsumerGroupHeartbeat` and
  `rd_kafka_mock_handle_ShareGroupHeartbeat`. Wave 35 recorded these inside the
  undifferentiated "not named by facts" bucket; separating the two decline
  reasons is what exposes them. Declining is sound
  (`weakening_stays_sound`), so this is a **precision** boundary and not a
  soundness one. Discharge: measure how many rounds those two functions
  actually need, and either raise the bound with the cost measured or establish
  a termination argument that removes it.
* **`JOINED-PROVENANCE-CYCLE-SITES`** — unchanged, and now measured to be the
  cause at exactly one of the six distinct sites
  (`rd_kafka_snappy_java_uncompress`). Discharge as wave 35 wrote it.
* **`CALL-FRAMES-FRAME-READER-PREMISE`** — **new.** The rule's soundness rests
  on `cfg.conditional_names` really being address-never-taken automatic locals.
  That set is built by pre-existing code (`frame_protected_locals`,
  `unique_local_names`, cards AOM-BINDING-POISON and C-GATEWAY-WAVE) which this
  card reads and does not write, and it is therefore **not** among the arms §5
  mutates. Its exclusions — static duration, file-scope, address-taken,
  shadowed, formal — are each fail-closed by construction, but the frame reader
  is now load-bearing for a *second* rule. Discharge: a mutation suite over
  `frame_protected_locals`' four exclusions driven from the call-frame side, or
  a proof that each exclusion is necessary for `FrameRespecting`.
* **`CALL-FRAMES-ALLOWLIST-LOCAL-ONLY`** — **new, and a vacuity finding about
  this card's own work.** The frame test inside `step` cannot currently fire
  through `derive`: `outgoing` applies the same `conditional_names` membership
  test after **every** node, so every name reaching a call is already in the
  frame. The guard is therefore a *local* enforcement of the premise, driven by
  a direct `step` test (`a_call_forgets_a_name_the_frame_list_does_not_carry`)
  and by §5's mutation, but no CFG can present it with an unrecognised name
  today. It is kept deliberately — without it the arm's soundness would depend
  on a caller's ordering rather than on its own text, which is worse — and it is
  recorded rather than presented as verified-in-place. Discharge: either derive
  the invariant explicitly (assert it, or restrict the type) so the guard is
  provably redundant, or introduce a route that reaches `step` without
  `outgoing`'s filter and drive the arm from it.
* **`AOM-DESTROY-JOINED-PROVENANCE`** — untouched, and **no claim is made**.
  Wave 35 measured `input_chunk_get`'s CFG acyclic with its one site off-cycle,
  and its recorded blocker is an address-taken `ic` binding crossing refused
  stores. An address-taken name is excluded from `frame_protected_locals`, so it
  is excluded from `conditional_names`, so it is excluded from this card's
  frame **by construction**: this rule cannot reach that residual, and it stands
  exactly as written.

## 8. The gate

Run at the card tip, rebased onto `d47e2bb2`, with
`git submodule update --init --recursive` done first. Every exit captured to a
file, never through a pipe.

| command | result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **4,586 passed / 0 failed / 8 ignored**, 232 result summaries, exit 0 |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 scripts/check_lean_audits.py` | **133 Lean audits**, exit 0 |

**The host row the brief asked about was green here.**
`closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request`
passed in the full `--no-fail-fast` suite, twice — once before the rebase and
once after — at a machine load average of **~38 on 16 cores**, with several
sibling cards building concurrently. `W36-CORPUS-CONTRACTS-3` reported it red at
load ~40 and green 12/12 in isolation; this card did **not** reproduce the red,
which is reported as the observation it is and not as evidence that the repair
at `e3d3a2ce` is either sound or unsound. Two green runs at high load are
consistent with both a fixed row and a load-sensitive one.

The disk check the brief requires: **129 GB free** at the time of the gate, above
the 60 GB floor, so the full gate ran rather than the crate-scoped subset.

## 9. For the integrator

Apply in these words.

1. **Ledger, the corpus baseline pointer.** The baseline moves to
   `docs/reports/joined-provenance-call-frames/artifacts/`, described as "wave
   36's second merged arbiter, whose BEFORE reproduces
   `w36-corpus-contracts-3`'s `diagnostics.json` and `chunk-contracts.json`
   byte for byte and which moved **zero** leaves, zero functions and zero
   units", with the chain list gaining `joined-provenance-call-frames` over
   `w36-corpus-contracts-3`.

2. **Ledger, the Cycle 19 corpus row. No count changes.** After the declarative
   arbiter's clause, add: "`JOINED-PROVENANCE-CALL-FRAMES` then moved **zero**
   leaves, zero functions and zero units — its AFTER is byte-identical to its
   BEFORE — so all counts above stand as `W36-CORPUS-CONTRACTS-3` left them:
   conditional provenance is now preserved across a call the frame shows it
   cannot write, which takes `release_site`'s naming premise from 0 of 14 to
   **6 of 14**, while the nullable-site premise still blocks all 14."
   272 / 4,308 / 49,922 / 1 and 205 Safe / 84 provisional Unsafe / 4,545 Unknown
   are all unchanged.

3. **`named_followups` — TWO REMOVALS, and they are the point of this item.**
   The union merge has lost retirements three waves running, so both are named
   explicitly:
   * **REMOVE `JOINED-PROVENANCE-FACTS-NAME-NOTHING`** — discharged. Its
     condition was "attribute those 14 individually to the rule that loses the
     name"; §2 is that attribution.
   * **REMOVE `JOINED-PROVENANCE-CALL-FRAMES`** — discharged. Wave 31's
     follow-on asked for preservation across calls to be proved; §3 lands it and
     §4 proves it.

4. **`named_followups` — three additions**, each with its discharge condition as
   written in §7: `JOINED-PROVENANCE-ROUND-BOUND`,
   `CALL-FRAMES-FRAME-READER-PREMISE`, `CALL-FRAMES-ALLOWLIST-LOCAL-ONLY`.

5. **Standing constraint 10b — a required amendment, in these words.** The
   sentence "No caller custody is seeded and calls forget prior relations" is
   **now false** and must not be left standing. Replace it with:

   > No caller custody is seeded. A **call** forgets every relation it could
   > have written and keeps only those it provably could not (card
   > `JOINED-PROVENANCE-CALL-FRAMES`, wave 36): it forgets its destination and
   > every `Out` slot — both already removed by `rebound_names` — and a
   > `Reallocated` parameter, and it forgets every name outside
   > `cfg.conditional_names`, the unshadowed automatic locals of this body whose
   > addresses it never takes. A name on that list designates a slot no callee
   > can reach under the same no-undefined-behaviour C object model constraint
   > 10a rests on, so the call cannot change what it denotes; passing the name
   > as an argument hands over its value, never its slot. An unrecognised name
   > is always forgotten, and the rule only ever weakens
   > (`QCore.JoinedProvenanceCallFrames.a_call_never_adds_a_name`), the pre-card
   > blanket forget being exactly this rule at the empty frame
   > (`the_empty_frame_is_the_old_blanket_forget`).

   and append `joined-provenance-call-frames` to that constraint's evidence
   pointer list. Please land this **in these words or record it `declined` with
   a ground** — a "reconciled" softening would leave the constraint asserting
   something the code no longer does.

6. **Lean audit count.** `check_lean_audits.py` now passes **133** audits
   (`AuditJoinedProvenanceCallFrames.lean` added, registered in
   `proof/QCore.lean`). Update the merged-gate row's Lean number when it is
   next re-measured at the integration tip.

7. **An ordinal conflict to settle, not silently resolved here.** The brief
   called this card wave 36's *first* arbiter; `W36-CORPUS-CONTRACTS-3`'s own
   increment paragraph calls **itself** the *second*; by merge order it is
   first and this card second. This report and this card's increment paragraph
   therefore identify both cards by **name and merge order** and claim no
   ordinal. One of the two increment paragraphs needs correcting, and the choice
   is the integrator's.

8. **A note on this card's own honesty check, offered rather than applied.**
   `CALL-FRAMES-ALLOWLIST-LOCAL-ONLY` (§7) is a `STABILITY-REEVAL`-style vacuity
   finding about code this card is shipping: a guard that cannot fire through
   the production route today. It is recorded as a residual with a discharge
   condition rather than removed, and the reasoning is in §7. If the next
   `STABILITY-REEVAL` firing wants a worked example of step 1 applied by the
   card that created the arm, this is one.

## 10. Reproduction

Scripts are archived under `scripts/`; they read the pinned tree at
`/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin` and
write only to the worktree-private `measure/` directory. Each audit run takes
about four and a half minutes unloaded.

```sh
git submodule update --init --recursive
cargo build --release -p qcore-compiler --example cycle19_fluent_bit_libraries_audit
cp target/release/examples/cycle19_fluent_bit_libraries_audit \
   measure/joined-provenance-call-frames/bin/rb-after
python3 -B measure/joined-provenance-call-frames/run_audit.py rb-after \
   measure/joined-provenance-call-frames/bin/rb-after
python3 -B measure/joined-provenance-call-frames/byte_identity.py \
   measure/joined-provenance-call-frames/runs/rb-before \
   docs/reports/w36-corpus-contracts-3/artifacts
python3 -B measure/joined-provenance-call-frames/leaves.py rb-before rb-after
python3 -B measure/joined-provenance-call-frames/mutations.py
python3 -B measure/joined-provenance-call-frames/archive.py rb-before rb-after
```

BEFORE is the same build with
`crates/qcore-c-gateway/src/legacy_joined_provenance.rs` at `d47e2bb2`. The
premise census additionally needs the temporary instrumentation, which
`scripts/instrument.py` generates from whichever version of the module it is
given — it detects the post-card signature and switches three anchors. That
instrumentation is **not** in the shipped source, and the published BEFORE and
AFTER artifacts were produced by binaries built without it.

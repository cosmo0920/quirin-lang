# JOINED-PROVENANCE-CALL-FRAMES — conditional provenance across a call

Wave 36. Card base `60958bf0`, rebased onto wave 36's first arbiter
(`W36-CORPUS-CONTRACTS-3`); this card is the wave's **second** arbiter and owns
a corpus re-pin.

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
17 uniquely-anchored edits; `logs/premise-base.jsonl` is its output.

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

*(This section is completed against wave 36's first arbiter's artifacts after
the rebase; see §6 of the final revision.)*

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

*(Re-run after the rebase; see §8 of the final revision.)*

## 9. For the integrator

*(Completed in the final revision.)*

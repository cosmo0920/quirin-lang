# C03-NFR — Implement the neutral-frame recursion route — 2026-09-08

Implementation card. Baseline `f8dbb2f`. Implements units **U1** (derivation
route), **U2** (greatest post-fixed-point descent), **U3** (certificate
rederivation), **U4** (Lean theorems) and **U7** (self-dependent record hygiene)
of [`docs/design/W4_RECURSIVE_SUMMARY_PROFILES.md`](../../design/W4_RECURSIVE_SUMMARY_PROFILES.md)
§9. The design card's own record is [`docs/reports/remaining-c03/README.md`](../remaining-c03/README.md).

**Nothing is promoted.** The full 185-unit Cycle 19 corpus was re-measured
before and after this change on identical inputs, and **all four artifacts plus
the Monkey source report are byte-identical** (§6). Raw units stay
2 Safe / 43 provisional Unsafe / 137 Unknown / 4 data-only on that run
(the archived pin's 42 is a separate, integrator-owned question — CORPUS-REPIN;
this card's before/after comparison is exact and self-contained). CT coverage
(20/50), the contract corpus (60 modules), `qcore-schemas` (5.0.0) and
`qcore-compiler` (0.2.0) are untouched. No schema change.

## 1. What landed

`crates/qcore-c-gateway/src/legacy_neutral.rs` (new, 242 lines) is a third W4
recursive route, consulted **only** after ordinary inference and both ranked
recursive routes have published nothing for an undeclared recursive component.

Where the contextual and unsigned-countdown routes constrain the **control
shape** and admit arbitrary custody effects, paying with a well-founded rank,
NFR takes the orthogonal axis: it fixes the **effect content** at the bottom
element — `N(m)`, every formal `Borrowed`, result `None` — and leaves the
control shape completely free (self, mutual, unranked, any arity).

### 1.1 Lattice and descent (§4.2)

Per member the assumption is a two-point lattice `Uncontracted ⊏ Neutral`;
component states are pointwise maps, so the lattice is finite of height
`|C| + 1`. The iteration starts at the top element, validates every member still
held at `Neutral` under the current map, demotes every failure, and repeats. Each
round that changes anything demotes at least one member, so it terminates in at
most `|C|` rounds on the **greatest post-fixed point** below all-`Neutral`. **No
join and no widening is used**; the intraprocedural domain and its joins are
byte-identical to before.

### 1.2 The ten obligations, and where each is checked

| # | Obligation | Where |
|---|---|---|
| V1 | no declared contract for any member | `infer_neutral_component`, first guard; a declared component keeps the existing route |
| V2 | ordinary inference and both ranked routes run first; NFR never replaces or weakens what they published | `derive_custody_summaries`: NFR runs only in the `else` arm of the contextual route **and** only when `pending.len() == component.len()` |
| V3 | the seeded assumption is exactly `N(m)` for every member, and nothing else is ever seeded for a member of its own component | `neutral_contract` is the only constructor in the module; there is no other candidate |
| V4 | component assumption map iterated per §4.2 | the `for _ in 0..=component.len()` descent |
| V5 | body re-analyzed with an **empty** entry state, `Safe`, **zero** diagnostics | `validate_member`: the entry is asserted empty (no parameter site, no out slot, non-nullable result) before analysis, plus condition (1) of the unchanged gate |
| V6 | unchanged `observations_match_contract` against `N(m)` | `validate_member` calls it verbatim |
| V7 | the neutral contract must be what validation *reads off* the body, with no fallback to a stronger mode | `propose_contract(&entry, &observations) == Some(N(m))` — an explicit equality, not merely "V5/V6 happened not to fail" |
| V8 | at least one exit reachable **in the abstract semantics** (explicitly not concretely) | condition (2) of the unchanged gate; `spin` in the admitted family is the fixture that makes the distinction visible |
| V9 | `ResourceSummaryRecord` / `ContractTrust::InferredValidated` / contract `N(m)`; no new schema field, trust label or wire change | publication reuses `CustodyDerivation::inferred`; asserted over the whole admitted family |
| V10 | the recheck re-runs the whole component computation from the certificate's own source and demands exact equality | `check_legacy_safety_certificate` already re-runs `derive_custody_summaries`; NFR is inside it, so this is not a second code path — see §5 |

### 1.3 U7 — self-dependent records

`CustodyDerivation` gains `self_dependent`. In a **recursive** component in which
**any** member carries a declared contract, no member publishes an
`InferredValidated` record: inside a strongly connected component a declaration
is what the recursive call site reads, so such a record repeats the declaration
rather than corroborating it. The declaration still governs every call site, so
no verdict moves; only the published record disappears, which is precisely the
presentational risk the design recorded (design §10, unit U7).

The rule is deliberately scoped to *recursive* components, not to every declared
function: a non-recursive declared function's derivation never reads its own
declaration back, and a test pins that such a record is still published.

## 2. Refusal matrix (implemented route, no declared contract)

All measured through the real gateway in
`crates/qcore-c-gateway/tests/neutral_frame_recursion.rs`.

| Body | Route outcome | Why |
|---|---|---|
| releases a formal | component withdrawn, `Unknown` | unresolved identity under the empty entry (V5) |
| consumes a formal | component withdrawn, `Unknown` | same |
| stores a formal into a global | component withdrawn, `Unknown` | same |
| calls an uncontracted external | component withdrawn, `Unknown` | unmodelled call (V5) |
| leaks a local acquisition | member `Unsafe`, nothing published | a body defect, reported as one |
| acquires and returns | component withdrawn, `Unknown` | result mode disagreement (V6/V7); there is no fallback to `Owned` |
| recursion through a pointer field (`seek(n->next, …)`) | `Unknown` | refused before any recursion rule (C02-P; §7) |
| heap-link chain teardown (`purge(n->next); free(n);`) | `Unknown` | §8 rejection boundary, kept closed |
| mixed mutual pair (one member releases) | **whole component** withdrawn | see §4 |
| divergent `purge` / `mint` | publishes **`N(m)` only** | see §3 |

## 3. The counterexample, refused

The design's §4.3 counterexample is the reason the seeded candidate is fixed at
the bottom element: with `resource_released(p)` *declared*,

```c
void purge(void *p, int n) { if (n > 0) { purge(p, n - 1); return; } purge(p, 1); }
```

validates against its own contract, because every concrete execution diverges and
"if the call returns, `p` was released" is vacuously true. Validation by
re-analysis alone therefore cannot refuse a circular effectful candidate.

The implemented route never reaches that state, and the test
`a_divergent_effectful_candidate_is_never_seeded_by_the_route` pins both halves:

* through the route, `purge` publishes `[Borrowed, Borrowed] → None` and the
  caller's acquisition is reported as the leak it is (`Unsafe`), instead of being
  explained away (`Safe`) as it is under the declared emulation;
* `void *mint(int n) { void *r = mint(n); return r; }` publishes `N(m)`, mints no
  owned result, and leaves its caller fail-closed `Unknown`.

**Any effectful recursive family still keeps a rank.** That is enforced
structurally (no candidate above ⊥ is constructible in this module) and recorded
in Lean by `ranked_calls_are_well_founded` and
`a_ranked_base_member_has_no_admitted_call`.

## 4. The probe's permissiveness gap, pinned rather than adopted

The design card measured that its declared-contract emulation is *more*
permissive than the route at exactly one point: a declaration outlives its own
member's refusal, so in a mixed mutual pair `even` stayed `Safe` while `odd` was
refused (probe `declared_emulation_does_not_reproduce_component_withdrawal`).

`the_route_withdraws_a_component_the_declared_emulation_keeps` runs **both** on
the same fixture in one test, so the divergence cannot silently close in either
direction:

| | `odd` | `even` |
|---|---|---|
| declared emulation | `Unknown` | **`Safe`** (artefact) |
| implemented route | `Unknown` | **`Unknown`** (`call to uncontracted function \`odd\``) |

The route matches the design. The probe's reading was **not** adopted.

## 5. Deviation from U2, recorded

Unit U2 anticipated that a member could survive its component's partial
demotion when every call edge of its own body into a demoted member is
graph-unreachable. The descent is implemented as specified — it demotes only
failing members and re-validates the survivors — but **no such member survives**,
and the reason is a containment gate rather than the lattice:

* a call into a demoted member compiles to a `LegacyInstruction::Unknown` node;
* on a graph-unreachable path the analysis never reaches it, so it raises no
  diagnostic and V5/V6 would pass;
* but `check_legacy_safety_certificate` refuses **any** function whose CFG
  retains an `Unknown` instruction.

Publishing such a member would therefore offer a summary its own containment
check later rejects, and — measured on the pre-existing gateway, with no
recursion involved — a translation unit in which every function is otherwise
`Safe` while one retains a dead uncontracted call is returned as a hard
`ProductionParserRejected` error rather than `Unknown`. `validate_member`
consequently refuses a body that retains an `Unknown` instruction, exactly as
`legacy_recursion::contextual_cfg` already refuses such a context CFG.

Publication is therefore observably **all-or-nothing over the component**, which
is the design's V4 default. This is a deliberate deviation from U2's optimism,
recorded here rather than silently taken, and pinned by
`a_graph_unreachable_edge_into_a_demoted_member_does_not_survive_the_descent`
(which also checks the same component *without* the dead edge publishes both
members, so the refusal is the retained instruction and not the component shape).

The Lean side proves the same collapse from the other direction:
`a_strongly_connected_component_publishes_all_or_nothing`.

**Pre-existing boundary, observed and not repaired:** the hard-error behaviour
above (`function \`f\` retains an unknown instruction` surfacing as
`LegacyCError::ProductionParserRejected` instead of a scoped `Unknown`) exists on
the unchanged gateway for any dead uncontracted call. NFR does not widen it,
because it now refuses exactly the bodies that would have reached it.

## 6. Census coverage actually achieved

This is the load-bearing honest finding of the card, and it corrects the design's
estimate.

**Held-out fixtures: nine admitted control shapes**, none recursing through a
pointer field — self recursion with a decreasing scalar; self recursion with no
decreasing argument; unguarded self recursion with no concrete exit; a body that
acquires and releases a local; a mutual pair; a three-member cycle; a scalar
accumulator over a borrowed handle; two borrowed pointer formals swapped on the
recursive call; a member handing one of its own formals back. Each publishes
exactly `N(m)`, turns its caller `Safe`, and rechecks.

**Real corpus at `66910c10`: reach is 0.** The design estimated ≈ 9 reachable
functions from the census's *argument-shape* cross-tab. That estimate does not
survive contact with V5, which requires the **whole body** to be modelled with
zero diagnostics, not merely the recursive argument to be scalar-shaped.
Measured on the projected 185-unit corpus:

* **12** self-recursive functions in **6** units actually reach derivation
  (`cio_os_mkpath`; `cJSON_Duplicate_rec`; `rd_kafka_idemp_set_state`,
  `rd_kafka_op_destroy`; `rd_kafka_q_apply`, `rd_kafka_q_pop_serve0`,
  `rd_kafka_q_purge0`, `rd_kafka_q_purge_toppar_version`, `rd_kafka_q_serve0`,
  `rd_kafka_q_serve_rkmessages`, `rd_kafka_q_serve_share_rkmessages`;
  `mk_rconf_read`). The census's 45 was a lexical count over 309 unpreprocessed
  files and is a different population.
* **All 12 are refused**, every one on a blocker other than the recursion. The
  dominant families across them are **155** `indirect memory access is outside
  the resource projection` and **106** `unmodeled call, aggregate or indirect
  access in branch condition`, plus unmodelled initializers, loop conditions,
  uncontracted callees (`__errno_location`, `stat`, `cJSON_New_Item`) and one
  unresolved identity.
* Therefore **no component in the corpus is admitted**, which is *proved* by the
  byte-identity in §7: any admission would have removed at least one
  `call to uncontracted function \`<member>\`` diagnostic.

**Consequences.** No verdict moves, no diagnostic changes, and — contrary to what
U1's acceptance criterion braced for — **no newly surfaced provisional `Unsafe`
unit** on the real corpus. The precision gain is real and demonstrated on
held-out fixtures; on this corpus it is latent, and it stays latent until
**U5 (C02-P borrowed pointer-field reads)** removes the dominant blocker family.
C02-P lands in a parallel worktree and this card does not depend on it.

## 7. Re-measurement

Both audits were run twice on identical prepared inputs — once at baseline
`f8dbb2f`, once at this card's checkpoint — with the same Clang
(`Ubuntu clang 21.1.8`) and the same build trees.

### 7.1 Full 185-unit Cycle 19 corpus, pinned revision `66910c10`

```sh
python3 -B scripts/prepare_cycle19_clang_audit.py \
  --fluent-bit ../../fluent-bit --revision 66910c10a4d7eafa810b84229cf2dcf7b0f26f97
# then the printed audit command, with QCORE_CLANG=/usr/bin/clang
```

| | baseline `f8dbb2f` | this card |
|---|---|---|
| totals | 185 files / 186 configurations: **2 Safe / 43 provisional Unsafe / 137 Unknown / 4 data-only / 0 gateway failures** | identical |
| `reports/resource-diagnostics.json` | — | **byte-identical** |
| `reports/resource-diagnostics.contracts.json` | — | **byte-identical** |
| `reports/resource-diagnostics.chunk-contracts.json` | — | **byte-identical** |
| `reports/resource-diagnostics.source-effects.json` | — | **byte-identical** |
| `reports/monkey-source.json` | — | **byte-identical** |

This run is the explicit-Clang route at `66910c10` on this host; its 43 provisional
`Unsafe` matches `docs/reports/cycle19-explicit-clang/README.md` rather than the
archived 42-unit pin. Reconciling the archive is **CORPUS-REPIN**, which is
integrator-owned and untouched here; what this card establishes is the exact
before/after equality on identical inputs.

### 7.2 Canonical CMetrics audit, pinned revision

```sh
python3 -B providers/package.py build --output build/cycle19/provider-packages-unit
python3 -B scripts/prepare_cmetrics_clang_audit.py \
  --fluent-bit ../../fluent-bit --revision 66910c10a4d7eafa810b84229cf2dcf7b0f26f97
cargo run -p qcore-audit -- <printed cmetrics-audit.json.config>
```

**1 checked function contract / 6 Unknown; raw unit provisional `Unsafe`** —
unchanged, with `cmetrics-audit.json` and `cmetrics-audit.assumptions.json` both
**byte-identical** before and after. The current-`HEAD` CMetrics route
(`ef0cb0b2`, `checkout_head`) is likewise byte-identical.

## 8. Deliberate pin updates

Seven pinned expectations changed. Every one keeps its load-bearing negative
assertion; none is weakened, dropped, or downgraded to pass.

| Pin | Change | Load-bearing half retained |
|---|---|---|
| `legacy_c.rs::internal_definitions_never_receive_an_automatic_neutral_contract` | the bare `subject(); ` recursion case is replaced by a recursive body that carries an effect | "being defined in the TU buys a function nothing; a call is modelled only through a declared contract or a **validated** summary" — NFR is a validated summary, which the pin's own prose permits |
| `legacy_summary.rs::direct_recursion_stays_unknown` | narrowed to *effectful* direct recursion | an effectful recursive body still stays `Unknown` with the uncontracted-call diagnostic |
| `legacy_summary.rs::a_self_consistent_recursive_summary_is_never_accepted` | now asserts the published contract for `bad` **is exactly neutral** and the caller's leak is still reported | the self-consistent `Released` reading is still never accepted — now checked *directly* rather than inferred from the absence of any summary |
| `legacy_summary.rs::general_alias_removal_discovery_remains_unsupported` | the self-recursive body now asserts the published contract carries **no `Removed` mode** | no removal is discovered from recursion |
| probe `undeclared_neutral_recursion_leaves_the_component_and_its_callers_unknown` | pins the *residual* gap (an undeclared component that carries an effect) | the motivating gap is closed for neutral bodies; the effectful half is unchanged |
| probe `assumed_neutral_components_validate_and_unblock_their_callers` | the record assertions become "withheld" (U7) | all three verdicts per fixture are unchanged |
| probe `an_assumed_effectful_contract_validates_vacuously_under_divergence` | same (U7) | the vacuity itself is untouched: the declared `Released` still validates and the caller's leak still disappears under the trusted declaration |

## 9. Lean inventory

New namespace `QCore.NeutralFrameRecursion`
(`proof/QCore/NeutralFrameRecursion.lean`, imported by `QCore.lean`), audited by
`proof/AuditNeutralFrameRecursion.lean`, which
`scripts/check_lean_audits.py` picks up automatically (34 → **35 audits**).
**34 public theorems, no new axiom, no placeholder**; every one depends only on
`propext`, `Classical.choice` and `Quot.sound`.

| Group | Theorems |
|---|---|
| Lattice, order, descent (8) | `step_descends`, `step_preserves_bounds`, `top_is_bounded`, `post_fixed_iff_stable`, `step_is_monotone`, `descent_reaches_a_post_fixed_point`, `descent_stays_bounded`, `descent_is_the_greatest_post_fixed_point` |
| Publication and refusal (6) | `every_published_member_validated_under_the_published_map`, `a_refused_member_is_never_published`, `an_always_refusing_component_publishes_nothing`, `an_always_validating_component_publishes_every_member` (non-vacuity), `cascade_demotion_propagates_along_calls`, `a_strongly_connected_component_publishes_all_or_nothing` |
| Why ⊥ and nowhere above (6) | `neutral_claim_is_prefix_closed`, `neutral_claim_is_independent_of_returning`, `an_exit_claim_depends_on_returning`, `a_divergent_run_satisfies_every_exit_claim`, `an_exit_claim_cannot_refuse_a_divergent_effectful_candidate`, `the_neutral_claim_refuses_an_effectful_run` |
| The rank an effectful family must carry (2) | `ranked_calls_are_well_founded`, `a_ranked_base_member_has_no_admitted_call` |
| What the published summary asserts (12) | `the_neutral_summary_binds_no_formal`, `the_neutral_summary_names_no_owner`, `the_neutral_summary_governs_every_formal`, `a_neutral_formal_carries_no_disposition`, `a_neutral_formal_writes_through_no_slot`, `a_neutral_formal_is_never_contained`, `a_neutral_exit_returns_nothing`, `result_of_exit_match`, `published_neutral_member_is_analyzed_safely`, `published_neutral_member_has_a_reachable_exit`, `published_neutral_member_matches_every_exit`, `a_published_neutral_member_hands_back_no_custody` |

The descent theorems assume **`MonotoneValid`** — validation is monotone in the
assumption map — which is a property of the abstract interpreter supplied as a
premise, not derived here. Clang import, the complete call inventory and CFG
construction remain trusted implementation premises, exactly as both existing
recursive routes record. Nothing here establishes termination or stack bounds for
any C function; `a_divergent_run_satisfies_every_exit_claim` is precisely the
statement that no exit-state contract can.

## 10. Tests

`crates/qcore-c-gateway/tests/neutral_frame_recursion.rs` (new, 10 tests, all
through the real Clang-backed public API, every `Safe` result rechecked by
`check_legacy_safety_certificate`):

1. `neutral_components_publish_the_bottom_element_and_unblock_their_callers` — the nine-shape admitted family (V3, V9, V10)
2. `a_body_that_carries_an_effect_refuses_the_whole_component` — the refusal matrix
3. `a_neutral_contract_states_no_precondition_and_hands_back_no_custody` — the empty alias obligation, both directions
4. `a_divergent_effectful_candidate_is_never_seeded_by_the_route` — §3
5. `the_route_withdraws_a_component_the_declared_emulation_keeps` — §4
6. `a_graph_unreachable_edge_into_a_demoted_member_does_not_survive_the_descent` — §5
7. `declared_and_ranked_routes_keep_precedence_over_the_neutral_route` — V1, V2 (contextual **and** unsigned-countdown fixtures keep their `Released` contracts)
8. `a_recursive_component_carrying_a_declaration_publishes_no_inferred_record` — U7, plus the non-recursive control that still publishes
9. `heap_link_chain_teardown_and_pointer_field_recursion_stay_refused` — §8 boundary
10. `certificate_rederivation_refuses_substitution_widening_and_forgery` — U3/V10: a substituted member, a widened component, a forged record for a function never published, a strengthened record for one that was, a withdrawn record, and a relabelled record

**No native sanitizer fixture is added.** The design calls for one only in §8's
rejection direction, and that fixture already exists beside the design card
(`chain-teardown-hazard.c`, with its two archived ASan/UBSan logs); this card
keeps that shape refused rather than admitting it, so there is nothing new to
execute natively. NFR itself changes no emitted C.

## 11. Gate

Run from this worktree at the card checkpoint. The Libevent submodule had to be
initialised first (`git -c protocol.file.allow=always submodule update --init
--recursive`, `d82464a`) — an environment condition of the isolated worktree, not
a finding.

| Check | Result |
|---|---|
| `cargo test --workspace` | **1483 passed**, 0 failed, 1 ignored (baseline `f8dbb2f` on this host: 1473; delta is exactly the 10 new tests, verified per-binary) |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 scripts/check_lean_audits.py` | **35 audits pass** (34 at baseline) |
| `cd providers && cargo test` | 58 passed |
| 185-unit Cycle 19 audit | 5/5 artifacts byte-identical (§7.1) |
| canonical CMetrics audit | 2/2 artifacts byte-identical (§7.2) |

Known load-flaky on this host, unrelated to this card and confirmed passing in
isolation: `cycle20_verifier_plugins::vp04_vp07_vp08_process_lifecycle_and_payload_budgets`
failed once under full-workspace load (a 500 ms process-transport budget) and
passes on a rerun; it also passes at baseline. This matches the previously
recorded P02/P03 timing flakiness.

## 12. Open boundaries

- **U5 (C02-P) gates the corpus payoff.** NFR's measured reach at `66910c10` is
  **0**, blocked by indirect memory access and unmodelled branch conditions, not
  by the recursion. Re-measure this card's reach once C02-P lands.
  **Done, 2026-09-09, card C03-REACH**
  ([`docs/reports/remaining-c03-reach/README.md`](../remaining-c03-reach/README.md)):
  with C02-P, O04-ARMS, O04-B08 and O04-READ-BOUND all landed the reach is
  **still 0**, and the population above is corrected — this card's "12
  self-recursive functions in 6 units", counted from published diagnostics, is
  46 components / 96 members across 21 units when read off the route itself.
  That card also measures that admitting the *largest* blocker family
  completely would still admit zero components, so the payoff is not gated on
  any single admission.
- **U6 (reachable-region release mode) is not started and must not be.** The
  design's §8 three-part boundary is unmet — no `ReleasedReachable` mode exists in
  `qcore-schemas`, the caller-side domain cannot represent region membership, and
  acyclicity is not a declared premise — and this card changes no schema. Heap-link
  chain teardown stays refused, pinned by test 9.
- **U2's surviving-member case is unreachable** under the certificate-containment
  firewall (§5); publication is all-or-nothing in practice.
- **The pre-existing hard-error path** for a dead uncontracted call (§5) is
  observed and not repaired.
- **Cross-translation-unit recursion** is invisible to component detection, which
  is per-translation-unit; NFR inherits that boundary from `legacy_summary.rs`.
- **`MonotoneValid`** is a premise, not a proved property of the Rust abstract
  interpreter (§9).
- **CORPUS-REPIN** (the archived 42-unit artifacts versus this host's 43) is
  untouched and integrator-owned.
- Nothing here establishes termination, stack bounds, or leak-freedom across a
  divergent call, and **no raw unit is promoted**.

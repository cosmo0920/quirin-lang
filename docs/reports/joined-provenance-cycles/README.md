# JOINED-PROVENANCE-CYCLES — the conditional-provenance relation on cyclic CFGs

Wave 35, second and last arbiter. Base **`ebc0fd49`** — the `origin/main` tip
carrying wave 35's first arbiter `C-GATEWAY-CONSTANT-DO-WHILE` (card commit
`148eef79`, merged at `3ead460b`), and which also carries the wave-34 close,
`SWEEP-BLIND-SPOTS`, `VERIFIER-HOST-CONTROL-BUDGET`, `CT-CTWIN-LANE` and
`MUTEX-SHARE-CROSS-ARMS`, none of which touch the gateway. The baseline this
card's BEFORE reproduces **byte for byte** is
`docs/reports/c-gateway-constant-do-while/artifacts/`. The BEFORE and AFTER runs
were repeated on **each** of the two tips this card was rebased through rather
than carried forward, because the audit binary links `qcore-frontend` and
`MUTEX-SHARE-CROSS-ARMS` edits it; both pairs agree, and the figures below are
the `ebc0fd49` run's.

The card was first measured at `a300a39d`; when the first arbiter landed,
everything below was **re-measured on its tree**, because removing the constant
`do … while (0)` back edge moves 654 functions out of this card's population
before it does anything (§2). Every count quoted here is against `ebc0fd49`,
never against `a300a39d`, and the BEFORE run was repeated once more after the
final rebase onto main rather than carried over from the pre-merge tree.

## 1. Headline

Wave 31 derived `site -> names` over **acyclic** custody CFGs only; a single
back edge anywhere in a function declined the whole function. This card
replaces that bail-out with the **greatest fixed point** of the same transfer
over the cyclic graph, restricted to sites whose creating node lies on no
cycle. The extension is proved sound in Lean, is exercised on the pinned
corpus, and **moves no count at all**.

| | |
|---|---|
| Corpus movement | **zero** — 0 leaves added, 0 withdrawn, 0 unit status moves, 0 function status moves |
| `leak` / `unknown_alias` / `unsupported` / `double_release` | **272 / 4,214 / 50,106 / 0** — all four unmoved from the first arbiter's re-pin |
| BEFORE reproduces | `c-gateway-constant-do-while/artifacts/` **byte for byte** on both verdict-bearing artifacts |
| Fixed point reached | 1,273 cyclic functions; **40** derive at least one real site fact |
| Servable population measured | **67 functions / 354 leaves** (the naive number is 659 / 2,718 — §2) |
| Lean | 7 theorems, `QCore.JoinedProvenanceCycles`, audited in `AuditJoinedProvenanceCycles.lean` |
| Mutations | **6 of 6 caught** |

The card lands a proved, tested, corpus-exercised extension whose measured
effect on the published verdicts is nil. That is the finding, not a
disappointment to be written around: §6 names exactly which premise stops it,
and the two residuals in §8 carry that measurement as their discharge
condition.

## 2. Measure first — three nested populations, and why the first one is wrong

The brief required the declined population to be counted before anything was
designed, and warned that the previous two gateway briefs were each cut on a
reach number that measured the wrong thing. Ground truth here comes from the
analyzer's **own CFG**, emitted by temporary instrumentation in
`legacy_joined_provenance::derive` (archived at
`scripts/legacy_joined_provenance.instrumented.rs`), never from a source-text
classifier. Crossed with the BEFORE run's diagnostics:

| population | functions | unresolved-identity leaves |
|---|---:|---:|
| 1 — cyclic CFG and ≥ 1 `unresolved resource identity` leaf | **659** | **2,718** |
| 2 — … and ≥ 1 resource site created anywhere in the CFG | **74** | **419** |
| 3 — … and ≥ 1 site whose creating node lies on no cycle | **67** | **354** |

Population 1 is the number this card's brief would naturally have quoted, and
it overstates the servable population by nearly **10×**. Of 4,771 projected
functions the instrumentation saw, 1,273 have a cyclic CFG; but 585 of the 659
carry no resource site at all, so there is nothing for a provenance relation to
be *about* — no fixed point over their CFGs can produce a fact, and counting
them measures definedness rather than derivability. Population 3's unit-projected
status is 43 `Unknown` and 24 provisional `Unsafe`.

**These are the figures on the first arbiter's tree, and the re-measurement was
not cosmetic.** On `a300a39d` the same three populations read 885 / 3,393, 80 / 439
and 73 / 374, against 1,927 cyclic functions. The first arbiter's removal of the
constant `do … while (0)` back edge moves **654** functions from cyclic to
acyclic — 1,927 − 654 = 1,273, reproducing its own measurement exactly — and
those 654 are now served by the wave-31 topological pass rather than by this
card. Six functions and 20 leaves left population 3 that way. Quoting the
`a300a39d` figures would have credited this card with reach that another card
had already taken.

**A caught error in this census.** The first run of population 2 returned
**zero**, because the instrumentation counted only bare `Acquire` nodes. In the
pinned corpus a contracted allocator arrives as a `ContractCall` carrying
`produced_sites`, so the real count is 74. The corrected instrumentation counts
both, and `acyclic_sites` in the shipped code counts both; the mutation
"a contracted allocator's produced site is no longer eligible" (§5) exists to
keep them from drifting apart.

**Caveat, measured rather than waved away.** The census keys by function name,
and a name recurs across translation units. Of 4,771 names, 36 had records that
differed between the analyzer's passes (W4 derivation, validation by
re-analysis, certificate recheck); of those, exactly **one** (`format_metric`)
differs on cyclicity itself and **none** differs on site eligibility. The
population figures are therefore accurate to within one function.

## 3. What landed

`crates/qcore-c-gateway/src/legacy_joined_provenance.rs`. The acyclic route is
**untouched in effect**: a topologically sortable graph still takes the wave-31
single pass. Only the bail-out changes.

* `fn outgoing(cfg, node, incoming) -> Vec<Facts>` factors the per-edge transfer
  — the instruction step, the `conditional_names` restriction, the null edge's
  exclusion of every site the tested name denotes — out of the acyclic loop, so
  the two routes compute the same thing by construction rather than by
  agreement.
* `fn fixed_point(cfg, reachable)` runs Kleene iteration from the **top**
  element: every node but the entry starts at "no site present", where every
  guarantee is vacuous, and each round recomputes a node as the meet of its
  predecessors' outgoing facts until nothing moves. Absence stays neutral and
  two present predecessors still intersect, so a claim can never be justified by
  the cycle alone — the entry's own concrete facts are an operand of the meet on
  every path into a loop. Iteration only ever weakens a claim, so the
  `FIXED_POINT_ROUNDS = 64` bound declines the function rather than publishing an
  unsettled state.
* `fn acyclic_sites(cfg, reachable)` is the soundness restriction. A site whose
  creating node lies on a cycle stands for one abstract resource per **site**,
  not per dynamic object; a per-site identity fact would then not be a statement
  about one of them, and settling its disposition could discharge an object that
  a later iteration replaced. Such sites derive nothing. A node lies on no cycle
  exactly when it is not reachable from its own successors.

Everything else in constraint 10b is unchanged: no caller custody is seeded,
calls still forget prior relations, the conditional shallow release still
settles only under `release_site`'s checked premises, and the `UnknownAlias`
identity refusal is retained. `legacy_c.rs`'s CFG builder was not touched —
card `C-GATEWAY-CONSTANT-DO-WHILE` owns it this wave.

## 4. The Lean

`proof/QCore/JoinedProvenanceCycles.lean`, audited by
`proof/AuditJoinedProvenanceCycles.lean`, built on wave 31's own `World` and
`guarantees` vocabulary in `QCore.CGatewayRefinements`. As with every gateway
module these are **abstract rule theorems, not extraction of the Rust
analyzer** (`GATEWAY-LEAN-ABSTRACT-NOT-EXTRACTION`); faithful token/type
extraction, the CFG construction and declared release effects remain premises.

* `fixed_point_is_sound` — **(b), soundness on a cyclic graph.** A family of
  claims satisfying the fixed-point *inequality* and sound at the entry is sound
  at every node on every path that reaches it. The induction is on the arrival
  itself (`Reach`), so a back edge is no different from a forward one; this is
  the theorem that makes the whole card admissible, and it needs no acyclicity
  hypothesis.
* `acyclic_solution_is_unique` — **(a), the conservative extension.** On a graph
  whose edges strictly increase a rank, the fixed-point *equation* has exactly
  one solution. The wave-31 topological pass computes a solution, so the fixed
  point is that same one and **no acyclic body's verdict can change**.
* `weakening_stays_sound`, `claiming_nothing_is_sound` — refusing is always
  available: dropping a site, clearing a name set, or declining a whole function
  at the iteration bound can never make a derivation unsound. This is the
  "monotone in the refusing direction" half.
* `meet_satisfies_the_inequality` — the meet is admissible, so the *greatest*
  solution is the one the inequality licenses, not merely some solution.
* `steps_increase_rank`, `an_off_cycle_node_does_not_return` — a node on no
  cycle cannot return to itself, which is what makes a per-site fact a statement
  about one object and justifies `acyclic_sites`.

Axiom surface, from the audit file: five theorems depend on no axioms;
`acyclic_solution_is_unique` depends on `propext` and `Quot.sound`. Both are
inside `AuditAll.lean`'s permitted closure.

## 5. Falsification — every new arm mutated

`scripts/mutations.py`. Each mutation breaks one rule and names the test that
must go red; a rule no mutation can disturb is a rule nothing verifies.

| mutation | caught by |
|---|---|
| the loop-created site restriction removed (every site eligible) | `a_site_created_inside_the_cycle_derives_nothing` |
| the fixed point declines everything (wave-31 behaviour restored) | `a_back_edge_no_longer_declines_an_off_cycle_acquisition`, `a_cycle_beside_an_off_cycle_acquisition_still_settles_the_release` |
| the meet at a join unions names instead of intersecting | `presence_controls_the_join_not_the_pointer_target_union` |
| the iteration stops after one round instead of at stability | both settling tests |
| a contracted allocator's produced site is no longer eligible | `a_cycle_beside_an_off_cycle_acquisition_still_settles_the_release` |
| the eligible-site filter no longer restricts published bindings | `an_eligible_site_does_not_carry_an_ineligible_one_with_it` |

**6 of 6 caught**, tree restored green. Two failures of this harness were
findings about the card's own work and are recorded rather than quietly fixed.
First, the C-level test originally written for the eligibility rule did **not**
drive it — the suite stayed green with the restriction deleted — because the
downstream premises mask the difference in a C body; the rule is now driven by
`a_site_created_inside_the_cycle_derives_nothing`, a direct `derive()` test on a
hand-built CFG where nothing can mask it. Second, the eligible-site *filter*
needed a graph carrying an eligible and an ineligible site at once, since a
graph with only the latter returns early and never reaches the filter; and when
that test was added the harness still reported `MISSED`, which turned out to be
a **mislabeled expectation in the harness rather than a gap in the tests** — the
mutation was caught, by the new test rather than by the one the row named. The
row now names the catching test, and the distinction is recorded because
"`MISSED`" and "the harness is pointing at the wrong test" are different
findings and only one of them is about the code. A third
mutation was **withdrawn as a no-op**: replacing the `None` predecessor skip
with an explicit `Facts::default()` is caught by nothing because the two are the
same lattice element — an absent site is neutral at a join, which is exactly
`QCore.CGatewayRefinements.absent_predecessor_is_neutral`. A mutation that
cannot change a result is not evidence that a rule is unverified.

## 6. The re-pin, and why nothing moved

BEFORE is the base tree's own binary, built from `ebc0fd49` with this card's
module reverted. It reproduces
`docs/reports/c-gateway-constant-do-while/artifacts/` **byte for byte** on both
verdict-bearing artifacts — `diagnostics` `9af9ee2e4e0837b6`, `chunk-contracts`
`c4f438e4ed3dab3c` — so the chain is attributable through wave 35's first
arbiter to wave 34's. (`source-effects` and `contracts` differ by checkout root
and identity digests, exactly as the three preceding arbiters recorded.)

The three-way leaf split over the pinned 185-unit corpus, leaf identity
`(unit file, function, preprocessed line, category, description)` — the
**internal** coordinate per constraint 12a, with `origin` carried for reading
only:

| | |
|---|---:|
| added (only in AFTER) | **0** |
| removed (only in BEFORE) | **0** |
| unmoved | **54,592** |
| unit status moves | **0** |
| function status moves | **0** |

`leak` stays **272** (constraint 7's chain is untouched, no leaf added or
withdrawn), `unknown_alias` **4,214**, `unsupported` **50,106**,
`double_release` **0** — the last as the first arbiter left it, this card
neither restoring nor adding a repeated-release row. Functions stay
**186 Safe / 84 provisional Unsafe / 4,564 Unknown**. No archive was edited
(constraint 21).

**This is not vacuity, and the difference is measured.** The new arm is reached:
1,273 cyclic functions run the fixed point and **40 of them derive at least one
real site fact** — among them `create_jws_compact_serialization` (3 sites), and
`decode_exponential_histogram_data_point`, `decode_histogram_points`,
`cmt_exp_histogram_to_explicit` and `rd_kafka_jwt_b64_decode_payload` (2 each).
The facts exist; they are simply never enough.

Where they stop is also measured. Instrumenting `release_site`'s site-premise
gate over the whole corpus, **with the fixed point active**:

| times the site-premise gate is reached | 14 |
|---|---:|
| times every premise passes | **0** |
| blocked by `not_named_by_facts` | 14 / 14 |
| blocked by `nullable_site` | 14 / 14 |
| blocked by `unresolved_effect` | 5 / 14 |

Two independent blockers hold at **every** one of the 14 sites: the derived
facts do not name the released pointer, *and* the site is in
`nullable_sites`. So even a provenance relation that named every pointer
perfectly would still move nothing here until the nullable-site premise is
addressed. That conjunction, not the acyclic restriction, is what keeps this
route's corpus yield at zero — and it is the reason §8's residuals are written
against `release_site`'s premises rather than against the fixed point. (At
`a300a39d` the same census read 21 sites with the same two blockers universal,
plus one `not_live`; the first arbiter's re-pin took the `not_live` site with
the `double_release` row it withdrew.)

`scripts/attribution.py` additionally checks the measured half of the
conservative-extension claim: every moved leaf must belong to a function the
instrumentation recorded as cyclic. With zero moved leaves the check passes
trivially, which is stated here rather than presented as evidence it did not
supply.

## 7. A pinned expectation updated deliberately

`an_overwritten_live_acquisition_is_not_a_conditional_lookup_acquisition`
carried four bodies. The fourth —
`void *p = external; if (!p) { p = q_maybe_alloc(); if (!p) return; } while (flag) { flag = 0; } q_free(p);`
— was added by commit `2492825b`, the wave-31 commit that *created*
`legacy_joined_provenance.rs`, and pinned that a cyclic CFG derives nothing. It
is not an overwrite case at all: nothing rebinds `p`, the acquisition lies off
the loop, and the loop touches only `flag`. It was pinning the restriction this
card is chartered to lift — wave 31's own report names
`JOINED-PROVENANCE-CYCLES` as the follow-on that would lift it.

The pin is therefore **updated deliberately, not weakened**: the body moved to
`a_cycle_beside_an_off_cycle_acquisition_still_settles_the_release`, which
asserts the release now settles *and* that the `UnknownAlias` identity refusal
and the non-`Safe` status are both retained. The three genuine overwrite bodies
stay, and a comment at the removal site records what moved and why. Nothing
flips `Unknown` to `Safe`: the function's status is `Unknown` before and after,
and the corpus shows zero status moves of either kind.

## 8. Boundaries and named follow-ons

* **`JOINED-PROVENANCE-CYCLE-SITES`** — a site created *inside* a cycle derives
  nothing, by design (§3). Discharge: a per-iteration object identity, or a
  proof that the site's abstract resource may be treated as one object across
  iterations. Until then the restriction is a measured boundary, driven by
  `a_site_created_inside_the_cycle_derives_nothing`, not an omission.
* **`JOINED-PROVENANCE-RELEASE-NULLABLE`** — the measured blocker. All 14 sites
  reaching `release_site`'s premise gate are in `nullable_sites`, so the
  conditional release settles nowhere in the pinned corpus regardless of
  provenance quality. Discharge: decide whether a nullable site may be settled
  when the release is a declared `nullable` contract, and re-measure the 14.
* **`JOINED-PROVENANCE-FACTS-NAME-NOTHING`** — the other half of the same
  conjunction: at all 14 sites the facts do not name the released pointer.
  Discharge: attribute those 14 individually to the rule that loses the name
  (calls forget prior relations is the first suspect, which is
  `JOINED-PROVENANCE-CALL-FRAMES`' territory).
* **`JOINED-PROVENANCE-CALL-FRAMES`** — wave 31's sibling follow-on, preservation
  across calls, is **not** taken here and its code is untouched. §6's
  `not_named_by_facts` measurement is the first quantitative evidence bearing on
  it, and is offered to whoever takes it.
* **`AOM-DESTROY-JOINED-PROVENANCE`** — the production `input_chunk_get`
  residual at L19847. The brief asked whether cycles are even its blocker before
  anything was claimed. **They are not, and the answer is exact rather than
  inferred**: the instrumentation records `input_chunk_get` with
  `cyclic: false`, 47 reachable nodes, one resource site and that site off-cycle.
  Its CFG carries no back edge at all, so it was already served by the wave-31
  topological pass and this card cannot reach it; it is in neither population 3
  nor the 40 deriving functions. Its recorded blocker — an address-taken `ic`
  binding crossing refused stores and the uncontracted
  `flb_input_chunk_get_projected_write_size` — is untouched, and the residual
  stands exactly as written. No claim is made about it.
* **`C-GATEWAY-CONSTANT-DO-WHILE` interaction — consumed and measured.** That
  card owns `legacy_c.rs`'s CFG builder this wave, and removing a constant back
  edge turns a cyclic body acyclic, moving it from this card's fixed point onto
  the wave-31 topological pass. This card is rebased onto it and re-measured on
  its tree: **654** functions make that move (§2), the two routes agree wherever
  both apply by `acyclic_solution_is_unique`, and the combined diagnostic effect
  of the flip plus this card is zero on today's corpus. The interaction is
  therefore no longer a named risk.

## 9. The gate

Run at the card tip, rebased onto `ebc0fd49`, with
`git submodule update --init --recursive` done first. Exits reported honestly.

| command | result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **4,580 passed / 0 failed / 8 ignored**, 230 test executables / 232 result summaries, exit 0 |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0, zero warnings |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit **0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit **0** |
| `python3 scripts/check_lean_audits.py` | exit 0, **132 Lean audits** passed (131 → 132, this card's) |
| `python3 -m unittest scripts.tests.test_build_matrix_sections` | 17/17 ok |

Provider tests are **carried, not re-measured**: no provider source, manifest
template or `qcore-schemas` major version moved, so constraint 18 is not
engaged.

**The contested host row, and which direction this card saw — both, on two
different trees.** `qcore-verifier-host`'s
`closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request`:

* On the **pre-rebase** tree (this card over `148eef79`, *without*
  `VERIFIER-HOST-CONTROL-BUDGET`) it was **RED under the full suite** —
  4,572 passed / 1 failed / 8 ignored, `Err(Timeout)` at
  `standalone_host.rs:212`, the sole failure in the run.
* On the **final** tree (rebased through `3ead460b` and `ebc0fd49`, which
  merge `VERIFIER-HOST-CONTROL-BUDGET`) it is **GREEN under the full suite**
  and **green 3/3 re-run in isolation**.

**The obvious inference from that pair is wrong, and a sibling card's evidence
is why.** It would read as a before/after across `VERIFIER-HOST-CONTROL-BUDGET`
— the repair, not the load, moving the row. But `MUTEX-SHARE-CROSS-ARMS`, also
wave 35 and also on a tree carrying that repair, records the same row "seen red
in isolation, red under load *and* green under load across the session". So the
row still flips in both directions **after** the repair, this card's four
observations are consistent with that and do not distinguish it from a
before/after, and the causal reading is withdrawn. What the four measurements
support is only the narrower claim already on record: the standing description
("red in isolation, green under a full suite") does not match the tree in either
direction. They are offered to `HOST-ROW-STARVATION-DESCRIPTION`
(`CT-CTWIN-LANE`) as further data points, not as its discharge. This card
touched nothing in `qcore-verifier-host` and claims nothing beyond what it ran.

## 10. Reproduction

Scripts are archived beside this report under `scripts/`; they read the pinned
tree at `/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin`
and write only to the worktree-private `measure/` directory. Each audit run
takes about four and a half minutes.

```sh
git submodule update --init --recursive
cargo build --release -p qcore-compiler --example cycle19_fluent_bit_libraries_audit
cp target/release/examples/cycle19_fluent_bit_libraries_audit \
   measure/joined-provenance-cycles/bin/audit-after
python3 -B measure/joined-provenance-cycles/run_audit.py after \
   measure/joined-provenance-cycles/bin/audit-after
python3 -B measure/joined-provenance-cycles/byte_identity.py \
   measure/joined-provenance-cycles/runs/before \
   docs/reports/c-gateway-constant-do-while/artifacts
python3 -B measure/joined-provenance-cycles/leaves.py
python3 -B measure/joined-provenance-cycles/census.py
python3 -B measure/joined-provenance-cycles/attribution.py
python3 -B measure/joined-provenance-cycles/mutations.py
```

The BEFORE binary is the same build with
`crates/qcore-c-gateway/src/legacy_joined_provenance.rs` at the card's base
commit `ebc0fd49`. `census.py` and `attribution.py` additionally need the
temporary instrumentation, archived verbatim as
`scripts/legacy_joined_provenance.instrumented.rs`; it is **not** in the shipped
source, and the published BEFORE/AFTER artifacts were produced by binaries built
without it.

## 11. For the integrator

Apply in these words.

1. **Ledger, the corpus baseline pointer.** Applied in this card's own commit:
   the baseline moves to `docs/reports/joined-provenance-cycles/artifacts/`,
   described as "wave 35's second arbiter, whose fresh BEFORE reproduces
   `c-gateway-constant-do-while`'s `diagnostics.json` and
   `chunk-contracts.json` byte for byte and which moved **zero** leaves, zero
   functions and zero units", with the chain list gaining
   `joined-provenance-cycles` over `c-gateway-constant-do-while` and the frozen
   count going from five to **six**.
2. **Ledger, the Cycle 19 corpus row.** Applied: after the first arbiter's
   `double_release` clause, "`JOINED-PROVENANCE-CYCLES` (wave 35's second
   arbiter) then moved **zero** leaves, zero functions and zero units, so all
   four counts above stand as its first arbiter left them: the
   conditional-provenance relation was extended to cyclic CFGs by a greatest
   fixed point, is exercised on 40 corpus functions, and is blocked at
   `release_site`'s nullable-site and naming premises at every one of the 14
   sites that reach them." **No count changes**: 272 / 4,214 / 50,106 / 0 and
   186 Safe / 84 provisional Unsafe / 4,564 Unknown are all as the first arbiter
   left them. Please re-read both edits against this report before merging, since
   they were written while arbiter 1 was in flight.
3. **Standing constraint 10b.** Replace the sentence "Conditional provenance is
   derived separately from the custody worklist, over acyclic CFGs only, for
   unshadowed automatic locals whose addresses are never taken" with:
   "Conditional provenance is derived separately from the custody worklist, for
   unshadowed automatic locals whose addresses are never taken. An acyclic CFG
   is solved in one topological pass; a cyclic CFG is solved by the greatest
   fixed point of the same transfer (card `JOINED-PROVENANCE-CYCLES`), which
   derives facts **only for sites whose creating node lies on no cycle** — a
   site created inside a loop stands for one abstract resource per site rather
   than per dynamic object — and which declines the function outright if it has
   not settled within its iteration bound." Then delete the later sentence
   "and cyclic CFGs derive no facts". Every other sentence of 10b stands
   unchanged, including "This does not close the production `input_chunk_get`
   residual", which this card re-measured and confirms (§8).
4. **Diagnostics namespace.** No code allocated, no band widened, no condition
   changed. This card is **not** an instance of the `Q-NAMESPACE-DECISION`
   condition-extension rule; it emits no new diagnostic.
5. **Constraint 14 ledger.** Add `JOINED-PROVENANCE-CYCLES` to the list of cards
   that "do not engage this constraint at all" — it reserves no word and does
   not move the lexer.
6. **Increment record.** One paragraph, newest first, in
   `docs/development/increments/wave-35.md`; that file already exists.
7. **Matrix — applied, and here is exactly what moved.** The three residual
   names in §8 (`JOINED-PROVENANCE-CYCLE-SITES`,
   `JOINED-PROVENANCE-RELEASE-NULLABLE`, `JOINED-PROVENANCE-FACTS-NAME-NOTHING`)
   are registered in `named_followups`, and the matrix was regenerated through
   `python3 docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/wave33-close/gate.json`
   — the same `--gate` the committed matrix records, passed deliberately so the
   regeneration does **not** clobber that integrator-set provenance label with
   `carried_from_previous_matrix` (constraint 25). Nothing was hand-edited into a
   derived field. The whole diff is: `named_followups` **95 → 98**, `lean_audits`
   **130 → 131** with `AuditJoinedProvenanceCycles.lean` added to the derived
   list, and the three entries themselves. `scripts/tests/test_build_matrix_sections.py`
   passes 17/17, byte-format test included. `AGENTS.md`'s own stated follow-on
   count is updated **95 → 98** in the same commit, since a headline number this
   card moves is this card's to keep true.
8. **`CONSTANT-DO-WHILE-JOINED-PROVENANCE-REACH` is answered, and may be
   retired in this card's record rather than by editing arbiter 1's.** That
   residual asked that this card's coverage be quoted against the
   do-while tree rather than `a300a39d`. It is: every figure in §2 and §6 is
   measured on `148eef79`, the 654-function flip is reproduced independently
   here (1,927 − 654 = 1,273 cyclic), and the combined diagnostic effect of the
   flip and this card is **zero**. What that residual could not say, and this
   card can, is that the 654 cost this card six functions and 20 leaves of
   servable population — reach that moved to the topological pass, not reach
   that was lost.
9. **Not to be read as coverage.** A sound extension that moves nothing is
   evidence about this route's premises, not new conformance. Nothing here
   promotes any unit or function, and the `Unsafe` findings the corpus already
   carried remain provisional and subject to their reports' boundaries.

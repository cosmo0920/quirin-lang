# C03 — Specify a finite recursive-summary profile — 2026-09-07

Design card. Baseline `a4492da`. The deliverable is
[`docs/design/W4_RECURSIVE_SUMMARY_PROFILES.md`](../../design/W4_RECURSIVE_SUMMARY_PROFILES.md);
this file is the archive record.

**No production code changed.** The whole diff is one new design document, one
new design-probe test, one native hazard fixture and its two logs, and this
record. No analysis, checker, contract, profile or schema file is touched, so
the pinned 185-unit artifacts, the CMetrics audit, the contract corpus (59
modules) and CT coverage (11/50 mapped) are unchanged by construction. Raw
Cycle 19 units remain **2 Safe / 42 provisional Unsafe / 137 Unknown** plus four
data-only units; **no raw unit is promoted** and no verdict moves.

## 1. Selected family and why

**Neutral-frame recursion (NFR)**: a strongly connected component of the call
graph whose members are published with the *neutral* contract — every formal
`Borrowed`, result `None` — validated under an assumption fixed at that same
neutral element.

The two implemented recursive routes (contextual zero/nonzero integer control;
unsigned `n - 1` countdown) constrain the **control shape** and admit arbitrary
custody effects, paying with a well-founded rank. NFR takes the orthogonal
axis: it constrains the **effect content** to the bottom element and leaves the
control shape completely free — self, mutual, unranked, any arity.

A read-only census of the Cycle 19 audit scope (309 `.c` files, 8,239 extracted
definitions, in `build/cycle19/fluent-bit-clang-source`) motivates the choice:
**45 self-recursive functions and 27 file-local cycles**, bucketed **A
custody-neutral 25 / B scalar-rank 2 / C heap-link chain 2 / D other 16**. The
implemented routes reach at most 3 of 45 functions in this corpus; the neutral
bucket is the largest by a wide margin. The census heuristic does not
preprocess, detects calls lexically (one function-pointer release,
`cJSON_Delete`, was found and reclassified by hand), uses a name-based resource
token list, and builds per-file call graphs only — so 27 cycles is a lower
bound and every count is an estimate, not an inventory.

## 2. Domain, in one paragraph

Per member the assumption is `A(m) ∈ {Uncontracted ⊏ Neutral}`; component
states are pointwise maps, the lattice is finite of height `|C| + 1`. The
iteration starts at all-`Neutral`, demotes every member whose body fails
validation, and repeats — a monotone descent computing the greatest
post-fixed point in at most `|C|` rounds. **No join and no widening are used**;
the intraprocedural domain and its joins are untouched. Publication is
all-or-nothing per component, because demoting one member makes its callers'
call sites unmodelled (cascade lemma). The alias obligation is **empty** by
construction: no formal is bound, no containment edge or out slot is
sanctioned, and the returned value is untracked, so a caller that treats it as
a resource is fail-closed rather than silently wrong.

The neutral contract is read as two statements — (N-exit) "if it returns,
nothing the caller holds changed" and (N-prefix) "at every point of the
execution, returning or not, no custody event touches a caller-held identity".
(N-prefix) is prefix-closed, which is exactly why the optimistic start is
admissible here and nowhere above it.

## 3. Validation contract

Ten obligations, V1–V10, in §5 of the design document: declared components
excluded (V1), existing routes first and never weakened (V2), only the neutral
candidate ever seeded for a member of its own component (V3), component-level
iteration and publication (V4), body re-analysis with an **empty** entry state
reporting `Safe` with zero diagnostics (V5), the unchanged
`observations_match_contract` gate (V6), refusal when ordinary derivation reads
a different contract off the same body (V7), abstract — explicitly not
concrete — exit reachability (V8), unchanged schema/trust surface (V9), and a
certificate recheck that repeats the entire component computation (V10).

## 4. Prototype and counterexample

The route is **emulated** over unchanged production code by putting the
candidate contract in the declared environment — a declared contract is what
the recursive call site sees, and the body is then checked against it by the
ordinary analyzer, which is exactly the assume-then-validate step. Probe:
[`crates/qcore-c-gateway/tests/c03_neutral_frame_recursion_probe.rs`](../../../crates/qcore-c-gateway/tests/c03_neutral_frame_recursion_probe.rs),
8 tests, 18 measured fixtures, all passing.

**Feasibility (positive).** Under the neutral assumption a self-recursive
member, a member with *no* decreasing argument, a mutual pair and a member that
acquires and releases a local all validate, publish
`[Borrowed, …] → None`, and turn their caller from `Unknown` into `Safe`.
Without any contract, the same component and its caller are both `Unknown`
today with `call to uncontracted function`.

**Discrimination (negative).** Releasing a formal, consuming a formal, leaking a
local acquisition, storing a formal into a global, acquiring and returning, and
calling an uncontracted external each refuse the candidate — as `UnknownAlias`,
`Leak`, `SummaryConflict` or `Unsupported`. The neutral assumption certifies
the *absence* of effects and can never certify one.

**Counterexample fixing the assumption at ⊥.** With `resource_released(p)` in
force, `void purge(void *p, int n) { if (n > 0) { purge(p, n - 1); return; } purge(p, 1); }`
validates against its own contract, publishes `Released`, and the caller's leak
disappears — the run is `Safe`. `void *mint(int n) { void *r = mint(n); return r; }`
mints `Owned{nonnull}` the same way. Every execution of both diverges, so the
exit reading is vacuously true. A variant with a concretely reachable base exit
that does not perform the effect *is* refused, so this is a divergence
artefact, not a hole in the exit comparison. **Validation by re-analysis alone
therefore cannot refuse a circular effectful candidate**; in production that
refusal comes from the derivation strategy, and any future effectful route must
carry a rank. Declared contracts are trusted assumptions, so this measurement
reports no production defect.

**Emulation gap, pinned not hidden.** A declaration outlives its own member's
refusal, so in a mixed mutual pair the emulation leaves the neutral member
`Safe` where the route would demote the component. That is the one acceptance
obligation U1 cannot inherit from this probe, and it is why V4 fixes
publication at component granularity.

## 5. Rejected family and its boundary

Heap-link chain teardown (`purge(n->next); free(n);` — the corpus's two destroy
cycles) is **rejected**, architecturally rather than for effort:
`ResourceParameterMode` names formals, so "releases everything reachable
through `next`" has no expressible form, and `resource_released(head)` is not a
sound abstraction of it — a caller holding an interior node keeps a live site
for freed memory. `ResourceReturnMode` has the matching gap: a returned pointer
cannot be declared an alias of an existing site.

Native evidence, fixed observation under ASan/UBSan
(`chain-teardown-hazard.c`, built with
`clang -std=c11 -Wall -Wextra -Werror -g -O1 -fsanitize=address,undefined -fno-omit-frame-pointer`):

| Run | Result | Log |
|---|---|---|
| `interior` | `heap-use-after-free` reading the interior node after `purge(head)`, freed inside `purge` | [`logs/hazard-interior.log`](logs/hazard-interior.log) |
| `cyclic` | `stack-overflow` inside `purge` — on a cyclic chain no release is ever reached, so every exit-state contract for it is vacuous | [`logs/hazard-cyclic.log`](logs/hazard-cyclic.log) |

Today the gateway refuses this shape at the indirect-access gate, so no false
`Safe` is reachable through it now; the incompleteness becomes exploitable
exactly when interior aliases become representable. **Boundary:** no inference
route may publish a custody contract for a chain-releasing function until a
reachable-region release mode exists in the schema, the caller-side domain can
represent membership in that region, and acyclicity is a declared, recorded
premise — all three, not any two.

## 6. Follow-up implementation units

Enumerated with acceptance criteria in §9 of the design document:
**U1** NFR derivation route (S) · **U2** greatest-fixed-point descent (S) ·
**U3** certificate rederivation (S) · **U4** Lean theorems (M) ·
**U5** C02-P borrowed pointer-field reads (M) · **U6** reachable-region release
mode (L, gated by the §8 boundary) · **U7** self-dependent record hygiene (S).
Recommended order U1 → U3 → U4, then U5, then re-measure, and only then
reconsider U6.

**Reach estimate, honestly bounded.** Recursion through a pointer field is
refused before any recursion rule is consulted (C02-P), and 16 of the 25
neutral recursive functions recurse that way. NFR alone reaches ≈ **9**
functions in this corpus; with U5 it reaches ≈ **25**. U1 must re-measure the
185-unit and CMetrics artifacts and record any newly surfaced provisional
`Unsafe` unit rather than suppressing it.

## 7. Gate

Run from the worktree root at this checkpoint.

| Check | Result |
|---|---|
| `cargo test --workspace` | **1005 passed**, 0 failed, across 74 test binaries (997 before this card; +8 design-probe tests) |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `cd proof && lake build` | ok |
| `python3 -B scripts/check_lean_audits.py` | 21 audits pass |
| `cargo test -p qcore-c-gateway --test c03_neutral_frame_recursion_probe` | 8 passed |
| ASan/UBSan hazard fixture | both runs fail as designed (§5); that failure *is* the evidence |

Command-by-command exit codes are in [`logs/gate.log`](logs/gate.log); the probe
run is in [`logs/c03-probe.log`](logs/c03-probe.log). The Libevent submodule had
to be initialised in this worktree before the gate would run
(`git -c protocol.file.allow=always submodule update --init --recursive`,
`d82464a`); that is an environment condition of the isolated worktree, not a
finding.

## 8. Open items

- The design's premise **NFR-P1** — the analyzer's fail-closed treatment of
  unresolved identities is complete for custody events — is the pre-existing
  gateway premise. NFR adds no trust but leans on it harder; U4 should state it
  explicitly rather than proving it.
- **Recorded, not repaired**: with a declared contract in force, W4 can publish
  an `InferredValidated` record for that same function that was derived *using*
  the declaration (measured on the two divergent counterexample fixtures).
  Declared contracts govern call sites regardless, so no extra authority is
  created and this is not a soundness defect; the risk is a consumer reading
  such a record as independent corroboration. Unit U7.
- Cross-translation-unit recursion is invisible to the census and is not
  addressed by the design; component detection in `legacy_summary.rs` is
  per-translation-unit, so U1 inherits that boundary.
- Nothing here establishes termination, stack bounds, or leak-freedom across a
  divergent call, and nothing is promoted.

## 9. Proposed edits to files this card may not touch

- `AGENTS.md` / `CLAUDE.md`: nothing is required. If the wave paragraph is
  extended later, the accurate sentence is: *"C03 specifies neutral-frame
  recursion — a recursive-summary profile whose assumption is fixed at the
  neutral contract, with an empty alias obligation and no rank — rejects
  heap-link chain teardown with a three-part boundary, and lands no production
  change; raw units, CT coverage and the contract corpus are unchanged."*
- `docs/reviews/r0_3/acceptance_matrix.json`: C03 is a `bounded_design` card
  and discharges no requirement row; if the matrix records design cards, this
  one closes with *design + rejection boundary*, not with an implementation.

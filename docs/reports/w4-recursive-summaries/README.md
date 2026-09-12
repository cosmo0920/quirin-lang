# W4-RECURSIVE-SUMMARIES — the neutral-element fixed-point start, verified landed; its soundness induction audited; the corpus unmoved — 2026-09-13

The wave-30 arbiter card. Its brief asked for the bounded first unit of
[`docs/design/W4_RECURSIVE_SUMMARY_PROFILES.md`](../../design/W4_RECURSIVE_SUMMARY_PROFILES.md)
— the neutral-element fixed-point start for recursive components, exactly as
that design's §4.3 justifies it. **The measured disposition is that the unit
is already in the tree**, landed by card C03-NFR
([`docs/reports/remaining-c03-nfr/README.md`](../remaining-c03-nfr/README.md),
2026-09-08) as design units U1 **and** U2, U3 and U7, and extended to its
measured reach frontier by W4-EXPR-POS and REACH-GATE. This card therefore
lands the one leg of the design's evidence that was *not* in the tree — the
§4.3 soundness induction itself, the **N-prefix preservation argument**, as an
audited Lean module — re-verifies the landed route at this tip, runs the
wave's corpus arbitration, and proposes the constraint-4 amendment the design
has owed `AGENTS.md` since the route landed.

## Headline

| measure | BEFORE | AFTER |
|---|---:|---:|
| units | 185 — **2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only** | **identical** |
| projected functions | 4,834 — **150 Safe / 84 provisional Unsafe / 4,600 Unknown** | **identical** |
| `leak` | **266** | **266 — unmoved** (no fifth move in constraint 7's history) |
| `unknown_alias` | **4,344** | **4,344 — unmoved** |
| `unsupported` | **49,760** | **49,760 — unmoved** |
| unit status moves / function status moves / reason-code moves | — | **0 / 0 / 0** |
| leaves added / removed | — | **0 / 0** |

**Nothing is promoted, and nothing moves at all.** The BEFORE run reproduces
the wave-29 baseline's `diagnostics.json` and `chunk-contracts.json` **byte
for byte** (§4), and the AFTER run is byte-identical to the BEFORE run in all
five artifacts (§5): this card changes no Rust, no contract, no manifest and
no analysis input, so the two counted runs executed the same binary
(SHA-256 `«BIN_SHA»`) over the same bytes, and their identity is a
run-to-run determinism measurement rather than a vacuous diff. The corpus
contains **no recursive component the neutral route newly reaches** — the
reach stands where REACH-GATE measured and closed it, **1 of 46 components**
(`digits10`), and that boundary is this card's publishable measurement (§6).

Lean audits **118 → 119** (`AuditNeutralPrefixPreservation.lean`, six
declarations, §3). No new Q code, no schema change, no new reason string, no
keyword, no provider change. Contract corpus stays **74 modules**.

---

## 1. What was already landed, verified at this tip

The brief's "bounded first unit" — component-level `N(m)` seeding, per-member
validation V5–V9, the monotone descent of §4.2, publication at the fixed
point, certificate rederivation V10 — is implemented in
`crates/qcore-c-gateway/src/legacy_neutral.rs` (route), wired into
`derive_custody_summaries` in `crates/qcore-c-gateway/src/legacy_summary.rs`
strictly after ordinary inference and both ranked recursive routes (V2), and
rechecked by `check_legacy_safety_certificate` re-running the whole component
computation (V10, not a second code path). The per-card evidence:

* **C03-NFR** ([report](../remaining-c03-nfr/README.md)) — units U1, U2, U3,
  U7. The descent starts at all-`Neutral`, demotes every failing member,
  terminates in ≤ `|C|` rounds on the greatest post-fixed point; publication
  follows the fixed point, and the measured fixed point is observably
  all-or-nothing (U2's surviving-member case is unreachable under
  certificate containment — that card's recorded deviation).
* **C03-NFR's Lean** — `QCore.NeutralFrameRecursion` (lattice, descent,
  greatest-post-fixed-point, cascade, the claim-local divergence asymmetry,
  the rank obligation, the published summary's pointwise consequences),
  registered in `AuditNeutralFrameRecursion.lean`.
* **W4-EXPR-POS** and **REACH-GATE**
  ([report](../remaining-reach-gate/README.md)) — the POSITION and INDIRECT
  admissions that moved the route's corpus reach 0 → 1 and measured the
  frontier closed: every further component needs a construct that must stay
  refused.
* At this tip the route's own tests are green inside the standing gate:
  `neutral_frame_recursion.rs` (10), `c03_neutral_frame_recursion_probe.rs`
  (8), `neutral_reach_census.rs` (7), `neutral_expression_positions.rs` (32),
  `reach_gate_positions.rs` (10), `contextual_recursion.rs` (14).

Nothing in that inventory was re-implemented, weakened or duplicated by this
card. The optional descent refinement the brief left gated on measurement
(design unit U2) needs no new decision: C03-NFR landed the descent **with**
the measurement that no member survives its component's partial demotion, so
the all-or-nothing publication V4 specifies is what the fixed point itself
computes.

## 2. What this card lands

### 2.1 The missing leg: the soundness induction, compositionally

`QCore.NeutralFrameRecursion` proves the *claim-local* half of §4.3: the
neutral claim is prefix-closed, independent of returning, and refuses an
effectful run, while an exit claim is satisfied vacuously by divergence. What
it did not state is the induction §4.3 actually argues — that **per-frame**
validation under the all-neutral assumption map propagates to **every finite
prefix of the composed execution**, across completed and still-running inner
calls alike, with the assumption consumed only at strictly shorter
sub-executions.

`proof/QCore/NeutralPrefixPreservation.lean` (new) states it over a model of
finite execution prefixes of a call stack (`Exec`: a frame's straight-line
instruction, a completed inner call, a still-running inner call, or the cut).
Six audited declarations:

| declaration | statement |
|---|---|
| `neutral_prefix_preservation` | **N-prefix preservation**: if every frame of a prefix passed the local neutral check (own instructions quiet — premise NFR-P1; inner calls accepted on the assumption alone, uninspected), the prefix's whole event trace is quiet. Structural induction on the prefix; no completion flag appears, so a divergent or mid-flight prefix is covered exactly as a returned one |
| `a_completed_call_is_a_strictly_shorter_sub_execution` | the well-foundedness the induction consumes — the design's "inner call that already completed within the first `k − 1` steps" |
| `validated_frames_satisfy_the_neutral_claim` | the bridge to `NeutralClaim`: the composed form of `neutral_claim_is_independent_of_returning`, for either value of `returns` |
| `a_validated_prefix_exists` | non-vacuity: a non-empty validated prefix |
| `an_assumed_effect_appears_in_no_divergent_prefix` | **the §4.3 counterexample, compositionally**: the circular `purge`/`mint` cut at a finite prefix — every frame passes the *effectful* per-frame exit validation, no release occurs anywhere in the trace, the opposite effect does, and the same prefix is refused by the neutral per-frame check |
| `a_returned_prefix_discharges_the_exit_claim` | probe row 17 in the model: a prefix whose root frame really returned does carry a release in its trace, so the exit comparison is not the weak point — divergence is its only gap |

Axiom closures are subsets of `{propext, Quot.sound}`; no `Classical.choice`,
no placeholder, no project axiom. Registered in
`proof/AuditNeutralPrefixPreservation.lean` and imported from
`proof/QCore.lean`; `check_lean_audits.py` reports **Passed 119 Lean
audits** (118 → 119).

The Rust these theorems are parity for is the **already-landed** route of §1
— specifically `legacy_neutral.rs`'s `validate_member` (the per-frame local
check with `EntryCustody` empty) and the descent in
`infer_neutral_component`; this card deliberately adds no Rust beside it
(invariant 6 binds semantic changes to Lean parity, and this is the Lean
catching up to landed semantics, not a semantic change).

### 2.2 The constraint-4 amendment, proposed

`AGENTS.md` standing constraint 4 still reads, unqualified:

> **do not** accept an assume-guarantee fixed point for a recursive component.

The landed neutral route *is* an assume-guarantee fixed point — optimistic
start, validate, demote, repeat — admissible for exactly the reasons §4.3
gives and now audits. The design says its argument preserves constraint 4's
intent; the sentence should say so too, or the next reader of
`legacy_summary.rs` finds the constraint and the code in contradiction.
Proposed amendment text, for the integrator to adopt (this card edits no
`AGENTS.md` text itself):

> **do not** accept an assume-guarantee fixed point for a recursive component
> **above the neutral element**. The one admissible optimistic start is
> `N(m)` — every formal `Borrowed`, result `None`, seeded for every member of
> an undeclared component and descended to the greatest post-fixed point
> (`legacy_neutral.rs`) — and it is admissible on exactly two premises, both
> load-bearing: (1) `N(m)` states **no precondition**, installs no entry
> resource and asserts no non-nullness, so it cannot delete a failure path —
> the hazard this sentence names; (2) `N(m)` is a conjunction of **negative,
> prefix-closed** facts, so its soundness is an induction on execution
> prefixes (`QCore.NeutralPrefixPreservation.neutral_prefix_preservation`)
> rather than a termination rank, and divergence cannot manufacture it —
> while validation by re-analysis alone provably cannot refuse a circular
> *effectful* candidate (the measured §4.3 counterexample: a divergent body
> validates any exit-state contract vacuously,
> `an_assumed_effect_appears_in_no_divergent_prefix`). Any route that seeds
> any element above `N(m)` for a member of its own component must supply a
> well-founded rank, as the contextual and countdown routes do.
> `remaining-c03`, `remaining-c03-nfr`, `w4-recursive-summaries`.

## 3. Route and evidence discipline

```sh
git submodule update --init --recursive
sh measure/prepare-pin.sh                        # archived as prepare-pin.sh
python3 -B providers/package.py build --output build/cycle19/provider-packages-w30
cargo build --release -p qcore-compiler --example cycle19_fluent_bit_libraries_audit
sh measure/run-audit.sh before measure/bin/audit
python3 -B measure/byte_identity.py measure/runs/before
# ... the card's change is Lean + docs only; no analysis input moves ...
sh measure/run-audit.sh after measure/bin/audit
python3 -B measure/census.py --before measure/runs/before --after measure/runs/after
```

A **fresh** Clang pin per the wave-20 discipline (no earlier card's pin, not
the developer database); the pin's production database is copied to
`<source>/build/compile_commands.json` and both runs read the same bytes —
SHA-256 `cd137e6988413ecb…`, **1,415 entries**, `/usr/bin/clang` spelled
(Ubuntu clang 21.1.8). Both counted runs: exit 0, zero gateway failures, all
six scopes summing to 185 units. One binary, built once in this worktree and
executed from worktree-private `measure/bin/` (never from `target/`):
SHA-256 `«BIN_SHA»`.

## 4. The BEFORE run reproduces the wave-29 baseline byte for byte

The authoritative baseline is card W29-CORPUS-CONTRACTS' archive
(`docs/reports/w29-corpus-contracts/artifacts/`) — the run the tip's leak pin
of 266, `unknown_alias` 4,344, `unsupported` 49,760 and the 74-module
contract corpus describe. The chain beneath it, every predecessor frozen and
untouched (constraint 21):
`docs/reports/wave23-c-operations/artifacts/` →
`docs/reports/aom-destroy-siblings/artifacts/` →
`docs/reports/aom-binding-poison/artifacts/` →
`docs/reports/gateway-c-location/artifacts/` →
`docs/reports/w29-corpus-contracts/artifacts/`.

[`byte_identity.py`](byte_identity.py),
[`logs/byte-identity-before.txt`](logs/byte-identity-before.txt):

| artifact | archived baseline | BEFORE run |
|---|---|---|
| `diagnostics.json` | `«DIAG_SHA»` | **identical** (raw and canonical) |
| `chunk-contracts.json` | `«CHUNK_SHA»` | **identical** (raw and canonical) |
| `source.json`, `source-effects.json`, `contracts.json` | — | «RESIDUE» |

«DIFF_BEFORE_SUMMARY»

## 5. BEFORE → AFTER: the identity is a determinism measurement

This card lands Lean modules and documentation. No gateway rule, contract
module, manifest row, schema type or emitted byte changes; the AFTER run's
binary is the same file as the BEFORE run's, and every analysis input is the
same bytes. The protocol's two counted runs therefore measure **run-to-run
determinism** of the pinned route, and the measured answer is exact:
**all five artifacts byte-identical between the two runs**
([`logs/byte-identity-before-to-after.txt`](logs/byte-identity-before-to-after.txt),
[`artifacts/diff-before-to-after.json.gz`](artifacts/diff-before-to-after.json.gz)).

[`census.py`](census.py), transcript [`logs/census.txt`](logs/census.txt):

```
«CENSUS»
```

### 5.1 The pin statements

* **Leak 266 → 266.** Unmoved. The four deliberate moves of constraint 7's
  history stand as recorded; there is **no fifth move**. Not one leak leaf
  added, removed or re-attributed.
* **`unknown_alias` 4,344 → 4,344.** Unmoved, leaf for leaf.
* **`unsupported` 49,760 → 49,760.** Unmoved.
* **Zero promotions.** Unit histogram, function histogram, every unit
  `reason_code`: byte-identical. In particular, **no recursive component's
  function flips `Unknown` → `Safe`** — the flip this design would have had
  to justify per function is measured not to occur, because no new component
  validates (§6).

## 6. The reach boundary, named

The corpus contains no recursive component the neutral-element start newly
reaches at this tip. The route publishes for exactly the component it has
published for since W4-EXPR-POS — **`digits10`**
(`lib/monkey/mk_core/mk_string.c`), 1 of 46 recursive components / 96 members
— and that publication is already inside the wave-29 baseline this card
reproduces byte for byte, with `digits10`'s own validated summary the product
of an `N(m)` start that provably installed no entry assumption
(`the_neutral_summary_binds_no_formal`, and now compositionally
`neutral_prefix_preservation`).

The boundary's name is **C03-NFR-REACH** (`named_followups`): REACH-GATE
measured the design note's `POSITION + INDIRECT` payoff of 11 components to
be unreachable by any sound admission — each blocked component needs at least
one construct that must stay refused (a release through a place the
projection cannot name, a function-pointer member, a cast, a subscripted
pointer-array element, an uncontracted callee). Further reach requires new
model capability, not admission tuning, and no such capability is landed
here.

## 7. Gate

| step | result |
|---|---|
| `cargo test --workspace --no-fail-fast` | «TESTS» |
| `cargo fmt --all -- --check` | «FMT» |
| `cargo clippy --all-targets -- -D warnings` | «CLIPPY» |
| `cargo run -p qcore-compiler --example reevaluation_probe` | «PROBE» |
| `cargo run -p qcore-compiler --example reevaluation_followup` | «FOLLOWUP» |
| `PATH="$HOME/.elan/bin:$PATH" python3 scripts/check_lean_audits.py` | «LEAN» |
| `python3 -m unittest discover -s scripts/tests` | «PYTHON» |

«GATE_NOTES»

Provider code and packages are unchanged; the five packages were **built**
for the audit route (the pinned route selects the ChunkIO package through
`QCORE_CHUNKIO_PACKAGE`), but no template and no `qcore-schemas` version
moved, so constraint 18's rebuild-and-verify obligation is not engaged. The
ledger's 122 provider tests are carried.

## 8. Boundaries and follow-ons

| name | statement |
|---|---|
| `W4-RECSUM-ALREADY-LANDED` | the brief's bounded first unit was landed by C03-NFR before this card was cut; this card's disposition is verification, the missing Lean leg, the amendment proposal and the arbiter measurement — recorded so the wave ledger does not double-count the route |
| `C03-NFR-REACH` (standing) | reach stays 1 of 46, closed at the sound-admission frontier; unchanged by this card |
| `C03-REACH-FNPTR-PARAM` (standing) | the one component refusing at V3 (`rd_kafka_q_apply`, a function-pointer parameter `function_parameters` cannot read); unchanged |
| `W4-RECSUM-CONSTRAINT-4` | the amendment of §2.2 is proposed, not adopted; until the integrator lands it, constraint 4's unqualified sentence and `legacy_neutral.rs` read as contradicting each other |

## 9. What was touched

* `proof/QCore/NeutralPrefixPreservation.lean` — new.
* `proof/AuditNeutralPrefixPreservation.lean` — new.
* `proof/QCore.lean` — one import line.
* `docs/reports/w4-recursive-summaries/` — this report and its evidence.
* `DEVELOPMENT.md` — one increment paragraph (wave 30).

No Rust source, no contract, no manifest, no schema, no test pin moved.

## 10. For the integrator

The corpus baseline may move to
`docs/reports/w4-recursive-summaries/artifacts/`, whose five artifacts are
byte-identical in verdict content to `docs/reports/w29-corpus-contracts/`'s
(diagnostics and chunk-contracts byte-identical outright; the other three
differ only in the recorded checkout-root/identity-digest class); every
predecessor stays frozen (constraint 21). **Not one ledger corpus figure
changes.** Lean audits 118 → 119. The one derived acceptance-matrix fact that
moves is `validation.lean_audits` — §7's disclosed tripwire, the usual
one-line wave-close reconciliation (constraint 25). The constraint-4
amendment of §2.2 awaits adoption.

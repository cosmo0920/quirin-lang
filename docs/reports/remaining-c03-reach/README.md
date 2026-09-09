# C03-REACH — re-measuring neutral-frame recursion's corpus reach — 2026-09-09

Follow-on to [C03-NFR](../remaining-c03-nfr/README.md), whose §12 records:
"NFR's measured reach at `66910c10` is **0** … Re-measure this card's reach
once C02-P lands." C02-P has landed, and so have O04-ARMS, O04-B08 and
O04-READ-BOUND.

**Baseline:** main `695f0e1` (wave-10 merged tree). **Result: the reach is
still 0.** No unit, function or diagnostic moves; the five pinned corpus
artifacts are byte-identical before and after this card's change. Nothing is
promoted. `qcore-schemas` stays 8.0.0; no crate version moves; no diagnostic
code is added or changed; no checker rule is widened or narrowed.

## 1. What landed

Three things, in order of weight.

1. **A direct measurement replaces an inference.** C03-NFR's reach figure was
   derived from published per-function diagnostics. Those are computed under
   the *ordinary* environment, in which a member's own recursive call is an
   uncontracted call — a different measurement from the one the neutral route
   makes, which examines the same body with `N(m)` in force for every member of
   its component. `crates/qcore-c-gateway/src/legacy_census.rs` (new) reads the
   refusal off the route itself.
2. **A corrected population.** C03-NFR reported "12 self-recursive functions in
   6 units actually reach derivation". The census finds **46 recursive
   components, 96 members, across 21 units** — all twelve of C03-NFR's are
   present, plus 80 more, including 17 mutually recursive components its
   self-recursion scan could not see. The zero is unchanged; the denominator it
   is zero out of was eight times too small.
3. **A measured disposition of the next admission**, §5 and
   [`docs/design/W4_NEUTRAL_EXPRESSION_POSITIONS.md`](../../design/W4_NEUTRAL_EXPRESSION_POSITIONS.md).
   The card's licence to implement inline applied only to "a trivial
   compositional gap in an existing admission"; the measurement shows the gap
   is neither trivial nor single-family, so the deliverable is a design note.

## 2. How the census is inert

`legacy_neutral::validate_member` becomes `explain_member`, returning the
refusing obligation instead of a bare `bool`. The route branches on `is_err`
alone, so what it publishes cannot depend on the naming. The one decomposition
is that `explain_member` tests `observations_match_contract`'s own first
condition — a `Safe` analysis with no diagnostics — before calling it, so that
a not-clean body is distinguishable from an exit disagreement; that is
behaviour-preserving because it is exactly what the gate tests first.

`CustodyDerivation` gains a `census` field, written by
`derive_custody_summaries` and read by `legacy_census` alone. No analysis, no
certificate, no report and no serialized artifact consults it. The public entry
point `neutral_reach_census` preprocesses and selects scope exactly as
`analyze_unrestricted_c11_with_inventory` does, then runs the same
`derive_custody_summaries` and returns only the census.

Seven tests in `crates/qcore-c-gateway/tests/neutral_reach_census.rs` pin every
disposal arm — admitted, refused at V5 on a not-clean body, refused at V5 on
the containment gate, declared (V1), credited to the ranked route (V2), empty
for a non-recursive unit, and all-or-nothing over a mutual pair. Each one also
runs the ordinary analysis on the same fixture and asserts the verdict and the
published summary record, so a census that could move a result would fail here
before it reached the corpus.

## 3. The census

Command, on the pinned prepared clone the canonical route uses:

```sh
QCORE_CLANG=/usr/bin/clang \
cargo run --quiet -p qcore-compiler --example c03_reach_census -- \
  <clone> <monkey>/compile_commands.json <census.json>
```

185 units examined — the same selection the pinned compatibility route makes.
Full data: [`artifacts/census.json.gz`](artifacts/census.json.gz)
(SHA-256 `db93d51ca10dccadf5e511850feb41570b577f1df92e732ac2e5799f5bae7e84`);
readable per-member table: [`census-table.md`](census-table.md).

| | |
|---|---|
| recursive components | **46** (29 self-recursive, 17 mutual) |
| members | **96** |
| units carrying one | 21 of 185 |
| libraries | librdkafka 32, ctraces 6, cmetrics 4, monkey-libevent 3, chunkio 1 |
| **admitted by the neutral route** | **0** |
| admitted by a ranked recursive route | 0 |
| kept by a declared contract (V1) | 0 |
| published by ordinary inference (V2) | 0 |

### 3.1 First refusing obligation

| obligation | members |
|---|---|
| `V5-retained-unknown-instruction` — the CFG retains an `Unknown` instruction | **94** |
| `V5-body-not-clean` — the analysis was not `Safe` with zero diagnostics | 1 |
| `V3-parameters-unreadable` — no `N(m)` candidate exists | 1 |

Ninety-four of ninety-six refuse at the **containment gate**, which runs
*before* the abstract interpretation. That is why this report quotes retained
instructions rather than published diagnostics: for 94 members the analysis
that would have produced diagnostics never ran.

The single `V5-body-not-clean` is `rd_kafka_topic_conf_destroy`, on one
unresolved resource identity. The single `V3` is `rd_kafka_q_apply`, whose
`int (*callback)(...)` declarator `function_parameters` cannot read; it is
recorded as follow-on **C03-REACH-FNPTR-PARAM** and is the only member no
blocker-family admission can help.

### 3.2 Blocker families, measured at the point of refusal

| occurrences | members (of 96) | family |
|---|---|---|
| 642 | 77 | indirect memory access is outside the resource projection |
| 450 | 73 | unmodeled access in a **branch condition** |
| 239 | 52 | call to an uncontracted function |
| 113 | 28 | unmodeled access in a **return expression** |
| 45 | 18 | unrecognized assignment destination |
| 28 | 17 | braced initializer, expression or declaration |
| 26 | 24 | unmodeled access in a **for-loop condition** |
| 12 | 11 | unmodeled access in a **switch expression** |
| 12 | 8 | unsupported assignment expression |
| 11 | 10 | unmodeled access in a **loop condition** |
| 6 | 6 | unmodeled access in a **do-while condition** |
| 3 | 2 | call with an unmodelled formal address or borrowed expression |
| 3 | 2 | unmodeled expression may mutate a formal resource identity |
| 1 | 1 | C operator requires type/operand analysis |

Distinct families per member: 1 for four members, 2 for 21, 3 for 32, 4 for 16,
and 5–9 for 22.

## 4. Comparison with C03-NFR's figures

C03-NFR recorded "**155** indirect memory access and **106** unmodeled call,
aggregate or indirect access in branch condition" over its twelve functions,
counted from the **published diagnostics**. Two readings, kept separate
because they are two different measurements:

| reading, restricted to C03-NFR's twelve | indirect access | branch condition |
|---|---|---|
| published diagnostics, C03-NFR's baseline | 155 | 106 |
| published diagnostics, wave-10 tree | **154** | **106** |
| this census — retained instructions under `N(m)` | 160 | 108 |

The first two rows are the like-for-like comparison: across C02-P, O04-ARMS,
O04-B08 and O04-READ-BOUND, **one** occurrence disappeared, from
`cJSON_Duplicate_rec`. The third row is higher because the census counts the
instructions the containment gate retains under the neutral environment, which
is not the same set as the diagnostics a completed analysis publishes — the
distinction §3.1 explains. The census's own totals over all 96 members are the
642 and 450 of §3.2.

Neither number is a criticism of the admissions that landed. C02-P, O04-ARMS,
O04-B08 and O04-READ-BOUND each targeted the constructor-to-caller path they
were scoped to and each measured its own effect there. They simply do not
intersect these bodies, which is a fact about the bodies.

## 5. Where the reach is 0: the most-common blocker, and its measured payoff

The most-common blocker is **`indirect memory access is outside the resource
projection`** — 642 occurrences in 77 of 96 members, 1.4× the next family and
present in 80% of refused members.

**Admitting it completely and soundly would admit zero components.**

That is the load-bearing finding of this card. The census computes, per family,
how many components would be admitted if that family were removed entirely — an
upper bound no real conjunctive whitelist reaches:

| family admitted completely | components admitted |
|---|---|
| indirect memory access (the largest) | **0** |
| unmodeled access in a branch condition | **0** |
| call to an uncontracted function | **0** |
| unmodeled access in a return expression | **1** (`digits10`) |
| every other family, individually | 0 |

Grouping by the gate that produces them — **POSITION** for the six
`unmodeled … in {branch, loop, for-loop, do-while, switch, return}` families,
which all come from one shared token predicate, and **INDIRECT** for the
member-access family:

| admitted | components | members |
|---|---|---|
| POSITION alone | 3 | 3 |
| INDIRECT alone | **0** | 0 |
| **POSITION + INDIRECT** | **11** | **17** |
| + braced initializers | 12 | 19 |
| + assignment-destination families | 15 | 30 |

The next admission must cover both halves or move nothing. That is the design
note's subject; §4 there classifies the 642 indirect-access sites from their
recorded source lines (291 chains, 238 single selections, 57 dereferences,
36 subscripts) and separates the part that may already be inside C02-P's stated
scope from the part — releasing through an unnamed place, `free(kvlist->values)`
— that must stay refused.

### 5.1 Why nothing was implemented inline

The card permitted an inline implementation only for a trivial compositional
gap. The measured gap is not one:

* it needs **both** halves to move anything;
* the POSITION half lives in `supported_neutral_expression`, a token-level
  predicate called during parsing with no access to the contract environment,
  and threading the environment there re-classifies branch conditions and
  return expressions in **all 185 units**, in the permissive direction;
* the single component one family away from admission is `digits10`, a
  `uint32_t digits10(uint64_t)` with no resource content whatsoever. Moving the
  reach from 0 to 1 on that function would not have justified a corpus-wide
  permissive change.

Naming that honestly is the deliverable. `digits10` is recorded in the design
note as the *minimal validation case* an implementing card should land first.

## 6. Corpus re-measurement

Two full runs of the canonical pinned route on this host, on identical prepared
inputs, with provider packages rebuilt at this tree:

```sh
python3 -B providers/package.py build --output <scratch>/provider-packages
QCORE_CLANG=/usr/bin/clang \
QCORE_CHUNKIO_PACKAGE=<scratch>/provider-packages/qcore.chunkio \
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  --pinned-evidence <clone> <monkey>/compile_commands.json \
  <out>/monkey-source.json <out>/resource-diagnostics.json
```

BEFORE at pristine `695f0e1`, AFTER at this card's checkpoint. Both exit 0 with
zero gateway failures, 185 units, and the merged headline unchanged.

| artifact | BEFORE vs AFTER | AFTER vs archived wave-10 baseline |
|---|---|---|
| `resource-diagnostics.json` | **byte-identical** | **byte-identical** |
| `resource-diagnostics.chunk-contracts.json` | **byte-identical** | **byte-identical** |
| `monkey-source.json` | **byte-identical** | **byte-identical** |
| `resource-diagnostics.contracts.json` | **byte-identical** | differs |
| `resource-diagnostics.source-effects.json` | **byte-identical** | differs |

The two artifacts that differ from the archive differ **identically before and
after this card**, which is what establishes the difference is environmental
rather than this card's. Both are the two residue classes wave 9 and wave 10
already record:

* `contracts.json`'s `sources` map is keyed by **absolute contract paths**, so
  it carries this worktree's directory (10 leaves, five paths, each present
  under both spellings with the same contract text);
* `source-effects.json` moves 32 digest/identity leaves — `build_digest`,
  `configuration_digest`, `obligation_id`, `task_digest` and
  `package_digest` — because those bind the contract source paths above and the
  provider package rebuilt into a scratch directory. **No `status`, verdict,
  reason or count field differs.**

The archived baseline therefore stays where wave 10 put it
(`docs/reports/wave10-corpus/artifacts/`); this card re-pins nothing.

## 7. Gate

Run from this worktree at the card checkpoint. Clang `Ubuntu clang 21.1.8`,
Libevent submodule at `d82464a`.

| check | result |
|---|---|
| `cargo test --workspace` | **2041 passed**, 0 failed, 4 ignored (127 binaries) |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 scripts/check_lean_audits.py` | **51 audits pass** (unchanged; §8) |
| `cd providers && cargo test` | 76 passed, 0 failed |
| `python3 -m unittest discover -s scripts/tests` | 125 passed |
| `providers/package.py build` | five packages built and verified |
| 185-unit corpus, BEFORE vs AFTER | 5/5 byte-identical (§6) |

The workspace count is 2041 against wave 10's 2034: exactly the seven new
census tests, and no other binary's count moves.

### 7.1 Known load-flaky, confirmed passing in isolation

`cycle20_verifier_plugins::vp04_vp07_vp08_process_lifecycle_and_payload_budgets`
failed once under full-workspace load on its 500 ms process-transport budget
(`assertion failed: elapsed < Duration::from_millis(500)`) and passed on a
rerun of that target alone (20 passed) and on a repeat of the whole workspace.
This is the flakiness already recorded for P02/P03 and in C03-NFR §11; the
assertion is deliberately not weakened.

## 8. Lean

**Judged not applicable, with the reasoning recorded** (the precedent is
T00-CT-SCHEMA, `docs/reports/remaining-t00-ct-schema/README.md`).

Hard invariant 6 requires matching Lean definitions and theorems for a
*semantic* change. This card makes none: the route publishes exactly what it
published before, on every input, and the corpus measurement in §6 is the
evidence. The relation the neutral route implements is already proved by
C03-NFR's 34 audited theorems in `QCore.NeutralFrameRecursion`, and this card
changes neither the relation nor its implementation's decisions — only what the
implementation *reports about* a decision it already made. A theorem about an
inert record would state that a measurement equals itself.

The design note in §5 is design only and proves nothing; its implementing card
carries the Lean obligation for whatever it admits.

## 9. Open boundaries

* **The reach is 0 and stays 0.** Nothing in this card admits a component,
  promotes a unit or moves a verdict.
* **C03-REACH-FNPTR-PARAM.** `function_parameters` cannot read a
  function-pointer declarator, so `rd_kafka_q_apply` has no `N(m)` candidate at
  all. One component; independent of every blocker family.
* **C02-P-ALIAS overlap, not resolved here.** Card C02-P-ALIAS runs in parallel
  and may make the projection strictly *more* refusing. This card's measurement
  used the wave-10 baseline
  (`docs/reports/wave10-corpus/artifacts/`, main `695f0e1`), and the
  before/after comparison in §6 is against pristine `695f0e1` on this host —
  not against C02-P-ALIAS's tree. If C02-P-ALIAS lands, the census numbers can
  only move in the refusing direction (more retained instructions, never
  fewer), so the reach-0 conclusion is stable under it; the per-family counts
  in §3.2 and the payoff table in §5 would need re-measuring. Cross-card
  reconciliation is the integrator's, deliberately not attempted here.
* **The census's own scope.** It reports the *first* refusing obligation under
  the all-`Neutral` seed. A member with several independent blockers is
  attributed to the first gate that stops it, which is why 94 members read
  `V5-retained-unknown-instruction` and only one reads `V5-body-not-clean`. The
  blocker families in §3.2 are the complete retained-instruction inventory for
  those members, so the family counts are not truncated by that ordering — only
  the obligation label is.
* **The payoff tables are upper bounds.** They assume a family is admitted
  *completely and soundly*, which no real conjunctive whitelist achieves. A
  real admission covering a fragment of POSITION + INDIRECT will admit fewer
  than 11 components, never more.
* **`build_matrix.py`, the CT manifest and the acceptance records are
  untouched**, as are AGENTS.md, CLAUDE.md, README.md and DEVELOPMENT.md.

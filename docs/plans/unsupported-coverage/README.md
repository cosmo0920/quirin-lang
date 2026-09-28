# Unsupported coverage: typed access paths and callee closure

Review date: 2026-09-27. Status: a proposed card queue, not dispatched work and
not a coverage claim. Priority is relative within this queue. It does not
override soundness repairs, G1/G4 ordering or the integrator's wave selection.

## 1. Measurements this plan rests on

**Source.** Every figure is read from the diagnostics of the 185-unit corpus
arbiter, archived at
[`w45-container-root-reach/integration/artifacts/diagnostics.json.gz`](../../reports/w45-container-root-reach/integration/artifacts/).
`main`'s diagnostics are byte-identical to it at `b1744a2a`, as every later
integration gate records.

**Headline figures.**
- Functions: 398 Safe / 9 provisional Unsafe / 4,427 Unknown.
- Unsupported leaves: 41,314, in 31 families.

The functions below are counted by the set of diagnostic families that block
them. A family counts as blocking whenever the function has at least one leaf in
it.

| Family (leaf description) | Leaves | Share |
|---|---:|---:|
| `IND`: indirect memory access is outside the resource projection | 17,042 | 41.2% |
| `UNC`: call to uncontracted function | 9,179 | 22.2% |
| `COND`: unmodeled call, aggregate or indirect access in a branch, loop or for-loop condition | 8,578 | 20.8% |
| `RET`: the same, in a return expression | 304 | 0.7% |
| `OTHER`: initializers, assignment forms, increments, operators and the rest | 6,211 | 15.0% |

**How many functions each fix could unblock at most.** A function counts when
its blocking families all lie inside the set being removed. This is an upper
bound: a modeled access can still fail its obligations.

| Removed families | Functions whose only blockers they are |
|---|---:|
| `IND` | 418 |
| `COND` | 83 |
| `UNC` | 436 |
| `IND` + `COND` | 763 |
| `IND` + `COND` + `RET` | 879 |
| `IND` + `COND` + `RET` + `UNC` | 2,045 |
| every unsupported family | 3,076 |

`unknown_alias` also blocks 1,359 functions, and a leak blocks 9.

**Uncontracted calls.**
- **Callee location:** 6,895 of the 9,179 uncontracted call sites (75%) call a
  function defined in the corpus: 2,170 of 2,560 distinct callees.
- **Callee verdicts:** 2,115 of those callees are themselves Unknown, 49 Safe and
  6 provisional Unsafe.
- **The long tail:** the 436 functions blocked only by `UNC` spread over 236
  distinct single callees. The largest is `cJSON_New_Item`, with 10.

## 2. Findings

1. **`IND` and `COND` are one problem in two positions.** Most `COND` leaves are
   the same pointer or member access appearing inside a controlling expression.
   Modeling access in statements alone reaches at most 418 functions. Modeling it
   everywhere reaches at most 879.
2. **The shape rules have reached their ceiling, and the ceiling is typing.**
   - **Current routes:** rules 5h–5l, the member-chain route and the
     call-argument route (reports `w42-indirect-access` through
     `w45-container-root-reach`). Each recognises token shapes, and each removed
     about 1,000 leaves.
   - **The residual classes are typing gaps**, as registered:
     - `W43-INDIRECT-UNTYPED-SINGLE-ACCESS`: 3,136 leaves.
     - `W44-CHAIN-NESTED-AGGREGATE`: 1,287.
     - `W44-CHAIN-TYPEDEF-ROOT`: 580.
     - `W44-CHAIN-UNDECLARED-ROOT`: 396.
     - `W44-CHAIN-INCOMPLETE-RECORD`: 195.
   - **Clang already has these types.** The gateway's typed-AST transport
     (`crates/qcore-c-gateway/src/typed_c_ast.rs`) serves the source-profile
     interpreters but not the resource projection.
3. **Uncontracted calls are mostly a bottom-up closure problem, not a contract
   backlog.**
   - Three quarters of the sites call corpus functions that are themselves
     Unknown.
   - A callee that becomes Safe serves its callers through the existing summary
     rules (W45-CALLEE-SUMMARY-CASCADE, constraint 8 for declared callees).
   - So making leaf functions Safe also unblocks callers. Only the external
     remainder, 2,284 sites to 390 callees, needs contracts in provider crates:
     libc only in `quirin-provider-libc`, and other libraries in their own
     crates.
4. **Pointer values stored into fields are a different model.** `p->next = q`
   and `p->buf = malloc(n)` move custody through fields. That belongs to the
   owned-field and container rules (5k, 10d, 10f), where STABILITY-REEVAL-5
   found `MEMBER-RELEASE-BEFORE-SUMMARY-RELEASE`: a double release published
   Safe. It is not part of the access-path model.

## 3. Priority queue

| Order | Card | Kind | Effort | Prerequisite | Intended decision |
|---|---|---|---|---|---|
| P0 | W45-SR5-SOUNDNESS | Soundness repair | High (done, `85a35431`) | none | Close the admitting findings of STABILITY-REEVAL-5 before any widening in the access and member areas, including `SCALAR-READ-PARENTHESISED-DOUBLE-DEREF` and `GATEWAY-5L-ELEMENT-TRACKED-AFTER-FOREIGN-UNLINK` |
| P1 | ACCESS-PATH-CENSUS | Measurement | Medium (done: [W46-ACCESS-PATH-CENSUS](../../reports/w46-access-path-census/README.md), prediction frozen: P2 38 Safe / 7 Unknown, P3 83 Safe / 137 Unknown) | P0 landed | Classify every `IND`, `COND` and `RET` leaf by its Clang-typed access path; predict, per function, verdicts rather than leaves; compute the callee-closure gain |
| P2 | ACCESS-PATH-STATEMENTS | Semantic implementation | High (bounded fallback landed; replacement parity open) | P1 | [Typed statement fallback](../../reports/w46-access-path-statements/README.md) runs beside retained shape routes; archived disagreements must be resolved before any route is retired |
| P3 | ACCESS-PATH-POSITIONS | Semantic implementation | High (landed: [W47-ACCESS-PATH-POSITIONS](../../reports/w47-access-path-positions/README.md), +60 Safe with the enum arm; residuals registered) | P2 | The same layer in conditions, returns and call arguments |
| P4 | CALLEE-CLOSURE | Measurement, then contracts | High (contracts); Medium (closure re-measurement) | P1 (measurement); P2 and P3 (the gain) | Measure the bottom-up gain; contract the external callees that block the most functions |
| P5 | FIELD-CUSTODY | Candidate only | High | P0; the owned-field Lean model | Pointer-valued stores into fields; not scheduled |
| P6 | ALIAS-RESIDUALS | Candidate only | Medium (planning) | P1's overlap figures | The 1,359 functions blocked by `unknown_alias`; a separate plan |

P1 is the only card that can be dispatched now, once P0 lands. Each later card's
brief must carry P1's function lists and predictions.

**Effort tiers.** Every card carries an effort level:
- **High:** cards whose error could publish a wrong Safe. That covers analyzer or gateway semantics, provider contracts, Lean, soundness repairs and stability reviews.
- **Medium:** measurement, census, native-observation, documentation and records-only cards.
- **Gate helpers** run on a smaller model at medium effort.

Wave 46 dispatched P1 at Medium and P4's external-contract arm (W46-EXTERNAL-CALLEE-CONTRACTS) at High.

### P1: ACCESS-PATH-CENSUS
- **Status:** measured by W46-ACCESS-PATH-CENSUS on base `9bce3986`. Its
  [PREDICTION.md](../../reports/w46-access-path-census/PREDICTION.md) carries the
  function lists P2 and P3 are measured against, and its report recommends one
  typed layer for statements, returns and call-free conditions first, with call
  arguments second.
- **Reuse the census route.** Reuse the existing census machinery (the
  `indirect-census` feature and the census-sink guard) rather than a new scanner.
  Classify each leaf from Clang's typed AST: the root (local, formal, global,
  call result), then each step (`.f`, `->f`, `*`, `[i]`) with its resolved type,
  and the position (statement, condition, return, call argument).
- **Mark each path.** A path is either expressible as a typed access path, or
  refused, with a named cause:
  - union;
  - bitfield;
  - incomplete record;
  - cast in the path;
  - pointer arithmetic;
  - volatile or atomic;
  - a pointer-valued store (which belongs to P5).
- **Predict per function:**
  - for every Unknown function whose blockers would all go, the obligations its
    accesses would carry (non-null and live per hop) and whether the current
    facts discharge them;
  - Safe, Unknown or provisional Unsafe, each with its reason.
  - Freeze the prediction before P2 starts.
- **Closure:** compute the call-graph closure. Which Unknown callers depend only
  on callees that the prediction makes Safe, and how many rounds that takes.
- **Records:** no analyzer change, and the corpus stays byte-identical.

### P2: ACCESS-PATH-STATEMENTS
- **Status:** the bounded fallback is integrated by
  [W46-CONTRACT-ACCESS-EXPANSION](../../reports/w46-contract-access-expansion/README.md).
  Exact source/frontend identity, positive field ownership and value types,
  and existing tracked-root obligations are required. Mixed paths with a
  tracked root remain refused. All legacy routes remain; replacement parity
  is open under `W46-ACCESS-PATH-PARITY`, with tracked mixed paths and later
  expression positions separately registered. The superseded isolated
  checkpoints are not validation of the final scalar-type repair.
- **Lowering.** Build a typed access path from Clang's AST, with typedefs
  resolved and record and field types known. Anything the AST cannot give
  exactly is refused (Unknown), never guessed.
- **Obligations.**
  - Every pointer the path reads through is non-null and live, generalising the
    per-hop obligation of the chain route.
  - A read or store of a non-pointer value leaves the custody frame unchanged.
  - A pointer value read through a path is a borrowed alias with no custody.
  - A pointer-valued store stays refused (P5).
- **Parity (hard invariant 3).**
  - The layer runs beside the shape routes, and every disagreement is recorded.
  - A shape route is retired only after the layer reproduces its pinned verdicts
    and coverage, with the diff archived under `docs/reports/`.
  - Each Safe to non-Safe move is a named correction.
- **Lean.**
  - Per-hop obligations suffice for the access to be defined in the model.
  - A non-pointer store preserves the frame.
  - Refusal is preserved for every unlisted path form.
- **Measure:** corpus arbiter BEFORE and AFTER, and the Fluent Bit slice at
  `046accf6`.

### P3: ACCESS-PATH-POSITIONS
- **Status:** landed as a fallback after every older route by
  [W47-ACCESS-PATH-POSITIONS](../../reports/w47-access-path-positions/README.md)
  (gateway 10k), with the owner-aware enum arm measured as its own step.
  Residuals: `W47-ACCESS-PATH-POSITION-RESIDUALS`,
  `W47-TYPED-ROOT-DECLARATION-READER`; parity stays open under
  `W46-ACCESS-PATH-PARITY`.
- **Positions:** apply P2's layer in controlling expressions (`if`, `while`,
  `for`, `?:`, `&&`/`||` with their evaluation order), in return expressions,
  and in call arguments.
- **Call arguments** keep the callee's contract in force: an access passed at a
  position that takes custody stays refused, as `W44-CALL-ARGUMENT-CUSTODY-POSITION`
  records.
- **Conditions:** a condition's access obligations hold on both branches, and a
  short-circuit operand's obligations only where it is evaluated.

### P4: CALLEE-CLOSURE
- **Status (round 2):** measured and contracted by
  [W47-CALLEE-CLOSURE-R2](../../reports/w47-callee-closure-r2/README.md) on
  the post-P3 corpus. The in-corpus closure frees 0 functions under the
  existing rules and 3 under a cross-unit route. Two external callees were
  declared (+3 Safe). The remaining ranking (`W47-EXTERNAL-RESIDUAL-RANKING-R2`)
  waits on decisions listed in that report's §6.
- **The in-corpus gain:** re-run P1's closure on the post-P3 corpus. That gain
  arrives through the existing summary rules. P4 adds no new trust.
- **External callees:** rank them by the functions they would unblock alone.
  Contract the top ones in their provider crates, where they are reviewed and
  pinned in revision mode, and checked, never trusted.

## 4. Acceptance for every card
Each card must have:
- a frozen `PREDICTION.md`;
- BEFORE reproducing the current baseline;
- AFTER with every moving function attributed;
- corrections named;
- new leaks or violations classified by reading.

Beyond that:
- **Fail-closed.** No refactor turns an Unknown into a Safe without discharged
  obligations.
- **Lean parity.** Registered in an `Audit*.lean`, with no placeholders and no
  project axioms.
- **Provisional Unsafe** from newly modeled accesses is never presented as an
  upstream defect.

## 5. Limits and expected yield
- The upper bounds in §1 count functions whose blockers would all be removed,
  not functions that become Safe. Accesses whose base may be null or released
  will give Unknown or provisional Unsafe, as they should. P1 exists to replace
  the bounds with a prediction.
- **Rough estimate:** P1 is one card. P2 and P3 are about two waves, and P4 runs
  in parallel.
- Leaf counts are not verdicts. The shape cards of waves 42–45 each removed about
  1,000 leaves and moved few functions, because the co-blockers remained. This
  plan orders the work by the functions it frees, not by the leaves it removes.

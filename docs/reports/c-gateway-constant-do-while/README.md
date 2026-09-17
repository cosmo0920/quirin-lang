# C-GATEWAY-CONSTANT-DO-WHILE — the constant `do … while (0)` exit

Wave 35, first arbiter. Base `a300a39d`. Branch
`worktree-agent-a4b9c03fd33dec337`.

`do S while (0)` executes `S` exactly once (C11 §6.8.5.2). The gateway's CFG
builder gave every `do`-`while` header both `body_entry` and `continuation` as
successors whatever its condition, so every such fragment carried a back edge
the C does not have. This card removes that edge for the single literal token
`0`, proves the removal in Lean, and re-pins the Cycle 19 corpus.

Removing an edge is the **admitting** direction, so invariant 1 governs
throughout: every section below states which direction it measures, and the
corpus section names every moved leaf.

## 1. The idiom count, re-derived on the preprocessed text

The card's brief quoted a source-level census — 121 occurrences of `while (0)`
in `lib/librdkafka-2.10.0/src`, 145 in `lib/monkey/mk_core`, 0 in cmetrics,
ctraces and chunkio — and asked for that count to be re-derived on the
**preprocessed** text of the corpus, on the ground that macro expansion is
where these live. It is, and the difference is the point of the card.

Two corrections to the quoted figures, both measured:

* The pinned tree carries **librdkafka 2.15.0**, not 2.10.0
  (`build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source/lib/librdkafka-2.15.0`).
  The count is unaffected: `while (0)` occurs **121** times in that directory's
  `*.c`/`*.h`, exactly the quoted number, so the version string was a slip and
  the measurement was sound. `lib/monkey/mk_core` reproduces at **145**.
* Those two directories are not the corpus. The audited 185 units span six
  libraries, and the source-level figure counts the wrong text anyway.

Preprocessed census — `measure/c-gateway-constant-do-while/ppcensus.py`, which
runs each of the 185 units' own compile-database entry through
`clang -E -P` (the text the gateway analyses, constraint 12a) and counts
`while\s*\(\s*0\s*\)` in the result. **185 units measured, 0 missing from the
compile database, 0 preprocessor failures.**

| Library | Preprocessed `while (0)` |
|---|---:|
| librdkafka | 13,361 |
| cmetrics | 841 |
| ctraces | 435 |
| monkey-libevent | 286 |
| fluent-bit-chunk | 130 |
| chunkio | 0 |
| **total** | **15,053** |

So the idiom is **15,053** occurrences across the pinned corpus, against the
266 a source-level grep of two directories sees — a factor of about 57. The
`chunkio` zero reproduces the brief's cross-check. The heaviest single units
are `rdkafka_mock_handlers.c` (2,073), `rdkafka_admin.c` (1,834) and
`rdkafka_request.c` (1,044); the per-unit table is
`measure/c-gateway-constant-do-while/ppcensus.json`.

**This is a count of the idiom, not a count of affected diagnostics.** The
corpus movement in §5 is far smaller than 15,053, for the reason §5.5 gives.
The census bounds the rule's reach from above; it does not measure it.

## 2. The defect, at its pinned site

The corpus's one `double_release` row, read out of the baseline archive
(`docs/reports/c-gateway-return-call-w4/artifacts/diagnostics.json.gz`):

```
unit      rdkafka_sasl_oauthbearer_oidc.c   (librdkafka)   status Unsafe
function  rd_kafka_jwt_b64_decode_payload                  status Unsafe
category  double_release
internal  L21340
origin    rdkafka_sasl_oauthbearer_oidc.c:192
text      repeated release of acquisition at rdkafka_sasl_oauthbearer_oidc.c:136
```

Both presented coordinates check out against the pinned source. Line 136 is
`converted_src = rd_malloc(len + 4);`, the acquisition. Line 192 is
`RD_IF_FREE(converted_src, rd_free);`, in the function's `done:` epilogue. And
`RD_IF_FREE` is (`lib/librdkafka-2.15.0/src/rd.h:436`):

```c
#define RD_IF_FREE(PTR, FUNC)   \
        do {                    \
                if ((PTR))      \
                        FUNC(PTR);      \
        } while (0)
```

The release is inside a `do … while (0)`. With a back edge from the header to
`body_entry`, the abstract interpretation re-enters the body with
`converted_src` already released and reports a repeated release. Card
`C-GATEWAY-WAVE` said in its own words that this is not a validated upstream
double free and named this card as the follow-on that must model the constant
exit first.

## 3. The rule, and why its key is a token

`crates/qcore-c-gateway/src/legacy_c.rs`:

```rust
fn condition_is_literal_zero(tokens: &[Token]) -> bool {
    matches!(tokens, [token] if token.text == "0")
}
```

carried on `Statement::DoWhile` as `condition_is_literal_zero`, and consumed in
the one place the CFG is built:

```rust
self.nodes[header].successors = if *condition_is_literal_zero {
    vec![continuation]
} else {
    vec![body_entry, continuation]
};
```

Three properties are deliberate.

**The key is the token, never an evaluated expression.** Standing constraint 5a
forbids deciding control flow from a computed value. Nothing in this predicate
folds a constant, reads a binding or consults a scalar fact, so `while ((0))`,
`while (1 - 1)`, `while (0 == 0)` and `while (0 && 0)` all answer `false` and
keep the CFG they had. §4 pins each of them.

**`false` is the conservative answer.** Keeping the back edge over-approximates
the path set, which can only add refusals; answering `true` removes paths. Any
future widening of this predicate is therefore an admitting change needing its
own measured re-pin, not a tidy-up.

**The header node is kept, and its exit edge is never the one removed.** The
node still exists with its line and its instruction, so no leaf's internal
coordinate moves for an unrelated reason, and `continuation` remains a
successor in both arms — this rule removes a path, never a destination
(`the_exit_edge_is_never_removed`).

Two things this card deliberately does **not** do. It does not generalise to
`while (1)`, `for (;;)` or any other constant — those are named follow-ons in
§8. And it leaves the 32 leaves of the *unmodeled* do-while condition refusal
(`unmodeled call, aggregate or indirect access in do-while condition`)
untouched: that is a different question, and §4 pins that it stays refused.

## 4. Tests, and the mutation campaign

`crates/qcore-c-gateway/tests/constant_do_while.rs`, nine tests through the
real gateway (`analyze_unrestricted_c11`), written in both directions.

Withdrawing half:

* `the_release_in_a_constant_do_while_is_not_repeatable` — the `RD_IF_FREE`
  shape itself, no `double_release`.
* `a_constant_do_while_is_its_body` — the `≡ S` claim measured rather than
  argued: for three bodies, wrapping in `do { … } while (0)` changes no
  diagnostic and no status. The comparison is on the sorted multiset of
  diagnostic kinds, so a node-numbering difference cannot make it pass or fail
  for the wrong reason.
* `a_continue_in_a_constant_do_while_exits` — `continue` targets the header,
  which is exactly C's "jump to the controlling expression", and now exits.
* `a_break_in_a_constant_do_while_reaches_the_same_continuation`.

Accusing half — nothing real was swallowed with the spurious path:

* `a_real_double_release_inside_a_constant_do_while_is_still_reported` — two
  releases *in one body* is still a double release.
* `a_leak_through_a_constant_do_while_is_still_reported`.
* `an_unmodeled_do_while_condition_keeps_its_refusal` — the 32 leaves' arm.

Boundary:

* `only_the_literal_zero_token_loses_the_back_edge` — `(0)`, `1 - 1`, `0 == 0`,
  `0 && 0`, `z_unknown(0)` each keep the back edge and each keep reporting the
  repeated release.
* `a_modeled_scalar_condition_is_a_separate_pre_existing_route` — see §6.

### Mutation campaign

`measure/c-gateway-constant-do-while/mutations.py` negates or widens each arm
this card added and requires every mutation to be caught by a named test. **All
six were caught**, and the script restores the source on every exit path.

| Mutation | Caught by |
|---|---|
| key always `true` | `only_the_literal_zero_token_loses_the_back_edge`, `a_modeled_scalar_condition_is_a_separate_pre_existing_route` |
| key always `false` | `the_release_in_a_constant_do_while_is_not_repeatable`, `a_constant_do_while_is_its_body`, `a_continue_…`, `a_break_…` |
| key drops the arity check (`0 == 0` fires) | `only_the_literal_zero_token_loses_the_back_edge` |
| successors drop the **exit** edge instead | `a_leak_…`, `the_release_…`, `a_continue_…`, `a_break_…`, `a_constant_do_while_is_its_body` |
| successors unconditional (every do-while sequential) | `only_the_literal_zero_token_loses_the_back_edge`, `a_modeled_scalar_condition_…` |
| successor **order** swapped on the unchanged arm | `a_modeled_scalar_condition_…` |

The last row was included as a control — a mutation expected to be *harmless*.
It was caught, which is itself a finding: **successor order is observable** in
the worklist schedule, so the unchanged arm's `vec![body_entry, continuation]`
ordering is load-bearing and must not be reordered incidentally
(`CONSTANT-DO-WHILE-SUCCESSOR-ORDER`, §8). This card does not reorder it.

Separately, the whole rule was measured against the **unmodified base**: with
`legacy_c.rs` restored from `HEAD`, `while (0)` reports `Unsafe` with one
`double_release`; with this card's tree it reports `Safe` with none. The change
is the sole cause, measured rather than inferred.

## 5. The corpus re-pin

### 5.1 The BEFORE gate

The BEFORE binary is this card's base (`a300a39d`) built before any edit,
sha256 `e98adac7…`. Its run reproduces the current baseline,
`docs/reports/c-gateway-return-call-w4/artifacts/`, **byte for byte** in both
verdict-bearing artifacts (`logs/byte-identity-before.txt`):

```
*diagnostics      raw IDENTICAL  canonical IDENTICAL   577a0fdaa8045448 / 577a0fdaa8045448
 source           raw IDENTICAL  canonical IDENTICAL   ae38f6c93dfbe341 / ae38f6c93dfbe341
*chunk-contracts  raw IDENTICAL  canonical IDENTICAL   c4f438e4ed3dab3c / c4f438e4ed3dab3c
 source-effects   raw differs    canonical differs
 contracts        raw differs    canonical differs
RESULT: the verdict-bearing artifacts reproduce the baseline BYTE FOR BYTE.
```

`source-effects` and `contracts` carry the checkout root and identity digests
and have differed at every re-pin in this chain for that reason; they differ
**identically** in BEFORE and AFTER and are not attributable to this card. The
chain is attributable.

### 5.2 What moved

AFTER is the new baseline, sha256 `58855bdf…`, audit exit **0**. Four of the
five artifacts are **byte-identical between BEFORE and AFTER** —
`chunk-contracts`, `source-effects`, `monkey-source` and `contracts` all
reproduce at the byte. Only `diagnostics.json` moved, and within it exactly one
leaf.

| Measure | BEFORE | AFTER |
|---|---:|---:|
| `double_release` | 1 | **0** |
| `leak` | 272 | **272** |
| `unknown_alias` | 4,214 | **4,214** |
| `unsupported` | 50,106 | **50,106** |
| units | 2 Safe / 42 Unsafe / 137 Unknown / 4 data-only | **unchanged** |
| functions (of 4,834) | 186 Safe / 85 Unsafe / 4,563 Unknown | **186 / 84 / 4,564** |

**Leak is 272 and did not move** (constraint 7): there is nothing to attribute
on that row. `unknown_alias` and `unsupported` did not move either.

**Zero unit status moves.** **One function status move**, and one leaf removed
with **zero added**:

```
librdkafka/rdkafka_sasl_oauthbearer_oidc.c::rd_kafka_jwt_b64_decode_payload
  status    Unsafe -> Unknown
  removed   internal L21340  double_release:
            repeated release of acquisition at rdkafka_sasl_oauthbearer_oidc.c:136
```

That is the row §2 identified, at the internal preprocessed coordinate
(constraint 12a), and it is the only leaf this card moves anywhere in the
corpus. The transport stays `HistoricalStdin` (constraint 11) and every earlier
archive is untouched (constraint 21).

### 5.3 Withdrawn, not discharged

The card asks which this is, and the answer is **withdrawn**.

`double_release` 1 → 0 moves `rd_kafka_jwt_b64_decode_payload` from provisional
`Unsafe` to **`Unknown`**, not to `Safe`. The function keeps its other **18**
leaves — 12 `unsupported` and 6 `unknown_alias` — so its obligations do not
discharge and invariant 1's fail-closed reading puts it in `Unknown`. Nothing
is promoted: `Safe` is 186 before and after, and the function count moves
`85 → 84` provisional `Unsafe` against `4,563 → 4,564` `Unknown`.

The unit `rdkafka_sasl_oauthbearer_oidc.c` **stays `Unsafe`**, because two other
functions in it (`ut_sasl_oauthbearer_oidc_assertion_create`, 24 leaks, and
`ut_sasl_oauthbearer_oidc_should_succeed`, 4) carry leak rows this card does not
touch. So the corpus's unit histogram is unchanged and the wave-31-through-34
record of "no unit status has moved at any arbiter" continues.

The withdrawal is the right verdict rather than a conservative one: the repeated
release was an artifact of a path the C does not have, proved in §7, so the row
was never evidence about the upstream code. It is withdrawn, and the function's
remaining refusals are why it is `Unknown` rather than `Safe`.

### 5.4 The three-way split over affected functions

| Split | Count | Detail |
|---|---:|---|
| leaf-free (every leaf withdrawn) | 0 | — |
| fewer leaves (withdrawn or exchanged) | **1** | `rd_kafka_jwt_b64_decode_payload`, 19 → 18 |
| same or more leaves | 0 | — |

Exactly one function is affected anywhere in the 185 units, it loses exactly one
leaf, and no leaf anywhere is exchanged for another refusal.

### 5.5 Why 15,053 expansions moved one leaf

The gap between §1's census and this section is the card's most useful measured
statement, and it is not a disappointment — it is the reach of the defect.

A spurious back edge is only observable where the body performs a resource
operation whose repetition the abstract interpretation distinguishes — that much
is a fact about the mechanism. Only a body that **releases or acquires a tracked
identity that survives to the header** can produce a different answer, and in a
corpus where 4,564 of 4,834 functions are `Unknown` that precondition is
evidently rare.

**Why the other 15,052 expansions are inert is an inference, not a
measurement.** The reading offered here — that most wrap logging, assertion,
list-link and field-assignment macros whose bodies the projection either refuses
already (so the statement is `unmodeled` with or without the edge) or models as
idempotent — is consistent with the one-leaf result and with the categories'
total immobility, but this card did not classify the 15,053 expansions and does
not claim to have. `CONSTANT-DO-WHILE-LATENT-REACH` (§8) carries the
measurement that would settle it.

So the honest reading is: the rule is exactly correct and its corpus reach today
is one leaf. Its value is larger than that leaf — it removes a wrong path from
**every** one of the 15,053 expansions, so it forecloses the same false
accusation everywhere it could next have appeared, including in functions that
later cards promote out of `Unknown`. That forward claim is deliberately
*not* quantified here (`CONSTANT-DO-WHILE-LATENT-REACH`, §8).

## 6. A neighbouring route this card does not own

While pinning the boundary family, `while (flag)` with `int flag = 0;` was
found to lose the back edge too — and it does so **on the unmodified base as
well**, measured both ways. That is a pre-existing route: where the analyzer
models a scalar binding's value precisely, it already decides a `do`-`while`
from the modeled value.

It was characterised rather than assumed, and it answers conservatively on
every shape probed:

| Prelude, then `while (flag)` | Back edge |
|---|---|
| `int flag = 0;` | removed |
| `int flag = 1;` | kept |
| `int flag = 0; flag = 1;` | kept (the reassignment is tracked) |
| `int flag = z_unknown(1);` | kept, and the call keeps its refusal |

So it is not a false-`Safe` source, and this card neither extends nor narrows
it. It is pinned by
`a_modeled_scalar_condition_is_a_separate_pre_existing_route` so that a later
refactor cannot move it silently. It is also why this card still had work to
do: a macro's `while (0)` is a literal token and no binding, so that route
never reached the corpus's 15,053 expansions.

## 7. The Lean

`proof/QCore/ConstantDoWhile.lean`, registered in `proof/QCore.lean` and
audited by `proof/AuditConstantDoWhile.lean`. **14 `#print axioms` lines, every
surface inside `AuditAll.lean`'s allowed closure**: five theorems depend on no
axiom at all (`constant_do_while_is_its_body`,
`a_non_literal_condition_is_unchanged`, `every_other_header_keeps_both_successors`
and the two release counts, which are `decide`), five on `propext` alone, and
four on `propext, Quot.sound`. No `sorry`, no project axiom, no placeholder.
`scripts/check_lean_audits.py` passes **130 audits** (129 before this card).

`Condition` distinguishes only `literalZero` from `other`, mirroring the token
predicate; nothing in the module evaluates an expression. Traces are lists of
atoms, `repeatTrace body n` is `n` back-to-back executions, `BackEdgeAdmits` is
the old model (any positive number of executions) and `Admits` the new one.

Exactness — the `≡ S` statement:

* `constant_do_while_is_its_body` : `Admits literalZero body t ↔ t = body`.
* `constant_do_while_is_one_iteration`, the same against `repeatTrace body 1`.

Direction — the half invariant 1 constrains:

* `removing_the_back_edge_only_removes_paths` : every path the new CFG admits
  was already admitted by the old one, **for every condition**. A diagnostic
  derived from the new CFG therefore comes from a path the old CFG also had:
  this change can withdraw a refusal and can never manufacture one.
* `the_successor_list_never_grows`, the same statement at the edge level.
* `a_non_literal_condition_is_unchanged`,
  `every_other_header_keeps_both_successors` — the boundary.
* `the_exit_edge_is_never_removed`.

Non-vacuity — the rule is not decoration:

* `the_back_edge_admits_a_second_execution` and
  `the_constant_exit_refuses_the_second_execution` (whose `body ≠ []` premise
  is necessary, not decorative: an empty body's second execution is
  observationally its first).
* `the_removed_paths_are_real`.
* `the_back_edge_admits_two_releases` / `the_constant_exit_admits_exactly_one_release`
  / `the_repeated_release_is_a_path_the_c_does_not_have` — the parting witness
  named as the corpus's own row: one release under the C, two under the back
  edge.

**Boundary of the Lean.** This is an abstract rule theorem about a model of the
CFG fragment, in the house style. It is not an extraction of the Rust CFG
builder and asserts no simulation between them (constraint 28; `AGENTS.md`'s
standing note that no extraction theorem exists in `proof/`). The binding
between model and implementation is the test file in §4 and the corpus
measurement in §5.

## 8. Named follow-ons

Each with a discharge condition.

* **`CONSTANT-DO-WHILE-ZERO-SPELLINGS`** — the rule claims the single token
  `0` only. `0u`, `0L`, `0U`, `0x0`, `00` and `'\0'` are literal zeros that a
  `do … while (…)` may equally spell, and each keeps its back edge today.
  Nothing in the pinned corpus's preprocessed text was found to use one in this
  position, but that was not exhaustively measured.
  **Discharge condition:** a preprocessed census of `do … while (E)` conditions
  over the 185 units that classifies every `E` a single-token integer constant
  expression, followed either by a widening of `condition_is_literal_zero` with
  its own measured re-pin, or by a recorded statement that the set is empty.
* **`CONSTANT-DO-WHILE-OTHER-CONSTANTS`** — `while (1)`, `for (;;)` and
  `while (0)`'s non-terminating siblings are deliberately untouched. Note the
  asymmetry of direction: `do S while (1)` never exits, so modelling *it*
  exactly would **remove the exit edge**, which is the accusing direction and a
  different risk profile from this card's.
  **Discharge condition:** a separate card, because the exit-removing direction
  needs its own Lean statement (an infinite trace set, not a one-element one)
  and its own measured re-pin; it must not be folded into this rule.
* **`CONSTANT-DO-WHILE-LATENT-REACH`** — §5.5 claims this rule forecloses the
  same false accusation across all 15,053 expansions, and that forward claim is
  **not quantified**. Today's measured reach is one leaf.
  **Discharge condition:** after a later card promotes a body out of `Unknown`,
  re-measure whether any newly modelled release sits inside a `do … while (0)`
  and record whether the count changed; equivalently, an instrumented count of
  how many of the 15,053 expansions contain a tracked-identity release at all.
* **`CONSTANT-DO-WHILE-SUCCESSOR-ORDER`** — the mutation campaign's control
  (swapping `vec![body_entry, continuation]` to `vec![continuation, body_entry]`
  on the **unchanged** arm) was expected to be harmless and was **caught**, so
  successor order is observable in the worklist schedule. This card does not
  reorder it, and the fact is recorded rather than repaired — it is the same
  scheduling sensitivity constraint 6 records for `report_leaks`
  (`C02-P-ALIAS-LEAK-ORDER`).
  **Discharge condition:** either a statement of which analyses depend on
  successor order and why that is intended, or a scheduling-neutral pre-pass of
  the kind constraint 6 already describes, with `AbstractState` byte-identical.
* **`CONSTANT-DO-WHILE-JOINED-PROVENANCE-REACH`** — §10 measures **654**
  functions moving cyclic → acyclic, so `legacy_joined_provenance::derive` now
  derives facts for 654 functions that previously got none, with a measured
  diagnostic effect of **zero** on this corpus. Zero today is not zero after the
  next change to that pre-pass, and wave 35's second arbiter
  (`JOINED-PROVENANCE-CYCLES`) is changing exactly it.
  **Discharge condition:** that card quotes its pre-pass coverage against this
  card's tree rather than against `a300a39d`, and states whether any of the 654
  newly-acyclic functions changes an answer under its own change — which is a
  re-run of `acyclicity.py` plus its own attribution, not a new measurement.
* **`CONSTANT-DO-WHILE-SCALAR-ROUTE-UNOWNED`** — §6's pre-existing
  modelled-scalar route decides a `do`-`while` from a **value**, where this card
  decides from a token. Both answers agree wherever they overlap and the scalar
  route answered conservatively on all four probed shapes, but it has no card,
  no Lean and no report of its own; it is pinned here only by one test this card
  happened to write.
  **Discharge condition:** a card that states the route's premise, measures its
  corpus reach, and either gives it a theorem or records it as a measured
  boundary — or a finding that the two routes can disagree, which would be the
  more interesting outcome.

## 9. Standing gate

Run at this card's tip in `AGENTS.md`'s order, every exit captured rather than
inferred (`measure/c-gateway-constant-do-while/gate.sh`; logs under `gate/`,
exits in `gate/exits.txt`).

| Step | Exit |
|---|---:|
| submodule update (`--init --recursive`) | **0** |
| `cargo test --workspace --no-fail-fast` | **0** |
| `cargo fmt --all -- --check` | **0** |
| `cargo clippy --all-targets -- -D warnings` | **0** |
| `cargo run -p qcore-compiler --example reevaluation_probe` | **0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | **0** |
| `python3 scripts/check_lean_audits.py` | **0** |

**4,568 passed / 0 failed / 8 ignored**, across **229** result summaries, and
**130 Lean audits**.

The delta against the `DOCS-COMPACT-6` tip (4,559 / 0 / 8, 228 summaries, 129
Lean) is **exactly this card's own additions and nothing else**: +9 tests in
+1 new test binary (`crates/qcore-c-gateway/tests/constant_do_while.rs`, all
nine listed in §4 and all nine green in this run), and +1 Lean audit
(`AuditConstantDoWhile.lean`). 4,559 + 9 = 4,568; 228 + 1 = 229; 129 + 1 = 130.
No pre-existing test changed its result.

**Which of constraint 16's load-sensitive rows this run saw**, since `AGENTS.md`
requires a gate report to say rather than to treat either outcome as settling
anything — all **green** here:

* `closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request`
  — **green in this run**. `STABILITY-REEVAL` #1 measured it **red 3/3 in
  isolation** in wave 34, and the ledger records that it is not reliably
  reproducible in either direction and is *not* one of constraint 16's five
  rows (`VERIFIER-HOST-LARGE-RESPONSE-CONTROL-RED`). This run settles nothing
  about it; it is reported as seen.
* `vp04_vp07_vp08_process_lifecycle_and_payload_budgets` — green.
* `enforced_process_limits::memory_growth_beyond_the_declared_limit_is_bounded`
  — green.

`--no-fail-fast` was used, so the number is a measurement and not a lower bound
(`GATE-NO-FAIL-FAST`). Provider tests were **not** re-measured and **122 is not
re-claimed here**: no provider source, manifest template or `qcore-schemas`
major version moved, so constraint 18 is not engaged.

One integrity check beyond the gate: rebuilding the release example from the
shipped source reproduces the archived AFTER binary's sha256 `58855bdf…`
**bit for bit**, so §5's artifacts provably come from the code in this diff and
the §10 measurement probe left no residue.

## 10. Interaction with `JOINED-PROVENANCE-CYCLES` (wave 35's second arbiter)

**This is the largest thing this card does, and it is not in the diagnostics.**

`legacy_joined_provenance::derive` topologically sorts the CFG and, when the
sort fails — when the CFG has a **cycle** — returns
`vec![Bindings::new(); cfg.nodes.len()]`: no conditional provenance facts at
all, for that whole function. Constraint 10b records that as "cyclic CFGs
derive no facts". Removing a back edge can therefore flip a function from *no
facts* to *facts derived*, which is a real interaction with wave 35's second
arbiter and is **measured here rather than argued**.

`measure/c-gateway-constant-do-while/acyclicity.py` builds two instrumented
binaries — one from `HEAD`, one from this card's tree — and runs each over the
pinned corpus. The probe runs the *same* Kahn sort `derive` runs, over the same
CFG, and is inserted into `legacy_c.rs`, **not** into
`legacy_joined_provenance.rs`, which this card stays out of. Both
instrumentations are reverted on every exit path (the script prints
`source restored: True`).

| | Functions |
|---|---:|
| analysed names seen in both runs | 4,771 |
| acyclic under both | 2,844 |
| cyclic under both | 1,272 |
| **cyclic → acyclic (this card's effect)** | **654** |
| acyclic → cyclic | **0** |
| mixed within one run | 1 |

**654 functions** — **34%** of the 1,926 whose CFGs were cyclic before this
card — now derive conditional provenance where they derived none. Nothing moves
the other way, which is the expected direction: removing edges can only destroy
cycles.

**And the measured diagnostic effect of that is zero.** §5.2's re-pin is the
same tree: `unknown_alias` is **4,214 before and after**, no leaf is added
anywhere, and the only leaf that moves is the `double_release` §2 identified.
So the pre-pass now runs on 654 more functions and changes no published answer
on any of them, in this corpus, today.

Both halves matter to the second arbiter, and neither is a conclusion about its
card:

* Its pre-pass's **reach grew by 654 functions** before it changed anything. Any
  corpus figure it quotes for "functions the pre-pass covers" must be taken
  against this card's tree, not against `a300a39d`.
* Those 654 are, by construction, functions whose **only** cycle was a constant
  `do … while (0)` — which is to say the macro idiom, not a real loop. If that
  card's work is about deriving facts across cycles, this card has already
  removed the population of *spurious* cycles from its problem, and what remains
  under `cyclic in both` (1,272) is closer to the genuine loops.

Two boundaries on this measurement, stated rather than smoothed. The probe keys
on the **bare function name**, so a name defined in two units appears once; one
name came back analysed both ways within a single run and is reported as
`mixed` rather than collapsed, so it is excluded from the flip count. And
"derives facts" is not "derives *useful* facts" — the measurement counts
functions the sort now completes for, not conclusions drawn.

## 11. Reproduction

```sh
git submodule update --init --recursive
cargo build --release -p qcore-compiler --example cycle19_fluent_bit_libraries_audit
cp target/release/examples/cycle19_fluent_bit_libraries_audit \
   measure/c-gateway-constant-do-while/bin/audit-after
python3 -B measure/c-gateway-constant-do-while/run_audit.py after \
   measure/c-gateway-constant-do-while/bin/audit-after
python3 -B measure/c-gateway-constant-do-while/byte_identity.py \
   measure/c-gateway-constant-do-while/runs/before \
   docs/reports/c-gateway-return-call-w4/artifacts
python3 -B measure/c-gateway-constant-do-while/ppcensus.py
python3 -B measure/c-gateway-constant-do-while/attribute.py
python3 -B measure/c-gateway-constant-do-while/mutations.py
python3 -B measure/c-gateway-constant-do-while/acyclicity.py
```

The BEFORE binary is the same build at base `a300a39d`, sha256 `e98adac7…`;
AFTER is `58855bdf…`. A corpus run takes four to seven minutes depending on
load (wave 35's second arbiter was building concurrently for the AFTER run —
audit output is deterministic, only wall time moved: BEFORE 278.7s, AFTER
399.7s). Scripts are archived beside this report under `scripts/`; they read
the pinned tree at
`/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin` and
write only to the worktree-private `measure/` directory. `mutations.py` and
`acyclicity.py` both patch `legacy_c.rs` temporarily and restore it on every
exit path; each prints whether the restore succeeded.

## 12. For the integrator

### 12.1 Applied by this card

* **`AGENTS.md` corpus baseline pointer and chain.** The headline sentence now
  reads `Headline numbers after C-GATEWAY-CONSTANT-DO-WHILE (2026-09-17)` with
  the baseline `docs/reports/c-gateway-constant-do-while/artifacts/`; the chain
  gains `c-gateway-constant-do-while` (wave 35) at its head and constraint 21's
  frozen list reads "those **six**" rather than "those five".
* **`AGENTS.md` corpus row.** `186 Safe / 84 provisional Unsafe / 4,564
  Unknown`, `double_release` **0**, "no unit status has moved at any arbiter
  from wave 31 through wave **35**", and this card's delta narrated in the
  existing chain sentence. Leak **272**, `unknown_alias` **4,214** and
  `unsupported` **50,106** are left as they were, because they did not move.
* **`docs/reviews/r0_3/acceptance_matrix.json`.** Regenerated with
  `python3 docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/wave33-close/gate.json`
  (the repo root is a positional argument; wave 34 has no close directory, so
  wave 33's gate is the one that preserves provenance, exactly as
  `C-GATEWAY-RETURN-CALL-W4` did). **Only the two derived Lean fields moved**:
  `validation.lean_audits` 129 → 130 and `AuditConstantDoWhile.lean` into
  `lean_audit_files`. Diff is 2 insertions, 1 deletion; nothing else in the file
  changed.
* **`docs/development/increments/wave-35.md`.** One paragraph, prepended.

### 12.2 Not applied — needs your decision

* **`AGENTS.md`'s merged-gate row still reads `129 Lean audits`; this card makes
  it 130.** I did **not** edit that row. It is a wave-close measurement
  explicitly attributed to the `DOCS-COMPACT-6` tip and carrying that pass's own
  qualifications about `closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request`
  and the carried provider 122; rewriting one figure inside it would silently
  re-attribute the whole row to a tip at which it was not measured. **Proposed
  amendment, to land in these words** — replace `**129 Lean audits**` with:

  > **130 Lean audits** (129 at the `DOCS-COMPACT-6` tip; `C-GATEWAY-CONSTANT-DO-WHILE`, wave 35, adds `AuditConstantDoWhile.lean`)

  and leave every other figure and qualification in that row untouched. If you
  would rather the row stay a single-tip measurement, **decline this and record
  it declined** — the matrix's derived count is already correct either way, and
  this card's own gate figures are in §9.

### 12.3 Proposed standing constraint — exact words to land

This card lands a "when touching X, do not Y" rule, so it belongs in the
standing-constraint inventory. `5a` and `5b` are taken; **`5c`** is free and is
the right family, because `5a` is precisely the rule this card's key obeys.
Insert after `5b`, under **W4 custody inference and the C gateway**:

> 5c. **A constant `do … while (0)` is sequential composition, and the key is
>     the token.** `do S while (0)` executes `S` exactly once (C11 §6.8.5.2),
>     so its CFG header has `continuation` as its **only** successor and the
>     back edge to `body_entry` is a path the C does not have. Removing that
>     edge is the **admitting** direction (invariant 1), which is why three
>     things must survive refactors. The rule is keyed on the **single literal
>     token `0`** and never on an evaluated expression — constraint 5a's rule,
>     applied here — so `while ((0))`, `while (1 - 1)`, `while (0 == 0)` and
>     `while (0 && 0)` keep the back edge, and answering "not literal zero" is
>     the conservative answer, so any widening of `condition_is_literal_zero`
>     (the unclaimed spellings `0u`, `0L`, `0x0`, `'\0'` included) is an
>     admitting change needing its own measured re-pin. The **exit** edge is
>     never the one removed and the header node is retained, so the rule removes
>     a path and never a destination, and no leaf's internal coordinate moves
>     for an unrelated reason. And the rule does **not** generalise to
>     `while (1)` or `for (;;)`: modelling those exactly would remove an *exit*
>     edge, which is the accusing direction and needs its own Lean statement and
>     re-pin (`CONSTANT-DO-WHILE-OTHER-CONSTANTS`). The 32 `unmodeled call,
>     aggregate or indirect access in do-while condition` leaves are a different
>     question and stay refused — measured unmoved at 32 across the re-pin.
>     Proved as `QCore.ConstantDoWhile`: `constant_do_while_is_its_body` the
>     `≡ S` equality, `removing_the_back_edge_only_removes_paths` and
>     `the_successor_list_never_grows` the conservativity for **every**
>     condition, `the_repeated_release_is_a_path_the_c_does_not_have` the
>     non-vacuity. Separately, and **not owned by that card**, a pre-existing
>     route already decides a `do`-`while` from a *modelled scalar value*; it
>     answers conservatively wherever measured and is pinned by
>     `a_modeled_scalar_condition_is_a_separate_pre_existing_route`
>     (`CONSTANT-DO-WHILE-SCALAR-ROUTE-UNOWNED`). Finally, removing a back edge
>     **makes a function acyclic**, and `legacy_joined_provenance::derive`
>     returns no conditional-provenance facts at all for a cyclic CFG
>     (constraint 10b) — so this rule silently changes *that* pre-pass's reach:
>     **654** of 4,771 analysed functions at this card's re-pin, with a measured
>     diagnostic effect of zero. Any widening of the key therefore moves two
>     subsystems and must re-measure both
>     (`measure/.../acyclicity.py`, `CONSTANT-DO-WHILE-JOINED-PROVENANCE-REACH`).
>     `c-gateway-constant-do-while`.

### 12.4 Constraint 14 and the Q namespace

This card **reserves no word, moves no lexer rule and allocates no `Q` code**,
and it is not even a condition extension: it changes the CFG, not a diagnostic's
condition. For constraint 14's "cards that do not engage this constraint at
all" list, **to land in these words**: add `C-GATEWAY-CONSTANT-DO-WHILE` to
wave 35's entry when that entry is written.

### 12.5 Residuals for the tracker

**Six** named follow-ons, each with a discharge condition in §8, none of which
reaches a tracker yet — `reconciliation_sweep.py --residuals` should find all
six: `CONSTANT-DO-WHILE-ZERO-SPELLINGS`, `CONSTANT-DO-WHILE-OTHER-CONSTANTS`,
`CONSTANT-DO-WHILE-LATENT-REACH`, `CONSTANT-DO-WHILE-SUCCESSOR-ORDER`,
`CONSTANT-DO-WHILE-SCALAR-ROUTE-UNOWNED`,
`CONSTANT-DO-WHILE-JOINED-PROVENANCE-REACH`.

### 12.6 Hand-off to wave 35's second arbiter

`JOINED-PROVENANCE-CYCLES` and this card touch the same pre-pass from opposite
sides and **do not conflict in the tree** — this card edits `legacy_c.rs` only
and never `legacy_joined_provenance.rs`. They do interact in the *measurements*,
and §10 quantifies it: this card makes **654** functions acyclic, so that card's
pre-pass runs on 654 more functions than it did at `a300a39d`, with a measured
diagnostic effect of zero on this corpus.

**When merging, order matters for attribution, not for correctness.** If this
card merges first (as the wave's first arbiter), that card's BEFORE run must
reproduce **this** card's archives — `docs/reports/c-gateway-constant-do-while/artifacts/`
— which is the chain rule working normally, and its coverage figures should be
quoted against this tree. If the order is reversed, this card's 654 is still
correct as a statement about `a300a39d`, but that card will need to re-measure
it. `measure/c-gateway-constant-do-while/acyclicity.py` is archived under this
report's `scripts/` and re-runs unmodified.

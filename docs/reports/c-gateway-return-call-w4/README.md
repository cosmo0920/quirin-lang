# C-GATEWAY-RETURN-CALL-W4 — the modeled call in `return` position

Wave 34's second card, and an **arbiter**: it owns the corpus re-pin.

Base `c4b24a9b` (main's tip at the card's start). Pinned corpus baseline
`docs/reports/c-gateway-return-position/artifacts/`.

## 1. Headline

Card `C-GATEWAY-RETURN-POSITION` (wave 33) withdrew `return CALL(args)` on two
grounds — a **yield** ground and a **soundness** ground. This card answers the
soundness ground with a rule plus Lean, lands it, and **re-derives the yield
ground's measurement, which comes out against the reason this card was
scheduled.**

* **The rule landed.** `return f(args);` is admitted when the call is modeled
  and its result mode is `None` or `Owned`; a `Reallocated` result is refused
  **by name**. The rule is a *desugaring* — the lowering is literally
  `PLACE = f(args); return PLACE;`, the pair the analyzer has decided since
  Cycle 9 — so `observe_exit` needed no arm for a call result and no new
  `LegacyInstruction` was added.
* **The corpus moved**: **7 functions `Unknown` → `Safe`**, none withdrawn from
  `Safe`, none made provisional `Unsafe`. **Zero unit status moves.** The leak
  pin is **unmoved at 272**, as are `double_release` 1. `unsupported`
  **50,117 → 50,106** (−11); `unknown_alias` **4,208 → 4,214** (+6).
* **The reach argument that scheduled this card does not survive
  re-derivation.** The brief's correction to wave 33's yield ground was that
  **236 of 335 (70.5%)** `return CALL(...)` callees are "W4-derivable" against
  the 12 that carry a declared contract. Re-derived here (§3): that figure
  measures **definedness**, not derivability. Constraint 4's second route needs
  a summary `observations_match_contract` **validated**, and validation requires
  the callee's own body to be fully modeled and defect-free. Over the 643
  `return CALL(...)` leaves in the current baseline, the callee's own body is
  `Safe` — the precondition for a validated summary — in **4 cases (0.6%)**.
  Of the 14 leaves this card actually withdrew, **13 were decided by a declared
  `.qcore` contract and exactly 1 by the W4 route.**

So wave 33's ground 1 was substantially right, and is recorded here as such.
The card still earns its place: ground 2 was a real, unanswered soundness gap,
it is now answered by a rule with Lean and a 7/7 mutation-checked test suite,
and the 7 promotions are real. But **this card is not the 236-leaf card its
brief anticipated, and no such card is available on the W4 route today** —
`RETURN-CALL-W4-REACH-BLOCKED` (§10) records what would have to change first.

## 2. What was withdrawn, in wave 33's own words

`docs/reports/c-gateway-return-position/README.md` §3:

1. **Yield.** "the decisive number is **12 of 335**. For the remaining 323,
   clearing the return refusal converts one refusal into another — the function
   gains a `call to uncontracted function` leaf and is **no better decided**."
2. **Soundness.** "Applying a callee's parameter custody at a return position
   means a `Consumed`/`Released` parameter mode would **discharge** a live
   obligation of the caller at that exit. That is invariant 1's forbidden
   direction, and it cannot be taken on a summary route whose exit observer
   (`observe_exit`) has no arm for a call result at all."

Ground 2 is a soundness argument. A larger reachable set is a yield argument and
**cannot answer it**; this card does not treat it as if it could.

## 3. The reach, re-derived

### 3.1 Why the inherited measurement was not inherited

The brief named wave 33's `logs/subshape-b-w4-reach.txt` and instructed that it
be re-derived rather than inherited, because that branch's census tool read the
**original** source line while refusals are recorded at the **preprocessed**
line. That defect is structurally absent here: card `GATEWAY-C-LOCATION`
(wave 27) made every diagnostic carry `origin`, the derived original
`file:line`, so `measure/.../reach.py` reads the line the diagnostic itself
names.

The classifier is nonetheless **checked rather than trusted**: every leaf the
AFTER run actually withdrew must classify as `call`, and the script exits
nonzero otherwise. It caught one bug of its own on the first run — a one-line
function body (`static void* XXH_malloc(size_t s) { return malloc(s); }`) whose
`return` does not begin the line — which was fixed before any number below was
read. The check now reports **14/14**.

### 3.2 The shape census, over the current baseline

1,470 return-expression refusal leaves remain after wave 33, across 1,134
functions; **478** functions are blocked by them and nothing else. (478 is
exactly wave 33's 511 minus the 33 it moved — an independent consistency check
on both measurements.)

| shape of the returned expression | leaves | share |
|---|---:|---:|
| `return CALL(...)` | **643** | 43.7% |
| the `return` is not complete on the line `origin` names | 455 | 31.0% |
| field chain or mixed | 226 | 15.4% |
| cast or parenthesized | 78 | 5.3% |
| conditional `?:` | 24 | 1.6% |
| subscript | 23 | 1.6% |
| other | 16 | 1.1% |
| dereference | 3 | 0.2% |
| plain name or literal | 2 | 0.1% |

643 `return CALL(...)` leaves over **372 distinct callees**.

### 3.3 The correction to the correction

Cross-cutting those 643 by how the callee could be modeled at all — `status` is
the callee's own projected status in the same unit, which is what W4 validation
turns on:

| the callee at this leaf | leaves | share |
|---|---:|---:|
| defined in this unit, own body **`Unknown`** — no validated summary | 366 | 56.9% |
| **not** defined in this unit (external) — stays `Unknown` | 264 | 41.1% |
| defined in this unit, own body `Unsafe` — no validated summary | 9 | 1.4% |
| defined in this unit **and own body `Safe`** — W4 can validate | **4** | **0.6%** |

On one population, so the shares are comparable: **379 of 643 (58.9%) have a
callee defined in the same `.c`** — the property the brief counted as 236 of 335
(70.5%) over wave 33's pre-landing population — while **4 (0.6%)** have one
whose body actually validates. The two populations differ (wave 33 measured
before its own 33 leaves moved; this measures what remains), but the conclusion
does not depend on which is used: the gap between the two columns is two orders
of magnitude either way.

That gap is the finding. "Defined in the same `.c`" is
a correct count of *that* property. It is not derivability: constraint 4's
second route publishes a summary only after `observations_match_contract`
re-runs the analyzer and finds the body **fully modeled and defect-free**
(`legacy_summary.rs` discipline, conditions 1–6). A body that is itself
`Unknown` — 56.9% of these leaves — yields no summary, so its callers gain
nothing.

The exemplars the brief named are the clearest illustration, and each was
checked rather than repeated (`scripts/exemplars.py`). The four
`destroy_data_point → destroy_*_data_point` fan-outs are four separate leaves
and their callees' own statuses are `Unknown`, **`Safe`**, `Unknown`, `Unknown`
— so exactly **one** is admitted here, which is this card's entire W4-route
yield. `cJSON_Parse → cJSON_ParseWithOpts`, `cJSON_Duplicate →
cJSON_Duplicate_rec` and `cmt_cat → append_context` are in the 366: the shape is
`call`, and the callee's own body is `Unknown`.
`ctr_span_set_trace_id_with_cid → ctr_span_set_trace_id` is **not** in the 366,
though its callee is also `Unknown` — its leaf falls in the incomplete-line
bucket of §3.2 (`RETURN-CALL-INCOMPLETE-LINE-455`), so this census makes no shape
claim about it. The distinction is recorded rather than smoothed over.

### 3.4 What actually decided the 14

| how the callee is modeled | leaves |
|---|---:|
| a declared `.qcore` contract (`malloc`, `rd_malloc`, `rd_calloc`, `rd_strdup`, `ALLOC`, `ALLOC_AND_ZERO`, `cio_chunk_*`) | **13** |
| a W4-derived validated summary (`destroy_data_point → destroy_summary_data_point`) | **1** |

So the mechanism that produced this card's yield is overwhelmingly the one
wave 33 counted, at roughly the magnitude wave 33 counted it.

## 4. The rule, and how it answers ground 2

### 4.1 The lowering is a desugaring

`return f(a);` and `{ T = f(a); return T; }` are the same C for a fresh `T`. The
gateway already models the second form — the call through
`Parser::classify_simple`, the return through `LegacyInstruction::EscapeIfOwned`.
`CfgBuilder` therefore lowers an admitted return-position call into exactly that
pair: a `ContractCall` node binding the result to one reserved synthetic place,
whose successor is an ordinary `EscapeIfOwned` exit on that place.

That answers ground 2's **mechanical** half without touching the observer:
`observe_exit` needs no arm for a call result, because after the desugaring
there is no call at the exit. No new `LegacyInstruction`, no new
`ReturnedValue` constructor, no change to the summary validator or the
certificate recheck.

### 4.2 The discharging direction, per result mode

`ResourceReturnMode` has exactly three constructors, so the soundness question
is finite.

* **`None`** — the destination is removed from the pointer environment, so the
  exit escapes nothing. Any resource the body still holds is still held at the
  exit and `report_leaks` reports it. This is the **accusing** direction: the
  `Unknown` being withdrawn marked the path unmodeled, and `report_leaks`
  *skips* an unmodeled exit.
* **`Owned`** — `apply_contract_call` calls `create_resource` on a site drawn
  from `Parser::next_site`, a monotonically increasing counter. The site is
  therefore **fresh by construction, not by analysis**: it is not a site of the
  pre-state. Escaping it discharges nothing the body owed before the call —
  which is precisely what ground 2 demands, about the one mode that can produce
  custody.
* **`Reallocated`** — **refused by name.** Here the result denotes the identity
  the caller passed at the named parameter: `apply_contract_call` records
  `conditional_replacements[fresh] = old`, so escaping it would carry the *old*
  site's obligation out of the exit. This is exactly "a `Consumed`/`Released`
  mode discharging a live obligation of the caller at that exit", and it is the
  case ground 2 was right about.

The **parameter** modes are applied by `apply_contract_call` unchanged. That is
not new trust: a statement-position call has applied them at every other
position since Cycle 9, and constraint 4's contract-or-validated-summary
discipline is the mechanism that licenses them. The return position licenses
nothing the call does not already license — which is what §4.1's desugaring
says formally.

### 4.3 Two premises that are checked, not assumed

* **The synthetic place must not collide.** `__qcore_returned_value` shares a
  namespace with the body's C names, so the admission requires it to be spelled
  nowhere in the analysed stream and **refuses** otherwise. C11 §7.1.3 reserves
  the spelling; the guard does not rely on that.
* **A declined probe leaves no trace.** The rule asks `classify_simple` to
  classify the synthetic assignment and then *inspects* the answer, so one
  existing lookup decides which contract is in force and constraint 8's
  withdrawal rewriting, the literal premises, the nested-call admission and the
  variadic tail all apply exactly as they do to a real assignment.
  `classify_simple`'s only mutation of the parser is `next_site`, so a declined
  probe restores it — otherwise every refused `return` would silently renumber
  the sites of the acquisitions after it, and site numbering is certificate
  identity. Driven by a white-box row, and caught by mutation (§8).
* **The call must be the whole expression.** Measured load-bearing, not
  stipulated: `classify_simple`'s own extent guard is conditioned on the
  statement naming a **protected** name, so for a call whose arguments name no
  tracked identity it does not fire. With this premise mutated away,
  `void *subject(void) { void *other = 0; return q_alloc(), other; }` — which
  allocates, returns something else and leaks — was measured **`Safe`**. That is
  a false `Safe`, so the premise is soundness evidence and not a shape
  preference.

## 5. The corpus re-pin, attributed

**BEFORE reproduces the baseline byte for byte** in both verdict-bearing
artifacts (`diagnostics`, `chunk-contracts`); `source` is byte-identical too.
`source-effects` and `contracts` differ, **identically in the BEFORE and AFTER
runs** (`d593131e8d7eaec8` / `919dd6dc6fa0b216` in both), so that difference is
the checkout root and identity digests this chain has carried at every re-pin
and is not attributable to this card. Log: `logs/byte-identity-before.txt`.

### 5.1 Headline counts

| count | before | after | delta |
|---|---:|---:|---:|
| `leak` | 272 | **272** | **0** |
| `double_release` | 1 | **1** | **0** |
| `unknown_alias` | 4,208 | **4,214** | **+6** |
| `unsupported` | 50,117 | **50,106** | **−11** |

Unit statuses: **2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only**,
**unmoved**. Function statuses: **179 → 186 Safe**, 85 provisional Unsafe
unmoved, 4,570 → 4,563 Unknown.

### 5.2 Leaves

**17 withdrawn, 12 added.**

Withdrawn:

| description | count |
|---|---:|
| `unmodeled call, aggregate or indirect access in return expression` | 14 |
| `call to uncontracted function \`flb_input_chunk_is_up\`` | 2 |
| `call to uncontracted function \`rd_list_alloc\`` | 1 |

The three `call to uncontracted function` withdrawals are **second-order and in
the discharging-by-deciding direction**: `flb_input_chunk_is_up` and
`rd_list_alloc` each became `Safe` here, so W4 derived a validated summary for
them, so their *callers'* uncontracted-call leaves were decided.

Added:

| description | count |
|---|---:|
| `nullable resource \`__qcore_returned_value\` used by a non-null contract` | 5 |
| `unresolved resource identity \`__qcore_returned_value\`` | 5 |
| `nullable resource \`rl\` used by a non-null contract` | 1 |
| `unresolved resource identity \`rl\`` | 1 |

All 12 are `unknown_alias`-class — **`Unknown`, never `Safe`**, so every one is
in the accusing direction. They say a true thing: the value this `return` hands
back may be null, and nothing in force says the function's own result may be.
The two naming `rl` are in `rd_list_new` and are second-order — `rd_list_alloc`
became `Safe`, so its caller's call is now modeled, which exposed that `rl` may
be null.

### 5.3 The three-way split, over the 478 functions blocked only by a return leaf

| outcome | count | share |
|---|---:|---:|
| **(1) end with zero leaves** | **7** | 1.5% |
| **(2) exchange the return leaf for a different refusal** | **0** | 0.0% |
| **(2b) lose some but not all of their return leaves** | **1** | 0.2% |
| **(3) unmoved — the return leaves remain** | **470** | 98.3% |

All 7 of bucket (1) reach `Safe`, and each certificate rechecks.

**Bucket (2) is empty, and by construction rather than by luck**, exactly as
sub-shape A's was: a callee with no contract in force is not admitted at all, so
clearing a return refusal never merely substitutes a `call to uncontracted
function` leaf — the outcome wave 33 named as unacceptable to publish as
progress. Pinned by `an_uncontracted_callee_keeps_the_return_refusal`.

Bucket **(2b)** has one member and is not an exchange: `destroy_data_point`
carries four `return CALL(...)` leaves — the four `destroy_*_data_point`
fan-outs — and exactly one of its callees has a body that validates, so it goes
4 → 3 and stays `Unknown`. That is this card's **entire** W4-route yield (§3.4),
and it is visible here as one function that moved and did not arrive.

### 5.3.1 The six functions outside the 478

Six functions gained a new leaf. **None is in the 478** — every one already
carried refusals other than the return leaf, so none was a candidate for
promotion under this card and none is in the split above.

`LZ4F_calloc`, `LZ4F_malloc`, `rd_dl_error`,
`rd_kafka_AuthorizedOperations_parse` and `ut_create_mock_jwt_template_file`
each had a return leaf withdrawn and gained the pair
`unresolved resource identity` / `nullable resource … used by a non-null
contract` naming `__qcore_returned_value`. `rd_list_new` gained the same pair
naming its real local `rl`, second-order: `rd_list_alloc` became `Safe`, so its
caller's call is now modeled.

All twelve added leaves are `unknown_alias`-class — **`Unknown`, never `Safe`** —
so every one is in the accusing direction, and each says a true thing: the value
this `return` hands back may be null, and nothing in force says the function's
own result may be. The residual `RETURN-CALL-NULLABLE-WRAPPER` (§10) records the
open question of whether the derivation should infer a nullable result for such
a wrapper instead.

### 5.4 The seven promotions

`__cmt_allocator_alloc` [cmt_decode_prometheus_remote_write.c],
`flb_input_chunk_get_size` and `flb_input_chunk_is_up` [flb_input_chunk.c],
`rd_kafka_mem_calloc` and `rd_kafka_mem_malloc` [rdkafka.c],
`rd_list_alloc` [rdlist.c], `XXH_malloc` [rdxxhash.c].

Every one is a thin allocator wrapper of the shape the card targets.
`flb_input_chunk_create` remains function-level `Safe`; the production
`input_chunk_get` residual stays `Unknown`.

## 6. Deliberate pin update

One existing pinned expectation moved, and it is reported rather than absorbed
(`CLAUDE.md` working rule 1; invariant 5).

`crates/qcore-c-gateway/tests/neutral_expression_positions.rs` ::
`pointer_arithmetic_in_a_call_actual_stays_unknown` asserted `Unknown` for
`return digits10((unsigned long) (p + n));`. It now asserts `Safe` and is renamed
`pointer_arithmetic_in_a_call_actual_agrees_with_the_statement_position`.

The ground is **measured, not argued**: the statement form of the identical C —
`unsigned int t = digits10((unsigned long)(p + n)); return t;` — was **already
`Safe` before this card**. So the `Unknown` that row pinned was the *return
position's* extra strictness and not a custody judgment, and the desugaring
aligns the two positions. The control is now written **into the test itself**, so
the alignment rests on a checked fact rather than on this paragraph.

What card W4-EXPR-POS measured there is unchanged and still true: the C02 operand
walker does refuse `p`, and the *neutral-expression* route still refuses that
actual. It is simply no longer the only way a `return` can be modeled. The
neighbouring dereference and subscript rows are the evidence that nothing broader
moved — both keep `Unknown`, because `classify_simple`'s own access gate refuses
`*` and `[` where it admits `+`.

## 7. Lean

`proof/QCore/ReturnCallCustody.lean`, registered in `QCore.lean`, audited by
`proof/AuditReturnCallCustody.lean`. Axiom-clean: `propext` and `Quot.sound`
only.

| theorem | content |
|---|---|
| `the_lowering_is_the_desugaring` | the admitted lowering **is** the two-statement form — `rfl`, and that is the whole content |
| `a_none_result_escapes_nothing` | the accusing direction: a `None` result leaves the state untouched |
| `a_produced_site_is_absent_from_the_pre_state` | freshness is structural, from the counter's invariant |
| `admitted_modes_discharge_nothing_prior` | **the theorem the card rests on**: for every admitted mode, every site of the pre-state is unchanged |
| `an_owned_result_escapes_only_the_site_it_produced` | sharper: exactly one site moves, and it is the produced one |
| `a_reallocated_result_would_have_discharged_its_parameter` | **non-vacuity of the refusal**, in constraint 15n's established shape: the refused arm is refused because admitting it *would* have moved a live prior obligation |
| `only_reallocation_is_refused` | the implemented gate refuses exactly that arm — closing the finite vocabulary to "admitted ⟺ discharges nothing prior" |

The parameter half is deliberately **not** modelled, and
`the_lowering_is_the_desugaring` is where that is stated formally rather than
left as a gap: whatever the call does to its arguments, it does it in the
position that always did it.

## 8. Tests, and their non-vacuity

`crates/qcore-c-gateway/tests/return_position_modeled_call.rs` (14 rows) plus one
white-box row in `crates/qcore-c-gateway/tests/unit/legacy_c.rs`. Held-out
contract fixture `tests/fixtures/contracts/r_return_call.qcore` — a new `r_*`
family, because no existing family has a callee that **returns a value** while
carrying a custody effect on a parameter (`q_free`/`q_transfer` return `unit`, so
`return q_free(p);` is not C).

The central property is an **equivalence**: an admitted `return f(a);` produces
exactly the status, certification and diagnostics of its hand-written
desugaring. An equivalence cannot be satisfied by refusing, which is what makes
it a stronger pin than a status assertion.

**7 of 7 mutations caught** (`logs/mutations.txt`). Every arm this card adds was
negated and the suite went red, naming the catching test:

| mutation | arm negated | caught by |
|---|---|---|
| `reallocated-admitted` | the `Reallocated` refusal | `a_reallocated_result_is_refused_at_a_return` |
| `owned-refused` | the `Owned` admission | 4 rows |
| `collision-guard-off` | the reserved-place guard | `a_body_that_spells_the_reserved_place_is_refused` |
| `whole-expression-off` | the whole-expression premise | `a_tail_after_the_call_cannot_launder_the_produced_site` |
| `exit-does-not-escape` | the exit's escape of the result | 3 rows |
| `site-restore-removed` | the declined probe's site restore | `a_declined_return_call_probe_restores_the_site_counter` |
| `admits-any-instruction` | the modeled-call gate | 3 rows |

The `whole-expression-off` row was **not** caught on the first campaign. Rather
than record it as an unexercised arm, the gap was investigated: the shape that
reaches past `classify_simple`'s own guard was found (a call with no tracked
argument and a tail after its `)`), measured to produce a **false `Safe`**, and
pinned. This is the project's live `RETURN-POSITION-ESCAPE-UNREACHABLE` finding
being acted on rather than repeated.

## 9. Gate

Full standing per-change gate, measured at this card's tip. Details in
`gate.json`.

| step | result |
|---|---|
| `git submodule update --init --recursive` | 0 — libevent at the pinned `d82464a2` |
| `cargo test --workspace --no-fail-fast` | **4,558 passed / 1 failed / 8 ignored**, exit 101 |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `reevaluation_probe` | **exit 0** |
| `reevaluation_followup` | **exit 0** |
| `scripts/check_lean_audits.py` | 0 — **129 audits**, axiom surface `propext` + `Quot.sound` |
| Python (`scripts/tests`, CI's module form) | 0 — **229 tests** |

The one failure is the registered
`VERIFIER-HOST-LARGE-RESPONSE-CONTROL-RED` row —
`closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request` at
`crates/qcore-verifier-host/tests/standalone_host.rs:212`, a wall-clock
throughput shortfall `STABILITY-REEVAL` #1 measured and this card neither owns
nor touched. Reported as the known row, with `--no-fail-fast` ensuring it hid
nothing behind it. The honest baseline for this host is **4,543 passed / 1
failed / 8 ignored**; this card adds **15** tests (14 end-to-end + 1 white-box),
and 4,543 + 15 = 4,558 exactly, so nothing else moved.

The gateway crate alone: **1,000 passed / 0 failed / 7 ignored**.

`providers/package.py` was **not** run, and the ground is stated rather than
assumed: constraint 18 requires a rebuild after a change to the manifest
template or to a `qcore-schemas` **major** version. This card changes neither —
`qcore-schemas` stays 14.0.0, no `provider-template.json` is touched, and no
provider crate is a dependency of `qcore-c-gateway`.

A second pinned expectation moved, and it is a **derived** field rather than a
judgment: `docs/reviews/r0_3/acceptance_matrix.json`'s
`validation.lean_audits` 128 → 129, because this card adds an `Audit*.lean`.
Regenerated with `docs/reports/remaining-d02/build_matrix.py` passing
`--gate docs/reports/wave33-close/gate.json`, which makes **only the two derived
Lean fields move** and preserves wave 33's gate provenance verbatim — without
that flag a whole-file regeneration would relabel the gate block
`carried_from_previous_matrix` and lose the attribution, which constraint 25
warns against.

## 10. Named follow-ons

* **`RETURN-CALL-W4-REACH-BLOCKED`** — the W4 route contributed **1** of this
  card's 14 leaves, because 366 of 643 `return CALL(...)` leaves have a callee
  whose own body is `Unknown` and therefore yields no validated summary
  (§3.3). No larger return-position card is available on this route until those
  bodies are modeled. **Discharge condition:** a measured statement of which
  refusal classes dominate those 366 callees' own bodies, which is the real
  next question and is deliberately not answered here.
* **`RETURN-CALL-SYNTHETIC-PLACE-PRESENTED`** — 10 published leaves name
  `__qcore_returned_value`, a place that appears in no source. The rows are true
  and `Unknown`-class, so this is presentation, not soundness. **Discharge
  condition:** render the reserved place as a phrase ("the value this call
  returned") wherever a diagnostic carries a place; deliberately not taken here
  because it touches every diagnostic that carries one.
* **`RETURN-CALL-NULLABLE-WRAPPER`** — the 5 functions of §5.3.1 whose new leaf
  is `nullable resource … used by a non-null contract` are allocator
  wrappers that legitimately return NULL. The complaint comes from analysing
  them without their *own* derived contract in force. **Discharge condition:**
  measure whether the derivation can infer `Owned { nullable: true }` for such a
  wrapper, and whether doing so is a promotion this project wants.
* **`RETURN-CALL-INCOMPLETE-LINE-455`** — 455 of 1,470 return leaves (31%) have
  an `origin` line on which the `return` is not complete, so §3.2 classifies
  them as shape-unknown rather than guessing. **Discharge condition:** a
  multi-line-aware classifier, or a measurement showing the class is
  uninteresting.

## 11. For the integrator

Land these in these words, or record a verdict with its ground
(`W32-RECONCILE-SWEEP` discipline).

1. **`AGENTS.md` headline numbers.** Corpus baseline becomes
   `docs/reports/c-gateway-return-call-w4/artifacts/`. Function statuses
   **179 → 186 Safe**, Unknown 4,570 → 4,563, provisional Unsafe **85 unmoved**.
   `unsupported` **50,117 → 50,106**. `unknown_alias` **4,208 → 4,214**. Leak
   **272 unmoved**; `double_release` **1 unmoved**. Unit statuses unmoved.
2. **Standing constraint 7** (the leak pin) gains one sentence: *"At the wave-34
   `C-GATEWAY-RETURN-CALL-W4` arbiter the pin is **unmoved at 272** — the first
   gateway admission card in this chain to move no leak leaf in either
   direction, because the admitted exits are allocator wrappers that retain
   nothing."*
3. **Standing constraint 4** gains the re-derivation, because it is a fact about
   that constraint's own second route: *"'Defined in this translation unit' is
   not 'W4-derivable': validation by re-analysis requires the callee's own body
   to be fully modeled and defect-free, and over the 643 `return CALL(...)`
   leaves of the pinned corpus that holds for **4 (0.6%)**, against 236 (70.5%)
   that are merely defined here (`C-GATEWAY-RETURN-CALL-W4` §3.3). A reach
   argument that counts definedness overstates this route by two orders of
   magnitude."*
4. **The deliberate pin update** in §6 — one renamed test with its control
   written in. It is not a weakening: the statement form was already `Safe`.
5. **Four residuals** from §10 into the tracker:
   `RETURN-CALL-W4-REACH-BLOCKED`, `RETURN-CALL-SYNTHETIC-PLACE-PRESENTED`,
   `RETURN-CALL-NULLABLE-WRAPPER`, `RETURN-CALL-INCOMPLETE-LINE-455`.
6. **Regenerate the acceptance matrix at the close with the wave-34 gate.** This
   card moved only `validation.lean_audits` 128 → 129 and deliberately passed
   wave 33's gate file so nothing else would move (§9); the close's own
   regeneration should carry its own `--gate`.
7. **`AGENTS.md`'s "Merged gate" ledger row still cites
   `docs/reports/wave32-close/gate.json`** and its counts (4,534 tests, 127 Lean
   audits, 227 Python). Waves 33 and 34 have both moved those numbers — this
   card measures 4,558/1/8, **129** Lean audits and **229** Python. A card
   should not overwrite a wave-close row, so it is left alone and flagged here
   rather than silently edited; it is two waves stale either way.
8. **The `STABILITY-REEVAL` cadence has a case here.** This card's mutation
   campaign found an arm its own first test set could not distinguish from its
   negation, investigated rather than filed it, and the shape it then found
   produced a **false `Safe`** (§8). That is the cadence's method working inside
   a card; the next firing may want to record it as a precedent for "investigate
   the uncaught mutation" rather than "report the unexercised arm".

## 12. Reproduction

```sh
git submodule update --init --recursive
cargo build --release -p qcore-compiler --example cycle19_fluent_bit_libraries_audit
cp target/release/examples/cycle19_fluent_bit_libraries_audit \
   measure/c-gateway-return-call-w4/bin/audit-after
python3 -B measure/c-gateway-return-call-w4/run_audit.py after \
   measure/c-gateway-return-call-w4/bin/audit-after
python3 -B measure/c-gateway-return-call-w4/byte_identity.py \
   measure/c-gateway-return-call-w4/runs/before \
   docs/reports/c-gateway-return-position/artifacts
python3 -B measure/c-gateway-return-call-w4/census.py
python3 -B measure/c-gateway-return-call-w4/reach.py
python3 -B measure/c-gateway-return-call-w4/mutations.py
```

The BEFORE binary is the same build at base `c4b24a9b`. Each run takes about
four and a half minutes. Scripts are archived beside this report under
`scripts/`; they read the pinned tree at
`/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin` and
write only to the worktree-private `measure/` directory.

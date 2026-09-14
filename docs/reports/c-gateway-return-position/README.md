# C-GATEWAY-RETURN-POSITION — a pointer-valued chain leaf in a `return`

Wave 33's only card, and therefore the wave's **arbiter**: it owns the corpus
re-pin.

## Headline

The C gateway now admits a **pointer-valued bounded typed access chain** in a
`return` expression, where card REACH-GATE admitted it only in a position that
discards the value. The admission is answered with `returned_place: None`, so
the exit is a `NoOp` and escapes nothing.

Measured on the pinned 185-unit Cycle 19 corpus, BEFORE → AFTER:

| Measure | Before | After | Delta |
|---|---:|---:|---:|
| `unsupported` | 50,150 | **50,117** | **−33** |
| `leak` | 272 | **272** | **0** |
| `unknown_alias` | 4,208 | **4,208** | **0** |
| `double_release` | 1 | **1** | **0** |
| unit statuses | 2 Safe / 42 Unsafe / 137 Unknown / 4 data-only | unchanged | **0 moves** |
| function statuses | 146 Safe / 85 Unsafe / 4,603 Unknown | **179 Safe** / 85 Unsafe / **4,570 Unknown** | **33 promotions** |

**33 leaves removed, 0 added.** All 33 are return-position refusals. Thirty-three
functions move `Unknown` → `Safe`; no function or unit is withdrawn from `Safe`,
none becomes provisional `Unsafe`, and **no unit status moves at all**. Raw
corpus units remain **unpromoted**.

**Sub-shape B (`return CALL(args)`) is withdrawn**, with its measurement
published in §3. A withdrawal with its grounds measured is this card's deliberate
close for that half.

The honest framing of the yield: the profile that motivated this card said the
return position blocks **511** functions and nothing else blocks them. This card
decides **33 of those 511 — 6.5%**. It is not "the return-position lever"; it is
one premise-bounded slice of it, and §4 says exactly which slice and why the
other 478 are untouched.

## 1. What changed

Two files, both in the target crate `crates/qcore-c-gateway/`:

* `src/neutral_position.rs` — the module header sentence that recorded the old
  refusal is **amended in place** rather than left to contradict the code, and
  `ExpressionRole::Returned`'s own documentation now carries the argument and the
  obligation it places on the parser.
* `src/legacy_c.rs` — the `Some("return")` arm of `Parser::parse_statement`:

```rust
let strict = self.supported_expression(&expression, ExpressionRole::Returned);
let untracked_pointer =
    !strict && self.supported_expression(&expression, ExpressionRole::Decides);
Ok(Statement::Return {
    line,
    writes: self.expression_writes(&expression),
    expression_supported: strict || untracked_pointer,
    returned_place: if untracked_pointer { None } else { canonical_place(&texts) },
})
```

The rule in one sentence: **the return position now admits what a condition
position admits, and the difference between them — the pointer the chain
produces — escapes nothing.**

Three properties of that shape are load-bearing:

1. **The strict rule is asked first and its answer is never weakened.** Where
   `ExpressionRole::Returned` already admitted, the instruction is exactly what
   it was, so `return p;` still emits `EscapeIfOwned` at `p`. The discharging
   direction is untouched by this card.
2. **The widening is answered `None`, never `canonical_place`.** This is not a
   convenience. `canonical_place` spells `view->head->next` perfectly well, and
   answering with it would ask the projection to escape a place the admission
   never established — invariant 1's forbidden direction.
3. **The recognition is positive and the fall-through is refusal.** An expression
   neither rule admits keeps the refusal it already had. No new vocabulary is
   added: a dereference, a subscript, an address-of, an uncontracted call, an
   untyped member and a chain past `ACCESS_CHAIN_LIMIT` refuse exactly as before.

No new `Q` code is allocated and none is needed — gateway diagnostics are
categories, not `Q` codes, so the §L21 namespace icebox does not engage. No
diagnostic text changes; the 33 moved leaves are **withdrawn**, not reworded.

## 2. Why admitting this is sound

Card REACH-GATE refused a pointer leaf in a `return` on a stated ground:

> `legacy_c`'s `EscapeIfOwned` models only a *canonical place*: a compound return
> expression that produced a pointer would escape it unmodeled.

That reasoning is **kept, and its missing answer supplied**. The F4 chain rule's
base premise is `!FunctionTypes::may_bind(base)` — `admitted_chain` refuses a
bound base at every depth. Every `->` place the projection actually tracks comes
from card O04-B08's aggregate store, whose base must be a body-acquired name,
which `may_bind` reports bound by construction. The two together give:

> **An admitted chain names no place the projection binds, at any depth. The
> pointer it produces is therefore not a tracked identity of this body, and the
> exit has nothing to escape.**

So `NoOp` is *faithful*, not optimistic. And the direction it moves is the
**accusing** one: the old `Unknown` marked the path unmodeled, and `report_leaks`
skips an unmodeled exit. Withdrawing the refusal exposes whatever that body still
owes at that exit — it cannot hide anything that was previously visible.

`may_bind` over-approximates binding (a name written, address-taken, or appearing
anywhere in a call's argument list, plus a protected formal at entry), and it
over-approximates in the direction that *increases* refusal, which is the correct
direction for a premise a soundness argument leans on.

## 3. Sub-shape B, measured and withdrawn

The card offered a second sub-shape: `return CALL(args)` where the callee has a
declared contract or a validated W4 summary. It is **withdrawn**. The grounds are
measured over the whole 511, not sampled.

First, what is already true: a call in return position whose contract is
all-`Borrowed` with result `None` is **already admitted** today, through
`admitted_neutral_call`. So sub-shape B's actual content is narrower than it
sounds — it is admitting calls whose contract is *not* neutral, by applying their
effects at the return position.

Second, the population. Of the 538 return-position leaves that block the 511
functions, 335 are `return CALL(...)`. Their callees:

| callee | count | share |
|---|---:|---:|
| named in no `.qcore` declaration, not otherwise reported | 227 | 67.8% |
| reported elsewhere as `call to uncontracted function` | 94 | 28.1% |
| **has a declared contract** | **12** | **3.6%** |
| compiler builtin (`__atomic_*`) | 2 | 0.6% |

Cross-cut by whether W4 could derive a summary at all (is the callee defined in
the unit's own `.c`?): 236 of 335 (70.5%) are same-unit definitions, 99 (29.6%)
are external and stay `Unknown` whatever happens.

So the decisive number is **12 of 335**. For the remaining 323, clearing the
return refusal converts one refusal into another — the function gains a `call to
uncontracted function` leaf and is **no better decided**. That is precisely the
outcome the card names as unacceptable to publish as progress.

Third, and decisively, the soundness cost. Applying a callee's parameter custody
at a return position means a `Consumed`/`Released` parameter mode would
**discharge** a live obligation of the caller at that exit. That is invariant 1's
forbidden direction, and it cannot be taken on a summary route whose exit
observer (`observe_exit`) has no arm for a call result at all — it constructs
`ReturnedValue` from `EscapeIfOwned`/`Escape` and from nothing else. Landing B
honestly means a new `LegacyInstruction`, a new `ReturnedValue` construction, and
matching changes in the summary validator and the certificate recheck. That is a
card, not a sub-shape, and it should not be attempted for a decisive yield of 12.

Reproduction: `measure/c-gateway-return-position/callees.py` and `w4_reach.py`
(worktree-private; the scripts are archived beside this report).

## 4. The real 511-function split, and why only 33 move

The card supplied a 120-function sample as a lead. Re-measured over the whole
population, against the pinned tree at `66910c10`, resolving all 538 leaves
(538/538 resolved):

| shape of the returned expression | leaves | share |
|---|---:|---:|
| `return CALL(...)` | 335 | 62.3% |
| `return <field chain>` | 75 | 13.9% |
| other (arithmetic/comparison over field reads, casts of chains) | 58 | 10.8% |
| conditional expression `?:` | 28 | 5.2% |
| statement not complete within the continuation window | 22 | 4.1% |
| subscript | 6 | 1.1% |
| cast | 5 | 0.9% |
| address-of | 5 | 0.9% |
| origin line is not itself a `return` (macro) | 2 | 0.4% |
| dereference | 1 | 0.2% |
| plain name | 1 | 0.2% |

The sample's "~63% call" reproduces (62.3%). Its "~30% field/deref chain" does
**not**: pure field chains are 13.9%, and reaching 30% requires adding the cast,
subscript, address-of, deref and arithmetic buckets, which are five different
refusals rather than one. Publishing the finer split is the point of
re-measuring.

### The three-way split the card requires

Of the 511 functions the baseline blocks on the return position and nothing else:

| outcome | count | share |
|---|---:|---:|
| **(1) end with zero leaves** | **33** | **6.5%** |
| **(2) exchange the return leaf for a different refusal** | **0** | **0.0%** |
| **(3) unmoved — the return leaf remains** | **478** | **93.5%** |

Bucket (2) is empty **by construction, not by luck**: this card admits only
expressions the neutral walk already fully accounts for, so an admitted return
carries no residue that could become a second refusal. That is the structural
difference from sub-shape B, where bucket (2) would have been the large one.

All 33 of bucket (1) reach `Safe`. **Leaf-free is not what makes them `Safe`** —
`Safe` additionally requires every obligation discharged and full analysed
coverage at the certificate gate (constraint 12), and the audit's certificate
recheck runs on each. These are accessor bodies that acquire and release nothing,
so both conditions hold; the histogram is the measured answer, not an inference
from the leaf count.

### Why 478 do not move

The F4 premises are strict, and every one of them is doing work here. The 75 pure
field chains are the natural candidate population, yet only 31 of them are
admitted. The rest fail on premises the card must not weaken: the member is not
typed **exactly** `struct TAG *` (a `char *`, a `cfl_sds_t`, a typedef the reader
will not resolve, a `void *`), the record is incomplete, a union, or has more than
one definition in the unit, or the base is bound because it is named in some
call's argument list elsewhere in the body. The remaining buckets — calls, casts,
subscripts, address-of, `?:` over unresolved members — are outside this card
entirely.

## 5. Every moved leaf, attributed

All 33 are removals of the description *`unmodeled call, aggregate or indirect
access in return expression`*. Zero leaves are added; zero non-`unsupported`
leaves move in either direction.

The full table, with each leaf's **internal preprocessed line** (its identity,
constraint 12a), its presented `origin`, and the C at that coordinate read back
out of the pinned source, is
[`logs/moved-leaves.txt`](logs/moved-leaves.txt). Read back, the 33 are:

* **31** a single-selection pointer-valued chain — `return acl->host;`,
  `return member->client_id;`, `return grpdesc->group_id;`,
  `return resource_span->resource;`, and so on: 22 in `rdkafka_admin.c`, 6 in
  `rdkafka_aux.c`, 3 in `rdkafka_cgrp.c`, 1 in `ctr_resource.c`;
* **1** pointer arithmetic over a chain leaf — `chunkio/cio_file.c:1306`,
  `return (cf->map + 2);`. `canonical_place` refuses this spelling anyway, so it
  was a `NoOp` under either answer;
* **1** a scalar comparison of two pointer leaves — `librdkafka/snappy.c:496`,
  `return w->op == w->op_limit;`. The strict rule refused it because a pointer
  *leaf* is read, even though the value produced is an `int`.

Each is a `base->field` read through a formal the body never binds. Nothing in
the list is an acquisition, a release, or a body carrying a live obligation.

## 6. Pin statements

* **Leak stays 272.** Constraint 7's number does **not** move at this arbiter.
  The historical chain (271 → 272 → 273 → 272 → 266 → 272) is unchanged and
  unextended by this card.
* **`unknown_alias` stays 4,208** and **`double_release` stays 1.**
* **`unsupported` 50,150 → 50,117** (−33), every leaf attributed in §5.
* **`chunk-contracts.json` is byte-identical** from the wave-31 baseline, through
  the BEFORE run, to the AFTER run. So are `source.json`,
  `source-effects.json` and `contracts.json` between BEFORE and AFTER:
  **`diagnostics.json` is the only artifact this card moves.**
* **Contract corpus unchanged at 76 modules.** This card declares nothing.
* **Zero unit status moves.** Raw units remain unpromoted; the Cycle 19 gates in
  `docs/cycles/CYCLE_19.md` are untouched.
* **All earlier archives stay frozen** (constraint 21). Nothing under
  `docs/reports/w31-corpus-contracts-2/`, `c-gateway-wave/`,
  `w4-recursive-summaries/` or any predecessor is edited; this card writes a new
  archive beside them.

### The BEFORE reproduction, stated explicitly

**The BEFORE run reproduces the current baseline
`docs/reports/w31-corpus-contracts-2/artifacts/` byte for byte in both
verdict-bearing artifacts:**

```
*diagnostics      raw IDENTICAL  canonical IDENTICAL   ffe47fe666a76417 / ffe47fe666a76417
 source           raw differs    canonical differs     e42fe7a9c40af1ca / ae38f6c93dfbe341
*chunk-contracts  raw IDENTICAL  canonical IDENTICAL   c4f438e4ed3dab3c / c4f438e4ed3dab3c
 source-effects   raw differs    canonical differs     e3fb5b62ece6a8a3 / f1a3c275d5ea0f43
 contracts        raw differs    canonical differs     dd1276cfa80f4850 / 17fe29bdb44860d0
```

The three differing artifacts are the checkout-root and identity-digest class
every re-pin in this chain has recorded; they carry no verdict. This is the
load-bearing fact of the measurement: because the BEFORE reproduces the baseline
exactly, every AFTER delta is **inside** the BEFORE and attributable to the
gateway change alone. The assertion is not merely logged — it is re-checked
inside `evidence.py`, which refuses to write any archive if it stops holding.

Transport is `HistoricalStdin` throughout (constraint 11), unchanged; the pin,
compile database (`5e6868d9…`, 1,415 entries) and ChunkIO package are the wave-31
chain's own. Full log: [`logs/byte-identity-before.txt`](logs/byte-identity-before.txt).

## 7. The proof boundary, in plain words

`proof/QCore/ReturnPositionEscape.lean`, audited by
`proof/AuditReturnPositionEscape.lean` (registered in `proof/QCore.lean`), twelve
theorems, all axiom-clean at `[propext]`.

**Proved**, about the rule and not about a witness:

* an admitted chain names **no tracked place**
  (`an_admitted_chain_names_no_tracked_place`), from `admitsChain`'s own shipped
  refusal of a bound base;
* the strict rule's answer is unchanged wherever it answered
  (`the_strict_answer_is_unchanged`) — this is why `return p;` still escapes;
* the widening replaces **refusals only**
  (`the_widening_only_replaces_refusals`) and **never manufactures an escape**
  (`the_widening_never_creates_an_escape`) — invariant 1's direction;
* an expression neither rule admits keeps its refusal
  (`an_unadmitted_expression_keeps_its_refusal`);
* the widened rule is **accusing**: it reports at least what the narrow rule
  reported (`the_widening_reports_at_least_as_much`), and where they differ only
  in the direction of exposing more (`the_widening_is_accusing`), shown
  non-vacuous by an exhibited witness.

**A premise, stated and not proved here:** `TrackedBasesAreBound` — that the
projection tracks no place whose base the body does not bind. That is the
aggregate store's construction fact, and `QCore.AggregateStore` is where it
lives. Faithful extraction of the declaration facts a `Chain` records is the same
obligation `QCore.TypedFieldAccess` already carries. These are abstract rule
theorems, not an extraction of the Rust analyzer and not a statement about C.

**Observed, not proved:** that the 33 corpus functions this card moves are the
right ones. The theorem says the rule cannot discharge; it does not say the
reader typed `rdkafka_admin.c` correctly. §5 answers that by reading every moved
leaf's C back out of the pinned source, which is evidence, not proof.

## 8. Tests, and their non-vacuity

`crates/qcore-c-gateway/tests/return_position_pointer_leaf.rs` — 9 tests, each
arm in both directions, plus the re-pinned pair in `reach_gate_positions.rs`.
Every refusal row is paired with the **same expression in a condition position**,
where it is admitted: a refusal that held for an unrelated reason would refuse
there too, so the pairing is what makes the row evidence about *this* rule.

### A deliberate re-pin

`reach_gate_positions.rs`'s
`a_pointer_leaf_is_admitted_in_a_condition_and_refused_in_a_return` pinned the
old refusal. It is **replaced by the stronger pair** — admitted *and* escaping
nothing — and renamed, not deleted; a new sibling
(`a_pointer_leaf_return_escapes_nothing_it_could_own`) pins the discharging
direction on a body carrying a live obligation. Recorded here and in the commit
message per `CLAUDE.md`'s working rule. No gate is weakened.

### Non-vacuity by mutation, and one arm that is vacuous

Full transcript: [`logs/mutations.txt`](logs/mutations.txt).

| mutation | result |
|---|---|
| **M2** never admit the pointer leaf in a return | **RED** — 5 tests |
| **M3** drop the strict-rule-first ordering | **RED** — `a_plain_canonical_place_still_escapes` |
| **M4** drop the `!may_bind(base)` base premise | **RED** — 7 tests, two files |
| **M1** escape the canonical place instead of `NoOp` | **GREEN — vacuous** |
| **M5** = M4 + M1 together | RED, but by exactly M4's tests |

**M1 is the finding of this section and it is reported as one.** The arm that
forces `returned_place: None` — the arm that *implements* invariant 1's direction
— is not distinguished by any test. Replacing it with `canonical_place(&texts)`
leaves the suite green.

The reason is the soundness argument itself: an `EscapeIfOwned` on an admitted
chain consults `state.pointers` for a place the base premise guarantees is never
there, and acts as a no-op. So the two answers **agree on every admitted
program** — which is proved, as
`the_two_answers_agree_on_every_admitted_chain`. M5 confirms the arm stays
unobservable even with the base premise removed, because those fixtures are then
refused at a different gate first.

The arm is therefore a **backstop**, kept rather than simplified away because
`without_the_base_premise_the_answers_part` proves the two answers separate — in
the discharging direction — the moment the base premise weakens. It is justified
by the theorem, **never by the green suite**, and the source comment at
`ExpressionRole::Returned` says so in those words. Residual
`RETURN-POSITION-ESCAPE-UNREACHABLE` below.

### A second honest boundary

The **accusing** direction is proved and test-driven
(`an_admitted_return_reports_the_obligation_it_could_have_laundered` measures a
leak the old refusal suppressed), but it is **not exercised by the corpus**: the
AFTER run adds **zero** leaves. All 33 moved functions are accessors with no live
obligation. The corpus therefore contains no instance of this card's accusing
arm, and the unchanged leak pin is evidence of that absence rather than evidence
that the arm works. Residual `RETURN-POSITION-ACCUSING-UNEXERCISED-IN-CORPUS`.

## 9. Gate

Run at the card tip. Exits reported honestly.

| Command | Result | Exit |
|---|---|---|
| `git submodule update --init --recursive` | libevent at `d82464a2…` | 0 |
| `cargo test --workspace --no-fail-fast` | **4,543 passed, 1 failed, 8 ignored**, 229 test binaries | **101** |
| `cargo fmt --all -- --check` | clean | 0 |
| `cargo clippy --all-targets -- -D warnings` | clean | 0 |
| `cargo run -p qcore-compiler --example reevaluation_probe` | clean | 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | clean | 0 |
| `python3 scripts/check_lean_audits.py` | **Passed 128 Lean audits** (127 → 128) | 0 |
| `python3 -m unittest discover -s scripts/tests` | Ran **227** tests, OK | 0 |
| `cycle19_fluent_bit_libraries_audit` BEFORE | five artifacts | 0 |
| `cycle19_fluent_bit_libraries_audit` AFTER | five artifacts | 0 |

### The one failure, reported as a finding and not absorbed

`qcore-verifier-host --test standalone_host`:
`closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request`,
at `standalone_host.rs:208`, `assert!(matches!(provider.verify(&request),
Err(ProviderFailure::Io(_))))`.

**It is pre-existing and this card cannot have caused it.** Three facts, each
measured rather than argued:

1. It fails **in isolation**, deterministically, 3 runs of 3 — so it is not one
   of constraint 16's load flakes, and re-running it does not make it pass.
2. It reproduces **at this card's base commit `37bc9004`**, checked out and run
   there directly.
3. `qcore-verifier-host` depends only on `qcore-frontend` and `qcore-schemas`.
   This card changes four files, all under `crates/qcore-c-gateway/`, which is
   outside that crate's dependency closure.

It is **not** one of constraint 16's five named rows, but it carries the same
shape as the confined-fixture row: an `Io(_)` expectation behind a
`timeout_ms = 100` deadline against a shell script, whose own comment
anticipates "a slower macOS scheduler". On this host the deadline appears to
expire first. **The assertion must not be weakened to make the gate green** —
that is `CLAUDE.md`'s first working rule. Follow-on
**`VERIFIER-HOST-CLOSED-INPUT-DEADLINE`** below.

### Note on the submodule

The libevent pin gate fires at the **end** of an audit run, so a worktree whose
submodule is uninitialised loses a full four-and-a-half-minute corpus run to a
`SourceRevisionMismatch` that reports the worktree's own HEAD as the "actual"
libevent revision. That happened here once and cost one BEFORE run.
`git submodule update --init --recursive` belongs **first** in any audit route,
as the wave-31 route already records — this card is a second, independent
confirmation of that ordering.

## 10. Named follow-ons

* **`RETURN-POSITION-ESCAPE-UNREACHABLE`** — the `returned_place: None` arm is
  unobservable by any test (measured: mutation M1 is green, M5 confirms it under
  a weakened base premise). *Discharge:* either a fixture that makes the
  projection track a place an admitted chain can name — which requires a card
  that weakens `admitted_chain`'s base premise, and which must then land this
  arm's test in the same change — or a standing decision that the arm's
  justification is `QCore.ReturnPositionEscape` alone, recorded as a measured
  boundary. Until then the arm must not be "simplified" away: it is the fail-closed
  spelling, and `without_the_base_premise_the_answers_part` is why.
* **`RETURN-POSITION-ACCUSING-UNEXERCISED-IN-CORPUS`** — the accusing arm is
  proved and unit-tested but reaches no function of the pinned 185-unit corpus
  (zero leaves added). *Discharge:* a corpus, fixture family or later card in
  which an admitted pointer-leaf return sits in a body with a live obligation, so
  the leak the widening exposes is measured rather than only constructed.
* **`RETURN-POSITION-SUBSHAPE-B`** — `return CALL(args)` with a non-neutral
  contract or validated summary, withdrawn here with its measurement (§3).
  *Discharge:* a card that lands a return-position effectful instruction together
  with its `ReturnedValue` construction in `observe_exit`, the matching summary
  validation and certificate recheck, and that states up front how many of the
  335 it **decides** rather than how many refusals it converts. Its decisive
  population today is 12.
* **`RETURN-POSITION-TYPED-MEMBER-REACH`** — 44 of the 75 pure field-chain
  returns are not admitted, dominated by members the reader will not type as
  exactly `struct TAG *` (`char *`, `cfl_sds_t`, typedefs, `void *`). *Discharge:*
  measure, per refused premise, how many functions a widened member typing would
  decide — **predicted before it is attempted**, as `AOM-POISON-CONDITION-FRAME`'s
  discharge condition requires — and only then decide whether the premise can be
  widened without losing the base-premise argument this card rests on.
* **`RETURN-POSITION-CONDITIONAL`** — 28 leaves are `?:` expressions over field
  reads. These refuse today for their operands, not for the conditional. *Discharge:*
  a measurement of how many would be admitted if every operand were an F4 chain.
* **`VERIFIER-HOST-CLOSED-INPUT-DEADLINE`** — `standalone_host.rs`'s
  `closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request`
  fails deterministically on this host, at this card's base commit and in
  isolation, outside this card's dependency closure (§9). *Discharge:* establish
  whether the `timeout_ms = 100` deadline is host-specific — if so, record the
  row beside constraint 16's five with its measured load/host condition, exactly
  as those five are recorded; if not, it is a real defect in the host's
  closed-input handling and needs a repair card. **Either way the assertion
  stands as written**: the row must not be relaxed to green the gate.

## 11. For the integrator

**The corpus baseline moves to `docs/reports/c-gateway-return-position/artifacts/`.**
The chain is `docs/reports/wave23-c-operations/artifacts/` (the frozen corpus
baseline) → `aom-destroy-siblings/` (wave 23) → `aom-binding-poison/` (wave 25) →
`gateway-c-location/` (wave 27, presentation-only) → `w29-corpus-contracts/`
(wave 29) → `w4-recursive-summaries/` (wave 30) → `c-gateway-wave/` (wave 31) →
`w31-corpus-contracts-2/` (wave 31, the previous baseline) → **this archive**
(wave 33), every predecessor frozen (constraint 21).

1. **`unsupported` 50,150 → 50,117** (−33). All 33 are withdrawn return-position
   refusals, itemised in §5 and in `logs/moved-leaves.txt`.
2. **Leak stays 272.** Constraint 7's chain is unchanged and this card adds no
   link to it. `unknown_alias` stays **4,208**; `double_release` stays **1**.
3. **Zero unit status moves.** Unit histogram unchanged: 2 Safe / 42 provisional
   Unsafe / 137 Unknown / 4 data-only. **Nothing is promoted at unit level and
   raw units remain unpromoted.**
4. **33 function promotions `Unknown` → `Safe`**; function histogram
   146/85/4,603 → **179 Safe / 85 Unsafe / 4,570 Unknown**. Zero withdrawals from
   `Safe`, zero new provisional `Unsafe`. These are deliberate and argued (§2),
   on the REACH-GATE precedent for an admission widening; they are **not** a
   refactor flipping `Unknown` to `Safe`.
5. **Contract corpus unchanged at 76 modules.** Nothing under `contract/qcore` is
   edited, so `qcore_contract_corpus.rs`'s pinned tables are untouched
   (constraint 5).
6. **Lean audits 127 → 128.** New: `proof/QCore/ReturnPositionEscape.lean` and
   `proof/AuditReturnPositionEscape.lean`, imported in `proof/QCore.lean`.
7. **The workspace gate is not green, and must not be made green here.** One
   failure, `qcore-verifier-host`'s
   `closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request`,
   measured pre-existing at this card's base commit and outside its dependency
   closure — §9 has the three measurements. Follow-on
   `VERIFIER-HOST-CLOSED-INPUT-DEADLINE`.

### What this card already landed (verify, do not re-apply)

Per `CLAUDE.md`'s sync rule these are the card's own responsibility and are
**already in the tree**:

* **`AGENTS.md`** — the baseline paragraph now names
  `docs/reports/c-gateway-return-position/artifacts/` and adds this card's
  byte-for-byte link to the attribution chain; the **Cycle 19 raw corpus** row
  carries `179 Safe / 85 / 4,570`, `unsupported` **50,117** and the unmoved leak
  **272**; the **Merged gate** row carries §9's numbers *including the one
  failure and why it is not a regression*; and new standing constraint **5b**
  states the rule, both measured boundaries and the two "do not" sentences.
* **`docs/development/increments/wave-33.md`** — created, one paragraph; the
  index pointer in `docs/development/increments/README.md` moved to it and the
  wave table gained its row.
* **`docs/reviews/r0_3/acceptance_matrix.json`** — `validation.lean_audits`
  127 → 128 and `"AuditReturnPositionEscape.lean"` inserted in
  `lean_audit_files`; `named_followups` 42 → **45** with this card's three
  otherwise-untracked residuals, and `dimensions.named_followups` moved with it.
  Edited **surgically, not regenerated** (constraint 25: whole-file regeneration
  must not clobber later reconciliations), and the writer was verified
  byte-exact on a round trip *before* it wrote anything — wave 32 recorded a
  reserialization to pre-HYGIENE-16 formatting as an integrator error, so that
  check is deliberate. The Lean edit was forced by the designed tripwire
  `test_build_matrix_sections.test_the_derived_facts_are_recomputed_rather_than_carried`,
  which failed until both values moved; the follow-on entries were prompted by
  `scripts/reconciliation_sweep.py --residuals`, which reported three of this
  card's six residual names reaching no tracker and now reports **zero**.
  `AGENTS.md`'s "`named_followups`, 40 entries" pointer was stale on arrival and
  is corrected to 45 in the same change.
* **`README.md`** — deliberately untouched, per the sync rule.

### What is left for the integrator

* **§L21: no change, and this is a decision rather than an omission.** No `Q`
  code is allocated, no `Q` condition is widened and no diagnostic text moves —
  gateway diagnostics are categories, not `Q` codes, so the namespace icebox
  does not engage. The 33 moved leaves are **withdrawn**, not reworded.
* **The corpus baseline pointer**, if the integrator keeps one outside
  `AGENTS.md`: it moves to this card's `artifacts/`.
* **Constraint 7's leak history** needs no new link — the pin did not move.

### Reconciliation note

This card's integrator-addressed items are the seven numbered points, the
already-landed list and the two left items above.

This card introduces **six** residual names:
`RETURN-POSITION-ESCAPE-UNREACHABLE`,
`RETURN-POSITION-ACCUSING-UNEXERCISED-IN-CORPUS`, `RETURN-POSITION-SUBSHAPE-B`,
`RETURN-POSITION-TYPED-MEMBER-REACH`, `RETURN-POSITION-CONDITIONAL` and
`VERIFIER-HOST-CLOSED-INPUT-DEADLINE`. All six carry discharge conditions in
§10, and **all six already reach a tracker** — the first two and the last are
named in `AGENTS.md` (standing constraint 5b and the merged-gate row, so they
are reachable from the rule and the measurement they qualify, not only from this
report), and the other three were added to `named_followups`.
`scripts/reconciliation_sweep.py --residuals` reported three untracked on first
run and reports **zero** now. That check was run *because* `DOCS-COMPACT-4`'s and
`DOCS-COMPACT-5`'s own findings about this process sat untracked for two waves;
leaving the sweep's output for the close would have repeated it.

## 12. Reproduction

```sh
git submodule update --init --recursive          # FIRST: the pin gate fires at the END of a run
cargo build --release -p qcore-compiler --example cycle19_fluent_bit_libraries_audit
cp target/release/examples/cycle19_fluent_bit_libraries_audit \
   measure/c-gateway-return-position/bin/audit-after
sh   measure/c-gateway-return-position/run-audit.sh after  measure/c-gateway-return-position/bin/audit-after
python3 measure/c-gateway-return-position/byte_identity.py measure/c-gateway-return-position/runs/before
python3 measure/c-gateway-return-position/census.py --before measure/c-gateway-return-position/runs/before \
                                                    --after  measure/c-gateway-return-position/runs/after
python3 measure/c-gateway-return-position/spotcheck.py
sh       measure/c-gateway-return-position/mutations.sh
python3 measure/c-gateway-return-position/evidence.py
```

The BEFORE binary is the same build with `crates/qcore-c-gateway/src/legacy_c.rs`
and `neutral_position.rs` at the card's base commit `37bc9004`. Binaries:
BEFORE `989dfd4c3b4fd5c4…`, AFTER `dbb2bc2bd9bf64b3…`. Both runs exit 0. Each
takes about four and a half minutes.

The scripts are archived beside this report under `scripts/`; they read the
pinned tree at `/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin`
and write only to the worktree-private `measure/` directory (`CLAUDE.md` hygiene
rule — never the shared scratchpad, never under `target/`).

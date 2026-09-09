# BORROW-RESIDUALS — the four `BORROW-LIVENESS` residuals, taken in soundness order — 2026-09-09

Card BORROW-LIVENESS gave a `Borrowed` position a real liveness precondition
and closed its record with four named residuals
([§7](../remaining-borrow-liveness/README.md)); card HYGIENE-14 then widened the
status condition and left all four
[unchanged](../remaining-hygiene-14/README.md) (§8).

| Residual | Recorded as | Disposition here |
|---|---|---|
| **`BORROW-LIVENESS-PRESTATE`** | "the precondition is read in the call's pre-state, so a single call that releases one argument and borrows the same name at another position is not refused here" | **Implemented.** A second, separate decision refuses it. |
| **`BORROW-LIVENESS-PLACES`** | "the rule decides plain names only … widening this rule to those places would duplicate another card's channel" | **Split by measurement.** The composite half of the *new* decision is implemented; the path rule's plain-name bound is a **measured** reasoned refusal — every composite route it declines is already refused strictly harder. |
| **`BORROW-LIVENESS-OUT`** | "`Out` … needs its own analysis and its own card" | **Reasoned refusal, measured.** There is no liveness question at an `Out` position; six arms plus a mixed one, each decided by an existing channel. |
| **`BORROW-LIVENESS-ASSUMPTION`** | "measure whether it now could, or record why not" | **Reasoned refusal, measured in both directions.** It could not, on three independent structural grounds. |

Nothing here is promoted, no diagnostic code is added, changed or downgraded,
`AbstractState`/`join`/`alias_facts` are byte-identical to the base, and the
pinned 185-unit corpus's verdict surface is byte-identical — with a BEFORE/AFTER
control from one identical binary byte-identical on **all five** artifacts
(§6.4).

---

## 1. `BORROW-LIVENESS-PRESTATE` — implemented

### 1.1 The measured hole

Measured on this card's base `19fbaa5`, with C02-P's declared-contract
vocabulary and a callee declaring `resource_released` at one position and
`resource_borrowed` at another:

```c
void w(int n) {
    struct y_item *one;
    one = y_take(n); if (!one) { return; }
    y_retire(one, one);      /* released at 0, borrowed at 1 */
}
```

**`Safe`, no diagnostics.** So is the mirror parameter order
(`y_retire_late(one, one)`), so is the definite-alias form through two distinct
declared names (`two = one; y_retire(one, two);`), and so is the aggregate
member form (`y_retire(b->item, b->item)`).

The landed rule cannot reach any of them: it reads `incoming[node]`, the state
*entering* the call, where the identity is still `LIVE`. That is the standard
reading of a precondition, it is what keeps the rule independent of the
parameter loop's order, and this card **does not change it**. The question it
was never asking is a different one, and it lives entirely inside one call.

### 1.2 The decision, and why the fail-closed argument closes

[`borrow_liveness::borrow_destroyed_by_this_call`](../../../crates/qcore-c-gateway/src/borrow_liveness.rs)
answers `DestroyedByThisCall` under two conjuncts read from the same settled
pre-state:

1. the `Borrowed` actual has a **settled denotation** — exactly one target, not
   `unknown`, not `may_be_null` — through ALIAS-LOCALS's own
   `Denotation::settled`, shared so the three rules that read "the projection
   committed" cannot drift apart;
2. some **`Released`** position of the *same* call has an actual whose
   denotation settles to that **same identity**.

Both halves are *must* facts of exactly the kind the parent card requires: the
projection committed to one identity on every reaching path, at both positions.

The fail-closed argument is not "this is a violation" — it is that **the
contract vocabulary carries no ordering between a callee's own declared
effects.** `resource_released(dead)` says the callee destroys that object;
`resource_borrowed(seen)` says it reads this one; nothing anywhere says which
happens first, and §C04.2's borrowed mode explicitly declares no custody
precondition that could supply one. An unproven call effect is `Unknown`
(AGENTS.md hard invariant 1), so the projection declines to certify. The
published claim is the existing `UnknownAlias` — the same decision, on the same
three grounds, that card BORROW-LIVENESS §3 recorded for the path rule, and for
the same reason it is not the `Unsafe` class: neither reachability nor a
declared obligation to contradict is established.

### 1.3 The two bounds, each mirroring the landed status table

* **Only `Released` destroys.** `Consumed`, `Stored` and `Removed` move or
  detach custody without destroying it — §C04.2 says `resource_consumed` "is not
  a proof of destruction" — and `Out` writes a slot. This is exactly
  `custody_definitely_ended`'s treatment of the four status bits transposed onto
  the seven declared modes, and it is decided over the whole mode domain
  (`only_released_destroys`), not sampled. A consumed-and-borrowed pair keeps
  its existing `Safe` (`a_consumed_and_borrowed_pair_is_not_a_destruction`), so
  `C02-P-ALIAS-CONSUME` is not closed through this back door.
* **Settled denotations only**, so every scalar actual is untouched. That bound
  is load-bearing rather than decorative — see the census in §5.

### 1.4 What was considered and declined

A **syntactic** variant was designed and rejected: refuse when the two positions
carry the *identical canonical place string*, which is a must fact by C
semantics rather than by the projection's identity model and would therefore
reach unsettled bindings too. It was declined on measurement, not taste:

* no fixture shape reaches it that the settled-identity rule does not already
  reach — the unsettled cases measured here (`may_be_null` unrefined, ambiguous
  targets) already carry their own `unknown_alias` and `unsupported`
  diagnostics from the release side, so the answer does not move;
* it would lose the scalar guard that `settled()` supplies for free, and would
  then need a separate one to avoid firing on the one production shape that has
  this parameter pair at all (§5.1).

Recorded as **`BORROW-RESIDUALS-SYNTACTIC`**: available, sound, and measured to
buy nothing here.

---

## 2. `BORROW-LIVENESS-PLACES` — split by measurement

The residual reads "plain names only … composite places reached other ways are
uncovered". Measurement splits it into two halves with opposite answers.

### 2.1 The half that was a real hole: implemented

The same-call decision is deliberately **not** restricted to plain names. A
composite place is admitted on either side whenever the projection settled it,
because on this question no other channel speaks at all — measured:

| Shape | BASE | Now |
|---|---|---|
| `b->item = c; y_retire(b->item, b->item);` | **`Safe`, no diagnostics** | `Unknown`, `[unknown_alias:b->item]` |
| the same in the mirror parameter order | **`Safe`, no diagnostics** | `Unknown`, `[unknown_alias:b->item]` |

O04-READ-BOUND's `bound_read_resolves` requires the owner and the field link to
be `LIVE`, and at the call's pre-state they are — so its channel is correctly
silent, and nothing else asked.

### 2.2 The half that was never a hole: reasoned refusal, measured

The **path** rule keeps its plain-name bound. The residual assumed that left
composite places undecided; the measurement is that it leaves none. Exactly
three composite shapes can occupy `arguments[index]` at a `Borrowed` position,
and every one is already refused **strictly harder** than this rule would refuse
it:

| Route | What admits it | Its own requirement | Measured |
|---|---|---|---|
| `aggregate_reads` (O04-READ-BOUND) | `aggregate_store::bound_read_actuals` | owner resolves to one `LIVE` body-acquired identity **and** every linked child is `LIVE` with no unresolved effect | released child → `Unknown [unknown_alias:b]`; released owner → `Unsafe [leak, unknown_alias:b]` |
| `aggregate_addresses` (O04-B09B10) | `aggregate_store::member_address_actuals` | `resolved_aggregate_owner` → one `LIVE` body-acquired identity | released owner → `Unknown [unknown_alias:b]`, with a live control that is `Safe` |
| C02-P borrowed pointer-field actual | `borrowed_actual::borrowed_actual_spans` | the base is in `untracked_bases`, a name this function **can never bind to a tracked identity** | `Safe`, and structurally so |

`LIVE` is strictly stronger than `custody_definitely_ended`'s complement: the
path rule admits `ESCAPED` alone and `CONTAINED`, both of which O04 refuses. So
lifting the plain-name bound on the path rule could not add a refusal — it could
only add a second diagnostic under a different variable name, which is what the
parent card meant by "duplicate another card's channel", now measured rather
than argued.

**Everything else never becomes a `ContractCall` at all.** `contains_unmodeled_access`
runs over the tokens *outside* the admitted borrowed-actual spans, and any `->`,
`.`, `[` or `]` there returns `LegacyInstruction::Unknown` before the call is
built (`legacy_c.rs`, `unsupported_access`). Measured:
`y_probe(one->next)` after releasing `one` is `Unknown [unknown_alias:one]`.

One boundary recorded, not repaired: C02-P's `untracked_bases` closure is the
premise that makes row 3 vacuous rather than lucky. It is inherited unchanged
and unproved here, exactly as C02-P records it.

---

## 3. `BORROW-LIVENESS-OUT` — reasoned refusal, measured

The residual says an `Out` actual is "the address of a slot, not a resource
pointer, so 'the identity is released' is not even the right question there".
That was an assertion; here it is a measurement over every arm an `Out` actual
can take.

| Arm | Shape | BASE and now | Why no liveness question exists |
|---|---|---|---|
| dead slot | `y_release(one); y_fill(&one);` | `Safe` | **re-acquisition.** The slot's address is valid whatever identity it held; refusing would refuse the idiom card BORROW-LIVENESS already pins `Safe` (`a_reacquired_identity_is_admitted_again`). |
| dead slot, then read | `… y_fill(&one); y_probe(one);` | `Safe` | the identity read is the *fresh* one the `Out` position produced. |
| live slot | `y_fill(&one);` with `one` live | `Unsafe`, `[leak]` | the overwritten identity is reported by the existing `Unsafe` channel. An `Out` precondition would add nothing. |
| composite slot | `y_fill(&b->item);` | `Unknown`, `[unknown_alias:b, unsupported]` | the one shape where "the slot's address may be dangling" could be asked is refused by the lexical access gate before the call is built. |
| forwarded slot | `y_fill(out)` on this function's own out formal | `Safe` | the place is `*out`; `transfer` requires the analysis in force to sanction that exact slot and refuses otherwise. There is no identity to be live or dead. |
| null slot | `y_fill(0)` | `Unknown`, `[unsupported]` | a null actual names no place, so the declared effect has nowhere to land and the call refuses on its own terms. |

An `Out` position **produces** custody rather than requiring it. That is decided
in the model too rather than asserted: `an_out_position_never_destroys` puts
`out` in the seven-mode domain `only_released_destroys` decides.

A seventh arm is worth naming because it is the one place the two questions meet:
a `Borrowed` position beside an `Out` one still gets the path rule's answer
unchanged (`a_dead_borrow_beside_an_out_slot_keeps_its_refusal`), so the `Out`
position's presence neither adds nor removes a refusal.

**Residual re-scoped, not carried:** `BORROW-LIVENESS-OUT` is closed as a
liveness question. What remains genuinely open there is unrelated to liveness —
whether a *sanctioned* slot's address is valid — and that is
`W4`'s out-forwarding channel, which already validates the sanctioned slot at
every exit.

---

## 4. `BORROW-LIVENESS-ASSUMPTION` — reasoned refusal, measured in both directions

The residual records that `crates/qcore-verification/src/source_effect.rs`
publishes `assumptions_used.borrowed_parameter_is_live: true` on every
certificate it issues, and asks whether this rule could now discharge it.

**It could not, on three independent grounds, each measured.**

### 4.1 Which certificates carry it, and about what

Three, in the pinned corpus's `source-effects.json` — `cio_chunk_is_locked`,
`cio_chunk_unlock` and `flb_input_chunk_release_lock`. Their declared contracts
(`contract/qcore/chunkio/lock_state.qcore`) spell the premise as
`requires chunk_is_live` over an **entry** parameter,
`chunk: borrow<'call, …, Chunk>`. The subject is the object a caller supplied,
not anything the body passes onward.

### 4.2 Ground one — an entry parameter is structurally undecidable here

Both decisions require `state.pointers` to bind the actual. `EntryCustody`
binds no formal in this route, so an uncontracted entry parameter has no
denotation at all and takes the `an_unbound_name_is_never_refused` /
`an_unbound_borrowed_actual_is_never_refused` path at every call. That is the
same conjunct §C04.2's scalar admission relies on; it cannot be relaxed for one
subject.

### 4.3 Ground two — the reach is zero and structurally so

The two rules decide actuals at `ContractCall` nodes, which exist only for
callees the **resource** vocabulary loads. `cio_chunk_is_locked` and
`cio_chunk_unlock` are declared without a `requires resource_contract_v1`
marker — they are the Boolean-field vocabulary — so the resource loader rejects
them and `flb_input_chunk_release_lock`'s one call never becomes a
`ContractCall`. The three premise-carrying functions measure `Safe` / `Unknown` /
`Safe` in the resource projection before and after this card, with byte-identical
diagnostics.

### 4.4 Ground three — a refusal-only rule discharges nothing

Neither codomain contains a value that asserts liveness: `BorrowLiveness` is
`{DefinitelyNotLive, NotDecided}` and `SameCallBorrow` is
`{DestroyedByThisCall, NotDecided}`. "Not decided" is the absence of a claim,
never evidence for one. Proved in the model as
`no_decision_discharges_a_premise` (neither transformer touches a published
assumption set on any input) and `every_outcome_either_refuses_or_changes_nothing`
(every outcome either leaves the verdict alone or withdraws a certificate).

The naming coincidence therefore stays a coincidence, and the residual is
recorded closed as a **question answered**, not as work deferred. Discharging
`borrowed_parameter_is_live` needs caller-side entry custody — a different
mechanism, and one this card does not begin.

---

## 5. Census — every firing counted

### 5.1 The declaration census: reach is zero, and structurally

The same-call rule can only fire where a **single declaration** carries both a
`Released` and a `Borrowed` parameter. Over all 64 contract modules:

| Figure | Count |
|---|---|
| `resource_contract_v1` declarations in `contract/qcore/` | **25** |
| …carrying a `Released` **and** a `Borrowed` parameter | **1** — `cio_chunk_close(chunk, delete)` |
| …carrying a `Consumed` and a `Borrowed` parameter | **0** |

And that one declaration cannot fire: its borrowed parameter `delete` is a
scalar flag, which §C04.2 admits in exactly that position and which the settled
guard excludes at every call site. The shape is pinned as an acceptance test
(`a_scalar_at_the_borrowed_position_is_untouched`) and as a module test
(`a_scalar_borrowed_actual_is_not_decided`), so a future widening that reached
it has to move both.

### 5.2 The run census

A null result is worth only its explanation, so the reach was **measured**. A
third instrumented run of the same pinned route emitted one scratch stderr line
per `Borrowed` position either rule considered, recording whether the actual was
bound, whether its denotation settled, how many `Released` positions the call
declared, how many of those settled, and whether each rule fired. The
instrumentation and the control switch of §6.4 were removed before this card's
commits, no production route emits anything, and the instrumented run's own
artifacts are **byte-identical to the clean run's on all five files**, so the
whole apparatus is proved inert. Aggregate:
[`artifacts/census.txt`](artifacts/census.txt); raw stream:
[`logs/census.raw.gz`](logs/census.raw.gz).

| Figure, over all 185 units | Count |
|---|---|
| `Borrowed` positions the two rules considered | **353**, across **108** distinct functions |
| of those, **unbound** — every scalar actual, and every name this projection never tracks | **319** |
| of those, **bound** to something | **34** |
| of the 34, **settling** on a single non-null identity | **0** |
| calls declaring a `Released` position **at all** beside a `Borrowed` one | **0** |
| path-rule refusals published | **0** |
| same-call refusals published | **0** |

Two things are worth reading off that table rather than skipping.

**The same-call rule's blocking conjunct is the *declaration*, not the state.**
Not one `Borrowed` position in the whole corpus sits in a call that also
declares a `Released` position, so the second conjunct is never even evaluated.
That is the run-time confirmation of §5.1's declaration census, reached
independently: `cio_chunk_close` is the only declaration with the shape, and no
analyzed statement puts a *bound* borrowed actual beside its released one.

**The path rule reproduces its parent card's zero, and the difference in the
denominator is itself informative.** Card BORROW-LIVENESS measured 267
considerations across 100 functions; this instrumentation sits *before* the
plain-name filter rather than after it, so it also counts the composite places
that card's `BORROW-LIVENESS-PLACES` bound skipped — 86 further considerations
in 8 further functions. The 34 bound considerations are the same 34, and they
still all fail to settle.

So this card, like its parent, **closes a real and demonstrated hole (§1.1,
§2.1) and costs the corpus nothing measurable.** That is evidence about these
185 units under this projection's small committed-binding surface, not evidence
that the shape is rare in C.

### 5.3 The one measured firing in the whole tree

`crates/qcore-c-gateway/tests/prod_contracts.rs`'s adversarial row 6 substitutes
`resource_borrowed(mask)` → `resource_released(mask, nonnull)` on `w_mask` and
analyses `w_mask(h, tag, 3, h)` — which passes `h` twice, so the **deliberately
unfaithful** declaration describes a call that releases and borrows one
identity. The mutated run moved `Safe` → `Unknown`.

**The pin was updated deliberately and both load-bearing halves kept**: the test
still asserts that the faithful mode leaks (1 leak) and that the substituted one
**discharges** the obligation (0 leaks), which is the property it exists to
protect. Two assertions were added rather than removed — that the faithful
declaration is out of this card's reach entirely, and that the substituted one
now names `h`. What moved is the published class of a declaration the card
itself calls unfaithful, and it moved in the refusing direction.

---

## 6. Corpus — the verdict surface is byte-identical

### 6.1 What was run

The canonical pinned route on the shared prepared pinned clone (Fluent Bit
`66910c10a4d7eafa810b84229cf2dcf7b0f26f97`, explicit `/usr/bin/clang` 21.1.8),
following [`docs/reports/corpus-repin/provenance.json`](../corpus-repin/provenance.json):

```sh
python3 -B providers/package.py build --output build/cycle19/provider-packages-residuals
QCORE_CLANG=/usr/bin/clang \
QCORE_CHUNKIO_PACKAGE=build/cycle19/provider-packages-residuals/qcore.chunkio \
  ./target/debug/examples/cycle19_fluent_bit_libraries_audit \
  --pinned-evidence <clone> <clone>/../monkey/monkey-build/compile_commands.json \
  <out>/monkey-source.json <out>/resource-diagnostics.json
```

Exit 0, **zero gateway failures**. Transcript:
[`logs/corpus-after.log.gz`](logs/corpus-after.log.gz).

### 6.2 The headline, and the three artifacts that are byte-identical

**185 units: 2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only —
unchanged**, per library: cmetrics 0/16/15/0, ctraces 1/2/13/1, librdkafka
0/10/81/3, chunkio 0/2/11/0, fluent-bit-chunk 0/1/0/0, monkey-libevent 1/11/17/0.

Against the current baseline
[`docs/reports/wave14-corpus/artifacts/`](../wave14-corpus/artifacts), measured
directly by SHA-256 rather than through a diff tool:

| Artifact | Result | Bytes |
|---|---|---|
| `resource-diagnostics.json` | **byte-identical** (`40691eab1fdd7b34…`) | 11,140,129 |
| `monkey-source.json` | **byte-identical** (`53d55b7e1fb174de…`) | 1,664,550 |
| `resource-diagnostics.chunk-contracts.json` | **byte-identical** (`c4f438e4ed3dab3c…`) | 4,186,178 |
| `resource-diagnostics.contracts.json` | differs — **path keys only** | 30,530 (was 30,152) |
| `resource-diagnostics.source-effects.json` | differs — **identity digests only** | 118,505 (unchanged) |

`diagnostics.json` is the whole verdict and diagnostic surface, and it is
**byte-identical**. That is the direct measurement that neither new refusal
fires anywhere in the pinned corpus, and it also confirms
`C02-P-ALIAS-LEAK-ORDER` is undisturbed: the leak inventory is inside that file.

### 6.3 Both differences attributed, exhaustively

**`contracts.json` — location provenance, to the byte.** An exhaustive key-wise
diff shows the *only* change is the absolute-path prefix of the nine `sources`
keys: this run's working directory is a delegated worktree, the baseline's was
the main checkout. Every contract body is character-identical.
`/.claude/worktrees/agent-aba9025d2957c0c7a` is 42 characters and 9 × 42 = **378**,
exactly the byte growth. `contract/qcore/` is untouched by this card and the
corpus stays 64 modules. This is the same deviation
[`corpus-repin/provenance.json`](../corpus-repin/provenance.json) already
records: "location provenance, not analysis".

**`source-effects.json` — identity digests on the three premise rows only.** The
exhaustive diff moves exactly `package_digest`, `build_digest`,
`configuration_digest`, `obligation_id`, `bounds` key and `task_digest` on
`results[16]`, `[24]` and `[63]` — `cio_chunk_is_locked`, `cio_chunk_unlock` and
`flb_input_chunk_release_lock`. **No status, no verdict, no assumption, no
coverage count and no path moves**, and the file's length is unchanged, which is
what a pure digest substitution looks like. Every one of those fields binds the
compile-task identity, the contract-source inventory (whose keys carry the same
worktree prefix) or the selected package path — all of which this run's working
directory and its freshly built `provider-packages-residuals` directory move,
and none of which this card's diff can reach.

### 6.4 The control that turns that argument into a measurement

The attribution above is a reading of two diffs. The decisive form is a
**BEFORE** run of the same route from the same working directory, and it was
run: a temporary runtime switch (`QCORE_BORROW_RESIDUALS_OFF`) disabled the
same-call refusal, so **one identical binary** produced both sides with nothing
else in the build, the cwd, the clone, the compile database or the selected
package able to differ. Both exit 0 with zero gateway failures. The switch and
the census instrumentation were removed before this card's commits.

| Artifact | BEFORE vs AFTER |
|---|---|
| `resource-diagnostics.json` | **byte-identical** |
| `monkey-source.json` | **byte-identical** |
| `resource-diagnostics.chunk-contracts.json` | **byte-identical** |
| `resource-diagnostics.contracts.json` | **byte-identical** |
| `resource-diagnostics.source-effects.json` | **byte-identical** |

All five. So neither §6.3 difference is attributable to this card: they are
present with the rule off and with the rule on, unchanged. And the same
apparatus proves itself inert — the instrumented, switch-carrying binary's
AFTER artifacts are byte-identical to the clean binary's, on all five files.

Transcripts: [`logs/corpus-after.log.gz`](logs/corpus-after.log.gz) (clean
binary), [`logs/corpus-census.log.gz`](logs/corpus-census.log.gz) (instrumented,
rule on), [`logs/corpus-before.log.gz`](logs/corpus-before.log.gz)
(instrumented, rule off).

### 6.5 This card does not move the pin

`docs/reports/wave14-corpus/artifacts/` remains the current 185-unit baseline
and no AFTER set is re-archived: the verdict surface is byte-identical to it,
and the two artifacts that differ do so only by this worktree's own path and the
digests derived from it. Re-archiving would pin a location, not a measurement.

---

## 7. Adversarial evidence

`crates/qcore-c-gateway/tests/borrow_residuals_boundary.rs` — **23** held-out
acceptance tests over a `y_*` API deliberately unrelated to ChunkIO, Fluent Bit
and to the `b_*` (C02-P), `g_*` (O04), `h_*` (O04-ARMS), `k_*` (O04-B08) and
`r_*` (O04-READ-BOUND) families. Every body was measured at the base with the
extension absent; each test's `BASE:` line records that answer. Twenty-two of
the twenty-three assert the **complete** diagnostic vector, so a rule that
swallowed a pre-existing leak or double release while adding its own refusal
would fail; the exception is
`a_member_address_through_a_released_owner_keeps_o04_s_refusal`, whose fixture
also carries an independent leak and which therefore asserts non-certification
plus O04's own refusal by name.

**Five measured movements**, all `Safe` → `Unknown`: the plain same-name pair in
both parameter orders, the definite-alias pair through two distinct names, and
the aggregate bound read in both parameter orders.

**Eighteen measured preservations**: the production scalar shape, the consumed
pair, distinct identities, the may-alias pair of two uncontracted formals
(C02-P-ALIAS's channel, untouched), C02-P's untracked-base field read, the
unadmitted member access, O04-READ-BOUND's released-child and released-owner
refusals, O04-B09B10's released-owner refusal with its live control, all six
`Out` arms plus the borrow-beside-`Out` arm, and the later borrow of a retired
identity, which the *path* rule already refused and still does.

`crates/qcore-c-gateway/src/borrow_liveness.rs` — **26** module tests
(10 new), covering the positive direction, the definite-alias form,
order-independence, the distinct-identity and no-destroying-position controls,
the scalar guard, all four ways either side fails to settle, and the
independence of the two decisions.

---

## 8. Lean inventory

[`proof/QCore/BorrowLiveness.lean`](../../../proof/QCore/BorrowLiveness.lean),
namespace `QCore.BorrowLiveness`, audited from the **existing**
[`proof/AuditBorrowLiveness.lean`](../../../proof/AuditBorrowLiveness.lean):
**37 → 58 audited theorems, 21 new, no new axiom, no placeholder, no `sorry`**.
Every one depends only on `propext` and `Quot.sound`; two depend on nothing.

No existing theorem was retired, restated or weakened. `liveness`,
`custodyDefinitelyEnded`, `Status`, `Denotation` and every proof above the new
section are byte-identical, so cards BORROW-LIVENESS and HYGIENE-14 keep their
records intact. The new decision is a **separate** definition with a **separate**
result type (`SameCall`), mirroring the module's standing rule that separate
relations stay separate.

* **the positive direction** — `a_call_that_destroys_the_identity_it_reads_is_refused`,
  `two_positions_settled_to_one_identity_are_refused`,
  `the_decision_does_not_depend_on_position_order`,
  `the_decision_reads_only_the_destroyed_identities` (over every pair of calls
  with the same destroyed set), `a_same_call_refusal_withdraws_a_certificate`;
* **the mode table, decided over its whole domain** — `only_released_destroys`,
  `a_consumption_in_the_same_call_is_not_a_destruction`,
  `an_out_position_never_destroys` (the `-OUT` residual, decided rather than
  asserted), `a_removed_or_stored_position_never_destroys`;
* **every way it fails closed** — `an_unbound_borrowed_actual_is_never_refused`,
  `an_unsettled_borrowed_actual_is_never_refused`,
  `an_unsettled_destroying_actual_refuses_nothing`,
  `a_distinct_identity_in_the_same_call_is_never_refused`,
  `a_call_that_destroys_nothing_is_never_refused` (over every input);
* **the converse and the separation** —
  `a_same_call_refusal_names_the_identity_it_claims`,
  `the_same_call_rule_reads_no_status` (an identity the path rule *admits* is
  refused when the call destroys it, so the two decisions are demonstrably
  independent);
* **the published class** — `the_same_call_rule_never_certifies`,
  `a_same_call_refusal_is_never_the_violation_class`,
  `an_existing_violation_survives_the_same_call_rule`;
* **`BORROW-LIVENESS-ASSUMPTION`** — `no_decision_discharges_a_premise`,
  `every_outcome_either_refuses_or_changes_nothing`.

The model is a model of the **decision**. That the Rust reader extracts the
bindings, the release events and the contract modes faithfully from C source is
a premise, not a theorem, exactly as in `QCore.MayAlias`,
`QCore.BorrowedActual` and the parent card.

**For the integrator:** this card adds **no** audit file, so
`scripts/check_lean_audits.py` stays at **64** audits and the acceptance
matrix's derived `validation.lean_audits` row does **not** move. No matrix
regeneration is required by this card.

---

## 9. Files changed

| Path | Change |
|---|---|
| `crates/qcore-c-gateway/src/borrow_liveness.rs` | `SameCallBorrow`, `SameCallSite`, `borrow_destroyed_by_this_call`, the module-header derivation, and 10 new module tests. `BorrowLiveness`, `BorrowSite`, `custody_definitely_ended`, `borrowed_actual_liveness` and every existing test are **byte-identical to the base** |
| `crates/qcore-c-gateway/src/legacy_c.rs` | `require_live_borrowed_actuals` resolves this call's destroying positions once and consults both decisions; one new private helper `denote_pointer`. `AbstractState`, `AbstractState::join`, `alias_facts`, `NodeAliasFacts`, `transfer`, `report_leaks` and `apply_resource_action` are **byte-identical to the base** |
| `crates/qcore-c-gateway/tests/borrow_residuals_boundary.rs` | **new** — 23 held-out acceptance tests |
| `crates/qcore-c-gateway/tests/fixtures/contracts/borrow_residuals.qcore` | **new** — the `y_*` declarations, including the `cio_chunk_close` production shape and a consuming counterpart |
| `crates/qcore-c-gateway/tests/prod_contracts.rs` | adversarial row 6's pin updated deliberately (§5.3); both load-bearing halves kept and two assertions added |
| `proof/QCore/BorrowLiveness.lean`, `proof/AuditBorrowLiveness.lean` | 21 new audited theorems in a new section; nothing above it changed |
| `docs/reports/remaining-borrow-residuals/` | this record, the census and the run transcripts |

`crates/qcore-c-gateway/src/may_alias.rs`,
`crates/qcore-c-gateway/src/borrowed_actual.rs`,
`crates/qcore-c-gateway/src/aggregate_store.rs` and
`crates/qcore-c-gateway/src/typed_field_access.rs` are **untouched**, so
ALIAS-LOCALS's relation, O04-READ-BOUND's `untracked_bases` premise,
O04-B09B10's endpoint identities and C02's whitelist are unchanged by
construction. `contract/qcore/` is untouched, so the corpus stays **64**
modules.

---

## 10. Gate results

Branch `worktree-agent-aba9025d2957c0c7a` in an isolated worktree on base
`19fbaa5` (the wave-14 consolidation), submodules initialised.

| Gate | Command | Result | Exit |
|---|---|---|---|
| Workspace tests | `cargo test --workspace --no-fail-fast` | **2,567 passed / 0 failed / 8 ignored** | 0 |
| Format | `cargo fmt --all -- --check` | clean | 0 |
| Clippy | `cargo clippy --all-targets -- -D warnings` | clean | 0 |
| Probe | `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 | 0 |
| Follow-up | `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 | 0 |
| Provider tests | `cargo test --locked --offline --manifest-path providers/Cargo.toml --workspace` | **102 passed / 0 failed** | 0 |
| Provider packages | `providers/package.py build` then `verify` on all five | **5/5 built and verified** | 0 |
| Lean audits | `python3 scripts/check_lean_audits.py` | **64 audits passed**, `AuditBorrowLiveness.lean` 37 → **58** theorems | 0 |
| Python tests | `python3 -m unittest discover -s scripts/tests` | **135 run, 0 failed** | 0 |
| Pinned 185-unit corpus | canonical pinned route, §6 | re-measured; verdict surface byte-identical, BEFORE/AFTER byte-identical on all five | 0 |

This card adds **33 test functions** — 10 module tests in `borrow_liveness.rs`
(16 → 26) and 23 held-out acceptance tests in `borrow_residuals_boundary.rs` —
and removes none. Two further assertions were added inside an existing
`prod_contracts.rs` test, which is still one test.

**Nothing is left failing for the integrator.**
`test_the_derived_facts_are_recomputed_rather_than_carried` **passes**: this
card adds no Lean audit *file*, so `validation.lean_audits` stays at 64 and the
acceptance matrix needs no regeneration.

### 10.1 One load-flaky observation, measured rather than waved away

An early full-workspace run — deliberately started while a 185-unit corpus audit
was saturating this host — recorded `-p qcore-verifier-host --test
standalone_host` exiting non-zero **with 3 passed / 0 failed**, i.e. a
harness-level exit with no failing assertion, in a target that spawns and
process-group-kills child processes. The authoritative run recorded in the table
above, on the committed tree with the host idle, records **0 failed** for that
target and for the workspace. This card's diff cannot reach that crate. **No
assertion was weakened.** This is the same class as the recorded P02
`RLIMIT_NPROC` and P03 confined-write flakes and is offered as a third
observation of it, not as a new finding.

### 10.2 One incidental repair, recorded

`cargo clippy` flagged `clippy::manual_contains` at
`crates/qcore-c-gateway/tests/prod_contracts.rs:738` on an expression that is
**byte-identical to the base** (`git show 19fbaa5:` confirms it verbatim). The
lint had been cache-masked in earlier gates and surfaced because this card's
gateway edit invalidated that target's cached check. It was rewritten to
clippy's own suggestion; the assertion, its operand and its message are
unchanged. Recorded for the integrator because the same lint will surface in any
other card that touches `qcore-c-gateway`.

---

## 11. Open boundaries

Each is a *retained* pre-existing answer or a measured bound, never a new
admission.

* **`BORROW-RESIDUALS-SYNTACTIC`** (new) — the place-string variant of the
  same-call rule (§1.4): sound, available, and measured to reach nothing the
  settled-identity rule does not. Taking it would need its own scalar guard.
* **`BORROW-RESIDUALS-SAME-CALL-MAY`** (new, cross-card) — two *distinct*
  declared names at the released and borrowed positions of one call, which the
  projection has **not** settled to one identity, are not decided
  (`a_may_alias_pair_keeps_exactly_its_existing_answer` pins the byte-identical
  answer). That is a *may*-alias question and it belongs to C02-P-ALIAS's
  channel, whose `alias_facts` are a forward fact over already-executed
  releases and therefore do not see a release at the node itself. Recorded, not
  repaired: touching it would move `alias_facts`, which this card holds
  byte-identical.
* **`BORROW-LIVENESS-PRESTATE`** — **closed** for the definite case; the
  pre-state reading of the *path* rule is unchanged and deliberate.
* **`BORROW-LIVENESS-PLACES`** — **closed** for the same-call decision;
  re-scoped for the path rule, where it is measured to leave nothing undecided
  (§2.2) and where widening would only duplicate O04's diagnostics.
* **`BORROW-LIVENESS-OUT`** — **closed as a liveness question** (§3).
* **`BORROW-LIVENESS-ASSUMPTION`** — **closed as a question answered** (§4).
  Discharging the premise needs caller-side entry custody, which is a different
  mechanism.
* **`BORROW-LIVENESS-MIXED`** — unchanged. HYGIENE-14's status-set condition is
  untouched, and the same-call rule mirrors it on the mode side rather than
  reinterpreting it.
* **`C02-P-ALIAS-CONSUME`** — unchanged and explicitly not closed through the
  back door, in the Rust (`a_consumed_and_borrowed_pair_is_not_a_destruction`)
  and in the Lean (`a_consumption_in_the_same_call_is_not_a_destruction`).
* **`C02-P-ALIAS-LEAK-ORDER`** — unchanged and undisturbed. Both decisions are
  read-only over the settled incoming states, after the work list is stable.
* **`ALIAS-LOCALS-EXIT-CUSTODY`** — unchanged. The refusal does not set
  `state.unmodeled`, exactly as the parent card's does not; a read changes no
  site's disposition. The per-exit channel is O04-B09B10's and this card
  neither widened nor narrowed it.
* **`C02-P-ALIAS-USES`** — unchanged. A position is decided only where the
  projection models a contracted call at all.
* **Reachability is not established**, which is why the published class is
  `Unknown`. No claim is made that any refused body contains a defect, and no
  claim at all is made about upstream code: every movement here is on synthetic
  fixtures.
* **The declared premises are inherited unchanged and unproved** — declaration
  completeness, C effective types, and C02-P's `untracked_bases` closure.
* **`docs/reviews/r0_3/acceptance_matrix.json`, `acceptance.json`,
  `ct_manifest.json`, `AGENTS.md`, `CLAUDE.md`, `README.md` and
  `DEVELOPMENT.md` were deliberately not touched** — the integrator
  consolidates those.

---

## 12. Scope and non-claims

* **No raw unit is promoted**, and no rule here can promote one: both decisions
  are `Unknown`-class and refusal-only, proved in the model.
* `Safe` in the `y_*` family is `Safe` for a synthetic declared custody
  projection over trusted test assumptions. The fixtures are declared test
  assumptions, not facts about any library.
* The §1.1 false `Safe` is a defect of **this projection**, demonstrated on a
  synthetic fixture. It is not a defect allegation about any upstream code, and
  §5.1 measures that the pinned corpus does not contain the shape.
* This is not a use-after-free analysis for C. It refuses one further shape the
  projection had already decided and had been failing to ask about.
* Projection coverage is not safety coverage; unknowns stay in the denominator.

---

## 13. Parallel cards in the same crate

NEIGHBOURS runs in parallel touching contract text only. Recorded overlap,
**not resolved here**:

* `crates/qcore-c-gateway/src/legacy_c.rs` — this card's edits are two localised
  sites, both inside `require_live_borrowed_actuals` and one new free function
  placed immediately before `classify_diagnostics`. No instruction, state field
  or CFG shape is added, so a textual conflict is unlikely outside those
  anchors. Card BORROW-LIVENESS recorded the same two anchors.
* `contract/qcore/` is **untouched** by this card, and its pinned per-file
  counts are unchanged, so NEIGHBOURS' contract-text edits and this card's
  gateway edits are disjoint by file. If NEIGHBOURS adds a declaration carrying
  both a `Released` and a `Borrowed` parameter, §5.1's census figure (1 of 25)
  moves and the same-call rule's corpus reach must be re-measured — recorded
  here for the integrator rather than pre-empted.

# O04-TRACE — the variadic tail and one level of nesting — 2026-09-09

The named continuation of card O04-B09B10
([`docs/reports/remaining-o04-b09b10/README.md`](../remaining-o04-b09b10/README.md)
§9), whose follow-on **`O04-B09B10-TRACE`** reads: *"C6's whole residual is one
statement: an uncontracted **variadic** callee whose second argument is a
**nested call**. It needs a declared variadic-tail form (O04-ARMS-VARIADIC)
*and* nested-call modeling, and §3.2 measures that nothing else stands between
C6 and a settled exit."*

Both halves are built, and they compose.

## Headline

**C6 discharges. `flb_input_chunk_create` reaches function-level `Safe` on the
byte-verbatim upstream body — all six exits `Discharged`, no diagnostic of any
kind.** That is a movement, and §3 states every premise it rests on. Card
O04-B09B10's labelled control predicted exactly this shape; it is now measured
on the verbatim source instead of on a substitution.

| Arm | O04-ARMS | O04-B08 | O04-READ-BOUND | O04-B09B10 | **This card** |
|---|---|---|---|---|---|
| **C1** — logging arm, `!chunk` | Discharged | Discharged | Discharged | Discharged | **Discharged** |
| **C2** — `up_force == -1` | Discharged | Discharged | Discharged | Discharged | **Discharged** |
| **C3** — header write `== -1` | Discharged | Discharged | Discharged | Discharged | **Discharged** |
| **C4** — `ic` calloc NULL | Discharged | Discharged | Discharged | Discharged | **Discharged** |
| **C5** — routes-mask arm | `Unknown` | Discharged | Discharged | Discharged | **Discharged** |
| **C6** — success + publication | `Unknown` (10) | `Unknown` (10) | `Unknown` (9) | `Unknown` (**1**) | **Discharged (0)** |

**Nothing is promoted.** The 185-unit headline is **2 Safe / 42 provisional
Unsafe / 137 Unknown / 4 data-only**, unchanged, on artifacts that are
**byte-identical before and after** (§5) — the whole-unit result for
`fluent-bit-chunk/flb_input_chunk.c` is unaffected because the pinned corpus
does not carry these two declared callee premises, and because
`cio_chunk_open` in the same fixture unit keeps its own refusals, the
fixture's *program* result stays `Unknown` too. The contract corpus stays at
**60 modules**, CT coverage stays **31/50**, and `qcore-schemas` is unchanged
at **9.0.0**.

---

## 1. What the card had to build, and why neither half was enough alone

Row 0 of card O04-B09B10's census, the last statement of C6:

```c
flb_trace("[input chunk] no matching route for input chunk '%s' with tag '%s'",
          flb_input_chunk_get_name(ic), tag);
```

| Obstacle | Recorded as | What it needed |
|---|---|---|
| `flb_trace` is `void flb_trace(const char *, ...)` and the resource-contract vocabulary had no variadic form. Card O04-ARMS declared `flb_error` at its single call site's arity instead | **`O04-ARMS-VARIADIC`** | a **declared variadic tail** in the C04 vocabulary |
| the second argument is a **call**, whose acquisitions, releases and transfers card O04-B09B10 found were being dropped **without a diagnostic** — and which it repaired into an explicit refusal | **`O04-B09B10-NESTED`** | **nested-call modeling**, converting that refusal into a bounded admission |

They are not variations of each other, and neither settles the statement
alone. §8's real-source substitution measures the coupling in both
directions: with `flb_trace` undeclared the statement is
`call to uncontracted function`; with `flb_trace` declared **without** its
tail the statement is `nested call … inside a modeled call` — because the
nested result then occupies a position the nested rule cannot call a data
position; with the tail but the accessor uncontracted it is
`nested call flb_input_chunk_get_name …` again.

---

## 2. Half one — `resource_variadic_data_tail()` in the C04 vocabulary

`ensures normal resource_variadic_data_tail();` is a new **whole-declaration**
clause in `qcore-contract`'s total-or-reject loader
([`contract_loader.rs`](../../../crates/qcore-contract/src/contract_loader.rs)).

### 2.1 The trust statement, stated honestly

**It declares** that the callee takes **no custody of**, and **retains no
pointer to**, any argument beyond the ones the declaration names.

**It does not verify the callee.** It is a trusted assumption of exactly the
same kind as `resource_borrowed`; nothing reads `flb_trace`'s body. It says
nothing about format strings, nothing about the tail being type-correct, and
nothing about the tail being read at all.

**It admits nothing on its own.** The gateway still requires, *per call site*,
that every tail actual really is data (§3.2, premises T1-T4). An actual
carrying a tracked resource identity is refused **under** the declaration
exactly as it is without one — the declaration covers what it covers. That is
proved (`the_declaration_alone_admits_nothing`,
`a_tail_actual_that_may_hold_an_identity_is_never_admitted`) and tested at
both the unit and the integration level.

### 2.2 Where it is recorded, and why not on the contract

The flag is **not** carried on `ResourceFunctionContract`: this card makes no
`qcore-schemas` change, so `qcore-schemas` stays 9.0.0 and every existing wire
byte and task digest is untouched. It is rederived from the environment's own
**retained trusted source texts** by
`contract_loader::declared_variadic_data_tails`.

Two fail-closed properties hold by construction, and both are pinned:

* an environment with **no declared sources** — an inferred W4 summary, a
  provider proposal, a directly constructed table — has **no tails**. A tail is
  a declaration, never an inference.
* a source set the loader would reject yields the **empty** set, never a
  partial one.

It also means the tail is rederived by W4 summary derivation, by W4 validation
and by `check_legacy_safety_certificate` from the certificate's own retained
bytes, exactly as the C02/C02-P/B08 declaration facts are. §7 pins that
direction: withdrawing the clause from a certificate's retained sources —
leaving the loaded contract table **byte-identical**, since the tail is not on
a contract — makes the independent recheck refuse.

### 2.3 Total-or-reject

The clause takes no argument (a tail has no index to point at) and may occur
at most once. `resource_variadic_data_tail(format)`, a duplicate clause, the
clause written as a `requires`, and the clause on an error exit each reject the
whole module, with no partial environment. It does not replace a parameter
mode: every named parameter still needs one.

### 2.4 The placement decision, weighed and recorded

The **vocabulary** is production: the loader clause ships. The **declaration
that `flb_trace` has a data tail** is a premise about `flb_trace`, and it lives
with the gateway test fixtures, in
[`o04_trace_callees.qcore`](../../../crates/qcore-c-gateway/tests/fixtures/contracts/o04_trace_callees.qcore),
following card O04-B09B10's precedent for exactly the same class of premise.

**So the pinned `contract/qcore` corpus count stays at 60 modules and no
clause-count pin moves.** That is deliberate, and the alternative was weighed:

| Put the declaration in… | Consequence |
|---|---|
| `contract/qcore/` | the corpus module count moves 60 → 61 and its clause-count pin gains a row (**both would have to be stated loudly**); *and* the declaration would apply to **every** `flb_trace` call site in the 185-unit corpus at once — hundreds of statements whose tail actuals this card has not measured. The bounded claim does not need it |
| gateway test fixtures (**chosen**) | corpus count untouched, no pin moves, the premise sits beside the measurement it supports, and the measured production reach of the tail vocabulary is **zero admitted statements** (§5.3, `O04-TRACE-REACH`) |

The honest consequence of the choice is recorded rather than hidden: the
payoff is on the fixture, and the production corpus gains **nothing** from
this card. Widening it is contract-text work, not analyzer work — the same
shape cards C03-NFR, O04-READ-BOUND and O04-B09B10 each recorded.

---

## 3. Half two — bounded nested-call modeling

[`crates/qcore-c-gateway/src/variadic_nested.rs`](../../../crates/qcore-c-gateway/src/variadic_nested.rs),
consumed by `legacy_c::classify_simple` and `legacy_c::transfer`.

### 3.1 The eight conjunctive premises

| # | Premise | Refused otherwise |
|---|---|---|
| **N0** | readable declarations (`FunctionTypes::is_readable`) | the whole function admits nothing |
| **N1** | the **enclosing** callee has a contract in force | an outer call this analyzer does not model already refuses on its own |
| **N2** | **exactly one** call other than the enclosing callee | two nested calls in sibling arguments, and two-level nesting `f(g(h(x)))`, both leave more than one |
| **N3** | the nested callee has a contract in force | an uncontracted inner call — nothing models its effects, which is why the statement was refused in the first place |
| **N4** | the nested call is a **whole argument** of the enclosing call: its callee is the argument's first token, its `)` the last | a cast, parentheses, arithmetic, a comparison, any surrounding expression |
| **N5** | the nested call's own argument list carries no call | (implied by N2; asserted locally so the depth bound stays explicit at one) |
| **N6** | the nested contract's **result mode is `None`** | `Owned`, `Reallocated` |
| **N7** | **every** parameter mode of the nested contract is `Borrowed` | `Owned`, `Consumed`, `Released`, `Stored`, `Removed`, `Out` |
| **N8** | the slot the nested result occupies needs no identity: a declared `Borrowed` parameter, **or** a position covered by a declared variadic data tail | every custody-bearing parameter; and an excess position with **no** declared tail |
| — | the nested call's actual count equals its declared parameter count | a nested arity mismatch is refused rather than zipped |

**Why N6 and N7 are the whole rule.** C leaves the evaluation order of a
call's arguments unspecified (C11 6.5.2.2p10). A nested call with any custody
effect would have to be ordered against the enclosing call's other arguments,
and this projection has no such order to give. The admission avoids the
question rather than answering it: under N6 and N7 the nested call's declared
effects are the **identity** on the abstract state — no disposition changes, no
liveness is demanded, nothing is produced — so inner-then-outer and
outer-then-inner both equal the enclosing call alone. Both directions are
proved in Lean (§6), *and* so is the converse: an owned result, a releasing
parameter and an owned parameter are each exhibited as **not** neutral, so the
premises are load-bearing rather than decorative.

**"Data-only or a borrowed handback" is one premise, by measurement.** The v1
result vocabulary has exactly **one** non-custody result mode,
`resource_return_none()`, so both phrasings of the card brief resolve to N6.
`flb_input_chunk_get_name` really is the second case: it hands back `ic->name`,
which the caller must not free. This projection does not track the handback at
all, so it also does **not** model that the handback aliases into `ic` — which
is precisely why N8 confines the result to a position needing no identity and
why it is never bound to a name. A body that stored the handback is a different
statement shape, and refused.

**Composition, in one code path.** An admitted nested call travels on
`LegacyInstruction::ContractCall` as `nested: Option<Box<NestedContractCall>>`
and is applied by `transfer` through the *same* `apply_contract_call` every
other modeled call goes through, before the enclosing call's own effects. The
may-alias obligations of its reads (C02-P-ALIAS) travel with the enclosing
call's in `alias_reads`, so `ic` is still subject to the alias gate at that
statement.

Everything else keeps card O04-B09B10's refusal, **verbatim** — the diagnostic
text is unchanged, and the test asserts the exact string.

### 3.2 The tail actuals — premises T1-T4

A tail actual is data when, conjunctively:

* **T1** — it is nonempty;
* **T2** — it carries no `&`, `*`, `[`, `]`, `.` and no `->`. No address
  escapes, no dereference, no subscript, no member access: each denotes storage
  this projection has no model for, and the first is exactly how a caller hands
  a callee a handle it could keep;
* **T3** — it carries no call. The one admitted nested call is decided by its
  own premises and skipped here; every other call refuses;
* **T4** — every identifier is an enumeration constant, a declared scalar
  variable, or a name **C02-P's closure says this body can never bind to a
  tracked resource identity**. String and character literals reach the walker as
  the tokenizer's single literal token, which is never bound, so they are
  admitted by the same clause rather than by a special case.

T4 is what makes the adversarial "a tail smuggling a custody pointer" row
refuse: `chunk` is bound by an opaque call, so `untracked_bases` refuses it,
and `flb_trace("%p", chunk)` is refused **under** the declaration.

**One existing closure was widened, deliberately and minimally.**
`borrowed_actual::seed_call_arguments` refuses the root names of every argument
at a position that is *not* declared `Borrowed`, "because the callee may change
their custody". Before this card, an actual beyond the declared vector had no
mode and was therefore always seeded risky — which would have made T4
unsatisfiable for **every** tail actual, including `tag`. Under a declared data
tail, such a position provably changes no custody *by the same declaration the
rule already trusts for a `Borrowed` position*, so it is exempted. The
exemption is decided from the **declaration alone**, never from the actual, so
a name risky for any other reason — an acquisition, an address, a custody
position elsewhere — stays risky. There is no circularity: the exemption does
not consult the data-only judgement it feeds.

Its blast radius is bounded and measured: the exemption can only fire for a
callee with a declared tail, and the pinned corpus has none, so `never_bound`
is unchanged there (§5, zero movement).

### 3.3 A second silent drop, repaired — the arity gate

Everything downstream of a modeled call zips the contract's parameter vector
against the actuals (`contract.parameters.iter().zip(arguments)`,
`arguments.get(index)`). An actual **beyond** that vector was therefore
dropped, without a diagnostic — the same class of hole card O04-B09B10 found
for a nested call, and the reason card O04-ARMS had to declare `flb_error` at
one arity. (O04-ARMS's fixture comment asserted that "an actual beyond the
declared parameter vector is never admitted"; measured here, that was **not**
true of the implementation. The comment is now accurate because the code
changed, not because the comment did.)

An actual beyond the declared vector is now refused with
`arguments beyond the declared parameters of \`NAME\` are outside the resource
projection`, unless the callee declares a variadic data tail **and** this call
site's tail actuals are data. Under-supply is deliberately unchanged: a missing
custody-bearing actual already reports on its own, and widening that is not
this card's subject.

**Measured reach of the repair on the pinned corpus: zero.** No call site in
185 units passes more actuals than its contract declares — if one did, a new
`Unsupported` diagnostic would appear, and §5 measures that none does.

---

## 3bis. The measured per-exit ledger

`flb_input_chunk_create`, on the **byte-verbatim** upstream body, under the
premise set of §4. Measured by
[`o04_trace.rs::print_the_measured_arm_ledger`](../../../crates/qcore-c-gateway/tests/o04_trace.rs)
and pinned in full — exit line, custody, and every site with its acquisition
line — by `the_whole_per_exit_ledger_is_pinned`, so a silent change in either
direction fails.

```
flb_input_chunk_create: Safe
  exit L277 Discharged sites=[(206, Escaped), (227, Escaped), (244, Escaped)]
  exit L251 Discharged sites=[(206, Released), (227, Released)]
  exit L231 Discharged sites=[(206, Released)]
  exit L225 Discharged sites=[(206, Released)]
  exit L218 Discharged sites=[(206, Released)]
  exit L211 Discharged sites=[]
  diagnostics: []
```

| O00 exit | preprocessed line | Arm | O04-B09B10 | **This card** | Residual |
|---|---|---|---|---|---|
| C1 | 211 | constructor returned NULL | Discharged | **Discharged** | none acquired |
| C2 | 218 | `up_force == -1` | Discharged | **Discharged** | chunk Released @206 |
| C3 | 225 | header write `== -1` | Discharged | **Discharged** | chunk Released @206 |
| C4 | 231 | `ic` calloc NULL | Discharged | **Discharged** | chunk Released @206 |
| C5 | 251 | `routes_mask` calloc NULL | Discharged | **Discharged** | chunk + `ic` Released @206, @227 |
| **C6** | **277** | **success + publication** | **`Unknown`, 1 refusal** | **Discharged** | all three **Escaped** @206, @227, @244 |

Site counts `[0,1,1,1,2]` on the five settled arms, every site `Released`, all
four cleanup arms naming the same acquisition site 206: **unchanged** from
cards O04-ARMS, O04-B08, O04-READ-BOUND and O04-B09B10. No exit is a
`Residual`, no `Leak` is reported, and **no `Mixed` site remains**.

C6 escapes all three identities — the chunk and the routes mask as live linked
children of the escaping aggregate (premise **P-B08-SHALLOW**), and `ic` itself
as the returned value. This is the shape card O04-B09B10's labelled control
predicted; the one difference is the exit line, **277** here against the
control's 276, because that control replaced the two-line `flb_trace` statement
with a one-line call. On the verbatim body the statement is still there, and
now modeled.

---

## 4. Every entry premise, named

O04's nine premises, O04-ARMS's two, O04-B08's two (**P-B08-SHALLOW** and the
`flb_time_now` declaration), O04-READ-BOUND's none and O04-B09B10's five carry
over **unchanged** and are still **assumed, not derived** — including that the
constructor's own body does not discharge its declared contract, and including
**`O04-B09B10-VALUE-DEPENDENT`** (`flb_hash_table_add`'s custody depends on the
value of `val_size`, and the declaration is faithful only at these call sites'
literal `0`). Two premises inherited from C02 are **not** proved: declaration
completeness and C effective types. C02-P's recorded may-alias boundary
(**C02-P-ALIAS**) is inherited and not widened.

This card adds **two** declared callee premises, in the new fixture module
[`o04_trace_callees.qcore`](../../../crates/qcore-c-gateway/tests/fixtures/contracts/o04_trace_callees.qcore).
Each is a **trusted assumption**, checked against the pinned upstream sources
rather than assumed:

| Callee | Declaration | Faithfulness, measured |
|---|---|---|
| `flb_trace(format, …)` | borrows `format`, returns no custody, **declares a variadic data tail** | faithful as to custody: the logging expansion formats and prints. The tail declaration asserts only that no tail argument's custody is taken or retained; it asserts nothing about the body, and the gateway independently requires each tail actual to be data |
| `flb_input_chunk_get_name(ic)` | borrows `ic`, returns no custody | faithful: it returns `ic->name`, a **borrowed handback** the caller must not free. That the handback aliases into `ic` is **not** modeled — the result is not tracked at all — which is exactly why it may occupy only a position needing no identity and is never bound |

**What a `Discharged` exit is.** A claim about *this projection under these
premises*, not a proof that the C is correct. In particular the containment
publications remain unmodeled: `mk_list_add` really does splice `ic` into
`in->chunks` and `flb_hash_table_add` really does record `ic` in a table (O00
blockers B03/R13). The per-exit claim is that every identity is **accounted
for**, never that it is **exclusively held**.

---

## 5. Corpus impact — zero movement, on every artifact

### 5.1 Provenance

Re-measured through the canonical pinned route on the shared prepared pinned
clone (Fluent Bit `66910c10a4d7eafa810b84229cf2dcf7b0f26f97`, explicit
`/usr/bin/clang` 21.1.8), following
[`docs/reports/corpus-repin/provenance.json`](../corpus-repin/provenance.json):

```sh
python3 -B providers/package.py build --output build/cycle19/provider-packages-trace
QCORE_CLANG=/usr/bin/clang \
QCORE_CHUNKIO_PACKAGE=build/cycle19/provider-packages-trace/qcore.chunkio \
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  --pinned-evidence \
  <pin>/fluent-bit-clang-source \
  <pin>/monkey/monkey-build/compile_commands.json \
  <out>/monkey-source.json <out>/resource-diagnostics.json
```

Run **twice from this same worktree**, with only `crates/qcore-c-gateway/src/`
and `crates/qcore-contract/src/` reverted to the branch base `bf210eb` between
runs, so the comparison carries no path provenance at all. Both exit 0 with
**zero gateway failures**. Transcripts:
[`logs/corpus-before.log.gz`](logs/corpus-before.log.gz),
[`logs/corpus-after.log.gz`](logs/corpus-after.log.gz). `build/cycle19/` is
gitignored, so the run script is not committed; the command it runs is complete
above. Nothing in the shared clone was written to.

### 5.2 Before → after: every artifact byte-identical

| Artifact | before | after | |
|---|---|---|---|
| `resource-diagnostics.json` | `a807b4714340509b…` | `a807b4714340509b…` | **identical** |
| `monkey-source.json` | `53d55b7e1fb174de…` | `53d55b7e1fb174de…` | **identical** |
| `resource-diagnostics.chunk-contracts.json` | `c4f438e4ed3dab3c…` | `c4f438e4ed3dab3c…` | **identical** |
| `resource-diagnostics.source-effects.json` | `46f7b980f989dece…` | `46f7b980f989dece…` | **identical** |
| `resource-diagnostics.contracts.json` | `0c50e3a256a4ade9…` | `0c50e3a256a4ade9…` | **identical** |

The independent census
([`census.py`](census.py), transcript at [`logs/census.txt`](logs/census.txt))
compares the two runs directly rather than trusting digests:

```
units: 185 -> 185;  added: [];  removed: []
unit status before/after: {'Safe': 2, 'Unknown': 137, 'Unsafe': 42, 'data_only': 4}
unit status moves: 0
NEWLY SURFACED PROVISIONAL UNSAFE UNITS: 0
unit reason_code moves: 0
projected functions: 4834 -> 4834;  function status moves: 0
leak: 271 -> 271 (+0);  unknown_alias: 4410 -> 4410 (+0);  unsupported: 49960 -> 49960 (+0)
units with diagnostic-count changes: 0
```

**No newly surfaced provisional `Unsafe` unit**, at any level: identical status
histogram, no unit row changed `status` or `reason_code`, all 4,834 projected
functions compared pairwise with zero moves, and zero units with any diagnostic
change in either direction. There is nothing to attribute, because nothing
moved.

### 5.3 The BEFORE run against the archived wave-12 baseline

[`docs/reports/wave12-corpus/artifacts/`](../wave12-corpus/artifacts/), the
current pinned baseline. Full diff:
[`logs/diff-before-vs-wave12.json.gz`](logs/diff-before-vs-wave12.json.gz).

| Artifact | before vs archived |
|---|---|
| `diagnostics.json` | **byte-identical** (`a807b4714340509b…`) |
| `chunk-contracts.json` | **byte-identical** (`c4f438e4ed3dab3c…`) |
| `source.json` | **byte-identical** (`53d55b7e1fb174de…`) |
| `contracts.json` | differs — **path-literal provenance residue only** |
| `source-effects.json` | differs — **package-identity residue only** (packages rebuilt for this run); 3 rows, the three checked function contracts, whose `resource_contracts:<digest>` obligation identifier moves. Status unchanged at 3 checked / 93 Unknown |

Both residue classes were recorded by CORPUS-REPIN, O04-ARMS, O04-B08, E01-F1,
O04-READ-BOUND, O04-B09B10 and the wave-9/10/11/12 consolidations, and both are
**identical between before and after here**, so they do not touch the isolation.

**The baseline is therefore NOT re-pinned.** The after-measurement reproduces
`docs/reports/wave12-corpus/artifacts/` byte for byte on all three
non-residue artifacts, so archiving a duplicate copy would add nothing; the
digests above and the census transcript are the record.

**`O04-TRACE-REACH`: measured production reach is zero admitted statements.**
No `contract/qcore` declaration carries a variadic data tail, so no tail
position is admitted anywhere in the corpus; and no corpus call site supplies
more actuals than its contract declares, so the §3.3 arity refusal never fires.
As in cards C03-NFR, O04-READ-BOUND and O04-B09B10, the payoff gates on how
much declared contract text exists.

### 5.4 The configured audits

Run in this same worktree, before and after, with only the two `src/`
directories reverted between runs.

| Audit | Result | before vs after |
|---|---|---|
| `contract/profiles/cmetrics-counter` | 1 unit: 0 Safe / 1 provisional Unsafe / 0 Unknown; **1 checked function contract / 6 Unknown** | **byte-identical transcript** |
| `contract/profiles/chunkio-core` | 13 units: 0 Safe / **2 provisional Unsafe** / 11 Unknown / 0 gateway failures | **byte-identical transcript** |
| `contract/profiles/libevent-core` | **not run** | needs a prepared CMake database absent from this environment; it rejects identically before and after |

Transcripts in [`logs/`](logs/).

---

## 6. Lean parity

**58 audits** (57 → 58). No new axiom, no placeholder, no `sorry`. Every one of
the **39** theorems in the new module is audited **by name**; a scripted check
compares the theorem list against the audit list and they are equal.

### 6.1 New — [`proof/QCore/VariadicNested.lean`](../../../proof/QCore/VariadicNested.lean), 39 theorems

Registered as
[`proof/AuditVariadicNested.lean`](../../../proof/AuditVariadicNested.lean).

* **Every nested premise refuses on its own** — seven theorems (N0-N5, the
  arity premise), including `more_than_one_nested_call_is_never_admitted`,
  which is two-level nesting and sibling nesting in one statement.
* **N6 and N7, refusing** — `an_owned_nested_result_is_never_admitted`,
  `a_reallocating_nested_result_is_never_admitted`, and
  `a_custody_bearing_nested_parameter_is_never_admitted` quantified over the
  *whole* mode vocabulary so a later widening cannot slip a mode past it, plus
  the contrapositive `an_admitted_nested_contract_is_neutral`.
* **N8, where the two halves compose** —
  `a_custody_slot_never_admits_a_nested_call` and
  `a_nested_call_beyond_the_named_vector_needs_a_declared_tail`: without the
  enclosing declaration's tail, a nested call beyond the named vector refuses.
* **The composition theorem, both orders** — `borrowed_fold_is_the_identity`,
  `an_admitted_nested_call_is_neutral`, `compose_inner_first`,
  `compose_outer_first` and `evaluation_order_is_unobservable`. This is the
  design's whole justification, proved rather than argued.
* **The converse, exhibited** — `an_owned_result_is_not_neutral`,
  `a_releasing_parameter_is_not_neutral` and
  `an_owned_parameter_is_not_neutral`. The third is why N7 says `borrowed`
  rather than "changes no disposition": an owned parameter changes nothing but
  *demands* liveness, and moving a demand across an unspecified evaluation
  order is exactly what must not happen.
* **The tail** — `an_excess_actual_without_a_declared_tail_is_refused` (the
  §3.3 repair as a theorem), each of T1-T4 refusing on its own,
  `every_admitted_tail_actual_is_data`, and
  `the_declaration_alone_admits_nothing`.
* **Non-vacuity in both directions** — an admitted nested call exists on
  **both** data slots and is refused on the custody slot; six syntactic
  premises and two declared-mode premises are separately flipped on the same
  witness; an admitted tail exists and each of its six premises flips it; and
  an empty excess list still needs the declaration.

The model is of the **decisions**, not of C semantics. Declared types, contract
modes, the C02-P closure's verdict on a name, C's object model and its
evaluation-order rule are premises supplied from outside; nothing proves the
reader extracts them faithfully, that a declared `Borrowed` mode really
borrows, or that a declared tail really takes no custody. Nothing here is a
claim about arbitrary C memory safety.

---

## 7. Deliberate pin updates

**None.** No pinned expectation moved, in either direction:

| Pin | Status |
|---|---|
| contract corpus module count (60) and every per-file clause count | **unchanged** — the two declarations are gateway test fixtures (§2.4) |
| the loader's rejected-module count (145) | **unchanged** |
| `third_party/libevent` submodule revision | **unchanged** |
| the wave-12 185-unit baseline artifacts | **unchanged** — the after-measurement reproduces them (§5.3) |
| every pinned expectation of cards O04, O04-ARMS, O04-B08, O04-READ-BOUND, O04-B09B10, C02, C02-P, C03-NFR, W4-EXPR-POS, ALIAS-LOCALS | **unchanged**, and all pass |
| card O04-B09B10's nested-call refusal text | **unchanged**, asserted verbatim by this card's tests |

`o04_path.qcore`, `o04_arms_callees.qcore`, `o04_b08_callees.qcore`,
`o04_read_bound_generic.qcore`, `o04_b09b10_callees.qcore` and
`o04_b09b10_generic.qcore` are **byte-unchanged**, so all five earlier cards
keep their exact premise sets. No gate was weakened, no diagnostic code was
added, removed or downgraded, and the two diagnostics this card introduces are
**refusals**, not admissions.

---

## 8. Evidence

### 8.1 Held-out generic family — 12 assertions

[`crates/qcore-c-gateway/tests/o04_trace.rs`](../../../crates/qcore-c-gateway/tests/o04_trace.rs),
a `v_*` API deliberately unrelated to ChunkIO, to Fluent Bit **and** to the
`g_*` (O04), `h_*` (O04-ARMS), `k_*` (O04-B08), `r_*` (O04-READ-BOUND) and
`m_*` (O04-B09B10) families. Every rule is stated over declared C types and
contract modes, so no name in it can matter.

| Test | Result |
|---|---|
| `a_nested_accessor_in_a_declared_tail_discharges_every_exit` | the real shape, synthetically — **Safe**, every exit `Discharged` |
| `an_admitted_nested_call_does_not_hide_a_leak` | the same body without the release is **Unsafe** with exactly one leak: precision in the defect direction too |
| `the_admitted_tail_actual_shapes_are_data` | declared scalar, enumeration constant, string literal and numeric literal in one accepted tail |
| `a_tail_smuggling_a_tracked_identity_is_never_admitted` | **adversarial row 1** — the aggregate, an owned child, `&b` and `b->flag` in a tail each refuse *under* the declaration |
| `a_nested_call_producing_custody_is_never_admitted` | **adversarial row 2** — an owned nested result keeps card O04-B09B10's refusal, asserted verbatim |
| `two_level_and_sibling_nesting_are_never_admitted` | **adversarial row 3** — `f(g(h(x)))` and `f(g(x), h(y))` |
| `a_nested_call_without_a_contract_is_never_admitted` | **adversarial row 4** — an uncontracted inner callee |
| `a_partial_nested_call_expression_is_never_admitted` | arithmetic around the call, and a nested call at a *custody* position |
| `an_excess_actual_without_a_declared_tail_is_refused` | the §3.3 arity repair, with the exact-arity control on the same callee still `Safe` |
| `a_declared_tail_covers_only_the_tail` | a tail actual carrying an identity is not rescued by the declaration |
| `the_variadic_tail_clause_is_total_or_reject` | the loader: the clause is rederived from retained sources; an empty environment has no tails; four malformed spellings each reject the whole module; the clause does not replace a parameter mode |
| `a_certificate_resting_on_the_tail_is_rederived_from_its_own_sources` | **the sharp substitution**: withdrawing the clause from the certificate's retained sources leaves the loaded contract table byte-identical (asserted) and the **independent recheck refuses** |

Plus **14 new unit tests** in `variadic_nested`'s own module, with no Clang:
the T1-T4 walker over declared scalars, enumeration constants, literals and a
name the body binds; the five indirection shapes; an unreadable function; and
the nested admission's whole premise set including the custody-position refusal
with its borrowed-position control.

### 8.2 Real-source recheck — 5 assertions plus one reporting aid

| Test | Result |
|---|---|
| `the_success_exit_discharges_and_the_caller_reaches_safe` | **the headline**: `Safe`, no diagnostics, six exits `Discharged`, C6 escaping all three identities |
| `the_whole_per_exit_ledger_is_pinned` | the complete six-exit ledger — line, custody, and every site with its acquisition line — pinned exactly (§3bis) |
| `withdrawing_either_declared_callee_restores_the_refusal` | three substitutions on the real fixture — no `flb_trace` declaration; `flb_trace` without its tail; the accessor uncontracted — each restores a refusal and returns C6 to `Unknown` |
| `the_constructor_boundary_is_unchanged` | `cio_chunk_open`: 4 Discharged / 2 Unknown, no leak alleged |
| `the_program_stays_unknown` | a function reaching `Safe` is not a unit reaching `Safe` |

---

## 9. Open boundaries

Carried forward unchanged: O04's blockers **B02-B07** and **B11**;
**O04-ARMS-SCALAR-CHAIN**; **C02-P-ALIAS**; **P-B08-SHALLOW**,
**O04-B08-BORROWED-AGG**, **O04-B08-DOT**, **O04-B08-CAST**;
**O04-READ-BOUND-SCALAR**, **O04-READ-BOUND-REACH**;
**O04-B09B10-VALUE-DEPENDENT**, **O04-B09B10-FRAME**, **O04-B09B10-REACH**.

Closed by this card, at the boundary §2 and §3 state: **`O04-B09B10-TRACE`**,
**`O04-ARMS-VARIADIC`**. **Superseded** at a narrower checked boundary:
**`O04-B09B10-NESTED`** — a nested call is no longer *always* a refusal, but
every nested call outside the eight premises still is.

| # | Boundary | Status |
|---|---|---|
| **O04-TRACE-REACH** | measured production reach of both families in the pinned corpus is **zero admitted statements** (§5.3): no `contract/qcore` declaration carries a tail, and no corpus call site over-supplies actuals. The payoff gates on declared contract text | **recorded, §5.3** |
| **O04-TRACE-DEPTH** | exactly **one** level and **one** nested call. Two-level nesting needs an evaluation order and an intermediate-value identity this projection still does not have | **open, refused by construction** |
| **O04-TRACE-EFFECTFUL** | a nested call with *any* declared custody effect is refused, because C's unspecified argument evaluation order would then be observable. Admitting one needs a sequencing model, not a wider whitelist | **open, deliberately conservative** |
| **O04-TRACE-HANDBACK** | a borrowed handback's aliasing into its argument is **not** modeled. Safety here rests on N8 confining it to a position needing no identity and on its never being bound; a body that stores one is refused | **open, recorded** |
| **O04-TRACE-UNDERSUPPLY** | the arity gate repairs **over**-supply only. A call supplying *fewer* actuals than its contract declares still reports only through the existing per-mode paths | **open, out of scope** |
| **O04-TRACE-TAILTRUST** | `resource_variadic_data_tail()` is a declaration. Nothing verifies that a callee really keeps no pointer to its tail; a `printf`-style callee that stashed one would be a false premise, not a defect this card can see | **open, a premise** |
| — | tail actuals carrying `&`, `*`, a subscript or a member access; a nested call that is part of an expression; a nested arity mismatch | **refused by construction** |

### 9.1 Cross-card overlap, recorded for the integrator, not resolved here

Three wave-13 cards run in parallel in `crates/qcore-c-gateway/`:

* **REACH-GATE**, **BORROW-LIVENESS** and **HASH-CONTRACTS**. This card
  touches `legacy_c.rs` (`classify_simple`'s contracted-call branch — the
  nested admission, the arity gate, the `&`-gate exemption, `call_arguments`
  refactored onto a new `call_argument_spans`; the `ContractCall` variant's new
  `nested` field and its application in `transfer`),
  `borrowed_actual.rs` (**one** widened conjunct in `seed_call_arguments`, plus
  the new `variadic_tails` parameter on `untracked_bases`),
  `typed_field_access.rs` (two additive accessors) and `contract_loader.rs`.
* `untracked_bases` gained a **parameter**, so any card calling it must pass
  the tail set (or `&BTreeSet::new()`); the two existing test-only call sites
  were updated in this card.
* If **BORROW-LIVENESS** changes what a `Borrowed` position demands, premise N7
  moves with it and §5 must be re-run: N7's neutrality argument is stated over
  the *declared* mode, and its Lean converse would need re-examination.
* If **HASH-CONTRACTS** adds `contract/qcore` text with a variadic tail, this
  card's `O04-TRACE-REACH` measurement of zero stops holding and §5 must be
  re-run.

**Recorded, not resolved.**

---

## 10. Scope and non-claims

- **No raw unit is promoted.** `fluent-bit-chunk/flb_input_chunk.c` keeps its
  185-unit verdict; the corpus aggregate is unchanged and no verdict moved.
- **A function reaching `Safe` is not a unit reaching `Safe`.** The fixture's
  program result stays `Unknown` because `cio_chunk_open` in the same unit
  keeps its own refusals, and the production unit does not carry these
  premises at all.
- A per-exit **Discharged** is a claim about *this projection under the named
  premises of §4*, not a proof that the C is correct. Every declared contract
  it rests on is a trusted assumption, and one of them
  (`O04-B09B10-VALUE-DEPENDENT`) is faithful only at the call sites measured.
- **The containment publications are not modeled.** O00 blockers B03/R13
  remain open. The per-exit claim is that every identity is accounted for,
  never that it is exclusively held.
- A provisional `Unsafe` is never presented as an upstream defect; the `v_*`
  family is synthetic and its `Safe` is `Safe` for a synthetic declared custody
  projection.
- **Unknown means Unknown.** An exit whose path carried a refusal alleges
  nothing in either direction.
- Projection coverage is not safety coverage; unknowns stay in the denominator.
- CT coverage is unchanged at 31/50; the contract corpus remains 60 modules;
  `qcore-schemas` is unchanged at 9.0.0; no plugin, protocol or evidence
  version moved.

---

## 11. Files changed

| Path | Change |
|---|---|
| `crates/qcore-contract/src/contract_loader.rs` | the `resource_variadic_data_tail()` clause; `variadic_data_tails` and `declared_variadic_data_tails`; `load_function` returns the flag |
| `crates/qcore-c-gateway/src/variadic_nested.rs` | new — `NestedCallActual`, `admitted_nested_call`, `tail_arguments_are_data`, `is_data_only_actual`; 14 unit tests |
| `crates/qcore-c-gateway/src/legacy_c.rs` | `NestedContractCall` and `ContractCall::nested`; the nested admission, the arity gate and the `&`-gate exemption in `classify_simple`; the nested application in `transfer`; `call_argument_spans` with `call_arguments` refactored onto it; `borrowed_argument_place`; `declared_variadic_tails` |
| `crates/qcore-c-gateway/src/borrowed_actual.rs` | one widened conjunct in `seed_call_arguments` plus the `variadic_tails` parameter (§3.2) |
| `crates/qcore-c-gateway/src/typed_field_access.rs` | one additive accessor, `declared_is_scalar` (the existing `is_enum_constant` is reused unchanged) |
| `crates/qcore-c-gateway/src/aggregate_store.rs` | one test-only call site updated for the new parameter |
| `crates/qcore-c-gateway/tests/o04_trace.rs` | new — 17 held-out assertions plus one ignored reporting aid |
| `crates/qcore-c-gateway/tests/fixtures/contracts/o04_trace_{callees,generic}.qcore` | new — the two declared callee premises and the held-out `v_*` API, both outside `contract/qcore` |
| `crates/qcore-c-gateway/tests/unit/legacy_c.rs` | one new struct field in one constructed instruction |
| `proof/QCore/VariadicNested.lean`, `proof/QCore.lean`, `proof/AuditVariadicNested.lean` | new module (39 theorems) and its axiom audit |
| `docs/reports/remaining-o04-trace/` | this record, the census tool and the transcripts |

No file owned by the integrator was touched: `AGENTS.md`, `CLAUDE.md`,
`README.md`, `DEVELOPMENT.md`, `docs/reevaluation/ct_manifest.json`,
`docs/reviews/r0_3/acceptance.json` and
`docs/reviews/r0_3/acceptance_matrix.json` are unchanged, as are
`contract/qcore/`, `crates/qcore-c-gateway/src/may_alias.rs`, every archived
predecessor report and every existing test fixture.

---

## 12. Gate results

Run in this worktree at branch `claude/o04-trace`, base `bf210eb`, with the
Libevent submodule initialised.

| Gate | Command | Observed | Exit |
|---|---|---|---|
| Workspace tests | `cargo test --workspace --no-fail-fast` | **2,349 passed / 0 failed / 6 ignored** | 0 |
| Format | `cargo fmt --all -- --check` | clean | 0 |
| Clippy | `cargo clippy --all-targets -- -D warnings` | clean | 0 |
| Probe | `cargo run -p qcore-compiler --example reevaluation_probe` | ran | 0 |
| Follow-up | `cargo run -p qcore-compiler --example reevaluation_followup` | ran | 0 |
| Lean audits | `python3 -B scripts/check_lean_audits.py` | **58 audits passed** (57 → 58) | 0 |
| Provider tests | `cargo test --locked --offline --manifest-path providers/Cargo.toml --workspace` | **87 passed / 0 failed** | 0 |
| Provider packages | `python3 -B providers/package.py build` | five packages built and verified | 0 |
| Python | `python3 -B -m unittest discover -s scripts/tests` | **125 ran, 1 failure** — see §12.2 | 1 |
| 185-unit corpus | pinned route, before and after | §5 | 0 / 0 |
| Configured audits | `qcore-audit` over cmetrics-counter and chunkio-core, before and after | §5.4 | 0 / 0 |

### 12.1 What this card contributes

**31 new assertions** — 17 held-out integration (`o04_trace.rs`) and 14
Clang-free unit tests (`variadic_nested`'s own module) — and **39 new Lean
theorems**, every one of them audited by name and none introducing an axiom
(every theorem depends only on `propext`, some additionally on `Quot.sound`).
The workspace tally is 2,349; no existing test was added, removed or renamed,
so the branch base stood at 2,318 — that figure is **derived by subtraction**,
not separately measured.

### 12.2 One integrator-owned pin must move — `validation.lean_audits`

`scripts/tests/test_build_matrix_sections.py::test_the_derived_facts_are_recomputed_rather_than_carried`
fails, expecting the committed `docs/reviews/r0_3/acceptance_matrix.json`
`validation` block to match the derived count. This card adds one
`Audit*.lean` file, so the derived count is now **58** and the committed value
is stale. **That is card GEN-REASON's tripwire working exactly as designed**,
not a defect and not a weakened gate.

That file is explicitly integrator-owned and this card is forbidden from
editing it, so the pin is **deliberately left for the integrator** to move when
regenerating the matrix with `--gate`:

* `validation.lean_audits`: 57 → **58**
* `validation.lean_audit_files`: add `AuditVariadicNested.lean`
* `validation.workspace_tests`: this card's contribution plus the rest of the
  wave

### 12.3 Load-flaky

`cycle20_verifier_plugins`'s `vp04_vp07_vp08_process_lifecycle_and_payload_budgets`
failed once in an early full-workspace run under the concurrent wave-13 fleet
and **passed in isolation**; it also passed in the final full run recorded
above. Its assertion was not touched. The P02 `RLIMIT_NPROC` and P03
confined-write tests are the other two recorded load-flaky cases; neither
failed here.

### 12.4 A note on the concurrent fleet

The corpus runs of §5 were executed while three other wave-13 cards were
running their own audits on the same host, which is why the BEFORE run took
substantially longer than the AFTER one. That affects wall-clock only: both
runs read the same pinned clone read-only, wrote to separate output
directories, and produced byte-identical artifacts.

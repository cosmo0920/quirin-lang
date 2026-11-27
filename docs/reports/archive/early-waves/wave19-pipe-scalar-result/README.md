# PIPE-SCALAR-RESULT — binding a non-acquiring call's scalar result (wave 19)

Card brief: *"No route today binds a scalar call result … Your deliverable: a
checked form that binds a **non-acquiring call's scalar result**
(`let n = f(x);`), plus the scalar-**argument** form the pipe-chain report notes
is also missing on the extern route."*

Named follow-on of `PIPE-CHAIN`
([`docs/reports/wave19-pipe-chain/README.md`](../wave19-pipe-chain/README.md)
§3, §7). Base commit: main `c7c0a3c`.

**What landed.** Two source forms on `source.extern-call.v1` — a scalar-result
`let` and a bare `NAME` at a declared scalar parameter — and one widening of the
pipe's intermediate-stage rule that makes the two reachable through `|>`. The
first of `PIPE-CHAIN`'s two measured boundaries under `value |> render |> log` is
retired: a `render` stage now exists, `x |> render |> scale` is expressible and
emits **byte-identical C11** against its hand-written statement sequence, and a
scalar binding is proved to leave the ledger exactly as the discarding call
would. Seventeen new audited Lean theorems in `QCore.ExternCall` and four in
`QCore.PipeDesugaring`, no new axiom, no new audit file, **no diagnostic code
added, moved or changed**, and no reserved word.

**What did not.** The second boundary, `STD-IO-COMPOSE`, is untouched by
construction: route selection in `driver.rs` is not edited, `print` still refuses
on the extern route, and the literal acceptance sentence stays unwritable. §7
says exactly what that card still needs.

---

## 1. BEFORE — the census the brief asked for

Every place a route decides what a `let` initializer or a call actual may be, and
its exact refusal on main `c7c0a3c`. Measured by compiling each shape, not read
off the source.

| Site | Shape | BEFORE |
|---|---|---|
| `source_extern.rs`, `StatementKind::Let` → `parse_adapter_statement` | `let n = qcore_extern_sum(b, 8);` | `Q025` "an extern acquisition must read `let NAME = CALLEE(ARGS) succeeds BOOL else raise ERROR;`" |
| same | `let u = qcore_extern_fill(b, 7);` (`unit` result) | `Q025`, same text |
| same | `let b = qcore_extern_alloc(8);` (acquiring, adapter dropped) | `Q025`, same text |
| same | `let total = 42;` (scalar literal) | `Q025`, same text |
| `source_extern.rs`, `Lowering::arguments`, `CallParameter::Data` arm | `qcore_extern_scale(n, 2)` with `n` a local | `Q024` "argument 1 does not bridge to the declared mode `Data(U32)`" — the catch-all, because the `Data` arms matched integer literals only |
| `pipe.rs`, `Stage::produces_a_result` | `b \|> qcore_extern_sum(8) \|> …` | `Q037` "`qcore_extern_sum` produces no value … only an acquisition `CALLEE(ARGS) succeeds BOOL else … raise ERROR` binds a result" |
| `pipe.rs`, bound-final-stage rule | `let n = b \|> qcore_extern_sum(8);` | `Q036` "`qcore_extern_sum` produces no value, so a bound pipeline cannot end in it" |
| sequential executable subset (`frontend.rs`) | `let n = 2;` | `Q500` "`let` must use `qcore_acquire(resource_id, succeeds, ErrorType)` in the executable subset" |
| sequential executable subset | `let n = helper(2);` | the body never reaches a `let` rule: the entry gate refuses first at `Q020` |
| `source_std_io.rs` | `let total = 42;` then `print dec total;` | **accepted** — the std-io route already has integer bindings, and they are the operands `Q028` requires |

Two things the census establishes and the card's framing did not assume.

1. The extern route's `let` was **one** production, not a family: everything that
   was not an acquisition fell into one `Q025`. That is why the BEFORE column has
   a single message four times.
2. The **plain sequential route has no call-statement form at all**, so there was
   nothing there to widen: its `let` is `qcore_acquire(…)`-only and its bodies
   admit no call to a non-zero-parameter function (§G02c). Widening it is not
   this card's shape and is named as a follow-on (§7, `SEQ-SCALAR-RESULT`)
   rather than attempted.

## 2. AFTER — the boundary that landed

The same shapes, re-measured.

| Shape | AFTER |
|---|---|
| `let n = qcore_extern_sum(b, 8);` | **accepted** |
| `qcore_extern_scale(n, 2);` with `n` a bound scalar | **accepted** |
| `let scaled = b \|> qcore_extern_sum(8) \|> qcore_extern_scale(2);` | **accepted**, byte-identical C to its hand-written twin |
| `let n = k_unchecked_add(2, 40);` (`unsafe extern fn`) | **accepted**, carrying the same `UncheckedLinkage` premise as the discarding call |
| `let u = qcore_extern_fill(b, 7);` (`unit`) | `Q024` "`qcore_extern_fill` declares the result `void`, which is not a custody-free scalar; only a scalar result may be bound to a local, and `qcore_extern_fill(ARGS);` discards it" |
| pointer result declared `resource_return_none()` | `Q024`, same rule, naming `uint8_t *` |
| `let b = qcore_extern_alloc(8);` (owned nullable return) | `Q025` "`qcore_extern_alloc` establishes fresh custody; its failure arm must be branched with `let NAME = … succeeds BOOL else raise ERROR;`" |
| the same for an out slot and for a declared reallocation | `Q025`, same rule |
| `let total = 42;` (scalar literal) | `Q025`, **unchanged** — not this card, follow-on `EXTERN-SCALAR-LITERAL` |
| `total \|> print dec \|> log info dec;` | `Q037`, **unchanged in code**, message now "it is a sink" |
| the sequential and std-io rows of §1 | **unchanged** |

### 2.1 The admission, stated exactly

The whole widening is one predicate,
`qcore_core::extern_call::binds_scalar`:

> the callee establishes fresh custody through **none** of the three acquisition
> sources, **and** its declared result is a `CallResult::Data(abi)` with
> `abi != Unit` and `!abi.is_pointer()`.

Each clause is load-bearing and was chosen against the closed C04 bridging table
rather than against the Rust type:

* `acquires` covers `resource_return_owned(nullable)`,
  `resource_return_reallocated(_, nullable)` **and** a declared
  `resource_out(_, nullable)` slot. The out slot is the case a rule written over
  the *result* alone would have got wrong: `qcore_extern_open` declares
  `resource_return_none()` with an `i32` status, so its result **is** a scalar
  and it still acquires. `an_out_writing_call_binds_no_scalar` is that case as a
  theorem.
* `unit` is excluded because there is no value.
* every pointer is excluded, **including a pointer declared
  `resource_return_none()`** — a shape the loader admits. Such a result is an
  address the declaration says nothing about; binding it would put an unmodelled
  address into a body under a name, which is exactly the laundering this card
  exists to make impossible.

The mandatory nullable-owned-return adapter, the error-arm disposition
discipline, `AdapterMissing`, `AdapterDepartingArgument` and every §C04 admission
rule are **byte-unchanged**. There is no second way to spell an acquisition:
`an_acquisition_has_no_scalar_binding_step` says the scalar step does not exist
for an acquiring callee, and the Rust reports `Q025` at the same shape.

### 2.2 Two namespaces, not one

`ValueId` is a distinct type alias from `ResourceId` and the two are kept in
separate maps, separate certificate fields and separate C variable prefixes
(`v1` beside `r1`). The separation is enforced at four places, each pinned:

| Place | Rule |
|---|---|
| `check_extern_program`'s argument admission | `ExternArgument::Value` is admitted at `CallParameter::Data` and at no other mode |
| `Lowering::arguments` | a bare `NAME` at a `Data` parameter resolves against the **value** map only; a live resource binding there refuses with "a declared scalar parameter carries data, never custody" |
| the ledger | a scalar binding never touches `live`; there is no path on which a `ValueId` becomes a `ResourceId` |
| `derived_custody` | unchanged — it filters `Data` parameters out, so a scalar actual contributes nothing to the summary the caller's passes consume |

A scalar binding therefore **opens no obligation**: an unread one is not a leak
(`an_unread_scalar_binding_is_not_a_leak`), which is the operational difference
between the value namespace and the ledger.

### 2.3 One measurement, recorded rather than designed around

The lowering carries a cross-namespace shadow refusal (`Q026`): a scalar `let`
whose name is a live resource binding, or an acquisition whose name is a bound
scalar. It is **unreachable from source**. Name resolution's own `Q012`
(duplicate local binding) runs over the parsed body before any lowering and
catches both directions first.

The arm is kept anyway, on the precedent of the pipe desugarer's block-boundary
arm (`PIPE-CHAIN` §6.4 of its own record): it is what keeps the lowering
fail-closed if a future route ever hands it a body name resolution did not see.
Both halves are pinned —
`a_binding_that_shadows_the_other_namespace_refuses` asserts `Q012` is present
**and** `Q026` is absent, so if the reachability ever changes the test says so
rather than silently passing.

Similarly, `qcore_extern_scale(nope, 2)` with an entirely unbound name refuses at
`Q013` (unknown name), not at the lowering's "not a bound scalar" arm. That arm
is reachable for a name that *does* resolve — a live resource binding at a data
parameter — which is the case above it in the same test.

## 3. The pipe payoff

`Stage::produces_a_result` changed from "the stage carries `succeeds`" to "the
stage is a **call** stage". That is still syntactic and still contract-free —
`print`/`log`/`flush` are a closed statement table — but it is now the honest
criterion, because after this card the acquisition adapter is no longer the only
`let`-with-call the language admits.

The consequence is deliberate and is a **pin update, not a promotion**: a
`unit`-returning callee in a non-final position still refuses, but at the
route's `Q024` naming its declared result rather than at the pipe's `Q037`
naming the pipe. That is the same answer the hand-written statement sequence
gets, which is the property the pipe is supposed to have.

```qcore
let scaled = b |> qcore_extern_sum(8) |> qcore_extern_scale(2);
```

desugars to, token for token:

```qcore
let ⟨t0⟩ = qcore_extern_sum(b, 8);
let scaled = qcore_extern_scale(⟨t0⟩, 2);
```

This is the brief's own `x |> f(a) |> g(b)` meaning `g(f(x, a), b)` over **data**
rather than over custody — `PIPE-CHAIN` shipped the custody version and recorded
the data version as unreachable. `scalar_result_piped.qcore` and
`scalar_result.qcore` emit byte-identical `c_source` and identical reference
observations, and the temporary's name reaches neither.

## 4. Evidence

### 4.1 Tests

| Target | Count |
|---|---|
| `compiler/tests/extern_calls.rs` | **70 → 80** (+10; the existing sanitized native test gains one fixture) |
| `compiler/tests/pipe_operator.rs` | **39 → 41** (+2 new; one existing test rewritten in both halves, §6.1) |
| `crates/qcore-core/tests/unit/extern_call.rs` | unchanged in count; 35 `ExternCall` literals gained `binding: None` |

Two shipped fixtures, both twins of pinned code rather than programs written to
make a test pass:

* `compiler/tests/fixtures/extern_calls/scalar_result.qcore` — `round_trip.qcore`
  with two scalar bindings added inside its own custody;
* `compiler/tests/fixtures/pipe/scalar_result_piped.qcore` — the piped twin.

One new declared callee, `qcore_extern_scale(value: i32, factor: i32) -> i32`,
added to the **test-fixture** contract module and its C implementation. It is the
scalar-in/scalar-out shape a scalar result can feed, and it carries custody in no
position. Fixture contracts live under `compiler/tests/fixtures/`, never under
`contract/qcore/` (§5).

Both directions are pinned throughout: the newly admitted forms *and*
the still-refused ones — a discarded/rebound acquiring result, all three
acquisition sources, a `unit` result bound, a pointer result bound, a resource
at a scalar parameter, a scalar at a departing parameter, an unbound scalar, and
a cross-namespace shadow.

### 4.2 Byte-equality and the certificate

`a_scalar_stage_feeds_the_next_stage_and_emits_its_hand_written_twin` asserts
`compile(chain).c_source == compile(hand_written).c_source` and equal reference
observations, on the `PIPE-OPERATOR` precedent. The emitted C names values by
identity (`v1`, `v2`) exactly as it names resources (`r1`), so the pipeline
temporary's *name* never reaches the C at all.

`ExternCallCertificate` gains one field, `scalar_bindings: Vec<(String,
ValueId)>` in trace order — published beside the custody fields and never inside
them, so a reader can see which calls moved data and which moved custody.
`a_tampered_scalar_binding_list_fails_the_independent_recheck` pins that the
independent rederivation covers it, exactly as it covers `acquisitions`.

### 4.3 Native evidence

`scalar_result.qcore` joins the fixed ASan/UBSan/LSan list of
`accepted_fixtures_run_natively_against_a_linked_c_implementation`
(`-std=c11 -Wall -Wextra -Werror -pedantic -g -O0
-fsanitize=address,undefined -fno-omit-frame-pointer`, `detect_leaks=1`), linking
the real `buffer_api.c`. It is a **negative control** for the widening: the
fixture still allocates, lends and releases one real buffer, so if binding a
scalar had touched the ledger LSan would report it.

The piped twin is **not** run separately, on purpose: it emits byte-identical C,
so a second run would execute the identical program. Recorded here rather than
padded.

**Honest scope**, two items:

* The observation of a bound call names the callee, the value and the arguments
  but **not the number returned**. A reference interpreter cannot compute a C
  function's result, so observing it would make the two runs disagree by
  construction. What both runs do establish is that the same call bound the same
  value in the same position of the same trace. Follow-on
  `EXTERN-SCALAR-OUTCOME`.
* The emitted binding is followed by `(void) vN;`. That is not decoration: the
  native runs compile with `-Werror`, and a bound scalar the source never reads
  is admitted (§2.2), so the cast is what keeps the translation unit
  warning-clean in both cases without making the emission depend on a lookahead.

### 4.4 Lean

No new audit file, so `check_lean_audits.py` stays at **80/80**.

`proof/QCore/ExternCall.lean` grows **93 → 110** theorems (+17), audited by the
existing `proof/AuditExternCall.lean`. `proof/QCore/PipeDesugaring.lean` grows
**56 → 60** (+4), audited by `proof/AuditPipeDesugaring.lean`. No new axiom, no
placeholder.

The card asked for linearity through the new binding to be *characterized*. It
is proved, in five theorems over an explicit two-field state
(`Env := ⟨live : Ledger, scalars : List Nat⟩`) whose whole point is that no
operation moves a name between the fields:

* `a_scalar_binding_leaves_the_ledger_unchanged` — with no departing argument the
  ledger is unmoved;
* `a_scalar_binding_has_the_discarding_calls_ledger` — with departing arguments
  it is **exactly `stepCall`'s**, so the binding itself contributes no custody
  delta at all;
* `a_scalar_binding_introduces_no_identity` — nothing enters the ledger;
* `a_scalar_binding_never_names_a_live_identity` — the bound name is not an
  identity of the resulting ledger, which is "a scalar binding must not launder a
  resource identity" as a statement about the model;
* `a_scalar_binding_over_a_live_identity_refuses` — the cross-namespace shadow
  has no admitted step.

Refusal completeness is a second family
(`an_owned_return_binds_no_scalar`, `a_reallocating_return_binds_no_scalar`,
`an_out_writing_call_binds_no_scalar`, `an_acquiring_callee_binds_no_scalar`,
`a_unit_result_binds_no_scalar`, `a_pointer_result_binds_no_scalar`), with
`refused_scalar_bindings_are_exactly_these` exhibiting the surface as one list so
a future widening changes a theorem rather than slipping past it, and
`the_scalar_binding_table_is_inhabited` plus
`a_bridged_scalar_returning_contract_binds_its_result` keeping it non-vacuous
through the admission table rather than by postulate.

**One wave-19 theorem changed, deliberately.**
`refused_chains_rewrite_to_nothing`'s third conjunct was `x |> f |> g` — an
adapterless call stage feeding a later one — and that shape is now admitted. It
is replaced by a **sink** in a non-final position, which no widening of the call
vocabulary can reach, and the admitted shape is pinned separately as
`a_call_stage_may_feed_a_later_stage` with the two-argument composition as
`a_two_argument_scalar_composition_is_its_hand_written_sequence`. Both halves of
the old rule survive: `an_adapter_stage_still_produces_a_result` and
`a_sink_still_produces_nothing`. That theorem's own doc comment promised a future
widening would have to change it rather than slip past it; this is that
happening.

**Not proved**, carried over from `PIPE-CHAIN`: that the Rust extraction agrees
with the model (a tested Rust boundary), and that a callee's C implementation
satisfies its declared contract (`ContractTrust::Declared`, §C06).

### 4.5 Corpus neutrality

**Corpus-neutral by construction, verified structurally.** No file under
`crates/qcore-c-gateway/`, `crates/qcore-audit/`, `contract/qcore/` or
`providers/` is touched. The whole diff is:

```
crates/qcore-core/src/extern_call.rs        (the model and the checker)
crates/qcore-core/src/extern_c_emit.rs      (emission and reference execution)
crates/qcore-core/tests/unit/extern_call.rs (binding: None on 35 literals)
crates/qcore-frontend/src/source_extern.rs  (the source lowering)
crates/qcore-frontend/src/pipe.rs           (produces_a_result)
compiler/tests/extern_calls.rs, pipe_operator.rs
compiler/tests/fixtures/extern_calls/{scalar_result.qcore, buffer_api.c,
                                      contracts/buffer_api.qcore}
compiler/tests/fixtures/pipe/scalar_result_piped.qcore
docs/QCORE_SPEC_r0_3.md, docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md
proof/QCore/{ExternCall,PipeDesugaring}.lean, proof/Audit{ExternCall,PipeDesugaring}.lean
docs/reports/wave19-pipe-scalar-result/README.md
```

The contract-corpus pin, the 185-unit artifacts, the leak count of 272 and the CT
manifest are main's and none of them moves for anything here.

### 4.6 Diagnostics

**No code was added, moved or changed.** Three existing codes gained conditions,
all inside the `ExternCall` pass's own `Q022`–`Q027` band per §L21, plus one
condition change on the `Pipe` band's `Q037`. Every one was recorded in spec
§L21 and grammar §G02d/§G02h in the same change.

| Code | New condition |
|---|---|
| `Q024` | a bound result whose declared result type is not a custody-free scalar (a `unit`, or any pointer, including one declared `resource_return_none()`); a resource binding passed at a declared scalar parameter; a scalar argument this body never bound |
| `Q025` | binding an *acquiring* callee's result to a scalar local — the adapter stays the one spelling of an acquisition |
| `Q026` | a `let` whose name already means the other kind of thing (unreachable, §2.3) |
| `Q037` | "produces no value" narrows to **sinks**; a `unit`-returning callee's non-final refusal moves to the route's `Q024` |

`Q020` and `Q021` are untouched and stay separate (standing constraint 2). The
`Q023` default is unmoved. `source.extern-call.v1` is still not the inline
source-contract consumer.

**No reserved word was added**, as the brief expected. `let` and `move` already
exist and the scalar-argument form is a bare `NAME`; the constraint-14 keyword
census was therefore not needed and no fixture surface moves.

## 5. Premises and boundaries

Everything the extern route already premised is unchanged and unweakened:
`ExternAssumption::DeclaredContract` per callee, `NoExtentTransport`,
`LinkageSuppliedByBuilder`, and `UncheckedLinkage` for an uncontracted callee.
This card adds **no new premise** — which is the point of restricting the
admission to custody-free scalars. Specifically:

* An `unsafe extern fn` scalar result **is** bindable, and it was free to admit:
  the `UncheckedLinkage` premise is deposited per *callee* in
  `check_extern_program`, not per call site, so binding a result cannot launder
  it. `an_unchecked_scalar_result_binds_and_keeps_its_waiver` pins the premise
  identical between the binding and the discarding spellings, by name and by
  declared ABI. No manifest surface is touched — the concurrent
  `UNSAFE-LINKAGE-ARTIFACT` card owns those.
* Scalar **width** is not checked. An `i32` result may be passed at a `u32`
  parameter, exactly as an integer literal may be today: the profile models a
  scalar as an `i64` datum and the ABI spelling is transported for emission
  only. That is not new to this card — it is the pre-existing treatment of
  `ExternArgument::Data` — and it is named here rather than left implicit.
  Follow-on `EXTERN-SCALAR-WIDTH`.
* The number a bound call returned is **not observed** (§4.3).
* Fixture contracts are fixture contracts: `qcore_extern_scale` is a declaration
  in a test-fixture module, and standing constraint 9 applies — a per-function
  result here rests on those declarations and not on production ones.

## 6. Deviations, found by measurement

1. **The pipe's intermediate-stage rule was widened**, which the brief scoped in
   ("a pipeline stage whose callee returns a scalar must be able to feed the next
   stage"). It changes one measured refusal from `Q037` to `Q024` with a
   cascade; both are fail-closed, both name the callee, and the measured cascade
   is pinned as measured rather than trimmed (§6.1).
2. **`parse_bound_call` is tried second, never first.** An acquisition with a
   malformed adapter tail must keep its own `Q025`, so the scalar shape requires
   the `;` to close the statement immediately after the argument list. An
   adapter statement can therefore never be reinterpreted as a scalar binding
   and lose its diagnostic.
3. **A scalar literal `let n = 42;` is deliberately NOT admitted.** It is not a
   call result, it is what `STD-IO-COMPOSE` needs, and adding it here would have
   widened the surface past the brief. Follow-on `EXTERN-SCALAR-LITERAL`.
4. **The cross-namespace shadow refusal is unreachable** and is recorded as such
   rather than deleted or claimed (§2.3).
5. **`(void) vN;` is emitted after every scalar binding** (§4.3), because an
   unread binding is admitted and the native runs use `-Werror`.
6. **A scalar-result callee may still carry departing custody.** Nothing in the
   bridging table couples the result mode to the parameter modes, so
   `lower_scalar_binding` returns its `move NAME` arguments and the caller
   applies exactly the discarding-call arm's departure bookkeeping. The converse
   — a binding that takes part in custody — is what `binds_scalar` forbids.

### 6.1 The one pin update, in full

`a_stage_that_produces_nothing_cannot_feed_a_later_stage` (wave 19,
`PIPE-CHAIN`) asserted `["Q037"]` for two cases. It now asserts `["Q024",
"Q024"]` for a `unit`-returning intermediate stage and keeps the sink half at
`["Q037"]` in the same test. Two notes on the measurement:

* the second `Q024` is the cascade — after the first statement refuses, the
  second reads a temporary the first never bound. It is pinned as measured.
* one of the wave-19 cases used `qcore_extern_free`, which
  `round_trip_piped.qcore` does not declare; with the pipe no longer refusing
  first, it surfaced as `Q013` (unknown name). The case was rewritten to use a
  declared callee, so the test measures the rule it names.

## 7. Named follow-ons

| Name | What it is |
|---|---|
| `STD-IO-COMPOSE` | *(pre-existing, the other half of the acceptance sentence)* a module that both calls an extern function and prints. **What it still needs, now that this card has landed:** (a) route selection in `driver.rs` must stop sending a print-and-extern module to the extern route unconditionally; (b) the extern route needs `print`/`log`/`flush` statements, or the std-io route needs extern calls; and (c) `EXTERN-SCALAR-LITERAL` below, since `Q028` requires a rendering operand to be one of the entry's integer bindings and a chain's temporary is one only if scalar bindings exist on whichever route wins. This card supplies (c)'s *mechanism* on the extern route and none of (a) or (b). |
| `EXTERN-SCALAR-LITERAL` | `let n = 42;` on the extern route. Custody-free by inspection; deliberately out of this card, which the brief scoped to call results. |
| `EXTERN-SCALAR-OUTCOME` | observing the value a bound call returned, which needs a declared literal outcome control on the `succeeds` precedent so the reference interpreter can reproduce it. |
| `EXTERN-SCALAR-WIDTH` | checking a scalar argument's declared width against its source, rather than modelling every scalar as an `i64` datum (§5). |
| `SEQ-SCALAR-RESULT` | the same two forms on the plain sequential executable subset, whose `let` is `qcore_acquire(…)`-only and whose bodies admit no argument-taking call at all (§1). Strictly larger than this card. |
| `PIPE-TAP`, `PIPE-ASSIGN`, `PIPE-BINDER-SPAN`, `PIPE-STMT-STAGES`, `PIPE-STAGE-ARITY` | *(unchanged from `PIPE-CHAIN`)*. `PIPE-TAP` is now more attractive than it was: with scalar values in the language, "sound for scalars (duplicating a scalar is not aliasing)" has instances. |

## 8. Gate

| Gate step | Result |
|---|---|
| `git submodule update --init --recursive` | run in this fresh worktree before the closing measurement |
| `cargo test --workspace --no-fail-fast` | **2,998 passed, 0 failed, 8 ignored** — composed, see §8.1 |
| `cargo test -p qcore-compiler --test extern_calls` | 80 pass |
| `cargo test -p qcore-compiler --test pipe_operator` | 41 pass |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit **0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit **0** |
| `python3 scripts/check_lean_audits.py` | **80/80**, no new axiom, no new audit file |

Measurement outputs are worktree-private under `measure/` (gitignored), never
under `target/` and never in the shared scratchpad.

### 8.1 One environment finding, recorded rather than worked around

The closing `cargo test --workspace --no-fail-fast` reported **2,997 passed, 1
failed, 8 ignored**. The single failure is
`vp04_vp07_vp08_process_lifecycle_and_payload_budgets`
(`compiler/tests/cycle20_verifier_plugins.rs:794`,
`assertion failed: elapsed < Duration::from_millis(500)`) — one of the four rows
standing constraint 16 (`P02-FLAKE`) records as load-sensitive, and the one that
record describes as "wall-clock, measured failing at load 50 and passing at load
37 on the same binary".

The assertion was **not** weakened. Three sibling worktrees were building and
testing on the same machine throughout this run (`agent-ad36e80f46cea2d40`,
`agent-a30544ad3d1f43abf`, `agent-af9137889524579b3` all had concurrent
`cargo test`/`rustc` processes), which is exactly the load condition the record
names. Re-run in isolation:

```
cargo test -p qcore-compiler --test cycle20_verifier_plugins -- --test-threads=1 \
    vp04_vp07_vp08_process_lifecycle_and_payload_budgets
test result: ok. 1 passed; 0 failed; 0 ignored; 19 filtered out; finished in 1.48s
```

The composed figure is therefore **2,998 passed, 0 failed, 8 ignored**, and no
file this card touches is reachable from that target: the diff contains no
provider, host or process-transport file (§4.5).

For the integrator: because the machine was shared for the whole run, wall-clock
timings here are not comparable with a quiet run. Nothing about the results
depends on it.

## 9. Draft increment paragraph

> Scalar call results (PIPE-SCALAR-RESULT): `source.extern-call.v1` gains
> `let NAME = CALLEE(ARGS);` for a **non-acquiring call whose declared result is
> a custody-free scalar**, and a bare `NAME` at a declared scalar parameter, which
> together retire the first of the two boundaries card `PIPE-CHAIN` measured
> under `value |> render |> log`. The whole widening is one predicate,
> `binds_scalar`: the callee must acquire through none of the three sources — an
> out slot is the case a rule written over the result alone would have got wrong,
> since it declares `resource_return_none()` with a scalar status and still
> acquires — and its result must be neither `unit` nor a pointer, a pointer
> declared `resource_return_none()` included, because such a result is an address
> the declaration says nothing about. The mandatory nullable-owned-return adapter
> and every acquisition rule are byte-unchanged, so an acquisition keeps exactly
> one spelling. Scalars and identities are **separate namespaces** — `ValueId`
> beside `ResourceId`, separate maps, separate certificate fields, `v1` beside
> `r1` in the emitted C — and the linearity claim is proved rather than asserted:
> a scalar binding's ledger is **exactly the discarding call's**, nothing enters
> the ledger, and the bound name is never an identity of it. An unread scalar
> binding is therefore not a leak. The pipe's intermediate-stage rule is widened
> from "carries `succeeds`" to "is a call stage", still syntactic and still
> contract-free, which makes `x |> render |> scale` expressible and emits
> **byte-identical C11** against its hand-written statement sequence; a
> `unit`-returning callee in a non-final position still refuses, at the route's
> `Q024` naming its declared result rather than at the pipe's `Q037`, which is
> the same answer the hand-written sequence gets. An `unsafe extern fn` scalar
> result binds and carries the identical `UncheckedLinkage` premise, because that
> premise is per callee rather than per call site. Two measurements are recorded:
> the lowering's cross-namespace shadow refusal is **unreachable**, name
> resolution's `Q012` catching every duplicate local binding first, and it is
> kept as a fail-closed backstop with both halves pinned; and the plain
> sequential route had nothing to widen, its `let` being `qcore_acquire`-only
> (follow-on `SEQ-SCALAR-RESULT`). Ten new extern tests (70 → 80) and two pipe
> tests (39 → 41) with one wave-19 pin updated in both halves, one sanitized
> ASan/UBSan/LSan run as a **negative** control for the widening, 21 new audited
> Lean theorems with one `refused_chains_rewrite_to_nothing` conjunct
> deliberately replaced, 80/80 audits and no new axiom. **No diagnostic code
> added, moved or changed and no reserved word** — `Q024`, `Q025`, `Q026` and
> `Q037` gained conditions, recorded in spec §L21 and grammars §G02d/§G02h in the
> same change. Corpus-neutral by construction. Evidence:
> `docs/reports/wave19-pipe-scalar-result/README.md`.

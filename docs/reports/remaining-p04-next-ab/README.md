# P04-NEXT-AB — the call-result def/use vocabulary through the process boundary

Workstream W5 (Providers). Bounded increment: the ChunkIO **filesystem
cleanup-attempt** capability — five real functions decided by a call-result
def/use analysis rather than a path replay — now travels to a separate
schemas-only process as a third transported source vocabulary and comes back
as a witness the trusted core replays. Its two dependents (`file_outcomes`,
`file_delete`) are evaluated honestly and **not** moved; the smallest sound
piece of the dependent design — the prerequisite identity a dependent may
carry, and the core's re-establishment of it — is implemented and tested, and
the measured blocker for the rest is pinned in code rather than asserted in
prose.

Nothing is promoted: no raw Cycle 19 unit changes class, no CT case is mapped,
no contract-corpus count moves, and the rules that decide this contract are one
implementation shared by both routes.

- Worktree cut from `main` at **`05c51a5`** (wave-8 consolidation); branch
  `worktree-agent-acb705bbad4783a2b`.
- Libevent submodule initialized at the pinned revision.
- The in-process adapter is **retained** (invariant 3). Route removal is a
  later P09 unit.

---

## 1. Why a third vocabulary, and what it is

P04-NEXT's census classified `file_attempts` as *movable-with-a-third-
vocabulary and deliberately not taken*:

> Its evidence is a `PathCallUse` def/use analysis (which call results are
> ignored, which are guarded, and their operation order), not a path replay.

That classification is confirmed by construction. The two existing transported
vocabularies cannot express this question:

| Existing vocabulary | Why it cannot carry this question |
|---|---|
| Boolean / counter path projections | They carry a path so the receiver can *evaluate* it to a field state. A def/use reading is about **operation order and dependency**, which an evaluation result discards |
| Teardown projection (P04-NEXT) | Structurally close, but it has **no `ReturnValue` and no `LocalStart`**, and its call vocabulary is closed to one co-transported helper or exactly `free`. `int r = close(cf->fd); if (r == -1) return -1;` needs both missing forms and a different callee. Widening the teardown enums would weaken a family whose closed table is the whole point |

So `qcore-schemas::call_use_summary` defines its own enums and shares nothing
with either. That is the card's "third vocabulary".

### 1.1 The vocabulary, exactly

**Expressions** (`CallUseExpressionRecord`, 8 forms): `Parameter`, `Symbol`,
`Integer`, `String`, `Field`, `Cast`, `Unary`, `Binary`.

Refused **by name** at projection, each with its own message: `LocalStorage`
(an address-taken stack slot), `LoopIndex`, `SizeOf`, `Aggregate`, `Index` (a
subscript), `Function` (a callee designator used as a value).

**Operations** (`CallUseOperationRecord`, 9 forms): `Read`, `Write`,
`LocalStart`, `LocalWrite`, `LocalEnd`, `Guard`, `Call`, `ReturnValue`,
`Return`.

Refused **by name**: `ScanLoop` (the in-process analyzer refuses structured
loops too, so this is not a new restriction) and `Terminate` (a declared
nonreturning call is not a complete path in this family).

**Result bindings.** A call's result is identified by its operation index. It
becomes observable only through a `Symbol { operation }` in a later operation's
own operands — that is the single form the reading follows. `LocalWrite` is the
`int r = f(...)` binding; `Write` is the store-to-field binding; `ReturnValue`
is the returned binding.

**Uses.** A use is any later operation of the same path whose operands depend
on the produced value: storing it, passing it, returning it, computing with it,
or guarding on it. A guard use is additionally recorded as such. Reloads
through memory are **not** inferred to retain a stored result's identity —
matching the receiver's own analyzer exactly.

**Branch shapes.** Exactly one: `Guard { condition, taken }` on a complete flat
path. There is no other branch form, and loops have no representation at all.

**Callees.** Admitted only against an explicit **closed callee table** matched
on both name and declared C signature, supplied to the importer and recorded on
the sealed import. Per selected function:

| Function | Admitted callees |
|---|---|
| `cio_file_close` | `munmap_file`, `cio_file_native_close`, `free` |
| `munmap_file` | `cio_file_native_unmap` |
| `cio_file_native_close` | `close` |
| `cio_file_native_unmap` | `munmap` |
| `cio_file_native_delete` | `unlink` |

There is no catch-all and no general call vocabulary. A body calling anything
else keeps its existing `Unknown` — pinned by a test that replaces
`free(cf->path)` with a declared `cio_free(cf->path)`, and by one that supplies
`free` at the wrong declared signature.

### 1.2 What actually crosses the boundary

The request carries `CallUseSourceInput`: a schema version plus one
`CallUseFamilyInput::FilesystemAttempt { body }`, where `body` is a
`CallUseFunctionRecord` (function, source revision, imported C signature,
complete path set, declared profile assumptions).

**No derived result travels, and neither does the policy.** This is stronger
than the teardown route, which had to transport the destructor's declared
owned-field inventory: the def/use reading is *policy-free* — every call of
every path is reported — so the contract's policy selection (which callee is
counted, which second callee, which signature) never leaves the trusted core.
A producer therefore cannot answer a narrower question than the one the core
evaluates. Three tests assert this over the real transported request: the
serialized projection contains `paths` and contains none of `uses`,
`guard_uses`, `calls`, `cleanup_calls` or `verdict`.

The process (`providers/common/call_use_effect.rs`, shipped in `qcore.chunkio`
**0.7.0**) walks the transported paths on its own and returns a
`CallResultUseRecord`: for each path, every call in operation order with the
operation indices that observe its result and the subset that are guards.

The core (`qcore-verification::call_use_source`) then:

1. refuses any task whose target identity, dialect, revision, platform or Clang
   target does not match its own import;
2. refuses any policy whose counted callees are not all in the closed table the
   projection was taken under (otherwise the policy would count zero calls to a
   callee the table could never have projected — a silently smaller question);
3. **evaluates the pinned contract itself** with `check_call_result_use` over
   its own summary; and
4. requires the proposal to equal the reading it computed from that
   certificate.

Acceptance terminates in the core's own evaluation. Agreement alone discharges
nothing, and a Safe proposal over a false contract is refused (§5, §6).

### 1.3 Obligations and identity

One obligation per subject function, under a new property
(`CallResultUseContract`), scope `Functions { the one symbol }`, bounds
`PathBounds::BoundedPaths { paths }` (these bodies are multi-path, so the exact
path count is bound into the task).

`contract_sources` binds five digests:

| Key | Binds |
|---|---|
| `qcore:call-use-source-input:v1` | the complete projection |
| `qcore:call-use-import:v1` | the importer's captured source/build identity |
| `qcore:call-use-contract:v1` | the pinned contract text |
| `qcore:call-use-policy:v1` | the selected policy (which callees the facts count) |
| `qcore:call-use-admitted-callees:v1` | the closed callee table the projection was taken under |

The last two are new relative to the teardown route and are load-bearing: a
wider table over the *same body* is a different task, and neither task's
witness satisfies the other (§5).

### 1.4 What is **not** established

A recorded `close`, `munmap` or `unlink` call is a recorded call. This route
establishes nothing about whether a descriptor was released, whether a mapping
was torn down, whether a file was deleted, or whether any OS resource changed
custody. The pinned contracts' `requires` premises (live exclusive record
provenance, exact compiled bindings, returning external-call frames) stay
declared premises and are recorded in each task's assumptions. The certificate's
own `status` says so in words, and that sentence is now one shared constant.

---

## 2. One decision procedure, not two

The generic decision procedure was **relocated** from the in-process ChunkIO
adapter into `crates/qcore-verification/src/call_use_contract.rs`
(`check_call_result_use`), and the adapter now selects the policy and
delegates. This is deliberate: P04-NEXT could share `check_list_unlink` and
`check_field_destructor` because they already lived in `qcore-verification`;
this capability's checker lived in the compiler adapter, so moving the route
without moving the checker would have created **two** implementations of the
same contract that could drift.

The relocation is mechanical. The fact computation moved verbatim; the four
edits are:

1. the tuple destructure became a struct destructure of `CallResultUsePolicy`;
2. `call.function == first` became `&call.function == first` (the shared policy
   type owns `String`s);
3. `crate::contract_observation::check_contract_facts` became
   `qcore_contract::contract_observation::check_contract_facts` — the same
   function through its own crate path;
4. the status literal became a named constant with the same bytes.

**The adapter's own type is untouched.** `FileAttemptEvidence` keeps its four
fields, their order, its `Serialize` derive and its private fields, so the
serialized shape the pinned Cycle 19 chunk-contract artifact carries cannot
have moved. A parity test asserts the four keys still appear in declaration
order in the serialized text.

One behavioural difference is recorded rather than hidden: `check` now selects
the policy *before* running the analyzer, so a body that is both an unsupported
function **and** contains a scan loop now reports
`unsupported filesystem result-use source` where it previously reported the
analyzer's loop message. Both are refusals; no accepted input changes; the
error is not reachable in any audited configuration (`check` is private and
every caller passes one of the five selected names). It is named here because
it is the only ordering change in the relocation.

---

## 3. Wire and schema versions

Per `crates/qcore-schemas/README.md`'s documented policy, three of the four
additions are **source-incompatible** for exhaustive matches and struct
literals:

| Addition | Kind |
|---|---|
| `SemanticWitness::CallResultUse` | new variant on an exhaustively matched enum |
| `ObligationProperty::CallResultUseContract` | new variant on an exhaustively matched enum |
| `VerificationRequestV2::call_use_source` | new required struct field (skipped when absent) |
| `call_use_summary` module | purely additive |

Additive was preferred and is not available. No existing witness variant can
hold this reading: `TeardownContract` carries alias-class or nullable-state
cases, `CounterContract` a single-path unsigned update, `ContractObservation` a
recorded-run record set. Reusing `NormalizedFact::subject` would be a strictly
weaker boundary than the typed field the four earlier projections established
(no `deny_unknown_fields`, no schema-version field, no `validate()` rule).

**Flagged prominently: this is a major schema bump.**

| Boundary | Before | After | Why |
|---|---|---|---|
| `qcore-schemas` crate | 7.0.0 | **8.0.0** | Exhaustive-variant precedent from 2.0.0, 4.0.0, 5.0.0, 7.0.0 |
| Plugin protocol | 1 | 1 | Unchanged |
| Evidence version | 1, 2 | 1, 2 | Unchanged |
| Boolean / counter / reactor / teardown projections | unchanged | unchanged | Untouched |
| Call-use source input | — | 1 | New |
| Call-use semantic profile | — | `qcore.c_call_result_use.v1` | New |
| Call-use input digest key | — | `qcore:call-use-source-input:v1` | New |
| Prerequisite identity digest key | — | `qcore:prerequisite-identity:v1` | New |
| ChunkIO package | 0.6.0 | **0.7.0** | Adds the call-result def/use route |

Existing wire bytes are unchanged: `call_use_source` is an `Option` that
defaults to absent and is skipped on serialization, exactly like the four
projections before it. `crates/qcore-schemas/tests/call_use_request_wire.rs`
pins that a request carrying none of the **five** projections serializes with
none of the five keys and round-trips, and
`teardown_request_wire.rs`'s existing pin is extended with the fifth key.

**Deliberate pin updates:**

- `providers/conformance/tests/packages.rs` pinned the ChunkIO package manifest
  at 0.6.0; it now pins 0.7.0. The new profile is reached through the `chunkio`
  target, whose profile list is already `*`, so the certificate checker, Lean
  family, capabilities, limits, trust class and every other target entry are
  unchanged.
- `providers/conformance/tests/composed_five_providers.rs` pinned the same
  version in its shipped-package inventory; it now pins 0.7.0.
- `compiler/tests/fixtures/w3/provider/{Cargo.toml,Cargo.lock}` require
  `qcore-schemas` 8.0.0 so the pinned standalone-provider fixture's `--locked`
  build resolves.

No gate was weakened and no expectation was deleted.

---

## 4. Parity evidence (old route retained)

`compiler/tests/call_use_route_parity.rs` (3 tests) imports the same C bodies
through both routes.

- **Certificate parity, all five selected bodies.** The serialized
  `FileAttemptEvidence` from `check_file_attempt_sources` /
  `check_file_delete_source` **equals** the serialized `CallResultUseCertificate`
  that `FileAttemptSourceTask::evaluate()` recomputes, for
  `cio_file_close`, `munmap_file`, `cio_file_native_close`,
  `cio_file_native_unmap` and `cio_file_native_delete`. The test asserts the
  count is exactly 5, so a silently skipped body fails it.
- **Refusal agreement.** Four changed bodies the in-process route refuses are
  refused by the moved route too — a guarded `cio_file_native_close` result, a
  dropped `free(cf->path)`, a removed `munmap_file` call, and a native close
  that stops guarding its `close` result — each with the in-process refusal
  asserted first as a control.
- **Serde shape.** The adapter's four keys still serialize in declaration
  order, and its `status` still says `Unknown`.

Parity at corpus scale is §7.

---

## 5. Mutation and refusal matrices

Measured from the test sources, not estimated: **122 refusal rows across three
layers, all refused.** No mutation was admitted, so no gate was changed and no
repair was required by the matrix itself.

### 5.1 Core-side (`crates/qcore-verification/tests/call_use_source.rs`, 9 tests) — 83 rows

**33 forged readings** driven through the core checker. The matrix runs 18
distinct forgeries against **two** bodies — the wrapper, whose four calls all
ignore their results, and the native close, whose single result is stored and
guarded — because several forgeries are vacuous on one of them and must bite on
the other. Each row is asserted to actually differ from the accepted reading
before it is submitted, and the applied-row count is pinned at exactly 33, so a
forgery that quietly becomes vacuous on both bodies fails the test.

The eighteen: emptied paths; a dropped path; a duplicated path; reordered
paths; a renumbered path; an erased call; an invented call; a renamed callee; a
moved call operation index; reordered calls within a path; a claimed use that
does not happen; a hidden real use; a claimed guarded result; a changed result
type; a renamed function; a moved revision; a changed signature; an invented
profile assumption.

**10 false-contract rows.** Every `ensures normal` predicate of the wrapper
policy, replaced by `false` and by an undeclared predicate. The honest reading
a producer would compute cannot discharge either, because the core evaluates
the contract itself. The row count is pinned exactly.

**4 identity and binding refusals:** a witness bound to another task; a
well-formed reading of another real function rebound to this task digest;
another witness kind (`ContractObservation`); a task the checker did not derive
even with a real witness.

**14 derivation refusals:** six incompatible target identities (another
profile, `c99`, a moved revision, a foreign platform, an emulated backend, an
empty library); an empty translation unit; an empty contract; and six policy
shapes — empty policy name, empty counted callee, empty signature, a signature
that is not the imported body's, and two policies counting a callee the closed
table could never project.

**4 changed-input refusals:** a narrower callee table refuses the projection
outright; a *wider* table over the same body is a different task and the two
witnesses do not cross **in either direction**; changed source bytes are a
different task whose witness is refused.

**7 vocabulary and selection refusals:** an unadmitted callee (`cio_free`); a
callee supplied at the wrong declared signature; a body that subscripts an
array; an empty function name; an empty callee table; a table entry with an
empty name; a table that repeats an entry.

**7 prerequisite refusals:** each of the three identity fields replaced and
each emptied, plus another real function's identity.

**4 request-admission refusals:** an unsupported projection version; an absent
projection with a live obligation; another family's obligation property; an
obligation naming a different function.

### 5.2 Provider-side (`providers/common/call_use_effect_tests.rs`, 6 tests) — 27 rows

The two fixtures are the **real projections** the gateway produces for these
bodies (`providers/common/fixtures/*_call_use_projection.json`), and a
conformance test asserts the live import still equals them, so the matrix
cannot drift into a shape the importer no longer emits.

- **Accepted readings:** the wrapper's four calls in order
  (`munmap_file`, `cio_file_native_close`, `free`, `free`) with strictly
  increasing operation indices and every result ignored; the native close's
  single `close` per path with `uses = [2, 3]` and `guard_uses = [3]`.
- **22 declined projections** (11 mutations × 2 fixtures, count pinned): wrong
  schema version; schema version zero; empty function name; empty revision;
  empty signature; no paths; an empty path; more paths than the cap; a path
  longer than the cap; an empty callee name; an expression deeper than the
  depth cap. The depth cap exists so a hostile projection cannot exhaust the
  stack; it declines rather than recursing.
- **5 request-gate declines:** another profile; another dialect; another
  capability; an absent projection; an unsupported projection version.
- **Precision, not silence:** a body that stops guarding its result stays
  *readable* and the reading **changes** — so the core's comparison and its own
  contract evaluation see the change rather than a normalized result.

### 5.3 End-to-end through the real built package (`providers/conformance/tests/call_use_capability.rs`, 2 tests) — 9 rows

`python3 providers/package.py build` is run, the real `qcore.chunkio` package is
registered as a `ProcessProvider`, and:

- **positive, three bodies:** `ProposedVerdict::Safe` for the native close (with
  the exact `uses`/`guard_uses` vectors), the wrapper, and the guarded deletion
  attempt under its own contract; certificate bound to the task; witness kind
  `call_result_use`;
- the transported request contains `paths` and **no** derived key;
- **missing evidence:** no core checker → Unknown with QP016 and no
  certificate; no registered provider → Unknown; the shipped `qcore.libevent`
  package, which does not implement this capability → Unknown with no
  certificate; disabled provider → Unknown;
- **tampered transport:** a well-formed body the core never imported (both
  guards removed) is refused by the host's digest binding with **QP001**;
  rebinding it to a recomputed task digest gets past that and is then refused
  by the core's own comparison, with no certificate;
- **false contract:** a Safe proposal over a contract asserting
  `ensures normal false` is rejected with **QP009**;
- changed source bytes and changed build options each retire the checker's
  context.

---

## 6. B — the dependents, evaluated honestly

`file_outcomes::check_file_outcome_sources` and
`file_delete::check_native_delete_source` consume `file_attempts`' certificate.
P04-NEXT classified both as **blocked-by-dependency**. That classification is
**incomplete**, and the correction is this card's main negative finding.

### 6.1 The design, specified

A dependent capability's request must never carry a prerequisite *certificate*
— nothing a producer sends may be read as a prerequisite conclusion. It carries
only an identity triple:

```rust
pub struct PrerequisiteIdentity {
    pub semantic_profile: String,   // which question was answered
    pub task_digest: String,        // the prerequisite task's own digest
    pub obligation_id: String,      // its single obligation
}
```

and the core:

1. re-derives the prerequisite task **from its own import**, with its own
   policy and its own contract;
2. computes the identity that task has;
3. refuses unless the claimed triple *is* that identity; and
4. re-runs the prerequisite's **own contract evaluation**, and refuses if it
   refuses.

So a dependent's admission is bought entirely by work the core does. The claim
is a lookup key, not evidence. The identity is bound into a dependent task's
`contract_sources` under `qcore:prerequisite-identity:v1`, so a dependent whose
prerequisite moved is a different task.

### 6.2 What is implemented and tested

`FileAttemptSourceTask::prerequisite_identity()` and
`revalidate_prerequisite()` are production code with 10 pinned rows:

- the identity serializes to exactly three fields and contains none of
  `verdict`, `safe`, `certificate`, `witness`, `cases`, `calls`, `paths` or
  `reading`;
- each of the three fields replaced, and each emptied, refuses (6 rows);
- another real function's identity refuses;
- two real imports of the same function differing only in source bytes have
  different identities and neither validates against the other's context
  (2 rows);
- re-establishing a prerequisite whose contract is false **refuses**, proving
  step 4 is not decorative.

Recorded honestly: **this API has no production consumer yet**, because no
dependent has been moved. It is exercised by tests, in the same position P05's
moved reactor capability occupies.

### 6.3 Why the dependents are not moved — measured

Both dependents are decided by `qcore-verification::cleanup_outcome`, whose
shape was measured rather than assumed:

| Measurement | Value |
|---|---|
| `cleanup_outcome.rs` total | 611 lines |
| The `Replay` engine (struct + impl) | **268 lines** |
| `check_cleanup_outcomes` case loop | 110 lines |
| `check_guarded_delete_outcomes` case loop | 119 lines |
| Symbolic value lattice (`Value`) | **9 constructors** |
| `CleanupProfile` declared fields | 11 |
| `CleanupCase` / `GuardedDeleteCase` fields | 9 each |
| Sampled `(entry, outcome)` domains | 4 and 5 |

Two obstacles, in order of decisiveness.

**(a) A transported `CleanupProfile` would have no contract-side referent.**
The teardown card could transport the destructor's owned-field inventory
because `memory_backend.qcore` *declares* it: `declared_owned_fields` extracts
exactly that set from the contract's `requires owned_or_null__…` clauses, and
the core refuses a projection naming any other set. The cleanup-outcome
contracts declare nothing of the sort. A pinned test tokenizes both contracts
into whole identifiers (line comments stripped) and asserts they contain none
of `fd`, `map`, `alloc_size`, `map_truncated_warned`, `path`, `close`,
`munmap`, `unlink`, `__errno_location`, `cio_errno_print`, `char`, `size_t` or
`owned_or_null__`, while the destructor contract still does declare its
inventory and both contracts do contain real identifiers (so the check is not
vacuous).

The profile is therefore **Rust-only declared data**, and it is
custody-bearing: its `ResetValue`s assert what a successful syscall leaves
behind (`fd = -1`, `map = NULL`, `alloc_size = 0`). Putting that on the wire
with nothing to cross-check it against is exactly the widening P04-NEXT's
closed-callee-table discipline exists to prevent.

One thing measured and recorded rather than glossed: `filesystem_delete.qcore`
*mentions* `fd`, `map` and `path` in its header prose ("The fd/map fields
control eligibility; they are bookkeeping, not proof of current OS custody").
Prose is not a declaration — `declared_owned_fields` reads clauses, and nothing
can read a comment — so the blocker stands, and the test strips comments
deliberately with that reasoning recorded beside it.

**(b) It is a fourth vocabulary, not a widening of the third.** The reading a
producer would have to compute (`CleanupCase`) is not syntactic: `field_writes`,
`fields_preserved` and `fields_reset` are functions of a symbolic *field state*
evolved by a 9-constructor value lattice over four (respectively five) sampled
`(entry, syscall outcome)` prestates, with the syscall's argument identity
checked against the initial field values. That is a symbolic replay evaluator,
not a def/use fold. By P04-NEXT's own rule — "a producer reimplementing the
algorithm over a new transported vocabulary is a separate capability
definition, not a widening of this one" — that is a separate card.

**Correction to the census:** `file_outcomes` and `file_delete` are *not*
merely dependency-blocked. Both blockers are independent of the dependency, and
(a) is not solvable by transporting more: it needs the contract to declare the
field/reset vocabulary first. Recorded as follow-on **P04-NEXT-B1** (declare the
cleanup field/reset vocabulary in the pinned contracts so a profile has a
referent) and **P04-NEXT-B2** (the symbolic cleanup-replay vocabulary), in that
order. The dependency mechanism itself (§6.1–6.2) is no longer a blocker.

---

## 7. Custody and audit invariants

No allocation, descriptor or mapping custody path was touched. `cleanup_outcome`,
`file_outcomes`, `file_delete`, `file_close`, `unmap_wrapper`, the `*_close`,
`nested_*`, `traversal`, `parent_teardown` and `teardown_source` routes are
**unedited**. The edits to existing production files are: a new schemas module,
a new optional request field that serializes away, one witness variant, one
obligation property, one `predicate_for` arm, one host digest check, the
mechanical relocation of §2, and a new gateway/verification/compiler module
each.

### 7.1 The 185-unit corpus, re-measured before and after

Re-measured through the canonical pinned route on a freshly prepared pinned
clone (Fluent Bit `66910c10a4d7eafa810b84229cf2dcf7b0f26f97`, explicit
`/usr/bin/clang` 21.1.8), **from the same worktree directory on both sides**,
with the provider packages rebuilt at each side:

```sh
python3 -B scripts/prepare_cycle19_clang_audit.py \
  --fluent-bit /home/cosmo/Documents/GitHub/fluent-bit \
  --revision 66910c10a4d7eafa810b84229cf2dcf7b0f26f97 \
  --clang clang --clangxx clang++ --output build/cycle19/clang/p04next-ab
QCORE_CLANG=/usr/bin/clang QCORE_CHUNKIO_PACKAGE=<packages>/qcore.chunkio \
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  --pinned-evidence build/cycle19/clang/p04next-ab/fluent-bit-clang-source \
  build/cycle19/clang/p04next-ab/monkey/monkey-build/compile_commands.json \
  <out>/monkey-source.json <out>/resource-diagnostics.json
```

| Run | Result |
|---|---|
| Base `05c51a5` | 185 units, 4,921 source functions: **2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only**, 0 gateway failures |
| This branch | 185 units, 4,921 source functions: **2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only**, 0 gateway failures |

`diff -r before/ after/` over all five artifacts reports **12 lines, in one
file**: `resource-diagnostics.source-effects.json`'s six recorded
`package_digest`/`provider_version` pairs, moving `0.6.0` → `0.7.0` as this card
deliberately bumped the ChunkIO package. `resource-diagnostics.json` (11 MB),
`resource-diagnostics.chunk-contracts.json` (4 MB),
`resource-diagnostics.contracts.json` and `monkey-source.json` are
**byte-identical**. The complete diff is archived as
[`corpus-185-unit.diff`](corpus-185-unit.diff), the per-library headlines as
[`corpus-185-unit-headlines.txt`](corpus-185-unit-headlines.txt).

Following CORPUS-REPIN's recorded deviation, the pinned route reads
`<clone>/build/compile_commands.json`, so the prepared CMake database was
copied there verbatim (sha256
`77b33c7695f93a73ac018933f6e60059cc53d0f64b9d4397ab2c2140f005d1ec`; Monkey
`2e577907fb7c1812b5d37d48127416ea09bb0058ed1b72fcb7c034d7a15c0580`). No gate
was bypassed and no command was rewritten.

**Harness validation.** The *before* run was additionally compared against the
archived wave-8 baseline (`docs/reports/remaining-e01-f1/artifacts/after/`):
`diagnostics.json` and `chunk-contracts.json` are **byte-identical** to the
archive, and `contracts.json` differs only in the three mechanical
absolute-path-derived fields CORPUS-REPIN recorded. `source-effects.json`
differs further because its source-import identity digests bind the compile
working directory, and this card ran from a different worktree than the
archive — location provenance, not analysis. That this is provenance and not
drift is settled by the before/after comparison above, which was run from one
directory and moved only the package identity.

**The archived baseline is not re-pinned by this card**: it is reproduced
byte-identically where analysis is concerned, so whatever baseline `main`
carries at merge time is unaffected.

### 7.2 The canonical CMetrics audit

Re-measured before and after on the same prepared build:

```
diff cmetrics-audit-before.json cmetrics-audit-after.json
46c46    "package_digest": 837d4771… → 4c56b6e6…
135c135  "package_digest": 837d4771… → 4c56b6e6…
```

Two lines, both the rebuilt `qcore.cmetrics` executable's digest — the same
effect REASON-CODES and P04-NEXT measured when they rebuilt the packages. Every
verdict, diagnostic, source digest, compile-task identity and the
checked/Unknown split are byte-identical: **one checked function contract, six
Unknown functions, and a provisional Unsafe raw unit**. Both reports and the
diff are archived beside this file.

**Nothing is promoted.** Raw Cycle 19 units remain 2 Safe / 42 provisional
Unsafe / 137 Unknown plus four data-only units; the contract corpus remains 60
modules; CT coverage remains 28/50 mapped; full language conformance remains
false.

---

## 8. Lean inventory

`proof/QCore/CallUseAdmission.lean` and `proof/AuditCallUseAdmission.lean` add
**28 audited theorems** in a new `QCore.CallUseAdmission` namespace, with **no
new axiom** (only `propext`, `Classical.choice` and `Quot.sound`, the allowed
closure). Two things are modelled and nothing else:

- *the vocabulary is closed* (9): scan loops, nonreturning terminations and
  expression forms outside the closed list have no projection; a callee outside
  the closed table has no projection; one refused operation refuses the whole
  path, from either end, and a scan loop anywhere in a path refuses it; the
  table is monotone (a narrower table never admits more) and load-bearing
  (there is a body one table projects and another refuses).
- *invariants of the reading* (3): every guarded use is a use — established as
  an invariant of the fold by induction, so it holds for **every** path in the
  vocabulary rather than for sampled ones; a call-free path reads as no calls.
- *non-vacuity* (3): the two real body shapes read exactly as the audit
  measured them (`uses = [2,3]`, `guard_uses = [3]`), and dropping the guard
  changes the reading rather than normalizing it away.
- *admission* (8): a refusing core admits nothing; every admission is backed by
  an evaluation the core ran; an admitted proposal **is** the core's reading;
  two admitted witnesses cannot disagree; changing what the core computes
  retires the witness; cross-task and foreign-task refusals; and non-vacuity.
- *the dependent boundary* (5): a foreign prerequisite is refused; a
  prerequisite whose own evaluation refuses can never be established whatever
  identity is claimed; every field of the identity is load-bearing; the core's
  own derived identity is established; and a prerequisite does not cross tasks.

Judged **not applicable** and recorded rather than modelled: the transported
vocabulary's faithfulness to Clang's typed AST (a gateway/importer premise, as
on every existing source route), and the contract-evaluation body itself, which
is the relocated `check_call_result_use` and is covered by the executable
parity of §4 and §7.

---

## 9. Gate results

Run in this worktree on Linux with Clang available:

| Gate | Result |
|---|---|
| `cargo test --workspace` | **1807 passed, 0 failed, 3 ignored** — against main's 1787 baseline, exactly +20 |
| `cargo fmt --all -- --check` | clean (workspace and `providers/`) |
| `cargo clippy --all-targets -- -D warnings` | clean (workspace and `providers/`) |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 -B scripts/check_lean_audits.py` | **45 audits passed**, no new axiom |
| `cargo test --manifest-path providers/Cargo.toml --workspace` | **75 passed, 0 failed** (67 baseline + 6 provider unit + 2 conformance) |
| `python3 -B -m unittest discover -s scripts/tests` | 108 passed |
| `python3 providers/package.py build` + `qcorec plugins verify` × 5 | all five packages built and verified (exit 0) |
| Canonical CMetrics audit, before vs after | 2 lines, both rebuilt `package_digest` (§7.2) |
| 185-unit corpus, before vs after, same directory | 12 lines, all the deliberate package bump; four artifacts byte-identical (§7.1) |

The +20 workspace tests are 9 core replay, 3 dependent-boundary, 4 schema wire,
3 parity, and 1 gateway `compile_fail` doctest.

**Known load-flaky, observed and recorded.** An intermediate full run failed
`vp04_vp07_vp08_process_lifecycle_and_payload_budgets`
(`compiler/tests/cycle20_verifier_plugins.rs`, a 500 ms wall bound) under
parallel load; it passed in isolation and passed in the final full run above.
Its assertion is deliberately **not** weakened. The baseline run at `05c51a5`
before any change failed the P02 `RLIMIT_NPROC` conformance test the same way;
that flake did not reproduce in the final run.

---

## 10. Open items and named blockers

- The in-process adapter for the moved capability is retained. Its removal,
  after this parity record, is a later unit (P09).
- **The moved capability has no configured audit route yet.** It is exercised
  by the parity, replay and conformance tests, not by the Cycle 19
  compatibility adapter, so no production report changes and no raw unit moves.
  The in-process route is what the audit still runs — and after §2 both run the
  same decision procedure, which is why the artifacts are byte-identical.
- **P04-NEXT-B1** — declare the cleanup field/reset vocabulary in
  `filesystem_outcomes.qcore` and `filesystem_delete.qcore` so a transported
  `CleanupProfile` has a machine-checkable referent. This must come first; it
  is a contract-corpus change with pin updates.
- **P04-NEXT-B2** — the symbolic cleanup-replay vocabulary (the fourth
  vocabulary) and the move of `file_outcomes` / `file_delete`. The dependent
  mechanism they need is implemented (§6.2); these two blockers are what remain.
- **P04-NEXT-AB-CONSUMER** — `PrerequisiteIdentity` and
  `revalidate_prerequisite` have no production caller until a dependent lands.
- `traversal` and `parent_teardown` stay blocked by the sealed `SourceMatch`
  boundary (**P04-NEXT-C**), and the seven composition modules can only follow
  their dependencies. Unchanged by this card.
- The route binds no profile-declared model library: it selects functions by
  explicit name, as the in-process route does, so `target.library` is checked
  for presence but not against a declared model (**P04-NEXT-D**, unchanged).
- The closed callee table is a caller assertion recorded on the import and bound
  into the task. The core checks that the policy's counted callees are all in it,
  but it does not derive the table from the contract — there is no contract
  vocabulary for callees. That is the same class of gap as B1 and is recorded
  here rather than hidden.
- `LocalStart` is representable in the vocabulary but does not occur in any of
  the five projected bodies. It is kept because it produces no symbol and
  refusing it would be arbitrary; it is named here so the vocabulary's
  inhabited fragment is not overstated.
- The relocation's one ordering change is recorded in §2.

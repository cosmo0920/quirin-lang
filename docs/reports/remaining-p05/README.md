# P05 — One Libevent/Monkey callback capability through the process boundary

Workstream W5 (Providers). Bounded increment: the reactor **callback assignment
inventory** — descriptor-backed callback registration, its event generation and
its declared callback authority — now travels to a separate schemas-only
process as a program projection and comes back as a witness the trusted core
replays. Nothing is promoted: no raw Cycle 19 unit changes class, no CT case is
mapped, no contract-corpus count moves, and the model checker that decides the
question is byte-identical to the one that already ran in process.

- Branch checkpoint before this increment: `9478b94` (worktree
  `worktree-agent-a3e43a1115b177966`, branch `worktree-agent-a3e43a1115b177966`).
- Baseline reproduced before any change: the workspace test suite passed with
  the Libevent submodule initialized at the pinned `d82464a2…`.
- The in-process route is **retained**, not deleted (acceptance 3). Route
  removal is P09's job.

## 1. Capability inventory: what ran only in process

`compiler/src/verifier_plugins/libevent/` holds four capability modules plus the
manifest loader. Each was read and classified by *which core checker decides it*
and *what a process would have to be able to say*:

| In-process route | Core checker | Status |
|---|---|---|
| `event_driven::check_event_program` (callback assignment inventory) | `qcore_core::event_reactor::check_event_program` | **Selected — moved by this card** |
| `event_driven::check_observation_refinement` | `qcore_core::event_reactor::check_observation_refinement` | Deferred: an observation witness claims *recorded native runs*, which needs a `NativeRunBinding` and the observed-run bounds discipline, not a program projection. That is a different witness kind with its own trust argument |
| `event_driven::verify_pinned_libevent_source` | `qcore_contract::profile_loader` | Structurally blocked: it verifies a checkout pin and digests of files on disk. A process cannot propose a source audit without being handed the tree, which is a filesystem-input capability this card does not open |
| `monkey_binding::check_monkey_bindings{,_with_rules}` | Monkey-specific composition in `external_integration.rs` | Deferred: relates callback bindings to Monkey wrapper roles and callee bounds; it consumes other summaries, so it can only follow its dependencies |
| `monkey_source`, `monkey_observation` | `typed_source_match` / observation replay | Blocked on the sealed `SourceMatch` boundary (W5 typed-body boundary) and on the observation-witness question above |

### Why the callback assignment inventory

It is the smallest well-bounded candidate that is genuinely a capability rather
than a composition: one program, one complete assignment inventory, one model
checker, no dependency on another certificate, no native observation, and no
filesystem input. It is also the capability the card names — descriptor
generations, callback registration/deregistration and parent custody are
exactly what an `Assign` record and the surrounding program carry.

`qcore-schemas::reactor` and `qcore-verification::reactor_callback` already
supported a *record exchange* for this inventory (see
`docs/reports/w5-reactor-callback-records/README.md`), but it had no
evidence-v2 task, no obligation, no witness kind and no provider package: the
existing fixture was a schemas-only child process invoked directly by a test,
outside the plugin protocol entirely. This card supplies the missing half.

**Honest boundary on "production":** unlike P04's counter helpers, the
in-process `libevent::check_event_program` route has **no live production
caller** — the Cycle 19 compatibility adapter does not build `EventProgram`s,
and the route is exercised by the Cycle 18 unit tests. So the parity question
here is real but not production-live. This card does not change that, and does
not claim to.

## 2. What actually crosses the boundary

The request carries `ReactorProgramInput`: the declared program in a bounded
operation vocabulary, the declared library and source revision, the opaque
declared build attributes, the live callback arguments and the profile
assumptions. **No derived result travels with it** — no inventory, no
certificate, no verdict — exactly as the counter route sends a path and not a
reading.

The vocabulary is deliberately narrower than the reactor model. There is no
record at all for bufferevent operations, raw socket-descriptor custody
(`CreateSocketPair` / `GetSocketError` / `CloseSocket`), ordering assumptions or
loop control, and no `Bufferevent` callback context. A program using any of them
cannot be projected, so its callbacks stay **Unknown** — there is no catch-all
variant and no partial projection. An empty accepted-build list is an
`UnsupportedBackend` refusal, never acceptance.

The process (`providers/common/reactor_callbacks.rs`, shipped in
`qcore.libevent` 0.2.0) walks that program on its own and returns a
`ReactorCallbackProposal`: which operations are callback registrations, at which
index, for which event identity and generation, and with which declared callback
authority. It runs no reactor model and is given no inventory to echo.

The core (`qcore-verification::reactor_source::ReactorCallbackTask`) then:

1. refuses any task whose identity does not equal its own derived task;
2. binds the proposal to its own program's **complete** assignment inventory
   with the unedited `check_declared_callback_proposal`, so a missing, added,
   duplicated, retargeted or authority-widened record is refused before the
   model is consulted at all; and
3. **runs `check_event_program` itself** over its own program.

Acceptance therefore terminates in the core's own model run. Agreement alone
discharges nothing, and a Safe proposal over a program the model refuses is
rejected (exercised below). The declared program, its callback authority and the
accepted build remain explicit premises, recorded in the task's assumptions —
this is not a source-derived claim about C.

The sealed import lives in `crates/qcore-verification/src/reactor_source.rs`:
`ReactorProgramImport` is constructed only by `import_reactor_program`, has
private fields, no `Deserialize` and no conversion from the wire projection,
with a `compile_fail` doctest against deserialization.

### Obligations and the coverage denominator

The route derives **two** obligations over the whole unit, reusing the reserved
event-trace properties rather than adding new ones:
`event_generation_discharged` and `callback_quiescent_at_teardown`. One
indivisible `check_event_program` run decides both — a stale generation and an
unquiescent teardown are rejected in the same pass — so neither can be
discharged without the other, and the checker returns
`task.coverage_for(EventTraces)`.

That is the same shape E02 flagged on the ChunkIO Boolean route ("returns its
whole task coverage set"). It is sound here for the same reason and is guarded
the same way: `check` requires exact equality with the checker's own derived
task before returning anything, so the denominator is always the core's own
derivation, never a provider claim. A test pins both the exact obligation pair
and the refusal of a foreign task.

## 3. Wire and schema versions

Per `crates/qcore-schemas/README.md`'s documented policy, two additions are
source-incompatible for exhaustive matches and struct literals — a
`SemanticWitness::ReactorCallback` variant and the new required
`VerificationRequestV2::reactor_program` field — so:

| Boundary | Before | After | Why |
|---|---|---|---|
| `qcore-schemas` crate | 4.0.0 | **5.0.0** | Exhaustive-variant precedent from 2.0.0 and 4.0.0 |
| Plugin protocol | 1 | 1 | Unchanged |
| Evidence version | 1, 2 | 1, 2 | Unchanged |
| Reactor callback proposal | 1 | 1 | **Unchanged** — reused as-is |
| Reactor program input | — | 1 | New |
| Reactor semantic profile | — | `qcore.reactor_callback_program.v1` | New |
| Reactor input digest key | — | `qcore:reactor-program-input:v1` | New |
| Libevent package | 0.1.0 | **0.2.0** | Adds the capability route |

**Why the change is forced** (the card asked for this to be justified, not
assumed). The witness enum is the only carrier of a replayed proposal, and no
existing variant can hold a callback assignment inventory: `ContractObservation`
is a recorded-run record set with decimal observation values, and reusing it
would misdescribe both what the witness is and what accepting it means. The
program projection *could* technically have travelled as JSON inside a
`NormalizedFact::subject`, the way `LifecycleSiteFact` does — but that encoding
has no `deny_unknown_fields`, no schema-version field and no `validate()` rule,
so it would be a strictly weaker boundary than the typed field the Boolean and
counter routes already established. No `ObligationProperty` variant was added
and `predicate_for` is untouched: the route reuses the two reserved event-trace
properties, which now have a derivation and a witness kind.

Existing wire bytes are unchanged: `reactor_program` is an `Option` that
defaults to absent and is skipped on serialization, exactly like `source_effect`
and `counter_effect`. `crates/qcore-schemas/tests/reactor_request_wire.rs` and
the amended `counter_request_wire.rs` both pin that a request without a
projection serializes with **none** of the three keys and round-trips.

Admission rules: a request carrying a reactor projection together with either
source projection is refused; a projection is admitted only with exactly one
obligation of each reserved property, both `ObligationSubject::Unit`, under
`EventTraces`. The quiescence property is the marker that *requires* a
projection — the generation property alone does not, because it predates this
route and `providers/conformance/tests/packages.rs` still names it in a
projection-free request. The host gains the fail-closed digest binding for the
new field, symmetric with `source_effect`: a projection that is not the one
pinned in the core task is Unknown under QP001.

**Deliberate pin updates:**

- `providers/conformance/tests/packages.rs` pinned the Libevent package manifest
  at 0.1.0 with `certificate_checker: qcore.unsupported_assertions.v1`. It now
  pins 0.2.0 and `qcore.reactor_callback_program.v1`, because the executable no
  longer only proposes unsupported assertions. Targets, capabilities, limits,
  trust class, the Lean family and every other entry are unchanged, and the
  built-in v1 adapter still answers Unknown for every capability.
- `compiler/tests/fixtures/w3/provider/Cargo.lock` records `qcore-schemas`
  5.0.0 so the pinned standalone-provider fixture's `--locked` build resolves.

No gate was weakened and no expectation was deleted.

## 4. Parity evidence (old route retained)

The in-process adapter is **not removed**, so parity is archived as an
executable comparison rather than a diff:
`compiler/tests/reactor_route_parity.rs` (4 tests) runs both routes over the
same programs, using the **shipped** `libevent` source-set profile's
accepted-build matrix, not a permissive fixture.

- For the one-shot descriptor-backed read callback and the persistent callback
  with a blocking delete, the `EventCertificate` from the in-process route
  (`libevent::check_event_program`) is **equal field for field** to the one the
  new route's `ReactorCallbackTask::evaluate()` computes — `freed_events`
  (the descriptor-backed event generations), `freed_bufferevents`,
  `closed_descriptors`, `dropped_arguments`, `checked_operations` and the whole
  `trace`. The observed trace is additionally asserted equal on its own, which
  is the observation half of acceptance 3.
- The proposal a provider must return is accepted and discharges exactly the
  core's own obligation set; the inventory names the real event id and
  generation the program assigned.
- Four programs the in-process route refuses — a stale event generation, a
  payload access the callback summary never authorized, a callback argument
  dropped while still pending, and teardown without callback quiescence — are
  refused on the portable route too, either lost at import or refused by the
  core's own model run. In neither case can a witness discharge the obligations.
- An unaccepted build (OpenSSL enabled against the declared matrix) and an
  empty accepted-build matrix stay Unknown on both routes.

The four archived Cycle 19 artifacts and the canonical CMetrics audit are
untouched by construction, not re-measured — see §6.

## 5. Negative and mutation results

### Core-side (`crates/qcore-verification/tests/reactor_source.rs`, 7 tests)

Import refusals (each keeps the program's callbacks Unknown): bufferevent
creation, socket-pair custody, socket close custody, socket error query,
ordering assumption, loop control, a bufferevent access context, an empty
program, an oversized program, an empty or unmatched accepted-build list, a
second declared base profile, and an empty declared library or revision.

Seventeen forged inventories, each refused against the core's own assignments:
emptied inventory; unsupported proposal version; duplicated record; a record at
an operation that is not an assignment; a record beyond the program; retargeted
event identity; retargeted generation; widened payload authority; escalated
payload access kind; widened trigger reasons; duplicated trigger reason;
duplicated payload permission; relabelled argument mode; retargeted callback
argument; granted deferral; granted task spawning; an invented extra assignment.

Binding and identity: a witness of another kind never satisfies the checker; a
witness bound to another task digest is refused before replay; a task the
checker did not derive is refused even with a real witness; a changed build
profile, changed declared assumptions and a changed program each produce a
different task digest and the cross-task witness is refused in both directions;
a moved revision refuses derivation against a target that does not name it; six
incompatible targets and an empty translation unit refuse derivation.

Anti-vacuousness: three programs the model refuses (stale generation,
unauthorized access, unquiescent teardown) cannot be discharged even by the
core's own exported inventory — the error names the model refusal.

### Provider-side (`providers/common/reactor_callbacks.rs`, 3 unit tests)

Accepted readings (registration index, event identity and generation, declared
authority; a second distinct registration read at its own index; `Delete`
counted as deregistration alongside `FreeEvent`), and fourteen refusals: wrong
schema version, empty library, empty revision, empty program, oversized
program, no leading `CreateBase`, no trailing `FreeBase`, a base freed that was
not created, a second `CreateBase`, a second `FreeBase`, an assignment on a
foreign base, a repeated event identity, no assignment at all, and an
assignment never deregistered.

### End-to-end (`providers/conformance/tests/reactor_callback_capability.rs`, 2 tests)

Runs the **real built package** through `ProcessProvider`:

- **positive**: `ProposedVerdict::Safe`, certificate bound to the task, witness
  kind `reactor_callback`, registration index 1, event id 1, generation 1,
  callback argument 9; the transported request contains `operations` and no
  `callbacks`/`proposal`; a program with a second registration is read
  independently at its own index;
- **missing evidence**: no core checker → Unknown with QP016 and no certificate;
  no registered provider → Unknown; disabled provider → Unknown; the shipped
  `qcore.chunkio` package, which does not implement this capability → Unknown
  with no certificate;
- **tampered witness/transport**: a well-formed projection the core never
  imported (every generation flipped to 2) is refused by the host's digest
  binding with QP001; rebinding it to a recomputed task digest gets past that
  and is then refused by the core's own comparison, with no certificate; a Safe
  proposal over a program the model refuses is rejected with QP009; a changed
  declared build profile retires the checker's context;
- a four-row forged-witness matrix (emptied inventory, retargeted generation,
  widened authority, moved registration index) is refused by the firewall.

## 6. Custody and audit invariants

No allocation, descriptor or mapping custody path was touched. The in-process
`check_event_program` body, `check_observation_refinement`, the Monkey binding /
source / observation modules and the pinned-source audit are unedited; the only
compiler-side addition is a new transport module. `EventFlags::bits` is a purely
additive accessor so a projection round-trips an invalid flag set unchanged
rather than normalizing it away, leaving `EventFlags::valid` to refuse it
downstream.

The four pinned 185-unit artifacts and the canonical CMetrics audit were **not
re-measured here**: this worktree has no prepared Clang CMake build tree and the
Fluent Bit checkout is off the profile pin that preparation requires. They are
unchanged by construction — every existing analysis body is untouched, and the
only edits to existing production files are additive (a new schemas module, a
new optional request field that serializes away, one witness variant, one host
digest check, one core accessor, and `reactor_program: None` at six existing
request literals). Re-measurement (CORPUS-REPIN) remains the named follow-up it
already was.

No Lean obligation is added: no semantic checking rule changed. The decision
procedure is the same `check_event_program` body, and the new code is transport
plus a binding gate in front of it. `python3 -B scripts/check_lean_audits.py`
passes 21 audits with no new axiom.

**Nothing is promoted.** Raw Cycle 19 units remain 2 Safe / 42 provisional
Unsafe / 137 Unknown plus four data-only units; CT coverage remains 11/50
mapped; the contract corpus remains 59 modules; full language conformance
remains false.

## 7. Gate results

Run in this worktree on Linux with Clang available:

| Gate | Result |
|---|---|
| `cargo test --workspace` | **1096 passed, 0 failed** (1080 baseline + 7 core replay + 4 parity + 4 schema wire + 1 gateway compile-fail doctest) |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `cargo test --manifest-path providers/Cargo.toml --workspace` | **56 passed, 0 failed** (51 baseline + 3 provider unit + 2 conformance) |
| `cargo fmt --manifest-path providers/Cargo.toml --all -- --check` | clean |
| `cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets -- -D warnings` | clean |
| `python3 -B -m unittest discover -s scripts/tests` | 43 passed |
| `cd proof && lake build` | 64 jobs, completed successfully |
| `python3 -B scripts/check_lean_audits.py` | 21 audits passed, no new axiom |

`lake` is not on the default `PATH` on this host; it was run from `~/.elan/bin`.
The Libevent submodule required
`git -c protocol.file.allow=always submodule update --init --recursive`.

## 8. Open items and named blockers

- The in-process adapter route is retained. Its removal, after this parity
  record, is a later unit (P09).
- **The moved capability has no production caller.** `libevent::check_event_program`
  is exercised by the Cycle 18 unit tests, not by the Cycle 19 compatibility
  adapter, so no production report changes and no raw unit moves. Binding a
  reactor program to real Monkey/Libevent source is the substantive next step
  and is *not* done here.
- Observation refinement (`check_observation_refinement`) stays in process: an
  observation witness claims recorded native runs and needs `NativeRunBinding`
  and the observed-run bounds discipline, which is a different witness kind with
  its own trust argument.
- The pinned-source audit (`verify_pinned_libevent_source`) needs a filesystem
  input capability this card does not open.
- Monkey binding, source and observation adapters remain blocked as recorded in
  §1 — the first on composition over other summaries, the latter two on the
  sealed `SourceMatch` boundary.
- Bufferevent custody, raw socket-descriptor custody, ordering assumptions,
  loop control and multi-base programs are outside the admitted family by
  construction. Widening the vocabulary is a separate decision, not a bug.
- The route binds no profile-declared callback model: the declared build
  attributes are opaque metadata bound only by digest, and the accepted-build
  decision is the caller's, exactly as on the in-process route.
- A systematic mutation matrix in the style of remaining-e02's 52 rows is not
  attempted; §5 is a targeted set of 17 core-side, 14 provider-side and 4
  end-to-end forgeries, not an enumeration.
- Registry-wide five-provider composition (P08) and the 185-unit corpus
  re-measurement (CORPUS-REPIN) stay open and are not discharged here.

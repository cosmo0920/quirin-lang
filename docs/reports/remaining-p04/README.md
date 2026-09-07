# P04 — One additional ChunkIO capability through the process boundary

Workstream W5 (Providers). Bounded increment: the unsigned unit-counter
contract, previously reachable only through the in-process ChunkIO adapter,
now travels to a separate schemas-only process as a typed path projection and
comes back as a witness the trusted core replays. Nothing is promoted: no raw
Cycle 19 unit changes class, no CT case is mapped, no contract-corpus count
moves, and the checking rule that decides the counter contract is byte-identical
to the one that already ran in process.

- Branch checkpoint before this increment: `a4492da` (worktree
  `claude/cycle-19-task-delegation-b1d9c5`).
- Baseline reproduced before any change: 997 workspace tests, 47 provider
  tests, 43 Python tests, 21 Lean audits, both reevaluation examples exit 0.
- Libevent submodule initialized at the pinned `d82464a2…`; the pin test passes.

## 1. Capability inventory: what still ran only in process

`compiler/src/verifier_plugins/chunkio/` holds fourteen modules. Each was read
and classified by *which core checker decides it* and *what a process would
have to be able to say*:

| In-process route | Core checker | Status |
|---|---|---|
| `chunk_source` (Boolean field) | `qcore-contract::chunk_contract` | **Already process-based** (package 0.3.0/0.4.0, 52-row mutation matrix in remaining-e02) |
| `teardown_source::check_counter_sources` | `qcore-verification::counter_contract` | **Selected — moved by this card** |
| `teardown_source::check_list_unlink_source` | `qcore-verification::list_unlink_contract` | Deferred: the checker relates *two* summaries (`mk_list_del` and `__mk_list_del`), so a projection must carry both paths and their caller/callee relationship; it is a strictly larger vocabulary than one path |
| `teardown_source::check_memory_backend_source` | `qcore-verification::field_destructor` | Deferred: the path contains a `free` call. Calls are opaque in the typed projection, and a portable call vocabulary is exactly the custody-bearing surface this card is told not to widen |
| `file_outcomes`, `file_delete`, `file_attempts` | `qcore-verification::cleanup_outcome` + `source_call_use` | Deferred: these certificates carry descriptor/mapping residual custody and feed the native-observation composition; moving them would touch the custody the card requires preserved |
| `traversal`, `parent_teardown` | `typed_source_match::check_source_match` | Structurally blocked: `SourceMatch` is a sealed reviewed-library wrapper with no public constructor, conversion or deserialization (W5 typed-body boundary). A process cannot propose one without breaking that boundary |
| `memory_close`, `filesystem_chunk_close`, `unsupported_close`, `file_close`, `unmap_wrapper`, `nested_memory`, `nested_cleanup` | compositions over the certificates above | Not single-obligation capabilities: each consumes other certificates, so it can only follow its dependencies |

### Why the counter contract

It is the smallest well-bounded candidate that is genuinely a capability rather
than a composition: one function, one complete source path, one contract module
(`contract/qcore/chunkio/counters.qcore`), no call in the path, no dependency on
another certificate, no native observation, and no allocation, descriptor or
mapping custody anywhere in its statement. It is also live production code — the
Cycle 19 compatibility adapter checks the four `cio_chunk_counter_total_*`
helpers through it — so the parity question is real rather than synthetic.

## 2. What actually crosses the boundary

The request carries `CounterSourceInput`: the imported path in a bounded typed
vocabulary (`Read` / `Write` / `ReturnValue` over `Parameter`, `Symbol`,
`Integer`, `Field`, `Binary`), the imported C signature, the declared root C
type and the profile assumptions. **No derived result travels with it** — no
field, no delta, no flags — exactly as the Boolean route sends statements and
not paths.

The process (`providers/common/counter_effect.rs`, shipped in
`qcore.chunkio` 0.5.0) reads that path on its own and returns a
`CounterUpdateRecord`: which field the path updates, in which direction, over
which unsigned storage type, and the two structural facts it read from the path
(the returned value is the read that follows the write; the update is the only
write).

The core (`qcore-verification::counter_source::CounterSourceTask`) then:

1. refuses any task whose target identity, dialect, revision, platform, backend
   or Clang target does not match its own import;
2. **evaluates the pinned contract itself** with the unmodified
   `check_counter_contract`, over its own `SourceSummary`; and
3. requires the proposal to equal the reading it computed from that certificate.

Acceptance therefore terminates in the core's own evaluation. Agreement alone
discharges nothing, and a Safe proposal over a false contract is refused
(exercised below). Live storage, exclusive access, the entry range and the
unsigned-C/`usize` ABI binding remain the contract's declared premises, recorded
in the task's assumptions — this is not a custody, lifecycle or whole-unit claim.

The sealed import lives in `crates/qcore-c-gateway/src/counter_source.rs`:
`CounterSourceImport` is constructed only by `import_counter_sources`, carries
the importer-captured identity, and has a `compile_fail` doctest against
deserialization. The identity record was renamed `SourceInputIdentity` (it was
never Boolean-specific) with `BooleanSourceIdentity` retained as an alias, in
the style of the W7 generic-name work.

## 3. Wire and schema versions

Per `crates/qcore-schemas/README.md`'s documented policy, three additions are
source-incompatible for exhaustive matches — a `SemanticWitness::CounterContract`
variant, an `ObligationProperty::CounterUpdateContract` variant, and the new
required `VerificationRequestV2::counter_effect` field — so:

| Boundary | Before | After | Why |
|---|---|---|---|
| `qcore-schemas` crate | 3.0.0 | **4.0.0** | Exhaustive-variant precedent from 2.0.0 |
| Plugin protocol | 1 | 1 | Unchanged |
| Evidence version | 1, 2 | 1, 2 | Unchanged |
| Boolean source input / profiles | 2, 3 / v2, v3 | unchanged | Untouched |
| Counter source input | — | 1 | New |
| Counter semantic profile | — | `qcore.c_counter_contract.v1` | New |
| Counter input digest key | — | `qcore:counter-source-input:v1` | New |
| ChunkIO package | 0.4.0 | **0.5.0** | Adds the capability route |

Existing wire bytes are unchanged: `counter_effect` is an `Option` that defaults
to absent and is skipped on serialization, exactly like `source_effect`.
`crates/qcore-schemas/tests/counter_request_wire.rs` pins that a request without
a projection serializes with neither key and round-trips.

The `qcore-schemas` README header still said `2.0.0` after P02 bumped the crate
to 3.0.0; it now reads 4.0.0 and carries the missing 3.0.0 section as well.

**Deliberate pin update:** `providers/conformance/tests/packages.rs` pinned the
ChunkIO package manifest at 0.4.0. It now pins 0.5.0 and the added
`qcore.c_counter_contract.v1` profile on the `fluent_bit` target. The certificate
checker, Lean theorem family, capabilities, limits, trust class and every other
target entry are unchanged. No gate was weakened and no expectation was deleted.

## 4. Parity evidence (old route retained)

The in-process adapter is **not removed** — deletion can be a later P09 unit —
so parity is archived as an executable comparison rather than a diff:
`compiler/tests/counter_route_parity.rs` imports the same C fixture through both
routes and asserts

- for all four `cio_chunk_counter_total_{add,sub,up_add,up_sub}` helpers, the
  serialized `CounterCertificate` from the old route
  (`teardown_source::check_counter_sources`) is **equal** to the certificate the
  new route's `CounterSourceTask::evaluate()` recomputes, and the field/delta
  pairs are the expected ones; and
- the reading a process must propose is accepted by the core checker with
  coverage equal to the task's own obligation set;
- four source changes that the old route refuses (`total_chunks_up--` for the
  `total_chunks` contract, a constant return, a double decrement, a signed
  counter type) are refused by the new route too — either the projection is lost
  at import or the contract evaluation fails; in neither case can a witness
  discharge the obligation.

Both tests pass. The four archived Cycle 19 artifacts and the canonical CMetrics
audit are untouched by construction, not re-measured — see §6.

## 5. Negative and mutation results

`crates/qcore-verification/tests/counter_source.rs` (6 tests, all rejections
verified):

- a false contract (the decrement declared as `delta == 1`) is refused for both
  a correct and an incorrect proposal — agreement cannot discharge it;
- nine forged readings each reject: delta, field, counter C type, root C type,
  function, source revision, either structural flag, and an injected profile
  assumption;
- a witness rebound to a foreign task digest rejects, and a witness of another
  kind (`ContractObservation`) never satisfies this checker;
- changed source bytes, changed build definitions, a changed contract and a
  changed declared root QCore type each produce a different task digest, and the
  cross-task witness rejects;
- six incompatible target identities (Boolean profile, `c99` dialect, wrong
  revision, foreign platform, emulated backend, empty library) refuse derivation,
  as do an empty translation unit, an empty contract and an empty root type;
- a path containing a call keeps its existing `Unknown` at import, and a second
  field write projects but is refused by the core's contract evaluation.

`providers/conformance/tests/counter_effect.rs` runs the **real built package**
through `ProcessProvider`:

- **positive**: `ProposedVerdict::Safe`, certificate bound to the task, witness
  kind `counter_contract`, `field = total_chunks`, `delta = -1`,
  `counter_c_type = unsigned long`; a real `++` source is independently read as
  `delta = 1` by the same process;
- **missing evidence**: no core checker → Unknown with QP016 and no certificate;
  no registered provider → Unknown; disabled provider → Unknown; the shipped
  `qcore.libevent` package, which does not implement this capability → Unknown
  with no certificate;
- **tampered witness/transport**: a well-formed projection the core never
  imported (operator flipped to `+`) is faithfully read by the process and then
  refused by the core with `QP009 counter witness differs from the checked
  source update`; rebinding it to a recomputed task digest is refused the same
  way; a Safe proposal over a false contract is refused with QP009; changed
  source bytes and changed build options each retire the checker's context;
- request shape: the transported projection contains `operations` and no
  `delta`/`field`; an unsupported input version and a request carrying both the
  Boolean and counter projections each fail `validate()`.

`providers/common/counter_effect.rs` carries two unit tests covering the
accepted readings and ten refusals (unsupported operator, signed storage type,
wrong result symbol, wrong signature, wrong version, empty function, foreign
root type, a second write, a truncated path, an oversized path).

## 6. Custody and audit invariants

No allocation, descriptor or mapping custody path was touched. The counter
statement contains no allocation, no descriptor, no mapping and no call; the
`cleanup_outcome`, `file_*`, `*_close`, `nested_*` and `traversal` routes are
byte-identical, and the native observation evidence under
`docs/reports/cycle19-*` is untouched. No observation is certified as a universal
source fact: the counter certificate remains a conditional statement about one
imported path under the contract's declared premises.

The canonical CMetrics audit and the four pinned 185-unit artifacts were **not
re-measured here**: this worktree has no prepared Clang CMake build tree, and
the developer Fluent Bit checkout is at `90a01091`, off the profile pin that
preparation requires. They are unchanged by construction — every existing
analysis body is untouched, the only edits to existing production files are
additive (a new module, a new optional request field that serializes away, two
new enum variants, three new accessors, one new `predicate_for` arm for the new
property, and a type rename with an alias) — and the audit-facing conformance
tests (`configured_audit`, `configured_source_contract`, `cmetrics_unit`) pass
unchanged. Re-measurement remains the named follow-up it already was.

No Lean obligation is added: no semantic checking rule changed. The counter
contract's decision procedure is the same `check_counter_contract` body, and the
new code is transport plus an equality gate in front of it.

## 7. Gate results

Run in this worktree on Linux with Clang available:

| Gate | Result |
|---|---|
| `cargo test --workspace` | **1008 passed, 0 failed** (997 baseline + 6 counter replay + 2 parity + 2 schema wire + 1 gateway compile-fail doctest) |
| `cargo fmt --all -- --check` | clean (workspace and `providers/`) |
| `cargo clippy --all-targets -- -D warnings` | clean (workspace and `providers/`) |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `cd proof && lake build` | 64 jobs, completed successfully |
| `python3 -B scripts/check_lean_audits.py` | 21 audits passed |
| `cargo test --manifest-path providers/Cargo.toml --workspace` | **50 passed, 0 failed** (47 baseline + 2 provider unit + 1 process conformance) |
| `python3 -B -m unittest discover -s scripts/tests` | 43 passed |

`lake` is not on the default `PATH` on this host; it was run from
`~/.elan/bin`. The Libevent submodule required
`git -c protocol.file.allow=always submodule update --init --recursive`.

## 8. Open items

- The in-process adapter route is retained. Its removal, after this parity
  record, is a later unit (P09).
- Only `qcore.chunkio` implements the counter capability; no configured audit
  route selects it yet, so no production report changes and no raw unit moves.
- List-unlink, memory-backend field destructor and the cleanup-outcome family
  remain in process, blocked as recorded in §1; the typed-source-match routes are
  blocked by the sealed `SourceMatch` boundary and need a different design, not
  a bigger vocabulary.
- The counter route binds no profile-declared model library: it selects
  functions by explicit name, as the existing in-process route does, so
  `target.library` is checked for presence but not against a declared field
  model. Profile-declared counter selection is open.
- A systematic mutation matrix for this capability, in the style of remaining-e02's
  52 rows, is not attempted here; §5 is a targeted set, not an enumeration.
- The 185-unit corpus re-measurement and re-pinning stays the existing named
  follow-up; this card does not discharge it.

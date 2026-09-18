# CT42-PROTOCOL-CELL — the shared protocol cell, surveyed, and its derivation landed for the socket route (wave 31)

**Card:** CT42-PROTOCOL-CELL (exploratory; wave 31).
**Question:** what is the smallest honest capability that makes CT42's
scenario — *"Two descriptors share an underlying logical state"*, expected
*"Preserve shared protocol-cell identity"* — expressible?
**Disposition:** the survey found the cell notion **already exists at model
level, declared**; the missing half is the **derivation** from source that
`docs/design/DISJOINT_FOOTPRINT_PROFILE.md` §8 names F4 and blocks CT42's F6
row on. This card lands the bounded F4 slice for the one source route where
two descriptor-valued names over one shared logical state exist today —
`source.socket.v1`'s completed connection — and states the measured boundary
for everything it does not land. CT42 itself stays **unmapped**: no manifest
row, no fixture, no matrix edit (card boundaries), and the wave-17 C-twin
blocker stands.

## 1. Survey — where the cell already lives, and where it does not

Three models were read before anything was designed.

| Model | What it has | What it lacks for CT42 |
|---|---|---|
| `crates/qcore-core/src/disjoint_footprint.rs` (T09) | The **whole identity-and-refusal half**: `DerivedFootprint::Descriptor { descriptor, cell }`, `FootprintEvent::DeclareDescriptorAlias`, the `SharedProtocolCell` refusal (`Q664`), the certificate's `protocol_cells` map, and the Lean theorems `descriptors_sharing_a_cell_are_never_disjoint` / `descriptors_over_distinct_cells_are_disjoint` (`QCore.DisjointFootprint`) | The cell is a **declaration** carrying the `DeclaredProtocolCellIdentity` premise — nothing derives it from source (§8 F4, blocking F6/CT42) |
| `crates/qcore-core/src/os_endpoint.rs` (O03) | The descriptor-slot / **open-file-description** / write-back-duty split — the POSIX shape of the dup cell — for the failure post-state of one `close` on one pinned target | No multi-holder structure (no reference count, one operation), and no production route consumes it |
| `crates/qcore-core/src/std_socket.rs` (`source.socket.v1`) | The **first source route with two descriptor-valued names over one shared logical state**: a completed connection's connector and accepted binding. The protocol invariant is already enforced **across the two names** — a `send` through one name refuses when the *other* end is closed ("this send has no receiver"), the in-flight bound at a `send` is read off the **peer's** committed stream, a `close` refuses while bytes the peer committed sit undrained, and a listener close refuses over a pending connect | The shared state has **no identity**: it is represented distributedly (mutual `peer: Option<String>` references plus per-end buffers), so no reader can ask "do these two names advance one cell?" |

So of the card's three goals: **(b) the cross-name refusal exists** (the
transport checker's, pinned by `compiler/tests/std_socket.rs` —
`a_stream_close_with_undrained_bytes_refuses_and_a_datagram_close_does_not`,
`a_listener_close_with_an_unaccepted_connection_refuses`, and the
peer-closed send arm in `check_socket_program`); **(a) the one-identity
representation exists declared but not derived**; and the gap this card can
close honestly is the derivation that connects them.

Two structural facts make the derivation exact rather than approximate:

1. **Every peer of every socket is a socket of the same program**
   (`SOCKET-EXTERNAL-PEER` is refused by name), so both custodians of every
   connection are enumerable at check time.
2. **In every admitted program, every `connect` is eventually `accept`ed**: a
   listener close over a pending connect refuses, and a leaked listener
   refuses in the ownership half — so completed connections are the whole
   pairing structure, and the transient pending-phase coupling never survives
   to an admitted program's end.

## 2. What landed

### 2.1 `qcore_core::socket_footprint` (Rust)

A projection module on the `socket_event` precedent — it decides no
disjointness rule and re-implements no transport rule:

* `derive_socket_cells(&SocketProgram) -> Result<SocketCells, Vec<SocketError>>`
  — **fail-closed**: it runs the unedited `check_socket_program` first and
  returns its refusals verbatim; a refused program derives no cells. For an
  admitted program it mirrors the checker's own bookkeeping (descriptors in
  acquisition order, identical to `SocketCertificate::descriptors`; the
  `connect` target read from the same `resolve_addresses` resolution the
  checker reads, never re-derived): a created socket gets a fresh singleton
  cell, and an `accept` gives the fresh binding its popped connector's cell —
  the completed connection, one cell with two custodians.
* `footprint_program(&SocketCells) -> FootprintProgram` — one
  `DeclareDescriptorAlias` per socket, in acquisition order, handed to the
  **unedited** `check_disjoint_footprint_program`. Two ends of one connection
  are refused there as `SharedProtocolCell` — **`Q664`, an existing code; this
  card allocates none and widens none** (the refusal is the footprint
  checker's own condition, reached by a new deriver).

Unit tests (7, `crates/qcore-core/tests/unit/socket_footprint.rs`): the pair
shares one cell and the listener does not; the derived descriptors agree with
the certificate's; the derived pair's `AssertDisjoint` is refused with the
exact `SharedProtocolCell` diagnostic and `code() == "Q664"`; the listener
decides disjoint from both ends and the certificate's `protocol_cells` map is
exactly the derived assignment; two connections derive two distinct cells and
a cross-pair question is admitted; a datagram pair derives singleton cells; a
leaking program is refused by the unedited ownership checker and derives
nothing.

### 2.2 `QCore.SocketFootprint` (Lean, audited)

The derivation as an executable model (`create`/`connect`/`accept` over a cell
assignment, pending queues and a fresh counter), with the freshness invariant
and the composition proved and registered in `proof/AuditSocketFootprint.lean`
(Lean audits 121 → **122**):

* `initial_is_bounded`, `assigned_cell_is_bounded`, `step_preserves_bounded`,
  `run_preserves_bounded` — every assigned cell is below the fresh counter;
* `create_assigns_the_fresh_cell`, `a_fresh_cell_is_no_existing_cell` —
  unpaired sockets can never collide with an existing cell;
* `an_accepted_pair_shares_one_cell`, `an_accept_keeps_every_other_assignment`,
  `a_create_keeps_every_other_assignment` — the accept pairs exactly the two
  ends and moves nobody else;
* `a_derived_pair_is_refused_as_a_shared_cell`,
  `a_distinct_derived_cell_decides_disjoint` — the composition with the
  **unedited** `QCore.DisjointFootprint.decideDisjoint`, plus a computed
  end-to-end example on the canonical stream program.

Axiom surfaces: `propext`/`Quot.sound` only, inside the `AuditAll` closure.

### 2.3 Docs

`DISJOINT_FOOTPRINT_PROFILE.md` §8's F4 row is annotated **partly landed** on
the F1 precedent (the row's dup/`fdopen`/chunk-handle scope and the
`event_reactor` connection stay open, stated in place).

## 3. Decisions, each with its ground

* **The connection cell covers `{connector, accepted}` and not the
  listener.** After the accept, the pair's traffic never touches the
  listener's own state; the pending-phase coupling (a listener close over a
  queued connect) is transient, cannot survive to an admitted program, and its
  refusal **stays the transport checker's** — the derivation does not restate
  it (`CT42-CELL-PENDING-PHASE`).
* **A datagram pair is deliberately not a cell** (`CT42-CELL-UDP-QUEUE`). A
  `sendto` appends to the receiver's queue, but the two sockets share no
  connection phase: either end closes independently, a queued datagram is
  discarded locally at close, and no operation through one name is refused on
  the other's *protocol* state. Answering "disjoint" for a UDP pair is the
  true answer, not an omission.
* **`DeclaredProtocolCellIdentity` stays on the certificate, with its ground
  supplied.** The footprint checker is unedited, so a certificate over this
  projection still carries the assumption; for exactly these cells the
  derivation grounds it — the pairing is read from the program's own decided
  connection structure, and `source.socket.v1` admits no external peer, so no
  undeclared descriptor advances the cell. Narrowing the assumption on the
  certificate itself would mean editing the T09 checker and is left named
  (`CT42-CELL-DERIVED-PREMISE`).
* **Fail-closed at both mouths.** A refused socket program derives nothing
  (the checker's own errors, verbatim); inside the walk, the two arms that are
  unreachable for admitted programs (an accept with nothing queued, a
  connector without a cell) assign a fresh singleton cell — the direction that
  *refuses more* disjointness questions than it admits, and shares nothing.

## 4. What is NOT landed — the measured boundary

* **No dup-like alias.** No source form on any route creates two names over
  one descriptor slot's open file description; `os_endpoint`'s
  description/descriptor split remains model-level. A `dup` source form needs
  new surface (and the census discipline) — a separate card
  (`CT42-CELL-DUP-SOURCE`).
* **No source syntax asks the disjointness question.** The derivation and the
  refusal compose at model level (exactly like T09 itself); no `.qcore`
  statement spells `AssertDisjoint` over sockets. Wiring the derived cells
  into a route's own diagnostics (e.g. a task capture naming two sockets of
  one cell) is F1×F4 composition work.
* **The C-twin arm of CT42 is untouched.** The wave-17 measured blocker —
  *"an int-typed resource identity is not expressible at all; the contract
  loader refuses an owned non-pointer result"* (`CT-CTWIN-DESCRIPTOR`,
  `docs/reports/wave17-ct40/README.md` §4) — stands unmodified.
* **Byte-identity (goal c) holds structurally**: no emission path, no checker,
  no diagnostic and no route-selection file is edited — the new module is
  additive, and every pre-existing program's behaviour is decided by files
  this card did not touch.

## 5. CT42 readiness statement

CT42 stays **unmapped, and its manifest row, fixtures and the matrix are
untouched** (card boundaries). What this card changes about its readiness:

* The scenario's **model half is now real and derived**: two descriptor-valued
  source names (a connected pair of an admitted socket program) whose shared
  underlying protocol state is one identity two custodians reference, with the
  operation-level invariant enforced by the transport checker (pre-existing)
  and the identity-level refusal (`Q664`) reachable from source-derived facts
  (this card).
* A future mapping lane has **two candidate arms**, either honest: (i) a
  source-stage fixture on the socket route whose scenario is the connected
  pair — its stage evidence would be the transport checker's cross-name
  refusals plus the derived-cell composition landed here; or (ii) the C-twin
  arm, which stays blocked on `CT-CTWIN-DESCRIPTOR` (int-typed resource
  identity in the contract loader) — a loader/W4 card, not a model card.
* What CT42 still lacks for either arm: a fixture, a manifest row and a lane's
  stage-evidence record (all outside this card's boundaries), and for the
  dup-reading of "two descriptors" the `CT42-CELL-DUP-SOURCE` surface.

## 6. Named follow-ons

| Id | What |
|---|---|
| `CT42-CELL-DUP-SOURCE` | a source form creating two names over one open description (dup/`fdopen` shape); needs census-clean surface and a custody story for the second name |
| `CT42-CELL-PENDING-PHASE` | if a route ever admits a program with a surviving pending connect, the pending-phase `{connector, listener}` coupling needs a cell or a refusal; today the transport checker owns it |
| `CT42-CELL-UDP-QUEUE` | the decision that a datagram pair is not a cell, recorded so a later card revisits it deliberately rather than silently |
| `CT42-CELL-DERIVED-PREMISE` | narrow `DeclaredProtocolCellIdentity` on the footprint certificate for derived cells (needs a deliberate T09 checker edit) |
| `CT42-MAP` | the mapping lane itself: fixture + manifest row + stage evidence on either arm of §5 |

## 7. Gate

Run at the card tip on this worktree (Linux). Measurement log:
`measure/workspace-tests.log` (worktree-private, gitignored).

| Gate step | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | exit 0 — **4,473 passed, 0 failed, 8 ignored** across 221 test binaries (baseline 4,466 + this card's 7; no constraint-16 row tripped) |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `PATH="$HOME/.elan/bin:$PATH" python3 scripts/check_lean_audits.py` | exit 0 — `Passed 122 Lean audits` (121 baseline + `AuditSocketFootprint.lean`) |
| `python3 -m unittest discover -s scripts/tests` | **199 OK of 200, 1 disclosed failure** (below) |

The Python row's one failure is the **disclosed lean tripwire**:
`test_build_matrix_sections.BuildMatrixSections.test_the_derived_facts_are_recomputed_rather_than_carried`,
failing as `['14.0.0', 121] != ['14.0.0', 122]` — this card adds an audit
(121 → 122) and the committed matrix's `validation.lean_audits` still pins
121. The matrix is regenerated at wave close by the integrator (constraint
25's `section_ownership` discipline); this card may not touch it.

---

## Published beside by `WAVE-38-CLOSE` (2026-09-18) — `CT42-CELL-FALLBACK-ARM-DIRECTION`

Appended, not edited (constraint 21): every line above the rule is the frozen
card text. §3's last bullet says the two arms "unreachable for admitted
programs" assign a fresh singleton cell in "the direction that *refuses more*
disjointness questions than it admits". Card `STABILITY-REEVAL-3` (wave 38,
§4) measured the opposite, and this close re-read the lines at `4f663934`:
`crates/qcore-core/src/socket_footprint.rs:184` (`continue`, a connector the
resolution cannot target) and `:211` (`None => acquire(entries, binding, None)`,
an accept with nothing queued) give the binding a fresh singleton cell
(`cell.unwrap_or(descriptor)`, `:167`), and the **unedited** T09 checker
(`crates/qcore-core/src/disjoint_footprint.rs:714-722`) returns
`Ok((true, …))` — *proved disjoint* — for two descriptors of distinct cells,
refusing `SharedProtocolCell` only when the cells are equal. So the fallback
arms **admit** disjointness for a binding whose real connector was not paired:
the admitting direction, not the refusing one. Harmless today only because
`check_socket_program` (`:149`) refuses first; load-bearing the moment
`CT42-CELL-ROUTE` routes the deriver, since any disagreement between its FIFO
pairing and `std_socket`'s peer assignment (`CT42-CELL-PENDING-PHASE`) reaches
an arm that admits. Registered as `CT42-CELL-FALLBACK-ARM-DIRECTION`
(`docs/reviews/r0_3/acceptance_matrix.json`), whose discharge is to make both
arms `Err` (or prove pairing agreement with `std_socket`'s resolution) before
any source certificate carries a derived cell; the route card's brief carries
that line. Line numbers are as measured at `4f663934`; this note archives no
new run and changes no code.

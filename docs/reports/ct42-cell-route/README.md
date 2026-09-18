# CT42-CELL-ROUTE (wave 39) — route the socket protocol cell from source, and map CT42

Base `e878b362` (= `origin/main` at branch time). Report directory:
`docs/reports/ct42-cell-route/`. Everything numbered below was measured on
this worktree; measurement logs live in the worktree-private `measure/` and
`private/ct42-cell-route/` (gitignored) and are cited by name, and the gate's
exit files are copied into `gate/` beside this file.

**Headline.** CT42 (*two descriptors share an underlying logical state; expected:
preserve shared protocol-cell identity*) is **mapped** on the socket arm:
`source.socket.v1` gained the one construct that asks the disjointness question
of two of its sockets — `disjoint NAME NAME;` — and the two ends of one
accepted TCP connection are refused as one cell by the **unedited**
`DisjointFootprintsV1` checker at `Q664`, reachable from a `.qcore` program
through `compile_files` for the first time. CT 46/50 → **47/50**: a grown
denominator, which is mapping and not conformance (constraint 23);
sanitized-native stays 10 and proof stays 3. Before routing, the card fixed the
direction of the deriver's two fallback arms (they admitted; they now refuse)
and took the T09 edit that narrows the footprint certificate's
`DeclaredProtocolCellIdentity` premise for derived cells. **One finding on the
way:** the HTTP composition admitted the question with no judgement until the
card composed the cell check there too (§4.4). **One reserved word**
(`disjoint`), census-clean. **No new `Q` code**: `Q664` reached by condition
extension, `Q080`/`Q081`/`Q085`/`Q095` widened and stated. Rust plus audited
Lean (`AuditSocketFootprint.lean` amended, `AuditSocketCellRoute.lean` new:
137 → **138** audits). Mutations **10 of 10** red as required, harness shipped.

## 1. Scope, and what was true at base (re-derived, not inherited)

* `qcore_core::socket_footprint::{derive_socket_cells, footprint_program}`
  (card `CT42-PROTOCOL-CELL`, wave 31) existed and were called from
  `crates/qcore-core/tests/unit/socket_footprint.rs` and nowhere else —
  re-measured with `rg` at `e878b362`: no caller in `crates/qcore-frontend`,
  `crates/qcore-verification` or `compiler/`. No `.qcore` statement spelled
  `AssertDisjoint` over sockets, so `Q664` had no source path
  (`CT42-CELL-UNROUTED`).
* The deriver's two "unreachable for an admitted program" arms
  (`socket_footprint.rs:184` `continue`, `:211` `None => acquire(entries,
  binding, None)`) gave a binding a fresh singleton cell
  (`cell.unwrap_or(descriptor)`), and `disjoint_footprint.rs`'s
  `decide_disjoint` returns `Ok((true, …))` — *proved disjoint* — for two
  descriptors of distinct cells. So the arms **admitted** a disjointness
  question over a binding and its real connector: the admitting direction,
  the opposite of what `ct42-protocol-cell/README.md` §3 said
  (`CT42-CELL-FALLBACK-ARM-DIRECTION`, measured by `STABILITY-REEVAL-3` §4).
  Harmless only because `check_socket_program` refused first.
* `FootprintEvent::DeclareDescriptorAlias` was the only alias event, and it
  stamped `DeclaredProtocolCellIdentity` on every certificate that used it —
  for derived cells too (`CT42-CELL-DERIVED-PREMISE`).
* The C-twin arm: the contract loader refuses an int-typed resource identity
  (`CT-CTWIN-DESCRIPTOR`), and a descriptor's `-1` failure value is not
  expressible in `qcore-schemas`' `ResourceReturnMode`. `CT-LANE-5` §3
  measured it; that is the native/legacy arm and is **out of this card's
  scope**. Only the checked/model stages are mapped (§5).
* Tracker at base: `CT42-CELL-UNROUTED`, `CT42-MAP`,
  `CT42-CELL-FALLBACK-ARM-DIRECTION`, `CT42-CELL-DERIVED-PREMISE`,
  `CT42-CELL-UDP-QUEUE`, `CT42-CELL-PENDING-PHASE`, `CT42-CELL-DUP-SOURCE`
  all registered (`register.py --show`, `measure/register-show.log`).

## 2. The direction fix, before routing

`crates/qcore-core/src/socket_footprint.rs` now has two functions where it
had one: `derive_socket_cells` (transport check, then the walk) and
`derive_cells_of_admitted` (the walk alone, over a program the caller has put
through the checker). The split exists so the arms can be **driven**: the
transport checker refuses both shapes first, so through `derive_socket_cells`
they are unreachable and a test could not have shown them non-vacuous.

Both arms are `Err` now, and a third with them:

| Arm | Before (wave 31) | After | Fixture (unit test) |
|---|---|---|---|
| `accept` with nothing queued on its listener | fresh singleton cell | `SocketCellError::UnpairedAccept { event, binding, listener }` | `an_unqueued_accept_is_refused_not_given_a_singleton_cell` |
| `connect` the shared resolution gives no listener | `continue` (the later accept then got a singleton cell) | `SocketCellError::UnpairedConnect { event, client }` | `an_unresolved_connect_is_refused_not_skipped` |
| queued connector with no cell | fresh singleton cell | `UnpairedAccept` (same arm, matched as `(connector, cell)`) | covered by the accept fixture's shape |

**Which condition is the true cause.** Measured: an accept with nothing
queued is `Q085`'s own words ("an `accept` with no connection queued"), and
a connect no bind answers to is `Q085` too (`SOCKET-EXTERNAL-PEER`). The
deriver's refusals are therefore reported at **`Q085`** by condition
extension — the determinism boundary's condition reached from the deriver —
not at `Q664` (which is "two descriptors of one cell", a different
condition) and not at `Q082`. The connect fixture uses a declared address no
bind answers to (`Endpoint::Address(127.0.0.1:8080)`), because a *named*
target resolves verbatim and cannot reach the arm; the first draft with
`Endpoint::Named("nobody")` reached the accept arm instead, which is recorded
here as the measurement that chose the fixture.

Non-vacuity by mutation (§6): M1 and M2 restore the wave-31 arms and the two
fixtures go red. Lean: `QCore.SocketFootprint.step` is **partial** now
(`Option State`), `an_unresolved_connect_is_refused`,
`an_unqueued_accept_is_refused`, `an_orphaned_connector_is_refused` state the
three refusals, and `an_admitted_accept_takes_an_existing_cell` states the
direction — an admitted accept's binding cell is a cell some connector already
held, never fresh. The corrected direction sentence is **published beside**
the frozen wave-31 report (`docs/reports/ct42-protocol-cell/README.md`,
appended section; constraint 21).

`CT42-CELL-PENDING-PHASE` is not met and stays open, amended: nothing here
admits a surviving pending connect, and if a later route did, the arm it would
reach now refuses rather than admits.

## 3. The T09 decision: the edit, taken

`crates/qcore-core/src/disjoint_footprint.rs` gains
`FootprintEvent::DeriveDescriptorAlias { handle, descriptor, cell }`, handled
in **the same match arm** as `DeclareDescriptorAlias` — same handle minting,
same `Descriptor` footprint, same `protocol_cells` row, same `AssertDisjoint`
decision — with one difference, the whole point: `let declared =
matches!(event, DeclareDescriptorAlias {..})` and the assumption is stamped
only `if declared`. The socket projection (`footprint_program`) now emits
derived aliases only, so a derived-cell certificate carries **no**
`DeclaredProtocolCellIdentity`.

Why this is a narrowing and not a relabelling. The premise reads "a declared
protocol cell really is the shared logical state, and no undeclared descriptor
also advances it". For a derived cell both halves have a ground outside
anyone's declaration: the pairing is read off the transport checker's decided
connection structure, the transport admits no external peer (`Q085`), and
every socket of the program is enumerated by the same walk. What remains is
the transport checker's admission — the route's evidence, not the footprint
checker's premise — plus the model-fidelity boundary every socket-route
certificate already carries (the emitted program's kernel pairs what the
model paired; a foreign connector at a declared address at run time is
`SOCKET-INET-ADDRESSES`' recorded bounded run failure, not a check-time fact).

What is proved and what is observed, in those words: the **derivation** is
proved (`an_accepted_pair_shares_one_cell`, `a_fresh_cell_is_no_existing_cell`,
the refusals above) and the **narrowing** is proved
(`QCore.SocketCellRoute.a_derived_projection_stamps_no_declared_identity`,
non-vacuous by `a_declared_alias_stamps_the_premise`, and
`derived_and_declared_aliases_decide_alike` for everything else). That the
transport model's cross-name coupling *is* the shared logical state — a send
through one end decided by the other's committed stream, a close refused
while the other's bytes sit unread — is the Rust checker's **measured**
behaviour; `QCore.StdSocket` carries no peer-coupling theorem. Registered as
`SOCKET-CELL-COUPLING-UNPROVED` with a discharge condition (§8).

Rust evidence: `a_derived_projection_carries_no_declared_cell_identity_premise`
(the derived projection's certificate has no premise; the same cells declared
do; `protocol_cells` and `derived_footprints` equal). Mutation M3 makes the
variant stamp the premise and the test goes red. `CT42-CELL-DERIVED-PREMISE`
is met by its first arm ("the T09 checker edit") and retired.

## 4. The source construct, its census, and where it is answered

### 4.1 The construct

```text
stmt ::= … | "disjoint" NAME NAME ";"   -- the question: disjoint custodians?
```

`disjoint A B;` asks whether `A` and `B` are disjoint custodians — whether no
protocol state one advances is advanced by the other. It is the one statement
of `source.socket.v1` that asks a question rather than performing an
operation:

* **Reader** (`crates/qcore-frontend/src/source_socket.rs`): exactly two
  names then `;`, else `Q080` (shape). `SocketUse::Question` makes it a
  socket use: it selects the route, it needs `import std.socket;` (`Q015`
  otherwise, measured), and in a helper it is refused as a construct outside
  the entry (`Q080`, pinned).
* **Transport half** (`crates/qcore-core/src/std_socket.rs`): both names
  must be live sockets of the entry — `Q081` for a name that is none, `Q082`
  for one already closed — and the statement contributes a `Read` of each to
  the inherited ownership checker, so a question after `close` is `Q083` then
  `Q082` (pinned `["Q083", "Q082"]`). It derives **no** effect and moves no
  transport state. A question over a **watched** (loaned) socket is admitted
  exactly as a `send` on one is — a `Read` under the shared loan — pinned on
  the reactor composition.
* **Emission**: the question **erases**. An admitted program with
  `disjoint client listener;` emits byte-identical C, and publishes a
  byte-identical observation, to the same program with that line blank
  (`a_listener_and_an_end_are_admitted_and_the_question_erases_from_emission`;
  mutation M6 makes it emit a comment and the test goes red).
* **The answer** (`socket_footprint::check_socket_cells`, called by the
  driver after the socket certificate's recheck): derive the cells, project
  one `DeriveDescriptorAlias` per socket, append one `AssertDisjoint` per
  question, hand the whole program to the unedited `DisjointFootprintsV1`
  checker. Its `SharedProtocolCell` comes back as
  `SocketCellError::Footprint { event, left, right, diagnostic }`, reported
  under the diagnostic's own code **`Q664`** and the **`Concurrency`** pass
  (the band's owner per §L21), attributed to the question's span, with a
  message naming both custodians and the cell. Every admitted question is a
  `DisjointnessJudgement` on the new `SocketCellCertificate`, published on
  `CheckedSocketModule::cells` for every admitted module (question or none)
  and rechecked (`recheck_socket_cell_certificate`, a tampered cell refused
  at `Q088`).

### 4.2 The keyword census (constraint 14), run

`disjoint` is a clause word at a statement head, never a reference; without
reservation name resolution collects it and the statement refuses at `Q013`
(the ten socket words' measured shape). Census over every `.qcore`/`.qc` file
in the tree (**525** files, `find` excluding `target/`) and every Rust string
literal:

* `.qcore`/`.qc`: **11** occurrences, **all inside `//` comments**
  (`examples/sockets/inet_v6_echo.qcore:6`, `compiler/tests/fixtures/slice_permissions/disjoint_halves.qcore:1`,
  and nine lines of CT07/CT47/CT49 fixture comments); **zero** in any QCore
  source position.
* Rust string literals containing the whole word: **13**, every one a test
  name, an `expect` message or prose (`native_library_contract.rs:486`,
  `slice_permissions.rs:359`, `write_lend.rs:320`,
  `write_lend_tasks_exercise.rs:750`, `task_route_tsan.rs:238,263`,
  `chunk_contract.rs:60`, `transfer.rs:225`, `socket_footprint.rs:161,274`,
  `disjoint_footprint.rs:1233` and `:28`, `std_string.rs:387`).

The already-reserved `assert` and `footprint` were considered and not taken:
`assert footprint A B;` names neither the question nor its answer, and the
spelling chosen is the checker's own vocabulary (`AssertDisjoint`,
`DisjointnessJudgement`). **+1 reserved word**, recorded in constraint 14's
ledger as the first since `LOOP-V1`, ending the wave-28-to-37 zero-word run
deliberately. The `statement_heads()` pin in `compiler/tests/std_socket.rs`
advanced from eight to nine, deliberately and stated in the test.

### 4.3 The recorded decision, re-affirmed: a datagram pair is not a cell

`CT42-CELL-UDP-QUEUE`'s discharge condition is "a later card either
re-affirms the decision with its ground or lands the datagram cell, in both
cases citing this entry". Re-affirmed, with its ground, citing the entry: a
`sendto` appends to the receiver's queue, but the two sockets share no
connection phase — either end closes independently, a queued datagram is
discarded locally at close, no operation through one name is refused on the
other's protocol state — so two datagram sockets are independent custodians
and `disjoint client server;` over a UDP pair is **admitted**
(`a_datagram_pair_is_admitted_as_disjoint`, and CT42's `datagram_pair.qcore`
accept control). Retired by re-affirmation; the decision is recorded in
constraint 15p so reopening it is deliberate.

### 4.4 The finding: the HTTP composition admitted the question unanswered

`source.http.v1`'s reader hands every statement it does not spell to
`source_socket::socket_statement`, so `disjoint client server;` lowered into
the HTTP transport program, and `compile_http_source` ran only the socket
checker (which admits the statement as custody-neutral) — never the cell
check. Measured with `quirin check` on `examples/http/exchange.qcore` plus
`disjoint client server;` after its `accept`:

* before the composition: exit **0**, `checked 1 function(s); entry main` —
  admitted with **no judgement** (`measure/http_disjoint.exit`, `.out`);
* after: exit **1**, `measure/http_disjoint.qcore:61:5: Q664: client and
  server are the two custodians of one accepted connection's protocol cell
  (cell 1) …` (`measure/http_disjoint_after.exit`, `.out`).

The driver now composes `check_socket_cells` over the HTTP route's
re-lowered transport (`std_http::lower_transport`, total on an admitted
program), places refusals by its `attribution` map on the HTTP statement the
reader wrote, reports a footprint refusal under `Q664`/`Concurrency` exactly
as the socket route does, and reports the derivation's own (unreachable)
refusals at `Q095` — that condition widened and stated. Two tests pin it
(`an_http_exchanges_two_ends_are_refused_as_one_cell_at_q664`,
`an_http_module_asking_about_its_listener_is_admitted_and_the_question_erases`),
mutation M9 empties the transport the check sees and the first goes red.
`CheckedHttpModule` gains `cells` too. A tracker entry this card had
registered on the pre-fix expectation (that HTTP refuses by shape) was
**withdrawn by the same card** before merge, the withdrawal recorded in
`scripts/register.py` (`WITHDRAW`) rather than silently dropped.

### 4.5 No new `Q` code (the `Q-NAMESPACE-DECISION` icebox, re-affirmed)

| Code | Widening | Stated in |
|---|---|---|
| `Q664` (`Concurrency`) | reached from source for the first time; the condition itself is not widened | §L21 `Q6xx` row; AGENTS.md `Q650`–`Q667` row |
| `Q080` | the question's shape (not exactly two names) | §L21 `Q080` row amendment; AGENTS.md `Q080` row |
| `Q081` | a question naming no socket of this entry | same |
| `Q085` | the cell derivation's two refusals (an accept with nothing queued; a connect the resolution gives no listener) | same |
| `Q095` | a refusal of the transport's cell derivation beside the transport checker's own, on the HTTP route | §L21 `Q090` row amendment; AGENTS.md `Q090` row |

`Q083` and `Q082` are reached unchanged (a question of a closed socket).

## 5. CT42 mapped: the stage record

Fixture directory `compiler/tests/fixtures/ct/CT42/`: `case.qcore` (the tcp
echo shape asking `disjoint client server;` after the accept), `expected.json`
(verdict `reject`, `sequential_reject` with `diagnostic_sequence: ["Q664"]`
and the message fragment naming both custodians), and five controls:

| Control | Expectation | Measured | Isolates |
|---|---|---|---|
| `listener_question.qcore` | accept, `StandardSocketV1` | checks, emits C byte-identical to the twin | the connection cell: asking about the listener is admitted |
| `two_connections.qcore` | accept | checks, emits | one connection's identity: cross-pair questions admitted |
| `datagram_pair.qcore` | accept | checks, emits | `CT42-CELL-UDP-QUEUE` re-affirmed |
| `closed_custodian.qcore` | reject `["Q083", "Q082"]` | inherited ownership and transport refusals | liveness, before any cell is derived |
| `not_a_socket.qcore` | reject `["Q081"]` | transport refusal | the custody position |

Harness: `MAPPED_CASES` and `SEQUENTIAL_REJECT_CASES` gain `CT42`
(`sequential` is the harness's name for the non-concurrent source routes; the
socket route is one, as the string route was for CT06/CT11), the
`execution_profile` label table gains `StandardSocketV1`, and both
`EVIDENCE_FILES` lists (`ct_corpus.rs`, `scripts/ct_stage_summary.py`) plus
`acceptance.json`'s `derived_from` gain
`docs/reports/ct42-cell-route/ct_stage_evidence.json`. That record carries the
stage matrix the harness rederives — `parse`, `elaborate`,
`sequential_whole_program` **executed** by `run_mapped_case` on the case's own
artifact (`Q664` is a band-600 refusal, so the whole-program check was reached);
every other stage `not_applicable`; `native_sanitized` and `proof`
**unassessed** — and `retired_unmapped_reasons` for CT42 naming
`docs/reports/ct-ctwin-lane/ct_stage_evidence.json` (the record holding the
live row; it keeps its bytes). The manifest row was written to match and
`ct_corpus` passes (`measure/ct-corpus.log`: 6 passed);
`scripts/ct_stage_summary.py --write` regenerated the acceptance record's
generated fields and `--check` reports no drift (mapped 47 / unmapped 3;
`measure/ct-summary-check.log`). The derived CT reasons map is **not** touched
by hand (constraint 25): the regeneration derives it.

Python tests that used CT42 as the live unmapped example
(`scripts/tests/test_ct_stage_summary.py`) now use CT41, the one case still
carrying a live lane row, each edit commented with the wave; the relabel
test's tally assertion becomes absence-and-sum-3 because CT41 was the shared
bucket's last holder. 303 Python tests OK after the edits (`measure/python2.log`).

What the mapping does not claim: no native, sanitizer or proof evidence. The
case is a refusal that emits nothing; the three accepted controls emit C that
no runner compiles or runs (`CT42-CONTROLS-NATIVE-UNRUN`, registered); the
Lean theorems are about the derivation and the checker, not this artifact's
reference run, so citing them as the proof stage would be the overstatement
constraint 23 forbids. The C-twin arm stays `CT-CTWIN-DESCRIPTOR`'s. Scenario
words with a boundary: *descriptors* is carried by the connection-cell
reading (two descriptor slots, one program, one thread), not the dup reading
(`CT42-CELL-DUP-SOURCE` left open).

## 6. Falsification (mutations, shipped)

`scripts/mutations.py`, log `mutations.log` (both in this directory). Each
mutation edits one arm, runs exactly the named test, writes the original bytes
back and asserts the round trip sha256-equal. Run at `9bea6b1e` minus the
docs (the code at that tip): **10 of 10 as required**.

| # | Arm | Test shown red |
|---|---|---|
| M0 | baseline, unmutated | both suites GREEN |
| M1 | deriver accept arm back to a singleton cell | `an_unqueued_accept_is_refused_not_given_a_singleton_cell` |
| M2 | deriver connect arm back to `continue` | `an_unresolved_connect_is_refused_not_skipped` |
| M3 | T09 stamps the premise for a derived alias too | `a_derived_projection_carries_no_declared_cell_identity_premise` |
| M4 | the route pushes no `AssertDisjoint` | `the_two_ends_of_one_connection_are_refused_as_one_cell_at_q664` |
| M5 | the question reads neither custodian | `a_question_asked_of_a_closed_socket_is_refused_by_the_inherited_checkers` |
| M6 | the question emits a line instead of erasing | `a_listener_and_an_end_are_admitted_and_the_question_erases_from_emission` |
| M7 | footprint refusal under the socket pass | `the_two_ends_of_one_connection_are_refused_as_one_cell_at_q664` (pass pin) |
| M8 | the reader admits three names | `a_malformed_question_is_q080` |
| M9 | the HTTP composition checks an empty transport | `an_http_exchanges_two_ends_are_refused_as_one_cell_at_q664` |

## 7. Gate

See §7 below the residuals — the figures are the detached run's own exit
files, copied into `gate/`, and `gate.json` beside this file.

## 8. Residuals and retirements (exact names)

**Retired** (each discharge condition read in `register.py --show` and met
word for word; `measure/register-show.log`):

* `CT42-CELL-UNROUTED` — "wire the deriver into the socket route behind
  source syntax that asks the disjointness question": §4.
* `CT42-MAP` — "a fixture, a manifest row and stage evidence on one of the
  two arms": §5, the socket arm.
* `CT42-CELL-FALLBACK-ARM-DIRECTION` — "publish the correction beside the
  frozen report and make both arms `Err` … before any source certificate
  carries a derived cell": §2, landed before the route.
* `CT42-CELL-DERIVED-PREMISE` — "the T09 checker edit, or a recorded
  statement": §3, the edit.
* `CT42-CELL-UDP-QUEUE` — "re-affirms the decision with its ground … citing
  this entry": §4.3.

**Amended:** `CT42-CELL-PENDING-PHASE` (the admitting arm is gone; the
discharge stands).

**Registered** (`recorded_by: CT42-CELL-ROUTE`, each with a discharge
condition):

* `SOCKET-CELL-COUPLING-UNPROVED` — the transport model's cross-name coupling
  *is* the shared logical state: measured, not proved; discharge is a
  `QCore.StdSocket` theorem of mutual dependence cited from the module doc.
* `CT42-CONTROLS-NATIVE-UNRUN` — the three accepted controls emit C no runner
  executes; discharge is a named sanitized run of `listener_question.qcore`'s
  emitted C, or a sequential-reject control runner on the `CT34-SANITIZED`
  precedent, crediting the control's file and never the case's.

**Withdrawn by this card before merge:** `SOCKET-CELL-QUESTION-COMPOSED-ROUTES`
(registered on the pre-fix HTTP expectation, false after §4.4; recorded in
`register.py`'s `WITHDRAW`).

**Left open, condition not met:** `CT42-CELL-DUP-SOURCE`,
`CT-CTWIN-DESCRIPTOR`.

Tracker: 228 → **225** entries (two registered, five retired, one withdrawn).

## 9. For the integrator

Every ledger item, as an item:

1. **CT row**: `46/50 mapped, 4 unmapped; sanitized-native 10 of 46; proof
   3 of 46` → `47/50 mapped, 3 unmapped; sanitized-native 10 of 47; proof 3
   of 47` — edited in AGENTS.md in this tree, with the CT42 sentence in the
   "unmapped" paragraph rewritten (CT42 left the set; "the three unmapped").
   A grown denominator is mapping, not conformance.
2. **Lean audits**: 137 → **138** (`AuditSocketCellRoute.lean` new;
   `AuditSocketFootprint.lean` amended with four new theorem names). The
   gate.json here records 138; the merged-gate row of AGENTS.md is the
   close's to move.
3. **`named_followups`**: 228 → **225**; the AGENTS.md count line edited in
   this tree. Registered: `SOCKET-CELL-COUPLING-UNPROVED`,
   `CT42-CONTROLS-NATIVE-UNRUN`. Retired: the five in §8. Amended:
   `CT42-CELL-PENDING-PHASE`. Withdrawn: `SOCKET-CELL-QUESTION-COMPOSED-ROUTES`.
   The matrix is regenerated with `--gate docs/reports/ct42-cell-route/gate.json`.
4. **Standing constraint 15p** landed in AGENTS.md in this tree, in these
   words: *A socket program's protocol cells are derived, never declared, and
   the derivation refuses where it cannot pair* — four rules (the unpaired
   arms refuse; a derived alias stamps no `DeclaredProtocolCellIdentity`;
   every composition that lowers to a socket program composes the cell check;
   a datagram pair is not a cell), quoted in full there.
5. **Constraint 14 ledger**: `CT42-CELL-ROUTE` **+1** (`disjoint`), the run
   sentence amended to "between `LOOP-V1` and wave 39".
6. **§L21**: three row amendments (`Q080`, `Q090`, `Q6xx`) in
   `docs/QCORE_SPEC_r0_3.md`; the AGENTS.md diagnostics table's `Q080`,
   `Q090` and `Q650`–`Q667` rows amended to match.
7. **Increment record**: `docs/development/increments/wave-39.md` created in
   wave-38's shape with this card's paragraph; the index's pointer and table
   row updated. Sibling cards' paragraphs union at the merge.
8. **Pinned expectations moved deliberately**: `statement_heads()` 8 → 9
   (`compiler/tests/std_socket.rs`); `MAPPED_CASES`,
   `SEQUENTIAL_REJECT_CASES`, `EVIDENCE_FILES` (23 → 24) in `ct_corpus.rs`;
   `EVIDENCE_FILES` in `scripts/ct_stage_summary.py`; the CT41/CT42 examples
   in `scripts/tests/test_ct_stage_summary.py`.
9. **Shared-file note (file ownership)**: this card edited
   `crates/qcore-verification/src/driver.rs` (the socket and HTTP compile
   functions and two module structs), `crates/qcore-frontend/src/frontend.rs`
   (one reserved word), `compiler/tests/ct_corpus.rs` (the three tables and
   one label arm), `scripts/ct_stage_summary.py` and
   `scripts/tests/test_ct_stage_summary.py` (the mapping mechanics),
   `docs/reviews/r0_3/acceptance.json` (`derived_from` and the generated
   fields via `--write`), `docs/reevaluation/ct_manifest.json` (the CT42 row).
   None is owned by a sibling wave-39 card by the common brief's list;
   `CT07-TWO-THREAD-PROOF-LEG` owns `ct_corpus.rs`'s `PROOF_GATED_CASES`
   only, which this card did not touch.
10. **Decision not taken**: `CT-CTWIN-DESCRIPTOR` (the loader cascade into
    `qcore-schemas`) — out of scope by the brief; the native/legacy arm stays
    unassessed in prose.
11. **The `Q-NAMESPACE-DECISION` icebox** ran its consequence again: five
    conditions widened, zero codes allocated (§4.5).

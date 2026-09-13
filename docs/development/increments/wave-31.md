# Wave 31 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

Write lends crossing the spawn boundary (2026-09-13): `WRITE-LEND-TASKS` lands
the task-crossing half the residual `SLICE-PERMISSIONS-WRITE-LEND` named —
"both parties are one sequential entry's bindings" — and the direction CT10's
rejection needs. A spawn capture `lend PLACE LOW..HIGH` delivers an ungraded
`lend<Buffer>` worker parameter: a **range write lease**, write authority over
`[LOW, HIGH)` of an owned Buffer's payload (`BUFFER_PAYLOAD_BYTES = 4`, the
`int32_t`'s real bytes) transferred to one worker for exactly spawn..join. The
rules live in the scoped-task ledger's new `ResourceState::WriteLeased` and
every layer inherits them: two leases are admitted exactly when disjoint —
decided by `ownership::axes_overlap`, made `pub` and called rather than restated
— the owner holds no whole-payload authority (read included) while any lease is
out, an empty lease refuses (a deliberate divergence from the sequential lend's
empty admission, in the refusing direction), `MAX_WRITERS = 2`, read shares and
leases never coexist, and only the creator's join reclaims. The transferability
judgment gains its fourth row (`TransferMode::LendWrite`,
`ShareProtocol::DisjointWriteLeases`, delivered `TransferKind::WriteLease`;
`TransferClass::MovedOnly` gains `mutably_shareable_under`), with the plain
`write` capture's pinned refusal re-decided untouched over the grown 14 x 5 x 14
matrices. Zero new codes (two condition widenings stated in spec §L21: `Q600`
boundary and ledger, `Q020` parameter), zero reserved words (`lend` contextual
in two new positions, censused over 487 fixture files), zero tokens. The
exclusivity statement is audited:
`QCore.WriteLendTasks.no_byte_has_two_write_authorities` — at any point of any
spawn/join interleaving (reachability over admitted ledger opens and closes) no
byte has two write authorities, owner-versus-holder and holder-versus-holder in
one count — backing the judgment's new admitting arm as `LeaseObligation` (Lean
audits 123 → **124**, `AuditWriteLendTasks.lean`). 17 end-to-end tests in
`compiler/tests/write_lend_tasks.rs` (the two-worker disjoint fixture with its
pinned lease trace, all four card refusals — overlap, owner write, release
before the join, varying bound — plus kind mismatches, a third holder, mixing,
temporal reuse, and one ASan/UBSan/LSan native run: two threads holding leases
of one allocation counted once); 4 ledger rows and re-pinned transfer matrices
in `qcore-core`. Scope honesty: the worker **holds** and does not yet
**exercise** the authority — no operation admits a lease in a worker body — so
no two-threads-mutating claim is made; that is the flagship residual
`WRITE-LEND-TASKS-EXERCISE`, beside `-EXTENT`, `-VARYING-BOUND`, `-EPOCH`, `-F1`,
`-WINDOWS-RUN` and `-PRELUDE-FRAGMENT`. CT07's recorded sequential-only boundary
is now dischargeable and CT10's concurrency half now exists at byte granularity
(readiness statements, no mapping; manifest and matrix untouched). Deliberate
pin updates disclosed: the transferability message texts, the capture
grammar-gate message, the task-parameter message, the transfer matrix pins, and
— found by the recovery gate rather than predicted — the **two F02 generated-C
digests**, whose fourth recorded move follows from the shared diagnostic
prelude's two new kinds reaching every task program; no pre-existing program's
printed bytes move, and F02's relation and refusal assertions are untouched and
green against fresh native runs. The card was written on `469f6509`, interrupted
mid-gate by a model usage limit, and recovered onto `2492825b` by a second
agent, which reconciled only `DEVELOPMENT.md` (the documentation split had
turned its increment section into a table of contents), repaired the digest pins
and re-measured every gate row. Final gate: **4,505** workspace tests (0 failed,
8 ignored, `--no-fail-fast`), **200** Python tests with the one expected
`lean_audits: 123 != 124` tripwire left for the integrator, **124** Lean audits,
both probes exit 0, fmt/clippy clean; measured while two other worktrees ran
their own workspace suites, an earlier run under that load tripping
constraint 16's `vp04_vp07_vp08_process_lifecycle_and_payload_budgets` row,
which the recorded run passes. Evidence:
`docs/reports/write-lend-tasks/README.md`.

Wave 31, card `CT-WIRE-LANE-3` (2026-09-13): the CT lane taking the two
decisions waves 29–30 made ready. **CT25 mapped, 42/50 → 43/50** — *"Cleanup
destroys owner after joins and loan expiry"*, expected Accept, the
`composed_epoch` route's second accepted case and its first **adopting** one,
mapped exactly as `docs/reports/region-joins-loan/README.md` §7 prescribed:
the fixtures moved from `epoch_credit/CT25_LOAN/` into
`compiler/tests/fixtures/ct/CT25/` with `region_joins_loan.rs` repointed (11
tests green on the moved bytes), one new refusing control
(`before_join.qcore`, `["Q600"]`) materializing the in-memory placement
mutation, and the `epoch_credit_split` verdict mode widened for the adopting
shape — the pinned evidence gains the region's **released duty list**
(`["owner"]`, compared wholesale against the driver's re-derived publication)
and the **derived capture list** (`[["read"]]`, against both the ledger and
the declared rows), and the duty premise splits in two (no released owner ⇒
CT22's duty-free no-`Acquire` assertion byte-for-byte; released owners ⇒
exactly one published body `Acquire` per released owner, the adoptions).
Every word of the scenario is a checked fact of an unedited model, stated
rule-by-rule in the semantic profile; the case's emission/native stages read
`unassessed` beside CT22's (`CT-COMPOSED-NATIVE` unchanged), sanitized-native
stays 7 of 43 and proof 2 of 43. **CT24 examined and withdrawn** with the
measurement published (the wave-26 mechanism): on the composed route — the
only route with an executed cleanup-destroy — a region before the join is
refused for its placement whatever its duties (adopting: the ledger's own
`LoanOutstanding` / a lend of the destroyed owner; duty-free: still refused),
so no control can isolate the destroy and the wave-9 attribution rule fails;
on the task route the wave-30 attributed `ScopeBusy` message names the
construct but the route executes no destroy at scope exit, and mapping there
would give CT24's "cleanup" a different referent from CT25's in one scenario
pair. The constraint-25 trap was **measured, not assumed**
(`measure/ct24_constraint25_probe.py`): mapping CT24 moves the derived
reasons map's keys to `["CT47"]` and reddens `check_matrix_ct_reasons`
against a matrix this lane did not regenerate — published in the record so a
future lane inherits a measurement. No retirement is declared for CT25,
correctly: its label was archive-held (no lane record ever carried a row), so
mapping removes it from the tally with nothing to retire, like CT21/CT06.
Zero new Q codes, zero reserved words, no crate-source change;
`EVIDENCE_FILES` 16 → 17 in both lists plus `derived_from`;
`acceptance.json` regenerated through `--write` with the hand-kept note's
stale "proof 0 of 42" refreshed to 2 of 43 and the CT24 reason re-measured;
Python pins 8 → 7 updated, suite **200 OK** (193 at the wave-30 close plus
the post-close `check_lean_audits` hardening's 7). Report and stage-evidence
record: `docs/reports/ct-wire-lane-3/`.

Wave 31, card `CT42-PROTOCOL-CELL` (2026-09-13): the exploratory
shared-protocol-cell card, closed as **survey plus the bounded F4 slice**
rather than withdrawal. The survey found the cell notion already real at
model level, *declared*: `disjoint_footprint` carries the identity
(`Descriptor { descriptor, cell }`), the refusal (`SharedProtocolCell`,
`Q664`) and the `DeclaredProtocolCellIdentity` premise, while
`source.socket.v1`'s completed connection is the one source shape with two
descriptor-valued names over one shared logical state — its cross-name
refusals pre-exist, but the shared state had no identity (mutual `peer`
references). What landed is the derivation that connects them:
`qcore_core::socket_footprint` derives protocol cells from an **admitted**
socket program (fail-closed — a refused program derives nothing; the
`connect` target read from the checker's own `resolve_addresses`, descriptors
numbered identically to the certificate's), giving each completed connection
one cell with two custodians and every other socket a singleton, then
projects one `DeclareDescriptorAlias` per socket onto the **unedited**
footprint checker, where the pair is refused at the existing `Q664` and the
listener decides disjoint. Zero new codes, zero reserved words, no emission,
checker or route-selection file touched (byte-identity structural). Lean:
`QCore.SocketFootprint` (freshness invariant, pairing theorems, composition
with `decideDisjoint`; audits 121 → **122**, `propext`/`Quot.sound` only).
Deliberate boundaries, each named in the report: a datagram pair is not a
cell (`CT42-CELL-UDP-QUEUE`), the pending phase stays the transport
checker's (`CT42-CELL-PENDING-PHASE`), no dup-like source form exists
(`CT42-CELL-DUP-SOURCE`), the derived-premise narrowing needs a T09 edit
(`CT42-CELL-DERIVED-PREMISE`). CT42 stays **unmapped** — manifest, matrix
and CT fixtures untouched; the wave-17 C-twin blocker
(`CT-CTWIN-DESCRIPTOR`) stands; the readiness statement and the mapping
lane's two candidate arms are `docs/reports/ct42-protocol-cell/README.md`
§5. Card gate: 4,473 workspace tests (0 failed, 8 ignored,
`--no-fail-fast`), fmt/clippy clean, both probes exit 0, `Passed 122 Lean
audits`, and the Python row 199 OK of 200 with the one failure being the
disclosed lean-audit matrix tripwire (the committed matrix pins 121 audits;
regeneration is the integrator's, constraint 25).

C gateway scalar writes, expression frames and conditional custody (2026-09-13):
`C-GATEWAY-WAVE` implements the selected `AOM-POISON-NOOP-INCREMENT` and
bounded `AOM-POISON-CONDITION-FRAME` repairs, and an acyclic conditional-identity
capability for `AOM-DESTROY-JOINED-PROVENANCE`. Scalar writes forget stale branch
facts, including the production `flag = !0` case exposed during the staged
review; complete expression refusals spare only positively unwritable automatic
bindings; a guarded release may settle a known local acquisition while retaining
the lookup's `UnknownAlias`. Unshadowed, non-address-taken locals and shallow
release contracts bound that last rule; the production `input_chunk_get` residual
remains open. Sixteen abstract Lean rule theorems carry explicit extraction,
contract and no-UB premises; no whole-C correctness claim is made. The preceding
worktree sweep retains all four secondary checkouts because each has unique
unmerged work, and verifies the existing recovery hashes. Fresh BEFORE reproduces
the wave-30 diagnostic and chunk-contract archives byte for byte; the five staged
measurements, every changed leaf, provisional diagnostic boundaries and new
baseline are owned by `docs/reports/c-gateway-wave/`. Final gate: **4,484** workspace
tests (0 failed, 8 ignored, `--no-fail-fast`), **200** Python tests, **123** Lean
audits, both probes exit 0, fmt/clippy clean; provider 122 carried unchanged.
The sandbox failure, its green outside-sandbox repeat and the final source's
separate green repeat are all retained. Contracts remain 74, CT mapping remains
42/50 (proof 2 of 42), and full language conformance remains false.

Documentation structure and bounded retrieval (2026-09-13): the main guides
adopt the compact-context reading strategy, and `DEVELOPMENT.md` becomes a
table of contents over focused setup, layout, cycle-evidence and increment
pages under `docs/development/`. Forty-one pages split at original section or
record boundaries; every old heading remains reachable from the root guide.
The former guide's full text is reconstructible from the migration record:
relative links are rebased to their original targets, the two current
build examples gain the standing gate's already-required `--no-fail-fast`,
and current setup/layout pages use the `quirin` binary name while keeping
the `qcorec` package name.
Historical checkpoint prose and all 44 standing constraints are preserved;
new increment paragraphs now go to the file named by the history index.
This changes documentation retrieval, not compiler semantics, verdicts or
conformance. The measurements, preservation checks and gate results are in
`docs/reports/development-docs-structure/`; the automated packet router and
the planned reduction of always-loaded agent guidance remain separate work.

<!-- migrated-section:start -->
Wave 31, card `CT42-PROTOCOL-CELL` (2026-09-13): the exploratory
shared-protocol-cell card, closed as **survey plus the bounded F4 slice**
rather than withdrawal. The survey found the cell notion already real at
model level, *declared*: `disjoint_footprint` carries the identity
(`Descriptor { descriptor, cell }`), the refusal (`SharedProtocolCell`,
`Q664`) and the `DeclaredProtocolCellIdentity` premise, while
`source.socket.v1`'s completed connection is the one source shape with two
descriptor-valued names over one shared logical state — its cross-name
refusals pre-exist, but the shared state had no identity (mutual `peer`
references). What landed is the derivation that connects them:
`qcore_core::socket_footprint` derives protocol cells from an **admitted**
socket program (fail-closed — a refused program derives nothing; the
`connect` target read from the checker's own `resolve_addresses`, descriptors
numbered identically to the certificate's), giving each completed connection
one cell with two custodians and every other socket a singleton, then
projects one `DeclareDescriptorAlias` per socket onto the **unedited**
footprint checker, where the pair is refused at the existing `Q664` and the
listener decides disjoint. Zero new codes, zero reserved words, no emission,
checker or route-selection file touched (byte-identity structural). Lean:
`QCore.SocketFootprint` (freshness invariant, pairing theorems, composition
with `decideDisjoint`; audits 121 → **122**, `propext`/`Quot.sound` only).
Deliberate boundaries, each named in the report: a datagram pair is not a
cell (`CT42-CELL-UDP-QUEUE`), the pending phase stays the transport
checker's (`CT42-CELL-PENDING-PHASE`), no dup-like source form exists
(`CT42-CELL-DUP-SOURCE`), the derived-premise narrowing needs a T09 edit
(`CT42-CELL-DERIVED-PREMISE`). CT42 stays **unmapped** — manifest, matrix
and CT fixtures untouched; the wave-17 C-twin blocker
(`CT-CTWIN-DESCRIPTOR`) stands; the readiness statement and the mapping
lane's two candidate arms are `docs/reports/ct42-protocol-cell/README.md`
§5. Card gate: 4,473 workspace tests (0 failed, 8 ignored,
`--no-fail-fast`), fmt/clippy clean, both probes exit 0, `Passed 122 Lean
audits`, and the Python row 199 OK of 200 with the one failure being the
disclosed lean-audit matrix tripwire (the committed matrix pins 121 audits;
regeneration is the integrator's, constraint 25).

Worktree census hygiene (2026-09-13, post-wave-30 close): the
`SOCKET-INET-ADDRESSES` census test
(`std_socket_inet.rs::this_card_reserves_no_word_and_its_address_rule_collides_with_nothing`)
walked the repository root recursively and, when run from the **primary
checkout**, recursed into `.claude/worktrees/*` — sibling checkouts of this
same repository — finding their copies of the two published examples and
failing against the pinned two-file expectation. The walker now skips any
subdirectory carrying its own `.git` entry (a nested git worktree or embedded
checkout is another tree, not this one), the same environment-shape family as
the wave-29 submodule-diagnostic precondition. The census itself is
unweakened: over one checkout's tree it visits exactly the files it always
visited (the >400-file floor and the two-example pin are untouched), and the
other test walkers were surveyed — every other `read_dir` use starts inside a
bounded directory, so this was the only root-recursive walk. Measured in both
environments: 36/36 in an isolated worktree, 36/36 from the primary checkout
after the fix.

<!-- migrated-section:end -->

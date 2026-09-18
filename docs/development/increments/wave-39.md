# Wave 39 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

**`CT07-TWO-THREAD-PROOF-LEG` (2026-09-18).** CT07's `proof` stage moves
`unassessed` → **executed** on a named host-unconditional test, and the proof
numerator moves **3 of 46 → 4 of 46** — evidence about one case's artifact,
never conformance (constraint 23); no other count moves (mapped 46, sanitized
10, no unit or function status, no corpus number). Base `e878b362`. **The
theorem bound** is wave 31's
`QCore.WriteLendTasks.no_byte_has_two_write_authorities` — for every ledger
state a spawn/join interleaving can *reach* and every byte, at most one write
authority — instantiated on CT07's own two-thread ledger state as
`QCore.Ct07LeaseLedger.ct07_no_byte_has_two_write_authorities`; wave 36's
`no_byte_is_written_by_two_parties` is instantiated on the same state, with
the artifact's two byte writes (writer 1 byte 0, writer 2 byte 2) shown
admitted against it — a consequence checked on top of the lease binding, not a
second binding route. **The binding is a new pattern, through the ledger**:
the CT01/CT37/CT03 route is closed by decision, `reference_trace` refusing
this run's `BufferLeased` event under constraint 15n as amended
(`CT07-PROOF-LEG-PATTERN-UNAVAILABLE`), and it was not touched and the
alphabet did not grow. Instead the new Lean module adds one general theorem,
`replay_reaches` — whatever state a successful replay of admitted opens and
closes produces from the empty ledger is `Reachable` — and
`compiler/tests/ct07_proof_leg.rs::ct07_artifact_is_the_lean_proved_disjoint_halves_shape`
lowers `fixtures/ct/CT07/case.qcore` through the real pipeline, reads the two
lease captures out of the checked task IR, runs the reference execution,
transcribes its lease opens and closes into the Lean `Step` vocabulary
(`[lease 1 [0,2), lease 2 [2,4), reclaim 1, reclaim 2]`, holders the
children's own identities, exactly `ct07Steps`), replays them with the
clause-for-clause transcription of `replay`, requires `Some` (the `Reachable`
witness), recomputes the kernel-decided pin `ct07_ledger_digest =
8876998389998880787` from the artifact with wrapping `u64` arithmetic, checks
the scenario shape `dischargesDisjointHalves` (two leases, two distinct
holders, every payload byte under **exactly one** write authority — the
theorem's own `writeAuthorities`, met with equality where the theorem proves
`≤ 1`), and drives the production `ScopedTaskLedger` through the same steps,
requiring its `(holder, range)` set to agree after every step and
`LeaseRange::overlaps` (`ownership::axes_overlap`) to agree with the
transcribed `overlaps` on the artifact's ranges — the `Reachable` term ↔
ledger-state correspondence the residual's discharge named. **Non-vacuity,
shipped** (`docs/reports/ct07-two-thread-proof-leg/scripts/mutate_ct07_proof_leg.py`,
`mutations.log`): the three fixture mutations are each measured at **two**
catch points — overlapping halves `[0,3)`/`[2,4)` and the empty half `[2,2)`
are caught by the **checker** (`Q600`, the ledger's `OverlappingLeases`; the
capture's "must cover at least one byte") before any artifact exists, and the
procedure fed their would-be steps directly refuses each at the clause the
theorem names (disjointness; nonemptiness); the single writer is **admitted**
by the checker — a reachable state the theorem holds of — and refused by the
**binding** at the steps assertion, the shape (bytes 2 and 3 under no
authority) and the pin; six rule mutations of the transcription each redden
the named test (one shows the shape's `== 1` strictly stronger than the
theorem's `≤ 1`); two Lean mutations fail `lake build` by `decide`. Lean audits
**137 → 138** (`AuditCt07LeaseLedger.lean`, 24 declarations, `propext`/
`Quot.sound` only). `PROOF_GATED_CASES` and `RUNNER_REGISTRY` gain the CT07
row; the manifest's CT07 proof row, the fixture's `stages_executed`, the
acceptance record's generated fields and `derived_from`, and both
`EVIDENCE_FILES` copies move in lockstep, the wave-37 `ct-lane-5` record
superseded from this card's own record (constraint 21). No `Q` code, no
keyword, no lexer change (census: +0, nothing to census). Retired, each
verified registered and met word for word: `CT07-TWO-THREAD-PROOF-LEG`,
`CT07-PROOF-LEG-PATTERN-UNAVAILABLE`; registered:
`CT07-PROOF-LEG-F02-PREMISE-PIN` (the leg's last assertion re-measures that
`AdmittedCausalOrder::derive` refuses CT07's run, so a later alphabet growth
turns the leg red by design and must remove the pin deliberately);
`named_followups` **228 → 227**. Gate at tip `81f783ce`: **4,644 passed / 0
failed / 9 ignored** over 235 summaries and 233 targets (+5 on the wave-38
close's 4,639), fmt/clippy clean, both probes 0, **138** Lean audits, Python
303 OK after the `--gate` regeneration (the detached run's own Python step red
on the two regeneration tripwires, `228 != 227` and `137` against `138`,
recorded in `gate.json`), Clang present, provider 122 carried.
Report: `docs/reports/ct07-two-thread-proof-leg/README.md`.

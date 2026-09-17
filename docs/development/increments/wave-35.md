# Wave 35 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

`MUTEX-SHARE-CROSS-ARMS` (2026-09-18), discharging `STABILITY-REEVAL` #1's
residual `MUTEX-SHARE-CROSS-UNEXERCISED-ARMS`: wave 32's `MUTEX-SHARE-CROSS`
wrote, under a heading naming the `STABILITY-REEVAL` cadence, that "there are
no unexercised arms in the card's own rules", and wave 34 measured **four** in
`lower_declared_share`. All four are now dispositioned, none retired — the
residual is explicit that deletion is not a discharge, since all four fail
closed and `FB-SYSTEM-EXTENSIONS` revives the dead pair. Arms **A** and **B**
(a non-Word token at the payload and mutex positions) are **driven**: they need
a header of exactly seven tokens with `under` at index 5, which is why the
wave-32 shape test never reached them — its three bodies are five and six
tokens and all stop at the length arm above. Arms **C** (`Ok(other)`) and **D**
(the payload lookup that misses) are **proved unreachable and kept as
justified backstops**, the `RETURN-POSITION-ESCAPE-UNREACHABLE` treatment: one
theorem that each is dead today, one saying what it would catch if its premise
dropped. `QCore.MutexShareCrossArms` (`AuditMutexShareCrossArms.lean`, Lean
audits **129 → 130**) carries nine theorems, axiom closure `propext` alone.
One of the residual's own claims is **sharpened rather than confirmed**: it
derived arm C's death from the documented "only pair", but the admitting arm
constructs its protocol as a **literal**, so the residue is empty for *every*
kind independently of that property — the two dead arms are therefore not dead
for the same reason, which the residual treated as one fact. Arm D's death is
published as the thin thing it is: the judgment reads the kind's **class** and
nothing else, so the arm is dead by **uniqueness alone**. Seven workspace tests
added; the one source change is behaviour-preserving (the payload lookup
performed once instead of twice, both arms and every message kept, all 16
wave-32 rows passing unmodified). Non-vacuity measured by a mutation harness,
**8 of 8 caught, 0 survivors**, including the pre-card pass-through judgment
restored — under which arm C becomes reachable *and* the card's test goes red.
The harness's own first version was a **measurement defect this card caused and
caught**: restoring with `shutil.copy2` preserves the backup's mtime, so cargo
skipped the rebuild and a whole workspace run reported two CT failures that
were the still-linked mutant and not the tree, with `git status` clean
throughout (`MUTATION-HARNESS-MTIME-RESTORE`; the same mistake in the
discharging direction yields a false green). Four further residuals recorded,
one of them a deliberate **measured boundary** — whether
`TransferKind::Unmodeled` is reachable at this site was not measured, and
postponement is recorded as the outcome it is rather than smoothed over. No CT
count, stage row, pin, archive or keyword moves; `docs/reports/mutex-share-cross/`
is superseded from this card's own record and **not edited** (constraint 21).
Gate at the card's tip: **4,566 workspace tests (4,563 passed, 3 failed, 8
ignored)** — 4,559 plus this card's seven — fmt and clippy clean, both probes
**exit 0**, **130 Lean audits**, **229 Python**. All three red rows are
`qcore-verifier-host`'s and none is this card's: the two
`confined_process_execution` rows are constraint 16's shape, red under the full
suite and **green in isolation**, while
`closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request` was
red **both** ways on this host (`Err(Timeout)` at `standalone_host.rs:212`) —
recorded as seen and not touched.


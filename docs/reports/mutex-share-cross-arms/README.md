# `MUTEX-SHARE-CROSS-ARMS` — four arms, four outcomes

Wave 35. Discharges `STABILITY-REEVAL` #1's residual
`MUTEX-SHARE-CROSS-UNEXERCISED-ARMS`
(`docs/reports/stability-reeval-1/README.md`).

## 1. The finding, and why it is a finding

Card `MUTEX-SHARE-CROSS` (wave 32) wrote, at
`docs/reports/mutex-share-cross/README.md:186` and under a section heading that
names the `STABILITY-REEVAL` cadence:

> Every refusal this card adds is driven by a named test; there are no
> unexercised arms in the card's own rules.

Card `STABILITY-REEVAL-1` (wave 34) measured four refusal arms in
`lower_declared_share` (`crates/qcore-frontend/src/source_mutex.rs`) that the
card's §6 table does not list and that no test drives. Two of the four are not
merely undriven but **unreachable from any program**.

The defect is not the arms. Dead fail-closed backstops are defensible, and this
card keeps all four. The defect is the **sentence**: it converts "the suite is
green" into "the arms are verified", which is precisely the inference the
`STABILITY-REEVAL` cadence exists to refuse. This report supersedes that
sentence from its own record; `docs/reports/mutex-share-cross/` is a frozen
archive and is **not edited** (constraint 21, the `gateway-c-location` pattern).

## 2. The four arms and their outcomes

Line numbers are this card's tree; the residual's are wave 34's.

| Arm | Site (now) | Shape | Outcome |
|---|---|---|---|
| **A** | `source_mutex.rs:1291-1296` | the payload position holds a token that is not a name | **driven** — test + 2 mutations |
| **B** | `source_mutex.rs:1297-1302` | the mutex position holds a token that is not a name | **driven** — test + 2 mutations |
| **C** | `source_mutex.rs:1343-1351` | `Ok(other)`: the judgment admits a *non-mutex* transfer | **proved unreachable, kept as a justified backstop** |
| **D** | `source_mutex.rs:1375-1383` | the payload lookup misses after the judgment admitted | **proved unreachable, kept as a justified backstop** |

No arm is retired. The residual is explicit that deletion is not a discharge —
all four fail closed, and `FB-SYSTEM-EXTENSIONS` revives both dead ones — and
§5 below states, for each, the exact premise whose loss makes it live.

## 3. Arms A and B — driven

Both arms need a header of **exactly seven tokens with `under` at index 5**.
That is why they were undriven rather than merely untested: the wave-32 shape
test `a_malformed_crossing_clause_names_the_form` supplies three bodies of five
and six tokens, and all three stop at the length-and-`under` arm above these
two. An integer literal is a well-formed `TokenKind::Integer` that `name`
refuses, which is the shape these arms exist for:

```
thread scope c share 3 under m { }        -> arm A
thread scope c share buf under 3 { }      -> arm B
```

Tests, in `crates/qcore-frontend/tests/mutex_share_cross_arms.rs`:

* `a_crossing_whose_payload_position_is_not_a_name_names_the_payload_rule`
* `a_crossing_whose_mutex_position_is_not_a_name_names_the_generation_rule`
* `the_wave_32_shape_bodies_stop_above_both_token_arms` — pins the *reason* the
  arms were undriven, so it cannot silently return
* `the_nesting_arm_still_precedes_both_token_arms` — orders the arms against
  each other rather than asserting each in isolation

The four arms sit within a few lines of each other, so each test asserts its
arm's own sentence **and** that every neighbouring arm's sentence is absent. A
test asserting only `Q600` would not tell any of them apart.

## 4. Arms C and D — proved, not asserted

`proof/QCore/MutexShareCrossArms.lean`, registered in
`proof/AuditMutexShareCrossArms.lean` and in `proof/QCore.lean`. Nine theorems;
axiom closure is `propext` alone (`armDLeansOnUniqueness` depends on nothing),
so no project axiom and no placeholder.

**Arm C is unreachable** because for `ShareUnderMutex` the judgment's admitting
arm constructs its protocol as a **literal**
(`crates/qcore-core/src/transfer.rs`), so an admitted capture *is* the value the
preceding `Ok` arm matches, for every kind in the vocabulary —
`theProtocolResidueIsEmpty`, and `theOkOtherArmIsUnreachable` in the shape the
Rust `match` is written in. This is a stronger fact than the residual's route to
it: the residual argued via the documented "only pair", but the literal makes
the residue empty for every kind, not only for the payload.

**Arm D is unreachable** in two halves. The non-local half is
`theLookupAlwaysHits`: no kind but `ProtectedResource` is admitted at all. The
local half is that `place` is `ProtectedResource` only in the branch the
lookup's own `Some` selected — and since this card that is **one binding read
twice, not two lookups** (§6), so it is read rather than argued.

**Non-vacuity.** Both are implications, and a judgment that refused every kind
would prove both while meaning the admitting path was dead too.
`theAdmittingCaseExists` rules that out, and the Rust mirrors assert the
admitting set is exactly `[ProtectedResource]` rather than merely consistent.

## 5. Why the dead arms are kept — the `RETURN-POSITION-ESCAPE-UNREACHABLE` half

That standard asks for two theorems per dead arm: one that it is dead today, and
one saying what it would catch if its premise dropped. Both are supplied,
because both premises are live project work rather than hypotheticals —
`FB-SYSTEM-EXTENSIONS` (lock order) is recorded in wave 34's own table as making
both arms reachable "with no test behind either".

**Arm C.** `thePassThroughJudgmentWouldReachTheProtocolArm` models the judgment
as it stood *before* card `MUTEX-SHARE-CROSS`, quoted from that card's own
comment in `transfer.rs`: "Before this card the arm took whatever protocol the
kind named." Under it an `own<Buffer>` is admitted as a share under
`CountedReadShares`. The load-bearing conjunct is the third:
`receivedKind` reads the transfer's **constructor and not its protocol**, so
`admits_capture`'s delivered-kind gate does *not* intercept that value — it
arrives at the lowering's match with the wrong protocol and the right delivered
kind, and arm C is the only refusal between it and an unprotected crossing.
`theCardsLiteralIsWhatEmptiesTheResidue` confirms the two judgments really
differ.

**Arm D.** `theJudgmentCannotTellTwoKindsOfOneClassApart`: the admitting
decision reads the kind's **class** and nothing else. So arm D is dead by the
*uniqueness* of the payload's class (`armDLeansOnUniqueness`) and by nothing
structural. A second mutex-bearing kind admits exactly as the payload does while
naming no protected resource of the module, and arm D is the refusal that
catches the miss. This is the honest statement of how thin arm D's death is, and
it is the reason the arm stays.

## 6. The one source change, and why it is behaviour-preserving

`lower_declared_share` performed the payload lookup twice — `contains_key` to
classify, `get` to bind. Arm D's local argument was therefore "same map, same
key", which a reader has to reconstruct. The lookup is now performed **once**
and the classification reads its result:

```rust
let declared_resource = state.resource_of.get(resource_name).copied();
let place = if declared_resource.is_some() { TransferKind::ProtectedResource } else …
…
let Some(resource) = declared_resource else { /* arm D, unchanged */ };
```

`contains_key` and `get(..).is_some()` are the same predicate on the same map
and key, so behaviour is unchanged; the arm is **kept**, with its message
byte-identical. The evidence that nothing moved is the wave-32 file:
`crates/qcore-frontend/tests/mutex_share_cross.rs` pins all 16 of that card's
rows including every message, and all 16 pass unmodified. Arms C and D now carry
in-source comments naming the theorems that justify them, so a later reader is
told the arm is a proved backstop instead of inferring it is covered.

## 7. Falsification — the evidence that the tests and theorems are not vacuous

`docs/reports/mutex-share-cross-arms/scripts/mutations.py` negates each arm and
each proved statement and records whether the check notices. Run from the
repository root; it restores every file and verifies the restore.

Measured result: **8 of 8 mutations caught, 0 survivors.**

| Mutation | Negates | Result |
|---|---|---|
| `arm-a-message` | arm A's sentence becomes arm B's | 1 test red |
| `arm-b-message` | arm B's sentence becomes arm A's | 1 test red |
| `arm-ab-positions-swapped` | which header index each arm reads | 3 tests red |
| `arm-c-pass-through` | the judgment's literal becomes a pass-through | 2 tests red |
| `arm-d-second-sharing-kind` | `OwnedBuffer` given the payload's class | 3 tests red |
| `lean-protocol-residue-is-empty` | arm C's theorem names the wrong protocol | build failed |
| `lean-lookup-always-hits` | arm D's theorem names the wrong kind | build failed |
| `lean-class-form-is-the-judgment` | `admitsOfClass`' mutex arm refuses | build failed |

`arm-c-pass-through` is worth naming twice: it is not an invented defect but the
**pre-card judgment restored**, and under it arm C becomes reachable *and* the
card's test goes red. That is the backstop earning its place.

### 7.1 A measurement-hygiene defect this card caused, caught and fixed

The harness's first version restored each mutated file with `shutil.copy2`,
which **preserves the backup's mtime**. A restored file is therefore *older*
than the fingerprint cargo recorded while building the mutant, so cargo
considers the crate up to date and **skips the rebuild** — every later `cargo`
invocation in that worktree silently links the mutated crate.

This was not theoretical. The first full workspace run after the harness
reported two failures — `ct03_proof_leg`'s
`ct03_artifact_is_the_lean_proved_shared_reads_shape` (`Q500`, "the derived
transferability judgment and the capture enumeration disagree about this
capture") and `ct_corpus`'s stage-agreement row — which were the
`arm-d-second-sharing-kind` mutation still linked into `qcore-core`, not a
defect in the tree. The working tree was pristine against `HEAD` throughout,
which is exactly what makes the failure mode dangerous: `git status` cannot see
it.

The harness now restores by **rewriting** the file and asserts the content
round-trips; the mutation run was repeated (8 of 8 caught again) and both
previously failing tests were re-run green before the gate run below. Recorded
as **`MUTATION-HARNESS-MTIME-RESTORE`** (§9), with a proposed `CLAUDE.md`
amendment in §11 — the hazard belongs beside that file's existing
measurement-hygiene rules, since the same mistake in the *discharging*
direction would have produced a false **green**.

## 8. What this card does not claim

* **No claim about the mutex protocol's soundness.** That is
  `QCore.MutexShareCross`, unedited here.
* **No emission or native evidence.** `MUTEX-SHARE-CROSS-EMISSION` (CT46's
  emission) is out of scope and untouched; no CT count moves, no stage row
  moves, and mapping is not conformance (constraint 23).
* **No new keyword, token or diagnostic code.** Constraint 14 does not engage —
  nothing is reserved and the lexer is not moved. `Q600`'s condition is not
  widened: every refusal here already existed at that code.
* **Arms C and D remain unexercised.** They are *justified*, not *driven*. The
  suite still cannot tell either from its negation, which is why the
  justification is a theorem and why §9 carries the residuals.

## 9. Named follow-ons

* **`MUTEX-SHARE-CROSS-ARM-C-REVIVAL`** — arm C is dead only while the mutex
  mode's admitting arm constructs its protocol as a literal. A second
  mutex-family protocol (an rwlock, a recursive mutex; constraint 27 records
  these as represented refusals today) turns that literal into a variable and
  makes the arm live. *Discharge:* the card that adds a second mutex-family
  protocol drives arm C with a test in the same change, or records here why it
  cannot.
* **`MUTEX-SHARE-CROSS-ARM-D-REVIVAL`** — arm D is dead by the uniqueness of the
  payload's class alone (`armDLeansOnUniqueness`). `FB-SYSTEM-EXTENSIONS`' lock
  order needs nesting and a second mutex-bearing kind, and the second kind alone
  revives the arm. *Discharge:* the card that adds a second kind sharing under
  the checked mutex drives arm D in the same change. Until then
  `only_the_protected_payload_reaches_the_lookup` and
  `exactly_one_kind_carries_the_payloads_class` are the tripwires: they go red
  on the day the premise drops, which is the earliest a test can speak here.
* **`MUTEX-SHARE-CROSS-UNMODELED-PLACE-UNMEASURED`** — this card measured the
  four arms the residual named and did **not** measure whether
  `TransferKind::Unmodeled` is reachable at this site. A name bound to neither a
  protected resource nor a generation classifies as `Unmodeled`, but an
  *undeclared* name refuses earlier at `Q013` (measured: `share nosuch under m`
  reports `Q013 unknown name`), so the arm's reachability turns on whether this
  route admits a binding of any third sort. The judgment-refusal arm is driven
  today for `MutexGeneration` only (`share m under m`). *Discharge:* measure it,
  and drive or record it as this card did for A–D. Recorded as a **measured
  boundary**, outcome 3 of the card's menu — postponement is a valid outcome and
  is recorded as one.
* **`MUTATION-HARNESS-MTIME-RESTORE`** — a mutation harness that restores a
  source file with `shutil.copy2` (or any mtime-preserving copy) leaves cargo's
  fingerprint newer than the restored file, so the **mutant stays linked** into
  every later build in that worktree while `git status` reads clean (§7.1, where
  it cost two false CT failures). Fixed in this card's own harness. *Discharge:*
  the hazard is generic to every mutation harness in the tree —
  `docs/reports/c-gateway-return-call-w4/scripts/mutations.py` is the other one
  and should be read for the same pattern — and the rule belongs in `CLAUDE.md`'s
  measurement-hygiene section, proposed in §11.
* **`PYTHON-AUDIT-COMMAND-ROW-LOAD-SENSITIVE`** — `test_check_lean_audits.py`'s
  `test_every_audit_runs_once_with_the_sequential_command` was observed **red
  once** under `unittest discover` while a full `cargo test --workspace` was
  compiling on the same host, and **green** in isolation, green in a paired run
  with the matrix tests, and green on a repeat `discover` without that load
  (both facts recorded, per the constraint-16 discipline). It is not this card's
  code: the test builds its own `TemporaryDirectory` fixture and touches nothing
  this card changed. Likely mechanism, stated as a hypothesis and not a
  measurement: `check_audits` calls the patched `run_step` from a four-worker
  `ThreadPoolExecutor`, and `unittest.mock`'s `call_args_list` is not
  thread-safe, so concurrent recording can corrupt the list the assertion reads.
  *Discharge:* reproduce under deliberate load, and if confirmed either record
  the row in constraint 16's load-sensitive inventory or make the assertion
  thread-safe. **Not** to be "fixed" by lowering `jobs`.

## 10. Gate

The full standing gate from `AGENTS.md`, run at this card's tip. Logs are
worktree-private under `measure/` per `CLAUDE.md`'s hygiene rule; exits are
reported as measured.

| Step | Result |
|---|---|
| `git submodule update --init --recursive` | exit 0 (libevent at `d82464a2`) |
| `cargo test --workspace --no-fail-fast` | **4,566 total: 4,563 passed, 3 failed, 8 ignored** — see below |
| `cargo fmt --all -- --check` | exit 0, clean |
| `cargo clippy --all-targets -- -D warnings` | exit 0, clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | **exit 0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | **exit 0** |
| `python3 scripts/check_lean_audits.py` | exit 0, **130 of 130** |
| `python3 -m unittest discover` (`scripts/tests`) | exit 0, **229 tests** |

**4,566 is 4,559 plus this card's seven**, which is the whole of the movement.
Lean audits are **129 → 130**, this card's one new audit.

**The three red rows are all `qcore-verifier-host`, and none is this card's**
— nothing here touches that crate, and each was re-verified in isolation, both
facts recorded per constraint 16's discipline:

* `confined_process_execution::an_admitted_path_still_works_under_confinement`
  and `::the_package_root_is_admitted_read_and_execute_only` — red under the
  full suite, **green in isolation** (12 of 12, exit 0). The
  constraint-16 shape exactly.
* `standalone_host::closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request`
  — the row the brief names as a known one. It was **red both ways here**: red
  under the full suite *and* red in isolation (`Err(Timeout)` where the
  assertion wants `Ok(VerificationResponse{…})`, a wall-clock row at
  `standalone_host.rs:212`). The brief describes it as red in isolation and
  green under a full suite; on this host it did not go green under load either.
  Reported, **not touched**.

No pinned expectation was edited, no fail-closed gate weakened, no diagnostic
downgraded, no frozen archive modified.

## 11. For the integrator

1. **The headline numbers move by this card's own additions only.** Seven
   workspace tests and one Lean audit are added; no existing test, count, pin or
   archive is edited. The ledger's Lean audit row goes **129 → 130**.
2. **`docs/reports/mutex-share-cross/README.md:186` is superseded from this
   record.** The frozen archive is not edited (constraint 21). If the ledger or
   any index quotes that sentence, it should point here instead. The correct
   replacement sentence is: *four arms of this card's rules are not driven by
   any test; two are driven as of wave 35, and two are unreachable and kept as
   backstops justified by `QCore.MutexShareCrossArms`.*
3. **Residual bookkeeping.** `MUTEX-SHARE-CROSS-UNEXERCISED-ARMS` is discharged
   and is replaced in `named_followups` by a successor entry recording the
   discharge and what remains open — the `TRACE-ALPHABET-LEASE-BOUNDARY`
   pattern, where the successor carries the story rather than the original being
   silently deleted. Three new residuals are in §9; all three reach the tracker.
4. **One of the residual's own claims was sharpened, not merely confirmed.** The
   residual derives arm C's death from the documented "only pair". The measured
   fact is stronger: the admitting arm's protocol is a *literal*, so the residue
   is empty for **every** kind, independent of the only-pair property. Arm D, by
   contrast, depends on the only-pair property exactly as the residual said, and
   §5 says so in those words. The two dead arms are therefore **not** dead for
   the same reason, which the residual treated as one fact.
5. **A seam, noted and not taken.** Constraint 15n and
   `TRACE-ALPHABET-LEASE-GROWTH-UNDECIDED` concern the F02 alphabet's growth for
   write leases. They are adjacent — both are about a boundary proved on one
   side and observed on the other — and this card touches neither.
6. **Proposed amendment text for standing constraint 14's closing tally.** Wave
   35's entry for this card belongs in the "Cards that do not engage this
   constraint at all" list: it reserves no word and does not move the lexer.
7. **Proposed amendment to `CLAUDE.md`'s "Worktree and measurement hygiene"**,
   in these words, as a new bullet after the measurement-outputs rule:

   > - **A mutation harness restores by rewriting, never by an mtime-preserving
   >   copy.** `shutil.copy2` restores the backup's timestamp, which leaves
   >   cargo's fingerprint from the mutant build *newer* than the restored
   >   source: cargo skips the rebuild and every later run in that worktree
   >   links the mutated crate while `git status` reads clean. Measured in wave
   >   35 (`MUTATION-HARNESS-MTIME-RESTORE`), where it produced two false CT
   >   failures; the same mistake in the discharging direction produces a false
   >   **green**, which is why this is a rule and not a tip. Restore by writing
   >   the original bytes back, and assert the round-trip.

   Landing it takes a second reader on
   `docs/reports/c-gateway-return-call-w4/scripts/mutations.py`, which this card
   did not audit and which is the other harness in the tree.
8. **Two residuals in §9 are about the project's measurement apparatus, not
   about this card's subject** (`MUTATION-HARNESS-MTIME-RESTORE`,
   `PYTHON-AUDIT-COMMAND-ROW-LOAD-SENSITIVE`). They are recorded here because
   this card is where they were measured, and both reach the tracker; neither is
   claimed to be repaired beyond this card's own harness.

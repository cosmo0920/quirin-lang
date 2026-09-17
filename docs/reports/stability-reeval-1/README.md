# STABILITY-REEVAL #1 — under what conditions each current claim is unstable

Card `STABILITY-REEVAL-1`, wave 34. First firing of the recurring cadence
`CLAUDE.md` established by user direction on 2026-09-13 ("first due after wave
33"). Base: `d3225f91`, the wave-33 close. Scope: **waves 32 and 33**.

This is an audit. It weakens no gate, edits no frozen archive, changes no
verdict rule, and promotes nothing. It adds two Python tests and no Rust or
Lean. Every finding lands as a **named residual with a discharge condition**;
§8 records what is postponed, which `CLAUDE.md` makes a valid outcome.

**The headline is not reassurance.** In the two waves that were told to apply
this discipline, the method found: a **live deterministic test failure standing
behind a published "0 failed"**; a card **asserting it has no unexercised arms
while carrying two provably dead ones**; `AGENTS.md` **contradicting itself on
the cadence's own occasioning case**; an `AGENTS.md` sentence **overstating a
mutation result 5/5 where the measurement is 3/5**; and an **uncarded commit in
a seam that is structurally invisible to the sweep**.

It also found the ledger's *numbers* to be in good order. Both results are
reported, because a findings-only report misrepresents the tree as surely as a
reassuring one.

---

## 1. What this card measured, and what it did not

| | |
|---|---|
| **Measured by running a command** | the contested host test (5 runs, 2 configurations); 227→229 Python; 128 Lean audits; both probes; the residual sweep; sha256 of five CT fixtures, of the matrix, and of the restored matrix after two mutations; the two added tests and both their mutations |
| **Measured by reading source and callers** | the two dead `source_mutex.rs` arms, against `transfer.rs`'s judgment; constraint 5b's Lean hypothesis; the sweep tool's path filter; the wave-32 vs wave-33 close directory contents; the `216`/`227` split in `validation.json`; the fused paragraph seam; the dangling theorem citations |
| **Delegated, then spot-checked by me** | the ledger-to-source reconciliation; the proved-vs-observed survey; the two mutation campaigns of §2.3; the uncarded-commit sweep. I independently re-verified the CT fixture hashes, `TrackedBasesAreBound`, the gate row, the followup counts, the `216`/`227` question and both structural defects in §4.5 |
| **Inferred, and labelled in place** | the "no test can ever distinguish" judgment in §2.3 rests on reading all six `state.pointers` insertion sites, not on a fixture |
| **Not attempted** | Windows and macOS arms (no such host); re-running the corpus arbiter myself; exhaustive semantic review of Lean statement *meanings* (§8) |

**A correction this card owes the record.** The briefing context supplied a
wave-32 `AGENTS.md` from a different worktree. Four "suspected drifts" in the
brief — 146 Safe, `unsupported` 50,150, 40 followups, a wave-32 gate citation —
are **false against this tree**; the wave-33 close updated all four correctly. A
card acting on the brief without checking would have "repaired" four numbers
that were already right. Recorded because this cadence's whole subject is
believing text over trees.

---

## 2. Step 1 — hunt vacuous greenness

### 2.1 `VERIFIER-HOST-LARGE-RESPONSE-CONTROL-RED` — a red test behind a published "0 failed"

**The most consequential finding on this card**, because it is not about
wording: a published gate figure does not reproduce.

Measured by me, on this tree at `d3225f91`, before this card touched anything:

```
cargo test -p qcore-verifier-host --test standalone_host closed_input_does_not_mask
  run1: FAILED. 0 passed; 1 failed   (3.50s)
  run2: FAILED. 0 passed; 1 failed   (3.48s)
  run3: FAILED. 0 passed; 1 failed   (3.46s)
```

Deterministic, **3/3, in isolation** — one test, no sibling load — so constraint
16's load-sensitivity explanation does not apply, and this test is not one of
constraint 16's five rows. The assertion is
`crates/qcore-verifier-host/tests/standalone_host.rs:212`,
`assert_eq!(provider.verify(&request), Ok(response))`, returning `Err(Timeout)`.
That assertion is **the admitting control the wave-33 repair card itself
added**.

The repair is present, not missing: `4aa24e28` is an ancestor of `HEAD`
(`git merge-base --is-ancestor`), and its progress-gated backoff stands at
`crates/qcore-verifier-host/src/lib.rs:1705,1757-1758`.

I separated "too slow for the budget" from "hangs", by mutation: raising
`manifest.limits.timeout_ms` from `1000` to `8000` at `standalone_host.rs:204`
makes the test **pass** (`1 passed`, 4.44s). The mutation was reverted and the
tree verified clean. The control is a **wall-clock throughput race** — about
8 MiB in and 8 MiB out through a pipe inside 1,000 ms — whose outcome is a
property of the host, not of the host code.

Independently corroborated: `docs/plans/fluent-bit-system-coverage/README.md:244-254`
records the same test, same assertion, same `Err(Timeout)`, with an isolated
rerun also exiting 101, stating it is "not classified as a flake". That record
predates the close.

Against that, `AGENTS.md`'s gate row publishes **4,544 passed / 0 failed** and
calls this defect "**repaired** rather than carried", and
`docs/reports/wave33-close/gate.json` says "all 227 binaries green".

**The finding is not that the repair is wrong.** It removed a real 1,024-sleep
defect and the code is sound — and `CARD-SCOPE-SELF-MERGE`'s "the assertion was
not weakened" claim was separately verified true (the original
`assert!(matches!(result, Err(ProviderFailure::Io(_))))` is retained against a
fresh provider, all three deadlines stand, and no refusal arm in `lib.rs` was
touched). The finding is that **"repaired" is an observation on one machine
presented as a property**, and that nothing in the tree records an environment
boundary for it.

> **Residual `VERIFIER-HOST-LARGE-RESPONSE-CONTROL-RED`.** The wave-33 repair
> card's own admitting control fails deterministically (3/3 in isolation) at
> `standalone_host.rs:212` on this host at the wave-33 close tip, while the
> ledger publishes 0 failed. Measured as a throughput shortfall, not a hang: an
> 8,000 ms deadline passes.
> **Discharge** — one of: (a) record the row beside constraint 16's five with
> its measured host condition, which `docs/reports/c-gateway-return-position/README.md`
> already requested in these words ("if so, record the row beside constraint
> 16's five with its measured load/host condition"); (b) make the control's
> budget a function of measured pipe throughput rather than a fixed 1,000 ms;
> (c) an evidenced host investigation establishing the cause.
> **Not a discharge:** shrinking the response, raising the deadline, or deleting
> the control (invariant 1).

### 2.2 `MUTEX-SHARE-CROSS-UNEXERCISED-ARMS` — the discipline asserted, not applied

`docs/reports/mutex-share-cross/README.md:186`, under a section headed "the
`STABILITY-REEVAL` discipline", states:

> "**Every refusal this card adds is driven by a named test**; there are no
> unexercised arms in the card's own rules."

Its §6 table lists ten rules. `lower_declared_share` in
`crates/qcore-frontend/src/source_mutex.rs` carries **four further refusal arms
the table does not mention**, none driven by any test. Two I verified
**unreachable from any program**:

* **`source_mutex.rs:1325-1336`**, the `Ok(other)` arm after `admits_capture`.
  Verified at `crates/qcore-core/src/transfer.rs:634-635`: for
  `TransferMode::ShareUnderMutex` the only `Ok` the judgment returns is
  `AdmittedTransfer::ShareUnderMutex(ShareProtocol::CheckedMutex)`, which the
  **preceding** arm matches. `transfer.rs:762` states `(ProtectedResource,
  ShareUnderMutex)` "is the **only** pair anywhere in the matrix that yields the
  mutex protocol". `Ok(other)` is unconstructible.
* **`source_mutex.rs:1348-1356`**, the `let Some(&resource) = state.resource_of.get(...) else`
  arm. Verified at `source_mutex.rs:1313-1319`: `place` is
  `TransferKind::ProtectedResource` **iff** `state.resource_of.contains_key(resource_name)`,
  and only `ProtectedResource` reaches here. Same map, same key — the `else`
  cannot be taken.

The other two (`:1292-1296`, `:1298-1302`) are reachable in principle but
**undriven**: the three shapes in `a_malformed_crossing_clause_names_the_form`
are all length-6 and stop at the length arm, and each refusal message string
occurs exactly once in the tree — its own definition.

Two dead fail-closed backstops are defensible. The sentence asserting they do
not exist is not. Compare `RETURN-POSITION-ESCAPE-UNREACHABLE`, landed one wave
later, which is named, measured by mutation, and tells the reader "do not read
the green suite as evidence for it; read the theorem" — the correct treatment of
exactly this situation.

> **Residual `MUTEX-SHARE-CROSS-UNEXERCISED-ARMS`. Discharge:** amend the card's
> §6 to list the four arms with their status, giving the two dead ones the
> `RETURN-POSITION-ESCAPE-UNREACHABLE` treatment; **or** drive the two reachable
> token arms with a test. **Not** a discharge: deleting the arms — they fail
> closed, and §5 names the card that makes them live.

### 2.3 Two named residuals re-measured — one accurate and terminal, one discharged with its ledger wrong

**`RETURN-POSITION-ESCAPE-UNREACHABLE`: accurate, and stronger than recorded.**
The arm is `crates/qcore-c-gateway/src/legacy_c.rs:3374-3386`; the mutation is
`returned_place: canonical_place(&texts)`, which compiles the node to
`EscapeIfOwned` instead of `NoOp`. Under that mutation: `qcore-c-gateway`
**985 passed / 0 failed** across 41 binaries; twelve further compiler binaries
**63 passed / 0 failed**; both probes exit 0; and the **full pinned 185-unit
corpus audit** reproduces the card's archived AFTER artifacts — `diagnostics.json`,
`monkey-source.json` and `chunk-contracts.json` all **byte-identical**. The card
measured vacuity over 3 binaries and 52 tests; this is 1,048 tests plus the
corpus. The arm **does execute** on real input during that run (the 33 withdrawn
return-position leaves are exactly `untracked_pointer` cases) and moves nothing.

The judgment is **(ii): a genuine semantic consequence, not missing coverage.**
Both consumers of `EscapeIfOwned` are guarded on `state.pointers` membership
(`legacy_c.rs:6757`, and `observe_exit` at `5827`), and the `NoOp` node falls to
the same `ReturnedValue::None` at `5844` — so when the key is absent the two
answers are *literally the same value*. Every `state.pointers` key is an
instruction destination or an entry-seeded plain formal name; chain-shaped keys
arise only via the aggregate-store route, which requires a base that `may_bind`
reports bound, while F4 admission requires `!may_bind(base)`. So
`canonical_place`'s string can never be a key. *(This last step is a reading of
all six insertion sites, not a fixture — labelled as inference.)*

> **Residual `RETURN-POSITION-ESCAPE-UNREACHABLE` — narrowed to terminal.** The
> residual's first listed discharge, "a fixture that makes the projection track
> a place an admitted chain can name", is **not available**: it requires a
> source change weakening `admitted_chain`, not a fixture. The only available
> discharge is "state it as a measured boundary", which the card already did.
> **Discharge:** mark it a closed measured boundary rather than pending
> follow-on work, so the tracker stops implying work that cannot be done. **But
> see §5** — a queued card removes the premise that makes it terminal.

**`TRACE-ALPHABET-LEASE-REFUSAL-UNEXERCISED`: substantively discharged — the arm
is genuinely driven.** `compiler/tests/trace_alphabet_lease_boundary.rs` (356
lines, 5 tests) passes, and
`a_real_lease_program_reaches_the_refusal_at_the_offending_events_own_index`
compiles `WRITE-LEND-TASKS`' flagship through `compile_files`, executes it,
locates the `TaskEvent::BufferLeased` **by shape**, and pins
`Err(UnsupportedObservedEvent { index: 2 })`. Wave 32's discharge claim is
**correct**. Two defects sit around it:

> **Residual `LEASE-BOUNDARY-MUTATION-COUNT-OVERSTATED`.** `AGENTS.md`
> constraint 15n says "every one of its **five** tests was shown non-vacuous by
> mutating the arm". Re-derived against
> `crates/qcore-core/src/task_trace_relation.rs:548-550`, the three mutations
> the card names turn **3 of 5** red (silent-drop 3/5, admitting-mis-encode 3/5,
> wrong-index 2/5; union **3**). The two never-red tests are *structurally*
> incapable of falsification by an arm mutation — one constructs the dropped
> encoding itself, the other exercises `parse_observed_trace`, a different
> route. The card's own report is honest (it publishes the three counts); the
> ledger's paraphrase inflates them to 5/5.
> **Discharge:** correct 15n to "three of its five tests, the other two
> exercising neighbouring routes by construction".

> **Residual `LEASE-BOUNDARY-MUTATION-EVIDENCE-UNSHIPPED`.**
> `docs/reports/trace-alphabet-lease-boundary/` contains **only `README.md`** —
> no `logs/`, no `scripts/`, no transcript — and its §3.1 records counts without
> per-test names. The counts *do* reproduce, but only because this card
> re-derived all three mutations by hand. The sibling card
> `c-gateway-return-position` ships `logs/mutations.txt` and `scripts/mutations.sh`
> naming every red test. **Discharge:** ship the mutation script and log, as the
> sibling card does. This is the mechanism that makes a non-vacuity claim
> checkable after the merge (§8.2).

### 2.4 Arms checked and found adequately driven — the measured negative

The transferability judgment's new `ProtocolUnavailable` arms are covered by
`judgment_is_total_over_both_vocabularies`, which executes all 16×6 pairs.
`AuditAll.lean`'s self-equality and exact-assumed-conclusion detectors carry
committed admitting and refusing controls. Two that are not:

> **Residual `AUDITALL-TRIPWIRE-CONTROLS-UNCOMMITTED`.** `AuditAll.lean`'s
> `statements == 0` / `checked == 0` tripwires and `lakefile.toml`'s
> `warningAsError = true` were validated by **manual, uncommitted injections**.
> Nothing in the tree re-runs them, so a refactor silently zeroing the audit
> count would pass. **Discharge:** commit the injections as controls, the way
> the statement detector's own `run_elab` control is committed.

> **Residual `CT-COMPOSED-NATIVE-NON-LINUX-VACUOUS-PASS`.** `run_composed_native`
> (`compiler/tests/ct_corpus.rs`) early-returns on non-Linux/macOS, so the runner
> **passes green on Windows while asserting nothing** about native evidence.
> Documented (`evidence_target=linux`) and not a defect — but a green suite on
> Windows is not evidence for the CT22/CT25 sanitized rows. **Discharge:** state
> it as a measured platform boundary beside "sanitized-native 9 of 45", which
> carries no platform qualifier today.

---

## 3. Step 2 — the proved side and the observed side

The pattern is **one-directional and consistent**: the card reports and the Lean
docstrings are careful; the disclosure **does not survive the trip into
`AGENTS.md`**, the one file the sync rule makes authoritative for agents.

### 3.1 `CT-SHARED-FIXTURE-DISCLOSURE` — measured, correct, reaching no tracker

Verified by `sha256sum`:

```
131f635d9579a7da9833a7e51f305a5aa3306b2542808c17558ced982f5e909c  CT01/case.qcore
131f635d9579a7da9833a7e51f305a5aa3306b2542808c17558ced982f5e909c  CT12/case.qcore
131f635d9579a7da9833a7e51f305a5aa3306b2542808c17558ced982f5e909c  CT14/case.qcore
7c75577b59733994bac462404ee51173d4376336b5351d1d9e9b6a380d634be6  CT03/case.qcore
890d275e9a42ecf782bec4bf053ae0ae60d516cfb53ff5b1fa87c852805875b2  CT37/case.qcore
```

**CT01, CT12 and CT14 are one byte-identical file.** Across the corpus, 45
mapped cases rest on **43 distinct artifacts** (40 `.qcore` programs + 3
`legacy_c` C twins).

The **proof numerator is honest** — CT01, CT03 and CT37 are pairwise distinct —
and wave 32's `PROOF-LEG-BATCH` behaved correctly, measuring this and
**withdrawing** two proof rows rather than banking them. The defect is that the
name reaches no tracker, and that `ct_manifest.json` records three distinct
`source_fixture` **paths**, so path-identity stands in for program-identity.

> **Residual `CT-SHARED-FIXTURE-DISCLOSURE`** (adopting `PROOF-LEG-BATCH`'s name
> and registering it). **Discharge:** one clause in the CT ledger row giving the
> artifact-vs-case count, **or** a content digest beside `source_fixture` so the
> manifest cannot present one program as three.

### 3.2 `GATEWAY-LEAN-ABSTRACT-NOT-EXTRACTION`

`docs/reports/c-gateway-wave/README.md` states it plainly: the gateway's Lean
modules are "abstract rule theorems, not extraction of the Rust analyzer or
whole-C correctness". Every gateway-backing module repeats the disclaimer in its
own docstring, and **no** extraction, refinement or simulation theorem exists
anywhere in `proof/`.

`grep -niE "not extraction|abstract (rule|model)|faithful extraction" AGENTS.md CLAUDE.md`
returns **nothing**. Sentences a careful reader may take as the stronger claim:
the repository map's "`proof/` | Lean 4 executable core and **safety theorems**";
invariant 6's "**Lean parity**", naming a correspondence nowhere proved; and
constraint 4's "**its soundness** is an induction on execution prefixes", where
"its" reads as `legacy_neutral.rs`'s.

Most tellingly, constraint **28** — headed "Trust boundaries that are true
statements, not TODOs" — enumerates the premises and **omits the two largest in
the gateway**: faithful C token/type extraction, and the Lean-model-to-Rust
correspondence.

> **Residual `GATEWAY-LEAN-ABSTRACT-NOT-EXTRACTION`. Discharge:** add both
> premises to constraint 28 and qualify the repository-map row. Documentation
> only; moves no count and no verdict.

### 3.3 `RETURN-POSITION-SOUNDNESS-PREMISE-ELIDED` — the sharpest instance

Constraint 5b says it is "sound to answer `None` because … `may_bind` reports
bound by construction — **proved as**
`QCore.ReturnPositionEscape.an_admitted_chain_names_no_tracked_place`."

I read the theorem (`proof/QCore/ReturnPositionEscape.lean:84-93`):

```lean
theorem an_admitted_chain_names_no_tracked_place
    (tracked : Chain → Bool) (inv : TrackedBasesAreBound tracked)
    (c : Chain) (adm : admitsChain c = true) : tracked c = false
```

`TrackedBasesAreBound` is an **explicit hypothesis**, and the module says so at
line 34: "A premise, stated and not proved here". The constraint joins premise
and theorem with "because … proved as", so the `may_bind` half — the
load-bearing half — reads as covered by the proof. **The Lean file is more
honest than the ledger.**

> **Residual `RETURN-POSITION-SOUNDNESS-PREMISE-ELIDED`. Discharge:** amend 5b
> to say the theorem is *conditional on* `TrackedBasesAreBound`, in the module's
> own words. §5 shows a queued card makes this premise load-bearing.

### 3.4 `LEDGER-CT-ROW-LEASE-ARM-STALE` — `AGENTS.md` contradicting itself

`AGENTS.md` **line 131** (CT ledger row) still says of the lease arm:

> "is **observed and not proved**: no test drives a lease-carrying reference run
> into it … (`TRACE-ALPHABET-LEASE-REFUSAL-UNEXERCISED`, the occasioning case
> for `CLAUDE.md`'s `STABILITY-REEVAL` cadence)"

**Constraint 15n, in the same file**, says the residual was discharged, "the arm
is reachable today", and the test "drives it". §2.3 measured the arm genuinely
driven, so line 131 is not merely stale but **measurably false**. The matrix is
correct (no live entry for the old id; its successor records the discharge), so
this is a stale *ledger sentence*. Its cause is measured in §4.1.

That the cadence's **occasioning case** is still mis-stated in the ledger, two
closes later, is the clearest single argument that this cadence is load-bearing.

> **Residual `LEDGER-CT-ROW-LEASE-ARM-STALE`. Discharge:** replace the clause
> with 15n's measured statement. Integrator work — this card does not edit
> `AGENTS.md` prose it did not author.

### 3.5 `LEASE-BOUNDARY-TRANSCRIPTION-PREMISE`, and a claim-word sweep that came back clean

All five theorems 15n names exist in `proof/QCore/TaskTraceLeaseBoundary.lean`.
But "the Rust pin `index: 2` is an instance rather than a coincidence" is proved
of `flagshipRun`, a **hand-transcribed Lean list**; the module discloses the
transcription premise, the constraint does not.

A sweep of all 18 theorem-shaped identifiers in `AGENTS.md` and `CLAUDE.md`
resolved **every one** to a real theorem in `proof/`. **No named theorem is
missing** — a genuine positive result. The one unattached claim word is
`AGENTS.md`'s "a **conservative** checker for existing C source", which carries
no theorem and no pointer.

> **Residual `LEASE-BOUNDARY-TRANSCRIPTION-PREMISE`. Discharge:** carry the
> module's transcription sentence into 15n, or bind the Lean term to the Rust
> pin by a generated digest.
> **Residual `GATEWAY-CONSERVATIVE-UNATTACHED`. Discharge:** point the phrase at
> the rule-level theorems with the abstract-model qualifier, or restate it as a
> design intent.

---

## 4. Step 3 — semantic edits made without a card

### 4.1 `WAVE-32-CLOSE-UNSWEPT` — and the measured cause of §3.4

Measured by `ls`: `docs/reports/wave32-close/` contains **`gate.json` only** — no
`README.md`, no `reconciliation.json`. `docs/reports/wave33-close/` contains all
three.

Wave 32 is **the wave that landed the rule** (`W32-RECONCILE-SWEEP`, after which
`CLAUDE.md` reads "The close **writes** `docs/reports/wave<N>-close/reconciliation.json`").
The mechanism was instituted and skipped in the same wave that instituted it.

Its sweep record covers **20 cards of waves 29–31**. Wave 32's own seven merged
cards are `W32-RECONCILE-SWEEP`, `CT-WIRE-LANE-4`, `MUTEX-SHARE-CROSS`,
`TRACE-ALPHABET-LEASE-BOUNDARY`, `PROOF-LEG-BATCH`, `CT-COMPOSED-NATIVE` and
`DOCS-COMPACT-5`. Only `docs-compact-5` appears in the swept list: **six of
seven were never swept.**

This is the measured cause of §3.4. `TRACE-ALPHABET-LEASE-BOUNDARY`'s report
§5.1 is headed, verbatim, "**`TRACE-ALPHABET-LEASE-REFUSAL-UNEXERCISED` — fully
discharged, retire it**", addressed "For the integrator, in terms usable against
`named_followups`". The matrix half was applied; the ledger half was not; nothing
checked. `MUTEX-SHARE-CROSS` (§2.2) is a second unswept card from the same set,
and `TRACE-ALPHABET-LEASE-BOUNDARY` is also the source of §2.3's two defects.
**Three of this card's findings trace to one skipped step.**

> **Residual `WAVE-32-CLOSE-UNSWEPT`. Discharge:** sweep wave 32's six unswept
> cards retrospectively and write `docs/reports/wave32-close/reconciliation.json`,
> as `W32-RECONCILE-SWEEP` did for waves 29–31. The tool decides nothing; a
> person must measure each item against the tree.

### 4.2 `SWEEP-RESIDUALS-SCANS-REPORTS-ONLY` and `WAVE-BASE-UNSWEPT` — two structural blind spots

I ran the project's own tool:

```
python3 scripts/reconciliation_sweep.py . --since 37bc9004 --residuals
→ 5 residual-shaped names in no tracker:
    LEAN-STATEMENT-SEMANTIC-REVIEW, CT10-REASON-ROW, DOCS-COMPACT-6,
    MATRIX-UPDATED-STALE-AT-WAVE-27, SYNC-RULE-TOOLING-COMMIT-SCOPE
```

It missed two more I found by grep: `CT-SHARED-FIXTURE-DISCLOSURE` and
`FB-PLAN-GATE-HOST-TIMEOUT`. The second is **structural**:
`scripts/reconciliation_sweep.py:288` discovers cards with
`git diff --name-only ref HEAD -- docs/reports` and filters
`line.startswith("docs/reports/")`. A residual introduced anywhere else —
`docs/plans/`, `docs/development/increments/`, `docs/cycles/`, a source comment —
is **invisible to `--residuals` by construction**. `FB-PLAN-GATE-HOST-TIMEOUT`
lives in `docs/plans/`, and it is the name attached to §2.1's live failure. The
wave-33 close ran the tool, got an answer, and the wave's most consequential
residual was not in it.

The second blind spot is the **range**: the sweep's `--since` base is exclusive,
so a commit that *is* the base is invisible to it. `37bc9004` (§4.3) is exactly
that commit, and it also falls after the wave-32 close, so no increment record
describes it either. It is in a seam both mechanisms skip, and the seam recurs
every wave.

A **third** blind spot, found while registering this card's own residuals and
the sharpest of the three: `scripts/reconciliation_sweep.py:248` defines
`TRACKERS` as the matrix, `AGENTS.md` **and** `CLAUDE.md`, so a residual named
anywhere in `AGENTS.md` — including inside a standing-constraint paragraph — is
counted as tracked. The consequence, measured: **`RETURN-POSITION-ESCAPE-UNREACHABLE`
and `RETURN-POSITION-ACCUSING-UNEXERCISED-IN-CORPUS` — the two vacuity residuals
wave 33 found by mutation and deliberately reported rather than smoothed over,
and the two this card's brief calls its strongest leads — had *no entry in
`named_followups` at all*.** They are named in constraint 5b, so the sweep was
silent about both and neither appeared in any open-work list. A constraint
paragraph states a **rule that must survive refactors**; the follow-on ledger
states **work that is open**. A name can be in the first and untracked as the
second — and this is the clearest possible case, because the wave that produced
them did everything else right.

> **Residual `SWEEP-RESIDUALS-SCANS-REPORTS-ONLY`. Discharge:** widen
> `--residuals`' scan to the other documentation roots, keeping the tool's
> "candidates for a human eye, never a verdict" contract. Bounded; does not touch
> `--check`.
> **Residual `WAVE-BASE-UNSWEPT`. Discharge:** make the close's sweep range
> inclusive of its base commit, or require the base to be the previous close's
> merge so nothing can land in the seam.
> **Residual `RESIDUAL-NAMED-ONLY-IN-A-CONSTRAINT-IS-UNTRACKED-WORK`. Discharge:**
> narrow `TRACKERS` so an `AGENTS.md` mention alone does not count as tracking a
> residual, **or** make the close require that every residual a card's report
> names reaches `named_followups` specifically. Both wave-33 residuals are now
> registered there, with this card's re-measurements.

### 4.3 `WINDOWS-COHERENCE-GATES-UNCARDED` — an uncarded commit, with its digest move exonerated

Commit `37bc9004` "Fix Windows coherence gates" appears in **no** increment
record, has **no** `docs/reports/<card>/`, and is in **no** reconciliation
record. Two of its three parts matter:

1. It moves two pinned F02 emission digests in `compiler/tests/windows_threads.rs:31,36`
   (`6524b41d0ccb910f → 6df369991465940e`, `0719a8dcb0192e7e → 0da423be2f16b8bb`).
   **Meaning did not move**, and this card corrects an earlier reading of its
   own: these are exactly the values card `WRITE-LEND-TASKS` (wave 31) re-pinned
   deliberately (`docs/reports/write-lend-tasks/README.md:246-247`). That card
   moved the Linux copy in `compiler/tests/f02_trace_relation.rs:79-80` and
   **missed the Windows-only twin**, which `#![cfg(target_os = "windows")]`
   keeps out of every Linux gate. So `37bc9004` brings a stale duplicate into
   agreement — a repair, not a new pin. What remains is that a pinned
   expectation moved in a commit whose message does not say so and which no
   report indexes, against `CLAUDE.md`'s "update the pin *deliberately* and say
   so in the commit message and report". One of the two is CT01's digest, and
   CT01 is proof-gated.
2. It adds `#![cfg(unix)]` to three `providers/conformance/tests/` files,
   **removing four test functions from the Windows provider gate**. The three
   shared fixtures use no unix-only API, so this is a behavioural exclusion, not
   a compile fix. It is invisible in the ledger because the Linux "122 provider
   (carried; unchanged)" figure does not move.

> **Residual `WINDOWS-COHERENCE-GATES-UNCARDED`. Discharge:** a retrospective
> record for `37bc9004` stating the re-pin deliberately, as `CARD-SCOPE-SELF-MERGE`
> was retro-promoted. The code is not challenged.
> **Residual `F02-DIGEST-PIN-DUPLICATED`.** The two digests are bare literals in
> two files with no shared source and no cross-check, which is *why* a wave of
> drift was invisible. **Discharge:** give both files one source, or add a
> host-unconditional test asserting the two pin sites agree.
> **Residual `PROVIDER-CONFORMANCE-UNIX-ONLY`.** After `37bc9004` the "122
> provider" figure is a **Unix** measurement and the ledger's "carried;
> unchanged" carries no platform qualifier. **Discharge:** qualify the row, or
> restore the four tests on Windows.

### 4.4 `LEDGER-BINARY-COUNT-CORRECTED-IN-ONE-FILE-ONLY`

`docs/reports/lean-proof-integrity/validation.json` settles the split:
`workspace.test_binaries = 216`, `workspace.result_summaries = 227` (with
`doc_test_result_summaries = 11`). So **216 binaries / 227 result summaries is
correct**, and `AGENTS.md`'s gate row is **right**.

The defect is the opposite of a miscitation: the correction was applied in
`AGENTS.md` **only**. "227 binaries" stands uncorrected in five places — the
`gate.json` note that `AGENTS.md`'s own row cites,
`docs/reports/wave33-close/README.md:169,181,184`, and
`docs/development/increments/wave-33.md:98,110`. `AGENTS.md` also asserts the
run reproduced the integrator's figure "exactly, test for test", so the two
labels cannot both be right.

> **Residual `LEDGER-BINARY-COUNT-CORRECTED-IN-ONE-FILE-ONLY`. Discharge:**
> propagate 216/227 to the five remaining sites, **or** state which figure each
> counts. Frozen-archive care applies: `gate.json` is a close artifact, so
> publish provenance beside it rather than editing it (constraint 21).

### 4.5 Two structural defects the wave-33 union merge created

The `d3225f91` union was resolved **carefully for `AGENTS.md`** — it takes the
close branch's stronger text and substitutes the corrected count into it, with
no duplication and no dropped sentence. `docs/development/increments/wave-33.md`
fared worse:

1. **Two contradictory numbers now sit in one file** — line 37 "216 test
   executables / 227 result summaries" (from one parent) against lines 98 and
   110 "227 binaries" (from the other). Neither parent contained both.
2. **A missing blank line at lines 45/46.** I verified with `cat -A`: the Lean
   paragraph's last line is followed immediately by "Wave 33 close
   (2026-09-15):". In Markdown the two increment entries render as **one fused
   paragraph**, against the sync rule's "one paragraph per increment, newest
   first".

> **Residual `WAVE-33-RECORD-FUSED-PARAGRAPH`. Discharge:** insert the blank
> line at `wave-33.md:45/46` and reconcile the 216/227 pair with §4.4. Trivial
> to fix, invisible until rendered, and it is the sync rule's own unit of record.

### 4.6 The Lean sweep is clean — and leaves two dangling citations

This card delegated a full theorem-name diff of `d117b74f` (the Lean 4.34
upgrade) and then spot-checked it. **13 theorems removed, 6 added**; every
removal verified vacuous by reading its statement and proof term — ten literal
self-equalities (`f x = f x := rfl`) and one existence-only theorem named as
uniqueness, matching the record exactly. Two **genuine** defects were repaired
rather than deleted: `BoundFieldRead.an_exposed_base_never_admits_a_custody_read`
and `TypedErrors.validated_catch_join_has_one_normal_ledger` each **assumed
their exact conclusion** and now derive it. `AuditAll.lean`'s axiom-closure
`allowed` array is byte-for-byte intact. The "128 Lean audits" figure is honest:
the single addition is `AuditReturnPositionEscape.lean` from
`C-GATEWAY-RETURN-POSITION`, not from the sweep. **The proof surface lost count,
not content.**

One loose end, verified by me:

> **Residual `DANGLING-THEOREM-CITATION-AFTER-LEAN-SWEEP`.**
> `QCore.BranchJoin.evaluation_is_state_free` was removed as a self-equality and
> is absent from `proof/` (it survives only in archived audit logs), but two
> frozen reports still cite it as a ground:
> `docs/reports/wave19-or-and-lowering/README.md:209` ("literal comparison
> (`BranchJoin.evaluation_is_state_free`)") and
> `docs/reports/wave19-if-statement/README.md:376`. Neither appears in the
> superseding report's "Relevant historical evidence" list. Correctly, the
> frozen reports were **not** edited (constraint 21). **Discharge:** name both
> in `docs/reports/lean-proof-integrity/README.md`'s supersession list, so a
> reader following the citation is not left at a theorem that no longer exists.

### 4.7 The measured negatives — searched for and not found

Recorded so the negative result is measured rather than assumed. Across
`8bf7fd33..d3225f91`: **only two commits touched `crates/` or `compiler/src/`**
outside card branches, one of them comment-only; **zero** renames or deletions
under those paths; **zero** new `#[ignore]`; **three** new `cfg` gates, all in
§4.3; **eleven** removed assertion lines, every one traced — the only
fail-closed-looking removal (`assert_ne!(measured.status, AnalysisStatus::Safe)`)
is replaced by a **strictly stronger pair**. An evil-merge scan over all 15
merges found four carrying resolution content; three moved no meaning
(a fail-closed length pin `[&str; 19] → [&str; 20]` edited correctly at
`e714ae3f`; a CRLF/path union at `d81e29df`; a verified-faithful conflict-marker
repair at `9f90fc42`), and the fourth is §4.5.

One historical note, wave 32, recorded without a residual because it is already
repaired: commit `1c7d8e45` rewrote the matrix at `indent=2`, producing a
**12,673-line diff whose only semantic change was `named_followups` 39 → 40** —
precisely the failure HYGIENE-16's byte format exists to prevent. The next
commit restored `indent=1`. **This card's second added test now fails on exactly
that condition** (§6).

### 4.8 What the ledger audit found sound

Every other headline number traced to a cited artifact and **reconciled**: 185
units (2/42/137/4), 4,834 functions (179/85/4,570), leak **272**,
`unknown_alias` **4,208**, `double_release` **1**, `unsupported` **50,117**, the
33 `Unknown → Safe` moves with zero unit moves, CT **45/50**, sanitized **9**,
proof **3**, contract corpus **76**, `named_followups` **47** (list, `dimensions`
and `AGENTS.md` prose all agreeing), and every crate version. The wave-33
arbiter's "BEFORE reproduces the wave-31 baseline byte for byte" was confirmed by
sha256 of the decompressed artifacts.

A sweep of the waves 32–33 reports stat'ed 200 backtick-quoted repo paths and
found **zero** missing committed paths — the wave-33 six-nonexistent-logs defect
did not recur. **28 citations point into gitignored `measure/` or `private/`**;
see §8.2.

---

## 5. Step 4 — what a queued card would make load-bearing

| Queued card | Rule made load-bearing | Why it is safe today | What breaks |
|---|---|---|---|
| `WRITE-LEND-TASKS-EXERCISE` | `TRACE-ALPHABET-LEASE-GROWTH-UNDECIDED` (15n) | no worker-side range write exists, so no admitting rule can be shown non-vacuous | CT03's method needs both halves; growing the observable alphabet becomes mandatory |
| `WRITE-LEND-TASKS-EXERCISE` | `CT47-WRITE-CAPTURE-UNEXERCISED` | the `write` capture parses, has no lowering, no fixture drives it | a never-exercised refusal becomes the live gate for a shape users write |
| **`FB-BACKLOG-CUSTODY`** | **`RETURN-POSITION-ACCUSING-UNEXERCISED-IN-CORPUS`**, and through it **constraint 7's leak pin 272** | the wave-33 widening added zero leak leaves: no pinned-corpus function has an admitted pointer-leaf return in a body holding a live obligation | backlog wrappers return chunk pointers through field chains in acquiring bodies — exactly that shape. The pin moves and the accusing arm is exercised **in production first**, never in a fixture |
| **`FB-BACKLOG-CUSTODY`** (member typing) | **`RETURN-POSITION-ESCAPE-UNREACHABLE`** (§2.3) and §3.3's elided premise | terminal **only** because `TrackedBasesAreBound` holds for every admitted chain | widening member typing is the one move that can put a projection-bound place on an admitted chain. The `None` answer stops being a no-op and starts **deciding escapes**, with a mutation-confirmed-vacuous arm as the decider — and §2.3's "no test can ever distinguish" stops being true |
| `FB-SYSTEM-EXTENSIONS` (lock order) | `MUTEX-SHARE-CROSS-MULTI`; constraint 27's refused mutex shapes; **§2.2's two dead arms** | one wrapper, one payload, one mutex, no nesting; the task route has no mutex | lock order is intrinsically nested and multi-mutex. A second sharing kind falsifies the pinned `the_mutex_protocol_is_requestable_for_exactly_one_kind` **and makes both dead arms reachable, with no test behind either** |
| `FB-BACKLOG-ACCOUNTING` | constraint 15j's "`Q637` deliberately not extended" | correct only while `spent ≤ granted` is structural and restored credit is not re-granted | path-sensitive or re-granted credit makes `Q637`'s absence a real gap |
| `FB-RETRY-LIFETIME` | constraint 15m's `Q600` ordering premise | no asynchronous completion exists; every composed region sits after all joins | retry **is** "children live at the opening"; the refusal must be proved correspondent, not relaxed |
| `FB-ENGINE-PROGRESS` | constraint 10b's acyclic-CFG-only restriction; constraint 4's `N(m)` neutral start | engine callback loops are not in the selected slice | the card's subject is callback **loops**; `N(m)` goes from one-component evidence to load-bearing at scale |
| `FB-SYSTEM-CENSUS` | constraint 11's `HistoricalStdin` pin; constraint 12a's depth-0 rule | the pinned corpus is the only thing analysed through that transport | a declared-file transport records a different `CompileTaskIdentity` for the same preprocessing |

The bold rows are the finding: **`FB-BACKLOG-CUSTODY` reaches both
return-position residuals at once**, and it is the card that turns §2.3's
terminal boundary back into an open question. Wave 33 named both boundaries
honestly without naming the queued card that retires the reasoning behind them.
Under `CLAUDE.md`'s step 4 that card must be named in the finding — it now is.

---

## 6. What this card changed

**Two Python tests**, in `scripts/tests/test_build_matrix_sections.py`, class
`CommittedMatrixIsSelfConsistent`. They discharge the **first half** of
`D02-DIMENSIONS-FOLLOWUP-COUNT`, a residual that **fired twice** (waves 32 and
33) and was caught both times by a person, never by the suite — because the two
existing tests compare *generator output against generator output*, both
self-consistent by construction, so neither can see drift in the **committed**
file.

Non-vacuity **measured by mutation**, not asserted:

| Mutation | Result |
|---|---|
| `dimensions.named_followups` 47 → **45** (the exact historical drift) | **RED** — `AssertionError: 45 != 47 : dimensions.named_followups is 45 against a named_followups of 47; regenerate through docs/reports/remaining-d02/build_matrix.py rather than repairing the count by hand` |
| re-serialise the matrix at `indent=2` (the exact `1c7d8e45` condition, §4.7) | **RED** — the byte-format assertion |
| unmutated | **GREEN**, 2 tests |

After both mutations the matrix was restored and verified byte-identical by
sha256 (`e5668f2e374926ead3e0bf899903fc51b050018c15f27a9c2724b871b6ddabd0`).

**Not discharged:** the residual also asks for a check over *each* derived
dimension. This card covers the two whose name **is** a section name
(`requirements`, `named_followups`); the derived cell and product counts
(`requirement_evidence_cells`, `ct_cells`, `ct_rows`, `unverified_spec_items`)
are left, because each needs its own derivation rule and a wrong one would pin a
wrong number. `D02-DIMENSIONS-FOLLOWUP-COUNT` stays open, **narrowed**, with its
remaining scope stated here and in its matrix entry.

Python tests move **227 → 229**, deliberately and attributably. No Rust, Lean,
model, corpus, contract, manifest or frozen archive changed. No gate weakened,
no verdict rule touched, nothing promoted.

---

## 7. Gate

Run at this card's tip after `git submodule update --init --recursive`
(`WORKTREE-SUBMODULE-UNGATED`; submodule checked out at `d82464a2`). Reported
honestly, including the failure.

| Command | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | see §7.1 — **one failure**, pre-existing, not caused by this card |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | **exit 0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | **exit 0** |
| `python3 scripts/check_lean_audits.py` | **exit 0**, **128 Lean audits** |
| `python3 -m unittest discover -s scripts/tests` | **229 passed**, OK (227 + this card's 2) |

**The Lean number did not move.** The brief flagged that `main` carries a Lean
4.34 upgrade and substantial proof changes landed after the reference figures
were measured, and asked for any difference to be attributed. There is none:
128 audits, matching both the ledger and `find proof -name "Audit*.lean" | wc -l`.
§4.6 accounts for what the proof changes did and did not remove.

### 7.1 The workspace failure, stated rather than hidden

The one failing row is
`qcore-verifier-host :: closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request`,
measured in §2.1: deterministic, 3/3 in isolation, on the **unmodified** tree at
`d3225f91` before this card touched anything. It is a **finding, not a build
failure**, and this card deliberately did not absorb it: repairing it is scoped
in §2.1's discharge condition, and doing it here would mean a measurement card
editing `crates/` — exactly the `CARD-SCOPE-SELF-MERGE` breach wave 33
registered.

---

## 8. Postponed, and why — recorded as outcomes

1. **Windows and macOS arms.** No such host. Every Windows-facing finding (§4.3,
   `CT-COMPOSED-NATIVE-NON-LINUX-VACUOUS-PASS`) rests on reading source and
   commits, never a run. No Windows or Apple claim is made.
2. **The gitignored-citation shape.** 28 citations in waves 32–33 reports point
   into `measure/` or `private/`, unverifiable from the merged tree. This is a
   consequence of `CLAUDE.md`'s own worktree-private measurement rule, so naming
   it a residual would be proposing a rule change this card was not cut to make.
   Recorded as a **measured boundary**: a card report's log citations are in
   general not checkable after the merge — the mechanism that let wave 33's six
   nonexistent logs stand, and the reason
   `LEASE-BOUNDARY-MUTATION-EVIDENCE-UNSHIPPED` (§2.3) matters more than it
   looks.
3. **Semantic review of Lean statement *meanings*.** §4.6 measured the *removals*
   and found them vacuous, which is more than the brief asked. It is **not** an
   exhaustive review that every surviving theorem says what its name claims; the
   card's own `LEAN-STATEMENT-SEMANTIC-REVIEW` declines that, and this card
   registers the name without attempting it. **No claim is made** that the
   surviving 8,884 statements are semantically apt.
4. **The `C-GATEWAY-INDIRECT-CENSUS` salvaged partial** (18,555 indirect-access
   leaves; 284 functions holding 833). Unaudited and reproducible from nothing
   on `main`. Not re-derived; it decides nothing and this card leaves it so.
5. **Repairing any finding.** This card is an audit. Every discharge condition
   is written for a later card or the integrator. The two Python tests are the
   single exception, and they add a check rather than change a rule.

---

## 9. For the integrator

An explicit reconciliation list, because §4.1 is a finding about this step being
skipped.

1. **Residuals registered — done, not proposed.** This card appended **27**
   entries to `named_followups` (**47 → 74**) via
   `docs/reports/stability-reeval-1/scripts/add_followups.py`, and amended
   `D02-DIMENSIONS-FOLLOWUP-COUNT`'s `why` to record §6's partial discharge. Two
   of the 27 are wave 33's own vacuity residuals, which had no entry (§4.2).
   The matrix was then **regenerated through
   `docs/reports/remaining-d02/build_matrix.py --gate docs/reports/wave33-close/gate.json`
   to a byte-identical fixed point** — so the append is generator-clean and
   `dimensions.named_followups` is derived, never hand-set (constraint 25).
   `AGENTS.md`'s follow-on count was updated **47 → 74** in the same change;
   that is the one `AGENTS.md` edit this card makes, and it is a count this card
   is responsible for.
2. **Apply §3.4** — the CT ledger row's lease clause contradicts constraint 15n
   in the same file and is measurably false. One sentence.
3. **Apply §2.3's `LEASE-BOUNDARY-MUTATION-COUNT-OVERSTATED`** — 15n says five
   of five; the measurement is three of five.
4. **Decide §2.1.** The only finding with a red test behind it. Options
   (a)/(b)/(c) are stated; weakening the assertion is not among them.
5. **Sweep wave 32 retrospectively** (§4.1). Six cards, including the two whose
   unswept integrator sections produced findings §2.2, §2.3 and §3.4.
6. **Fix §4.5's blank line** and reconcile §4.4's 216/227 across the five
   remaining sites, using provenance-beside rather than editing `gate.json`
   (constraint 21).
7. **Gate row:** Python **227 → 229**; Lean **128**, unmoved; workspace tests
   reported with one pre-existing failure (§7.1).
8. The four wave-33 deferred decisions (`CT10-REASON-ROW`, `DOCS-COMPACT-6`,
   `MATRIX-UPDATED-STALE-AT-WAVE-27`, `SYNC-RULE-TOOLING-COMMIT-SCOPE`) are
   registered here so they stop being invisible; they remain the integrator's
   and the user's to decide, and this card takes none of them. `37bc9004` (§4.3)
   is a live instance of the fourth.
9. `CLAUDE.md` is **byte-identical to the wave-32 tip** — wave 33 did not touch
   it — so its platform records and its own `STABILITY-REEVAL` cadence note are
   a wave behind, and `DOCS-COMPACT-6` is due "around wave 34" by its own text.
   This card did not run it.

---

## 10. Residual index

All 27 registered in `named_followups` (47 → 74). "New" = first named by this
card; "adopted" = named elsewhere and reaching no tracker until now.

| Residual | § | Origin |
|---|---|---|
| `VERIFIER-HOST-LARGE-RESPONSE-CONTROL-RED` | 2.1 | new |
| `MUTEX-SHARE-CROSS-UNEXERCISED-ARMS` | 2.2 | new |
| `RETURN-POSITION-ESCAPE-UNREACHABLE` | 2.3 | adopted (wave 33, no entry) |
| `RETURN-POSITION-ACCUSING-UNEXERCISED-IN-CORPUS` | 2.3, 5 | adopted (wave 33, no entry) |
| `LEASE-BOUNDARY-MUTATION-COUNT-OVERSTATED` | 2.3 | new |
| `LEASE-BOUNDARY-MUTATION-EVIDENCE-UNSHIPPED` | 2.3 | new |
| `AUDITALL-TRIPWIRE-CONTROLS-UNCOMMITTED` | 2.4 | new |
| `CT-COMPOSED-NATIVE-NON-LINUX-VACUOUS-PASS` | 2.4 | new |
| `CT-SHARED-FIXTURE-DISCLOSURE` | 3.1 | adopted (wave 32) |
| `GATEWAY-LEAN-ABSTRACT-NOT-EXTRACTION` | 3.2 | new |
| `RETURN-POSITION-SOUNDNESS-PREMISE-ELIDED` | 3.3 | new |
| `LEDGER-CT-ROW-LEASE-ARM-STALE` | 3.4 | new |
| `LEASE-BOUNDARY-TRANSCRIPTION-PREMISE` | 3.5 | new |
| `GATEWAY-CONSERVATIVE-UNATTACHED` | 3.5 | new |
| `WAVE-32-CLOSE-UNSWEPT` | 4.1 | new |
| `SWEEP-RESIDUALS-SCANS-REPORTS-ONLY` | 4.2 | new |
| `WAVE-BASE-UNSWEPT` | 4.2 | new |
| `RESIDUAL-NAMED-ONLY-IN-A-CONSTRAINT-IS-UNTRACKED-WORK` | 4.2 | new |
| `WINDOWS-COHERENCE-GATES-UNCARDED` | 4.3 | new |
| `F02-DIGEST-PIN-DUPLICATED` | 4.3 | new |
| `PROVIDER-CONFORMANCE-UNIX-ONLY` | 4.3 | new |
| `LEDGER-BINARY-COUNT-CORRECTED-IN-ONE-FILE-ONLY` | 4.4 | new |
| `WAVE-33-RECORD-FUSED-PARAGRAPH` | 4.5 | new |
| `DANGLING-THEOREM-CITATION-AFTER-LEAN-SWEEP` | 4.6 | new |
| `FB-PLAN-GATE-HOST-TIMEOUT` | 2.1, 4.2 | adopted (wave 33) |
| `LEAN-STATEMENT-SEMANTIC-REVIEW` | 8.3 | adopted (wave 33) |
| `WAVE-33-DEFERRED-DECISIONS-UNTRACKED` | 9.8 | adopted (wave 33, four names) |

`D02-DIMENSIONS-FOLLOWUP-COUNT` is **amended, not added** — partially discharged
by §6 and narrowed to the derived cell and product counts.

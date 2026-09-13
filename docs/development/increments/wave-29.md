# Wave 29 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

<!-- migrated-section:start -->
Wave 29 close (2026-09-13): **six cards** merged one at a time under the
failure-gated chain — `CT47-COPY-CAPTURE`, `CT22-WIRE`, `CT01-PROOF-LEG`,
`REGION-JOINS-DUTIES`, `SLICE-PERMISSIONS-WRITE-LEND` and the arbiter
`W29-CORPUS-CONTRACTS` — plus the user's Windows test fixes merged mid-chain.
The wave's head is the manifest's **first executed proof row**: CT01's
scenario property proved in Lean of the checked artifact's reference run and
its admitted renderings, bound to the case's own fixture by a named
host-unconditional test, native execution deliberately observation-only —
**proof 0 of 41 → 1 of 42**, gated by the pinned fail-closed
`PROOF_GATED_CASES` table, the stage evidence superseding card T00's frozen
archive from its own record (constraint 21). CT coverage 41/50 → **42/50**
(`CT22-WIRE`, on `REGION-CREDIT-JOINS`' evidence, the wave-28 narrowed reason
retired in its own record); sanitized-native 7 of 42. The language head is the
**write lend** (`lend(BUFFER, LOW, HIGH)`): write authority over a half-open
byte range, decided by the lend-extended cycle-3 ownership IR — the
`LendRange` footprint fraction `STD-BUFFER-SLICE` predicted — with constraint
15h amended on the card's own proposal; beside it `REGION-JOINS-DUTIES`
deliberately supersedes 15m's duty-free premise (amended on its proposal),
landing CT25's duty half. `CT47-COPY-CAPTURE` re-attributes the spawn
boundary: a `copy` of `own<Buffer>` refuses at `Q600` from the transferability
judgment naming the missing contract instead of the `Q500` backend catch-all
(kept as the unreachable fail-closed backstop), the refused set proved
unchanged in Lean. The wave ran **one arbiter**, deciding-direction only:
two faithful size-query contracts (corpus 73 → **74**), BEFORE run
byte-identical to the wave-27 baseline, AFTER run removing exactly thirteen
`unsupported` uncontracted-call leaves (49,773 → **49,760**), leak **266
unmoved** (no fifth move), zero status moves, nothing promoted; the corpus
baseline moves to `docs/reports/w29-corpus-contracts/artifacts/`. Zero
reserved words (`lend` contextual, censused +0) and zero new diagnostic codes
across the wave. Lean audits 115 → **118**. Merged gate at the close tip:
4,445 workspace tests (8 ignored, `--no-fail-fast`), 122 provider (carried),
193 Python, both probes exit 0, fmt/clippy clean
(`docs/reports/wave29-close/gate.json`); constraint-16 load rows that tripped
under concurrent suites were re-verified in isolation, both facts recorded
each time. The acceptance matrix is regenerated through `build_matrix.py
--gate` (hand-kept sections carried; the derived-fact tripwire and the CT
chain both green at the close tip). `DOCS-COMPACT-4` is due on the standing
cadence and now has waves 27–29 as material. Nothing is promoted; full
language conformance stays **false**.

W29-CORPUS-CONTRACTS (2026-09-12, wave 29): the wave's one arbiter card —
additional `Unknown`-class leaves of the pinned 185-unit corpus decided by
declaring **faithful** contracts, on the PROD-CONTRACTS → NEIGHBOURS → AOM
chain's arrangement. One new module,
`contract/qcore/fluent_bit/input_chunk_sizes.qcore` (corpus **73 → 74**,
resource-v1 declarations 39 → 41, manifest-selected modules 18 → 19, every pin
updated deliberately), declares `flb_input_chunk_get_size` and
`flb_input_chunk_get_real_size` — both read against their pinned bodies
(`src/flb_input_chunk.c:756-759`, `:767-790` at
`66910c10a4d7eafa810b84229cf2dcf7b0f26f97`), pure size queries whose every
transitive callee was read too, so `resource_borrowed(ic)` /
`resource_return_none()` are the only faithful modes; `ssize_t` spells `isize`
under the LP64 declared-ABI projection, and nullability is deliberately not
claimed (`resource_borrowed` imposes no non-null precondition — the weaker
statement). The BEFORE run reproduces the wave-27 `gateway-c-location`
baseline's `diagnostics.json` and `chunk-contracts.json` **byte for byte**;
the AFTER run moves exactly the **thirteen** `unsupported`
`call to uncontracted function` leaves that named the two callees (11 + 2,
all in `fluent-bit-chunk/flb_input_chunk.c`, each attributed with function,
internal line, origin coordinate and mechanism in the card report §5), with
**zero leaves added**: every actual resolved, so the admitted calls produced
no replacement refusal. Leak pin **266 unmoved** (no fifth move in
constraint 7's history), `unknown_alias` **4,344 unmoved**, `unsupported`
49,773 → **49,760**; zero unit status moves, zero function status moves, zero
reason-code moves — nothing is promoted, and the defining unit stays
provisional `Unsafe` on its unchanged five leak leaves. Both counted runs exit
0 on a fresh Clang pin (1,415 entries) with the same binary (the card changes
no Rust; the only input difference is the manifest and the module it selects);
one discarded first AFTER attempt (exit 101, truncated output, no artifacts)
is archived and recorded as environment-class, contributing to no verdict. No
new diagnostic code, no keyword, no Lean module (no semantic surface moves —
the AOM-DESTROY-SIBLINGS precedent). New baseline
`docs/reports/w29-corpus-contracts/artifacts/`, chained on the frozen
predecessors (constraint 21); named follow-ons `W29-CONTENT-SIZE-UNDECLARED`
and `W29-SIZE-SIBLINGS-UNDECLARED`. Gate at the card tip: workspace tests,
fmt/clippy, both probes, Lean audits and Python suite per the card report §6.
Evidence: `docs/reports/w29-corpus-contracts/README.md`.

Wave 29, card `SLICE-PERMISSIONS-WRITE-LEND` (2026-09-12): the write half of
CT07 the read card named — `let NAME = lend(BUFFER, LOW, HIGH);` on
`source.buffer.v1`, a loan a store can name, so **two parties mutate two
disjoint ranges of one buffer independently** while both loans are live.
Accesses through a loan are decided against the loan's own `[0, HIGH-LOW)` and
rebased into the owner's coordinates; the window runs from the lend to the last
statement naming the loan. The custody shape is the card's head: residual
`STD-BUFFER-SLICE`'s footprint fraction now exists — the cycle-3 ownership IR
gains a `LendRange` borrow node carrying a half-open byte interval plus
`WriteRange`/`ReadRange` footprint-stating accesses, and the **IR itself**
decides the exclusivity rule (a second lend admitted exactly when disjoint,
`OverlappingLends` otherwise; empty lends conflict with nothing; exclusivity is
temporal), the range-wise transfer (owner reads and writes refused inside a
live lend's range, admitted outside every one; an unresolvable index conflicts
with everything, `WRITE-LEND-COMPUTED-INDEX`), the rebasing containment
(`AccessOutsideLendRange`) and the release arm, which is the existing
`DropWhileBorrowed` unchanged — every one arriving at the route through the
inherited `Q054`. A module with no lend builds the custody operation list it
always built, by a structural branch. Zero new codes (`Q051`/`Q052`/`Q053`/
`Q054`/`Q055` and the composed route's `Q077` condition widenings, stated in
§L21) and zero reserved words (`lend` is contextual on the
`len`/`concat`/`borrow_utf8` precedent, censused; `let lend = 5;` is still a
program). Fail-closed boundaries, each a named residual: a `for` beside a lend
(`SLICE-PERMISSIONS-LOOP-WINDOW`), crossed windows — borrow nodes are lexical —
(`SLICE-PERMISSIONS-CROSSED-WINDOWS`), an acquisition inside a window
(`WRITE-LEND-NESTED-ACQUIRE`), a lend bound outside the lowering-visible tier
(`WRITE-LEND-BOUND-FROM-BYTE`), re-lending (`WRITE-LEND-RELEND`), and a lend
beside `borrow_utf8` on the composed string route
(`SLICE-PERMISSIONS-COMPOSED-LEND`). 31 end-to-end tests in
`compiler/tests/write_lend.rs` (one sanitized native run — ASan is
load-bearing: a displaced-pointer write is the construct's worst failure mode —
Linux arm only, `WRITE-LEND-WINDOWS-RUN`), 7 IR-level unit rows,
`QCore.WriteLend` + `AuditWriteLend.lean` (25 declarations audited by name,
115 → **116** Lean audits; exclusivity is the read card's own set-theoretic
`overlaps` imported, every byte has exactly one write authority, disjoint
lends' rebased accesses cannot collide, a confined window preserves every
unlent byte). Two fixture programs a CT lane can point at
(`disjoint_write_halves.qcore`, `overlapping_lends.qcore`); the lane statement,
the proposed 15h amendment and the boundaries are in
`docs/reports/slice-permissions-write-lend/README.md`. Corpus-neutral by
construction: no gateway, contract, audit or provider file in the diff, leak
pin 266 untouched, nothing promoted.

REGION-JOINS-DUTIES (2026-09-12, wave 29): the remainder `REGION-CREDIT-JOINS`
named, landed as CT25's **duty half** — *"cleanup destroys owner after
joins"*, expected Accept, now expressible and accepted end to end. The
composed region body on `source.epoch.tasks.v1` admits the task route's own
acquisition and release — `let NAME = qcore_buffer_new(INT)?;` sealing a real
`CleanupAction::Destroy` duty, `drop NAME;` of a region owner erasing one — so
the sealed pack carries duties beside the remainder and the opening's
`discharge` destroys the remaining owners in **reverse acquisition order,
after every join**. This **deliberately supersedes constraint 15m's duty-free
premise** (amendment text proposed in the card report §2 for the integrator).
The emission story is the sequential route's precedent read at the text level:
the region **erases into its straight-line twin** — the split keeps
acquisitions and body releases at their own byte offsets in the task half and
writes one `drop NAME;` per remaining duty into the blanked space after them,
first-fit, newlines never crossed, refusing when it does not fit
(`REGION-JOINS-ERASURE-FIT`) — so the three composed checkers stay unedited
and the destruction is a real emitted, custody-checked, reference-executed
program (a native run under counting allocator hooks frees every buffer,
worker result and region owner both). A duty-free region still blanks
completely, byte for byte (the wave-28 measurement kept). Both published
figures are re-derived before publication: `abandoned_credit` against the
reader's quantity fold and the new `released` against its duty fold, over the
model's own cleanup trace. Every refusal that stands and is not the admitted
one keeps its exact code: the after-joins `Q600` both directions, `Q635`/`Q636`,
the model's `Q303`–`Q309` (Q305/Q306 now reachable with a real duty), and the
sequential `qcore_acquire` spelling stays refused. Measured boundary: two
simultaneously live owners refuse at the task route's **existing** custody
`Q600` — the second acquisition's `?` failure edge discards the first, the
erased program's real leak path (`REGION-JOINS-MULTI-OWNER`); the sequenced
spelling is admitted. CT25's **loan-expiry half is not landed**: read-loan
captures stay refused (`T08-COMPOSE-LOAN`), so *loan expiry* still has no
referent on this route — a lane should not map CT25 until it decides whether
the duty half alone carries the scenario (the card report §7 states both
readings). Zero new codes (`Q302` condition widenings, stated in §L21), zero
reserved words (every admitted token already parsed on this route), no model
edited, no manifest/matrix/corpus file touched. Rust: 9 new end-to-end tests
(`compiler/tests/region_joins_duties.rs`) on 8 fixtures under
`compiler/tests/fixtures/epoch_credit/CT25/`, 3 new frontend unit tests; Lean:
`QCore.RegionJoinsDuties`, 14 audited theorems, audits 115 → **116**.
Evidence: `docs/reports/region-joins-duties/README.md`.

Wave 29, card `CT01-PROOF-LEG` (2026-09-12): the first **proof-stage
capability** for a mapped CT case, delivered without flipping the row — the
manifest and the acceptance matrix are untouched, proof still reads 0 of 41,
and the exact proposed stage-record terms are in the report's §6 for the
integrator (`docs/reports/ct01-proof-leg/README.md`). The Lean side
(`proof/QCore/Ct01OwnedTransfer.lean`, audit `AuditCt01OwnedTransfer`, Lean
audits 115 → **116**) proves CT01's scenario property on the case's own trace
terms: a total decision procedure `dischargesOwnedTransfer` with meaning
theorems for arbitrary traces (the parent performs exactly allocate, spawn,
join, read, drop, complete in that order; the moved generation is released
exactly once, by the parent, after the join; the child's only completion
returns the generation the parent's only allocation created), the
kernel-checked acceptance of `TaskTraceRelation.roundTrip` — F02's recorded
transcription of CT01's reference run — the **symbolic closure**
`relate roundTrip observed = 0 → dischargesOwnedTransfer observed = true` over
arbitrary admitted renderings, five named refusal mutants, and the
transcription pin `mixTrace roundTrip = 13023433800416048247`. The named
host-unconditional runner
(`compiler/tests/ct01_proof_leg.rs::ct01_artifact_is_the_lean_proved_owned_transfer_shape`)
compiles CT01's own fixture through the real pipeline, checks the derived
reference events against the recorded F02 fixture trace event for event,
recomputes the Lean-proved digest from the artifact with wrapping `u64`
arithmetic, runs the same total decision procedure, and re-checks the five
mutants on the refusing side. Boundaries stated, not implied: **no native
refinement is claimed** (the emitted C stays covered only by
`run_native_owned_transfer`'s byte comparison and F02's relation checks), and
the Lean–Rust tie is F02's checked-digest transcription agreement, not a
verified extraction. Two registered instances (`ReflBEq`/`LawfulBEq` for the
F02 alphabet) were added in the new module; `TaskTraceRelation.lean` itself is
untouched, `git diff` over `crates/` and `compiler/src/` is empty, no
diagnostic code is allocated and no word is reserved.

Wave 29, card `CT22-WIRE` (2026-09-12): the wave-29 CT mapping lane, one case.
**CT22** — *"Quant opening discards parent-local unused credit after joins"*,
expected Accept — is **mapped** on the `composed_epoch` route, converting
wave-28 card `REGION-CREDIT-JOINS`'s capability into a manifest row exactly as
its §7 prescribed: CT coverage **41/50 → 42/50 mapped**, 8 unmapped, by
mapping, not conformance (constraint 23). The `epoch_credit_split` verdict
mode is **widened for the accepting shape** — the case's cascade is empty
exactly when it pins its own published figures, and the runner compares them
wholesale: the credit-recovery split under the conservation law, the region's
independently re-derived `abandoned_credit` of 3, and the two parent-local
premises re-asserted on the published objects (no `Acquire` in the published
region body; every task-ledger span before the region statement). The
fixtures move from `epoch_credit/CT22/` into `fixtures/ct/CT22/` (the CT23
precedent, `region_credit_joins.rs` repointed, all 9 tests green on the moved
bytes), plus two new refusing controls measured before pinning (`spawn_after`
`["Q600"]`, `acquiring` `["Q302"]`); `ct_corpus.rs`'s pinned tables grow
deliberately (`MAPPED_CASES` 42, `EPOCH_CREDIT_SPLIT_CASES` + CT22,
`EVIDENCE_FILES` 13). One deliberate stage-derivation rule: an **admitted**
composed-epoch module's `c_emission`/`native_execution` read `unassessed` —
it really emits the task route's own C that no harness runner executes, an
open item, never a claim and never `not_applicable` (named follow-on
`CT-COMPOSED-NATIVE`). The wave-28 lane's narrowed CT22 row is **retired in
this lane's own record** (`retired_unmapped_reasons`; the wave-28 bytes
untouched, constraint 21), `acceptance.json` is regenerated through
`ct_stage_summary.py --write` with `derived_from` grown, and the derived
`newly_decidable_not_mapped_reasons` map is untouched and verified — keys
`["CT24", "CT47"]` before and after (constraint 25). Sanitized-native stays 7
(now of 42), proof 0 of 42; zero new Q codes, zero condition widenings, zero
reserved words, no crate-source change, no gateway/contract/audit/provider
file touched (corpus carried, leak pin 266). Gate at the card tip: 4,392
workspace tests (8 ignored, `--no-fail-fast`, 0 failed — all five
constraint-16 rows green on the first attempt), 193 Python OK, 115 Lean
audits, both probes exit 0, fmt/clippy clean, `ct_stage_summary.py . --check`
exit 0 at `mapped 42 / unmapped 8`. Evidence:
`docs/reports/ct22-wire/README.md`.

Wave 29, card `CT47-COPY-CAPTURE` (2026-09-12): the boundary refusal CT47
names is now the transferability judgment's own. A task-creation capture that
`qcore_core::transfer` (card `CT46-SENDSYNC`) and the capture enumeration
agree to refuse is refused **by the judgment**, attributed to the capture at
the crossing construct — `Q600`, a condition widening stated in §L21, one
message per `TransferRefusal` variant naming the missing transfer/share
contract and the contracts the kind does carry — where before it fell through
to the enumeration's executable-subset catch-all at `Q500` (`CEmission`), the
attribution gap the wave-26 lane measured and refused to map CT47 on
(`docs/reports/ct-wire-lane/README.md` §3.8). The refused set is unchanged
and that is proved, not asserted: `QCore.Transferability` gains the refusal
half of the judgment (`Refusal`, `refuse`, `refuseCapture`) with
`theBoundaryIsDecidedExactlyOnce` partitioning the whole 13 × 4 × 13 matrix
against the admissions, registered in the existing `AuditTransferability`
(audits stay 115). The grammar gate (`Q000`, a modeless capture) and the
`Q500` catch-all are untouched — the catch-all stays as the unreachable
fail-closed backstop — and the wave-26 lane's three shapes are re-measured
and pinned by
`transferability.rs::ct47s_measured_shapes_a_copy_capture_is_refused_by_the_judgment`:
`Q000` byte-identical, `copy buffer` moved `Q500 → Q600`, the declared read
share still admitted. Scope landed is the **nearest expressible shape**: the
language still has no raw pointer type, so the copy of an owned handle stands
in for the case's bit-copy and the residue is published rather than papered
over. CT47 stays **unmapped** — its manifest and derived-matrix rows belong
to a mapping lane (constraint 25); this card delivers the capability, the
readiness statement and the updated hand-kept `acceptance.json` reason.
Zero new codes, zero reserved words, no gateway/contract/audit/provider file
touched (corpus rows carried, leak pin 266 untouched by construction).
Evidence: `docs/reports/ct47-copy-capture/README.md`.

<!-- migrated-section:end -->

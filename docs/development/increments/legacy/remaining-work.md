# Historical guidance: remaining work and delegated increments

[Development table of contents](../../../../DEVELOPMENT.md) · [History index](../README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../../AGENTS.md); each card's report owns its evidence.

<!-- migrated-section:start -->
#### Remaining-work wave accounting (2026-09-07)

*Moved verbatim from `AGENTS.md` lines 1146-1179 — fuller variant with the milestone-criteria list and named follow-ups.*

Remaining-work wave accounting (2026-09-07, implementation checkpoint `67b1a9a`):
the twenty-five cards recorded
under `docs/reports/remaining-*/` are reconciled into one traceable matrix,
`docs/reviews/r0_3/acceptance_matrix.json`, with a readable companion at
`docs/reports/remaining-d02/README.md`. This paragraph supersedes nothing above:
each card's own record keeps its checkpoint, counts and boundaries. Milestone
criteria met, at their stated boundaries: the documented checkpoint, submodule
pin, branch integration, CT inventory, contract corpus and report presence all
reconcile, and the recorded 819/45/43/20 baseline reproduced exactly before any
change; the R01–R10 soundness gates were verified adversarially with no
soundness failure found and no production semantics changed by the V cards;
source-derived obligation completeness and evidence identity were checked
against 52 replay/cache mutations and a compile-task identity that retires a
witness when a used header changes, with zero accepted mutations; the six
CMetrics Unknown functions are classified into a ranked, corpus-measured blocker
inventory; one generic blocker family — typed direct scalar-field access — is
implemented as a conjunctive type-only whitelist with 31 Lean theorems and both
precision directions exercised; a second pinned source set is audited from
configuration alone with no analysis-code change; the provider execution profile
now installs the limits a manifest declares on Linux and refuses rather than
silently downgrading, with an accurate non-sandbox trust boundary; the CT ledger
is stage-derived, so no mapped case claims a stage its executed evidence does
not carry. Nothing is promoted: raw Cycle 19 units remain 2 Safe / 42
provisional Unsafe / 137 Unknown plus four data-only units, CT coverage remains
11/50 mapped, the contract corpus remains 59 modules and full language
conformance remains false. `qcore-schemas` is now 3.0.0 (P02's public
`PluginLimits::additional` field); prose revision 0.3, plugin protocol v1,
evidence v2 and program schema/profile v2 are unchanged. Current gate: 997
workspace tests, 47 provider tests, 43 Python tests, 21 Lean audits, both
reevaluation examples exit 0, fmt and clippy clean. Named follow-ups: 185-unit
corpus re-measurement and deliberate re-pinning after C02 and G02; E01-F1
definition enumeration; C02-P; O04; T03; P03; P01-F2; G03-F1/F2; the L21.1
reason-code implementation increment; and the T00 stage-derived CT manifest
schema.

*Moved verbatim from `CLAUDE.md` lines 896-906 — condensed variant.*

Remaining-work wave accounting (2026-09-07): the wave's twenty-five
`docs/reports/remaining-*/` cards are reconciled into
`docs/reviews/r0_3/acceptance_matrix.json` (17 requirements x 4 independently
marked evidence classes, plus stage-derived CT rows) with a readable companion
at `docs/reports/remaining-d02/README.md`. AGENTS.md carries the single
consolidated status paragraph; read it there rather than duplicating it. The
implementation checkpoint for that wave is `67b1a9a`. Nothing is promoted: raw
Cycle 19 units stay 2 Safe / 42 provisional Unsafe / 137 Unknown plus four
data-only units, CT coverage stays 11/50 mapped, the contract corpus stays 59
modules and full language conformance stays false. `qcore-schemas` is 3.0.0;
prose revision 0.3, protocol v1 and evidence v2 are unchanged.

#### Delegated increment (2026-09-08)

*Moved verbatim from `AGENTS.md` lines 1181-1246 — the five-card wave and its `Nothing is promoted` closing paragraph; byte-identical in CLAUDE.md 908-973.*

Delegated increment (2026-09-08): five bounded cards landed through isolated
worktrees with per-card reports; this paragraph supersedes nothing above and
each card's record keeps its own boundaries.
- Multi-provider composition (E03): seventeen composition properties verified
  against unchanged code and pinned; two fail-closed repairs in
  `compose_verification` — an unchecked `Unsafe` component outcome now
  downgrades to Unknown under QP010 instead of being republished verbatim, and
  unequal component premise sets refuse under QP012 instead of publishing one
  component's assumptions. Fifteen audited Lean theorems in a new
  `Composition` namespace, no new axiom. Four completeness boundaries stay
  recorded open (E03-B in the acceptance matrix). Evidence:
  `docs/reports/remaining-e03/README.md`.
- Enforced confinement (P03): the verifier host gains a target-qualified
  `linux-confined-v1` profile composing P02's setrlimit plan with Landlock
  filesystem confinement and a `CLONE_NEWUSER|CLONE_NEWNET` network namespace
  installed before exec. No default policy; five fail-closed pre-spawn refusal
  grounds, each a scoped Unknown under QP006 with no downgrade. Twelve
  adversarial fixtures with legacy controls measure a blocked localhost
  connect (0 vs 1 accepted connections), blocked out-of-path read/write and
  working admitted paths. This is confinement of two named resource classes,
  not an OS sandbox: no seccomp, no cgroup, no other namespace, and admitted
  paths stay fully reachable. The default remains `unenforced-legacy`.
  Evidence: `docs/reports/remaining-p03/README.md`.
- ChunkIO counter through the process boundary (P04): the unsigned
  unit-counter contract now runs as package `qcore.chunkio` 0.5.0, computing
  its own typed path projection with no derived result in the request; the
  core evaluates the pinned contract on its sealed import and only then
  requires agreement, so agreement alone discharges nothing. Byte-identical
  certificate parity with the retained in-process route on all four helpers;
  missing-evidence, tampered and 25 mutation/refusal cases fail closed.
  `qcore-schemas` is 4.0.0 (three source-incompatible additions; existing wire
  bytes unchanged and pinned). Thirteen other in-process ChunkIO capabilities
  keep named blockers. Evidence: `docs/reports/remaining-p04/README.md`.
- Source mutex/guard lowering (T03): `source.mutex.guard.v1` lowers a lexical
  `with lock m as g` guard onto the unedited `CheckedMutexInvariantV1`
  checker, with C11 emission and independent reference execution compared
  line-for-line under four fixed ASan/UBSan/LSan runs. Twenty-nine tests pin
  exact diagnostic vectors across Q610–Q622 with Q600/Q013 refusals; guard
  binders stay body-local and no diagnostic code was added or changed. Fourteen
  new public Lean theorems; the Lean event model still lacks a
  resource-destruction constructor, so that half stays Rust-tested. CT26,
  CT27, CT29 and CT30 mappings are proposed with measured stages, but the
  manifest is unchanged pending harness fixture integration: CT stays 11/50.
  Evidence: `docs/reports/remaining-t03/README.md`.
- Recursive-summary design (C03): neutral-frame recursion is specified as the
  next W4 family — a two-point per-function lattice with monotone descent to
  the greatest post-fixed point, an empty alias obligation and a ten-item
  post-fixed-point validation contract — motivated by a 309-file census
  (25 of 45 self-recursive functions in scope are effect-neutral; the
  implemented routes reach at most 3). A demonstrated counterexample shows
  validation-by-reanalysis alone cannot refuse a circular effectful candidate,
  so any effectful route keeps a rank. Design only: no production file
  changed, and heap-link chain teardown is rejected with recorded ASan
  hazards. Evidence: `docs/reports/remaining-c03/README.md` and
  `docs/design/W4_RECURSIVE_SUMMARY_PROFILES.md`.

Nothing is promoted by this increment: raw Cycle 19 units stay 2 Safe /
42 provisional Unsafe / 137 Unknown plus four data-only units, CT coverage
stays 11/50 mapped, the contract corpus stays 59 modules and full language
conformance stays false. The four pinned 185-unit artifacts were not
re-measured (CORPUS-REPIN remains open). Merged gate at this checkpoint:
1080 workspace tests, 51 provider tests, 43 Python tests, 21 Lean audits,
both reevaluation examples exit 0, fmt and clippy clean. A pre-existing P02
`RLIMIT_NPROC` conformance test is load-flaky (it counts every process of the
real UID); recorded in `docs/reports/remaining-p03/README.md` §9.1 with the
assertion deliberately not weakened.

#### Second delegated increment (2026-09-08)

*Moved verbatim from `AGENTS.md` lines 1248-1349 — the seven-card wave and its `Nothing is promoted` closing paragraph; byte-identical in CLAUDE.md 975-1076.*

Second delegated increment (2026-09-08): seven bounded cards landed through a
second delegated wave (five isolated worktrees plus two lanes coordinated in
the integration worktree under explicit per-lane file ownership); this
paragraph supersedes nothing above and each card's record keeps its own
boundaries.
- Scoped read loans (T04): `ScopedReadLoansV1` extends the checked finite task
  profile with one parent-owned Buffer lent to at most one scoped child as a
  read-only loan closed by creator join. The ledger adds a `Loaned` owner
  state with shared-read versus exclusive-use gates; a loan is not a value
  (refused as result, entry parameter, copy/move/re-loan source and drop
  operand). Failed creation restores captures; failed join reclaims nothing.
  `borrow<Buffer>` parameters and `read PLACE share FULL` captures lower with
  no parser change; native and reference runs pass the same pointer and
  differ only in what the caller keeps, under fixed ASan/UBSan/LSan runs.
  Seventeen public Lean theorems in a new `QCore.ReadLoan` namespace; 18 new
  tests. ThreadSanitizer does not intercept C11 `thrd_create` (measured
  SEGV), so data-race freedom rests on the checker's no-write-while-loaned
  rule, not on measurement. CT24/CT25 mappings are proposed in the report
  only; Buffer writes, fractional shares (T05), write loans and nested
  workers remain refused. Evidence: `docs/reports/remaining-t04/README.md`.
- Reactor callback capability through the process boundary (P05): the reactor
  callback assignment inventory (registration index, event identity and
  generation, declared callback authority) now runs as package
  `qcore.libevent` 0.2.0, computing its proposal from a transported program
  input that carries no derived result; the sealed core import rebinds the
  proposal to its own program and re-runs `check_event_program` itself, so
  agreement alone discharges nothing. Bufferevent operations, raw socket
  custody, ordering assumptions and loop control have no wire record and stay
  Unknown; an empty accepted-build list is `UnsupportedBackend`, never
  acceptance. `qcore-schemas` is 5.0.0 (two forced source-incompatible
  additions; existing wire bytes pinned byte-identical). The moved capability
  has no production caller yet — it is exercised by the Cycle 18 unit tests,
  not the Cycle 19 audit — and the in-process route is retained (P09).
  Evidence: `docs/reports/remaining-p05/README.md`.
- CTraces capability definition (P06): design only, by measurement — a census
  of all 273 function definitions in the 17 pinned CTraces units finds zero
  functions any implemented checker accepts, and the `ctraces` profile entry
  imports and asserts nothing. `ctraces.span.dropped_counts.v1` is specified
  over the four dropped-count setters with a 19-row malicious-witness matrix
  and six negative controls; four refusal tests drive the real importer over
  real upstream bodies and panic if any CTraces function ever yields a
  summary. Two dangling manifest identities (an unimplemented
  `certificate_checker` name and an `evidence_versions` mismatch) are
  recorded as follow-on P06-I4, not repaired. The contract text stays outside
  `contract/qcore/`, so the corpus count is untouched. Evidence:
  `docs/reports/remaining-p06/README.md`.
- librdkafka capability definition (P07): design only, by measurement — the
  adapter imports nothing, and its six declared-but-unloaded mechanism
  operations are mostly unfaithful to the real source (a function-like macro,
  an absent symbol, a wrong arity, a void return taking the entry as input,
  and one unsound postcondition); these are defects in declared unloaded
  contract text, not upstream defect claims, and nothing derived from them.
  A 96-function census finds zero admissible functions with one single-reason
  near miss sizing follow-on P07-U1.
  `qcore.librdkafka.queue_final_destroy.v1` is specified with two disjoint
  outcomes and seven premises; reclamation, callbacks and concurrency are
  excluded, not assumed. Evidence: `docs/reports/remaining-p07/README.md`.
- Constructor-to-caller custody (O04): on byte-verbatim Fluent Bit `90a0109`
  bodies, the three modeled failure arms of `flb_input_chunk_create` release
  the chunk produced at `cio_chunk_open` exactly once, all naming the same
  acquisition site; the constructor's four guard/allocation-failure exits are
  discharged while its rollback and success exits stay Unknown, so its
  declared contract remains a named premise, not a fact its body discharges.
  The C02-P borrowed-actual admission module records per-exit residuals;
  37 Lean theorems in `QCore.ExitResiduals` include both fail-closed
  directions. The canonical CMetrics audit is byte-identical before and
  after; both functions stay Unknown, and one C03 pin was deliberately
  updated (its named blocker moved to the branch condition, recorded in the
  test and report). Evidence: `docs/reports/remaining-o04/README.md`.
- Checker/model correspondence (F01): `check_concurrent` and
  `QCore.Concurrency.Lifetime.step`/`run` are tied by a seven-conjunct
  representation relation checked without instrumenting the checker, with
  domain and transcription agreement pinned as digests both sides compute
  independently; the whole depth-3 domain is decided in Lean and depths 3-4
  run differentially in Rust, with no divergence found in the claimed
  direction. Eight new theorems depend only on `propext` and `Quot.sound`.
  The claim is bounded at trace length 4 and not inductive; the transcription
  is digest-checked, not mechanically extracted; data races, unauthorized
  access and permission overflow are outside every stated correspondence; the
  `CheckedMutexInvariantV1` pair remains open. Evidence:
  `docs/reports/remaining-f01/README.md`.
- T03 CT integration: CT26, CT27, CT29 and CT30 are mapped with stage-derived
  evidence and exact ordered diagnostic cascades; CT coverage moves from
  11/50 to 15/50 mapped. CT26 is the first harness-gated sanitized-native CT
  case. CT28 and CT31 stay unmapped with recorded reasons; `build_matrix.py`
  is recorded out of date via a `generator_status` field rather than silently
  regenerated, and a stale T00 blocker label is corrected in both acceptance
  records. No semantic or diagnostic-code change. Evidence:
  `docs/reports/remaining-t03-ct-integration/README.md`.

Nothing is promoted by this increment: raw Cycle 19 units stay 2 Safe /
42 provisional Unsafe / 137 Unknown plus four data-only units, the contract
corpus stays 59 modules and full language conformance stays false. CT
coverage advances to 15/50 mapped, by executed-stage evidence only. The four
pinned 185-unit artifacts were not re-measured (CORPUS-REPIN remains open and
now names C02, G02 and O04). `qcore-schemas` is 5.0.0. Merged gate at this
checkpoint: 1141 workspace tests, 56 provider tests, 43 Python tests, 23 Lean
audits, both reevaluation examples exit 0, fmt and clippy clean in both
workspaces. Process note: the shared git stash stack caused one
cross-worktree stash consumption during the wave; the affected work was fully
recovered from dangling commits and nothing was lost. Prefer temporary WIP
commits over `git stash` in this repository.

<!-- migrated-section:end -->

# O03 — Specify one real OS failure post-state profile — 2026-09-08

A **bounded design** card, closed by a Rust implementation and a Lean
formalization of the specified relation.

**Closing evidence.**

| Deliverable | Contents |
|---|---|
| [`docs/design/OS_FAILURE_POSTSTATE_PROFILE.md`](../../design/OS_FAILURE_POSTSTATE_PROFILE.md) | the specification: the state relation, its primary sources, premises, refusal boundaries and follow-on units |
| [`crates/qcore-core/src/os_endpoint.rs`](../../../crates/qcore-core/src/os_endpoint.rs) | the relation **decided in Rust** — five-state descriptor taxonomy, three-identity separation, target-qualified admission, conservative residual duties, join and claim refusal — with **39 tests** |
| [`proof/QCore/OsEndpointPostState.lean`](../../../proof/QCore/OsEndpointPostState.lean) | the matching Lean model, **27 theorems**, six of them decided over the whole `3 × 64 × 10` domain |
| [`proof/AuditOsEndpointPostState.lean`](../../../proof/AuditOsEndpointPostState.lean) | the axiom audit — every theorem depends only on `propext` and `Quot.sound` |

**Supplementary evidence, not the closing evidence.**

| Artifact | Contents |
|---|---|
| [`scripts/o03_close_poststate_relation.py`](../../../scripts/o03_close_poststate_relation.py) | the first executable statement of the relation, and the **real-syscall probe** |
| [`scripts/tests/test_o03_close_poststate_relation.py`](../../../scripts/tests/test_o03_close_poststate_relation.py) | 38 Python tests, 12 of them refusals |
| [`logs/relation_and_probe.json`](logs/relation_and_probe.json) | the recorded relation table and probe output |

The Python harness is retained for one thing the Rust model cannot do: it
executes **real, unsuppressed `close`/`open`/`dup`/`fcntl` syscalls** (§4.3).
Its refusal matrix is superseded by the Rust tests and is kept only as a
cross-check; the relation itself is decided in Rust.

**Nothing is wired into a production audit route.** `crates/qcore-core` is a
dependency-free L1 kernel, and no analyzer, driver, contract or audit path calls
`close_poststate`. Consuming it needs relational branch analysis, recorded as
follow-on unit **U3**. `qcore-schemas` is untouched, no `.qcore` module,
contract count, diagnostic code, pinned expectation or CT mapping moved, and
nothing is promoted.

## 1. What was selected, and why

> **`int close(int fd)` on `x86_64-unknown-linux-gnu`** — any Linux ABI target;
> the behaviour specified is the kernel's, not the C library's.

`close` is where every teardown path in the Cycle 19 corpus ends — `cio_file_close`,
`cio_file_down`, ChunkIO's native wrapper, and libevent's `evutil_closesocket`,
which on every non-Windows target *is* `close(sock)` (verified at the pinned
submodule `d82464a2`, `third_party/libevent/evutil.c:614-621`).

It was chosen over `munmap(2)` for three reasons, all recorded in the design
document §1 and §11: it has a documented failure post-state at all; that
post-state is **target-divergent**, which forces the target qualification the
verifier host's `linux-rlimit-v1`/`linux-confined-v1` profiles already use; and
its failure does **not** mean "nothing happened", so any relation that reads
`-1` as a no-op is wrong in the direction that licenses a documented hazard.

## 2. The post-state taxonomy

Three identities are separated (design §3), because one liveness bit cannot say
"the descriptor is gone **and** nothing was discharged":

| Identity | Ended by |
|---|---|
| **D** descriptor slot `(process, number, generation)` | `close`, per the relation below |
| **F** open file description | release of its **last** reference — a `dup`, a forked duplicate or an in-flight syscall in another thread all count |
| **W** deferred write-back duty | `fsync(2)` only — **never** `close` |

Five descriptor post-states, all reachable and all pinned by a test:

| State | Duties | Recovery on that number |
|---|---|---|
| **NotAttempted** | release; stays in the denominator | `close` |
| **Attempted** — an *analysis* gap (result or errno unobserved) | the poisoned set | **none** |
| **Retained** | release; stays in the denominator | `close` |
| **Released** | none for D | none — the generation is over |
| **Indeterminate** — a *documentation* gap | the poisoned set | **none** |

`Attempted` and `Indeterminate` carry identical duties and differ in whether a
better analysis could resolve them. That distinction keeps a solvable
imprecision from being filed as a platform fact.

The relation for the selected target (design §7.1):

| Outcome | D | F | W |
|---|---|---|---|
| not attempted | NotAttempted | Retained | Unknown |
| result unobserved | **Attempted** | Unknown | Unknown |
| `0` | **Released** | Released iff last reference | **Unknown** — never Discharged |
| `-1` EBADF | **Indeterminate** + premise violation | Unknown | Unknown |
| `-1` EINTR | **Released** | Unknown | **Failed** |
| `-1` EIO/ENOSPC/EDQUOT | **Released** | Released iff last reference | **Failed** |
| `-1` other observed errno | **Released** | Unknown | Unknown |
| `-1` errno unobserved | **Attempted** | Unknown | Unknown |

## 3. Documentation and observation are kept apart

Every post-state claim is attributed to a primary source — principally Linux
man-pages **6.17** `close(2)`, dated **2026-02-08** (`manpages-dev 6.17-1`,
regenerable with `man 2 close`), plus `open(2)`, `dup(2)`, `mmap(2)` and the
POSIX.1-2008 / POSIX.1-2024 EINTR clauses as that page reports them.

**The divergence runs the opposite way from the familiar story.** POSIX.1-2008
left the descriptor's state unspecified after EINTR; **POSIX.1-2024 standardized
the HP-UX behaviour**, in which the descriptor stays open and `close` must be
called again, making Linux documented-non-conforming with no plans to change.
So the two targets prescribe **opposite** recovery for the same errno: one
*requires* the retry the other documents as a hazard. That is the direct reason
an unpinned target yields `Indeterminate` with **no permitted recovery at all**,
and the direct reason no portable close contract is published.

**No observation is promoted to an API guarantee.** The Cycle 19
[descriptor-failure](../cycle19-file-descriptor-failures/README.md) and
[reopen-failure](../cycle19-reopen-failures/README.md) packages suppress a
selected syscall and return a fixed error *without reaching the kernel*. Their
statements are sound about the wrapper — and the wrapper necessarily leaves the
descriptor open, which is the **POSIX.1-2024** post-state, not the Linux one. A
test (`SuppressedFixtureContrastTest`) pins that disagreement so the two can
never be quietly conflated. This is not a defect claim against those packages;
their result boundaries already scope them to recorded executions, and
contract spec §C10 already forbids deriving universal OS behaviour from a
suppressing wrapper.

## 4. Evidence

### 4.0 The Rust model — `crates/qcore-core/src/os_endpoint.rs`

`close_poststate(target, premises, outcome)` decides the relation. It carries
the full vocabulary of the specification: `Target` (Linux / POSIX.1-2024 /
unpinned), the five-valued `DescriptorState`, `DescriptionState`,
`WriteBackState`, the seven `Duty` variants, `Recovery`, the seven named
`Premise`s, `Epoch` with an explicit generation, `join`, and `check_claim`.
Domain errors are `Result`, not panics: the not-attempted arm on a dead epoch is
`RelationError::NoEpochToDescribe`, and a cross-target join is
`RelationError::MismatchedTargets`.

**39 tests**, of which twelve are refusals of forged post-states and six are
exhaustive over the whole `3 × 64 × 10` domain the relation accepts:

| Group | Tests |
|---|---|
| taxonomy (acceptance 1) | all five states reachable; not-attempted ≠ attempted; attempted and indeterminate share duties but not identity |
| the Linux arms | success releases; EINTR releases; all three deferred-IO errnos release and fail the data duty; EBADF releases nothing and records the A2 violation; unobserved errno cannot conclude; denied record locks poison the whole relation |
| **domain-exhaustive invariants** | no outcome anywhere discharges the data duty · no poisoned state anywhere permits a retry · a released epoch permits nothing on its own generation · only a live epoch may be closed · the description is released only with the last reference · an empty claim is admitted and proves nothing |
| target divergence | POSIX.1-2024 retains on EINTR; the two targets prescribe opposite recovery; an unpinned target is indeterminate and permits no retry |
| **twelve forged post-states** | §4.1 |
| epoch identity (acceptance 2) | a reused number is a distinct epoch |
| joins | disagreement widens to indeterminate with no recovery and no release duty; agreement is preserved exactly; a join never invents a recovery, checked pairwise over the whole Linux domain; a cross-target join is refused |
| the card's distinction | the suppressed-wrapper post-state is not the Linux post-state |

### 4.0.1 The Lean model — `proof/QCore/OsEndpointPostState.lean`

**27 theorems**, printed by `AuditOsEndpointPostState.lean`, every one depending
only on `propext` and `Quot.sound`. No new axiom, no `sorry`, no placeholder.

Six of them are decided over the entire domain and then lifted to genuine
universally quantified statements — `mem_allTargets` / `mem_allOutcomes` /
`mem_allPremises` prove the enumerations complete, and `of_domainAll` bridges a
decided `Bool` check to `∀ t p o`:

* `no_outcome_ever_discharges_the_data_duty`
* `a_poisoned_state_permits_no_recovery` (acceptance item 3)
* `a_poisoned_state_carries_the_conservative_residual` — a poisoned epoch is
  never *also* owed a plain release, because that would contradict the no-retry
  duty
* `only_a_live_epoch_may_be_closed`
* `the_description_is_released_only_with_the_last_reference`
* `denied_record_locks_are_never_released`

Plus the determined arms (`linux_eintr_releases_the_descriptor`,
`posix2024_eintr_retains_the_descriptor`,
`the_targets_prescribe_opposite_recovery_on_eintr`,
`an_unpinned_eintr_is_indeterminate_and_permits_no_retry`,
`linux_deferred_io_releases_and_fails_the_data_duty`, `ebadf_never_releases`),
the premises a release requires (`a_release_requires_an_observed_result`,
`a_failed_close_releases_only_with_an_observed_errno`,
`success_without_sole_custody_never_releases`,
`the_not_attempted_arm_requires_a_live_epoch`), non-vacuity
(`every_descriptor_state_is_reachable`,
`attempted_and_indeterminate_differ_only_in_provenance`), the three join
theorems, `a_reused_number_is_a_distinct_epoch`, and
`the_suppressed_wrapper_post_state_is_not_the_linux_post_state`.

**Coexistence with `QCore.DescriptorCustody`.** That module's
`accepted_failed_close_retains_custody` states the "returns on failure" policy
over its own record type and is **left exactly as it is** — it is a pinned
artifact, and reconciling it is follow-on unit **U4** (§5). The new module states
the corrected, target-qualified relation *alongside* it, in its own namespace,
over its own types; the two do not import or contradict each other as Lean
developments, and their disagreement is a modelling disagreement recorded in
prose, not a proof conflict. Both are audited and both pass.

### 4.1 Refusals — 12 forged post-states, all rejected

EINTR retains on Linux · EINTR releases on POSIX.1-2024 · a retry is permitted
after a failed Linux close · a retry is permitted from `Indeterminate` · EBADF
released the epoch · an unobserved result released the epoch · success
discharged the data duty · a failed close discharged the data duty · the
description died without the last reference · success identified the epoch
without sole custody · a poisoned epoch left the denominator · **an `Attempted`
epoch claimed as `Retained`**. A thirteenth case admits the **true** post-state,
so the refusals are not vacuous.

The Python matrix's twelfth refusal was an *unknown post-state field*, which is
a class the Rust vocabulary makes unrepresentable — a claim naming a field or a
state that does not exist fails to compile. The Rust port substitutes the
equivalently motivated run-time confusion: treating the analysis gap as a live
epoch, which is precisely the mistake that would license a retry. The
substitution is recorded in the test's own doc comment.

### 4.2 Probe — real, unsuppressed syscalls (supplementary)

Four probes run real `open`/`close`/`dup`/`fcntl` on Linux `7.0.0-30-generic`,
`x86_64`, glibc 2.43, in an exclusive temporary directory:

| Probe | Measured |
|---|---|
| **P1** | after `close` returns `0`, `fcntl(n, F_GETFD)` gives EBADF — the slot really is released |
| **P2** | a second `close` on the same number really returns `-1`/EBADF and ends no epoch |
| **P3** | the number (fd `3`) came **straight back** from the next `open`, bound to a **different inode** — one number, two distinct epochs |
| **P4** | after `dup` then closing the original, the original number is EBADF while the duplicate still reads the file — **F outlived D** |

**P3 is the measured justification for the no-retry rule** (acceptance item 3).
The hazard is not hypothetical: the freed number is recycled immediately, so a
retried `close` would close the *new* epoch — and would return `0`, not EBADF.
P4 is the measured justification for separating D from F.

### 4.3 What is not measured, and said so in the artifact

The probe cannot observe EINTR (needs an interrupting signal on an interruptible
filesystem) or EIO/ENOSPC/EDQUOT (needs a failing device, a full filesystem or an
exhausted quota). **Those two arms rest on the man page alone.** The probe's own
JSON records them under `not_observed`, and a test asserts that no observation
mentions EINTR. Binding them is follow-on unit **U7**.

## 5. Two existing declarations the relation contradicts

Both are **recorded, not repaired** — they are pinned artifacts and repairing
either is an implementation change outside a bounded design card. Neither is an
upstream defect claim; both are defects in *this repository's declared model
text*, in the sense P07 used.

| # | Site | Problem |
|---|---|---|
| **U4** | `proof/QCore/DescriptorCustody.lean` — `Close.Valid`, theorem `accepted_failed_close_retains_custody` | hard-codes `succeeded = false → liveAfter = true`, i.e. §C10's "Returns on failure". On the selected target that (a) **rejects a truthful record** of a real failed close, and (b) models the epoch as still usable, so a later use or retry on that number is not refused. The root cause is representational: one `liveAfter : Bool` cannot express "gone **and** nothing discharged". |
| **U5** | `contract/qcore/monkey/composition.qcore` — `evutil_closesocket`'s `ensures error(SocketCloseError) descriptor_custody_is_preserved` | on Linux this function *is* `close(sock)`, so the error postcondition asserts custody preservation for an operation that releases on failure, and thereby prescribes the forbidden retry. `monkey/descriptor_observation.qcore` names this file as the home of "the general conditional descriptor model", so it is where a reader would look. |

**Nothing is derived from either today.** The `.qcore` clause carries no
`requires resource_contract_v1` marker, so the resource loader never loads it;
its 4 clauses are pinned by `compiler/tests/qcore_contract_corpus.rs`; and
neither `descriptor_custody_is_preserved` nor `SocketCloseError` appears in any
Rust identifier under `crates/` or `compiler/` (verified). The Lean predicate is
a validity check on submitted records, and its conservative intent — refusing to
credit a failed close as a discharge — is correct as far as it goes.

## 6. Follow-on units

| # | Unit |
|---|---|
| **U1** | `EndpointOperationContract` schema record — **specified in the design doc, deliberately not added**. It would be a source-incompatible addition, moving `qcore-schemas` to **6.0.0**. |
| **U2** | `requires os_endpoint_contract_v1` loader vocabulary, total-or-reject, never reinterpreting v1 |
| **U3** | gateway transfer function. **Blocked on relational branch analysis**: the relation is result-dependent, and both C02 §7 and O04 §7 leave branch conditions out of scope. Until then every real call site joins to `Attempted` — still sound, still fail-closed, and still strictly stronger than a v1 `resource_released` declaration, which would claim an unconditional discharge. |
| **U4** | correct `DescriptorCustody.lean`'s `Close.Valid` to a target-qualified relation. `QCore.OsEndpointPostState` now models the corrected relation **alongside** it (§4.0.1); the pinned theorem is untouched, and unifying the two record types is the remaining work. |
| **U5** | re-examine `composition.qcore`'s `evutil_closesocket` error postcondition (changes pinned corpus tables) |
| **U6** | a `munmap` sibling profile — a **different** relation, not a parameter of this one: its identity is a range not an epoch, its documented failure genuinely *is* "Returns on failure", and its failure sentinel is `MAP_FAILED` on the acquisition side |
| **U7** | EINTR and ENOSPC fixtures to bind the two unmeasured arms |

## 7. Acceptance items

| # | Item | Where |
|---|---|---|
| 1 | distinguish not-attempted, attempted, retained, released and indeterminate/poisoned post-states as justified by the API | design §6.1; Rust `DescriptorState` with `all_five_descriptor_states_are_reachable` (over the whole domain), `not_attempted_is_distinct_from_attempted` and `attempted_and_indeterminate_share_duties_but_not_identity`; Lean `every_descriptor_state_is_reachable` and `attempted_and_indeterminate_differ_only_in_provenance` |
| 2 | sentinels and descriptor generations explicit; v1 nullable-pointer meanings unchanged | design §8: the success test is `ret == 0`, spelled as `Outcome::Zero` rather than a nullability test; `Epoch { kind, number, generation }` with the §C10 no-merge rule pinned by Rust `a_reused_number_is_a_distinct_epoch` and Lean `a_reused_number_is_a_distinct_epoch`, **and measured by probe P3**; `qcore-schemas` and every v1 mode are untouched, and an audit records that no raw descriptor syscall is declared with v1 `resource_released` today |
| 3 | Unknown outcomes retain conservative residual duties; no automatic retry prescribed for an indeterminate reused descriptor | design §6.1 and §7.3. Proved over the **whole domain** in both languages: Rust `no_poisoned_state_anywhere_permits_a_retry` and `only_a_live_epoch_may_be_closed`; Lean `a_poisoned_state_permits_no_recovery`, `a_poisoned_state_carries_the_conservative_residual` and `only_a_live_epoch_may_be_closed`. Two Rust refusal tests reject a claimed retry, and probe P3 measures the number recycling that makes the retry dangerous |

Out of scope, as the card requires: no universal close/mmap failure policy, no
OS durability theorem, no implicit resource-v1 extension.

## 8. Verdicts carried forward unchanged

| Item | Value | Changed by O03 |
|---|---|---|
| Raw Cycle 19 units | 2 Safe / 42 provisional Unsafe / 137 Unknown + 4 data-only | no |
| Pinned 185-unit artifacts | untouched (CORPUS-REPIN remains open) | no |
| CT coverage | 15/50 mapped | no |
| Contract corpus | 59 modules | no |
| `qcore-schemas` | 5.0.0 | no |
| Full language conformance | false | no |

## 9. Gate results

Run in this card's isolated worktree, on branch
`worktree-agent-a7beea0ea8b1db9ae` based at `c60fd91` (the second delegated
increment's consolidation commit), with the Libevent submodule initialized.

The Lean toolchain is at `~/.elan/bin` and is not on the default `PATH`; the two
Lean gates were run with `PATH="$HOME/.elan/bin:$PATH"`. Lean 4.33.1,
Lake 5.0.0.

| Gate | Command | Observed | Exit |
|---|---|---|---|
| Workspace tests | `cargo test --workspace` | **1,180 passed / 0 failed / 1 ignored** (1,141 → 1,180, **+39**) | 0 |
| Format | `cargo fmt --all -- --check` | clean | 0 |
| Clippy | `cargo clippy --all-targets -- -D warnings` | clean | 0 |
| Probe | `cargo run -p qcore-compiler --example reevaluation_probe` | ran | 0 |
| Follow-up | `cargo run -p qcore-compiler --example reevaluation_followup` | ran | 0 |
| Python | `python3 -B -m unittest discover -s scripts/tests` | **81 ran, OK** (43 pre-existing + 38 supplementary) | 0 |
| **Lean build** | `cd proof && lake build` | **68 jobs, completed**; `QCore.OsEndpointPostState` built in 5.1 s | 0 |
| **Lean audits** | `python3 -B scripts/check_lean_audits.py` | **24 audits passed** (23 → 24) | 0 |
| Lean axiom surface | `lake env lean -DwarningAsError=true AuditOsEndpointPostState.lean` | all **27** theorems depend only on `propext` and `Quot.sound`; no new axiom, no `sorry`, no placeholder | 0 |
| Provider tests | `cargo test --locked --offline --manifest-path providers/Cargo.toml --workspace` | **56 passed / 0 failed** | 0 |

**One flaky failure, recorded rather than hidden.** The first full
`cargo test --workspace` run failed one pre-existing P03 confinement test,
`qcore-verifier-host` →
`a_confined_fixture_cannot_write_outside_the_admitted_paths`, with
`Broken pipe (os error 32)` where `Terminated` was expected. It passes on
re-run, and both re-runs of the whole workspace are green. It touches no file
this card changes — the card's Rust lives in `crates/qcore-core`, which
`qcore-verifier-host` does not depend on for this fixture — and it is the same
class of environment-sensitive flake AGENTS.md already records for the P02
`RLIMIT_NPROC` conformance test. **No assertion was weakened.**

## 10. Files written

| Path | Change |
|---|---|
| `crates/qcore-core/src/os_endpoint.rs` | **new** — the relation decided in Rust, plus 39 tests |
| `crates/qcore-core/src/lib.rs` | module declaration and re-exports only |
| `proof/QCore/OsEndpointPostState.lean` | **new** — the Lean model, 27 theorems |
| `proof/AuditOsEndpointPostState.lean` | **new** — the axiom audit (23 → 24 audits) |
| `proof/QCore.lean` | one `import` line |
| `docs/design/OS_FAILURE_POSTSTATE_PROFILE.md` | the specification |
| `docs/reports/remaining-o03/README.md` | this record |
| `docs/reports/remaining-o03/logs/relation_and_probe.json` | recorded relation table and probe output |
| `scripts/o03_close_poststate_relation.py` | supplementary executable relation and real-syscall probe |
| `scripts/tests/test_o03_close_poststate_relation.py` | 38 supplementary tests |

`proof/QCore/DescriptorCustody.lean` is **untouched** (follow-on U4). Nothing
under `compiler/`, `contract/`, `crates/qcore-schemas/` or `providers/` was
modified, and no production audit route calls the new module. `AGENTS.md`,
`CLAUDE.md`, `docs/reevaluation/ct_manifest.json` and
`docs/reviews/r0_3/acceptance_matrix.json` were not edited; §11 proposes the
ledger text instead.

## 11. Proposed ledger text (for the integrator to consolidate)

> **One real OS failure post-state profile (O03):** design only. The failure
> post-state relation of Linux `close(2)` on `x86_64-unknown-linux-gnu` is
> specified in `docs/design/OS_FAILURE_POSTSTATE_PROFILE.md`, grounded in Linux
> man-pages 6.17 `close(2)` (2026-02-08) and the POSIX.1-2008/2024 EINTR
> clauses, and kept explicitly distinct from the project's suppressed-syscall
> observations. Three identities are separated — descriptor slot, open file
> description and deferred write-back duty — because one liveness bit cannot
> express "the descriptor is gone **and** nothing was discharged". Five
> descriptor post-states (not-attempted, attempted, retained, released,
> indeterminate) are derived, with `Attempted` an analysis gap and
> `Indeterminate` a documentation gap carrying identical conservative duties.
> On Linux a failed close **releases** the descriptor except on EBADF, so no
> post-state permits an automatic retry; on POSIX.1-2024 the descriptor is
> **retained** and the retry is required, so an unpinned target yields
> `Indeterminate` with no permitted recovery and no portable close contract is
> published. Success tests (`ret == 0`) and epoch generations are explicit and
> v1 nullable-pointer meanings are unchanged. The relation is **decided in
> Rust** in the dependency-free `crates/qcore-core/src/os_endpoint.rs`
> (39 tests, twelve of them refusals of forged post-states, six exhaustive over
> the whole `3 × 64 × 10` domain) and **formalized in Lean** as
> `QCore.OsEndpointPostState` (27 theorems in a new `AuditOsEndpointPostState`,
> depending only on `propext` and `Quot.sound`, with the no-retry and
> no-durability invariants proved over the whole domain). The new Lean module
> **coexists with** `QCore.DescriptorCustody`, whose
> `accepted_failed_close_retains_custody` is deliberately left untouched as
> follow-on U4. A supplementary Python harness runs four real unsuppressed
> syscall probes measuring descriptor release, EBADF, immediate number reuse
> with a distinct inode, and a description outliving its descriptor; the EINTR
> and EIO arms are unmeasured and rest on documentation alone. Two existing
> declarations are recorded as contradicting the relation and deliberately not
> repaired: that Lean theorem, and
> `contract/qcore/monkey/composition.qcore`'s `evutil_closesocket` error
> postcondition; nothing derives from either. Seven follow-on units are named,
> including a specified-but-unadded `EndpointOperationContract` schema record
> (which would move `qcore-schemas` to 6.0.0) and a gateway transfer function
> blocked on relational branch analysis. **No production audit route consumes
> the module** and nothing is promoted. Gate: 1,180 workspace tests, 24 Lean
> audits, 81 Python tests, 56 provider tests, both reevaluation examples exit 0,
> fmt and clippy clean. Evidence: `docs/reports/remaining-o03/README.md`.

Acceptance-matrix rows this card supports (O03): the endpoint post-state
specification, its Rust implementation and its Lean formalization. It supplies
**no** production-integration evidence class — U1–U3 remain open, and U4/U5 are
newly recorded open defects in declared model text.

## 12. Scope and non-claims

* **Not wired in.** `crates/qcore-core` is a dependency-free L1 kernel and no
  analyzer, driver, contract or audit route calls `close_poststate`. Consuming
  it is follow-on U3.
* **One operation, one target.** No universal close or `munmap` policy; §11 of
  the design records why `munmap` needs a different relation.
* **No durability result.** `W` is never discharged by any arm of the relation,
  on any target, and a test pins that.
* **No resource-v1 extension.** v1 modes, the loader, the 59-module corpus and
  every pinned clause count are unchanged.
* **The Lean theorems are about the model, not the kernel.** They prove that the
  *relation* is fail-closed — that it never discharges a data duty, never
  permits a retry from a poisoned state, never releases without an observed
  result. Whether the relation is *faithful* to Linux is a premise read from
  documentation [M], and whether a real call took a given arm is a premise
  supplied from outside. Nothing here verifies the kernel.
* **The Rust and Lean models are two statements of one relation, not a verified
  refinement.** They are written to agree and are checked separately; no
  extraction or mechanical correspondence ties them. Establishing that link is
  the same class of work F01 did for `check_concurrent`.
* **Not a proof of the arms themselves.** The Linux arms are read from
  documentation, not verified against the kernel. Two of them are unmeasured
  here. The probe's four observations bind one host and nothing beyond it.
* **A `Released` derivation is a claim about the descriptor slot only** — it
  says nothing about the open file description without the last-reference
  premise, and nothing about data durability ever.
* **No promotion, no verdict change, no diagnostic code added, removed or
  downgraded, no gate weakened, no pinned expectation edited.**

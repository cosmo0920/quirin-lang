# O03 — Specify one real OS failure post-state profile — 2026-09-08

A **bounded design** card. The deliverable is a reviewed specification plus a
small validation harness:

* [`docs/design/OS_FAILURE_POSTSTATE_PROFILE.md`](../../design/OS_FAILURE_POSTSTATE_PROFILE.md)
  — the state relation, its premises, its refusal boundaries and its follow-on
  implementation units;
* [`scripts/o03_close_poststate_relation.py`](../../../scripts/o03_close_poststate_relation.py)
  — the relation, stated once and executably, plus a real-syscall probe;
* [`scripts/tests/test_o03_close_poststate_relation.py`](../../../scripts/tests/test_o03_close_poststate_relation.py)
  — 38 tests, 12 of them refusals of forged post-states;
* [`logs/relation_and_probe.json`](logs/relation_and_probe.json) — the recorded
  relation table and probe output.

**No production code changed.** No crate imports the harness, `qcore-schemas` is
untouched, no `.qcore` module, contract count, diagnostic code, pinned
expectation, gate or CT mapping moved, and nothing is promoted.

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

### 4.1 Refusals — 12 forged post-states, all rejected

EINTR retains on Linux · EINTR releases on POSIX.1-2024 · a retry is permitted
after a failed Linux close · a retry is permitted from `Indeterminate` · EBADF
released the epoch · an unobserved result released the epoch · success
discharged the data duty · a failed close discharged the data duty · the
description died without the last reference · success identified the epoch
without sole custody · a poisoned epoch left the denominator · an unknown
post-state field. A thirteenth case admits the **true** post-state, so the
refusals are not vacuous.

### 4.2 Probe — real, unsuppressed syscalls

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
| **U4** | correct `DescriptorCustody.lean`'s `Close.Valid` to a target-qualified relation |
| **U5** | re-examine `composition.qcore`'s `evutil_closesocket` error postcondition (changes pinned corpus tables) |
| **U6** | a `munmap` sibling profile — a **different** relation, not a parameter of this one: its identity is a range not an epoch, its documented failure genuinely *is* "Returns on failure", and its failure sentinel is `MAP_FAILED` on the acquisition side |
| **U7** | EINTR and ENOSPC fixtures to bind the two unmeasured arms |

## 7. Acceptance items

| # | Item | Where |
|---|---|---|
| 1 | distinguish not-attempted, attempted, retained, released and indeterminate/poisoned post-states as justified by the API | design §6.1; all five derived and pinned by `test_all_five_descriptor_states_are_reachable`, with not-attempted/attempted and attempted/indeterminate separated by their own tests |
| 2 | sentinels and descriptor generations explicit; v1 nullable-pointer meanings unchanged | design §8: success test is `ret == 0`, spelled; epochs are `(kind, number, generation)` with the §C10 no-merge rule pinned by a test **and measured by probe P3**; v1 modes untouched, and an audit records that no raw descriptor syscall is declared with v1 `resource_released` today |
| 3 | Unknown outcomes retain conservative residual duties; no automatic retry prescribed for an indeterminate reused descriptor | design §6.1 and §7.3: the poisoned duty set forbids **both** use and retry and keeps the endpoint in the live denominator; `permitted_recovery` is empty in every poisoned state; two tests refuse a claimed retry, and probe P3 measures the recycling that makes the retry dangerous |

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

| Gate | Command | Observed | Exit |
|---|---|---|---|
| Workspace tests | `cargo test --workspace` | **1,141 passed / 0 failed / 1 ignored** | 0 |
| Format | `cargo fmt --all -- --check` | clean | 0 |
| Clippy | `cargo clippy --all-targets -- -D warnings` | clean | 0 |
| Probe | `cargo run -p qcore-compiler --example reevaluation_probe` | ran | 0 |
| Follow-up | `cargo run -p qcore-compiler --example reevaluation_followup` | ran | 0 |
| Python | `python3 -B -m unittest discover -s scripts/tests` | **81 ran, OK** (43 pre-existing + **38 new**) | 0 |
| Lean build | `cd proof && lake build` | **not run** | — |
| Lean audits | `python3 -B scripts/check_lean_audits.py` | **not run** | — |

**The Lean gate was not run: `lake` and `lean` are not installed on this host.**
That is why this card adds **no Lean material** — writing a `QCore` module that
could not be compiled, axiom-audited or checked against
`-DwarningAsError=true` would put an unverified file in front of the
integrator's gate. Formalizing the relation is the natural companion to U4 and
should land with it, on a host with the toolchain. No existing Lean file was
touched, so the 23 audits are unaffected by construction.

The workspace test count is unchanged from the branch baseline: this card adds
no Rust test. The only moving number is the Python suite, 43 → 81.

## 10. Files written

| Path | Change |
|---|---|
| `docs/design/OS_FAILURE_POSTSTATE_PROFILE.md` | new — the specification |
| `docs/reports/remaining-o03/README.md` | new — this record |
| `docs/reports/remaining-o03/logs/relation_and_probe.json` | new — recorded relation table and probe output |
| `scripts/o03_close_poststate_relation.py` | new — the executable relation and probe |
| `scripts/tests/test_o03_close_poststate_relation.py` | new — 38 tests |

Nothing under `crates/`, `compiler/`, `contract/`, `proof/` or `providers/` was
modified. `AGENTS.md`, `CLAUDE.md`, `docs/reevaluation/ct_manifest.json` and
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
> v1 nullable-pointer meanings are unchanged. A 38-test harness refuses 12
> forged post-states and four real unsuppressed syscall probes measure
> descriptor release, EBADF, immediate number reuse with a distinct inode, and a
> description outliving its descriptor; EINTR and EIO arms are unmeasured and
> rest on documentation alone. Two existing declarations are recorded as
> contradicting the relation and deliberately **not repaired**:
> `proof/QCore/DescriptorCustody.lean`'s `accepted_failed_close_retains_custody`
> and `contract/qcore/monkey/composition.qcore`'s `evutil_closesocket` error
> postcondition; nothing derives from either. Seven follow-on units are named,
> including a specified-but-unadded `EndpointOperationContract` schema record
> (which would move `qcore-schemas` to 6.0.0) and a gateway transfer function
> blocked on relational branch analysis. No production code changed, nothing is
> promoted, and the Lean gate was not run because `lake` is absent on this host.
> Evidence: `docs/reports/remaining-o03/README.md`.

Acceptance-matrix rows this card supports (O03): the endpoint post-state
specification and its harness evidence. It supplies **no** implementation
evidence class — U1–U3 remain open, and U4/U5 are newly recorded open defects in
declared model text.

## 12. Scope and non-claims

* **Design only.** Nothing is wired into a production audit route, and the
  harness is not importable by any crate.
* **One operation, one target.** No universal close or `munmap` policy; §11 of
  the design records why `munmap` needs a different relation.
* **No durability result.** `W` is never discharged by any arm of the relation,
  on any target, and a test pins that.
* **No resource-v1 extension.** v1 modes, the loader, the 59-module corpus and
  every pinned clause count are unchanged.
* **Not a proof.** The Linux arms are read from documentation, not verified
  against the kernel. Two of them are unmeasured here. The probe's four
  observations bind one host and nothing beyond it.
* **A `Released` derivation is a claim about the descriptor slot only** — it
  says nothing about the open file description without the last-reference
  premise, and nothing about data durability ever.
* **No promotion, no verdict change, no diagnostic code added, removed or
  downgraded, no gate weakened, no pinned expectation edited.**

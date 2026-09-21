# One real OS failure post-state profile: Linux `close(2)`

**Status:** specified, implemented as a model, and formalized. The relation is
decided in [`crates/qcore-core/src/os_endpoint.rs`](../../crates/qcore-core/src/os_endpoint.rs)
and formalized in
[`proof/QCore/OsEndpointPostState.lean`](../../proof/QCore/OsEndpointPostState.lean).
**Nothing is wired into a production audit route** — `qcore-core` is the
dependency-free L1 kernel and no analyzer, driver, contract or audit path calls
the relation; consuming it is follow-on unit U3 (§12). No verdict, contract
corpus count, CT mapping or raw Cycle 19 unit changes on its account. Card O03;
evidence record
[`docs/reports/remaining-o03/README.md`](../reports/remaining-o03/README.md).

This document specifies the failure post-state relation of **one** endpoint
operation on **one** target:

> `int close(int fd)` on **`x86_64-unknown-linux-gnu`** (any Linux ABI target;
> the behaviour is the kernel's, not the C library's).

It is deliberately not a close/unmap policy, not a durability result, and not an
extension of resource-contract v1. Contract spec
[`QCORE_CONTRACT_SPEC_r0_3.md`](../QCORE_CONTRACT_SPEC_r0_3.md) §C10 requires a
cleanup relation to choose one of four failure policies and to represent every
resulting obligation, and §C04.4 records that scalar failure sentinels such as
`-1` "are not representable as tracked endpoint acquisition outcomes by the
existing nullability-only return mode". This document is the first concrete
discharge of that requirement for a single operation.

The relation is **decided** by `qcore_core::os_endpoint::close_poststate`
(39 tests) and **formalized** as `QCore.OsEndpointPostState` (27 theorems,
audited by `proof/AuditOsEndpointPostState.lean`). A supplementary Python
harness,
[`scripts/o03_close_poststate_relation.py`](../../scripts/o03_close_poststate_relation.py)
with
[its tests](../../scripts/tests/test_o03_close_poststate_relation.py), keeps the
one thing the other two cannot do: it executes **real, unsuppressed syscalls**
(§9.2).

---

## 1. Why `close(2)`, and why Linux

`close` is the operation every teardown path in the Cycle 19 corpus terminates
in — `cio_file_close`, `cio_file_down`, `cio_file_unix.c`'s native wrapper, and
libevent's `evutil_closesocket`, which on every non-Windows target *is* `close`
(verified at the pinned submodule revision `d82464a2`,
`third_party/libevent/evutil.c:614-621`).

It is also the operation whose failure post-state is most widely assumed and
least often true. Three properties make it the right single subject:

1. **It has a documented failure post-state at all.** Most of the corpus's
   endpoint operations do not.
2. **That post-state is target-divergent, in a direction that changed
   recently.** Two conforming implementations are required to do *opposite*
   things after the same error. This forces the target qualification that the
   verifier host's `linux-rlimit-v1` and `linux-confined-v1` execution profiles
   already use, rather than allowing a portable-looking contract that is false
   somewhere.
3. **Its failure does not mean what "failure" means for `malloc`.** A failed
   `close` on Linux still releases the descriptor. Any relation that reads
   "returned `-1`" as "nothing happened" is wrong here, and wrong in the
   direction that permits a retry the platform documents as a hazard.

`munmap(2)` was the alternative candidate. §11 records why it needs a different
relation rather than a parameter of this one.

## 2. Primary sources

Every post-state claim below is attributed to one of these. Nothing is inferred
from this repository's fixtures; §10 states why that separation is the point of
the card.

| Ref | Source | Sections used |
|---|---|---|
| **[M]** | Linux man-pages **6.17**, `close(2)`, dated **2026-02-08** (Debian/Ubuntu `manpages-dev 6.17-1`; regenerate with `man 2 close`) | DESCRIPTION, RETURN VALUE, ERRORS, CAVEATS ("Multithreaded processes and close()", "Dealing with error returns from close()") |
| **[MO]** | Linux man-pages 6.17, `open(2)` | DESCRIPTION, NOTES "Open file descriptions" |
| **[MD]** | Linux man-pages 6.17, `dup(2)` | DESCRIPTION |
| **[MM]** | Linux man-pages 6.17, `mmap(2)`/`munmap(2)` | RETURN VALUE, ERRORS (§11 only) |
| **[P8]** | POSIX.1-2008, `close()`, as quoted by [M] | the EINTR clause |
| **[P24]** | POSIX.1-2024, `close()`, as reported by [M] | the EINTR clause |

[M]'s load-bearing statements, in its own words, quoted minimally:

* the Linux kernel "always releases the file descriptor early in the close
  operation", with the error-producing steps happening later;
* "Retrying the close() after a failure return is the wrong thing to do";
* other implementations similarly always close, "except in the case of EBADF";
* under [P8] the state of `fd` after EINTR "is unspecified";
* [P24] "standardized the behavior of HP-UX", in which the descriptor stays open
  and `close` must be called again — "making Linux and many other
  implementations non-conforming", with no plans to change Linux;
* "A successful close does not guarantee that the data has been successfully
  saved to disk".

**Note the direction of the divergence.** The familiar version of this story is
"POSIX leaves it unspecified, Linux always closes". As of [P24] the standard now
*requires* the descriptor to stay open on EINTR, and Linux is documented as
non-conforming and not changing. A relation written against "POSIX" and a
relation written against "Linux" therefore prescribe opposite recovery, and
neither is a safe default for the other. Whichever version of the standard is
in force, a *portable* close contract is unpublishable; see §7.

## 3. Three identities, not one

[M] DESCRIPTION and [MO] NOTES separate objects the project's current descriptor
model conflates. The relation tracks all three separately, and a v1-style
single liveness bit cannot express the result.

| Identity | What it is | Created by | Ended by |
|---|---|---|---|
| **D — descriptor slot** | the per-process table entry, identified by `(process, number, generation)`. Contract spec §C10: an endpoint is identified by kind, acquisition generation and lifetime, never by a number alone. | `open`, `dup`, `socket`, `accept`, … | `close` on Linux, per §6 |
| **F — open file description** | the system-wide open-file entry [MO]. Reference-counted: every duplicate descriptor from `dup(2)`/`fork(2)` [MD] and **every in-flight syscall holding it** [M CAVEATS] is a reference. | `open` (each `open` makes a new one [MO]) | the release of its *last* reference |
| **W — deferred write-back duty** | unflushed data attached to F. `close` may report an earlier `write`'s failure [M CAVEATS], and success does not mean the data reached the disk. | any buffered `write` | `fsync(2)` — **never** `close` |

Two consequences the relation encodes:

* **Descriptor release is not description release.** Closing one of two
  duplicates ends D and leaves F alive. Measured directly, probe P4 (§9).
* **Neither is durability.** No arm of the relation ever derives
  `WriteBackDischarged`; a test pins that over every target and outcome.

A fourth footprint is deliberately *refused* rather than modelled: [M]
DESCRIPTION records that closing removes every POSIX record lock the process
holds on the underlying file, regardless of which descriptor took the lock.
That effect escapes all three identities, so premise **A7** below must hold or
the relation answers `Indeterminate`.

## 4. Call-site premises

Each is a premise the relation *consumes*. None is established here; a consumer
must discharge or carry each one explicitly.

| # | Premise | Consumed by |
|---|---|---|
| **A1** | the target is pinned to Linux | every non-`Released`/`Retained` distinction in §6 vs §7 |
| **A2** | the analysis holds a live epoch `(n, g)` and `fd` evaluates to `n` for that epoch | every arm; refuted by EBADF |
| **A3** | no other thread or task may close or reuse `n` concurrently | the **success** arm — see below |
| **A4** | the `int` result is bound and comparable against `0` | the partition in §5 |
| **A5** | `errno` is read on the failure branch and not clobbered in between | separating EBADF from the release-anyway family |
| **A6** | this epoch is the last reference to its description: no live `dup`, no forked duplicate, no in-flight syscall in another thread | **only** the F-side conclusion |
| **A7** | the process holds no POSIX record locks on the underlying file | the whole relation (else `Indeterminate`) |

**A3 is load-bearing on the success arm, not decoration.** `close` returning `0`
proves that *some* epoch on number `n` was closed. Only sole custody identifies
that epoch with the one the analysis was tracking. Without A3 the relation
answers `Indeterminate` for a *successful* close — which is the correct answer,
because the dangerous double close does not report EBADF at all: if the number
had already been recycled, the second call closes an unrelated live epoch and
returns zero. Probe P3 (§9) measures the recycling that makes this reachable.

## 5. Outcome partition

Exactly one of these holds for one call. `Ω` is total over what a consumer can
observe, which is why "result unobserved" and "errno unobserved" are outcomes
rather than gaps.

| # | Outcome |
|---|---|
| **O0** | not attempted on this path |
| **O1** | attempted, result unobserved (discarded return, or an unresolvable call site) |
| **O2** | returned `0` |
| **O3** | returned `-1`, `errno == EBADF` |
| **O4** | returned `-1`, `errno == EINTR` |
| **O5** | returned `-1`, `errno ∈ {EIO, ENOSPC, EDQUOT}` — the deferred write-back family [M ERRORS] |
| **O6** | returned `-1`, some other observed `errno` |
| **O7** | returned `-1`, `errno` unobserved |

The success test is **`ret == 0`**, spelled explicitly. It is not `ret != -1`,
and it is not nullability; §8 records why that matters.

## 6. The taxonomy

### 6.1 Descriptor post-states

Five, all reachable; a test enumerates them and asserts the set is exactly this.

| State | Meaning | Duties | Recovery permitted on `n` |
|---|---|---|---|
| **NotAttempted** | no call was issued on this path; the epoch is untouched | `must_release_epoch`, `epoch_stays_in_live_denominator` | `close` |
| **Attempted** | a call was issued and the *analysis* cannot map the outcome to a terminal state (O1, O7) | the poisoned set | **none** |
| **Retained** | the epoch is known live after the call | `must_release_epoch`, `epoch_stays_in_live_denominator`, and any data duty | `close` |
| **Released** | the epoch ended; `n` may be rebound by a **fresh generation** | none for D | none — the generation is over |
| **Indeterminate** | the *documentation* does not determine the fate (O3, or a target-divergent arm) | the poisoned set | **none** |

**`Attempted` and `Indeterminate` carry identical duties and differ in why.**
`Attempted` is an analysis gap: reading the result and `errno`, or resolving the
call site, resolves it. `Indeterminate` is a documentation gap: no amount of
better analysis resolves it, only pinning the target or an external oracle. The
distinction is not cosmetic — it is exactly the difference between work a
follow-on unit can do and work it cannot, and it keeps a solvable imprecision
from being filed as a platform fact.

The **poisoned duty set** is the conservative residual, and it forbids both
directions at once:

```
must_not_use_number            no read/write/fcntl/dup through n
must_not_retry_close           no close(n) — see §7
must_not_treat_number_as_free  n is not available for a fresh epoch's identity
epoch_stays_in_live_denominator  the endpoint stays counted as unresolved
must_report_unresolved_endpoint  the enclosing result is Unknown for descriptor custody
```

Nothing discharges a poisoned epoch inside a program. Process termination
closes it, but that is an OS premise outside every modelled scope, it cannot
justify a `Safe` verdict for a library, and it does not discharge **W**.

### 6.2 Description and write-back post-states

`DescriptionReleased` / `DescriptionRetained` / `DescriptionUnknown`, and
`WriteBackFailed` / `WriteBackUnknown`. `WriteBackDischarged` exists in the
vocabulary solely so that a test can assert the relation never derives it.

## 7. The relation

### 7.1 `R_close[linux]` — the selected target

| Outcome | D | F | W | Recovery |
|---|---|---|---|---|
| O0 not attempted | **NotAttempted** | Retained | Unknown | `close` |
| O1 result unobserved | **Attempted** | Unknown | Unknown | none |
| O2 `0` | **Released** | Released if **A6**, else Retained | **Unknown** — never Discharged [M CAVEATS] | — |
| O3 EBADF | **Indeterminate** + violation of **A2** | Unknown | Unknown | none |
| O4 EINTR | **Released** [M CAVEATS] | Unknown | **Failed** | **none** |
| O5 EIO/ENOSPC/EDQUOT | **Released** [M CAVEATS] | Released if **A6**, else Retained | **Failed** | none |
| O6 other observed errno | **Released** ([M]: always closes except EBADF) | Unknown | Unknown | none |
| O7 errno unobserved | **Attempted** | Unknown | Unknown | none |

Three of these are the point of the card.

* **O4 and O5 release the descriptor.** A failed `close` is not a no-op and not
  a retained obligation on this target. Modelling it as "custody preserved"
  both over-counts live descriptors *and*, far worse, licenses the retry [M]
  documents as wrong.
* **EBADF is the only documented non-releasing error**, which is precisely why
  O1 and O7 cannot conclude a release: without the result or `errno`, the one
  arm that behaves differently has not been excluded. This is what makes
  "check the return value of `close`" a soundness requirement of the profile
  rather than hygiene advice.
* **O3 is a refutation, not information.** Under A2 the analysis believed the
  epoch live; EBADF says it was not. The honest post-state is poisoned with a
  recorded premise violation — not `Retained`, because the thing that would be
  retained has been shown not to exist.

### 7.2 `R_close[posix2024]` — the contrast

| Outcome | D | Recovery |
|---|---|---|
| O2 `0` | Released | — |
| O3 EBADF | Indeterminate + A2 violation | none |
| O4 EINTR | **Retained** [P24] | **`close` — required** |
| O5, O6 | **Indeterminate** — [P8] "was silent on this point" and [P24] legislates only EINTR | none |

### 7.3 `R_close[unpinned]` — why no portable contract is published

The join, per outcome. On O4 it is not that less is known on an unpinned
target; it is that the two targets prescribe **opposite** recovery: [P24]
*requires* the retry that [M] documents as a hazard. So:

> On an unpinned target, EINTR yields **Indeterminate** with the full poisoned
> duty set and **no permitted recovery at all**.

This is the direct answer to the card's acceptance item 3. The fail-closed
response to an indeterminate descriptor is neither "retry" nor "assume
released": it is to carry the residual, refuse every operation on the number,
and keep the endpoint in the denominator. Only pinning the target removes the
indeterminacy — which is why this profile is target-qualified by construction,
matching the host's existing `linux-rlimit-v1` / `linux-confined-v1` naming.

### 7.4 Joins

Two paths reaching one program point: agreement is preserved exactly;
disagreement widens to `Indeterminate` with the poisoned duty set, and
`must_release_epoch` is dropped because it contradicts `must_not_retry_close`.
A join never produces a permitted recovery that both sides did not permit.

### 7.5 Against the §C10 policy table

Contract spec §C10 offers four cleanup policies. `close` on Linux is **none of
them cleanly**, which is itself the finding:

| §C10 policy | Fit for Linux `close` |
|---|---|
| Infallible release | no — failure is documented and reachable |
| Consumes on attempt | **closest**, for D — but only because the *kernel* releases early, and it is false for EBADF, so "on attempt" is not the right premise |
| Returns on failure | **false on this target**, and the assumption behind the retry hazard |
| Poisons on failure | correct for O1/O3/O7 and for every unpinned divergence, but not for O4/O5, which are determinate releases |

The relation therefore needs a *per-identity, per-outcome* disposition rather
than one policy label for the operation. That is the structural request this
design makes of any future endpoint-contract schema (§12, U1).

## 8. Sentinels, generations, and what stays unchanged in v1

**Acceptance item 2.** Everything below is explicit, and resource-contract v1 is
untouched.

* **Success test.** Declared as `success_test: ret == 0`. Not `ret != -1`, not a
  nullability test. §C04.4 already records that `-1`-style scalar sentinels and
  non-null sentinels like `MAP_FAILED` are not representable by v1's
  nullability-only return mode, and this profile does not attempt to reinterpret
  them: it introduces its own success test rather than widening v1's.
* **v1 nullable-pointer meanings are unchanged.** `resource_return_owned(n)`,
  `resource_released(p, n)` and the rest keep their §C04.2/§C04.3 semantics
  exactly. Nothing in this profile is loaded by the resource loader, and
  `requires resource_contract_v1` is untouched.
* **A `close`-like syscall must not be declared with v1 `resource_released`.**
  That mode discharges unconditionally and cannot express O3, O4 or O7. Audited:
  `contract/qcore` contains six `resource_released` declarations — five
  `free`-shaped (`libc/memory`, `monkey/memory`, `librdkafka/memory`,
  `fluent_bit/resource_memory` ×2) and one library wrapper
  (`chunkio/resource_open_close`'s `cio_chunk_close`). **No raw descriptor syscall is declared in v1
  today**, so this rule forbids a future mistake rather than describing a
  present one.
* **Generations are explicit.** An epoch is `(kind = Descriptor, number,
  generation)`. `Released` ends generation *g*; a later acquisition returning
  the same number opens *g+1*. §C10's no-merge rule is carried as a side
  condition and pinned by a test — and by probe P3, which measures the kernel
  handing the same number straight back to an unrelated file.

## 9. Implementation and evidence

### 9.0 Where the relation lives

| Artifact | Role |
|---|---|
| `crates/qcore-core/src/os_endpoint.rs` | **decides** the relation. `close_poststate`, `join`, `check_claim`, `Epoch`; domain errors are `Result`, not panics. **39 tests**, twelve of them refusals, six exhaustive over the whole `3 × 64 × 10` accepted domain. |
| `proof/QCore/OsEndpointPostState.lean` | **formalizes** it. **27 theorems**; `mem_all*` prove the enumerations complete and `of_domainAll` lifts a decided `Bool` check to a genuine `∀ t p o`, so the no-retry, no-durability, only-a-live-epoch-may-close, last-reference and record-lock invariants hold over the entire domain. Audited by `proof/AuditOsEndpointPostState.lean`; every theorem depends only on `propext` and `Quot.sound`. |
| `scripts/o03_close_poststate_relation.py` (+ tests) | **supplementary**: the real-syscall probe (§9.2), and a 38-test cross-check of the same matrix. |

The Lean module **coexists with** `QCore.DescriptorCustody` (§13.1) rather than
replacing it: that module's pinned theorem is untouched, the two live in
separate namespaces over separate record types, and reconciling them is U4.

The Rust and Lean developments are two statements of one relation, written to
agree and checked separately. No extraction or mechanical correspondence ties
them; establishing that link is the same class of work F01 did for
`check_concurrent`.

Recorded probe output:
[`logs/relation_and_probe.json`](../reports/remaining-o03/logs/relation_and_probe.json).

### 9.1 Refusals

Twelve forged post-states are refused, each one a claim somebody could
plausibly want to make:

EINTR retains on Linux · EINTR releases on POSIX.1-2024 · a retry is permitted
after a failed Linux close · a retry is permitted from `Indeterminate` · EBADF
released the epoch · an unobserved result released the epoch · success
discharged the data duty · a failed close discharged the data duty · the
description died without the last reference · success identified the epoch
without sole custody · a poisoned epoch left the denominator · an unknown
post-state field · **and one admission**, so the refusals are not vacuous.

### 9.2 The probe: real syscalls, two arms

Four probes execute real, unsuppressed `open`/`close`/`dup`/`fcntl`, measured on
Linux `7.0.0-30-generic`, `x86_64`, glibc 2.43:

| Probe | Measured |
|---|---|
| **P1** | after `close` returns `0`, `fcntl(n, F_GETFD)` gives EBADF — the slot really is released |
| **P2** | closing the same number again really returns `-1`/EBADF, ending no epoch |
| **P3** | the number (fd `3`) came **straight back** from the next `open`, bound to a different inode — distinct epochs on one number |
| **P4** | after `dup` then closing the original, the original number is EBADF while the duplicate still reads the file — F outlived D |

**P3 is the measured justification for the no-retry rule.** The hazard is not
hypothetical: the freed number is reused immediately, so a retried `close` would
close the new epoch — and would return `0`, not EBADF.

**What the probe deliberately does not do.** It cannot observe O4 or O5: EINTR
needs an interrupting signal on a filesystem whose close is interruptible, and
EIO/ENOSPC/EDQUOT need a failing device, a full filesystem or an exhausted
quota. Those two arms rest on [M] alone, and the probe's own output records that
under `not_observed`. A test asserts no observation mentions EINTR.

## 10. What the suppressed-syscall fixtures do and do not witness

The Cycle 19 packages
([descriptor failures](../reports/cycle19-file-descriptor-failures/README.md),
[reopen failures](../reports/cycle19-reopen-failures/README.md)) suppress a
selected `close`/`fstat`/`mmap` and return a fixed error **without reaching the
kernel**. They report, accurately, that a failed close leaves the descriptor in
the down ring, that down-ring membership and a zero up counter cannot discharge
descriptor custody, and that residuals survive parent teardown.

Those are sound statements *about the wrapper*. They are **not** evidence about
real `close(2)`, and the direction matters:

> A wrapper that returns `-1`/EIO without calling the syscall necessarily leaves
> the descriptor open. That is the **POSIX.1-2024 / HP-UX** post-state. On the
> real Linux target the same errno leaves the descriptor **released**.

So the fixtures observe the opposite of the target's documented behaviour for
that arm — legitimately, because they never claimed otherwise; §C10 already
requires that "a future profile MUST encode its actual supported failure
semantics" and "MUST NOT derive universal OS behavior from a wrapper that merely
suppresses a selected syscall". This document is that profile, and a test
(`SuppressedFixtureContrastTest`) pins the disagreement so the two can never be
quietly conflated.

**Nothing above is a defect claim against those packages.** Their result
boundaries already scope them to recorded executions.

## 11. Why `munmap` is not a parameter of this relation

[MM]: `munmap` returns `0` or `-1`, "probably" with EINVAL. Three structural
differences forbid reusing the close relation:

1. **The identity is a range, not an epoch.** `munmap` can unmap part of a
   mapping, splitting one mapping into two. `(kind, number, generation)` does
   not describe it.
2. **Its documented failure genuinely is "Returns on failure."** EINVAL means
   the arguments were rejected, so nothing was unmapped. This is the §C10 policy
   that is *false* for Linux `close` — so a shared "endpoint release" policy
   would have to be wrong for one of them.
3. **Its failure sentinel is on the acquisition side, not the release side.**
   `mmap` fails with non-null `MAP_FAILED`, which §C04.4 already names as
   unrepresentable in v1, and which the reopen-failure package measured being
   stored into a live field.

A `munmap` profile is follow-on unit U6; this document does not sketch its
relation.

## 12. Follow-on implementation units

None of these is implemented here. Each is sized so it can be taken alone.

| # | Unit | Notes |
|---|---|---|
| **U1** | **Schema record.** An `EndpointOperationContract` in `qcore-schemas`: endpoint kind, target triple, success test, an outcome→disposition table per identity, the premise vector, and the permitted-recovery set. Per this card's instructions it is **specified, not added**. It would be a source-incompatible addition, so `qcore-schemas` would move to **6.0.0** under the documented policy. |
| **U2** | **Loader vocabulary.** `requires os_endpoint_contract_v1`, total-or-reject, beside `resource_contract_v1` and never silently reinterpreting it. A declaration missing a target, a success test or any outcome row rejects whole. |
| **U3** | **Gateway transfer function.** The blocking dependency is **relational branch analysis**: the relation is result-dependent, and both C02 §7 and O04 §7 record branch conditions as out of scope. Until an arm can be selected from `ret == 0` and an `errno` comparison, every real call site joins to **`Attempted`**. That is still sound, still fail-closed, and still strictly stronger than a v1 `resource_released` declaration, which would claim an unconditional discharge. |
| **U4** | **`proof/QCore/DescriptorCustody.lean`.** `Close.Valid` hard-codes `succeeded = false → liveAfter = true` (theorem `accepted_failed_close_retains_custody`). See §13. `QCore.OsEndpointPostState` now states the corrected relation alongside it; the pinned theorem is deliberately untouched, and unifying the two record types is the remaining work. |
| **U5** | **`providers/qcore-provider-libevent/contracts/monkey/composition.qcore`.** `evutil_closesocket`'s `ensures error(SocketCloseError) descriptor_custody_is_preserved`. See §13. |
| **U6** | **`munmap` sibling profile**, per §11. |
| **U7** | **Evidence for O4/O5.** A `SIGSTOP`/timer-driven EINTR fixture on an interruptible filesystem, and a small-loopback-device ENOSPC fixture, would bind the two arms this card takes from documentation alone. Both need privileges the current harness does not assume. |

## 13. Two existing declarations this relation contradicts

Both are recorded as findings with follow-on units, **not repaired here**: they
are pinned artifacts, and repairing either is an implementation change outside a
bounded design card. Neither is an upstream defect claim — both are defects in
*this repository's declared model text*, in the sense P07 used.

### 13.1 `proof/QCore/DescriptorCustody.lean`

```lean
def Close.Valid (close : Close) : Prop :=
  (close.liveBefore = true ∧ close.succeeded = true ∧ close.liveAfter = false ∧ …) ∨
  (close.liveBefore = true ∧ close.succeeded = false ∧ close.liveAfter = true  ∧ …)
```

The second disjunct is the §C10 "Returns on failure" policy, and
`accepted_failed_close_retains_custody` states it as a theorem about accepted
records. Its intent — refusing to credit a failed close as a discharge — is
right, and in the leak direction it is conservative. But on the selected target
it is unfaithful in two ways:

1. A **truthful** record of a real Linux O5 failure (`succeeded = false`,
   `liveAfter = false`) is **rejected** by `checkClose`.
2. `liveAfter = true` models the epoch as usable, so a later use of, or retry on,
   that number is not refused — which is the documented hazard.

The root cause is representational: one `liveAfter : Bool` cannot express
"the descriptor is gone **and** nothing is discharged". That is exactly what the
three-identity split of §3 and the poisoned states of §6.1 exist to say.

### 13.2 `providers/qcore-provider-libevent/contracts/monkey/composition.qcore`

```qcore
extern fn evutil_closesocket(descriptor: own<Descriptor> use one)
    -> result<unit, SocketCloseError>
…
    ensures error(SocketCloseError) descriptor_custody_is_preserved;
```

On Linux this function is `close(sock)`, so the error postcondition asserts
"Returns on failure" for an operation that releases on failure — and thereby
prescribes the retry [M] forbids. `monkey/descriptor_observation.qcore` names
this file as the home of "the general conditional descriptor model", so it is
where a reader would look.

**Nothing is derived from it.** The clause carries no `resource_contract_v1`
marker, so the resource loader never loads it; its clause count is pinned by
`compiler/tests/qcore_contract_corpus.rs` (`evutil_closesocket`, 4 clauses); and
no Rust identifier `descriptor_custody_is_preserved` or `SocketCloseError`
exists anywhere in `crates/` or `compiler/`. It is parsed, pinned and
semantically inert. Repairing it under U5 changes the pinned corpus tables and
must be done deliberately.

## 14. Non-claims

* **No universal close or mmap failure policy.** One operation, one target.
* **No durability result.** `W` is never discharged by any arm; only `fsync(2)`
  is named as capable of it, and its own failure semantics are not specified here.
* **No resource-v1 extension.** v1 nullable-pointer meanings, the loader, the
  contract corpus (59 modules) and every clause count are unchanged.
* **No production wiring.** `qcore-core` is the dependency-free L1 kernel and no
  analyzer, driver, contract or audit route calls `close_poststate`. No
  diagnostic code, gate or pinned expectation changed, and
  `proof/QCore/DescriptorCustody.lean` was not touched.
* **No promotion.** Raw Cycle 19 units stay 2 Safe / 42 provisional Unsafe /
  137 Unknown plus four data-only units; CT coverage is untouched.
* **The Lean theorems are about the model, not the kernel.** They establish that
  the *relation* is fail-closed. Whether it is faithful to Linux is a premise
  read from [M], and which arm a real call took is a premise supplied from
  outside.
* **Not a proof of the arms.** The Linux arms are read from documentation [M],
  not verified against the kernel; O4 and O5 are unmeasured here (§9.2); the
  probe's four observations are of one host and bind nothing beyond it.
* **A `Released` derivation is a claim about D only.** It says nothing about F
  without A6, and nothing about W ever.

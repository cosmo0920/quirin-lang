# O03-U4/U5 — Repair the two declared-model defects O03 recorded — 2026-09-09

O03 specified the target-qualified Linux `close(2)` failure post-state relation,
decided it in `crates/qcore-core/src/os_endpoint.rs` and formalized it as
`QCore.OsEndpointPostState`, and then **recorded two existing declarations that
the relation contradicts without repairing either**
([`docs/reports/remaining-o03/README.md`](../remaining-o03/README.md) §5,
[`docs/design/OS_FAILURE_POSTSTATE_PROFILE.md`](../../design/OS_FAILURE_POSTSTATE_PROFILE.md)
§13). This card repairs both, and reconciles the one Rust route that mirrored
the same mistake.

Neither is an upstream defect claim. Both were defects in **this repository's
declared model text**, in the sense P07 used.

| Unit | Site | Repaired to |
|---|---|---|
| **U4** | `proof/QCore/DescriptorCustody.lean` — `Close.Valid`, theorem `accepted_failed_close_retains_custody` | the legacy record's failure arm is **withdrawn**; failed closes are represented by a new `TargetedClose` whose validity is decided by `closePostState` itself |
| **U4-R** | `crates/qcore-core/src/event_reactor.rs` — the Cycle 18 `CloseSocket` failure branch, which carried the same "a failed close retains the obligation" comment and rule | a failed close now **poisons** the number: no retry, no use, no reuse, custody still unresolved |
| **U5** | `contract/qcore/monkey/composition.qcore` — `evutil_closesocket`'s `ensures error(SocketCloseError) descriptor_custody_is_preserved` | four error clauses stating an unresolved endpoint, the reuse and retry prohibitions and the platform error channel, plus one `any` clause separating durability |

Every change moves in the conservative direction: where the old models claimed
retention the relation cannot support, the new ones answer **unresolved**, and
one program shape the reactor previously **accepted** is now refused.

---

## 1. What the old clause said, and why it is wrong here

`docs/design/OS_FAILURE_POSTSTATE_PROFILE.md` §7.1, from Linux man-pages 6.17
`close(2)`: the kernel "always releases the file descriptor early in the close
operation", so on Linux a failed `close` **releases** the descriptor on every
errno except `EBADF`, and "Retrying the close() after a failure return is the
wrong thing to do". POSIX.1-2024 legislates the opposite for `EINTR` — the
descriptor stays open and `close` must be called again — so with **no pinned
target** the honest answer is `Indeterminate` with the full poisoned duty set
and **no permitted recovery at all**.

The old Lean clause was:

```lean
def Close.Valid (close : Close) : Prop :=
  (close.liveBefore = true ∧ close.succeeded = true  ∧ close.liveAfter = false ∧ close.closeRecorded = true) ∨
  (close.liveBefore = true ∧ close.succeeded = false ∧ close.liveAfter = true  ∧ close.closeRecorded = false)
```

The second disjunct is contract spec §C10's "Returns on failure" policy, stated
unconditionally and without a target. It fails in **both** directions:

1. it **rejects a truthful record** of a real Linux `EIO`/`EINTR` failure, whose
   post-state is `liveAfter = false`; and
2. `liveAfter = true` models the epoch as usable, so a later use of — or retry
   on — that number is not refused. That is the documented hazard, and O03's
   probe P3 measured why it is not hypothetical: the freed number came straight
   back from the next `open`, bound to a different inode, so a retried close
   would close an unrelated live epoch **and return `0`**.

The root cause is representational, exactly as §13.1 diagnosed: one
`liveAfter : Bool` cannot express "the descriptor is gone **and** nothing was
discharged".

---

## 2. U4 — the Lean repair

### 2.1 Shape

`proof/QCore/DescriptorCustody.lean` now `import`s `QCore.OsEndpointPostState`
(one-way; that module is unchanged except for a stale doc paragraph, §2.5).

* The historical four-field `Close` record **keeps its exact shape** —
  `liveBefore`, `succeeded`, `liveAfter`, `closeRecorded` — and its **success**
  arm. Its failure arm is withdrawn, so `checkClose` returns `none` for every
  failed close rather than certifying a false retention.
* A new `TargetedClose` carries the target, the premise vector, the observed
  outcome, the five-valued `descriptorAfter`, the claimed `mayRetryClose`, the
  claimed `custodyDischarged` and a `dataDutyDischarged` claim that is always
  refused. `TargetedClose.Valid` is **not an independent policy**: it demands
  that `closePostState target premises outcome` derive exactly what the record
  claims, with custody discharged **exactly** when the descriptor is `released`.
  The declared model therefore cannot drift from the relation again.
* All new declarations are **appended after** `Teardown`, so every
  auto-generated instance name (`instDecidableValid`, `_1`, `_2`, `_3`, the
  `Close.mk`/`decEq` families) keeps its number and its meaning.

### 2.2 Historical alias surface

`proof/QCore/MonkeyEvent.lean` re-exports 221 historical declarations, including
`Close.liveAfter`, `Close.succeeded`, `Close.mk`, `instDecidableValid_1` and
`accepted_failed_close_retains_custody`. **It is unedited and still compiles**:
no field, constructor, instance or theorem name was removed or renumbered.
`proof/AuditGenericNames.lean` is unedited and still passes.

### 2.3 Changed theorems, by name

| Theorem | Old meaning | New meaning |
|---|---|---|
| `accepted_successful_close_discharges_once` | given an accepted record with `succeeded = true`: `liveBefore = true ∧ liveAfter = false ∧ closeRecorded = true` | **statement unchanged and still true**; the proof is one case shorter because the failure disjunct is gone. The `succeeded` hypothesis is now redundant (validity forces it) and is retained as `_succeeded` so the signature and arity do not move |
| `accepted_failed_close_retains_custody` | given an accepted record with `succeeded = false`: `liveBefore = true ∧ liveAfter = true ∧ closeRecorded = false` — the descriptor still live, usable and retriable, on every target | **`False`** — the hypotheses are contradictory, because no failed close is admitted by this record type at all. This is a refusal, not a vacuity: `the_returns_on_failure_record_is_refused` exhibits the concrete record `⟨true, false, true, false⟩` that the old clause accepted and `checkClose` now rejects |

No other pre-existing theorem's statement moved. `accepted_error_query_is_borrowed`,
the two socket-pair theorems and both teardown theorems are byte-identical.

A caller that used `accepted_failed_close_retains_custody` for its old
conclusion now gets a **type error**, which is the intended outcome: the meaning
changed, so the break is deliberate and visible rather than silent.

### 2.4 New theorems (16)

All in `QCore.DescriptorCustody`, all printed by `proof/AuditCycle19.lean`, all
depending only on `propext` and `Quot.sound`. No new axiom, no `sorry`, no
placeholder.

| Theorem | Content |
|---|---|
| `a_failed_legacy_close_record_is_refused` | the same refusal under a name that says what it establishes |
| `the_returns_on_failure_record_is_refused` | the concrete record the withdrawn clause accepted is rejected |
| `the_legacy_success_arm_is_the_relation_under_sole_custody` | the surviving legacy arm **is** the relation's `zero` arm under premise A3, on every target; the premise was implicit before and is named now |
| `accepted_close_agrees_with_the_endpoint_relation` | every field of an accepted record equals the derived post-state |
| `no_accepted_close_discharges_the_data_duty` | no record may claim durability |
| `accepted_custody_is_discharged_exactly_when_the_epoch_is_released` | a retained or poisoned epoch still owes its obligation |
| `a_failed_close_retains_the_descriptor_only_on_posix2024_eintr` | **the corrected content of the old theorem**, decided over the whole `3 × 64 × 10` domain: retention on a failure occurs only where the documentation states it |
| `a_failed_linux_close_releases_the_descriptor_and_permits_no_retry` | the five non-`EBADF` Linux errnos release and permit nothing |
| `an_unpinned_failed_close_is_poisoned_and_permits_no_recovery` | over the whole domain: with no target pinned, every failed close is poisoned, permits nothing, and carries `noRetryClose`, `noUse`, `noTreatNumberAsFree` and `keepInDenominator` |
| `the_targeted_returns_on_failure_record_is_refused` | the old policy restated with a target, refused |
| `the_truthful_linux_deferred_io_record_is_accepted` | the record the old clause **rejected** is admitted — the refusals are not vacuous |
| `a_retry_claim_after_a_failed_linux_close_is_refused` | all seven failure errnos |
| `a_durability_claim_is_refused` | durability refused even on a successful close |
| `a_successful_close_without_sole_custody_is_not_a_discharge` | A3 is load-bearing on the success arm |
| `close_relation_domain_size` | the enumerated domain is 1920 triples |
| `close_relation_digest` | §4 |

### 2.5 One doc-only edit outside `DescriptorCustody.lean`

`proof/QCore/OsEndpointPostState.lean`'s header said "Reconciling them is
recorded as follow-on unit U4 and is deliberately not done here". That paragraph
is now false, so it is rewritten to state the one-way dependency. **No
definition, statement or proof in that module changed**, and its 27 theorems and
`AuditOsEndpointPostState.lean` are untouched.

---

## 3. U4-R — the Rust route that mirrored the old clause

`crates/qcore-core/src/event_reactor.rs` carried the same rule in code, with the
comment *"A failed close retains the obligation. The terminal custody check
rejects an absent retry."* Under that rule a Cycle 18 program that closed a
descriptor, failed, and then **retried successfully** was **accepted**. The
reactor's `EventOperation::CloseSocket` vocabulary carries neither a target nor
an `errno`, so the relation's answer for it is the unpinned one: poisoned, no
recovery.

The repair adds one `DescriptorCustody::Poisoned` custody state. A failed
`CloseSocket` now records it, which means:

| Duty from the relation | Reactor behaviour after the repair |
|---|---|
| `NoRetryClose` | a retried close is refused with `L03DescriptorCustody` **at its own operation index** |
| `NoUse` | `GetSocketError` on that number is refused |
| `NoTreatNumberAsFree` | a later `CreateSocketPair` claiming that number is refused (the entry stays in the custody map), and `CreateBufferevent` over it is refused |
| `KeepInDenominator` | the terminal custody sweep still refuses, exactly as before |

This is strictly more refusing. No diagnostic code was added, removed or
changed; `L03DescriptorCustody` is reused. The pre-existing Cycle 18 tests are
unedited and all pass, including
`failed_or_duplicate_socket_close_is_fail_closed`, whose expected terminal
diagnostic is byte-identical.

The new `qcore-core` test
`a_failed_close_agrees_with_the_unpinned_endpoint_relation` reads the four
duties **out of `close_poststate` itself** for all seven failure errnos, then
exercises each one against the reactor, so the reactor's disposition is pinned
to the relation rather than restated by hand.

**Divergence recorded, not repaired.** The reactor still has no target and no
errno in its event vocabulary, so it can never reach the Linux `Released` arm
and can never distinguish `EBADF` from `EIO`. It answers with the join of both
targets, which is sound and is the conservative direction; carrying a target and
an errno into the Cycle 18 event model is out of this card's scope and is
recorded as open (§8, O03-U4-REACTOR-TARGET).

---

## 4. The agreement pin: Lean model versus the `os_endpoint` relation

U4 makes the declared model depend on the relation, so a silent divergence
between the Rust and Lean statements of that relation would now be a divergence
between the Lean model and the Rust checker. O03 explicitly left that link
unmade ("two statements of one relation, written to agree and checked
separately; no extraction or mechanical correspondence ties them").

This card closes it the way F01 did for `check_concurrent`:

* `crates/qcore-core/tests/os_endpoint_correspondence.rs` and
  `QCore.DescriptorCustody` enumerate the **same** domain in the **same** order —
  `allTargets`, then `allPremises` with `epochLive` outermost and `false` before
  `true`, then `allOutcomes` — and each independently computes the same two
  numbers over `closePostState` / `close_poststate`:

| Pin | Value | Rust | Lean |
|---|---|---|---|
| domain size | `1920` | `the_enumerated_domain_is_the_one_the_lean_model_enumerates` | `close_relation_domain_size` |
| relation digest | `11372109764820645253` | `the_relation_agrees_with_the_lean_model_over_the_whole_domain` | `close_relation_digest` |

The digest folds the target, all five descriptor states, the description state,
the write-back state, all seven duty bits and the permitted recovery of **every**
answer, plus the encoded input, so a difference in any field of any answer on
either side moves it. The out-of-domain answer encodes as `0` on both sides, so
the `NoEpochToDescribe` refusal is covered too.

**What this is not.** A digest is a comparison device, not a proof device: two
different encodings agreeing would be a collision, and the Rust encoding is
transcribed by hand from the Lean one rather than extracted from it. Neither
side is thereby shown faithful to the kernel — that stays a premise read from
the man page, as the design document §14 records. The claim is exactly:
*the two implementations of the relation answer identically on every input of
the declared domain.*

A deliberately `#[ignore]`d `recompute_pins` test prints both numbers when they
must be re-pinned; it is not part of the gate.

`crates/qcore-core/src/os_endpoint.rs` also gains the Rust twin of the Lean
record layer — `CloseRecord`, `check_close_record`, `CloseRecordRefusal` — so
the same admission rule is stated in both languages and both halves of the old
defect are pinned in Rust as well (§5). Refusals carry module-local keys; no
diagnostic code is claimed for this model-level admission.

---

## 5. U5 — the `evutil_closesocket` clause, against the pinned source

Read at the pinned submodule revision `d82464a2`,
`third_party/libevent/evutil.c:613-621`:

```c
int
evutil_closesocket(evutil_socket_t sock)
{
#ifndef _WIN32
	return close(sock);
#else
	return closesocket(sock);
#endif
}
```

Two facts follow, and both are now stated in the declared text rather than
assumed:

1. **It is a thin forwarder that translates no error.** Unlike other libevent
   sites it does not call `EVUTIL_SET_SOCKET_ERROR`. So on Windows the failure
   is reported through `WSAGetLastError()` — which `evutil_socket_geterror` reads
   (`include/event2/util.h:523-531`) — and on every non-Windows build through
   `errno` (`include/event2/util.h:561-566`). Reading one channel on the other
   platform reads an unrelated value.
2. **On the audited non-Windows build this call *is* `close(2)`**, so its failure
   post-state is the target-qualified one: on Linux the descriptor is already
   released except on `EBADF`, and POSIX.1-2024 requires the retry Linux
   documents as a hazard, so with no pinned target the endpoint is indeterminate
   and no recovery is permitted.

The superseded clause `ensures error(SocketCloseError) descriptor_custody_is_preserved`
asserted a live, reusable descriptor and thereby prescribed exactly that
forbidden retry. It is replaced by:

```qcore
    ensures error(SocketCloseError) descriptor_custody_is_unresolved;
    ensures error(SocketCloseError) descriptor_number_must_not_be_reused;
    ensures error(SocketCloseError) no_close_retry_is_permitted;
    ensures error(SocketCloseError) close_error_is_read_on_the_platform_channel;
    ensures any data_durability_is_not_discharged;
```

The `requires` clauses and the `ensures normal descriptor_custody_is_discharged_once`
arm are unchanged. `evutil_socket_geterror` keeps its own
`ensures any descriptor_custody_is_preserved` clause, which is correct: it is a
borrow guarded by `requires descriptor_is_live`.

A comment in `contract/qcore/monkey/descriptor_observation.qcore` — the file that
names `composition.qcore` as the home of "the general conditional descriptor
model" — now points the reader at the post-state profile. That file's clause
counts are unchanged.

---

## 6. Pin updates, stated deliberately

| Pin | From | To | Reason |
|---|---|---|---|
| `compiler/tests/qcore_contract_corpus.rs`, `("monkey/composition.qcore", "evutil_closesocket", …)` | `4` | `8` | the one false error clause is replaced by four clauses plus one `any` clause separating durability. The replacement is recorded in the table entry's own comment |
| `docs/reports/cycle19-native-rules/{evidence,metadata}.json` and `docs/reports/cycle19-callback-binding/{evidence,metadata}.json`, `contract/qcore/monkey/composition.qcore` = `98bdd22b…0425b5` | — | **left as archived** | these are archived provenance snapshots of runs executed against the pre-repair text. They are deliberately **not** rewritten: the recorded runs really did read those bytes. The file's digest is now `ee16fa5d7b8bd36f355397045a7c12545db28392eaad113b454f862096215e73`; no test consumes the archived value (verified by search), and the change is measured inert for the only live consumer (§7) |

No other pinned expectation moved. The contract corpus stays **60 modules**, the
loader's rejected-declaration count is untouched (no declaration was added or
removed, and `composition.qcore` still carries no `requires resource_contract_v1`
marker, so the resource loader still never loads it), and no diagnostic code was
added, removed or downgraded.

---

## 7. Cross-check: what consumes these declarations

| Consumer | Result |
|---|---|
| `check_monkey_bindings` (`compiler/src/verifier_plugins/libevent/monkey_binding.rs`), the only live reader of `composition.qcore` | evaluates contracts for `cb_event`, `cb_timeout` and `cb_wait_2_timeout` only. All 8 of its tests pass unchanged, including `native_callbacks_bind_source_paths_and_qcore_predicates` with its pinned 3 checked executions and 6 checked postconditions, and `source_and_contract_mutations_change_verdicts`. The U5 change is **measured inert** for this route |
| the resource contract loader (`compiler/tests/contract_loader.rs`) | 10 tests pass unchanged, including `every_corpus_module_outside_the_v1_vocabulary_is_rejected_not_skipped` |
| Cycle 18 event-reactor checker | `compiler` route: 19 `event_driven` tests pass unedited; `qcore-core` route: 5 tests pass (4 pre-existing unedited, 1 new) |
| `crates/qcore-verification/tests/reactor_source.rs`, `compiler/tests/reactor_route_parity.rs` | pass unchanged |
| Lean | `QCore.MonkeyEvent` and `AuditGenericNames.lean` unedited and passing; `AuditOsEndpointPostState.lean` unedited and passing |

No certificate whose premises referenced the old clauses required re-validation
by construction: the `.qcore` clause is semantically inert (no
`descriptor_custody_is_preserved` or `SocketCloseError` identifier exists in any
Rust source under `crates/` or `compiler/`, as O03 verified and this card
re-verified), and the Lean `Close` predicate is a validity check on submitted
records with no Rust producer. The 185-unit corpus artifacts are untouched and
were not re-measured: nothing on the C gateway or audit route changed.

---

## 8. Gate results

Run in this card's isolated worktree with the Libevent submodule initialized.
Lean 4.33.1 via `PATH="$HOME/.elan/bin:$PATH"`.

| Gate | Command | Observed | Exit |
|---|---|---|---|
| Workspace tests | `cargo test --workspace` | **1,664 passed / 0 failed / 2 ignored** (12 new tests; the second ignored test is this card's deliberate `recompute_pins`) | 0 |
| Format | `cargo fmt --all -- --check` | clean | 0 |
| Clippy | `cargo clippy --all-targets -- -D warnings` | clean | 0 |
| Probe | `cargo run -p qcore-compiler --example reevaluation_probe` | ran | 0 |
| Follow-up | `cargo run -p qcore-compiler --example reevaluation_followup` | ran | 0 |
| Lean audits | `python3 scripts/check_lean_audits.py` | **38 audits passed**; `AuditCycle19.lean` grows by 16 printed theorems, every one within `propext` / `Quot.sound` | 0 |
| Provider tests | `cargo test --locked --offline --manifest-path providers/Cargo.toml --workspace` | **58 passed / 0 failed** | 0 |
| Python | `python3 -B -m unittest discover -s scripts/tests` | **108 ran, OK** | 0 |

**One load-flaky failure, recorded rather than hidden.** The first full
`cargo test --workspace` run failed `cycle20_verifier_plugins` →
`vp04_vp07_vp08_process_lifecycle_and_payload_budgets` at
`assert!(elapsed < Duration::from_millis(500))`. It is the wall-clock assertion
the delegation brief already names as load-flaky; it passed **3 of 3** re-runs in
isolation once the machine settled, and the second full workspace run was green.
It touches nothing this card changed. **No assertion was weakened.**

New tests, by home:

| File | Tests |
|---|---|
| `crates/qcore-core/src/os_endpoint.rs` | 8 — the record layer: the old returns-on-failure record refused, the truthful Linux failure record accepted, no retry after any of seven failure errnos, retention only on POSIX.1-2024 `EINTR`, unpinned failures poisoned, no durability claim, success without sole custody, discharge ⇔ release over the whole domain (39 → 47) |
| `crates/qcore-core/tests/unit/event_reactor.rs` | 1 — the reactor's failed-close disposition read out of `close_poststate` and exercised against all four duties (4 → 5) |
| `crates/qcore-core/tests/os_endpoint_correspondence.rs` | 2 + 1 ignored — domain and digest agreement with Lean |
| `compiler/tests/qcore_contract_corpus.rs` | 1 — the corrected `evutil_closesocket` error arm pinned exactly, and the withdrawn clause refused by name on that declaration |

---

## 9. Verdicts carried forward unchanged

| Item | Value | Changed by this card |
|---|---|---|
| Raw Cycle 19 units | 2 Safe / 42 provisional Unsafe / 137 Unknown + 4 data-only | no |
| Pinned 185-unit artifacts | untouched, not re-measured | no |
| CT coverage | 24/50 mapped | no |
| Contract corpus | 60 modules | no |
| `qcore-schemas` | 6.0.0 — **untouched** | no |
| Full language conformance | false | no |

Nothing is promoted. No diagnostic code was added, removed or downgraded, and no
fail-closed gate was weakened — every behavioural change in this card refuses
strictly more.

---

## 10. Open boundaries

* **O03-U4-REACTOR-TARGET.** The Cycle 18 event vocabulary carries no target and
  no `errno`, so the reactor necessarily answers with the unpinned join. It can
  never conclude the Linux `Released` arm and never separate `EBADF` from `EIO`.
  Carrying those into `EventOperation::CloseSocket` — and deciding what a
  `Released` descriptor means for the reactor's terminal sweep — is not done
  here.
* **No production consumer.** `close_poststate` and `check_close_record` are
  still called by no analyzer, driver, contract or audit route; O03's follow-on
  **U3** (gateway transfer function, blocked on relational branch analysis) is
  unchanged and open. This card repairs declared models; it does not wire the
  relation in.
* **The digest is a comparison, not an extraction.** §4. The Lean-to-Rust link
  for this relation is now as strong as F01's for `check_concurrent`, and no
  stronger.
* **The Linux arms are still read from documentation.** O03's `EINTR` and
  `EIO`/`ENOSPC`/`EDQUOT` arms remain unmeasured (its follow-on **U7**), so the
  repaired declarations rest on the man page for exactly those two arms.
* **U5's Windows half is declared, not exercised.** The clause now states the
  platform error channel correctly, but the audited build is non-Windows and no
  fixture executes the Windows `closesocket` arm.
* **The remaining O03 follow-ons are untouched**: U1 (schema record), U2 (loader
  vocabulary), U3, U6 (`munmap` sibling profile) and U7.

## 11. Files written

| Path | Change |
|---|---|
| `proof/QCore/DescriptorCustody.lean` | U4: legacy failure arm withdrawn, two theorems restated, `TargetedClose` layer and the domain digest appended (16 new theorems) |
| `proof/QCore/OsEndpointPostState.lean` | one stale doc paragraph; **no definition, statement or proof changed** |
| `proof/AuditCycle19.lean` | 16 new `#print axioms` lines |
| `crates/qcore-core/src/os_endpoint.rs` | `CloseRecord` / `check_close_record` / `CloseRecordRefusal` and 8 tests |
| `crates/qcore-core/src/lib.rs` | three re-exports |
| `crates/qcore-core/src/event_reactor.rs` | U4-R: `DescriptorCustody::Poisoned` and its three refusal sites |
| `crates/qcore-core/tests/unit/event_reactor.rs` | 1 test |
| `crates/qcore-core/tests/os_endpoint_correspondence.rs` | **new** — the Rust half of the agreement pin |
| `contract/qcore/monkey/composition.qcore` | U5: the corrected `evutil_closesocket` error arm, with the pinned-source excerpt and both consequences recorded in comments |
| `contract/qcore/monkey/descriptor_observation.qcore` | comment only |
| `compiler/tests/qcore_contract_corpus.rs` | the deliberate `4 → 8` pin update with its reason, and 1 new test |
| `docs/reports/remaining-o03-u4u5/README.md` | this record |

`AGENTS.md`, `CLAUDE.md`, `README.md`, `DEVELOPMENT.md`,
`docs/reevaluation/ct_manifest.json`, `docs/reviews/r0_3/acceptance.json` and
`docs/reviews/r0_3/acceptance_matrix.json` were not edited; §12 proposes the
ledger text instead.

## 12. Proposed ledger text (for the integrator to consolidate)

> **O03's two declared-model defects repaired (O03-U4/U5):** the
> `QCore.DescriptorCustody` close clause is target-qualified. Its historical
> four-field `Close` record keeps its shape, its success arm and every name
> `QCore.MonkeyEvent` re-exports, but its "returns on failure" arm — which both
> rejected a truthful record of a real Linux failure and modelled the epoch as
> usable, licensing the documented retry hazard — is **withdrawn**;
> `accepted_failed_close_retains_custody` keeps its name and now concludes
> `False`, a refusal exhibited by the concrete record the old clause accepted.
> Failed closes are represented by a new `TargetedClose` whose validity is
> decided by `QCore.OsEndpointPostState.closePostState` itself, so the declared
> model cannot drift from the relation; retention on a failure is admitted only
> on POSIX.1-2024 `EINTR`, proved over the whole `3 × 64 × 10` domain, and with
> no pinned target every failed close is poisoned with no permitted recovery.
> 16 new audited theorems, no new axiom. The one Rust route that mirrored the
> old rule — the Cycle 18 reactor's `CloseSocket` failure branch, which
> **accepted** a failed close followed by a successful retry — now poisons the
> number: retry, use, reuse and bufferevent attachment are each refused at their
> own operation under the existing `L03DescriptorCustody`, with the terminal
> sweep unchanged. A new whole-domain digest (`1920` triples,
> `11372109764820645253`) pins the Rust `close_poststate` and the Lean
> `closePostState` to identical answers, closing O03's recorded
> no-correspondence gap in the F01 style; it is a comparison device, not an
> extraction, and neither side is shown faithful to the kernel.
> `contract/qcore/monkey/composition.qcore`'s `evutil_closesocket` error arm no
> longer asserts preserved custody: read at pinned `d82464a2`, the function is a
> thin forwarder that translates no error, so it is `close(2)` on the audited
> build and reports through `errno` there and `WSAGetLastError()` on Windows;
> the arm now states an unresolved endpoint, the reuse and retry prohibitions
> and the platform error channel, with durability separated on every exit. One
> deliberate pin update (that declaration's clause count, 4 → 8, recorded in the
> table's own comment); the two archived Cycle 19 evidence packages keep their
> pre-repair digest snapshots, and the change is measured inert for the only
> live consumer. Nothing is promoted, no diagnostic code moved, no gate was
> weakened, and the contract corpus stays 60 modules. Gate: 1,664 workspace
> tests (2 ignored), 58 provider tests, 108 Python tests, 38 Lean audits, both
> reevaluation examples exit 0, fmt and clippy clean. Evidence:
> `docs/reports/remaining-o03-u4u5/README.md`.

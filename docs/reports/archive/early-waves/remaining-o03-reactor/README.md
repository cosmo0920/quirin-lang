# O03-REACTOR — a declared close-failure premise for the reactor — 2026-09-09

Follow-on **O03-U4-REACTOR-TARGET**, recorded open by
[`docs/reports/remaining-o03-u4u5/README.md`](../remaining-o03-u4u5/README.md)
§3 and §10:

> The Cycle 18 event vocabulary carries no target and no `errno`, so the reactor
> necessarily answers with the unpinned join. It can never conclude the Linux
> `Released` arm and never separate `EBADF` from `EIO`.

This card extends the vocabulary so a `CloseSocket` failure **may** carry a
declared `(target, errno, record-locks)` premise, and makes the reactor answer
with the relation's classification for that pinned target instead of the
unpinned join. Absent a declaration nothing moves.

Nothing here observes a platform. The declaration is a **declared premise**, in
the same sense as this project's profile data: supplied as text by a declaring
adapter, resolved total-or-reject, copied into the certificate as a named
premise, and rederived by an independent recheck. No `errno` is read, no
syscall is executed, and no verdict is promoted.

---

## 1. The vocabulary

`qcore_core::event_reactor::EventOperation::CloseSocket` gains one field:

```rust
CloseSocket {
    descriptor: DescriptorId,
    succeeds: bool,
    declared_failure: Option<DeclaredCloseFailure>,
}

pub struct DeclaredCloseFailure {
    pub target: String,       // DECLARED_CLOSE_TARGET_SPELLINGS
    pub errno: String,        // DECLARED_CLOSE_ERRNO_SPELLINGS
    pub record_locks: String, // DECLARED_CLOSE_RECORD_LOCK_SPELLINGS
}
```

Three closed vocabularies, each resolved total-or-reject:

| Component | Accepted spellings | Resolves to |
|---|---|---|
| `target` | `linux`, `posix-2024`, `unpinned` | `os_endpoint::Target` |
| `errno` | `ebadf`, `eintr`, `eio`, `enospc`, `edquot`, `other`, `unobserved` | `os_endpoint::Errno` |
| `record_locks` | `none-held`, `unknown` | premise A7 (`Premises::no_record_locks`) |

`record_locks` is separate on purpose. A7 — *the process holds no POSIX record
locks on the file* — is consumed by the **whole** relation, because closing
removes every record lock the process holds on the file whichever descriptor
took it, an effect outside all three modelled identities. Folding it silently
into the declaration would have hidden a premise the answer rests on; declaring
`unknown` is not a refusal but keeps the poisoned answer on every target.

### 1.1 The premises the reactor supplies, and why

`ClassifiedCloseFailure::premises` is not a default and not a grant. It is:

| Premise | Value | Ground |
|---|---|---|
| A2 `epoch_live` | `true` | this arm is reached only with the number under this trace's socket-pair custody |
| A3 `sole_custodian` | `true` | the reactor's descriptor model is exclusive; a number in the custody map has one owner and cannot be re-created while live. It affects only the `Zero` outcome, so it changes no answer on this branch |
| A4 `result_observed` | `true` | the trace records the `-1` return (`succeeds: false`) |
| A5 `errno_observed` | `true` | the declaration names the errno class; declaring `unobserved` still poisons |
| A6 `last_reference` | **`false`** | the reactor models no open file description and must not claim one was released |
| A7 `no_record_locks` | declared | the declaration's own spelling |

## 2. The classification, per target and errno

The reactor decides nothing of its own: it calls
`os_endpoint::close_poststate(target, premises, MinusOne(errno))` and reads
`PostState::descriptor` plus the permitted recovery.

| `descriptor` | permitted recovery | Reactor disposition |
|---|---|---|
| `Released` | — | `ReleasedNumberRetired` |
| `Retained` | contains `Close` | `RetainedRetryPermitted` |
| anything else | — | `Poisoned` |

Which gives, with `record_locks: none-held`:

| errno | `linux` | `posix-2024` | `unpinned` |
|---|---|---|---|
| `ebadf` | Poisoned | Poisoned | Poisoned |
| `eintr` | **Released, number retired** | **Retained, one retry permitted** | Poisoned |
| `eio` | **Released, number retired** | Poisoned | Poisoned |
| `enospc` | **Released, number retired** | Poisoned | Poisoned |
| `edquot` | **Released, number retired** | Poisoned | Poisoned |
| `other` | **Released, number retired** | Poisoned | Poisoned |
| `unobserved` | Poisoned | Poisoned | Poisoned |

With `record_locks: unknown` every cell is `Poisoned`. With any unrecognized
spelling in any component the **event is refused**
(`EventDiagnostic::L03DescriptorCustody` at its own operation index) and the
number is poisoned as well.

What each disposition means in the reactor's own model:

* **Released, number retired** — the epoch ended, so descriptor custody is
  discharged and the terminal sweep no longer refuses; the number joins
  `closed_descriptors`, exactly as a successful close retires it, so a later
  retry, `GetSocketError`, `CreateSocketPair` or `CreateBufferevent` on that
  number is still refused. The relation's `ReportPossibleDataLoss` duty travels
  in the certificate: a release is never a durability claim.
* **Retained, one retry permitted** — the epoch is live and still owed. Custody
  is unchanged, so the terminal sweep still requires a close, and the retry
  POSIX.1-2024 mandates is admitted. This is the only row where it is.
* **Poisoned** — unchanged from O03-U4: neither usable, retriable nor free.

### 2.1 Two directions, stated plainly

This card is not uniformly more refusing, and that is its point. Under a
declared premise two program shapes are now **accepted** that the pre-card
reactor refused: a failed close pinned to a releasing Linux errno, and a
POSIX.1-2024 `EINTR` failure followed by a successful retry. Both acceptances
rest entirely on a premise a human declared, which is recorded by name in the
certificate and rederived by the recheck. Every other direction is unchanged or
strictly more refusing: an unresolvable declaration now produces a diagnostic
where none existed, and a failure premise on a `CloseSocket` **success** branch
— a contradictory event — is refused and poisons the number rather than closing
it.

**The default is not weakened.** With `declared_failure: None` the answer is the
unpinned one, byte for byte: the number is poisoned, the terminal sweep refuses,
and retry, use and reuse are each refused at their own operation index. O03-U4's
`a_failed_close_agrees_with_the_unpinned_endpoint_relation` is unedited and
passes.

## 3. The certificate premise and the independent recheck

`EventCertificate` gains `declared_close_premises: Vec<DeclaredClosePremise>`,
empty for every program written before this vocabulary existed. Each entry
carries the operation index, the descriptor, the declaration verbatim, the
resolved `(target, errno)`, the premise vector, the **whole** derived
`PostState` and the disposition — so a reader sees which platform claim a
verdict rests on and what the relation answered for it.

`recheck_declared_close_premises(program, certificate)` does not inspect the
recorded classification for plausibility. It re-resolves the program's own
spellings, re-derives the post-state from `close_poststate`, recomputes the
disposition, and requires the recorded inventory to be exactly that, in order.
Refused: a substituted declaration, a substituted target/errno/premise vector/
descriptor state/duty set/permitted recovery/disposition, a substituted
descriptor or operation index, an invented or dropped premise, a program whose
declaration the reactor refuses, and a declaration on a success arm.
`compiler`'s Libevent facade re-exports it and contributes no authority: it
converts its own program and the kernel rederives everything.

## 4. Digest correspondence: old and new

O03-U4 pinned one digest over the relation's `3 × 64 × 10` domain. The reactor's
classification is a second implementation surface, so the correspondence grows
to cover it. The Rust half is
`crates/qcore-core/tests/os_endpoint_correspondence.rs`; the Lean half is
`QCore.DescriptorCustody`. Both enumerate the same rows in the same order,
encode with the same arithmetic and compute the same numbers independently.

| Pin | Domain | Value | Rust | Lean |
|---|---|---|---|---|
| relation (**unchanged**) | 1920 triples | `11372109764820645253` | `the_relation_agrees_with_the_lean_model_over_the_whole_domain` | `close_relation_digest` |
| relation + reactor arm (**new**) | 2016 rows | `2640189996995932678` | `the_reactor_arm_agrees_with_the_lean_model_over_the_whole_domain` | `close_admission_digest` |

The old pin is deliberately **kept rather than replaced**: the relation itself
did not change, so its number must not move, and keeping it proves that. The new
digest is the old fold *continued* over the 96 reactor rows (`4 × 8 × 3`), with
inputs offset by `4096` so the two halves cannot be confused — a test asserts
the offset exceeds every relation input.

The reactor domain uses the three closed alphabets plus **one representative
unrecognized spelling** per component. That is faithful because every
unrecognized spelling takes the same refusal, which is pinned separately over
nine concrete spellings (`Linux`, `posix2024`, `" linux"`, `""`, `EIO`,
`eagain`, `none_held`, …) by
`an_unrecognized_declared_spelling_refuses_the_event`.

**What a digest is not.** A comparison device, not a proof device. Two different
encodings agreeing would be a collision; the Rust encoding is transcribed by
hand from the Lean one, not extracted from it; and neither side is thereby shown
faithful to the kernel, which stays a premise read from the man page.

## 5. Lean delta

All in `QCore.DescriptorCustody`, appended after the existing digest section, so
every auto-generated instance name `QCore.MonkeyEvent` re-exports keeps its
number. `MonkeyEvent.lean` and `AuditGenericNames.lean` are unedited and pass.
**No existing theorem statement changed**; `AuditCycle19.lean` gains nine
`#print axioms` lines. No new axiom, no `sorry`, no placeholder — every new
theorem depends only on `propext` / `Quot.sound`.

New declarations: `DeclaredTarget`, `DeclaredErrno`, `DeclaredLocks`, the three
`resolveDeclared*`, `reactorPremises`, `Disposition`, `dispositionOf`,
`reactorDisposition`, the three `allDeclared*` lists, `reactorDomainRows`, three
membership lemmas, `reactorAll` / `of_reactorAll`, the digest encoders and
`closeAdmissionDigest`.

| Theorem | Content |
|---|---|
| `the_reactor_reads_its_disposition_out_of_the_relation` | over the whole `4 × 8 × 3` domain: the disposition **is** `dispositionOf` applied to the relation's own answer, so the reactor holds no independent policy |
| `only_a_declared_posix2024_eintr_retains_the_epoch` | filtering the whole domain for retention yields exactly `[(posix2024, eintr, noneHeld)]` |
| `a_declared_linux_failure_releases_except_on_ebadf_or_an_unobserved_errno` | the Linux column of §2, decided |
| `a_declared_unpinned_failure_is_poisoned` | declaring `unpinned` and declaring nothing are the same answer |
| `a_denied_record_lock_premise_is_poisoned_everywhere` | A7 declared `unknown` poisons on every target and errno |
| `an_unrecognized_declaration_is_refused_and_nothing_else_is` | both directions: unrecognized refuses, recognized never does |
| `a_released_declaration_never_permits_a_retry` | a release never permits the documented-hazard retry, and never discharges the data duty |
| `close_admission_domain_size` | `1920 + 96 = 2016` |
| `close_admission_digest` | §4, proved from `close_relation_digest` so the relation's own pin is load-bearing |

## 6. Schema check, done first

**The event record is not schema-borne, so `qcore-schemas` is untouched and
stays 8.0.0.** `qcore_schemas::reactor_program::ReactorOperationRecord` has no
close variant at all — its own header records that "raw socket-descriptor
custody … have no record here, so a program that uses them cannot be projected
at all" — and `qcore_verification::reactor_source::operation` refuses every
`CloseSocket` with `UnsupportedOperation`. A declared premise therefore cannot
travel silently to a provider or be dropped on the way. A new mutation row in
`crates/qcore-verification/tests/reactor_source.rs` pins that the **declared**
form is refused too, alongside the existing undeclared one.

## 7. Version movement, flagged

`qcore-compiler` moves **0.3.0 → 0.4.0**: the public
`event_driven::EventOperation::CloseSocket` variant gains a field and the
re-exported `EventCertificate` gains one, both source-incompatible additions.
`crates/qcorec`'s dependency requirement moves with it. The Libevent facade is
kept lossless — it transports the declaration unchanged rather than becoming
unable to express what the kernel accepts — while establishing nothing about a
platform itself. `qcore-core` keeps `0.1.0` (it has never been versioned).

## 8. Files written

| Path | Change |
|---|---|
| `crates/qcore-core/src/event_reactor.rs` | the declared vocabulary, `classify_declared_close_failure`, the certificate premise, the `CloseSocket` arm and `recheck_declared_close_premises` |
| `crates/qcore-core/tests/unit/event_reactor.rs` | 7 new tests (the existing 5 unedited except for `declared_failure: None`) |
| `crates/qcore-core/tests/os_endpoint_correspondence.rs` | 2 new tests plus the extended `recompute_pins` |
| `crates/qcore-verification/tests/reactor_source.rs` | 1 new mutation row |
| `compiler/src/verifier_plugins/libevent/event_driven.rs` | facade field, conversion, re-exports and the delegating recheck |
| `compiler/src/lib.rs` | re-exports |
| `compiler/Cargo.toml`, `crates/qcorec/Cargo.toml` | 0.3.0 → 0.4.0 |
| `compiler/tests/{unit/event_driven.rs,workspace_layering.rs}` | `declared_failure: None` at existing construction sites |
| `proof/QCore/DescriptorCustody.lean` | the reactor arm and the extended digest |
| `proof/AuditCycle19.lean` | 9 new `#print axioms` lines |
| `docs/reports/remaining-o03-reactor/README.md` | this record |

`AGENTS.md`, `CLAUDE.md`, `README.md`, `DEVELOPMENT.md`,
`docs/reevaluation/ct_manifest.json`, `docs/reviews/r0_3/acceptance.json` and
`docs/reviews/r0_3/acceptance_matrix.json` were **not** edited; §11 proposes the
ledger text instead.

## 9. Gate

Linux x86_64, Lean via `PATH="$HOME/.elan/bin:$PATH"`, Libevent submodule
initialized at `d82464a2`.

| Gate | Observed | Exit |
|---|---|---|
| `cargo test --workspace --no-fail-fast` | **1,897 passed / 1 failed / 4 ignored** (+9 tests over the wave-9 baseline of 1,888) | see below |
| `cargo fmt --all -- --check` | clean | 0 |
| `cargo clippy --all-targets -- -D warnings` | clean | 0 |
| `cargo run -p qcore-compiler --example reevaluation_probe` | ran | 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | ran | 0 |
| `python3 scripts/check_lean_audits.py` | **48 audits passed** | 0 |
| `cargo test --manifest-path providers/Cargo.toml --workspace` | **75 passed** (single-threaded; see below) | 0 |
| `python3 -B -m unittest discover -s scripts/tests` | **125 ran, OK** | 0 |

**Three load-driven failures, recorded rather than hidden. No assertion was
weakened.** The host was carrying a load average of 33–65 on 16 cores during
this run (a parallel delegated wave).

* `cycle20_verifier_plugins::vp04_vp07_vp08_process_lifecycle_and_payload_budgets`
  failed its `elapsed < 500 ms` wall-clock assertion — the failure the
  delegation brief names as load-flaky (P02-FLAKE). It failed 3/3 at load 64
  and passed 2/2 once the load fell to 42. It touches nothing this card changed.
* `qcore-provider-conformance`'s
  `pinned_second_codebase_runs_configured_process_checks_without_a_rust_import_adapter`
  and `configured_separate_helper_reaches_real_process_frame_execution` failed
  with provider **timeouts** (`QP006`) under the default parallel harness. Both
  targets pass 7/7 and 5/5 with `--test-threads=1`. Neither route touches the
  reactor: they exercise the configured C-gateway audit and the Boolean-field
  process provider.

## 10. Open boundaries

* **No adapter declares anything yet.** The Libevent adapter transports the
  declaration but establishes no platform and no `errno`: `evutil_closesocket`
  is `close(2)` only on non-Windows builds, and the source model carries no
  errno at all. Deriving a declaration from a build profile plus a source-visible
  errno read — rather than accepting it as declared text — is **O03-REACTOR-SRC**
  and is not done here.
* **The declaration is trusted.** A wrong declaration produces a wrong answer.
  Nothing checks that the program really ran on the declared platform or really
  observed the declared errno; the recheck only ties the answer to the
  declaration, and the certificate makes the claim visible. This is the same
  trust class as declared profile data, and it is why a release arm is reachable
  at all.
* **The Linux arms are still read from documentation.** O03's follow-on **U7**
  is unchanged: the `EINTR` and `EIO`/`ENOSPC`/`EDQUOT` arms are unmeasured, so
  a declared Linux release rests on the man page for exactly those rows.
* **A7 and A3 are asserted, not derived.** `sole_custodian` is justified inside
  the reactor's closed-world custody map and affects no answer on this branch;
  `no_record_locks` is entirely declared.
* **No production consumer.** `close_poststate` still feeds no analyzer, driver,
  contract or audit route (O03's **U3**, blocked on relational branch analysis).
  This card widens a model vocabulary; it wires nothing in.
* **No CT movement, no promotion.** CT coverage stays 30/50 mapped, the contract
  corpus stays 60 modules, the pinned 185-unit artifacts are untouched and were
  not re-measured (nothing on the C gateway or audit route changed), and raw
  Cycle 19 units stay 2 Safe / 42 provisional Unsafe / 137 Unknown plus four
  data-only units. No diagnostic code was added, removed or changed.

## 11. Proposed ledger text (for the integrator to consolidate)

> **Declared reactor close-failure premises (O03-REACTOR, closing
> O03-U4-REACTOR-TARGET):** `EventOperation::CloseSocket` may now carry an
> optional declared `(target, errno, record-locks)` premise — declared text
> resolved through three closed vocabularies, never an observation — and the
> reactor then answers with `os_endpoint::close_poststate`'s classification for
> that pinned target instead of the unpinned join: POSIX.1-2024 `EINTR` retains
> the epoch and admits exactly one retry, the five releasing Linux errno classes
> release it and retire the number, and `EBADF`, an unobserved errno, a denied
> A7 record-lock premise and every unpinned row stay poisoned. Absence is not a
> default: with no declaration the answer is byte-for-byte the pre-card poisoned
> one, an unrecognized spelling in any component refuses the event rather than
> defaulting, and a failure premise on the success branch is refused as
> contradictory. Two program shapes the pre-card reactor refused are now
> accepted, both resting on a named declared premise recorded in the
> certificate: `EventCertificate` carries every declaration with the whole
> derived post-state, and `recheck_declared_close_premises` rederives the
> resolution, the post-state and the disposition from the relation, refusing
> substituted declarations, classifications, descriptors, indices and
> inventories. The digest correspondence grows to cover the new arm: the
> relation's 1920-triple pin `11372109764820645253` is deliberately kept
> unchanged, and a new 2016-row pin `2640189996995932678` continues that fold
> over the reactor's `4 × 8 × 3` declared domain, computed independently in
> Rust and Lean. Nine new audited theorems in `QCore.DescriptorCustody`, no new
> axiom, no existing statement moved. `qcore-schemas` is untouched and stays
> 8.0.0 — the close operation has no wire record and is refused at projection,
> now pinned for the declared form too; `qcore-compiler` moves 0.3.0 → 0.4.0 for
> the two source-incompatible public additions. Nothing is promoted: no
> diagnostic code moved, CT coverage stays 30/50, the contract corpus stays 60
> modules and the pinned 185-unit artifacts are untouched. Gate: 1,897 workspace
> tests (4 ignored, one load-flaky wall-clock failure recorded), 75 provider
> tests, 125 Python tests, 48 Lean audits, both reevaluation examples exit 0,
> fmt and clippy clean. Evidence:
> `docs/reports/remaining-o03-reactor/README.md`.

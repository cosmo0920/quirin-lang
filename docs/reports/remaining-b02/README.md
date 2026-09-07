# B02 — Stale-review delta ledger for r0.3 requirements R01–R10

**Date:** 2026-09-07. **Worktree HEAD:** `96ffd5e`.
**Machine-readable record:** [`ledger.json`](ledger.json).

This card classifies each requirement in the supplied revision-0.3 review
([`docs/reviews/r0_3/supplied/REVIEW.md`](../../reviews/r0_3/supplied/REVIEW.md))
against the current implementation, using tests and probes actually executed in
this worktree. It is a report-only card: no tracked file was modified, no
semantic patch was written, and no verdict was promoted. Full language
conformance remains **false** and raw Cycle 19 units remain **unpromoted**.

A historical "open" status in the supplied review is not by itself grounds to
reopen a finding. Where current regressions demonstrate the repair, the
requirement is classified `verified-fixed` and the tests are cited. Where the
requirement's subject lies outside the currently admitted profile, it is
classified `unsupported-by-profile` with the exact gate that rejects it.

## Revision boundary

`docs/reviews/r0_3/acceptance.json` names `e3e7c2c` as the latest verified
implementation revision. HEAD `96ffd5e` is `6c2782f` plus two documentation-only
report commits (B00 `6072947`, B01 `96ffd5e`). No implementation commit landed
after `e3e7c2c`, so the acceptance record's checkpoint is still the right one to
measure against.

## The review's own reproduction commands are inapplicable

The supplied review advertises:

```sh
cargo run --manifest-path docs/reviews/r0_3/probe/Cargo.toml
cargo run --manifest-path docs/reviews/r0_3/probe/Cargo.toml -- --require-conformance
```

`docs/reviews/r0_3/probe/` does not exist. This is not a regression:
`docs/reviews/r0_3/REVIEW.md` lines 8–10 already record that the advertised
archive, Rust probes and validation logs were never supplied, and the review
itself states the probe package "was not run here" and "the probe's own build
remains unverified."

For R01/R02/R03/R06 the probe is superseded by in-tree regression tests. For
R04/R05/R07 the review's *minimal source probes* were reconstructed verbatim and
run through the current `qcorec` CLI; those results are reported below and are
the strongest evidence in this card, because they exercise the exact sources the
reviewer named.

## Classification table

| ID | Requirement | Classification | Tests run | Result | Remaining boundary |
|---|---|---|---|---|---|
| R01 | Destroyed-generation access and destruction quiescence | **verified-fixed** | `cargo test -p qcore-core --lib concurrency::` | 22 passed, 0 failed | Model-level only; `TrustedExplicitTrace`, no arbitrary-source extraction |
| R02 | Terminal child task execution after join | **verified-fixed** | same, plus `--test ct_corpus` | 22 passed / 4 passed, 0 failed | Source-side task track is 11/50 CT mapped; loans, owned graphs, nested workers open |
| R03 | Explicit mutex and synchronization trust boundary | **unsupported-by-profile** | `cargo test -p qcore-core --lib concurrency::` | 22 passed, 0 failed | No checked-invariant profile exists; boundary carried as named certificate assumptions (H06) |
| R04 | Ordinary source result type and exit compatibility | **verified-fixed** (general result typing: unsupported-by-profile, `Q020`) | `-p qcore-frontend --lib review_`, `-p qcore-compiler --lib review_`, 8 live CLI probes | 3 + 3 passed; `-> bool` now rejects, exit 1 | Only i32/unit are typed; owned/error results reject as unsupported |
| R05 | Unconditional axis declaration validation | **verified-fixed** | `-p qcore-frontend --lib review_`, live CLI probe | 3 passed; probe rejects `Q016`, exit 1 | Closed index/profile limits remain |
| R06 | Exact versus maximum loop custody | **verified-fixed** | `cargo test -p qcore-core --lib resource_accounting::` | 5 passed, 0 failed | Exact counts are supplied premises; no identity discharge; source loops unsupported |
| R07 | Signed decimal minimum literals | **verified-fixed** | `-p qcore-compiler --lib review_`, `-p qcore-contract --lib review_r07`, 4 live probes | 3 + 2 passed; extrema behave correctly | No arbitrary expression or result semantics added |
| R08 | Consumer-specific contract coverage | **verified-fixed** | `--test contract_consumers`, `--test contract_loader --test qcore_contract_corpus` | 1 + 10 + 2 passed, 0 failed | Arbitrary theorem proving and caller-precondition discharge open (H10) |
| R09 | Current specification, commands and diagnostic ownership | **verified-fixed** | `--test module_modes`, plus doc audit | 4 passed, 0 failed; entry points and Q7xx ownership single-direction | Three documentation-quality notes below; code-level diagnostic golden tests remain future work |
| R10 | Artifact/evidence composition and snapshot provenance | **verified-fixed** | `-p qcore-verification --lib`, `--test w3_evidence_v2 --test cycle20_verifier_plugins`, `-p qcore-verifier-host --test diagnostic_attribution`, `--test ct_corpus`, both reevaluation examples | 13 + 8 + 20 + 5 + 4 passed; both examples exit 0 | Native/build provenance is declared trusted; whole-program and source-model refinement open |

**Still-failing: none.** No minimal reproducer for a V-card repair was found.

## Live probes of the review's own minimal sources

Run against `./target/debug/qcorec check`. Every one of these reproduces the
r0.3-required behaviour; the first row is the exact probe R04 named as accepted
on the reviewed snapshot.

| Source | Exit | Diagnostic |
|---|---|---|
| `fn main() -> bool { return 2; }` | 1 | `Q020`: the executable result profile supports only i32 and unit |
| `fn main() -> i32 { return 2; }` | 0 | accepted |
| `fn main() -> Unknown { return 0; }` | 1 | `Q020` (unknown result type) |
| `fn main() -> own<Item> { return 0; }` | 1 | `Q020` (invalid owned-result shape) |
| `fn main() -> unit { return 0; }` | 1 | `Q020`: a unit result requires a bare return |
| `fn main() -> i32 { return; }` | 1 | `Q020`: an i32 result requires a signed 32-bit integer literal return |
| `fn main() -> i32 { if (true) { return 1; } }` | 1 | `Q500`: structured control flow is not in the Cycle 12 executable subset |
| `fn leaf() -> unit { return; } fn main() -> i32 { let x: i32 = leaf(); return 0; }` | 1 | `Q500`: `let` must use `qcore_acquire(...)` in the executable subset |
| `quant axis bad : credit<Missing>; fn main() -> i32 { return 0; }` | 1 | `Q016`: unknown dimension `Missing` |
| `fn main() -> i32 { return -2147483648; }` | 0 | accepted |
| `fn main() -> i32 { return -0; }` | 0 | accepted |
| `fn main() -> i32 { return 2147483648; }` | 1 | `Q020` (out of range) |
| `fn main() -> i32 { return -2147483649; }` | 1 | `Q020` (out of range) |

Two R04 acceptance rows — inconsistent function exits, and call/result signature
mismatch — reject at `Q500` behind the Cycle 12 executable-subset gate rather
than by a result-type check. They are gated fail-closed, but they are not
independently exercised as typed-result checks, and this card does not claim
they are.

## R03: why unsupported-by-profile rather than still-failing

`crates/qcore-core/src/concurrency.rs:205` declares exactly one
`ConcurrencyProfile` variant, `TrustedExplicitTrace`, and `concurrency.rs:547`
stamps it on every certificate. The review's R03 acceptance row "deny
unrelated-lock access in a checked-invariant profile" names a profile that does
not exist, so it is outside the admitted profile rather than a check that fails.
There is no rejection diagnostic here; the boundary is expressed as named
certificate assumptions.

What the review *required as a qualification* — "the certificate must not be
advertised as having established that premise" — is verified. The test
`certificates_name_trusted_model_authority_without_removing_guard_semantics`
asserts that an empty program carries only
`InitialAuthorityAndUniqueLifetimeIds`, a guarded access adds
`GuardFootprintAuthority`, a synchronized transfer adds
`SynchronizedCustodyEdges`, and flipping `synchronized` to `false` makes
`check_concurrent` reject. The review also warned against "repairing" the model
by deleting mutex authorization; `same_mutex_establishes_happens_before` and
`different_mutexes_do_not_hide_a_race` confirm the intended guard semantics were
kept. Checked mutex-footprint invariants remain queued as HANDOFF **H06**.

## R06: the separated evidence vocabulary

`LoopBoundEvidence` now distinguishes `Exact` / `AtMost` / `Terminating` /
`Widened` / `Unbounded` (`crates/qcore-core/src/resource_accounting.rs:15`). For
a nonzero-net body across bounds 0..=2 and both net signs,
`AtMost`/`Terminating`/`Widened` with bound ≥ 1 return
`Err(LoopSummaryError::NonExactCustody)`; bound 0 yields `Flow::zero()`. Mutating
a valid `Exact` certificate to `AtMost` is rejected by `check_loop_summary`
(`NonExactCustody`) and by `loop_ledger_morphism`
(`CategoryError::InvalidLoopSummary`) — the negative mutation test the review
asked for. Zero-net bodies keep `AtMost` and are proved to cover every actual
iteration count in `0..=bound`, with a forged summary rejected as
`InvalidCertificate`. The test file states in-line that the summary API carries
no allocation-identity discharge claim.

## R09 residual documentation notes

These are documentation-quality observations, not defects in the repair. None is
a semantic regression and none is repaired here, since B02 writes no tracked
files.

1. `docs/reports/cycle19-native-contracts/README.md:133,135` still spells
   `cargo run --quiet -p qcore-compiler --bin qcorec`. This is explicitly
   permitted by R09's own recorded boundary ("historical reports retain their
   original provenance") and is **not** counted as drift.
2. `AGENTS.md:294` says `Q700`–`Q703` are core scope diagnostics, while
   `docs/QCORE_SPEC_r0_3.md:418` names only `Q700` and `Q701` as retaining their
   meanings. The implementation is the broader one: `Q700` (resolution failure),
   `Q701` (obligations undischarged), `Q702` (conflicting evidence) and `Q703`
   (unvalidated provider allegation) all exist in
   `crates/qcore-verification/src/driver.rs:165-169` and
   `src/verify_dispatch.rs:40-41,222,230-233`. AGENTS.md is accurate; the spec
   row is narrower than the code. No diagnostic is missing or duplicated.
3. `docs/HANDOFF_2026-09-07.md`'s R01–R10 table is missing its markdown header
   separator row, so it does not render as a table. Cosmetic only.

## Ledger-quality drift found in R04

`acceptance.json`'s `R04.remaining_boundary` and the R04 row in
`docs/HANDOFF_2026-09-07.md` both carry a **verbatim copy of R02's**
ScopedOwnedTasksV1 task-lifetime text, which describes loans, owned graphs and
native join evidence and says nothing about result typing. The correct R04
boundary is the one recorded in this card: i32/unit results are checked, and
every other result type rejects as unsupported-by-profile at `Q020`.

No test depends on that prose, so this is a documentation card, not a V-card
repair. It is recorded here rather than fixed because B02 must not modify
tracked files.

## `unverified_specs/` disposition

`unverified_specs/` **does not exist in this worktree**. It exists in the primary
checkout at `/home/cosmo/Documents/GitHub/qcore-c/unverified_specs`, is untracked
in git, and is not covered by `.gitignore` — which is why it is absent from every
worktree. It holds five files, and all five are **byte-identical (sha256
verified)** to their archived copies under `docs/reviews/r0_3/supplied/`.

| Item | Disposition | Rationale |
|---|---|---|
| `REVIEW.md` | **adopted** | Basis of revision 0.3; archived byte-identically in `supplied/`; local ledger is `docs/reviews/r0_3/REVIEW.md` + `acceptance.json`. Its R01–R10 are the subject of this card. |
| `QCORE_SPEC_r0_3.md` | **adopted-and-amended** | Normative at `docs/QCORE_SPEC_r0_3.md`, which differs from the original by 99 lines recording implemented boundaries (obsolete CLI spelling at :109, Q7xx ownership at :418,:422). Pristine original retained in `supplied/`. |
| `QCORE_CONTRACT_SPEC_r0_3.md` | **adopted-and-amended** | Normative at `docs/QCORE_CONTRACT_SPEC_r0_3.md`, amended with the bounded scalar/unit Boolean-field interpretation. Pristine original retained. |
| `QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` | **adopted-and-amended** | Consumer grammar inventory at `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md`, amended as source-language increments landed. Pristine original retained. |
| `QCORE_DESIGN_GRAMMAR_r0_3.bnf` | **adopted-as-target-only** | Target grammar at `docs/QCORE_DESIGN_GRAMMAR_r0_3.bnf`. HANDOFF records it is "not proof that all constructs execute", so it carries no implementation claim. |

Two advertised items were never supplied and are dispositioned **superseded**:
the `docs/reviews/r0_3/probe/` Rust package (superseded by in-tree regressions
and the live CLI probes above), and the reviewer's original archive,
`acceptance.json`, `source_inventory.json` and `validation.json` (the local
acceptance record and source inventory are new local records per
`docs/reviews/r0_3/REVIEW.md` lines 8–10; the 43-test Python evidence-validator
suite and the standing gate replace the reviewer's `validation.json`).

## Scope statement

Every classification above is bounded by the profile it was measured in. This
card establishes that the ten reviewed requirements have current, executed
regression coverage at their stated boundaries; it does **not** claim full
language conformance, does not promote any raw Cycle 19 unit, and does not widen
the CT mapping, which remains 11 mapped / 39 unmapped as pinned by
`compiler/tests/ct_corpus.rs`.

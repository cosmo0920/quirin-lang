# Remaining CT implementation integration

Integration record, 2026-09-19. Completed combined validation; CT41 mapping remains open.

## Result and boundaries

Six GPT-5.6 Sol medium agents worked in two waves of three: implementation
checkpoints followed by independent completion reviews in the same isolated
card worktrees. The integration branch starts from `b3ec447b`, incorporates
`7d2c1fa6`, and preserves that main snapshot while another task updates main.

| Card | Implemented question | Evidence boundary |
|---|---|---|
| [CT24](../ct24-cleanup-task-composition/README.md) | Does cleanup release an owner while its child loan remains live? | The actual task ledger refuses; after-join and independent-owner controls admit. Success-arm credit evidence is conditional. |
| [CT10](../ct10-source-layout/README.md) | Are simultaneous field writes independent under a declared layout? | Shared bit-field units refuse Q661. Property-only successful spawn/join window; no task-lifetime proof or C emission. |
| [CT41](../ct41-sync-model/README.md) | Does a successful release/acquire observation order a payload write/read? | Dedicated bounded C API with declared invocation counts. The original CT41 case remains unmapped; plain and volatile both lack synchronization. |

Model proofs, case-bound proof stages and native observations remain distinct.
These changes establish no Fluent Bit production verdict. The
[production queue](../../plans/fluent-bit-system-coverage/README.md) now names
candidate transfers and their missing obligations; its source census stays first.

## Review corrections

CT24 preserves the original after-all-joins check for direct task-scope regions;
only the exact parsed spawn-success arm gains the new placement. Cleanup is
bound to its real resource and source position, and its Lean theorem now binds
the cleanup trace to the same singleton action.

CT10's certificate and CLI explicitly carry the successful-window assumption.
Its case credits footprint lowering/checking, not the scoped-task ledger. The
existing footprint checker and shared-memory-location refusal are unchanged.
The first full gate exposed new footprint-stage metadata marked
`not_applicable` on still-unmapped cases; those rows must remain `unassessed`.
The card records that failure and the subsequent correction separately.

CT41 rejects narrowing integer conversions, semantic attributes and assembler
accesses that evade reference counts, and redeclarations that could introduce
another declaration identity for a selected object or function. Static AST
access counts do not prove dynamic invocation counts: those remain declared
bounds in the certificate alongside source/configuration identity and review
premises for extraction and Rust/Lean correspondence. The full gate also
caught a forbidden gateway-to-core dependency: extraction stays in the gateway,
while `qcore-verification` now composes those facts with the core judgment.

## Integration and validation

[Gate](gate.json) and [captured facts](gate-facts.json) record source head
`72a648f169a3d13846df0d0811e08093036b93d3`, retained in this branch's ancestry.
Rust/Lean/Cargo source stayed unchanged through publication; later commits
reconcile documentation and Python scratch fixtures. The source gate ran
09:33:27–09:48:29 UTC on 2026-09-19, followed by final matrix/Python checks.

| Command | Exit | Result |
|---|---:|---|
| `git submodule update --init --recursive` | 0 | pinned libevent initialized |
| `cargo test --workspace --no-fail-fast` | 0 | **4,808 passed, 0 failed, 10 ignored**; 246 summaries over 235 Running targets and 9 doc-test sections |
| `cargo fmt --all -- --check` | 0 | clean |
| `cargo clippy --all-targets -- -D warnings` | 0 | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | completed |
| `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | completed |
| `python3 scripts/check_lean_audits.py` | 0 | **145 audits** |
| `python3 -m unittest discover -s scripts/tests` | 0 | **305 tests**, including post-regeneration verification |
| `clang --version` | 0 | Ubuntu Clang 21.1.8, recorded in this measuring worktree |

Cargo used `CARGO_BUILD_JOBS=2`, `CARGO_PROFILE_DEV_DEBUG=0`,
`CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_INCREMENTAL=0`. Sanitizer-enabled native
checks ran with normal process access; sanitizers were not disabled. Provider
**122** remains carried Unix evidence, not a new measurement. Private logs and
individual exit files are under
`/home/cosmo/Documents/GitHub/qcore-c/private/ct-remaining/integration/private/ct-integration/gate-20260919T093327Z/`.

Each card preserves its failed/incomplete runs and carries an
`integration-gate.json` referencing this combined measurement. CT10's manifest
metadata failure, CT24's lost wrapper exit and CT41's layering failure are not
rewritten as passing runs. No pinned diagnostic or historical evidence was
weakened. The documentation preservation check recovered all **50 rules and
6 hard invariants**; the largest policy page remains below 32 KiB. Local links
and the generated report index pass their checks.

## Reconciliation

The verified union maps **49/50** cases; CT41 is the only unmapped case.
Case-bound proof stays **5/49**, gated sanitized-native **10/49** on Linux/macOS.
Two registered entries, `CT10-BYTE-ENCODING-UNSOUND` and `CT10-REASON-ROW`,
are retired. The first is discharged by actual bit-field storage-unit derivation;
the second is superseded by the executed CT10 mapping, so no new unmapped
reason row is needed. No frozen reason row is deleted. CT24's prospective
name was not a registered entry and is not counted as a retirement.

`CT10-TASK-FAILURE-LIFETIME` is registered for composing the source profile
with real task-ledger failure recovery and matching Lean semantics before it
can publish a scoped-task certificate. `CT41-TYPED-SOURCE-UNDISCRIMINATING`
remains registered with a narrowed, current boundary. Registered follow-ups
therefore move **264 → 263**. The next numbered wave close must carry this
record's resolved CT10 reason decision: the decision-discovery script only
reads numbered close directories. The general dual-tracking issue remains
under `DECISION-ALSO-REGISTERED-AS-FOLLOWUP`.

The before/after residual sweep reports 52 → 53 candidate names, adding only
the deliberately discharged `CT10-BYTE-ENCODING-UNSOUND`; no new unowned
residual was introduced by these retirements. `CT10-REASON-ROW` remains
mentioned by the retained dual-tracking entry, with its resolution recorded
here. These are lexical candidates, not counts of open defects. Private
records: `private/ct-integration/{registered-before.json,residuals-before.txt,residuals-after.txt}`.
The [reconciliation record](reconciliation.json) covers all 21 items. Gate-dependent
items are now applied; the duplicate, ambiguous CT24 profile wording is explicitly
declined in favor of the precise consolidated creator-arm grammar. Unrelated
prior decisions remain deferred with their grounds.

The publication check caught one incorrect reconciliation assertion: the matrix's
reason map retains `_derivation` metadata after its last case retires. The
assertion now checks the empty case list and absence of the CT24 row; evidence
data was unchanged. The failed check and corrected rerun are retained as
`reconciliation.log` and `reconciliation-final.log` in the same private run.

## Rebase onto main

On 2026-09-19 the published branch `a0584a50` was rebased, preserving its
merge topology, onto main `026314c8`. Replayed conflicts retain the reviewed
CT integration results and main’s TSAN evidence. Code, tests, fixtures,
scripts and Lean proofs are byte-identical to the published CT branch.
The [resolution record](rebase-gate/resolution.json) records both identities,
the local backup and the exact pre-evidence tree delta.

The integration section above describes the original publication checkpoint;
its ancestry statement applies to that original branch. Rewriting history maps
its source checkpoint `72a648f1` to `fd673475`. Original gate files remain
unchanged. A fresh [rebase gate](rebase-gate/gate.json) and
[captured facts](rebase-gate/gate-facts.json) measure rebased head `637ef1a3`:
**4,808 Rust passed, 0 failed, 10 ignored; 145 Lean audits; 305 Python tests**.
All commands listed in the original gate table returned 0 again; Rust produced
246 summaries across 235 Running targets and 9 doc-test sections. Python was
verified after matrix regeneration. The same Cargo environment was used,
and Clang was invoked again in this worktree. Provider 122 remains carried
evidence. Private command logs and captured exits are under
`/home/cosmo/Documents/GitHub/qcore-c/private/ct-remaining/integration/private/ct-integration/rebase-gate-20260919T101431Z/`.

Coverage and the 263 registered follow-ups are unchanged. This rebases the CT
branch onto main; it does not merge that branch into main.

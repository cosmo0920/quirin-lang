# B00 — Current-state ledger: checkpoint identity and halt handoff reconciliation

Date: 2026-09-07. Worktree:
`.claude/worktrees/qcore-execution-audit-plan-24520f`. Machine-readable
companion: [`ledger.json`](ledger.json).

This is a read-only reconciliation of repository state against the claims in
`AGENTS.md`, `CLAUDE.md` and `docs/HANDOFF_2026-09-07.md`. It changes no
implementation, promotes no verdict, maps no CT case and rewrites no historical
report. It asserts nothing about test results it did not run: the suite counts
quoted below are quoted as *recorded claims*, not as re-executed evidence.

The subagents-halted instruction in `AGENTS.md`, `CLAUDE.md` and
`docs/HANDOFF_2026-09-07.md` is intact and unmodified by this card. Nothing was
restarted.

## 1. Checkpoint identity

| Item | Value |
|---|---|
| HEAD | `6c2782fa3b3922ed0b17bd6d626d94b5caf64e36` |
| Branch | `claude/qcore-execution-audit-plan-24520f` |
| `main` | `6c2782fa3b3922ed0b17bd6d626d94b5caf64e36` (identical to HEAD) |
| Documented implementation checkpoint | `e3e7c2c` |
| Commits `main` is ahead of `e3e7c2c` | 1 |
| Tracked working tree | clean |

`6c2782f` — "Record halted-agent handoff and reconcile remaining v0.3
requirements" — changed 10 files, +317/−17:

```
AGENTS.md  CLAUDE.md  README.md
docs/CYCLE19_BUILD_INPUTS.md      docs/GENERALIZATION_PLAN.md
docs/HANDOFF_2026-09-07.md        docs/ROADMAP_AFTER_CYCLE_9.md
docs/VALIDATION.md                docs/reviews/r0_3/REVIEW.md
docs/reviews/r0_3/acceptance.json
```

**Classification: documentation-and-bookkeeping-only.** Every path is Markdown
or a `docs/reviews/` acceptance JSON. No `.rs`, `.qcore`, `Cargo.toml`,
`proof/`, `contract/`, `providers/` or `scripts/` path is touched.

**Therefore the distinction the handoff draws is correct and holds today:**
`e3e7c2c` is the last commit that changed implementation; `6c2782f` is the
documentation reconciliation layered on top of it. Any evidence claim carrying
suite counts, audit artifacts or verdicts is properly attributed to `e3e7c2c`.

Working-tree note: `git status --porcelain` reports only two untracked
directories — `docs/reports/remaining-b00/` (this card's deliverable) and
`docs/reports/remaining-b01/` (a concurrently running sibling card, containing
only `logs/` at the time of this reading). No tracked file is modified.

## 2. Submodule revision vs the pinned expectation

| Source | Revision |
|---|---|
| Gitlink recorded in HEAD (`git ls-tree HEAD third_party/`) | `d82464a277d0f42703702c4dfd9af6af38595a83` |
| Submodule checkout HEAD (`git -C third_party/libevent rev-parse HEAD`) | `d82464a277d0f42703702c4dfd9af6af38595a83` |
| Declared pin, `contract/profiles/libevent/profile.json` | `d82464a277d0f42703702c4dfd9af6af38595a83` |
| `git describe --tags` in the checkout | `release-2.2.2-alpha-11-gd82464a2` |
| Checkout dirty? | no |

**All three agree. No discrepancy.**

The asserting test is
`compiler/tests/libevent_submodule.rs::libevent_submodule_matches_the_audited_revision`.
It resolves `LIBEVENT_SUBMODULE_PATH` on the filesystem, calls
`verify_pinned_libevent_source`, and asserts `evidence.commit == pin.git_commit`
plus per-file and per-witness counts drawn from the loaded profile — so it binds
the *checkout contents*, not `git submodule status` output. It additionally
requires `epoll.c` to be covered by the audited file set.

Observed transient, recorded for honesty: the **first** `git submodule status`
invocation of this session printed the `-` (uninitialized) prefix; every
subsequent invocation printed the initialized form (leading space, matching
SHA). The checkout was populated and correct throughout — 113 entries, a `.git`
file pointing at `.git/worktrees/<wt>/modules/third_party/libevent`, and a clean
`git status` inside it. This is recorded as a worktree-local submodule-gitdir
reporting transient, not as a missing or wrong checkout.

## 3. Branch integration

42 local branches. **Every one is an ancestor of `main` (`git merge-base
--is-ancestor` true) and 0 commits ahead of `main`.** The full tip-SHA table is
in [`ledger.json`](ledger.json) under `branches.table`. `main` itself is at
`6c2782f`, equal to `origin/main`.

`codex/cycle19-native-teardown` displays `[ahead 3]` in `git branch -v`; that is
ahead of its *remote tracking branch* (`origin/codex/cycle19-native-teardown` at
`28008b7`), not ahead of `main`. It is fully merged into `main`.

This confirms the handoff's statement that retained worktrees and branches are
history, not evidence of live workers. 30 worktrees remain registered (15 under
`.claude/worktrees/`, 15 under `/tmp/`); none was entered or modified.

## 4. CT inventory

`docs/reevaluation/ct_manifest.json` (schema_version 1) holds 50 cases:
**11 `mapped`, 39 `unmapped`.** This reconciles exactly with the "11/50 mapped,
39 unmapped" reported in `AGENTS.md`, `CLAUDE.md`,
`docs/HANDOFF_2026-09-07.md` and `docs/reviews/r0_3/acceptance.json`
(`ct_mapped: 11`, `ct_unmapped: 39`). No discrepancy.

The 11 mapped IDs are **CT01, CT02, CT12, CT13, CT14, CT15, CT16, CT19, CT20,
CT37, CT50** — identical to the list in the handoff's "Exact CT backlog"
section.

Per-stage evidence (from the manifest's own `stages` fields):

| ID | parser | type_checker | reference_execution | cfg | native_backend | proof | evidence_target |
|---|---|---|---|---|---|---|---|
| CT01 | executed | executed | executed | executed | executed | unassessed | linux |
| CT02 | executed | executed | unassessed | unassessed | unassessed | unassessed | — |
| CT12 | executed | executed | executed | executed | executed | unassessed | linux |
| CT13 | executed | executed | executed | executed | executed | unassessed | linux |
| CT14 | executed | executed | executed | executed | executed | unassessed | linux |
| CT15 | executed | executed | executed | executed | executed | unassessed | linux |
| CT16 | executed | executed | executed | executed | executed | unassessed | linux |
| CT19 | executed | executed | unassessed | unassessed | unassessed | unassessed | — |
| CT20 | executed | executed | executed | executed | executed | unassessed | linux |
| CT37 | executed | executed | executed | executed | executed | unassessed | linux |
| CT50 | executed | executed | executed | executed | executed | unassessed | linux |

Two mapped cases (CT02, CT19) are `Reject` cases executed at parser and type
checker only, which is consistent with their manifest text: the rejected program
is not natively executed, and each names a separate accepted control fixture.
**`proof` is `unassessed` for all 11.** Mapping is not conformance; the manifest
note says so explicitly and this ledger does not upgrade it.

The 39 unmapped IDs are CT03–CT11, CT17, CT18, CT21–CT36, CT38–CT49 (full list
in `ledger.json`).

## 5. Contract corpus

**59 `.qcore` modules on disk** under `contract/qcore/`, matching the reported 59
and the pinned assertion at `compiler/tests/qcore_contract_corpus.rs:364`
(`assert_eq!(sources.len(), 59, "unexpected contract corpus size")`). No
discrepancy.

Distribution: `chunkio` 40, `monkey` 5, `librdkafka` 4, `fluent_bit` 3, `cfl` 2,
`library` 2, `cmetrics` 1, `language` 1, `libc` 1.

The second pin, `EXPECTED_CONTRACTS` in the same test, holds **100 extern
function entries across 32 files, totalling 519 clauses**. The 32 < 59 gap is
*not* a coverage gap: `contract_surface_is_pinned_as_structured_clauses` walks
all 59 modules and panics if any `ExternFunction` declaration lacks a contract,
so the 27 files absent from the table are those declaring no extern functions.
Editing `contract/qcore` requires updating both pins deliberately, per invariant
5.

## 6. Report presence

Referenced from `AGENTS.md`, `CLAUDE.md` and `README.md`: **73 distinct
`docs/reports/<dir>` paths.** Present on disk: **116 directories.**

- **Referenced but missing: 0.** Every referenced report directory exists.
- **Directories lacking `README.md`: 0**, excluding the two in-progress
  directories created this session (`remaining-b00`, `remaining-b01`).
- **Present but unreferenced: 41.** These are superseded Cycle19 and r0.3
  evidence packages (`cycle19-*` ×32, `r0-3-*`/`r03-*` ×5, plus
  `fluent-bit-contracts-2026-09-05`, `generalization-integration-20260907`,
  `w3-observation-trust`, `w3-process-stdin-exit-race`). The status ledgers cite
  only each chain's latest successor. This is retained history, not a defect; no
  historical report was read for rewriting.

Full lists in `ledger.json` under `report_presence`.

## 7. Crate paths after the W1/W4/W5 moves

**Every path named in the AGENTS.md repository map exists** — all 30 checked
paths resolve, including the relocated
`crates/qcore-c-gateway/src/{legacy_c,legacy_summary,typed_source}.rs`, the four
`crates/qcore-verification/src/*.rs` replay modules, and
`crates/qcore-core/src/event_reactor.rs`.

`qcore-audit` is located at **`crates/qcore-audit`**.

Workspace members declared in the root `Cargo.toml` (10):
`crates/qcore-audit`, `compiler`, `crates/qcore-c-gateway`,
`crates/qcore-contract`, `crates/qcore-core`, `crates/qcore-frontend`,
`crates/qcore-schemas`, `crates/qcore-verifier-host`,
`crates/qcore-verification`, `crates/qcorec`. `providers/` is a separate
workspace with six members.

Two documentation findings (not defects in code; recorded, not fixed here — this
card is a ledger and its write scope is this directory only):

- **F1.** The AGENTS.md repository-map table has 25 rows and contains **no row
  for `crates/qcore-audit` or `providers/`.** Both exist and are described only
  in prose further down the file. A reader consulting the map alone will not
  find the L3 audit driver or the provider packages.
- **F2.** AGENTS.md states "The current workspace has nine crates." `crates/`
  holds exactly nine members, but the Cargo workspace declares **ten** (the nine
  plus `compiler`). The sentence is true of the directory and misleading as a
  workspace count.

## 8. Final handoff state

`docs/HANDOFF_2026-09-07.md` (222 lines, added by `6c2782f`, committed, tree
clean) records:

**Claimed done, with its own qualifications attached:** integration of inherited
agent work onto `main`; v0.3 spec/grammar adoption; W0/W1 schema boundary, crate
layering and total-or-reject contract loading; W2 profiles and configured
imports; W3 bounded v2 witnesses with v1 `Safe` still refused; W4 validated
parameter/return, `Out`/`Stored`/`Reallocated`/`Removed` custody, nullable paths,
immutable `Out` aliases and bounded ranked recursion; W5 decomposition plus five
standalone packages; W6 real source/native tasks; W7 diagnostic attribution and
local Lean/CI gates; persistent Cycle19 build inputs.

**Explicitly open:** 13 numbered items H01–H13 in proposed resumption order —
non-Boolean lifecycle/callback source derivation (H01), first-party semantic
extraction for Libevent/CTraces/librdkafka (H02), full registry-composed Fluent
Bit audit (H03), compatibility-route deletion after parity (H04), loans and
fractional shares (H05), checked mutex invariants (H06), general
error/control/nested workers (H07), general W4 aliases and recursive fixed
points (H08), broader compiler/build support (H09), contract consumer
unification (H10), Cycle19 provenance (H11), Cycle18 sanitizer/observation
evidence (H12), hosted CI (H13). Plus all Cycle20 acceptance gates, and R01–R10
each retaining a named boundary.

**Headline counts it records at `e3e7c2c`** (quoted, not re-executed here): 819
workspace tests, 45 provider tests, 43 Python tests, Lean build + 20 axiom
audits, 59 contract modules, schema Rust API 2.0.0; full 185-unit corpus at
**2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only**; full-corpus
process sidecar 3 checked / 93 Unknown; separately configured CMetrics audit
1 checked / 6 Unknown with a provisional Unsafe raw unit. It states these are
different scopes, not additive coverage, and that raw units remain unpromoted.

Cross-checked against `docs/reviews/r0_3/acceptance.json`, which independently
records `latest_verified_implementation_revision: e3e7c2c`, `ct_mapped: 11`,
`ct_unmapped: 39`, `workspace_tests: 819`, `provider_tests: 45`,
`python_evidence_validator_tests: 43`, `full_conformance: false` and
`full_language_conformance: false`. **These agree; the handoff's bookkeeping is
internally consistent with the acceptance record.**

The handoff's closing self-description — "This handoff changes documentation and
acceptance bookkeeping only… it does not promote a requirement, CT case,
provider capability or source verdict" — is **confirmed by the `6c2782f` file
list in §1.**

## Discrepancies found

Everything the status ledgers assert about checkpoint identity, submodule pin,
branch integration, CT counts, corpus size and report presence **reconciles**.
The only findings are documentation-completeness items:

| # | Finding | Severity |
|---|---|---|
| F1 | AGENTS.md repository map omits `crates/qcore-audit` and `providers/` | documentation gap |
| F2 | AGENTS.md "nine crates" is a `crates/` count, not the 10-member workspace count | wording imprecision |
| F3 | First `git submodule status` of a fresh session may print the uninitialized `-` prefix while the checkout is in fact present, clean and at the pin | reporting transient; no action |

No fail-closed gate, pinned expectation, verdict, CT mapping or
Safe/Unsafe/Unknown wording was changed by this card.

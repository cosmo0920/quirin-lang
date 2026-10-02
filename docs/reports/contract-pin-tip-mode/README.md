# CONTRACT-PIN-TIP-MODE: contracts that track the tip of a fix branch

Wave 48 supplemental card (High), 2026-10-02. Base: local `main` at
`73beab5d8` (BEFORE measured there). Branch `contract-pin-tip-mode`.
Prediction frozen first: [PREDICTION.md](PREDICTION.md) (commit `b4957ab28`,
before any Rust code; the Lean draft preceded it, disclosed there).

## Why

SUP04-VARIADIC-UPSTREAM-CANDIDATES: the user confirmed CG26's `mk_vhost_set`
reading in the Monkey repository (the `!value` early return inside the
`va_arg` loop returned without `va_end`). The fix is Monkey commit
`21a2c1908b42596317717436e5ce0420805bd523`, the tip of branch
`cosmo0920-missing-va_end-on-null` (the release candidate). The user asked
for a contract for the **fixed** function, pinned not to one revision but to
the **tip** of the fix branch, through a new tip lock mode (chosen over the
flexible mode and over re-pinning per tip). On 2026-10-02 the user added the
notice requirement: every surface shows that a tip contract targets a future,
unreleased release, `info` in force and `warning` not.

## The tip mode (contract spec C06.2, rule 12b)

The specification is the C06.2 amendment *Tip identities* in
[the contract spec](../../QCORE_CONTRACT_SPEC_r0_3.md#c062-revision-modes-pinned-and-flexible-contracts);
the rule is [source identity 12b](../../agents/source-identity.md#constraint-12b).
No `.qv` grammar changes: the mode lives in the lock.

Design decisions (each is a choice the brief left open):

1. **Lock-only mode, exclusive entries.** The declaration stays pinned; its
   lock entry holds reviewed identities *or* one tip identity, never both, so
   an earlier reviewed revision can never keep a tip contract in force after
   the tip moves.
2. **Lock format version 3**, written only for a lock holding a tip entry (a
   top-level `tip` list); every other lock stays version 2 byte for byte.
   Versions 1 and 2 are still read. A version-2 lock with a tip entry, a
   version-3 lock without one, and version 4 are malformed.
3. **The entry** records function, module, contract digest, repository
   (`name`, informational `url`, absolute `path` where `pin` found it), the
   full refname, an optional free-text `release_label`, and the **reviewed**
   commit and its sites.
4. **Locating on any host**: `QUIRIN_TIP_REPOSITORIES` (`NAME=PATH`, the
   platform's path-list form; a malformed value refuses the run), then the
   recorded path. The URL is never fetched; nothing is cloned.
5. **Read-only resolution**: `rev-parse --verify REF^{commit}`, `cat-file -e`
   and `merge-base --is-ancestor` (the tip must **descend from the reviewed
   commit**: a rewritten branch, or another repository at the path, is not
   tracked), then `ls-tree`/`cat-file --batch` of that commit's blobs. A
   location is accepted only at the top of a repository (`.git`, or a bare
   repository), so Git never reads an enclosing one.
6. **Decision**: in force iff the contract text matches, the tip resolved and
   its sites equal the audited sites, paths included (`tip_decision`, the Lean
   `decision`). Unresolvable (repository-missing, ref-missing, diverged,
   unreadable, no-site) → new reason `tip-unresolved`; resolved but different
   → `source-changed` with the ref and commit in the detail. Both are held
   exactly as any pin mismatch (`contract-pin-mismatch`).
7. **Reviewed versus tracked**: each tip record in the sidecar carries the
   resolution (located by/at, resolved commit, tip sites, reviewed commit,
   `moved_since_review`, `sites_changed_since_review`); an in-force tip is the
   named assumption `tip-tracked:FUNCTION` (`assumptions.tip_tracked`) and its
   inventory status is `tip-in-force`, never `pinned-in-force`.
8. **The notice** names function, repository (and URL), ref, release label
   and resolved commit and states that the contract targets a future,
   unreleased release (a release candidate), not a released version; when the
   tip moved since review it adds that the commit was not reviewed. `info` in
   force, `warning` not (with the reason). Surfaces: `pin --track-tip` and
   every other `pin` run that keeps a tip entry, `pin --check`,
   `pin --verify-contracts` (no source: `info` when the tip resolves on the
   host, `warning` when not), every audit route's tally (`contract revision
   notice [...]` lines; the configured route, which prints no tally, prints
   the lines alone), the sidecar (`tip_notices`, each record's `notice`) and
   `compare` (`tip_tracking`, `compare notice (...)` lines). Presentation
   only: computed after every verdict; a run without a tip entry is
   byte-identical (no new key, unchanged tally).
9. **`pin`**: `--track-tip REF --function NAME` (optional
   `--repository-name`, `--repository-url`, `--release-label`) records an
   entry reviewed at REF's current commit, replacing a reviewed entry;
   `--untrack-tip` converts it back to a reviewed identity of `HEAD`; every
   other run, `--regenerate` and `--drop-revision` keep it as written, a
   changed contract text being reported. `--check` counts "in force through
   the tip" (exit 0 allows it).
10. **No path-prefix mapping** (registered, `TIP-VENDORED-PATH-PREFIX`).

Interaction with C06.1/C06.3/C06.4 is unchanged: a review marker withholds
first; an in-force tip contract is checked against any analysed body like any
declaration in force; a held one leaves the body an undeclared definition.

## Implementation

- `crates/qcore-audit/src/contract_tip.rs` (new): entry, resolver, decision
  table `TIP_VECTORS`, notice; unit tests bind `TIP_VECTORS` to the Lean
  `tipVectors` (`tip_lean_vectors_match`).
- `crates/qcore-audit/src/contract_revision.rs`: lock v3, `PinStanding::
  InForceAtTip` and the tip field of `Mismatch`, `pin_standing` (now taking the
  resolver; the single decision for audits and `pin --check`), `apply_pins`
  (all four audit routes and the arbiter), the section's tip records, counts,
  notices, assumptions and `TIP_MEANING`, the tally, `notice_lines`.
- `crates/qcore-audit/src/contract_pin.rs`: `--track-tip`, `--untrack-tip`,
  tip-aware update, check, verify and regenerate.
- `crates/qcore-audit/src/revision_compare.rs`: `tip_tracking` and its counts.
- `crates/qcore-audit/src/lib.rs`: the configured route prints notices.
- `crates/qcore-audit/src/c_identity.rs`: `SourceTree::read_sites_at`.
- `crates/qcore-schemas/src/contract_revision.rs`: `PinMismatchReason::TipUnresolved`.

## Lean

`QCore.ContractTipMode` (16 theorems, registered in
`proof/AuditContractTipMode.lean`; 277 → 278 audits): a reviewed pin means
what it meant (`a_reviewed_pin_means_what_it_meant`, `effective_mode`); in
force iff the tip resolved to the audited identity
(`a_tip_is_in_force_iff_its_sites_are_the_audited_ones`); an unresolved tip is
never in force (`an_unresolved_tip_is_never_in_force`), nor a moved one with
different sites (`a_tip_whose_sites_differ_is_held`); at the audited sites it
is in force with an info notice; dependants of out-of-force tips are
`Unknown`/`contractPinMismatch`, neither `Safe` nor `Unsafe`; a tip decides
exactly as a reviewed pin at its sites
(`a_tip_decides_as_a_reviewed_pin_at_its_sites`); the notice is info exactly
in force, the label and the notices decide nothing
(`the_release_label_decides_nothing`, `the_notice_changes_no_verdict`); an
in-force tip is a listed assumption and a reviewed pin never is; the decision
table holds and is the model (`tip_vectors_hold`, `the_decision_is_the_model`).

## The contract

`providers/qcore-provider-libevent/contracts/monkey/vhost_set.qv`
(cherry-picked from `fd4a0da6b`). Its clauses were read against the fixed
body (`21a2c190:mk_server/mk_lib.c:509-541`, `mk_vhost_lookup` at :435,
`mk_vhost_set_property` above it): `ctx` and `vid` borrowed, a borrowed
variadic tail (values copied by `mk_string_dup`, keys compared), no result.
Every clause holds on every path; **no clause changed**. The comment gained
what v1 cannot state (the body changes `ctx`'s interior; a `DocumentRoot`
key overwrites `documentroot.data` without releasing an earlier copy — read
only, not validated, not presented as a defect) and the "Revision mode"
section now states the tip-mode reality. Lock entry (format 3): repository
`monkey` (`https://github.com/cosmo0920/monkey`,
`/home/cosmo/Documents/GitHub/monkey`), ref
`refs/heads/cosmo0920-missing-va_end-on-null`, reviewed at `21a2c190`, label
"Monkey release after the va_end fix (release candidate)"
([log](artifacts/track-tip.log)). The module is manifest-selected
(`cycle19-resource-contracts.json`), as the resource-view union requires.
Pins moved deliberately: `contract_loader.rs` 122 → 123 modules, 58 → 59
selected, 262 → 263 v1 declarations, 58 → 59 source assumptions, and
`mk_vhost_set` added to the previous-mode table (borrowed, borrowed, none;
not counted-user neutral); `qcore_contract_corpus.rs` 122 → 123 sources and
`("monkey/vhost_set.qv", "mk_vhost_set", 5)`.

## Controls (`crates/qcore-audit/tests/contract_tip_mode.rs`)

On a Git history of the generic `revision_mode` fixture, the checkout being
both the audited source and the tracked repository (branch `fix`):

| Control | Result |
|---|---|
| tip resolves and matches | in force, `matched_tip_commit` = A, `tip-in-force`, `tip-tracked:box_drop`, info on `pin --track-tip`, `pin --check` (exit 0), audit tally and sidecar |
| verdicts with and without the tip path | report `units` byte-identical to the same contract pinned by a reviewed identity at A (no tip key, no notice there) |
| tip moves with the audited source | in force at B, `moved_since_review`, notice "was not reviewed" |
| tip moved, audited source A | `source-changed` (detail names the ref), warning on tally, sidecar and `pin --check` (exit 1); `user_drop` held with `contract-pin-mismatch`; reviewed `item_free` still `pinned-in-force` |
| ref missing / diverged / repository not located | `tip-unresolved` with `ref-missing` / `diverged` / `repository-missing` (`located_by: environment`), warning |
| unresolvable tip vs missing entry | report `units` byte-identical (`not-in-lock`) |
| malformed `QUIRIN_TIP_REPOSITORIES` | audit refuses |
| `compare` | `tip_tracking.moved` A → B, `compare notice (after) [warning]` |
| lock | kept by `pin` and `--regenerate`, `--untrack-tip` restores the v2 lock byte for byte; v2-with-tip, v3-without-tip, v4 and a relative path refuse |

## Measurements

Corpus arbiter (`66910c10`, `--jobs 4`, scripts in [scripts/](scripts/)):

| Run | Functions | `resource-diagnostics.json` | sidecar | `audit.log` |
|---|---|---|---|---|
| BEFORE, base `73beab5d8` | 647 / 10 / 4,177 | `ed384eb0…` | `af927d8e…` | `39b9601b…` |
| AFTER, tip `8981780cc` (code), fmt-only after | 647 / 10 / 4,177 | `ed384eb0…` | `18dbdf2b…` | `a582bf8c…` |

Custody is **byte-identical**, as predicted: no function moves, no
correction. `chunk-contracts`, `source-effects` and `monkey-source` are
byte-identical. Every other difference is attributed: the sidecar gains the
selected module in `manifest`/`sources`, and in `contract_revision` 132
pinned (131 in force, **1 not in force: `mk_vhost_set`, `source-changed`
against the tip at `21a2c190`**, differing in `include/monkey/mk_lib.h`,
`lib/monkey/include/monkey/mk_lib.h`, `lib/monkey/mk_server/mk_lib.c`,
`mk_server/mk_lib.c`), 0 held call sites, the tip counts, `tip_meaning`, one
**warning** notice and the libevent lock's new digest
([excerpt](artifacts/after-contract-revision-tip.json)); `audit.log` gains the
tip clause and the notice line ([lines](artifacts/after-audit-revision-lines.txt))
besides its output path. The typed-path parity stream differs byte-wise only
in record order and Clang AST node addresses: as a multiset with `"0x…"`
values erased it is identical (559,656 records, `715d0f06…`,
`scripts/parity_multiset.py`).

At the Monkey checkout (`pin --check --source-root
/home/cosmo/Documents/GitHub/monkey --crate qcore-provider-libevent`):
`mk_vhost_set` **in force through the tip** at `21a2c190`, **info** notice,
exit 0 with `--function mk_vhost_set` ([log](artifacts/monkey-check.log));
the 14 other libevent entries, pinned at Fluent Bit paths, are out of force
there (exit 1, [log](artifacts/monkey-check-all.log)), the residual below.
`pin --verify-contracts` lists the tip entry with an info notice
([log](artifacts/verify-contracts.log)). The Monkey repository was only read:
`git status --porcelain` empty and both `HEAD` and the branch at `21a2c190`
after every command.

## Gate

See [gate.json](gate.json) and [gate/](gate/).

## Follow-ups

- Amended `SUP04-VARIADIC-UPSTREAM-CANDIDATES`: the `mk_vhost_set` half is
  confirmed by the user and contracted; the `rd_kafka_cgrp_rejoin` half stays
  unvalidated.
- Amended `PIN-NOT-IN-LOCK-ARM-UNREACHED`: its no-entry arm is now driven.
- Registered `TIP-VENDORED-PATH-PREFIX`, `TIP-NOTICE-ROUTES-UNDRIVEN`,
  `TIP-REPOSITORY-HOST-LOCATION` (689 → 692).

# CONTRACT-SUPERSESSION

Card CONTRACT-SUPERSESSION is the contracts half of seam G of the
[pluggable-cards plan](../../plans/pluggable-cards/README.md) (phase P1). It
implements user decision 9 (contracts compose by newer-stamp supersession) and
decision 9a of 2026-10-26: option A (re-pin every provider once so each
declaration gets a reviewed `pinned_at`) plus a rolling window of three Fluent
Bit revisions' lock sets. The governing text is contract spec
[C06.6](../../QCORE_CONTRACT_SPEC_r0_3.md#c066-contract-supersession-across-bundles).

Base: local main `373e41ee8` (AUDIT-COMPOSITION) merged with the integrator's
CONTRACT-LOCKS landing branch `integrate/amendments-schema` at `1638e581a`
(a fast-forward), which is identity run 1's BASE. The branch later merged
origin/main `e37e6703d`, `8a015cd83` (CFG-RELEASE, EVALUATION-PROFILES) and
`9c58892cf` (MEASURE-TOOL, RECORDS-TOOLING, the CMetrics getter-ownership
closure). The last merge moved `quirin-audit-plugin-measure` to qcore-schemas
22.0.0 and regenerated the CFL and CMetrics `contract-inventory.lock` with
`providers/contracts.py sync`. No verdict changes: no pinned
module declares a function twice, and the identity run below shows every
report unchanged apart from the lock digests the re-pin moves.

## What changed

**Lock format version 4** (`crates/qcore-audit/src/contract_revision.rs`).
The pin locks (`contracts.lock`, `bounds-contracts.lock`,
`outer-contracts.lock`) gain a per-entry `pinned_at` (pinned, flexible and tip
entries) and a per-lock `fluent_bit_revision`, the revision the lock set is
tied to. The plan calls this "lock version 3", but card CONTRACT-PIN-TIP-MODE
had already given version 3 to tip-entry locks, so the stamped format is
version 4. Every lock `pin` writes is version 4. Versions 1 to 3 are still
read for history (frozen reports' inputs, fixtures); their entries carry no
stamp, so they are in force exactly as before under C06.2 but never supersede
(fail-closed). A version below 4 carrying a stamp or a tie is malformed.

**`quirin-audit pin`** (`crates/qcore-audit/src/contract_pin.rs`). Every entry
a run adds or changes carries the run's stamp (`--pinned-at STAMP`, by default
the clock at the run). A re-pin carries a later stamp than the entry it
replaces: an explicit stamp that is not later refuses; a clock stamp that is
not later (two runs in one second) moves to one second after the replaced
stamp. `--stamp-only` is the one-time migration: it stamps every unstamped
entry whose contract text is current, changes no identity, and ties the lock to
the checkout's `HEAD`; a stale lock refuses. `--verify-contracts` (and so
`providers/contracts.py check`) now also requires version 4, a tie and a valid
stamp on every entry, and checks the lock-set window.

**Lock-set window** (`crates/qcore-audit/src/pin_window.rs`,
`providers/pin-lock-window.json`). At most three lock sets: the live one in
`providers/<crate>/` and up to two archived byte for byte under
`providers/pin-lock-sets/<revision>/` with their digests. `pin --window-add REV`
archives the live set and re-ties the live locks to `REV`, and refuses a fourth
set; `--window-retire` drops the oldest archived set (never the live one);
`--window-rotate REV` is the re-base step (retire when full, then add);
`--window-create REV` and `--window-check` create and verify it. Today the
window holds exactly one set, tied to `046accf6aded7906f66a1baf0202da349e521736`;
no past revision is invented.

**Loader supersession** (`crates/qcore-contract/src/contract_supersession.rs`,
`contract_loader.rs`; decision in `crates/qcore-schemas/src/contract_supersession.rs`).
The loader's hard "duplicate function contract" error is replaced. A snapshot
with no duplicate loads exactly as before and reads no stamp. Otherwise
`decide` keeps in force the candidate with the greatest valid stamp when every
candidate is stamped and the newest agree on their text. Every other
declaration is cut out of its module: its bytes become spaces, its line breaks
stay, and a module left empty is dropped. Each cut declaration is retained
(`ResourceContracts::superseded`) with its text, digest and stamp, and those of
the winner. Equal newest stamps on different texts, or any missing, malformed,
stale (taken for another text) or unpinned stamp, withhold the function:
every candidate is cut and retained under `ResourceContracts::contract_conflict`.
The audit routes pass the stamps of their locks (`LockStamps`, through
`ContractInput::load` and the pinned route); every other caller passes none,
so every duplicate it meets is a conflict. A cut in a module elaborated through
inclusion refuses the selection, because its evidence would no longer replay.
The `contract_revision` section gains a `supersession` part only when it is
non-empty.

**`contract-conflict`** (qcore-schemas 21.0.0 → 22.0.0). The reason category
was added the way AUDIT-COMPOSITION added `composition-conflict`. Its subject
is the analyser and it is not human-resolvable. The crate takes a major bump
under the exhaustive-variant policy, and the twelve manifests and three
lockfiles move to 22.0.0. The classify marker `whose contract bundles conflict`
is spelled by no earlier site. The legacy-C gateway holds every call of a
conflicted function (`legacy_conflict_hold.rs`) as it holds a pin mismatch,
but the row is `Unknown`, never `Pending`. The hold is wired at the three
rewrite sites, the classifier, the reserved-spelling escape and the
dependency reader. Spec additions: C06.6, an L21.1 row, and amendments to
`crates/qcore-schemas/README.md` and `docs/agents/providers.md`.

**Lean** (`proof/QCore/ContractSupersession.lean`, `proof/AuditContractSupersession.lean`,
27 audited theorems). The proofs cover:

- determinism: the decision reads only the set, so it is associative,
  commutative and idempotent on duplicates;
- newest wins, including a strictly newest candidate;
- a tie on different texts is a conflict, as is any unusable stamp, and no
  candidates means no text;
- retention: the text in force is a newest candidate's, every candidate is in
  force or recorded, and the record holds only candidates;
- a conflict is `Unknown`: never `Safe` or `Pending`, never lowering;
- eight shared vectors, which `crates/qcore-schemas/tests/contract_supersession.rs`
  parses out of the Lean file and checks against the Rust `decide`.

`QCore.ReasonCategory` gains `contractConflict` and the family
`heldCallContractConflict`. The roots were regenerated with
`scripts/gen_lean_roots.py`.

## Re-pin review

`scripts/repin.sh` ran `pin --stamp-only --pinned-at 2026-10-25T18:10:00Z`
(resource, `--bounds`, `--outer`) at the prepared Fluent Bit checkout of
`046accf6`. It then created the window and ran `--verify-contracts` and
`--window-check`; all six steps exited 0 (logs:
`/media/Data/GitHub/quirin-lang/private/contract-supersession/repin/`).
`046accf6` is the latest Fluent Bit revision the locks review: Fluent Bit
5.1.3, committed 2026-09-25. `66910c10` is 5.1.2, committed 2026-09-04.
`34155955` is a synthetic slice export commit, not an upstream revision.

The brief expected 18 pin-lock diffs. There are **17** pin locks, and all 17
changed: ten resource, five bounds and two outer locks. Together they hold
612 entries: 479 pinned, 132 flexible (libc) and 1 tip (`mk_vhost_set`).
[repin-review.json](repin-review.json) (`scripts/review_repin.py`) shows
every lock passes these checks:

- with `pinned_at`, `fluent_bit_revision` and `lock_version` removed, the
  BASE and TIP documents are equal as JSON;
- no entry gained a field other than `pinned_at`;
- every entry carries the one stamp;
- every lock is tied to `046accf6…` and is version 4;
- no BASE lock had a stamp or tie.

No clause, digest, module or identity changed. After the merge of `9c58892cf`,
`review_repin.py` was rerun against that base: 17 locks, 612 entries, ok. The
BASE pin locks are byte-identical to `1638e581a`'s. `pin --verify-contracts`
exits 0, and every lock lists every current declaration, including the CMetrics
`map_lifetime.qv` change and CFL's new `hash_state.qv`. `providers/contracts.py
check` exited 0 after the merge of `e37e6703d`. The `9c58892cf` merge
regenerated the CFL and CMetrics inventory locks with `contracts.py sync`.

The live eight-home corpus input is `66910c10`, while the lock set is tied to
`046accf6` as decision 9a directs. Moving the live profiles' inputs is
REBASE-CORPUS / EVALUATION-PROFILES work. Every identity at `66910c10` is
kept, so pins stay in force there.

## Identity run

**Result: every report artifact of the pinned eight-home corpus, the
tail-pause slice and the system selection is byte-identical between BASE
`1638e581a` and TIP `53ace1277`, apart from one checked substitution.** In
each route's `resource-diagnostics.contracts.json`, the SHA-256 the audit
records for each pin lock it read (`contract_revision.locks`) moves from
the BASE lock's digest to the re-pinned lock's. After that one-to-one
substitution the file is equal byte for byte. The substitution map is
computed from the two snapshots' 17 lock files and is injective. Ten resource
locks occur, libevent, librdkafka and fluent-bit twice each; no bounds or
outer lock digest reaches a report on these routes. Nothing else differs.
The component sidecars are byte-identical, so the loaded environment's
contract digest did not move. Both sides set `SOURCE_DATE_EPOCH`, so Monkey's
`__DATE__`/`__TIME__` lifetime-source key matched exactly, with no swap.
Record: [identity-run1.json](identity-run1.json) (`scripts/compare_identity.py`).

| Route | Byte-identical | Identical after the lock-digest substitution | Window (UTC) |
|---|---|---|---|
| corpus | 8 (`resource-diagnostics.json`, `.components.json`, `.chunk-contracts.json`, `.source-effects.json`, `monkey-source.json`, `corpus-null.jsonl`, `corpus-null.summary.json`, `corpus-outer.json`) | `resource-diagnostics.contracts.json` (13 occurrences, 10 locks) | 19:18 (queued from 18:40) to 22:03 |
| slice | 7 (same set without Monkey) | `resource-diagnostics.contracts.json` | 22:03 to 23:40 |
| system | 7 | `resource-diagnostics.contracts.json` | 23:40 to 02:01 |

The method is AUDIT-COMPOSITION's (`scripts/`). The steps were:

1. `prepare_identity.sh` made the re-spelled inputs mirror
   (`mirror_inputs.py`) and two snapshots, each a `git archive` with
   `third_party/libevent` cloned at `d82464a2`.
2. It built one release binary per side in its own mount namespace
   (`ns_side.sh`, `unshare -rm`), with the side's snapshot bound at
   `private/cs/side-src` and `QCORE_AUDIT_BUILD_COMMIT=contract-supersession-identity`.
   Binaries: BASE `133c34c3…`, TIP `37427394…`.
3. `run_identity.sh` ran the W48-BU-SINGLE-LEAF-BODIES driver per route,
   BASE and TIP side by side, each under an `audit` slot (7 GiB, `--jobs 3`),
   with `SOURCE_DATE_EPOCH=1790357241` on both sides.

As in AUDIT-COMPOSITION, the corpus driver exited 101 on both sides
(`identity_error`) with `audit_exit` 0. The only changed input is the libevent
clone's `.git/index` stat cache, which the pinned route's own `git` call
refreshed. Slice and system exited 0. Logs and driver records differ only in
binary digests, times, the console's preparation timing line and the
snapshot digests in `invocation.json`, all listed in identity.json. Console
and driver logs are in
`/media/Data/GitHub/quirin-lang/private/contract-supersession/identity-1/`;
the route outputs are in the card worktree's `private/cs/identity-1/`.

**Identity run 2 is incomplete and is not evidence.** After the merge of
`9c58892cf`, the coordinator asked that BASE be the merged main. Run 2 (BASE
`9c58892cf`, TIP `d301d757c`) prepared its snapshots and both release builds
(exit 0). Its corpus route had started at 2026-10-26T07:15Z when the
coordinator asked for the branch to land now, and it was stopped. Logs are in
`/media/Data/GitHub/quirin-lang/private/contract-supersession/{prep-2,identity-2}/`.
No route finished and no comparison was made. Run 1 is the only completed
identity evidence. It measures this card's diff against `1638e581a`.

**Source equivalence after run 1.** Measured TIP `53ace1277`. The branch
then took four changes:

- a fix to the test module of `pin_window.rs` (Clippy);
- documentation;
- origin/main `e37e6703d`, which holds records only;
- origin/main `8a015cd83`, which holds CFG-RELEASE and EVALUATION-PROFILES.

The branch also merged `9c58892cf`, and after the stopped gate it fixed the
two defects that gate found (below). `8a015cd83` changed one code file,
`crates/qcore-audit/src/projection.rs` (CFG-RELEASE). `9c58892cf` brought the
MEASURE-TOOL crate, `crates/qcore-c-gateway` frame-audit changes and the CFL
and CMetrics contracts. Each carries its own evidence. This card's analysis
changes are those measured in run 1, plus the post-gate refusal of a function
declared twice in one module, which restores the base behaviour for that case.
The pin locks are byte-identical between `1638e581a` and `9c58892cf`. Because
run 2 did not complete, identity on the merged tip is **not measured**; the
landing gate and the integrator's measurement are the arbiter.

## Gate

The scoped card gate, `python3 scripts/gate_scope.py --base 8a015cd83 --out
private/gate`, ran on `3a1f44a8e` in full mode, with Lean all and the provider
gate. It was **stopped before completion** when origin/main moved to
`9c58892cf`, so it is not a passing gate. Logs are in the card worktree's
`private/gate/`.

| Step | Exit | Count / note |
|---|---|---|
| submodules | 0 | libevent at `d82464a2` |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| Rust suite (`run_test_binaries.py`, rust-suite slot) | 101 | 558 units; two failures, both fixed below |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| `check_lean_audits.py` (lean slot) | 1 | `QCore.ReasonCategory` still counted 12 categories; fixed below |
| Python suite | 0 | |
| `providers/package.py build`, verify all, verify `qcore.libevent` | 0, 0, 0 | 5 packages |
| provider `cargo test` | 143 | stopped with the gate |

The two Rust failures were the loader tests that pinned the old hard
duplicate error: `compiler/tests/contract_loader.rs`
`duplicate_functions_and_inputs_are_rejected_even_when_identical` and
`contract_inherit_adoption.rs`'s "one module selected twice" refusal. Both now
assert C06.6's behaviour. A function two modules declare is withheld as a
conflict with both texts retained, and a module selected twice withholds its
functions. The review also found that a function declared twice **in one
module** had become a conflict. That is a malformed module, not a composition
of bundles, so `supersede_sources` refuses it again ("declared twice in one
module: duplicate function contract"), as before this card. The Lean count is
13. These four fixes (`proof/QCore/ReasonCategory.lean`,
`crates/qcore-contract/src/contract_supersession.rs`,
`compiler/tests/contract_loader.rs`, `compiler/tests/contract_inherit_adoption.rs`)
landed through the integrator's commit `007060bb0` on
`integrate/amendments-schema`, taken verbatim from this worktree, and not
through this branch.

After the fixes, in this worktree:

- the affected tests pass: `contract_inherit_adoption` 9/9, `qcore-contract`
  and `contract_conflict_hold`;
- `lake build` exits 0 (435 jobs);
- `check_lean_audits.py` exits 0 (376 audits);
- Clippy, fmt and the Python suite (513 tests) passed after the `9c58892cf`
  merge;
- `evaluation_profiles.py check` exits 0 (3 live, 6 frozen);
- `apply_amendments.py --check` exits 0.

`contract_loader` has one remaining failure:
`every_corpus_module_outside_the_v1_vocabulary_is_rejected_not_skipped`
(corpus module count 143, expected 142). It is **inherited from origin/main**:
`9c58892cf` added `providers/quirin-provider-cfl/contracts/cfl/hash_state.qv`
without updating that count. This card does not change it. Logs:
`/media/Data/GitHub/quirin-lang/private/contract-supersession/{fix-tests,lean-full,lean-audits}.log`.
The full gate was not rerun on the final tip.

## Boundaries

- Supersession applies to resource declarations. The bounds and outer views
  keep their own duplicate refusals; their locks carry stamps that no loader
  consults yet.
- Only the legacy-C gateway names the conflict. Another consumer of the
  environment, such as the checked extern-call route, sees an uncontracted
  callee, which is `Unknown` under a different reason.
- A cut inside a module elaborated through inclusion refuses the whole
  selection. This is fail-closed, but it does not decide the duplicate.
- Archived lock sets are records. No audit route can yet select an archived
  set to re-audit a previous revision.
- The later-stamp rule is enforced when `pin` writes. `pin --check` does not
  compare stamps with the committed history.
- The Lean model is abstract: texts, modules and stamps are naturals, and an
  unusable stamp is `none`. Its correspondence to the Rust code rests on the
  parsed vectors and on review (constraint 28).

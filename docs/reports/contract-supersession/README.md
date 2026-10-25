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
(a fast-forward), which is this card's BASE. No verdict changes: no pinned
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

No clause, digest, module or identity changed. `providers/contracts.py check`
exit: see the gate.

The live eight-home corpus input is `66910c10`, while the lock set is tied to
`046accf6` as decision 9a directs. Moving the live profiles' inputs is
REBASE-CORPUS / EVALUATION-PROFILES work. Every identity at `66910c10` is
kept, so pins stay in force there.

## Identity run

IDENTITY-PLACEHOLDER

## Gate

GATE-PLACEHOLDER

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

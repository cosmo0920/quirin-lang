# FB-EVENT-LOOP-RETENTION-AND-REGISTRATION

Card authority: ledger entries `FB-EVENT-LOOP-RETENTION-AND-REGISTRATION` and
`W47-LOOP-OWNER-UNDECLARABLE-OPERATIONS`; contract specification C04.14 /
C04.19 and gateway rules 12t / 12n; rule 10p; the census
[FB-QUANT-CONTRACT-CENSUS](../fb-quant-contract-census/README.md) (P5a). The
census and prediction, frozen before implementation and the pause point it
stopped at, are [PREDICTION.md](PREDICTION.md).

Base: local `main` `8b360766c` (round 30, rebased onto RELATIONAL-REWRITE-LINE-ORDER).

## User decisions (verbatim)

From `w48-common-r30.md` (2026-10-16), decision 1:

> **Event-loop holder:** "Tracked holder path". A holder reached through a member is spelled as a path and tracked by the analysis, e.g. `resource_retained_by(data, ins.evl)`. No assumption is involved.

From the coordinator's message after the pause (USER DECISIONS 2026-10-16):

> **1. Removal: "R3: quant state".** `quant live #loop: #holds(event) |-> #absent;` — `#holds(P)` is a new value-keyword application, under the `#` keyword rule: `#` covers binders and value keywords. **Meaning:** after the call, the loop formal no longer holds the registration of `P`. Every other registration is unchanged. **Composition is exactly your R1 semantics:** a removal drops only the exactly matching record, and an unmatched removal keeps it, so a later release refuses (never Safe, never Unsafe). `mk_event_channel_destroy` is declared without a removal, because its removal condition can't be stated.
>
> **2. Forwarding: `#id` with a where-binding, "Result + list".** `ensures normal resource_callback_member_registered(#id, flb_input_collector.cb_collect, cb_collect) where #id = #result in ins.collectors;` **Meaning:** `#id` is the setter's returned id, naming the collector record linked into `ins->collectors`. A later call taking that id resolves the same record. **Loader rules (total-or-reject):** `#id` must be bound by its `where` clause; `in` takes a holder path (`FORMAL.FIELD`); `#result` must be an integer result. **Body check:** run a forwarding body check in rule 15w's discipline. The body must create exactly one record, link it into that list, store `cb` into the member, and return that record's id. Anything else withdraws the declaration. **Lean parity:** the forwarded registration equals the in-body store, and an id that resolves to no record, or to an ambiguous one, never decides.
>
> **3. Cross-function loop lifetime (T1/T2): "Follow-up card".** Register it with its own DISCHARGE, naming T1 and T2. Narrow both ledger entries as you planned.

## What changed

* **Holder paths** (rule 12t amended, C04.14.3, grammar G05):
  `resource_retained_by(P, FORMAL.FIELD[.FIELD])`, read by position (a record
  tag there is an unknown parameter and rejects). At a call a holder actual
  spelled as a member path below an unrebound formal, or the actual at the
  declared root followed by its members, is a **path holder** when the body
  names the formal only through member accesses off the path
  (`path_stable`, premise `P-HOLDER-PATH-STABLE`); a retained `&X->f` is an
  interior of `X`'s object. A path-held record outlives the frame and refuses
  at an exit unless a removal of exactly it ended it; a release of an identity
  while an interior is held refuses.
* **Removal of one registration** (`quant live #H: #holds(P) |-> #absent;`):
  drops only the record its actuals key, nothing otherwise.
* **Registration forwarded through an id** (rule 12n amended, C04.19.3):
  loader `by_id_clause`, `CallbackClause::MemberRegistered::by_id`; read as
  the setter form at calls and in the run's closure; body check
  `registration_by_id.rs` (helper followed one level), withdrawal otherwise.
* **The access gate** reads a withheld declaration's borrowed positions as the
  function map's (`legacy_call_argument_access.rs`), discharging
  `RETAINING-CALL-MEMBER-ACTUAL`: without it every member actual of a retaining
  call (`coll->evl`, `&coll->event`) was refused before rule 12t saw it.
* **Declarations**, each body read at 66910c10, 046accf6 and the 90978ac6
  export ([`scripts/bodies.py`](scripts/bodies.py),
  [`artifacts/bodies.json`](artifacts/bodies.json)) and pinned at all three
  ([`scripts/pin.sh`](scripts/pin.sh)):
  * `qcore-provider-libevent` `monkey/event_loop.qv`: `mk_event_add`,
    `mk_event_timeout_create`, `mk_event_channel_create` (retain `data` in
    `loop`), `mk_event_del`, `mk_event_timeout_destroy` (`#holds` removals),
    `mk_event_channel_destroy` (no removal), `mk_event_wait` (borrowed); the
    descriptor effects of the four timer/channel operations under the named
    premise `mk_event_descriptors`.
  * `quirin-provider-fluent-bit` `fluent_bit/input_collector_registration.qv`:
    `flb_input_set_collector_time` / `_event` / `_socket`, each a registration
    forwarded through its id. `flb_input_set_collector_server` does not exist
    at any audited revision.
  * Not declared: `flb_sched_timer_cb_create` (a record returned through an
    out slot and linked into `sched->timers`; no decided form) and the
    collector operations by id (no consumer resolves an id yet).
* **Lean**: `QCore.RetentionPath` (12 audited theorems), `QCore.RegistrationById`
  (9); audits 329 -> 331.
* **Tests**: `crates/qcore-contract/tests/event_loop_clauses.rs` (the three forms
  and their rejections, the provider modules load),
  `crates/qcore-c-gateway/tests/event_loop_retention.rs` (a path holder
  removed from its own path, still holding, removed elsewhere, one of two
  removed; an unresolved and a rewritten path; a removal without a record; a
  loop destroyed before deregistration and an object released while held; a
  frame object's interior; a declared path holder; forwarding standing and
  withdrawn), unit tests of `path_stable`, the path vectors and the forwarding
  vectors.

## Measurements

BEFORE: binary `5e3dd883…` built from the git archive of `6590dca2f` (the
base's code); AFTER: binary `ee617404…` built from the archive of the tip
`70f4cff38` (later commits change tests and records only). Each run under the
shared heavy lock with `QCORE_NULL_PROPERTY` ([`scripts/build.sh`](scripts/build.sh),
[`scripts/measure.sh`](scripts/measure.sh)); `run.json`'s `binary_sha256`
equals each binary; identities in [`artifacts/runs.json`](artifacts/runs.json),
row-level moves in `artifacts/compare-{corpus,system,slice}.json`
([`scripts/compare.py`](scripts/compare.py)). The first BEFORE corpus attempt
exited 1 after 763 s (its exported `third_party/libevent` had no `.git`:
`Pin(GitUnavailable)`); it is kept as a failed attempt, not evidence, and
`before2` reran it with a git clone of the pinned submodule.

| Route | Custody before → after | Null before → after | Report |
| --- | --- | --- | --- |
| corpus `66910c10` | 938 / 9 / 3,887 → **939 / 9 / 3,886** | 1,113 / 1,435 / 2,355 / 18 → identical | `de6343c7…` → `1277df07…` |
| system (8 files) | 93 / 4 / 351 → 93 / 4 / 351 | 84 / 125 / 245 → identical | `238bb021…` → `4c295a47…` |
| tail-pause slice | 52 / 1 / 347 / 4 Pending → identical | 71 / 103 / 232 → identical | `f8b3c3b5…` → `5502fc27…` |

The BEFORE figures equal round 30's and main's corpus digest.

**The one status move, read by hand.** `mk_core/mk_event.c::mk_event_timeout_disable`
(Monkey's own source in the corpus) Unknown → **Safe**. Its body is
`return mk_event_del(loop, (struct mk_event *) data);`; its one leaf was
`call to uncontracted function mk_event_del`. The call is now decided under
`mk_event_del`'s declaration (`contract_dependencies`: `mk_event_del`): both
formals borrowed, the result none, and the removal `#holds(event)` drops
nothing because the body holds no record (a cast actual is no spelling a
record keys on). Clauses it rests on: `mk_event_del`'s `resource_borrowed(loop)`,
`resource_borrowed(event)`, `quant live #loop: #holds(event) |-> #absent`,
`resource_return_none()`, a declared assumption pinned at both revisions (the
unit defines `mk_event_del`, whose body is not checked against a removal).
No premise. It is a custody verdict: that the removal itself is right is the
declaration's.

**Leaf trades (no status move).** Corpus 13 rows (Monkey's server: `mk_lib_*`,
`mk_net_*`, `mk_sched_*`, `mk_server_*`, `thread_cb_init_vars`): 23
`call to uncontracted function` leaves of the Monkey operations leave; 11
`indirect memory access` leaves (the access gate's own refusal at a member
actual the hook did not decide: a local's interior, `sched->loop` below a local)
and 5 `unresolved resource identity` leaves (a store or a call through the
formal after the removal) arrive; `mk_net_conn_create` stays provisional
Unsafe. System 17 rows: 21 leave; 4 `retention-undecided` (`flb_sched_create`,
`schedule_request_now`: a caller's loop holding at the exit), 6 gate leaves and
6 `unresolved resource identity` arrive. Slice 15 rows: 27 leave, 19 gate
leaves arrive (`in_tail_init` and `flb_tail_fs_inotify_init`: the setter calls'
scalar member actuals, now read by the gate behind the declared setter;
`flb_tail_fs_stat_init` loses both its leaves). No `Unsafe`, leak or violation
is added on any route, and no Safe row moves: **no correction**.

**Prediction against outcome.** PREDICTION.md predicted at most +1 system Safe
(`flb_sched_timer_cb_disable`), none on the slice and none on the corpus.
Outcome: no system move — `flb_sched_timer_cb_disable`'s one leaf trades
`call to uncontracted function mk_event_timeout_destroy` for the access gate's
`indirect memory access` at the same call (the gate does not admit the
`timer->sched->evl` actual in the real unit; a fixture with the same chain over
smaller records is admitted and decided), and the fixture of its shape
(`tests/event_loop_retention.rs`, `disable_then_store`) shows the store
`timer->timer_fd = -1` after the call refused by an older rule for a plain
borrowed callee too. One corpus gain the prediction did not name
(`mk_event_timeout_disable`, Monkey's own unit). Slice as predicted.

**Obligations.** OB-10, OR-9, C1, C2, C4, C5: no row they map to moves
(`flb_engine_start`, `flb_engine_shutdown`, `flb_sched_*`, `flb_input_*`,
`input_thread`, `flb_destroy` stay Unknown); **none discharges**, as
predicted: each rests on cross-function ordering.

## Ledger

`named_followups` 789 → 790 ([`scripts/update_ledger.py`](scripts/update_ledger.py),
idempotent): retired `RETAINING-CALL-MEMBER-ACTUAL` (its DISCHARGE, the access
gate reading a withheld declaration's borrowed positions as the function map's,
holds and is measured above); narrowed `FB-EVENT-LOOP-RETENTION-AND-REGISTRATION`
(left: the timer registration through an out slot, the collector operations by
id) and `W47-LOOP-OWNER-UNDECLARABLE-OPERATIONS` (left: the loop-owner
obligation of the seven declared operations); registered
`FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME` (decision 3: T1, a run-wide closure per
loop member; T2, retentions forwarded into summaries) and
`MK-EVENT-DESCRIPTOR-CUSTODY` (the premise `mk_event_descriptors`).

## Follow-ons

* `flb_sched_timer_cb_disable` (system) and Monkey's server functions: the
  access gate keeps refusing some member actuals of the declared calls
  (`timer->sched->evl` in the real unit, why not established here), and a
  store through a formal after a call that received a view of it refuses;
  neither is this card's rule, and freeing them needs no event-loop
  vocabulary.
* The pin checks: `pin --check` for `qcore-provider-libevent` exits 1 at every
  revision (`mk_vhost_set` tracks a Monkey release-candidate tip the audited
  source does not match) and for `quirin-provider-fluent-bit` at the 90978ac6
  export (functions recorded only at 66910c10 / 046accf6), both on the base's
  locks too ([`scripts/pin.sh`](scripts/pin.sh)); every pin this card adds is
  in force at all three revisions.

## Rule ids

Amended in place: 12t, 12n (gateway), 10p (one sentence); contract
specification C04.9 (one sentence), C04.14.3 (new subsection), C04.19.3 (one
bullet); grammar G05. No new rule id; `31`–`33` stay free; the bold duplicate
check is empty.

## Gate

Standing gate and provider gate at the rebased tip `7f3956ca4` on base
`aa8d8b030` (main after `b93c8f8b9` restored the consultation log on declined
probes, which the access gate's withheld-declaration lookup reaches, so the
whole gate was rerun), under one exclusive flock of the shared heavy lock for
the whole gate ([`scripts/gate-whole.sh`](scripts/gate-whole.sh),
[`scripts/gate.sh`](scripts/gate.sh), [`scripts/finish-gate.sh`](scripts/finish-gate.sh);
logs `private/fbel/gate-g3` of this worktree; exits in [`gate/`](gate/),
summary [`gate.json`](gate.json)):

| Step | Exit | Count |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | 0 | |
| `git submodule update --init --recursive` | 0 | |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --locked --all-targets -- -D warnings` | 0 | |
| `scripts/run_test_binaries.py` | 0 | 506 units, 519 summaries, 8,176 passed, 0 failed, 15 ignored, no rerun |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| `scripts/check_lean_audits.py` | 0 | 331 audits |
| `python3 -m unittest discover -s scripts/tests` (08) | 0 | 374 tests |
| `build_matrix.py . --gate gate.json` (09a), the Python suite (09b) | 0, 0 | 374 tests |
| `build_docs_index.py`, `--check` | 0, 0 | |
| `providers/package.py build`, `verify qcore.libevent`, `verify qcore.librdkafka` | 0, 0, 0 | |
| `cargo check --locked` in `providers/`, `providers/contracts.py check` | 0, 0 | |
| `cargo test` in `providers/` | 0 | 133 passed, 0 failed |
| `python3 -m unittest discover -s providers/tests` | 0 | 25 |
| provider `fmt --check`, `clippy -D warnings` | 0, 0 | |

Earlier attempts, kept as attempts: `g1` (at `21ca60d8a`, base `8b360766c`)
failed clippy (`map` over `filter_map` in the new route test), three
`provider_selection` module counts (61 -> 63), the Fluent Bit provider crate's
contract index (the new module unlisted) and the Python corpus count
(131 -> 133), repaired in the next commit (tests, the providers workspace's own
index, Python tests: no source the measured `quirin-audit` binary is built
from); `g2` (at `520e7d12b`, same base) passed every step. A targeted check
after the first rebase (onto `a5e4d39d8`, [`scripts/rebase-check.sh`](scripts/rebase-check.sh))
found main's own `QCore.ReturnCodeHandOff` not building, which main fixed in
`aa8d8b030`. Pins: [`scripts/pin.sh`](scripts/pin.sh) pinned all ten
declarations at the three revisions (exit 0 each) and `pin --verify-contracts`
exits 0.

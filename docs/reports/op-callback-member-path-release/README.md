# OP-CALLBACK-MEMBER-PATH-RELEASE: a callback's release or move-out at a member path

Card OP-CALLBACK-MEMBER-PATH-RELEASE (wave 48, card-high). New gateway rule
**30**; contract specification **C04.19.6**; grammar G05/G05d; Lean
`QCore.MemberPathRelease`; `QCore.BodyClauseStanding` gains evidence `30`.
Base: local `main` `39287f12e`. The census, the syntax proposal and the
frozen [prediction](PREDICTION.md) were committed (`0a7bdb325`) at the
brief's pause point, before any implementation code.

## 1. User decision (2026-10-15, verbatim)

"B: state + set" and "Build it now".

The integrator's instructions with it are recorded in C04.19.6: alternative
B of the [prediction](PREDICTION.md) section 3 exactly as proposed (the
four value keywords, the set `{V, #kept}` and no other set, a guard that is
absent or the hand-off's complement, unnamed paths kept, the composition
table exactly, no move destination, the share-fetch producer as a named
premise).

## 2. Census

[PREDICTION.md](PREDICTION.md) section 2 (every body read in full at
`66910c10` and `046accf6`; the two `lib/librdkafka-2.15.0/src` trees are
byte-identical). `rd_kafka_op_destroy` calls the candidate before its
switch, so the only invocation where a member-path state reaches the
caller's state is its `rko_op_cb(_, _, rko)`. librdkafka's own composition
releases nothing twice: every path a candidate empties is one an arm
releases null- or count-guarded, and every path a candidate takes and no
arm releases is asserted null by the arm or never read by it.

## 3. Mechanism

* **Surface** (`crates/qcore-contract/src/contract_member_path.rs`, schema
  `qcore_schemas::member_path`). `quant live #P: TAG.PATH |-> V [when
  #result OP C];`, told apart from a hand-off (`#P: #owner …`) and a result
  state (`#result: …`). `V` is `#released`, `#moved`, `#stale`, `#kept` or
  exactly `{V', #kept}`; every other set rejects (`quant-live-path-set`).
  The binder is a borrowed depth-one pointer formal, or the hand-off's
  `resource_owned` formal; a guard stands only beside a hand-off of the same
  formal and is its complement; one state per path, none below another.
  The four words join the reserved keywords of a `#` form (no parameter in
  the tree beside a `#` form carries one). Stored as
  `RetainingContract::member_paths`; the declaration is withheld from the
  function map.
* **Composition** (rule 30, `legacy_member_path_route.rs`). At a decided
  invocation (the unit's or the run's closure, and a decided registration
  through `decide_member`), a candidate stating member paths reads as its
  custody contract (after rule 12m's caller branch check when it also hands
  its formal); for each tracked object the caller linked at a member path of
  the object handed at a parameter position, each candidate's state is
  composed with the arm join at the path by C04.19.6's table and joined over
  the candidates, **before** `apply_arms`. A path every candidate leaves
  `#kept` is left to the arms as before. A direct call of a member-path
  declaration refuses (`member-path-direct`; the corpus has none).
* **Body check** (`member_path_check.rs`, evidence `30`). Contradictions
  withdraw the states in the defining unit; validation needs the body itself
  to establish every stated reading; anything else is a declared assumption.
  The formal's pointee is read through the unit's typedefs
  (`MemberFacts::record_of_node`): Clang's JSON gives no desugared spelling
  for a pointer to a typedef, and the first measured binary (`a1`), which
  read only the spelled type, withdrew all eight in their units (its 38
  `rdkafka_admin.c` leaves kept naming `rd_kafka_DeleteRecords_leaders_queried_cb`);
  `a2` is the corrected binary measured below.

Tests: `crates/qcore-contract/src/contract_member_path.rs` (every value, the
set, each rejection, the guard spellings),
`crates/qcore-schemas/src/member_path.rs` (the composition vectors, shared
with Lean and checked against its text), `member_path_check.rs` (validated,
assumption and each contradiction), and
`crates/qcore-c-gateway/tests/member_path_release.rs` on route G: a released
path then destroy (`Safe` with and without the arm), a moved path then
destroy (`Safe`), an unstated path (`callback-uncontracted`;
`member-path-unstated` below a stated path), a double-release attempt
(`member-path-double-release`; `member-path-stale` with no arm), the may-set
(`Safe` where the arm releases, `member-path-undecided` where none does), an
unnamed path left to the arms, a direct call, and a body that validates or
contradicts its states.

## 4. Declarations

`qcore-provider-librdkafka/contracts/librdkafka/client.qv`, the card's
section ([`write_contracts.py`](scripts/write_contracts.py)), each naming
`rk_no_plugin_library` (every candidate reaches `rd_kafka_log_buf`):

| candidate | states | standing (corpus) |
| --- | --- | --- |
| `rd_kafka_admin_worker` | hand-off `== KEEP`; `!= KEEP`: `admin_request.eonce \|-> #released`, `args`, `fanout_parent` `\|-> {#moved, #kept}`, `replyq.q \|-> {#released, #kept}` | declared assumption (helpers) |
| `…query_leaders_async_worker_op_cb` | hand-off `== KEEP`; `!= KEEP`: `leaders.eonce`, `leaders.replyq.q` `\|-> #released`, `partitions \|-> {#moved, #kept}`, `opaque \|-> {#stale, #kept}` | declared assumption (a wrapper) |
| `rd_kafka_admin_fanout_worker` | `admin_result.fanout_parent \|-> #released` | partly validated (`30`) |
| `rd_kafka_DeleteRecords_leaders_queried_cb` | `leaders.opaque \|-> #stale` | declared assumption (`goto err`) |
| `rd_kafka_ListOffsets_leaders_queried_cb` | `leaders.opaque \|-> #stale` | partly validated (`30`) |
| `rd_kafka_share_commit_async_fanout_op` | `share_commit_async_fanout.ack_batches \|-> {#released, #kept}` | partly validated (`30`) |
| `rd_kafka_share_fetch_fanout_op` | `share_fetch_fanout.ack_batches \|-> {#released, #kept}`; premise `share_ack_batches_nonempty` | partly validated (`30`) |
| `rd_kafka_share_commit_sync_fanout_op` | `ack_batches \|-> #released`, `results`, `rd_kafka_op_s.rko_replyq.q` `\|-> #moved` | partly validated (`30`) |

Pinned at both revisions ([`pin.sh`](scripts/pin.sh), logs
`private/mpr/pin-a1/`): the crate's pin 0 at both, its check 0 at both,
the whole-tree check 1 at both only for the libevent crate's tip-mode
`mk_vhost_set` (by design), `--verify-contracts` 0; the two hand-offs carry
`rdkafka_op.h`'s `rd_kafka_op_res_t` enumeration in their identity.
`providers/contracts.py sync` and `check` 0. Pins: resource-v1 declarations
457 → **465**, retaining 80 → 88 (`compiler/tests/contract_loader.rs`), and
the per-declaration clause counts (`compiler/tests/qcore_contract_corpus.rs`).
Declarations consulted 420 → 428.

## 5. Measurement

BEFORE `before` (`2260bdae…`, the base) and AFTER `a2` (`d2d91163…`, the
card's code at `9d008180c` with its contracts), each `run.json` naming its
binary, every route with the null property
([summary](artifacts/summary.json), [`after.sh`](scripts/after.sh)):

| route | BEFORE | AFTER | report sha256 | wall (s) |
| --- | --- | --- | --- | --- |
| corpus | 936 / 9 / 3,889 | **936 / 9 / 3,889** | `99d6362f…` → `71387bd3…` | 670.7 → 922.5 |
| system | 93 / 4 / 351 | 93 / 4 / 351 | `cb069b68…` → `13d2f98b…` (contract digests) | 81.0 → 96.0 |
| slice | 52 / 1 / 347, 4 Pending | the same | `2f67e756…` → `bda122de…` (the same) | 38.7 → 50.4 |
| null | 1,113 / 1,435 / 2,355 / 18 Unsafe; 84 / 125 / 245; 71 / 103 / 232 | every row identical (4,921, 237, 271 rows) | | |

Wall times were measured once each on a host whose load average stood at
about 10 (other work outside this card); they are not a performance claim.

* **Moving rows** ([compare](artifacts/compare-corpus.json),
  [leaves before](artifacts/leaves-before-corpus.json) and
  [after](artifacts/leaves-after-corpus.json)): exactly the predicted 83
  Unknown rows, each trading the text of its `rd_kafka_op_destroy` leaves:
  all **131** now name `rd_kafka_oauthbearer_refresh_op`
  (`callback-uncontracted`), the one `rko_op_cb` candidate of the run's 27
  still read uncontracted (its own member invocation). System and slice:
  none ([system](artifacts/compare-system.json),
  [slice](artifacts/compare-slice.json)).
* **No Safe gain and no Safe → non-Safe move**; no leak, violation or
  `Unsafe` added; the fold demoted nothing. No caller links a tracked
  object at a stated path (the one candidate site,
  `rd_kafka_topic_partition_list_query_leaders_async`, refuses each store
  into the op), so the composition acts on no corpus row.

**Prediction against outcome.** As predicted on every route (section 5 of
the prediction). Not predicted: the first binary's withdrawals (section 3).

## 6. Lean

`proof/QCore/MemberPathRelease.lean`, audited by
`proof/AuditMemberPathRelease.lean`: `gone_leaves_exactly_once` and
`stays_never_leaves` (no double release, no missed release, over every
reading the state allows and every arm the join allows),
`a_stale_path_never_decides`, `an_unstated_member_never_decides`,
`a_refusal_refuses_the_join`, `a_gone_join_is_gone_for_every_candidate`,
`the_set_is_the_join_of_its_members`, `the_complement_partitions_the_results`,
`compose_vectors_hold` (Rust `COMPOSE_VECTORS`, compared with the Lean text
by `compose_vectors_are_the_lean_vectors`). `QCore.BodyClauseStanding`: the
evidence `r30` covering `memberState`. Lean audits 323 → **324**.

## 7. Ledger

[`scripts/update_ledger.py`](scripts/update_ledger.py), 780 → **780**.

* **Retired:** `OP-CALLBACK-CANDIDATE-CONTRACTS` (the user's decision taken,
  the eight declared with their bodies read at both revisions, measured on
  `rd_kafka_op_destroy`'s callers).
* **Amended:** `HANDOFF-CANDIDATE-OWN-CALLBACKS` (the 131 leaves now name
  `rd_kafka_oauthbearer_refresh_op` alone).
* **Registered:** `MEMBER-PATH-STATES-THROUGH-UNIT-HELPERS` (three
  declarations the body check leaves as declared assumptions).

Not registered, stated in rule 30: a direct call of a member-path
declaration refuses (no corpus site), and a decided registration of a
candidate that also hands its formal reads uncontracted at `decide_member`,
as before this card (no corpus leaf).

## 8. Gate

Gate **g1** ([gate.json](gate.json), exits in [gate/](gate/), logs
`private/mpr/gate-g1/`): every code step at `a49391665` (after it, records
only: rule 13u's amendment note at `c30a51c79`, the docs steps' head), the
standing gate and the provider gate under one exclusive flock on the shared
heavy lock (08:09:45Z–08:33:50Z; [`gate-run.sh`](scripts/gate-run.sh),
[`gate.sh`](scripts/gate.sh), [`finish-gate.sh`](scripts/finish-gate.sh)).
The measured binary `a2` was built at `9d008180c`; the later commits change
records only.

| Step | Exit | Result |
| --- | --- | --- |
| `cargo check --workspace --all-targets` | 0 | |
| submodule update | 0 | |
| `cargo fmt --check` | 0 | |
| `clippy --locked --all-targets -D warnings` | 0 | |
| `run_test_binaries.py` | 0 | 499 units, 512 summaries, **8,098 passed, 0 failed, 15 ignored** (`private/mpr/tests-g1/`), no rerun |
| both reevaluation probes | 0, 0 | |
| `check_lean_audits.py` | 0 | **324** audits passed |
| provider gate | 0 each | `package.py` build, verify `qcore.libevent` and `qcore.librdkafka`; `cargo check --locked` in `providers/`; `contracts.py check`; `cargo test` 133 passed / 0 failed; providers Python 25 OK; fmt; clippy |
| `cargo clean --profile dev` | 0 | |
| Python suite before the matrix regeneration (08) | **1** | fails only `test_the_derived_facts_are_recomputed_rather_than_carried` (the committed matrix's Lean count, 323 → 324), as for every card adding an audit |
| docs index check (09) | 0 | |
| `build_matrix.py . --gate gate.json` (09a), Python suite (09b, 365 tests), docs index (10b) and its check (10c) | 0, 0, 0, 0 | |

Pins: [`pin.sh`](scripts/pin.sh) (section 4). The bold rule-id duplicate
check (`rg -o --no-filename '^[0-9]{1,2}[a-z]?\. \*\*' docs/agents/*.md |
sort | uniq -d`) is empty. Clang: [`gate/clang-version.txt`](gate/clang-version.txt).

## Files

* `crates/qcore-schemas/src/{member_path,retained_by,handoff,lib}.rs`;
  `crates/qcore-contract/src/{contract_member_path,contract_loader,contract_retained_by}.rs`;
  `crates/qcore-c-gateway/src/{legacy_member_path_route,member_path_check,legacy_callback_route,legacy_constructed_route,legacy_retained_by,legacy_reference_result,contract_clause_standing,legacy_c,lib}.rs`,
  `tests/member_path_release.rs`, `tests/fixtures/contracts/member_paths.qv`.
* Contracts: `providers/qcore-provider-librdkafka/contracts/librdkafka/client.qv`,
  `contracts.lock`, `contracts.json`; pins `compiler/tests/{contract_loader,qcore_contract_corpus}.rs`.
* Lean: `proof/QCore/MemberPathRelease.lean`, `proof/AuditMemberPathRelease.lean`,
  `proof/QCore/BodyClauseStanding.lean`, `proof/QCore.lean`.
* Records: `docs/agents/{gateway,README}.md`, the spec, the grammar,
  `docs/STATUS.md`, `docs/development/increments/wave-48-part8.md`,
  `docs/reviews/r0_3/acceptance_matrix.json`, `docs/reports/INDEX.md`.

# Scheduler element destroyer in production (rule 34)

Card W48-SCHED-CLEANUP-PRODUCTION connects Monkey's scheduler cleanup to a
production consumer. Gateway rule [34](../../agents/gateway.md#constraint-34)
lets a declaration state `resource_unpublished(p)` beside
`resource_released(p, …)`: the *element destroyer*. A caller applies rule
10d/10f's unlink first and then the release. The Monkey provider declares
`mk_http_thread_destroy` so, with eight named premises.

Fresh pinned corpus BASE `4dd8c8013` / TIP `b2f708d49`: **977 → 999 Safe,
3,848 → 3,826 Unknown, 9 provisional Unsafe unchanged**. Rule 34 accounts for
exactly two of the 22 moves: `sched_thread_cleanup` and
`mk_sched_threads_destroy_conn` become custody Safe, each conditional on
`mk_http_thread_destroy`'s eight named premises. Both stay Unknown on the null
property, so their combined verdict is unchanged, and every loaded runtime
scenario moves both back to Unknown because no outer envelope covers those
premises. The other 20 moves belong to the sibling rules 32 (8) and 35 (12),
which main carried into this TIP. The planner's two callers,
`mk_sched_threads_purge` and `mk_sched_threads_destroy_all`, stay Unknown; the
reason is below and the residual is registered. No Safe row regresses, and no
new Unsafe appears.

The [prediction](PREDICTION.md) was committed (`385eaedd3`) before the first
semantic commit (`e2ef6e832`). Its row prediction holds: the two rows it
named move, the two planner callers stay Unknown for the reason it gave, and
`mk_http_thread_destroy`'s own row stays Unknown.

## What changed

**Rule 34** (`e2ef6e832`, docs `d28d4cdc3`).
`qcore_contract::contract_counted_users::validate` admits an unlink beside a
release on the same parameter, and nothing else beside it.
`container_publication::destroyer_split` takes those unlinks out of a call's
container operands; the `ContractCall` arm of `legacy_transfer.rs` applies
them **before** the call's own reading. The release therefore ends exactly the
custody the unlink took:

- rule 10d's restored published site (an object this path published), or
- rule 10f(2)'s admitted foreign element: a `container_of` element of a safe
  iteration, or a `->next` lookup, over a container rooted at an unrebound
  formal, a local initialised from one, or a static object, at its fresh site.

Every other entry keeps the unlink's refusal. Rule 5l's refusals for a plain
iteration and a saved successor are unchanged; a use after the destroyer reads
a released binding; a release after it is a double release. Rule 10f(3) is
unchanged: a body that lowers such an unlink publishes no summary.

**Declaration.** `mk_http_thread_destroy` in `qcore-provider-libevent`
(`contracts/monkey/intrusive_list.qv`), pinned at Fluent Bit `046accf6` and
`66910c10`, where its source (`lib/monkey/mk_server/mk_http_thread.c:251-272`
and the inline `mk_thread_destroy`, `mk_thread_libco.h:64-78`) is
byte-identical. It states `resource_unpublished(mth)`,
`resource_released(mth, nonnull)` and no result. What no generic rule proves
is carried as eight `resource_premise` clauses that reuse the
[scheduler decomposition](../scheduler-lifetime-contracts/README.md)'s
hypothesis names:

| Premise | Decomposition hypotheses | Residual named in W48-BOTTOM-UP-RESIDUAL-PLAN |
|---|---|---|
| `scheduler_exact_parent_embedding` | `exact_parent_embedding`, `parent_live` | SCHEDULER-LIST-EMBEDDING-TRANSPORT |
| `scheduler_callback_footprint` | `callback_footprint` | SCHEDULER-CALLBACK-EFFECT-CLOSURE |
| `scheduler_tls_params_owned` | `tls_params_owned`, `tls_api_success` | SCHEDULER-TLS-RETIREMENT, SCHEDULER-CREATE-TLS-LIBCO-FAILURES |
| `scheduler_callee_live` | `callee_live`, `suspended_coroutines` | SCHEDULER-CREATE-TLS-LIBCO-FAILURES |
| `scheduler_borrowed_chain_live` | `borrowed_chain_live` | SCHEDULER-CALLBACK-EFFECT-CLOSURE |
| `scheduler_valid_membership` | `valid_membership`, `no_concurrent_writer` | SCHEDULER-LIST-EMBEDDING-TRANSPORT |
| `scheduler_no_later_alias_use` | `no_later_alias_use` | SCHEDULER-EXTERNAL-ALIAS-RETIREMENT |
| `scheduler_valgrind_request_neutral` | (Valgrind client request under `MK_HAVE_VALGRIND`) | none; new here (SCHED-DESTROYER-PREMISES) |

Source readings, not assumptions (both revisions):

- **`cb_destroy`.** The only inserters into `sched->threads` and
  `threads_purge` are `mk_http_thread_create` and `mk_http_thread_purge`.
  Creation calls `mk_thread_new(sizeof(...), NULL)`, so every element's
  `cb_destroy` is NULL at creation, and no other Monkey source writes it. The
  premise still states it: a NULL callback, or a returning one that releases
  and mutates neither the saved successor, nor its parent, nor any other list
  element. No neutral-callback reading is made.
- **The embedded-data NULL branch** returns before insertion, so it never
  reaches cleanup.
- **A NULL `co_create` result** leaves a linked element whose `callee` is
  NULL. `thread_params_set` switches to it before any cleanup, so no cleanup
  state rests on it beyond `co_delete`'s own premise (`scheduler_callee_live`).
- **TLS.** The params slot is freed and cleared by the body
  (`scheduler_tls_params_owned` states ownership and API success). The current
  thread's `mk_thread` key is not cleared by destruction; the declaration
  claims nothing about it, and `scheduler_no_later_alias_use` names it among
  the aliases that must not be read again.

The finite model's refusal (the scheduler decomposition's cleanup checker) is a
checker refusal, not a C rollback path; nothing here reads it as one.

**Lean.** `QCore.ElementDestroyer` (`proof/QCore/ElementDestroyer.lean`,
registered in `proof/AuditElementDestroyer.lean`, 13 theorems, no placeholder,
no project axiom) bridges the scheduler model's destroy trace to the caller's
reading: `destroying_an_admitted_element_is_safe`,
`destroying_an_unadmitted_element_is_refused`,
`the_release_never_precedes_the_unlink`, `destroying_twice_is_never_safe`,
`a_release_after_the_destroyer_is_a_violation`,
`an_unlink_before_the_destroyer_is_refused`,
`destroying_a_published_object_is_safe`, `a_destroying_body_is_withheld`,
`the_modelled_destroy_is_an_unlink_then_a_release`,
`the_modelled_destroy_of_an_admitted_element_is_safe`,
`a_modelled_cleanup_unlinks_and_retires`, and the two clause-placement
theorems. Rust: `crates/qcore-c-gateway/tests/element_destroyer.rs` (nine
tests, including `a_destroying_body_publishes_no_summary` and
`destroying_the_saved_successor_is_refused`).

**Pins.** v1 declarations 492 → 493; `mk_http_thread_destroy` 12 clauses
(`compiler/tests/qcore_contract_corpus.rs`, `compiler/tests/contract_loader.rs`).
The provider inventory (`providers/qcore-provider-libevent/contracts.json`) and
lock are updated.

## Measurement

**Route.** The conditional eight-home pinned corpus with the batch4 link map
([link-map.corpus.json](link-map.corpus.json)): `scripts/run_pinned_audit.py`
from each side's `git archive` snapshot, with its own release binary,
profiles, llvm-21 Clang for configured routes, and the null and outer
projections on. This is the route of the retained BASE2/TIP2 pair, so the
figures compare across the cards that use it. Driver:
[scripts/pair_job.sh](scripts/pair_job.sh) and
[scripts/measure.py](scripts/measure.py), under the shared validation lock,
`--jobs 4` per side.

**Input re-spelling.** The checkout moved to `/media/Data/GitHub/quirin-lang`,
and the pinned compile databases still spell the old prefix. The audit selects
units by comparing path strings, so the first pair on the new base (`final-2`)
selected no unit and failed on both sides; it is a failed attempt. The
measured pair reads a mirror that
[scripts/mirror_inputs.py](scripts/mirror_inputs.py) writes, copied from card
W48-BU-SINGLE-LEAF-BODIES so the figures stay comparable. The script copies the
inputs byte for byte with digests checked and changes only the path prefix
inside the two compile databases. Manifest sha256 `3c7d2bb0…a6466`.

| Side | Revision | Binary sha256 | Exit | Custody Safe / Unknown / Unsafe |
|---|---|---|---|---|
| BASE | `4dd8c8013` | `12c088f4…0e50` | 0 | 977 / 3,848 / 9 |
| TIP | `b2f708d49` | `7be5cdf4…b442` | 0 | 999 / 3,826 / 9 |

Both sides cover 4,834 rows, and input, binary and link-map identities held
across each run. Later commits on the branch change only this report and the
attempts ledger. The null projection keeps 4,921 records on both sides, with
null Safe 2,549 unchanged. Combined Safe goes 849 → 859; rule 34 adds none of
that.

**Attribution** ([native-comparison-4dd8c8013.json](native-comparison-4dd8c8013.json),
[scripts/attribute.py](scripts/attribute.py)). 648 rows differ beyond the
environment digests. Each is assigned by what changed in it: a row that rests
on, or lost a leaf naming, `mk_http_thread_destroy`, or a Monkey
scheduler/http-thread row, is rule 34; one naming `rd_kafka_log0` or
`rd_kafka_log_buf` is rule 32. Every other row is rule 35, by elimination,
because `array_member_decay.rs` and `literal_dereference.rs` are the only other
semantic change between BASE and TIP.

| Rule | Status moves | Same-status leaf/dependency changes | Null records changed |
|---|---|---|---|
| 34 (this card) | 2 | 1 | 2 |
| 32 (sibling, W48-RK-LOG-CALLER-FRAME) | 8 | 546 | 11 |
| 35 (sibling, W48-BU-SINGLE-LEAF-BODIES, by elimination) | 12 | 79 | 21 |

Rule 34's rows:

- `sched_thread_cleanup` (`mk_scheduler.c:614`): Unknown → Safe. It loses its
  only leaf, `call to uncontracted function mk_http_thread_destroy` at `:625`.
  Assumptions: the eight `mk_http_thread_destroy` premises (its
  `hypothesis_dependencies` name the eight atoms), plus rule 10f(2)'s
  P-CRR-MEMBER and P-CRR-LINKED at the caller.
- `mk_sched_threads_destroy_conn` (`:657`): Unknown → Safe. It loses
  `unresolved resource identity mth` at `:668` and the uncontracted call at
  `:669`. Same assumptions.
- `mk_http_thread_destroy`'s own row stays Unknown, as predicted. With its
  declaration in force it gains two leaves, `unresolved resource identity mth`
  at `mk_http_thread.c:267` and `:268` (the reads of `mth` after its own
  unlink), so its standing is a declared assumption, not validation.
- Runtime projection: in both loaded scenarios (`flb_external_plugin`,
  `rk_plugin_library`), both rows move custody Safe → Unknown. The reasons
  name each of the eight premises as `outer-uncovered`.

The sibling moves match their own cards' claims. Rule 32 moves
`rd_kafka_log0` and its seven callers (its open review concern, that
`rd_kafka_log0` rests on its own declared assumption, is that card's). Rule 35
moves twelve rows across `rdkafka.c`, `rdkafka_mock.c`, `rdkafka_conf.c`,
`rdkafka_txnmgr.c`, `rdkafka_sasl_oauthbearer_oidc.c`,
`rdkafka_telemetry_decode.c` and `mk_config.c`, by lifting branch-condition
and indirect-access leaves. This card does not review them.

**System and tail-pause slice: not run.** Neither selection holds a Monkey
unit. `mk_http_thread_destroy`, `sched_thread_cleanup` and
`mk_sched_threads_*` are named only under `lib/monkey/`, and rule 34 changes
behaviour only for a declaration that states an unlink beside a release, of
which this is the only one. The sibling rules measure those routes themselves.

The retained BASE2/TIP2 pair on `49a946c8a`
([native-comparison-49a946c8a.json](native-comparison-49a946c8a.json)) isolates
rule 34 alone, on the pre-move inputs. It gives 977 → 979 Safe, the same two
rows, and three rows differing beyond the digests; this pair reproduces that
subset exactly.

## Why the planner's two callers did not move

The planner scored `sched_thread_cleanup` 2 for `mk_sched_threads_purge`
(`mk_scheduler.c:634-640`) and `mk_sched_threads_destroy_all` (`:642-649`),
whose bodies are only calls to it. At TIP each keeps exactly the leaves it had
at BASE:

- `mk_sched_threads_purge`: `call to uncontracted function
  sched_thread_cleanup` at `mk_scheduler.c:637`;
- `mk_sched_threads_destroy_all`: the same leaf at `:645` and `:646`.

`sched_thread_cleanup` is now Safe, but its body lowers a foreign unlink (the
destroyer's unlink of each element of its list formal), so rule 10f(3)'s gate
(`has_foreign_unlink` in `observations_match_contract`) publishes no summary
for it. That gate is load-bearing: a summary would read the list formal as
`Borrowed`, although the body ended every element the list held, and a caller
that had published its own object into that list would keep restoring it.
Rule 10f and rule 34 both forbid dropping the gate without a caller-side rule
that forgets the published sites the call may release. No such rule exists,
and the summary vocabulary has no clause for "releases every element of the
container at #k" (W44-CALLEE-DESTRUCTOR-SUMMARY records
`resource_elements_dropped` as registry-only). Freeing the two callers is
therefore a new semantic rule (a container-drain summary effect, its caller
application, a schema change, Lean and a fresh measurement), registered as
SCHED-CLEANUP-CALLER-DRAIN rather than approximated here. The null property is
not what holds them: both are null Safe at TIP, under the inferred `#sched`
precondition.

`mk_sched_threads_destroy_conn` moved instead because it inlines the same loop
(`:657-674`) and calls `mk_http_thread_destroy` directly.

## Null property

Both promoted rows stay Unknown on `c.null.deref.v1`, so their combined verdict
is Unknown. Their candidates are the list traversal reads (`list->next`,
`head->next`, `&sched->threads->next` at `mk_scheduler.c:623` and `:666`),
`mth->session` at `:668`, and the moved `mk_http_thread_destroy` precondition
on `mth` at `:625` / `:669`. A circular-list non-NULL successor and a non-NULL
session are not modelled. This card does not change the null property; the
residual is registered as SCHED-CLEANUP-NULL-TRAVERSAL.

## Gate

Run on the final card commit as one locked job,
[scripts/final_gate_job.sh](scripts/final_gate_job.sh): the full standing gate
([scripts/gate_job.sh](scripts/gate_job.sh): submodules,
`scripts/run_test_binaries.py`, `cargo fmt --check`, Clippy with
`-D warnings`, both reevaluation examples, `scripts/check_lean_audits.py`,
root Python), then the provider gate
([scripts/provider_gate.sh](scripts/provider_gate.sh)) and `pin --check` of
the Monkey provider at both audited revisions with the BASE and TIP binaries
([scripts/pin_check.sh](scripts/pin_check.sh)). Incremental compilation and
dev/test debug info are off.

Results at `ceec066ee` (`private/w48-sched/final-gate-1`; `head` and
`head-after` equal, tree clean):

| Step | Exit | Count |
|---|---|---|
| submodules | 0 | |
| Rust (`run_test_binaries.py`) | 0 | 545 units / 558 summaries, 8,587 passed, 0 failed, 15 ignored |
| `cargo fmt --check` | 0 | |
| Clippy `-D warnings` | 0 | |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| Lean audits | 0 | 369 passed |
| root Python | 0 | 374 tests |
| provider Rust workspace | 0 | 43 summaries, 133 passed, 0 failed, 1 ignored |
| provider fmt / Clippy | 0 / 0 | |
| provider Python | **1** | 25 tests, 1 failure (below) |
| provider inventory | 0 | |
| package build, five verifies | 0, 0 ×5 | |
| `pin --check` libevent, `046accf6` / `66910c10` | 1 / 1 (BASE 1 / 1) | TIP 26 recorded, BASE 25; at both, the only not-in-force entry is the pre-existing `mk_vhost_set` |
| `pin --verify-contracts` | 0 (BASE 0) | |

The provider Python failure is a sibling effect, not this card's.
`test_the_real_workspace_manifest_is_current` pins 65 resource-view modules,
and rule 32's `librdkafka/log_caller_frame.qv`, which main added to
`providers/contract-load-path.json`, makes it 66. The pin is already 66 on
`card/w48-rk-log-caller-frame` (`896b2da33`). This card's own provider change
passed the same suite at `489dab482` and `bde5716da` (provider gates 2 and 3,
25 tests). The first `pin --check` runs passed the snapshot root, not its
manifest file, and were refused (`QI014`, exit 2). They are retained, and the
reruns `pin-base-2` and `pin-tip-2` are the evidence.

## Attempts

[attempts.json](attempts.json) lists every run. Interrupted and failed runs
are not evidence: `final-1` (BASE `4dd8c8013` / TIP `2fe7708fa`), interrupted
by the human halt; `final-2` (TIP `dd006ebb0`), which selected no unit before
the input re-spelling; and the earlier gate attempts. Evidence is `final-3`
(the pair) and `final-gate-1` (the gate), plus the retained earlier pairs and
provider gates.

## Follow-ups

Registered in [integration-amendments.json](integration-amendments.json):

- **SCHED-CLEANUP-CALLER-DRAIN**: a container-drain summary effect and its
  caller rule, which would free `mk_sched_threads_purge` and
  `mk_sched_threads_destroy_all`.
- **SCHED-DESTROYER-PREMISES**: the eight premises, each Open source truth,
  mapped to the scheduler residuals of W48-BOTTOM-UP-RESIDUAL-PLAN, with one
  DISCHARGE per premise.
- **SCHED-CLEANUP-NULL-TRAVERSAL**: the null candidates that keep both promoted
  rows' combined verdict Unknown.

Not this card's to review: rule 35 has no gateway text on main (its branch
carries it), and rule 32 has its own open concern.

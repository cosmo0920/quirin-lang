# CG22-CALLBACK-REGISTRY-CONTRACT: callback contracts

Card CG22-CALLBACK-REGISTRY-CONTRACT (wave 48, round 21), gateway rules
**13u–13z**, contract specification **C04.19**, grammar **G05**, Lean
`QCore.CallbackContracts`. Base: local `main` at `84ec54d1d`
(DEEP-AST-TRANSPORTS-AND-WALL-TIME's gate records); no later main during the
card. The [prediction](PREDICTION.md) was frozen and committed (`4c220b5de`)
before any implementation code.

## 1. The decision and what the card built

USER DECISION 2026-10-05 (`W48-CALLEE-DECISION-TABLE` class 1, "split by
shape"): shape (c), callbacks over retained elements, goes through CG22's
callback contracts. Rule 11p's core relation (`callback_trace`,
`QCore.IndirectCalls`) existed; no contract stated a callback, so every
callback-taking callee was uncontracted or read its callback as a borrowed
pointer. The card adds the vocabulary (C04.19), total-or-reject and pinned
like every clause, withheld from the function map with the retaining
declarations, so a consumer that knows nothing of callbacks sees an
uncontracted callee:

| clause | shape | route G (rule) |
| --- | --- | --- |
| `resource_callback_invoked(F, A…)` | synchronous invocation | 13u |
| `resource_callback_table(T, A…)` | table dispatch | 13v |
| `resource_callback_registered(F, H)`, `_registry_invoked(H, A…)`, `_unregistered(H)` | registry | 13w |
| `resource_element_added(E, H)`, `resource_elements_dropped(H)` | element custody | 13x |
| `resource_released_if_constructed(H)` | flag-conditioned self-release (`RD_LIST_F_ALLOCATED`) | 13x |
| body checks of the invocation and of a registration in a parameter holder | evidence `13y` | 13y |

**Loop-safe composition (rule 13u).** A callee calls a candidate zero or more
times; the call reads as its custody contract exactly when every candidate's
own contract is the identity on the caller's state (each position
`resource_borrowed` or `resource_owned`, no result, no kind, no side clause).
Then any number of invocations is the identity
(`invocations_of_identities_are_identity`). `resource_owned` is a requirement,
not an effect: the callee meets it with what it supplies (`_`), and at a
parameter position the call itself carries it (the call's position is read
`resource_owned(n)`, the requirement a direct call of the candidate with that
actual carries; `requirements_are_owned_parameters`). The one release admitted
is a registry callback's at an `element` position (element custody). Every
other case refuses with a named reason: an open candidate set
(`callback-undecided`, naming rule 11p's reason), a candidate with no contract
in force (`callback-uncontracted`), a non-identity candidate
(`callback-effectful`). **No callback is read as benign.**

**Fail-closed registries (rules 13w, 13x).** A registration is recorded only
for a holder the calling body names (a frame object `&L` named only at holder
positions, or a tracked identity, a constructor's result included) and is in
force only when every arriving path made it alike; any registry use without
one refuses (`callback-registry-undecided`). An element's custody passes only
into a releasing registry held by a tracked identity (contained; released by
the registry's invocation, never by a plain release of the holder); a frame
registry refuses it (`callback-element-custody-undecided`); no or identity
callbacks only retain it (rule 12t).

**Body checks (rule 13y).** In a unit that defines the body: an invocation
stands only when `F` is only the callee of direct calls (`F(…)`, `(*F)(…)`)
with the declared actuals; a registration only when `F` is only the value of
stores `H->M = F;`. A failing body withdraws the declaration in that unit
(`callback-withdrawn`). `13y` evidence covers exactly those clauses
(`ClauseKind::CallbackChecked`, rule 13i); every other callback clause is a
declared assumption (`ClauseKind::Callback`).

Design refinements made during implementation, each from a measured run (a1,
`0a6b07ec…`), none changing the prediction's totals:

* `resource_owned` admitted as a requirement (the frozen design said every
  position `resource_borrowed`): W4 summarises a comparator that dereferences
  its formals as `resource_owned(nonnull)`, which is a precondition, not a
  custody effect; reading it as effectful would refuse every comparator.
* Placement by agreement: glibc's `__glibc_const_generic` expands `bsearch` to a
  `_Generic` whose two associations each call `bsearch`; the token stream
  (W44-C11-GENERIC-SELECTION) reads one call, Clang two. A site stands when
  every Clang call of the callee on the line resolves alike (rule 11p's own
  placement rule).
* Rule 13y reads an inline `RET (*NAME)(…)` parameter declarator's name
  (`rd_list_init`'s `free_cb`); an unread name never validates (before the
  fix, an empty name made a zero-occurrence check pass vacuously in the
  invocation check; no corpus declaration reached that arm in a1, as its only
  inline-declarator member is a registration, whose check needs a store).

## 2. BEFORE

Binary `bin/before` (`75e0dcb2…`, `run.json`'s `binary_sha256`), built from
`84ec54d1d`: corpus **881 / 9 / 3,944** (`ab8f5f14…`, the brief's), system
87 / 4 / 357, slice 45 / 1 / 354, 4 Pending; Safe roll-up 418 / 94 / 23 / 346
(no declaration / only validated / partly validated / a declared assumption).
Member sizes and the callback candidates in
[PREDICTION.md](PREDICTION.md) ([sizes](artifacts/size-before-corpus.json),
[candidates](artifacts/candidates-before.json),
[registrations](artifacts/registrations-before-corpus.json)).

## 3. Contracts

Each body read at `66910c10` and `046accf6` ([bodies](artifacts/bodies.txt),
`scripts/body.py`): every member body is byte-identical except the cmetrics
and ctraces map unpackers, to which `046accf6` adds a duplicate-key check
(each handler then called at most once per map; the same reading). Pinned at
both revisions (`scripts/pin.sh`, logs `private/cg22r/pin-a1/`): every
per-crate `pin --check` 0 at both; the whole-tree check 1 at both only for the
libevent crate's tip-mode `mk_vhost_set`, not in force at a Fluent Bit tree by
design (MONKEY-VHOST-PROPERTY-CUSTODY); `--verify-contracts` 0;
`providers/contracts.py sync` then `check` 0.

| crate, module | declaration | clauses |
| --- | --- | --- |
| `quirin-provider-libc` `stdlib.qv` | `bsearch` | `invoked(compar, key, _)`, `return_interior(base, nullable)` |
| | `qsort` | `invoked(compar, _, _)` |
| `quirin-provider-libc` `threads.qv` | `pthread_once` | `invoked(init_routine)` (at most once, read as zero or more) |
| `qcore-provider-cmetrics` `mpack_utils.qv` | `cmt_mpack_unpack_array` | `invoked(entry_processor_callback, reader, _, context)` |
| | `cmt_mpack_unpack_map` | `table(callback_list, reader, _, context)` |
| `qcore-provider-ctraces` `context_lifetime.qv` | `ctr_mpack_unpack_array`, `ctr_mpack_unpack_map` | the same |
| `qcore-provider-librdkafka` `client.qv` | `rd_list_init` | `registered(free_cb, rl)`, `return_alias(rl, nonnull)` |
| | `rd_list_new` | `registered(free_cb, result)`, `return_owned(nonnull)` |
| | `rd_list_add` | `element_added(elem, rl)`, `return_borrowed_from(rl, nullable)` |
| | `rd_list_destroy` | `registry_invoked(rl, element)`, `elements_dropped(rl)`, `unregistered(rl)`, `released_if_constructed(rl)` |
| | `rd_list_clear` | `registry_invoked(rl, element)`, `elements_dropped(rl)` |
| | `pb_write`, `pb_encode_varint` | `registry_invoked(stream, stream, buf, count)` / `(stream, stream, _, _)` |
| `qcore-provider-librdkafka` `topic_partition.qv` | `rd_kafka_topic_partition_list_sort` (re-declared) | `invoked(cmp, _, _, opaque)` |
| `quirin-provider-msgpack` `packer.qv` | `msgpack_packer_init` (re-declared) | `retained_by(data, pk)`, `registered(callback, pk)` |

Rule 13y on the measured bodies: validated `cmt_mpack_unpack_array`,
`ctr_mpack_unpack_array`, `rd_list_init`, `msgpack_packer_init` (`13y`
evidence, partly validated: their `resource_borrowed` formals stay uncovered);
withdrawn `rd_kafka_topic_partition_list_sort` in `rdkafka_partition.c` (its
body rebinds `cmp` to a default and hands it to `rd_qsort_r`). The libc
declarations have no body in the corpus. Pinned tables: 363 → **377**
resource-v1 declarations (`compiler/tests/contract_loader.rs`; 28 → 44
withheld into the retaining table), the per-file clause pins in
`compiler/tests/qcore_contract_corpus.rs`.

**Not declared.** `rd_kafka_q_serve*` / `rd_kafka_q_pop_serve*`
(`rdkafka_queue.c:558-649`): the reference part is decidable (rule 13b: the
forward queue is kept by `rd_kafka_q_fwd_get` and dropped by
`rd_kafka_q_destroy` in the same body, the caller's `rkq` is only locked and
dequeued from), but the callback (through `rd_kafka_op_handle`) receives ops
the callee moved to a local queue and must take them over (destroy or
re-enqueue): a callback with custody of a callee-supplied object, which rule
13u refuses, so every call would refuse. `rd_kafka_op_destroy` calls the op's
own `rko_op_cb` (a member read through a pointer, registered by the op's
creator: an open set under rule 11p) and releases a payload chosen by the op
type from a union: **not tractable** here. `call_once` (librdkafka's
`rdkafka.c`; `tinycthread.c` may define it), `pthread_getspecific` /
`mk_sched_get_thread_conf` (a result returning what a holder retains) and the
invoking side of msgpack's registry (`msgpack_pack_*`, header inline bodies)
are not declared.

**The served-queue premise (task 4) stays.** `rd_kafka_q_wait_result` and
`rd_kafka_q_purge0` run the op callbacks of popped ops (`rd_kafka_op_handle`
→ `rd_kafka_op_call`, `rd_kafka_op_destroy`'s `ERR__DESTROY` call). Those
callbacks are read from `rko->rko_op_cb`, registered by whoever created the
op (often another thread); no registration of them is ever in force in the
calling body, so rule 13w cannot state them as checked clauses. The premise
`resource_premise(served_queue, …)` (C04.18) is kept unchanged.

## 4. AFTER

Binary `bin/a3` (`3b204784…`, `run.json`'s `binary_sha256`), built from
`a22b07d8a` (the committed source of every rule; `a2`, `e6e07bc2…`, measured
the same totals before the refusal texts named rule 11p's reason).

| route | BEFORE | AFTER | report sha256 | wall (s) |
| --- | --- | --- | --- | ---: |
| corpus | 881 / 9 / 3,944 | **879 / 9 / 3,946** | `ab8f5f14…` → `c0320aca…` | 449.3 → 450.2 |
| system | 87 / 4 / 357 | SYSTEM_A3 | SYSTEM_DIGEST | 59.7 → SYSTEM_S |
| slice | 45 / 1 / 354, 4 Pending | SLICE_A3 | SLICE_DIGEST | 28.2 → SLICE_S |

**Every moving function attributed** ([corpus](artifacts/compare-a3-corpus.json),
[system](artifacts/compare-a3-system.json), [slice](artifacts/compare-a3-slice.json)):
230 corpus rows move, 2 in status.

* **Named corrections (Safe → Unknown):**
  `rd_kafka_topic_partition_list_sort_by_topic` and `_by_topic_id`
  (`rdkafka_partition.c`). Both were `Safe` only because
  `rd_kafka_topic_partition_list_sort` read its comparator as borrowed
  (`RDKAFKA-SORT-COMPARATOR-EFFECTS`, named in that module's comment, never
  registered); their comparators (`rd_kafka_topic_partition_cmp_opaque`,
  `_by_id_cmp_opaque`) are `Unknown`, and the re-declaration is withdrawn in
  their unit by rule 13y (`callback-withdrawn`). Predicted exactly.
* **No Unknown → Safe.** `rd_kafka_ApiVersion_check`, the one possible mover
  the prediction named, stays Unknown: its comparator
  `rd_kafka_ApiVersion_key_cmp` is a header `static` (`rdkafka_proto.h:208`)
  W4 does not summarise in the unit (no row), so it has no contract in force
  (`callback-uncontracted`). Measured once on the unit preprocessed with its
  compile command and analysed with `stdlib.qv` alone through
  `analyze_unrestricted_c11` (which summarises every definition of the text,
  the comparator included; a temporary harness, not shipped), the function
  was Safe: the gap is the summary scope, not the vocabulary.
* **Leaves only (228 rows, all Unknown before and after).** 226 attributed to a
  declared member (a lost or gained leaf naming it): `rd_list_destroy` 110,
  `rd_list_add` 66, `rd_list_init` 46, `rd_list_new` 26, the unpackers 36,
  `pb_write`/`pb_encode_varint` 12, `bsearch` 6, `qsort` 3, `rd_list_clear`
  4, `pthread_once` 2, `rd_kafka_topic_partition_list_sort` 7. Three read by
  hand: `flb_input_chunk_create` and `flb_input_chunk_map` (the
  `msgpack_packer_init` statement is now refused at its member actual by the
  access gate, and a later use of `ic` reads unresolved), and
  `rd_kafka_topic_partition_list_has_duplicates` (it lost the summary of the
  corrected `rd_kafka_topic_partition_list_sort_by_topic`).
* **No leak, violation or `Unsafe` added**; the 9 provisional `Unsafe` rows do
  not move. Leaf families: 427 `call to uncontracted function` removed;
  added the named refusals below, 114 access-gate refusals at member
  actuals of the now-classified calls (`&rk->rk_conf.partition_assignors`:
  the gate reads an actual of a contracted call where an uncontracted call
  was refused whole), 6 loop re-acquisitions and 2 discarded owned results
  (`rd_list_new` in a loop, or into a member).

Rule keys on the AFTER corpus ([reasons](artifacts/reasons-a3-corpus.json);
leaves / functions / decided alone):

| key | leaves / functions / alone | callees |
| --- | --- | --- |
| `callback-registry-undecided` | 239 / 112 / 5 | `rd_list_destroy` 131, `rd_list_init` 49, `rd_list_add` 44, `pb_write` 8, `pb_encode_varint` 5, `rd_list_new` 2 |
| `callback-uncontracted` | 46 / 44 / 17 | the unpackers 36, `bsearch` 6, `qsort` 3, … |
| `callback-undecided` | 10 / 9 / 1 | a null comparator of `rd_kafka_topic_partition_list_sort` 6, `pthread_once` over an unprototyped designator (an implicit `BitCast`) 2, a formal 2 |
| `callback-element-custody-undecided` | 3 / 3 / 0 | `rd_list_add` into a frame list with a free callback |
| `callback-withdrawn` | 2 / 2 / 2 | `rd_kafka_topic_partition_list_sort` |
| `callback-effectful` | 0 | |

The 44 candidates `callback-uncontracted` names
(`scripts/candidate_status.py`): 40 are `Unknown` in their own unit (the
cmetrics and ctraces decoder trees, whose leaves are not Safe, and four
librdkafka comparators), 3 are `Safe` in another unit
(`rd_kafka_metadata_broker_cmp`, `_internal_cmp`, `rd_strcmp3`: no
cross-unit summary route, `W44-CALLEE-CROSS-UNIT-SUMMARY`), 1 has no row
(`rd_kafka_ApiVersion_key_cmp`).

**Uncontracted callees** (`scripts/rank.py`): Unknown functions blocked by
uncontracted calls alone 523 → **487**, by one callee alone 366 → **335** (the
36 now carry named callback refusals; none freed).

**Safe roll-up** ([corpus](artifacts/rollup-a3-corpus.json)): **418 / 94 / 23 /
344** of 879 (no declaration / only validated / partly validated / a declared
assumption): the two corrected rows rested on a declared assumption. 344
declarations consulted (330 before): 54 validated, 35 partly validated
(`cmt_mpack_unpack_array`, `ctr_mpack_unpack_array`, `rd_list_init`, and
`msgpack_packer_init` up from a declared assumption, each by `13y`), 254
declared assumptions, 1 held.

## 5. Parity, compared in full

PARITY_SECTION

## 6. Prediction against outcome

| predicted | measured |
| --- | --- |
| corpus 879–880 / 9 / 3,944–3,945 | **879 / 9 / 3,946** |
| exactly two corrections, the sort callers | the two, as named |
| `rd_kafka_ApiVersion_check` Safe or Unknown | Unknown (header comparator unsummarised) |
| no leak, violation, Unsafe added | none |
| system and slice: no status move | STATUS_SYSTEM_SLICE |
| registry callers Unknown (holders through formals) | 239 `callback-registry-undecided` leaves, 5 functions decided alone by them |
| decoders trade the uncontracted leaf for `callback-uncontracted` | 36 leaves, as predicted |

The prediction's design said "every parameter `resource_borrowed`"; the
implementation admits `resource_owned` as a requirement (section 1). The
change admits no `Safe` (none moved to Safe) and was taken because the
borrowed-only reading would have refused every comparator W4 summarises.

## 7. Lean

`proof/QCore/CallbackContracts.lean`, audited by
`proof/AuditCallbackContracts.lean` (27 theorems, axioms within `propext`,
`Quot.sound`; no placeholder, no project axiom): loop-safe composition
(`invocations_of_identities_are_identity`,
`an_identity_decision_is_the_custody_reading`), the refusals
(`decide_refuses_an_open_set`, `_an_empty_set`, `_a_missing_contract`,
`_an_effectful_candidate`, `decide_never_releases_outside_a_registry`,
`decide_identity_needs_identities`), the reading of a contract
(`identity_only_borrows_or_requires`, `a_release_is_only_of_an_element`,
`requirements_are_owned_parameters`), registries
(`an_invocation_needs_a_registration`, `an_element_needs_a_registration`,
`a_clear_needs_a_registration`, `an_undecided_registration_refuses`,
`the_callback_trace_agrees` with `QCore.IndirectCalls.callbackTrace`) and
element custody (`a_moved_element_is_released_by_its_registry`,
`a_moved_element_left_at_the_exit_is_lost`,
`a_moved_element_dropped_without_invocation_is_lost`,
`a_retained_element_returns_on_clear`,
`a_retained_element_at_the_exit_is_refused`,
`releasing_needs_every_candidate`), and three vector lists bound to the Rust
tables by `callback_contracts::tests` (`decisionVectors`, `registryVectors`,
`classifyVectors`, each row rendered and compared). `QCore.BodyClauseStanding`
gains the clause kinds `callback` and `callbackChecked` and the evidence
`r13y` (its coverage and standing vectors, bound by
`tests/contract_clause_standing.rs` and the audit's
`contract_dependencies_report.rs`). Lean audits 303 → **304**.

## 8. Ledger

LEDGER_SECTION

## 9. Gate

GATE_SECTION

## Files

* Vocabulary: `crates/qcore-schemas/src/callback.rs`,
  `crates/qcore-schemas/src/retained_by.rs` (`callbacks`,
  `callbacks_withdrawn`), `crates/qcore-contract/src/contract_callback.rs`,
  hooks in `contract_loader.rs`; `docs/QCORE_CONTRACT_SPEC_r0_3.md` (C04.19;
  C04.14's shape (c) pointer), `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` (G05).
* Route G: `crates/qcore-c-gateway/src/callback_contracts.rs` (decision core,
  shared vectors), `legacy_callback_route.rs` (operands, AST resolution, table
  closure, registry transfer, release refusal, join, body checks, W4 order
  edges); hooks in `legacy_retained_by.rs` (`retaining_call`, the owned
  requirement, `Operands::callbacks` with a derived-form `Debug`),
  `legacy_transfer.rs` (`AbstractState::registries`, the join, the transfer,
  the release refusal), `legacy_c.rs` (module, the three `settle` entries),
  `legacy_summary.rs` (edges), `contract_clause_standing.rs` (clause kinds,
  `13y`), `legacy_reference_result.rs` (constructor fields).
* Lean: `proof/QCore/CallbackContracts.lean`,
  `proof/AuditCallbackContracts.lean`, `proof/QCore/BodyClauseStanding.lean`,
  `proof/QCore.lean`.
* Tests: `crates/qcore-contract/tests/callback_clause.rs`,
  `crates/qcore-c-gateway/tests/cg22_callback_registry.rs`,
  `tests/fixtures/contracts/callbacks.qv`; pins
  `compiler/tests/contract_loader.rs`, `compiler/tests/qcore_contract_corpus.rs`,
  `crates/qcore-audit/tests/contract_dependencies_report.rs`.
* Contracts: `providers/quirin-provider-libc/contracts/libc/{stdlib,threads}.qv`,
  `providers/qcore-provider-cmetrics/contracts/cmetrics/mpack_utils.qv`,
  `providers/qcore-provider-ctraces/contracts/ctraces/context_lifetime.qv`,
  `providers/qcore-provider-librdkafka/contracts/librdkafka/{client,topic_partition}.qv`,
  `providers/quirin-provider-msgpack/contracts/msgpack/packer.qv`, their
  `contracts.lock`, `contracts.json`, `providers/contract-load-path.json`.
* Records: `docs/agents/gateway.md` (13u–13z; 11p and 13i amended),
  `docs/agents/README.md`, `docs/STATUS.md`,
  `docs/development/increments/wave-48-part5.md`,
  `docs/reviews/r0_3/acceptance_matrix.json`, `docs/reports/INDEX.md`.
* Scripts: `build.sh`, `measure.sh`, `measure-binary.sh`, `pin.sh`, `sync.sh`,
  `lean.sh`, `targeted.sh`, `body.py`, `size_members.py`,
  `callback_candidates.py`, `registrations.py`, `totals.py`, `compare.py`,
  `parity.py`, `parity-all.sh`, `rank.py`, `reasons.py`,
  `candidate_status.py`, `prefix_size.py`, `clause_rollup.py`, `gate.sh`,
  `provider-gate.sh`, `write_gate_json.py`, `update_ledger.py`.

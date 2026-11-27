# CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 4: callee contracts and rule 66

Card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS, round 4. USER DECISION 2026-11-21
(through the integrator's brief): follow up the indirect-access refusals with
a round on the largest class, the callee contracts. Base `9b107caf1`.
[PREDICTION.md](PREDICTION.md) was committed (`1d3df585d`) before the first
implementation commit (`e9aeaa511`). Three pinned neutral declarations
(`rd_jitter`, `XXH32_finalize`, `XXH64_finalize`), gateway
[rule 66](../../../../agents/gateway.md#constraint-66), Lean
`QCore.TypedefScalarElement` (6 audited theorems).

## 1. Census and ranking (step a)

Route and inputs: [evidence/runs.md](evidence/runs.md) (the opt-in `current
--monkey-linked` over Fluent Bit 046accf6, as round 3 took it; the moved
prepared inputs needed their path prefix rewritten in a private copy).
Base: **1,150 Safe / 2 Pending / 3,645 Unknown / 7 Unsafe**, 2,154 Unknown
rows carrying `indirect memory access is outside the resource projection`
(9,066 leaves), **222** carrying nothing else. Round 3's AST census scripts,
unchanged ([scripts/census.sh](scripts/census.sh): 180 of 180 units
sha256-matched, 9,082 leaf groups, 0 unbound), give
[census/base-refined-sole-shapes.md](census/base-refined-sole-shapes.md): the
class `uncontracted-callee-unread-actual` blocks **40** functions alone (56
mentioning), the same 40 as round 3.

[scripts/rank.py](scripts/rank.py) ranks the 40 by what frees each
([census/rank-base.json](census/rank-base.json)): every callee body read at
046accf6, its honest contract recorded in the script. **1 is freed by a
contract alone, 2 by a contract and a reading, 37 by neither**: their
callees' honest contracts are not neutral, so no declaration this card may
write frees them.

| what blocks the function | functions | callees |
|---|---:|---|
| an allocating out-parameter whose slot is the caller's member (an owned value into a formal's object: MEMBER-EFFECT-STORES) | 13 | `cmt_mpack_consume_string_tag` 5, `ctr_mpack_consume_string_or_nil_tag` 5 (`*output_buffer = cfl_sds_create_size(…)` into `&opts->description` and the like), `rd_kafka_gzip_compress`, `rd_kafka_lz4_compress`, `rd_kafka_snappy_compress_slice` (`&ciov->iov_base`) |
| an owned constructor result stored into a member or returned | 6 | `rd_kafka_topic_partition_list_copy` 2, `rd_kafka_topic_partition_list_new`, `rd_kafka_topic_partition_copy`, `rd_kafka_MemberDescription_new` (static, same unit), `rd_kafka_group_result_new` with a nested `rd_kafka_error_copy` |
| container element custody (decision class 1, rule 46) | 5 | `rd_list_elem`, `rd_list_find` (and a comparator), `rd_list_copy`, Monkey's `mk_list_init` 2 (a self-initialised embedded head) |
| a timer callback registration (decision class 1) | 4 | `rd_kafka_timer_start0` |
| the premised logger with nested static-string actuals | 3 | `rd_kafka_log0` (three named premises, never Safe outright) with `rd_kafka_err2str`, `rd_kafka_topic_name`, `rd_kafka_offset2str` nested |
| reference counts and queue references (decision class 3) | 3 | `rd_atomic32_add` (`memory.qv` keeps it undeclared), `rd_kafka_q_fwd_set0`, `rd_kafka_HeartbeatRequest` with `rd_kafka_replyq_make` |
| an enqueue or a timer-list insertion (custody into a container) | 2 | `rd_kafka_msgq_enq_sorted0`, `rd_kafka_timer_schedule_next` (static) |
| a write through the stream's callback member, at a return-position cast view | 1 | `pb_encode_string` (CG10-CAST-VIEW-RETURN-POSITION) |
| **a neutral callee: freed by this card** | **3** | `rd_jitter` (contract alone); `XXH32_finalize`, `XXH64_finalize` (contract and rule 66) |

Round 2 measured that a reader lift alone frees none of the class; this
round measures the converse for the three neutral callees: `rd_jitter`'s
actuals (`(int)((float)rkb->rkb_reconnect_backoff_ms * 0.75)`) were already
read once the callee was declared (rule 14a), so the contract alone frees
it; the finalizers' callers also need rule 66.

## 2. Declarations (step b)

`providers/qcore-provider-librdkafka/contracts/librdkafka/lz4_stream.qv`, beside
the neutral LZ4 codec declarations, each with its body quoted and its ABI
stated. They were first written beside round 2's helpers in `memory.qv`
(the tip-1 measurement); the first gate failed
`production_symbolic_adoption` there, because `memory.qv`'s bytes are
registered by QUANT-CONTRACT-HYPOTHESES' frozen migration record
(`after_sha256`), so they moved, unchanged, to `lz4_stream.qv`
(`1bcd0ebe4`; `memory.qv` is byte-identical to the base) and were
measured again (tip 2, section 4):

* `rd_jitter(int low, int high)` (`rdrand.c:130-133`): `rd_rand()` (a
  `rand_r` over a thread-local seed, or `rand()`) mixed with two `int`
  values; reads no caller memory. Every formal `resource_borrowed`,
  `resource_return_none()`.
* `XXH32_finalize(U32, const void *ptr, size_t, XXH_endianess, XXH_alignment)`
  (`rdxxhash.c:290-348`) and `XXH64_finalize` (`:701-808`), both `static`:
  walk a local copy of `ptr` over the tail bytes, reading them, and return
  the mixed scalar. The enumerations are `u32`, the type Clang states for
  them (rule 12u).

Pinned with `quirin-audit pin` at both audited revisions
([scripts/pin.sh](scripts/pin.sh): the corpus revision from a shared clone
of the local Fluent Bit repository checked out at `66910c10`, the system
revision the prepared checkout; `pin` 0 and 0, `pin --check --crate` 0 at
`66910c10`, 1 at `046accf6` only for `rd_kafka_log0`/`rd_kafka_log_buf`,
which are in force there through their `66910c10` identity, a pre-existing
state this card does not touch; `pin --verify-contracts` 0). `rdrand.c`,
`rdrand.h` and `rdxxhash.c` are byte-identical at both revisions.
`providers/contracts.py sync` regenerated `contract-inventory.lock` and
`contracts.json`; `compiler/tests/contract_loader.rs` lists the three among
the neutral declarations.

**Not validated.** None of the three bodies validates in its home unit:
`rd_jitter`'s row is `Unknown` on `call to uncontracted function \`rd_rand\``
(a static helper over a thread-local seed), and both finalizers' rows on
`unmodeled increment or decrement expression` (the walk's `*p++`), so each is
a declared assumption pinned to the text read, not a checked summary
(registered: CG10-R4-NEUTRAL-DECLARATIONS-UNVALIDATED).

## 3. Rule 66 (step c)

Located by a held-out fixture of `XXH32_digest_endian`'s statement: with the
finalizer declared, `return XXH32_finalize(h32, state->mem32, state->memsize,
endian, XXH_aligned);` stayed refused, while the same call with `&state->mem32[0]`,
or with an `unsigned int m[4]` member, was `Safe`. Rule 35's second arm reads
a decayed array member as `&PATH[0]` only when the array type's element is
builtin scalar words, and Clang states no desugared spelling for an array of
a typedef (`uint32_t mem32[4]` is `uint32_t[4]`). Rule 66
(`typedef_scalar_element.rs`; a hook in `rule_hosts.rs::Reader::decayed_address`
asked only after rule 35 refuses and only while rule 35 is enabled; the
shared `EnumFacts::scalar_typedefs` and `Reader::scalar_typedefs` in
`typed_access_path.rs`) reads the element `T` through the unit's own
`TypedefDecl`s: when every one resolves to the same builtin scalar, rule 35's
admission applies unchanged. A pointer, record, `volatile`, `_Atomic` or
disagreeing resolution, a multi-word element, a second dimension and a
non-member operand keep the refusal. Lean `QCore.TypedefScalarElement`
(`an_agreeing_scalar_typedef_is_its_spelling`,
`a_disagreeing_typedef_reads_nothing`, `a_non_scalar_resolution_reads_nothing`,
`a_non_member_is_never_read`, `decision_vectors_hold`,
`the_decision_is_the_models`), bound to `DECISION_VECTORS`.

The errno, nested-call and cast actuals the brief names do not decide any of
the 40: a nested call appears only beside a non-neutral callee (the logger,
the heartbeat, the group-result copy, the timer schedule), and the
`rd_jitter` casts were already read. Rule 66 is the one reading a neutral
callee's caller needed.

**Controls** (in the existing target
`crates/qcore-c-gateway/tests/cg10_sole_residual_contracts.rs`, so no new
test unit for the gate read census): `rd_jitter` in the backoff shape is
`Safe` with the declaration and `Unknown` without it; a buffer beside the
neutral call still leaks (`Unsafe`); the `XXH32_digest_endian` shape is
`Safe`; arrays of a pointer, record or `volatile` typedef element stay
`Unknown`. Unit tests in `typedef_scalar_element.rs` (4: the decision
vectors, the element parser, agreement, the node reading).

## 4. Measurement

| run | Safe / Pending / Unknown / Unsafe | access rows | access alone | access leaves |
|---|---|---:|---:|---:|
| base `9b107caf1` (`6acb901c…`) | 1,150 / 2 / 3,645 / 7 | 2,154 | 222 | 9,066 |
| tip 1 `df45ff563` (`40a3b51e…`, declarations in `memory.qv`) | 1,157 / 2 / 3,638 / 7 | 2,148 | 218 | 9,046 |
| **tip 2 `1bcd0ebe4` (same binary, declarations in `lz4_stream.qv`)** | **1,157 / 2 / 3,638 / 7** | **2,148** | **218** | **9,046** |

Tip 2's report equals tip 1's row for row (statuses and leaves), so the
evidence below, computed against tip 1, holds for tip 2.

**Seven moves, every one Unknown -> Safe** ([evidence/moved-rows.jsonl](evidence/moved-rows.jsonl)):

| row | base leaves | what moved it | rests on |
|---|---|---|---|
| `rdkafka_broker.c::rd_kafka_broker_update_reconnect_backoff` | 1 access (`backoff = rd_jitter(…)`) | the `rd_jitter` declaration (rule 14a reads the statement) | `rd_jitter` |
| `rdxxhash.c::XXH32_digest_endian` | 1 access (the return) | the `XXH32_finalize` declaration and rule 66 (`state->mem32`) | `XXH32_finalize` |
| `rdxxhash.c::XXH64_digest_endian` | 1 access | the `XXH64_finalize` declaration and rule 66 (`state->mem64`) | `XXH64_finalize`; summaries `XXH64_round`, `XXH64_mergeRound` |
| `rdxxhash.c::XXH32_digest` | 2 `call to uncontracted function \`XXH32_digest_endian\`` | `XXH32_digest_endian`'s summary | `XXH32_finalize`; summaries `XXH32_digest_endian`, `XXH_isLittleEndian` |
| `rdxxhash.c::XXH64_digest` | 2 the same for `XXH64_digest_endian` | its summary | `XXH64_finalize`; summaries |
| `lz4.c::LZ4_prepareTable` | 1 access (`MEM_INIT(cctx->hashTable, 0, …)`, `LZ4_u32 hashTable[…]`) | rule 66 | `memset` |
| `lz4hc.c::LZ4HC_clearTables` | 2 access (`hashTable`, `chainTable`, `LZ4_u32`/`LZ4_u16` elements) | rule 66 | `memset` |

**No Safe -> non-Safe move, no new Unsafe, no Pending moved** (Unsafe 7 and
Pending 2 on both runs, the same rows). Every moved row rests on a declared
assumption (Safe rows on one 527 -> 534): the three new declarations, none
validated (section 2), and libc's `memset`.

**Leaves** ([evidence/changed-rows-base-tip1.txt](evidence/changed-rows-base-tip1.txt),
26 rows change; 19 stay Unknown): access leaves 9,066 -> 9,046 (20 removed,
none added); `call to uncontracted function` leaves for `rd_jitter` (8 in 7
rows), `XXH32/64_finalize` (2), `XXH32/64_digest_endian` (4) and
`rd_kafka_broker_update_reconnect_backoff` (6, in `rd_ut_reconnect_backoff`)
removed. Added, each on a row that stays Unknown: seven `call to
uncontracted function` for LZ4's static position helpers in
`LZ4_compress_generic_validated` (its hash-table actuals are now read and the
helpers are next), `unresolved resource identity` `mpid`
(`rd_kafka_mock_pid_new`) and `rtmr` (`rd_kafka_timer_exp_backoff`):
re-attributions. `LZ4HC_init_internal` loses eight leaves (its call to the
now-summarised `LZ4HC_clearTables` and the leaves after it) and keeps others.

**Against the prediction.** The three predicted moves happened, and the two
`XXH*_digest` callers (predicted possible); rule 66 elsewhere predicted 0-15,
measured 2 (`LZ4_prepareTable`, `LZ4HC_clearTables`); access-alone predicted
219-221, measured 218 (five sole rows left it: the three predicted and the
two LZ4 rows of the sub-object-address class; `LZ4HC_init_internal` joined
it, its other leaves gone with `LZ4HC_clearTables`'s summary); access leaves predicted
10-200, measured 20. No Safe -> non-Safe and no new Unsafe, as predicted.

**Pinned routes (corpus, system, slice).** `quirin-audit measure --declared`
cannot run on this host (no `build/cycle19` inputs or baselines). Prediction
for the integrator: on the corpus (and twelve-home) the seven rows where the
route analyses `rdkafka_broker.c`, `rdxxhash.c`, `lz4.c` and `lz4hc.c` (the
three declarations are pinned at `66910c10`, whose bodies are byte-identical);
the measure-prediction block lists the three the prediction named before the
measurement, so the integrator should expect `XXH32_digest`, `XXH64_digest`,
`LZ4_prepareTable` and `LZ4HC_clearTables` as unpredicted extra moves.
System and slice: none (they analyse none of these units).

## 5. What remains (step d)

On the tip ([census/tip-refined-sole-shapes.md](census/tip-refined-sole-shapes.md),
[census/rank-tip.json](census/rank-tip.json)): **218 functions** blocked by
the access refusal alone. The class blocks **37 alone** (53 mentioning), each
blocked by its callee's custody question (section 1's table without its last
row: out-parameters into members 13, constructors into members 6,
containers 5, timer callbacks 4, the premised logger 3, reference counts 3,
enqueues 2, the stream callback 1). The other classes, functions freed alone:
several 46, pointer member read at an actual 17, formal array elements
released 12, owned result into a formal's member 11, embedded list address
11, pointer stored into a member 10, mock nested return 8, indirect call 7,
pointer arithmetic store 7, `?:`/cast/descriptor 7, union access 6,
sub-object address 5 (7 on the base: rule 66 freed the two LZ4 rows), owned
result into a local member 5, aggregate value 5, union pointer store 4,
other pointer store 4, 10 smaller classes 16.

No callee of the class is left that a neutral declaration frees, so the
contract half of the class is a set of custody questions, each with its
owner: MEMBER-EFFECT-STORES (13 + 6), decision class 1 (5 + 4), decision
class 3 (3), the logger premises (3), CG10-CAST-VIEW-RETURN-POSITION (1).

## 6. Gate

Scoped card gate `python3 scripts/gate_scope.py --base 9b107caf1 --out
private/r4/gate-2` through `private/toolchain/env.sh` (Clang 21, Lean 4.35)
on `d00195628` (the code and contracts of the final tip; later commits change
this report only); mode full (the diff reaches `qcore-c-gateway`), provider
gate included. Record [gate/gate-2.json](gate/gate-2.json), logs in the
worktree's `private/r4/gate-2/`.

| step | exit | result |
|---|---:|---|
| submodules | 0 | 1 submodule |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| `run_test_binaries.py` | 0 | 670 units, 715 summaries; 9,822 passed, 0 failed, 21 ignored, 148 filtered |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `check_lean_audits.py` | 0 | 470 audits |
| `python3 -m unittest discover -s scripts/tests` | **1** | 779 tests, 1 failure: `test_gate_slot_queue.Status.test_each_waiter_shows_its_queue_position` (`'holder' != 'waiting for a slot'`) |
| provider package build / verify-all / verify | 0 / 0 / 0 | five packages verified |
| provider `cargo test` / fmt / clippy | 0 / 0 / 0 | 49 summaries, 166 passed, 0 failed, 1 ignored |

**The Python failure is the shared slot queue, not this card.** The same test
failed in the first attempt; run alone (`-p test_gate_slot_queue.py`) it
passes (39 tests OK), and the whole suite rerun standalone with the card base
the gate passes (`QUIRIN_CARD_GATE_BASE=9b107caf1… python3 -m unittest
discover -s scripts/tests`, `private/r4/python-rerun2.log`) exits **0, 779
tests OK**. (A rerun without that variable, `python-rerun.log`, fails only
the derived-matrix check the gate relaxes for a card: the matrix's Lean audit
count 469 against 470 with this card's audit, which the integrator's
regeneration moves.) The test inspects the live gate-slot status while this
gate's own steps hold slots in a contended queue of other worktrees.

**Earlier attempt, not evidence.** `private/r4/gate-1` (code at
`b5758c406`, declarations in `memory.qv`): rust suite 9,819 passed, 3 failed:
`contract_inherit_qcore_provider` (2: the hand-written `memory.qv` parity
fixture) and `production_symbolic_adoption` (1: `memory.qv`'s bytes are
registered by QUANT-CONTRACT-HYPOTHESES' frozen migration record). Both are
pinned identities of `memory.qv`; rather than edit a frozen report's record,
the declarations moved to `lz4_stream.qv` (section 2) and the tip was
measured again. The same Python flake; the attempt was stopped before its
provider steps finished. The gate read census was not re-recorded: no test
target was added (the controls are in the existing
`cg10_sole_residual_contracts` target).

## 7. Follow-ups (in [integration-amendments.json](integration-amendments.json))

* **CG10-INDIRECT-ACCESS-SOLE-RESIDUALS** amended with this round (not
  retired: section 5).
* **CG10-R4-NEUTRAL-DECLARATIONS-UNVALIDATED** (new): the three declarations
  are trusted assumptions; their bodies do not validate (`rd_rand`'s
  thread-local seed; the finalizers' pointer walks).
* **CG10-OWNED-OUT-PARAMETER-MEMBER-SLOT** (new): 13 sole functions hand an
  allocating out-parameter a member slot of the caller's object
  (`cmt_mpack_consume_string_tag`, `ctr_mpack_consume_string_or_nil_tag`,
  the three compressors); no contract form states an owned value delivered
  into a member path, and the caller's store is MEMBER-EFFECT-STORES'.
* **RK-TOPIC-PARTITION-LIST-CONSTRUCTORS** (new): `rd_kafka_topic_partition_list_new`,
  `_list_copy` and `rd_kafka_topic_partition_copy` have no declaration (their
  bodies allocate; `_copy` takes partition references through
  `rd_kafka_topic_partition_update`); declaring them owned is a measured
  card's (it can expose leaks in their many callers).
* Not registered again: decision classes 1 and 3, the logger premises
  (W48-RK-LOG-CALLER-FRAME), CG10-CAST-VIEW-RETURN-POSITION,
  MEMBER-EFFECT-STORES, W45-CALLEE-CASCADE-RESIDUAL.

**Files likely to conflict.** `typed_access_path.rs` (the `EnumFacts` field,
one call in `EnumFacts::collect`, the `Reader::scalar_typedefs` accessor),
`rule_hosts.rs` (`decayed_address`), `lib.rs` (one `mod` line),
`providers/qcore-provider-librdkafka/contracts/librdkafka/lz4_stream.qv`, its
`contracts.lock`, `contracts.json` and `contract-inventory.lock`,
`compiler/tests/contract_loader.rs`, the generated `docs/agents/gateway.md`,
`docs/agents/README.md`, `proof/QCore.lean` (`proof/AuditAll.lean` is unchanged). The OpenSSL
provider is not touched.

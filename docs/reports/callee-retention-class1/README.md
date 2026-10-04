# CALLEE-RETENTION-CLASS1: class 1 split by shape

Card CALLEE-RETENTION-CLASS1 (wave 48, card-high). Base: local `main` at
`6fc62c015`, which it stayed at. [PREDICTION.md](PREDICTION.md) was committed
at `54933c5b7`, before any implementation code or contract.

## 1. The decision

**USER DECISION 2026-10-05**, recorded verbatim in the contract specification
(C04.14) and in `W48-CALLEE-DECISION-TABLE`: on class 1 ("callbacks and
retained pointers", 36 callees deciding 94 functions alone on the
W45-CALLEE-CASCADE-RESIDUAL run) the user chose **"split by shape"**:

* **(a) custody-taking inserts** → a conditional custody transfer on success,
  with the existing relational postconditions (CG23, rule 11r), no new
  vocabulary;
* **(b) a pointer retained in a holder** → a new `retained_by(P, holder)`
  clause: the caller keeps custody but must establish that `P`'s object
  outlives the holder (or the holder is reset or destroyed first), otherwise
  Pending or Unknown, never Safe; a release while held is a hazard,
  provisional at most, never an upstream defect;
* **(c) callbacks over retained elements** → CG22's callback contracts (rule
  11p); where that vocabulary does not cover a member, record exactly what is
  missing and size it.

## 2. BEFORE

**Reproduced.** Binary `private/crc1/bin/before`, sha256 `4fdeb725…` (equal to
`run.json`'s `binary_sha256`): corpus `66910c10` **803 / 9 / 4,022**,
`resource-diagnostics.json` `2734991a…`, byte for byte the brief's; system
85 / 4 / 359 and slice 45 / 1 / 354 with 4 Pending (digests `dae935e5…`,
`eaefe6a1…` embed this worktree's paths). `scripts/rank.py` gives the W45
AFTER population exactly (652; 473 decided by one callee alone), and
`scripts/tally.py` with W45's classification is byte-identical to its
`tally-after.txt` ([tally](artifacts/tally-before.txt); rankings for
[corpus](artifacts/rank-before-corpus.json), [system](artifacts/rank-before-system.json),
[slice](artifacts/rank-before-slice.json)). Every class-1 member is sized on
the BEFORE corpus in [size-before-corpus.json](artifacts/size-before-corpus.json)
(`scripts/size_members.py`: leaves naming it, functions carrying one,
functions it decides alone).

## 3. Stage (a): custody-taking inserts (gateway rule 12s)

**What reading the members changed.** Every cited (a) caller is a forwarding
wrapper (`return f(ARGS);`), which rule 11r refuses (`relational-untested`),
and even a literal-return function whose exit-specific summary validates kept
an Unknown row (rule 11r analysed its row contract-free). Declaring the
dispositions alone would have freed none of the cited functions, so stage
(a) needed one route, in the existing vocabulary: **rule 12s**, relational
forwarding and the per-class row (`legacy_relational_forwarding.rs`;
contract specification C04.13 *Forwarding*; Lean
`QCore.RelationalForwarding`). The design is in the rule; two measured
iterations shaped it:

* **a1** (`after-a1`, `64698db4…`): the corpus was **byte-identical** to the
  BEFORE although the sidecar reported every new declaration pinned in force. Reading why
  found three gaps in rule 11r's integration, latent since CG23 because no
  corpus callee had carried a disposition: (i) a unit that withdraws any
  declaration (LOADER-ABI's `withdraw_environment`, the checked-specification
  reduction) rebuilt its environment from the function map and **dropped
  every disposition** (the refusing direction: the callee read as
  uncontracted); (ii) no disposition was **compared with the unit's C
  declaration**; (iii) `apply_pins` skipped every name outside the function
  map, so a pinned disposition was **in force at any revision** (the
  admitting direction). All three are closed in this card (rule 12s(3)): the
  reductions carry the side tables (`ResourceContracts::carry_side_declarations`),
  `AbiAgreement::read` reads them, and a pin mismatch withholds a disposition
  or a retaining declaration from its own table and restores it there
  (`PinMismatch::{relational, retaining}`, `with_human_holds_in_force`,
  `in_force_before_pins`, `check_source_assumptions`). The system route moved
  two rows on a1 (`flb_output_task_queue_enqueue`, rule 12s(2), below).
* **a2** (`after-a2`, `9e69830f…`): **+7**. `ctr_attributes_set_array` and
  `_kvlist` stayed Unknown, for two reasons a probe isolated: the first
  lowering (`{ if (f(ARGS) OP K) return R; for (;;); }`) moved the call into
  an `if` at the return's line, where no typed reader matched the member-read
  actual `attr->kv`, so the lexical gate refused it; and their declared
  classes said `nullable` where the forwarding route's validated summary binds
  the formal non-null, which rule 11r(5) compares exactly, so the body check
  **withdrew** both declarations in their own unit. The lowering now reads
  the forwarded callee under its class installed under its own name (the
  return keeps its spelling and position), and the two ctraces declarations
  say `nonnull` (the stronger precondition, refusing for a caller), re-pinned.

**Contracts** (each read at both revisions, byte-identical there; pinned with
`quirin-audit pin` at `66910c10` and `046accf6`, `pin --check` per crate 0 at
both, `--verify-contracts` 0; the whole-tree check exits 1 at both as on the
base, its one row not in force being the libevent crate's tip-mode
`mk_vhost_set`; logs `private/crc1/pin-a1/`, `pin-a3/`):

| crate, module | declaration | classes |
| --- | --- | --- |
| `qcore-provider-librdkafka` `cjson.qv` | `add_item_to_array` (static, `cJSON.c:2012`) | `ne 0`: `item` consumed; `eq 0`: owned; nullable; `array` borrowed |
| | `replace_item_in_object` (static, `:2412`) | `ne 0`: `replacement` consumed; `eq 0`: owned; nullable |
| `quirin-provider-cfl` `sds_lifetime.qv` | `cfl_kvlist_insert_array`, `_kvlist` | `eq 0`: `value` consumed; `ne 0`: owned (`-1` and `-2` both leave it the caller's); nullable |
| `qcore-provider-ctraces` `context_lifetime.qv` | `ctr_attributes_set_array`, `_kvlist` | the cfl insert's classes, `nonnull` (body-checked by rule 12s in `ctr_attributes.c`) |

`add_item_to_object` is **not** declared: its `constant_key` arm stores the
caller's `string` in the item (a retention, stage (b)), a retention does not
stand beside a disposition, and its callers pass `false`/`true` as casts,
which no literal premise reads.

**Measured** (binary `after-a3`, `331fb897…`; same contracts as the final
tree for (a)):

| route | BEFORE | AFTER (a) | digest |
| --- | --- | --- | --- |
| corpus | 803 / 9 / 4,022 | **812 / 9 / 4,013** | `2734991a…` → `cc47d362…` |
| system | 85 / 4 / 359 | **87 / 4 / 357** | `dae935e5…` → `9010f47a…` |
| slice | 45 / 1 / 354, 4 Pending | unchanged, no row moves | `eaefe6a1…` → `e77e4c8a…` |

Corpus: **+9 Unknown → Safe**, no other status move
([comparison](artifacts/compare-a-corpus.json)): `cJSON_AddItemToArray`
(over `add_item_to_array`), `cJSON_ReplaceItemInObject` and
`_CaseSensitive` (over `replace_item_in_object`), `ctr_attributes_set_array`
and `_kvlist` (their own rows, decided per class over the cfl inserts), and
the four `ctr_span_set_attribute_*` / `ctr_span_event_set_attribute_*` setters
(over the declared ctraces setters, cross-unit). Six more functions changed
leaves without a status move, each read by hand: three `cJSON.c` returns of
`add_item_to_array` the route does not forward (`add_item_to_object`,
`cJSON_AddItemReferenceToArray`, `cJSON_InsertItemInArray`: each also returns
`false`, a cast, or passes a nested call) now read `relational-untested`;
`get_or_create_external_metadata_kvlist` returns the kvlist it just handed to
`root` (an `unresolved resource identity`: the caller would receive a pointer
into `root`, class 8); `clone_exemplars_to_kvlist` tests its two inserts but
their actuals do not resolve to one identity (two `unresolved resource
identity` leaves where the uncontracted calls stood); `convert_kvlist_value`
passes a member read to an untested insert (the access gate). No Safe →
non-Safe move, no leak or violation added (`Unsafe` 9 → 9).

System: **+2 rows Unknown → Safe**, one function in both of its units
(`flb_output_task_queue_enqueue`, [comparison](artifacts/compare-a-system.json)):
rule 11r had already inferred its exit-specific disposition (`-1`: `retry`
released; `0`: `retry` stored in the queued task the queue holds, consumed),
and rule 12s(2) now decides its row per class instead of contract-free. Read
by hand at `046accf6` (`src/flb_output.c:280-303`) and reproduced on a probe,
where a caller freeing `retry` after a `0` return is `release_after_escape`.
Slice: no row moves. After stage (a) the class-1 tally is 32 callees / 86
functions ([tally](artifacts/tally-after-a.txt); population 652 → 642).

## 4. Stage (b): retained pointers (gateway rule 12t)

**The clause** (contract specification C04.14, grammar G05; loader
`contract_retained_by.rs`, schema `qcore_schemas::retained_by`):

```qcore
ensures normal resource_retained_by(P, H);   // H: a parameter, or `global`
ensures normal resource_holder_reset(H);
```

`P` and a holder parameter are distinct depth-one pointer parameters,
`resource_borrowed`, with no kind; `global` is storage of static duration or a
holder no caller object denotes (another thread's queue). The caller keeps
custody of `P`'s object. The declaration is **withheld from the function
map** (`ResourceContracts::retaining`), so every consumer that does not
implement the clause sees an uncontracted callee: a retained pointer is never
borrowed and forgotten, and no inference replaces the declaration in a unit
that defines its body.

**What "outlives" is decided from** (route G, rule 12t; `retained_by.rs` and
`legacy_retained_by.rs`):

| `P` \ holder | frame `&L` | tracked heap holder | `global` | anything else |
| --- | --- | --- | --- | --- |
| string literal | outlives | outlives | outlives | outlives |
| a caller's object (unrebound, untracked formal) | outlives (the frame ends first) | recorded until the holder is released or reset; refused at an exit where it still holds | undecided | undecided |
| one tracked identity | recorded until `L` is reset or the frame ends | recorded until the holder is released or reset | undecided | undecided |
| anything else | undecided | undecided | undecided | undecided |

`L` is a frame holder only when it is one automatic, non-pointer, non-array
object of the function (`in_place_custody::one_automatic_object`) named
nowhere but as whole `&L` actuals of retaining or resetting callees, so what
it holds is never copied out and its storage ends at the return. While a
record stands, a release or transfer of the held identity is refused
(`retention-released-while-held`), never accused: a dangling pointer left in
a holder is a hazard only if the holder reads it, which the caller does not
decide. Everything undecided is refused (`retention-undecided`): **Unknown**.
The user's rule allows Pending for a named, human-resolvable reason; a new
held-call reason code and its audit reporting are registered
(`RETENTION-UNDECIDED-PENDING`) rather than built here. A retention is not
forwarded into a summary: a wrapper that hands its own formal to its own
formal holder decides nothing (`RETENTION-SUMMARY-FORWARDING`).

**Controls** (`crates/qcore-c-gateway/tests/callee_retention_class1.rs`, over
the synthetic `r_*` fixture): a caller's object in a frame holder, a frame
holder reset before the release, and a heap holder released before the
pointer are **Safe**; a release before the holder's reset and a holder
outliving its pointer are **refused** (no accusation); a string literal in
the `global` holder is Safe and a heap pointer there is refused; a holder the
caller supplied, and a frame holder whose contents are copied out, are
**Unknown**. The same table and paths are `DECISION_VECTORS` and
`TRACE_VECTORS`, shared with `QCore.RetainedBy`.

**Contracts** (each body read at both revisions: `lz4.c` and `lz4hc.c` are
byte-identical at `66910c10` and `046accf6`; `append_to_ring_buffer` is
identical, its file differs elsewhere; pinned at both, `pin --check` per
crate 0 at both, `--verify-contracts` 0, logs `private/crc1/pin-b1/`):

| crate, module | declaration | retention |
| --- | --- | --- |
| `qcore-provider-librdkafka` `lz4_stream.qv` | `LZ4_compress_HC_extStateHC` | `src` retained by `state` (`LZ4HC_init_internal` stores `prefixStart`, `end`, `dictStart = src`, `lz4hc.c:100-116`) |
| | `LZ4_compress_HC_continue` | `src` retained by the stream (the same pointers, `:1112-1149`) |
| | `LZ4_compress_fast_continue` | `source` retained by the stream (`dictionary = source`, `lz4.c:1709`) |
| | `LZ4_compress_fast_extState` | the state reset, then `source` retained (pointers only with `byPtr`, a 32-bit table; declared on every target, the refusing direction) |
| `quirin-provider-fluent-bit` `input_chunk_helpers.qv` | `append_to_ring_buffer` (static) | `ins` retained by `global` (the raw chunk keeps `cr->ins` in `ins->rb` for the engine thread, `src/flb_input_chunk.c:3584`, `:3635`) |

Not declared, each recorded with its size in `CLASS1-UNDECLARABLE-RETAINERS`:
`rd_kafka_confval_set_type` (an enum typedef parameter; only its `PTR` arm
retains `valuep`), `rd_kafka_interceptor_method_add` (an enum typedef
result), `cJSON_ParseWithLengthOpts` (the `global_error` holder plus an
interior pointer of `value` through `return_parse_end`, which no out-slot mode
states), `add_item_to_object` (a retention beside a disposition).

**Measured** (binary `after-b1`, `19a16144…`: the stage-(a) tree plus the
five declarations and the reserved prefix):

| route | AFTER (a) | AFTER (b) | digest |
| --- | --- | --- | --- |
| corpus | 812 / 9 / 4,013 | **812 / 9 / 4,013** | `cc47d362…` → `a43d02cb…` |
| system | 87 / 4 / 357 | 87 / 4 / 357 | `9010f47a…` → `2d304081…` |
| slice | 45 / 1 / 354, 4 Pending | unchanged | `e77e4c8a…` → `a042f8ce…` |

**No status moves**, as predicted ([corpus](artifacts/compare-b-corpus.json),
[system](artifacts/compare-b-system.json), [slice](artifacts/compare-b-slice.json)):
every caller passes a formal as the pointer, its holder, or both, and the
user's rule keeps those undecided. Twenty corpus functions trade
``call to uncontracted function`` for what now stands: 16
`retention-undecided` (the seven LZ4 wrappers over a formal state or stream,
the four `lz4frame.c` blocks over a cast holder, `LZ4_compress_HC` and
`LZ4_compress_fast` over a pointer local — `statePtr` from a cast allocation,
`ctxPtr = &ctx` — and the three `flb_input_chunk` enqueuers over `global`), 3
the nested `LZ4_compressBound` actual the uncontracted call hid
(`LZ4_compressHC_withStateHC`, `LZ4_compressHC2_withStateHC`,
`LZ4_compressHC_continue`) and 1 the access gate
(`LZ4_compress_destSize_extState`, `*srcSizePtr`). System: the three
enqueuers in both units. The two possible movers the prediction named
(`LZ4_compress_HC`, `LZ4_compress_fast`) stayed Unknown: their holders are
pointer locals, not `&L`. No leak, violation or `Unsafe` added; no release
while held occurs in the corpus.

**Whole card** ([corpus](artifacts/compare-final-corpus.json),
[system](artifacts/compare-final-system.json), [slice](artifacts/compare-final-slice.json)):
corpus 803 / 9 / 4,022 → **812 / 9 / 4,013** (+9), system 85 / 4 / 359 → 87 / 4 /
357 (+2 rows), slice unchanged; **625** Unknown functions remain blocked by
uncontracted calls alone (449 by one callee alone; [tally](artifacts/tally-after.txt),
rankings for [corpus](artifacts/rank-after-corpus.json),
[system](artifacts/rank-after-system.json), [slice](artifacts/rank-after-slice.json)):
652 less the 9 freed and 18 that now carry a named leaf instead. Class 1:
**36 callees / 94 functions → 27 / 72**.

## 5. Stage (c): callbacks over retained elements — recorded, not implemented

The callback contract vocabulary the decision names does not exist:
`CG22-CALLBACK-REGISTRY-CONTRACT` is open (rule 11p states no capture and no
unregistration, so route G has no registry). Each member was read at both
revisions (every body byte-identical except the two map unpackers, to which
`046accf6` adds a duplicate-key check) and is sized on the BEFORE corpus
([sizes](artifacts/size-before-corpus.json); leaves naming it / functions
carrying one / functions it decides alone). No code: the stage's AFTER is
stage (b)'s.

| member | leaves / functions / alone | what is missing |
| --- | ---: | --- |
| `rd_list_destroy` | 184 / 110 / 3 | runs the free callback `rd_list_init` stored in the list (`rl_free_cb`) over every element the list retained (`rd_list_destroy_elems`, `rdlist.c:290-308`), then frees the list only when `RD_LIST_F_ALLOCATED` is set: a **registry** of the callback stored at initialisation, an **element-custody** relation of `rd_list_add` (custody moves into a list that has a free callback, is merely retained by one without), and a **flag-conditioned self-release** |
| `rd_list_clear` | 4 / 4 / 3 | the same over the elements, no self-release |
| `cmt_mpack_unpack_array` / `_map` | 12 / 12 / 5, 10 / 10 / 3 | the array form invokes its callback argument synchronously, zero or more times, with `context` (`cmt_mpack_utils.c:246-303`): a **synchronous invocation** clause (`f` called during the call only, with these actuals, its own contract applied per call under a loop-safe composition); the map form dispatches through the caller's callback **table** (`callback_list[i].handler`, chosen by the entry's key, `:177-…`), which also needs the table's candidates closed (rule 11p's constant-table reading) |
| `ctr_mpack_unpack_array` / `_map` | 5 / 5 / 5, 9 / 9 / 3 | the same (ctraces) |
| `pb_write`, `pb_encode_varint` | 8 / 8 / 2, 5 / 4 / 2 | invokes the callback the stream holds (`stream->callback`, `pb_encode.c:83-104`): an invocation through a **member of a borrowed object** (an open set under rule 11p) needs the registry that set it |
| `rd_kafka_q_serve0`, `_pop_serve0`, `_serve_maybe_consume` | 3 / 3 / 2, 3 / 3 / 2, 2 / 2 / 2 | serve ops through a callback argument or the queue's own `rkq_serve`; `cb_type` is an enum typedef (`W43-LIBC-ABI-READER-SHAPES`) and the ops carry reference counts (class 3) |
| `rd_kafka_op_destroy` | 132 / 83 / 2 | calls the op's own callback (`rko_op_cb`, a member) with `ERR__DESTROY`, then releases a payload chosen by the op type from a union (`rdkafka_op.c:345-620`) and drops references (class 3): a **type-switched union release** beside the registry |

The older class-1 members, against what (b) and (c) need:

| member | leaves / functions / alone | reading |
| --- | ---: | --- |
| `bsearch` | 7 / 6 / 1 | a synchronous comparator over `base`'s elements and an interior result into `base` (C04.3.1 states the result; the comparator needs the synchronous-invocation clause) |
| `qsort` | 4 / 3 / 0 | a synchronous comparator permuting `base` in place: the synchronous-invocation clause |
| `pthread_once` | 2 / 2 / 1 | invokes `init_routine` at most once across threads: an at-most-once invocation, not expressible |
| `pthread_setspecific` | 15 / 10 / 1 | falls out of (b): `resource_retained_by(value, global)` (the key's slot outlives every frame); not declared here, since every caller passes a pointer the slot keeps beyond the frame, so it would only rename the refusal (`retention-undecided`), and libc contracts live in `quirin-provider-libc` |
| `pthread_getspecific` | 16 / 15 / 1 | returns what a slot retains: a **"returns what a holder retains"** result mode, missing |
| `mk_sched_get_thread_conf` | 9 / 9 / 0 | Monkey's wrapper returning `pthread_getspecific`'s result: the same |

## 6. Prediction against outcome

| stage | predicted | measured |
| --- | --- | --- |
| (a) corpus | +5 to +9 (ceiling 9) and 0 to 3 literal-return helpers | **+9** (the ceiling), no helper |
| (a) system, slice | no move | system **+2 rows** (one helper, `flb_output_task_queue_enqueue`, both units: a correction, the prediction placed the helper movers in the corpus); slice none |
| (b) | 0 to Safe; about 15 leaves re-attributed; at most 2 (`LZ4_compress_HC`, `LZ4_compress_fast`) | 0; 20 functions re-attributed (16 to `retention-undecided`); neither possible mover moved |
| (c) | no change | no change |
| new leaks or violations | 0 to 2 | 0 |

Two departures from the frozen design, both measured: rule 12s first lowered a
forwarded return to a decided `if` and now reads the callee under its class
installed under its own name (a1, a2 in section 3); and the three integration
gaps of rule 11r (dispositions dropped by unit reductions, never ABI-checked,
never pin-evaluated) were found by the byte-identical a1 run and closed here.
The undecided retention is reported Unknown, not Pending, as the frozen design
said.

## 7. Parity, compared in full

`scripts/parity.py` ([`parity-all.sh`](scripts/parity-all.sh)) compares every
row of the BEFORE and AFTER typed-path parity streams, in order and as
multisets, with Clang AST node addresses erased and, because this card added a
`retention` field to `ContractCall` whose Debug spelling every row printing a
lowered call now carries, its empty value (`, retention: None`) erased too
(before that erasure 17 functions outside the moving set differed only in that
spelling, e.g. `cio_file_write`'s `SubObjectStore` row; [`parity_rows.py`](scripts/parity_rows.py)
prints one function's rows).

| route | stage | rows (BEFORE → AFTER) | multiset identical | functions differing | outside the moving set |
| --- | --- | --- | --- | --- | --- |
| corpus | (a) | 668,705 → 668,790 | no | 12 | none |
| system | (a) | 141,186 → 141,210 | no | 1 | none |
| slice | (a) | 50,426 → 50,426 | **yes** | 0 | none |
| corpus | card | 668,705 → 668,259 | no | 24 | the five retaining callees' own bodies |
| system | card | 141,186 → 139,962 | no | 3 | `append_to_ring_buffer`'s body |
| slice | card | 50,426 → 50,426 | **yes** | 0 | none |

The rows the whole card removes outside the moving functions are the
consultations W4 recorded while inferring a summary for each retaining
callee's own body (`append_to_ring_buffer` 312 corpus and 1,248 system rows,
`LZ4_compress_fast_continue` 172, `_extState` 36, `LZ4_compress_HC_extStateHC`
8, `_continue` 6): a retaining declaration is never replaced by an inference,
so its body is no longer inferred (`RETAINED-BY-BODY-CHECK`); their own rows do
not move. ([stage (a)](artifacts/parity-a-corpus.json), [card](artifacts/parity-final-corpus.json),
with the system and slice files beside them.)

**`mk_vhost_set` at the tip** ([`tip-probe.sh`](scripts/tip-probe.sh),
[rows](artifacts/tip-rows-after.txt)): byte-identical to
W45-CALLEE-CASCADE-RESIDUAL's record; no Monkey contract changed. The tip's one
refusal is the uncontracted `mk_vhost_set_property` (its DocumentRoot store,
real behaviour, refused).

## 8. Lean

Two modules, registered in `AuditRelationalForwarding.lean` and
`AuditRetainedBy.lean` and imported by `QCore.lean`; no placeholder, no
project axiom (289 → 291 audits):

* **`QCore.RelationalForwarding`** (rule 12s, over `QCore.RelationalPost`; 10
  audited theorems): `every_path_is_kept_by_one_class` and
  `no_path_is_kept_by_two_classes` (the restricted bodies partition the
  paths, from totality and disjointness), `a_forwarded_call_applies_the_callee_class`
  (on the restricted body of `r` the decided test's true edge selects exactly
  the callee's class `r`, which the implementation installs under the
  callee's name), `the_published_class_holds_at_every_exit`,
  `a_caller_edge_selects_the_exit_disposition` (through rule 11r's `select`),
  `the_row_covers_every_path`, `a_conversion_can_move_a_value_between_classes`
  (why the result types must be spelled alike), and the helpers
  `representative_sat`, `meets_self`, `disjoint_others_miss`.
* **`QCore.RetainedBy`** (rule 12t; 20 audited theorems): the decision table
  (`static_outlives_every_holder`, `foreign_holder_is_undecided`,
  `global_holder_of_a_non_static_is_undecided`,
  `a_caller_object_outlives_a_frame_holder`), the path replay
  (`run_append`, `undecided_is_refused`, `release_refused_iff`), the two
  soundness statements — `a_safe_path_never_releases_a_held_identity` (on a
  path published `Safe`, no release reaches an identity a live record holds:
  the use-after-free hazard is excluded) and
  `a_safe_path_leaves_no_tracked_holder_holding` (only frame holders, which
  end with the frame, may remain at the exit) — the brief's controls
  (`reset_before_release_is_safe`, `release_before_reset_is_refused`,
  `a_holder_released_first_is_safe`, `a_holder_outliving_its_pointer_is_refused`,
  `a_holder_holding_at_the_exit_is_refused`, `a_frame_holder_dies_at_the_exit`,
  `global_holder_controls`, `an_unknown_holder_is_refused`) and the shared
  vectors `decision_vectors_hold` and `trace_vectors_hold`, which bind
  `retained_by.rs`'s `DECISION_VECTORS` and `TRACE_VECTORS` through its unit
  tests.

## 9. Ledger

`scripts/update_ledger.py` (follow-ups 705 → 710):

* **`W48-CALLEE-DECISION-TABLE`** amended: USER DECISION 2026-10-05 verbatim,
  and class 1 re-sized, 36 callees / 94 functions → 27 / 72, with what each
  shape took and what stays.
* **`W45-CALLEE-CASCADE-RESIDUAL`** amended: the remaining population, 625
  (449 by one callee alone; system 26 / 18, slice 38 / 27), and its head.
* **`CG22-CALLBACK-REGISTRY-CONTRACT`** amended with shape (c): every member,
  what is missing and its size (section 5).
* **`CG23-DISPOSITION-TEST-POSITIONS`**, **`CG23-EXIT-SPECIFIC-BLOCKED-HELPERS`**
  and **`W43-LIBC-ABI-READER-SHAPES`** amended (the measurable untested
  population, rule 12s(2)'s first helper, two undeclarable retainers).
* Registered: **`RETENTION-UNDECIDED-PENDING`** (16 corpus functions, 11 sole;
  system 3, 2 sole), **`RETENTION-SUMMARY-FORWARDING`** (7 LZ4 wrappers),
  **`RETAINED-BY-BODY-CHECK`** (the five declarations),
  **`CLASS1-UNDECLARABLE-RETAINERS`** (4 callees, 14 functions alone) and
  **`RELATIONAL-BODY-CHECK-EXACT-NULLABILITY`** (rule 11r(5)'s exact
  comparison, two declarations forced to `nonnull`).

## 10. Gate

GATE-SECTION

## Files

* Rule 12s: `crates/qcore-c-gateway/src/legacy_relational_forwarding.rs`
  (new); hooks in `legacy_summary.rs`, `legacy_relational_summary.rs`,
  `legacy_c.rs` (`analyze_defined_function`); the side-table repairs in
  `crates/qcore-schemas/src/contract.rs` (`carry_side_declarations`,
  `withhold_for_pin_mismatch`, `with_human_holds_in_force`),
  `contract_revision.rs` (`PinMismatch`), `crates/qcore-contract/src/contract_revision.rs`
  (`in_force_before_pins`), `contract_loader.rs` (`check_source_assumptions`),
  `crates/qcore-audit/src/contract_revision.rs` (`apply_pins`),
  `legacy_declared_abi.rs`, `legacy_checked_spec.rs`.
* Rule 12t: `crates/qcore-schemas/src/retained_by.rs`,
  `crates/qcore-contract/src/contract_retained_by.rs`,
  `crates/qcore-c-gateway/src/retained_by.rs`, `legacy_retained_by.rs` (new);
  hooks in `contract_loader.rs`, `legacy_c.rs` (`ContractCall::retention`, the
  minted environment), `legacy_classify.rs`, `legacy_transfer.rs`
  (`AbstractState::retained`, the transfer, release and exit hooks),
  `legacy_summary.rs` (no inference over a retaining name),
  `legacy_reserved_spelling.rs`, `legacy_in_place_custody.rs`,
  `legacy_joined_provenance.rs` and `tests/unit/legacy_c.rs` (the new field).
* Lean: `proof/QCore/RelationalForwarding.lean`, `proof/QCore/RetainedBy.lean`,
  `proof/AuditRelationalForwarding.lean`, `proof/AuditRetainedBy.lean`,
  `proof/QCore.lean`.
* Tests: `crates/qcore-c-gateway/tests/callee_retention_class1.rs`,
  `tests/fixtures/contracts/retained_by.qv`,
  `crates/qcore-contract/tests/retained_by_clause.rs`; pins
  `compiler/tests/contract_loader.rs`, `compiler/tests/qcore_contract_corpus.rs`.
* Contracts: `providers/qcore-provider-librdkafka/contracts/librdkafka/{cjson,lz4_stream}.qv`,
  `providers/quirin-provider-cfl/contracts/cfl/sds_lifetime.qv`,
  `providers/qcore-provider-ctraces/contracts/ctraces/context_lifetime.qv`,
  `providers/quirin-provider-fluent-bit/contracts/fluent_bit/input_chunk_helpers.qv`,
  their `contracts.lock` and (the `qcore-provider-*` crates) `contracts.json`.
* Records: `docs/QCORE_CONTRACT_SPEC_r0_3.md` (C04.13 *Forwarding*, C04.14),
  `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` (G05), `docs/agents/gateway.md`
  (12s, 12t), `docs/agents/README.md`, `docs/STATUS.md`,
  `docs/development/increments/wave-48-part5.md`,
  `docs/reviews/r0_3/acceptance_matrix.json`, `docs/reports/INDEX.md`.
* Scripts: `build.sh`, `measure.sh`, `measure-binary.sh`, `pin.sh`, `sync.sh`,
  `rank.py`, `rank-all.sh`, `tally.py`, `body.py`, `defs.py`, `compare.py`,
  `parity.py`, `parity_rows.py`, `size_members.py`, `targeted.sh`,
  `pins-test.sh`, `tip-probe.sh`, `tip_rows.py`, `gate.sh`, `provider-gate.sh`,
  `finish-gate.sh`, `update_ledger.py`.

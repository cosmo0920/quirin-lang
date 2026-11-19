# AG1-INDIRECT-MEMBER-ACCESS: the access refusal, by sub-shape; rule 61

Card AG1-INDIRECT-MEMBER-ACCESS (round 33, 2026-11-18). USER DECISION
(2026-11-18, verbatim, through the integrator's brief): "We need to launch
those of the implementation cards to resolve the biggest gaps of analyzing
works of the corpus of fluent-bit source tree." Base `a9c6dcf44`
(integrate/cg25-thread-key). Gateway [rule 61](../../agents/gateway.md#constraint-61),
Lean `QCore.TypedefAggregateBase` (9 audited theorems,
`proof/AuditTypedefAggregateBase.lean`). [PREDICTION.md](PREDICTION.md) was
committed before the implementation commit.

## 1. Census (method step 1)

**Input.** The user's `current --monkey-linked` run at Fluent Bit 046accf6
(`build/cycle19/clang/046accf6…/fluent-bit-clang-source/reports/resource-diagnostics.json`,
read only): 1,100 Safe / 3,825 Unknown / 9 Unsafe function rows; 2,285
Unknown rows carry `indirect memory access is outside the resource
projection` (the integrator's 2,290 counts five more rows; this card's
`extract.py` counts Unknown rows only), **232** carry nothing else.

**Method.** MEMBER-ACCESS-PROJECTION's AST census, reused with recorded
changes (`scripts/`): `prep_units.py` rebuilt every analysed text with the
gateway's own preprocessing and accepted it only on the report's
`preprocessed_source_sha256` (**186 of 186** match; one Monkey unit has no
compile identity and is skipped); `classify.py` binds each access leaf to
its function's Clang constructs at its line (9,560 leaf groups, 7 unbound);
`premise.py` gives each leaf one premise; `refine.py` gives each sole-blocker
function a concrete sub-shape and names its owner. The census tables are
[census/premise-table.md](census/premise-table.md) (all rows),
[census/path-table.md](census/path-table.md),
[census/sole-shapes.md](census/sole-shapes.md),
[census/refined-sole-shapes.md](census/refined-sole-shapes.md) and every
sole-blocker site with its analysed statement,
[census/sole-sites.txt](census/sole-sites.txt).

**The analyzer's own reading.** A unit-scoped `current` run of the card base
with the `indirect-census` feature over the 63 non-Monkey units holding a
sole-blocker row (`scripts/census-run.sh census-sole`, `--selection`, binary
`400317f6…`, 92 minutes, exit 0) reproduces the user's rows for those units:
3,393 of 3,394 rows identical in status and leaves (`scripts/compare.py`);
the one difference, `ut_create_fetch_op_with_offset`, is a run-wide fold
(`the run's stores into rd_kafka_op_s.rko_op_cb`) a selection cannot see. Its
gate stream ([census/store-base-causes.md](census/store-base-causes.md))
names why the declaration reader did not type each refused member store's
base: on an automatic local, 869 refusals in 267 functions answer
`TypedefDefinesAggregate` and 72 in 14 functions `TypedefDisagreeing` (depth 1), beside
703 on a typed (`Committed`) local; on a formal, 565 answer
`TypedefDefinesAggregate`. Most of those are scalar stores a later route
(rule 10t) reads; the owned-result stores among them are rule 61's shape.

### 1.1 All rows, by premise (rows mentioning; rows blocked by that premise alone)

| premise | leaf groups | Unknown rows mentioning | sole-blocker rows (premise alone) |
|---|---:|---:|---:|
| `read:pointer-member` | 2,174 | 906 | 42 |
| `call:uncontracted-callee` | 1,893 | 772 | 31 |
| `address:sub-object` | 1,778 | 685 | 35 |
| `form:union` | 740 | 275 | 10 |
| `store:pointer-name` | 465 | 289 | 14 |
| `store:pointer-call-result` | 395 | 240 | 19 |
| `store:pointer-other` | 365 | 239 | 10 |
| `store:pointer-identity-free` / `-null` | 275 / 233 | 83 / 153 | 0 |
| `read:custody-free` | 184 | 112 | 7 |
| every other premise | 1,058 | — | 23 |
| several premises in one row | | | 41 |

### 1.2 The 232 sole-blocker rows, by concrete sub-shape

Ranked by rows the sub-shape alone blocks ([refined-sole-shapes.md](census/refined-sole-shapes.md)
has samples):

| sub-shape | alone | mentioning | owner / why it stays |
|---|---:|---:|---|
| several sub-shapes in one row | 41 | | |
| uncontracted callee whose actual neither reader reads (`rd_kafka_log0`, `rd_kafka_timer_start0`, `rd_list_copy`, `rd_kafka_topic_partition_list_copy`, `mk_list_init`) | 31 | 41 | contracts; rule 12k cannot name the callee until the actual is read |
| an embedded list's address handed to a list helper (`rd_list_add(&c->config, e)`, `rd_list_destroy(&conf->interceptors.on_new)`, `rd_kafka_interceptor_method_add(&rk->…)`) | 22 | 22 | container custody: decision class 1 / rule 46 |
| a pointer member read at an actual whose callee's mode or another premise decides (`strlen(a->p) + strlen(b->q)`, `open(cf->path, …)`) | 17 | 20 | rule 14a admits one call only (below); descriptor cells (rule 38) |
| a formal's or local's pointer stored into a member (`pmi->member = member`, `map->parent = (void *) counter`) | 14 | 21 | decision class 1 (`retained_by`), rule 10v (Q36 example C) |
| a sub-object address at another position (`memset(cctx->hashTable, …)`, `memcpy(copy->base64str, …)`) | 12 | 18 | the callee's mode / interior bounds |
| a formal array's elements released (`*_destroy_array`: `for (…) X_destroy(a[i]);`) | 12 | 12 | rule 5k / W45-MEMBER-RELEASE-PREMISES, by design |
| an owned result stored into a **formal's** member (`rbuf->rbuf_extra = rd_malloc(…)`, `span->attributes = …`) | 11 | 15 | MEMBER-EFFECT-STORES (rule 38i names it where its premises hold) |
| a call through a member or function pointer | 9 | 11 | rule 11p |
| `return rd_kafka_op_err_destroy(rd_kafka_op_req(mcluster->ops, rko, -1));` (the mock API) | 8 | 18 | `rd_kafka_op_err_destroy` has no contract (cross-unit body) |
| an owned result stored into a **local** aggregate's member (`ret->host = rd_strdup(host)`) | 7 | 10 | **rule 61 where the base is alias-spelled** (section 2) |
| a custody-free read beside `?:`, a cast or a descriptor (`close(cf->fd)`) | 7 | 8 | rules 10z, 38 |
| pointer arithmetic stored into a member (`w->op_limit = w->op + len`) | 7 | 8 | rule 10v (an identity-carrying value) |
| a union member path (`LZ4_stream_t`'s `internal_donotuse`, `rko->rko_u.mock.name = rd_strdup(…)`) | 9 | 28 | rule 10w (a pointer store through a union refuses) |
| 12 smaller sub-shapes (an aggregate value, another pointer store, a member released with the parent kept, a log macro, file-scope and static roots, …) | 25 | | each at most 5 |

**What earlier cards refused on purpose, not re-admitted here.** Rule 14a's
"**Do not** admit two calls" (`QCore.ResidualAccessReadings.two_calls_are_never_read`):
a statement holding two `Borrowed`-only calls stays refused; extending it is
a follow-up below, not done here. Rule 10v's identity-carrying values (a
formal's or local's pointer, arithmetic, a member read stored into a member;
Q36's missing owner premise). Rule 10w: a pointer store through a union.
Rule 5k P4 / W45-MEMBER-RELEASE-PREMISES: a member or element of a formal
released while the formal is kept (rule 38j admits the member half under
its own checks; elements in a loop stay refused). Decision class 1
(`retained_by`, the user's 2026-10-05 split) for pointers retained in a
member or a container, class 3 for reference-count keeps
(`rd_kafka_toppar_keep0` stored into `rko->rko_rktp`), MEMBER-EFFECT-STORES
for owned values stored into a formal's member. CG10's "yield about one per
lift" stopping rule still describes the sole set: every remaining sub-shape
above three rows is a design refusal, a decision class or a contract
question.

## 2. Rule 61: a local spelled through a struct alias (method steps 2 and 4)

**The shape.** librdkafka's constructors allocate a record through a typedef
and store owned results into it:

```c
rd_kafka_topic_partition_t *rktpar = rd_calloc(1, sizeof(*rktpar));
rktpar->topic = rd_strdup(topic);       /* refused: indirect memory access */
return rktpar;
```

The keyword spelling (`struct rd_kafka_topic_partition_s *rktpar`) is
admitted by O04-B08's aggregate store (a field link from a live owner this
body acquired; probe s1 at the base, `Safe`). The alias spelling is refused
because the lexical declaration reader answers `Opaque` for a typedef that
**defines** its struct (`typedef struct TAG { … } T;`, kept byte for byte by
ABI-TYPEDEF-ADDRESS-OF-MEMBER) and `None` for a name declared by a forward
and a defining typedef it cannot compare (`rdkafka.h`'s
`typedef struct rd_kafka_Node_s rd_kafka_Node_t;` beside
`rdkafka_aux.h`'s defining one), so B08's premise 2 ("the base is exactly
`struct TAG *`") never held. Probes at the base: s2 (a forward typedef only)
`Safe`; s3 (defining) and s10 (forward and defining) refused at the store.

**Why reading it is sound.** C11 6.7.8p3: a typedef name is a synonym for the
type it names. When every typedef declaration of `A` in the unit names one
`struct TAG`, a local declared exactly `A * NAME` has type `struct TAG *`; B08's
premise is then a true fact, and every other premise (one acquired binding,
no escape, no alias, the member classes of `struct TAG`, the flow-sensitive
owner check, P-B08-SHALLOW) still decides exactly as for the keyword
spelling (`QCore.TypedefAggregateBase.the_store_decides_alike_over_either_spelling`).
The reading refuses a union, anonymous, pointer, disagreeing or incomplete
alias, a qualified or multi-declarator declaration, a formal, a name declared
twice and a name the shared reader typed otherwise; the shared classifier's
answer is unchanged for every other consumer. The forward-and-defining pair
also makes `sizeof(*NAME)` an unevaluated operand (a struct type is never
variably modified, C11 6.7.6.2p2, 6.5.3.4p2), which the base scan needs
(probe s10 was refused at that `*` before the second hook).

**A record this body allocated (the owned-binding fact).** The first unit
reproduction of the committed rule (`b10211ad4`, section 4) moved three rows
Safe -> Unknown: `parpriv = rd_kafka_topic_partition_get_private(rktpar);
parpriv->leader_epoch = e;` binds an alias-spelled local from an accessor
whose declared result is `resource_return_borrowed_from(rktpar, nonnull)`;
once read as an aggregate-store base, the owner check refused it (not an
identity this body acquired) where the typed route (rule 10t) had read the
scalar store. Rule 61 therefore also requires the local's one binding to be a
direct call (under at most one pointer cast) whose contract in force returns
an **owned** result. That is the census shape (a constructor's own
allocation) and leaves every other base to the routes that read it before.
PREDICTION.md revision 2 was committed with the first measurement before
the narrowing commit (`c3c57d83d`).

**Re-attribution, not admission.** `rd_kafka_Node_new` stores `rack` on one
arm only (`if (rack != NULL) ret->rack = rd_strdup(rack);`); the store is now
read and the join's partial link meets rule 11c's escape refusal (`field-link
escape: a linked child may still be live on a path the escape does not carry
it`), unchanged and honest.

**Fixtures** (`crates/qcore-c-gateway/tests/ag1_indirect_member_access.rs`,
6 tests; every `Safe` result's certificate rechecked): admitted, a defining
alias and a forward-and-defining alias constructor (both declaration forms);
kept non-`Safe`, a released owner leaving its child live (P-B08-SHALLOW), an
aliased base, a release before the store, an overwrite in a loop, a store
through a second path (`slot = &ret->host; *slot = …`), a union alias, a
pointer alias, two declarators, a formal base (rule 38i's name, never the
gate's text), and the one-arm store (rule 11c). Unit tests in
`typedef_aggregate_base.rs` (6) bind the decision to the Lean vectors (128)
and cover the alias table's refusals and the owned binding (a borrowed
accessor result, an uncontracted call, a copy, two bindings, none).

**Lean** `QCore.TypedefAggregateBase`: `an_agreeing_alias_names_its_struct_everywhere`,
`a_disagreeing_alias_names_nothing`, `an_other_declaration_names_nothing`,
`a_missing_fact_never_admits` (seven facts, the owned binding among them),
`decision_vectors_hold`,
`an_admitted_local_is_the_struct_pointer`,
`the_store_decides_alike_over_either_spelling` (over
`QCore.AggregateStore.admitsStore`), `a_refused_alias_keeps_the_refusal`,
`a_struct_alias_is_never_variably_modified`; `propext` only, no
placeholders.

## 3. Shared-file hook points

Every edit outside the new module is a delimited hook
(`AG1-INDIRECT-MEMBER-ACCESS (rule 61) hook`):

- `crates/qcore-c-gateway/src/typed_field_access.rs`: the field
  `TypedAccessModel::struct_alias_heads`; the free function `struct_head`
  (above `read_aggregate_definition`); the accessor
  `TypedAccessModel::struct_alias_tag` (after `defining_struct_alias`); one
  call in `read_typedefs` before the `typedefs` merge; one match arm
  (`Some(None)`) in `never_variably_modified_at`.
- `crates/qcore-c-gateway/src/aggregate_store.rs` (not on the brief's shared
  list): `AggregateStoreBases::alias_tags` and `base_tag`; the candidate test
  of premise 2 in `store_bases`; `insulated_bases`' extra parameter and its
  `&BASE->F` arm; `admitted_field`'s tag; `member_address_actuals`' tag.
- `crates/qcore-c-gateway/src/lib.rs`: `mod typedef_aggregate_base;`.
- `docs/reports/libc-reading-site-evidence/bindings.json`: the
  declaration-reader digest of `typed_field_access.rs` (re-pinned; a card
  that also edits `typed_field_access.rs` must recompute it after the merge).
- Fallback for the integrator if the `typed_field_access.rs` hooks conflict:
  every row this card moved uses a **defining** typedef only, which the
  existing `TypedAccessModel::defining_struct_alias` already answers; reading
  through it instead (no `struct_alias_heads`, no `never_variably_modified_at`
  arm) keeps the eight moves and loses only the forward-and-defining pair
  (`rd_kafka_Node_new`'s re-attribution). Not measured.
- None of `legacy_c.rs`, `legacy_cfg_builder.rs`, `legacy_transfer.rs` or
  `typed_access_path.rs` is touched.

## 4. Measurement (unit reproduction of the user's `current`)

The user's full `current` route (2+ hours, ~10 GB) was not rerun. Every
non-Monkey unit holding an access row (135 units, `private/ag1/selection-all.json`
from `scripts/selection.py --all`) was run with `quirin-audit current
--selection` twice, report only (`AG1_REPORT_ONLY=1 scripts/census-run.sh`):
BEFORE the card base's census-feature binary (`400317f6…`), AFTER the card's
(`ff179785…` for the committed rule `b10211ad4`, `52d92287…` for the narrowed
rule `c3c57d83d`); each exit 0. BEFORE reproduces the user's rows for these
units, 4,430 of 4,431 identical (the one difference is section 1's run-wide
fold, `ut_create_fetch_op_with_offset`). Monkey (14 sole-blocker rows) is
analysed only through `--monkey-linked` and was not reproduced; rule 61 reads
no Monkey shape the census found.

| side | Safe / Unknown / Unsafe (135 units) | access rows | access alone | access leaves |
|---|---|---:|---:|---:|
| BEFORE (`a9c6dcf44`) | 939 / 3,485 / 7 | 2,140 | 218 | 9,021 |
| first AFTER (`b10211ad4`) | 942 / 3,480 / 9 | | | 8,993 |
| **AFTER (`c3c57d83d`)** | **945 / 3,477 / 9** | **2,130** | **215** | **8,994** |

**The narrowed rule's moves** ([artifacts/compare-current-v2.json](artifacts/compare-current-v2.json)),
exactly PREDICTION.md revision 2's block, no Safe -> non-Safe move:

- Unknown -> Safe (6): `rd_kafka_topic_partition_new` and
  `rd_kafka_topic_partition_new_with_id_and_name` (`rdkafka_partition.c`),
  `cJSON_CreateStringReference`, `cJSON_CreateObjectReference`,
  `cJSON_CreateArrayReference` (`cJSON.c`, `cJSON *item =
  cJSON_New_Item(&global_hooks)`, the owned `cJSON_New_Item` declaration),
  `ut_ack_create_message` (`rdunittest_acknowledge.c`).
- Unknown -> **Unsafe** (2, provisional): `ut_case_acknowledge_accept`
  (`live obligation acquired at rdunittest_acknowledge.c:261`) and
  `ut_case_acknowledge_multiple_partitions` (`:533`, `:534`). With
  `ut_ack_create_message` now validated as returning an owned message, its
  librdkafka **unit-test** callers are read to leak the message on the paths
  where an `RD_UT_ASSERT` fails and returns before `ut_ack_destroy_message`;
  their success paths release it. This is the analysis's provisional finding
  on test code, not a claim about librdkafka; each row also keeps its
  uncontracted callees.
- Leaves: 27 access leaves removed, none added; 31 rows change leaves.
  Removed beside them: `call to uncontracted function ut_ack_create_message`
  10, `rd_kafka_topic_partition_new` 2, `unresolved resource identity`
  `msg`/`item` 12, `nullable resource item` 3. Added (re-attributions on rows
  that stay Unknown): `unresolved resource identity` 20 (`batches`, `node`,
  `cpair`, …: later uses of a base whose store is now read), `call to
  uncontracted function cJSON_strdup` 2, rule 11c's `field-link escape` 1
  (`rd_kafka_Node_new`, as predicted), `nullable resource node` 1, and the
  four leak leaves above.

**The first AFTER** (the committed rule before the owned-binding fact;
[artifacts/compare-current-v1.json](artifacts/compare-current-v1.json)) moved
the same eight rows plus `rd_kafka_topic_partition_set_leader_epoch`,
`rd_kafka_topic_partition_set_current_leader_epoch` and
`rd_kafka_topic_partition_set_from_fetch_pos` **Safe -> Unknown** (section 2),
which the narrowing removed.

**Against the first prediction.** Revision 1 named two rows; one moved
(`rd_kafka_topic_partition_new`). `rd_kafka_share_ack_batch_entry_new` did not
move: its member `types` is declared `rd_kafka_share_internal_acknowledgement_type
*types;` over a **defining enum typedef**, which no B08 member class reads (a
member-typedef shape, AG1-POINTER-TYPEDEF-MEMBER). Revision 1's "no new
Unsafe" and "no Safe -> non-Safe" were wrong for the committed rule; revision
2 holds exactly. `rd_kafka_UserScramCredentialDeletion_new` (a forward-only
typedef over a struct with a union member: an incomplete record for B08) and
`rd_kafka_topic_partition_new_from_rktp` (a deep member chain in the call's
actuals) keep the access refusal, as section 1's owners say.

**The user's `current` run, expected after landing.** The same eight moves
(the reproduced units are the full run's), access-alone rows 232 -> 229, no
Monkey change. Corpus and twelve-home: the moves of revision 2's block where
the corpus analyses those units (`rdkafka_partition.c` and
`rdkafka_share_acknowledgement.c` byte-identical at `66910c10`); system and
slice: none.

## 5. Gate

Scoped card gate `python3 scripts/gate_scope.py --base a9c6dcf44 --out
private/gate-2` (`scripts/gate.sh 2`) at `da221ce11` (code as `c3c57d83d`;
mode full: the diff reaches qcore-c-gateway), exit 1; logs in the worktree's
`private/gate-2/`, summary `private/gate-2/gate.json`:

| step | exit | result |
|---|---:|---|
| submodules | 0 | 1 submodule |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| `run_test_binaries.py` (rust-suite slot) | 101 | 661 units, 706 summaries; 9,699 passed, **5 failed**, 21 ignored, 148 filtered (`private/gate-2/rust-suite/`) |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `check_lean_audits.py` | 0 | 449 audits |
| `python3 -m unittest discover -s scripts/tests` | 1 | 779 tests, 1 failure: `test_gate_slot_starvation.Resident.test_it_is_reported_while_the_job_runs` (resident set 60,140 KiB < 65,536) |

**The five Rust failures.** `libc_reading_site_evidence` (3) and
`reading_site_domain` (1) failed only with `reading-site: artifact reference
mismatch: crates/qcore-c-gateway/src/typed_field_access.rs`:
LIBC-READING-SITE-EVIDENCE's binding manifest pins that file's sha256 as its
declaration-reader artifact. Rule 61's hooks change the file, so the digest
was re-pinned deliberately (`748c82224`, `f2429ce0…` -> `9f155d2d…`, anchor
`fn scalar_pointer_result` unchanged); both binaries then pass alone (6 and 1
tests, `private/ag1/rerun1/reading-site.log`, exit 0).
`libcurl_user_crate_versions` (1): `the_libcurl_ranges_follow_the_audited_curlver_header`
expects `Unknown` once the fixture tree's `curl/curlver.h` is removed, but the
audit then reads the host's `/usr/include/x86_64-linux-gnu/curl/curlver.h`
(8.18.0, installed 2026-10-13) and answers `Safe`: an environment dependence
of that test, untouched by this card (no libcurl, header or version reader
changed). Rerun alone in a rust-suite slot it fails the same assertion
(`libcurl_user_crate_versions.rs:335`, 1 passed, 1 failed, exit 101,
`private/ag1/rerun1/libcurl.log`): a host-environment failure, recorded, not
caused by this card.
**The Python failure** is the timing- and load-sensitive gate-slot resident
test; rerun alone it passes (`private/ag1/py-rerun.log`, exit 0), recorded
per the brief.

Targeted runs before the gate: `cargo test -p qcore-c-gateway --lib` (1,317
passed at the first rule; the module's 6 unit tests at the narrowed rule),
`tests/ag1_indirect_member_access.rs` (6 passed), `cargo clippy -p
qcore-c-gateway --all-targets -- -D warnings` 0, `lake build` of every Lean
target 0 (508 jobs), `apply_amendments.py --check` 0. An earlier gate,
`private/gate-1`, was cancelled when the rule was narrowed and is not
evidence; one AFTER build failed for a full disk (`private/ag1/build-after.log`)
and is not evidence.

## 6. Follow-ups (registered in [integration-amendments.json](integration-amendments.json))

- **AG1-TWO-BORROWED-CALLS** (new): rule 14a's one-call limit; several
  `Borrowed`-only, `None`-result calls in one scalar statement or nested
  (`strlen(a->p) + strlen(b->q)`, `cmt_atomic_store(&d->f,
  cmt_atomic_load(&s->f))`; probes s11, s12).
- **RK-OP-ERR-DESTROY-CONTRACT** (new): the mock API's nested return; 8 rows
  alone, 18 mentioning.
- **AG1-POINTER-TYPEDEF-MEMBER** (new): a member declared through a pointer
  typedef (`cfl_sds_t`) or a pointer to a defining-typedef enum
  (`rd_kafka_share_ack_batch_entry_new`'s `types`) is in no B08 member class.
- **AG1-FORMAL-ARRAY-ELEMENTS-RELEASED** (new, a user decision): the twelve
  `*_destroy_array` helpers.
- **W44-CALL-RESULT-INDIRECT-DESTINATION** and **MEMBER-EFFECT-STORES**
  (amended with this census).
- Not registered again: decision class 1 (embedded lists, retained
  pointers), class 3 (reference-count keeps), rules 10v and 10w's refusals
  and the uncontracted-callee residual already tracked as
  W45-CALLEE-CASCADE-RESIDUAL.

**Environment note.** `/media/Data` filled to 0 bytes several times during
the card; this card's own debug test build (17 GB, deleted at once) was one
contributor at 21:1x JST. One AFTER build attempt failed with ENOSPC
(`private/ag1/build-after.log`, retained, not evidence) and was rebuilt; the
first gate attempt (`private/gate-1`) was cancelled when the rule was
narrowed and is not evidence.

## Integrator measurement (2026-11-19)

Measured by the integrator with `quirin-audit measure` over the four live routes
(`crates/quirin-audit-plugin-measure/inputs/live.json`), base
`rev:a9c6dcf44`, tip `rev:e94468466` (the only later commit, 03561d8ce, edits
this README), declared against this card's PREDICTION.md
(`private/measure/ag1-20261118T185528Z`; comparison beside this file as
[integration/native-comparison.json.gz](integration/native-comparison.json.gz),
sha256 `bfa7a076665267e95f85efdc18477b215c5ea5a648b77f550764224c3fbea284`
uncompressed).

- corpus: **match**: 8 moved, 0 missing, 0 unpredicted. Unknown→Safe:
  `rd_kafka_topic_partition_new`, `rd_kafka_topic_partition_new_with_id_and_name`,
  `cJSON_CreateStringReference`, `cJSON_CreateObjectReference`,
  `cJSON_CreateArrayReference`, `ut_ack_create_message`. Unknown→Unsafe
  (provisional): `ut_case_acknowledge_accept` and
  `ut_case_acknowledge_multiple_partitions`, librdkafka's own unit tests in
  `rdunittest_acknowledge.c`. Their new leaves are `leak` reports at the
  assertion macros' early returns, where an object the test allocated is still
  live; they are provisional and are not presented as upstream defects.
- twelve-home: the measurement reports **mismatch** only because the
  prediction carries no `route twelve-home` block (its prose says
  "same as corpus"); the 8 unpredicted moves there are exactly the 8 declared
  for corpus, with the same before/after statuses.
- system, slice: match, 0 moves.

### Combined landing measurement

All three round-33 cards together (`integrate/ag-gaps` 8a85e233f against
base `rev:2bf47a710`, the CG25 landing tip; no declaration;
`private/measure/agall-20261119T081826Z`; beside this file as
[integration/combined-native-comparison.json.gz](integration/combined-native-comparison.json.gz),
sha256 `1b23b95f4b81679293be66bf9e3c0b485e37a7b41c20593feae68e033de553e9`
uncompressed): corpus and twelve-home 42 moved (Unknown −42, Safe +40,
Unsafe +2), system 5 moved (Unknown −5, Safe +5), slice 0. The moved row set,
with statuses, equals the union of the three cards' separate measurements on
every route: the readings do not interact. Gate:
[integration/gate.json](integration/gate.json).

# CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 3: prediction

Committed before the implementation commit. Base `f916d3007` (main
`ffb41c748` is the same code plus two record-only commits). USER DECISION
(2026-11-20, verbatim): "Also, we always blocked indirect-memory access so we
must proceed to jump on it." and "Also, we need to proceed other types of
indirect memory accesses."

**Disclosure.** The tip binary with both readings was built and its opt-in
`current` run (`private/r3/run-tip1`) finished while the census was being
computed; its report was **not read** before this file was committed. The
predictions below come from the base census only.

## 1. Census on the base (method step 1)

Route: the opt-in `quirin-audit current --monkey-linked` over Fluent Bit
046accf6 (`openssl-ssl-ctx-callback-slots/evidence/opt-in/opt-in-current.sh`,
inputs prepared on this host into `private/fb-046accf6`), base binary
`indirect-census` build of `f916d3007`, exit 0:
**1,139 Safe / 2 Pending / 3,656 Unknown / 7 Unsafe** function rows.
**2,198** Unknown rows carry `indirect memory access is outside the resource
projection` (9,186 leaves; librdkafka 1,688 rows, cmetrics 192, Monkey 146,
ctraces 82, fluent-bit-chunk 50, chunkio 40); **242** carry nothing else.

AST census (AG1's `prep_units.py` 180 of 180 units sha256-matched,
`classify.py` 9,202 leaf groups, 0 unbound, `premise.py`, `refine.py`):
[census/premise-table.md](census/premise-table.md) (table b),
[census/refined-sole-shapes.md](census/refined-sole-shapes.md) (table a),
[census/candidates.md](census/candidates.md) (the two readings' reach).

### Table (a): sole-blocker classes, by functions freed alone

| sub-shape | alone | mentioning | owner |
|---|---:|---:|---|
| several sub-shapes | 46 | | |
| uncontracted callee, an actual neither reader reads | 40 | 55 | contracts (W45-CALLEE-CASCADE-RESIDUAL) |
| embedded list address to a list helper | 22 | 22 | decision class 1 / rule 46 |
| pointer member read at an actual (callee mode, two calls) | 17 | 20 | **rule 67** for two sibling calls; the callee's mode otherwise |
| formal's or local's pointer stored into a member | 13 | 19 | decision class 1 (`retained_by`), rule 10v |
| sub-object address at another position | 12 | 17 | callee's mode, interior bounds |
| formal array's elements released | 12 | 12 | rule 5k, by design (AG1-FORMAL-ARRAY-ELEMENTS-RELEASED) |
| owned result into a formal's member | 11 | 14 | MEMBER-EFFECT-STORES |
| `return rd_kafka_op_err_destroy(rd_kafka_op_req(…))` | 8 | 18 | RK-OP-ERR-DESTROY-CONTRACT |
| call through a member or function pointer | 8 | 10 | rule 11p |
| custody-free read beside `?:`, a cast, a descriptor | 7 | 8 | rules 10z, 38 |
| pointer arithmetic stored into a member | 7 | 8 | rule 10v |
| union member access / pointer store through a union | 5 / 4 | 10 / 18 | rule 10w |
| owned result into a local member | 5 | 8 | B08 premises; **rule 68** where the member is a pointer typedef |
| 14 smaller sub-shapes | 25 | | each at most 5 |

### Table (b): every Unknown row mentioning the refusal, by premise

| premise | leaf groups | rows mentioning | sole rows (premise alone) |
|---|---:|---:|---:|
| `read:pointer-member` | 2,085 | 877 | 42 |
| `address:sub-object` | 1,867 | 688 | 35 |
| `call:uncontracted-callee` | 1,688 | 711 | 40 |
| `form:union` | 732 | 273 | 10 |
| `store:pointer-name` / `-call-result` / `-other` | 448 / 367 / 354 | 278 / 220 / 230 | 13 / 17 / 10 |
| `store:pointer-identity-free` / `-null` | 274 / 215 | 82 / 137 | 0 / 0 |
| `read:custody-free` | 184 | 112 | 7 |
| every other premise | 988 | | 22 |

## 2. Readings (method step 2)

**Rule 67 (AG1-TWO-BORROWED-CALLS; lifts rule 14a's "do not admit two
calls").** Rule 14a's statement shapes with several sibling calls (none
inside another's actuals), each `None`-result and all-`Borrowed`, each
lowered alone. Census reach: 1,084 leaf groups in 423 Unknown rows hold two or
more direct calls all contracted in the run (the AST census cannot tell a
`None` result or a scalar statement shape); 1,144 more in 447 rows hold an
uncontracted callee (no admission: the reading declines). Sole rows all of
whose leaves hold two calls: 21; of them **three** have only contracted
callees: `get_stream_path` (`len = strlen(ctx->options.root_path) +
strlen(st->name) + 2;`), `append_metric_value` (a nested
`cmt_math_uint64_to_d64(cmt_atomic_load(…))`, refused by the sibling premise,
and aggregate copies: stays) and `rd_kafka_queue_poll` (a call result bound
to a pointer `rko`: not a scalar statement, stays).

**Rule 68 (AG1-POINTER-TYPEDEF-MEMBER).** A member declared through an
agreeing pointer typedef (`cfl_sds_t name;`) joins the single-pointer and
borrowable member classes its `char *` spelling joins. Census reach (members
whose analysed declaration spells a pointer typedef of the unit, almost all
`cfl_sds_t`): 189 leaf groups; owned-result stores into a local's member
(O04-B08's shape) 19 leaf groups in 14 rows, plus `flb_sds_t` 2 and
`cothread_t` 2; reads 54 + 35 (most already read by the Clang-typed routes,
which see the desugared type). Sole rows all of whose leaves touch such a
member: 18 (`ctr_instrumentation_scope_create`; 15 ctraces/cmetrics
`unpack_*` handing `&x->sds_member` to the uncontracted
`cmt_mpack_consume_string_tag` / `ctr_mpack_consume_string_tag`: a contract
question, re-attribution at most; `compare_fqname`, `compare_label_keys`: an
indirect call, rule 11p).

## 3. Predicted moves (opt-in `current`)

* Rule 67: Unknown -> Safe **0 to 2** (`get_stream_path` predicted; one more
  among the non-sole rows whose only other blockers are access leaves on the
  same two-call statements). Leaves: access leaves removed on statements of
  the "all contracted" class where every callee is `None`/all-`Borrowed`
  (predicted 50-300 removed), re-attributed to the identities or callees the
  newly read statements expose; no status move elsewhere.
* Rule 68: Unknown -> Safe **1 to 3** (`ctr_instrumentation_scope_create`
  predicted; possibly a cmetrics/ctraces constructor storing `cfl_sds_create`
  results whose other leaves are those stores). Leaves: the 14 B08 rows'
  store leaves removed or re-attributed (an unresolved identity, rule 11c's
  escape, a `nullable` leaf); the `unpack_*` rows re-attributed to
  `call to uncontracted function cmt_mpack_consume_string_tag` where rule 12k
  then reads the actual (no status move).
* **No Safe -> non-Safe move; no new Unsafe.** Access-alone rows 242 ->
  239-241.

The opt-in `current` route (this card's arbiter): `cio_stream.c::get_stream_path`
and `ctr_scope.c::ctr_instrumentation_scope_create` Unknown -> Safe.

```measure-prediction
route corpus
move get_stream_path Unknown -> Safe
move ctr_instrumentation_scope_create Unknown -> Safe
route system
none
route slice
none
```

Corpus (`66910c10`) and twelve-home: the same two rows where the corpus
analyses chunkio's `cio_stream.c` and ctraces' `ctr_scope.c` (to be confirmed
by the integrator's `quirin-audit measure --declared`, which cannot run on
this host: no `build/cycle19` inputs or baselines); system and slice: none.

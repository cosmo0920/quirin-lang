# CHAINED-ASSIGNMENT-INNER-DESTINATION prediction

Committed after the census (README §2, `census.json`) and before the
semantic change. Base: `baseline:7b726587bdf6a8213f451fa1c78297bb5e476ef7`
from the main checkout's `private/baselines/`: `eight-home` (corpus),
`fluent-bit-system` (system) and `fluent-bit-tail-pause` (slice). The card's
base `1e522ca5d` moved no verdict from it (brief).

**What changes.** `classify_simple` gains a guard: an admission of a statement
in which an assignment's value holds another assignment is replaced by a named
refusal (it never admits). Rule 10s's elaborator, asked after every older
route refused, lowers a chain as its sequential assignments (and now also
`T a = b = E;`). Only statements holding a write inside an assigned value can
move; the census lists every such statement inside an analysed function.

**Affected rows at the base** (census §2; every other row's lowering is
byte-identical):

| Route | Function (unit) | Base | Predicted | Why |
|---|---|---|---|---|
| corpus | `rd_kafka_sasl_safe_string` (`rdkafka_sasl_scram.c`) | Unsafe | Unknown | `:243` `d = safe = rd_malloc(len + 1)` was read as `d = rd_malloc(…)`; now `safe = rd_malloc(…)` then `d = safe`, so `return safe;` (`:249`) escapes the allocation and the leak (its only `leak` leaf) is withdrawn. The row keeps `indirect memory access` (`:214`, the `char *safe = NULL, *d = NULL` declaration) and its unresolved aliases, so it is Unknown, never Safe. The card's fixture of the shape loses `loop re-acquisition` at `:243` and gains `nullable resource \`safe\` used by a non-null contract` (the `safe = NULL` initial value reaching `return`); either way the row stays Unknown. |
| corpus | `mk_http_parser` (`mk_server/mk_http_parser.c`) | Unknown | Unknown | `:1100` `p->start = p->header_val = p->i` and the two `header_scope_eq` expansions (`:1023`, `:1038`, `p->header_min = p->header_max = x`) were admitted by the typed scalar-field route as one `IndirectScalarAccess` each; now refused by the guard and lowered by rule 10s as one access per link on the same base. No custody place moves; leaves on these lines may be re-attributed (content change only). |
| corpus | every other census row (19 functions) | Unknown | Unknown | Already refused by an older route (the guard keeps a refusal byte for byte), lowered by rule 10s before the card (`parse_array`, `parse_object`, `rd_kafka_mock_partition_pidstate_get`), or not compiled in the configuration (`rdxxhash.c` under `XXH_ACCEPT_NULL_INPUT_POINTER` = 0; `rwlock_init` is `_TTHREAD_WIN32_` only). |
| system | none | | | The census finds no write inside an assigned value in an analysed function of the eight system files. |
| slice | none | | | Likewise for the tail-pause slice. |

```measure-prediction
route corpus
move librdkafka::rdkafka_sasl_scram.c::rd_kafka_sasl_safe_string Unsafe -> Unknown
delta Unsafe -1
delta Unknown +1
route system
none
route slice
none
```

**The user's `current` run** (Fluent Bit `046accf6`, profile
`contract/profiles/fluent-bit`), rerun by the integrator after landing:

- `rd_kafka_sasl_safe_string` Unsafe → Unknown, for the reason above; the
  unit `lib/librdkafka-2.15.0/src/rdkafka_sasl_scram.c`, whose only finding it
  was, leaves provisional Unsafe for Unknown (provisional Unsafe −1, Unknown
  +1). Never Safe from this rule alone.
- `mk_http_parser` (`lib/monkey/mk_server/mk_http_parser.c`) stays Unknown;
  its three chain lines are lowered one access per link (content only).
- No callee leaves the uncontracted list and no function leaves the "blocked
  only by uncontracted calls" set: no contract changes, and the one moved
  row keeps refusals that are not uncontracted calls.
- No other unit changes status.

Any other movement is a measurement finding, not an effect predicted here.

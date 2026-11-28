# OTEL-PROTO-PHASE-B1 integration (round 41–42 landing)

The round 41–42 landing (`integrate/r41-r42`) merged G1-CLOSURE-FORMULAS,
CONNECT-LIBC-EFFECT-CLAIMS, OTEL-PROTO-AUDIT and OTEL-PROTO-PHASE-B1. The other
three cards predict no move, so B1's prediction ([PREDICTION.md](../PREDICTION.md))
is the declared prediction of the landing. The gate record is
[gate.json](gate.json); the raw scope record stays private, named there by its SHA-256.

## Measurement

Measured twice with `quirin-audit measure` over the four live routes. The
second record is the one beside this file
([native-comparison.json.gz](native-comparison.json.gz)).

1. **Declared**, base `rev:1f672bfca`, tip `rev:351ad67c2` (before main's r84–r86):
   `--declared ../PREDICTION.md` gave **mismatch** on corpus and twelve-home and
   **match** on system and slice. All seven declared rows moved, plus five rows
   the prediction did not name.
2. **Merged tree**, base `rev:287dce8ed` (main after r84–r86), tip
   `rev:1a289aa10`: the same twelve moves on corpus and twelve-home, none on system
   or slice, 0 unmatched rows. Main's rule 46 change does not interact with B1.

| Route | Before (Safe / Unknown / Unsafe) | After |
|---|---|---|
| corpus | 1,190 / 3,632 / 12 | 1,200 / 3,620 / 14 |
| twelve-home | 1,190 / 3,632 / 12 | 1,200 / 3,620 / 14 |
| system | 124 / 320 / 4 | unchanged |
| slice | 65 / 4 Pending / 334 / 1 | unchanged |

### The five unpredicted moves

Each moved row's after-side contract dependencies gain a B1 `opentelemetry__*`
declaration or an `initialize_*` summary that B1's contracts made analysable.
None depends on the libc or G1 cards.

| Row | Move | Leaves removed or added |
|---|---|---|
| `cmetrics::cmt_encode_opentelemetry.c::otlp_kvpair_value_initialize` | Unknown → Safe | uncontracted `key_value__init` and the possibly-null result leaf removed |
| `ctraces::ctr_encode_opentelemetry.c::otlp_kvpair_value_initialize` | Unknown → Safe | same |
| `cmetrics::cmt_encode_opentelemetry.c::render_opentelemetry_context_to_sds` | Unknown → Safe | uncontracted `metrics_data__get_packed_size` / `__pack` and the `result_buffer` alias and null leaves removed |
| `ctraces::ctr_encode_opentelemetry.c::set_spans` | Unknown → Unsafe (provisional) | leak: obligation acquired at :876 live at :891 |
| `ctraces::ctr_encode_opentelemetry.c::set_scope_spans` | Unknown → Unsafe (provisional) | leak: obligation acquired at :984 live at :1000 |

**Source reading** (pinned copy, `lib/ctraces/src/ctr_encode_opentelemetry.c`).
In both loops the array is allocated before the loop (`initialize_spans` :876,
`initialize_scope_spans` :984). When the first element's `initialize_span()` /
`initialize_scope_span()` returns NULL (its `calloc` failed), the cleanup
`destroy_spans` / `destroy_scope_spans`, which frees the array, is guarded by
`index > 0`, so the function returns NULL with the array still held. The path
needs an allocation failure. These are provisional Unsafe rows of this record,
not upstream defect reports.

**Decision.** USER DECISION 2026-11-28: "Land with record". The committed
prediction is left as written. Follow-up
`OTEL-PROTO-PHASE-B1-CALLER-PREDICTION` records the gap: the B2+B3 prediction
names the callers of every row it moves, and the two provisional Unsafe rows
get a native or witness check before any claim beyond this record.

## Landing repairs

- `docs/reports/otel-proto-audit/census.json` exceeded the 512 KiB report
  ceiling. The writer now splits the per-function and per-site rows into a
  byte-stable `census-rows.json.gz`, and `load_census` merges them back; the
  round trip is lossless.
- The gate read census was below 90% coverage (703 of 784 units). The 24 new
  test units were traced and merged (727 of 784), as in 1f024cb7a.
- G1-CLOSURE-FORMULAS lands as a supplement to P0-G1-CLOSURE-RECORD (user
  decision "Supplement"). The INTEGRATOR-QUEUE placeholders main held for it
  and its four follow-ups were dropped at the merge.

# CFL-MPACK-EXTERNAL-DECLARATIONS prediction

Base: `baseline:c8c7edf21`, from the main checkout's
`private/baselines/c8c7edf2165a2065c234f3a63b4eed23e937ced6/`. Its three
routes are `eight-home` (corpus, Fluent Bit `66910c10`), `fluent-bit-system`
(system, `046accf6`) and `fluent-bit-tail-pause` (slice, `34155955`). The tip
is `rev:25c143cbd`, the last code commit. Revision 2 changes no code or
contract.

## Revision 2 (after the integrator's measurement, 2026-11-05)

Revision 1 was committed (`19c39ce56`) before any contract changed; it is
kept verbatim below. The integrator measured it at
`private/measure/cfl-mpack-20261104T220100Z`:

- **System and slice** matched.
- **Corpus** did not: 16 moves where 7 were declared, and two unit moves.
  - All 7 declared moves happened.
  - 8 more rows moved Unknown -> Safe.
  - `decode_statsd_message` moved Unknown -> Unsafe.

Revision 1 counted only rows whose sole leaves were in-scope uncontracted
calls. The 9 extra rows carried other leaves too: an `unresolved resource
identity` leaf on the fetched value, on `out` or on `attr`, or a
`slot-move-undecided` leaf. The uncontracted call had produced those leaves,
and the declaration removed them. README §4 reviews each row. In summary:

- **The 8 new Safe rows** rest only on this card's stated clauses and on
  declarations and summaries already in force:
  - `cfl_kvlist_fetch`'s borrowed result, read through its fields:
    `fetch_metadata_int64_key`, `kvlist_fetch_bool`, `kvlist_fetch_double`,
    `kvlist_fetch_uint64`. Also `resolve_data_point_start_timestamp`,
    through `kvlist_fetch_uint64`'s new summary.
  - The owned `cfl_sds_create_size` result with the borrowed length setters:
    `double_to_string`, whose `cfl_sds_cat_safe` slot move is now decided on
    a tracked local; and `ctr_id_to_lower_base16`.
  - The `cfl_kvlist_destroy` release of `attr->kv`: `ctr_attributes_destroy`.
- **The new Unsafe row is a provisional Unsafe, validated as a reachable
  double release from the bodies at all three revisions.**
  - In `decode_statsd_message`, the COUNTER, GAUGE, SET and TIMER cases
    call `cfl_sds_destroy(metric_name)` when the entry decoder fails. They
    then `break` to the unconditional `cfl_sds_destroy(metric_name)` at
    `:490`.
  - `cmt_*_create` copies the name (`cmt_opts_init`: `opts->name =
    cfl_sds_create(name)`), so the caller keeps custody of `metric_name`.
  - `cfl_sds_destroy` is not idempotent: it calls `free(head)` on a heap
    string.
  - The failure is reachable from input: a counter line with 17 distinct
    tag keys makes `decode_labels` return
    `CMT_DECODE_STATSD_INVALID_ARGUMENT_ERROR`.
  - README §4 gives the witness. The `cfl_sds_create_len` declaration is
    right, so nothing in it changes.

```measure-prediction
route corpus
move cmetrics::cmt_encode_cloudwatch_emf.c::pack_summary_metric Unknown -> Safe
move ctraces::ctr_attributes.c::ctr_attributes_create Unknown -> Safe
move ctraces::ctr_decode_opentelemetry.c::convert_bool_value Unknown -> Safe
move ctraces::ctr_decode_opentelemetry.c::convert_bytes_value Unknown -> Safe
move ctraces::ctr_decode_opentelemetry.c::convert_double_value Unknown -> Safe
move ctraces::ctr_decode_opentelemetry.c::convert_int_value Unknown -> Safe
move ctraces::ctr_decode_opentelemetry.c::convert_string_value Unknown -> Safe
move cmetrics::cmt_encode_opentelemetry.c::fetch_metadata_int64_key Unknown -> Safe
move cmetrics::cmt_encode_opentelemetry.c::kvlist_fetch_bool Unknown -> Safe
move cmetrics::cmt_encode_opentelemetry.c::kvlist_fetch_double Unknown -> Safe
move cmetrics::cmt_encode_opentelemetry.c::kvlist_fetch_uint64 Unknown -> Safe
move cmetrics::cmt_encode_opentelemetry.c::resolve_data_point_start_timestamp Unknown -> Safe
move cmetrics::cmt_encode_splunk_hec.c::double_to_string Unknown -> Safe
move ctraces::ctr_attributes.c::ctr_attributes_destroy Unknown -> Safe
move ctraces::ctr_id.c::ctr_id_to_lower_base16 Unknown -> Safe
move cmetrics::cmt_decode_statsd.c::decode_statsd_message Unknown -> Unsafe
delta Safe +15
delta Unsafe +1
route system
none
route slice
none
```

Corpus unit moves follow: `ctraces::ctr_attributes.c` Unknown -> Safe
(every function Safe), and `cmetrics::cmt_decode_statsd.c` Unknown ->
Unsafe (provisional).

**The user's `current` run, revised** (Fluent Bit `046accf6`, profile
`contract/profiles/fluent-bit`), rerun by the integrator after landing.

The rows are read from the run report. The source facts are these:

- `cmt_decode_statsd.c`, `cmt_encode_opentelemetry.c`,
  `cmt_encode_splunk_hec.c`, `cmt_opts.c` and the `cmt_counter/gauge/untyped`
  files are byte-identical at all three revisions.
- The ctraces files differ, but `ctr_attributes_create`,
  `ctr_attributes_destroy` (which adds a leading NULL return) and
  `ctr_id_to_lower_base16` keep their shape at 046accf6.
- `convert_bytes_value` adds a call on `ctr_val->ctr_attr->kv`, which is
  still the copying `referenced = 0` arm.

Predicted effect:

- **Callees leaving the uncontracted list:** unchanged from revision 1. 55
  callees and 197 pairs leave: 2,790 -> 2,735 callees, 7,264 -> 7,067
  pairs. Exception (README §4): `cfl_sds_printf`'s argument-carrying calls
  may still be counted under its name.
- **Functions leaving the "only uncontracted" set: nine**, out of 508.
  - The eight of revision 1: the seven corpus rows and `pack_uint64`.
  - `resolve_data_point_start_timestamp`, whose only leaf is the
    uncontracted `kvlist_fetch_uint64`.
- **Function moves:**
  - Unknown -> Safe, sixteen rows: the fifteen corpus rows above plus
    `pack_uint64`.
  - Unknown -> provisional Unsafe, one row: `decode_statsd_message`.
- **Unit status changes, two:**
  - `lib/ctraces/src/ctr_attributes.c` Unknown -> Safe;
  - `lib/cmetrics/src/cmt_decode_statsd.c` Unknown -> provisional Unsafe.
  - The units go from 8 Safe / 7 provisional Unsafe / 167 Unknown /
    4 data-only to 9 / 8 / 165 / 4.

## Revision 1 (committed `19c39ce56`, superseded)

The block below is revision 1's `measure-prediction` block, kept as text.

```text
route corpus
move cmetrics::cmt_encode_cloudwatch_emf.c::pack_summary_metric Unknown -> Safe
move ctraces::ctr_attributes.c::ctr_attributes_create Unknown -> Safe
move ctraces::ctr_decode_opentelemetry.c::convert_bool_value Unknown -> Safe
move ctraces::ctr_decode_opentelemetry.c::convert_bytes_value Unknown -> Safe
move ctraces::ctr_decode_opentelemetry.c::convert_double_value Unknown -> Safe
move ctraces::ctr_decode_opentelemetry.c::convert_int_value Unknown -> Safe
move ctraces::ctr_decode_opentelemetry.c::convert_string_value Unknown -> Safe
delta Safe +7
route system
none
route slice
none
```

Revision 1's prose, verbatim:

> **What will change.** New resource-v1 declarations, each read from the
> callee's body (`lib/cfl`, and the mpack copies under `lib/cmetrics/lib/mpack`
> and `lib/ctraces/lib/mpack`, byte-identical at all three revisions), appended
> to the two already-selected modules `cfl/sds_lifetime.qv`
> (`quirin-provider-cfl`) and `mpack/writer.qv` (`quirin-provider-mpack`), and
> pinned. The planned set is [census/declared.txt](census/declared.txt): 30 CFL
> and 25 mpack names. No analyser, gateway or Lean code changes.
>
> **Affected rows at the base.** [census/routes.py](census/routes.py) lists
> every row with a `call to uncontracted function` leaf naming a `cfl_` or
> `mpack_` callee:
>
> - **corpus**: 174 rows. Eight carry no other leaf. Seven of those have
>   every such callee in the planned set
>   ([census/cascade.py](census/cascade.py), round 1; no later round frees
>   anything). The eighth, `create_encoder_arena`, waits on the arena pair,
>   which W47-ARENA-POOL-RESULT-MODE owns. It stays Unknown.
> - **system**: 4 rows, all naming only `cfl_list_add`/`cfl_list_del`, which
>   are out of scope. No row moves.
> - **slice**: 4 rows, naming only `cfl_atomic_load`/`cfl_atomic_store`. These
>   are not in the user's run or this card's scope. No row moves.
>
> The seven corpus rows are predicted Unknown -> Safe:
>
> - `pack_summary_metric` (`mpack_start_map` beside the declared writer
>   primitives);
> - `ctr_attributes_create` (`cfl_kvlist_create`: an owned result stored into
>   a member of the fresh `attr` and returned, or released with `attr` on the
>   failure arm);
> - the five `convert_*_value` helpers of `ctr_decode_opentelemetry.c`. Their
>   `cfl_array_append_*` and `cfl_kvlist_insert_bytes` calls copy. The bytes
>   calls pass `CFL_FALSE`, which is the declared literal `0` after
>   preprocessing.
>
> The other 166 corpus rows only trade leaves. The prediction names no
> Safe -> non-Safe move. It names no Unknown -> Unsafe move either, though a
> newly declared owned result can expose a leak candidate that the uncontracted
> call masked. Any such row is a measurement finding to review, not a predicted
> effect.
>
> **The user's `current` run**: 55 callees and 197 pairs leave the
> uncontracted list (2,790 -> 2,735 callees, 7,264 -> 7,067 pairs); eight
> functions leave the 508 only-uncontracted set (the seven above plus
> `pack_uint64`), each predicted Safe; no unit changes status (8 Safe /
> 7 provisional Unsafe / 167 Unknown / 4 data-only).

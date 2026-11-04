# CFL-MPACK-EXTERNAL-DECLARATIONS prediction

Committed before any contract text changes. Base: `baseline:c8c7edf21`, from
the main checkout's `private/baselines/c8c7edf2165a2065c234f3a63b4eed23e937ced6/`:
`eight-home` (corpus, Fluent Bit `66910c10`), `fluent-bit-system` (system,
`046accf6`) and `fluent-bit-tail-pause` (slice, `34155955`).

**What will change.** New resource-v1 declarations, each read from the
callee's body (`lib/cfl`, and the mpack copies under `lib/cmetrics/lib/mpack`
and `lib/ctraces/lib/mpack`, byte-identical at all three revisions), appended
to the two already-selected modules `cfl/sds_lifetime.qv`
(`quirin-provider-cfl`) and `mpack/writer.qv` (`quirin-provider-mpack`), and
pinned. The planned set is [census/declared.txt](census/declared.txt): 30 CFL
and 25 mpack names. No analyser, gateway or Lean code changes.

**Affected rows at the base.** [census/routes.py](census/routes.py) lists
every row with a `call to uncontracted function` leaf naming a `cfl_` or
`mpack_` callee:

- **corpus**: 174 rows. Eight carry no other leaf. Seven of those have
  every such callee in the planned set
  ([census/cascade.py](census/cascade.py), round 1; no later round frees
  anything). The eighth, `create_encoder_arena`, waits on the arena pair,
  which W47-ARENA-POOL-RESULT-MODE owns. It stays Unknown.
- **system**: 4 rows, all naming only `cfl_list_add`/`cfl_list_del`, which
  are out of scope. No row moves.
- **slice**: 4 rows, naming only `cfl_atomic_load`/`cfl_atomic_store`. These
  are not in the user's run or this card's scope. No row moves.

The seven corpus rows are predicted Unknown -> Safe:

- `pack_summary_metric` (`mpack_start_map` beside the declared writer
  primitives);
- `ctr_attributes_create` (`cfl_kvlist_create`: an owned result stored into
  a member of the fresh `attr` and returned, or released with `attr` on the
  failure arm);
- the five `convert_*_value` helpers of `ctr_decode_opentelemetry.c`. Their
  `cfl_array_append_*` and `cfl_kvlist_insert_bytes` calls copy. The bytes
  calls pass `CFL_FALSE`, which is the declared literal `0` after
  preprocessing.

The other 166 corpus rows only trade leaves. The prediction names no
Safe -> non-Safe move. It names no Unknown -> Unsafe move either, though a
newly declared owned result can expose a leak candidate that the uncontracted
call masked. Any such row is a measurement finding to review, not a predicted
effect.

```measure-prediction
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

**The user's `current` run** (Fluent Bit `046accf6`, profile
`contract/profiles/fluent-bit`), rerun by the integrator after landing
([census/current.py](census/current.py)):

- **Callees leaving the uncontracted list.** The run has 78 `cfl_`/`mpack_`
  callees in 306 pairs. All 55 planned names appear in it, in 197 pairs, and
  all 55 leave the list. The withheld forms are the dispositions, the slot
  move and the borrowed result. Their call sites read the route's own
  diagnostics instead (`relational-untested`, `slot-move-undecided`,
  `interior-borrow-ended`) wherever the caller does not decide them. The run
  goes from 2,790 to 2,735 uncontracted callees and from 7,264 to 7,067 pairs.
- **Functions leaving the "only uncontracted" set.** Eight functions leave
  the 508: the seven above plus `pack_uint64`
  (`lib/ctraces/src/ctr_encode_msgpack.c`, `mpack_write_u64`). Each is
  predicted Safe.
- **Unit status changes.** None. Every unit with one of these rows has other
  Unknown functions. The units stay 8 Safe / 7 provisional Unsafe /
  167 Unknown / 4 data-only.

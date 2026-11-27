# K2-NO-TYPED-ACCESS: prediction

Frozen before the rule-79 implementation commit and before any measurement of
an implementation build (USER DECISION 2026-11-25, verbatim: "Build the 2
rows"; option 2 of [README](README.md) §3). The integrator measures with base
`rev:5087f9295` (or its landed successor). The forecast reads the re-census of
the user's 2026-11-24 `current` run ([README](README.md) §1–§2), the base
probes ([probes/](probes/)), and the corpus tree at Fluent Bit `66910c10`
(read-only), whose `lib/librdkafka-2.15.0/src/lz4.c:783` and
`rdkafka_share_acknowledgement.c:1120-1121` are the same text as `046accf6`'s.

## Mechanism and what it can move

**Rule 79 (1)** admits a controlling expression that every older route
refused, whose Clang elaboration holds a value region (`&&`, `||`, `?:`), at
least one `sizeof`/`_Alignof` of a type name that is not variably modified,
and no event but `Branch`/`Merge` and reads of unaliased automatic objects by
name: it lowers to no instruction and a branch refining neither edge. Before
the rule such a condition was always refused (the condition lowering lowers
no `Branch`), so the rule only adds admissions.

**Rule 79 (2)** drops a run `NAME * sizeof` from the declaration candidates
of the total-or-reject declaration reader, which read it as a declaration,
failed, and left the whole function's declared types unread. A function whose
body holds such a run now has its declared types read; every other function
reads exactly as before.

In the prepared trees the source spelling `( NAME * sizeof (` occurs in five
functions of the user's `current` run (scan:
[scripts/product_sites.py](scripts/product_sites.py),
[census/product-sites.json](census/product-sites.json)) and in six more files
of the corpus tree (`skywalking.c`, `cfl_array.c`, `metrics.c`,
`flb_aws_credentials_process.c` ×2, `rdkafka.c`); a macro expansion can add
more. The only connective condition with a type-name `sizeof` found by a
source scan is `lz4.c:783`; `lz4.c:1927`
(`(sizeof(length)<8) && unlikely(…)`) has an expression operand and a call and
stays refused.

## Predicted moves

| Route | Function | Today | Predicted |
|---|---|---|---|
| `current` (user run) | `LZ4_hashPosition` (librdkafka `lz4.c`) | Unknown | Safe |
| `current` (user run) | `rd_kafka_share_partition_offsets_list_new` (librdkafka `rdkafka_share_acknowledgement.c`) | Unknown | Safe |
| corpus (eight-home) | the same two | Unknown | Safe (premise: the corpus rows carry the same sole leaf; the second is listed Unknown with it in `contract/contributions/37/eight-home.json`) |
| twelve-home | the same two | Unknown | Safe (the corpus inputs) |
| system, slice | — | — | no move (neither selection holds `lz4.c`, `rdkafka_share_acknowledgement.c` or a product run) |

No `Safe -> non-Safe` move and no new `Unsafe` on any route. Rule 79 (1) adds
admissions only and owes nothing on an admitted condition. Rule 79 (2) gives
the types-dependent routes of the functions above their declared types: a
route that admits more can expose a `leak` or `use_after_release` candidate
that the unread types hid. None is forecast (the five `current` functions'
other leaves are pointer stores, uncontracted callees and pointer-arithmetic
stores, read below), but the measurement should look for one there.

## Predicted leaf changes (`current`)

* `LZ4_hashPosition`: its one branch-condition leaf (`lz4.c:783`) removed.
* `LZ4_getPosition`, `LZ4_putPosition`: each loses
  ``call to uncontracted function `LZ4_hashPosition` `` (the same-unit summary
  is now Safe); each keeps its other callee's leaf (`LZ4_getPositionOnHash`,
  `LZ4_putPositionOnHash`) and stays Unknown. `LZ4_compress_generic_validated`
  may lose a leaf of the same callee; it stays Unknown (70 leaves).
* `rd_kafka_share_partition_offsets_list_new`: its one gate leaf (`:1120`)
  removed.
* `rd_kafka_share_partition_offsets_new`: its `:1082` gate leaf removed (the
  same statement shape); its `:1085` leaf (a pointer-valued member store)
  stays; Unknown.
* `rd_kafka_share_build_partition_offsets_list`: loses
  ``call to uncontracted function `rd_kafka_share_partition_offsets_list_new` ``;
  stays Unknown (its other leaves).
* `rd_kafka_DescribeGroups_resp_cb` (108 leaves) and Monkey `mk_iov_create`
  (pointer-arithmetic stores, uncontracted `mk_iov_init`): declared types now
  read; leaf exchanges possible, no row move.
* Zero change of `leak`, `double_release`, `summary_conflict` counts expected.

```measure-prediction
route corpus
move LZ4_hashPosition Unknown -> Safe
move rd_kafka_share_partition_offsets_list_new Unknown -> Safe
delta Safe +2
route twelve-home
move LZ4_hashPosition Unknown -> Safe
move rd_kafka_share_partition_offsets_list_new Unknown -> Safe
delta Safe +2
route system
none
route slice
none
```

## Expected effect on `current`

Two rows Unknown -> Safe, 1,173 -> 1,175 Safe (the K2 bundle's draft forecast
of 15, band 10-20, is not reached: [README](README.md) §1-§3). The builtins
(`LZ4_NbCommonBytes`, `log2_floor`) stay out (`K2-BUILTIN-BIT-COUNTS`, open
user choices).

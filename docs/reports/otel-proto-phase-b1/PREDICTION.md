# OTEL-PROTO-PHASE-B1: prediction

Frozen before the crate's contracts are written (USER DECISIONS 2026-11-27,
verbatim: "New provider crate", then "Phase it, accept 7"; option 1, phase B1
of [the audit's hand-back](../otel-proto-audit/README.md) §6). Base: the
Phase A tip `7b918859c` (on origin/main `9903b027a`). The forecast reads the
r40 landing measurement (`private/measure/r40-landing/sides/tip/`, quirin-audit
built from `c0cb77da3`) and the corpus tree at Fluent Bit `66910c10`, whose
`lib/fluent-otel-proto/` is byte-identical to `046accf6`'s.

## Mechanism and what it can move

A new shipped provider crate `quirin-provider-fluent-otel-proto` declares
custody contracts for the 78 generated fluent-otel-proto functions the OTel
consumer units call (52 `__init`, 9 `__pack`, 9 `__get_packed_size`, 4
`__unpack`, 4 `__free_unpacked`) and the 4 protobuf-c runtime entry points they
reach (`protobuf_c_message_unpack`, `_free_unpacked`, `_pack`,
`_get_packed_size`):

* `__unpack`: the allocator, length and data are borrowed; the result is an
  owned, nullable allocation (NULL on any decode or allocation failure);
* `__free_unpacked`: a nullable release of the message (NULL is a no-op),
  the allocator borrowed;
* `__init`, `__pack`, `__get_packed_size`: every parameter borrowed, no
  custody returned (each body copies a static initializer into, or reads,
  memory the caller holds).

Before the crate every call of these functions is the leaf ``call to
uncontracted function `…` ``. A declaration removes exactly that leaf; every
other leaf of the calling function stays. A row moves to Safe only when the
removed leaves were its only obstacle on both properties. The OTP premises
(`docs/reports/otel-proto-audit/formulas.json`) stay unread by any verdict.

On corpus, 37 custody-Unknown rows carry a generated callee among their leaves
and 7 carry nothing else; their null property is already Safe. system and slice
hold no unit that calls a generated function.

## Predicted moves

| Route | Function | Today (custody / null / combined) | Predicted |
|---|---|---|---|
| corpus | `cmetrics::cmt_encode_opentelemetry_batch.c::metrics_batch_view_init` | Unknown / Safe / Unknown | Safe |
| corpus | `ctraces::ctr_encode_opentelemetry.c::initialize_export_service_request` | Unknown / Safe / Unknown | Safe |
| corpus | `ctraces::ctr_encode_opentelemetry.c::initialize_instrumentation_scope` | Unknown / Safe / Unknown | Safe |
| corpus | `ctraces::ctr_encode_opentelemetry.c::initialize_resource` | Unknown / Safe / Unknown | Safe |
| corpus | `ctraces::ctr_encode_opentelemetry.c::initialize_resource_span` | Unknown / Safe / Unknown | Safe |
| corpus | `ctraces::ctr_encode_opentelemetry.c::initialize_scope_span` | Unknown / Safe / Unknown | Safe |
| corpus | `ctraces::ctr_encode_opentelemetry.c::initialize_span` | Unknown / Safe / Unknown | Safe |
| twelve-home | the same seven | the same | Safe |
| system, slice | — | — | no move |

The seven are the upper set the user accepted: each calls exactly one
`__init` on memory it holds (a checked `calloc`/`malloc` result it returns, or
`&batch->request` of its formal). The `cmetrics` functions of the same names
(`cmt_encode_opentelemetry.c::initialize_resource`,
`::initialize_instrumentation_scope`) keep other leaves and stay Unknown.

No other row moves: the other 30 rows that call a generated function keep
other leaves. No `Safe -> non-Safe` move and no new `Unsafe`: the three corpus
functions that unpack (`cmt_decode_opentelemetry_create`,
`cmt_encode_opentelemetry_split_payload`, `ctr_decode_opentelemetry_create`)
release the result with the matching `__free_unpacked` on every path the
bodies show, so the now-owned result adds no leak; an unpack result is
dereferenced only under its NULL test (Phase A, O0: 7 sites, all guarded).
Any move outside this table is a stop-and-hand-back (user condition).

```measure-prediction
route corpus
move cmetrics::cmt_encode_opentelemetry_batch.c::metrics_batch_view_init Unknown -> Safe
move ctraces::ctr_encode_opentelemetry.c::initialize_export_service_request Unknown -> Safe
move ctraces::ctr_encode_opentelemetry.c::initialize_instrumentation_scope Unknown -> Safe
move ctraces::ctr_encode_opentelemetry.c::initialize_resource Unknown -> Safe
move ctraces::ctr_encode_opentelemetry.c::initialize_resource_span Unknown -> Safe
move ctraces::ctr_encode_opentelemetry.c::initialize_scope_span Unknown -> Safe
move ctraces::ctr_encode_opentelemetry.c::initialize_span Unknown -> Safe
delta Safe +7
route twelve-home
move cmetrics::cmt_encode_opentelemetry_batch.c::metrics_batch_view_init Unknown -> Safe
move ctraces::ctr_encode_opentelemetry.c::initialize_export_service_request Unknown -> Safe
move ctraces::ctr_encode_opentelemetry.c::initialize_instrumentation_scope Unknown -> Safe
move ctraces::ctr_encode_opentelemetry.c::initialize_resource Unknown -> Safe
move ctraces::ctr_encode_opentelemetry.c::initialize_resource_span Unknown -> Safe
move ctraces::ctr_encode_opentelemetry.c::initialize_scope_span Unknown -> Safe
move ctraces::ctr_encode_opentelemetry.c::initialize_span Unknown -> Safe
delta Safe +7
route system
none
route slice
none
```

## Predicted leaf changes (corpus, twelve-home)

* Every ``call to uncontracted function `opentelemetry__proto__…` `` leaf
  disappears (the r40 corpus run has 34 distinct generated callees among
  them); no `protobuf_c_*` leaf exists today.
* The 30 other rows that called a generated function keep their other leaves
  (unresolved aliases, indirect member access, other uncontracted callees) and
  stay Unknown; the unpack results become tracked owned values, so a later
  read through them may be reported as a different leaf, never as a release.
* Zero change of `leak`, `double_release` and `summary_conflict` counts.

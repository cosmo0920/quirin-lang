# CG13-UNION-AND-BITFIELD-PLACES: frozen prediction

Frozen before any implementation, on base `f19204fa` (= local `main` =
`origin/main`), after the BEFORE corpus arbiter finished and before any
source change. BEFORE (`private/cg13/before`, exit 0) reproduces CG02's
archived AFTER byte for byte: `resource-diagnostics.json` `963d7c9f…`,
543 Safe / 10 provisional Unsafe / 4,281 Unknown.

## The inherited list, re-derived

CG00's list is [`../cg00-clang-gap-baseline/predictions/CG13.md`](../cg00-clang-gap-baseline/predictions/CG13.md)
(sha256 `a75f7d44e997781b2d504ef338d9e7f7c4a75c9850537a88737b896290b2551e`),
measured on `ae17152b…`. [`scripts/population.py`](scripts/population.py)
re-derives it on this base with CG00's own leaf attribution (a current leaf
with CG00's unit, function, internal line and normalised description keeps its
attribution; 42 current leaves have no counterpart):

| Measure | CG00 (`ae17152b`) | Re-derived (`963d7c9f`) |
|---|---:|---:|
| M06 leaves / Unknown functions | 1,629 / 467 | **1,629 / 467** (none moved by CG10 or CG02) |
| M07 leaves | 0 | **0** |
| freed alone, alias allowed / strict | 55 / 48 | **61 / 48** |
| freed with one other card | 48 | 98 (callee closure 25, CG10 25, uncontracted external 13, CG07 9, CG14 6, …) |

All 55 of CG00's functions are still in the list; none has moved. Six are new
(their other blocker was removed by CG10 or CG02): `rd_kafka_share_ack_commit_cb_register_op`
and the five telemetry `calculate_*_avg` functions.

## Amendments, with reasons

CG00's figures are upper bounds and its attribution labels a leaf by the
**first** path cause in a fixed order in which `union` precedes `call`,
`cast`, `address-of` and the pointer-valued store. Reading the 61 functions'
leaves ([`scripts/leaves_of.py`](scripts/leaves_of.py)) against the card's
scope:

1. **Stays refused by design.** A pointer-valued store through a union member
   (`…internal_donotuse.dictCtx = dictCtx`, `op->rko_u.admin_result.result_cb
   = …`, `*fac = rkev->rko_u.log.fac`): card CG11, and the brief's
   custody-pointer refusal. A release of a pointer read through a union
   member (`free(value->string_value)`, `rd_free(rko->rko_u.admin_result.errstr)`,
   `rd_list_destroy(options->….u.PTR)`): a pointer read through a union is a
   target-free borrowed alias, so a custody-taking position keeps its refusal
   (rule 10k (3)). A cast inside the path (`((LZ4_unalign*)memPtr)->u16`), an
   indirect call and an address of a deeper place (`&rko->rko_u.admin_result.results`)
   keep their refusals (CG22, the cast and address rules).
2. **Admitted by this card.** A scalar read, a pointer-value read (P2's
   target-free borrowed alias) and a scalar store through a union member step,
   on a never-bound root (W46's premise) or a tracked root (rule 10t's
   decision, the union step added additively). A store through a member of a
   union that can hold a pointer carries W43's refused-write frame (every
   binding a store could rewrite is poisoned); a store through a
   custody-free union is frame-neutral. A pointer-value read through a union
   member is refused when the function stores through another member of that
   union type (type punning, M22).
3. **Bit-fields.** A bit-field read or store is a custody-neutral scalar
   access to its storage unit (C11 memory location); a store is a
   read-modify-write of the unit. M07 has no corpus leaf, so bit-fields move
   nothing in the corpus.

## Predicted outcome

| Measure | Prediction |
|---|---|
| BEFORE reproduces `963d7c9f…` byte-identically | **held** (measured before freezing) |
| Unknown → Safe, corpus | **16** (band 6–40); upper bound 48 strict / 61 with alias |
| of those, in the re-derived list of 61 | ≥ 60% |
| expected among them | `metric_data_point_count`, `convert_any_value`, the five `convert_*_value`, `pack_variant`, `LZ4_resetStream_fast`, `LZ4_favorDecompressionSpeed`, `LZ4_setCompressionLevel`, `rd_kafka_admin_timeout_remains(_us)`, `rd_kafka_event_opaque`, `rd_kafka_event_stats`, `rd_kafka_event_topic_partition_list`, `rd_kafka_message_latency`; the telemetry `calculate_*` stores of a call result are uncertain (a call on the right side may keep its own refusal) |
| stay Unknown among the 61 | the destroy functions (release of a union pointer), the pointer-valued stores, the casts, the indirect calls, `&…admin_result.results` actuals, every function with an alias leaf that stays |
| Safe → non-Safe, corpus | **0**; any is a named correction |
| provisional Unsafe added | 0 (band 0–2), each classified by reading |
| M06 leaves removed | 400–900 of 1,629 |
| `unknown_alias` leaves added | 20–300 (a union place now admitted whose root fails the flow reading) |
| system slice (`046accf6`) | 0–10 functions move (msgpack `via.*` unions), none Safe → non-Safe |
| tail-pause slice (`90978ac6`) | 0–3 functions move |

Confidence in the point estimate is low: CG00's first-cause attribution hides
the second cause of a union leaf, and the telemetry functions (20 of the 61)
decide much of the band.

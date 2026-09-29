# CG02-ELABORATION-CORE: frozen prediction

Frozen before any route hook is committed (the only code at freeze time is the
unconnected `elaboration.rs` core). Base: local `main` at `e182d403`
(= origin/main at dispatch). Card CG00's frozen prediction for this card is
[`../cg00-clang-gap-baseline/predictions/CG02.md`](../cg00-clang-gap-baseline/predictions/CG02.md);
this file re-freezes it, amended only where a reason is given.

## BEFORE

The corpus arbiter at base (`private/cg02/before/`, exit 0) reproduces the
baseline: `resource-diagnostics.json` `ae17152b05516be7…`,
`…chunk-contracts.json` `cbbcfd9c…`, `monkey-source.json` `57acfc58…`;
526 Safe / 10 provisional Unsafe / 4,298 Unknown.

## What the card connects

Route G, as a fallback after every existing route, in three positions:
statements (including one-declarator declarations), `return` expressions, and
call-free controlling expressions. Admitted: compound assignment and
increment/decrement of a scalar local or typed scalar path (statement or value
position), assignment used as a value, and comma sequencing (each operand a
statement the existing classifier decides). Route T gains no admission: no
consumer decides the value forms (CG01's census), so T names them in its
refusal.

## Predicted verdicts for CG00's twelve functions

CG00's upper bound is 12 freed alone (10 strict). Amended to **7 Safe**, upper
bound; the five others are refused by name by this card's own scope limits:

| Function | CG00 | Amended | Reason |
|---|---|---|---|
| `chunkio.c::cio_enable_file_trimming` | Safe | Safe | `ctx->options.flags \|= …`: compound on a mixed `->`/`.` path (never-bound root) |
| `chunkio.c::cio_disable_file_trimming` | Safe | Safe | same, `&= ~…` |
| `lz4hc.c::LZ4HC_literalsPrice` | Safe | Safe | `price += …` on a local, scalar cast in the value |
| `rdhdrhistogram.c::rd_hdr_histogram_record` | Safe | Safe | three `hdr->…++` statements, one through `->`/`[]` |
| `rdkafka_coord.c::rd_kafka_coord_req_keep` | Safe | Safe | `creq->creq_refcnt++` |
| `rdkafka_transport.c::rd_kafka_transport_poll_set` | Safe | Safe | `rktrans->rktrans_pfd[0].events \|= event` |
| `rdkafka_transport.c::rd_kafka_transport_poll_clear` | Safe | Safe | same, `&= ~event` |
| `crc32c.c::gf2_matrix_times` | Safe | **Unknown** | the blocking leaf is `mat++` on a *formal* pointer: pointer arithmetic (CG16), refused by name |
| `crc32c.c::crc32c_sw` | Unknown | Unknown | `*next++` in a value position: pointer arithmetic (CG16) |
| `mk_utils.c::mk_utils_hex2int` | Safe | **Unknown** | `(c = *hex++) && …`: a formal pointer step inside a value `&&` condition (CG16, CG06); `res *= 0x10` is admitted |
| `rdkafka_broker.c::rd_kafka_broker_ops_serve` | Safe | **Unknown** | the comma sits in a `&&` condition whose other leaf assigns an owned call result (CG04, CG06) |
| `rdkafka_offset.c::mk_esc_filename` | Unknown | Unknown | `esclen-- > 0` is admitted; `*(o++) = *(esc++)` is pointer arithmetic (CG16) |

## Whole-corpus prediction

1. **No Safe → non-Safe move.** Every hook fires only on a statement,
   return or condition an existing route refused, so a Safe function (no
   refusal) lowers byte-identically.
2. **At most 7 Unknown → Safe** among the twelve; further Unknown → Safe moves
   are possible only for functions CG00 counted as freed with a partner card
   whose partner leaf this card also happens to lower (predicted 0).
3. **Unknown → provisional Unsafe: 0 predicted.** A newly lowered statement can
   expose a leak later in a body that was blocked before; any such move is
   classified by reading, never presented as an upstream defect.
4. **Unknown → Unknown** functions lose leaves (E03/E05 statement leaves on
   typed places, `X-- > 0` conditions, comma statements).
5. **Sidecars.** `monkey-source.json` and `…chunk-contracts.json`
   byte-identical (no pinned T function reaches a value form).
   `parity.jsonl` gains `elaboration-*` rows.
6. **Absorbed `W43-RETURN-COMMA-MACRO`** (69 leaves / 34 functions registered):
   the comma is split, but nanopb's left operand
   `stream->errmsg = stream->errmsg ? stream->errmsg : "…"` is a value `?:`
   (CG06) *and* a pointer-valued member store (CG11), so its leaves stay refused,
   now by name; re-registered under CG06/CG11 with the re-measured size.
</content>
</invoke>

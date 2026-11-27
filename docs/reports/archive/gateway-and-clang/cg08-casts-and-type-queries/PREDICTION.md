# CG08-CASTS-AND-TYPE-QUERIES: frozen prediction

Frozen before any implementation commit, on base `ca634dc1` (local `main`,
which contains the round-5 base `3e69322c` and W48-QR-C-STRUCT-IMPORT). The
BEFORE arbiter on this base reproduces the baseline byte for byte:
`resource-diagnostics.json` `c012e78506cd…1440`, **586 Safe / 10 provisional
Unsafe / 4,238 Unknown** (`private/cg08/before/`, `--jobs 8`, 5 min 35 s).

CG00's frozen list (`../cg00-clang-gap-baseline/predictions/CG08.md`, measured
on `ae17152b`) is re-derived here on the current base with
`scripts/candidates.py`. It amends CG00's figures as follows, each with its
reason.

## 1. The candidates on this base

Two of CG00's 34 are already Safe, freed by earlier cards:
`rdkafka_mock.c::rd_kafka_mock_cluster_op_io` and
`rdkafka_request.c::rd_kafkap_leader_discovery_tmpabuf_add_alloc_topics`.
The other 32 are Unknown. Reading each blocking line (not CG00's leaf
classes) gives this card's hooks:

| Planned hook | Candidates (blocking line shape) | Predicted |
|---|---|---|
| H4: an explicit pointer-to-pointer cast (`BitCast`/`NoOp`) at the top of a pointer-destination statement is decided as the uncast statement | `unpack_metric_start_ts`, `unpack_metric_ts`, `get_context_arena`, `LZ4_putIndexOnHash`, `LZ4F_writeLE32`, `LZ4F_writeLE64`, `safe_read_bool`, `rd_kafka_assignor_cmp_idx`, `rd_kafka_topic_partition_partition`, `rd_kafka_share_ack_entries_offset_find_cmp_ptr`, `rd_kafka_share_ack_entries_sort_cmp_ptr`, `input_chunk_has_magic_bytes` (`T *p = (T *)q;`, `p = (T *)q;`) | Safe, upper bound 12 |
| H4, alias leaf stays | `unpack_meta_aggregation_type`, `unpack_metric_value_uint64` | Unknown (`unknown_alias`) |
| H5: the elaborator reads unevaluated type-query operands and the listed cast representations | `LZ4HC_rotatePattern` (`size_t const b = (r & (sizeof(pattern) - 1)) << 3;`) | Safe, upper bound 1 |
| H5, alias leaf stays | `rd_kafka_anyconf_is_any_modified` (call-free `for` condition over `sizeof` of a member) | Unknown |
| H6: a Clang-confirmed unevaluated `sizeof` operand leaves the lexical gate of an ordinary call statement | `LZ4HC_clearTables` (`memset(hc4->hashTable, 0, sizeof(hc4->hashTable))`; the array-member actual may keep a refusal) | Safe, upper bound 1 |
| none of this card's (second cause) | `LZ4HC_reverseCountPattern` (cast then pointer arithmetic, CG16), `finalize_checksum` (`cf->map + 2`, CG16), `ut_write_msgset_v2_header` (store through a cast of `buf + offset`, CG16), `LZ4F_returnErrorCode` (a braced `enum` static assertion), `rd_kafka_header_copy` (a cast-call return beside the alias), `mk_server_lib_notify_started`, `rd_kafka_pattern_list_copy`, `varint_encode32` | Unknown |
| alias leaves stay | `mk_config_signature`, `rd_kafka_mock_partition_offset_for_leader_epoch`, the three `PushTelemetry_decoded_*`, `rd_kafka_messages_new`, `rd_kafka_topic_partition_list_sort` | Unknown |

**Predicted Unknown → Safe among CG00's candidates: at most 14** (CG00: 22
strict, 34 with alias leaves). Functions outside CG00's list can move too:
H4, H5 and H6 apply wherever their shape occurs, so the corpus figure is
measured, not predicted, and every mover is attributed.

## 2. A soundness repair found while reproducing E15 (predicted corrections)

Reproducing the `PointerToBoolean` part of E15 at the route-G level found a
wrong Safe on base:

- `bool f(void) { void *p = q_alloc(); return p; }` is **Safe**: the `return`
  route escapes `p` although C converts it to `bool` (a null test), so the
  caller receives no pointer and the allocation leaks. `return (_Bool)p;`,
  `return p != 0;` and `return !p;` are correctly Unsafe (leak).
- `bool b = p; return b;` is Safe for the same reason (the statement binds
  `b` to `p`'s identity), and `bool b = p; q_free(p); return b;` is a false
  custody conflict.

H1/H3 repair this: a `return`, declaration or assignment whose value Clang
converts with `PointerToBoolean` is never an escape or a binding; it is the
elaborator's null-test value (a read of the pointer, no custody effect) or a
refusal. **Predicted: every function whose Safe rested on such an escape or
binding moves to Unsafe (the leak) or Unknown; each is a named correction.**
Size unknown before the AFTER run.

## 3. Cast-return discharge (`CAST-RETURN-UNCOMMITTED-OPERAND`)

H2: a `return (INTEGER-TYPE) h;` the lexical reader does not commit to
(`void *h`, a pointer typedef) is refused when Clang's cast kind is
`PointerToIntegral` and `h` may be bound. Predicted: provisional Unsafe
(leak) or Safe → Unknown only where such a return exists on a tracked
pointer; CG00 recorded no corpus leaf for it (`W44-LEAK-PIN-AUDIT`: 0).

## 4. Out of scope, predicted unchanged

Numeric transfer (CG20), pointer arithmetic and comparison (CG16), pointer
stores (CG11), `_Generic` (CG07), atomic accesses (CG19). The nanopb comma
returns (`CG02-COMMA-OPERAND-RESIDUALS`, 67 over 32 functions) are predicted
to stay Unknown: past the `IntegralToBoolean` this card admits, they refuse at
rule 10z's position check and then at CG11's pointer member store.

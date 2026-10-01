# CG18-FOOTPRINT-CONNECTION — frozen prediction

Frozen on base `b59aaae2` (main, with CG07), after the BEFORE on all three
routes and before any release binary with the card's code was built or
measured. The gateway hook was drafted in the worktree when this was written;
no AFTER existed.

## 1. CG00's prediction, re-frozen

CG00-CLANG-GAP-BASELINE wrote no `predictions/CG18.md`: its attribution gave
CG18's entries (M20, M21, M22, M26, W08) **0 leaves and 0 functions** in the
corpus arbiter, because the gateway publishes those limits as `Safe` or
`unknown_alias`, never as unsupported leaves (its README §3.3). That
prediction — no function freed — is re-frozen here as the card's own for
those five entries, amended by one item with its reason:

- **Amendment.** CG13-UNION-AND-BITFIELD-PLACES amended this card's ledger
  entry: an unordered pair of two field places the core model proves
  disjoint stays refused, "admitting it, and any other field pair, is this
  card's". Those refusals are the elaborator's `not proved disjoint (CG10)`,
  which CG00 attributed to CG10, not to CG18. The card admits exactly the
  pairs of two members of one struct object that the core model decides
  disjoint from Clang's layout (rule 11k (1)); every cross-object pair stays
  refused (M21).

## 2. BEFORE (base `b59aaae2`, binary `1797b7af…`)

| Route | Digest | Safe / Unsafe / Unknown (/ Pending) |
|---|---|---|
| Corpus arbiter (`66910c10`) | `8b9e4047…` (= the integrator's main figure) | 608 / 10 / 4,216 |
| System | `819a2841…` | 41 / 2 / 191 |
| Tail-pause slice | `d663914d…` | 28 / 1 / 240 / 1 |

The elaborator's path-pair refusals in the BEFORE parity streams: corpus
**30 consultations in 8 (function, pair) rows, 7 functions**; system 0;
slice 0.

| Function | Pair | Same base? | Predicted |
|---|---|---|---|
| `mk_http_parser` | `p->header_max` / `p->header_min` | yes | admitted if both are members of one struct |
| `mk_http_parser` | `p->header_val` / `p->start` | yes | admitted if both are members of one struct |
| `rd_kafka_mock_partition_pidstate_get` | `pidstate->window` / `pidstate->hi` | yes | admitted if both are plain members |
| `decode_static_field` | `size*` / `field->array_size` | no | refused |
| `rd_kafka_send` | `rkb->rkb_corrid` / `rkbuf->rkbuf_corrid` | no | refused (M21) |
| `rd_kafka_toppar_enq_msg` | `rktp->…` / `rkm->…` | no | refused (M21) |
| `rd_kafka_mock_sharegroup_target_assignment_set` | `mshgrp->…` / `member->…` | no | refused (M21) |
| `mk_sched_register_thread` | `server->…` / `worker->…` | no | refused (M21) |

## 3. Prediction

| Measure | Point | Band |
|---|---|---|
| Corpus Unknown → Safe | 0 | 0–2 (the two same-base functions carry other leaves unless measured otherwise) |
| Corpus Safe → non-Safe | 0 | 0 (the hook only admits; no admission removes a refusal an older route made) |
| Provisional Unsafe added | 0 | 0–1 (an admitted statement can expose a later leaf) |
| Corpus functions whose leaves change | 2 | 0–3 (`mk_http_parser`, `rd_kafka_mock_partition_pidstate_get`) |
| Path-pair consultations admitted (parity `footprint-member-pair`) | 3 pairs | 0–3 |
| System, tail-pause slice | byte-identical | no candidate pair on either |
| Core model, frontend composition | no corpus reach | the core events and `footprint_mutex` have no route in the arbiter |

A new premise in a published corpus verdict: **none** predicted. The member
pairs rest on `DeclaredRecordLayout` (Clang's layout, trust boundary 28,
already stated by rule 10w); `P-DISTINCT-ALLOCATIONS-DISJOINT`,
`P-EFFECTIVE-TYPE` and `P-BYTE-REPRESENTATION` appear only on core
certificates, which no arbiter route publishes.

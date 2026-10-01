# CG21-CALL-EXITS-AND-BOUNDARIES: frozen prediction

Frozen before any AFTER measurement. Honest ordering note: the code was
drafted while the BEFORE corpus run (70 minutes on a host at load 60–80) was
in flight, so this file was written after the implementation and before its
first AFTER run; nothing below was read off an AFTER artifact.

## BEFORE (base `b6a3dc42`, private/cg21/before*)

- Corpus arbiter (`66910c10`): **629 / 10 / 4,195**,
  `resource-diagnostics.json` sha256 `3d53a5d0…` — the brief's baseline,
  reproduced byte for byte.
- `scripts/census.py` over it (artifacts/census-before.json):

| Class | Leaves | Functions | Unknown, blocked by it alone |
|---|---:|---:|---:|
| `__assert_fail` (uncontracted) | 1,232 | 533 | 11 |
| `exit` (uncontracted) | 22 | 13 | 1 |
| `abort` (uncontracted) | 11 | 10 | 1 |
| the three together | 1,265 | — | **13** |
| `__asm__` as a callee (the token reader's misnaming) | 6 | 2 | 0 |
| `__builtin_offsetof` as a callee (an extension node) | 39 | 23 | 0 |
| `__builtin_va_arg` as a callee (an extension node) | 17 | 12 | 0 |
| setjmp/longjmp family, `pthread_exit`, `_exit`, `__builtin_unreachable`, `__builtin_trap` | 0 | 0 | 0 |

  The 13: `mk_header_set_http_status`, `mk_socket_reset` (exit/abort);
  `isFullyBalanced0`, `rd_kafka_cgrp_clear_wait_resp`,
  `rd_kafka_cgrp_set_wait_resp`, `rd_kafka_confval_get_int`,
  `rd_list_set_cnt`, `get_hash_table`, `get_u32_at_offset`, and
  tinycthread_extra.c's four `rwlock_*` (`__assert_fail`).
- Cited claims: W43-LIBC-NORETURN's "exit blocks 13 functions and abort 10"
  and "together 33 leaves" reproduce (13 / 10; 22 + 11). CG07's 1,232 leaves
  on 533 functions reproduce; its "10 Unknown functions blocked by it alone"
  is now **11** (`snappy.c::get_hash_table` joined at a later base).
  cleanup attributes, computed `goto`, SEH and `asm goto` have no leaf of
  their own at BEFORE (they read as IND, a parse failure or a misnamed call),
  so their corpus counts come from the AFTER parity rows.

## Prediction (corpus)

1. **Leaves.** Every `__assert_fail` / `abort` / `exit` leaf in a statement
   position disappears; glibc's assert lowers (rule 11g) to a statement
   `__assert_fail (…);`, so ≥ 1,200 of the 1,232 go. Any that remain are in an
   operand position (a strict-ANSI `?:` assert) or unresolved, and then carry
   the continuation (no leaf) or the new `no-return contract … not
   established` leaf.
2. **Moves.** Unknown → Safe for the 13 above (upper bound for the direct
   move), each attributed to the lifted leaf; a cascade through summaries of
   newly Safe helpers (the tinycthread rwlock wrappers, `rd_list_set_cnt`) to
   their callers is possible and will be attributed. Unknown → Unsafe only
   where a body's other leaves are gone and a leak on a non-assert path was
   already reported (no hidden leak is unhidden: the refused call wrote no
   tracked name). **No Safe → non-Safe move**: the cleanup / asm / SEH /
   label-address boundaries are asked first, and a corpus body admitted
   through one of them would be a named correction — predicted none.
3. **Renames, no move.** `__builtin_offsetof` (39) and `__builtin_va_arg`
   (17) leaves change wording to the extension-node refusal; `__asm__` (6)
   to the inline-assembly boundary.
4. Resulting totals: Safe 629 + (13 … 13 + cascade), Unsafe 10 (+0 … +2),
   Unknown down by the same.

## Prediction (system route and tail-pause slice)

Added before either route's AFTER run, from their BEFORE runs over an
extraction of the base tree (`scripts/measure-routes.sh`; the `current` route
reads contract modules from the tree it runs in, so the BEFORE binary cannot
read this card's tree). Totals reproduce the brief's: system **44 / 2 / 188**,
slice **29 / 1 / 239 / 1 Pending**. Their digests (`6e65757c…`,
`499081be…`) differ from the brief's `79404fed…` / `93cc660c…` because the
report embeds the profile path (8 occurrences of the extraction's path in
review-hold descriptions); `scripts/identity.py` digests the reports with
that path normalised.

- System: one `exit` leaf (1 function) and 3 `__builtin_offsetof` leaves; no
  function is blocked by the three callees alone. Predicted: the `exit` leaf
  goes, the 3 offsetof leaves are renamed, **no status move**.
- Slice: 2 `__builtin_offsetof` and 4 `__builtin_va_arg` leaves (renamed),
  no exit leaf. Predicted: **no status move**, only the renames.

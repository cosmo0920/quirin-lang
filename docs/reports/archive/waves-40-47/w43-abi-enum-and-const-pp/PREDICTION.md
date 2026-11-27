# W43-ABI-ENUM-AND-CONST-PP: prediction

Frozen before any implementation code or contract. Base: local `main` at
`4f55fa4c4`.

## BEFORE (reproduced)

Binary `private/w43aecp/bin/before`, sha256 `0a83e728…` (equal to `run.json`'s
`binary_sha256`):

| route | Safe / Unsafe / Unknown | Pending | digest |
| --- | --- | --- | --- |
| corpus `66910c10` | **812 / 9 / 4,013** | 0 | `a43d02cb…` (byte for byte the brief's) |
| system | 87 / 4 / 357 | 0 | `63bf8063…` (embeds this worktree's paths) |
| slice | 45 / 1 / 354 | 4 | `1614b88b…` (embeds this worktree's paths) |

Ranking (`scripts/rank.py`): population 625, 449 decided by one callee alone,
as CALLEE-RETENTION-CLASS1 left it. Member sizes
([size-before-corpus.json](artifacts/size-before-corpus.json); functions
decided alone):

* enum shape, 9 callees / **28**: `LZ4_decompress_generic` 9,
  `rd_kafka_error_new_v` 4, `rd_kafka_error_new` 3, and 2 each for
  `rd_kafka_AclBinding_new0`, `rd_kafka_ConsumerGroupDescription_new`,
  `rd_kafka_admin_add_config0`, `rd_kafka_err_action`,
  `rd_kafka_oauthbearer_set_token_failure`, `rd_kafka_toppars_pause_resume`
  (reproduces the ledger exactly);
* const-pp shape, 3 / **7**: `rd_flags2str` 3, `rd_kafka_anyconf_dump` 2,
  `rd_slice_reader0` 2 (reproduces);
* class-1 undeclarables: `rd_kafka_confval_set_type` 5,
  `rd_kafka_interceptor_method_add` 4, `add_item_to_object` 3,
  `cJSON_ParseWithLengthOpts` 2 (reproduces);
* `getrlimit` / `setrlimit`: 0 alone; one function is blocked by exactly the
  pair (1 leaf each).

## A correction found while reading, before implementing

The brief (and `W43-LIBC-ABI-READER-SHAPES`, `CLASS1-UNDECLARABLE-RETAINERS`)
says `rd_kafka_confval_set_type` retains `valuep` "only when `valuetype` is
the PTR constant". Read at `66910c10` (`rdkafka_conf.c:3763-3874`, identical
at `046accf6`), the retaining arm is `case RD_KAFKA_CONFVAL_PTR:` of
`switch (confval->valuetype)`, the **holder's own stored type**, and that arm
never reads the `valuetype` argument. A guard on `valuetype` would therefore
be false of the body (a caller passing the INT constant with a PTR-typed
`confval` still has `valuep` retained). The predicted contract is the
**cautious** one: an unguarded `resource_retained_by(valuep, confval)`. No
corpus declaration will carry a guard unless another member's body is found
to switch its custody on an argument's value.

## Predicted outcome

1. **Reader** (rule 12u, enum integer projection from Clang; rule 12v,
   pointer to pointer to const): no existing declaration changes outcome;
   every currently agreed (unit, name) pair stays agreed, every withdrawn one
   with no enum or const-pp position stays withdrawn.
2. **Contracts** (each read at both revisions, pinned):
   * enum: the nine callees above, plus `getrlimit`/`setrlimit` in
     `quirin-provider-libc`;
   * const-pp: `rd_flags2str` (`desc` borrowed, result an alias of `dst`) and
     `rd_kafka_anyconf_dump` (owned result); `rd_slice_reader0` is **not**
     predicted declarable: it stores an interior pointer of `slice` through
     `p`, the out-slot shape `strtol` was refused for (no v1 mode states it);
   * class 1: `rd_kafka_confval_set_type` (cautious retention) and
     `rd_kafka_interceptor_method_add` (`ic_opaque` and `func` retained by
     `list`).
3. **Corpus**: Unknown → Safe **+12 to +30** (ceiling 28 enum + 5 const-pp
   decided alone + up to 4 freed in pairs; each freed function may still
   carry a leaf the uncontracted call hid). The class-1 callers (9 alone) are
   predicted to stay Unknown under `retention-undecided`: every caller's
   holder is a member reached through a formal. 0 to 3 new provisional
   leaks or violations from the owned results of `rd_kafka_error_new*`,
   `rd_kafka_AclBinding_new0`, `rd_kafka_ConsumerGroupDescription_new` and
   `rd_kafka_anyconf_dump`, each read by hand. **No Safe → non-Safe move.**
4. **System**: 0 to 1 row moves (`getrlimit`/`setrlimit`); **slice**: no row
   moves.
5. **Guards** (decision 3): grammar, loader, call-site decision, cautious
   union and the in-unit body check land with fixtures and Lean; corpus
   effect 0.
6. **Lean**: two new audited modules (the reader widening; the guard
   semantics with the cautious-join theorem).

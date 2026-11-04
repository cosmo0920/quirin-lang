# LIBC-GAP-DECLARATIONS prediction

Committed before any contract changes, on base `c8c7edf21` (origin/main). The
census is in [README.md](README.md), section 1. Affected rows are relisted at
`baseline:c8c7edf21` (`private/baselines/c8c7edf2165a2065c234f3a63b4eed23e937ced6/`
in the main checkout, reports
`eight-home/corpus/corpus`, `fluent-bit-system/system` and
`fluent-bit-tail-pause/slice`).

> **USER REQUEST 2026-11-05** (verbatim): "Now, we're still needed to fill
> the gap of the missing contracts." Then: "Launch 1 and 2".

## What the card will change

The card adds `quirin-provider-libc` declarations, all `revision flexible`,
for 23 names. Each uses only the existing vocabulary:

- **No custody** (`resource_borrowed` on every parameter, `resource_return_none()`):
  `tolower`, `toupper`, `abs`, `sqrt`, `sigdelset`, `pthread_cancel`,
  `fread` (a use of its stream), `lseek`, `ftruncate`, `posix_fallocate`,
  `shutdown`, `poll`, `sendmsg`, `getrandom`, `readlink`, `chmod`, `chown`,
  `msync`.
- **An exact alias**: `strcat` returns `dest` (`resource_return_alias(dest, nonnull)`).
- **A retention** (C04.14, rule 12t): `pthread_setspecific` keeps `pointer`
  in thread-specific storage that no caller object denotes, so it is
  `resource_retained_by(pointer, global)`. A heap or caller pointer there is
  `retention-undecided`, never `Safe`.
- **A result the caller does not own**: `pthread_getspecific`
  (`resource_return_none()`). A release of that result stays refused as an
  unresolved identity.
- **A derived out slot with a retention**: `getpwnam_r` and `getgrnam_r`
  write `resultbuf` or NULL through `result` (`resource_out_derived(result,
  resultbuf, nonnull)`). The record keeps pointers into `buffer`
  (`resource_retained_by(buffer, resultbuf)`), so releasing `buffer` while the
  record lives is refused.

The other 33 names stay undeclared, with the grounds in the README. No gateway,
loader or Lean change is planned.

## Rows (baseline `c8c7edf21`)

**Corpus.** 46 rows name a callee from the list above. Each loses its
`call to uncontracted function` leaf for these callees.

- **Four rows have no other blocker and are predicted Unknown -> Safe:**
  - `rdkafka_transport.c::rd_kafka_transport_shutdown` (`shutdown`);
  - `cio_file_unix.c::cio_file_native_resize` (`posix_fallocate`, `ftruncate`);
  - `cio_file_unix.c::cio_file_native_sync` (`msync`);
  - `mk_server/mk_plugin.c::mk_plugin_sched_get_thread_conf` (returns
    `pthread_getspecific`'s result).
- **Two rows also have no other blocker but stay Unknown:**
  `mk_sched_set_request_list` and `mk_sched_thread_lists_init`. They keep a
  `retention-undecided` leaf, because `pthread_setspecific` retains a caller
  pointer or a list head in `global`.
- **The other 40 rows stay Unknown.** Each has blockers of its own.
  - `pthread_setspecific` callers may gain `retention-undecided`.
  - `getpwnam_r`/`getgrnam_r` callers may gain
    `retention-released-while-held`, because they free the buffer before
    returning.
  - Some uses of `pthread_getspecific`'s result may become
    `unresolved resource identity` leaves.
- **Callers of the four new Safe functions keep their status.**
  `cio_file_resize`, `cio_file_sync` and the mock handlers may lose a leaf
  through a new summary; each has other blockers.

**System.** No row names these callees. Nothing moves.

**Slice.** Five `in_tail` rows name `lseek` or `readlink`:
`compute_offset_marker`, `flb_tail_file_name`,
`flb_tail_file_reset_on_truncate`, `set_file_position` and `tail_fs_check`.
They lose those leaves and stay Unknown.

```measure-prediction
route corpus
move rdkafka_transport.c::rd_kafka_transport_shutdown Unknown -> Safe
move cio_file_unix.c::cio_file_native_resize Unknown -> Safe
move cio_file_unix.c::cio_file_native_sync Unknown -> Safe
move mk_server/mk_plugin.c::mk_plugin_sched_get_thread_conf Unknown -> Safe
delta Safe +4
route system
none
route slice
none
```

No row may move to `Unsafe`. A move to `Safe` other than these four is a
finding to explain before landing.

## The user's `current` run (Fluent Bit 046accf6)

That run's report has 2,790 uncontracted callees, of which 56 are
libc/builtin names. Predicted after landing:

- **23 callees leave the uncontracted list (52 blocked pairs).** Two of them
  are replaced by another refusal:
  - `pthread_setspecific`'s ten pairs become `retention-undecided`.
  - `getpwnam_r`/`getgrnam_r` become `retention-released-while-held` in
    `cio_file_native_lookup_user`/`_group`.
- **The other 33 libc/builtin callees stay** (103 pairs):
  - eight builtins (59 pairs), an open user choice;
  - `realloc` (6), an open user choice;
  - 24 refused names (38 pairs).
- **Seven functions leave the "blocked only by uncontracted calls" set** (508
  in the run):
  - `ctr_random_get`, `rd_kafka_transport_shutdown`,
    `cio_file_native_resize`, `cio_file_native_sync` and
    `mk_plugin_sched_get_thread_conf` are predicted **Safe**.
    `ctr_random_get` is the least certain: its actual is
    `(char *) buf + offset`.
  - `mk_sched_set_request_list` and `mk_sched_thread_lists_init` stay
    Unknown on `retention-undecided`.
- **No unit changes status.** Every unit that holds these functions has other
  Unknown functions.

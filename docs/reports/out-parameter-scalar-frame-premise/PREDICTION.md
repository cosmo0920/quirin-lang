# OUT-PARAMETER-SCALAR-FRAME-PREMISE prediction

Committed before the semantic change, on a branch cut from
OUT-PARAMETER-BODY-SUMMARY's tip `bb9412c2a`. The measurement's base side is
`rev:41d1b48a5` (that card's measured tip).

User decision, 2026-11-07 (verbatim): "A: effective-type". Rule 11r(6)'s
per-class analysis keeps a pointer-typed out slot across a store through a
*second* out formal when Clang types that store's lvalue as a non-character
integer or enumeration, under the listed premise `effective-type:` (C11
6.5p7), recorded in the row's assumptions. It does not apply in a unit built
with `-fno-strict-aliasing`.

Flag census ([`census/flags.json`](census/flags.json)): no Fluent Bit unit
of the 046accf6 databases (`current`) or of the measured routes' mirrored
databases (corpus, system, slice) carries `-fno-strict-aliasing`, chunkio and
librdkafka included. The Monkey route's database passes it to 27 entries,
all of them Monkey's bundled libevent sources (`buffer.c`, `event.c`,
`http.c`, …); the premise is withheld there.

Only rows whose sole blocker is the refused slot can move. Of the seven such
rows at the base, the three chunkio rows are exactly the shape (`int`,
literal returns, fresh `void **`/`char **` slot on `0`, unwritten on `-1`,
then `*size = n` through a `size_t *` formal):

```measure-prediction
route corpus
move cio_utils_read_file Unknown -> Safe
move cio_file_content_copy Unknown -> Safe
move cio_memfs_content_copy Unknown -> Safe
delta Safe +3
route system
none
route slice
none
```

`mk_vhost_get` (borrowed elements) and
`rd_kafka_consumer_group_metadata_write` (a possibly-null site, non-`int`
result) are not this shape and stay `Unknown`. No caller moves: the three
functions are called only from `cio_chunk.c`, another unit, and rule 4c is off
on these routes. Rows elsewhere may gain no premise: the premise is attached
only to rows that rest on such a class (the function and in-unit callers that
read its relation), so no other row's content should change.

**The user's `current` run** (Fluent Bit 046accf6): the same three functions
become `Safe` (each with the `effective-type:` premise in its assumptions);
no unit status changes (their units keep other `Unknown` rows); no callee
leaves the uncontracted list and no function leaves the "only uncontracted"
set.

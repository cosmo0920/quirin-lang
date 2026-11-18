# AG1-INDIRECT-MEMBER-ACCESS: prediction

Committed before the implementation commit (gateway rule 61). The integrator
measures with base `rev:a9c6dcf44` on corpus (eight-home), twelve-home,
system and slice. Derived from the census ([README](README.md) §1), the base
probes (README §2: s1–s16) and the libraries' own declarations; the card's
unit reproduction of the user's `current` (BEFORE and AFTER over the same
135 units) is reported against this file in README §4, not used to write it.

## Mechanism

Rule 61 changes one fact the aggregate store (O04-B08) is given: a local
declared exactly `A * NAME`, where every typedef declaration of `A` names one
complete, non-union struct, is `struct TAG *` for B08's base premises and
member reads; such an alias's pointee is never variably modified for a
`sizeof` operand. Nothing else is read differently. So a row can move only
where a member store or bound member read through such a local was refused
**only** for its spelling.

## Rows it can touch

- librdkafka's constructors over **defining** typedefs
  (`typedef struct rd_kafka_topic_partition_s { … } rd_kafka_topic_partition_t;`,
  `rd_kafka_share_ack_batch_entry_t`, `rd_kafka_topic_partition_list_t`, …)
  and over a forward-and-defining pair (`rd_kafka_Node_t`). The census counts
  232 access leaf groups that store a call result into `L->F` of a local
  spelled through a typedef name, in 13 sole-blocker rows' units (README §1,
  [census/local-owned-store-spelling.txt](census/local-owned-store-spelling.txt));
  not every such typedef defines its struct, so this is an upper bound on the
  leaves rule 61 reads.
- Keyword-spelled locals, formals, unions, forward-only typedefs and pointer
  typedefs are untouched.

## Predicted moves

**Corpus** (`66910c10`): its `rdkafka_partition.c` and
`rdkafka_share_acknowledgement.c` are byte-identical to the 046accf6 tree
the census read.

- `rdkafka_partition.c::rd_kafka_topic_partition_new` Unknown -> Safe: its
  only leaf is `rktpar->topic = rd_strdup(topic);` on a defining alias, an
  acquired `rd_calloc` base, then a scalar store and `return rktpar;`.
- `rdkafka_share_acknowledgement.c::rd_kafka_share_ack_batch_entry_new`
  Unknown -> Safe: its only leaf is `entry->types = rd_calloc(…);` on a
  defining alias, after scalar stores into the same acquired base, then
  `return entry;`.
- Possibly further Unknown -> Safe rows whose every leaf is such a store or
  a `unknown_alias` leaf the refused store caused (the refused statement
  writes its base, so later uses of the base are unresolved): **0 to 10**
  more, not named (unpredicted moves to explain).
- `rd_kafka_Node_new` stays Unknown: its `rack` store is on one arm, so the
  read store meets rule 11c's partial-link escape refusal (a re-attribution).
- **No Safe -> non-Safe move, no new Unsafe.** The reading only adds a
  premise fact; every other premise still refuses as before.

**Twelve-home** reads the corpus inputs: the same moves.

**System, slice**: none (Fluent Bit core files spell their records with the
`struct` keyword; a scalar member access through an alias-typed local is
already read by the typed routes).

```measure-prediction
route corpus
move rdkafka_partition.c::rd_kafka_topic_partition_new Unknown -> Safe
move rdkafka_share_acknowledgement.c::rd_kafka_share_ack_batch_entry_new Unknown -> Safe
route system
none
route slice
none
```

## The user's `current` run (046accf6)

- The same two rows Unknown -> Safe, and possibly 0 to 10 more of the same
  kind; `rd_kafka_Node_new` re-attributed to rule 11c.
- Access leaves removed: the alias-spelled local owned-result stores the
  rule reads (up to the 232 leaf groups above, fewer where the typedef is
  forward-only); `unknown_alias` leaves on the same bases go where the store
  was their cause.
- Sole-blocker rows of the access refusal: 232 -> about 225–230.
- No Safe -> non-Safe move.

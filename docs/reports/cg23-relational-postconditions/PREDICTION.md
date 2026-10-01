# CG23-RELATIONAL-POSTCONDITIONS — frozen prediction

Frozen 2026-10-01, before any AFTER build or measurement. The vocabulary, the
route-G hooks and their synthetic tests were written before this file (the
order is stated, not hidden); no corpus, system or slice run of the changed
code exists when it is committed.

## BEFORE (main `b6a3dc42`, this worktree's release binary)

Corpus `66910c10`, `--jobs 4`: **629 Safe / 10 Unsafe / 4,195 Unknown**,
`resource-diagnostics.json` `3d53a5d0b6728d4986a910bd81318a040995076bbded1970f9963022fcc3fcf2`,
equal to the brief's baseline. System and slice: recorded in the report from
the same BEFORE run.

## Census (`scripts/census.py`, raw sources, upper bounds)

The corpus can move only through the exit-specific route (rule 11r (4)): no
corpus contract module uses `resource_on_return`, so the declared vocabulary
moves nothing.

* 105 lexical candidate helpers: `int NAME(…)` definitions every `return` of
  which is an integer literal (41 of them only through a capitalised macro,
  which preprocessing may or may not make a literal), at least two values,
  each the callee of some `call to uncontracted function` leaf.
* 178 such call-site leaves in 100 non-Safe functions: 54 tested where they
  stand (or stored and tested next), 124 not.
* Freed alone (every non-Safe leaf a tested candidate site): 3 functions —
  `cmt_cat.c::append_context`, `ctr_mpack_utils.c::ctr_mpack_consume_int32_tag`,
  `ctr_mpack_utils.c::ctr_mpack_consume_uint32_tag`.

A candidate helper publishes only if each per-value body validates through
the ordinary route; most candidates' bodies hold other refusals (their own
uncontracted calls), so far fewer than 105 publish.

## Prediction

1. **Status.** Unknown → Safe: between 0 and 3, point estimate 1 (one of the
   three freed-alone functions). No Safe → non-Safe move (no Safe function
   carries an uncontracted leaf, and both rewrites are the identity without
   a disposition). Unknown → Unsafe: 0 expected; any that appears is a
   newly admitted call site exposing a leak or violation and is classified by
   reading.
2. **Leaves.** Every helper that publishes turns its untested call sites from
   `call to uncontracted function` to `relational-untested` (same status),
   at most 124 leaves; its tested sites lose that leaf, at most 54.
3. **System and tail-pause slice.** No status move; leaf-description changes
   only as in 2.
4. **Absorbed entries.** None is discharged by a corpus movement:
   `CONDITIONAL-RELEASE-BY-LITERAL` and `W47-SUMMARY-ARGUMENT-CONDITIONAL-NULL`
   condition on an argument, `W45-FB-BACKLOG-MAP-ARMS` and the
   `realpath`/`pthread_getspecific` parts on a pointer result or retained
   storage, `CJSON-DELETE-SIBLING-CHAIN` on a reachable set;
   `W43-DESCRIPTOR-REFUSED-SHAPES`'s status-witnessed out list (`getaddrinfo`)
   becomes expressible, with the declaration left to a provider card.
   Sizes at BEFORE (corpus; leaves / functions / sole-blocked Unknown):
   `rd_kafka_topic_partition_destroy0` 2/2/2, `realpath` 1/1/0,
   `pthread_getspecific` 24/16/1, `getaddrinfo` 4/4/0, `freeaddrinfo` 6/4/0,
   `fcntl` 1/1/1, `accept4` 1/1/1, `fts_open`/`fts_close` 1/1/0 each,
   `glob`/`globfree` 1/1/0 each, `cJSON_Delete` 0 leaves,
   `ut_sasl_oauthbearer_oidc_assertion_parse_from_file` Unsafe with 2 leak
   leaves.

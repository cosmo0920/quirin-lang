# W47-TYPED-ROOT-DECLARATION-READER — frozen prediction

Frozen before any analyzer change. Base `3ba72320` (local `main`, "Merge
runtime imports and explicit contract checks after complete gate").

## BEFORE reproduces the P3 AFTER identity

The base binary (`quirin-audit pinned --jobs 5` through
`scripts/run_pinned_audit.py`, the arbiter's current form: the
`cycle19_fluent_bit_libraries_audit` example that `w43-return-expression`'s
`run_audit.py` calls was retired by `e4a73427`, and
`runtime-source-imports/audit-comparison.json` records the two byte-identical)
exits 0. Against `w47-access-path-positions/artifacts/after/corpus/` (raw
SHA-256s checked after decompression): `resource-diagnostics.json`,
`monkey-source.json` and the chunk contracts are **byte-identical**, and the
sorted parity stream and its contexts are identical. The contracts and
source-effects sidecars differ; every difference is the `.qcore` → `.qv`
migration of `f4018a92` on `9b2ac682..3ba72320` (module paths, lock and module
digests, the included-module text, and the resource-contract ids and
build/configuration/task digests derived from them), none verdict-bearing.
This is the first post-merge measurement of P3's AFTER, and it reproduces it.

## Per-function cause (step 1)

A temporary trace (never committed; `scripts/trace_twelve_patch.py`, run by
`scripts/diag.py`, output in `artifacts/cause/`) printed, at each typed hook
of the twelve functions, the lexical reader's answers and the typed reader's
answer under **all-true premises**. For all twelve the lexical declaration
reader **is readable**, every root is `never_bound` and not `may_bind`, and the
typed reader **still refuses with every premise granted**. The premise is not
the cause for any of the ten returns:

| Function | Row | Exact reason (Clang node) |
|---|---|---|
| `rd_kafka_Node_host`, `rd_kafka_Node_rack` | `node->host` / `->rack` | value `char *const` (member through a `const` pointee); `is_pointer` requires a trailing `*` |
| `rd_kafka_mock_cluster_bootstraps` | `mcluster->bootstraps` | value `char *const` |
| `rd_kafka_mock_cluster_handle` | `mcluster->rk` | value `rd_kafka_t *const` |
| `rd_kafka_opaque` | `rk->rk_conf.opaque` | value `void *const` |
| `rd_kafka_handle_mock_cluster` | `rk->rk_mock.cluster` | value `rd_kafka_mock_cluster_t *const` |
| `rd_kafka_messages_get` | `messages->elems[index]` | element `rd_kafka_message_t *const` |
| `rd_kafka_share_partition_offsets_list_get` | `list->elems[index]` | element `rd_kafka_share_partition_offsets_t *const` |
| `rd_kafka_ConfigEntry_name` | `entry->kv->name` | `char[1]` array member returned by decay: an interior address, refused by rule 5j by design |
| `rd_kafka_share_partition_offsets_offsets` | `partition_offsets->offsets` | `const int64_t[]` array member by decay: rule 5j |
| `rd_kafka_topic_partition_get_leader_epoch`, `…_get_current_leader_epoch` | `parpriv = rktpar->_private` | the lexical declared-local answer is `Opaque` for `const rd_kafka_topic_partition_private_t *parpriv` (so no `condition-assignment` row is written), **and** the typed statement reader refuses the value: an `ImplicitCastExpr BitCast` from `void *` (which `strip` refuses) over a `void *const` member |

The P3 control `a_redeclared_typedef_formal_keeps_the_pointer_return_refusal`
does not isolate a declaration-reader cause either: its getter takes `const
g_t *`, so `node->host` is `char *const`. With a non-`const` formal the same
doubly declared typedef certifies at the base (control
`the_redeclared_typedef_control_refuses_for_its_const_pointee`, run at the
base: `Safe`; the `const` form `Unknown` with the return refusal only).

## Prediction

1. **None of the twelve becomes Safe.** Eight refuse on a `T *const` value, two
   on rule 5j, and the two `leader_epoch` getters, whose declared-local premise
   the AST reader will answer (`parpriv` is one automatic, non-formal pointer
   local), still refuse on the `BitCast`/`void *const` value. The eight and the
   two are registered as `W47-TYPED-CONST-POINTER-VALUE`; the array decays stay
   refused by rule 5j. The follow-up's mechanism is discharged, its motivating
   misses are not.
2. **Where the lexical reader answers, nothing changes.** The served never-bound
   and `may_bind` answers are the lexical ones whenever the lexical declaration
   reader read the function, and the declared-local answer keeps every lexical
   `yes`. No leaf of a function outside the servable set below moves.
3. **The servable population** (the all-true-premise census,
   `scripts/trace_premise_patch.py`, `artifacts/cause/premise-census.json.gz`):
   typed-position rows refused **only** by a premise, de-duplicated, where the
   lexical reader is unreadable — 202 condition rows in 46 functions, 169
   call-actual rows in 38, 6 condition-assignment rows in 4, 3 return rows in 3
   — plus 16 condition-assignment rows in 15 functions whose lexical
   declared-local answer is false: at most **396 rows**, in 79 function rows of
   the corpus. The AST closure is never less conservative than the lexical
   closure, so this is an upper bound; roots the body really binds keep their
   refusal.
4. **No function status moves.** `scripts/forecast.py`: every one of the 79
   function rows is Unknown and carries at least one leaf that is not a
   COND/RET/IND refusal at a servable row (the least blocked:
   `rd_kafka_poll_set_consumer`, `rd_kafka_assignment_lost`,
   `rd_kafka_unsubscribe`, each with one other leaf). Predicted: 0 moves, no
   Safe → non-Safe move, no new provisional Unsafe, no added leak or violation.
   Leaf movement: COND, IND and RET leaves removed only at servable rows (at
   most one per row), with `unknown_alias` leaves possibly added in their place
   (a typed read exchanges a refusal for its per-hop obligations).
5. **System slice** (`046accf6`): the same mechanism, predicted 0 function or
   configuration-row moves; any leaf change attributed the same way.
6. **Parity stream.** New `root-declarations` rows (in a parity run, one per
   analysis of every function with a unique Clang definition, with the
   lexical and AST answers side by side and every disagreement); every older
   row unchanged except where a served answer admits a row the base refused.

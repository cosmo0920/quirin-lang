# Integration confirmation (combined tree)

The integrator re-ran this card on 2026-09-25, on merge commit `b128e237`. It is
merged onto main `5a154e25`, after W44-C11-GENERIC-SELECTION,
W44-TEST-SUITE-WALL-TIME and W44-INDIRECT-CALL-ARGUMENT-ACCESS. The card merged
main `70ce6b44` itself; its 125 pre-merge leaks were re-checked there (122
withdrawn by the refused hand-on rule, 2 real unit-test leaks remain). Only
records conflicted:
* `proof/QCore.lean` imports (both kept);
* the ledger was merged per entry: 389 + 3 − 1 = **391**;
* the increment paragraph was prepended;
* the report index was regenerated;
* `STATUS.md` kept main's corpus rows for re-derivation and the card's
  indexed-collections row.

After the merge, integration gate `gate-integrate-collection-1` failed one of
the card's own tests on the combined tree; see [Integrator fix](#integrator-fix)
below. Commit `7edfd6b9` repairs it, and this confirmation is the re-run gate
on that commit.

The run used the integrator's own worktree, `CARGO_INCREMENTAL=0` and Clang
21.1.8. The figures are in [gate.json](gate.json).

## Gate

| Step | Exit | Result |
|---|---|---|
| core `cargo test --workspace --no-fail-fast` | 0 | **5,596 passed / 0 failed / 11 ignored** over 297 summaries |
| fmt, clippy `-D warnings`, both reevaluation examples, docs index | 0 | clean |
| `check_lean_audits.py` | 0 | 188 audits |
| Python before matrix regeneration | 1 | only `test_the_derived_facts_are_recomputed_rather_than_carried`, on the Lean count |
| Provider suite: tests, fmt, clippy, isolated mutation control, `contracts.py check`, 5 × build + verify, provider Python | 0 | 124 / 0 / 0 over 39 summaries; 13 Python tests |
| release corpus run | 0 | — |
| `baseline_identity.py` against `w44-indirect-call-argument-access/integration/artifacts` | 1 | by design; this card moves leaves |
| composition check ([composition-check.txt](composition-check.txt), [explained-leaves.json](explained-leaves.json)) | 0 | **composes** |
| `build_matrix.py --gate`, then Python | 0 / 0 | 349 tests |

## Composition

**Card delta.** Against `w44-libc-residuals/integration/artifacts`, the card
removes 230 leaves and adds 249. It moves 6 functions and 1 unit.

**Overlap: 0 leaves.**

**Explained: 14 combined-only unknown_alias leaves and one `_Generic` →
`bsearch` exchange**, in [explained-leaves.json](explained-leaves.json).

* 13 functions (`rd_kafka_AlterConsumerGroupOffsets_new`,
  `rd_kafka_ConfigResource_new`, `rd_kafka_DeleteConsumerGroupOffsets_new`,
  `rd_kafka_DeleteGroup_new`, `rd_kafka_DeleteTopic_new`,
  `rd_kafka_ListConsumerGroupOffsets_new`, `rd_kafka_NewPartitions_new`,
  `rd_kafka_group_result_new`, `rd_kafka_topic_result_new`,
  `rd_kafka_error_copy`, `rd_kafka_error_new_v`, `rd_kafka_lwtopic_new`,
  `rd_regex_comp`) each report an added `unresolved resource identity`
  unknown_alias leaf (14 leaves total: `rd_kafka_topic_result_new` carries 2).
  Combined-only, refusal for refusal, every function stays Unknown: the
  collection card binds `x = rd_calloc(1, sizeof(*x) + ...)` /
  `rd_malloc(sizeof(*x) + ...)` to an identity; the next statement, a member
  store through `x` that the lexical gate refuses (constraint 5i), names `x`,
  so W44-LEAK-PIN-AUDIT's refused hand-on rule leaves `x` unresolved; the
  following call argument that W44-INDIRECT-CALL-ARGUMENT-ACCESS admits as a
  typed read of `x->field` then reports unknown_alias on that unresolved
  identity instead of being admitted. On either card alone `x` was never bound
  (collection card absent) or the call argument stayed refused (call-argument
  card absent).
* `rd_kafka_ApiVersion_check`: on the card's own base (before
  W44-C11-GENERIC-SELECTION) the collection card's admission at
  `rdkafka_feature.c:374` exchanged a sizeof refusal for `call to uncontracted
  function \`_Generic\``. On the combined tree glibc's const-generic `bsearch`
  selection is read at the same position, so it reports `call to uncontracted
  function \`bsearch\`` (registered `W44-BSEARCH-UNDECLARED`) instead.

**Combined movement** (functions Safe / provisional Unsafe / Unknown; units
Safe / provisional Unsafe / Unknown / data-only):

| | main `5a154e25` | combined |
|---|---|---|
| functions | 302 / 10 / 4,522 | **306 / 12 / 4,516** |
| units | 3 / 9 / 169 / 4 | 3 / 10 / 168 / 4 |
| leak | 35 | 37 |
| unknown_alias | 3,392 | 3,507 |
| unsupported | 43,674 | **43,590** |

* **Unknown → Safe (4):** `XXH32_copyState`, `XXH64_copyState`
  (`memcpy(dst, src, sizeof(*dst))`), `clear_env`
  (`memset(env, 0, sizeof(*env))`), `rd_kafka_anyconf_clear_all_is_modified`.
  None decide through a collection element node.
* **Unknown → provisional Unsafe (2):** `unittest_http_get_params_append` and
  `ut_sasl_oauthbearer_oidc_assertion_process_template_file`, moving the unit
  `rdhttp.c` Unknown → provisional Unsafe with them. Both are real unit-test
  leaks, read as such by the card; this is not an upstream defect claim.

The combined run is archived as the new baseline under [artifacts/](artifacts/).

## Integrator fix

The first combined gate (kept privately as `gate-integrate-collection-1`)
failed the collection card's own test
`a_file_scope_name_keeps_the_refusal`: it printed Safe where the card pins
Unknown. The cause was W44-INDIRECT-CALL-ARGUMENT-ACCESS's `fold_type_sizes`,
which read a file-scope object name as a type name — on the combined tree it
folded a `sizeof(*g)` operand naming a file-scope object the same way it folds
a typedef or tag, so the sizeof collapsed to a constant instead of keeping its
refusal.

Commit `7edfd6b9` makes the fold follow that card's own documented rule: an
operand name only folds as a type when it is a typedef the unit's reader
recorded (`TypedAccessModel::declares_typedef`) or a tag following
`struct`/`union`/`enum`. Anything else keeps the refusal. The fix is
refusing-direction only, and adds a unit test pinning both the typedef/tag
folding and the file-scope refusal.

The Safe the first run printed was semantically correct on its own terms —
`sizeof` does not evaluate a non-VLA operand, and a VLA cannot have file scope
— but it rested on misclassifying the name, which the fix corrects without
changing that semantic fact.

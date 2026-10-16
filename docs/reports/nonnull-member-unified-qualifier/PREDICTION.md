# NONNULL-MEMBER-UNIFIED-QUALIFIER: prediction

Frozen before any loader, analyser, contract or Lean change. Base: local
`main` `3f8f1a617`. The BEFORE binary is built from the unchanged base tree
(`scripts/build.sh before`) and measured on the base's contracts.

## What changes

1. **Grammar (decision 3, 2026-10-16).** Bounds v1 reads one clause,
   `nonnull_member`, in three kinds:
   * plain `requires nonnull_member(TAG, FIELD);` (rule 14t);
   * stable `requires nonnull_member(TAG, FIELD) where quant #scope [[stable]];` (rule 14x);
   * scope `requires nonnull_member(TAG, FIELD, P) where quant #scope [[scope]];` (rule 14z).

   They load to exactly today's three facts (`NonnullMember`,
   `NonnullMemberStable`, `NonnullMemberScope`). `nonnull_member_stable` and
   `nonnull_member_scope` are rejected with a diagnostic naming the new form;
   no alias.
2. **Migration.** Every declaration using an old word is rewritten:
   librdkafka's 67 (`null_deref.qv`: 1 stable, 66 scope) and Fluent Bit's 14
   (4 stable, 10 scope). No other tracked contract module uses either word.
   Each is re-pinned at 66910c10 and 046accf6.
3. **Premise.** Bounds v1 admits `resource_premise` beside every kind. The
   5 plain clauses on 4 librdkafka declarations that meet
   LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER's reachability rule (`rd_kafka_toppar_new0`,
   `rd_kafka_topic_new0` with two clauses, `rd_kafka_cgrp_new`,
   `rd_kafka_transport_new`) name `rk_no_plugin_library`, re-pinned; the
   premise is keyed for plain members by `member:TAG.FIELD`.

## Predicted measurement (corpus, system, slice, with the null property)

* **Custody:** verdicts and totals unchanged everywhere (corpus 939 / 9 /
  3,886, system 95 / 4 / 349, slice 52 / 1 / 347 / 4 Pending). The corpus
  custody report is byte-identical or differs only in contract-source
  identity fields; system and slice likewise.
* **Null totals unchanged:** corpus 1,114 / 1,435 / 2,354 / 18 Unsafe, system
  86 / 125 / 243, slice 69 / 121 / 216.
* **Null rows:** every row equals its BEFORE row once its `premises` field,
  its `premise:` dependencies and its bounds-environment digest are removed.
  No verdict, reason, check, scope or other dependency moves. The evidence
  labels that name the fact kinds (`nonnull_member_stable` /
  `nonnull_member_scope` in assumptions and check subjects) are kept as
  written, so they do not move either.
* **Premise listing:** on the corpus, exactly the rows whose dependency set
  names one of `member:rd_kafka_toppar_s.rktp_rkt`,
  `member:rd_kafka_topic_s.rkt_rk`, `member:rd_kafka_topic_s.rkt_topic`,
  `member:rd_kafka_cgrp_s.rkcg_rk`, `member:rd_kafka_transport_s.rktrans_rkb`
  gain `rk_no_plugin_library` (or gain those members in `through` when they
  already list it through `stable:` or a scope function). The count is
  measured, not predicted. System and slice analyse no librdkafka unit: no
  row there gains a premise.
* **Fold:** the five members' `member_declarations` records gain
  `premises`; every other fold record is unchanged.
* **Sidecar:** only the re-pinned declarations' digests and premises, module
  and lock digests move.

Any other movement is a named correction.

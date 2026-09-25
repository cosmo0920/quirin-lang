# Integration confirmation (combined tree)

The integrator re-ran this card on 2026-09-26, on merge commit `d25b5ebe`. It is
merged onto main `5355998d`, after W44-INDIRECT-CHAIN-ACCESS and
W44-CORPUS-CALLEE-SUMMARIES. Only documentation conflicted. The run used the
integrator's own worktree, `CARGO_INCREMENTAL=0` and Clang 21.1.8. The figures are
in [gate.json](gate.json).

## Gate

| Step | Exit | Result |
|---|---|---|
| core `cargo test --workspace --no-fail-fast` | 0 | **5,454 passed / 0 failed / 11 ignored**, 291 summaries |
| fmt, clippy `-D warnings`, both reevaluation examples, docs index | 0 | clean |
| `check_lean_audits.py` | 0 | 180 audits |
| Python before matrix regeneration | 1 | only the derived-facts check, on the Lean count |
| Provider suite: tests, fmt, clippy, isolated mutation control, `contracts.py check`, 5 × build + verify, provider Python | 0 | 124 / 0 / 0; 13 Python tests |
| release corpus run | 0 | — |
| `baseline_identity.py` against `w44-corpus-callee-summaries/integration/artifacts` | 1 | by design; this card moves leaves |
| composition check with explained leaves ([composition-check.txt](composition-check.txt), [explained-leaves.json](explained-leaves.json)) | 0 | **composes** |
| `build_matrix.py --gate`, then Python | 0 / 0 | 367 follow-ups; 330 tests |

## Composition

**Card delta.** Against `w43-indexed-resource-collections/integration/artifacts`,
the card removes 294 leaves and adds 36, with no function or unit moves. Among the
removals is the `snappy.c` null-path false leak, so the leak count goes from 271
to 270.

**Combined tree.** The combined tree removes 13 more leaves and adds 2 more. Each
of the 15 is explained in `explained-leaves.json` and comes from one of three
interactions with W44-CORPUS-CALLEE-SUMMARIES, which the card's own report
anticipated. The card's rule uses the normal contract lookup, so any newly
summarized callee reaches conditions.

1. **`rd_uvarint_enc_*` summaries (6 leaves).** `rdvarint.h`'s `static`
   `rd_uvarint_enc_u64` is blocked only by `if (unlikely(of >= dstsize))`. This
   card admits `unlikely()`, so the body validates, and the reached-static-helper
   route then publishes summaries for `_u64`, `_i64` and `_i32`. The
   uncontracted-call leaves at their call sites go away in:
   * `rd_kafka_header_add`
   * `rd_kafka_msgset_writer_write_msg_v2`
   * `do_test_rd_uvarint_enc_i64`
2. **Condition callees summarized only by the other card (6 leaves).** These
   conditions call static helpers that only the reached-static-helper route
   summarizes, for example `flb_input_paused` and `flb_input_is_threaded` from
   `flb_input.h`. The refusal leaves go away in:
   * `ut_mean`
   * `rd_kafka_ssl_read_cert_chain_from_BIO`
   * `rd_kafka_txn_handle_AddOffsetsToTxn`
   * `flb_input_chunk_append_raw`
   * `flb_input_chunk_ring_buffer_collector`
   * `input_chunk_append_raw`
3. **Exchanged refusals (2 leaves).** In two places a desugared condition reads
   an unresolved identity and records it as `unknown_alias`, which is a refusal:
   * `err` in `rd_kafka_txn_handle_AddOffsetsToTxn`
   * `in` in `flb_input_chunk_set_limits`

No function changes status on the combined tree.

## Integrator decisions recorded

* **`W44-CONDITION-DISCARDED-DERIVED-RESULT`: accepted.** A derived result that a
  condition discards owns nothing. That is the classifier's existing rule for
  `memcpy(d, s, n);`.
* **`W44-B02-RESIDUAL-DESCRIPTOR-READ`: registered as a finding.** The older
  one-call route reads a tracked descriptor inside a comparison. A comparison
  only reads and never stores, so custody is unaffected.

The combined run is archived as the new baseline under [artifacts/](artifacts/).

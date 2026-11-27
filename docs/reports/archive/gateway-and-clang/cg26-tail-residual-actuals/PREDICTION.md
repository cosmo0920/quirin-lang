# CG26-TAIL-RESIDUAL-ACTUALS: prediction (frozen before the implementation)

Base: local `main` `cb3085365`. BEFORE (this worktree's release
`quirin-audit`, `39953f7d…`; corpus `66910c10`, `--jobs 4`):
**649 / 9 / 4,176**, `resource-diagnostics.json` `450a9e54…`, equal to the
brief. [Residual census](artifacts/residuals-before-corpus.txt)
(`scripts/residuals.py`).

## Scope (amended before this file was frozen)

The integrator's scope change arrived before this prediction: the card
CAST-FORMALIZATION (rule 12h) owns every cast reading. This card reads
(1) plain-name trailing actuals in functions whose lexical declarations are
unreadable, (2) non-cast scalar arithmetic and conditional actuals, and (3) the
assignment form `r = snprintf(…)`. An actual containing an explicit cast is
refused by name (`cast`) and left to CAST-FORMALIZATION. Rule id: **12g**.

## The reading

A new route-G reading (`variadic_tail_actuals.rs`, rule 12g), asked inside rule
11w's statement fallback only for a trailing actual T1-T8 refused: the call's
Clang argument node is classified as data (literals, enumeration constants,
constant-size `sizeof`, arithmetic objects that carry no integer-carried
identity, pointer or array objects rule 10l's served never-bound premise
answers, and unary/binary arithmetic, comparison or logical operators and
`?:` whose every operand is data), as one identity-carrying pointer name (a
`Borrowed` place of that name only under a declared borrowed tail and only for
a name rule 10l answers as declared once), or refused by name (cast, pointer
arithmetic, identity arm, identity operand, identity under the data tail,
integer identity, unanswered root, unread node). The statement's refusal text
is unchanged wherever the reading refuses.

## Census at the base (reproduces the registered sizes)

- Corpus: **40 tail leaves in 23 functions**, from 34 statements with a
  `variadic-tail-statement` row (24 in functions whose lexical declarations
  are unreadable, 10 in readable ones), plus **6 leaves** in
  `unittest_http_get` (4) and `unittest_http_get_params_append` (2) whose
  statements have no row: rule 11w's fallback never matched their Clang
  node. The registered split (19 plain names / 14 cast-arithmetic-conditional
  / 1 assignment) is the predecessor's; at this base the 34 statements are:
  6 with a cast actual, 3 with an identity-carrying conditional (`data ? … :
  …` in `rd_kafka_ssl_error0` ×2, `val2 ? val2 : "NULL"` in
  `unittest_untyped_map`), and 25 others.
- None of the 23 functions is blocked by its tail leaves alone (as the ledger
  records).
- System: 4 leaves, 2 functions (`flb_coro_destroy` 2: a cast;
  `flb_storage_quarantine_chunk` 2: `(void *) src`, a cast). Slice: none.

## Predictions

1. **Corpus status: no move** (649 / 9 / 4,176 again). No Safe → non-Safe move.
   Allowance: at most 2 Unknown → Unsafe, each only by a leak or violation on
   a path the admitted statement now lets the flow read, read and classified
   in the report; none expected.
2. **Tail leaves: 40 → 15** (−25), all in the 25 statements above:
   - gone (15 functions): `rd_kafka_sticky_assignor_unittest` 4,
     `unittest_ssl` 4, `rd_kafka_range_assignor_unittest` 3,
     `rd_string_render` 3, `do_unittest_erase`,
     `do_unittest_write_read_payload_correctness` (`max_cnt * sizeof (crc)`),
     `mk_server_info`, `rd_kafka_topic_new0`, `rd_kafka_version_str` (the
     assignment form), `rd_msghdr_print`, `rd_unittest`,
     `unit_test_telemetry`, `unittest_fetcher_share_filter_forward`,
     `unittest_msgq_order` (`fifo ? "…" : "…"`), `ut_string_split`, 1 each;
   - kept (15 leaves, 8 functions): casts in `ut_quantile`,
     `rd_kafka_anyconf_get0`, `rd_kafka_snappy_java_uncompress`,
     `rd_kafka_conf_set_ssl_cert`, `unittest_untyped_map` ×2 (refused
     `cast`); identity conditionals in `rd_kafka_ssl_error0` ×2
     (`identity-operand`: `data` is address-taken) and `unittest_untyped_map`
     ×1 (`identity-operand`: `val2` holds a call result); the 6 unmatched
     leaves of the two `rdhttp.c` tests.
3. **Other leaves of the 15 functions** may change: a name now read as a
   `Borrowed` place that an earlier refused statement wrote reports an
   unresolved identity (rule 11d), and the statements after the admitted one
   are read; any added leaf is attributed in the report. No leaf other than
   those of the 15 functions changes.
4. **System**: no status move; its 4 tail leaves stay (both casts). **Slice**:
   byte-identical up to the embedded binary identity (no tail leaf there).
5. **Round-12 tip probe**: `mk_vhost_set` and `mk_vhost_set_property`
   unchanged (neither calls a declared-tail callee with a residual actual).
6. Lean: one new module, `QCore.VariadicTailActuals`, with an audit file; Lean
   audits 282 → 283.

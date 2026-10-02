# SUP07-REFUSED-TEST-ACCUSATION: prediction

Frozen before any implementation code (the rule's decision, its hook and its
helper). Committed before it: the census (`legacy_leak_witness.rs`, the two
pure readings in `legacy_refused_test.rs` it shares with the rule), the
controls (`tests/sup07_refused_test_accusation.rs`) and the measurement
scripts (`9cf0960a8`). Base: local `main` at `cffe232d4`.

## BEFORE (measured on `cffe232d4`, release `quirin-audit`)

| Run | Totals | `resource-diagnostics.json` |
|---|---|---|
| Corpus arbiter (`66910c10`, `--jobs 4`) | **647 / 11 / 4,176**; leak 42, summary_conflict 2, unknown_alias 11,463, unsupported 29,110 | `a7d68d49…` (= the brief's main) |
| Fluent Bit system route | 83 / 4 / 361; counted_user 4, summary_conflict 4, unknown_alias 1,119, unsupported 2,612 | `09423cd7…` |
| Tail-pause slice | 45 / 1 / 354 / 4 Pending; counted_user 1 | `86810e4d…` |

The system file differs from MONKEY-VHOST-PROPERTY-CUSTODY's AFTER
(`21b08170…`) only in identity fields (53 differences: per-unit
`resource_contract_sources` and `inventory_sha256`, and review-hold module
paths naming the worktree); every status and leaf is equal.

## Reproduction

`mk_http_error_page` is a provisional `Unsafe` with one leak leaf, reported at
the `return -1` of `mk_http.c:399` for the copy acquired at :395. The control
`monkey_mk_http_error_page_is_unknown_without_a_leak` (the pinned body) fails
on BEFORE with `Unsafe` and the leaves `unknown_alias@:398 temp` and
`leak@:399<-:395`, beside the refused :392 (`indirect memory access is
outside the resource projection`, the `*message` read) and the uncontracted
`mk_string_html_escape` / `mk_string_build`. The analyzer's path: the :392
refusal poisons `temp`; the else arm binds it to the nullable copy; at the
join `temp` is the copy plus an unknown value; rule 10b's refinement of
`if (!temp)` returns `Unknown` (identity refusal, `UnknownAlias temp`) and
keeps the copy `LIVE` on the null edge; `report_leaks` accuses it at :399.

By reading (`mk_server/mk_http.c:383-411` and `mk_core/mk_memory.c:51` in the
pinned tree): on the null edge either `mk_ptr_to_buf` returned NULL (no copy
was made) or `mk_string_dup` returned NULL (no obligation). The accusation is
false, and only the refused test kept it. The reading holds; the card goes on.

Census (the off-by-default `indirect-census` build, `QCORE_LEAK_WITNESS`;
each run's diagnostics byte-identical to the plain BEFORE: corpus `a7d68d49…`,
system `09423cd7…`, slice `86810e4d…`):

- **Corpus:** 102 leak witnesses (42 leaves, repeated over W4 invocations):
  86 have a path avoiding every refused test's null edge whose binding may
  denote the site (`independent`), 7 have no settled path, and 9 rest on such
  an edge on every path: 3 `mk_http_error_page` (joined provenance ties the
  copy to `temp` at :398) and 6 `mk_sched_add_connection` (the :201 and :205
  allocations at the :210 exit of `if (!conn)`; **not** tied: `conn` is not
  among the body's conditional names, because the lexical address reader
  `frame_protected_locals` reads `&conn->event` as taking `conn`'s address).
  133 refused-test null edges whose binding denotes a site, in 46 functions;
  21 (9 distinct) are tied, in 7 functions: `mk_http_error_page` and six
  librdkafka functions, all `Unknown`
  (`rd_kafka_ConsumerGroupDescribeResponse_parse`,
  `rd_kafka_DescribeConsumerGroupsResponse_parse`,
  `rd_kafka_oidc_token_jwt_bearer_refresh_cb`,
  `rd_kafka_oidc_token_metadata_azure_imds_refresh_cb`,
  `rd_kafka_jwt_b64_decode_payload`,
  `rd_kafka_oidc_assertion_parse_template_file`).
- **System, slice:** no leak witness; 16 and 8 refused-test null edges, each
  in one function, none tied.
- **Other violations:** the summary conflicts (corpus 2, system 4) and
  counted-user refutations (system 4, slice 1) sit in functions with no
  refused test whose binding denotes a site, so none rests on one.

## The rule (gateway 12e)

On the null (invalid) edge of a test the older refinement could not decide
(`PointerRefinement::Unknown`), each site the tested binding may denote and
joined provenance ties to the tested name at the test is settled: removed
exactly as a decided null test's null edge removes its site. The tie says the
name denotes the site on every path on which the site exists, so on that edge
the site is NULL or absent. The identity refusal (`UnknownAlias`) and the
edge's `unmodeled` flag stay, so the function never becomes `Safe` by it.
Untied and untargeted sites, the other edge and every decided test are kept.

Not an unresolved-effect marker: the marker is joined by union, so it would
also withdraw a leak established on the test's other edge where the two meet
(the control `a_leak_on_the_non_null_edge_stays_accused_across_the_join`).
Not every refused-test-only claim: without the tie the site may be live under
no name on that edge (the control `a_copy_lost_before_the_refused_test_stays_accused`),
which is why `mk_sched_add_connection` is out of reach here.

## Predicted AFTER

1. **Corpus 647 / 10 / 4,177.** `mk_http_error_page` Unsafe → Unknown, its
   leak leaf removed (leak 42 → 41), every other leaf of it unchanged.
2. `mk_sched_add_connection` stays a provisional `Unsafe` (untied); it stays
   with `LEAK-UNDECIDED-NULL-TEST`, amended with the census and the reason.
3. The six librdkafka functions stay `Unknown`; their leaves do not change
   (their tied sites carry no leaf on those edges). Any leaf that does move
   is read and named.
4. No other corpus function changes. No Safe → non-Safe move, no added leak
   or violation, no other status move.
5. System and slice: no status or leaf change (no tied edge); the files are
   byte-identical to BEFORE.
6. The probe (`mk_vhost_set` at the tip and master, `mk_vhost_set_property`):
   unchanged (tip `mk_vhost_set` Unknown on :533 only; `mk_vhost_set_property`
   :502/:503 only).
7. The six controls pass (the three failing on BEFORE are the shape, the
   Monkey body and the leak beside it); Lean audits 280 → 281.

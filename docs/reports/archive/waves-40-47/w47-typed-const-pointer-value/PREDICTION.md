# W47-TYPED-CONST-POINTER-VALUE — frozen prediction

Frozen before any analyzer change. Base `66be515f` (local `main`, "Merge
PROVIDER-PY-DUAL-STEM-FIXTURE-STALE onto the round-1 integration"; equal to
`origin/main`). The negative controls (`crates/qcore-c-gateway/tests/typed_const_pointer_value.rs`)
were written and run at the base first ([artifacts/cause/controls-at-base.txt](artifacts/cause/controls-at-base.txt)).

## BEFORE reproduces the round-1 baseline

Arbiter: `QCORE_TYPED_PATH_PARITY=OUT/parity.jsonl scripts/run_pinned_audit.py
OUT 66be515f --inputs-root /home/cosmo/Documents/GitHub/qcore-c/build/cycle19
--jobs 5` and `w46-contract-gap-expansion/scripts/run_system.py OUT --jobs 5`
at `046accf6`, each with `QCORE_TYPED_PATH_PARITY`. Both exit 0. Against the
archive `w47-callee-closure-r2/integration/artifacts/{corpus,system}/`
(decompressed, every manifest raw SHA-256 checked by `scripts/unpack.py`):
corpus `resource-diagnostics.json`, `monkey-source.json` and the chunk
contracts are byte-identical and the sorted parity stream is identical; the
contracts sidecar is identical once the worktree root is aliased; the
source-effects sidecar differs only in cwd-bound build, configuration and task
digests and the ids derived from them; the parity contexts differ only in the
frontend binary digest. System: 0 changed rows (configuration rows 63 / 4 /
381; unsupported 2,908, unknown_alias 985); the raw file differs only in the
path-bound `resource_contract_sources` and `inventory_sha256` digests
([before-identity/](artifacts/before-identity/)).
So BEFORE is **510 Safe / 10 provisional Unsafe / 4,314 Unknown**, units
6 / 8 / 167 / 4, leak 36, unknown_alias 7,443, unsupported 36,209.

## The census (step 0)

A temporary patch (`scripts/census_patch.py`, never committed; run by
`scripts/diag.py` and `scripts/diag_system.py`) marks, at the base, every typed
row the **unchanged** API would admit — every other check and every root
premise included — except that the value check refused a pointer value whose
only top-level qualifier is `const` (bit 1), or `strip` refused an implicit
`BitCast` from `void *` to an object pointer at the top of a
pointer-destination statement (bit 2). The row keeps its refusal. Output:
[census-rows.tsv](artifacts/cause/census-rows.tsv),
[forecast.json](artifacts/cause/forecast.json) (`scripts/forecast.py`).

Corpus: **221** distinct rows in **155** functions — bit 1: 74 condition, 34
call-actual, 29 statement, 9 return, 1 condition-assignment; bit 2: 67
statement, 2 condition-assignment; both: 5 condition-assignment. 141 of the
functions have BEFORE rows (all Unknown); 14 are header `static inline`
bodies with no reported row (`rd_slice_offset`, `rd_kafkap_str_cmp`,
`rd_kafka_topic_partition_get_private`, …).

## Prediction

**Step 1 (a `T *const` value).**

1. **Eight of the ten become Safe**: `rd_kafka_Node_host`,
   `rd_kafka_Node_rack`, `rd_kafka_mock_cluster_bootstraps`,
   `rd_kafka_mock_cluster_handle`, `rd_kafka_opaque`,
   `rd_kafka_handle_mock_cluster`, `rd_kafka_messages_get`,
   `rd_kafka_share_partition_offsets_list_get`. Each has one leaf, the return
   refusal on a census row, and its root is an unwritten formal.
2. **One more is forecast Safe**: `rd_kafka_message_leader_epoch`, whose one
   leaf is its controlling expression's refusal and whose census row is the
   operand `! rkmessage -> rkt` (`rd_kafka_topic_t *const`). The census reads
   operands in order and records only the first refusal, so another operand
   of the same condition may still refuse; this one is less certain.
3. `rd_kafka_ConfigEntry_name` and `rd_kafka_share_partition_offsets_offsets`
   stay Unknown (rule 5j, array decay), as the follow-up says.
4. **Leaf exchanges**: of 84 function rows with a bit-1 census row, 78 have
   COND/RET/IND leaves on 109 census lines; each such leaf may be removed
   (one per row) and `unknown_alias` leaves may be added in its place (the
   read's per-hop obligations on a root the body binds, and rule 10b on a
   borrowed destination). No other leaf family moves.
5. **Cascades**: the 14 header bodies' rows may make a body validate and its
   derived summary serve callers in the same unit; any such move is a
   summary cascade and will be attributed as one. None is forecast.
6. No Safe → non-Safe move, no new provisional Unsafe, no added leak or
   violation. A newly admitted statement ends rule 10a's poison of the
   bindings it could not write; if that exposes a leak it is classified by
   reading.

**Step 2 (the `void *` bit-cast), measured separately on top of step 1.**

7. `rd_kafka_topic_partition_get_leader_epoch` and
   `…_get_current_leader_epoch` **stay Unknown**: their condition-assignment
   `parpriv = rktpar->_private` becomes P2's borrowed read (the member is
   `void *const`, so it needs step 1 too), and the null test on `parpriv`
   keeps its identity refusal (rule 10b), so the COND leaf is exchanged for
   `unknown_alias:parpriv` (as P3's `an_assignment_operand_is_a_borrowed_read_then_the_null_branch`).
   Of the ten named functions, therefore, **eight** become Safe.
8. 57 more function rows (bit 2 or 3 only) exchange leaves the same way
   (`mk_iov_print`'s `c = mk_io->io[i].iov_base` is the one whose every leaf
   is covered: forecast Unknown with `unknown_alias:c`, since `c[j]` then
   reads through the borrowed alias). No status move is forecast for step 2.
9. The same exclusions as 6.

**Both steps.**

10. Interlock: no function moved to Safe has an admitted-row root in the
    nested-assignment disputed class (roots above are unwritten formals); the
    check is re-run on every AFTER Safe from the AFTER parity
    `root-declarations` rows.
11. **System slice** (`046accf6`): the system census
    ([census-system-rows.tsv](artifacts/cause/census-system-rows.tsv)) is
    the forecast; see the addendum below.
12. The P3 control `a_redeclared_typedef_formal_keeps_the_pointer_return_refusal`
    and rule 10l's `the_redeclared_typedef_control_refuses_for_its_const_pointee`
    now certify: their cause (`char *const`) is exactly what this card admits.
    Both are renamed and their assertions changed to `Safe`.

## Addendum: the system slice (written before the change)

The system census at `046accf6` (same patch, `scripts/diag_system.py`):
**14** rows in **6** functions (12 configuration rows, all Unknown) —
`flb_input_chunk_write_header_v2` (4 conditions and 4 actuals reading
`routes[index].label` / `.plugin_name` through a `const` element),
`input_chunk_collect_output_references` and `task_collect_output_references`
(`label = route->label`, bit 1), `flb_input_chunk_protect`,
`flb_input_chunk_release_space_compound` and `flb_sched_event_handler`
(`void *` members into typed locals, bit 2). No function has every leaf
covered ([forecast-system.json](artifacts/cause/forecast-system.json)):
**no configuration-row or unique-function status moves in either step**;
step 1 exchanges leaves in 6 configuration rows, step 2 in the other 6, each
at a census row, with `unknown_alias` leaves possibly added.

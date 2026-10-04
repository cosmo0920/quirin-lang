# CALLEE-RETENTION-CLASS1: prediction (frozen before any implementation code or contract)

Base: local `main` at `6fc62c015`. Written after the BEFORE runs and the
reading below, before any analyzer code, contract text or Lean was written.

## BEFORE (reproduced)

Binary `private/crc1/bin/before`, sha256 `4fdeb725…` (equal to `run.json`'s
`binary_sha256`):

* corpus `66910c10`: **803 / 9 / 4,022**, `resource-diagnostics.json`
  `2734991a…`, byte for byte the brief's;
* system: 85 / 4 / 359 (`dae935e5…`, this worktree's paths);
* tail-pause slice: 45 / 1 / 354 with 4 Pending (`eaefe6a1…`).

`scripts/rank.py` over the corpus gives the W45 AFTER population exactly: 652
functions blocked by uncontracted calls alone, 473 by one callee alone;
`scripts/tally.py` with W45's classification is byte-identical to
`w45-callee-cascade-residual/artifacts/tally-after.txt` (class 1: 36 callees,
94 functions decided alone).

## What reading the members changed before any code

* **Every (a) caller is a forwarding wrapper.** `cJSON_AddItemToArray`,
  `cJSON_ReplaceItemInObject{,CaseSensitive}`, `ctr_attributes_set_array/_kvlist`
  and the four `ctr_span*_set_attribute_array/_kvlist` are each
  `return f(…);` of the custody-taking insert. Rule 11r refuses such a call
  (`relational-untested`: its value is not tested where it stands), and the
  exit-specific route reads only literal returns. Measured on a probe:
  `int wrap(void *p) { return rp_take(p); }` is Unknown with
  `relational-untested`, and even a literal-return body whose exit-specific
  summary validates (`wrap_tested`) keeps an **Unknown** row, because its row
  is analysed contract-free. So declaring the dispositions alone frees none of
  the cited (a) functions.
* **Stage (a) therefore needs one route, in the existing vocabulary** (no new
  clause): gateway rule **12s**, relational forwarding. (1) A body whose every
  `return` is a plain integer literal or `return f(ARGS);` of one callee with a
  disposition (one class set) gets the callee's classes, each validated by the
  ordinary route on the body restricted to that class's exits (the forwarded
  call read through rule 11r's decided test on the restricted body). (2) A
  function whose disposition is in force (declared and not withdrawn, or
  inferred) has its own row decided per class: Safe only when every class's
  restricted body validates against that class's contract with no
  diagnostic; otherwise the contract-free row as today.
* **`add_item_to_object` is not (a).** Its `constant_key` arm stores the
  caller's `string` in the item (`item->string = cast_away_const(string)`),
  a retained pointer, and `false`/`true` preprocess to casts
  (`((cJSON_bool)0)`), which C04.5.1 refuses as a literal premise. It moves to
  (b)'s reading and is left undeclared in (a).
* **`rd_kafka_confval_set_type` cannot be declared**: its `valuetype` is an
  enum typedef, which the declared-ABI reader refuses
  (`W43-LIBC-ABI-READER-SHAPES`), and only its `PTR` arm retains `valuep`.
* **`cJSON_ParseWithLengthOpts`** retains `value` in the file-scope
  `global_error` and also writes an interior pointer of `value` through
  `return_parse_end`, which no out-slot mode states.
* **Every (b) caller passes a formal as both `P` and holder** (the LZ4
  `*_withState*`/`*_continue` wrappers, `flb_input_chunk_ring_buffer_enqueue*`,
  the `rd_kafka_AdminOptions_set_*` setters, the interceptor adders). Under
  the user's rule an undecided holder lifetime is Pending or Unknown, never
  Safe.
* **(c) has no vocabulary.** `CG22-CALLBACK-REGISTRY-CONTRACT` is open: no
  contract states a callback capture or an invocation over retained elements.

## Design to be implemented

* **(a)** Rule 12s as above (`legacy_relational_forwarding.rs`, thin hooks in
  `legacy_relational_summary.rs` / `legacy_summary.rs` and
  `analyze_defined_function`), Lean `QCore.RelationalForwarding`. Contracts
  (pinned at `66910c10` and `046accf6`): `add_item_to_array` (`ne 0` consumed,
  `eq 0` owned, `item` nullable), `replace_item_in_object` (same shape on
  `replacement`) in `qcore-provider-librdkafka`; `cfl_kvlist_insert_array`,
  `cfl_kvlist_insert_kvlist` (`eq 0` consumed, `ne 0` owned) in
  `quirin-provider-cfl`; `ctr_attributes_set_array`, `_kvlist` (the same,
  body-checked by the forwarding route in their own unit) in
  `qcore-provider-ctraces`, so the cross-unit `ctr_span*` callers read them.
* **(b)** A new clause `ensures normal resource_retained_by(p, h);` (`h` a
  distinct pointer parameter or the word `global`) and
  `ensures normal resource_holder_reset(h);`, in the contract specification
  (C04.14), grammar G05 and Lean `QCore.RetainedBy`. A declaration carrying
  either is **withheld from the function map** (C06.1's precedent), so every
  consumer that does not implement it sees an uncontracted callee: a retained
  pointer is never read as borrowed. Route G (rule **12t**) decides
  "outlives" from: a static `P` (a string literal); a frame holder `&L` (`L` an
  automatic object of the body, named only at holder positions), which dies
  at return, with `P` a caller-supplied formal the body holds no custody of,
  or a tracked identity not released while held; a tracked heap holder
  released (or reset) before `P` is; a holder reset call. Everything else —
  a formal, a member through a pointer or a global holder of a non-static `P`
  — is **Unknown** with the named reason `retention-undecided` (Pending would
  need a new reason code and its audit reporting; registered as a follow-up).
  A release of `P` while a live holder retains it is **refused**, never
  `Unsafe`: a dangling pointer left in a holder is a hazard only if the
  holder reads it. Contracts (pinned): `LZ4_compress_HC_extStateHC`,
  `LZ4_compress_HC_continue`, `LZ4_compress_fast_continue`,
  `LZ4_compress_fast_extState` (`src` retained by the state),
  `append_to_ring_buffer` (`ins` retained by a holder outside the caller,
  `global`), where each body read at both revisions supports it.
* **(c)** Recorded and sized; no code.

## Predicted movement

| stage | corpus | system | slice |
| --- | --- | --- | --- |
| (a) | **+5 to +9 Unknown → Safe** (ceiling 9: `cJSON_AddItemToArray`, `cJSON_ReplaceItemInObject`, `_CaseSensitive`, `ctr_attributes_set_array`, `_kvlist`, the four `ctr_span*` setters), plus 0 to 3 literal-return helpers whose per-class row now validates; 0 Safe → non-Safe | no move | no move |
| (b) | **0 moves to Safe**; leaves re-attributed from ``call to uncontracted function`` to `retention-undecided` in about 15 functions (LZ4 wrappers, ring-buffer enqueuers); at most 2 Unknown → Safe through a decided frame holder (`LZ4_compress_HC`, `LZ4_compress_fast` own rows, if their holder is read as a frame or tracked heap holder) | 0 moves; 2 leaves re-attributed (`append_to_ring_buffer`) | no move |
| (c) | no change (no code) | no change | no change |

New leaves expected: `relational-untested` at untested call sites of the new
dispositions (bounded by their current uncontracted leaves: 4 + 2 + 2 + 4 +
4 = 16 in 13 functions, fewer where tested); no new leak, violation or
`Unsafe` (0 to 2 if a forwarded or decided body exposes a statement the
uncontracted call hid, each read by hand). The four W45-cited heads of (b)
(`LZ4_compress_HC_extStateHC` 4, `_continue` 3 + 3, `_extState` 2) and
`append_to_ring_buffer` 2 stay Unknown, as the user's rule requires for a
formal holder.

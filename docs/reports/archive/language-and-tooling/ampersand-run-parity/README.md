# AMPERSAND-RUN-PARITY-OTHER-READERS: every `&` address decision reads the run parity

Card AMPERSAND-RUN-PARITY-OTHER-READERS (USER DECISION 2026-11-23: "launch the
ampersand parity card"). Base `9193b196a` (main). [PREDICTION.md](PREDICTION.md)
was committed (`996c5b7d6`) before the implementation (`73cfa12e4`,
`96218cc75`). Gateway rule [74e](../../../../agents/gateway.md#constraint-74e),
extending rule 74d (C); `QCore.AmpersandRunParity` with its audit.

## 1. Census

The importer emits one token per `&`, so `p && q` is `p & & q` and `p && &q`
is `p & & & q`. By maximal munch (C11 6.4p4), and because `a & &b` is not
conforming (C11 6.5.10p2), the last `&` of a run before an operand is an `&`
operator exactly when the run is odd. Every `"&"`, `'&'`, `"&&"` and `Amp`
match in `crates/` (376 lines, 133 files, the base's non-test code) was read.
Lines are the base's. Classes: **unsound** (an address missed), **admitting**
(an address read where none is, and the reading admits), **fail-closed** (an
address read where none is, and the reading refuses), **correct**.

### Changed: unsound (8)

| site | decides | base reading | fixture failing at the base |
|---|---|---|---|
| `loop_owner_instance.rs:549` `address_taken` (19 callers among the loop-owner premise derivations) | `&name` taken | run of exactly one `&` (`texts[k-1] == "&" && texts[k-2] != "&"`) | `loop_owner_coro_plugin.rs` `nc_a_helper_addressing_its_instance_after_a_logical_and_keeps_the_route` |
| `loop_owner_instance.rs:1820` copy-member check | `&x->m` of the copy member | exactly one | `loop_owner_typed_route.rs` `nc_the_copy_member_addressed_after_a_logical_and_keeps_the_escape` |
| `loop_owner_holder_opaque.rs:605` carrier `written` | `&c->m` through a holder pointer | exactly one | `loop_owner_holder_region.rs` `nc_a_holder_byte_addressed_after_a_logical_and_keeps_the_route` |
| `loop_owner_holder_opaque.rs:137` `flow_of` `logical_before` | value tested by `&&` | any `& &` before the value is a logical AND, so `p && &v` was "compared" | `loop_owner_holder_region.rs` `nc_an_init_callback_that_addresses_its_data_after_a_logical_and_keeps_the_route` |
| `thread_entry_census.rs:2598` | instance address | exactly one | `loop_owner_typed_route.rs` `nc_the_instance_addressed_after_a_logical_and_is_underived` |
| `thread_entry_census.rs:2707` | address of the copy's holder | exactly one | `loop_owner_typed_route.rs` `nc_the_copys_holder_addressed_after_a_logical_and_is_underived` |
| `thread_entry_census.rs:2724` | copy's address passed on | exactly one | `loop_owner_typed_route.rs` `nc_a_copy_addressed_after_a_logical_and_is_underived` |
| `loop_owner_coro_plugin.rs:1156` call index | `f (` is a call | dropped after any `&`, so `x && f(i)` hid a caller whose argument the instance premise never checked | `loop_owner_coro_plugin.rs` `nc_a_registration_called_after_a_logical_and_by_another_plugin_keeps_the_route` |

The call index is now a plain call reading: `& f (` is never an address (a
call's result is not an lvalue, C11 6.5.3.2p1). Each of the eight fixtures
was run with the base's gateway sources restored (`git checkout 9193b196a --
crates/qcore-c-gateway/src`) and **failed**, each by deriving the premise the
address should have withdrawn (logs in the worktree's
`private/arp/base-fixtures/`); the two controls
(`a_logical_and_before_the_copys_holder_takes_no_address`,
`an_init_callback_that_tests_its_data_after_a_logical_and_derives_the_holder_opaque`)
pass at both. Unit tests pin `address_taken` and `flow_of` directly.

### Changed: admitting (2)

| site | decides | base reading |
|---|---|---|
| `legacy_retained_by.rs:234` `only_holder_mentions` | whole `&name` actual of a retaining callee | any `&`: `f(x, p && name)` admitted as a holder mention |
| `legacy_callback_route.rs:450` `only_registry_mentions` | whole `&name` actual of a holder-naming callee | any `&` |

Both now read the parity, so `p && name)` falls through to the refusal. (A
holder is a record object; `p && record` is not a conforming operand, so the
corpus is not expected to spell it.)

### Changed: fail-closed (18)

Each took any `&` before a name as its address (`previous == Some("&")`), so
`p && q` refused `q`; each now refuses after an odd run only, keeping its own
reading of a single `&` (binary after an operand where it had one):
`legacy_callback_route.rs:2375` (`rebound`), `legacy_retained_by.rs:330`,
`legacy_list_iteration.rs:640`, `:745`, `legacy_reference_result.rs:282`,
`indexed_collection.rs:905`, `aggregate_store.rs:322`,
`registration_by_id.rs:320` (no effect: a non-touching mention continues
either way, a touching one is refused either way), `:499`,
`typed_field_access.rs:1814`, `borrowed_actual.rs:260` (a census reason
label), `:406`, `legacy_retention_guard.rs:629`,
`legacy_reference_count.rs:774`, `legacy_slot_move_borrow.rs:1041`,
`legacy_refused_operands.rs:396` (had the binary reading; read a run of two as
unary), `legacy_in_place_custody.rs:149`, and
`legacy_call_instantiation.rs:845` (rule 5j's `residual_takes_an_address`,
which read every `&` after a `&` as unary).

`legacy_c.rs` `frame_protected_locals` (rule 74d) now calls the same helper.
`ampersand_runs::READERS` lists the 19 files and 28 call sites; a test counts
them in the source.

### Unchanged

* **Imprecise, fail-closed, parity irrelevant.** `thread_entry_census.rs:1764`
  (`address_of` before `name (`: a binary `x & f(y)` makes `f` escape, the
  conservative direction; an odd run before `f (` is not conforming).
  `lexical_nested_assignment.rs:239` with rule 71's
  `logical_and_operand.rs:62`: a run of two after an operand end is a logical
  AND, three is an address; other runs stay an address (fail-closed).
* **Fused token streams** (a fuser with `&&` and longest match is maximal
  munch): `condvar_census.rs:455-458` and its readers `:984`, `:1009`,
  `:1155`, `:1700`, `:2414`, `:2600`, `:2812-2815`;
  `condvar_census/stop_paths.rs:421`, `:436`, `:495`, `:696-697`, `:875`,
  `:895`; `legacy_container_publication.rs:254`, `:464` (`fuse`),
  `:546-588`; `owned_out_member.rs:248`, `:304`, `:387` (over `fuse`);
  `thread_entry_census.rs:600`, `:633` (its condition lexer) and `:1666`,
  `:1734`, `:1738`, `:3116`.
* **Exact shapes of an argument or a fixed spelling** (a slice that starts at
  `(` or `,`, so `["&", name]` is exactly one `&`): `legacy_list_iteration.rs:328`,
  `:332`, `:340`, `:499`, `:683`; `legacy_tracked_self_init.rs:424`;
  `aggregate_store.rs:499`, `:765`; `registration_by_id.rs:332`;
  `legacy_member_effect.rs:197`, `:352`, `:443` (keyed by those actuals);
  `legacy_slot_move_borrow.rs:239`, `:262`, `:954`;
  `legacy_callback_route.rs:1363`; `legacy_retained_by.rs:642`, `:1113`;
  `legacy_in_place_custody.rs:461-462`; `legacy_relational_post.rs:347`;
  `variadic_tail.rs:101`; `legacy_token_shapes.rs:483`, `:535`;
  `legacy_embedded_container.rs:97`; `legacy_member_custody_attribution.rs:132`;
  `legacy_descriptor_cells.rs:364`, `:367`, `:2039`;
  `legacy_call_argument_access.rs:455`, `:463`, `:469`, `:519`, `:783`;
  `legacy_sub_object_store.rs:271`; `readonly_actual_frame.rs:74`;
  `loop_owner_premises_instance.rs:367-387`, `:569`, `:789`, `:1125-1131`;
  `loop_owner_coro_plugin.rs:1526`; `legacy_conditional_embedded_storage.rs:139-334`;
  `thread_entry_census.rs:2694`, `:2739`; `loop_owner_property_keys.rs:947`.
* **Connective detectors** (any run of two or more contains a `&&`; the
  splitters consume the first pair and read the rest as an operand):
  `legacy_condition_acquisition.rs:129`; `legacy_return_family.rs:323`,
  `:418-422`; `legacy_condition_call.rs:34`; `legacy_value_connective.rs:110`;
  `legacy_retention_guard.rs:539-540`; `legacy_member_release.rs:899`, `:909`,
  `:929`; `legacy_condition_leaf_reads.rs:219`; `legacy_condition_desugar.rs:325`,
  `:797`; `call_exits.rs:293-294` (`&&label`); `loop_owner_holder_opaque.rs:136`.
* **Refuse on any `&`** (fail-closed by design): `legacy_classify.rs:1661`;
  `borrowed_actual.rs:297`, `:385`, `:554`; `variadic_nested.rs:379`, `:586`;
  `index_traversal.rs:142`; `neutral_position.rs:441`;
  `typed_field_access.rs:2981`; `legacy_in_place_custody.rs:814`;
  `legacy_relational_forwarding.rs:237` (a declaration search).
* **Compound-assignment heads and binary-operator tables** (`& =`, `a & b`):
  `legacy_c.rs:1137`; `legacy_descriptor_custody.rs:158`, `:370`;
  `legacy_reference_result.rs:190`; `legacy_chained_assignment.rs:159`;
  `indexed_collection.rs:895`; `aggregate_store.rs:344`, `:1021`;
  `loop_owner_holder_opaque.rs:117`; `loop_owner_instance.rs:1795`;
  `legacy_list_iteration.rs:730`; `typed_field_access.rs:1776`, `:2630`,
  `:2640`, `:2881`; `legacy_retention_guard.rs:623`;
  `loop_owner_coro_plugin.rs:340`; `thread_entry_census.rs:2793`;
  `borrowed_actual.rs:430`, `:607`; `legacy_member_custody_attribution.rs:168`;
  `legacy_refused_operands.rs:304`; `legacy_descriptor_cells.rs:1926`, `:1996`;
  `lexical_nested_assignment.rs:62`; `publication.rs:454`;
  `variadic_tail_actuals.rs:164`; `typed_source_nodes.rs:125-128`.
* **Place strings, not token streams**: `legacy_pointer_store.rs:98`,
  `bound_untracked.rs:263`, `legacy_slot_move_borrow.rs:565`,
  `storage_duration.rs:600`, `legacy_link_member.rs:180`,
  `list_length_bound.rs:870`, `legacy_descriptor_cells.rs:274`.
* **Typed AST `opcode` readers** (Clang's parse): every other gateway match,
  in `legacy_atomic_route`, `index_extent`, `loop_measure`,
  `constructed_state`, `statement_expression`, `legacy_tracked_self_init`,
  `member_store_retention`, `call_instantiation`, `callback_member_closure`,
  `va_list_protocol`, `elaboration`, `legacy_elaboration`, `null_deref/*`,
  `list_length_bound`, `void_view`, `value_connective`,
  `typed_access_positions`, `thread_local_read`, `arm_release_check`,
  `c_numeric_obligations`, `legacy_value_connective:574`, `nested_call_effects`,
  `typed_dispatch_census`, `typed_access_path`, `member_path_reading`,
  `typed_source`, `collector_identity`, `indirect_calls`, `handoff_branch`,
  `storage_duration:463/706`, `loop_lifetime_source`, `typed_position_shapes`,
  `custody_free_copy`, `c_numeric`, `cast_semantics`.
* **Other crates**: all correct. AST readers
  (`gateway-rules/rule-36-exact-formal-return/src/source.rs:264`,
  `quirin-audit-plugin-openssl/src/checker.rs:255`,
  `quirin-source-analysis/src/libevent/monkey_binding.rs:363/385`,
  `qcore-verification` `event_payload.rs`, `memory_close_composition.rs`,
  `cycle18_callback_obligations.rs`); maximal-munch lexers
  (`qcore-frontend/src/lexer.rs:422/444`, `qcore-audit/src/c_identity.rs:475`,
  `quirin-audit-plugin-libcurl/src/source.rs:36`,
  `quirin-audit-plugin-openssl/src/checker.rs:71/77`); a compound head
  (`rule-35-literal-decay/src/literal_read.rs:115`); shell metacharacters
  (`compile_db.rs:469`, `openssl/src/shell.rs:81`); URL bytes
  (`std_http.rs:675`); own grammars (`result_set.rs`, `cleanup_lowering.rs`,
  `bounds_quant.rs`, `premise_formula.rs`, `driver.rs`); place strings
  (`projection.rs:311`, `null_fold.rs:189`).

## 2. Changes

`crates/qcore-c-gateway/src/ampersand_runs.rs` (new): `run_before`,
`ends_in_address` (odd), `ends_in_logical_and` (even, at least two),
`unary_address_runs` (rule 74d's, moved from `legacy_c.rs`), `READERS`,
`DECISION_VECTORS`. Each changed site calls it in place of its neighbour test,
with a `Rule 74e` comment. No other logic changed.

## 3. Tests and pins

* `crates/qcore-c-gateway/tests/ampersand_run_parity.rs` (6 fixtures, through
  rule 10a's frame): `p && &q` taken; `p && q` not taken (and `Safe` without
  the refused statement); `p & q` keeps the binary reading (the frame drops the
  name, harmlessly); `&&&q` taken; `&(q)` taken; `a & &q` rejected by Clang
  (the excluded spelling never reaches a reader). These pass at the base too:
  the frame was already parity (rule 74d).
* The eight loop-owner fixtures and two controls above, in
  `crates/quirin-source-analysis/tests/` beside the harnesses that build a
  Fluent Bit shape (the brief named `crates/qcore-c-gateway/tests`; the
  loop-owner census is only reachable with those harnesses, and copying them
  would duplicate several hundred lines).
* Unit tests: `ampersand_runs` (vectors, run agreement, the `READERS` count),
  `loop_owner_instance::address_taken`, `loop_owner_holder_opaque::flow_of`,
  `legacy_call_instantiation::residual_takes_an_address`.

**Pins:** one, deliberate:
`docs/reports/libc-reading-site-evidence/bindings.json` pins the sha256 of
`typed_field_access.rs` as LIBC-READING-SITE-EVIDENCE's declaration-reader
artifact; the file changed (one site, `is_written_at`), so the digest is
re-pinned (`01c9867d8`, anchor `fn scalar_pointer_result` unchanged, as
`faf337fa7` and `748c82224` did; JSON takes no comment, so the reason is in
the commit message and here). No verdict expectation moved: every other
pinned test in the gate passes unchanged.

## 4. Measurement

Opt-in `current --monkey-linked` over Fluent Bit 046accf6, each run on its
own `git archive` tree and release binary ([evidence/runs.md](evidence/runs.md)).

| run | Safe / Pending / Unknown / Unsafe | access rows / alone / leaves |
|---|---|---|
| base `9193b196a` | 1,171 / 2 / 3,623 / 8 | 2,118 / 215 / 8,992 |
| **tip `96218cc75`** (later commits change a pin, docs and the report only) | **1,172 / 2 / 3,622 / 8** | 2,118 / 215 / 8,992 |

**One status move, Unknown → Safe; no Safe row lost**
([moved-rows-base-tip.jsonl](evidence/moved-rows-base-tip.jsonl)):

* `rdkafka_telemetry_encode.c` `get_member_id` (librdkafka 2.15.0,
  `src/rdkafka_telemetry_encode.c:668`): `return rk->rk_cgrp &&
  rk->rk_cgrp->rkcg_member_id && … ? (const char *) …->str : NULL;`. Its
  only base leaf, `unmodeled call, aggregate or indirect access in return
  expression` (line 20722 of the unit), came from rule 5j's
  `residual_takes_an_address` reading the second `&` of each `&&` as a unary
  address. The body takes no address; it rests on no declaration.

Three more rows change leaves only, all Unknown both sides
([changed-rows-base-tip.txt](evidence/changed-rows-base-tip.txt)), each an
`unresolved resource identity` leaf gone because a name after `&&` is no
longer read as address-taken: `cmt_cat.c` `summary_label_keys_match`
(`left_head != &left->label_keys && right_head != &right->label_keys`),
`ctr_decode_opentelemetry.c` `convert_otel_attrs` (`n_attributes > 0 &&
otel_attr == NULL`) and `ctr_span_set_links` (`n_links > 0 && links ==
NULL`).

**Against the prediction.** As predicted: no move from the unsound or
admitting sites (the loop-owner and thread-entry derivations do not feed the
opt-in route's rows), and leaf-only changes in a handful of rows from the
fail-closed sites. Not predicted by name: the one Unknown → Safe, which the
prediction allowed for ("possibly a few Unknown→Safe where it was the row's
sole obstacle") but its `measure-prediction` block (`none`) does not list.

**Pinned routes** (`quirin-audit measure --declared`) cannot run on this host
(no `build/cycle19` inputs or baselines). For the integrator:

* corpus routes: `get_member_id` (librdkafka, if the route reads
  `rdkafka_telemetry_encode.c`) may move Unknown → Safe, and leaf-only
  changes as above; any other move is unpredicted;
* system and tail-pause slice routes: these derive the loop-owner and
  thread-entry premises the unsound fixes touch. A premise that held only
  because `p && &x` (or a caller after `x && f(…)`) was misread would now be
  withdrawn (Safe → Unknown); none is expected, and any such move is the fix
  working and should be shown with its address in the source.

## 5. Gate

Scoped card gate `python3 scripts/gate_scope.py --base 9193b196a --out
private/arp/gate-3` through `private/toolchain/env.sh` (Clang 21, Lean 4.35),
head `01c9867d8` (later commits change only this report and its evidence).
Record: [gate/gate-3.json](gate/gate-3.json); logs in the worktree's
`private/arp/gate-3/`.

| step | exit | result |
|---|---:|---|
| submodules | 0 | 1 submodule |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| `run_test_binaries.py` | 0 | 679 units, 724 summaries; 9,938 passed, 0 failed, 21 ignored, 148 filtered |
| `reevaluation_probe` / `reevaluation_followup` | 0 / 0 | |
| `check_lean_audits.py` | 0 | 487 audits |
| `python3 -m unittest discover -s scripts/tests` | 0 | 807 tests |

Discarded attempts: `gate-1` (head `d154c4d70`) was stopped when the
`legacy_call_instantiation` site was added; `gate-2` (head `96218cc75`) exited
1 with 4 failures, all `reading-site: artifact reference mismatch:
crates/qcore-c-gateway/src/typed_field_access.rs` (the pin above), every other
step 0. No provider file changed, so the provider gate was not selected.

## 6. Ledger and follow-ups

**AMPERSAND-RUN-PARITY-OTHER-READERS is retired.** Its DISCHARGE clause asked
to route address-taking through the run-parity reading, audit each site and
pin `p && &q`: every address decision of the unfused token stream now reads
`ampersand_runs` (28 call sites, counted), every `&` match was audited (§1),
and `p && &q` is pinned through the frame and through each fixed reader.

Registered in [integration-amendments.json](integration-amendments.json):

* **CAST-BEFORE-AMPERSAND-READ-AS-BINARY**: `legacy_refused_operands`
  decides a single `&` by the preceding token (`ends_operand`), so `) &`
  reads as a binary AND and a cast `(T *) &x->f` hands on `x->f`'s value
  rather than the address chain (`legacy_call_instantiation` checks
  `is_cast_close`; this reader does not). Out of this card's scope (the brief
  kept each reader's single-`&` reading); it is the unsound direction.

**Files likely to conflict.**

* `crates/qcore-c-gateway/src/lib.rs` (module list),
  `legacy_c.rs` (`unary_address_runs` now delegates), and the 18 changed
  reader files listed in §1, one line or a few each.
* `crates/quirin-source-analysis/tests/loop_owner_{typed_route,holder_region,coro_plugin}.rs`
  (appended tests).
* `docs/reports/libc-reading-site-evidence/bindings.json` (the digest; any
  other card touching `typed_field_access.rs` re-pins it too).
* `docs/agents/rules/74d.md` (one appended line), `74e.md` (new).
* Generated: `docs/agents/gateway.md`, `docs/agents/README.md`,
  `proof/QCore.lean`; rerun their generators on conflict.

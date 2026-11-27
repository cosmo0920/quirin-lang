# FB-CHUNK-LIFECYCLE: a static function's constant-fixed formal, and the chunk-lifecycle census

Card FB-CHUNK-LIFECYCLE, base `76c1f9cfc` (r77). P1.1 of the
[Fluent Bit system plan](../../plans/fluent-bit-system-coverage/README.md)
(FB-BACKLOG-CUSTODY, remaining multi-output and wrapper arms), starting from
P0-SB-REMOVE-CHUNK-DESTROY-FLAG. New gateway rule
[90](../../agents/gateway.md#constraint-90), Lean
`QCore.StaticConstantFormal`. [PREDICTION.md](PREDICTION.md) was committed
before the semantic change.

## The row and the decision

`sb_remove_chunk_from_segregated_backlog(target_chunk, backlog, destroy)`
(`plugins/in_storage_backlog/sb.c:214-246`) unlinks the matching wrapper
(`mk_list_del`, declared `resource_unpublished`) and frees it only
`if (destroy)`. Over an unconstrained `destroy` the false arm exits with the
unlinked wrapper live, and the system row was a provisional `Unsafe`
(`live obligation acquired at sb.c:226`). The function is `static`, its
address is never taken, and its one call
(`sb_remove_chunk_from_segregated_backlogs`, `sb.c:258`) passes `FLB_TRUE`,
which preprocesses to `1`.

Of the two readings the follow-up offered, the card takes the caller-scoped
one, which fail-closed settles without a new contract clause:

* A `destroy`-dependent declared effect was rejected. The wrapper is not an
  argument or the result, so "handed back to the caller" has nothing to name
  in the v1 vocabulary. On the false arm nothing holds the unlinked wrapper,
  so a declaration saying otherwise would hide a leak.
* Rule 90 reads the static function at its callers' actuals. Internal
  linkage (C11 6.2.2p3) and a census of every occurrence of the identifier
  (definition, file-scope prototype or direct call, nothing else) make the
  unit's direct calls every invocation of the body. If each of them passes a
  decimal constant of one truth at an integer formal that the body spells
  only as `if (NAME)` / `if (!NAME)`, each branch on it keeps the arm C takes.
  The row carries the premise `static-call-sites-closed`, so its scope is in
  its assumptions. The false arm is not discharged: under a zero actual it is
  the arm kept and its leak stands, and with mixed or variable actuals
  nothing is selected.

Rule text, conditions (1)-(6) and the refusals:
[rule 90](../../agents/rules/90.md); implementation
`crates/qcore-c-gateway/src/legacy_static_constant_formal.rs`, one hook in
`build_function_cfg_with` (`legacy_c.rs`), so W4 derivation, rule 5l's
checked-specification settlement, the row and certificate rederivation all
read one graph. Lean: `every_call_agrees`,
`selected_arm_is_taken_at_every_call`, `selection_is_cs_choice`,
`selection_only_removes_edges`, the five refusal theorems,
`a_zero_actual_keeps_the_zero_arm` and `a_read_constant_fits`. The Rust to
model correspondence is by review (trust boundary 28).

## Probe and pins

`crates/qcore-c-gateway/tests/clang_gap_phase4/static_constant_formal.rs`. At base (commit
`ca098c30b`), `static void subject(int destroy) { void *p = q_alloc(); if
(destroy) q_free(p); }` is `Unsafe` under every caller set. At tip:

| Shape | Base | Tip |
|---|---|---|
| every call `subject(1)` (one call, two calls, two callers, `((1))`, `1` and `2`, beside a prototype, under `if`/`else`, cast to `void`) | Unsafe | **Safe** |
| `if (!keep) q_free(p)`, every call `subject(0)` | Unsafe | **Safe** |
| `if (destroy)` with `subject(0)`; `if (!keep)` with `subject(1)` | Unsafe | Unsafe (the kept arm is the leaking arm) |
| `1` and `0` mixed; a variable; `fp = subject` / `&subject`; a designator in a table; no caller; `10000`, `01`, `-1`, `1u`, `(int) 1`; a block-scope declaration; external linkage; `volatile int` / `char` formal; `destroy && opaque()`; a written or shadowed formal; `while (destroy)` | Unsafe | not Safe (unchanged) |

The premise is listed in the selected row's outcome and absent otherwise.
Unit tests pin the constant and formal readers. The probe was committed as
its own target, `tests/static_constant_formal.rs`. It is now a module of
`clang_gap_phase4.rs`, so the suite gains no test target. With one more
target, `test_gate_scope_census`' 90% coverage threshold over the frozen
read census fails, 703 not greater than 703.8.

**Existing pins the rule reaches** (first gate run, 3 failures; all pass
after the edits below):

* `clang_gap_phase4::k11_an_exit_specific_disposition_has_no_summary`: its
  helper `rel(p, c)` released `if (c)`, and its one call passed `1`. That is
  rule 90's shape, so the caller became Safe. The no-summary control now
  passes the caller's own variable, and the constant form is pinned Safe
  beside it.
* `out_parameter_scalar_frame_premise::the_other_refusals_of_rule_11r6_stand`:
  the constant `k` actual is now the caller's variable, so both classes are
  read as the shape intends. Verdicts and leaves are unchanged; the only
  difference had been the premise in the list.
* `out_slot_call_write_aliasing::the_chunkio_file_content_copy_shape_refuses`:
  the real-shape pin (`set_down` passed as `1`) now expects the premise on
  both rows. Its refusals are unchanged.

## Measurement

Base `76c1f9cfc` and tip `3570e45f5`, each from its own `git archive` tree
and its own release `quirin-audit` (sha256 `3384bcef…a85efa494b6b9` and
`f4d91834…fc444f6`), over the prepared Fluent Bit 046accf6 inputs. Scripts
in [scripts/](scripts/). `opt-in-current.sh`, `totals.py`, `moved.py` and
`changed_rows.py` are RETURN-FAMILY-UNTRACKED-MEMBER-CAST's, reused verbatim.
`run-system.sh` is the same run with `--selection
contract/profiles/fluent-bit/system-selection.json` and no Monkey database.

| Route | Base | Tip | Moves |
|---|---|---|---|
| system slice (448 rows, 15 units) | 114 / 0 / 329 / 5; units 0 / 12 / 3 | **114 / 0 / 330 / 4**; units 0 / 13 / 2 | `sb.c::sb_remove_chunk_from_segregated_backlog` Unsafe -> Unknown; unit `sb.c` Unsafe -> Unknown |
| opt-in `current`, `--monkey-linked` (4,804 rows, 181 units) | 1,205 / 2 / 3,590 / 7 | 1,205 / 2 / 3,590 / 7 | none |

Function rows are Safe / Pending / Unknown / provisional Unsafe; unit rows
are Safe / Unknown / provisional Unsafe.

These are exactly the predicted figures. The system row loses one leaf,
`live obligation acquired at sb.c:226`. It keeps the indirect access in its
size-accounting block (`sb.c:228-238`) and `declared specification ... is not
established`. On the system slice, ten `sb.c` rows carry the premise
`static-call-sites-closed`: the function and every row whose derivation
reaches it. No other leaf changes on either route.

On `current`, the selection also fires in 22 librdkafka units, on
header-level static helpers that every in-unit call passes one constant;
the candidate census read only `.c` files. 497 rows there now list the
premise: 485 Unknown and 12 that were already Safe at base. That adds one
assumption to those 12 Safe rows' verdict contract and moves no verdict and
no leaf. No row moves Unknown -> Safe, so no Safe move needs justification.

The four measurement runs were started directly, not through
`scripts/gate_slot.py` as the brief asks for audit-sized jobs. That is a
process lapse, recorded here, and it does not affect the figures: each run
has its own tree, binary and output directory, and every run exited 0.

Pinned routes (`quirin-audit measure --declared`) cannot run on this host:
there are no `build/cycle19` inputs or baselines. That route was not run,
and nothing about it is claimed here.

## Chunk-lifecycle census (step 1)

Script `scripts/lifecycle_census.py` over both system-slice reports
(artifacts `census-base.json.gz`, `census-tip.json.gz`): every row of
`plugins/in_storage_backlog/sb.c` and of `src/flb_input_chunk.c` (two
configurations, identical rows). The slice does not cover the other
backlog wrappers (the output queue's own task wrappers live in
`flb_task.c`/`flb_output.c`, P1.3's routes).

`sb.c` at tip: unit Unknown (base Unsafe), 6 Safe / 14 Unknown rows (base
6 / 13 / 1 Unsafe). The Safe rows are `sb_allocate_chunk`,
`sb_destroy_chunk`, `sb_find_segregated_backlog_by_output_plugin_instance`,
`sb_get_context`, `sb_get_releasable_output_queue_space` and
`sb_is_rejected_stream`. Unknown rows, their leaves and the owner:

| Row | Leaves at tip | Owner |
|---|---|---|
| `sb_remove_chunk_from_segregated_backlog` | indirect access in the size-accounting block (`sb.c:228-238`); its specification not established | FB-BACKLOG-ACCOUNTING (OB-8); W45-FB-BACKLOG-PUBLICATION-BLOCKED (amended) |
| `sb_remove_chunk_from_segregated_backlogs` | uncontracted `sb_remove_chunk_from_segregated_backlog` (its specification and W4 summary need the body to report nothing); its own specification | the row above; W45-FB-BACKLOG-REACHABLE-RELEASE |
| `sb_destroy_backlog`, `sb_destroy_backlogs` | uncontracted callee (the cascade); specification | W45-FB-BACKLOG-REACHABLE-RELEASE, W45-FB-BACKLOG-EXIT-WRAPPERS |
| `sb_release_output_queue_space` | uncontracted `sb_remove_chunk_from_segregated_backlogs`; indirect access; unresolved `underlying_chunk`; specification | FB-AUDIT-03-CROSS-OUTPUT-EVICTION, W45-FB-BACKLOG-REACHABLE-RELEASE |
| `sb_append_chunk_to_segregated_backlog` | indirect access (the saturation test); specification | W45-FB-BACKLOG-PUBLICATION-BLOCKED |
| `sb_append_chunk_to_segregated_backlogs` | 3 indirect accesses; uncontracted `cio_chunk_get_real_size`, `flb_input_chunk_get_tag` | W45-FB-BACKLOG-CIO-RETURN-CODES; the -3 arm FB-AUDIT-03-PARTIAL-FANOUT / W45-FB-BACKLOG-REFERENCE-VOCABULARY |
| `sb_segregate_chunks` | unresolved `chunk` (5), `context` (4); indirect accesses | W45-FB-BACKLOG-CIO-RETURN-CODES, W45-FB-BACKLOG-REFERENCE-VOCABULARY |
| `sb_allocate_backlogs` | loop re-acquisition; unresolved `backlog`, `context`; uncontracted `sb_destroy_backlogs`; specification | W45-FB-BACKLOG-REACHABLE-RELEASE |
| `cb_queue_chunks` | 30 leaves: uncontracted `sb_remove_chunk_from_segregated_backlog` (5), indirect accesses, branch conditions, ChunkIO calls | W45-FB-BACKLOG-MAP-ARMS, W45-FB-BACKLOG-CIO-RETURN-CODES |
| `cb_sb_init`, `cb_sb_exit`, `cb_sb_pause`, `cb_sb_resume` | declared loop-owner obligations (rule 10p), indirect accesses, unresolved `ctx` | P3.5 (per-unit routes refuse 10p callees) |

`flb_input_chunk.c`: 16 Safe / 48 Unknown, unchanged by this card.
`flb_input_chunk_map` (90 leaves) is W45-FB-BACKLOG-MAP-ARMS; the
`*_output_size_*`, `release_route`, `release_space*`, `set_limits` and
`update_output_instances` rows are FB-BACKLOG-ACCOUNTING; the
`flb_input_chunk_destroy*` rows (uncontracted `flb_hash_table_del_ptr`,
unresolved `ic`) are W45-FB-BACKLOG-REACHABLE-RELEASE.

**The next slice.** Every remaining `sb.c` custody row now rests on the
accounting reads (`backlog->ins->fs_backlog_chunks_size`), on ChunkIO
return codes, or on the reference relation. The accounting reads are the
parallel card FB-BACKLOG-ACCOUNTING's subject (rule 92), and the other two
need vocabulary (a C09 relation clause, return-code dispositions over
ChunkIO). None is a bounded gateway slice this card could take without
entering another card's subject or adding a clause, so the card stops at
the census. The acceptance clauses stand as they were: the partial fan-out
(-3) arm, premature close and double ownership are not decided anywhere on
this route. Their rows are Unknown, never Safe, which is the required
fail-closed answer.

## Follow-ups

* **P0-SB-REMOVE-CHUNK-DESTROY-FLAG**: retired. The row is decided under
  its callers' actuals: the leak on `destroy == 0` is gone, because no
  invocation supplies it. The row is Unknown on the accounting access and
  the withdrawn specification. It is not Safe, and nothing is presented
  upstream.
* **W45-FB-BACKLOG-PUBLICATION-BLOCKED**: amended. The same counter read now
  also holds back `sb_remove_chunk_from_segregated_backlog`'s specification,
  and through it the whole removal cascade.
* Rule 90's own limits (no loop test, no compound condition, no computed or
  macro-keyword actual such as C23 `true`, no external linkage) are fail-closed
  refusals stated in the rule. They are not open work.

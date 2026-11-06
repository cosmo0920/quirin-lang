# MEMBER-EFFECT-SUMMARY

Round 20, 2026-11-05. **USER DECISION** (verbatim), answering
MEMBER-ACCESS-PROJECTION's stop
([README §3](../member-access-projection/README.md#3-stop-member-custody-through-a-formal-parent-user-choice)):
"Option 1 (rec.)": a member-effect summary, with option 3's re-attribution
done first as the verdict-neutral step. Base `855b34ade` (origin/main).
Predictions: [PREDICTION.md](PREDICTION.md) (revision 1 before step 1,
revision 2 before step 2).

## 1. Step 1: rule 38i, the refusal names its premise

[Rule 38i](../../agents/rules/38i.md),
`crates/qcore-c-gateway/src/legacy_member_custody_attribution.rs`. Rule 5k is
unchanged. Asked last for a statement (after rule 38d), only for exactly the
gate's refusal `indirect memory access is outside the resource projection` of
a statement with no braces that reads no tracked descriptor, whose base `P` is
a formal no statement of the body writes or takes the address of
(`CallerFormals`):

| premise (census) | statement | name |
|---|---|---|
| member release with the parent kept | `R(.., P->F, ..)`, `Released` at that position (contract in force, no hold), the reading with every actual borrowed admits | `member-release-through-formal` |
| an embedded member's address released | `R(.., &P->F[.G…], ..)`, the same | `member-address-release-through-formal` |
| custody-carrying store into a member | `P->F = V`, `V` a name the lexical reader declares a pointer, or a whole call whose actuals read no memory and whose contract returns `Owned`/`Reallocated` | `member-store-through-formal` |

The node keeps its writes and stays a non-held `Unknown`: no verdict moves
(`QCore.MemberCustodyAttribution`: `the_verdict_of_every_node_is_unchanged`,
`the_status_of_a_function_is_unchanged`, `a_name_needs_every_premise` and
fourteen more). Fixtures in `tests/member_effect_summary.rs` keep a local
parent, a rebound formal, a value that reads memory, a chain below the member
and an uncontracted callee (rule 12k's name) unnamed. A formal declared as a
pointer to a `union` is never named either (the census's `form:union`, rule
10w's ground), which keeps `union_bitfield_places` and `clang_gap_phase2`
unchanged.

**Pins flipped deliberately** (the first card gate run, 16 failures, each a
text change with the status unchanged, or rule 38j's admission):
`access_path_position_residuals` (`c_seize(&q->lock)`),
`access_path_statements` (`s->p = q`), `address_of_member`
(`a_release(&hold->lock)`), `cell_protocol_close_actual` (`close(s->aux)`),
`cell_protocol_fb_declarations` and `cell_protocol_per_end_guards`
(`mk_event_loop_destroy(thi->evl)`), `cell_protocol_release_window_frames`
(`flb_task_queue_destroy(ins->singleplex_queue)`),
`indirect_access_sole_residuals` (`q_free(a->name)`), `indirect_scalar_access`
(`r->next = s`), `sub_object_stores` (`h->name = x_dup(s)`),
`typed_field_access` (`d->pane = n`), the unit `a_non_null_release_keeps_the_refusal`:
all still `Unknown` under rule 38i's name. Rule 38j flips the unit
`a_body_that_keeps_its_formal_gets_no_member_release`: `node_clear` is now
`Safe` with a member effect, and its caller, whose actual is a tracked local,
is refused `member-effect-call-refused`. The probe-state field table names
the new parser field.

**Re-attribution table on the user's `current` run (046accf6)**, estimated
from MEMBER-ACCESS-PROJECTION's census
([`scripts/estimate_step1.py`](scripts/estimate_step1.py); the census has no
route answers, so the direction of the error is stated in PREDICTION.md):

| premise | sites | functions | sole-blocker functions |
|---|---:|---:|---:|
| `member-release-through-formal` | 72 | 53 | 4 |
| `member-store-through-formal` | 160 | 119 | 11 |
| `member-address-release-through-formal` | 1 | 1 | 0 |
| total | 233 | 164 | 15 |

## 2. Step 2: rule 38j, the member-release half of the summary

[Rule 38j](../../agents/rules/38j.md),
`crates/qcore-c-gateway/src/legacy_member_effect.rs`, schema
`crates/qcore-schemas/src/member_effect.rs`.

**Body check** (`BodyReading::read`). Per formal `B`, over a body with no
`goto`/`setjmp`: release events (`B->F` whole at a `Released { nullable:
true }` position) and call events (`B` or `&B->F` whole at a position whose
member-effect summary is in force: the cascade). Rule 5k's P1 (every other
occurrence of `B` is `B->G`, a null test of `B` or `R(B)` releasing its only
parameter), P2 (one event per place; other occurrences null tests before it
or null-constant stores after it), P3 (no loop around it); each call event's
callee characterised by rule 5k's W45-SR5 reading and, for a whole-base
actual, touching no other place the body has an event on. With an `R(B)`,
P4 discharges the effect inside the body and nothing is published. Without
one the body keeps `B` and states each released path `stale`, or `cleared`
when a null store follows the release in its block on every path. A
`nonnull` release is never an event (nothing establishes an untracked member
non-null); a declared function states no effect (P-B08-SHALLOW).

**Where the summary lives.** A W4 summary is a `ResourceFunctionContract`
read by every consumer: direct calls, callback candidates, cross-unit
records and imports, rule 5k's readings. An effect function's custody
contract alone (`resource_borrowed(P)`) would hide the release from all of
them, so it never enters that table: `derive_custody_summaries` installs the
validated contract with its effect in `ResourceContracts::member_effects`
(singleton components only), a function's parser installs it under the
reserved minted name `qcore_member_effect__F` (escaped from any source
spelling, as rule 12t's retained contracts are), and only
`Parser::member_effect_call` classifies a whole-statement call under it, for
a `(callee, line)` this body's own check admitted. Every other call is
refused `member-effect-call-refused`; every other reader sees an
uncontracted function, as on the base. The certificate replay re-derives the
table with the summaries (its `summaries` list carries no effect function,
by the same withholding).

**Lean** `QCore.MemberEffectSummary`: `body_sound` (every path of a checked
body, a sublist of its events since nothing repeats, is free of use after
release and double release of the place), `cascade_sound` (replacing each
call event by any trace of its callee keeps that, given the callee's stated
effect, its characterised touches and its own `body_sound`),
`an_ordinary_reader_sees_no_contract`, and the refusal lemmas; no axioms
beyond `propext`, no placeholders.

**Fixtures** (`tests/member_effect_summary.rs`, every `Safe` result's
certificate rechecked): `cmt_opts_exit`, `metric_release_storage` and
`cmt_metric_exp_hist_snapshot_destroy` shapes are `Safe`; a destructor
calling the helper then releasing the parent (`clear1(x); free(x);`, the
embedded `clear1(&w->in); free(w);`, `obj_destroy(x)`) is `Safe`; a caller
that keeps the parent states the effect in turn (`outer`, then `top`); a
second release or a read of the released member after the call, through the
cascade or the embedded member, the same object at two positions, a callee
reaching a place released before the call, a local parent, a call through a
pointer and a call in a `return` stay non-`Safe`.

### 2.1 New `Safe` rows, per function (predicted, PREDICTION.md revision 2)

- `cmt_map.c::metric_release_storage(struct cmt_metric *metric)`: four
  `free(metric->F)` of distinct pointer members, nullable, top level, then
  four `metric->F = NULL;` in the same block; `metric` used only as
  `metric->F`. Effect: four paths `cleared`. Its callers keep other blockers.
- `cmt_metric.c::cmt_metric_exp_hist_snapshot_destroy`: a null test of the
  base, then per member `if (s->F != NULL) { free(s->F); s->F = NULL; }`.
  Effect: two paths `cleared`.
- `cmt_opts.c::cmt_opts_exit(struct cmt_opts *opts)`: five `if (opts->F) {
  cfl_sds_destroy(opts->F); }` (`resource_released(s, nullable)`). Effect:
  five paths `stale`. Its callers (`cmt_counter_destroy` and siblings) pass
  `&counter->opts` and then `free(counter)`, the embedded P4 shape, but keep
  `cfl_list_del`/`cmt_map_destroy` uncontracted.
- `mk_server/mk_lib.c::free_chunk_header`: `mk_mem_free(input->buffer);
  input->buffer = NULL;` (`mk_mem_free` inherits `free`). Effect: `cleared`.
  It is used as a callback; its summary is withheld, so that use stays
  uncontracted.
- slice `plugins/in_tail/tail_fs_stat.c::flb_tail_fs_stat_remove`:
  `if (file->fs_backend != NULL) { flb_free(file->fs_backend);
  file->fs_backend = NULL; }`. Effect: `cleared`.

The three typedef-spelled bodies (`rd_kafka_mock_sgrp_record_release`,
`rd_kafka_offset_file_close`, `mk_ptr_free`) stay Unknown
(W45-MEMBER-RELEASE-UNTYPED-DECLARATIONS).

## 3. Not done: the store half

`P->F` takes a custody-carrying value (`map->unit = cfl_sds_create(unit)`,
`confval->name = name`) is not implemented; its statements keep step 1's
`member-store-through-formal`. It needs the caller's place map to gain a
field link (CG11's owned-field model) for a tracked parent and the old value
of an untracked member to be established custody-free at the caller, beside
the same withholding. Registered as MEMBER-EFFECT-STORES with that design.

## 4. Stop: the declared form for bodiless callees (user choice)

The brief asks for a declared form of the member effect for bodiless
callees. Two existing vocabularies already speak about a callee's effect on
a formal's members, with different readings:

- grammar G05d, `quant live #P: TAG.PATH |-> #released | #moved | #stale |
  #kept;` (contract spec C04.19.6, rule 30), whose direct call (card
  FB-TEARDOWN-DESTROY-DECLARATIONS) affects only a tracked object the caller
  **linked** at the path; an untracked member "stays" (P-B08-SHALLOW). It is
  declared in force today (`teardown_destroy.qv`, the librdkafka callback
  candidates, `#stale` included);
- the descriptor-cell lower form `resource_member_released(P, T.F, stale)`
  (C04, rule 38), which already owns that clause name.

Rule 38j's reading is stricter: a released member of a formal's object is
never used again by any caller, checked lexically at every call. Fail-closed
does not settle which vocabulary a bodiless callee states it in, because the
existing one is in force with its own reading.

Options:

1. **(recommended)** Make G05d the declared form: a member-path declaration
   with no hand-off whose states are `#stale` or `#released` also enters the
   member-effect table, and its direct call is read through rule 38j's caller
   check wherever the actual is a formal's object (rule 30's linked-object
   composition unchanged otherwise). One vocabulary, the stricter reading at
   untracked members; existing declarations must be re-measured first (a
   direct call that rule 30 read as `stays` may now refuse).
2. A new resource clause (a name other than `resource_member_released`, a new
   grammar id) loaded only into the member-effect table, rule 30 unchanged:
   two vocabularies with two readings of one fact.
3. No declared form: bodiless callees keep G05d's reading or stay
   uncontracted.

## 5. Gate and measurement

Card gate, `python3 scripts/gate_scope.py --base 855b34ade` (selection: the
whole Rust suite, since `qcore-c-gateway` and `qcore-schemas` change; all of
Lean, since a proof root changes; Python; no provider gate):

- Run 1 at `a86dddc1d` ([gate-run1.json](gate-run1.json)): submodules, fmt,
  Clippy, both reevaluation examples and Lean (412 audits) exit 0; Rust suite
  exit 101, 612 units, 9,239 passed, 16 failed (the pins of section 1);
  Python exit 1.
- Run 2 at `4444f818c` ([gate-run2.json](gate-run2.json)): submodules, fmt,
  both examples exit 0; **Rust suite exit 0, 612 units, 9,255 passed, 0
  failed, 16 ignored**; Lean exit 0 (412 audits); Clippy exit 101 (a test
  constant the re-pin left unused in `indirect_scalar_access.rs`); Python
  exit 1, 577 tests, the one failure the stale matrix Lean-audit count
  (`test_build_matrix_sections`: 410 recorded, 412 now, this card's two
  audits), expected until the integrator applies the records.
- After run 2, the unused constant removed (the last commit of the card): `cargo fmt
  --all -- --check` exit 0, `cargo clippy --all-targets -- -D warnings`
  exit 0 (log `private/gate2/clippy-rerun.log`), `indirect_scalar_access`
  24 passed. No code outside that test file changed after run 2.

Measurement (integrator): `quirin-audit measure` against
`baseline:c8c7edf21` with `--declared docs/reports/member-effect-summary/PREDICTION.md`.

## 6. Follow-ups

- **MEMBER-EFFECT-DECLARED-FORM** (user choice, §4).
- **MEMBER-EFFECT-STORES** (§3).
- W45-MEMBER-RELEASE-PREMISES, W44-CALL-RESULT-INDIRECT-DESTINATION,
  MEMBER-CUSTODY-THROUGH-FORMAL-PARENT amended (integration-amendments.json).
- Not attempted: call events in a `return`, condition or nested position, and
  local (tracked) parents, which stay refused.

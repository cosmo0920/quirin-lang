# FRAME-HOLDER-SHADOWED-NAME

Discharges `FRAME-HOLDER-SHADOWED-NAME`, which CG25-IN-PLACE-REGEX registered
(r64, [its report](../../gateway-and-clang/cg25-in-place-regex/README.md) §5). USER DECISION
2026-11-24, "launch them all". Base `3ca85c7a6` (main, r65). Gateway
[rule 74o](../../../../agents/gateway.md#constraint-74o).

## 1. Census

Every caller of `in_place_custody::one_automatic_object` at base:

| Caller | Asks `every_reference_is` at base | Class |
|---|---|---|
| `Parser::automatic_object` (`legacy_in_place_custody.rs`, r64) | yes | guarded; every in-place actual (rule 74i (2)) |
| rule 12t's retention frame holder (`legacy_retained_by.rs`, `holder`, `&NAME` arm) | **no** | frame holder of a retaining or resetting call |
| `frame_registry_holder` (`legacy_callback_route.rs`) | **no** | frame registry of a callback-registry call |

No other caller exists (`rg one_automatic_object crates/`). The other
"automatic object" readers (`aggregate_value::automatic_object`,
`legacy_variadics::automatic_local`, `storage_duration::automatic_ids`) are
keyed by a Clang declaration id, not by a name, so a name that two objects
share cannot reach them.

## 2. Reachability and direction

Both unguarded callers are reachable, and both are fail-open: they produce
a wrong `Safe`.

Each reader runs a token census first (`only_holder_mentions`,
`only_registry_mentions`). It admits a name when every mention is a whole
`&NAME` actual at a holder position, or the one declarator. A file-scope
object `g` that the body names only as `&g` at holder positions passes that
census. A block-scope `struct st g` that shadows it is the function's one
`VarDecl` of the name, so `one_automatic_object` holds too. The file-scope
`&g` was then read as the frame holder `Frame("g")`, one key for two
objects. The probes are in `tests/frame_holder_shadowed_name.rs`, with
file-scope `struct st g;` and `struct list gl;`:

| Probe | Shape | Base | Why base was wrong |
|---|---|---|---|
| F1 | `{ struct st g; r_keep(&g, src); } return r_keep(&g, src);` | **Safe** | A caller's `src` "outlives" the frame, but the second holder is a program-lifetime object |
| F2 | `r_keep(&g, src); { struct st g; return r_keep(&g, src); }` | **Safe** | The same, with the file-scope mention first |
| F3 | `p = r_alloc(8); …; r_keep(&g, p); { struct st g; r_reset(&g); } r_free(p);` | **Safe** | The local's reset dropped the file-scope record, so `p` was released while the file-scope `g` still holds it |
| F4 | `{ struct list gl; c_list_init(&gl, 0, NULL); } c_list_add(&gl, "x"); c_list_destroy(&gl);` | **Safe** | The file-scope registry's destroy was read against the local's registration (no callback), though its callback is one the body never saw |
| F5 | `c_list_add(&gl, "x"); { struct list gl; c_list_init(&gl, 0, NULL); c_list_destroy(&gl); }` | Unknown | Already refused: the add precedes any registration of the key. The direction here is a refusal only |

Under rule 12t's own table the true reading of the file-scope `&g` is
`foreign` for a holder this body cannot decide. A `global` holder of a
non-static object is undecided too. Either way the result is
`retention-undecided`, never `Safe`.

## 3. Changes

* `crates/qcore-c-gateway/src/legacy_retained_by.rs`: `holder`'s `&NAME`
  arm also asks `in_place_custody::every_reference_is`. The change is local
  to that closure.
* `crates/qcore-c-gateway/src/legacy_callback_route.rs`:
  `frame_registry_holder` also asks `every_reference_is`.
* `crates/qcore-c-gateway/src/legacy_in_place_custody.rs`: the doc comment
  of `one_automatic_object` only. It now says that every caller pairs it
  with `every_reference_is`.
* `crates/qcore-c-gateway/tests/frame_holder_shadowed_name.rs`: new.
* `proof/QCore/FrameHolderOneDeclaration.lean`,
  `proof/AuditFrameHolderOneDeclaration.lean`: new. `proof/QCore.lean` was
  regenerated.
* `docs/agents/rules/74o.md`: new. `docs/agents/gateway.md` and
  `docs/agents/README.md` were regenerated.

A name that is not one declaration is no frame holder. A retention there is
`retention-undecided`. A registry there is `callback-registry-undecided`.
A reset there is not read, so it drops nothing, which can only keep a
record longer. A shadowing local whose every reference is its own stays a
frame holder. Only `Safe -> Unknown` is possible.

## 4. Pins

`tests/frame_holder_shadowed_name.rs` was committed before the
implementation (`b8b84208c`). At base it ran 2 passed and 3 failed, and
every failing shape read `Safe`: F1, F2, F3 and F4
(`private/card-logs/pins-at-base.log`). At tip it runs 5 passed:

| Test | Shapes | Base | Tip |
|---|---|---|---|
| `a_frame_holder_that_is_one_declaration_stays_safe` | a non-shadowing frame holder; a shadowing local `g` beside the unnamed file-scope `g`, at function scope and in a block | Safe | Safe (control) |
| `a_file_scope_holder_behind_a_shadowing_local_is_not_a_frame_holder` | F1, F2 | Safe | Unknown, `retention-undecided` |
| `a_shadowing_reset_never_ends_a_file_scope_record` | F3 | Safe | Unknown, `retention-undecided` |
| `a_frame_registry_that_is_one_declaration_stays_safe` | a frame registry `l`; a shadowing `gl` beside the unnamed file-scope `gl` | Safe | Safe (control) |
| `a_file_scope_registry_behind_a_shadowing_local_is_not_a_frame_registry` | F4, F5 | Safe, Unknown | Unknown, `callback-registry-undecided` |

`tests/callee_retention_class1.rs` and `tests/cg22_callback_registry.rs`
pass unchanged, as does the whole suite (§7).

**Lean**, `QCore.FrameHolderOneDeclaration`. This module extends
`QCore.InPlaceCertifiedUse.oneDeclaration` (r64) from in-place objects to
frame holders. A `Spelling` carries the census verdict and Clang's
declaration and reference ids. `frame12t` is the census plus one
declaration, and `frame74o` is the census plus `oneDeclaration`.

* `a_frame_holder_was_one_before`: `frame74o` implies `frame12t`.
* `a_shadowed_name_is_not_a_frame_holder` and `a_shadowed_holder_is_undecided`
  cover a name that has a reference outside its one declaration.
* `the_readings_agree_on_one_declaration`: the control.
* **`no_verdict_becomes_safe`**: a spelled path (`retainAt`, `resetAt`, and
  every other `QCore.RetainedBy` event) that replays `Safe` under rule 74o
  replayed `Safe` under rule 12t. The proof is a simulation: rule 12t's
  records are a subset of rule 74o's (`step_mono`, `lower_mono`,
  `run_lowerAll_mono`).
* `a_registry_key_is_the_old_one_or_refused`, `a_shadowed_registry_is_refused`.
* Witnesses: probes F1 and F3 are `Safe` under `frame12t` and refused under
  `frame74o`. F1's true holder, `global`, refuses too. F4's key was a frame
  key and is refused. The control is `Safe` under both readings.

The audit's `#print axioms` lines list only the standard axioms
(`private/card-logs/lean-audit.log`).

## 5. Measurement against the prediction

The arbiter is opt-in `current --monkey-linked` over Fluent Bit 046accf6.
Each side ran from its own `git archive` tree with its own release binary
([runs](evidence/runs.md)):

| | Safe / Pending / Unknown / Unsafe | moved rows |
|---|---|---|
| base `3ca85c7a6` | 1,171 / 2 / 3,623 / 8 | |
| tip `10770a4db` | 1,171 / 2 / 3,623 / 8 | none |

**The measurement matches the prediction.** No row changed status or leaf
multiset. Access-gate rows and leaves (2,118 / 215 / 8,992) and the
contract-dependency line are unchanged. No measured Fluent Bit or Monkey
function has a frame holder or frame registry whose name a file-scope object
shares in the same body. The fix is a soundness closure with no corpus cost.

**Pinned routes.** `quirin-audit measure --declared` cannot run on this
host, which has no `build/cycle19` inputs and no baselines. It was not run
and no result is claimed. The [prediction](PREDICTION.md) block expects no
move on corpus, system or slice. The integrator measures with base
`rev:3ca85c7a6`.

## 6. Files likely to conflict

* `crates/qcore-c-gateway/src/legacy_retained_by.rs`:
  CALLBACK-MEMBER-RELEASED-EFFECT-ORDER also edits it, in the callback block
  near r60's `callback-beside-release` check. This card's hunk is inside
  `holder`, about 55 lines above that block, at line 650.
* `crates/qcore-c-gateway/src/legacy_in_place_custody.rs`: a doc comment
  only. CG25-IN-PLACE-REGEX-REGERROR-FAILURE-EDGE may edit this file.
* `crates/qcore-c-gateway/src/legacy_callback_route.rs`:
  `frame_registry_holder` only.
* `docs/agents/gateway.md`, `docs/agents/README.md`, `proof/QCore.lean`:
  generated. Rerun `gen_rule_docs.py` and `gen_lean_roots.py`.
* `docs/reviews/r0_3/acceptance_matrix.json` and the increment record are
  written only by `apply_amendments.py`.

## 7. Gate

`sh private/toolchain/env.sh python3 scripts/gate_scope.py --base 3ca85c7a6
--out private/gate-1` ran at `2f8531c19` (the implementation commit
`10770a4db` plus a rustfmt of the pins) and exited 0. The record is
[gate.json](gate.json). The scope was full, because the diff reaches
`qcore-c-gateway`. All Lean was checked, because `proof/QCore.lean` changed.
No provider gate ran, because `providers/` is unchanged. The two measurement
runs (§5) ran at the same time as the gate.

| Step | Exit | Counts |
|---|---|---|
| submodules, fmt, clippy | 0, 0, 0 | |
| Rust suite | 0 | 685 units, 730 summaries; 10,014 passed, 0 failed, 24 ignored, 148 filtered |
| reevaluation probe, follow-up | 0, 0 | |
| Lean (`check_lean_audits.py`) | 0 | 497 audits |
| Python suite | 0 | 807 tests, 0 skipped |

The logs are in `private/gate-1/`. The commits after `2f8531c19` add only
this directory's records. The record checks were rerun after them (§9).

## 8. Follow-ups

* `NAME-KEYED-HOLDER-SHADOWING` (registered, direction not assessed). Rule
  12t's other name-keyed readings are the caller's object ("an unrebound
  formal this body holds no custody of") and the path holder (a member path
  below an unrebound formal that the body names only through member
  accesses off the path, `path_holder`, P-HOLDER-PATH-STABLE). Each is
  decided from the formal's name in the token stream. A block-scope local
  that shadows the formal, named through the same member path, may be read
  as the formal. That is the analogue of this card's shape for formals,
  which `one_automatic_object` and `every_reference_is` do not cover
  (`every_reference_is` reads only `VarDecl`s). Not probed here.

## 9. Record checks after the gate

Run at the records commit:

* `python3 scripts/apply_amendments.py docs/reports/frame-holder-shadowed-name/integration-amendments.json --check`: 0.
* `python3 scripts/gen_rule_docs.py --check`: 0.
* `QUIRIN_CARD_GATE_BASE=3ca85c7a6 python3 -m unittest discover -s scripts/tests`:
  0, 807 tests, 0 skipped (`private/card-logs/python-recheck.log`).

## Increment paragraph

The paragraph `integration-amendments.json` carries for the increment record:

> **FRAME-HOLDER-SHADOWED-NAME** (USER DECISION 2026-11-24, "launch them all"; gateway rule 74o; `QCore.FrameHolderOneDeclaration`; [report](README.md)). Discharges `FRAME-HOLDER-SHADOWED-NAME` (r64), assessed reachable and fail-open. The two callers of `one_automatic_object` without `every_reference_is`, rule 12t's retention frame holder (`legacy_retained_by.rs` `holder`) and the callback registry's `frame_registry_holder` (`legacy_callback_route.rs`), read a file-scope `&g` as the frame holder when a block-scope `g` shadowed it and every mention was a holder actual: a caller's object retained by the file-scope `g` (probes F1, F2), the local's reset dropping the file-scope record before a release (F3) and the file-scope registry destroyed under the local's registration (F4) were each `Safe`. Both readers now also ask `every_reference_is`, so a name that is not one declaration is no frame holder (`retention-undecided`, `callback-registry-undecided`); a shadowing local whose every reference is its own stays one. `Parser::automatic_object` was already guarded, and the other automatic-object readers are keyed by declaration id. Lean extends `InPlaceCertifiedUse.oneDeclaration` to frame holders: rule 74o's reading is a frame only where rule 12t's was, and a spelled path that replays `Safe` under it replayed `Safe` before (`no_verdict_becomes_safe`, by a record-subset simulation); F1, F3 and F4 are `Safe` under rule 12t and refused now. `tests/frame_holder_shadowed_name.rs` (2 passed, 3 failed at base, every failing shape `Safe`; 5 passed at tip). Opt-in `current --monkey-linked` over Fluent Bit 046accf6, each side on its own tree and binary: base 3ca85c7a6 and tip 10770a4db both 1,171/2/3,623/8 with no changed row, as predicted. Scoped gate at 2f8531c19: all 8 steps 0 (10,014 passed, 497 Lean audits, 807 Python tests). Pinned routes are not runnable on the host; no move predicted. Follow-up `NAME-KEYED-HOLDER-SHADOWING` (formal-keyed readings, unassessed).

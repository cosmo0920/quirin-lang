# NAME-KEYED-HOLDER-SHADOWING

Discharges `NAME-KEYED-HOLDER-SHADOWING`, which FRAME-HOLDER-SHADOWED-NAME
registered ([its report](../frame-holder-shadowed-name/README.md) §8). USER
DECISION 2026-11-25, "lanich four of queued cards". Base `43410b230`
(main). Gateway [rule 86](../../agents/gateway.md#constraint-86).

## 1. The readings

Rule 12t decides four readings from a formal's spelling. Each starts from
`CallerFormals::contains`, a token census meaning "an unrebound formal":

| Reading | Where (`legacy_retained_by.rs`) | Rule 12t's reading |
|---|---|---|
| retained actual `NAME` | `retained_spelling`, `Formal` | a caller's object: outlives a frame holder, recorded in a tracked or path holder |
| interior `&NAME->f` | `retained_spelling`, `Interior(.., true)` | a member of a caller's object |
| path holder `NAME->f` | `path_holder_at` | storage the caller supplied, outliving the frame |
| removal | `retained_spelling` and `holder`, keyed on the above | drops the exactly matching record |

The census cannot tell the formal from a block-scope local that shadows it
under the same spelling. The alias branch of `retained_spelling`
(`immutable_holder_alias`) is keyed by Clang's declaration ids, not by the
spelling, so it is not one of these readings. Rule 38's `formal` closure
for descriptor cells and the other `CallerFormals` consumers lie outside
rule 12t (§6).

## 2. Probes and direction

`tests/name_keyed_holder_shadowing.rs`, with synthetic `r_*` and `ev_*`
callees. Committed in two steps before the implementation: `a4bfa1e91`
(5 tests, all passing at base) and `16b0e3b4e` (the jumped-over shapes,
failing at base, `private/card-logs/pins-j-at-base.log`).

| Group | Shapes | Base | Why |
|---|---|---|---|
| S1-S7 (retained) | inner-block array, for-init, pointer, `__auto_type`, typedef'd array, each in a frame or tracked holder | Unknown | the classifier's guard refuses the declaration (`local declaration may shadow a formal resource identity`, `legacy_classify.rs`) |
| M1-M5 (removal, interior) | removal through the shadow, the shadow's record removed through the formal, a shadow or its interior in a tracked loop | Unknown | the same guard |
| H1-H4 (path holder) | path root shadowed in a block, behind an alias, in a for-init | Unknown | the guard, and the path census: the shadow's declarator is a bare mention of the root, so `path_stable` fails |
| U1-U16 | the declaration opens with `__attribute__`, `_Alignas` or `__typeof__`, is parenthesized, or is an enumeration constant | Unknown | the guard does not recognize these declarations. An incidental route refuses each: an uncontracted call (`__attribute__`, `_Alignas`, `ch (src)`), an indirect access, an unmodelled initializer |
| **J1-J9** | a shadow whose declaration a `switch` (before its first label) or a `goto` jumps over | **rejected** | the guard's refusal lies on no path, so it decides nothing. The shadow is read as the formal, and the verdict is `Safe` on every path. Only the internal certificate check stops it: `certificate check failed: function `subject` retains an unknown instruction` |
| J10 | the path root jumped-shadowed | Unknown | the path census |

**Direction.** The jumped shapes reach an admitting reading. In J2-J4 a
local array is read as a caller's object, which "outlives" a frame holder.
In J5 it is recorded in a tracked holder and released with it. In J6 a
removal through the shadow drops the formal's record, so the formal's
registration does not refuse at the exit. In J7 the shadow's record is
removed through the formal, and in J8 an interior of the shadow is held by
a tracked loop. At base every one of these was `Safe` at the route. The
certificate check rejects the unit as an error, so no wrong `Safe` was
published. But the reading itself was fail-open, and the only refusal was
the last line of defence.

## 3. Change (rule 86)

* `crates/qcore-c-gateway/src/legacy_formal_declaration.rs` (new):
  `every_reference_is_the_formal(function, name)`. The name has exactly one
  declaration in the function (counting `VarDecl`, `ParmVarDecl`,
  `FunctionDecl`, `EnumConstantDecl` and `TypedefDecl` of the name at any
  depth), that declaration is a direct `ParmVarDecl` with an id, and every
  `DeclRefExpr` naming the name refers to that id.
  `Parser::one_declaration_formal` is the census plus this check. Without
  Clang's reading of the function, no name is a formal. Four unit tests.
* `legacy_c.rs`: the module line (hook).
* `legacy_retained_by.rs`, three hooks. `retained_spelling`'s `Formal` arm
  and its `Interior` caller flag, and `path_holder_at`'s root check, now ask
  `one_declaration_formal` instead of `caller_formals.contains`.

A formal that fails the check is a plain name (`RetainedSpelling::Name`).
That is a tracked identity, or `retention-undecided`. The `Interior` of
such a name is undecided. A path rooted there is no path holder
(`retention-undecided`). A removal keyed on either is unmatched: it drops
nothing and poisons what it may name. Only `Safe -> Unknown` is possible.
This is the analogue, for formals, of rule 74o's `one_automatic_object`
plus `every_reference_is`. The check is stricter than scope: J1 names the
formal outside the shadow's block and refuses too, because rule 86 reads
declarations, not scopes.

**Lean**, `QCore.NameKeyedHolderShadowing`, which extends
`QCore.InPlaceCertifiedUse.oneDeclaration` and simulates over
`QCore.RetentionPath`:

* `a_formal_was_one_before`, `a_shadowed_name_is_not_a_formal` and
  `a_foreign_reference_is_not_a_formal` cover the reading, with the
  `the_readings_agree_on_one_declaration` control.
* `retained_weakens`, `holder_weakens`, `decision_weakens` and
  `key_weakens`: under rule 86 a retention decides as under rule 12t or is
  undecided, and a removal key keeps its sides or makes them undecided.
* `remove_covers`, `step_covers`, `lower_covers`, `run_lowerAll_covers` and
  **`no_verdict_becomes_safe`**: a spelled path that replays `Safe` under
  rule 86 replayed `Safe` under rule 12t. The proof is a simulation: every
  rule 12t record is covered by a rule 86 record of the same object and
  holder, poisoned at least as much. A weakened removal drops no more and
  poisons no less.
* Witnesses: `probe_j2_was_safe_and_is_refused`,
  `probe_j6_was_safe_and_is_refused`, `a_shadowed_removal_drops_nothing` and
  `the_control_is_safe_under_both`.

The audit (`proof/AuditNameKeyedHolderShadowing.lean`) lists only
`propext`, `Quot.sound` and `Classical.choice`.

Rule text: [`docs/agents/rules/86.md`](../../agents/rules/86.md).
`docs/agents/gateway.md`, `docs/agents/README.md` and `proof/QCore.lean`
were regenerated.

## 4. Pins

| Test | Shapes | Base | Tip |
|---|---|---|---|
| `an_unshadowed_formal_keeps_its_readings` | a formal in a frame and in a tracked holder, a path holder and its removal, a formal in a tracked loop, an interior of a formal, an interior removal | Safe | Safe (control) |
| `a_shadowed_retained_formal_refuses_at_the_guard` | S1-S7 | Unknown | Unknown (guard) |
| `a_shadowed_formal_never_keys_a_removal` | M1-M5 | Unknown | Unknown (guard) |
| `a_shadowed_path_root_is_no_path_holder` | H1-H4 | Unknown | Unknown |
| `an_unguarded_shadow_is_refused_and_never_read_as_the_formal` | U1-U16 | Unknown | Unknown (incidental routes, recorded) |
| `a_jumped_shadow_is_refused_at_the_reading` | J1-J10 | J1-J9 rejected by the certificate check | Unknown, `retention-undecided` (J10 indirect access) |

At base 5 passed and 1 failed. At tip 6 passed
(`private/card-logs/pins-at-tip.log`). `tests/frame_holder_shadowed_name.rs`,
`tests/event_loop_retention.rs` and `tests/callee_retention_class1.rs` pass
unchanged.

## 5. Measurement against the prediction

The arbiter is opt-in `current --monkey-linked` over Fluent Bit 046accf6.
Each side ran from its own `git archive` tree with its own release binary
([runs](evidence/runs.md)):

| | Safe / Pending / Unknown / Unsafe | moved rows |
|---|---|---|
| base `43410b230` | 1,187 / 2 / 3,607 / 8 | |
| tip `182767251` | 1,187 / 2 / 3,607 / 8 | none |

**The measurement matches the [prediction](PREDICTION.md).** No row
changed status or leaf multiset. Access rows and leaves (2,105 / 215 /
8,917) and the contract-dependency line are unchanged. No Unknown→Safe
move needs justifying. No measured Fluent Bit or Monkey function reads a
formal-keyed retention, path holder or removal on a name that a second
declaration in the same body shares, and no unit was rejected at base for
such a shape.

**Pinned routes.** `quirin-audit measure --declared` cannot run on this
host, which has no `build/cycle19` inputs and no baselines. It was not run
and no result is claimed. The prediction expects no move. The integrator
measures with base `rev:43410b230`.

## 6. Files likely to conflict

* `crates/qcore-c-gateway/src/legacy_c.rs`: one module line beside
  `retained_by_route` (`#[path = "legacy_formal_declaration.rs"] mod
  formal_declaration;`).
* `crates/qcore-c-gateway/src/legacy_retained_by.rs`: three one-line hooks,
  in `retained_spelling` (the `Formal` arm and the `Interior` caller flag)
  and in `path_holder_at` (the final root check).
* `docs/agents/gateway.md`, `docs/agents/README.md`, `proof/QCore.lean`:
  generated. Rerun `gen_rule_docs.py` and `gen_lean_roots.py`.
* `docs/reviews/r0_3/acceptance_matrix.json` and the increment record are
  written only by `apply_amendments.py`.

## 7. Gate

`sh private/toolchain/env.sh python3 scripts/gate_scope.py --base 43410b230
--out private/gate` ran at `182767251`, the implementation commit. It exited
1, only because of the six known base failures. The record is
[gate.json](gate.json). The scope was full, because the diff reaches
`qcore-c-gateway`. All Lean was checked, because `proof/QCore.lean` changed.
No provider gate ran, because `providers/` is unchanged.

| Step | Exit | Counts |
|---|---|---|
| submodules, fmt, clippy | 0, 0, 0 | |
| Rust suite | 101 | 697 units, 742 summaries; 10,113 passed, **6 failed**, 24 ignored, 148 filtered |
| reevaluation probe, follow-up | 0, 0 | |
| Lean (`check_lean_audits.py`) | 0 | 509 audits |
| Python suite | 0 | 807 tests, 0 skipped |

The six failures are exactly the K5-LOCAL-ORIGIN-ALIASES (rule 77) pins
known to fail at base and awaiting a user decision:
`scalar_read_identity::pointer_destination_retains_opaque_identity_refusal`,
`detach_lookup_membership::production_lookup_baseline_refuses_the_unresolved_returned_cursor`,
`access_path_positions::an_assignment_operand_is_a_borrowed_read_then_the_null_branch`,
`typed_const_pointer_value::a_void_pointer_bit_cast_is_a_borrowed_read`,
`lexical_closure_nested_assignment::a_never_bound_root_still_certifies`
and `typed_root_declarations::an_assignment_operand_reads_the_ast_declared_local`.
No other test failed. The logs are in `private/gate/`.
`llp64_fixture_scan.py --stdin` over the card's test files exited 0.
The commits after `182767251` add only this directory's records.

## 8. Follow-ups

* `JUMPED-SHADOW-FORMAL-READINGS` (registered, direction not assessed).
  The classifier's shadow guard has two gaps. A declaration that control
  jumps over puts its refusal on no path, and a declaration that opens
  with `__attribute__`, `_Alignas` or `__typeof__`, or is parenthesized,
  is not recognized and is refused only incidentally. Rule 86 closes rule
  12t's readings. The other formal-keyed consumers still decide from the
  spelling: `CallerFormals` in container publication, member custody
  attribution, slot moves, rule 38's descriptor-cell `formal` closure and
  `legacy_classify`, and the W4 entry custody. Those are not probed.

## Increment paragraph

> **NAME-KEYED-HOLDER-SHADOWING** (USER DECISION 2026-11-25, "lanich four of queued cards"; gateway rule 86; `QCore.NameKeyedHolderShadowing`; [report](README.md)). Discharges `NAME-KEYED-HOLDER-SHADOWING` (registered by FRAME-HOLDER-SHADOWED-NAME). Rule 12t's formal-keyed readings (a retained formal or its interior as a caller's object, a path holder below a formal, a removal keyed on either) decided from the token census `CallerFormals::contains`. Probed at each with inner-block, for-init, pointer, array, typedef'd-array and `__auto_type` shadows, every shape refused at base at the classifier's shadow guard (S, M, H). Declarations the guard does not recognize (`__attribute__`, `_Alignas`, `__typeof__`, parenthesized, enumeration constants) refused incidentally (U1-U16). But a shadow whose declaration a `switch` or `goto` jumps over puts the guard on no path: the shadow was read as the formal, the route reached `Safe`, and only the internal certificate check rejected the unit (J1-J9). Each reading now also requires the name to be one declaration, the `ParmVarDecl` every `DeclRefExpr` of the name designates (`legacy_formal_declaration.rs`). A name that fails is a plain name (`retention-undecided`), its path no path holder, and its removal unmatched. Lean: `no_verdict_becomes_safe` by a record-cover simulation over `QCore.RetentionPath`; J2 and J6 are `Safe` under rule 12t's reading and refused now. `tests/name_keyed_holder_shadowing.rs` (5 passed, 1 failed at base; 6 passed at tip). Opt-in `current --monkey-linked` over Fluent Bit 046accf6, each side on its own tree and binary: base 43410b230 and tip 182767251 both 1,187/2/3,607/8 with no changed row, as predicted. Scoped gate at 182767251: every step 0 except the Rust suite (10,113 passed, 6 failed, exactly the six known K5-LOCAL-ORIGIN-ALIASES base failures); 509 Lean audits; 807 Python tests. Pinned routes are not runnable on the host; no move predicted. Follow-up `JUMPED-SHADOW-FORMAL-READINGS` (the guard's two gaps at the other formal-keyed consumers, unassessed).

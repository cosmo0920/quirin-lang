# JUMPED-SHADOW-FORMAL-READINGS

Discharges `JUMPED-SHADOW-FORMAL-READINGS`, which NAME-KEYED-HOLDER-SHADOWING
registered ([its report](../name-keyed-holder-shadowing/README.md) §8).
USER DECISION 2026-11-25, "launch the JUMPED-SHADOW-FORMAL-READINGS card".
Base `4fef267f4` (the NAME-KEYED-HOLDER-SHADOWING tip). Gateway
[rule 86](../../agents/gateway.md#constraint-86), extended; no new rule id.

## 1. The consumers

Rule 86 made rule 12t's formal-keyed readings ask Clang's declaration
reading. Every other consumer of a formal's spelling still read the token
census `CallerFormals` (or, for W4, the spelling itself):

| Consumer | Where | Reading |
|---|---|---|
| container publication | `legacy_container_publication.rs` `container`, `operands`, `settle_foreign_unlinks` | a container rooted at a stable formal, or at a derived local `L = P->F`, is memory the caller supplied (`reaches_caller`, `formal_root`) |
| rule 38's descriptor-cell `formal` closure | `legacy_retained_by.rs` (cell operands), `legacy_descriptor_cells.rs` | a cell root that is a stable formal is the caller's |
| slot moves | `legacy_slot_move_borrow.rs` `slot_operands` | a stable formal passed whole is a caller's slot |
| member custody attribution | `legacy_member_custody_attribution.rs` | names a refusal "member of the formal `P`" (a re-attribution only) |
| counted users | `legacy_counted_users.rs` `stable_formals` | an operation on a stable formal is attributed to it; clauses are decided from those |
| W4 entry custody | the whole route-G state, keyed by name | a formal's spelling is the formal's entry identity throughout the body |
| the classifier's guard | `legacy_classify.rs` | refuses a declaration statement naming a formal, lexically |

## 2. Probes

`tests/jumped_shadow_formal_readings.rs`, committed first as a printed probe
table (`66339083c`, `private/card-logs/probes-at-base.log`), then turned into
pins; the pins were also run on the base tree
(`private/card-logs/pins-at-base.log`: 1 passed, 6 failed). Shapes: J
(a shadow whose declaration a `switch` before its first label, or a `goto`,
jumps over) and U (`__attribute__`, `_Alignas`, `__typeof__`, a
parenthesized declarator, an enumeration constant).

| Consumer | Shapes | Base | Admitting at base | Tip |
|---|---|---|---|---|
| container publication, formal root | CJ1, CJ2, CJ4 | rejected: `certificate check failed: function subject retains an unknown instruction` | **yes** (the route reached `Safe`; only the certificate check stopped it) | Unknown, entry refusal |
| container publication, derived local | CJ5 (jumped), CJ6 (inner block, not jumped) | **`Safe`** | **yes, past the certificate check** (a node published into a local array read as the caller's list) | Unknown (`does not show outlives it`) |
| container publication, other | CJ3 (alias derived), CSJ1, CSU1 (static root) | Unknown (alias, static census) | no | Unknown |
| container publication, U | CU1-CU6, CE1 | Unknown (indirect access, `__attribute__` as a call, braced initializer) | no (incidental) | Unknown, entry refusal |
| descriptor-cell `formal` closure | DCJ1-DCJ3 | rejected by the certificate check | **yes** | Unknown, entry refusal |
| descriptor cells, U | DCU1, DCU2 | Unknown (indirect access) | no (incidental) | Unknown, entry refusal |
| W4 entry custody | WDJ1, WDJ2 (callee row); WJ1, WJ2 (a caller of it) | rejected by the certificate check | **yes** (a release through the shadow read as the formal's) | Unknown, entry refusal; the caller finds no summary |
| W4 entry custody, U | WDU1, WDU2, WU1-WU4, WJ3 | Unknown (alias, guard, uncontracted callee) | no | Unknown |
| slot moves | SMJ1-SMJ4, SMU1-SMU3 | Unknown (slot-mention census: the declarator mentions the slot) | no | Unknown, entry refusal |
| counted users | KJ1-KJ3, KDJ1 | Unknown (the pass refuses every `switch` and jump) | no | Unknown |
| counted users, U | KU1, KU2, KDU1 | Unknown by an indirect access, **clause `Held`** | the clause record read the shadow as the formal; the row was saved by an incidental route | Unknown, operations unattributed, no `Held` clause |
| member custody attribution | MCJ1, MCJ2, MCU1 | Unknown, the leaf naming the shadow "the formal `b`" | no (re-attribution only) | Unknown, gate text and entry refusal |

Controls, `Safe` at base and tip: C0, CD0 (an unshadowed derived local),
CS0, K0 and `inc_body`'s clause `Held`, SM0, SM00, DC0, MC0, W0, WD0.

**Direction.** Three consumers admitted a jumped shadow (container
publication, the descriptor cells, W4 entry custody), each stopped only by
the internal certificate check, which rule 86 already treated as fail-open.
The derived local of container publication was worse: a shadow of `h` (`h =
holder->list`), jumped over or simply in an inner block, was read as the
caller's container and the row was `Safe`, a wrong `Safe` published. No
derived-local name was checked against its declarations at all.

## 3. Change (rule 86 extended)

* `legacy_formal_declaration.rs` (rule 86's module):
  `every_reference_is_the_local` (a local's analogue of
  `every_reference_is_the_formal`: one declaration of the name, a `VarDecl`,
  every `DeclRefExpr` designating it), `shadowed_formals` (the named direct
  parameters that are not one declaration), `one_declaration_formals` (the
  counted-user pass's filter), and two parser hooks:
  `read_formals_by_declaration` before the body is parsed and
  `shadow_refusal` after it.
* `CallerFormals` is read by declaration where it is built
  (`CallerFormals::by_declaration`): a stable formal only where
  `every_reference_is_the_formal`, a derived local only where
  `every_reference_is_the_local` and its root stays stable. `all` and
  `statics` are kept (a name in `all` refuses where it is read). Every
  consumer reads the narrowed set, so container publication, its foreign
  unlinks and `formal_root`, slot moves, member custody attribution, rule
  38's `formal` closure and rule 12t need no hook of their own.
* The counted-user pass's stable formals are read the same.
* **The guard at the entry (gap (a) and (b)).** A function with a formal that
  is not one declaration, in Clang's reading (a second declaration of the
  name anywhere in the body, whatever its spelling, reached or jumped over,
  or a reference to another declaration), gets an `Unknown` instruction at
  its entry, on every path: `local declaration may shadow a formal resource
  identity: `P` is not one declaration`. This is the guard reading Clang's
  declarations instead of token shapes. It is placed at the entry, not at the
  declaration statement in `legacy_classify.rs`, because a refusal at the
  statement is exactly what a jump puts on no path. The lexical guard in
  `legacy_classify.rs` is kept, and now records that it fired.
* **Without Clang's reading** of the function (a unit Clang does not accept,
  as many token fixtures are), no derived local is read, the formals keep the
  census, and the entry refuses when the body may hide a shadow from the
  lexical guard: a `switch` or `goto` where the lexical guard refused a
  declaration, a token opening a declaration the guard does not recognize
  (`__attribute__`, `_Alignas`, `__typeof__`, `typeof`, `__auto_type`,
  `enum`, `typedef`, ...), or a statement opening `T ( P` with `T` a name a
  `typedef` of the unit mentions. The completeness of that list is a stated
  premise. Where the body spells no identifier of the parameter list, no
  reading is asked for at all (every parameter is then one declaration).
  A first version refused every function without Clang's reading;
  it failed 60 token-fixture lib tests and was replaced by this list
  (`private/card-logs/gateway-tests-wip.log`).

Only `Safe -> Unknown`. Rule 86's earlier readings and every earlier refusal
stand.

**Lean**, `QCore.JumpedShadowFormalReadings`, extending
`QCore.NameKeyedHolderShadowing`: `local86`, the narrowed `reaches86` and
`reaches_narrows` (every consumer's keyed reading under the extension was
one under the census), `safe_narrows`, **`no_verdict_becomes_safe`**
(entry guard plus narrowed reading `Safe` implies census `Safe`),
`a_shadowed_formal_refuses_at_entry`, `a_keyed_event_needs_one_declaration`,
and the witnesses `probe_cj1_was_safe_and_is_refused`,
`probe_cj5_was_safe_and_is_refused`, `probe_wdj1_was_safe_and_is_refused`,
`the_controls_are_safe_under_both`. The audit
(`proof/AuditJumpedShadowFormalReadings.lean`) lists only `propext` and
`Quot.sound`.

Rule text: [`docs/agents/rules/86.md`](../../agents/rules/86.md) (new
paragraph and summary). `docs/agents/gateway.md`, `docs/agents/README.md`
and `proof/QCore.lean` were regenerated.

## 4. Pins

`tests/jumped_shadow_formal_readings.rs`, 7 tests, plus 2 unit tests in
`legacy_formal_declaration.rs`:

| Test | Shapes | Base | Tip |
|---|---|---|---|
| `unshadowed_names_keep_their_readings` | the controls | pass | pass |
| `a_shadowed_container_root_is_not_the_callers` | CJ1-CJ6, CU1-CU6, CE1, CJ3, CSJ1, CSU1 | fail (CJ1, CJ2, CJ4 rejected; CJ5, CJ6 `Safe`) | pass |
| `a_shadowed_formal_attributes_no_counted_user` | KJ1-KJ3, KU1, KU2, KDJ1, KDU1 | fail (no entry refusal; KU1, KU2, KDU1 `Held`) | pass |
| `a_shadowed_slot_is_no_callers_slot` | SMJ1-SMJ4, SMU1-SMU3 | fail (refused, but not at the entry) | pass |
| `a_shadowed_cell_root_is_not_the_formal` | DCJ1-DCJ3, DCU1, DCU2 | fail (DCJ rejected) | pass |
| `a_shadowed_formal_has_no_entry_custody` | WDJ1, WDJ2, WDU1, WDU2, MCJ1, MCJ2, MCU1, WJ1-WJ3, WU1-WU4 | fail (WDJ rejected) | pass |
| `without_clangs_reading_an_unguarded_body_refuses` | the token route without Clang's reading | fail | pass |

`legacy_probe_state.rs`'s field-class pin now lists the new parser field
`shadow_guard_refused` as a `MonotoneFact` (only ever set; a discarded probe
that set it leaves a refusal).

## 5. Measurement against the prediction

Opt-in `current --monkey-linked` over Fluent Bit 046accf6, each side from its
own `git archive` tree with its own release binary ([runs](evidence/runs.md)):

| | Safe / Pending / Unknown / Unsafe | access rows / alone / leaves | moved rows |
|---|---|---|---|
| base `4fef267f4` | 1,187 / 2 / 3,607 / 8 | 2,105 / 215 / 8,917 | |
| tip `befb0df69` (and the first tip `243556432`) | 1,187 / 2 / 3,607 / 8 | 2,105 / 215 / 8,917 | none |

Two rows changed leaves only, both `Unknown -> Unknown`:
`lib/librdkafka-2.15.0/src/rdkafka_admin.c` `rd_kafka_CreateAclsResponse_parse`
and `rd_kafka_DeleteAclsResponse_parse`. Each gains the entry refusal for the
formal `errstr`, which a block-scope `char *errstr = NULL;` shadows (the
lexical guard already refused that declaration). The second also gains
`unresolved resource identity `reply``, a consequence of the entry refusal's
unbounded writes. The contract-dependency line is unchanged.

**Against the [prediction](PREDICTION.md):** it predicted 0-10 `Safe ->
Unknown` moves and leaf-only changes on rows already `Unknown` because of a
shadowing declaration, and no `Unknown -> Safe`. Measured: no status move,
and two leaf-only changes of exactly the predicted kind. Inside the
prediction, at its low end. No measured Fluent Bit or Monkey function had a
`Safe` row resting on a shadowed formal or derived local.

**Pinned routes.** `quirin-audit measure --declared` cannot run on this host
(no `build/cycle19` inputs or baselines). It was not run and no result is
claimed. The prediction's block expects no move; the integrator measures with
base `rev:4fef267f4`.

## 6. Files likely to conflict

* `crates/qcore-c-gateway/src/legacy_c.rs`: two hooks around
  `parser.parse_block()` (`// JUMPED-SHADOW-FORMAL-READINGS hook (rule 86)`
  before it, and the entry refusal inserted before `line_order_refusal`
  after it), the `shadow_guard_refused` field in `struct Parser` (after
  `call_graph_components`) and its initializer.
* `crates/qcore-c-gateway/src/legacy_classify.rs`: one line in the lexical
  shadow guard (`if may_shadow_formal`), recording that it fired.
* `crates/qcore-c-gateway/src/legacy_container_publication.rs`: the new
  method `CallerFormals::by_declaration`, before `is_formal`.
* `crates/qcore-c-gateway/src/legacy_counted_users.rs`: the `stable`
  binding in `Unit::evaluate`.
* `crates/qcore-c-gateway/src/legacy_probe_state.rs` and
  `tests/unit/legacy_probe_state.rs`: the field's class, destructuring,
  initializer and the `MonotoneFact` pin.
* `crates/qcore-c-gateway/src/legacy_formal_declaration.rs`: rule 86's own
  module (also changed by any other rule 86 follow-up).
* `docs/agents/rules/86.md`; generated `docs/agents/gateway.md`,
  `docs/agents/README.md`, `proof/QCore.lean` (rerun `gen_rule_docs.py` and
  `gen_lean_roots.py`).
* `docs/reviews/r0_3/acceptance_matrix.json` and the increment record are
  written only by `apply_amendments.py`.

## 7. Gate

`sh private/toolchain/env.sh python3 scripts/gate_scope.py --base 4fef267f4
--out private/gate` ran at `befb0df69`, the last code commit. It exited 1,
only because of the six known base failures. The record is
[gate.json](gate.json). The scope was full (the diff reaches
`qcore-c-gateway`); all Lean was checked (`proof/QCore.lean` changed); no
provider gate ran (`providers/` is unchanged).

| Step | Exit | Counts |
|---|---|---|
| submodules, fmt, clippy | 0, 0, 0 | |
| Rust suite | 101 | 698 units, 743 summaries; 10,122 passed, **6 failed**, 24 ignored, 148 filtered |
| reevaluation probe, follow-up | 0, 0 | |
| Lean (`check_lean_audits.py`) | 0 | 510 audits |
| Python suite | 0 | 807 tests, 0 skipped |

The six failures are exactly the K5-LOCAL-ORIGIN-ALIASES (rule 77) pins
that fail at this base and are re-pinned on main by R34-K-REPAIR:
`scalar_read_identity::pointer_destination_retains_opaque_identity_refusal`,
`detach_lookup_membership::production_lookup_baseline_refuses_the_unresolved_returned_cursor`,
`access_path_positions::an_assignment_operand_is_a_borrowed_read_then_the_null_branch`,
`typed_const_pointer_value::a_void_pointer_bit_cast_is_a_borrowed_read`,
`lexical_closure_nested_assignment::a_never_bound_root_still_certifies`
and `typed_root_declarations::an_assignment_operand_reads_the_ast_declared_local`.
Logs: `private/gate/`. A first gate at `243556432` (`private/gate-243556432/`)
had a seventh failure, `qcore-audit` `configured_cli::one_configured_unit_runs_each_preparation_stage_once`:
the eager Clang reading added a second `-fsyntax-only` invocation for a
parameterless unit. `befb0df69` asks no reading where the body names no
parameter, which restores the pinned count. `llp64_fixture_scan.py --stdin`
over the card's test files exited 0. The commits after `befb0df69` add only
this directory's records.

## 8. Follow-ups

* None registered for a soundness gap this card found and left open. Two
  residues are recorded as premises in rule 86, not as defects: Clang's
  declaration and reference ids (trust boundary 28, as before), and, for a
  function Clang does not read, the completeness of the lexical list.
* Static-root containers read their shadowing from
  `PublicationFacts::destinations` (lexical); the probes (CSJ1, CSU1) refuse
  at base and tip, so nothing is registered.

**Amendments.** [integration-amendments.json](integration-amendments.json)
retires `JUMPED-SHADOW-FORMAL-READINGS`. `apply_amendments.py --check
--profiles-base 4fef267f4` refuses it here because the base card's
registration is not yet applied to the acceptance matrix: `apply_amendments:
refused: ledger[0] retire JUMPED-SHADOW-FORMAL-READINGS: no registered
follow-up has this id`. Apply NAME-KEYED-HOLDER-SHADOWING's amendments first.

## Increment paragraph

> **JUMPED-SHADOW-FORMAL-READINGS** (USER DECISION 2026-11-25, "launch the JUMPED-SHADOW-FORMAL-READINGS card"; gateway rule 86, extended; `QCore.JumpedShadowFormalReadings`; [report](README.md)). Discharges `JUMPED-SHADOW-FORMAL-READINGS` (registered by NAME-KEYED-HOLDER-SHADOWING). Probed every other consumer of a formal's spelling with jumped-over shadows (a `switch` before its first label, a `goto`) and unrecognized declarations (`__attribute__`, `_Alignas`, `__typeof__`, parenthesized, enumeration constants). Container publication rooted at a jumped shadow, rule 38's descriptor-cell `formal` closure and W4 entry custody (a release through the shadow read as the formal's) reached `Safe` that only the internal certificate check stopped (CJ1, CJ2, CJ4, DCJ1-DCJ3, WDJ1, WDJ2, WJ1, WJ2). A derived local of container publication (`h = holder->list`) shadowed by a jumped or inner-block local was read as the caller's container and was `Safe` outright (CJ5, CJ6). The counted-user pass recorded a `Held` clause for an unrecognized shadow (KU1, KU2, KDU1); slot moves and member custody attribution did not admit. `CallerFormals` is now read by declaration where it is built (a stable formal only where its name is one declaration, a derived local only where its own name is one `VarDecl` and its root stays stable), the counted-user pass's stable formals likewise, and a formal that is not one declaration in Clang's reading refuses its function at the entry, on every path; without Clang's reading, no derived local is read and the entry refuses where the body may hide a shadow from the lexical guard. Lean: `no_verdict_becomes_safe` (entry guard and narrowed reading `Safe` imply census `Safe`), with the CJ1, CJ5 and WDJ1 witnesses. `tests/jumped_shadow_formal_readings.rs` (1 passed, 6 failed at base; 7 passed at tip). Opt-in `current --monkey-linked` over Fluent Bit 046accf6, each side on its own tree and binary: base 4fef267f4 and tip befb0df69 both 1,187/2/3,607/8, no status move; two librdkafka rows (`rd_kafka_CreateAclsResponse_parse`, `rd_kafka_DeleteAclsResponse_parse`) gained leaves only, `Unknown -> Unknown`, within the prediction. Scoped gate at befb0df69: every step 0 except the Rust suite (10,122 passed, 6 failed, exactly the six known K5-LOCAL-ORIGIN-ALIASES base failures); 510 Lean audits; 807 Python tests. Pinned routes are not runnable on the host; no move predicted. No follow-up registered.

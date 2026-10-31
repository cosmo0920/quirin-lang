# RULE32A-PILOT: rule 32a's caller-frame hold as a rule package, and what "off" means for a hold

The last pilot of `GATEWAY-RULE-API-PILOT-MIGRATIONS` (pluggable-cards
[plan](../../plans/pluggable-cards/README.md) seam B, phase P2), which also
discharges `GATEWAY-RULE-API-HOLD-OFF`. Base `db2795100`; prediction
`15a76ea84` (committed before any code edit), code `492f0418a`; later commits
change docs, this report and its scripts only. Rule 32a
(`docs/agents/rules/32a.md`, card W48-RK-LOG-CALLER-FRAME) is a restriction:
inside a caller-frame member's own body, a call to a member of the same frame
whose declaration is in force is held `Unknown` under
`caller-frame-self-reliance`, so no member's row reads its own frame's
declaration.

## 1. The decision: a hold is not switchable

Recorded as the plan's [§6 item 10](../../plans/pluggable-cards/README.md#6-decisions)
(a card decision under the hard invariants, not a user decision; §3 B
points to it). The two non-widening options the brief named:

- **(a) fixed** (chosen): registry row `off = "not-switchable"`; a
  `QCORE_GATEWAY_RULES_OFF` list naming `32a` is refused (the analyzer, and
  so `quirin-audit measure --rules-off 32a`, refuses to run); the host asks the
  package under every list, a refused one included.
- **(b) an off that holds more** (rejected): hold every call to a frame
  member, from any caller. It never widens, but it moves no frame member's
  row: every row it moves is a caller *outside* the frame, which rule 32a by
  definition leaves to the declaration. Its "contribution" would be rule
  32's (what the declarations give outside callers, the 7 eight-home rows of
  W48-RK-LOG-CALLER-FRAME) filed under 32a, a mislabelled ledger.
- A refusing off, as every other package's, would remove the hold and the
  logger's own row would read its sibling's declaration and be `Safe` again,
  the halted regression. It widens and was never offered.

Lean `QCore.CallerFrameHoldSwitch` states all three: `a_refusing_off_would_widen`,
`holding_more_changes_no_member_row` and `holding_more_moves_an_outside_row`
(with `holding_more_never_certifies_a_link`), and for the chosen reading
`the_switch_never_removes_the_hold` and `the_switch_never_widens`. This is
rule 34's precedent (a guard whose absence would widen stays unswitchable),
applied to a rule that is all guard.

**Contribution ledger.** Rule 32a is registered, so `known_rule` accepts
`32a`; it has **no switch-measured ledger**, by this decision, and none is
written or transcribed. The measurable quantity behind
`CONTRIBUTION-LEDGER-RULE-32-32A` is rule 32's and needs a rule-32 switch
(amended in that entry, section 6).

**A latent widening closed.** Before this card, `gateway_rules::enabled`
answered `false` for *every* id when the switch list was refused (its
fallback: every switchable rule off). That was harmless while no hold read
the switch; a hold asked through it would have been removed by a refused list.
`enabled` now answers `true` for a `NotSwitchable` id before reading the
list, so a hold is on under every list (`RulesOff::all`, the recorded
`effective_off`, already excluded holds).

## 2. What changed

**Rule API** (`qcore-gateway-rule-api`, additive): re-exports
`qcore_schemas::reason::ReasonCategory`, so a package can reserve its reason
key under its §L21.1 category; the crate docs name `traits::TokenHold` as
used. The existing `TokenHold<H>` trait, `HoldQuery` and `Hold` were enough;
the frame reading is the package's own host capability (as rule 36's
`SourceHost`), so the API gained no hold vocabulary.

**The package** `crates/gateway-rules/rule-32a-caller-frame-hold` (id `32a`,
API dependency only): `hold.rs` holds `SELF_RELIANCE` (the reserved
`ReasonKey`), the host capability `FrameHost: Host` (`frames()`, the
registered member sets; `declaration_in_force(function)`), `CallerFrameHold:
TokenHold<H>` for every `FrameHost`, `within_one_frame`, `decide` (held under
`SELF_RELIANCE` iff the call stays in one frame and the callee's declaration
is in force, otherwise refuse; the admission carries no obligation) and
`DECISION_VECTORS`, whose tests bind it to the Lean `holdVectors`, drive the
package through `consult`, and read the registry row's key, crate and off.

**The host** (`legacy_caller_frame_hold.rs`): `FrameHoldHost` implements
`Host` and `FrameHost` over the unit's contracts and
`qcore_contract::caller_frame::FRAMES`. `hold_call_sites` keeps its
signature and its three callers (the three token pipelines are unchanged);
for each `name (` inside a registered member's body at a call position it
asks the package through `consult` and the registry switch, once per
enclosing member body, and spells the hold as before. Moved out of the host:
`is_frame_member(name)`, the in-force test as a filter and
`within_one_frame(owner, name)`, i.e. the decision. Kept: the member bodies,
`is_call_position`, the held spelling, `hold_description`/`held_call` and
`CALLER_FRAME_SELF_RELIANCE` (now the package key's `&str`).

**Registry**: row `32a` (`crate = "rule-32a-caller-frame-hold"`, placement
`{ position = "token-hold", arm = "caller-frame-hold", route = "hold::decide" }`,
Lean `QCore.RkLogCallerFrame` and `QCore.CallerFrameHoldSwitch`,
`reason_keys = [{ key = "caller-frame-self-reliance", category =
"incomplete-obligation-coverage" }]`, `off = "not-switchable"`), the first
hold and the first reserved reason key; `gateway_rule_registry.rs`
regenerated (it now imports `ReasonKey` and `ReasonCategory`).

**Records**: rule 32a's document (the package, not switchable, the new Lean
module and test); `gateway.md` regenerated; the plan's §6 item 10 and §3 B;
`architecture.md`'s `crates/gateway-rules/` row.

## 3. Tests and Lean

- `crates/qcore-c-gateway/tests/caller_frame_hold.rs` **unchanged**, 7 passed.
- New `tests/rule32a_hold.rs` (5, re-running its own binary with the
  variable set): with nothing off the member is held and the outside caller
  is `Safe`; `32a` and `35,32a` are refused (`cannot be switched off`), the
  fallback's `effective_off` holds 35 and 34 but not 32a, and the member is
  still held; an accepted `33,34,35,36,37` leaves the hold on.
- Gateway unit tests: `rule_32a_is_a_fixed_hold` (row, placement, reserved key
  equal to the package's and the host's, four lists naming 32a refused,
  `enabled("32a")` under an override list, `RulesOff::all` excludes it); the
  registry law test now asserts exactly the hold rows are `NotSwitchable` and
  that the one hold is 32a.
- Package unit tests (6): the decision vectors, the Lean binding, members and
  recursion held under the key, outside callers / undeclared callees /
  different frames refused, the registry row's key, a toy host through
  `consult`.
- Python `test_gateway_registry.py`: the registered hold, its key and its
  refusing-off mutation fail closed; a second hold must be not switchable and
  may not reserve 32a's key.
- Lean `QCore.CallerFrameHoldSwitch` (`proof/AuditCallerFrameHoldSwitch.lean`,
  13 audited theorems, axioms `propext` and `Quot.sound` at most, no
  placeholder): the package's table (`the_hold_vectors_are_the_package`,
  `every_hold_vector_is_the_package_decision`), `holds_iff` (the decision is
  `withinFrame` on a declared callee), `a_held_call_reads_unknown`,
  `an_undeclared_callee_is_not_held`; the switch
  (`a_list_naming_the_hold_is_refused`, `a_hold_is_asked_under_every_list`,
  `the_switch_never_removes_the_hold`, `the_switch_never_widens`); and the
  rejected offs (section 1). `QCore.RkLogCallerFrame` is unchanged and still
  audited.

## 4. Identity run

MEASUREMENT-PENDING

## 5. Gate

GATE-PENDING

## 6. Ledger and follow-ups

- `GATEWAY-RULE-API-HOLD-OFF` **retired**: the decision is in the plan (§6
  item 10) and the switch (fixed: refused when named, on under every list)
  has its Rust, Python and Lean tests.
- `GATEWAY-RULE-API-PILOT-MIGRATIONS` **retired**: every pilot landed as its
  own card with an identity run (35, 37 on their own; 33, 34, 36 through
  RULE36-PILOT's shared run; 32a here), and every switchable pilot has a
  switch-measured ledger (35, 37, 33, 34, 36). Rule 32a has none by the
  HOLD-OFF decision: a fixed hold has no switch to measure, and the entry's
  "switch-measured contribution ledger" clause cannot apply to it without a
  widening or a mislabelled switch. This is the one judgment in the
  retirement; the integrator may instead keep the entry open on that clause.
- `CONTRIBUTION-LEDGER-RULE-32-32A` **amended**: `--rules-off 32a` is refused
  by design, so its DISCHARGE is now rule 32's alone (a card that registers
  rule 32 with a refusing switch over the outside callers' reading of the
  caller-frame declarations, then measures `--rules-off 32` and compares the
  ledger with the recorded 7 eight-home rows).

## 7. Parity cost (plan decision 6)

PARITY-PENDING

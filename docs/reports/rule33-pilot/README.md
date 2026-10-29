# RULE33-PILOT: rule 33 as a rule package

Card RULE33-PILOT, the fourth pilot of `GATEWAY-RULE-API-PILOT-MIGRATIONS`
([plan](../../plans/pluggable-cards/README.md) §3 B and §4;
[interface report](../gateway-rule-api/README.md) §7: "a statement-position
package with caller expansion"). Base `80c8aea98`. Implementation
`a98b6112e`; [prediction](PREDICTION.md) committed before it (`6ab326999`);
rule document and architecture row `dc72dee7e`.

## 1. What moved

Rule 33 ([rule](../../agents/rules/33.md),
[card](../detach-result-wrapper-composition/README.md)) is a pure admission:
it adds a build-stream expansion and a validated frame row, and restricts
nothing the gateway did before it. So the whole rule is switchable; no guard
stays behind.

| Part | Before | After |
|---|---|---|
| Pure decisions (`materialize`, `accept_exit`, `validates`, `selection`, the shared Lean vector tables) | `qcore-c-gateway/src/removed_selection.rs` | moved verbatim into the package (the file is deleted; `list_lookup::lookup` is `children.get(index).copied()`, inlined) |
| Frame word shapes (NULL normalization, `F`'s 29-word body, `G`'s 12-word body) | `legacy_selection.rs` `detach_shape`/`release_shape` | package `detach_body`/`release_body`; the host reads the formals (`signature`) and asks them |
| Caller expansion (statement prefixes, decimal constant 0–255, the constituent calls) and the frame's own body as constituent calls | `legacy_selection.rs` `expand`, `own_body`, `caller_statement` | package, as a `StatementExpansion` answer: the whole rewritten body, each word anchored to the body word whose line it takes |
| Frame row admission (clean, selecting, every exit accepted) | `legacy_selection.rs` `row` calling `validates` | package, as an `EntryRow` answer over the host's observed `ValidationRun` |
| Frame reading against contracts, lookup frame, type model and linkage; the selection entry; `materialize` on the abstract state; the derivation's install | `legacy_selection.rs`, `legacy_summary.rs`, `legacy_transfer.rs` | unchanged in the host (it reads the gateway's vocabulary) |

Files:

- `crates/gateway-rules/rule-33-removed-selection/` (new): the package
  (`RemovedSelection`, `ID = "33"`, `Frame`, `ValidationRun`), the moved
  decisions and their 5 tests (the Lean vector bindings now read
  `proof/QCore/RemovedSelection.lean` from the new crate), and 4 new tests
  (exact frame shapes, caller expansion and its anchors, the package's two
  answers, the dependency set: the rule interface only).
- `crates/qcore-gateway-rule-api/src/statement_expansion.rs` (new, additive):
  `StatementExpansion` (`BodyQuery`, `Expanded`, a reserved prefix and a cheap
  candidate read) and `EntryRow`, neither generic over `Host`: bodies are word
  lists and the frame and run types belong to the package. One test. Nothing
  existing in the API changed; `lib.rs` gains the module and re-export lines.
- `crates/qcore-c-gateway/src/legacy_selection.rs`: the host side. `expand`
  returns at once with rule 33 off, otherwise reads the frames as before and
  asks the package through `consult`; an answer with an anchor outside the
  body is refused. `row` asks the package's `EntryRow`. `enabled()` reads the
  switch. `legacy_summary.rs` installs no frame with rule 33 off.
- Registry row `33` (crate `rule-33-removed-selection`, Lean
  `QCore.RemovedSelection` and `QCore.RemovedSelectionSwitch`, `off =
  "refuses"`), two arms: `statement-rewrite` / `removed-selection-expansion`
  (`selection_route::expand`) and `classification` /
  `removed-selection-entry` (`selection_route::row`). Regenerated
  `gateway_rule_registry.rs`; `legacy_route_table.rs` is unchanged (arms
  have no table). `known_rule` now accepts `33`.
- Workspace member, gateway dependency, `workspace_layering.rs` pin, the
  gateway's registry test (a body lives in the gateway or in its own
  `rule-<id>-*` crate), `tests/rule33_switch.rs`, rule document 33 (and the
  generated `docs/agents/gateway.md`), the architecture row,
  `proof/QCore.lean` (generated).

**Placement note.** No route-map position names the per-CFG-build body
rewrites: ROUTE-MAP §1 lists them (`_Generic`, literal premises, 11r) beside
the token holds, and `token-hold` is a hold, which would force
`not-switchable` (wrong here: refusing only removes an expansion). The
expansion is registered at `statement-rewrite`, its effect (a caller
statement replaced by its constituent calls). The frame row is decided in the
custody derivation, which no position names either; it is registered at
`classification` because the entry's materialization is read at the frame's
own lookup call (the contracted arm, §2.2), as RULE37-PILOT placed its
settlement arm. A body-rewrite position is a candidate for the interface
(section 5).

## 2. What off means

`QCORE_GATEWAY_RULES_OFF=33` asks neither arm. `expand` reads nothing and
rewrites nothing, and the derivation installs no frame, so a frame's row is
the ordinary contract-free analysis and the materialization never runs (it
exists only under an installed frame's entry). That is the code path of the
parent of `c3301ff41` (rule 33's commit) at all three hooks.

- The retained cJSON frames (`tests/rule33_switch.rs`, production `cjson.qv`):
  `cJSON_DetachItemFromArray` is `Unknown` naming the nested
  `get_array_item`, `cJSON_DeleteItemFromArray` is `Unknown` naming the nested
  `cJSON_DetachItemFromArray`, and the decimal-constant caller is `Unknown`
  naming the uncontracted `cJSON_DeleteItemFromArray`: rule 33's report's
  BASE refusals. The caller's CFG no longer calls the detach. Exactly those
  three rows leave `Safe`; no row of the unit is `Safe` only with 33 off.
- **Why nothing stays unswitchable.** With 33 on, a validated frame skips the
  derivation's older routes (`continue`); off, the frame name takes them, as
  before rule 33. An older route could only add a summary for a frame body if
  it admitted the nested call in `return D(P, L(P, (size_t)W));` or `X(F(P,
  W));`, which no route does at BASE (the measured off run, section 3, has no
  row moving toward `Safe`). Were one to do so later, the admission would be
  that route's own audited reading, not a widening by the switch; the Lean
  statement keeps the older routes' soundness as a named premise.

Lean (`QCore.RemovedSelectionSwitch`, 10 theorems,
`AuditRemovedSelectionSwitch.lean`, axioms `propext`/`Quot.sound` only): on
is rule 33's model (`the_switch_on_reads_rule_33`,
`the_switch_on_is_the_expansion`), off installs no frame and expands nothing
(`the_switch_off_installs_no_frame`, `the_switch_off_expands_nothing`), an
admitted row validates (`an_admitted_row_validates`), the off row is the
ordinary analysis, and off never widens a frame row or a caller
(`the_switch_off_never_widens_a_frame_row`,
`the_switch_off_never_widens_a_caller`), with the retained release frame as a
decided vector. Premises kept separate: Rust/model correspondence, the host's
frame reading and run observations, the older routes' soundness.

## 3. Identity run and contribution

MEASUREMENT

## 4. Gate

GATE

## 5. Parity cost (plan decision 6)

COST

## 6. Likely conflicts and follow-ups

CONFLICTS

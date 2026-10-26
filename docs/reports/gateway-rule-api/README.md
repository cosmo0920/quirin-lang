# GATEWAY-RULE-API: the gateway rule interface, registry and rule switches

Card GATEWAY-RULE-API, the first card of phase P2 of the
[pluggable-cards plan](../../plans/pluggable-cards/README.md) (seam B, with the
RULE-SWITCHES mechanism of user decision 3). Base `5813ddab9`. It moves no rule
body and changes no verdict: with no rule switched off, the corpus, system and
slice reports are byte-identical to BASE (section 4).

## 1. What changed

**The API crate** `crates/qcore-gateway-rule-api` (depends on `qcore-schemas`
only; `qcore-contract` is permitted by the plan and by a test that holds the
set, but nothing needs it yet):

- `Position`: one per position of the frozen
  [route map](../gateway-analysis-path-optimization/ROUTE-MAP.md) (token holds
  §1, statement pre-classification, rewrites, fallbacks and attribution §2.1,
  classification §2.2, call holds §2.2/§6, conditions §3, return routes and
  return fallbacks §4, typed-reader arms §7), and one trait per position in
  `traits`. A package is generic over the analysis host (`Host`: its token,
  decision, statement, condition and typed-node types plus three accessors),
  so a rule crate reads the gateway's vocabulary without depending on the
  gateway.
- `Answer`: `Admit(Admission { lowering, obligations })` or `Refuse`;
  obligations are named premises or open obligations under a reserved key.
- `consult`: the route semigroup, unchanged. First answer wins; a disabled
  entry is not asked and counts as a refusal; the end of the table is the
  host's existing refusal.
- `ReasonKey` and `check_reservations`: each reason key is reserved by one
  rule, kebab-case, never with an environment category.
- `RuleEntry`, `Placement`, `Off`, `check_registry`: the registry model and its
  laws (unique plain ids; each placement has exactly one of a consultation
  order or an amended predicate (`arm`); orders `1..=n` per position; a package
  at a hold position is `NotSwitchable`).
- `RulesOff`: `QCORE_GATEWAY_RULES_OFF` parsed against the registry. An unknown
  id, a hold or a malformed element is an error, never ignored.

**The registry** `crates/gateway-rules/registry.toml` (fields `id`, `crate`,
`placements` with `position` and `order` or `arm` and `route`, `lean`,
`document`, `reason_keys`, `off`) describes today's routes: the fifteen
statement fallbacks and eight return fallbacks (fourteen rules, card names
standing where the code names no rule), plus rules 35, 10m and 14c as arms
(below). Every body stays in `qcore-c-gateway`.

**The generated tables.** `scripts/gen_gateway_registry.py` (style of
`gen_lean_roots.py`) validates the registry fail-closed and writes
`crates/qcore-c-gateway/src/gateway_rule_registry.rs` (the `RuleEntry` rows)
and the `generated:registry` region of `legacy_route_table.rs` (the two
tables). `legacy_route_table.rs` is now the registry's reader: each row is a
`qcore_gateway_rule_api` package of its position (`StatementFallback` /
`ReturnFallback` traits over `Parser` as `Host`), the closures the hand-written
tables held are named `ask` functions, and both dispatchers call `consult`.
Tests: `scripts/tests/test_gateway_registry.py` (committed equals generated;
every ordered placement equals the frozen route map's row, rule and route, for
all 23 rows, parsed from `ROUTE-MAP.md` §2.1 rows 7-21 and §4 step 3; broken
registries, holds, missing markers and a moved route-map row fail); the
gateway's `the_tables_are_the_registry_placements` and the kept
`the_fallback_tables_keep_the_documented_order`.

**The switches** (`qcore_c_gateway::gateway_rules`). Each registered rule reads
the list once per process through `enabled(id)`: a table row whose rule is off
is not asked; an arm keeps its reader's old refusal. `check()` returns the
list or its refusal; `quirin-audit` refuses to start under a refused list
(exit 1, the reason on stderr). A library caller that does not stop gets every
switchable rule off, with one stderr line, so the analysis only refuses more.
The run records the list in every report component (`rule_set.off`, the
effective list, which is what `quirin-audit measure` verifies after a run), in
`quirin-audit current`'s and the configured route's `invocation.json`
(`environment.QCORE_GATEWAY_RULES_OFF` and `gateway_rules_off`), in
`run_pinned_audit.py`'s `run.json`, and, as every `QCORE_*` variable already
is, in a checkpoint's run identity. The binary names the variable, which is
the advertisement `measure` checks (`sides::advertises_rule_switch`).

**Arm constants.** `LITERAL_ARM` and `DECAY_ARM` (rule 35), `CONST_VALUE_ARM`
and `VOID_BIT_CAST_ARM` (rule 10m) and `ELEMENT_ARM` (rule 14c) each covered
their whole rule and their `false` was the documented old refusal, so each
became its rule's switch (trivially equivalent with nothing off). Not
replaced, registered for the migration cards: `ADDRESS_ARM` and
`ANONYMOUS_ENUM_ARM` cover two of rule 10n's four parts, so a `10n` switch over
them would claim more than it does; `ENUM_ARM` (card W47-ACCESS-PATH-POSITIONS
step B) is asserted at compile time by a test.

**Also:** `compiler/tests/workspace_layering.rs` admits the new crate and the
gateway's edge to it (a deliberate pin update); architecture rows; the
measure usage row; the stale "no switch exists" comments in
`qcore-schemas::composition::RuleSet` and `qcore-audit::report_components`.

## 2. What "off" means, and its boundaries

Off removes a package's answers and nothing else. It is fail-closed by
construction for every registered rule, with three readings stated rather than
proved:

- **Later rows may answer.** A row switched off falls through, so a later row
  can admit a statement the off row would have decided. Every row's admission
  stands on its own (each is a separately audited rule), so this is no
  Unknown-to-Safe by refactor; but a contribution measured by switching a rule
  off is "the run without that rule", not "the rows that rule decided". In
  particular rule 11i ends the statement table even with a re-worded refusal
  (route-map finding 9); with 11i off, 11u, 11w and 12h are asked for those
  statements.
- **10u can replace an admission.** Its entry condition includes an admitted
  `ContractCall` with an unbound custody-taking position; with 10u off that
  call stays, and the transfer refuses it (`arguments do not match resource
  contract`, `call_instantiation`'s total-or-reject rule), as before 10u.
- **Holds are not switchable.** At a hold position an admission is the hold,
  so a refusal would widen. `check_registry` and the generator require
  `off = "not-switchable"` there and the switch list refuses such an id. No
  hold is registered yet; rule 32a's migration card decides whether a hold gets
  a hold-everything off or stays fixed (`GATEWAY-RULE-API-HOLD-OFF`).

## 3. Tests added

API crate: 15 unit tests (consultation, switches, registry laws, reason keys,
a toy host composing packages, the dependency set). Gateway: `gateway_rules`
(4: registry laws, override, refused lists, every replaced arm on and off),
`route_table` (registry equals tables), and the integration test
`tests/gateway_rule_switches.rs`, which re-runs its own binary with the
variable set: with nothing off rule 35's two fixtures are `Safe`; with `35`
(and ` 35 `) off both keep their old refusals; with `12f,10s` off they stay
`Safe`; with an unknown id the list is refused, every switchable rule is off
and neither fixture is `Safe`. `qcore-audit`: the component rule set records a
given off-list. Python: `test_gateway_registry.py` (8).

## 4. Identity run

IDENTITY_RESULT

## 5. The rules-off proof

RULES_OFF_RESULT

## 6. Gate

GATE_RESULT

## 7. Follow-ups and the pilot migration order

Registered: `GATEWAY-RULE-API-HOLD-OFF`, `GATEWAY-RULE-API-UNREPLACED-ARMS`,
`GATEWAY-RULE-API-PILOT-MIGRATIONS`, `GATEWAY-RULE-API-OTHER-ENTRY-POINTS`;
amended: `MEASURE-TOOL-RULES-OFF-LIVE`, `GAPO-DISPATCHER-COVERAGE`.

Recommended order for the pilot rule cards (the plan lists 32a, 33, 34, 35,
36, 37; the order below is by how much new interface each needs):

1. **35** first: already a registry entry with a live switch, two pure
   decision arms with Lean decision vectors (`QCore.LiteralDecayReadings`);
   it fixes the `ClassificationArm` and `TypedReaderArm` host capabilities.
2. **37**: the typed `sizeof` exemption and the named `asserted_allocation`
   premise, the first use of `Obligation::Premise`.
3. **36**: `ExactFormalReturn` in the derived-result CFG, the first
   `ReturnRoute` package.
4. **33**: the selection frame (`removed_selection.rs` is already pure), a
   statement-position package with caller expansion.
5. **34**: the destroyer split acts in the transfer (`legacy_transfer.rs`
   `ContractCall` arm), which no route-map position covers; it needs a
   transfer position added to the API first.
6. **32a** last: a token hold, which needs the hold-off decision above and the
   `TokenHold` capability; it also reserves the first reason key
   (`caller-frame-self-reliance`).

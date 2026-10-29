# RULE34-PILOT: gateway rule 34 at a new transfer position

The fifth pilot of `GATEWAY-RULE-API-PILOT-MIGRATIONS` (pluggable-cards
[plan](../../plans/pluggable-cards/README.md) seam B and §4, phase P2). Base
`80c8aea98`; measured tip `a78c190dd` (the code; later commits change this
report, its scripts and records, and the ledgers only). Rule 34
(`docs/agents/rules/34.md`, the element destroyer) acted inside the transfer,
which no route-map position covered
([GATEWAY-RULE-API §7](../gateway-rule-api/README.md#7-follow-ups-and-the-pilot-migration-order)).
The card adds that position to the rule interface and then moves rule 34 onto
it, into `crates/gateway-rules/rule-34-element-destroyer`.

## 1. What changed

**The transfer position** (additive, `qcore-gateway-rule-api`):

- `Position::Transfer` (`transfer`, route map §10), last in `Position::ALL`:
  it reads what every other position lowered. Not a hold.
- `transfer.rs` (own file): `DeclaredEffect { unlinks, releases }` (one
  declared container effect beside its parameter's mode), `Phase`
  (`BeforeCall`, `AfterCall`), `TransferQuery { callee, effects }` and
  `TransferArm::ask -> Answer<Vec<Phase>>`. Like RULE37-PILOT's capabilities
  it needs none of the host's token or node vocabulary, so it is not generic
  over `Host`.
- The route map gains §10, an appended section read at `80c8aea98` that names
  the `ContractCall` arm's order (nested effects, the call's own reading,
  container effects, retentions); sections 1-9 are unchanged.
- Order tests: `every_position_follows_the_route_map_order` (Rust: the major
  section never decreases, transfer last, names unique) and
  `test_positions_follow_the_route_map_order` (Python: the generator's
  `POSITIONS` equal `Position::ALL` in order and spelling, and the route map
  has a section for every position's number). The generator accepts
  `transfer`.

**The rule package** `rule-34-element-destroyer` (id `34`; depends on the
API only): `destroyer_order.rs` holds `ElementDestroyer: TransferArm`,
`is_destroyer` (an unlink of a released parameter), `phase`, `order` (admit
the phases when some effect is a destroyer's unlink, else refuse: not rule
34's shape) and `DECISION_VECTORS`. Its tests bind the table to the Lean
`phaseVectors` (read from `proof/`), and drive the arm through `consult`.

**The gateway as host** (`crates/qcore-c-gateway/src/transfer_rules.rs`, own
file): `container_order(called, contract, operands)` builds the query from
the contract, asks the arms through `consult` and the registry switch, reads
an admission fail-closed (one phase per effect, no obligation), and splits
the operands by phase exactly as the deleted
`container_publication::destroyer_split` did (same zip with the operands).
When no arm admits:

- a call with no destroyer's unlink keeps the host's order (every operand
  after the call), which is what `destroyer_split` returned for it;
- a call that still has one is refused by the **host's guard**:
  `transfer_base` reports `element destroyer `NAME`: no transfer rule orders
  its unlink before its release (gateway rule 34 is off)` before any of the
  call's effects apply, marks the arguments' own identities unresolved (as
  the CTWIN-PUBLISH-OUT refusal does, so a release the call was going to make
  is not reported as a leak) and poisons every binding.

The guard is **kept unswitchable in the gateway**, as the task asked for any
off reading that would widen. Without it, 34 off would fall back to the
host's order, release before unlink, which is the reading rule 34 forbids
(`the_release_never_precedes_the_unlink` is its model) and which the Rust host
never modelled for an admitted element. It is the end of the transfer table,
not a package, so `QCORE_GATEWAY_RULES_OFF` cannot remove it: off only removes
rule 34's admission. Before rule 34 no such declaration loaded, so its callers
read `call to uncontracted function`; with 34 off they read `Unknown` again,
under the new leaf.

**Registry**: row `34` (`crate = "rule-34-element-destroyer"`, placement
`{ position = "transfer", arm = "destroyer-order", route =
"destroyer_order::order" }`, Lean `QCore.ElementDestroyer` and
`QCore.ElementDestroyerSwitch`, `off = "refuses"`); `gateway_rule_registry.rs`
regenerated. `known_rule` now accepts `34` (it reads the registry). The
gateway's registry test admits rule packages (`rule-<id>-…`) and binds rule
34's row to the package id and the transfer position; the layering test pins
the package's and the gateway's new edges.

**Lean** `QCore.ElementDestroyerSwitch` (`proof/AuditElementDestroyerSwitch.lean`,
14 audited theorems, no placeholder, axioms `propext` only): the arm's phases
are the Rust table (`the_phase_vectors_are_the_arm`,
`every_phase_vector_is_the_arm_phase`), an admission has one phase per effect,
only a destroyer moves before the call; the host with the switch
(`hostOrder`): on orders every destroyer first, off refuses a destroyer
(`the_switch_off_refuses_a_destroyer`), a call without one ignores the
switch; the caller's reading (`destroyUnder`): on is exactly
`QCore.ElementDestroyer.lower` (`the_switch_on_is_rule_34`), off reads
`Unknown`, `the_switch_off_never_widens` and
`the_switch_off_never_admits_a_refused_destroy`. `gen_lean_roots.py` adds the
module to `proof/QCore.lean`. `QCore.ElementDestroyer` changes only its doc
comment (the implementation pointer).

**Tests**: the nine `element_destroyer.rs` fixtures pass unchanged with every
rule on; three tests are added: every destroying fixture keeps its verdict on,
and a child run under `QCORE_GATEWAY_RULES_OFF=34` reads every destroying
fixture `Unknown` (the guard's leaf wherever the call reaches the transfer;
the plain iteration keeps rule 5l's refusal from its lowering), with no leak
and no double release, while `mk_list_del` + `q_free` with no destroyer stays
`Safe`. Gateway unit tests (`tests/unit/transfer_rules.rs`) check the query,
the old split on, the guard off, and the host order on and off for calls
without a destroyer. The API has one consultation test for the arm.

## 2. Measurement

MEASUREMENT

## 3. Identity (BASE → TIP, every rule on)

IDENTITY

## 4. Rule 34's contribution (TIP with 34 off → TIP)

CONTRIBUTION

## 5. Parity cost (plan decision 6)

| | |
|---|---|
| Decision code moved | 30 gateway lines (`destroyer_split` with its doc) → 78 package lines (`destroyer_order.rs`) + 29 `lib.rs`; the growth is the trait impl, the table and docs |
| New interface | API `transfer.rs` 131 lines (91 without its test), `position.rs` +37 (one variant, its test); route map §10, 18 lines; generator +6 |
| Host | `transfer_rules.rs` 112 lines; `legacy_transfer.rs` +62 −13 (the guard and the arm's use of it) |
| Tests | package 116 lines; gateway unit 135; `element_destroyer.rs` +156 (the switched child); Python +18 |
| Lean | `ElementDestroyerSwitch.lean` 167 lines, audit 18 |
| Whole code diff | 31 files, +1,244 −63 (`80c8aea98..a78c190dd`, docs included) |
| Active time | about 1 h 30 min: reading 35 min, the position and the package 20 min, host, guard and tests 25 min, Lean 10 min |
| Wall clock | WALL |

Friction:

1. **No position, so the API grew first.** The transfer is not a parser
   position: an arm there orders effects of an already-admitted call. A
   vocabulary-free trait fitted; the `Host` vocabulary was not needed.
2. **The off reading needed a guard.** Rule 34 is an ordering, not an
   admission. Its refusal does not mean "older route decides": the older
   order is the one the rule forbids. The guard is host code that recognises
   the shape the package decides (an unlink of a released parameter), so the
   predicate exists twice (package `is_destroyer`, host `needs_an_order`).
   A pure admission (35, 37) needs no such guard.
3. **"Pre-rule-34 behaviour" is a contract fact, not a gateway one.** Before
   rule 34 the loader rejected the declaration; the loader rule
   (`contract_counted_users::validate`) is out of the gateway switch's reach,
   so off reproduces the verdict (`Unknown`), not the old leaf.
4. **Shared files again**: `registry.toml`, generated registry,
   `workspace_layering.rs`, the gateway's registry test, `Cargo.toml`/lock,
   `qcore-c-gateway/Cargo.toml` and `lib.rs`, the API's `lib.rs`,
   `architecture.md`'s row, `proof/QCore.lean`.
5. **Derived matrix count**: the new audit file makes the committed matrix's
   `lean_audits` stale (390 → 391), so the Python suite fails one test
   (`test_the_derived_facts_are_recomputed_rather_than_carried`) until the
   integrator regenerates it. Rule 37's audit file does the same, so after
   both land the count is 392.
6. **Identity needs a checker outside the tool** (BASE is the published
   baseline, TIP a revision build); the two pilots' checkers were reused.

## 6. Gate

GATE

## 7. Boundaries

- The move changes no verdict with every rule on. That is measured on the
  three live profiles, not proved; the Lean model states the arm and switch,
  and the Rust/model correspondence is a premise (trust boundary 28).
- With 34 off, a destroyed element's caller reads `Unknown` under the guard's
  leaf, not under the pre-rule-34 `call to uncontracted function` leaf (the
  loader is outside the switch).
- Contribution is rule 34 switched alone; interactions with 32 and 35 (whose
  rows W48-SCHED-CLEANUP-PRODUCTION separated by attribution) are unmeasured.
- Out of scope: other rules, GATEWAY-RULE-API-HOLD-OFF, contract edits.

## 8. Likely conflicts and follow-ups

Files RULE35-PILOT and RULE37-PILOT also touch (take both sides, rerun the
generators): `crates/gateway-rules/registry.toml` and
`gateway_rule_registry.rs` (`scripts/gen_gateway_registry.py`), `Cargo.toml`
members and `Cargo.lock` (`cargo update -w`), `compiler/tests/workspace_layering.rs`
(every rule crate and every gateway edge), `crates/qcore-c-gateway/Cargo.toml`,
`crates/qcore-c-gateway/src/lib.rs`, the registry test in `gateway_rules.rs`
(all three relax the all-in-gateway assertion), `crates/qcore-gateway-rule-api/src/lib.rs`
(module and re-export lines), `scripts/gen_gateway_registry.py` (rule 35's
crate and arm-route checks; this card's route `destroyer_order::order` is a
`pub fn` of the package's `src/destroyer_order.rs`, so it passes them),
`scripts/tests/test_gateway_registry.py`, `proof/QCore.lean`
(`scripts/gen_lean_roots.py`), `docs/development/architecture.md` (the
`crates/gateway-rules/` row), `docs/agents/gateway.md` (`scripts/gen_rule_docs.py`)
and `docs/development/contributions.md` (regenerated from the ledgers).

Follow-ups ([amendments](integration-amendments.json)):

- Amended `GATEWAY-RULE-API-PILOT-MIGRATIONS`: rule 34 migrated at the new
  transfer position; remaining 36, 33 (running) and 32a.
- Retired `CONTRIBUTION-LEDGER-RULE-34`: its discharge condition is met
  (section 4).
- Appended to `MEASURE-CORPUS-SIDE-CWD-IN-BUILD-IDENTITY`: this run's
  substitution.

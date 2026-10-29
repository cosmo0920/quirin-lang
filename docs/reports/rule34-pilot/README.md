# RULE34-PILOT: gateway rule 34 at a new transfer position

The fifth pilot of `GATEWAY-RULE-API-PILOT-MIGRATIONS` (pluggable-cards
[plan](../../plans/pluggable-cards/README.md) seam B and §4, phase P2). Base
`80c8aea98`; the code is `a78c190dd` (later commits change this report, its
scripts, the ledgers and records only). The card was halted by the user
mid-measurement and its commits were merged onto main `62c8e461c`
([landing](../pilots-r1-landing/integration/README.md#halted-cards-and-tip-validation));
round 2 ([section 2](#2-measurement)) finishes its evidence from there. Rule 34
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

**Before the halt (round 1), the card's own runs**, each with
[`scripts/identity.sh`](scripts/identity.sh) (`--side
base=baseline:80c8aea98`, `--side tip=rev:a78c190dd`, `--rules-off 34`,
`--declared PREDICTION.md`, three live profiles, 24 h quota). Outputs under
`private/rule34-pilot/` of this worktree:

| Attempt | Started | Outcome | Evidence? |
|---|---|---|---|
| `measure-1` | 01:46Z | refused: a mistyped revision (`a78c190dd0`), nothing ran | no |
| `measure-2` | 01:46Z | every route failed at 03:43Z with `No space left on device` (the shared disk reached 100%) | no: it measured the disk |
| `measure-3` | 03:44Z | system and slice complete on `tip` and `tip-off`; the corpus routes were still queued for audit slots when the halt stopped the run (no exit, no record) | partial, below; not the card's identity record |

From `measure-3`'s completed routes, checked by hand against the published
baseline (`private/rule34-pilot/pre-identity-*.json`,
[`scripts/check_identity.sh`](scripts/check_identity.sh)):

- system and slice TIP against `baseline:80c8aea98`: every differing report
  artifact (`resource-diagnostics{,.components,.contracts}.json`, the null
  `*.jsonl`) is equal after normalising the revision and its derived digests
  with lists as multisets, and the differing values form one bijection
  (system 6 values, slice 4, two of them the revision itself; RULE37-PILOT's
  [`substitution.py`](scripts/substitution.py), exit 0). The other report
  artifacts are byte-identical. These are RULE37-PILOT's figures for the same
  baseline. RULE35-PILOT's stricter pairing
  ([`substitution_ids.py`](scripts/substitution_ids.py), extended here to the
  40-hex revision) is inconclusive on these files (exit 1, "list groups stay
  ambiguous under the map"): the revision moves every derived digest, so tied
  list items cannot be paired. That is a limit of the checker, not a
  difference.
- system and slice `tip-off` (34 off) against `tip`: every artifact
  byte-identical except `resource-diagnostics.components.json`, which equals
  TIP once each component's recorded `rule_set.off` (`["34"]`, 30 and 32
  components) is cleared. Rule 34 off moves nothing on either route, as
  predicted (no Monkey unit).

**Round 2 (this evidence round).** Per the integrator's round-2 brief
(`private/integrator-briefs/pilots-r2-evidence.md`), this card runs no
measurement. RULE36-PILOT runs one shared `quirin-audit
measure` for the three halted pilots: `--side base=rev:0d15e7b19` (main with
every change but the 33, 34 and 36 moves) against `--side tip=rev:62c8e461c`
(main with them), `--rules-off 33,34,36 --per-rule`, the combined prediction
(identity), three live profiles, output
`/media/Data/GitHub/quirin-lang/private/pilots-r2-measure/`; its record is
RULE36-PILOT's ([report](../rule36-pilot/README.md)). Rule 34's ledger is
written from that run's `tip-off-34` side.

The run completed (about 2026-10-29T09:32Z to 21:44Z, 12 h 12 min wall,
plugin exit 0, record status `complete`, no failures, built at one
`--build-commit`). Its record is
[`../rule36-pilot/native-comparison.json`](../rule36-pilot/native-comparison.json)
(sha256 `93035654cecf…`), the path RULE36-PILOT and RULE33-PILOT commit it
at; this card commits the same bytes at the same path, because
`contribution_ledgers.py write` stamps the ledgers with a record inside the
tree.

## 3. Identity (BASE → TIP, every rule on)

From the shared run (BASE `rev:0d15e7b19` → TIP `rev:62c8e461c`, which
differ by exactly the 33, 34 and 36 moves): the declared check **matches**
on all three routes (`none`; no unpredicted or missing row, no delta
mismatch).

| Route | Rows | Moved | Content changed | Artifacts |
|---|---:|---:|---:|---|
| system | 448 | 0 | 0 | byte-identical |
| slice | 404 | 0 | 0 | byte-identical |
| corpus | 4,834 | 0 | 0 | equal under a checked one-to-one substitution |

The corpus route is `incomplete` only by the named `composition-conflict`
(MEASURE-CORPUS-SIDE-CWD-IN-BUILD-IDENTITY): every one of its 4,834 rows is
environment-only. The five differing corpus artifacts pass RULE35-PILOT's
strict pairing ([`substitution_ids.py`](scripts/substitution_ids.py), exit 0,
`private/rule34-pilot/r2-corpus-ids.jsonl`): `corpus-null.jsonl`, the
components, contracts, diagnostics and source-effects files are each equal
under a one-to-one token substitution with lists as multisets (180, 302,
175, 302 and 10 tokens substituted). Here both sides share the revision and
the build commit, so the strict pairing concludes, unlike against the
baseline in round 1. RULE36-PILOT's report owns this run's record; the
figures above are read from the committed copy.

## 4. Rule 34's contribution (TIP with 34 off → TIP)

From the same run's `tip-off-34` side (rule 34 switched alone; `--per-rule`
also measured 33 and 36 alone and all three together; **no interaction**:
no row moves only with two or more rules off):

| Profile | 34 off | 34 on | Rows moved |
|---|---|---|---|
| eight-home (corpus) | Safe 1,011 / Unknown 3,813 / Unsafe 10 | 1,013 / 3,811 / 10 | 2 Unknown → Safe |
| fluent-bit-system | 101 / 343 / 4 | same | none |
| fluent-bit-tail-pause | Pending 4 / Safe 57 / Unknown 342 / Unsafe 1 | same | none |

The two rows are exactly W48-SCHED-CLEANUP-PRODUCTION's
[two](../w48-sched-cleanup-production/README.md#measurement):
`sched_thread_cleanup` (`mk_scheduler.c:614`), which with 34 off has the
guard's leaf at `:625`, and `mk_sched_threads_destroy_conn` (`:657`), which
with 34 off has the guard's leaf at `:669` and `unresolved resource identity
mth` at `:668`. Both rest on the eight `mk_http_thread_destroy` premises
(`scheduler_borrowed_chain_live`, `scheduler_callback_footprint`,
`scheduler_callee_live`, `scheduler_exact_parent_embedding`,
`scheduler_no_later_alias_use`, `scheduler_tls_params_owned`,
`scheduler_valgrind_request_neutral`, `scheduler_valid_membership`). Off
gives the guard's leaf where W48 had the pre-rule-34 `call to uncontracted
function` leaf, as section 1 predicts. No row moves toward `Safe` with 34
off, no leaves are traded on unmoved rows, and no touched row stays Unknown.
The tool's post-run off-list check passed.

**Ledgers.** `contribution_ledgers.py write --record
docs/reports/rule36-pilot/native-comparison.json` writes one group of ledgers
per profile for rules 33, 34 and 36. `check` and `consistency` refuse any
part of a group on its own (`the group lists [33, 34, 36], the ledgers are
[34]`), so the whole group is committed:
`contract/contributions/{33,34,36}/{eight-home,fluent-bit-system,fluent-bit-tail-pause}.json`.
The record, all nine ledgers and the regenerated contributions page are
byte-identical to RULE36-PILOT's (`63639c5d6`) and RULE33-PILOT's
(`6dea720e7`), so the three cards merge without conflict. `check` (15
ledgers valid) and `consistency` exit 0.

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
| Wall clock (round 1) | about 6 h 10 min to the halt (09:55 to 16:07 local): code committed after 50 min; the rest was measurement, almost all of it queued for audit slots, and one disk-exhausted attempt |
| Wall clock (round 2) | records, ledger and gate only; the measurement is shared with RULE33/36 and charged to RULE36-PILOT |

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
5. **Derived matrix count**: the new audit file made the committed matrix's
   `lean_audits` stale (390 → 391 on this card's base), so the round-1
   Python suite failed one test
   (`test_the_derived_facts_are_recomputed_rather_than_carried`); the
   integrator regenerated the matrix at the landing (`62c8e461c`).
6. **Identity needs a checker outside the tool** (BASE is the published
   baseline, TIP a revision build); the two pilots' checkers were reused.
   RULE35-PILOT's stricter pairing cannot decide when the whole revision
   moves (ties stay ambiguous), so against a baseline only RULE37-PILOT's
   normalising check concludes.
7. **The measurement is the cost, and it is contended.** Three cards'
   measurements and suites shared the audit slots; this card's corpus routes
   waited more than three hours without admission, and one attempt was lost
   to a full shared disk. One shared `--per-rule` run per round (round 2)
   is the cheaper shape.

## 6. Gate

**Round 1** (no card gate before the halt; checks on `a78c190dd`, logs in
this worktree's `private/rule34-pilot/`):

- Targeted tests passed. Package and API: 22. Gateway lib `transfer_rules`
  and `gateway_rules`: 7. `element_destroyer`: 12. `gateway_rule_switches`
  and `workspace_layering` passed too.
- fmt and workspace Clippy (`--all-targets -D warnings`) exit 0.
- `gate_lean.py` exit 0 (`QCore.ElementDestroyerSwitch`, 1 audit).
- Python: 555 tests, 1 failure, the matrix's derived count (friction 5).

The integrator's full gate on the merged tip `442dfa6f0`/`dc5d07c95`
([landing](../pilots-r1-landing/integration/README.md#halted-cards-and-tip-validation))
covers this card's code: Rust 8,927 passed, 0 failed; Lean 396 audits.

**Round 2: the card gate**, `python3 scripts/gate_scope.py --base 62c8e461c
--out private/gate-r2-b` on `fbfd2a900`, exit 0. It ran in scoped mode
(records only: no crate is affected, no Lean, no provider gate):

- submodules: 1, none moved;
- fmt exit 0;
- Python: 556 tests, 0 skipped, exit 0.

Console log: `private/rule34-pilot/gate-r2-3.log`; record:
[`gate/gate.json`](gate/gate.json). The first round-2 attempt (`gate-r2`, on
`58b717296`) was stopped before it finished, when the record path changed;
its output directory blocked a rerun at the same path (`gate-r2-2.log`, exit
1, `FileExistsError`). The commit after `fbfd2a900` adds only this record and
this section.

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

## 8. Conflicts and follow-ups

The shared-file conflicts with RULE35/36/37/33 (`registry.toml` and its
generated tables, the workspace and gateway manifests and lock, the layering
test, the gateway's registry test, the rule API's `lib.rs` re-exports,
`proof/QCore.lean`, `architecture.md`'s row, `gateway.md`) were resolved by
the integrator at `62c8e461c`, keeping both sides of the API re-exports.
Rule 34's route `destroyer_order::order` is a `pub fn` of the package's
`src/destroyer_order.rs`, so it passes RULE35-PILOT's generator check
unchanged. Round 2 changes `docs/reports/rule34-pilot/`, and adds the shared
record `docs/reports/rule36-pilot/native-comparison.json`, the ledger group
`contract/contributions/{33,34,36}/` and the regenerated
`docs/development/contributions.md`. The last three are byte-identical to
RULE36/33's commits, so they merge cleanly.

Follow-ups ([amendments](integration-amendments.json)):

- Amended `GATEWAY-RULE-API-PILOT-MIGRATIONS`: rule 34 migrated at the new
  transfer position, with its evidence; 32a remains.
- Retired `CONTRIBUTION-LEDGER-RULE-34`: its discharge condition is met
  (section 4).
- Appended to `GATEWAY-RULE-API-ARM-OBLIGATIONS`: the transfer host also
  refuses an admission carrying obligations.

# RULE36-PILOT: rule 36 as its own rule package

Card RULE36-PILOT, the third pilot of GATEWAY-RULE-API-PILOT-MIGRATIONS (plan
[pluggable-cards](../../plans/pluggable-cards/README.md) §3 B and §4;
[GATEWAY-RULE-API](../gateway-rule-api/README.md) §7: rule 36 is the first
`ReturnRoute` package). Base `80c8aea98`; halted on 2026-10-29 and merged
onto main `62c8e461c` by the integrator, then finished in round 2 against that
main (section 6). The rule is unchanged; its body moved and it gained a
switch. [Prediction](PREDICTION.md) committed first (`215bb6b52`).

## 1. What moved

**The package** `crates/gateway-rules/rule-36-exact-formal-return` (depends on
`qcore-gateway-rule-api` only; `compiler/tests/workspace_layering.rs` holds
it, and the gateway's edge to it):

- `source.rs`: the declaration's matching depth-one pointer formals
  (`pointer_formals`, formerly the body of `declared_abi::exact_pointer_formals`),
  the lexical candidate (`candidate`), and the typed source conjunction
  (`exact_formal_returns` with the walks `object_pointer`, `value_name`,
  `references`, `inspect_body`, `original_formal`, `inspect_returns`), generic
  over two host capabilities the package defines: `SourceHost` (total reads of
  a typed node: kind, string attribute, object field, type, children, and the
  host's positive pointer-type test) and `FormalReturnHost` (the typed root
  reader's answered-and-never-bound premise per formal, a node's
  once-read complete span, its line).
- `route.rs`: `ExactFormalReturnRoute`, the first implementation of the API's
  existing `ReturnRoute` trait, over `ExactFormalHost` (whether the host lowers
  the derived-result CFG, the confirmed formal of `(line, name)`, and the
  host's exact-formal lowering). Its decision `route::decide` is the
  registry's route.
- `flow.rs`: the exit classifier `classify` over the binding's facts, and the
  fourteen flow vectors shared with `QCore.ExactFormalReturn.flowVectors`, now
  also as data (`FLOW_VECTORS`).

**The host** `crates/qcore-c-gateway/src/legacy_exact_formal_return.rs` keeps
what reads or writes the gateway's own state: the `FormalSource` host over the
typed `StatementIndex`, the parser's `ExactFormalHost` (cache of confirmed
returns, `Statement::Return` with `LegacyInstruction::ExactFormalReturn`), the
abstract-state adapter of `classify`, `apply` (the refusal's `UnknownAlias`
and exposure), and the stack-control walk of rule 13q. The instruction
variant and every passive consumer of it (CFG builder, derived-summary exit
check, storage duration, descriptor custody, no-return paths, IR store) stay
as they were: with the rule off nothing produces the instruction.

**The switch.** Registry row `36` (`crate = "rule-36-exact-formal-return"`,
`lean = [QCore.ExactFormalReturn, QCore.ExactFormalReturnSwitch]`, one arm
`exact-formal-return` at position `return-route`, route `route::decide`,
`off = "refuses"`), so `RulesOff` and `known_rule` accept `36`. The host asks
the package only through `gateway_rules::enabled("36")`: off, `candidate` is
false (no derived-result CFG is built for the rule and the live-entry route's
neutral summary is not refined), the return route is not asked
(`consult` skips it, the return keeps its pre-rule-36 lowering), and
`classify` refuses (so the transfer adds only the refusal's `UnknownAlias`
and exposure). Each is a removal of an admission.

The route is registered as an arm, not as an ordered row: route map §4
step 2 predates rule 36 and the generator only orders positions that have a
generated table; a one-row `return-route` table would have needed a frozen
route-map row that does not exist. No API change was needed: `Position::
ReturnRoute` and the `ReturnRoute` trait already existed.

## 2. Tests and Lean

- Package (9): the flow vectors against Lean's order and codes; the
  classifier's single-untracked-base case; the source conjunction over a toy
  AST (confirmed formal; a second identity for the name, an unbound premise
  not answered, an undelimited body or an incomplete return span decline; a
  non-pointer returned value and a written formal confirm nothing; a lexical
  return the AST lacks is dropped); casts; shadowing and address-taking;
  `pointer_formals`; `candidate`; the route's decision and its `consult` with
  the switch off.
- Gateway: the existing rule-36 unit tests now drive the package through the
  host (flow vectors through the abstract state, equal to the package's
  `FLOW_VECTORS`; Clang-JSON casts and shadowing; the candidate), plus
  `off_the_classifier_refuses_and_the_transfer_keeps_custody`;
  `gateway_rules`' registry test admits hosted packages and checks rule 36's
  crate and id; `derived_result_returned.rs` (24) unchanged and green.
- `tests/rule36_switch.rs` re-runs its binary with the variable set. Nothing
  off: the one-path shape and the bare identity certify `Derived(Exact)` and
  their callers are `Safe`. `36` off (also ` 36 ` and `35,36`): the one-path
  wrapper keeps exactly the pre-rule-36 leaf the rule-36 commit `0faab87fa`
  removed from `mixed_exits_are_refused` (`unmodeled call, aggregate or
  indirect access in return expression`) and its caller is `Unknown` on the
  uncontracted call; the bare identity keeps the live-entry route's neutral
  summary (`ResourceReturnMode::None`) instead of `Derived(Exact)`. `35,12f`
  off leaves rule 36 certifying.
- Lean: `QCore.ExactFormalReturn` unchanged and still audited
  (`AuditDerivedResultInference.lean`). New `QCore.ExactFormalReturnSwitch`
  with `proof/AuditExactFormalReturnSwitch.lean` (13 theorems, axioms
  `propext` at most, no placeholder): on is exactly the classifier, transfer,
  route and refinement; off refuses, validates no exit, never widens an exit,
  keeps every owed resource, binding and frame while adding the unknown, asks
  no route, and retains the old summary; every switched refinement keeps the
  parameter custody vector. `gen_lean_roots.py` regenerated `proof/QCore.lean`.

## 3. Identity run and contribution

One shared measurement for rules 33, 34 and 36 (brief
`pilots-r2-evidence.md`), owned by this card:
[`scripts/pilots-r2-measure.sh`](scripts/pilots-r2-measure.sh), BASE
`rev:0d15e7b19` (main without the three moves), TIP `rev:62c8e461c` (main with
them), both built by the tool at its fixed path under one build-commit label
(`pilots-r2-identity`), `--profile` for the three live profiles,
`--rules-off 33,34,36 --per-rule`, `--declared` [PREDICTION-r2.md](PREDICTION-r2.md)
(committed first, `1fa2c956e`). Run 2026-10-29T09:32Z to 21:44Z, exit 0,
`/media/Data` 108 GB free at start and 455 GB at the end (no `ENOSPC` in any
route log); output `/media/Data/GitHub/quirin-lang/private/pilots-r2-measure/`,
record committed as [native-comparison.json](native-comparison.json) (status
`complete`, no failure).

**Identity (BASE -> TIP).** The declared check matches on all three routes
(no move predicted, none measured). The tool's comparison: system and slice
complete with 0 moves, 0 content changes, 0 unmatched rows; corpus 0 moves, 0
content changes, 0 unmatched, marked incomplete only for the named
`composition-conflict` of MEASURE-CORPUS-SIDE-CWD-IN-BUILD-IDENTITY.
Checked by hand ([r2-identity/](r2-identity/), script
[`scripts/r2_identity.sh`](scripts/r2_identity.sh)):

- system and slice: every report artifact byte-identical
  (`resource-diagnostics.json`, its components, contracts, source-effects and
  chunk-contracts, the null property, its summary, the outer projection, the
  exit); only console, stderr and the two invocation records differ (paths,
  times, binary digest).
- corpus: `monkey-source.json`, `chunk-contracts` and the null summary and
  outer projection byte-identical; the rest equal under one checked
  one-to-one substitution. RULE35-PILOT's `substitution_ids.py` (exit 0):
  every artifact equal under a bijective token map, lists as multisets
  (`resource-diagnostics.json` 302 of 653 digests substituted, components
  302/654, contracts 175/2,122, source-effects 10/29 with 3 contract ids,
  `corpus-null.jsonl` 180/616). This card's `corpus_substitution.py` (exit 0):
  185 units, 4,834 functions, 180 build identities mapped one to one (one per
  unit with functions, none unchanged), 0 rows moved or changed.

**Contribution (TIP-off -> TIP, one binary).** Rule 36's ledgers
(`contract/contributions/36/{eight-home,fluent-bit-system,fluent-bit-tail-pause}.json`):
**0 rows on every profile**, no leaves traded, no touched row left Unknown,
confirming card W48-BU-CMT-MAP-METRIC-GET's measured gain of 0. The same
record holds rule 33's (eight-home +2 Safe: `cJSON_DetachItemFromArray`,
`cJSON_DeleteItemFromArray`, under `detach_list_representation`) and rule 34's
(eight-home +2 Safe: `sched_thread_cleanup`, `mk_sched_threads_destroy_conn`,
under the eight `scheduler_*` premises of `mk_http_thread_destroy`) ledgers;
system and slice 0 for both. The joint side (`tip-off`, all three off) moves
exactly those four rows, and no row moves only jointly: **no interaction**.
`contribution_ledgers.py write` writes every ledger of a record, so this card
wrote all nine (one record, one group per profile; `check`: 15 ledgers valid;
`consistency`: base Pending 4, Safe 57, Unknown 342, Unsafe 1 plus the
contributions equals the real-run total on eight-home) and regenerated
`docs/development/contributions.md`. The tool's post-run off-list check
passed (every switched side's components record exactly its list);
MEASURE-TOOL-RULES-OFF-LIVE was already retired by an earlier pilot, and this
run is a further real instance with `--per-rule`.

Round 1's attempts (section 6) are superseded and are not evidence here.

## 4. Gate

**Round 2** (base `62c8e461c`): GATER2

**Round 1** (before the halt): `python3 scripts/gate_scope.py --base 80c8aea98 --out private/gate-1` on
`6c3f84e31` (code tip `375ec9326` plus the measurement script; the diff
reaches `qcore-c-gateway`, so the scope is the full Rust suite and every Lean
audit), console log
`/media/Data/GitHub/quirin-lang/private/rule36-pilot/gate-1.log`, logs and
`gate.json` in the worktree's `private/gate-1/` (copied to
[gate-r1.json](gate-r1.json)). Overall exit 1, from the Python suite alone:

| Step | Exit | Counts |
|---|---|---|
| submodules | 0 | 1 submodule at its pin |
| fmt, clippy (`--all-targets -D warnings`) | 0, 0 | |
| Rust suite (`run_test_binaries.py`, rust-suite slot, waited 7,461 s) | 0 | 575 units; 8,862 passed, 0 failed, 15 ignored |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| `check_lean_audits.py` (lean slot) | 0 | 391 audits |
| Python suite | 1 | 554 tests, 1 failure |

The one failure is `test_build_matrix_sections.test_the_derived_facts_are_recomputed_rather_than_carried`:
the committed acceptance matrix records `lean_audits` 390 and the tree now has
391 audit files (this card's `AuditExactFormalReturnSwitch.lean`). The matrix's
derived fields are regenerated by the integrator at landing (`land.py`); a card
edits the matrix only through its amendments, so this is the expected residue
of adding an audit, not a defect (friction 6 below). Earlier targeted runs on
the same code: package 9 passed; `rule36_switch` 5 passed; gateway rule-36,
registry and deep-AST stack tests 19 passed (1 ignored, an external unit);
`derived_result_returned` 24 passed; Python registry tests 8 passed; the new
Lean module built and its audit printed `propext` at most.

## 5. Parity cost (plan decision 6)

Measured on this card (start 2026-10-28T11:40Z):

| Item | Cost |
|---|---|
| Decision code moved out of the gateway | about 280 lines: the classifier (25), the candidate (30), the source conjunction and its six walks (180), the declaration's pointer-formal decision (20), the route condition (15); `legacy_c.rs` lost its 17-line inline route |
| New package | 624 non-test lines (`source.rs` 320, `flow.rs` 189 of which 80 are the shared vector table, `route.rs` 69, `lib.rs` 46) and 9 tests (about 330 test lines) |
| Gateway host | `legacy_exact_formal_return.rs` 305 -> 293 non-test lines: the moved decisions were replaced by an adapter of the same size (`FormalSource` host 70, `ExactFormalHost` 40, the route call 45, the state adapter 25) |
| Switch, tests, records | registry row, hosted-package test, `tests/rule36_switch.rs` (240), Lean switch module (123 + 17 audit lines) |
| Implementation time | about 25 minutes from prediction (11:40Z) to the code commit (`375ec9326`, 12:05Z), plus 30 minutes of tests and Lean; everything after it was gate and measurement queueing |
| Wall clock | dominated by the shared slots: the driver's release build waited about 5 h for an audit slot; the gate's Rust suite waited 2 h 04 min for its slot and ran 20 min; each round-1 measurement took about 5 h, mostly behind other cards' corpus routes, and none survived (section 6); the round-2 shared measurement of six sides (BASE, TIP, all three off, each one off) took 12 h 12 min and serves three cards |

Friction, in the order met:

1. **The API's return query does not fit the first return route.**
   `ReturnQuery::decided` ("what the return routes decided so far") has no
   value before the first route, so the host passes its own refusal
   (`expression_supported: false`, no evaluation), which the route never
   reads; `ReturnQuery::origin` is not optional, while `return_statement` is
   also the lowering of operands the elaborators hand back without a stream
   place, so the host passes `usize::MAX`. Both are unread by rule 36; an
   optional origin and a separate first-route query would remove the
   placeholders (follow-up).
2. **A typed-node reading is a cross-rule capability the API does not have
   yet.** RULE35-PILOT adds `TypedHost` to the API in parallel; this card kept
   `SourceHost` (the same reads plus `node_field`) in its package rather than
   add a conflicting copy to the API. The two become one capability at
   whichever merge lands second (follow-up).
3. **One switch, four host points, one placement.** Rule 36 acts at the
   derived-CFG pre-filter and the summary refinement (`candidate`, two
   callers), the return route and the exit classifier read by two transfer
   functions. Only the route is a route-map position; the registry has no
   position for a summary-inference pre-filter or a transfer classifier, so
   they are documented under the one `return-route` arm and covered by the
   switch tests and `QCore.ExactFormalReturnSwitch`, not by the registry.
4. **The frozen route map has no row for rule 36**, so the route is an arm,
   not an ordered `return-route` table row; the generator orders only the two
   fallback tables.
5. **Baseline reuse needs the landing's route spelling.** The published
   baseline is verified by route digest, and `measure --profile` spells the
   eight-home link map at `contract/evaluations/eight-home/link-map.json`
   while the landing's publication used `inputs/live.json`
   (`docs/reports/w48-bu-single-leaf-bodies/link-map.corpus.json`, the same
   bytes). The plan's §4 step 3 (`--profile ... --side base=baseline:<sha>`)
   is therefore refused against today's baselines; identity-1 ran 5 h before
   the refusal (follow-up).
6. **The Python suite fails on a new audit file until the matrix is
   regenerated**: `test_build_matrix_sections` pins the committed matrix's
   `lean_audits` (390) against the tree (391). The matrix is the integrator's
   record, regenerated at landing, so the card gate cannot be all green for
   any card that adds a Lean audit (RULE37-PILOT will meet the same).
7. The worktree guard refuses compound shell commands whose paths contain the
   repository's parent directory name, so slot-wrapped and measurement
   commands had to be written as scripts (`docs/reports/rule36-pilot/scripts/`
   and the worktree's ignored `private/rule36/`).

For plan decision 6: a self-contained rule whose decisions are pure moves
cheaply (minutes), but the host does not shrink, since adapters replace the
moved code one for one, and the rule's switch reaches host points the registry
cannot name. The cost of a pilot is its evidence (a full gate and two-profile
measurement under shared slots), not its code.

## 6. The halt and round 2

The card was halted on the user's request on 2026-10-29 while its third
measurement attempt waited for corpus slots. Its commits (prediction
`215bb6b52`, code `375ec9326`, scripts, the uncommitted draft of this report as
WIP `a89e765d9`) were merged onto main by the integrator
([landing record](../pilots-r1-landing/integration/README.md), section
"Halted cards and tip validation"), unchanged except for the shared
`gateway_rules.rs` hosted-package list, which now names every package; main's
full gate at `442dfa6f0`/`dc5d07c95` passed with this code. RULE35-PILOT's
`TypedHost` landed in the API beside this package's `SourceHost`.

**Measured before the halt** (round 1, base `80c8aea98`, TIP `375ec9326`, all
superseded by round 2 and kept as attempts under
`/media/Data/GitHub/quirin-lang/private/rule36-pilot/`):

- `identity-1` (`--profile`, `--side base=baseline:80c8aea98`): every TIP
  and TIP-off route exited 0, then the tool refused the published baseline on
  the route digest (friction 5), so no record compared BASE and no ledger was
  built. The tool's own `tip-off -> tip` comparison completed on all three
  routes with 0 moves, 0 content changes, 0 unmatched rows. Checked by hand
  against the baseline: system and slice report artifacts byte-identical
  (only logs and invocation records differ); corpus equal under the one-to-one
  substitution (`scripts/corpus_substitution.py`: 180 build identities
  bijective, 0 rows moved, every artifact equal exactly or with remaining
  derived digests blanked and the same number of distinct digests).
- `identity-2` (`--inputs live.json`, the baseline's spelling): its TIP
  routes completed, its TIP-off corpus route died on `ENOSPC` when
  `/media/Data` filled (12:00-13:00 JST on 2026-10-29), and no record was
  written. A disk-exhausted run measures disk: discarded, together with
  `identity-2b`, a comparison-only run that read its TIP side (it matched the
  declared identity on all three routes).
- `identity-3`: stopped by the halt before its corpus routes ran.

**Round 2** (brief `pilots-r2-evidence.md`): one shared measurement for rules
33, 34 and 36, owned by this card, BASE `rev:0d15e7b19` (main without the three
moves) against TIP `rev:62c8e461c` (main with them), `--rules-off 33,34,36
--per-rule`, declared against the [combined prediction](PREDICTION-r2.md),
committed first (`1fa2c956e`). Section 3 holds its result.

## 7. Follow-ups

Registered by this card's [amendments](integration-amendments.json):

- `RULE36-SOURCEHOST-TYPEDHOST-UNIFY`: the package's `SourceHost` repeats the
  API's `TypedHost` (RULE35-PILOT) plus `node_field`; make one capability.
- `GATEWAY-RULE-API-RETURN-QUERY-FIRST-ROUTE`: `ReturnQuery::origin` is not
  optional and `decided` has no value before the first return route, so the
  host passes placeholders (friction 1).
- `MEASURE-BASELINE-ROUTE-SPELLING`: a published baseline is reusable only by
  a measurement that spells its routes as the publication did (`inputs/live.json`);
  `--profile`, which plan §4 step 3 prescribes, is refused (friction 5).
- `CONTRIBUTION-LEDGER-WRITE-SHARED-RECORD`: `contribution_ledgers.py write`
  writes every ledger of a record and `check_contribution` requires one
  amendments file to list them all, so a measurement shared by several cards
  has one writer; the per-card ledger ownership the round-2 brief assumes is
  not expressible.
- `CARD-GATE-NEW-AUDIT-MATRIX-COUNT`: a card that adds a Lean audit fails the
  Python suite on the committed matrix's derived `lean_audits` until the
  integrator regenerates it (friction 6).

Amended: `GATEWAY-RULE-API-PILOT-MIGRATIONS` (rule 36 migrated, identity run
and ledgers; 32a remains).

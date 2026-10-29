# RULE33-PILOT: rule 33 as a rule package

Card RULE33-PILOT, the fourth pilot of `GATEWAY-RULE-API-PILOT-MIGRATIONS`
([plan](../../plans/pluggable-cards/README.md) §3 B and §4;
[interface report](../gateway-rule-api/README.md) §7: "a statement-position
package with caller expansion"). Base `80c8aea98`. Implementation
`a98b6112e`; [prediction](PREDICTION.md) committed before it (`6ab326999`);
rule document and architecture row `dc72dee7e`. The card was halted on the
user's request while its measurement ran and merged onto main as found
(`16df251ad`, draft report `15e03d523`); the integrator added
`selection_route::{expand,row}`, thin wrappers over the package's two arms,
so the registry routes satisfy RULE35-PILOT's generator check
([landing](../pilots-r1-landing/integration/README.md), "Halted cards and tip
validation"). This round (pilots round 2, base `62c8e461c`) adds the
evidence: section 3 records what ran before the halt and what the shared
measurement adds.

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

**Before the halt (no evidence).** `measure-1`
(`/media/Data/GitHub/quirin-lang/private/rule33-pilot/measure-1/`, script
`private/rule33/measure.sh` of the card's worktree) started at 05:25:56Z:
`--side base=baseline:80c8aea98 --side tip=rev:a98b6112e --rules-off 33
--declared PREDICTION.md`, release host and plugin from `release-build-2`.
It was stopped by the halt at about 07:07Z with the tip and tip-off corpus
routes unfinished; it wrote no report and no comparison. It is kept as an
incomplete attempt and is not used.

**This round: the shared measurement.** RULE36-PILOT ran one `quirin-audit
measure` for the three halted pilots
(`/media/Data/GitHub/quirin-lang/private/pilots-r2-measure/`, 09:32:40Z to
21:44:54Z, exit 0, status `complete`, no failures): `--side
base=rev:0d15e7b19 --side tip=rev:62c8e461c --rules-off 33,34,36 --per-rule
--declared` the combined prediction (RULE36-PILOT's `PREDICTION-r2.md`, which
carries this card's `none` block). Its record is
`docs/reports/rule36-pilot/native-comparison.json` (sha256 `93035654cecf…`);
this card commits a byte-identical copy at that path, since the ledgers name
the record they come from (below).

**Identity (BASE `0d15e7b19` -> TIP `62c8e461c`, the three moves together).**
The declared check is `match` on all three routes (no unpredicted, missing or
mismatched row). Slice: 404 rows identical; system: 448 identical; corpus:
4,834 rows, 0 moved, 0 content changes, 0 unmatched, every row differing only
in the environment fields, totals 1,013 Safe / 3,811 Unknown / 10 Unsafe on
both sides. The corpus comparison is marked `incomplete` for "composition-
conflict in the union of the two sides": both sides were built under one
fixed build commit, so each corpus unit row has two components under one
analyzer revision in the union (180 of the 185 unit rows listed; 365
components in the union). That classification, and
the digest substitution of the differing corpus artifacts, are RULE36-PILOT's
to report (it owns the run and the checkers); this card records the result by
reference and adds no check of its own.

**Rule 33's contribution (`tip-off-33 -> tip`, one binary, rule 33 alone;
interactions measured by `--per-rule`).** Ledgers
`contract/contributions/33/{eight-home,fluent-bit-system,fluent-bit-tail-pause}.json`:

| Profile | Rule 33 off | Rule 33 on | Rows moved |
|---|---|---|---|
| eight-home (corpus) | 1,011 Safe / 3,813 Unknown / 10 Unsafe | 1,013 / 3,811 / 10 | 2 Unknown -> Safe, both under `cJSON_DetachItemViaPointer`'s `detach_list_representation` |
| fluent-bit-system | 101 / 343 / 4 | same | none |
| fluent-bit-tail-pause | 57 / 4 Pending / 342 / 1 | same | none |

The two rows are librdkafka's `cJSON.c` `cJSON_DetachItemFromArray` (off:
"nested call `get_array_item` inside a modeled call is outside the resource
projection") and `cJSON_DeleteItemFromArray` (off: the nested
`cJSON_DetachItemFromArray`): exactly the prediction, and exactly the two rows
rule 33's own card recorded as a BASE -> TIP delta across revisions (975 ->
977 Safe; [report](../detach-result-wrapper-composition/README.md)). No row
moves toward `Safe` with rule 33 off, no touched row stays Unknown, no leaf is
traded elsewhere, and the tool reports no interaction: the joint off side
(`tip-off`, 33, 34 and 36 together) moves 4 rows, rule 33's 2 and rule 34's 2.
The tool's post-run off-list check passed (status `complete`, no failures;
the `tip-off-33` components record `rule_set.off = ["33"]`, `tip` none).

**The group is indivisible.** `contribution_ledgers.py write` writes every
ledger of the record (rules 33, 34 and 36, three profiles each), and `check`
refuses a per-rule group committed in part (`the group lists [33, 34, 36],
the ledgers are [33]`). So this card commits all nine, written from the
record's path in RULE36-PILOT's report directory; RULE34-PILOT and
RULE36-PILOT writing from the same path produce the same bytes. `check` (15
ledgers) and `consistency` exit 0; the contributions page is regenerated.

## 4. Tests and gate

Focused runs before the halt (on `a98b6112e`'s tree): the package 9/9 tests
(5 moved, 4 new), `qcore-gateway-rule-api` 16/16 (with the new capability
test), `detach_wrapper_composition`
6/6 unchanged, `rule33_switch` 5/5 (nothing off: the three rule-33 rows and
four rule-independent lookups `Safe`; `33` off in a child process: the
pre-rule-33 refusals, exactly the three rows leave `Safe`; `35` off: rule 33
still admits), Clippy `-D warnings` on the three crates exit 0, `lake build
QCore.RemovedSelectionSwitch` and its audit exit 0.

Card gate before the halt: `gate_scope.py --base 80c8aea98 --out
private/gate-2` on `dc72dee7e` (full mode: workspace manifest, lockfile,
`qcore-c-gateway`), 03:45–04:31Z. Submodules, fmt, Clippy, both probes exit
0; Rust suite exit 0, 575 units, 597 summaries, 8,854 passed, 0 failed, 15
ignored (2,452 s); `check_lean_audits.py` exit 0, 391 audits; Python exit 1,
554 tests, 1 failure: `test_the_derived_facts_are_recomputed_rather_than_carried`
(committed matrix `lean_audits` 390 against 391 files, the new
`AuditRemovedSelectionSwitch.lean`), the integrator's regeneration. Logs:
`/media/Data/GitHub/quirin-lang/private/rule33-pilot/gate-2.log` and
`gate-2-logs/`. `gate-1` (01:37Z) and the first release build died on the
shared disk's exhaustion (`No space left on device`); they are kept as
failed attempts and are no evidence. The landing's full gate on `442dfa6f0`
then ran this code merged with the other pilots (8,927 passed, 0 failed).

Card gate of this round: `gate_scope.py --base 62c8e461c --out private/gate-r2`
on `8d0cb911b` (the ledgers, record copy, report and amendments; scoped:
no crate, no Lean), exit 0: submodules, fmt, Python 556 tests, 0 failures
([record](gate.json); console
`/media/Data/GitHub/quirin-lang/private/rule33-pilot/gate-r2.log`, steps
`gate-r2-logs/`). The commit after it adds only this paragraph and the gate
record.

## 5. Parity cost (plan decision 6)

- **Lines.** About 700 lines of rule code moved: the pure decisions
  (`removed_selection.rs`, 371 lines with their tests) and about 330 lines
  of word-level frame, expansion and statement code from
  `legacy_selection.rs` (−331/+101 there). The package is 994 lines (627
  before its tests); new in it are about 90 lines of package glue
  (`RemovedSelection`, `ValidationRun`, anchors) and 4 tests. Around that:
  204 lines of API (two capabilities and a test), 224 of switch integration
  test, 132 of Lean, 21 registry lines and 12 generated, 7 of host glue in
  `legacy_summary.rs`/`gateway_rules.rs`, 30 of manifests and pins. In all
  1,782 added and 712 deleted lines; unlike rule 37 (about 10 lines of rule
  code), most of this card's added lines are the rule itself, moved.
- **Time.** Reading and design about 0.4 h (01:10–01:23Z, prediction
  committed); implementation, Lean and focused tests 0.2 h (code commit
  01:35Z); documents 2 min. The card gate took 46 min of execution but
  started 2 h late because the shared disk was full; the release build took
  4 min after 1 h 40 min waiting for an audit slot. The card's own
  measurement ran 1 h 40 min until the halt and is superseded (section 3).
  Evidence this round: the shared measurement took 12 h 12 min of wall clock for three rules (six sides), one run instead of three; writing the ledgers and this report about 1 h.
- **Friction.**
  1. The frame row and the body rewrite have no route-map position: §1 lists
     the CFG-build rewrites beside the holds, and the custody derivation's
     row routes are not mapped. Both arms are placed by effect, as rule 37's
     settlement was (registered as `GATEWAY-RULE-API-BODY-REWRITE-POSITION`).
  2. The existing positional traits read the host's vocabulary; a body
     rewrite needs only words and the source line of each output word. The
     anchor (index of the input word whose line an output word takes) keeps
     the expanded stream byte-identical without exposing `Token`.
  3. The frame and run types are the rule's, so the two capabilities carry
     them as associated types rather than in the API; the host builds them
     after its own checks, which keeps the host's contract, type-model and
     linkage readings out of the package.
  4. The registry test and the layering pin are shared-file edits every
     pilot repeats; RULE35's generator check later required each arm's route
     to be a `pub fn` of the package, which the integrator added at merge.
  5. A new Lean audit file makes the committed matrix's derived count stale,
     so the card gate's Python suite fails until the integrator regenerates
     it (as for rule 37).

## 6. Likely conflicts and follow-ups

The code is on main already. This round adds only records: this report,
`integration-amendments.json`, and the rule-33 ledgers under
`contract/contributions/33/`. RULE36-PILOT and RULE34-PILOT amend the same
`GATEWAY-RULE-API-PILOT-MIGRATIONS` entry and regenerate the same
contributions page (`contribution_ledgers.py render`; rerun on a conflict).

Follow-ups ([amendments](integration-amendments.json)):

- **Registered `GATEWAY-RULE-API-BODY-REWRITE-POSITION`**: friction 1.
- **Amended `GATEWAY-RULE-API-PILOT-MIGRATIONS`** for rule 33 (not retired:
  32a remains).
- **Retired `CONTRIBUTION-LEDGER-RULE-33`**: the switch-measured ledgers are
  written and consistent, and equal the two rows rule 33's card recorded.
- `MEASURE-TOOL-RULES-OFF-LIVE` was already retired by RULE37-PILOT; this
  run is a further complete `--rules-off` run (with `--per-rule`).

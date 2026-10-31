# RULE32A-PILOT: rule 32a's caller-frame hold as a rule package, and what "off" means for a hold

The last pilot of `GATEWAY-RULE-API-PILOT-MIGRATIONS` (pluggable-cards
[plan](../../plans/pluggable-cards/README.md) seam B, phase P2), which also
discharges `GATEWAY-RULE-API-HOLD-OFF`. Base `db2795100`; prediction
`15a76ea84` (committed before any code edit), code `492f0418a` (the measured
TIP). The branch then merged origin/main `98b1bb785` (ARM-OBLIGATIONS, on the
integrator's request) and `c74537b7c` routes the hold through
`arm_obligations::consult` (section 2); every other later commit changes docs,
this report and its scripts only. Rule 32a
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
enclosing member body, and spells the hold as before. After the merge of
ARM-OBLIGATIONS (`98b1bb785`), the consultation is
`crate::arm_obligations::consult` (`c74537b7c`), as that card's source scan
requires of every package admission: the hold's admission carries no
obligation, so it is its lowering whether or not a unit frame is open, and
nothing changes (targeted check below). Moved out of the host:
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

Two attempts, outputs under this worktree's `private/rule32a-pilot/`:

| Attempt | Script | Window (UTC) | Outcome |
|---|---|---|---|
| `measure-1` | [`identity.sh`](scripts/identity.sh): `--side base=baseline:db2795100`, `--side tip=rev:492f0418a --build-commit db2795100…`, `--profile` ×3, `--declared` | 10-31 08:35 – 16:35 | TIP built and ran all three routes, exit 0 each (the corpus waited about 4 h for an audit slot); the plugin then **refused BASE**: each profile's baseline records a different `inputs.route_digest`. No comparison; status `failed` |
| `measure-2` | [`identity-2.sh`](scripts/identity-2.sh): `--side base=run:<landing>/sides/tip`, `--side tip=run:measure-1/sides/tip`, same profiles and prediction | 10-31 16:37 – 16:37 | exit 0, status `complete`, no failure: **the record** ([native-comparison.json](native-comparison.json), sha256 `c818baa27c7d…`) |

Why `measure-1` refused the baseline: `baseline:db2795100` was published by
`land.py publish` from a measurement that spells its routes from the explicit
inputs (`inputs/live.json`, link map
`docs/reports/w48-bu-single-leaf-bodies/link-map.corpus.json`), while this run
spells `--profile` (`contract/evaluations/eight-home/link-map.json`, the same
bytes, digest `98a4db76…`). The route digest covers the spelling, so the
baseline check refuses: the known hazard `MEASURE-BASELINE-ROUTE-SPELLING`,
which this card should have checked before choosing the baseline side. `measure-2` therefore reads, as
written, the landing measurement's own `sides/tip` (rev:`db2795100`, built at
the fixed path with build commit `db2795100`), the directory the baseline was
published from: [`landing_equals_baseline.py`](scripts/landing_equals_baseline.py)
checks that each of the 41 report files the three `baseline.json` records list
is in that side with its published SHA-256 (exit 0, 0 differ; the profiles'
own `run.json` are not compared). BASE is so the brief's `rev:db2795100`, run
by the integrator's landing; TIP is `rev:492f0418a`.

**Identity (BASE → TIP).** The declared check **matches** on all three
routes (`none`: no unpredicted or missing row, no delta mismatch). Each
side reproduces its profile's committed baseline (`evaluation_profiles
compare`, exit 0, both sides).

| Route | Rows | Safe / Unknown / Unsafe (Pending) | Moved | Content changed | Artifacts |
|---|---:|---|---:|---:|---|
| system | 448 | 101 / 343 / 4 | 0 | 0 | byte-identical |
| slice | 404 | 57 / 342 / 1 (Pending 4) | 0 | 0 | byte-identical |
| corpus | 4,834 | 1,013 / 3,811 / 10 | 0 | 0 | equal under a checked one-to-one substitution |

System and slice: every report artifact (`resource-diagnostics.json`, its
components, contracts, source-effects and chunk-contracts, the null property,
its summary, the outer projection, the exit) is byte-identical; only
`console.log`, `stderr.log` and the two invocation records differ (paths,
times, binary digest). The corpus route is `incomplete` only by the named
`composition-conflict` (MEASURE-CORPUS-SIDE-CWD-IN-BUILD-IDENTITY); its unit
and function rows have 0 moves and 0 content changes, 0 unmatched. By hand
([`check_identity.sh`](scripts/check_identity.sh), outputs in
[identity/](identity/)): `monkey-source.json`, `chunk-contracts`, the null
summary and the outer projection are byte-identical; RULE36-PILOT's
`corpus_substitution.py` (exit 0): 185 units, 4,834 functions, 180 build
identities mapped one to one (none unchanged), 0 rows moved or changed, every
file equal under the substitution or with the remaining digests blanked and
lists as multisets; RULE35-PILOT's `substitution_ids.py` (exit 0): every
differing artifact equal under a bijective token map
(`resource-diagnostics.json` 302 of 653 tokens, components 302/654, contracts
175/2,122, source-effects 10/29 with 3 contract ids, `corpus-null.jsonl`
180/616). These are the figures of the earlier pilots' identity runs.

No `--rules-off` side: rule 32a is fixed, and `--rules-off 32a` is refused by
the analyzer.

**After the merge** (`c74537b7c`, targeted check, not a re-measurement, on
the integrator's instruction): the gateway's library tests including
ARM-OBLIGATIONS' source scan (597 passed, 1 ignored, filtered to
`arm_obligations`, `gateway_rules`, the hold and its children),
`caller_frame_hold.rs` 7 passed, `rule32a_hold.rs` 5 passed, `cargo fmt`
clean, Clippy `-D warnings` clean on `qcore-c-gateway` and the package. The
card gate (section 5) runs on the merged branch.

## 5. Gate

`python3 scripts/gate_scope.py --base db2795100 --out private/gate` on
`42ce08ae9` (the merged branch with every code, Lean and record change; the
commit after it adds only this section and [gate.json](gate.json)). Console
log `/media/Data/GitHub/quirin-lang/private/rule32a-pilot/gate.log`, step
logs in the worktree's `private/gate/`. Scope: full (the workspace manifest
and lockfile change, and the diff reaches `qcore-c-gateway`), all Lean (a
proof root changed), no provider gate. Overall exit 1, from one known Python
test:

| Step | Exit | Counts |
|---|---|---|
| submodules | 0 | 1 submodule at its pin |
| fmt, Clippy (`--all-targets -D warnings`) | 0, 0 | |
| Rust suite (`run_test_binaries.py`, rust-suite slot) | 0 | 586 units, 623 summaries; 8,956 passed, 0 failed, 15 ignored |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| Lean (`check_lean_audits.py`) | 0 | 398 audits |
| Python suite | 1 | 568 tests, 1 failure |

The one failure is
`test_build_matrix_sections.test_the_derived_facts_are_recomputed_rather_than_carried`:
the committed acceptance matrix records `lean_audits` 397 and the tree has 398
(this card's `AuditCallerFrameHoldSwitch.lean`). The matrix's derived fields
are the integrator's, regenerated at landing, so this is the expected residue
of adding an audit (`CARD-GATE-NEW-AUDIT-MATRIX-COUNT`, as for RULE36-PILOT),
not a defect.

## 6. Ledger and follow-ups

- `MEASURE-BASELINE-ROUTE-SPELLING` **amended**: a second instance (this
  card's `measure-1`, about 8 h of TIP before the refusal), and the recovery
  this card used: `--side base=run:<landing>/sides/tip` while the landing
  measurement's side is still on disk, checked file by file against the
  baseline's digests.

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

| | |
|---|---|
| Decision code moved | about 10 gateway lines (the member test, the in-force filter and `within_one_frame` in `hold_call_sites`) → 90 package lines (`hold.rs`) + 37 `lib.rs`; the growth is the host capability, the trait impl, the table and docs |
| New interface | none beyond the `ReasonCategory` re-export (+6): `TokenHold`, `HoldQuery` and `Hold` already existed |
| Host | `legacy_caller_frame_hold.rs` +95 −14 (the host type, the consultation); `gateway_rules.rs` +94 −6 (the hold in `enabled`, its tests) |
| Tests | package 201 lines; `rule32a_hold.rs` 167; Python +22 −6 |
| Lean | `CallerFrameHoldSwitch.lean` 214 lines, audit 17 |
| Whole code diff | 19 files, +1,046 −27 (`db2795100..492f0418a`, prediction included); the merge follow-up +9 −6 |
| Active time | about 1 h: the brief to the code commit about 16 min after the brief was written (17:18 to 17:34 local), then docs, report and the merge follow-up |
| Wall clock | about 8 h 20 min to the identity record: TIP's routes 8 h (4 h of it the corpus waiting for an audit slot behind other cards' Rust suites), the refused baseline, then a 7-second `run:` comparison |

Friction:

1. **"Off" was the whole question.** The code move was small; the decision
   and its proof (why neither off is acceptable) were the card.
2. **A latent widening in the switch reader** (section 1): `enabled` was
   written for packages whose off only refuses; a hold needed it to read the
   registry's `off` first.
3. **The published baseline is not reusable under `--profile`.** A baseline
   published by `land.py` from `inputs/live.json` refuses a `--profile`
   measurement over byte-equal inputs, after TIP has been measured. Cost here:
   one wasted BASE check, recovered with `run:` because the landing side was
   still on disk (`MEASURE-BASELINE-ROUTE-SPELLING`, already registered by
   RULE36-PILOT; this card did not check the ledger before choosing the side).
4. **Shared files**: `registry.toml`, the generated registry, `Cargo.toml` and
   the lock, `qcore-c-gateway/Cargo.toml`, `workspace_layering.rs`, the
   gateway's `gateway_rules.rs` tests, the API's `lib.rs`, `proof/QCore.lean`,
   `architecture.md`'s row, the plan, and, after ARM-OBLIGATIONS, the
   consultation call.
5. **New audit file**: the committed matrix's derived `lean_audits` is stale
   by one (CARD-GATE-NEW-AUDIT-MATRIX-COUNT), so the card gate's Python suite
   is expected to fail that one test (section 5).

Over the six pilots: moving a rule's decision costs well under an hour of
code each time. The evidence (one identity run per move, 8-14 h under slot
contention) is the cost, and a fixed hold adds a decision record but no
ledger.

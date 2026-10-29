# RULE37-PILOT: rule 37 as a rule package

Card RULE37-PILOT, the second pilot of `GATEWAY-RULE-API-PILOT-MIGRATIONS`
([plan](../../plans/pluggable-cards/README.md) §3 B and §4;
[interface report](../gateway-rule-api/README.md) §7). Base `80c8aea98`.
Implementation `6b6a3a438`; [prediction](PREDICTION.md) committed before it
(`1ac59451c`).

## 1. What moved

Rule 37 ([rule](../../agents/rules/37.md),
[card](../w48-bu-rk-result-constructors/README.md)) has three parts in the
gateway. Two are admissions and moved; one is a guard and stays.

| Part | Before | After |
|---|---|---|
| Typed `sizeof` exemption: a mention inside a checked, non-VLA `sizeof` operand is not an escape of the store base | inline in `aggregate_store::store_bases` and `insulated_bases` | package arm `sizeof-operand-exemption` (typed reader); the host computes the checked spans with its existing reader and asks the package |
| `asserted_allocation` premise: a checked specification naming it is a wrapper whose non-null result rests on a terminating assertion | contract text only, adopted by the generic checked-specification settlement (W45) | package arm `asserted-allocation-premise` at `legacy_checked_spec::settle`: such a specification stands only while rule 37 admits it, and the admission carries `Obligation::Premise("asserted_allocation")`, which the declaration must state (the first use of `Obligation::Premise`) |
| Null readers' checked-result guard: a checked custody specification is never an unchecked non-null fact | `null_deref.rs`, `walk.rs`, `witness.rs` | unchanged, in the host, never switched off: it is a restriction, so switching it off would widen |

Files:

- `crates/gateway-rules/rule-37-result-constructors/` (new): the package
  (`ResultConstructors`, `ID = "37"`, `ASSERTED_ALLOCATION`), its span
  well-formedness check (fail-closed: spans that do not start at `sizeof`, run
  past the body, cover no operand, overlap or are out of order refuse the whole
  answer) and 6 tests, including Rust vectors of the Lean switch theorems and
  the dependency-set test (the rule interface only).
- `crates/qcore-gateway-rule-api/src/checked_readings.rs` (new, additive): two
  host capabilities whose queries need none of the host's vocabulary,
  `CheckedOperandArm` (§7) and `SpecificationPremiseArm` (§2.2/§6), and one
  test. Nothing existing in the API changed.
- `crates/qcore-c-gateway/src/result_constructor_rule.rs` (new): the host side,
  building the queries and composing answers with `consult` under
  `gateway_rules::enabled("37")`; `aggregate_store.rs` asks it for the exempt
  spans; `legacy_checked_spec.rs` asks it for premise withdrawals (new
  `Withdrawal::PremiseNotAdmitted`; unreadable premises withdraw every checked
  specification, as an unreadable checked set already does).
- Registry row `37` (crate `rule-37-result-constructors`, Lean
  `QCore.ResultConstructors` and `QCore.ResultConstructorSwitch`, `off =
  "refuses"`), regenerated `gateway_rule_registry.rs`; `legacy_route_table.rs`
  is unchanged (arms have no table). `known_rule` now accepts `37`.
- Workspace member, gateway dependency, `workspace_layering.rs` pin (the rule
  crate may depend on the API only; the gateway on the rule crate), the
  gateway's registry test (a body lives in the gateway or in its own
  `rule-<id>-*` crate), `tests/rule37_switch.rs`, rule document 37, the
  architecture row, `proof/QCore.lean` (generated).

**Placement note.** No route-map position names the checked-specification
settlement: it runs before the parser and decides which declarations the
classifier's contracted arm sees. The premise arm is registered at
`classification` because that is where its effect is read (§6, the contracted
or uncontracted arm of a call). A hold position would have forced
`not-switchable`, which is wrong here: refusing withdraws.

## 2. What off means

`QCORE_GATEWAY_RULES_OFF=37` asks neither arm.

- The store scan gets no exempt span. That is the scan's code before rule 37
  (`1f1615cfa^`) exactly, so a `sizeof(*e)` mention is a dereference of the
  base again (premise 4) and the store is refused: the fixtures of
  `result_constructors.rs` and `checked_allocator_constructors.rs` keep the
  refusal `UnknownAlias e` + `indirect memory access is outside the resource
  projection` (`rule37_switch.rs`).
- Every checked specification naming `asserted_allocation` (today `rd_calloc`,
  `rd_strdup`) is withdrawn with "the gateway rule owning its named premise
  does not admit it (switched off)", so the defining rows are `Unknown`.
- **Boundary, stated rather than closed.** A withdrawn checked specification
  leaves its callers whatever W4 validates from an in-unit body (the existing
  W45 withdrawal). For an asserting wrapper body that is the non-null result
  itself, so a constructor with no `sizeof` (`rd_calloc(1, 16)`) stays `Safe`
  with rule 37 off, on W4's summary, whose row names no
  `asserted_allocation` premise. This is no Unknown-to-Safe by the move (the
  row is `Safe` with 37 on as well) and pre-dates rule 37 for any undeclared
  asserting wrapper; the contract environment before rule 37 (nullable
  `inherit libc::calloc`) cannot be restored by a switch, and contract edits
  are out of scope. Registered as `RULE37-W4-ASSERT-PREMISE-VISIBILITY`.

Lean (`QCore.ResultConstructorSwitch`, 13 theorems, `AuditResultConstructorSwitch.lean`):
the switch on is rule 37's model (`the_switch_on_reads_rule_37`,
`the_switch_on_is_the_constructor_step`), off is the old scan
(`the_switch_off_reads_the_old_scan`), off never widens the scan or the
specifications (`the_switch_off_never_widens_the_scan`,
`the_switch_off_never_widens_the_specifications`), off withdraws exactly the
premised specifications and an admitted one carries the premise. Premises kept
separate: Rust/model correspondence, fidelity of the host's checked spans and
of the declared premises.

## 3. Identity run and contribution

One `quirin-audit measure` run (release host and plugin built from
`6b6a3a438`), `--side base=baseline:80c8aea98` (the integrator's published
baseline, file digests verified by the tool), `--side tip=rev:6b6a3a438`,
`--rules-off 37`, `--declared PREDICTION.md`, `--inputs live.json`,
`SOURCE_DATE_EPOCH=1767225600`: exit 0, status `complete`, no failures,
2026-10-28T11:36Z to 2026-10-29T01:05Z (most of it queued for audit slots
behind other cards' suites and measurements). Record:
[`native-comparison.json`](native-comparison.json) (sha256 `78a83e75ed31…`).
Private output: `/media/Data/GitHub/quirin-lang/private/rule37-pilot/measure-2/`
(`measure-1` is a refused first attempt, a mistyped revision, nothing ran).

**Identity (BASE -> TIP).** The declared check matches on all three routes
(`none`, no unpredicted or missing row). Every route: 0 moved rows, 0 content
changes, 0 unmatched rows, unit verdicts unchanged; every function row
differs only in the tool's environment fields (`assumptions`,
`build_identity`, `environment_sha256`, `resource_contracts_sha256`, `id`).

| Route | Rows | Safe / Pending / Unknown / Unsafe (both sides) | Identical artifacts | Differing artifacts | Checked substitution |
|---|---:|---|---:|---:|---|
| corpus | 4,834 | 1,013 / 0 / 3,811 / 10 | 4 | 5 | 218 values, bijective |
| system | 448 | 101 / 0 / 343 / 4 | 4 | 4 | 6 values, bijective |
| slice | 404 | 57 / 4 / 342 / 1 | 4 | 4 | 4 values, bijective |

The differing artifacts were checked with
[`scripts/substitution.py`](scripts/substitution.py) (results
`/media/Data/GitHub/quirin-lang/private/rule37-pilot/identity-{corpus,system,slice}.json`, each exit 0):
after replacing the revision (`80c8aea98…` / `6b6a3a438…`) and every 24- to
64-hex-digit digest in values and keys, and reading lists as multisets, each
pair of files is equal; the differing scalars and keys then form one
bijection per route. On each route two of the substituted values carry
the revision itself (`analyzer_revision`, the hypothesis graph's
`evidence_version`), and the rest are digests derived from it (build
identities, task, configuration, obligation ids). Evidence lists are sorted
by those digests, so they reorder; items equal after normalization are left
unpaired and counted, not guessed. This is the same class of movement as
MEASURE-CORPUS-SIDE-CWD-IN-BUILD-IDENTITY (here the revision, not the working
directory, since both sides ran as `sides/tip`).

**Contribution (TIP with 37 off -> TIP)**, ledgers
`contract/contributions/37/{eight-home,fluent-bit-system,fluent-bit-tail-pause}.json`
written by `contribution_ledgers.py write`, `check` and `consistency` exit 0
(first committed ledgers; base + contribution = each profile's real-run
total). The tool's post-run off-list check passed: tip-off components record
`rule_set.off = ["37"]`, tip and base none.

| Profile | Rule 37 off | Rule 37 on | Rows moved |
|---|---|---|---|
| eight-home (corpus) | 1,008 Safe / 3,816 Unknown / 10 Unsafe | 1,013 / 3,811 / 10 | 5 Unknown -> Safe, all under `asserted_allocation`; 260 touched rows stay Unknown; leaves −36 +22 |
| fluent-bit-system | 101 / 343 / 4 | same | none |
| fluent-bit-tail-pause | 57 / 4 Pending / 342 / 1 | same | none |

The five rows, all librdkafka (each resting on `rd_calloc` and, except the
last, `rd_strdup` with `asserted_allocation`): `rdkafka_admin.c`
`rd_kafka_ConsumerGroupListing_new`, `rd_kafka_ConsumerGroupListing_copy`,
`rd_kafka_ConsumerGroupListing_copy_opaque`,
`rd_kafka_AlterUserScramCredentials_result_response_new`, and
`rdkafka_aux.c` `rd_kafka_acl_result_new`. Off, they refuse with `indirect
memory access is outside the resource projection` (the `sizeof(*e)` mention)
or with a call to the uncontracted constructor below them. No row moves toward
`Safe` with the rule off. Rule 37's own card recorded a forecast, not measured
rows ([prediction](../w48-bu-rk-result-constructors/PREDICTION.md): five
primary rows); three of its five forecast rows are measured here
(`ConsumerGroupListing_new`, `_copy`, `_copy_opaque`), `topic_partition_new`
and `topic_partition_new_with_id_and_name` are not among the moved rows, and
two rows it did not name are.

## 4. Gate

`python3 scripts/gate_scope.py --base 80c8aea98 --out private/gate-1` on
`a5ef39093` (the code of `6b6a3a438` plus the report draft); full mode
(workspace manifest, lockfile, `qcore-c-gateway`). Record [`gate.json`](gate.json);
console `/media/Data/GitHub/quirin-lang/private/rule37-pilot/gate-1.log`, step logs
`/media/Data/GitHub/quirin-lang/private/rule37-pilot/gate-1-logs/`
(copied from the worktree's `private/gate-1/`). Exit 1, for one Python test:

| Step | Exit | Counts |
|---|---:|---|
| submodules | 0 | 1 |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| Rust suite (`run_test_binaries.py`, whole workspace) | 0 | 575 units, 597 summaries; 8,858 passed, 0 failed, 15 ignored (8,972 s) |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| `check_lean_audits.py` | 0 | 391 audits |
| Python suite | 1 | 554 tests, 1 failure |

The failure is `test_build_matrix_sections.test_the_derived_facts_are_recomputed_rather_than_carried`:
the committed acceptance matrix carries `lean_audits` 390 and the tree now has
391 audit files (`AuditResultConstructorSwitch.lean`). The matrix is the
integrator's record, regenerated at landing; nothing else failed. The
records committed after the gate (this report, the comparison record, the
ledgers, `scripts/substitution.py`) reach no crate (`gate_scope.py --plan`:
0 affected crates, Python only); the Python suite was rerun on the final
commit: PYTHON-RERUN.

## 5. Parity cost (plan decision 6)

- **Lines.** The rule's own code in the gateway was small: 4 lines of
  `aggregate_store.rs` replaced by one call, plus the null guard left in place.
  The move added 281 lines in the rule crate (about 140 of them tests and
  docs), 150 in the API (two capabilities, one test), 186 of host glue
  (`result_constructor_rule.rs`, including the off-semantics of the premise and
  one test), 28 in `legacy_checked_spec.rs`, 307 of switch integration tests,
  131 of Lean, 17 registry lines and 12 generated. About 1,140 added lines
  for about 10 lines of rule code: the cost is the interface, the switch's
  new off-reading and its proof, not the move.
- **Time.** Reading and design about 0.5 h; implementation, Lean and focused
  tests about 0.8 h (first commit 09:31Z, code 09:46Z). The card gate took
  2.6 h (the Rust suite alone 2.5 h). The measurement took 13.5 h wall,
  mostly waiting for audit slots; it used the integrator's freshly published
  baseline instead of rebuilding BASE. Writing the substitution check and the
  ledgers took about 1 h.
- **Friction.**
  1. Rule 37 had no premise *arm* before the move. Its premise was contract
     text that the generic W45 settlement adopted. A switch therefore needed a
     new off-reading: withdraw the specification. That reading has a boundary
     (section 2). Rules that are pure admissions will not need this.
  2. No route-map position names the checked-specification settlement. The
     premise arm sits at `classification` by its effect. Holds were
     unusable, because they cannot be switched.
  3. The interface's arm traits read the host's vocabulary (`Host`, tokens,
     nodes). Rule 37 needs none of it, so its capabilities are non-generic,
     in their own file to avoid RULE35-PILOT's.
  4. The gateway's registry test pinned every body in the gateway, and the
     layering test pins each crate edge. Both are deliberate shared-file
     edits that every pilot repeats.
  5. A new Lean audit file makes the committed matrix's derived count stale,
     so a card gate cannot pass the Python suite without editing an
     integrator record.
  6. The measure tool reports the identity's differing artifacts but does not
     check the substitution itself. Revision-derived digests, 24-digit
     obligation keys and digest-sorted evidence needed a checker of their own.

## 6. Likely conflicts and follow-ups

Files RULE35-PILOT will also touch: `crates/gateway-rules/registry.toml` and
the generated `gateway_rule_registry.rs` (rerun
`scripts/gen_gateway_registry.py`), `Cargo.toml` members and `Cargo.lock`
(take both crates, `cargo update -w`), `compiler/tests/workspace_layering.rs`
(both rule crates and both gateway edges), `crates/qcore-c-gateway/Cargo.toml`,
`crates/qcore-c-gateway/src/lib.rs`, the registry test in `gateway_rules.rs`
(both relax the all-in-gateway assertion), `crates/qcore-gateway-rule-api/src/lib.rs`
(module and re-export lines; this card's capabilities are in their own file),
`proof/QCore.lean` (rerun `scripts/gen_lean_roots.py`),
`docs/development/architecture.md` (the `crates/gateway-rules/` row) and
`docs/agents/gateway.md` (rerun `scripts/gen_rule_docs.py`).

Follow-ups ([amendments](integration-amendments.json)):

- **Registered `RULE37-W4-ASSERT-PREMISE-VISIBILITY`**: section 2's boundary.
- **Amended `GATEWAY-RULE-API-PILOT-MIGRATIONS`**: rule 37 is migrated; 35
  (concurrent), 36, 33, 34 and 32a remain; the settlement has no route-map
  position.
- **Retired `CONTRIBUTION-LEDGER-RULE-37`** and
  **`MEASURE-TOOL-RULES-OFF-LIVE`**: the first real `--rules-off` measurement
  on a migrated rule completed its off-list check, and its ledgers are
  written and consistent.

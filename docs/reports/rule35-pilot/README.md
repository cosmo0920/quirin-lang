# RULE35-PILOT: gateway rule 35 as its own rule package

The first pilot of `GATEWAY-RULE-API-PILOT-MIGRATIONS` (pluggable-cards
[plan](../../plans/pluggable-cards/README.md) seam B and §4, phase P2). Base
`80c8aea98`; measured tip `555bd17db` (the code; later commits change tests,
records and this report only). Rule 35 (`docs/agents/rules/35.md`) moves out of
`qcore-c-gateway` into `crates/gateway-rules/rule-35-literal-decay` behind the
[GATEWAY-RULE-API](../gateway-rule-api/README.md) interface. No verdict, leaf
or diagnostic changes with every rule on (section 3); rule 35's contribution is
now a switch-measured ledger (section 4).

## 1. What changed

**The rule package** `crates/gateway-rules/rule-35-literal-decay` (package
`rule-35-literal-decay`, id `35`) depends on `qcore-gateway-rule-api` only:

- `literal_read.rs`: arm 1, the classification arm `literal-read`
  (`LiteralRead: ClassificationArm<H>` for every `H: LexicalHost`): `decide`,
  the literal-run `follow` reading, `exempt`, and `DECISION_VECTORS`.
- `array_member_decay.rs`: arm 2, the typed-reader arm `array-member-decay`
  (`ArrayMemberDecay: TypedReaderArm<H>` for every `H: TypedHost`): `decide`,
  `element_of`, `decayed_operand`, `decide_operand`, `DECISION_VECTORS`, and
  the arm's parity witness.
- Tests: both decision tables, their Lean binding to
  `QCore.LiteralDecayReadings` (`literalVectors`, `decayVectors`, read from
  `proof/` as before), the element reader, and both packages through a toy host
  whose literal spelling differs from the gateway's (so the package is seen to
  take it from the host). The decision code is the gateway's, moved: the only
  change inside it is that the switch is no longer read in `decide` and
  `decayed_operand` (the host gates the package instead), and `exempt` refuses a
  token that is not `*` where the gateway had a `debug_assert`.

**The host capabilities** (rule API `traits.rs`; report §7 asked rule 35 to fix
`ClassificationArm` and `TypedReaderArm` so 37 and 36 can reuse them):

- `LexicalHost: Host` with `is_literal(token)`: which tokens are literals. The
  spelling (`__qcore_literal__`) stays the host's.
- `TypedHost: Host` with total reads of one typed node: `node_kind`,
  `node_attribute(node, key)`, `node_type` (desugared, else written), and
  `node_children`.
- `TypedReaderArm` now has three methods. `reads(node)`: the node has the
  arm's shape, so the reader asks no older route for it. `ask(node)`: admits a
  `TypedReading` the host performs (non-exhaustive; rule 35 adds
  `ElementAddress(place)`), or refuses, and the reader refuses the node.
  `witness(node)`: the decision as a parity row (`Witness`). The old
  `ask -> Answer<()>` could not hand the host the place to read nor separate
  "not my shape" (older routes answer) from "my shape, refused". Rule 35 needs
  that split to keep the gateway's exact control flow. `ClassificationArm` is
  unchanged.

**The gateway as host** (`crates/qcore-c-gateway/src/rule_hosts.rs`):

- `ArmHost<'t>` implements `Host`, `LexicalHost` and `TypedHost` over token
  texts and Clang JSON.
- `literal_read(texts, at)` and `decay_reads(node)` / `decay_reading(node,
  decisions)` call the packages through `consult` and the registry switch
  (`gateway_rules::enabled`).
- `Reader::decayed_address` performs `ElementAddress` exactly as before: rule
  10n(1)'s address reading of every hop, the array not read. It refuses a
  reading it does not know.
- `witness_row` writes the parity row's old shape (`{"rule": "35",
  "decayed_member": {"kind", "type", "element"}, "admitted", "refusal"}`), with
  a test.
- Fail-closed: an admission carrying obligations is refused, because no hosted
  arm's placement can carry a premise yet. Rule 37's `asserted_allocation`
  premise needs that path.

The gateway keeps no rule-35 decision. `literal_dereference.rs`,
`array_member_decay.rs` and their unit tests are deleted. The two hooks
(`legacy_token_shapes::contains_unmodeled_access`,
`typed_access_positions::StatementIndex::argument`) call `rule_hosts`.

**Registry and generator.** Rule 35's row names `crate =
"rule-35-literal-decay"` and the routes `literal_read::decide` and
`array_member_decay::decide`; `gateway_rule_registry.rs` is regenerated.
`gen_gateway_registry.py` now refuses two things:

- a `crate` that is neither `qcore-c-gateway` nor a package
  `crates/gateway-rules/<crate>/` whose manifest names it;
- an arm route of such a package that is not a `pub fn` of its
  `src/<module>.rs`.

**Tests added or changed.**

- `compiler/tests/workspace_layering.rs`:
  - admits the package and the gateway's edge to it (a deliberate pin
    update);
  - adds `rule_packages_depend_on_the_api_only`: every package under
    `crates/gateway-rules/`, listed from tracked files, is a workspace member
    and depends on the API (and `qcore-core`) only.
- `gateway_rules.rs`: the registry test accepts hosted packages, and a new
  test binds each hosted package's id to its registry row. The arm-switch
  test now drives the hosted calls.
- `scripts/tests/test_gateway_registry.py`: the arm-constant check reads the
  package, and a new test covers the package row and the generator's two
  refusals.

**Docs and proof.** Rule document 35 (and the generated `gateway.md`), the
architecture row, and the Lean module's doc comments name the package. Its
theorems are unchanged, so `QCore.LiteralDecayReadings` and
`AuditLiteralDecayReadings` stay the rule's Lean modules. The proof did not
move; no new module, and `gen_lean_roots.py` output is unchanged.

The existing fixtures (`tests/literal_decay_readings.rs`, 7) and the switch
tests (`tests/gateway_rule_switches.rs`, 6) pass unchanged. With `35` off,
both fixtures keep their old refusals.

## 2. Measurement

One `quirin-audit measure` run, [`scripts/identity.sh`](scripts/identity.sh)
(the three live profiles; `--side base=rev:80c8aea98`,
`--side tip=rev:555bd17db`, `--rules-off 35`, `--declared PREDICTION.md`, one
`--build-commit`, 24 h quota). It ran 2026-10-28 09:47Z to 20:00Z (10 h 13 min,
under slot contention with two other cards), exit 0, status `complete`, no
failures. The record is [`native-comparison.json`](native-comparison.json).
Logs are under `/media/Data/GitHub/quirin-lang/private/rule35-pilot/`
(`identity-1.log`, `identity-1/`). No `baseline:80c8aea98` was published, so
BASE was built from its revision.

## 3. Identity (BASE → TIP, every rule on)

| Route | Rows | Moves | Status deltas | Artifacts |
|---|---|---|---|---|
| system | 448 | 0 | 0 | byte-identical |
| slice | 404 | 0 | 0 | byte-identical |
| corpus | 4,834 | 0 | 0 | equal under a checked one-to-one substitution |

The declared check matches on all three routes (`none` predicted). Every
profile's TIP figures reproduce its committed baseline (`evaluation_profiles`
compare exit 0 on base and tip, all three). This also measures the slice
against a merged base, which GATEWAY-RULE-API could not
(its base/slice route never ran).

**Corpus substitution.** The corpus route is `incomplete` only by the named
`composition-conflict`, the known MEASURE-CORPUS-SIDE-CWD-IN-BUILD-IDENTITY
effect: each side's unit build identity digests its own working directory.
The union holds 365 components for 185 rows; rows, units, content and
diagnostics are equal. Five artifacts differ; four are byte-identical. The
checks, by [`scripts/corpus_identity.sh`](scripts/corpus_identity.sh) (output
`private/rule35-pilot/identity-1-corpus-*.jsonl`):

- `corpus-null.jsonl` and `resource-diagnostics.contracts.json`: byte-identical
  under a one-to-one 64-hex substitution (GATEWAY-RULE-API's `substitution.py`;
  180 and 175 digests substituted).
- `resource-diagnostics.json`, `.components.json` and `.source-effects.json`
  also differ in order and in 24-hex `resource_contracts:` ids derived from
  the moved digests. The new [`scripts/substitution_ids.py`](scripts/substitution_ids.py)
  pairs 64-hex digests and 24-hex ids by walking both documents. List
  elements are matched by blanked text; ties are resolved by the map the rest
  of the document fixes. It requires the map and its inverse to be one to one,
  and the substituted BASE to equal TIP with lists as multisets. All three
  pass (653, 654 and 29 tokens; 302, 302 and 10 substituted).
- GATEWAY-RULE-API's `identity_check.py` (exit 0): every file equal with
  digests blanked, and the build identities map one to one, one per unit
  (4,834 rows, 180 units).

## 4. Rule 35's contribution (TIP with 35 off → TIP)

| Profile | Rows moved | Leaves traded | Touched rows staying Unknown |
|---|---|---|---|
| eight-home (corpus) | 12 Unknown → Safe | −123 +8 (103 rows) | 104 |
| fluent-bit-system | 0 | −4 +0 | 4 |
| fluent-bit-tail-pause | 0 | −11 +3 | 9 |

The twelve rows are exactly W48-BU-SINGLE-LEAF-BODIES's
[twelve](../w48-bu-single-leaf-bodies/README.md#rule-35s-twelve-moves-corpus-tip-off--tip),
measured there by a compile-time arms-off build. They are
`rd_kafka_mock_cluster_io_{set_event,set_events,clear_events}`,
`rd_kafka_mock_connection_set_blocking`, `rd_kafka_log_syslog`,
`rd_kafka_set_logger`, `rd_kafka_oidc_assertion_get_algo_label`,
`unit_test_telemetry_decode_error`, `rd_kafka_txn_state_transition_is_valid`,
`rd_kafka_anyconf_set_modified`, `rd_kafka_log_print` and
`mk_config_signature`. No row rests on a named premise. No other status moves
on any route. GATEWAY-RULE-API's incomplete run saw corpus 12, system 0 and
slice 0; this complete run confirms those figures. The leaf trades are new
here: that run recorded none, and they are within the "changes only their
leaves" of the rule's own report.

The post-run off-list check completed: tip-off components record
`off = ["35"]`, and the other sides record none. The ledgers
`contract/contributions/35/{eight-home,fluent-bit-system,fluent-bit-tail-pause}.json`
were written by `contribution_ledgers.py write` from the committed record.
`check` (3 valid; `known_rule` accepts `35` from the registry) and
`consistency` (base + rule 35 = each profile's real-run total) exit 0. The
contributions page is regenerated. Rule 35 was switched alone, so its
interactions with other rules were not measured.

## 5. Parity cost (plan decision 6)

| | |
|---|---|
| Decision code moved | 370 gateway lines (`literal_dereference.rs` 134, `array_member_decay.rs` 236) → 424 package lines (152 + 239 + 33 `lib.rs`); the growth is docs, the package structs and the trait impls |
| Tests moved or new | 209 gateway unit-test lines → 284 package lines + 83 toy host; 117 gateway host-test lines |
| New interface | rule API +69 lines (two capabilities, `TypedReading`, `Witness`, the three-method `TypedReaderArm`); gateway host `rule_hosts.rs` 194 lines; generator +30 |
| Whole diff to the measured code | 25 files, +1,201 −464 (crates, compiler, scripts, proof) |
| Active time | about 1 h 15 min: reading 15 min, implementation and targeted tests 30 min, fixing one tree-walk tripwire, the corpus checker 30 min |
| Wall clock | about 10 h 30 min, dominated by the 10 h 13 min measurement (9 route runs) |

Friction:

1. **`TypedReaderArm` had to change.** `Answer<()>` cannot carry the place
   to read or tell "not my shape" from "refused". 37 and 36 now inherit the
   three-method form; a typed arm that only gates needs only `reads` and `ask`.
2. **Host types.** The lexical gate runs on `&[&str]` without a `Parser`, so
   a second host (`ArmHost`) was needed beside `Parser`'s. Its statement and
   condition types are empty: the API's single `Host` couples vocabularies an
   arm never uses.
3. **Admissions with obligations are refused** at the hosted arms. Rule 37's
   premise needs a carrying path at its position first.
4. **Rule 29's tree-walk tripwire** (`test_tracked_files`) fails a new
   `read_dir` or `.glob(` in tests. Use `tests/support/tracked_files.rs`.
5. **The corpus identity** still needs a manual checker outside the tool
   (MEASURE-CORPUS-SIDE-CWD-IN-BUILD-IDENTITY). The 64-hex checker no longer
   suffices once 24-hex ids and id-sorted order move.
6. **The measurement is the cost.** The code move was about 30 minutes; the
   three-side, three-route run was ten hours under contention. Per-pilot cost
   is dominated by measurement, not by porting.

## 6. Gate

The scoped card gate (`python3 scripts/gate_scope.py --base 80c8aea98 --out
private/gate`) runs in full mode: the diff touches `Cargo.toml`,
`Cargo.lock` and `qcore-c-gateway`. Console logs are
`/media/Data/GitHub/quirin-lang/private/rule35-pilot/gate-<n>.log`; the record
is [`gate/gate.json`](gate/gate.json).

- **gate-1** on `555bd17db` exited 1. Submodules, fmt, Clippy, the Rust suite
  (574 units, 8,849 passed, 0 failed, 15 ignored; 4,626 s including slot wait),
  both probes and Lean (`QCore.LiteralDecayReadings`, 1 audit) exited 0. The
  Python suite failed one test of 555,
  `test_every_remaining_tree_walk_in_the_tests_is_a_reviewed_one`, on the new
  walks in `workspace_layering.rs` and `test_gateway_registry.py`. Fixed in
  `f2f821e63`; no analyzer code changed.
- **gate-2** on `3608e432b` (the report commit) exited 1 on the submodule
  step only: `third_party/libevent` had lost its registration in the shared
  `.git/config` while the card was idle (not a change of the card's). Every
  other step exited 0: the Rust suite had 574 units, 8,849 passed and 0
  failed; Python ran 555 tests. `git submodule update --init --recursive`
  re-registered it.
- **gate-3** on `3608e432b` exited 0, which is the card gate. Every step
  passed:
  - submodules: 1, none moved;
  - fmt and Clippy;
  - Rust suite: 574 units, 594 summaries, 8,849 passed, 0 failed, 15
    ignored, 24 filtered. Its 7,413 s were 6,273 s of slot wait and 19 min
    of run.
  - both probes;
  - Lean: `QCore.LiteralDecayReadings`, 1 audit;
  - Python: 555 tests, 0 skipped.

  The log is `private/rule35-pilot/gate-3.log` and the record is
  [`gate/gate.json`](gate/gate.json). The commit after `3608e432b` adds only
  this record and this paragraph.

## 7. Boundaries

- The move changes no verdict. That is measured on the three live profiles,
  not proved; the Rust decision code and its Lean vectors are unchanged.
- The corpus identity is a substitution check, not byte identity. It
  inherits MEASURE-CORPUS-SIDE-CWD-IN-BUILD-IDENTITY.
- Contribution is "the run without rule 35", one switch alone. Interactions
  with rules 32 and 34 (whose rows the rule-35 card separated by a compile-time
  build) are unmeasured.
- Out of scope: other rules, GATEWAY-RULE-API-HOLD-OFF, contract edits.

## 8. Follow-ups

- Amended `GATEWAY-RULE-API-PILOT-MIGRATIONS`: rule 35 done, with its cost
  and the interface it fixed.
- Retired `CONTRIBUTION-LEDGER-RULE-35` and `MEASURE-TOOL-RULES-OFF-LIVE`: their
  discharge conditions are met.
- Appended to `MEASURE-CORPUS-SIDE-CWD-IN-BUILD-IDENTITY`: the 24-hex
  substitution checker.
- Registered `GATEWAY-RULES-PRODUCTION-SOURCE-SCANS`: test scans of
  production sources that select `crates/*/src` skip
  `crates/gateway-rules/*/src` (for example `qcore-verification`'s
  `composition_bounds.rs` vacuity tripwire).
- Registered `GATEWAY-RULE-API-ARM-OBLIGATIONS`: hosted arms refuse
  admissions with obligations.

# RULE-SWITCH-ENTRY-POINTS: every gateway entry point records or refuses the rule switch

Card RULE-SWITCH-ENTRY-POINTS discharges `GATEWAY-RULE-API-OTHER-ENTRY-POINTS`
([plan](../../plans/pluggable-cards/README.md) seam B). Base `98b1bb785`.

`QCORE_GATEWAY_RULES_OFF` switches gateway rules off at run time. GATEWAY-RULE-API
made the audit routes record the effective list (`rule_set.off` in every report
component, and `invocation.json` for `current` and the configuration route).
Every other binary that links the gateway read the list silently: a result made
with rules off could be published without saying so. Now every entry point
either records the list in what it publishes, or refuses a non-empty list
before it starts. A malformed or non-switchable list (unknown id, hold,
malformed element) is refused everywhere, as before.

## 1. The entry points and how they were found

Three methods, each checked against the others:

1. **Link reachability** (`cargo metadata` of both workspaces, and the same
   closure computed from the tracked manifests by the new test). An entry
   point is any binary or example of a workspace member. It can consult the
   gateway's route tables or registry only if `qcore-c-gateway` is in its
   dependency closure: normal dependencies for a binary, plus its own
   package's dev-dependencies for an example.
2. **The binaries' own bytes.** The gateway's switch module puts the variable
   name into every binary that links it. This is the advertisement
   `quirin-audit measure` checks (`sides::advertises_rule_switch`). Debug builds:
   `quirin-audit` (4 occurrences), `quirin-dump-checkpoint` (2), `quirin` (2)
   and `quirin-audit-measure` (2, it only sets the variable for its switched
   sides). `quirin-audit-libevent` and `quirin-audit-example` have 0.
3. **Dispatch and writers inside `quirin-audit`.** `subcommand_plugins::BUILTINS`
   is held to `main.rs`'s dispatch by an existing test. For each built-in I
   read the route to its output writer. Only `Report::write_with_revision_key`
   calls `report_components::provenance`, which records `rule_set.off`. It is
   reached from `current` (`current.rs`), `pinned` (`pinned_route.rs`),
   `contracts` (`no_build.rs`) and the configuration route (`lib.rs`). Every
   other built-in writes its own JSON with no rule set. The host clears a
   subcommand plugin's environment except its allow-list
   (`quirin_audit_plugin::ENVIRONMENT_ALLOW_LIST`), so a plugin never saw the
   variable at all: it was dropped.

**Entry points that link the gateway (40):**

| Entry point | Publishes the list? | Now |
|---|---|---|
| `quirin-audit CONFIG.json` | components + `invocation.json` | records (unchanged) |
| `quirin-audit current` | components + `invocation.json` with `--report` | records; refuses a non-empty list without a report |
| `quirin-audit contracts` | components with `--report` | records; refuses without a report |
| `quirin-audit pinned` | components with its diagnostic report (`run_pinned_audit.py` also records `run.json`) | records; refuses without the diagnostic report |
| `quirin-audit prepare --run` | through the chained `current` | records; `prepare` without `--run` refuses |
| `quirin-audit` `atomic-publication`, `callback-task-handoff`, `callback-task-handoff-fixture`, `checkpoint`, `compare`, `cycle18-composition`, `cycle18-obligations`, `event-payload`, `native-platform-matrix`, `observed-wait`, `pin`, `plugin`, `timeout-teardown`, `wait-failures` | no (some run the gateway's lowering, `compare` classifies holds) | refuses |
| `quirin-audit NAME` for a subcommand plugin, `--list-plugins`, `--plugin-path ...` | no (the environment is cleared) | refuses (use `measure --rules-off`) |
| `quirin-dump-checkpoint` | no | refuses |
| `quirin` (`qcorec`) | no | refuses |
| 37 examples: `qcore-audit` (12), `qcore-c-gateway` (7), `quirin-source-analysis` (7), `qcore-compiler` (11) | no | each refuses as its first statement |

**Entry points that do not link the gateway:**

- `quirin-audit-measure`: records each side's list in its record and ledgers.
  After the run it checks that every switched side's components record exactly
  that list and every other side's record none (`rules_off_mismatch`). It only
  receives the variable through the host, which now refuses it.
- `quirin-audit-libevent`, `quirin-audit-example`.
- The provider workspace's binaries: the five `qcore-provider-*` binaries,
  `quirin-plugin`, `quirin-plugin-example-checker` and the three conformance
  binaries. Conformance links the gateway only as a dev-dependency, so only its
  tests link it.
- Examples of `qcore-contract` (12), `qcore-core` (1) and `qcore-frontend` (2).

Python drivers start these binaries and inherit their behaviour.
`run_pinned_audit.py` also records the variable in `run.json`.

## 2. What changed

- `qcore_c_gateway::gateway_rules::refuse_unrecorded(entry_point)` returns the
  refusal of a refused list, or of any non-empty list. The message names the
  entry point, the list and the routes that record it.
  `exit_if_unrecorded(entry_point)` prints it and exits 1. No switch parsing
  changed.
- `qcore_audit::rule_switch_entry` (new). `RECORDING` (`contracts`, `current`,
  `pinned`, `prepare`) and `UNRECORDED` (the 14 others) partition `BUILTINS`,
  and a unit test holds the partition. `admit` replaces `main.rs`'s bare
  `check()`. A recording command line, or the configuration route, gets the
  old check. `prepare` without `--run`, an unrecorded built-in and a plugin
  command line get `refuse_unrecorded`.
- `current_cli.rs` and `no_build.rs` refuse when the resolved report is `None`.
  `pinned_cli.rs` refuses when the diagnostic report is absent.
- `quirin-dump-checkpoint`, `quirin` (through the new re-export
  `qcore_compiler::gateway_rules`) and the 37 examples call
  `exit_if_unrecorded` first.
- `docs/development/audit-subcommand-plugins.md`, `--rules-off` row: says where
  a hand-set variable is honoured.

**Lean:** none. No verdict semantics or semantic law changed. With nothing
off every changed site is the old code path. With a list, a non-recording
process now refuses before it analyses anything. That is a process-level
admission, not a verdict rule.

## 3. Tests

- `crates/qcore-audit/tests/rule_switch_entry_points.rs` (4 tests):
  - `every_gateway_entry_point_admits_the_switch`: the census above, from
    tracked manifests and sources (rule 29). Every entry point that links the
    gateway must start `main` with `gateway_rules::exit_if_unrecorded(` or
    `refuse_unrecorded(`, or call `rule_switch_entry::admit(` before its first
    dispatch. A floor keeps the census from passing by finding nothing: at
    least 40 entry points, named linked ones (the three binaries, an example
    of each crate), and named unlinked ones (the plugins, a provider binary, a
    conformance binary whose package dev-depends on the gateway, a
    `qcore-contract` example). A new gateway entry point without an admission
    fails this test. Mutation run once by hand: deleting the hook from
    `examples/premise_graph.rs` failed it, naming the file. The file was then
    restored with `git checkout`.
  - `the_checker_catches_each_missing_or_misplaced_admission`: four admitted
    mains and nine mutants. The mutants have no hook, a hook in a line comment,
    a hook in a block comment, the hook after a statement, the hook only in
    another function, the hook in a raw string, the hook in an escaped string,
    `admit` after a dispatch, and no `main`.
  - `every_unrecorded_builtin_refuses_a_switch_list_and_a_recording_one_starts`:
    - each of the 14 unrecorded built-ins refuses `35` (exit 1, empty stdout)
      and refuses `35,no-such-rule`, and runs as before with the variable
      unset, empty or blank;
    - `prepare` without `--run` and three plugin command lines refuse;
    - `current`, `contracts` and `pinned` are admitted;
    - `pinned` without a diagnostic report refuses.
  - `the_checkpoint_dump_refuses_a_switch_list`: refuses `35` and a malformed
    list, and runs with the variable unset or empty.
- The recording routes, run under a list, beside their fixtures (Clang
  present, none skipped):
  - `no_build_cli.rs`: `contracts` with `35,10s` records `["10s","35"]` in
    every component and `[]` with nothing off. Without a report it refuses
    before analysing; a refused list is refused.
  - `current_cli.rs`: `current` with `35` records `["35"]` in every component,
    in `invocation.json` `environment` and in `gateway_rules_off`. Without a
    report it refuses `35` and runs with nothing off.
  - `configured_cli.rs`: a configuration run with `10s, 35` and with an empty
    list records it in components and invocation.
- `crates/qcorec/tests/cli.rs`: `quirin check` refuses `35` and
  `35,no-such-rule`, and runs with the variable unset or empty.
- `rule_switch_entry` unit tests (2): the partition, and the classification of
  built-ins, plugins, `prepare` and configuration paths.
- By hand: `reevaluation_probe`, `premise_graph`, `member_path_probe` and
  `loop_owner_census` (one example per linking crate) each exit 1 with the
  refusal under `QCORE_GATEWAY_RULES_OFF=35`. With the variable unset, the
  gate runs both reevaluation examples (exit 0).

## 4. Identity

With an empty list no published byte can change:

- The recording routes' writers are untouched.
- `admit` on a recording command line is the old `check()`.
- Each new refusal returns `Ok(())` when the list is empty, so every other
  entry point runs its old code.

Only refusal messages are new, and only a non-empty list reaches them. No
measure identity run is needed. The targeted artifact diff:

- Fixture: two units under `tree/` (`plain.c` with `plain` and a conditional
  leak `leaky`; `literal.c` with a returned literal, a returned member and
  `configured`) and one provider crate holding `alloc`/`dispose` heap
  contracts. These are the `no_build_cli.rs` fixture's contracts.
- Command: `quirin-audit contracts --source-root tree --load-path
  providers/contract-load-path.json --report out/report.json --jobs 1`, run
  with the base build (`98b1bb785`, debug) and the tip build (debug), same
  directory, same argv, Clang 21.1.8.
- Result: all four artifacts (`report.json`, `.components.json`,
  `.contracts.json`, `.invocation.json`) are **byte-identical**, with the
  variable unset and also with `QCORE_GATEWAY_RULES_OFF=35` (a recording
  route's switched output is unchanged too).
- [`identity/artifact-sha256.txt`](identity/artifact-sha256.txt) lists the 16
  digests. Under `35` only the component sidecar differs from the unswitched
  run, by `rule_set.off = ["35"]`.

The recording routes' outputs under a list are also covered by the tests in §3.

## 5. Boundaries

- The census is at link level. An entry point that links the gateway but
  never reaches a route table (`quirin-dump-checkpoint`, `checkpoint`, `pin`,
  `plugin views`) now refuses a non-empty list too. That is stricter than
  needed, and fail-closed. The scan checks that an admission is present, not
  that a recording entry point really records: `rule_switch_entry::RECORDING`
  is the reviewed list of those, and its routes' recording is held by the
  route tests in §3.
- The census sees Cargo's automatic and declared bin/example targets of
  workspace members. Integration test binaries are not entry points and set
  the variable themselves (the switch tests).
- `contracts` and `pinned` record the list only in components, not in an
  invocation record. This is unchanged: GATEWAY-RULE-API chose the component
  as the field `measure` verifies.

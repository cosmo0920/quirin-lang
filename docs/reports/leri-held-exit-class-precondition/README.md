# LERI-HELD-FORMAL-LINK-OBLIGATION: the link precondition without the unlink, per exit class

Card LERI-HELD-FORMAL-LINK-OBLIGATION (the residual FB-RETRY-OR6-ELEMENT-IDENTITY
left; user decision 2026-11-27: discharge it with a contract schema field).
Base `0956f4d21` (r80). Gateway rule **91** amended in place (new part (5),
no new id); contract specification §C04.8 amended; schema field
`ResourceFunctionContract::link_preconditions`; declared spelling `requires
resource_links_valid(p, MEMBER)`; Lean `QCore.ExitClassLinkPrecondition`.
Commits: probe `5ffea5579` (base verdicts pinned),
[prediction](PREDICTION.md) `d6cb61949`, semantic `9a6a85c71` (the measured
tip), `52d291956` (the measured blocker pinned, artifacts).

## Headline

* **The schema field exists and is checked end to end.** A contract may
  state `resource_links_valid(p, MEMBER)`: the caller owes `p`'s `MEMBER`
  links valid on entry, and the clause says nothing about the exit (the
  position's own mode is the exit effect). W4 attaches it where rule 91's
  destroyer clause cannot be stated, so rule 11r's exit-specific route now
  summarises a body that destroys its formal on one exit class and stores or
  keeps it on another: the destroy class is an element destroyer, the store
  class is `Consumed` (or `Owned`) beside the precondition, never `Released`.
  Every caller checks it on every class before the call. On the fixtures the
  enqueue shape, its kept variant, a never-returning destroy path and two
  callers move Unknown → Safe; every other caller refuses at a named leaf.
* **No native row moved** (fluent-bit-system route and the census units,
  base `0956f4d21` against tip `9a6a85c71`, each from its own `git archive`
  tree and release binary): 116 / 0 / 327 / 5 on both sides, zero rows with a
  changed status or leaf. **The prediction was wrong**: it expected
  `flb_output_task_queue_enqueue` Unknown → Safe. In the real body the failure
  path calls `flb_errno()` (`flb_errno_print(errno, __FILE__, __LINE__)`)
  before `flb_task_retry_destroy(retry)`; `errno` is `(*__errno_location())`,
  a nested call and a non-name actual, which rule 46's kill policy (shared by
  rule 91) reads as reaching every object, so the carried record is forgotten
  before the destroyer and no class validates. A fixture pins that shape
  (`errno-before-the-destroyer`, Unknown). Every other census row is blocked
  earlier still (section 4).

## 1. Mechanism

`crates/qcore-c-gateway/src/legacy_carried_link.rs` (rule 91's module):

1. **Schema and loader.** `link_preconditions: BTreeMap<usize, String>`,
   omitted when empty (every existing contract serialises byte for byte as
   before), not part of the custody projection, and counted by
   `has_container_effect`, so every consumer that does not check it refuses
   the call. Declared as `requires resource_links_valid(p, MEMBER)` at an
   owned, consumed or released pointer position with no container effect,
   once per parameter; rejected beside a §C04.13 disposition and beside every
   clause a container clause is rejected beside (`Collected::is_empty`).
2. **The body.** The entry seeds `Owed(MEMBER)`, which rule 43's held arm
   admits and spends and rule 46's kill policy forgets exactly as rule 91's
   `Ready`. At an exit a formal the precondition alone carries reads `Mixed`
   when the carried destroyer took it (the clause states no unlink), its
   disposition otherwise (`observed_for`).
3. **The summary.** `infer_cfg_summary` keeps rule 91's candidate first;
   only when its fresh run fails does it widen the candidate with the
   precondition at each carried formal it binds but does not release
   (`attach_preconditions`) and check that by another fresh run. A formal
   handed directly to a position carrying the clause is a candidate too, so
   the obligation carries on to the caller's own formal. Rule 11r's
   exit-specific route runs this per returned value; its summary publishes
   only when every class states the same entry obligations
   (`classes_agree`, a hook in `infer_exit_specific`).
4. **The caller.** In the transfer, before the call's own reading
   (`check_preconditions`): the actual is an object the caller published, one
   it holds with its own `MEMBER` initialisation, or its own formal carried
   with `MEMBER` (not spent; the check spends nothing). Anything else, a
   foreign element included, refuses the call. Route G reads one class per
   edge from the same pre-call state, so the check runs on every class.

**Hook points in shared files:** `legacy_transfer.rs` (one call before rule
34's `apply_destroyers` in the `ContractCall` arm), `legacy_summary.rs` (the
widened second fresh run at the end of `infer_cfg_summary`),
`legacy_relational_summary.rs` (`classes_agree` in `infer_exit_specific`),
`legacy_container_publication.rs` (`operands` lowers nothing for a
precondition alone), `contract_clause_standing.rs` (the clause's label),
`qcore-schemas/src/contract.rs` (the field), `qcore-contract/src/contract_loader.rs`
and `contract_counted_users.rs` (the requirement and its placement).

## 2. Controls

`crates/qcore-c-gateway/tests/link_member_premises.rs`, module
`carried_link_obligation`, `exit_class_fixtures` (base pinned at
`5ffea5579`), and `the_loader_reads_the_link_precondition`:

| Fixture | Base | Tip | Refusal at tip |
|---|---|---|---|
| enqueue (destroy on `-1`, store on `0`) | Unknown | **Safe** | |
| enqueue keeping `r` on `0` | Unknown | **Safe** | |
| destroy path never returns (ordinary route) | Unknown | **Safe** | |
| caller: own `_head` initialisation, tests the result | Unknown | **Safe** | |
| caller: its own formal, carried again | Unknown | **Safe** | |
| caller: not initialised | Unknown | Unknown | precondition refused; rule 46 held refusal on `-1` |
| caller: initialises `_rotate` only | Unknown | Unknown | precondition refused |
| caller: foreign element through `_head` | Unknown | Unknown | precondition refused (a foreign element does not discharge it) |
| caller: published object | Unknown | Unknown | transfer of a published object on class `0` |
| caller: result not tested | Unknown | Unknown | relational-untested |
| caller of a declared precondition, initialised | (not pinned) | **Safe** | |
| caller of a declared precondition, not initialised | (not pinned) | Unknown | precondition refused |
| declared `released` + precondition, body uses the destroyer | (not pinned) | Unknown | the release is through an unlink the clause does not state |
| frees `r` with `q_free` on `0` | Unknown | Unknown | held refusal |
| writes `r` before the destroyer | Unknown | Unknown | held refusal |
| `errno`-style report before the destroyer | — | Unknown | held refusal (the measured blocker) |

The loader test accepts owned, consumed and released positions and rejects
a borrowed position, a precondition beside an element destroyer's unlink, a
duplicate, a malformed arity, an unknown parameter and a §C04.13 module. No
fixture is accused. The unit tests carry the Lean vectors
(`PRE_OBSERVE_VECTORS`, `PRE_DISCHARGE_VECTORS`), the `Owed` record's
admission, spending and kill, and the attachment and class agreement.

## 3. Lean

`proof/QCore/ExitClassLinkPrecondition.lean`, audited by
`proof/AuditExitClassLinkPrecondition.lean` (13 theorems, axioms `propext`
and `Quot.sound` only, no placeholder). A precondition-only formal is
observed released only when the destroyer did not take it
(`a_precondition_release_is_not_the_destroyers`); rule 91's formals read as
before (`a_destroyer_clause_reads_as_rule_91`); every observation only
refutes. Classes that disagree publish nothing
(`disagreeing_classes_publish_nothing`); when they agree, one discharge at
the pre-call heap establishes every class's entry seed
(`one_discharge_seeds_every_class`, through rule 91's
`the_seeded_record_holds`). Each caller discharge establishes the
precondition (`the_precondition_discharge_establishes_it`, with
P-PUBLISHED-LINKED as a hypothesis); a foreign element or a spent record
does not; the check spends nothing. `vectors_hold` equals the Rust vectors.
The candidates' reading, the kill policy, every consumer's check or refusal,
route G's per-edge reading and the Rust correspondence are trust boundary
28's.

## 4. Measurement

**Routes.** The fluent-bit-system route ([scripts/system.sh](scripts/system.sh),
driven by [scripts/side.sh](scripts/side.sh) from each revision's own
`git archive` tree and release binary) over this host's preparation
`private/fb-046accf6`; and the same command over the census units the system
route does not read (`flb_input.c`, `in_tail`'s `tail_file.c` and
`tail_fs_inotify.c`; [scripts/census-units.sh](scripts/census-units.sh),
[scripts/census-selection.json](scripts/census-selection.json)). The opt-in
current route reads none of the census units (FB-RETRY-OR6-ELEMENT-IDENTITY
section 5) and was not rerun. Pinned routes (`quirin-audit measure
--declared`) cannot run on this host (no published live-route inputs);
nothing is fabricated for them. Base binary `0d3904d3…`, tip `a4569940…`;
raw reports in `private/m/{sys,census}-{base,tip}` (all exit 0), digests in
[artifacts/digests.txt](artifacts/digests.txt).

| | Safe | Pending | Unknown | Unsafe |
|---|---|---|---|---|
| system, base and tip | 116 | 0 | 327 | 5 |
| census units, base and tip | 34 | 1 | 165 | 0 |

No row moved and no row's leaves changed on either route
([artifacts/sys-changed.txt](artifacts/sys-changed.txt) and
[artifacts/census-changed.txt](artifacts/census-changed.txt) are empty). The
null property's custody, null and combined readings are identical; the
records differ only in `hypothesis_context` (evidence version strings).

**Every census row, with the leaf that still blocks it** (base and tip
identical):

* `flb_output_task_queue_enqueue` (`flb_output.c`): `flb_errno()` before the
  destroyer, read by rule 46's kill policy as reaching every object (a
  nested call, `*__errno_location()`, and a non-name actual), so the
  carried record dies and the held refusal stands
  (`unlink by flb_task_retry_destroy of retry, which this function has not
  published`, through the fallback).
* `flb_output_net_property_check`, `flb_input_net_property_check`,
  `flb_input_oauth2_jwt_property_check`, `flb_input_http_server_property_check`:
  `ins->net_config_map = …` (an indirect memory access, unmodelled) before
  the destroyer.
* `flb_input_plugin_property_check`: uncontracted calls, indirect accesses,
  a loop re-acquisition and a member store through `ins` before the
  destroyer.
* `flb_output_enable_multi_threading`: the uncontracted
  `flb_output_thread_pool_create(config, ins)` in the branch condition.
* `flb_task_retry_reschedule`, `flb_engine_dispatch_retry`: uncontracted
  calls (`flb_sched_request_create`, `flb_task_retry_reschedule`, …) and
  refuted callees before the destroyer.
* `check_purge_deleted_file`, `reconcile_file_state` (`in_tail`): the
  destroyer `flb_tail_file_remove`'s pinned contract is not in force
  (contract-pin-mismatch), so no destroyer is applied at all.

**Prediction against measured.** Predicted: `flb_output_task_queue_enqueue`
Unknown → Safe (Safe +2, Unknown −2) and a leaf trade in
`flb_output_task_singleplex_enqueue`. Measured: nothing. The prediction read
the base leaves but not the body's failure path, whose `flb_errno()` the
fixture lacked. No Unknown → Safe, no Safe → Unknown, no accusation added or
removed.

## 5. Gate

See [gate.json](gate.json) (scoped card gate, `--base 0956f4d21`, through
`private/toolchain/env.sh`; logs `private/gate/`). LLP64 fixture scan over
the changed test files: exit 0 (0 counted, 0 found), run as the archived
`docs/reports/archive/language-and-tooling/windows-ci-llp64-fixtures-2/scripts/llp64_fixture_scan.py
--no-repository-contracts --contracts providers --contracts
crates/qcore-c-gateway/tests/fixtures --stdin` (the path
`docs/development/windows-types.md` names predates the archive move, and the
archived script's repository-root lookup resolves to `docs/reports/`; the
flags pass the same contract directories it would read).

## 6. Follow-ups

[integration-amendments.json](integration-amendments.json) amends
LERI-HELD-FORMAL-LINK-OBLIGATION: the schema field, its body, summary and
caller sides are discharged; what remains is that no measured row reaches
its destroyer with the carried record alive, each for the leaf in section 4.
The nearest is the enqueue's `flb_errno()`: a kill-policy refinement for a
contracted call whose non-name actuals and nested calls provably hand no
pointer to the object (a scalar read through `__errno_location()`, string
and integer literals) would let that row's classes validate. It registers
nothing new; `profiles: []` (no evaluation baseline changes).

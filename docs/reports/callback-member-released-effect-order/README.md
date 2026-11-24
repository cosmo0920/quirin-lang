# CALLBACK-MEMBER-RELEASED-EFFECT-ORDER: a candidate's member effect after the release

* **Base:** main `3ca85c7a6` (r65). **Card:** the registered follow-up of
  CALLBACK-MEMBER-RELEASED-REQUIREMENT-FOLD. USER DECISION 2026-11-24:
  "launch them all".
* **Commits:** [PREDICTION](PREDICTION.md), fixture and mutation evidence
  `9e0eb8118` (before any implementation); Lean module, rule 74p, C04.19.6
  paragraph and the table probe `f133d7b5c` (the measured tip); a text
  correction (the table form composes nothing) `e7ea41f47` (the gated tree);
  this report.

## 1. Soundness finding

**The plain member form is already fail-closed. No refusal is added and no
gateway source changes.** The transfer (`legacy_transfer.rs`) applies
`apply_contract_call`, which includes the release, and then
`callback_route::apply`, so the callback route composes a candidate's
member-path states after the release. In C the callback runs first. Two facts
from the base make the late order sound:

* The release of site `s` (`resource_action`) writes `s`'s status, the
  statuses and `contained` entries of what `s` contains
  (`cascade_resource_action`), and the retentions `s` holds
  (`holder_released`). It never writes `field_links` (premise P-B08-SHALLOW),
  `constructed`, `registries` or `pointers`. So the composition reads the
  released object's links, and a deferred member reading reads its
  constructed registration, exactly as they were before the release.
* `legacy_member_path_route::apply` takes a linked object (`gone`) only
  while it is live (`if single && live`). Otherwise it refuses
  `member-path-undecided`.

So where the composition and the release touch disjoint sites, the two
orders commute. Where a linked object is one the release itself releases
(the object, or one it contains), the late order reads it as released and
refuses; in C that case is a double release. The guards of the release only
refuse while something is live, and the composition only lowers liveness. So
whenever the late order admits, the C order admits too, with the same state
(Lean `late_is_the_c_order`). The late order is stricter, never looser
(`the_late_order_is_stricter`). No `Unknown` becomes `Safe` through the order.

r60's refusal for the slot form (`callback-beside-release`) stays as it is.
It is stricter than this card needs, but it is fail-closed, and r60's
admission depends on it.

## 2. Census: forms that can name a released object beside a candidate effect

| Form | Candidate effect applied after the release | Reading |
|---|---|---|
| plain `resource_callback_member_invoked(OBJ released, …)` (12n) | candidates' member-path states (`member_paths`) | sound by §1 (`rd_kafka_op_destroy`) |
| the same, closure refused and object carrying a constructed registration (13w, 14e) | `member_uses`: `decide_member` reads `constructed`, then the same composition | sound by §1: the release writes no `constructed` |
| member invocation on another object handing the released one as an actual | `member_paths` at that position | sound by §1 |
| `resource_callback_invoked` / `_or_default` handing a released actual (13u) | `member_paths` | sound by §1 (fixture `c_op_run`) |
| `resource_callback_table` handing a released actual (13v) | **none**: the table branch collects no member-path state | out of scope; follow-up CALLBACK-TABLE-MEMBER-PATH-STATES (§6) |
| `resource_callback_slot_invoked(OBJ released, …)` (r60) | any effect refuses `callback-beside-release` | unchanged |
| `resource_callback_registry_invoked` (13w) on a released holder | `ReleasesElements` cascade of the holder's elements | the release itself refuses while the registry holds elements (`refuse_release_of_registry`); with none, the cascade is empty in either order. A requirement refuses `callback-effectful` |
| the declaration's own arms (12o) and helper-destroyer states (rule 51, `mk_thread_destroy`) | applied after the release | not candidate effects; they read the same links and liveness, so §1's argument covers them |
| reference drop beside a slot (OPENSSL-FREE-SESSION-REMOVE-INVOCATION) | refuses `callback-beside-drop` | unchanged |

The shipped member invocations through a released object are only the plain
form, both non-null: `rd_kafka_op_destroy` (with arms) and
`mk_thread_destroy` (its own states). The fixture's control test loads them.

## 3. Changes and pins

* `crates/qcore-c-gateway/tests/callback_member_released_effect_order.rs`
  (new target, 6 tests). It uses rule 30's `member_paths.qv` plus a
  synthetic module (`c_op_free`: a plain member release with no arms;
  `c_op_run`: the synchronous form; `c_op_keep`: stores an item into the op's
  custody).
  * `a_plain_member_release_composes_its_candidate_after_the_release`: a
    `#released` payload is `Safe`; a kept payload is `Unsafe` (leaked).
  * `a_linked_object_the_release_contains_is_never_safe`: a payload the op
    also contains refuses `member-path-undecided` in the plain member, arms
    and synchronous forms.
  * `a_path_designating_the_released_object_is_never_safe`: at this base the
    store `o->self = o` is itself refused (indirect access), so this case
    reaches no composition. The Lean model covers it
    (`order_vectors_hold`, `late 0 [] [] [0]`).
  * `a_synchronous_invocation_through_a_release_composes_after_it` and
    `the_arms_member_release_reads_as_rule_30`.
  * `the_shipped_member_releases_are_the_plain_form`: the controls.
* **Pins at base.** No analysis source changes, so the pins hold at base
  ([`evidence/base-fixture.log`](evidence/base-fixture.log), 6 passed). To
  show that the pin holds the property the order relies on, the shipped
  harness [`scripts/mutation.py`](scripts/mutation.py) drops the liveness
  read (`if single && live` → `if single`). Its logs are in
  [`evidence/mutation/`](evidence/mutation/). Mutant: exit 101, and
  `a_linked_object_the_release_contains_is_never_safe` fails with
  `c_op_free(o)` **`Safe`**: the late order would certify a double release.
  The file is restored by rewriting its bytes (sha256 `d1bc43ab…` round
  trip asserted). The baseline run after the restore exits 0. That is 1
  mutation, killed.
* [Rule 74p](../../agents/rules/74p.md) (new; generated
  `docs/agents/gateway.md` and `docs/agents/README.md`). C04.19.6 gains a
  "Through a released object" paragraph.
* Lean `QCore.CallbackReleasedEffectOrder` with
  `proof/AuditCallbackReleasedEffectOrder.lean` (10 theorems; axioms
  `propext`, `Quot.sound`; no `sorry`, no project axiom; `lake build` and
  the audit in `private/logs/lean-2.log`, `lean-audit-2.log`). Generated
  `proof/QCore.lean`. It models liveness and links; `release s D G` and
  `compose C` are modelled as the Rust code decides them. It proves
  `the_release_writes_no_link`, `late_admits_only_disjoint`,
  `late_is_the_c_order`,
  `the_late_order_never_admits_what_the_c_order_refuses`,
  `a_link_the_release_releases_refuses`, `a_kept_path_changes_nothing`,
  `the_late_order_is_stricter` and `order_vectors_hold`, with the lemmas
  `kill_comm` and `kill_lowers`.
* No provider contract changes, so no lock changes.

## 4. Measurement against the prediction

The opt-in run is `quirin-audit current --monkey-linked` over Fluent Bit
046accf6 (`private/fb-046accf6`), driven by [`scripts/run.sh`](scripts/run.sh).
These are r65's scripts with this worktree's paths. Each side is a
`git archive` tree with its own release binary and its own
`package.py checkers`. Outputs are in `private/cmeo/run-{base,tip}` (not
committed). The two runs overlapped each other and the gate, so their wall
times are not comparable.

| Run | Tree | Binary sha256 | Exit (build / checkers / run) | Safe / Pending / Unknown / Unsafe |
|---|---|---|---|---|
| base (00:12-01:50Z) | `3ca85c7a6` | `dcc4a4a2...9062e95f` | 0 / 0 / 0 | 1171 / 2 / 3623 / 8 |
| tip (00:29-01:59Z) | `f133d7b5c` | `54ad8729...563d4ff4` | 0 / 0 / 0 | 1171 / 2 / 3623 / 8 |

The [totals](evidence/totals-base.json) ([tip](evidence/totals-tip.json))
are equal, and so are the indirect-access figures (2118 rows, 215 alone,
8992 leaves). [`moved-rows-base-tip.jsonl`](evidence/moved-rows-base-tip.jsonl)
is `{"moves": {}}`, and [`changed-rows-base-tip.txt`](evidence/changed-rows-base-tip.txt)
is empty. The consoles differ only in preparation timings. **Controls:** the
112 rows whose diagnostics or dependencies name `rd_kafka_op_destroy` or
`mk_thread_destroy` are 104 `Unknown` and 8 `Safe` on both sides.

**Against the prediction:** 0 status moves and 0 content changes, as
predicted. One stated expectation missed: the prediction expected r65's tip
totals (1172 / 2 / 3622 / 8) on both sides. The base `3ca85c7a6` measures
1171 / 2 / 3623 / 8. r65's figures were taken at its own base and tip
(`2c60986b0`, `9180fc26f`), before the r65 landing and its amendments, so
this difference predates this card, and both of this card's sides agree.

**Pinned routes:** not run. This host has no `build/cycle19` inputs or
baselines, so `quirin-audit measure --declared` cannot run here, and no
result was fabricated. The prediction's `measure-prediction` block (none on
corpus, system and slice) is for the integrator.

## 5. Gate

**Record:** [`gate/gate-2.json`](gate/gate-2.json). **Command:** `sh
private/toolchain/env.sh python3 scripts/gate_scope.py --base 3ca85c7a6
--out private/gate-2` (Clang 21, Lean 4.35). The log is
`private/logs/gate-2.log`. The tree was `e7ea41f47`, with this report's
README and gate record uncommitted (docs only). Scoped run, Lean all, no
provider gate (`providers/` unchanged). **Exit 0, 10 steps passed.**

| Step | Exit | Counts |
|---|---:|---|
| submodules, `fmt` | 0, 0 | `third_party/libevent` checked out |
| `clippy` (5 package slots) | 0 | |
| Rust suite (scoped) | 0 | 12 targets; 61 passed, 0 failed, 0 ignored |
| Lean (`check_lean_audits.py`) | 0 | 497 audits |
| Python suite | 0 | 807 tests |

**Earlier attempt:** `private/gate-1` (tree `f133d7b5c`) was stopped a few
minutes in, by its task handle, to correct the table text. It is retained
and discarded as evidence.

**Outside the gate** (`private/logs/`): the fixture runs (`fixture-1..4.log`;
1 was a contract load error and 2 to 4 passed), the table probes
(`probe-table-1.log`, `probe-table-2.log`), and `lake build` with the
audit's axioms (`lean-1.log`, `lean-audit-1.log`, `lean-2.log`,
`lean-audit-2.log`).

## 6. Conflicts and follow-ups

**Files likely to conflict:** `docs/QCORE_CONTRACT_SPEC_r0_3.md` (C04.19.6,
after the join paragraph). Generated files, to be resolved by rerunning their
generators: `docs/agents/gateway.md`, `docs/agents/README.md` and
`proof/QCore.lean`. `legacy_retained_by.rs` is not touched, so there is no
conflict with FRAME-HOLDER-SHADOWED-NAME.

**Follow-up registered:** CALLBACK-TABLE-MEMBER-PATH-STATES.
`callback_operands` collects candidate member-path states for the
synchronous and member forms, but not for `resource_callback_table`.
`classified_in` reads such a candidate as its custody contract, so its state
is dropped. The probe
([`evidence/probe-table.rs.txt`](evidence/probe-table.rs.txt),
[`probe-table.log`](evidence/probe-table.log); measured once, over synthetic
callees) links an item, dispatches a table holding `c_cb_releases` (states
`op.payload |-> #released`) and then frees the item. The result is **`Safe`**.
The same candidate through the synchronous form is `Unsafe`. No shipped
table candidate is known to state member paths, so no shipped row is shown to
move. This is outside this card's ordering scope.

**Increment paragraph** (in [`integration-amendments.json`](integration-amendments.json)):
it retires CALLBACK-MEMBER-RELEASED-EFFECT-ORDER and registers the follow-up
above. It has no `status_edits`: no headline figure moves, and the Lean audit
count is the landing gate's figure.

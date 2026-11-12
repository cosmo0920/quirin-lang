# LIST-ENTRY-RELEASE-IDENTITY: the released list element and its summary

Card LIST-ENTRY-RELEASE-IDENTITY (Fluent Bit queue, P1.3 retry route and the
backlog route). Base `74105dd0e`. Gateway rule **43** (new); rules 10f(3)
and 34 amended; contract specification §C04.8 amended; schema
`ContainerEffect::ElementsReleased` (qcore-schemas **25.0.0**); Lean
`QCore.ElementReleaseSummary`; ten Fluent Bit destroyer declarations gain
`resource_unpublished`. The [prediction](PREDICTION.md) and the census were
committed at `0ba8ff758`, before any tip measurement (the gateway code had
been drafted, unmeasured; the prediction says so).

## Headline

* **Census.** The dominant generic refusal shape at a release is the
  **element of a formal-rooted list released by a declared destroyer**:
  system 50 of 160 destroyer leaves (10 rows), slice 11 (5 rows). Rule 10f(2)
  already gives such an element an identity, but only at an unlink, and no
  declared destroyer stated its unlink, so its release read an unbound name.
* **Mechanism.** (A) the ten destroyers whose bodies unlink their object
  (read at the three trees, [unlinks.json](artifacts/unlinks.json)) declare
  `resource_unpublished`, so rule 34 takes the element's custody first; the
  loader now admits that clause beside rule 30's member-path states, which
  seven of them carry (a generic prerequisite the census found blocking 34 of
  the 50 system leaves and all 11 slice leaves). (B) rule 43 part 1: a
  destroyer may take an object the path already holds, so formal hand-offs
  keep their release. (C) rule 43 part 2, the released-element summary
  (W45-FOREIGN-UNLINK-SUMMARY): a body whose element custody is rooted at its
  formals publishes `ElementsReleased` there; callers forget their published
  objects and carry the clause on.
* **Measured (this card's preparations, binaries `2136b319…` base and
  `07764b22…` tip):** system custody 103 / 4 / 341 → **107** / 4 / **337**
  (`flb_engine_destroy_tasks`, `flb_storage_input_destroy`, both
  configurations, Unknown → Safe); slice 58 / 1 / 341 / 4 Pending →
  **59** / 1 / **340** / 4 (`flb_tail_file_remove_all` Unknown → Safe). Exactly
  the predicted moves. No Safe row moved, no leak, double release or Unsafe
  added; the null property's tallies are unchanged on both routes. Arbiter
  comparison: section 6.
* **Part C reaches no native caller.** Every caller of a body that now
  publishes the clause on these routes is in another unit (or the body is
  still Unknown), and derived summaries do not cross units: measured,
  follow-up `LERI-CROSS-UNIT-ELEMENTS-RELEASED`. It is exercised by fixtures
  only.
* **OR-6** is unchanged (the mutually recursive pair publishes nothing
  first); `W45-FB-OR6-ELEMENT-IDENTITY` stays open.

## 1. Setup

The host had no Clang driver, no Lean toolchain and no prepared inputs. Clang
21.1.8 was extracted from the distribution's own `clang-21` and
`libclang-common-21-dev` packages (matching the installed `libclang-cpp21`)
into worktree-private `private/toolchain/`, Lean 4.35.0 through `elan` into
the same directory, and the three routes were prepared with this base's
`quirin-audit prepare` (`--clang` the extracted driver): system at
`046accf6`, the tail-pause slice at upstream `90978ac6` (the same tree as the
published synthetic export `34155955`, not that commit), the corpus at
`66910c10` with `providers/package.py build` for its provider packages
([inputs](artifacts/inputs.json)). The absolute figures therefore differ
from the published baselines (paths and the slice commit differ); every
comparison here is base against tip on the same inputs.

## 2. Census (criterion: pick the dominant generic mechanism)

[census.py](scripts/census.py) over the BEFORE reports
([system](artifacts/census-system-before.json),
[slice](artifacts/census-slice-before.json)); destroyer callees only (the
`flb_free`-family leaves are listed in the artifacts and are not this card's).

| Route | Shape | Leaves | Rows |
|---|---|---|---|
| system | unresolved identity, element of a formal-rooted list | **50** | 10 |
| system | unresolved identity, other actual (local, poisoned) | 42 | 13 |
| system | unlink of a formal's own object (W45-FORMAL-ENTRY-UNLINK) | 20 | 9 |
| system | indirect access, member-read actual | 16 | 6 |
| system | uncontracted undeclared teardown function | 16 | 4 |
| system | unresolved identity, formal | 10 | 4 |
| system | element of another container; unlink of one | 6; 8 | 2; 2 |
| slice | unresolved identity, other (`ctx` of `flb_tail_config_create`/`in_tail_init`) | 44 | 6 |
| slice | unresolved identity, formal | 16 | 8 |
| slice | unresolved identity, element of a formal-rooted list | **11** | 5 |
| slice | unlink of a formal's own object | 10 | 6 |
| slice | element-other; undeclared teardown; member | 5; 4; 2 | 3; 3; 1 |

The slice's largest group is one local (`ctx`), not a list entry: out of
scope. The element shape is the only list-entry shape both routes share.

## 3. Mechanism

**(A) Declarations** (`quirin-provider-fluent-bit`):
`ensures normal resource_unpublished(P);` beside the release of
`flb_sched_timer_destroy`, `flb_sched_request_destroy`, `flb_task_destroy`,
`flb_output_flush_destroy`, `flb_output_instance_destroy`,
`flb_input_instance_destroy`, `flb_input_coro_destroy`,
`flb_tail_file_remove` (teardown_destroy.qv), `flb_task_retry_destroy`
(task_users.qv) and `flb_input_chunk_destroy` (input_chunk_destroy.qv). Every
body calls `mk_list_del(&P->_head)` before `flb_free(P)` at every tree read
(`flb_task_destroy` only when the task is not an orphan, i.e. when a list
holds it; `flb_sched_request_destroy` after its `NULL` return). Not stated for
the five destroyers whose bodies unlink nothing of their own. `providers/contracts.py sync`
rewrote the inventory lock; the ten were re-pinned after review with
[pin.py](scripts/pin.py) at the trees they had been pinned at (section 7).

**(A′) Loader** (`contract_loader.rs`, §C04.8 amended): a declaration whose
only retaining clauses are member-path states may state the destroyer's
unlink of the formal it releases. The call is classified under its custody
contract (`retaining_call` → `classify_simple`), so rule 34's transfer arm
orders the unlink before the release and rule 30's states apply as before.
Every other container clause beside a retaining clause still rejects. Before
this, adding the clause dropped `teardown_destroy.qv` out of the crate's
resource view (measured by the sync: the module left `resource_view`).

**(B) Rule 43 part 1** (`destroyer_takes_held`): rule 34 also admits an entry
denoting one live, unpublished identity the path holds and no arriving path
unlinked (`AbstractState::unlinked`, union join). Without it the declarations
would have cost `flb_sched_timer_cb_destroy` its Safe row (a W4-bound formal
handed to `flb_sched_timer_destroy`); with it, that row and the body
acquisitions keep their release. An unlinked object keeps the refusal.

**(C) Rule 43 parts 2–3** (`required_effects`, `attach`, `gate`,
`forget_published`): see [rule 43](../../agents/rules/43.md).
`Foreign::roots` records the formals an admitted element's containers are
rooted at; `Operand::ElementsReleased` carries a callee's clause with its
root. The W4 gate accepts a body with element custody only when its contract
states exactly the required clauses; the attachment is made in
`infer_cfg_summary`, so every other inference route still refuses such a
body. A declared contract is compared with a derived summary on custody
projections only (the derived clause is never part of that comparison).

**Premises** (stated, not proved): P-CRR-MEMBER, P-CRR-LINKED (rule 10f),
P-LERI-CALLER-PUBLISHED, P-LERI-HELD-LINKS, P-LERI-LINK-MEMBER (rule 43).

**RETURN-POSITION-SOUNDNESS-PREMISE-ELIDED / -CONTRACT-ROUTE.** Nothing here
creates a projection-tracked `->` place: a foreign element and a held object
are plain names bound to sites, `forget_published` only withdraws sites, and
no `Stored`/`Removed` mode is added. `TrackedBasesAreBound` is not put at risk
by this card; both entries are unchanged (re-read, no amendment).

## 4. Controls

`crates/qcore-c-gateway/tests/element_release_summary.rs` (7 tests, one
switched child with `QCORE_GATEWAY_RULES_OFF=43`), module unit tests (the
Lean vectors), and the amended `element_destroyer.rs` and
`container_root_reach.rs`:

| Fixture | rule 43 on | off |
|---|---|---|
| drain helper (rule 34 over a formal's list) | Safe | Safe |
| its caller; through a derived local; through a wrapper; `drain_unlink(&q->done)` | **Safe** | Unknown (uncontracted) |
| object published, then the drain, then its unlink | Unknown (refused on `x`, no accusation) | Unknown |
| drain of a static list: its caller | Unknown (summary withheld) | Unknown |
| caller whose actual is a local list: its caller | Unknown (withheld) | Unknown |
| member-path destroyer over a formal-rooted element (loader, A′) | Safe | Safe |
| caller of a member-path drain | **Safe** | Unknown |
| body acquisition destroyed; formal handed to a destroyer, its caller | **Safe** | Unknown |
| held object unlinked, then destroyed | Unknown (rule 34's refusal) | Unknown |
| held object destroyed, then freed | Unsafe (double release) | Unsafe |

Amended pinned expectations (attributed, not weakened): rule 34's
`DESTROYING` row `q_alloc(); item_destroy(e)` Unknown → Safe and two entries
leave its "every entry rule 10f does not admit" list (rule 43 admits them);
rule 10f(3)'s two "publishes no summary" tests now accept the refusal on the
forgotten `x` (the caller still never reads `Borrowed` and accuses nothing).

## 5. Lean

`proof/QCore/ElementReleaseSummary.lean`, audited by
`proof/AuditElementReleaseSummary.lean` (20 theorems, `propext` and
`Quot.sound` only): the held arm is sound against a concrete link/free model
(`the_destroy_rule_is_sound`), the unlinked refusal is necessary
(`an_unlinked_object_has_no_defined_destroy`) and the arm only replaces a
refusal (`the_held_arm_only_replaces_a_refusal`); the gate is exact
(`the_gate_is_exact`) and static/other roots withhold
(`a_static_root_withholds_the_summary`, `an_other_root_withholds_the_summary`);
the clause is sound at the caller (`forgetting_covers_every_release`) and a
borrowed reading would not be (`the_borrowed_reading_would_trust_a_released_object`);
`vectors_hold` equals the Rust `ELEMENT_RELEASE_VECTORS`.

## 6. Native measurement and the arbiter

Quick runs (`private/route.sh`, the measure plugin's route environment, null
and outer projections on), base binary `2136b319…`, tip `07764b22…`
(commit `513719d31`):

| Route | BEFORE custody | AFTER custody | null (= both) |
|---|---|---|---|
| system (448 rows) | 103 / 4 / 341 | **107** / 4 / **337** | 211 Safe / 243 Unknown |
| slice (404 rows) | 58 / 1 / 341 / 4 Pending | **59** / 1 / **340** / 4 | 190 / 216 |

Every moved row ([system](artifacts/compare-system-tip1.json),
[slice](artifacts/compare-slice-tip1.json), [compare.py](scripts/compare.py))
was read:

* **Status moves (5 rows).** `flb_engine_destroy_tasks` (×2): its one leaf
  was `flb_task_destroy(task)` on a safe-iteration element of the formal
  `tasks`; rule 34 now takes it (A + A′). `flb_storage_input_destroy` (×2):
  `flb_input_chunk_destroy(ic)` on an element of `&in->chunks` (A).
  `flb_tail_file_remove_all`: two `flb_tail_file_remove(file)` over
  `&ctx->files_static`/`files_event` (A + A′). Each rests on the declaration
  (a declared assumption, pinned) and P-CRR-MEMBER/P-CRR-LINKED; the null row
  stays Unknown, so the combined verdict does not move.
* **Leaves lost (27 system per configuration, 9 slice):** every
  `unresolved resource identity` at a destroyer whose actual is an admitted
  element: `flb_output_init_all` 13, `flb_sched_destroy` 6 (four elements and
  the two `sched` reads that their poisoning had caused), and one each in
  `flb_output_exit`, `flb_task_destroy`, `flb_task_retry_clean`,
  `flb_sched_timer_cleanup`, the two `*_release_space`; slice
  `flb_input_init_all` 3, `flb_input_exit_all`, `flb_tail_file_reclaim`,
  `flb_tail_file_purge`, `in_tail_collect_pending`, `in_tail_collect_static`.
  These rows stay Unknown on other leaves (member reads before the destroy,
  uncontracted calls).
* **Leaves gained (28 system per configuration, 14 slice), all one kind:**
  `unlink by DESTROYER of X, which this function has not published` where X
  is not an admitted element, a published or a held object (a poisoned
  formal, a never-bound local, an element of an unadmitted container). The
  call was already refused (its release read `unresolved resource identity
  X`, which mostly stays); the unlink's refusal is reported beside it. A
  fail-closed exchange: no row's status moves on it, and the pre-existing
  provisional Unsafe `flb_output_task_flush` only gains this leaf.

**Arbiter.** `quirin-audit measure --declared PREDICTION.md`, built from this
card's tip (`target/release`, measure plugin `3ca75e37…`), over the three
routes of [inputs.json](artifacts/inputs.json). The host refuses user
namespaces (`unshare: write failed /proc/self/uid_map`), so a `rev:` side
cannot be built; both sides are checkout directories instead: `git archive`
exports of `74105dd0e` (base) and `f4057708f` (tip, the measured code; later
commits change only records) under worktree-private `private/{base,tip}-src`,
each built in place by the tool, with the `third_party/libevent` submodule a
clone at its pinned `d82464a2`. One measurement-driver adaptation in both
exports, disclosed: `scripts/run_pinned_audit.py` hard-codes
`QCORE_CLANG=/usr/bin/clang`, which this host does not have; both exports
point it at the extracted Clang 21.1.8 instead (the compile databases already
name that driver). Earlier attempts, kept as evidence of what failed:
`measure/arbiter-1` (`rev:` sides, the namespace refusal), `arbiter-2` (the
tip side was the live worktree, whose records changed during the run, so its
routes exit 101; the corpus hit `/usr/bin/clang`), `arbiter-3` (system and
slice complete; the corpus refused the libevent pin because the export was
not a git checkout).

[native-comparison-system-slice.json](artifacts/native-comparison-system-slice.json)
(`measure/arbiter-3`, base binary `bf1ce1e3…`, tip `6cb348cc…`):

| Route | base | tip | unknown_alias | unsupported | declared |
|---|---|---|---|---|---|
| system | 103 / 4 / 341 | **107** / 4 / **337** | 950 → 896 | 2,350 → 2,406 | **match** (no unpredicted, no missing move) |
| slice | 58 / 1 / 341 / 4 Pending | **59** / 1 / **340** / 4 | 1,091 → 1,075 | 2,441 → 2,462 | **match** |

The arbiter's moved rows are exactly section 6's (four system rows, one slice
row), and its leaf deltas equal the quick runs' (system 27 resolved and 28
unlink refusals per configuration; slice 16 and 21 over both
configurations of `flb_input.c`).

CORPUS_PLACEHOLDER

## 7. Pins and locks

`providers/contracts.py sync` (inventory lock: ten declarations gain one
clause each; the load path's resource view keeps `teardown_destroy.qv` after
A′). [pin.py](scripts/pin.py) re-pinned the ten (logs
`private/pin-1/`): new identities at `66910c10`, `046accf6` and `90978ac6`
(the last stands for the slice export, whose synthetic commit `34155955`
cannot be recreated; its content is the same tree). `pin --check` at the
three trees exits 1 with the pre-existing not-in-force set
(`flb_config_set_program_name`, `flb_tail_config_destroy`, the instance and
tail destroyers at trees whose bodies differ); `pin --verify-contracts` 0.

## 8. Gate

`python3 scripts/gate_scope.py --base 74105dd0e` on `7bc4ec072` (code equal to
the measured `f4057708f`; [gate.json](gate.json), logs
`private/gate-scope/20261112T035537Z-2144909/`; Clang recording
[gate/clang-version.txt](gate/clang-version.txt), taken in this worktree).
Selection: the whole Rust suite (the diff reaches `qcore-schemas`,
`qcore-contract` and `qcore-c-gateway`), all Lean, the Python suite, the
provider gate.

| Step | Exit | Counts |
|---|---|---|
| submodules, fmt, clippy (`--all-targets -D warnings`) | 0 / 0 / 0 | |
| Rust suite (`run_test_binaries.py`) | **101** | 628 units, 670 summaries; 9,359 passed, **36 failed**, 17 ignored |
| both reevaluation probes | 0 / 0 | |
| Lean (`check_lean_audits.py`) | 0 | 427 audits (this card adds `AuditElementReleaseSummary.lean`) |
| Python suite | 0 | 756 tests |
| provider package build, verify-all, verify | 0 / 0 / 0 | |
| provider `cargo test` | 0 | 133 passed, 0 failed, 1 ignored |
| provider fmt, clippy | 0 / 0 | |

**The 36 failures are base's, every one.** 33 are in eleven test targets that
fail identically on an export of base `74105dd0e` run on this host
(`private/base-tests.sh`, logs `private/logs/base-tests/`; the failed names
[at base](gate/base-failed-tests.txt) and [at the tip](gate/tip-failed-tests.txt)
differ only by the three below): base `74105dd0e`'s Apache-2.0 header commit
broke tests that require a contract module to start with `module contract;`
(`prod_contracts`, `neighbours`, `hash_contracts`) and whole-source
inventories (`guarded_*`, `hypothesis_bindings`, `production_symbolic_adoption`,
`reading_site_*`, `libc_reading_site_evidence`, `contract_dependencies_report`,
`source_contract_config`). The other three are `task_route_tsan`'s
symbol-name assertions, which also fail at base on this host
([names](gate/base-failed-tsan.txt), `private/logs/tsan-base.log`): its
ThreadSanitizer reports carry no symbol names here. An earlier run
(`gate-scope/20261112T032752Z-1646244`, 129 failed) predates installing the
sanitizer runtimes and the card's `contract_loader` pin update; it is
discarded as validation evidence.

## 9. Follow-ups

[integration-amendments.json](integration-amendments.json): retires
`W45-FOREIGN-UNLINK-SUMMARY`; amends `FB-TEARDOWN-RELEASE-ACTUAL-IDENTITIES`
(advanced, still open for non-element actuals), `W45-FB-OR6-ELEMENT-IDENTITY`
(blocker now the recursive pair), `W45-FB-BACKLOG-REACHABLE-RELEASE`; appends
to `RETURN-POSITION-PREMISE-CONTRACT-ROUTE`; registers
`LERI-CROSS-UNIT-ELEMENTS-RELEASED`, `LERI-ELEMENTS-RELEASED-DECLARED-FORM`,
`LERI-HELD-AND-LINK-MEMBER-PREMISES`, `LERI-SLICE-EXPORT-PIN-IDENTITY`. No
evaluation-profile baseline is changed (`profiles: []`): the figures here are
on this card's preparations; the integrator's landing measurement on the
published inputs owns the baselines.

# VERDICT-CONTRACT-PREMISE-ANNOTATIONS: comment-only premises as clauses

Card VERDICT-CONTRACT-PREMISE-ANNOTATIONS (wave 48, card-high), the ledger
entry VERDICT-CONTRACT-DEPENDENCIES registered, under the user's direction of
2026-10-09: "handle as safe as possible". Base: local `main` at `9564eeb6d`.
The card was then rebased onto `84967949c` (VERDICT-BODY-STANDING-CLAUSE-SCOPE,
gateway rules 13i–13k) and measured again there (§4b).
[PREDICTION.md](PREDICTION.md) was committed before any contract text
changed: `e3679750d` before the rebase, `f27e511af` after it. Other commits
named below by their pre-rebase hash map as follows: `41eb05a27` →
`82b08525f`, `5511ac4fb` → `08d9c3dbe`, `f2d26a7d5` → `4bfec6ee7`.

**Published property and scope.** No new property and no verdict change:
custody (`c.custody`) on the pinned corpus, the system route and the
tail-pause slice. The card fills in the assumptions component of the verdict
contract (contract specification C04.18, C06.5): a premise that a declaration
rests on, which was stated only in a comment, is now a clause. So every row
that rests on that declaration names the premise. No gateway rule is added
(13l is not needed). The card changes no Rust and no Lean.

## 1. What changed

Eleven declarations in six modules gain exactly one clause each,
`requires resource_premise(NAME, "STATEMENT");`. It goes after the
declaration's last contract-level marker. Each module also gains a comment
line under the comment that states the premise. No other clause changes. The
texts are in [`scripts/annotate.py`](scripts/annotate.py), which applies
them, and `--check` verifies that each is present once. Each statement repeats
the comment's own claim with its citation, so that a reviewer can check it
against the source.

| premise | declarations (module) | what it states |
| --- | --- | --- |
| `synchronous_output_reachability` | `flb_output_task_singleplex_enqueue` (`fluent_bit/output_singleplex.qv`) | In-tree outputs never set `FLB_OUTPUT_SYNCHRONOUS`, so the function runs in production only for a proxy output that does. W45-FB-RETRY-LIFETIME says OR-7 (`flb_output_task_flush`) needs no such premise, so OR-7 does not carry it. |
| `ingestion_active` (C3) | `flb_input_collector_resume` (`fluent_bit/loop_owner.qv`) | The function returns 0 without resuming while `config->is_ingestion_active` is false (`src/flb_input.c:3221` at `66910c10`, `:3241` at `046accf6` and `34155955`). So a 0 means "running" only while ingestion is active. |
| `nested_call_admission` | `__errno_location`, `strerror` (`libc/errno.qv`) | The gateway reads a nested call or a dereferenced result only where the nested-call admission holds at the site. For `__errno_location` that is N1–N10 with N4-D (O04-TRACE, CONSTRUCTOR-RESIDUALS). `strerror` meets N3, N6 and N7, and the admission checks the rest per site. |
| `cjson_list_invariant` | `add_item_to_array` (`librdkafka/cjson.qv`) | cJSON's list invariant: a non-empty array's `child->prev` is never NULL. If a caller broke it, the body returns true without linking, and the declaration would read that as a transfer. |
| `cjson_allocator_hooks` | `cJSON_New_Item` (`librdkafka/cjson.qv`) | `hooks->allocate` returns a fresh allocation that `cJSON_Delete` releases through the matching hook (`malloc`/`free` in `global_hooks`, `cJSON.c:178`, `:186`). A user pair is a premise of whoever installed it. Nothing is stated about a hook change between the allocation and its release. |
| `asserted_allocation` | `rd_kafka_toppar_new0`, `rd_kafka_toppar_desired_add`, `rd_kafka_q_new0`, `ut_create_mock_queue` (`librdkafka/client.qv`) | Each non-null result is allocated with `rd_calloc`/`rd_malloc` (`rd.h:139-149`), which assert their result. The audited compile commands define no `NDEBUG`. (`desired_add`'s result comes from `rd_kafka_toppar_new0`'s reference.) |
| `untracked_handback` | `flb_input_chunk_get_name` (`fluent_bit/input_chunk_name.qv`) | The result aliases into `ic` and is not tracked. `resource_return_none` holds only where the result is never bound or stored, which O04-TRACE's N6/N8 admission checks per site. The alias itself is outside every claim (B03/R13, `O04-TRACE-HANDBACK`). |

**A correction to the ledger entry: `AOM-DESTROY-CORRUPTED-UNPAID-PREMISE`
is not attached.** The name comes from card AOM-DESTROY-SIBLINGS (§11 of its
report). It records that `flb_input_chunk_destroy_corrupted` is deliberately
left **undeclared**: writing its contract would have added a premise that
pays for nothing. It is not an assumption that `flb_input_chunk_destroy` or
`flb_input_chunk_destroy_direct_routes` rests on. Attaching it would put a
false statement on every row that rests on those two declarations: 5 corpus
and 14 system rows, and 3 and 8 for the sibling. Neither module changes. No
other comment in either module states a premise. Their "NOT CLAIMED" items
(retention, the bounds of `route_count`, failure of `cio_chunk_close`) are
scope boundaries of a property outside the custody clauses (§2, class S).

## 2. The sweep

[`scripts/sweep.py`](scripts/sweep.py) reads every tracked `.qv` under
`providers/` and `contract/` at the base and lists every comment line that
says "premise" or "assum…". It classifies each line, first by a manual table
of the lines read one by one, then by patterns. A line that no rule
classifies fails the run, and so does a manual entry that no line uses.
Output: [`sweep.json`](sweep.json), with 215 lines in 71 modules.

| class | lines | meaning | attached? |
| --- | ---: | --- | --- |
| A | 10 | a condition its declaration rests on that no clause states | yes: the eleven declarations of §1 |
| D | 118 | the declaration as a whole is a trusted assumption ("ONE TRUSTED ASSUMPTION", "a declaration is a premise") | no: each row already names the declaration and its body standing (C06.5) |
| R | 19 | a run-level named assumption: the libc/C-standard ABI premise (`libc-c-standard-abi`), the tip-tracked `mk_vhost_set` | no: each run already names it |
| M | 22 | method: a fact "measured / compared / checked, not assumed", or what callers "may assume" of a declared defined function | no: not a premise |
| C | 13 | already a clause: a literal-argument premise (C04.5), the variadic data tail, review-pending reasons (C06.1), the served-queue premise | no: already machine-readable |
| U | 11 | a note about a function deliberately not declared ("cost a premise and buy nothing"; `AOM-DESTROY-CORRUPTED-UNPAID-PREMISE`) | no: there is no declaration to attach to |
| B | 6 | a declaration unchecked because no analysed body checks it (cross-unit, header text, no normal exit) | no: its body standing, `declared-assumption`, already says so |
| S | 1 | a property outside the custody clauses is not claimed (LZ4 decoder bounds) | no: scope boundary |
| N | 15 | the module is not resource-v1 (chunkio lifecycle and source-frame policies, Monkey composition and event payload) | no: C04.18's clause is not in their grammar; registered (§7) |

The other phrasings ("rests on", "relies on", "holds under", "unchecked", "not
modelled", "invariant") were also read by hand. They found only the cJSON
list invariant (A, above) and lines of the classes above: `metric_values.qv`
(an unchecked bucket index: S), `lz4_stream.qv` (the unchecked decoder: S),
`stdio.qv` (the `%n` count and `va_start`/`va_end` pairing: S),
`pthread_mutex.qv` (the separate mutex model: S), `sds_lifetime.qv` (the
header offset, which the abstraction fixes: M), `memory.qv`
(`W46-HEADER-INLINE-DECLARED-UNCHECKED`: B), `input_instance.qv` and
`lib_engine_thread.qv` (review-pending reasons: C).

Three judgement calls, each made in the conservative direction:

* `client.qv`'s sentence "the count arithmetic and the non-null results stay
  declared assumptions". The non-null half rests on a build fact (no
  `NDEBUG`) that no clause states, so it is attached. The count arithmetic
  *is* the declarations' own reference clauses, so it stays the declared
  assumption that the body standing reports. VERDICT-BODY-STANDING-CLAUSE-SCOPE
  tracks that a rule 13d check validates the return shape only.
* `nested_call_admission` and `untracked_handback` name route conditions that
  the gateway **checks** per site. Calling them premises claims less than
  calling them checked, and their comments call them premises.
* `topic_partition.qv` ("the premise a reader would want stated") is
  rationale about the projection's own model: custody is tracked through the
  list pointer. It is not an assumption about the C function (M).

## 3. Control and re-pin

[`scripts/pin.sh`](scripts/pin.sh) uses the BEFORE binary; `pin` is
unchanged code. Logs: `private/vcpa/pin-control/` and `private/vcpa/pin-a1/`.

* **Control** (`check-only`, after the edit and before the re-pin): each of
  the nine pinned declarations reads `not in force (contract-changed)` at
  `66910c10` and `046accf6` (librdkafka: 6 not in force; fluent-bit: 3). At
  the slice export `34155955`, `flb_input_collector_resume` is also not in
  force: fluent-bit there has 14 recorded and 3 not in force. The two libc
  declarations read `stale lock`. Every check exits 1, and so does
  `--verify-contracts`.
* **Re-pin** at `66910c10` and `046accf6` for all eleven. Then, for
  `flb_input_collector_resume` alone, at `34155955`, the tail-pause slice's
  export: its lock already held an identity reviewed there
  (W47-LOOP-OWNER-CONTRACTS), and `pin` restarts a changed declaration from
  the pinned revision alone. Without this step the declaration would have
  left force on the slice (2 rows). This is a step the brief did not name. No
  other changed declaration had an identity at `34155955`. Every pin exits 0.
  The lock diffs are exactly the eleven `contract_sha256` lines; each
  identity's revisions are the base's. Afterwards:
  * each changed crate's `--check` exits 0 at `66910c10` and `046accf6`;
  * at `34155955`, fluent-bit has 15 recorded, 52 in force through another
    revision and 0 not in force (exit 1, as on the base: not every entry is
    recorded there);
  * the whole-tree `--check` exits 1 at each revision, as on the base (the
    libevent crate's tip-mode `mk_vhost_set`);
  * `--verify-contracts` exits 0.
* `python3 providers/contracts.py sync` (exit 0) changes only
  `qcore-provider-librdkafka/contracts.json` (two module digests). No
  published module changed, so `contract-load-path.json` does not move.

## 4. Identity

One binary measures both sides: `private/vcpa/bin/base`, `289ec7fb…`, which
equals each `run.json`'s `binary_sha256`. The card changes no code, and the
binary embeds none of the edited modules. Contracts are read from the
worktree at run time. Every route exits 0.

* BEFORE wall time: corpus 464.3 s, system 63.8 s, slice 29.4 s.
* AFTER wall time: 453.1 s, 63.1 s, 28.2 s.

[`scripts/identity.sh`](scripts/identity.sh) runs
[`identity.py`](scripts/identity.py) and [`parity.py`](scripts/parity.py)
on each route.

| route | totals (before = after) | report | verdict rows | rows' `contract_dependencies` | sidecar paths differing (outside the contract input) | parity multiset |
| --- | --- | --- | --- | --- | --- | --- |
| corpus | 881 / 9 / 3,944 | `96852f4e…` **byte-identical** | `97cbdcf3…` equal | equal | 110 (0) | identical (662,667 rows) |
| system | 87 / 4 / 357 | `0985ced7…` → `85b2d675…` (45 paths, all `compile_configuration` contract-source digests) | `09b7a59e…` equal | equal | 111 (0) | identical (139,962) |
| slice | 45 / 1 / 354, 4 Pending | `f0e8f358…` → `f4af3113…` (48 paths, the same) | `ce1be80e…` equal | equal | 107 (0) | identical (50,407) |

Every differing sidecar path is a contract input:

* the six modules' texts and digests, and every pinned entry's
  `module_sha256` in those modules;
* the three locks' digests;
* `contract_sha256` and `premises` for the eleven declarations only, in
  `contract_revision`, in the dependency section's declaration records and
  its top-20 list, and in the current route's `contract_premises`.

The parity streams never match in order between two `--jobs 4` runs (the
previous card's finding). As multisets they are identical, with 0 rows only
before and 0 only after. The verdict-row digests equal the ones the previous
card published for the same routes.

**Safe rows naming each premise** (from the AFTER sidecars; a row that rests
on two of the declarations counts under each):

| premise | corpus Safe / rows | system | slice | standing (corpus) |
| --- | --- | --- | --- | --- |
| `synchronous_output_reachability` | not consulted | 0 / 0 | not consulted | declared assumption (system) |
| `ingestion_active` | not consulted | 0 / 0 | 0 / 2 | declared assumption |
| `nested_call_admission` | 1 / 196 (`__errno_location` 1 / 156, `strerror` 0 / 40) | 6 / 68 | 3 / 62 | declared assumption (flexible, libc) |
| `cjson_list_invariant` | 1 / 5 | – | – | declared assumption |
| `cjson_allocator_hooks` | 7 / 29 | – | – | declared assumption |
| `asserted_allocation` | 0 / 51 (`rd_kafka_q_new0` 34, `ut_create_mock_queue` 8, `rd_kafka_toppar_new0` 5, `desired_add` 4) | – | – | validated (rule 13d) |
| `untracked_handback` | 1 / 9 | 2 / 24 | – | validated |

`rd_kafka_q_new0` keeps rule 13e's two result-holder premises beside the new
one. The Safe roll-ups are unchanged:

* corpus: 418 rows rest on no declaration, 85 only on validated ones, 378 on
  a declared assumption;
* system: 47 / 3 / 37;
* slice: 29 / 0 / 16.

**`mk_vhost_set` tip probe** (round 12): [`scripts/tip-probe-base.sh`](scripts/tip-probe-base.sh)
measures BEFORE with the base contracts. It restores the nine files and
checks their digests. [`scripts/tip_rows.py`](scripts/tip_rows.py) reads the
rows:

* verdict rows and dependencies are identical on both copies (tip
  `f5a334e4…`, master `a5e8e265…`);
* `mk_vhost_set` is Unknown, with 1 leaf at the tip and 6 on master (held);
* `mk_vhost_set_property` is Unknown with 2 leaves on both. None of its five
  declarations changed.

## 4b. On the rebased base (`84967949c`)

The base moved under the card. VERDICT-BODY-STANDING-CLAUSE-SCOPE now gives
body standing per clause (rules 13i–13k), and no body evidence covers a named
premise (rule 13i). So a declaration that was fully validated becomes
`partly-validated` when it gains a premise. That consequence follows from the
landed rules; it was not foreseen in the frozen prediction, which predates
them.

I measured again with one binary built at the rebased tip,
`private/vcpa/bin/rebased` `971441aa…`, which equals every `run.json`.

* **BEFORE2** runs with `84967949c`'s six modules and three locks in place
  ([`scripts/with-base-contracts.sh`](scripts/with-base-contracts.sh)
  rewrites them, then restores them with their digests asserted).
* **AFTER2** runs with the card's files.
* Wall time, BEFORE2 → AFTER2: corpus 461.2 → 513.1 s, system 69.5 → 73.1 s,
  slice 33.4 → 29.5 s. The run was under shared load, so these are not a cost
  measurement.

The identity check now also lists rules 13i–13k differences. They are
allowed only for a changed declaration's own coverage and record, and for the
counts and Safe roll-up derived from them.

| route | verdict rows (= main's) | rows' `contract_dependencies` | report paths differing (13i coverage / other) | sidecar paths (13i–13k / other) | parity multiset |
| --- | --- | --- | --- | --- | --- |
| corpus | `97cbdcf3…` equal | equal | 5 (5 / 0) | 127 (17 / 0) | identical (662,667) |
| system | `09b7a59e…` equal | equal | 47 (2 / 0; 45 contract-source digests) | 119 (8 / 0) | identical (139,962) |
| slice | `ce1be80e…` equal | equal | 48 (0 / 0; 48 contract-source digests) | 107 (0 / 0) | identical (50,407) |

The corpus report differs from BEFORE2 (`c1e6e7d1…` → `2a484e21…`) in
exactly five paths, each a premise clause listed in a unit's
`contract_body_evidence.clauses` as uncovered:

* `resource_premise(asserted_allocation)` of `rd_kafka_toppar_new0`,
  `rd_kafka_toppar_desired_add`, `rd_kafka_q_new0` and `ut_create_mock_queue`;
* `resource_premise(untracked_handback)` of `flb_input_chunk_get_name`.

| standing move | corpus | system | slice |
| --- | --- | --- | --- |
| validated → partly-validated | `flb_input_chunk_get_name` (`untracked_handback` uncovered), `ut_create_mock_queue` (`asserted_allocation` uncovered) | `flb_input_chunk_get_name` | none |
| already partly-validated, one more uncovered clause | `rd_kafka_toppar_new0`, `rd_kafka_toppar_desired_add`, `rd_kafka_q_new0` | – | – |
| declarations validated / partly validated | 48 / 29 → **46 / 31** | 8 / 5 → **7 / 6** | 4 / 1, unchanged |
| Safe rows: none / only validated / partly validated / declared assumption | 418 / 64 / 22 / 377 → **418 / 63 / 23 / 377** | 47 / 3 / 2 / 35 → **47 / 1 / 4 / 35** | 29 / 0 / 2 / 14, unchanged |

The Safe rows that moved rest on `flb_input_chunk_get_name`: 1 in the corpus
and 2 in the system route. Each now counts as resting on a partly validated
declaration, because its premise is a stated assumption that no body
establishes. That is the conservative label. The Safe rows naming each
premise are unchanged from the table above.

The `mk_vhost_set` tip probe on the rebased binary reads the same:
`with-base-contracts.sh` for BEFORE, then `tip_rows.py` on `private/vcpa/tip-rebased` and
`private/vcpa/tip-a2`. Verdict rows and dependencies are identical on both
copies (`f5a334e4…`, `a5e8e265…`), and the two rows are as above.

## 5. Prediction against outcome

| prediction | outcome |
| --- | --- |
| control: the nine read `contract-changed` | held (the libc pair `stale lock`) |
| re-pin at both revisions plus `34155955` for resume; crate checks 0, verify 0 | held |
| no verdict moves; dependency rows equal; parity multisets identical | held on all three routes |
| corpus report byte-identical | held (`96852f4e…`) |
| system/slice reports differ only in contract-source digests | held |
| sidecars differ only in contract inputs and the eleven declarations' digests and premises | held (plus `module_sha256` of every pinned entry in the six modules, a contract input) |
| Safe rows per premise = BEFORE's per-declaration Safe rows | held exactly |
| Lean: none | held |
| eleven clause pins +1, no count moves; librdkafka inventory only | held |
| (rebased onto `84967949c`) no verdict or dependency row moves | held on all three routes; not predicted: rule 13i labels two validated declarations `partly-validated` (§4b), and the corpus report is no longer byte-identical (five premise clauses of rule 13i's coverage) |

## 6. Lean

None. The clause changes no mode and no verdict reads it (C04.18). The
existing `QCore.ContractDependencies` (`publishing_the_set_changes_no_verdict`)
already covers the claim that publishing a dependency record moves nothing.
No `.lean` file changed.

## 7. Gate

**On the rebased tip (the gate of record).** After the rebase, `cargo check
--workspace --all-targets` and the conflict-marker grep are clean. The
scratchpad `audit_card_lines.sh 9564eeb6d 74e2844f4` listed only this card's
own STATUS and increment lines: I resolved those conflicts to main's text and
wrote the lines again with the rebased figures. Gate `g2` ran with every
step under the shared lock, code steps at `5c601a438`; logs are in
`private/vcpa/gate-g2/`.

* `cargo check --workspace --all-targets`: 0.
* Submodule: 0. fmt: 0. clippy `--locked`: 0.
* `run_test_binaries.py`: 461 units, 474 summaries, **7,728 passed, 0
  failed**, 11 ignored, exit 0 (`private/vcpa/tests-gate-g2/`).
* Both reevaluation probes: 0.
* `check_lean_audits.py`: **300** audits passed.
* The docs steps and the matrix regeneration are recorded in
  [gate/](gate/) (08, 09, 09a, 09b, 10b, 10c).

`providers/` is byte-identical between the provider gate `pg2` (below) and
the rebased tip, and main changed nothing under `providers/`, so `pg2`
stands.

**On the first base (`9564eeb6d`).** Gate `g1` ran with every step under the shared lock. The code steps ran at
`5511ac4fb`, which has the final contracts and pins; the later commits change
records only. Logs are in `private/vcpa/gate-g1/`.

* `cargo check --workspace --all-targets`: 0.
* Submodule: 0. fmt: 0. clippy `--locked`: 0.
* `run_test_binaries.py`: 460 units, 473 summaries, **7,718 passed, 0
  failed**, 11 ignored, exit 0 (`private/vcpa/tests-gate-g1/`).
* Both reevaluation probes: 0.
* `check_lean_audits.py`: **299** audits passed.
* Python suite: **365 OK** (08). Docs index `--check`: 0 (09).
* Then `build_matrix.py --gate gate.json` (09a) 0, the Python suite again
  (09b) 365 OK, and the docs index build and `--check` (10b, 10c) 0.

Before the gate, the targeted `qcore_contract_corpus` and `contract_loader`
tests passed (`private/vcpa/targeted-1.log`).

The first provider gate, `pg1`, is **discarded as validation evidence**.
Steps p1–p4 exited 0. Then the host disk filled ("No space left on device";
another card's build and this worktree's 50 GB dev target were both on the
host), and p5–p8 recorded no exit. This worktree's dev target was cleaned
(`cargo clean --profile dev`), the disk recovered to 75 GB free, and the
gate was rerun as `pg2` at `f2d26a7d5`, where `providers/` is the same as at
the code steps:

* `cargo check --locked`: 0. `cargo test`: 129 passed. fmt: 0. clippy: 0.
* `contracts.py check`: 0.
* Package build: 0. Verify of all five packages: 0.
* Providers Python suite: 25 OK.

Records: [gate.json](gate.json) and [gate/](gate/).

## 8. Ledger

* `VERDICT-CONTRACT-PREMISE-ANNOTATIONS` is **retired**. Each premise its
  entry names is attached to the declarations it covers: the reachability
  premise, C3, and N1–N10. The fourth name, AOM-DESTROY-CORRUPTED-UNPAID-PREMISE,
  is a correction: it is not a premise of any declaration (§1). The
  declarations were re-pinned and measured with no verdict movement. The
  sweep added four more premises over seven declarations.
* `VERDICT-PREMISES-NON-RESOURCE-CONTRACTS` is **registered**: the 15 comment
  lines in 14 non-resource-v1 modules (two crates) that state caller premises
  of contract families with no premise clause. Ledger 734 → 734 on the
  rebased base (733 → 733 at `9564eeb6d`).

## Files

* Contracts:
  * `providers/quirin-provider-fluent-bit/contracts/fluent_bit/{output_singleplex,loop_owner,input_chunk_name}.qv`;
  * `providers/qcore-provider-librdkafka/contracts/librdkafka/{cjson,client}.qv`;
  * `providers/quirin-provider-libc/contracts/libc/errno.qv`;
  * the three crates' `contracts.lock`;
  * `providers/qcore-provider-librdkafka/contracts.json`.
* Pins: `compiler/tests/qcore_contract_corpus.rs`, eleven clause rows +1:
  * `flb_output_task_singleplex_enqueue` 9 → 10
  * `flb_input_collector_resume` 6 → 7
  * `flb_input_chunk_get_name` 3 → 4
  * `__errno_location` 3 → 4
  * `strerror` 4 → 5
  * `cJSON_New_Item` 3 → 4
  * `add_item_to_array` 5 → 6
  * `rd_kafka_toppar_new0` 7 → 8
  * `rd_kafka_toppar_desired_add` 5 → 6
  * `rd_kafka_q_new0` 9 → 10
  * `ut_create_mock_queue` 3 → 4
* Scripts: [`scripts/`](scripts/). Artifact: [`sweep.json`](sweep.json).

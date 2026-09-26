# W45-CONTAINER-ROOT-REACH: where a container outlives the function, and the foreign unlink

Card W45-CONTAINER-ROOT-REACH, 2026-09-26. Base `12553fb3` ("Record the
w45-gateway-memo-key integration gate"). The measured code is `34d8a449`;
later commits change only comments, tests, Lean and records. The
[prediction](PREDICTION.md) and the census were frozen at `0c3a031c`, before
any implementation. Gateway constraint 10f; contract specification unchanged
(no clause is added); no contract added or changed.

## Headline

* **The census corrects the brief's premise.** No publication in the corpus
  or the Fluent Bit slice is refused by rule 10d for its container: the
  container-root refusal has **zero** leaves. The four
  `MK-LIST-ADD-BORROWED-PUBLICATION` functions publish into formal-chain
  containers (`mk_vhost_create`'s alias into a body-acquired owner) and are
  blocked first by their entry: an uncontracted `mk_string_dup` store into the
  object refuses and poisons its binding before the link. Widening container
  roots reaches none of them.
* **The dominant 10d refusal was the foreign unlink**, and it is now modelled
  (rule 6): an element reached through a container rooted at an unrebound
  formal becomes the body's custody at its unlink, once; a leak of it is a
  leak and a double release a violation. **Static containers** are admitted
  for publication (rule 5, 0 census sites).
* **A body with a foreign unlink publishes no summary.** A caller would
  otherwise read `Borrowed` for the container and keep restoring its own
  published objects the callee released
  (`the_borrowed_summary_would_hide_a_double_release`).
* **Corpus:** 12 leaves removed, 2 added, 9 functions; **no status move**,
  leak 34 unchanged, no Safe → non-Safe. **Slice:** 18 leaves removed, 3
  added, 10 function rows; no status move, no leak.
* **Obligations:** OB-3, OB-7 and OB-10 lose the foreign-unlink blocker in
  `sb_remove_chunk_from_segregated_backlog` and `sb_destroy_backlogs` and stay
  **Unknown**; the retry OR table is **identical** (OR-1, OR-7 refuted; OR-6
  not stated; OR-2 not reached).
* **Lean:** `QCore.ContainerRootReach`, 19 audited theorems.

## 1. Bases

| Measurement | Result |
|---|---|
| Arbiter BEFORE (`12553fb3`, `run_audit.py --jobs 8`) | Reproduces [w45-fb-backlog-custody/integration/artifacts](../w45-fb-backlog-custody/integration/artifacts/): `resource-diagnostics.json`, chunk contracts and Monkey source byte-identical; the contracts sidecar differs only in two `meaning` strings later cards reworded; `verdict_bearing_identical: true` ([before-identity.json](artifacts/arbiter/before-identity.json)) |
| Slice BEFORE at `046accf6` (fresh private clone attempt-1, `scripts/prepare-attempt.sh`, exit 0) | units 0 / 2 / 13; functions 28 / 2 / 204 / 3; unsupported 3,338, unknown_alias 717, counted_user 4: the published figures ([before.json.gz](artifacts/slice/before.json.gz)) |
| After merging `main` `675e7289` (W45-AUDIT-CHECKPOINTS) at `45bea57e` | the arbiter's five artifacts are byte-identical to this card's AFTER ([merged-identity.json](artifacts/arbiter/merged-identity.json)) and the slice report is byte-identical to its AFTER |

## 2. Census (criterion 1)

Every `mk_list_add` / `mk_list_del` site of the corpus units and the slice,
classified by root shape and by the leaf the audit reports at the call's line
(`scripts/census.py`, `scripts/census_rank.py`;
[before](census/ranked-before.json), [after](census/ranked-after.json)). The
lexical classification was read site by site; `census_rank.py`'s `OVERRIDES`
records each correction.

| Shape | Corpus sites / functions | Slice sites / functions | First blocker (BEFORE) | Decision |
|---|---|---|---|---|
| C1 publication into an unrebound formal's chain or a local initialised from one | 32 / 22 | 17 / 17 | entry identity, whole-call refusal, 10d entry refusal; never the container | admitted already (10d) |
| U1 unlink of an element of such a container (safe-iteration current element, first/last lookup) | 16 / 12 | 10 / 8 | **10d unlink 12 / 9** | **admitted (rule 6)** |
| U6 unlink of a formal's own object, container unnamed | 12 / 11 | 10 / 9 | 10d unlink 9 / 8 | refused: `W45-FORMAL-ENTRY-UNLINK` |
| C4 publication into a container a callee returned | 10 / 6 | 1 / 1 | whole-call refusal, identity | refused |
| C3 publication into a body-acquired owner published or returned | 8 / 5 | 3 / 1 | identity, whole-call refusal | refused: `W45-CONTAINER-ROOT-REACH` (re-scoped) |
| U3 element of a callee's (thread-local) container | 2 / 2 | — | 10d unlink | refused |
| U4 element of a local list | — | 1 / 1 | 10d unlink | refused |
| C2 / U2 static container | 0 | 0 | — | **admitted (rule 5)** |
| C5, C6, U0, U5, U7, plain-iteration unlink | 1 each | 1–2 | — | refused (U0 is 10d's own restore) |

**Ranking.** By functions blocked through a 10d leaf: U1 (9 corpus, 7 slice),
U6 (8, 7), U3 (2), U4 (1), U7 (1). No container shape blocks anything first.

## 3. The mechanism (criteria 2–3)

`crates/qcore-c-gateway/src/legacy_container_publication.rs`, with thin hooks:
`Frame::head` and one proposal in `legacy_list_iteration.rs`; the parser's
`element_origins`, the statics passed to `CallerFormals`, the post-parse
lowering, and the foreign site in `instruction_produced_sites` /
`offset_produced_sites` in `legacy_c.rs`; one line in the W4 gate
(`legacy_summary::observations_match_contract`).

**Rule 5, static containers (P-CRR-STATIC).** `&G` or `&G.F…` with `G` in
`PublicationFacts::destinations` (an unshadowed file-scope object or a
`static` local) and not a formal or derived local is `Container::Static`, and
a publication into it discharges exactly as into a caller's container. Why it
outlives the function: static storage lives for the program. Why constraint
10's refusal of a store into a global does not apply: a list link is
additive; no later store to the global can lose the object the way an
overwritten slot does. `G->F` (a pointer-valued global) stays refused. Two
limits measured by the controls: `&G.F` is refused by the call's member-address
gate before any effect, and rule 5l reads no static head, so a static list's
element is never bound; both stay refused, fail-closed.

**Rule 6, the foreign unlink (P-CRR-MEMBER, P-CRR-LINKED).**

* Rule 5l's element binding proposes the container its pointer reaches: the
  active iteration's head for a cursor (never for a saved successor), `H` for
  `(H)->next` / `->prev`, nothing for a never-bound formal. The parser keeps
  the proposal only when the binding is the statement's final instruction,
  keyed by the element token's line.
* After the whole body is parsed, `settle_foreign_unlinks` gives an unlink of
  `E` a fresh site when every write of `E` in the body is a `E = ({ … })`
  binding kept that way, every container is `CallerFormal` or `Static`, and
  `E` is neither a formal nor a static name. The site is listed among the
  instruction's produced sites, so formal sites and the recursion combiner
  stay disjoint.
* At the transfer, if `E` holds no binding (the element binding unbinds it),
  the site becomes `LIVE`, non-null and unpublished, and `E` is bound to it,
  exactly as a non-null acquisition in a loop. A bound `E` keeps 10d's rule:
  restore only a published site, refuse otherwise.

Why the element is the body's to hold: the container owns what it links
(P-CRR-MEMBER), the container is one the caller handed the body to modify
(a formal's) or the program's (a static), and the element is still linked when
unlinked (P-CRR-LINKED, the twin of P-ILI-CALLEE-FRAME). Both premises are
stated, not proved.

**The summary gate.** `has_foreign_unlink` fails the W4 gate for every body
whose CFG lowers a foreign unlink. The function's own row is decided as
before; its callers see an uncontracted call. Its cost: `flb_task_queue_destroy`
keeps the unresolved identity of the `queue` it frees
(`W45-FOREIGN-UNLINK-SUMMARY`).

**Invariants kept.** A release while linked is accused
(`releasing_a_foreign_element_after_republishing_it_is_accused`); a double
or cross-list publication stays Unknown (a published site is not live); an
unlink restores custody only of what the body may own (its own publication,
or an element of a formal's or static container); a return of a published
object stays refused.

## 4. Controls (criterion 5)

`crates/qcore-c-gateway/tests/container_root_reach.rs`, 14 tests; 2 unit
tests in the module.

| Control | Result |
|---|---|
| publication into a global list (`static` and external `struct mk_list`, `static` local) | Safe |
| publication into a caller member chain (`&q->items`; a local initialised from `h->queue`) | Safe |
| foreign unlink then free (safe iteration over `&q->items`, over a list formal, first-element lookup, two iterations over two lists) | Safe |
| foreign unlink then re-publication (flush_one's move) | Safe |
| publication into a local list, into a body-acquired owner | Unknown, 10d refusal, no leak |
| publication through a pointer-valued global; into a static record's member | Unknown (call refused first), no leak |
| foreign unlink then leak (in a loop; on a `destroy == 0` arm) | provisional Unsafe, 1 leak |
| foreign unlink then double free | provisional Unsafe, double release |
| foreign element released after re-publication | provisional Unsafe, release after escape |
| unlink from a callee's container, a local list, a rebound formal's list, an element name initialised to `NULL`, a static list rule 5l cannot read | Unknown, 10d refusal, no leak |
| a plain `foreach` unlinking its current element | Unknown, rule 5l refusal |
| a formal's own object | refused |
| an element unlinked twice | refused |
| a draining helper | helper Safe; its caller Unknown (no summary) |

The existing `retry_lifetime.rs` (20), `refused_call_hand_on.rs` (6) and
`intrusive_list_iteration.rs` (19) pass unchanged.

## 5. Corpus arbiter (185 units)

AFTER on `34d8a449` ([artifacts/arbiter](artifacts/arbiter/), the new
baseline; contracts sidecar byte-identical to BEFORE's). Every leaf is listed
in [attribution/corpus.json](artifacts/attribution/corpus.json).

| | BEFORE | AFTER |
|---|---|---|
| units | 5 / 8 / 168 / 4 | 5 / 8 / 168 / 4 |
| functions | 398 / 9 / 4,427 | 398 / 9 / 4,427 |
| leak / unknown_alias / unsupported | 34 / 3,570 / 41,325 | 34 / **3,571** / **41,314** |

* **Removed (12):** 11 `unlink by mk_list_del of X, which this function has
  not published` in 9 functions (`mk_rconf_free` ×3, `mk_rconf_free_entries`,
  `mk_string_split_free`, `mk_config_listeners_free`,
  `mk_fifo_worker_destroy_all`, `mk_http_request_free_list`,
  `mk_server_listen_exit`, `mk_vhost_destroy`'s `ep`,
  `mk_vhost_handler_free`), and `mk_server_listen_exit`'s unresolved `listen`
  at its release, now discharged.
* **Added (2):** `mk_vhost_destroy`'s unresolved `ep` at the null tests
  `if (ep->file)` and `if (ep->real_path)`: the element is now a tracked
  identity whose pointer members the body never linked, so the read refuses
  (an exchange, fail-closed).
* **Not moved:** `mk_vhost_destroy`'s `halias` (declared `= NULL`, so not an
  element-only name) and the U3/U6/U7 unlinks keep their refusals. In 8
  functions the element's own release keeps its unresolved identity: a
  member release or null test before it refuses and poisons the element
  (`W45-FOREIGN-UNLINK-RESIDUAL-SHAPES` part 6).
* **No added leak or violation.** The named risk, `mk_http_request_free_list`
  (its embedded `sr_fixed` element is not freed), is withdrawn by the refused
  `mk_http_request_free` call that receives the element.

## 6. Fluent Bit slice at `046accf6`

`scripts/run-current.sh attempt-1 RUN BINARY --selection
contract/profiles/fluent-bit/system-selection.json --jobs 8` (the command of
[w45-fb-backlog-custody §4](../w45-fb-backlog-custody/README.md)); AFTER is
[after.json.gz](artifacts/slice/after.json.gz), every leaf in
[attribution/slice.json](artifacts/attribution/slice.json).

Units 0 / 2 / 13 and functions 28 / 2 / 204 / 3, unchanged; unsupported
3,338 → 3,327, unknown_alias 717 → 713, counted_user 4.

* **Removed (18, both variants):** the unlink refusals of
  `sb_destroy_backlogs`, `sb_remove_chunk_from_segregated_backlog`,
  `flb_output_plugin_property_check`, `flb_output_task_queue_flush_one`,
  `flb_output_task_singleplex_flush_next` and `flb_task_queue_destroy` (×2),
  and the unresolved identities of the released elements in
  `flb_output_task_singleplex_flush_next` and `flb_task_queue_destroy`.
* **Added (3):** `sb_remove_chunk_from_segregated_backlog`'s `chunk` is
  nullable at `sb_destroy_chunk` (a refused condition before it poisons the
  element); `flush_one` reads `queued_task->retry` after re-publishing it
  (flb_output.c:339), a read through a published object.
* **Not moved:** `schedule_request_promote` binds `request` over
  `&sched->requests_wait` and over its local `failed_requests`, so none of its
  three unlinks is admitted.
* `flb_task_queue_destroy` is left with one leaf, the `queue` it frees, which
  the summary gate keeps unresolved.
* The predicted `destroy == 0` leak in
  `sb_remove_chunk_from_segregated_backlog` does not appear: the element is
  poisoned before that exit.

## 7. Obligations (criterion 4)

**Backlog OB table** ([ob-table.json](ob-table.json);
`python3 docs/reports/w45-fb-backlog-custody/scripts/ob_table.py AFTER BEFORE
ob-table.json` over the two slice reports; its BEFORE column reads the
specification lines of a pre-backlog report and so says `no specification`,
the AFTER column is the table). All ten obligations stay **Unknown**.

| OB | Leg | Blockers before → after |
|---|---|---|
| OB-3 | `sb_remove_chunk_from_segregated_backlog` | foreign-unlink refusal **gone**; a branch condition, the byte counter's indirect access (OB-8), `chunk` poisoned (unresolved, then nullable at its release) remain |
| OB-7 | `sb_release_output_queue_space`, `sb_remove_chunk_from_segregated_backlogs` | unchanged: the uncontracted helpers (their specifications are not established), reachable release |
| OB-10 | `sb_destroy_backlogs` | foreign-unlink refusal **gone**; the uncontracted `sb_destroy_backlog` and `backlog`'s unresolved release remain |

**Retry OR table** ([or-table.json](or-table.json); `scripts/or_table.py`
over the same two reports): every `user_count_clauses` record is identical.
OR-1 refuted at flb_output.c:376 and OR-7 at :541 (provisional Unsafe
candidates, never upstream defects), OR-6 not stated (advanced: `queued_task`
is an identity after its unlink, but flush_next publishes no summary), OR-2
not reached.

No contract is added or changed, so there is nothing to review or pin;
`pin --check` at both revisions and `pin --verify-contracts` exit 0 (§10).

## 8. Lean (criterion 6)

`proof/QCore/ContainerRootReach.lean`, audited in
`proof/AuditContainerRootReach.lean` (19 theorems, `propext` only):

* each admitted root outlives the call under its premise
  (`an_admitted_root_outlives_the_call`, `a_static_root_lives_for_the_program`);
* refusal is preserved for every unlisted shape
  (`only_the_listed_roots_are_admitted`,
  `a_local_container_does_not_outlive_the_call`,
  `a_root_without_a_premise_is_refused`,
  `an_unlink_from_an_unadmitted_root_is_refused`,
  `a_republication_into_a_local_container_is_refused`);
* a foreign unlink transfers custody exactly once
  (`a_foreign_unlink_hands_the_element_to_the_body`,
  `a_second_unlink_is_refused`, `custody_moves_once`), and the body must
  release or re-publish (`…then_release_is_safe`,
  `…then_republication_is_safe`, `…then_leak_is_a_leak`,
  `…then_double_release_is_a_violation`, `a_linked_element_owes_nothing`);
* the summary gate (`the_borrowed_summary_would_hide_a_double_release`,
  `the_draining_body_is_withheld`).

The lexical readers, the premises and the Rust correspondence are trust
boundary 28's, pinned by the tests of §4.

## 9. Prediction against outcome

| Prediction | Outcome |
|---|---|
| corpus: 10–12 of 12 U1 unlink leaves removed | **11** (`halias` is `NULL`-initialised) |
| corpus: 8–12 unresolved releases after the unlink removed | **1**. Miss: a member release or null test before the release refuses and poisons the element |
| corpus: exchanges 0–15 | **2** |
| corpus: publication leaves unchanged | **confirmed** |
| Unknown → Safe 0–1; Safe → non-Safe 0; units unchanged | **0; 0; unchanged** |
| leak 34 → 34–35 (`mk_http_request_free_list` risk) | **34**; the risk is withdrawn by a refused call |
| slice: 7–9 U1 unlink leaves removed per variant; U4 kept; statuses unchanged | **7** per variant (`schedule_request_promote`'s two formal-list unlinks share a name with its local list and stay refused); U4 kept; statuses unchanged |
| slice: leak +0–1 on `destroy == 0` | **0** |
| OB-3, OB-7, OB-10 lose the foreign-unlink blocker, stay Unknown | OB-3 and OB-10 lose it (OB-7's legs never carried it themselves: it is in their uncontracted callees); all Unknown |
| OR table unchanged | **identical** |

## 10. Gate

`scripts/gate.sh LOG_DIR`: the standing gate of `docs/agents/validation.md`,
the provider gate (constraint 18), and `pin --check` at both revisions. Each
exit is captured to its own file, `CARGO_INCREMENTAL=0`, `cargo clean
--profile dev` after the test step. Commands, exits and counts are in
[gate.json](gate.json); logs are under the worktree-private
`private/crr/gate-final/`.

Final gate on `0c44b274` (merged with `main` `675e7289`): every step exits 0
except the provider test step.

| Step | Result |
|---|---|
| submodule, fmt, clippy, both reevaluation probes, docs index, `contracts.py check`, package build and verify, provider fmt and clippy, release build | 0 each |
| Rust suite (`scripts/run_test_binaries.py`) | 0: 310 units, 320 summaries; **5,937 passed**, 0 failed, 11 ignored |
| Lean audits | 0: **205** passed |
| Root Python tests; `providers/tests` | 0: **358**; **15** |
| `pin --check` at `046accf6` and `66910c10`, `pin --verify-contracts` | 0 each |
| Provider `cargo test` | **101**: one `configured_audit` test (`checked_function_contracts` 1 against 2); a rerun failed a different test of the same binary; the binary passes single-threaded (7/7), and a `--no-fail-fast` rerun of the step exits 0 with **129 passed**, 0 failed, 1 ignored |

The provider failure is load-sensitive and not this card's: the fixture's
verifier-plugin process runs under a 5 s timeout, the host's load average was
40–47 on 16 CPUs from concurrent cards, the failing test varies between runs,
and the card touches no provider or plugin code. The pre-merge gate on
`e0bb2063` (discarded) passed the provider tests; it failed the root Python
suite on the matrix's derived counts, repaired by regenerating them, and one
`package_identity` test on an `ETXTBSY` race that passes in the final gate.

## 11. Follow-ups ([followups.json](followups.json))

* **Retired:** `W45-FB-BACKLOG-FOREIGN-UNLINK` — its two named retirement legs
  and 11 of its corpus leaves are discharged; the shapes still refused are
  registered below with their sizes.
* **Registered:** `W45-FORMAL-ENTRY-UNLINK` (corpus 12 sites / 11 functions,
  slice 10 / 9); `W45-FOREIGN-UNLINK-SUMMARY` (the summary gate's cost);
  `W45-FOREIGN-UNLINK-RESIDUAL-SHAPES` (callee containers 2, initialised
  names 1, mixed local containers 3 slice, embedded 1, static heads 0, and
  the element's member uses in 8 corpus and 2 slice functions).
* **Re-scoped:** `W45-CONTAINER-ROOT-REACH` (the census above: no container
  root blocks its four functions; the owner relation stays open);
  `W45-FB-OR6-ELEMENT-IDENTITY` amended. `W45-FB-BACKLOG-PUBLICATION-BLOCKED`
  is unchanged: OB-1's blocker is the counter access, not a container.

## Reproduce

```sh
python3 docs/reports/w43-return-expression/scripts/run_audit.py FRESH_DIR COMMIT --jobs 8
docs/reports/w45-container-root-reach/scripts/prepare-attempt.sh attempt-N BINARY
docs/reports/w45-container-root-reach/scripts/run-current.sh attempt-N RUN BINARY --selection contract/profiles/fluent-bit/system-selection.json --jobs 8
python3 docs/reports/w45-container-root-reach/scripts/census.py corpus DIAGNOSTICS OUT.json
python3 docs/reports/w45-container-root-reach/scripts/census.py slice REPORT SOURCE_ROOT OUT.json
python3 docs/reports/w45-container-root-reach/scripts/census_rank.py CORPUS.json SLICE.json OUT.json
python3 docs/reports/w45-container-root-reach/scripts/diff_reports.py BEFORE AFTER OUT.json
python3 docs/reports/w45-container-root-reach/scripts/or_table.py BEFORE AFTER OUT.json
python3 docs/reports/w45-fb-backlog-custody/scripts/ob_table.py AFTER BEFORE OUT.json
```

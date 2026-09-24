# W43-INDEXED-RESOURCE-COLLECTIONS: arrays whose elements are owned resources

Card W43-INDEXED-RESOURCE-COLLECTIONS, 2026-09-25, on base `ec70709a` (main
after the qcore-provider naming integration) merged with the finished
W43-INDIRECT-ACCESS-GENERAL branch (`6bb838c8`, merge `9c7774dc`). The card
teaches the C gateway to track **indexed resources**: a heap array whose
elements hold owned resources, filled and released in index loops. The
forecast was frozen before any rule existed ([PREDICTION.md](PREDICTION.md),
commit `ea33ff70`).

## Headline

* **Census first** ([§1](#1-census)). The pinned corpus has very few such
  arrays. There are 12 stores of an acquiring call's result into an array
  element, 4 stores of a name an acquiring call bound, and 16 contracted
  element releases. Around them are 20 loops: 9 fill (1 canonical counted),
  11 release (9 canonical) and **0 partial-cleanup** loops. Of the 12 Unknown
  functions holding a leaf on a collection line, **0 would clear fully**. The
  array allocation itself is the dominant blocker: `v = malloc(n * sizeof(*v))`
  is refused for its `sizeof(*v)` operand, in **224** leaves on **211** Unknown
  functions, 4 of them sole-blocked.
* **The rule** ([§2](#2-the-rule)). A heap array this body acquired, declared
  `T **v`, is a collection. Its element obligations are a partition of
  `[0, ∞)` into half-open segments over symbolic bounds (`0`, `i`, `i + 1`,
  `n`, a literal). Each segment is unfilled, null, live or released. The rule
  admits the following, and every operation must be proved on the segments:
  * `v[I] = ACQUIRE(...)` and `v[I] = NULL`;
  * `RELEASE(v[I])` and `if (!v[I])`;
  * a canonical counted loop `for (i = 0; i < n; i++)`, with its exit
    equation `i = max(0, n)`;
  * an append counter `v[n++]`.

  An operation whose array does not resolve to exactly one live, non-null
  identity this body acquired keeps the classifier's refusal **byte for
  byte**. No bounds claim is made.
* **Decided on fixtures.** The brief's shape (fill, partial cleanup on failure,
  release loop) is `Safe` with a rechecked certificate, and so is returning the
  filled collection. The following are provisional `Unsafe` candidates:
  * freeing the array with live elements, and a cleanup that frees only the
    array: element leaks;
  * a second release, element by element or by loop: `double_release`;
  * reading a released element, or an element through a released array:
    `use_after_release`;
  * storing over a live non-null element: a leak.

  Every unproved range refuses. That includes releasing `[0, n)` after filling
  `[0, i)`, a `break`, a bound written in the body, a narrower unsigned index,
  a borrowing callee, and paths that disagree. There are 30 tests
  ([§4](#4-tests)).
* **Lean.** `QCore.IndexedCollection` has 44 theorems, audited by
  `AuditIndexedCollection.lean`. It has no placeholders and no project axioms
  ([§3](#3-lean-parity)).
* **Arbiter** ([§5](#5-arbiter)). BEFORE reproduces the W43-INDIRECT-ACCESS-GENERAL
  diagnostics byte for byte. AFTER removes **0** leaves and adds **0**. It
  moves **0** functions and **0** units. The leak pin stays at **271**, and
  there are **0** violation candidates. The diagnostics differ from BEFORE
  only in the report's `scope` string, and the other four artifacts are
  byte-identical. This is exactly the frozen forecast.
  * The census-feature run shows **27** element operations lowered in **19**
    corpus functions: 21 stores or releases and 6 null tests.
  * In every one the array is a formal, a cast alias of one, or an array whose
    own allocation is refused. So every one keeps its fallback refusal, and
    the leaf is present in both runs.
* **Ledger: 307 → 314.** Nothing is retired. `W43-INDIRECT-POINTER-VALUE-STORE`
  is not discharged: its `a[i] = q` shape is exactly the name store this card
  keeps refused. Seven residuals are registered ([§7](#7-ledger)).

| Measure | BEFORE (W43-INDIRECT-ACCESS-GENERAL) | AFTER | Delta |
|---|---:|---:|---:|
| Functions Safe / provisional Unsafe / Unknown | 269 / 84 / 4,481 | 269 / 84 / 4,481 | 0 |
| Units Safe / provisional Unsafe / Unknown / data-only | 3 / 42 / 136 / 4 | 3 / 42 / 136 / 4 | 0 |
| `leak` / `unknown_alias` / `unsupported` | 271 / 4,727 / 47,832 | 271 / 4,727 / 47,832 | 0 |
| `double_release` / `invalid_release` / `use_after_release` | 0 / 0 / 0 | 0 / 0 / 0 | 0 |
| element operations lowered (census run) | — | 27 in 19 functions, all on their fallback | — |

## 1. Census

[`scripts/collection_census.py`](scripts/collection_census.py) reads source
tokens of the analysed `-E -P` unit texts. It captured them in a private
capture run of the base binary through a transparent Clang wrapper
([`scripts/clang-tee.sh`](scripts/clang-tee.sh),
[`scripts/capture_run.sh`](scripts/capture_run.sh)), and paired 180 of 185
units with `w43-libc-contracts/scripts/unit_texts.py`. The 5 unpaired units
are the same ones as before, and they project no function. The capture run's
diagnostics are byte-identical to BEFORE. Its `source-effects` differ only
because the wrapper's path is the recorded Clang
([run-identities.json](run-identities.json)).

An **acquiring** call is a selected declaration whose result is
`resource_return_owned` (16 names, `contract inherit` resolved). A
**releasing** call has a released or consumed parameter (13 names). W4
summaries are invisible to the census. Full output is in
[census/census.txt](census/census.txt) and
[census/census.json.gz](census/census.json.gz).

| Question | Measured |
|---|---|
| (a) element stores of an acquiring call's result / of a name it bound | **12 / 4** (index: name 8, `n++` 5, expression 3); besides them 53 other-name, 41 null, 25 uncontracted-call stores |
| (b) element releases through a contracted release | **16** (index: name 13, expression 2, constant 1); no `close(fds[i])` (no descriptor release is contracted) |
| (c) loops around (a)/(b) | 20: 9 fill (1 canonical), 11 release (9 canonical), **0 partial cleanup**; the corpus fills by `v[n++] = …` inside pointer or list traversals, and releases with `for` or `while (i-- > 0)` |
| (d) Unknown functions with a collection leaf / clearing fully | **12** (43 collection leaves) / **0** |
| (e) the allocation's own blocker, `sizeof(*v)` | **224** leaves, **211** Unknown functions, **4** sole-blocked |

The census lists every function with an element store or release. Two
patterns account for most of them:

* **Destroy helpers over a caller's array.** Examples are
  `destroy_prometheus_label_list`, `free_metrics` and
  `rd_kafka_conf_dump_free`. Their base is an unbound formal.
* **Builders whose array allocation is refused.** Examples are
  `rd_kafka_mock_sharegroup_get_member_ids`, `rd_kafka_ListGroups_resp_cb` and
  `rd_kafka_anyconf_dump`. They are refused for `sizeof(*v)`, a `->` in the
  size, or no null test.

The brief's own example also reads `src[i]` in the acquiring call's argument.
That call-argument access is `W43-INDIRECT-CALL-ARGUMENT-ACCESS`, and it keeps
the call refused.

## 2. The rule

The syntax lives in
[`crates/qcore-c-gateway/src/indexed_collection.rs`](../../../crates/qcore-c-gateway/src/indexed_collection.rs),
together with the segment domain and its 5 unit tests. The instructions and
transfer live in `legacy_c.rs`. The declaration facts are two additions to
`typed_field_access.rs`, both read by the same total-or-reject reader:
`TypedAccessModel::integer_class` and `declarator_facts`.

### 2.1 Admission (classifier)

The rule is consulted only for a statement the existing classifier returned as
`Unknown`, and for an `if` condition that is otherwise refused. It has these
premises:

* **Base.** `v` is declared exactly `T **v` at every declaration, and it is a
  slot no other lvalue designates (`frame_protected`).
* **Index.** The index is one of:
  * a literal;
  * a declared integer with that slot premise;
  * a counter `n++` of rank `int` if signed, or `long` if unsigned, which
    therefore cannot wrap.
* **Store.** `v[I] = CALL(...)` is admitted when the ordinary classifier,
  reading the statement with the element place renamed to a plain temporary,
  lowers it to one contracted call. That call must have an owned result and
  every parameter borrowed. The declared-ABI and literal-premise withdrawals,
  the borrowed actuals and the nested-call rule are therefore the ones every
  call already had. A declined probe restores the site counter.
  `v[I] = NULL` is admitted as well.
* **Release.** `CALLEE(v[I])` must classify to a one-parameter
  `resource_released` call with no result.
* **Null test.** `!v[I]`, `v[I]`, `v[I] == NULL` or `NULL == v[I]`.

Each lowering carries the refusal it replaces as its `fallback`
(`ElementStore`, `ElementRelease`, `BranchOnElementNull`).

A `for` loop around at least one lowered operation becomes
`CountedLoopHeader`/`CountedLoopStep` when all of these hold:

* its clauses spell `I = 0; I < B; I++` and were lowered to exactly
  `Clear I` and `ForgetScalar I`;
* `I` and `B` are declared integers with the slot premise;
* an unsigned index is no narrower than an unsigned bound (or the bound is a
  literal that fits an `int`);
* the body writes neither name and has no `break`, `goto`, label or `switch`.

The loop keeps the exit equation `I = max(0, B)`. No other loop changes: a
function with no element operation has a byte-identical CFG.

### 2.2 Transfer

* **Collection identity.** The base must pass `resolved_aggregate_owner`
  (one known, non-null, live identity this body acquired, not a caller's)
  exactly as a member store's aggregate must. Otherwise the node **is** its
  fallback: the same diagnostic, the same frame, and no effect of the inner
  call.
* **Element store.** The inner call runs as an ordinary call, with the
  element place as its destination. Its one fresh identity then leaves the
  resource map, and every other binding of it is made unknown: its custody now
  belongs to the collection. The store must prove where the index falls:
  * **at or past the end:** append, with an unfilled gap if needed;
  * **inside an unfilled, null or released segment:** split it;
  * **inside a live non-null segment:** overwrite, which is a leak candidate.

  A possibly-null live element, or an index that cannot be placed, refuses
  with `UnknownAlias` on the element place, and the collection becomes
  `Unknown`.
* **Element release.** A live element becomes released. A null element is a
  no-op, but only under a nullable contract. The same holds for a
  possibly-null live element. A released element is a `double_release`
  candidate. Anything else refuses.
* **Null test.** A possibly-null live element is split on each edge. A
  provably live or provably null element makes the other edge infeasible. It
  is an `if`, never a loop header, so constraint 5c's exit-edge rule is
  untouched. A released element is a `use_after_release` candidate, and an
  unfilled one refuses.
* **Loops.**
  * A counted loop's body edge carries `I < B`.
  * Its step renames every bound `I + k` to `I + (k - 1)`.
  * Its exit edge rewrites `I` to `B`. An offset other than 0 refuses, and an
    index still zero at the exit records a clamped zero for the bound.
  * A body edge whose bound is provably zero when clamped is **vacuous for
    the collection part only**: the index, never negative, cannot be below it.
    The edge and every other map are kept, so no control flow is removed.
  * Every other write to a name a bound reads makes the collection `Unknown`.
  * Joins keep the more symbolic of the two forms when the other collapses to
    it under its own facts, and otherwise go to `Unknown`. The rule therefore
    proves:
    * the fill invariant `[0, i)` live;
    * the sweep `[0, j)` released, `[j, n)` live;
    * the partial cleanup releasing exactly the filled prefix, with the failed
      element null.
* **Escape.** At an exit, the result depends on what happened to the array:
  * **released, still held or refuted, with a live segment:** the elements
    leak (a `leak` candidate, reported from the settled exit state);
  * **escaped, whether returned, consumed or delivered through an out slot:**
    the elements go with it, as a member link's children do;
  * **escaped on only some paths, or `Unknown`:** no claim, and the exit is
    refused unless its path already is.

  A returned or out-slot array still owing elements is observed as
  `Ambiguous`, so no one-identity W4 summary is published for it.
* **Owner uses.** Handing an array that may still owe an element to any call
  position other than a release or a consumption refuses before the call's
  effects. That includes the element store's own inner call.
* **Settled reporting.** Every accusation comes from the **settled** states,
  as constraint 6 asks: `double_release`, `use_after_release` (an element, or
  any element operation through a released array), overwrite leaks and exit
  element leaks. Each is provisional `Unsafe` under the existing policy and
  never a validated upstream defect.
* **Scheduling.** In a function with no element operation the state carries
  no collection, no fact and no vacuous flag. Its join and its work-list
  schedule are therefore exactly as before.

### 2.3 Property scope (invariant 2)

The report's `scope` string and spec L18 now add this clause: *an indexed
collection's element ranges decide which indices owe a release, never that an
index is inside the allocation.* This is the brief's "no bounds claim".

### 2.4 Descriptor kinds

A live segment records one element kind: an owned pointer identity. The kind
is a property of the slot, not of the collection. Descriptor elements (`int`
fds) therefore slot in as a second live kind once
W43-DESCRIPTOR-RESOURCE-TRACKING lands. No descriptor function is declared
here.

## 3. Lean parity

[`QCore.IndexedCollection`](../../../proof/QCore/IndexedCollection.lean) is
audited by [`AuditIndexedCollection.lean`](../../../proof/AuditIndexedCollection.lean).
It has 44 theorems, no placeholders and no project axioms. It models the
following:

* **Meaning.** A segment list denotes an element-state function under a
  valuation, and bounds are clamped (`Term.eval`, `cell`).
* **Step and exit.** `the_step_preserves_every_element` and
  `the_exit_substitution_preserves_every_element`, under the exit equation.
* **Fill.** `the_fill_form_is_the_filled_prefix`,
  `one_fill_iteration_keeps_the_invariant`,
  `the_stepped_fill_is_the_fill_form`, `the_fill_exit_is_the_whole_range`,
  and `the_fill_entry_collapses_to_the_fill_form`, which is the join at the
  header.
* **Release sweep.** `one_release_iteration_releases_exactly_element_j`,
  `the_stepped_sweep_is_the_sweep_form`, `the_sweep_exit_owes_nothing` and
  `the_sweep_entry_collapses_to_the_sweep_form`.
* **Partial cleanup.** `the_cleanup_starts_from_the_failed_fill` and
  `the_partial_cleanup_discharges_exactly_the_prefix`.
* **Facts.**
  * `a_body_fact_orders_the_bounds` is the Rust `le_fact` condition
    `kt + ka - 1 <= kb`.
  * `a_known_zero_evaluates_its_bounds`,
    `a_clamped_zero_bounds_nonpositive_offsets`,
    `an_exit_at_zero_clamps_the_bound`.
  * `a_zero_bound_admits_no_iteration` is the vacuous edge.
* **Decisions and violations.**
  * `a_second_release_is_a_double_release`, `only_a_live_element_is_released`
    and `an_unfilled_element_is_never_released`.
  * `reading_a_released_element_is_a_use_after_release` and
    `an_unfilled_element_is_never_read`.
  * `overwriting_a_live_element_is_a_leak` and
    `a_possibly_null_live_element_refuses_a_store`.
  * The exit claims: `freeing_the_array_with_live_elements_leaks_them`,
    `an_escaped_array_carries_its_elements`,
    `an_array_owing_nothing_is_discharged`, `unknown_ranges_claim_nothing` and
    `a_never_filled_array_owes_nothing`.
* **Join, classifier, fallback and verdicts.**
  `the_joined_form_describes_both_sides`,
  `an_admitted_statement_keeps_its_lowering`,
  `a_refusal_without_an_element_shape_is_kept`,
  `an_unresolved_array_is_exactly_the_fallback`, `a_refusal_is_never_safe`
  and `a_candidate_is_unsafe`.
* **Owner uses.** `a_borrow_of_an_owing_collection_refuses`,
  `a_reallocation_of_an_owing_collection_refuses` and
  `releasing_or_consuming_the_array_is_admitted`.
* **Non-vacuity.** `three_filled_elements_are_live`, and a sweep of three that
  owes nothing.

Some premises are stated, not proved. They fall under trust boundary 28, and
are what the recognition reads:
* the C facts behind the exit equation: the index starts at zero, is written
  only by its step, and does not wrap;
* the bound is not written;
* no `break`, `goto` or label is taken;
* the slot premise of every bound name.

The same holds for the token and type extraction, and for the Rust–Lean
correspondence.

## 4. Tests

[`crates/qcore-c-gateway/tests/indexed_collection.rs`](../../../crates/qcore-c-gateway/tests/indexed_collection.rs)
has 30 tests. Each asserts the complete diagnostic vector with the status, and
rechecks every `Safe` certificate.

* **Safe:**
  * the brief's fill, partial cleanup and release;
  * returning the filled collection;
  * a never-filled array;
  * a null-initialised array;
  * an append counter released by a counted loop;
  * constant indices with a non-null acquirer and release;
  * a consumed collection.
* **Unsafe:**
  * an element leak;
  * a cleanup that frees only the array;
  * a double release, element by element and loop by loop;
  * a released element read;
  * an element released through a released array;
  * an overwritten live element.
* **Refused:**
  * releasing past the filled prefix;
  * a bound written in the body;
  * a `break`;
  * a narrower unsigned index;
  * a borrowing callee;
  * an inner call borrowing the collection;
  * a possibly-null element given to a non-null release;
  * an element filled on one path only;
  * disagreeing paths at an exit;
  * a helper returning a collection (no summary is published);
  * an array reallocated in a loop.
* **Kept byte for byte:**
  * an unguarded array;
  * a caller's array;
  * an index expression;
  * a named-pointer store;
  * a scalar array.

The alias probe example renders the new instructions. No existing pin moved.

## 5. Arbiter

### 5.1 Runs

Each run uses
[`../w43-return-expression/scripts/run_audit.py`](../w43-return-expression/scripts/run_audit.py)
through the W43-INDIRECT-ACCESS-GENERAL copy (with `--binary`), with the same
pin, Clang and ChunkIO package as the baseline.

| Run | Binary | Tree | Exit | Record |
|---|---|---|---:|---|
| BEFORE | `7ba13086…` | merge `9c7774dc` | 0 | [before-run.json](before-run.json), [before-identity.json](before-identity.json) |
| capture | `7ba13086…` | the same, through the tee wrapper | 0 | private; see §1 |
| AFTER | `b6278fd7…` | `850bed9c` (the audit binary is unchanged by the later commits) | 0 | [after-run.json](after-run.json), [after-diff.json](after-diff.json), [artifacts/](artifacts/) |
| CENSUS (after) | `20e4ee44…` | `850bed9c` + `indirect-census` | 0 | [census-after-run.json](census-after-run.json), [census/element-reach.txt](census/element-reach.txt) |

**BEFORE against the W43-INDIRECT-ACCESS-GENERAL baseline**
([`scripts/identity.sh`](scripts/identity.sh)):

* diagnostics, chunk-contracts and source are byte-identical;
* `contracts` differs, because main's verdict-neutral contract changes (the
  inherit adoption and naming) sit on this merged base;
* `source-effects` carries the checkout-path digests the brief lists.

**AFTER against BEFORE** ([`scripts/diff_runs.py`](scripts/diff_runs.py),
[run-identities.json](run-identities.json)):

* 0 leaves removed, 0 added;
* 0 function and 0 unit status moves;
* identical counts;
* no top-level field differs except `scope`;
* chunk-contracts, contracts, source-effects and source are byte-identical.

The census run's five artifacts are byte-identical to AFTER, which is the
feature's inertness claim.

Two earlier AFTER runs, on `48c468c1` and `ae3f1439`, are byte-identical to
this one and kept privately. One AFTER run was stopped by its task handle
because it was launched with a mislabelled source commit. It was set aside as
`aborted-after3-mislabelled` and is not evidence.

### 5.2 Attribution

There is nothing to attribute leaf by leaf: no leaf moved. The census run
explains why.
[`scripts/element_reach.py`](scripts/element_reach.py)
([census/element-reach.txt](census/element-reach.txt)) finds **27** distinct
element operations lowered in 19 functions: 21 stores or releases and 6 null
tests. For each one, the leaf carrying its fallback's description is present
in both BEFORE and AFTER at its line. The operations and their arrays are:

* **The cmetrics `destroy_*_list` helpers.** There are seven, and each writes
  `NULL` into a caller's `T **` list.
* **The destroy helpers over a caller's array.**
  `destroy_prometheus_label_list`/`_sample_list`, `free_metrics`,
  `free_resource_attributes`, `rd_kafka_DeleteGroup_destroy_array`,
  `rd_kafka_DeleteTopic_destroy_array` and
  `rd_kafka_mock_request_destroy_array`.
* **`rd_kafka_conf_dump_free`.** It acts through `_arr = (char **) arr`, an
  unknown binding.
* **`rd_kafka_ListGroups_resp_cb` and
  `rd_kafka_telemetry_encode_metrics`.** Their allocation statement is
  refused.
* **Null tests in `append_data_point_to_metric` and
  `initialize_exemplars_from_metadata`.** These test caller arrays.

The source census counted 16 contracted releases. The lowering reached 21
stores or releases. The difference is the 9 null stores in destroy helpers
and three `*_destroy_array` releases, whose contracts the source-level regex
did not parse. The analyzer's own reading, above, is the authority.

### 5.3 Prediction against outcome

| Forecast | Measured | |
|---|---|---|
| 0 / 0 leaves removed / added | 0 / 0 | hit |
| 0 function, 0 unit moves | 0 / 0 | hit |
| leak 271, `unknown_alias`/`unsupported` unchanged | 271, unchanged | hit |
| 0 violation candidates | 0 | hit |
| only the `scope` string differs | only the `scope` string differs | hit |
| no statement passes the base premise | none did: 27 lowered operations, all on their fallback | hit |

The frozen file is not edited.

### 5.4 The leak pin

**271, unmoved (constraint 7).** No element leak was reported, because no
corpus array resolved. The pin records the accusing arms' absence from the
corpus (`W43-COLLECTION-ACCUSING-UNEXERCISED-IN-CORPUS`), not their
verification.

The new baseline is [artifacts/](artifacts/), with its
[manifest.json](artifacts/manifest.json). The file
`docs/reports/w43-indirect-access-general/artifacts/` is its frozen
predecessor.

## 6. Validation

Measured in this worktree (`agent-a21f70f046fa88069`) by
[`scripts/run_gate.py`](scripts/run_gate.py). That is
W43-INDIRECT-ACCESS-GENERAL's runner with `CARGO_INCREMENTAL=0`, following the
coordinator's disk rule. It captures every exit to a file and never through a
pipe. Logs are in `private/gate/` and `private/provider-gate/`, and the
generator input is [gate.json](gate.json).

GATE_TABLE

## 7. Ledger

**Retired: none.** `W43-INDIRECT-POINTER-VALUE-STORE` names `p->f = q`,
`*pp = q` and `a[i] = q`. Its array part is a store of a *name*, which this
card deliberately keeps refused (`W43-COLLECTION-NAME-STORE`). The card admits
only stores of an acquiring call's result and of `NULL`, and no leaf of that
entry moved. It stays registered and unedited.

**Registered** ([followups.json](followups.json)):

| Id | What stays open |
|---|---|
| `W43-COLLECTION-SIZEOF-DEREFERENCE-ALLOCATION` | `sizeof(*v)` keeps the allocation refused: 224 / 211 / 4 |
| `W43-COLLECTION-NAME-STORE` | `v[i] = p` stays refused (the stored name's later uses) |
| `W43-COLLECTION-LOOP-SHAPES` | only the canonical counted `for`; append-over-traversal fills, `while (i-- > 0)` sweeps, member bounds; duplication with W43-INDEX-TRAVERSAL's fixpoint |
| `W43-COLLECTION-CONTRACT-VOCABULARY` | no clause for a collection result or a collection release: destroy helpers stay Unknown, returned collections publish no summary |
| `W43-COLLECTION-AUTOMATIC-AND-DESCRIPTOR-ARRAYS` | `T *a[N]` owners and `int fds[N]` elements |
| `W43-COLLECTION-ACCUSING-UNEXERCISED-IN-CORPUS` | element leak / double release / use after release reach no corpus function |
| `W43-COLLECTION-UNREACHED-ELEMENT-NODE` | the certificate firewall's retained-`Unknown` caution and element nodes carrying a fallback |

**Noted, not retired:**

* `W43-INDIRECT-CALL-ARGUMENT-ACCESS`: the brief's own `strdup(src[i])` is
  refused by it.
* `POISON-UNMASK-ON-ADMISSION`: no admission reached the corpus, so it was not
  measured here.

Follow-ups go from **307 to 314**. `corpus.cycle19_raw_units` points at this
report's artifacts, and its previous contents are in
[previous-corpus-row.json](previous-corpus-row.json).

## 8. For the integrator

* **Proposed gateway amendment**, a new constraint 5i beside 5g/5h, in these
  words:

  > W43-INDEXED-RESOURCE-COLLECTIONS adds indexed resource collections. For a
  > statement or `if` condition the existing classifier refused, over a base
  > declared `T **v` and an index that is a literal, a declared integer, or a
  > non-wrapping counter `n++` (every name a slot no other lvalue designates),
  > `v[I] = ACQUIRE(...)` (an owned-result, all-borrowed contract call as the
  > ordinary classifier reads it), `v[I] = NULL`, `RELEASE(v[I])`
  > (one-parameter released contract, no result) and a null test of `v[I]`
  > lower to element instructions carrying the refusal as their fallback. When
  > `v` does not resolve to one live, non-null identity this body acquired,
  > the node is its fallback byte for byte. Otherwise the element obligations
  > are half-open segments over symbolic bounds, every operation is proved on
  > them or refuses with `UnknownAlias`, a canonical counted
  > `for (i = 0; i < n; i++)` around an element operation (no write of either
  > name, no `break`/`goto`/label/`switch`, no unsigned wrap) carries `i < n`
  > into its body and `i = max(0, n)` out of it, and a body edge whose bound
  > is zero when clamped is vacuous for the collection part only; no control
  > flow is removed. Element leaks at an exit, `double_release`,
  > `use_after_release` and overwrite leaks are reported from the settled
  > states; a returned or out-slot array still owing elements publishes no
  > one-identity summary; handing it to a call other than a release or a
  > consumption refuses. The collection decides which indices owe a release,
  > never bounds.

* **Constraint 7:** record W43-INDEXED-RESOURCE-COLLECTIONS as leaving the pin
  at 271, with no leaf added or withdrawn.
* **Duplication with W43-INDEX-TRAVERSAL.** That card owns custody-neutral
  index and pointer traversal and the loop fixpoint for custody-neutral
  facts. It had not landed. This card's index facts are local to collections
  and consumed only by them:
  * the body-edge `i < n` and the exit equation;
  * the known and clamped zeros;
  * the vacuous edge.

  They must be moved onto that card's shared loop analysis when both are on
  main (`W43-COLLECTION-LOOP-SHAPES`).
* **W43-DESCRIPTOR-RESOURCE-TRACKING.** The live slot's element kind is where
  a descriptor kind belongs (§2.4).
* **Files likely to conflict with main:**
  * `crates/qcore-c-gateway/src/legacy_c.rs`:
    * `LegacyInstruction` (five variants);
    * `Statement::If`/`For` (two fields), `Parser` (two fields),
      `classify_simple`, and the `if`/`for` parse and compile arms;
    * `AbstractState` (five fields) and its join;
    * `transfer`, which is now a wrapper over `transfer_base`;
    * `TransferContext`, the edge loop and the settled post-passes;
    * `observe_exit`/`observe_out_slot`/`observe_residual`,
      `refine_pointer_null`, `rebound_names`, `first_free_site` and
      `acquisition_lines`.
  * `crates/qcore-c-gateway/src/typed_field_access.rs`: two appended
    functions and a struct at the end.
  * `legacy_joined_provenance.rs` and `legacy_recursion.rs`, each with one arm.
  * `lib.rs` (a module).
  * `crates/qcore-audit/src/report.rs`: the `scope` string. It changes every
    future diagnostics artifact.
  * `compiler/examples/alias_aggregate_probe.rs`.
  * `proof/QCore.lean`.
  * `docs/QCORE_SPEC_r0_3.md` §L18.
  * The shared records: `docs/reviews/r0_3/acceptance_matrix.json`,
    `docs/STATUS.md`, `docs/development/increments/wave-43.md` and
    `README.md`, and `docs/reports/INDEX.md`.
* **The merge base.** This branch merged W43-INDIRECT-ACCESS-GENERAL
  (`6bb838c8`) into `ec70709a` itself (merge `9c7774dc`).
  * The ledger was resolved entry by entry: 298 on main plus the card's 12,
    minus its 3 retired, is 307.
  * `STATUS.md` keeps main's rows and adds the card's.
  * The increment index names `wave-43.md` as the current record, with both
    wave-41 continuations listed.
* The `contracts` and `source-effects` artifacts embed this worktree's path,
  as earlier isolated cards' artifacts did.

## Reproduction

```sh
python3 docs/reports/w43-indirect-access-general/scripts/run_audit.py <before> <base> --binary <base binary>
docs/reports/w43-indexed-resource-collections/scripts/identity.sh <before>
docs/reports/w43-indexed-resource-collections/scripts/capture_run.sh <base binary> <capture>
python3 docs/reports/w43-libc-contracts/scripts/unit_texts.py <before>/resource-diagnostics.json <capture>/ppP <unit-texts>
cp <before>/resource-diagnostics.contracts.json <before>/contracts.json
python3 docs/reports/w43-indexed-resource-collections/scripts/collection_census.py <before> <unit-texts> census.json census.txt
python3 docs/reports/w43-indexed-resource-collections/scripts/show_function.py <before> <unit-texts> <unit key> <function>
python3 docs/reports/w43-indirect-access-general/scripts/run_audit.py <after> <commit> --binary <after binary>
python3 docs/reports/w43-indirect-access-general/scripts/run_audit.py <census-after> <commit>+census --census --binary <census binary>
python3 docs/reports/w43-indexed-resource-collections/scripts/diff_runs.py <before> <after> after-diff.json
python3 docs/reports/w43-indexed-resource-collections/scripts/element_reach.py <census-after> <after> <before> element-reach.json element-reach.txt
python3 docs/reports/w43-indexed-resource-collections/scripts/run_identities.py <runs> run-identities.json before:after after:census-after before:capture
python3 docs/reports/w43-libc-contracts/scripts/archive.py <after> docs/reports/w43-indexed-resource-collections/artifacts
python3 docs/reports/w43-indexed-resource-collections/scripts/update_ledger.py
python3 docs/reports/w43-indexed-resource-collections/scripts/run_gate.py <gate-dir> submodules clang test fmt clippy probe followup lean census
```

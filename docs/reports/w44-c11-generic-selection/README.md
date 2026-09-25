# W44-C11-GENERIC-SELECTION: C11 generic selections, read by selection invariance

Card W44-C11-GENERIC-SELECTION, 2026-09-25. Base: main `70ce6b44`. The card
makes the C gateway read C11 generic selections, `_Generic (control, T1: e1,
…, default: ed)` (C11 §6.5.1.1), soundly, with a Lean model and audit.
Before it, every selection reached the classifier as a call to an uncontracted
function `_Generic`, or as an unmodeled call in the condition, return or
argument it sat in.

A [prediction](PREDICTION.md) was committed (`14118941`) before any corpus run
of the reader. As that file discloses, the reader, its fixtures and the Lean
model were written, and the fixtures run, before the commit; only the BEFORE
and capture runs of the unchanged base existed. [§6](#6-prediction-against-outcome)
compares the prediction with the outcome.

## Headline

* **Census: 85 selections, all glibc's.** Every `_Generic` in the 185 analysed
  units is glibc 2.43's C23 const-preserving `__glibc_const_generic` expansion
  (`strchr` 51, `bsearch` 13, `strstr` 13, `strrchr` 6, `memchr` 2), in 36
  functions of 25 units, all inside function bodies, none nested. There is no
  wide-character search, no `strpbrk` and no other source (mpack's
  `mpack_write` selection is expanded in no analysed unit).
* **Route 1, selection invariance, built.** A selection whose associations are
  one expression up to enclosing parentheses and pointer casts is lowered to
  that expression before the body is parsed; the controlling expression is
  dropped. A removed cast needs a direct call the unit declares with an
  object-pointer result. Every other selection is left byte for byte as it was.
  Route 2 (Clang's `GenericSelectionExpr`) was not built; §2 says why.
* **Lean.** `QCore.GenericSelection` models C11's meaning (the control is not
  evaluated; exactly the selected association is) and proves that an admitted
  selection's associations all evaluate as the core, so the lowered effect is
  independent of the selection, and that the whole rewrite preserves every
  defined outcome whatever the compiler selects. `AuditGenericSelection.lean`,
  25 theorems, no placeholder, no project axiom.
* **Arbiter on `70ce6b44`.** BEFORE reproduces the W44-LIBC-RESIDUALS
  integration baseline's diagnostics byte for byte. AFTER: **37 leaves removed,
  34 added, every one attributed**, all in the 36 selection functions or in the
  one caller of a newly `Safe` one. The 14 `_Generic` leaves go (6 exchanged
  for `bsearch`), 22 condition refusals go, 13 `unknown_alias` and 15 leaks are
  added. **1 function Unknown → Safe** (`mk_string_char_search`). No unit moves.
* **Leak 19 → 34.** All 15 added leaks are in
  `ut_sasl_oauthbearer_oidc_assertion_create`, already provisional `Unsafe`:
  the same two `rd_strndup` objects the leak audit classed (a) at the
  function's 9 earlier leaves, now reported at the later `RD_UT_ASSERT` failure
  returns. Read: 15 (a), 0 artifacts, 0 premise gaps. No violation candidate.
  No function becomes provisional `Unsafe`.
* **Follow-ups 377 → 378.** Nothing to retire (no registered entry named the
  selection); `W44-BSEARCH-UNDECLARED` registered.

| Measure | BEFORE (`70ce6b44`) | AFTER | Delta |
|---|---:|---:|---:|
| Functions Safe / provisional Unsafe / Unknown | 300 / 9 / 4,525 | **301 / 9 / 4,524** | +1 / 0 / −1 |
| Units Safe / provisional Unsafe / Unknown / data-only | 3 / 8 / 170 / 4 | 3 / 8 / 170 / 4 | 0 |
| `unsupported` | 43,885 | **43,854** | −31 |
| `unknown_alias` | 3,357 | **3,370** | +13 |
| `leak` | 19 | **34** | +15 |
| `summary_conflict` / `double_release` / `invalid_release` / `use_after_release` | 1 / 0 / 0 / 0 | 1 / 0 / 0 / 0 | 0 |
| `call to uncontracted function _Generic` leaves / functions | 14 / 10 | **0 / 0** | −14 / −10 |
| Uncontracted-call leaves / Unknown functions / sole-blocked | 9,221 / 2,618 / 409 | 9,214 / 2,616 / 408 | −7 / −2 / −1 |
| Indirect-access, return-expression families | 18,101 / 3,357 / 428; 495 / 417 / 147 | unchanged | 0 |

Family counts: [families-before.json](attribution/families-before.json),
[families-after.json](attribution/families-after.json)
([`family_counts.py`](scripts/family_counts.py); BEFORE reproduces the STATUS
figures).

## 1. Census

**Method.** A private run of the pinned audit with `QCORE_CLANG` set to the
transparent wrapper `docs/reports/w43-libc-result-alias/scripts/clang-tee.sh`
([`run_capture.py`](scripts/run_capture.py)) saved every `-E -P` text the
analyzer read; its diagnostics are byte-identical to BEFORE
(`988ced1a…`, [capture-run.json](runs/capture-run.json)).
[`generic_census.py`](scripts/generic_census.py) reads the 187 distinct texts,
matches each to its diagnostics unit by the functions it defines, and records
for every `_Generic` token its unit, line, enclosing function, shape, position,
nesting, the verdict of an independent Python reading of the admission rule,
and the BEFORE refusals of the statement it sits in.
[`census_table.py`](scripts/census_table.py) summarises per function.
Outputs: [census.json](census/census.json),
[census-table.json](census/census-table.json).

| Axis | Count |
|---|---|
| Occurrences / functions / units | 85 / 36 / 25 (23 librdkafka; Monkey `mk_core/mk_string.c`, `mk_server/mk_config.c`) |
| Shape | glibc `__glibc_const_generic`: `strchr` 51, `bsearch` 13, `strstr` 13, `strrchr` 6, `memchr` 2; other 0 |
| Nesting | top level 85; in another selection's control or association 0 |
| Position (the token before the selection) | `=` 40 (31 assignments, 9 declaration initializers; many are `if ((t = strchr (…)))`), condition operand 35, whole condition 9, parenthesised 1 |
| Python reading of the rule | admitted 85 |
| BEFORE refusals at the 85 statements | `call to uncontracted function _Generic` 14; branch condition 56; braced initializer 28; indirect access 12; loop and for-loop condition 3; return expression 2; unsupported assignment expression 1; plus 9 leak leaves in `ut_sasl_oauthbearer_oidc_assertion_create` |
| Functions' BEFORE status | Unknown 35, provisional Unsafe 1 |

The 14 leaves of the card's table are the statements where the selection was
the first call the classifier met. The other 71 sit in statements the
classifier already refused for another reason, most of them conditions whose
calls were the selections.

## 2. Route chosen and why

**Route 1 (selection invariance).** All 85 occurrences are the glibc shape,
whose two associations are `CALL` and `(CTYPE) (CALL)`: the same call up to a
qualifier-adding pointer cast. Lowering to `CALL` is exactly what a pre-C23
header produced, so the lowered statement meets the ordinary classifier and
the call's declared contract as if the source had spelled it; a derived
`strchr` result stays a derived result, with no cast in the way. Route 1
decides from the preprocessed token stream alone, so W4 derivation, W4
validation and certificate rechecking reproduce it, as constraint 8 requires of
the ABI and mode readers.

**Route 2 (Clang-resolved) was not built.** It would read the typed Clang AST,
which the gateway keeps off every decision a certificate must reproduce
(constraint 8's reasoning), and it would admit nothing route 1 refuses in this
corpus: route 1 admits all 85. It stays available if a non-invariant selection
ever appears; none does.

### 2.1 The reader

`crates/qcore-c-gateway/src/legacy_generic_selection.rs`,
`resolve_in_body`, is called once at the top of `build_function_cfg_with`
(one commented hook in `legacy_c.rs`), so every CFG build — ordinary, derived
return, W4 derivation and validation, certificate rechecking — sees the same
lowered body. Only the body's tokens change. A selection is admitted only when:

1. it is `_Generic ( control , key : expression {, key : expression} )` with a
   nonempty control, nonempty keys and expressions, and at most one `default`;
2. after enclosing parentheses and leading pointer casts are removed, every
   association is token for token the same core. A cast is removed only when
   its type name is identifiers, then `*`s and pointer qualifiers, and nothing
   else, and its operand is a whole parenthesised group, a direct call or a
   single name (so `(T *) a + b` is never read as a cast of `a + b`);
3. if any cast was removed, the core is a direct call `NAME ( … )` and
   `declared_abi::declares_object_pointer_result` reads the unit's own C
   declaration of `NAME` (through any withdrawal prefix) as returning an object
   pointer. This is the same declaration reader and token stream as the ABI
   gate.

Associations are resolved innermost first; the control is never rewritten,
because it is dropped whole. A refused selection, inner selections included, is
emitted unchanged and keeps whatever refusal it had. The lowered core is
parenthesised unless it is a direct call or a name, keeping the selection's
primary-expression precedence. No `LegacyInstruction` variant is added, so
`descriptor_custody::admits_descriptor_read` is not touched; constraint 5i and
the refused hand-on rule apply to the lowered statement unchanged.

### 2.2 Why it is sound

Evaluating the selected association evaluates its core once (casts and
parentheses evaluate nothing) and yields the core's value converted by the
removed casts. Each removed cast converts an object pointer to an object
pointer, which leaves the designated object unchanged (C11 §6.3.2.3p2, p7) under
the no-undefined-behaviour object model the gateway already assumes
(`AOM-POISON-NO-UB`). So every association has the core's effects and value,
whichever the compiler selects, and C does not evaluate the control.

The equality in rule 2 is over the analyzer's token view, which collapses
string and character literals to one token. Two cores that differ only in a
literal compare equal; the analyzer lowers both identically because it reads
nothing else, and its lowering covers whichever is selected. That is trust
boundary 28's tokenizer premise, already stated for `QCore.LiteralTailActual`.
In glibc's shape both associations are the same macro argument, so the
literals are equal in fact.

## 3. Lean

`proof/QCore/GenericSelection.lean`, registered in `proof/QCore.lean` and
audited by `proof/AuditGenericSelection.lean` (25 `#print axioms`; each depends
on `propext` and at most `Quot.sound`). The model has abstract expressions
(opaque leaves, pointer and non-pointer casts, parentheses, binary operators,
generic selections), a deterministic evaluation to an effect trace and a value,
and the compiler's selection as an opaque function of the control and the keys.

* C11 §6.5.1.1: `exactly_the_selected_association_is_evaluated`,
  `the_control_is_unevaluated`, and in the example semantics
  `a_controls_effect_never_appears` (a control with effect `99` contributes
  nothing under either selection).
* Route 1: `an_admitted_association_evaluates_as_the_core`,
  `the_lowered_effect_is_independent_of_the_selection`, and for the rewrite as
  a whole, `resolve_preserves_outcomes` (every defined outcome of every
  expression is preserved, nested selections included, for every selection
  function), with `a_refused_selection_is_unchanged` and
  `the_reader_ignores_the_control`.
* What refuses: `differing_associations_refuse`,
  `a_cast_without_a_pointer_core_refuses`, `a_non_pointer_cast_is_not_removed`,
  `two_defaults_refuse`, `no_association_refuses`.
* Non-vacuity: the glibc shape resolves to the call and both of its selections
  evaluate as the call; a nested selection resolves to the call; a
  non-invariant selection and a pointer cast of an integer call are refused,
  and in each the two selections evaluate differently, so no lowering could be
  right for both; the example semantics satisfies both premises.

Two premises are hypotheses, not theorems (trust boundary 28):
`ValuePreserving` (a pointer cast leaves a pointer's value) and
`PointerResults` (a call the unit declares with a pointer result yields a
pointer). The token reader's parse of casts, operands and associations is the
extraction premise; the Rust keeps its operand rule narrow for that reason. As
with every gateway module, the Lean is an abstract rule model, and its
correspondence with the Rust is maintained by review.

## 4. Tests

`crates/qcore-c-gateway/tests/generic_selection.rs` (11 tests) reproduces
glibc's macro over the generic `d_*` fixture surface, so `d_chr` is spelled
exactly as a `_GNU_SOURCE` unit spells `strchr`. Each admitting fixture is
analysed twice, through the macro and with the bare call, and the complete
diagnostic vectors and statuses must be equal and equal a pinned outcome;
`Safe` results recheck their certificate.

| Fixture | Outcome |
|---|---|
| const argument (the cast is selected) | Safe, as the call |
| mutable argument (the call is selected), then `free(base)` and a read | Unsafe `use_after_release`, as the call: the derivation survives |
| mutable argument, `free(p)` of the interior pointer | Unsafe `invalid_release`, as the call |
| declaration initializer | Safe, as the call |
| condition `if (strchr …)`, `if (!strchr …)` | Safe, as the call |
| return, call argument | the call's outcome, with no `_Generic` leaf |
| nested (`strchr (strchr (…), …)` through the macro; a selection inside a selection's association) | the call's outcome; Safe |
| control with a side effect (`_Generic (d_alloc (8), …)`) | Safe, no leak; the same allocation evaluated is a leak |
| differing associations (two calls; a call and a name) | refused: `call to uncontracted function _Generic` |
| pointer cast of an integer call | refused |
| cast of a prefix (`(const char *) d_chr (s, 58) + 1`) | refused |

Nine unit tests in the module pin the reader's token output.

**Reachability.** [`mutate.py`](scripts/mutate.py) rewrites one line, runs the
fixture file, restores the bytes and checks the round trip
([mutations/](mutations/)): disabling the hook fails the 8 admitting tests;
making the pointer-result check always pass fails
`a_cast_of_an_integer_result_refuses`; disabling the invariance comparison
fails `associations_that_differ_refuse` and
`a_cast_that_does_not_cover_the_association_refuses`. All three round trips
are byte-identical, and the gate below ran after the last mutation. The hook
mutation was run by the script's first, single-mutation revision; the shipped
harness includes it unchanged. The closed-operand rule of §2.1 is not
separately reachable: a removed cast whose core is not a direct call already
refuses under rule 3, so the rule is kept as a guard on the extraction
premise, not as a tested arm.

## 5. Arbiter

Command shape: `docs/reports/w43-return-expression/scripts/run_audit.py`
(pinned Fluent Bit source, Monkey compile database, `QCORE_CLANG=/usr/bin/clang`,
wave-20 ChunkIO package), release binary of this worktree.

**BEFORE** (`70ce6b44`, [before-run.json](runs/before-run.json),
[before-identity.txt](runs/before-identity.txt), `baseline_identity.py` with
this worktree aliased to the integrator's root): diagnostics, chunk-contracts
and source are byte-identical to `docs/reports/w44-libc-residuals/integration/artifacts/`;
contracts is identical once the root is aliased; source-effects differs only in
the path-bound digests, as W44-LIBC-RESIDUALS recorded for the same comparison.

**AFTER** (`3e5a93a0`, [after-run.json](runs/after-run.json)): only the
diagnostics differ from BEFORE; contracts, chunk-contracts, source-effects and
source are byte-identical. Archived as the new baseline in
[artifacts/](artifacts/) (manifest raw diagnostics `bef6001b…`).

**Attribution** ([`attribute_generic.py`](scripts/attribute_generic.py),
[attribution.json](attribution/attribution.json)) classifies every differing
leaf of every function; anything outside the classes exits 1. It exits 0.

| Class | Removed | Added |
|---|---:|---:|
| `generic_leaf_removed`: the 14 `_Generic` leaves | 14 | 0 |
| `at_selection_statement`: the lowered call's own answer at the statement | 22 unsupported (condition refusals) | 6 unsupported (`bsearch`), 3 unknown_alias, 15 leak |
| `elsewhere_in_selection_function` | 0 | 10 unknown_alias |
| `newly_safe_callee`: a caller's leaf naming a function now `Safe` | 1 | 0 |
| unattributed | 0 | 0 |

### 5.1 Every function that moves

* `mk_string_char_search` (Monkey, `mk_string.c`): **Unknown → Safe**. Its only
  leaf was the selection around `memchr (string, c, len)`; lowered, `memchr`'s
  declared interior result binds `p`, `if (p)` tests it and `return (p - string)`
  returns a scalar. Relative to the declared `memchr` premise.
* `mk_string_split_line`: loses `call to uncontracted function
  mk_string_char_search` (W4 now derives a validated summary for its `Safe`
  same-unit callee); stays Unknown on 9 other leaves.
* `rd_dl_open`, `rd_string_render`, `_mk_string_search`: their `_Generic`
  leaves go with no replacement (the lowered `strrchr`, `strstr`, `strchr`,
  `memchr` calls are admitted); `rd_dl_open` gains `unknown_alias td` at line 131,
  whose condition tests the second lowered `strrchr` result, taken from `fname`
  (the first result on one path, the formal `path` on the other). All stay
  Unknown on other leaves.
* The 6 `bsearch` statements (`ut_populate_internal_topic_metadata`,
  `rd_kafka_Node_new_from_brokers` ×2, `rd_kafka_populate_metadata_topic_racks`,
  `rd_kafka_topic_assignment_state_rack_search`,
  `rd_kafka_partition_internal_find_rack`): each `_Generic` leaf is exchanged
  for `call to uncontracted function bsearch` at the same line.
* Conditions now desugared by W44-BRANCH-CONDITION-CALLS, because their calls
  are contracted `strchr`/`strstr`/`strrchr` instead of `_Generic`: 22 branch
  and loop condition refusals go in `rd_addrinfo_prepare`, `rd_dl_error`,
  `rd_kafka_assignors_init`, `rd_kafka_broker_name_parse` (3),
  `rd_kafka_cert_new`, `rd_kafka_pattern_list_parse`,
  `rd_kafka_plugins_conf_set0`, `rd_kafka_sasl_client_new`,
  `rd_kafka_transport_ssl_handshake` (2),
  `rd_kafka_transport_ssl_set_endpoint_id` and
  `ut_sasl_oauthbearer_oidc_assertion_create` (9). Where the condition is
  `(t = strchr (s, c))` over a name `s` the projection cannot resolve (an
  `rd_strdupa` buffer, an unmodeled result), the desugared null test and the
  following reads report `unknown_alias` on `s` or `t`: 13 leaves in
  `rd_dl_error`, `rd_dl_open`, `rd_kafka_assignors_init`,
  `rd_kafka_broker_name_parse`, `rd_kafka_pattern_list_parse` and
  `rd_kafka_plugins_conf_set0`, the exchange constraint 5h names. All stay
  Unknown.
* `ut_sasl_oauthbearer_oidc_assertion_create`: stays provisional `Unsafe`; see
  §5.2.

### 5.2 Added leaks, classified by reading

`ut_sasl_oauthbearer_oidc_assertion_create` is a unit test. It allocates
`header_part = rd_strndup (jwt, …)` (line 1774) and `payload_part =
rd_strndup (…)` (1775), then runs twelve `RD_UT_ASSERT (cond, …)` (lines 1777–1798), each of
which returns 1 without freeing either when `cond` fails, before freeing both
at 1801–1802. BEFORE reported the two objects live at the failure returns of
lines 1777–1783 (9 leaves), which W44-LEAK-PIN-AUDIT classed (a),
test-failure path ([class-a.md](../w44-leak-pin-audit/class-a.md)). The
assertions at 1783–1798 test `!strchr (header_part, …)` and the like; each
condition was refused while its calls were `_Generic`, and the refused hand-on
rule withdrew the accusation because the refused statement names the objects.
Lowered, the conditions are desugared, the statements are no longer refused,
and each failure return reports both objects: **15 added leaves, all (a)**,
validated by reading (the object is allocated, never freed or handed on before
the `return 1`, and `strchr`/`strlen` only read it), reachable only when the
unit test fails. 0 artifacts (b), 0 premise gaps (c). The function was already
provisional `Unsafe`; no function or unit changes status because of them.
Provisional candidates, not upstream defect claims.

## 6. Prediction against outcome

| # | Predicted | Measured | Held |
|---|---|---|---|
| P1 | 14 `_Generic` leaves removed | 14 removed, family empty | yes |
| P2 | the 6 `bsearch` exchanged 1:1 | 6 exchanged at the same lines | yes |
| P3 | the other 8 removed with no replacement at the statement | 8 removed; `rd_dl_open` gains an `unknown_alias` on the next line, not at the statement | yes |
| P4 | `mk_string_char_search` → Safe; no other table function moves; the oidc test stays Unsafe | as predicted | yes |
| P5 | `mk_string_split_line` loses its `mk_string_char_search` leaf, stays Unknown | as predicted | yes |
| P6 | 0–30 condition leaves removed, ≤ 2 further moves, none to Unsafe | 22 removed; 0 further moves | yes |
| P7 | leak stays 19; violations 0 | **leak 34 (+15, all class (a))**; violations 0 | **no** |
| P8 | `unknown_alias` within ±15 | +13 | yes |
| P9 | no unit moves; other artifacts unchanged | as predicted | yes |
| P10 | every delta in a selection function or P5's caller | as predicted | yes |

P7 failed because I did not follow the desugared conditions through the refused
hand-on rule: the leak audit had withdrawn exactly these accusations because
the refused condition statements named the objects, and admitting the
statements restores them. The direction is the accusing one, on validated
leaks, in a function that was already provisional `Unsafe`.

## 7. Validation

Run with [`run_gate.py`](scripts/run_gate.py) (copied from W44-LIBC-RESIDUALS:
`CARGO_INCREMENTAL=0`, each exit captured from the process, never through a
pipe) into the worktree-private `measure/gate/`; summary in
[gate.json](gate.json), Clang recorded in this checkout
([clang-version.txt](clang-version.txt), exit 0). Code measured: `3e5a93a0`.
`cargo clean --profile dev` ran after the test and probe steps.

| Step | Exit | Result |
|---|---:|---|
| `git submodule update --init --recursive` | 0 | |
| `cargo fmt --all -- --check` | 0 | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | clean |
| `cargo test --workspace --no-fail-fast` | 0 | **5,525 passed / 0 failed / 11 ignored**, 293 summaries (main 5,505; +11 fixtures, +9 unit tests) |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| `python3 scripts/check_lean_audits.py` | 0 | **186** audits (185 on main + `AuditGenericSelection`) |
| `python3 -m unittest discover -s scripts/tests` | 0 | **330** tests, after the final matrix regeneration |
| `python3 scripts/build_docs_index.py --check` | 0 | |
| provider `cargo test --workspace --locked --offline --no-fail-fast` | 0 | **124 passed / 0 failed**, 39 summaries |
| provider isolated package mutation control | 0 | 1 passed |
| provider fmt / clippy | 0 / 0 | clean |
| `python3 providers/contracts.py check` | 0 | |
| `providers/package.py build`, `verify` ×5 | 0 | five packages |
| provider Python | 0 | 13 tests |
| `docs/reports/remaining-d02/build_matrix.py . --gate gate.json` | 0 | 378 follow-ups, 111 contract modules, 186 Lean audits |

The load-sensitive `vp04_vp07_vp08_process_lifecycle_and_payload_budgets` did
not flake. The `indirect-census` feature build was not run: the card touches no
census code. The three mutation controls of §4 ran before this gate.

## 8. Follow-ups

Registered ([followups.json](followups.json), via
[`update_ledger.py`](scripts/update_ledger.py); 377 → 378):

* **`W44-BSEARCH-UNDECLARED`** — 13 of the 85 selections are `bsearch`; the 6
  that were `_Generic` leaves are now `call to uncontracted function bsearch`
  in 5 functions, none blocked by it alone. A declaration needs a stated
  premise for the comparator callback.

Retired: none; no registered follow-up named the selection (the script checks).
Route 2 is not registered: it would decide no occurrence route 1 refuses.

The corpus row `corpus.cycle19_raw_units` is rewritten from the AFTER
artifacts; the previous row is kept in
[previous-corpus-row.json](previous-corpus-row.json).

## 9. Files

* Rust: `crates/qcore-c-gateway/src/legacy_generic_selection.rs` (new); hooks
  in `crates/qcore-c-gateway/src/legacy_c.rs` (module declaration and one call
  in `build_function_cfg_with`); `declares_object_pointer_result` in
  `crates/qcore-c-gateway/src/legacy_declared_abi.rs`;
  `crates/qcore-c-gateway/tests/generic_selection.rs` (new).
* Lean: `proof/QCore/GenericSelection.lean`, `proof/AuditGenericSelection.lean`
  (new); `proof/QCore.lean` (import).
* Docs: `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` (G10), `docs/STATUS.md`,
  `docs/development/increments/wave-44.md`,
  `docs/reviews/r0_3/acceptance_matrix.json`, `docs/reports/INDEX.md`.

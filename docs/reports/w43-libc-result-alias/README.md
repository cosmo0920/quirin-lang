# W43-LIBC-RESULT-ALIAS: derived results, the `__restrict` reader, and thirteen libc declarations

Card W43-LIBC-RESULT-ALIAS, 2026-09-24, on base `a7a936d9` (main after the
W43-LIBC-CONTRACTS integration). `memcpy` returns its first argument, and
`strchr` returns a pointer into its first argument or NULL. The resource
vocabulary could not say either, so card W43-LIBC-CONTRACTS left both families
undeclared, and the string-scanning code built on them stayed Unknown. This card
adds a **derived-result** mode to the vocabulary and implements it in the C
gateway, where a derived result is a non-owning pointer. It models the mode in
Lean and repairs the declared-ABI reader for a qualifier after a parameter's
last `*`. It then declares thirteen libc functions with the mode, only in
`providers/quirin-provider-libc`.

It discharges `LIBC-MEM-RESULT-ALIAS`, `W43-LIBC-RESULT-ALIAS-NAMES` and
`W43-LIBC-POINTER-QUALIFIER-READER`, and it registers six residuals.

## Headline

* **Vocabulary.** Two new predicates, `resource_return_alias(p, n)` (exact:
  the result *is* `p`) and `resource_return_interior(p, n)` (a pointer into
  `p`'s object, or NULL). Both load to `ResourceReturnMode::Derived`, which
  also records `const_dropped`. They are an addition to resource vocabulary
  version 1, like the variadic tail and the literal premise before them.
  Contract spec [§C04.3.1](../../QCORE_CONTRACT_SPEC_r0_3.md) states the
  semantics.
* **Semantics** (gateway `legacy_c.rs`, Lean `QCore.DerivedResult`):
  * A derived result owns nothing. A discarded one is exactly a discarded
    `none`.
  * Releasing it is never a release of the base:
    * an **interior** pointer handed to a release is an `invalid_release`
      violation candidate;
    * every other custody position refuses. That includes an **exact** alias,
      whose release C *does* treat as the base's; see
      [§1.2](#12-one-deliberate-difference-from-the-brief).
  * Releasing the base invalidates every derived pointer. A later use, release
    or return is a `use_after_release` candidate.
  * A derived null test narrows only the derived pointer.
  * Fail-closed and the verdict contract are unchanged.
* **Reader.** A qualifier after a parameter declarator's last `*` is read as
  the unqualified type (C11 6.7.6.3p15). A qualifier between two `*` still
  refuses, and result declarators are not widened. The Lean model is
  `QCore.PointerQualifierReader`.
* **Declared: 13**, each a trusted assumption, and every one agrees with every
  captured unit that declares it (2,211 census rows, 0 withdrawn):
  * **exact aliases:** `memcpy`, `memmove`, `memset`, `strcpy`, `strncpy`,
    `gmtime_r`, `localtime_r`;
  * **interior pointers:** `strchr`, `strrchr`, `strstr`, `strcasestr`,
    `memchr`, `memrchr`.

  **Refused by name: 2**, `realpath` (a conditional alias) and
  `pthread_getspecific` (retained storage).
* **Arbiter, in three separately attributed classes.** BEFORE reproduces the
  W43-LIBC-CONTRACTS diagnostics byte for byte.
  * **(a) The reader change alone** moves **nothing**. All five artifacts are
    byte-identical to BEFORE, and none of the 8,292 census rows of the 114
    selected declarations changes its outcome.
  * **(b) Declarations.** The seven exact aliases remove 169 leaves and add 20,
    and **11 functions move from Unknown to Safe**. The six interior pointers
    remove 14 leaves and add 1, and **1 function moves from Unknown to Safe**
    (`mk_string_char_search_r`). Three `if (strstr(…))`-shaped branch
    conditions are now modelled.
  * **(c) Violation candidates: none.** 0 `invalid_release` and
    0 `use_after_release` leaves. The accusing arms are unit-tested but reach no
    corpus function (`W43-RESULT-ALIAS-ACCUSING-UNEXERCISED-IN-CORPUS`).
* **Totals.** Functions go 245 / 84 / 4,505 → **257 / 84 / 4,493**
  (Safe / provisional Unsafe / Unknown). Units do not move. The leak count stays
  **272**; `unknown_alias` goes 4,655 → 4,657 and `unsupported` 49,094 →
  48,930. Both attributions exit 0 with `attribution_complete: true`.
* **The printf family is now available.** The census reads `printf`,
  `fprintf`, `snprintf` and `sprintf` as agreeing in all 168 units that declare
  them, where the base reader withdrew them at every call site. None is
  declared here; that is `LIBC-PRINTF-FAMILY`'s leaf-directed card.

| Measure | BEFORE (W43-LIBC-CONTRACTS) | AFTER | Delta |
|---|---:|---:|---:|
| Functions Safe / provisional Unsafe / Unknown | 245 / 84 / 4,505 | **257 / 84 / 4,493** | +12 / 0 / −12 |
| Units Safe / provisional Unsafe / Unknown / data-only | 2 / 42 / 137 / 4 | 2 / 42 / 137 / 4 | 0 |
| `unsupported` | 49,094 | **48,930** | −164 |
| `unknown_alias` | 4,655 | **4,657** | +2 |
| `leak` / `summary_conflict` / `double_release` | 272 / 1 / 0 | 272 / 1 / 0 | 0 |
| `invalid_release` / `use_after_release` | — | 0 / 0 | 0 |
| `call to uncontracted function` leaves / functions / sole | 11,801 / 2,728 / 397 | 11,652 / 2,688 / 384 | −149 / −40 / −13 |
| Unknown functions blocked only by one uncontracted callee | 306 (233 callees) | 295 (230 callees) | −11 |
| return-expression leaves / functions / sole | 497 / 420 / 144 | 499 / 422 / 146 | +2 / +2 / +2 |
| branch-condition leaves / functions / sole | 8,715 / 2,409 / 45 | 8,712 / 2,408 / 47 | −3 / −1 / +2 |
| Contract corpus modules / manifest-selected / resource-v1 declarations | 107 / 44 / 114 | 107 / 44 / **127** | 0 / 0 / +13 |

The census rows count Unknown functions, as
[blocker-census.txt](blocker-census.txt) and [sole-callees.txt](sole-callees.txt)
do. The predecessor rows are W43-LIBC-CONTRACTS' own census files.

## 1. The derived-result mode

### 1.1 Vocabulary, placement and version

```qcore
extern fn memcpy(dest: ptr<mut, u8>, src: ptr<const, u8>, n: usize) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(dest);
    ensures normal resource_borrowed(src);
    ensures normal resource_borrowed(n);
    ensures normal resource_return_alias(dest, nonnull);
};
```

`ResourceReturnMode::Derived { parameter, extent, nullable, const_dropped }`
([`qcore-schemas/src/contract.rs`](../../../crates/qcore-schemas/src/contract.rs)).
The loader ([`contract_loader.rs`](../../../crates/qcore-contract/src/contract_loader.rs))
admits the mode only under all of these conditions:

* the base is a `resource_borrowed` parameter;
* the base is typed `ptr<mut, u8>` or `ptr<const, u8>`;
* the result has one of those two types.

It rejects the whole module otherwise: a custody-bearing base, a scalar,
`ptr<mut, ptr<mut, u8>>`, a scalar result, an unknown name, a second result mode
and a malformed nullability each reject it (`derived_result.rs`,
`every_other_placement_is_rejected`). `const_dropped` is set exactly when the
base is `const` and the result mutable.

**Version.** This is an addition to vocabulary version 1, not a new version:

* The marker is unchanged.
* Every module that loaded before loads to the same functions. The pinned
  corpus tables and the old-table parity test pass unchanged for all 114
  earlier declarations.
* A loader that predates the predicates **rejects** a module that uses them. The
  base census tool, run on the new declarations, stops with
  ``unsupported predicate `resource_return_alias(dest, nonnull)` ``
  ([abi-evidence.json](abi-evidence.json), `new13` rows).

No new evidence version is minted. Every published verdict already carries the
exact module texts in `source_assumptions`, and the audit's `contracts`
artifact holds each selected source. The evidence identity therefore names the
declarations a verdict rests on, and with them the vocabulary.

### 1.2 One deliberate difference from the brief

The brief asked that releasing any derived result be a violation candidate. That
is true of an interior pointer: freeing it is an invalid free unless its offset
happens to be zero (C11 7.22.3.3p2). It is **false** of an exact alias, because
`free(memcpy(b, s, n))` frees `b` validly. Accusing it would publish a
provisional `Unsafe` for correct C.

The rule is therefore split by extent:

* an **interior** result handed to a release is an `invalid_release`
  candidate;
* an **exact** alias handed to any custody position is refused with
  `unknown_alias`, and the base carries an unresolved effect. It is neither
  credited nor accused.

Both keep the brief's other requirement: the base's obligation is never
discharged through the derived pointer. Crediting an exact release would be
faithful, but it is the discharging direction and is registered for its own
measured card (`W43-RESULT-ALIAS-EXACT-RELEASE`).

### 1.3 What the gateway does

A derived binding is an `AbstractPointer` whose new `derivation` field is
`Derived { extent, const_dropped }`. It **shares its base's targets** and
creates no site, so releasing the base invalidates it through the ordinary site
status.

| Rule | Where | Test (`crates/qcore-c-gateway/tests/derived_result.rs`) |
|---|---|---|
| A discarded result binds nothing, and a condition-position call is admitted | classifier: `discarded owned result` gate, `produced_count` | `a_discarded_exact_alias_…`, `a_derived_result_tested_in_a_condition_…` |
| The bound result shares the base's targets; an untracked base gives an unknown derived binding | `derive_pointer` | `scanning_a_borrowed_string_certifies` |
| A base must be a place, a place ± one token, a string literal or `&name`; a *bound* result over any other base is refused | `derived_base` | `a_bound_result_over_an_unmodeled_base_expression_is_refused`, `an_offset_base_is_the_same_object` |
| No custody effect goes through a derived pointer: an interior release is left for the settled pass, and everything else refuses | `derived_custody_refusal`, `apply_resource_action`, `require_live_resource` | `releasing_an_interior_pointer_…`, `releasing_an_exact_alias_is_refused_…`, `consuming_or_returning_…` |
| `invalid_release` / `use_after_release`, read from the **settled** states (constraint 6) | `report_derived_pointer_violations` | `…used_after_its_base_is_released…`, `releasing_an_interior_pointer_after_its_base…` |
| A null test narrows only the derived binding; a mixed binding refuses; a base's non-null edge does not reach derived aliases | `refine_pointer_null` | `a_null_derived_pointer_does_not_make_its_base_disappear`, `a_binding_derived_on_only_some_paths_is_refused` |
| Derived nullness never marks the base's site nullable at a join | `AbstractState::join` | (the same leak witness) |
| Never an aggregate owner, store base or member link | `resolved_aggregate_owner`, `scalar_member_store_base_resolves`, `bound_read_resolves`, the `stored_field` refusal | `a_derived_pointer_stored_into_a_member_is_refused` |
| Never returned: `escapes_only_fresh_custody` is false, and `EscapeIfOwned` refuses | `legacy_return_call.rs` | `consuming_or_returning_…` |
| Never delivered through an out slot, whether by an alias store or as a call result; an exit reads a derived slot or return as ambiguous | `transfer`, `apply_contract_call`, `observe_out_slot`, `observe_exit` | `a_derived_pointer_is_never_delivered_through_an_out_slot` |
| A body defined in the unit is never checked against a declared derived result | `EntryCustody::build`, `result_matches` | (falls back to the contract-free analysis) |
| The extern-call model refuses it rather than projecting it onto `None` | `extern_bridge::return_mode` | `a_derived_result_is_refused_not_projected_onto_none` |
| The custody duty is `Absent` | `legacy_declared_modes.rs` | (mode gate unchanged) |

Two diagnostic kinds are new: `LegacyDiagnosticKind::InvalidRelease` and
`UseAfterRelease`, published as `invalid_release` and `use_after_release`. Both
are safety issues in `classify_diagnostics` and path-poisoning, like
`double_release`. **Neither is a validated upstream defect.** A candidate
becomes `Unsafe` only under the existing provisional-`Unsafe` policy. The
analyzer does not know an interior pointer's offset. `use_after_release`
follows `double_release`'s reading of the `RELEASED` bit: the base was released
on *some* path reaching the use.

The `const_dropped` flag is carried on the binding through copies, joins and
re-derivation. A direct write through a const-dropped pointer is already
refused by the indirect-store gate. A **borrowed-position** write (for example,
`memcpy(p, …)` with `p` from `strchr`) is not distinguished from a read,
because v1 has no read/write fact and the transfer function does not see
declared types. That is registered, not implemented
(`W43-RESULT-ALIAS-CONST-WRITE`).

### 1.4 Lean parity

[`QCore.DerivedResult`](../../../proof/QCore/DerivedResult.lean) models the
binding and the rules, audited by
[`AuditDerivedResult.lean`](../../../proof/AuditDerivedResult.lean) with 31
theorems. It covers these directions:

* **non-owning:** `deriving_changes_no_status`,
  `deriving_binds_only_its_destination`, `a_derived_pointer_targets_its_base`;
* **releasing a derived pointer is not a release of its base:**
  `releasing_a_derived_pointer_never_releases_its_base` (every status),
  `releasing_an_interior_pointer_is_a_candidate`,
  `releasing_a_live_exact_alias_is_refused`,
  `releasing_a_mixed_pointer_is_refused`;
* **invalidation:** `a_use_after_the_base_is_released_is_a_candidate`,
  `a_release_after_the_base_is_released_is_a_candidate`, and their composition
  with the base's own release,
  `releasing_the_base_invalidates_every_derived_pointer`;
* **out slots:** `a_derived_pointer_is_never_delivered`, beside
  `an_identity_is_delivered`;
* **null tests:** `a_derived_null_test_keeps_every_status` and
  `…_every_other_binding`, beside `an_identity_null_test_removes_its_site`,
  the rule the derived binding must not inherit;
* **no `Safe` from any of it:** `a_candidate_is_never_safe`,
  `a_refusal_is_never_safe`;
* **conservativity:** `identity_bindings_are_unaffected`. On identity bindings
  every instruction except `derive` is the pre-card transfer;
* **the join and the flag:** `join_of_equal_is_itself`,
  `identity_and_derived_join_to_mixed`, `a_null_path_keeps_the_other_relation`,
  `mixed_absorbs`, `const_dropped_survives_derivation`,
  `an_interior_step_is_interior`;
* **five witness programs**, including
  `the_null_derived_pointer_leaves_its_base_live` beside
  `an_identity_null_test_would_have_hidden_it`, the non-vacuity of the
  narrowing.

[`QCore.PointerQualifierReader`](../../../proof/QCore/PointerQualifierReader.lean)
([audit](../../../proof/AuditPointerQualifierReader.lean), 12 theorems) models
the reader step:

* conservativity: `the_widening_only_adds`, `the_widening_loses_no_reading`;
* the rule as an equation: `the_widening_reads_the_unqualified_type`;
* the rule's limits: `a_qualifier_between_two_stars_still_refuses`,
  `a_declarator_without_a_star_is_unchanged`, `drop_is_idempotent`;
* the measured glibc witness, which was unreadable, is now `ptr<const, u8>`,
  and keeps its pointee constness.

Both modules are placeholder-free and use no project axioms. As with every
gateway module, the Rust–Lean correspondence is maintained by review, not
proved (trust boundary 28).

## 2. The reader change, alone: class (a)

`legacy_declared_abi.rs` gains `drop_parameter_object_qualifiers`, applied in
`project_parameter` after the parameter name is dropped. Eight new table rows
in `crates/qcore-c-gateway/tests/loader_abi.rs` cover the rule:

* `__restrict`, `const` and several trailing qualifiers each agree;
* the pointee's constness is kept;
* a qualifier between two `*` refuses, both on its own and with a trailing one.

`a_result_ending_in_a_qualifier_is_not_widened` pins the result boundary.

**Measured effect: nil.** The card's binary with no new declaration (the
`reader` run) reproduces **all five** BEFORE artifacts byte for byte
([classes.json](classes.json), `a_reader_change_alone`). The same census tool
was built twice: from the base tree and from this card's tree. It was run over
the 185 units captured in this worktree ([§6](#6-arbiter)). For the 114
declarations selected at the base, all **8,292** (unit, name) rows keep their
outcome: 7,763 agreed, 352 undeclared, 113 return-type withdrawals (the GNU
`strerror_r`) and 64 unreadable. The last four prototypes that
W43-LIBC-CONTRACTS dropped for `__restrict` were never shipped, so nothing
shipped depended on the gap.

## 3. Declarations and the ABI gate

Every declaration sits at the end of its module and is appended, not
reordered: `libc/string.qcore` gains eleven and `libc/time.qcore` two. Each
module's header comment gains a pointer to the new section. Each prototype was
read from the unit's own glibc header, and each is a **trusted assumption**
listed in its module comment.

| Function | Result mode | Units that declare it, all agreeing | Leaves / functions at the baseline |
|---|---|---:|---|
| `memcpy` | `alias(dest, nonnull)` | 181 | 70 / 54 |
| `memset` | `alias(s, nonnull)` | 181 | 61 / 44 |
| `memmove` | `alias(dest, nonnull)` | 181 | 3 / 3 |
| `strcpy` | `alias(dest, nonnull)` | 181 | 6 / 4 |
| `strncpy` | `alias(dest, nonnull)` | 181 | 2 / 2 |
| `gmtime_r` | `alias(tp, nullable)` | 144 | 2 / 2 |
| `localtime_r` | `alias(tp, nullable)` | 144 | 1 / 1 |
| `strchr` | `interior(s, nullable)`, `const_dropped` | 181 | 4 / 3 |
| `strrchr` | `interior(s, nullable)`, `const_dropped` | 181 | 1 / 1 |
| `strstr` | `interior(haystack, nullable)`, `const_dropped` | 181 | 2 / 1 |
| `strcasestr` | `interior(haystack, nullable)`, `const_dropped` | 181 | 2 / 2 |
| `memchr` | `interior(s, nullable)`, `const_dropped` | 181 | 1 / 1 |
| `memrchr` | `interior(s, nullable)`, `const_dropped` | 113 | 1 / 1 |

The census was run with this card's tool over the new declarations
([abi-evidence.json](abi-evidence.json), `new13`): **2,211 rows, all
`agreed`, 0 withdrawn**. `memcpy`, `strcpy`, `strncpy`, `gmtime_r` and
`localtime_r` spell `__restrict`, so without §2 all five would have been
withdrawn everywhere. The arbiter confirms the census: AFTER contains no
`whose declared contract was withdrawn` leaf for any of the thirteen names.

**Refused, by name** (`W43-RESULT-ALIAS-CONDITIONAL-AND-RETAINED`):

* `realpath(path, resolved)` returns `resolved`, or a fresh allocation when
  `resolved` is NULL. That is a value-dependent result, which C04.4 excludes.
* `pthread_getspecific(key)` returns storage an earlier `pthread_setspecific`
  retained, which aliases no argument of the call.

## 4. Placement

No wrapper and no `contract inherit` is used. The libc placement test
(`scripts/tests/test_contract_corpus.py`) passes unchanged: every new name is a
listed libc/POSIX name, and every declaration is inside
`providers/quirin-provider-libc`. The crate's `CONTRACTS` index,
`contract/corpus.json` and `cycle19-resource-contracts.json` need no edit,
because both modules were already indexed and selected.

## 5. Pin changes, each deliberate

* `compiler/tests/qcore_contract_corpus.rs`: thirteen clause pins, in module
  order after `strerror_r` and `timegm`. Each has 2 clauses plus 1 per
  parameter.
* `compiler/tests/contract_loader.rs`:
  * (selected, v1 declarations, rejected) (44, 114, 207) → **(44, 127, 207)**;
  * the old-table parity list gains the thirteen derived entries.
* `compiler/src/lib.rs` and the gateway re-export `DerivedExtent`.
* Exhaustive renderers of `LegacyDiagnosticKind` gain the two new kinds:
  `compiler/tests/ct_corpus.rs`, the gateway tests `publication_custody.rs`,
  `borrow_liveness_boundary.rs` and `borrow_residuals_boundary.rs`, the
  `qcore-audit` report mapping and the `alias_aggregate_probe` and
  `c03_reach_census` examples. One gateway unit test's `AbstractPointer`
  literal gains `derivation: Identity`. No expectation changes.

No assertion is weakened.

## 6. Arbiter

The runs use [`../w43-return-expression/scripts/run_audit.py`](../w43-return-expression/scripts/run_audit.py),
with the same command shape, Clang and ChunkIO package as the baseline. BEFORE
used a release binary of the base tree. The three staged runs used this card's
binary with the manifest unchanged and the declarations added stage by stage.
AFTER is a confirmation run on the final implementation commit, and it is the
archived baseline.

| Run | Binary (sha256) | Declarations | Exit | Window (UTC) | Record |
|---|---|---|---:|---|---|
| BEFORE | `c5fc4d5b…` (base `a7a936d9`) | base | 0 | 12:36:18–12:40:56 | [before-run.json](before-run.json) |
| READER | `754aa51e…` (`0e9b5a76`) | base | 0 | 12:54:41–12:58:53 | [reader-run.json](reader-run.json) |
| EXACT | `754aa51e…` | + 7 exact aliases | 0 | 13:05:50–13:10:27 | [exact-run.json](exact-run.json) |
| FULL | `754aa51e…` | + 6 interior pointers | 0 | 13:10:42–13:15:05 | [full-run.json](full-run.json) |
| AFTER | `2cbe761a…` (`912ed8b7`) | all 13 | 0 | 13:33:24–13:38:18 | [after-run.json](after-run.json), [artifacts/run.json](artifacts/run.json) |

**BEFORE vs the W43-LIBC-CONTRACTS baseline**
([before-identity.json](before-identity.json)):

* diagnostics, chunk-contracts and source are **byte-identical**;
* `contracts` differs only by the checkout root, and is identical once it is
  aliased;
* `source-effects` carries the checkout-path digests the brief lists.

**AFTER vs FULL:** all five artifacts are byte-identical. Between the two
binaries the card added tests, documents, a re-export, and one fail-closed
refusal: a derived pointer delivered through an out slot. A first confirmation
run on `89860d8b` (binary `fdbd0d8e…`, 13:18:50–13:23:21) was also
byte-identical to FULL. It was superseded by this one after the out-slot
refusal landed, and is kept privately.

**AFTER vs BEFORE:** chunk-contracts, source-effects and source are
byte-identical. `contracts` differs only in the source text of
`libc/string.qcore` and `libc/time.qcore`.

A private capture run used a transparent Clang wrapper
([scripts/clang-tee.sh](scripts/clang-tee.sh)) with the READER configuration.
Its diagnostics are byte-identical to READER. It supplied:

* the 185 line-marker-stripped units for the census;
* the `-E -P` texts for attribution (180 paired, and the same 5 unpaired units
  as before, which project no function).

### 6.1 Attribution, in three classes

[`scripts/classes.py`](scripts/classes.py) splits the movement. It uses the
W43-LIBC-CONTRACTS attribution script
([`attribute.py`](../w43-libc-contracts/scripts/attribute.py)) once per stage,
each time with only that stage's own declarations. Full rows are in
[attribution-exact.json.gz](attribution-exact.json.gz) and
[attribution-interior.json.gz](attribution-interior.json.gz). Both stages exit
0 with `attribution_complete: true`; no unit status moves, and no unit field
moves outside its own leaf delta.

| Class | Removed | Added | Function moves | Families |
|---|---:|---:|---|---|
| (a) reader change alone | 0 | 0 | none | all five artifacts identical |
| (b) exact alias | 169 | 20 | 11 Unknown → Safe | `unsupported` −151, `unknown_alias` +2 |
| (b) interior pointer | 14 | 1 | 1 Unknown → Safe | `unsupported` −13 |
| (c) violation candidates | — | **0** | none | `invalid_release` 0, `use_after_release` 0 |

**Exact-alias stage.**

Removed:

* 145 declared-callee leaves: `memcpy` 70, `memset` 61, `strcpy` 6,
  `memmove` 3, `strncpy` 2, `gmtime_r` 2 and `localtime_r` 1;
* 8 at the newly validated summaries of Safe `snappy.c` helpers;
* 15 same-line `indirect memory access` refusals, whose line's call is now
  modelled;
* 1 same-function leaf.

Added:

* 9 `call has an unmodeled formal address or borrowed expression`, where the
  modelled `memcpy`/`memset` now meets its pre-existing borrowed-expression
  gate (`flb_input_chunk.c`, `rdrand.c`, `rdstring.c`);
* 8 `nested call`, where the call's own argument is an uncontracted call, as
  in `memcpy(d, s, strlen(s))`;
* 2 `unknown_alias` on `server`, where `memset(&server->…)` now meets the
  member-address rule;
* 1 return-expression refusal, `XXH_memcpy`'s `return memcpy(…)`.

**Interior-pointer stage.**

Removed:

* 11 declared-callee leaves;
* **3 branch-condition refusals** that the condition route now models. This is
  the string-scanning shape the card exists for:
  * `if (strstr(str, "e+"))` and `if (!strchr(str, '.'))` in
    `cmt_encode_splunk_hec.c`;
  * `if (NULL != memchr(…))` in `cmt_mpack_utils.c`.

Added:

* 1 return-expression refusal, `mk_string_casestr`'s
  `return strcasestr(…)` (`W43-RESULT-ALIAS-RETURNED`).

**The 12 functions that move Unknown → Safe**, each Safe **relative to the
declared premises** (invariant 2's reading, not a proof of the C):

| Function | Unit | Its BEFORE blocker |
|---|---|---|
| `XXH_read32`, `XXH_read64` | `rdxxhash.c` | `memcpy` |
| `get_unaligned32`, `get_unaligned64`, `put_unaligned16`, `put_unaligned32`, `put_unaligned64` | `snappy.c` | `memcpy` |
| `get_eight_bytes_at` | `snappy.c` | `get_unaligned64`, now Safe and summarised |
| `ctr_opts_init` | `ctraces.c` | `memset` |
| `LZ4_resetStream` | `lz4.c` | `memset` |
| `consume_bytes` | `mk_server/mk_fifo.c` | `memmove(buf, buf + bytes, …)` |
| `mk_string_char_search_r` | `mk_core/mk_string.c` | `memrchr` (`p = memrchr(string, c, len); if (p) return p - string;`) |

The last row is the brief's case in miniature. A string-scanning helper that
binds an interior pointer, null-tests it and computes an offset from it is now
placed in `Safe`, and it would be `Unsafe` if it freed the pointer or used it
after freeing its base.

`LIBC-MEM-RESULT-ALIAS` forecast eight functions blocked by `memcpy` alone.
Seven moved. The eighth, `XXH_memcpy`, returns the derived result, which is
refused.

**Leak pin (constraint 7).** The count stays at **272**, with no leak leaf added
or withdrawn. This is the fourth declarative arbiter in a row to leave the pin
alone. Every new declaration borrows every argument, and a derived result never
discharges its base. The new baseline is [artifacts/](artifacts/)
([manifest.json](artifacts/manifest.json)).
`docs/reports/w43-libc-contracts/artifacts/` is its frozen predecessor and is
not edited.

## 7. What the reader change does for the printf family

The fourteen prototypes W43-LIBC-CONTRACTS withdrew for `__restrict` were
re-measured with both census tools
([measurement/restrict14.qcore](measurement/restrict14.qv), a
measurement-only module; [abi-evidence.json](abi-evidence.json), `restrict14`).
The base tool withdrew every row. This card's tool reads them as follows:

| Prototype | Units agreeing (after) | Still withdrawn |
|---|---:|---|
| `printf`, `fprintf`, `snprintf`, `sprintf` | 168 each | 0 |
| `stat`, `strftime`, `gettimeofday`, `getsockopt` | 109, 144, 107, 105 | 0 |
| `dlsym`, `cnd_timedwait`, `pthread_sigmask`, `regerror` | 4, 83, 5, 1 | 0 |
| `getpeername` | 12 | 93: `__SOCKADDR_ARG` is a transparent union under `_GNU_SOURCE` |
| `regexec` | 0 | 1: `regmatch_t __pmatch[__restrict_arr]` puts the qualifier in an array bound |

`LIBC-PRINTF-FAMILY` is therefore no longer blocked on the reader. Its own
condition, a dedicated leaf-directed card, is unchanged, and none of the four is
declared here. The ten non-printf names are registered as
`W43-LIBC-RESTRICT-PROTOTYPES-UNDECLARED`.

## 8. Validation

Measured in this worktree (`agent-a4d52a22505f866ac`) by
[`scripts/run_gate.py`](scripts/run_gate.py). Its one change from
W43-LIBC-CONTRACTS' runner is `cargo clippy --workspace`, as the brief states.
It sets `CARGO_BUILD_JOBS=6` and `RUST_TEST_THREADS=2`, turns debug info off,
and captures every exit to a file, never through a pipe. The core and provider
gates ran on the implementation commit `912ed8b7`. Later commits change
documents, the ledger, report scripts and evidence only. Logs are
worktree-private under `measure/`. The generator input is
[gate.json](gate.json), with the [Clang recording](clang-version.txt) taken in
this checkout beside it.

| Command | Exit | Result | Log |
|---|---:|---|---|
| `git submodule update --init --recursive` | 0 | libevent at `d82464a2…` | `measure/gate/submodules.log` |
| `clang --version` | 0 | Ubuntu clang 21.1.8 | `measure/gate/clang.log` |
| `cargo test --workspace --no-fail-fast` | 0 | **5,160 passed / 0 failed / 11 ignored**, 273 summaries | `measure/gate/test.log` |
| `cargo fmt --all -- --check` | 0 | clean | `measure/gate/fmt.log` |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | clean | `measure/gate/clippy.log` |
| `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | | `measure/gate/probe.log` |
| `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | | `measure/gate/followup.log` |
| `python3 scripts/check_lean_audits.py --jobs 4` | 0 | **169** audits | `measure/gate/lean.log` |
| `python3 docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/w43-libc-result-alias/gate.json` | 0 | 294 follow-ups, 107 contract modules, 169 Lean audits | `measure/build-matrix.log` |
| `python3 scripts/build_docs_index.py`, then `--check` | 0 / 0 | index regenerated and matching | `measure/docs-index-check.log` |
| `python3 -m unittest discover -s scripts/tests` (after the final regeneration) | 0 | **319** tests | `measure/python-final.log` |
| `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline --no-fail-fast` | 0 | **109 passed / 0 failed / 0 ignored**, 39 summaries | `measure/provider-gate/ptest.log` |
| `… -p qcore-provider-conformance --test packages real_packages_preserve_unknown_and_reject_mutations … -- --exact --test-threads=1` | 0 | 1 passed (isolated, constraint 16) | `measure/provider-gate/pisolated.log` |
| `cargo fmt --manifest-path providers/Cargo.toml --all -- --check` | 0 | clean | `measure/provider-gate/pfmt.log` |
| `cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings` | 0 | clean | `measure/provider-gate/pclippy.log` |
| `python3 providers/contracts.py check` | 0 | 72 provider-owned, inventories unchanged | `measure/provider-gate/pcontracts.log` |
| `python3 providers/package.py build --output measure/provider-gate/packages` | 0 | 5 packages | `measure/provider-gate/pbuild.log` |
| `python3 providers/package.py verify …/<id>` (×5: chunkio, cmetrics, ctraces, libevent, librdkafka) | 0 ×5 | | `measure/provider-gate/pverify.log` |
| `python3 -m unittest discover -s providers/tests` | 0 | 5 tests | `measure/provider-gate/ppython.log` |

**Counts.** This card adds 21 Rust tests and one test binary:
`derived_result.rs` (19), a result row test in `loader_abi.rs` and the
`extern_bridge` refusal test. It also adds two Lean audits. The loader-ABI table
test gains eight rows inside one existing test. The base's own gate figures at
`a7a936d9` were not remeasured here, so the delta is stated by test, not by
subtraction. No `P02-FLAKE` row failed.

**Attempts that are not the gate, disclosed.**

* The first gate run was stopped by hand while `cargo test` was in progress. A
  fail-closed gap had been found: a derived pointer delivered through an out
  slot. It was repaired (`40d4d2cb`, Lean `912ed8b7`) and the arbiter's AFTER
  run was repeated on the new binary. Its log is `measure/gate-aborted-1/`.
* The second run's `cargo test` failed 1 test (exit 101 in `std_socket_inet`,
  `this_card_reserves_no_word_and_its_address_rule_collides_with_nothing`). The
  cause was a tree-wide scan finding `examples/**/*.qcore` inside a base-tree
  copy this card had extracted under `measure/base-src/` to build the old census
  tool. The copy was deleted, the private draft modules were renamed out of
  `*.qcore`, and the gate was rerun from scratch. That run is the one above; the
  failed one is kept at `measure/gate-aborted-2/`.
* A pre-regeneration Python run failed 2 of 319 (exit 1):
  `dimensions.named_followups` and the derived-facts check, before the matrix
  was regenerated. This is the order dependence the validation rule names.

## 9. Ledger and status

* **Retired**, each verbatim with its ground in
  [retired-followups.json](retired-followups.json):
  * `LIBC-MEM-RESULT-ALIAS`, whose v1 result mode now exists and whose
    `memcpy`/`memset` are declared and measured;
  * `W43-LIBC-RESULT-ALIAS-NAMES`: the exact and interior readings are
    implemented, thirteen of its fifteen names are declared, and two are
    refused by name;
  * `W43-LIBC-POINTER-QUALIFIER-READER`: the reader is repaired with Rust, Lean
    and a nil class-(a) arbiter, and the fourteen prototypes are measured. Its
    declarative half is rehomed below.
* **Registered** ([followups.json](followups.json)):
  * `W43-RESULT-ALIAS-ACCUSING-UNEXERCISED-IN-CORPUS`;
  * `W43-RESULT-ALIAS-CONST-WRITE`;
  * `W43-RESULT-ALIAS-RETURNED`;
  * `W43-RESULT-ALIAS-EXACT-RELEASE`;
  * `W43-RESULT-ALIAS-CONDITIONAL-AND-RETAINED`;
  * `W43-LIBC-RESTRICT-PROTOTYPES-UNDECLARED`.
* Follow-ups go from **291 to 294**. `corpus.cycle19_raw_units` points at this
  report's artifacts, and its previous contents are in
  [previous-corpus-row.json](previous-corpus-row.json). `docs/STATUS.md`'s
  corpus, contract-corpus and follow-up rows are updated from this arbiter.

## 10. For the integrator

* **Constraint 7:** the leak-pin chain can record W43-LIBC-RESULT-ALIAS as the
  fourth consecutive declarative arbiter to leave the pin at 272: 13
  declarations and 12 function moves.
* **Constraint 8:** the reader paragraph can record that a qualifier after a
  *parameter's* last `*` is now read (C11 6.7.6.3p15). A qualifier between two
  `*`, an array-bound qualifier and a result's trailing qualifier still refuse.
  This card leaves rule text to the integrator, as W43-LIBC-CONTRACTS did.
* **Files likely to conflict with main:**
  * `providers/quirin-provider-libc/contracts/libc/string.qcore`: this card
    appends at the end, and CONTRACT-INHERIT-ADOPTION is adding `strndup`
    there. Keep both append blocks.
  * `compiler/tests/qcore_contract_corpus.rs` and
    `compiler/tests/contract_loader.rs` (declaration pins);
  * `docs/reviews/r0_3/acceptance_matrix.json`, `docs/STATUS.md`,
    `docs/development/increments/wave-41-continuation.md` and
    `docs/reports/INDEX.md`.
  * `crates/qcore-c-gateway/src/legacy_c.rs`, if another card edits
    `apply_contract_call`, `AbstractPointer` or `refine_pointer_null`.
* The `contracts` artifact embeds this worktree's path in its source keys, as
  earlier isolated cards' artifacts did.

## 11. Remaining work

The registered residuals are listed here in order of measured leverage.

1. **The printf family.** It is now readable: `fprintf` sole-blocks 3
   functions, and `snprintf` and `sprintf` 1 each. It is
   `LIBC-PRINTF-FAMILY`'s leaf-directed card.
2. **Returning a derived result** (`W43-RESULT-ALIAS-RETURNED`). An inferred
   derived-result summary would move `XXH_memcpy` and `mk_string_casestr`.
3. **Exercising the accusing arms** on real code
   (`W43-RESULT-ALIAS-ACCUSING-UNEXERCISED-IN-CORPUS`).
4. **Const-dropped writes, crediting an exact alias's release, `realpath` and
   `pthread_getspecific`.** Each has its own entry.

Most string-scanning bodies in the corpus stay Unknown for reasons this card
does not touch. The dominant blocker is `*p = '\0'` and the other indirect
stores (`indirect memory access`, 17,665 leaves). `cmt_decode_statsd.c`'s
`statsd_process_line` is the clearest example: its three `strchr`/`strstr`
leaves are gone, and eleven indirect-access refusals remain.

## Reproduction

```sh
python3 docs/reports/w43-return-expression/scripts/run_audit.py <before> a7a936d9              # base binary
python3 docs/reports/w43-return-expression/scripts/run_audit.py <reader> 0e9b5a76              # card binary, base declarations
python3 docs/reports/w43-libc-result-alias/scripts/apply_contracts.py exact                    # (drafts in measure/drafts)
python3 docs/reports/w43-return-expression/scripts/run_audit.py <exact> 0e9b5a76+exact-declarations
python3 docs/reports/w43-libc-result-alias/scripts/apply_contracts.py interior
python3 docs/reports/w43-return-expression/scripts/run_audit.py <full> 0e9b5a76+all-declarations
python3 docs/reports/w43-return-expression/scripts/run_audit.py <after> 912ed8b7               # final binary
bash docs/reports/w43-libc-result-alias/scripts/capture_run.sh <capture-run>                   # tee wrapper, READER configuration
python3 docs/reports/w43-libc-result-alias/scripts/strip.py
sh docs/reports/w43-libc-result-alias/scripts/census_all.sh                                    # base and card census tools
python3 docs/reports/w43-libc-result-alias/scripts/census_summary.py
python3 docs/reports/w43-libc-contracts/scripts/unit_texts.py <before>/resource-diagnostics.json <capture>/ppP <unit-texts>
python3 docs/reports/w43-libc-contracts/scripts/attribute.py <reader> <exact> <unit-texts> <attr-exact.json> <string-exact draft> <time-exact draft>
python3 docs/reports/w43-libc-contracts/scripts/attribute.py <exact> <full> <unit-texts> <attr-interior.json> <string-interior draft>
python3 docs/reports/w43-libc-result-alias/scripts/classes.py <before> <reader> <exact> <full> <attr-exact.json> <attr-interior.json> classes.json
python3 docs/reports/w43-libc-contracts/scripts/archive.py <after> docs/reports/w43-libc-result-alias/artifacts
python3 docs/plans/cycle19-custody-cards/blocker_census.py docs/reports/w43-libc-result-alias/artifacts
python3 docs/reports/w43-return-expression/scripts/sole_callees.py docs/reports/w43-libc-result-alias/artifacts 30
python3 docs/reports/w43-libc-result-alias/scripts/update_ledger.py
```

The drafts are the three appended blocks as they stand at the end of the two
modules. The private raw outputs are in this worktree's `measure/`, under
`before/`, `reader/`, `exact/`, `full/`, `after/`, `after-89860d8b/`, `capture/` (wrapper captures
and plain units), `census/`, `unit-texts/`, `gate/` and `provider-gate/`.

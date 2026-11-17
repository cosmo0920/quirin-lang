# LIBRARY-VERSION-HOLD-CURRENT-ROUTE-PENDING

A fail-closed defect (rule 53, contract specification C06.2), launched by the
user on 2026-11-18. Registered by
[LIBCURL-PROVIDER-PLUGIN](../libcurl-provider-plugin/README.md) (section 6;
[comparison](../libcurl-provider-plugin/evidence/callers-comparison.json)).
Base `681fca01a`.

## 1. Prediction (committed before the measurement)

Written after the cause was localised and fixed (sections 2 and 3), before
any Fluent Bit run.

* **The opt-in libcurl run** (the registering card's `callers.sh` over
  librdkafka's two libcurl callers at Fluent Bit 046accf6, on this card's own
  preparation). With the base binary the defect reproduces: `with-default`
  and `with-full` publish `rd_http_global_init` `Pending` with the
  human-resolvable `contract-pin-mismatch` text. With the tip binary:
  `without` 6 Safe / 0 Pending / 39 Unknown / 4 Unsafe as before;
  `with-selected` (every libcurl declaration in force) `rd_http_global_init`
  `Pending` (the binding's `plugin-claim-uncertified`), 6 / 1 / 38 / 4;
  `with-default` and `with-full` (unreadable) `rd_http_global_init`
  `Unknown` with "call to `curl_global_init` whose contract is not in force
  at the audited library version (library-version-unreadable)", 6 / 0 / 39 /
  4. No other row moves on any side; no row reaches `Safe`; nothing moves
  into or out of `Unsafe`.
* **The pinned and live routes** (corpus, system, tail-pause, `pinned`): no
  row can move, by construction (section 4), so they are not measured.

```measure-prediction
route corpus
none
route system
none
route tail-pause
none
```

## 2. The cause

The registering card suspected that the current route, or its checker-binding
join, lost the hold's reason. Neither did. A fixture with the current route, a
ranged user declaration and a bound checker published an out-of-range hold
`Unknown` correctly. The defect needs one more ingredient: **a gateway pass
that rebuilds the unit's environment**.

Four places build a reduced environment from `ResourceContracts::default()`.
Each carries the function map, the retained texts and the cross-unit imports,
and three carry the side tables. None carried `pin_mismatch`. The four places
are (lines at base):

* `AbiAgreement::withdraw_environment`
  (`crates/qcore-c-gateway/src/legacy_declared_abi.rs:574`). It runs when the
  unit's own C declarations contradict any contracted declaration.
* `legacy_checked_spec::reduced`
  (`crates/qcore-c-gateway/src/legacy_checked_spec.rs:133`). It withdraws
  every `resource_body_checked` specification that the unit does not define
  and establish. In practice that means nearly every unit under a selection
  that loads such specifications, the 69-module Fluent Bit selection among
  them.
* `legacy_neutral::component_environment` (`:234`) and
  `legacy_summary::component_lower_environment` (`:806`), the round
  environments of recursive components.

The withheld calls had already been rewritten (`hold_call_sites`, under the
full environment), so each call stayed held. But
`legacy_pin_hold::hold_description_for` (`legacy_classify.rs:1277`) then
found no withheld entry and fell back to the human-resolvable
`hold_description`. So every library-version hold in such a unit (out of
range, unreadable or ambiguous) was published `Pending`
(`contract-pin-mismatch`) instead of `Unknown`. The defect was on every route
that publishes rows, not only `current`.

The card localised the cause in two steps:

* **The ABI gate.** The first commit (`825af79af`) is a failing test. The
  current and contracts routes publish an out-of-range hold `Pending` with the
  human-resolvable text when the caller's unit contradicts another
  declaration's ABI. Fixing the ABI gate alone did not fix the libcurl run:
  `rd_http_global_init` stayed `Pending` (section 6, first tip binary).
* **The other rebuilds.** Those were the remaining rebuilds, so all four now
  carry the table. A gateway test reproduces the checked-specification
  reduction, and it fails without that carry.

The binding was not involved. A held call applies no contract, so no link
folds onto it (`user_plugins::fold_unit` joins, never replaces).

## 3. The change

* `ResourceContracts::carry_pin_mismatch`
  (`crates/qcore-schemas/src/contract.rs`) copies the withheld table, with
  its reasons. All four rebuilds call it, next to `carry_cross_unit_imports`,
  and those are all of that function's call sites. The read goes through
  `pin_mismatch()`, which every unit's hold rewrite already records in the
  lookup log, so the log does not change.
* Rule 53 is amended in place with one sentence: the hold reads its reason
  from the environment the unit is classified under, every rebuilt
  environment carries the table, and a binding's claim is joined. The topic
  page is regenerated.
* Lean (`QCore.LibraryVersionRange`, audited in `AuditLibraryVersionRange`)
  adds six theorems. `heldBy` models `hold_description_for` as a verdict, and
  `reduce` models a rebuild that keeps or drops the table:
  * `heldBy_agrees_with_held`: on a held call, the classifier's verdict is the
    split.
  * `dropped_table_lifts_a_range_failure`: the defect.
  * `carried_reduction_keeps_the_verdict` and
    `carried_reduction_agrees_with_held`: the fix.
  * `binding_never_lowers_a_hold` and `pending_binding_keeps_the_split`: a
    `Pending` claim leaves a range failure `Unknown` and a changed version
    `Pending`.

  No axioms, no placeholders.

### Tests

* `crates/qcore-c-gateway/tests/contract_pin_hold.rs`,
  `a_library_version_hold_keeps_its_reason_under_reduced_environments`. It
  covers the four library-version reasons under three environments:
  * no reduction;
  * the ABI gate's, with `long k_size(void)` against `k_size() -> u32`;
  * the checked-specification gate's, with an undefined
    `resource_body_checked` `k_spec`.

  Changed is `Pending` with the pin-mismatch text. The other three reasons
  are `Unknown` with `library_version_hold_description`. With the ABI gate's
  carry removed, or the checked-specification gate's, the test fails at
  `LibraryVersionOutOfRange` under that reduction (`Pending`). No test
  reaches the two recursive-component carries; they are there by the same
  argument.
* `crates/qcore-audit/tests/library_version_hold_routes.rs` runs the
  `current`, `contracts` and configured routes through the shipped binary. The
  fixture is `library_version_route_database.rs`'s, plus an unranged,
  unbound `fakelib_size` that the caller's unit contradicts, and a checker
  binding whose package answers `Safe` without evidence (so it is `Pending`,
  `plugin-claim-uncertified`). It runs at five standings against a lock
  pinned at 2.5.0, with the range `since "2.0"`:

  | Standing | Database | Record reason | `use_reset` |
  |---|---|---|---|
  | in force | `-Ibundled` (2.5.0) | none | `Pending` (the binding's link) |
  | changed | `-Ichanged` (2.6.0) | `library-version-changed` | `Pending`, pin-mismatch text (negative control) |
  | out of range | `-Iold` (1.9.0) | `library-version-out-of-range` | `Unknown`, library-version text |
  | unreadable | no include directory, none on the default path | `library-version-unreadable` | `Unknown`, library-version text |
  | ambiguous | `-Ibundled` / `-Iother` (3.0.0) | `library-version-ambiguous` | `Unknown`, library-version text |

  A held row carries no plugin link, and the human-resolvable text appears
  only for a changed version. The `pinned` route reads only manifests, never
  a user load path, so it cannot be given a ranged fixture. It classifies
  through the same gateway function (section 4).

### Found on the way (out of scope)

The configured route reads a user load path (its configuration's
`load_path`) but runs none of its checker bindings. It writes no
`plugin_checks` section and no `plugin_links`. So on this fixture the
in-force caller is **`Safe`** on the configured route and `Pending` on
`current` and `contracts`. That is below rule 53's "Unknown -> Pending at
most" for user contracts. It amends W45-PLUGIN-ROUTE-COVERAGE, whose text
assumed that the configured route never reads a user load path. The test
checks only the hold's classification on that route.

## 4. Why the pinned and live routes cannot move

The change touches one value: the reduced environments a gateway pass
rebuilds (section 2). That environment now holds the full environment's
`pin_mismatch` table instead of an empty one.

* With no withheld declaration, the carried table is empty and the reduced
  environment is unchanged.
* The shipped load path and the routes' manifests state no library version
  range (rule 53: the user crates ride the user load path only). So every
  withheld entry on those routes has a human-resolvable reason, and
  `hold_description_for` returns `hold_description` either way.
* The other readers of the table in a unit's environment are:
  * `has_guarded_interface`, which differs only for a withheld
    condition-bearing retaining declaration;
  * `holds_name`, the reuse check, which reads the run environment;
  * `contract_dependencies::row_dependencies`, which is handed the audit's
    run environment (`qcore-audit/src/contract_dependencies.rs:455`), not
    the reduced one.

  Certificates (`legacy_c.rs:2903`) and cross-unit homes carry the run
  environment.
* The corpus's only withheld pin is `mk_vhost_set` (STATUS: `source-changed`
  against the tip of Monkey's fix branch, called by no corpus unit). Its
  contract is plain (`resource_borrowed`, `resource_return_none`), not a
  retaining one.

So no published byte of the pinned, corpus, system or tail-pause routes can
differ. They are not measured: this host has no shared prepared corpus
inputs, and the integrator's landing measurement is the arbiter.

## 5. Interaction with OPENSSL-PROVIDER-REMAINING-CALLEES

That card (branch tip `c714ef64d`, not merged here) adds
`user_plugins::fold_dependencies`. It joins every bound declaration listed in
a row's `contract_dependencies.declarations` that `fold_unit` did not link.
This card does not touch either join. The two are compatible, for three
reasons:

* A held call's callee is spelled with the hold prefix and recorded under
  `contract_dependencies.held`, never under `declarations`
  (`contract_dependencies::read`, `Spelling::Held`). So neither fold links a
  binding to it. The dependencies are computed against the run environment,
  which this card does not change.
* Both folds only join (`Safe < Pending < Unknown < Unsafe`). A
  range-failure hold is `Unknown` from the classifier, and a `Pending` claim
  leaves it `Unknown` whichever path folds it
  (`pending_binding_keeps_the_split`).
* `library_version_hold_routes.rs` asserts that a held row carries no
  `plugin_links`. That stays true after the merge, because the held callee is
  not a declaration the row rests on.

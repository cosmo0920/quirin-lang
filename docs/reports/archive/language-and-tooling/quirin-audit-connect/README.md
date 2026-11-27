# QUIRIN-AUDIT-CONNECT: connecting measurements and contracts

Round 38, base `9aed1913a`. User decisions of 2026-11-26: "we need to
generalize to handle the connecting two or more measurements and contracts
with some of the subcommand for quirin-audit", measurements first, both; and
"a capability for declaration of dependencies each of contracts across the
crates" (Part C, building on Part B).

Nothing here turns `Unknown` into `Safe`: the two subcommands only join and
report, and the declared dependencies only hold declarations. Every refusal
and condition is a formula of the CX block of the C04.18.9 typed rendering
(`qcore_schemas::connect::FORMULAS`, 60 formulas over 80 `cx_` terms and 10
sorts), checked by the premise-formula validator `guarded_proposition`
(`crates/qcore-frontend/tests/connect_formulas.rs`); records name the
conditions they apply by id. The Lean model is `QCore.MeasureConnect`.

## Part A: `quirin-audit measure chain`

```text
quirin-audit --plugin-path DIR --output OUT measure chain \
    (--step RECORD[#BEFORE->AFTER] | --runs BEFORE_DIR AFTER_DIR)... \
    [--declared N=PREDICTION.md]... [--attribute N=RECORD[#BEFORE->AFTER]]...
```

The family is the measurement plugin's (`measure publish`, `measure run`): a
step is a saved native-comparison record (`.json` or `.json.gz`) and the
comparison it names, or two measured side directories compared with the
measurement's own row comparison. Nothing is re-measured. The output is
`OUT/chain.json`, schema `quirin-measure-chain/1`:

| Key | What |
|---|---|
| `steps[]` | each step's source (path, SHA-256, comparison), measurement tool and both sides' identities |
| `links[]` | per consecutive pair, `joined` and every refused condition (`formula`, field, both values) |
| `segments[]` | the maximal joined runs of steps; per segment its points, per-step contract-bundle and dependency-graph changes, and per route its status, causes, per-step counts and deltas, the net status delta (only when complete), rekeyed rows and every reconciled row |
| `segments[].routes.R.rows[]` | per row: verdict at every point, `moved_in`, `net`, `pattern` (`moved`, `cancelled`, `cascade`, `evidence-only`), per-step evidence (leaves, other fields, contract-dependency added/removed), attribution, `dependency_changes`, `review` |
| `unattributed_moves`, `unattributed_new_safe` | every move no prediction covers; every such new `Safe` (flagged, never accepted; the run fails) |
| `formulas` | the conditions the record applies, as written |

**Links** (`chain::link`; formulas `cx_joined_is_every_identity`,
`cx_every_identity_is_joined`): the later step's before side must be the
earlier step's after side,

`forall (#s: cx_step) { forall (#t: cx_step) { (#cx_joined(#s,#t) => ((#cx_next(#s,#t) && forall (#f: cx_field) { #cx_after_id(#s,#f) == #cx_before_id(#t,#f) }) && forall (#r: cx_route) { ((#cx_compares(#s,#r) && #cx_compares(#t,#r)) => #cx_after_inputs(#s,#r) == #cx_before_inputs(#t,#r)) })) } }`

with the fields tool, revision, analyzer binary, contract bundles (required:
an unrecorded one refuses, `cx_refuse_unrecorded`), build commit,
link-resolution, rule switch and dependency graph, and the per-route inputs
(compile-database digests, route digest, spelling, source date) of every
shared route. Each field has its refusal (`cx_refuse_tool`,
`cx_refuse_revision`, `cx_refuse_analyzer`, `cx_refuse_bundles`,
`cx_refuse_build_commit`, `cx_refuse_link_resolution`, `cx_refuse_rules_off`,
`cx_refuse_dependency_graph`, `cx_refuse_inputs`, `cx_refuse_no_shared_route`),
e.g. `forall (#s: cx_step) { forall (#t: cx_step) { (!#cx_after_id(#s,#cx_f_revision) == #cx_before_id(#t,#cx_f_revision) => !#cx_joined(#s,#t)) } }`.
A refused link is never bridged: it splits the chain into segments, each
reconciled alone, and the chain is `refused`.

**Completeness** (`cx_complete_is_every_step` and the six `cx_incomplete_*`):
a route's chain is complete only when every step compared it with status
`complete`, no unmatched row or unit row, no composition conflict and no
row whose consecutive listings disagree at the side between them.

**Rows** (`cx_unlisted_is_unchanged`, `cx_net_is_first`, `cx_net_is_last`,
`cx_moved_is_a_change`, `cx_moved_is_reported`, `cx_cancelled_is_reported`,
`cx_cascade_is_reported`, `cx_evidence_change_is_reported`,
`cx_reports_only`): a row's verdict at a side is the after verdict of its
nearest listing on the left, else the before verdict of its nearest listing
on the right; a step that does not list a row kept it. Every moved row is
reported with its verdict at every side, also when its moves cancel
(`A -> B -> A`) or cascade; nothing is summed, and the net status delta is
counted from the first and last sides' own totals. A renamed or rekeyed row
(the same library, file and function under another key) is reported as such.

**Attribution** (`cx_new_safe_is_a_move_to_safe`,
`cx_unattributed_move_is_listed`, `cx_unattributed_new_safe_is_reviewed`,
`cx_review_is_never_accepted`, `cx_dependency_change_listed`): a move is
attributed by a `--declared` prediction for its step, by the step record's
own declared check, or by a landed card's own record (`--attribute`) that
measured the same move and predicted it. A unit row's move is attributed when
every function row of the unit that moved is. A dependency change (Part C)
is listed beside a move it may explain and is never an attribution.

## Part B: `quirin-audit measure compose`

```text
quirin-audit --plugin-path DIR --output OUT measure compose --load-path FILE... \
    [--bundle CRATE]... [--report REPORT.json] [--libc-names FILE]
```

A bundle is one contract crate of a load path (the shipped
`providers/contract-load-path.json` or a user load path that extends it),
read as data as the plugin SDK reads user crates: its load-path record, its
`contracts.lock` (every reviewed declaration with digest and pin stamp) and
its `quirin-plugin.json`, each with its SHA-256 (`bundles[].identity`). The
output is `OUT/composition.json`, schema `quirin-contract-composition/1`.

* **Declarations** (`qcore_schemas::connect::compose_bundles`): a libc/POSIX
  name declared by any bundle but `quirin-provider-libc` is refused
  (`cx_refuse_libc_redeclared`:
  `forall (#b: cx_bundle) { forall (#g: cx_fn) { (((#cx_declares(#b,#g) && #cx_libc_name(#g)) && !#cx_libc_owner(#b)) => #cx_refused(#b,#g)) } }`);
  several declarations of one function are decided by their pin stamps
  exactly as the loader decides them (`contract_supersession::decide`,
  C06.6), and an undecided set is refused
  (`cx_refuse_conflicting_declarations`). A refused declaration contributes
  `Unknown` (`cx_refused_is_unknown`). Without a libc name list (the libc
  crate not composed and no `--libc-names`) the composition refuses to run.
* **Verdicts** (with `--report`): every function row resting on a composed
  declaration or carrying checker links is recomposed as the four-value join
  (`ReasonedVerdict::fold`, the semigroup `compose.rs`'s `fold_verdicts` and
  `plugin_verdicts::compose` fold with) of its own status, each declaration's
  standing, each dependency hold and each checker link, every contributor
  kept with its own reasons (`cx_composed_bounds_every_member`,
  `cx_composed_bounds_the_host`, `cx_composed_is_a_member`,
  `cx_every_reason_is_kept`). A link's verdict must be its firewall class's
  (rule 10h: an uncertified `Safe` or unvalidated `Unsafe` is `Pending`,
  `cx_uncertified_plugin_is_pending`); a record stating another verdict is
  refused and composed `Unknown` (`cx_refuse_misclassified_record`). A row the
  composition raises above its published verdict is listed (`raised`); none
  is lowered.

Why not `qcore-audit`'s `compose.rs` directly: it composes one run's `.qo`
checkpoints, and the measurement plugin links only the SDK and the schema
crate (`workspace_layering`). Both use the one algebra, `qcore_schemas::verdict`
(the join and `ReasonedVerdict`), and the one supersession decision; there is
no second composition model.

## Part C: declared contract dependencies

```qcore
requires resource_contract_dependency("quirin-provider-libc", libc::malloc, since "0.1.0");
requires resource_contract_dependency("quirin-provider-cfl", cfl::cfl_list_size,
    since "0.1.0", removed "0.2.0", sha256 "<64 hex>");
```

Contract spec C15.2, grammar G05 `contract_dependency`. The clause names the
crate (package name), the declaration (`NS::NAME`, the crate's derived
namespace) and an identity: a version range over the crate's version and/or
the SHA-256 of the target declaration's normalised text (the lock's
`contract_sha256`). The loader reads it (`qcore_contract::contract_dependency`)
and records it per declaration; the load-path manifest records each crate's
dependencies (`contract_dependencies`, written by `providers/contracts.py
sync` and `quirin-audit plugin views --write`), checked against the module
texts and refused on a cycle (`QI014`). After the pins and library versions
the audit resolves them (`qcore_audit::declared_dependencies`):

`forall (#d: cx_dep) { (!#cx_satisfied(#d) => #cx_held(#cx_dependent_bundle(#d),#cx_dependent(#d))) }`,
`forall (#d: cx_dep) { (#cx_held(#cx_target_bundle(#d),#cx_target(#d)) => #cx_held(#cx_dependent_bundle(#d),#cx_dependent(#d))) }`,
`forall (#b: cx_bundle) { forall (#g: cx_fn) { (#cx_on_cycle(#b,#g) => #cx_held(#b,#g)) } }`,
`forall (#b: cx_bundle) { forall (#g: cx_fn) { (#cx_held(#b,#g) => !#cx_contribution(#b,#g) == #cx_safe) } }`.

| Failure | Formula | Hold |
|---|---|---|
| crate missing, declaration unloaded | `cx_missing_is_unknown` | `dependency-unresolved`, `Unknown` (`missing-premise`) |
| crate not a declared dependency, libc target named elsewhere | `cx_undeclared_crate_is_unknown`, `cx_refuse_libc_target_elsewhere` | `Unknown` |
| version out of range | `cx_out_of_range_is_unknown` | `Unknown` |
| digest changed | `cx_digest_change_is_at_least_pending` | `dependency-changed`, `Pending` (`contract-pin-mismatch`, review and re-pin) |
| a held target | `cx_held_dependency_holds_dependents` | the target's class |
| a cycle | `cx_cycle_holds`, `cx_cycle_is_unknown` | `Unknown` (and the manifest refuses it) |

A held declaration is withheld through the pin-mismatch table, so its call
sites are held exactly as a pin mismatch's (`legacy_pin_hold`; the
`Unknown` one reads ``call to `NAME` whose declared contract dependency does
not resolve (dependency-unresolved)``). The report's `contract_revision`
section carries `contract_dependencies` (graph, every target's identity,
cycles, holds) only when some declaration states one; `measure` records each
side's graph per route, so `measure chain` refuses a link whose graphs differ
and lists a dependency change beside a move; `measure compose` joins each
hold into the rows resting on the held declaration.

**Not adopted.** No shipped contract states a dependency in this card: one
clause changes its declaration's normalised text, so the declaration's pinned
`contract_sha256` no longer covers it (`contract-changed`, `Pending`) until a
person re-pins it (`adopting_a_dependency_changes_the_declaration_digest`).
The scratch trial (`scripts/adoption_trial.py`) adds one clause to `onig_new`
(`quirin-provider-onigmo`, `onigmo/regex.qv`); what follows is: the module
diff (one line), the load path's new `contract_dependencies` entry, the lock
entry `2e6db4915c757fa66b59f50584310b05d95818191db4ebea467eea5936d6a223` no
longer covering the text, a `[dependencies]` edge from onigmo to
`quirin-provider-libc` (Cargo and `providers/Cargo.lock`), the user load
path, and the checker binding's module digest. Adopting across onigmo,
librdkafka and cmetrics needs a review and re-pin per declaration: follow-up
CONNECT-ADOPT-DECLARED-DEPENDENCIES.

## Lean: `QCore.MeasureConnect`

One definition per CX term (`cxTerms`, 80) and one theorem per formula
(`formula_<id>`, 60; `formulaTable` states each as written), and:

* `recon_sound`: with faithful step records the reconstruction is the
  reports' verdict at every point; `chain_net_is_last`: the net verdict of a
  joined chain is its last step's after verdict (`joined_same_report`: a
  joined link reads one report, the determinism premise);
* `link_refused_of_differing_identity`;
* `a_cancelled_move_is_reported`;
* `composition_is_the_join`, `composition_keeps_every_reason`,
  `composition_is_order_independent`;
* `held_dependency_holds_dependents`, `held_is_order_independent`,
  `resolution_is_acyclic`.

90 audited theorems, standard axioms only (`propext`, `Classical.choice`,
`Quot.sound`), no placeholder. Premises stated, not proved: a side's report is
a function of its identity; the Rust resolver's iteration reaches the least
fixed point `Held`; the records read are the trusted producers'.

## Demonstration on committed records

`scripts/demonstrate.py` (no re-measurement; `demonstration/summary.json`,
each chain's record gzipped):

| Chain | Links | Result |
|---|---|---|
| landing-556460e31 → sp1 (`b7f86a0de -> 2c07390b6`) | refused: revision, analyzer, bundles, build commit, tool | two segments |
| landing-556460e31 → MEMBER-EFFECT-COMPONENT-SCOPE (`7b726587b -> 662dc54a4`) | refused: revision, build commit, tool, unrecorded (a baseline side's analyzer) | a shared base, not a chain |
| landing-556460e31 → landing-777fba16e (`c8c7edf21 -> 777fba16e`) | refused: revision, build commit, unrecorded | two segments |
| OUT-PARAMETER-BODY-SUMMARY → OUT-PARAMETER-SCALAR-FRAME-PREMISE → OUT-SLOT-CALL-WRITE-ALIASING | joined, joined | complete; `cio_file_content_copy` cancelled (Unknown, Unknown, Safe, Unknown), four moves attributed |
| SP2 → SP3 → SP4 | joined, joined | complete; no move, evidence only |
| MEMBER-EFFECT-COMPONENT-SCOPE → MEMBER-RELEASE-UNTYPED-BASE | joined | complete; the first card's four unpredicted moves flagged |

As expected, the landing record of 556460e31 joins none of the later records:
every later landing was measured at another base. With its cards' own
integration records (`--attribute 1=...`, 12 records) the tool reproduces the
page's hand attribution: 22 of the 28 moved rows are attributed to the card
that measured and predicted them (and the `librdkafka` unit row is derived),
and the 6 moves those cards measured unpredicted (MEMBER-EFFECT-COMPONENT-SCOPE's
`ctr_id_destroy` and `destroy_chunk_raw` in corpus and both system units;
SCALAR-STATEMENT-FORMS's `LZ4_createStreamDecode` and `LZ4_initStream`,
justified in prose in its report) are flagged for review, not accepted. The
netted-out `cio_file_content_copy` the page names is the cancelled row of the
out-parameter chain.

## Fixtures

* `crates/quirin-audit-plugin-measure/tests/chain.rs` (7): a clean three-step
  chain with a cancelling move, a cascade and a verdict-unchanged evidence
  change; links broken by a commit, an inputs digest, a contract bundle and an
  unrecorded analyzer; an incomplete route with a rekeyed row; an
  unattributed new `Safe` (flagged, failing), attributed by `--declared` and
  by a card's record (`--attribute`, predicted and not); a move beside a
  dependency change, and a link refused for a differing graph; measured side
  directories chained without re-measurement.
* `crates/quirin-audit-plugin-measure/tests/compose.rs` (4): two bundles
  agreeing and disagreeing, a duplicate libc declaration refused, a plugin
  `Pending` kept with its reasons (and a misclassified record composed
  `Unknown`), declared dependencies satisfied, missing, out of range,
  digest-changed, transitively held and on a cycle; no libc list refused.
* `qcore_schemas::connect` (9), `qcore_contract::contract_dependency` (3),
  `qcore_audit::declared_dependencies` (4: satisfied, every unresolved class
  withheld, none, the adoption digest), the gateway's
  `contract_pin_hold::a_dependency_hold_is_classified_by_its_reason`, the
  formula tests (3) and `providers/tests/test_contract_load_path.py`'s
  `ContractDependencies` (2).

## Records and follow-ups

`qcore-schemas` 29.0.0 (two `PinMismatchReason` variants; one commit, so a
merge can renumber it). The CX block is appended after K6; the two block-order
tests (`plugin_guarded_symbols`, `type_query_positions`) admit it. Follow-ups
are registered in [integration-amendments.json](integration-amendments.json).

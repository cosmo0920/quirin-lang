# AUDIT-COMPOSITION

Card AUDIT-COMPOSITION builds the results half of the pluggable-cards plan's
seam G (`docs/plans/pluggable-cards/README.md`). It follows the user decision
of 2026-10-24: results and contracts integrate only through a semigroup
interface that never contaminates any audited unit's result. Contract-bundle
supersession by pin stamp is the separate card CONTRACT-SUPERSESSION and is not
implemented here. The card changes no verdict, route or analysis. The report's
unit rows become a derived view over per-row components, and the identity run
below shows every existing report artifact is byte-identical to BASE.

Base: `52c8dd71b` (origin/main at launch). The branch merged local main as it
moved: GATE-ELASTIC, GENERATED-ROOTS, and finally `c3d8ad3d4`
(AUDIT-CANONICAL-PATHS and CMetrics). The identity run measured TIP
`26be252f7` against BASE (see "Source equivalence after the run").

## What changed

**`qcore-schemas::composition` (crate 20.1.0 → 21.0.0).** This module holds
serde types and pure functions only:

- `Component`: `(unit, row, property, profile, provenance, verdict tuple,
  reason leaves)` plus the producer's opaque `detail`.
- `Provenance`: the analyzer revision, the `RuleSet` with its off-list (a data
  slot, empty until RULE-SWITCHES), the contract-bundle digests, and an optional
  `CheckerIdentity`.
- `VerdictTuple`: the verdict, scope, bounds, assumptions and evidence version.
- `Contribution`: a set of components. `union` is `⊕`, the provenance-keyed
  union. `derive(row)` returns the `Verdict::join` over the row's components
  and carries every component's assumptions and reasons. `remove` is the
  re-join without a contribution.
- `ComponentSidecar`: a contribution plus the order in which a report presents
  its rows. `validate` refuses a row that is presented twice, presented without
  a component, or never presented.

A `Contribution` deserializes through `insert`, so stored data cannot hold a
state that `⊕` would not have built.

**New reason category `composition-conflict`.** Its subject is the analyser and
it is not human-resolvable. The crate's policy treats a new variant on the
exhaustive `ReasonCategory` as a major change, following the 17.0.0 and 18.0.0
precedents, so the crate is now 21.0.0. The closed vocabulary grows from eleven
categories to twelve. The classify marker is `composition-conflict: `, a string
no earlier site spells, so no existing reason text classifies differently. The
nine root-workspace manifests that pin `qcore-schemas`, `providers/Cargo.toml`,
the W3 provider fixture and the three lockfiles were moved to 21.0.0. The spec
gains a §L21.1 table row and a §L21.2 paragraph, "Composition across results".
`proof/QCore/ReasonCategory.lean` gains the category and its family.

**The report emitter (`crates/qcore-audit`).** One choke point carries the
change: `AuditReport::write_with_revision_key`, used by the pinned route, the
current route, the configured route (`lib.rs`) and `quirin-audit contracts`.
`report_components::sidecar` turns every unit row into one component, written
first to `<report>.components.json` (compact JSON). The report's `units` are then
rendered from that sidecar by `report_components::render`, never written from the
rows directly. A row with one component and no conflict renders as that
component's detail, borrowed, so the bytes are unchanged. Each of the four
routes records its contract bundle (`record_contract_bundle`: SHA-256 of the
loaded `ResourceContracts`, as `certificate_context` takes it) just before the
write. No verdict logic, route selection or analysis is touched.

The component of a unit row is built as follows:

| Field | Value |
|---|---|
| unit | `{library, file}` of the row |
| row | `unit#n`, where `n` counts earlier rows with the same unit in report order. The system report has repeated `(library, file)` rows (one per compile configuration), so the unit alone is not a key |
| property | `resource-custody.v1` (as `projection::hypothesis_context` names it) |
| profile | the row's library (source-set) profile; binding it to an evaluation profile is EVALUATION-PROFILES' work |
| provenance | `qcore-audit@<version>:<QCORE_AUDIT_BUILD_COMMIT or unrecorded-build>`; rule set `qcore-c-gateway:built-in-route-map` with an empty off-list; bundle `resource-contracts` → digest; no external checker |
| verdict | `checkpoint::unit_verdict(status)`: a verdict spelling is itself, `data_only` is `Safe` (the join's identity, no obligation), and `gateway_failure` or any unknown spelling is `Unknown` |
| scope, bounds | the checkpointed route's run-identity spellings |
| assumptions | named premises the row records: the digest of `assumptions` and of `cross_unit_link_resolution`, and for each function `contract:` declarations, `summary:` summaries and `null-premise:` names (rule 13h's `contract_dependencies`, rule 14n's `null_premises`). This is a projection of the row; the detail stays complete |
| reasons | the row's `reason` with its parsed `reason_code` (`null` stays no category) |
| detail | the row itself |

**Fail-closed.** If no component can be built for a row, because it names no
unit, carries no status or is not an object, the row's component is a refusal:
`Unknown`, `composition: no component could be built for this row: <why>`, with
the row kept beside it. An unavailable contract digest refuses every row the
same way. A presented row with no component renders `Unknown`. A conflicted row
renders at least `Unknown`, with `reason_code: composition-conflict`. None of
these falls back to writing the unchecked row.

**Laws.** Two places test them, and they share one vector list:

- Rust, `crates/qcore-schemas/tests/audit_composition.rs`. Generated
  contributions over a 96-component universe, 3,000 deterministic rounds per
  law, check associativity, commutativity, identity, idempotence and erasing
  nothing. They also check that a byte-identical duplicate changes nothing and
  conflicts with nothing, that the join never lowers a verdict, and that
  assumptions and reasons only grow. A conflict gives at least `Unknown`
  (exactly `Unknown` without `Unsafe`), keeps both components and carries the
  category. Removal of a disjoint contribution restores the prior set and
  derivation (more than 1,000 disjoint pairs reached). The tests also cover a
  wire round trip through the union, an external checker as a distinct
  provenance, and sidecar validation.
- Lean, `proof/QCore/AuditComposition.lean`, registered in
  `proof/AuditAuditComposition.lean` (35 audited theorems, standard axioms only:
  `propext`, `Quot.sound`). It proves the same laws in general. Every derived
  quantity reads only the set of components (`derive_congr`), so the derived rows
  inherit the algebra. The file also proves `the_join_never_lowers`,
  `adding_never_removes_an_assumption`, `a_conflict_keeps_both`,
  `a_conflict_without_unsafe_is_unknown`, `removal_restores_the_derivation`, and
  `derive_none_iff` / `the_empty_contribution_publishes_nothing` (absence is
  refusal).
- The 14 shared vectors (`vectors`, decided by `vectors_hold`) are parsed out of
  the Lean file by the Rust test and checked against `Contribution`.

`crates/qcore-audit/src/report_components.rs`'s own tests cover the rendered
rows: byte equality, borrowing, repeated units, refusals, conflict rendering,
`check` refusing drift, and the sidecar's wire round trip.

## Decisions taken inside the brief

1. **Conflict and `Unsafe`.** The brief says a conflict "leaves the row
   Unknown". The derivation joins `Unknown` into the row and keeps every
   component's own verdict in the join. A conflicted row is therefore exactly
   `Unknown` unless some component is itself `Unsafe`. In that case it stays
   `Unsafe`, carrying the conflict reason, because the join never lowers a
   verdict, which is a law this card must also hold. Replacing the conflicted
   verdicts with `Unknown` would lower an `Unsafe` row when a component is
   added, breaking monotonicity. This boundary is proved
   (`a_conflict_without_unsafe_is_unknown`) and stated in §L21.2.
2. **The conflict reason names no count.** A leaf such as "2 distinct
   components" would be replaced when a third component arrives, so adding a
   component would remove a reason. The generated law test caught this. The
   leaf names the row and the analyzer revision only.
3. **Absence is refusal.** `derive` of a row with no component is `None`,
   never the join's identity `Safe`.
4. **The acceptance matrix's derived fields were regenerated**
   (`build_matrix.py .`, on main's copy at each merge). Against main
   `c3d8ad3d4` the change is: schema version 21.0.0, Lean audits 373 with the
   new file listed, and `dimensions.named_followups` 830 → 832, a stale count
   carried from main. Gate figures are carried and labelled as carried. Without
   this, `test_the_derived_facts_are_recomputed_rather_than_carried` fails on
   the card's own tree. The integrator owns the matrix and should regenerate it
   at landing.
5. **Lean roots** are generated (GENERATED-ROOTS). `proof/QCore.lean` gains
   `import QCore.AuditComposition` through `scripts/gen_lean_roots.py`, not by
   hand.

## Identity run

**Result: every existing report artifact of the pinned eight-home corpus, the
tail-pause slice and the system selection is byte-identical between BASE
`52c8dd71b` and TIP `26be252f7`. The only new outputs are the three sidecars.
Every published unit row re-derives from its sidecar.** Record:
[identity.json](identity.json), from `scripts/compare_identity.py` over run 3.

How it was run (`scripts/`). The snapshots are `git archive` of each revision,
with `third_party/libevent` cloned at the pinned `d82464a2`. The pinned route
verifies that clone. Without it, `git rev-parse` resolves the enclosing
worktree's HEAD, which is what failed run 1's corpus on both sides. Each side
is built (`build_side.sh`) and run (`run_identity.sh`) in its own mount
namespace (`ns_side.sh`, `unshare -rm`), with its snapshot bound at the one path
`private/ac/side-src` and with one explicit
`QCORE_AUDIT_BUILD_COMMIT=audit-composition-identity`.

This is needed because the audit embeds its own source path in contract-module
paths, and digests over those paths reach the report (`assumptions`,
`build_identity`, `home_environment_sha256`, hypothesis ids). Its build commit
also reaches the report, in the hypothesis graph's `evidence_version`
(`crates/qcore-audit/build.rs`). Run 2 kept separate snapshot paths and each
side's own commit, and differed in exactly those digests and strings. They
cannot be checked by a textual substitution, so they were removed at the
source. The routes are the W48-BU-SINGLE-LEAF-BODIES driver (`measure.py`)
through its re-spelled mirror (`mirror_inputs.py`; AUDIT-CANONICAL-PATHS had
not landed when the runs started). Each route ran under an `audit` slot
(`gate_slot.py`, 7 GiB, `--jobs 3`), BASE and TIP side by side. Binaries:
BASE `c414ad52…`, TIP `997633eb…`.

| Route | Report artifacts byte-identical | New on TIP | Unit rows re-derived (TIP and BASE report vs TIP sidecar) |
|---|---|---|---|
| corpus | 7: `resource-diagnostics.json`, `monkey-source.json`, `.chunk-contracts.json`, `.source-effects.json`, `corpus-null.jsonl`, `corpus-null.summary.json`, `corpus-outer.json` | `resource-diagnostics.components.json` (80.8 MB) | 185 rows, 185 components, 0 conflicts (12 Safe incl. 4 data-only, 166 Unknown, 7 Unsafe), all `unit#0` |
| slice | 6 (same set; `slice-*`) | sidecar (3.1 MB) | 16 rows, 0 conflicts (15 Unknown, 1 Unsafe), 3 rows `unit#1` |
| system | 6 (same set; `system-*`) | sidecar (6.9 MB) | 15 rows, 0 conflicts (13 Unknown, 2 Unsafe), 7 rows `unit#1` |

Every exception is checked and listed, never skipped:

- **`resource-diagnostics.contracts.json` differs in one map key per route,
  for a reason unrelated to this card.** One key of
  `callback_members/lifetime_protocols/…/summaries` is a lifetime-source unit
  identity, and `loop_lifetime_source::read_unit` hashes the unit's
  preprocessed text. Monkey's `mk_bin/monkey.c` expands `__DATE__`/`__TIME__`,
  so that identity depends on the minute of preprocessing, whatever the binary.
  Across all four corpus runs exactly one key varies. Two runs of the same BASE
  code (runs 2 and 3) disagree on it. Run 2's BASE and TIP, started together,
  agree on it. The comparison accepts the file only if exactly one key differs,
  and only if swapping its text gives the TIP document as JSON and as a line
  multiset. Follow-up: LIFETIME-SOURCE-IDENTITY-TIME-MACROS.
- **Logs and driver records** are compared after the substitution of the BASE
  run directory by the TIP one. `audit.log` and every
  `resource-diagnostics.invocation.json` are equal after it. What remains is the
  binary digest and timestamps in `run.json`/`console.log`, the `preparation:`
  wall-clock line of the current route's console, and `invocation.json`'s
  snapshot-tree digests. Those name the two revisions' own source files, by
  design.
- **The corpus driver exited 101 on both sides** with `identity_error`. The
  only changed input is `third_party/libevent/.git/index`, the libevent clone's
  stat cache, which the pinned route's own `git` call refreshed. `audit_exit`
  is 0 on both sides. The slice and system routes, which ran after the index
  was refreshed, exited 0.

Earlier attempts, retained but not evidence ([attempts.json](attempts.json)):
run 1 (libevent clone absent; corpus failed on both sides before writing a
report; system phase stopped by captured PIDs while queued) and run 2 (separate
snapshot paths; corpus exit 0, with the path and build-commit digests above).

**Source equivalence after the run.** TIP `26be252f7` was measured. The branch
then merged local main `c3d8ad3d4` (AUDIT-CANONICAL-PATHS, CMetrics,
records). That merge changed no line of `crates/qcore-schemas`,
`report.rs` or `report_components.rs`. Next to main's own changes in
`current.rs`, `pinned_route.rs` and `lib.rs`, it carries this card's one
`record_contract_bundle` call at each write site unchanged. Main's changes
carry their own identity evidence, and no identity run was repeated on the
merged tip. The landing gate is the arbiter.

## Gate

GATE-PLACEHOLDER

## Boundaries

- Only unit rows written through `AuditReport::write_with_revision_key` carry
  components. The checkpoint-composed report (`compose.rs`, which streams rows
  from `.qo` checkpoints) writes no sidecar. The property rows (null, bounds,
  outer projection) and their summaries are not componentized either.
  Follow-up: COMPOSITION-PROPERTY-AND-CHECKPOINT-ROWS.
- The analyzer revision is weak identity. A build without
  `QCORE_AUDIT_BUILD_COMMIT` is `…:unrecorded-build`, so two different such
  builds share a provenance key. Joined, they surface as a conflict rather than
  as an unnoticed union, which is fail-closed. The binary digest is not in the
  provenance. Follow-up: COMPOSITION-ANALYZER-IDENTITY.
- The component's `profile` is the library profile, not an evaluation profile
  (EVALUATION-PROFILES).
- The checker-plugin firewall still folds plugin answers into the host's row
  (`user_plugins::fold_unit`); it does not yet emit external-checker components.
  The contract loader still unions bundles its own way. Follow-ups:
  COMPOSITION-PLUGIN-FIREWALL-WIRING, COMPOSITION-CONTRACT-LOADER-WIRING.
- MEASURE-TOOL, a `quirin-audit` subcommand under user decision 8, is the
  intended consumer: it composes a baseline with a TIP contribution, and derives
  a rule's contribution as a removal. Follow-up: MEASURE-TOOL-COMPOSITION-CONSUMER.
- This card does not establish that a `Pending` component names a
  human-resolvable reason. The component carries the row's status as it is.
- The Lean model is abstract (identifiers as naturals, bytes as structural
  equality). Its correspondence to the Rust is maintained by the shared vectors
  and review, not by proof (constraint 28).

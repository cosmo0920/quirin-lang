# LINK-STANDING

Card base `a0ac2cae9` (FLUENT-BIT-LINK-PREPARE's tip on origin/main
`1e522ca5d`). Brief: round 28, rule 40. This card implements the user's
decision on BODY-SUMMARY-REUSE-LINK-STANDING ("Option A (rec.)", 2026-11-06):
the cross-unit link resolution of the `current` route is derived from the
linked Fluent Bit executable and fed to rule 4c unchanged. It also follows
the user's decision of 2026-11-08 on Monkey ("Analyse linked config"): rule
40 has no Monkey case, and decides every home by the same conditions.
[PREDICTION.md](PREDICTION.md) was committed before the semantic change.

## Rule 40

[Rule 40](../../agents/rules/40.md) is a new rule. One sentence in
[rule 4c](../../agents/rules/4c.md) points to it. With
`QCORE_CROSS_UNIT_LINK_RESOLUTION` naming a `quirin-link-resolution/1`
artifact, the `current` route uses the artifact's homes instead of a stated
map. Rule 4c's export, census, selection, reachability scope, replay budget,
two-sided replay and certificate check are unchanged. Rule 40 decides which
names rule 4c sees, and which of a caller's held homes it keeps.

The option excludes `QCORE_CROSS_UNIT_LINK_MAP`. A pinned route refuses it.
Checkpoint composition, contracts-only runs and per-library provider scoping
refuse it, as they refuse rule 4c. When the option is absent, the route is
unchanged.

### Conditions and refusals

The conditions are checked in this order. The first one that fails is the
refusal (`cross_unit_links::decide`, Lean `QCore.CrossUnitSummary.standing`).
Every refusal leaves the call `call to uncontracted function`, exactly as
without the artifact.

| # | Condition | Checked | Refusal phrase |
|---:|---|---|---|
| 1 | the artifact's compile-database digest equals the route's | run | the link resolution artifact's compile database differs from the route's |
| 2 | executable, link command, map, driver, linker and every linked input still have their digests | run | a digest the link resolution artifact binds does not verify |
| 3 | the caller's configuration (file, directory, command digest) is linked and reproduced | caller | caller configuration is not a linked, reproduced configuration of the executable |
| 4 | the image is an executable | home | the linked image is not an executable |
| 5 | the definition is in the executable | home | home is not defined in the executable |
| 6 | strong global function (not weak, common, unique, ifunc) | home | home is not a strong global function definition |
| 7 | default visibility | home | home definition does not have default visibility |
| 8 | the only definition among the linked objects | home | home has more than one definition among the linked objects |
| 9 | no dynamic relocation against it | home | the executable resolves the home at run time (dynamic relocation) |
| 10 | the artifact refuses none of the link's own facts | home | the artifact refuses the home's link facts |
| 11 | the home configuration is linked and reproduced | home | home configuration is not a linked, reproduced configuration of the executable |
| 12 | the route analyses that configuration | home | home configuration is not a configuration the route analyses |
| 13 | the route's `library/file` key names only it | home | home unit key names more than one route configuration |
| 14 | Clang gives the callee's type in both units | caller | call type unavailable |
| 15 | the caller's type equals the home definition's | caller | call type differs from the home definition's type |

When a run-level condition fails, the route prints `cross-unit import phase
refused: <reason>: <detail>`, holds no import, and records the refusal in
the run's `cross_unit_link_resolution` assumption. A refused home is never
exported, and the assumption counts these refusals by reason. A caller-level
refusal drops a held home after rule 4c's scope and budget have run
(`cross_unit_retain_held`). Such refusals are listed on the caller's unit row
(`cross_unit_link_refusals`) and printed as `cross-unit import F refused:
<reason>`.

Notes on single conditions:

- **10.** The route does not take the artifact's route-relative refusals
  ("not a unit of the current route", "ambiguous key"). It decides them
  again in conditions 12 and 13, on the configurations it actually analyses.
  So a home becomes derivable as soon as the route analyses its linked
  configuration, with no change to the artifact or the rule.
- **12.** This condition is how Monkey is refused on this base (see "Monkey"
  below).
- **7.** The brief names default visibility. The prepare step records hidden
  visibility but does not refuse it. Rule 40 does refuse it. FLUENT-BIT-LINK-PREPARE
  found every derived definition at default visibility.

### The call-type check

`qcore-c-gateway/src/link_call_type.rs` reads the type from the units the
route already preprocesses. No C++ body is read, and DWARF is not used. For
each held home it makes two Clang launches, one on the caller's text and one
on the home's (cached per home text):

1. **The declaration in force.** Clang's name-filtered JSON dump of the
   unit's text, with one probe appended:
   `typedef __typeof__(NAME) __quirin_link_call_type_NAME;`. The probe's
   name contains `NAME`, so one launch returns every declaration of the name
   and the probe. Every `FunctionDecl` of exactly `NAME` must be at file scope
   (no `parentDeclContextId`) and must spell one type. Then the type in force
   at every call, or at the definition, is the probe's type.
2. **The canonical signature.** The probe's type tree is rendered as follows:
   - sugar is removed (`TypedefType`, `ElaboratedType`, `ParenType`,
     `MacroQualifiedType`, `TypeOfExprType`, `DecayedType`);
   - parameters' own top-level qualifiers are dropped (C11 6.7.6.3p15);
   - the prototype's attributes (`variadic`, `cc`) are kept;
   - a function without a prototype, an unnamed tag or any other node kind
     (`AttributedType`, variable-length arrays, `_Atomic`, ...) refuses.
3. **Records.** Every tagged record the signature names is compared by tag.
   Where both units complete it, Clang's canonical record layout
   (`-fdump-record-layouts-complete -fdump-record-layouts-canonical`) must be
   equal. That layout gives offsets, member names and canonical member types,
   with embedded records expanded. A position-based `(unnamed at …)` is
   normalised. A record that one unit leaves incomplete is opaque there: the
   unit cannot name its members. A record laid out twice with different text
   in one unit refuses.

Premises: Clang's extraction is faithful (trust boundary 28); C11 6.2.7 and
6.5.2.2p9, under which a call through a type compatible with the
definition's type is defined. What stays assumed: a record reached only
through a member pointer is compared by tag alone, and enumeration constants
are not compared (follow-up `LINK-STANDING-INDIRECT-RECORD-IDENTITY`).

### Assumptions on the verdict contract

When the import phase runs under a derived resolution,
`cross_unit_link_resolution` contains:

- `"evidence": "derived"` and `derived_from`: the artifact path and SHA-256,
  and the compile-database, executable and map digests;
- the derived home count and the refusals by reason;
- for each imported home, its identities and the artifact's definition
  record.

It contains no `premise`. The derived `LinkMap` is digest-pinned by the
artifact's SHA-256, and that digest becomes the link-resolution identity rule
4c binds in each caller's and home's evidence. So a different artifact
re-audits every caller.

## Lean

`QCore.CrossUnitSummary` is extended, and `proof/AuditCrossUnitSummary.lean`
prints every new theorem. There is no placeholder and no project axiom.

| Theorem | Statement |
|---|---|
| `standing_none_iff` | a pair resolves exactly when every condition holds |
| `failed_condition_refuses` | any failed condition refuses |
| `database_mismatch_refuses`, `unverified_digest_refuses`, `unlinked_caller_refuses`, `shared_object_refuses`, `weak_definition_refuses`, `duplicate_definition_refuses`, `interposable_refuses`, `unlinked_home_configuration_refuses`, `unanalysed_linked_configuration_refuses`, `ambiguous_key_refuses`, `unavailable_type_refuses`, `unequal_type_refuses` | one refusal per condition the brief names |
| `typeAgrees_iff`, `missing_caller_type_disagrees` | the type condition: both types given and equal |
| `derived_resolution_discharges` | a resolved pair, with rule 4c's per-home checks, is `admitted` under the derived premise (the obligation the stated map discharged) |
| `derived_admission_is_valid` | an admitted derived import carries what `admitted_import_is_valid` gives a stated one |
| `derived_refusal_is_not_admitted` | every refusal is a missing resolution |
| `refused_home_supplies_nothing` | a refused home never supplies a summary: an undeclared call to it stays uncontracted |
| `standing_vectors_hold` | the 20 `standingVectors`, which equal the Rust `STANDING_VECTORS` (`standing_vectors_hold` test) |

`derivedPremise` makes the stated premise's `resolves` relation a checked
fact for one caller: `resolves name unit := standing (facts name unit) =
none`. The Rust/model boundary is the extraction of each condition's
Boolean from the artifact, the route's configurations and Clang.

## Fixtures

In `crates/qcore-audit/src/cross_unit_links.rs`, over a synthetic artifact
whose bound files are real files with their recorded digests:

| Test | Shows |
|---|---|
| `a_derived_resolution_decides_every_home_condition` | `callee` is derived under the route's own key, and the map's digest is the artifact's. The other homes are refused one per reason: shared-object, weak, hidden, duplicate, dynamically relocated, not reproduced (artifact fact), unlinked home configuration, **ambiguous key**, and a home whose **analysed configuration differs from the linked one** (the route analyses `-DLIBEVENT`; refused by condition 12). The same configuration is unlinked as a caller. |
| `a_changed_database_or_bound_file_refuses_the_artifact` | **digest mismatch**: another database refuses (1). A rebuilt linked input or a relinked executable refuses (2). Restoring the input verifies again. |
| `a_derived_home_is_held_only_by_a_linked_caller_with_an_equal_type` | The **derived home is adopted**: a linked caller whose declaration spells the home's typedef differently but has the same canonical type and layout is `Safe`, and its certificate replays the home. An **unlinked caller is withheld** (refusal 3, row `Unknown`). A **type mismatch** (`long *` against `struct s *`, refusal 15), an unequal record layout (refusal 15) and a block-scope declaration (refusal 14) each leave the call uncontracted. |
| `a_caller_over_the_budget_holds_no_derived_home` | **budget exceeded**: 25 derived homes, all called. Rule 4c withholds them all, rule 40 refuses nothing further, and the row is `Unknown`. |
| `standing_vectors_hold` | the decision table against the Lean vectors |

End to end, in `crates/qcore-audit/tests/link_standing.rs`
(`the_current_route_adopts_a_derived_home_only_under_its_conditions`), the
test runs `quirin-audit current` on a two-unit checkout:

- without the artifact, `subject`'s call to `release` in another unit is
  uncontracted (`Unknown`);
- with the artifact, `subject` is `Safe`. Its declaration
  `void release(byte *)` uses a typedef, but the canonical type is equal.
  The run's assumption names `"evidence": "derived"`;
- an artifact for another database prints the run-level refusal, and
  `subject` stays `Unknown`;
- an artifact that does not link `caller.c` leaves `subject` `Unknown`, and
  its row lists `cross_unit_link_refusals`;
- a stated map together with an artifact is refused as exclusive.

In `crates/qcore-c-gateway/src/link_call_type.rs`:

- `typedefs_desugar_and_parameter_qualifiers_drop`;
- `unprototyped_block_scope_and_disagreeing_declarations_refuse`;
- `layouts_compare_where_both_units_complete_a_record`.

## Monkey

There is no Monkey rule, as the user's decision of 2026-11-08 requires. On
this base, the route analyses Monkey from the separate libevent database. No
configuration of that database is among the artifact's configurations, so:

- a Monkey caller fails condition 3 and holds no import;
- a Monkey home (the executable links `lib/monkey` from Fluent Bit's own
  configuration) fails condition 12, because the route does not analyse that
  configuration.

This covers the 24 Monkey callees and their 55 sites. Once
CURRENT-MONKEY-LINKED-CONFIGURATION makes the route analyse the linked
configuration, both conditions hold with no change to rule 40. The fixture
`elsewhere` above is that case in miniature.

## Option C (stopgap)

After rule 40, the two remaining cross-unit sole-blocked functions are
`mk_mq_send` (`mk_fifo_send`) and `net_plain_writev` (`mk_iov_send`). Both
wait on Monkey. A declared body-checked contract would need these:

- a `qcore.monkey` provider declaration for `mk_fifo_send` and for
  `mk_iov_send`, each validated by rule 13g against its home body at the
  pinned Monkey revision;
- `mk_fifo_send` writes a message through the FIFO's pipe file descriptors
  under the queue's mutex;
- `mk_iov_send` writes the iovec through a socket file descriptor and
  returns the byte count.

Their effects, file-descriptor writes and a result range, are not a
mechanical rule-13g declaration, so this card writes neither contract.
CURRENT-MONKEY-LINKED-CONFIGURATION is expected to free both through rule 40
instead.

## Prediction

[PREDICTION.md](PREDICTION.md):

- **corpus, system and slice: none.** They are pinned routes on the stated
  eight-home map, and rule 40 does not apply to them.
- **`current` without the artifact:** unchanged byte for byte.
- **`current` with the artifact** (upper bound, from the 165-home experiment
  restricted to derived non-Monkey homes): at most 14 Unknown -> Safe (5 of
  them cross-unit sole-blocked functions), at most 246 Unknown -> Unknown
  leaf changes, at most 1 Unsafe -> Unsafe, and at most 357 resolved import
  leaves. No unit status changes.

These are upper bounds for three reasons. A type refusal keeps a call
uncontracted. The 1,384 derived names can push busy callers past the
24-home budget, which withholds them. And deeper leaves can be exposed. The
integrator measures, and reruns `current` with
`QCORE_CROSS_UNIT_LINK_RESOLUTION` set to the `run-1-second` artifact
(SHA-256 `c6535626…38690c9b`, database `6ab94a13…f9dd3de3`).

Cost: the type check adds two Clang parses per held home, one of the caller
text and one of the home text (cached per home), and one layout dump per
unit whose held signatures name a record. This was not measured here.

Measurement command (r13-common):

```sh
target/release/quirin-audit --plugin-path target/release --output <dir> --timeout-ms 36000000 measure \
  --inputs crates/quirin-audit-plugin-measure/inputs/live.json --inputs-base /media/Data/GitHub/quirin-lang \
  --side base=baseline:7b726587bdf6a8213f451fa1c78297bb5e476ef7 --side tip=rev:1f04fb45a \
  --declared docs/reports/link-standing/PREDICTION.md
```

## Follow-ups

- `LINK-STANDING-INDIRECT-RECORD-IDENTITY`: records reached only through a
  member pointer are compared by tag, and enumeration constants are not
  compared. Discharge: a transitive layout comparison, or a DWARF
  cross-check of the home's subprogram type.

## Gate

`python3 scripts/gate_scope.py --base a0ac2cae9 --out private/gate-3` at
`1f04fb45a`: **exit 0**, mode full (the diff reaches `qcore-c-gateway`). The
record is [`gate.json`](gate.json). Console log outside the worktree, in the
session scratchpad (`gate-3.console.log`).

| Step | Exit | Counts |
|---|---:|---|
| submodules | 0 | 1 submodule |
| fmt | 0 | |
| clippy | 0 | |
| rust-suite | 0 | 617 units, 9,300 passed, 0 failed, 16 ignored (8,712 s including the slot wait) |
| reevaluation probe / follow-up | 0 / 0 | |
| lean | 0 | 1 audit (`AuditCrossUnitSummary.lean`: no placeholder; axioms only `propext`, `Quot.sound`) |
| python | 0 | 685 tests |

`private/gate-2` was the previous attempt, at `76f10d7ae`. Every step passed
except python, which had 1 failure. `test_tracked_files` refused the
end-to-end fixture's directory walk (`fs::read_dir`). `1f04fb45a` reads the
two report files by name instead. That attempt's rust suite also passed
(9,300/0). An earlier attempt (`private/gate-1`) was stopped before its
suites ran, so that the end-to-end fixture could be added.

`python3 scripts/apply_amendments.py docs/reports/link-standing/integration-amendments.json
--check`: exit 0 on a copy of the tree with FLUENT-BIT-LINK-PREPARE's
amendments applied. On this base alone, it refuses the amendment of
CURRENT-MONKEY-CONFIGURATION-NOT-LINKED, which that card registers.

## Records

- [`integration-amendments.json`](integration-amendments.json)
- [`gate.json`](gate.json)

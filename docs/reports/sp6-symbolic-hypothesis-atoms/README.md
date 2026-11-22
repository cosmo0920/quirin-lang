# SP6-SYMBOLIC-HYPOTHESIS-ATOMS

User decision 2026-11-20 (verbatim): "we don't want to write down the
sentences on premise clauses. We really dislike this kind of results so we
must write down as condition symbols like |-> or other types of logical
symbols to represent the prerequisites which compose for premise
conditions." User 2026-11-22, on SP5 leaving the cJSON hypotheses as
English: "We need to convert to be symbolized atoms of hypothesis."

Base `38cf960fa` (branch `integrate/sp5-premise-ban`: SP5 merged onto
origin/main `b069dbda8`). Prediction: [PREDICTION.md](PREDICTION.md)
(committed first, `60469abe6`). The card discharges
`SP2-DETACH-HYPOTHESES-TYPED-ATOMS` and extends SP5's ban to hypotheses: no
`resource_hypothesis(NAME, "…")` atom loads anywhere; outside
`docs/reports/` the string form survives only as refusal cases, the frozen
compiler fixture (read through the legacy reader) and the reader's own
refusal text.

## 1. Census

Pattern `resource_hypothesis\([a-z_0-9]+, *\\?"` (the string form, as source
or as a Rust string literal) outside `docs/reports/`, by file, at base and
after; plus what it cannot see: atoms built with `format!` (pattern
`resource_hypothesis\(\{`), opaque declared atoms inside frozen graph JSON a
test reads, and readers of frozen texts.

| File | Role | Base | After | What changed |
|---|---|---|---|---|
| `providers/qcore-provider-librdkafka/contracts/librdkafka/cjson.qv` | the nine `detach_*` atoms of `cJSON_DetachItemViaPointer` (production) | 9 | 0 | typed atoms (§2); the nine guarded prerequisite symbols repeat each formula; binding marker refreshed |
| `crates/quirin-source-analysis/src/cjson_local_binding.rs` | the native checker's standalone finite graph (production code) | 3 | 0 | typed atoms over the bound native domain (§6, `QCore.LocalTypedAtoms`); expected graph repointed to this card's |
| `crates/quirin-source-analysis/src/plugin_source_witnesses.rs` | the plugin fixture checker's standalone graph, built with `format!` (production code) | 4 (`format!`) | 0 | typed atoms over the fixture trace (§6) |
| `crates/qcore-contract/examples/support/contract_lifecycle.qv`, `premise_flow.qv` | example fixtures | 1, 3 | 0 | typed atoms |
| `crates/qcore-contract/tests/contract_lifecycle.rs`, `premise_flow.rs`, `contract_views.rs`, `premise_metadata_bridge.rs` | fixtures | 2, 2, 1, 4 | 0 | typed atoms (the bridge's original atom is now a typed clause, its `statement_literal` the definition source) |
| `crates/qcore-contract/tests/hypothesis.rs` | graph tests | 8 | 5 | typed atoms; the 5 are refusal cases (`QF002`); the v1 wire test keeps a legacy-only v1 graph's exact bytes and shows a v1 graph of opaque declared atoms refused |
| `crates/qcore-contract/tests/plugin_premise_decomposition.rs` | bounds fixture | 2 | 1 | typed atoms; 1 refusal case |
| `crates/qcore-contract/tests/cjson_typed_atoms.rs` (new) | the card's tests | — | 8 | refusal cases on every route, and the recorded sentences shown refused |
| `compiler/tests/premise_formula_operators.rs` | compiler route | 0 | 3 | refusal test (`Q000` naming `QF002`) |
| `compiler/tests/fixtures/contract_inherit_qcore_provider/librdkafka_memory_hand_written.qv` | frozen token fixture | 7 | 7 | unchanged bytes, read only through the legacy reader (its seven `plugin_*` atoms lifted to their typed clauses) |
| `compiler/tests/contract_inherit_qcore_provider.rs` | wrote the frozen string atoms back into the current text before parsing | restore | 0 | no restoration: the fixture is lifted instead |
| `tests/support/legacy_prose_premise.rs` | legacy reader | — | 1 (`format!`) | lifts recorded string atoms, graph atoms and guarded symbols (§4) |
| `tests/support/sp2_premise_checkpoint.rs` | byte restoration | — | — | first writes this card's recorded `cjson.qv` edits back (two digests) |
| frozen `.qv` texts a test replays: `quant-contract-hypotheses/plugin/before/**` (7 `plugin_*` atoms × 166 declarations, 8 libc reading atoms) and `premise-decomposition-cycle1/proposed-cjson.qv` (9) | string atoms | read as is | lifted | read through `lift` |
| frozen graph JSON a test reads: `guarded-contract-adoption/adoption-plan.json`, `premise-guarded-decomposition/applications/{cjson,plugin}-graph.json` and `cjson.json`'s bindings | opaque declared atoms (9 detach, 7 plugin) | read as is | lifted | read through `lift_graph`/`lift_bindings` |
| `docs/reports/sp2-symbolic-librdkafka-premises/bindings/cjson-expected-graph.json` | the expected cJSON graph (opaque atoms) | read by 4 consumers | historical | consumers read this card's `bindings/cjson-expected-graph.json` |

Total by the pattern: 35 matches in 11 files at base (plus 4 `format!`-built
in 1); after, 16 in 4 files tracked before this card plus the new test file,
every one a refusal case or the frozen fixture. No shipped `.qv` states one
(9 at base, 0 after). `crates/qcore-contract/src/caller_frame.rs`'s list of
admitted metadata heads, `providers/quirin-provider-libc/src/lib.rs`'s
belt-and-braces scan and `qcore-audit`'s two line filters name the head
without a statement and are kept.

## 2. The nine typed atoms

Each is the typed rendering (C04.18.9's SP6 block, §3) of its part of the
declaration's premise formula `detach_list_representation`: parameters are
the registered terms that part uses, in their rendered sorts, so each atom
means what those conjuncts mean. As stated in `cjson.qv`:

```text
quant proposition #detach_chain(#child: predicate<obj,obj>, #children: predicate<obj,obj>, #first: predicate<obj,obj>, #live: predicate<obj>, #next: predicate<obj,obj>, #next_reach: predicate<obj,obj>, #owned: predicate<obj,obj>, #parent: obj, #pred: predicate<obj,obj,obj>) = ((((((#live(#parent) && forall (#c: obj) { (#owned(#parent,#c) => #children(#parent,#c)) }) && forall (#c: obj) { (#children(#parent,#c) => (#owned(#parent,#c) && #live(#c))) }) && forall (#c: obj) { (#child(#parent,#c) => #first(#parent,#c)) }) && forall (#c: obj) { (#first(#parent,#c) => #child(#parent,#c)) }) && forall (#b: obj) { (#children(#parent,#b) => forall (#a: obj) { (#pred(#parent,#b,#a) => #next(#a,#b)) }) }) && forall (#c: obj) { (#children(#parent,#c) => !#next_reach(#c,#c)) })
quant proposition #detach_distinct(#nodup: predicate<obj>, #parent: obj) = #nodup(#parent)
quant proposition #detach_empty(#child: predicate<obj,obj>, #children: predicate<obj,obj>, #parent: obj) = ((forall (#c: obj) { !#child(#parent,#c) } => forall (#c: obj) { !#children(#parent,#c) }) && (forall (#c: obj) { !#children(#parent,#c) } => forall (#c: obj) { !#child(#parent,#c) }))
quant proposition #detach_first_prev(#first: predicate<obj,obj>, #last: predicate<obj,obj>, #parent: obj, #prev: predicate<obj,obj>) = forall (#f: obj) { (#first(#parent,#f) => forall (#l: obj) { (#last(#parent,#l) => #prev(#f,#l)) }) }
quant proposition #detach_last_next(#last: predicate<obj,obj>, #next: predicate<obj,obj>, #parent: obj) = forall (#l: obj) { (#last(#parent,#l) => forall (#y: obj) { !#next(#l,#y) }) }
quant proposition #detach_link_targets(#children: predicate<obj,obj>, #live: predicate<obj>, #next: predicate<obj,obj>, #parent: obj, #prev: predicate<obj,obj>) = forall (#c: obj) { (#children(#parent,#c) => forall (#y: obj) { ((#next(#c,#y) || #prev(#c,#y)) => (#children(#parent,#y) && #live(#y))) }) }
quant proposition #detach_no_writer(#concurrent: predicate<fn>, #keeps_links: predicate<obj,run>, #parent: obj, #runs: predicate<fn,run>) = forall (#f: fn) { (#concurrent(#f) => forall (#r: run) { (#runs(#f,#r) => #keeps_links(#parent,#r)) }) }
quant proposition #detach_parent_outside(#children: predicate<obj,obj>, #parent: obj) = !#children(#parent,#parent)
quant proposition #detach_predecessor(#children: predicate<obj,obj>, #parent: obj, #pred: predicate<obj,obj,obj>, #prev: predicate<obj,obj>) = forall (#b: obj) { (#children(#parent,#b) => forall (#a: obj) { (#pred(#parent,#b,#a) => #prev(#b,#a)) }) }
```

**Scoping.** Every old sentence began "For this removed-result declaration".
Each atom is a clause of exactly that declaration (a declaration-local graph
atom, C04.18.1), and `#parent` is the registered term "the object the call's
parent argument designates", so every atom is about this declaration's call
and its `parent` argument, as the sentence said, and about nothing wider.

### Faithfulness tables

SP2 already mapped the premise's sentence to its formula phrase by phrase
(SP2 report §5, `detach_list_representation`); each atom's sentence is one
part of that sentence, so its formula is the matching part, rendered.

**`detach_chain`** — "the live parent represents exactly its owned children by a finite acyclic child and next chain"

| Phrase | Formula part |
|---|---|
| "the live parent" | `#live(#parent)` |
| "represents exactly its owned children" | `forall #c { #owned(#parent,#c) => #children(#parent,#c) }` and `forall #c { #children(#parent,#c) => (#owned(#parent,#c) && #live(#c)) }` (exact custody domain, every child live) |
| "by a finite" | `#children`'s registered meaning: the finite child sequence (Lean `List`) |
| "child and next chain" | `#child(#parent)` is `#first(#parent)` (both inclusions); each child after another is that one's `#next` (`#pred(#parent,#b,#a) => #next(#a,#b)`) |
| "acyclic" | `forall #c { #children(#parent,#c) => !#next_reach(#c,#c) }` |

**`detach_distinct`** — "chain child identities are pairwise distinct": `#nodup(#parent)` (the child sequence names no identity twice).

**`detach_empty`** — "child is NULL precisely when the child and next chain is empty"

| Phrase | Formula part |
|---|---|
| "child is NULL" | `forall #c { !#child(#parent,#c) }` (a pointer member's term is empty when NULL) |
| "precisely when" | both implications (`=>` each way) |
| "the child and next chain is empty" | `forall #c { !#children(#parent,#c) }` |

**`detach_first_prev`** — "the first child's prev is the last child": `forall #f { #first(#parent,#f) => forall #l { #last(#parent,#l) => #prev(#f,#l) } }`.

**`detach_last_next`** — "the last child's next is NULL": `forall #l { #last(#parent,#l) => forall #y { !#next(#l,#y) } }`.

**`detach_link_targets`** — "every non-NULL next or prev target is a live child of that same parent"

| Phrase | Formula part |
|---|---|
| "every … next or prev target" (of a child) | `forall #c { #children(#parent,#c) => forall #y { (#next(#c,#y) \|\| #prev(#c,#y)) => … } }` |
| "non-NULL" | a NULL member designates nothing, so no `#y` |
| "is a live child of that same parent" | `#children(#parent,#y) && #live(#y)` |

**`detach_no_writer`** — "no concurrent writer mutates these links during the call": SP2's `#concurrent |-> #keeps_links(#parent)`, rendered: every run (`#runs`) of every step another thread runs during the call (`#concurrent`) writes neither the parent's child member nor a child's next or prev (`#keeps_links`).

**`detach_parent_outside`** — "the parent is not a child": `!#children(#parent,#parent)`.

**`detach_predecessor`** — "each nonfirst child's prev is its predecessor": `forall #b { #children(#parent,#b) => forall #a { #pred(#parent,#b,#a) => #prev(#b,#a) } }` (a first child has no predecessor, so the quantifier says nothing of it).

No phrase is dropped and no conjunct narrower than the sentence; the Lean
equivalences (§7) make that mechanical. There was no second defensible
formalisation: each part is already proved equal to its
`CjsonRepresentationPremises` definition, the propositions the binding
manifest names.

**Migrated together.** The binding manifest (`quant_binding_migration`, run
on the base text with the ban's two files at base, `private/logs/migration-1.log`,
exit 0, "1 open binding references refreshed"): manifest
`ef40a078…` → [`27c07365…`](bindings/27c0736584d81d659e44dd29eae891ccdc37cfeaf019a57a620fadaa73d910a1.json),
graph `9c460d73…` → `4393a3d0…`, each atom's `atom_sha256`, `quantifiers`
and `conclusion` now the typed definition's (no English); the model and audit
files and their digests are unchanged. The expected graph is
[bindings/cjson-expected-graph.json](bindings/cjson-expected-graph.json)
(graph version 3), read by `cjson_local_binding`, its example,
`hypothesis.rs` and `hypothesis_bindings.rs`; `qcore-audit`'s
`contract_dependencies_report.rs` and `production_symbolic_adoption.rs` read
the new manifest. The guarded adoption plan is replayed through the legacy
reader (§4).

## 3. Vocabulary: the SP6 block

C04.18.9 gains a typed rendering of its terms (specification table, Rust
`RENDERED_VALUE_SORTS` / `RENDERED_TERMS` / `rendered_sort` /
`check_rendering` in `qcore_frontend::premise_formula`, Lean
`QCore.CjsonTypedAtoms.render`). Every term the nine atoms use was already
registered (`#parent`, `#live`, `#owned`, `#children`, `#child`, `#first`,
`#last`, `#pred`, `#next`, `#prev`, `#next_reach`, `#nodup`, `#concurrent`,
`#keeps_links`). Missing was only what `|->` hides:

| Name | G01p sort | Meaning |
|---|---|---|
| `run` | value sort | one run of a callback: what an `eff` term is a property of |
| `#runs` | `predicate<fn,run>` | the runs of a callback, the semantics of `\|->` (`QCore.PremiseFormula.Model.runs`) |

Neither enters the premise `VOCABULARY` (no premise formula names them), so
the premise grammar, decision vectors and Lean `vocab` are unchanged.
`cjson_typed_atoms.rs` `the_nine_atoms_render_registered_terms` checks every
parameter of every atom against the table.

## 4. The ban and where it fires

`QF002` (next free id in SP5's `QF***`; `QF001` the only one used, checked
across the tree): a clause headed `resource_hypothesis`, under any
introducer and with any arguments (the string was the form's only
statement).

* **Structural parser** (`frontend.rs`, through
  `qcore_frontend::premise_formula::hypothesis_clause`): every module on
  every route parses first, so resource v1, bounds v1, model contracts,
  provider packages, plugin views and inclusion elaboration all refuse; on
  the compiler route the user sees `Q000` whose message names `QF002`.
* **Graph reader** (`qcore_contract::hypothesis::declaration`): refuses the
  head by name too (never reached past the parser).
* **Graph validator** (`hypothesis::validate`, hence `decode`, `digest`,
  replay): an `opaque` atom whose ID is not a generated `legacy_atom_*` is
  refused with `QF002`, on the wire as in source.

**The generated legacy atom.** It is the erasure of a named premise (a
C04.18.9 formula's canonical printing, or a literal-argument or rule-owned
record's statement), not a hypothesis anyone states; it keeps its opaque
statement under the existing rules (plain, or a canonical formula when it
holds `\`). A typed atom can never take a legacy ID (G01p refuses
`legacy_`). `ATOM_DECISION_VECTORS` (Rust) / `hypothesisVectors` (Lean)
record both cases.

**Other changes the ban needed.** A typed atom defined twice refuses even
when identical (G01p; string atoms coalesced identical repeats).
`premise_metadata_bridge` (an additive-metadata bridge) reads a typed atom as
an original atom, its `statement_literal` the definition source. Contract
inheritance keeps a `quant proposition` clause literal, as it keeps every
other graph clause (C04.18.1: graph symbols are copied unchanged): a typed
atom is closed (every name a bound parameter), so renaming a parent formal
inside it only renamed its own parameters and broke the graph's identity
(`#parent` and atom IDs equal to a formal name, as `#a` in a fixture, were
renamed; `hypothesis.rs` and `premise_flow.rs` inheritance tests caught it).

**The legacy reader.** `tests/support/legacy_prose_premise.rs` (same file,
same confinement test `the_legacy_prose_reader_is_confined_to_test_code`)
now also lifts each recorded string atom (`migration.json`, `hypotheses`: the
nine `detach_*` typed here, and the seven `plugin_*` and eight libc reading
atoms of `errno.qv` typed by QUANT-CONTRACT-HYPOTHESES, whose frozen
`before` texts tests still replay; each with its retired sentence, typed
clause, formula and graph atom): `lift` (source clauses, and guarded prerequisite
symbols that repeated a sentence), `lift_graph` (opaque graph atoms, graph
version raised to 3) and `lift_bindings` (guarded metadata bindings). An
unrecorded string atom stays a string and is refused.
`sp2_premise_checkpoint` first writes this card's recorded `cjson.qv` edits
back (`migration.json`, `files`: nine atoms, nine guarded symbols, one
comment, the marker; both digests checked), so SP2's exact byte restoration
still holds. No production flag admits a string atom.

## 5. Migrated tests and fixtures

Generic test atoms state `quant proposition #ID(#live: predicate<obj>,
#parent: obj) = #live(#parent);` (a second distinct one `#nodup(#parent)`, a
third `!#children(#parent,#parent)`), registered-term renderings read by no
verdict. Tests of the string form now test its refusal
(`hypothesis.rs`, `plugin_premise_decomposition.rs`,
`cjson_typed_atoms.rs` `a_string_hypothesis_is_refused_by_name_on_every_route`,
`compiler/tests/premise_formula_operators.rs`
`a_string_hypothesis_is_refused_by_name_at_parse`). The frozen replays
(guarded adoption plan, guarded applications, symbolic adoption inventory,
the guarded pin test) check every frozen digest on the restored bytes and
replay on the current module with plan, graphs and bindings lifted.

## 6. Source-local typed atoms

Two checkers build their own bounded graphs; their atoms are typed over the
model each binds (`QCore.LocalTypedAtoms`):

* `cjson_local_binding`: `#finite_entry_equations` (every recorded snapshot
  satisfies the eight `detach_*` atoms other than `detach_no_writer`, as
  predicates over snapshots), `#finite_detach_post` (every post comparison
  holds), `#incomplete_fixed_case_domain` (`exists (#k: case_key)
  { (#recorded_count(#k) != #required_count(#k)) }`: the multisets differ).
* `plugin_source_witnesses`: traces as `#length()` and `#at(i)` over
  integer indices, markers as values: the exact four-marker trace,
  membership as an in-range index, the last marker at `#length() - 1`, and
  the Go implication.

## 7. Lean

`QCore.PremiseFormula.CjsonTypedAtoms` (audit
`AuditPremiseFormulaCjsonTypedAtoms.lean`, 26 theorems): `render M` (each
parameter its term's model component), the nine atoms as
`QCore.QuantProposition.Expr` (bound to `cjson.qv` by
`the_nine_atoms_are_the_lean_definitions`), `*_atom_reads` (what each states
of any model), `chain_atom_iff` … `predecessor_atom_iff` and
`typed_atoms_are_the_definitions` (each atom, in `detachModel e`, is its
`CjsonRepresentationPremises` definition), `detach_formula_is_the_nine_typed_atoms`
(from SP2's `detach_formula_is_the_nine_atoms`), `parent_outside_atom_can_fail`,
`string_is_not_a_hypothesis`, `declared_accepted_iff_typed`,
`hypothesis_vectors_agree` (bound to `ATOM_DECISION_VECTORS`).
`QCore.LocalTypedAtoms` (audit `AuditLocalTypedAtoms.lean`, 9 theorems): the
seven source-local atoms each equivalent to its bound proposition.
Placeholder-free; standard axioms only.

## 8. Semantics

Hypothesis atoms are evidence-only metadata (C04.18.1, C04.18.7): no verdict,
standing, certificate or classifier reads one. The cJSON declaration's
interpreted contract, premises, erasure and flows are unchanged
(`guarded_metadata_bridge::check` compares the loaded contracts and premises
of the plan's before and after texts). Predicted: 0 status moves on corpus,
twelve-home, system and slice ([PREDICTION.md](PREDICTION.md)).

## 9. Locks, reviewed

Order: `providers/contracts.py sync` (`private/logs/sync-1.log`, exit 0
with the expected refusal "stale lock: `cJSON_DetachItemViaPointer`'s
contract text changed since it was pinned"); `quirin-audit pin
--source-root <Fluent Bit at REV> --crate qcore-provider-librdkafka
--function cJSON_DetachItemViaPointer` at `046accf6` and then `66910c10` (a
`--shared --no-checkout` clone of the local Fluent Bit repository under the
worktree's ignored `private/`, `HEAD` set to each revision;
`private/pin-1/pin-*.log`, both exit 0); `contracts.py sync` again
(`sync-2.log`, exit 0). Diff:

* `contracts.lock`: the `cJSON_DetachItemViaPointer` entry's
  `contract_sha256` (`9038573d…` → `7b1a99ac…`) and `pinned_at`; its
  identity keeps the same two sites and the same two revisions. Nothing
  else changes.
* `contract-inventory.lock` and `contracts.json`: `librdkafka/cjson.qv`'s
  digest (`2bb7c253…` → `0fcd9833…`) and the binding manifest reference
  (`ef40a078…` → `27c07365…`); declaration and clause counts unchanged (nine
  `requires resource_hypothesis` clauses became nine `quant proposition`
  clauses).
* `bounds-contracts.lock`, `outer-contracts.lock`,
  `providers/contract-load-path.json`: unchanged.

## 10. Gate

**Gate 1** (`python3 scripts/gate_scope.py --base 38cf960fa --out
private/gate-1` on `fad05f4a6`, the card merged with origin/main
`5977e52a4` at `40d25649c`; full mode, `proof/QCore.lean` changed;
[record](gate/gate-1.json)) exited 1:

| Step | Exit | Counts |
|---|---|---|
| submodules, fmt | 0, 0 | |
| clippy | 101 | one `manual_map` in `premise_formula::check_rendering` |
| rust-suite | 101 | 676 units, 9886 passed, **1 failed**, 21 ignored |
| reevaluation probe, follow-up | 0, 0 | |
| lean (`check_lean_audits.py`) | 0 | 484 audits (both new audits included) |
| python | 0 | 807 tests |
| provider package build, verify (all, libevent) | 0, 0, 0 | |
| provider test | 0 | 174 passed, 1 ignored |
| provider fmt, clippy | 0, 0 | |

The one failure is the recorded host failure
`qcore-audit::libcurl_user_crate_versions::the_libcurl_ranges_follow_the_audited_curlver_header`
(system `curlver.h`), not fixed. The clippy refusal was fixed in the next
commit (`Option::map`, no behaviour change); after it: `cargo fmt --all --
--check` exit 0, `cargo clippy --all-targets -- -D warnings` exit 0
(`private/logs/clippy-2.log`), and `qcore-frontend` (lib) with
`qcore-contract`'s lib and `cjson_typed_atoms` exit 0, 133 passed
(`private/logs/t-after-clippy.log`). Earlier scoped runs: `qcore-contract`
whole exit 0 (`private/logs/t-contract-6.log`); `qcore-contract`,
`qcore-frontend`, `quirin-source-analysis`, `qcore-audit`, `qcore-compiler`
(`private/logs/t-multi-5.log`) only the host failure and one
`production_symbolic_adoption` replay of libc's frozen string atoms, fixed by
recording them (§4). Logs: `private/gate-1/`; console outside the worktree.

**Measurement** (the integrator's): `target/release/quirin-audit
--plugin-path target/release --output <dir> --timeout-ms 36000000 measure
--inputs crates/quirin-audit-plugin-measure/inputs/live.json --inputs-base
/media/Data/GitHub/quirin-lang --side base=baseline:<main baseline> --side
tip=rev:<last code commit> --declared docs/reports/sp6-symbolic-hypothesis-atoms/PREDICTION.md`.

## 11. Follow-ups

* `SP6-GUARDED-SYMBOL-PROSE` (registered): the other eight
  `resource_guarded_symbol_v1` texts of `cJSON_DetachItemViaPointer`
  (`detach`, `environment`, `failure_case`, `invalid_membership`,
  `membership`, `original_list_unchanged`, `remaining_list_valid`,
  `success_case`) and the guarded document's readable text are still English;
  `membership` is a prerequisite symbol. They are not hypothesis atoms (out
  of this card's scope).
* Binding manifests' `environment.variables` descriptions and other
  `FormalBinding` records built in code (`cjson_local_binding`,
  `plugin_source_witnesses`) still describe their environments in English;
  they are not conditions a declaration states. Recorded, not registered.

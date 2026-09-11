# Card `NATIVE-LIBRARY-SOURCE-BINDING` — the source file's own library binding

Wave 23. Follow-on row of
[`docs/reports/wave22-native-library-header/README.md`](../wave22-native-library-header/README.md)
§11: *"binding a manifest from the source file … rather than by symbol name
across the selection. Still needs a registry name or a new form, and still a
constraint-14 census."* Carried unchanged since wave 20.

Design: [`docs/design/NATIVE_LIBRARY_LINKAGE_V1.md`](../../design/NATIVE_LIBRARY_LINKAGE_V1.md)
§4.1. Spec: `docs/QCORE_SPEC_r0_3.md` §L14.5 and the §L21 row for `Q065`–`Q067`.
Grammar: `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` §G02d.3.

**Second deliverable, design-only:**
[`docs/design/STD_HTTP_BINDING_V1.md`](../../design/STD_HTTP_BINDING_V1.md) — the
decision record for how `std.http` should exist. Status *proposed*; it claims no
implementation and names no library as chosen. Summarised in §9 below.

## 1. What was open

Cards `UNSAFE-LINKAGE-ARTIFACT` (wave 19), `NATIVE-LIBRARY-CONTRACT-SLICE`
(wave 20) and `NATIVE-LIBRARY-HEADER` (wave 22) built the manifest: a build's
identity, its ABI premises, its symbols bound to declared contracts, and its
exported header read and compared. What none of them built is a statement *in
the source file* about which build it meant.

The selection is the **builder's** act (`CompileOptions::native_libraries`), and
resolution was by **symbol name across the whole selection**: whichever selected
manifest happened to bind a symbol supplied that symbol's artifact premise. Two
consequences, both silent:

1. a source file's certificate could name a build the file never mentioned;
2. adding a manifest to a build's selection could change what a file's
   declarations resolved to, with nothing in the file to compare against.

## 2. The form, and why this one

**Landed:** `extern library NAME;`, an import-adjacent file-scope declaration.

```qcore
import std.extern;
extern library qcore_buffer_demo;

extern fn qcore_extern_alloc(size: usize) -> ptr<mut, u8>;
…
```

### 2.1 Against the registry-name option

The follow-on row suggested a registry name (`import std.buffer;`) as one
candidate. It was considered and **rejected**, on three grounds:

1. **A registry name is a capability, a binding is an identity.** An `import`
   grants vocabulary and is gated by `Q014`/`Q015`; a binding grants nothing and
   states which build an already-granted vocabulary resolves against. Conflating
   them would mean that importing `std.buffer` for `source.buffer.v1` — a route
   with no extern boundary at all (card `STD-BUFFER`) — silently attached a
   linkage premise the file never asked for.
2. **The registry is closed and the language owns it.** It holds eight names
   today (`qcore_core::module_imports`); a third-party library cannot mint one. A registry-name binding could
   therefore only ever bind libraries *the language ships*, which is the
   opposite of what the manifest exists for — design §1's "a bundled capability
   module … implemented by a native compiled library" is about the builder's
   library, not only about ours.
3. **A registry name carries no identity.** `import std.buffer;` names no
   artifact and no hash, so a certificate could not record which build the file
   bound without going back to the selection — which is the state this card
   exists to leave.

`std.buffer` is besides already taken, by an unrelated source route with its own
`Q051`–`Q057` band.

### 2.2 Why `extern library NAME;`

* **It is the trust act's own vocabulary.** `extern` already introduces the
  declarations whose resolution this decides.
* **It declares no name**, so it is read where `import` is read — at file scope,
  before declarations — rather than entering the namespace name resolution
  walks. A manifest identity is not a nameable thing in the program.
* **It mints no reserved word** (§3).
* **It carries enough identity by resolution rather than by transcription.** The
  file spells the manifest's own `library` field; the machinery supplies the
  manifest key, the artifact identity (`library@hash`), the four ABI premises and
  the header identity, and publishes all of them. Spelling the hash in the source
  was considered and declined: it would be a second place the identity could be
  wrong, and the manifest is already the place it is right. The *resolution* is
  what the premise records, which is what a later selection resolving the same
  name to a different build makes visible.

## 3. Constraint-14 keyword census

**Zero new reserved words.** Both words already exist in the source grammar:

| Word | Already reserved by | Position |
|---|---|---|
| `extern` | `source.extern-call.v1` (`extern fn`), `UNSAFE-LINKAGE` (`unsafe extern fn`) | declaration head |
| `library` | the module-mode header, `module library;` | after `module` |

What is new is the **two-word sequence `extern library` at declaration
position**, which before this card was a hard parse error: the declaration
reader admitted only `fn` after `extern`, so any such file failed at `Q000`
("expected a QCore declaration"). This is the `unsafe extern fn` precedent
exactly — an existing word in front of an existing form — and `library` remains
an ordinary identifier in every other position, pinned by
`library_remains_an_ordinary_identifier_outside_the_binding_position`.

### 3.1 The census, and its one real finding

The corpus and fixture census found **no** occurrence of `extern library` (0
files, all extensions). `library` appears 35 times in `.qcore` files — every one
a prose comment — and `module library;` appears in 14 Rust fixtures, none of them
at a position this form claims.

The finding is not in the spelling but in the **`NAME`**, which is a
builder-chosen library name and therefore an arbitrary identifier appearing in
the token stream. Twenty-two call sites read `SourceProgram::lexical_tokens`.
Most window to a *statement span inside a function body* (or take a span
argument) and are structurally blind to a file-scope binding;
`pipe::check_source_operators` scans the whole stream but reads only operator
symbols, of which the form has none but `;`. **Two scan the whole stream for a
single bare word**, and those are the exposure:

| Reader | Matches on | Exposure |
|---|---|---|
| `qcore_frontend::module_imports::concurrency_span` | `qcore_buffer_new`, `qcore_buffer_read`, `qcore_buffer_write`, `qcore_error_payload`, `spawn`, `join`, and the pairs `task <` / `mutex <` / `mutex guards` / `mutex rank` | a library named one of the six single words would select the concurrency route and be refused for a missing `import std.concurrency;` — **from its own name** |
| `qcore_frontend::source_tasks::has_source_tasks` | `qcore_buffer_new`, `qcore_buffer_read`, `qcore_buffer_write` | the same, for the task route |

The pair forms are unreachable: a binding is `extern library NAME ;`, so `NAME`
is always followed by `;`. Four other whole-stream readers were checked and are
unreachable for the same reason — `source_mutex::has_source_mutex` and
`declares_lock_rank` need a two-word pair, `source_epoch::has_source_epoch` needs
`epoch {`, and `pipe::check_source_operators` reads operator tokens the form has
none of.

**Repair:** `SourceProgram::in_library_binding` records the binding spans and
both readers skip tokens inside them — the same exclusion `concurrency_span`
already applies to a ghost field inside a declared contract, for the same reason:
these tokens are a *binding*, not a use. Pinned by
`a_library_named_after_a_route_selecting_word_does_not_select_that_route` (a
library literally named `spawn`) and
`a_library_named_after_a_task_selecting_intrinsic_does_not_select_that_route`
(named `qcore_buffer_new`). Recorded as a standing constraint in `AGENTS.md`
constraint 14.

## 4. Semantics

### 4.1 Opt-in, and what that means precisely

A file declaring **no** binding gets the *unbound* view: resolution is
selection-wide, exactly as at §L14.2–§L14.4. Its emitted C, its premise list and
its certificate are byte-identical to the pre-card run. This is not a claim about
intent; it is what `the_bound_route_emits_the_same_c_as_the_symbol_name_route`
and `an_unbound_file_publishes_no_binding_premise` measure.

`BoundSelection` distinguishes `modules: None` (said nothing) from
`modules: Some(ms)` (bound `ms`) rather than letting an empty list stand for
both. The second cannot arise — a file that declared a binding at all resolved at
least one — and the two are different statements.

### 4.2 The refusals

| Condition | Code | Disposition |
|---|---|---|
| a library name no selected manifest declares | `Q065` | refuse; the message lists the names the selection *does* declare |
| a name two or more selected manifests declare | `Q065` | refuse as ambiguous — the ground `Q040` refuses a symbol bound twice on: a binding that is not a function has no defined premise, and choosing by selection order would make the premise depend on the order the builder listed manifests |
| one name bound twice in a file | `Q065` | refuse; taking the first would make a repeated line silently ignorable |
| a binding with no manifest selected at all | `Q065` | refuse; the unknown-library arm, whose message says so |
| a symbol this file declares that a manifest it did **not** bind binds | `Q066` | refuse |
| a certificate that lost a binding premise | `Q067` | refuse to publish |

**`Q066` is the load-bearing one, and its disposition is the argument.** The
weaker admissible reading below a narrowed symbol is the file's premise *without*
an artifact — which is **accepted**. So falling back to it would reward a
contradiction with a premise that says less: a file could stop its certificate
naming a build simply by binding a different library. That is exactly `Q041`'s
condition seen from the other side, and it takes `Q041`'s disposition from the
one rule all these levels obey — *withdraw only where the weaker admissible
reading refuses.*

The contracted mirror is the same code at the same condition, emitted **before**
the `Q042` gate it guards, so an unresident contract cannot resolve past the
manifest that claims its symbol.

### 4.3 What is deliberately not refused

A symbol **no** selected manifest binds is untouched: it keeps the bare waiver
premise or its declared contract, byte for byte. Absence of evidence is not
evidence, and the two absences are different — one is a symbol nothing claims,
the other a symbol something claims and this file did not bind. Only the second
is a contradiction. Pinned by `a_symbol_no_selected_manifest_binds_is_not_foreign`.

### 4.4 A binding is checked wherever it is written

A binding is resolved **before route selection**, not only where the extern
route reads it. Without that, a module carrying `extern library NAME;` and no
extern surface would hold a declaration nothing ever looked at, and an
unresolvable one would be silently ignorable — which is the shape this
repository refuses everywhere else.

The check is **guarded on the file having written a binding**, deliberately:
loading the selection unconditionally would let a manifest that does not load
refuse a module that never mentioned one, which would move programs written
before this card. A module that declares a binding is a new program by
construction, so nothing existing travels that branch. Both halves are pinned —
`a_binding_in_a_module_with_no_extern_surface_is_still_resolved` and
`a_module_that_writes_no_binding_never_loads_the_selection`.

### 4.5 File-scope isolation

Resolution is **per input file**, and the *declaring* file's binding decides a
declaration — card `UNSAFE-LINKAGE`'s own rule that declaring a symbol is itself
the trust act, not calling it. The driver checks a *combined* source, so file
identity survives only as `ranges`; `file_index` maps a binding's span and a
declaration's span to their inputs.

Both directions are pinned:

* `one_file_s_binding_does_not_reach_a_sibling_file_s_declaration` — a bound
  entry file and an unbound sibling compile together; the sibling's waived
  declaration resolves selection-wide exactly as before this card, so a binding
  in another compilation input granted it nothing and took nothing away;
* `moving_the_sibling_declaration_into_the_bound_file_refuses` — the same
  program with one declaration moved across the file boundary is `Q066`.

This is constraint 13's discipline applied to a binding.

### 4.6 The premise

`ExternAssumption::SourceBoundLibrary(SourceLibraryBinding)`, one per resolved
binding, carrying the spelled name, the manifest key, the artifact identity and
the ABI premises (and the header identity where the manifest bound one). It is a
field of `ExternProgram`, so `recheck_extern_certificate` re-derives it from the
program alone.

It **stacks** and replaces nothing: the callee premises say what each symbol was
read as, this one says which build the file declared its extern surface to come
from, and neither is derivable from the other. A module that binds a library and
calls nothing it exports still carries it — the binding is a trust act whether or
not a call site consumed it, and that is why it is a program field rather than a
derived fact about the callees.

The identity remains a **stated pin**. This card resolves a *name to a
declaration*: it opens no file and hashes nothing, for the reasons
`crates/qcore-core/src/native_library.rs`'s module documentation gives.

## 5. Diagnostic band

**`Q065`–`Q067`**, the `NativeLibrary` pass's **fourth** range, allocated through
the §L21 process and contiguous with the third (`Q063`–`Q064`).

> **Integrator note.** Other wave-23 cards also allocate. If `Q065`+ is taken by
> a card that ships first, relocate this **unshipped** band per `T07`/`T09`; the
> code-to-condition mapping is stable under a shift. Sites:
> `docs/QCORE_SPEC_r0_3.md` §L21 row and §L14.5,
> `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` §G02d.3,
> `crates/qcore-frontend/src/frontend.rs` (`SemanticPass::NativeLibrary` doc),
> `crates/qcore-verification/src/driver.rs` (four emission sites, three codes),
> `AGENTS.md`'s diagnostics table.

## 6. What changed

| File | Change |
|---|---|
| `crates/qcore-frontend/src/frontend.rs` | the `extern library NAME;` parse, `SourceProgram::library_bindings` / `in_library_binding`, the pass doc |
| `crates/qcore-frontend/src/module_imports.rs` | binding-span exclusion in `concurrency_span` (census §3.1) |
| `crates/qcore-frontend/src/source_tasks.rs` | the same exclusion in `has_source_tasks` |
| `crates/qcore-core/src/native_library.rs` | `SourceLibraryBinding`, `BindingRefusal`, `BoundSelection`, `NativeLibraries::{bind_source, binding_module, library_names}` |
| `crates/qcore-core/src/extern_call.rs` | `ExternProgram::libraries`, `ExternAssumption::SourceBoundLibrary`, `required_source_binding_assumptions`, `certificate_carries_source_bindings` |
| `crates/qcore-frontend/src/source_extern.rs` | the bindings travel into the lowered program |
| `crates/qcore-frontend/src/source_composed.rs` | the same for the composed route |
| `crates/qcore-verification/src/driver.rs` | `bind_source_libraries`, `file_index`, `AdmittedExternSurface`, the `Q065`/`Q066`/`Q067` emission sites, per-file resolution at every manifest lookup, and the pre-route-selection binding check of §4.4 |
| `crates/qcorec/src/main.rs` | the premise's own rendered line |
| `proof/QCore/NativeLibrarySourceBinding.lean`, `proof/AuditNativeLibrarySourceBinding.lean`, `proof/QCore.lean` | the Lean model and its audit |
| Spec, grammars, `NATIVE_LIBRARY_LINKAGE_V1.md`, `AGENTS.md` | §L14.5, the §L21 row, §G02d.3, the "landed since" row, the diagnostics table and constraint 14 |

**Not touched, by construction:** `contract/qcore/` (the corpus pin is
untouched — this card adds no contract module), `providers/`,
`examples/threads`, `examples/events`, the pinned 185-unit artifacts, the CT
manifest, `DEVELOPMENT.md`, `docs/reviews/r0_3/acceptance_matrix.json`, and the
`crates/qcore-c-gateway` exported-header reader owned this wave by
`NATIVE-LIBRARY-HEADER-DECORATION`.

## 7. Tests

### 7.1 `compiler/tests/native_library_source_binding.rs` — 20 tests

*A bound file resolves through its binding:*
`a_bound_file_resolves_its_declared_extern_surface_through_the_named_library`,
`the_binding_premise_names_the_artifact_the_file_bound`,
`the_binding_premise_stacks_and_replaces_no_callee_premise`,
`the_bound_route_emits_the_same_c_as_the_symbol_name_route`.

*An unbound file is unchanged:* `an_unbound_file_publishes_no_binding_premise`,
`an_unbound_file_still_resolves_selection_wide`.

*Everything else refuses:*
`a_binding_naming_a_library_no_selected_manifest_declares_refuses`,
`a_binding_with_no_manifest_selected_at_all_refuses`,
`two_manifests_declaring_one_library_name_make_the_binding_ambiguous`,
`a_repeated_binding_of_one_library_refuses`,
`a_waived_symbol_of_a_manifest_this_file_did_not_bind_refuses`,
`a_contracted_symbol_of_a_manifest_this_file_did_not_bind_refuses`,
`a_symbol_no_selected_manifest_binds_is_not_foreign`.

*A binding is checked wherever it is written:*
`a_binding_in_a_module_with_no_extern_surface_is_still_resolved`,
`a_module_that_writes_no_binding_never_loads_the_selection`.

*File-scope isolation, both ways:*
`one_file_s_binding_does_not_reach_a_sibling_file_s_declaration`,
`moving_the_sibling_declaration_into_the_bound_file_refuses`.

*Constraint-14 census outcome:*
`a_library_named_after_a_route_selecting_word_does_not_select_that_route`,
`a_library_named_after_a_task_selecting_intrinsic_does_not_select_that_route`,
`library_remains_an_ordinary_identifier_outside_the_binding_position`.

### 7.2 `crates/qcore-core/tests/unit/native_library.rs` — 11 new rows

`an_empty_binding_list_is_the_unbound_selection_wide_view`,
`a_binding_resolves_its_own_manifest_and_narrows_away_the_other`,
`a_symbol_narrowed_away_is_refused_rather_than_silently_unresident`,
`a_symbol_no_selected_manifest_binds_is_foreign_to_nobody`,
`a_library_name_no_selected_manifest_declares_refuses_and_names_the_selection`,
`a_binding_against_an_empty_selection_refuses`,
`two_manifests_declaring_one_library_name_refuse_as_ambiguous`,
`a_repeated_binding_of_one_name_refuses`,
`two_bindings_resolve_both_manifests_and_leave_nothing_foreign`,
`a_binding_only_ever_narrows`,
`the_binding_carries_the_manifests_premises_and_header`.

## 8. Lean

`proof/QCore/NativeLibrarySourceBinding.lean`, registered in `proof/QCore.lean`
and audited by `proof/AuditNativeLibrarySourceBinding.lean` (16 theorems). The
vocabulary is imported from `QCore.NativeLibrary` — identity, ABI premises,
header identity — so there is one premise vocabulary and not two that could
drift.

| Group | Theorems |
|---|---|
| the binding is a function | `an_unknown_name_refuses`, `an_ambiguous_name_refuses`, `a_repeated_name_refuses`, `resolution_is_a_function` |
| an unbound file is unchanged | `the_empty_binding_is_unbound`, `an_unbound_view_resolves_selection_wide`, `an_unbound_view_is_never_foreign` |
| narrowing, and its refusal | `a_binding_only_narrows`, **`a_narrowed_symbol_is_refused`**, `a_symbol_no_manifest_binds_is_not_foreign`, `resolving_and_refusing_are_exclusive` |
| no leak across inputs | `a_file_s_view_is_its_own`, `an_unbound_sibling_is_unmoved` |
| the premise names the build | `the_premise_names_the_build`, `the_identity_is_the_declared_one` |
| non-vacuity | `the_route_binds_narrows_and_refuses` |

`a_narrowed_symbol_is_refused` is the one that keeps this card from being a
weakening: *exactly* where a binding removes a resolution, the file is owed a
refusal. With `a_binding_only_narrows` beside it, the pair says a binding can
only ever remove a resolution, and never removes one silently.

Axiom closures: fifteen theorems close over `propext` alone; the non-vacuity row
adds `Classical.choice` and `Quot.sound`. No project axioms, no placeholders.

**Stated boundary of the model.** The source spelling, the parser, the driver's
per-file span arithmetic and the diagnostic attribution are Rust boundaries,
tested there. What the model is about is what a *binding* means: which manifests
a file resolves against, that the resolution is a function, and that narrowing
refuses. That the named artifact is the one the linker used, and that the library
conforms to anything, remain premises — unchanged by this card, which resolves a
name to a declaration and opens no file.

## 9. Second deliverable: `docs/design/STD_HTTP_BINDING_V1.md`

Design-only, status **"proposed — strategy recommendation recorded; final choice
reserved to the user."** It records two candidate strategies for how `std.http`
should exist — (a) native QCore HTTP, (b) a contracted native-library binding of
an existing C HTTP library through the L3 manifest route — and the integrator's
standing recommendation of **(b)**, on five grounds: it is what the linkage
spectrum exists for; the Fluent Bit heritage makes it the same activity this
repository has done for twenty cycles; it is honest about what is proved, where
(a)'s verified fraction would be small and easy to overstate; its failure mode is
a premise violation against a named artifact rather than a bug in this
repository; and it composes with `source.event.v1`. The honest counter-argument —
that (b) makes the language's most visible standard module a trusted declaration
about somebody else's code — is recorded rather than suppressed.

It inventories the prerequisites that have landed (the manifest, the contract
slice, the exported-header level, **this card's source binding**, `std.buffer`,
`std.event` v1, and the `std.string` amendment as design-only) and the named gaps
that remain (`STD-BUFFER-EXTERN`, `STD-BUFFER-ABI-EXTENT`, `STRING-SLICE-ABI`,
`NATIVE-LIBRARY-HEADER-DECORATION`, `-INCLUDE`, `-FNPTR`, `EXTERN-MODES-SENTINEL`,
struct transport, concurrent extern calls, and library selection itself).

**No library is named as chosen.** §6 records selection *criteria* only — closed
allocator custody, a small stable surface, readable headers, scalar-and-pointer
signatures, no unwinding and no threads of its own, a bundlable licence, an
auditable build, prior art in the corpus — and requires a selection card to
publish its comparison against them, including the criteria the chosen library
fails.

## 10. Gate

| Step | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` | **3,611 passed, 0 failed, 8 ignored** across 198 test binaries; process exit 0 |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | **exit 0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | **exit 0** |
| `python3 scripts/check_lean_audits.py` | **96/96 audits passed** (95 → 96: `AuditNativeLibrarySourceBinding.lean`) |

The workspace run was measured under **heavy machine load** — other worktrees'
audit runs, load average 40–57 throughout — and constraint 16's five
load-sensitive rows all passed anyway, so none needed the isolated re-run that
rule provides for. Exit 0 is the honest figure; nothing was retried.

The `--no-fail-fast` flag is on the run because the first attempt stopped at
`libevent_submodule_matches_the_audited_revision`: the worktree's submodule was
not initialised. `git submodule update --init --recursive` fixed it, and the
recorded run is after that, on the pinned revision
`d82464a277d0f42703702c4dfd9af6af38595a83`. That was a checkout-setup finding,
not a change of this card's.

**Measured, not asserted:** the `quirin` premise line of §6 was run end to end —
`quirin check --native-library MANIFEST FILE` on a file that binds a library and
declares a waived callee prints the binding premise beside the waiver's, in the
same prose vocabulary the two existing arms use.

## 11. Named follow-ons

| Name | What is open |
|---|---|
| `SOURCE-BINDING-HASH-PIN` | letting the source spell `NAME@HASH` and refusing a resolution to a different build. Deliberately not v1: it would be a second place the identity could be wrong, and the value is only in *pinning against drift*, which needs a story for how a source file is updated when a library is rebuilt |
| `SOURCE-BINDING-REQUIRED` | a mode in which an unbound file is refused rather than resolved selection-wide. It would make the card's guarantee total instead of opt-in, and it would break every program written before it; it needs a migration argument this card does not have |
| `SOURCE-BINDING-UNUSED` | refusing a binding no declaration of the file resolves through — the analogue of `ManifestRefusal::NoSymbols`. Declined here because a binding that resolves nothing still records a true statement (this file was checked against this build), and refusing it would punish a file for a declaration it has not written yet |
| `SOURCE-BINDING-CLI-EXAMPLE` | the premise line added to `crates/qcorec/src/main.rs` **is** reachable through `quirin` — the CLI selects manifests (`--native-library`), only the extern *contract* environment is missing (`UNSAFE-LINKAGE-CLI`) — so a `quirin check` of a file binding a library and declaring waived callees prints it. What is open is a *published example* of that invocation, held to `EXAMPLES-SUITE`'s standard; this card ships the rendering and the fixtures, not an example |
| `L21-DUPLICATE-ROWS` | **found, not caused, and not repaired here.** `docs/QCORE_SPEC_r0_3.md` §L21 carries the `Q050` row twice and the `Q063`–`Q064` row twice, an integration artifact predating this card. The table is integrator surface and the duplication is byte-identical in each pair, so nothing is ambiguous; it should be deduplicated by whoever owns the next §L21 reconciliation |

The `std.http` follow-ons (`STD-HTTP-LIBRARY-SELECTION`,
`STD-HTTP-CONTRACT-SLICE`, `STD-HTTP-REGISTRY-NAME`, `STD-HTTP-NATIVE-HARNESS`,
`STD-HTTP-EVENT-COMPOSE`, `STD-HTTP-NATIVE-ALTERNATIVE`) are listed in
`STD_HTTP_BINDING_V1.md` §8 and are **design-only entries**: none is startable
until §5's gaps close.

## 12. Integrator reconciliation

1. **Diagnostic band `Q065`–`Q067`** — see the note in §5.
2. **`docs/reviews/r0_3/acceptance_matrix.json`** is integrator surface and is
   not edited here. `NATIVE-LIBRARY-SOURCE-BINDING` should be retired from
   `named_followups`; the five §11 follow-ons and the six `std.http` ones should
   be entered. The derived `lean_audits` row grows by one
   (`AuditNativeLibrarySourceBinding`).
3. **`AGENTS.md`** — this card edited two places and no headline number: the
   diagnostics-namespace table (one new row) and constraint 14 (the census
   outcome, which is a standing "do not remove this exclusion" rule).
   `DEVELOPMENT.md`'s increment paragraph is the integrator's.
4. **Corpus-neutral by construction.** No contract module, no manifest fixture
   and no `.qcore` corpus file was added or edited, so
   `compiler/tests/qcore_contract_corpus.rs`'s pins are untouched and the
   185-unit artifacts are unmoved. The two `docs/design` documents this card
   edits are its own and `NATIVE_LIBRARY_LINKAGE_V1.md`'s "landed since" table.

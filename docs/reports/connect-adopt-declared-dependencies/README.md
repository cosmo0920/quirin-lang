# CONNECT-ADOPT-DECLARED-DEPENDENCIES: census of verdict reliances

Round 40, base `1ecaf97e9`, branch `r40-adopt-dependencies`. User decision of
2026-11-26: "Every reliance", "which is related to validate whether Safe or
Unsafe". Later the same day the user added a linkage syntax: `premises { link {
quant #link CRATE::FUNCTION } ... }`, `quant #link CRATE::FUNCTION`, and a
module-level `#include quirin_provider_libc::free as quant #link`.

This commit holds the census only. Nothing is applied: no contract, lock,
manifest, grammar or Lean file changes. The card stops here because of
decisions D1 to D3 and the open grammar points G1 to G7 below.
The brief and the user addition both require a hand-back with options for
these.

## The rule, as a formula (draft, unchecked)

The draft adds three CX terms:

* `cx_reads(#b,#g,#c,#h)`: declaration `g` of bundle `b` reads declaration
  `h` of bundle `c`.
* `cx_decides(#b,#g,#c,#h)`: that reading can discharge a `Safe` obligation
  or validate an `Unsafe` violation.
* `cx_links(#b,#g,#c,#h)`: `g` states a link to `c::h`.

`cx_relies` is the conjunction of the first two. None of these terms is
registered yet, so the premise-formula validator has not checked the
formulas. Checking them comes after the decisions.

* `cx_relies_is_a_deciding_read`:
  `forall (#b: cx_bundle) { forall (#g: cx_fn) { forall (#c: cx_bundle) { forall (#h: cx_fn) { (#cx_relies(#b,#g,#c,#h) => (#cx_reads(#b,#g,#c,#h) && #cx_decides(#b,#g,#c,#h))) } } } }`
  and its converse `cx_deciding_read_relies`.
* `cx_reliance_is_linked`:
  `forall (#b: cx_bundle) { forall (#g: cx_fn) { forall (#c: cx_bundle) { forall (#h: cx_fn) { ((#cx_relies(#b,#g,#c,#h) && !#b == #c) => #cx_links(#b,#g,#c,#h)) } } } }`.
  The enforcement check fails on every counterexample.
* Converse refusals:
  * `cx_undeciding_read_not_required`:
    `(#cx_reads(..) && !#cx_decides(..)) => !#cx_relies(..)`.
    A reading that can only yield `Unknown` or `Pending` is not required.
  * `cx_same_bundle_not_required`: `#b == #c => !#cx_relies(..)` for the
    purpose of this rule.
  * `cx_co_use_is_not_reliance`:
    `(#cx_co_used(..) && !#cx_reads(..)) => !#cx_relies(..)`.
    Two declarations used by one function row are not a reliance.
  * The existing `cx_refuse_libc_target_elsewhere`. A libc target always
    names `quirin-provider-libc`.

## Census

The census covers 14 crates: 11 on the shipped load path and 3 on the user
load path (libcurl, onigmo, openssl). It counts 189 modules and 1,149 distinct
declared function names per crate. Every edge is in [census.json](census.json), which
[scripts/census.py](scripts/census.py) writes. Each entry gives the clause or
reading, the target's lock identity, the Cargo edge and whether the edge can
be satisfied under C15.2. The body readings come from
[scripts/callees.py](scripts/callees.py) over
`plugins/in_storage_backlog/sb.c` and librdkafka 2.15.0 `src/rd.h`. Both
files were read from the recorded linked source,
`private/current-linked-556460e31/run/fluent-bit-clang-source`.

| Dependent crate | Target crate | Edges | Declarations | Kind | Can C15.2 satisfy it? |
|---|---|---:|---:|---|---|
| fluent-bit | libc | 6 | 6 | 5 inherit, 1 body check (`memset`) | yes (Cargo edge) |
| librdkafka | libc | 5 | 5 | 3 inherit, 2 rule-37 checked specifications | yes |
| libevent | libc | 3 | 3 | inherit | yes |
| fluent-bit | qcore-provider-libevent | 5 | 5 | 4 body checks (`mk_list_add`/`mk_list_del`), 1 call frame | **no** |
| fluent-bit | qcore-provider-chunkio | 11 | 3 | body checks (`cio_chunk_*`) | **no** |
| **total** | | **30** | **22** | 11 inherit, 18 body check, 1 frame | 16 not satisfiable |

The census finds three reliance kinds:

1. **Inherit** (11 edges): `contract inherit libc::F`. The child elaborates
   to the parent's clauses, so every discharge through the child reads the
   parent's clause. The five `#include` edges (4 modules) are all covered by
   these 11 edges, because every included module supplies an inherited
   parent.
2. **Body-checked specification** (18 edges).
   * `resource_body_checked` on 8 storage-backlog declarations. A
     specification is withdrawn unless the gateway's body reading discharges
     it, and that reading applies each callee's declaration.
   * Rule 37 `rd_calloc` and `rd_strdup`. Their nonnull results are
     established through `libc::calloc` and `libc::strdup`.

   Every one of these is a reading that can decide a verdict. The edge is
   required only if the specification can discharge. A specification whose
   body also calls an undeclared function (for example `cio_chunk_lock`,
   `cio_error_get` or `mk_list_init`) may never discharge. If so, its edges
   are only `Unknown`-yielding readings and are not required. Before the
   re-pin, each specification's established status at `1ecaf97e9` must be
   confirmed from a measured report.
3. **Call frame of another crate's function** (1 declaration, 2 clauses):
   `flb_config_set_program_name` holds `quant cell mk_list_del:
   flb_input_instance |-> #untouched` and the same clause for
   `flb_output_instance`. `legacy_cell_frames` applies these frames at every
   `mk_list_del` call. A `Safe` across that call rests on the frame and on
   `libevent::mk_list_del` together.

Readings that are not required, each with its ground:

* **libevent observation records.** The libevent crate names libc symbols
  in strings: `callback_task_handoff.qv` `effect_send`, `effect_perror` and
  `effect_errno_location`; the `timeout_teardown.qv` `path_calls`; and the
  `native_platform_matrix.qv` `backend == "poll"`. These name a C symbol and
  read no libc clause. Follow-up CONNECT-LIBC-EFFECT-CLAIMS: decide whether
  task and defer effect claims about libc callees belong in
  `quirin-provider-libc`.
* **Gateway built-ins.** `__assert_fail` and the other no-return names in
  `null_deref/ast.rs` are built into the gateway and are not declarations.
  The rule-37 failure arm uses them.
* **Shared words.** `close`, `time`, `read`, `write`, `access`, `listen`,
  `shutdown` and `exit` appear as clause vocabulary or parameter names. They
  only spell libc function names.
* **Type and member names.** Struct types and member paths, such as the
  mpack types in cmetrics and ctraces and the fluent-bit types, are not
  declarations.
* **Co-use in one function row.** A report's per-row
  `contract_dependencies.declarations` lists both declarations, but
  `cx_co_use_is_not_reliance` excludes this.
* **Same-crate readings.**
  * The storage-backlog bodies call `flb_calloc` and `flb_free`. These are
    fluent-bit declarations that reach libc only through the inherit edges.
  * `rd_realloc` is written out and is not body-checked. libc declares no
    `realloc`.
* **Duplicate `mk_list_del`.** `chunkio/list_unlink.qv` also declares
  `mk_list_del`, but that module is not in the resource view. The resource
  route reads `monkey/intrusive_list.qv`, so the target is
  `libevent::mk_list_del`.
* **User crates.** onigmo, libcurl and openssl read no other crate's
  declaration. Their checkers compare declarations with documentation
  snapshots. The QUIRIN-AUDIT-CONNECT trial's `onig_new -> libc::malloc` was
  a scratch demonstration, not a reliance.

**Re-pin count.** 22 declarations, under the stop bound of about 150. If D1
and D2 below go as recommended, 11 are re-pinned: `rd_calloc`, `rd_strdup`,
the 8 storage-backlog specifications and `flb_config_set_program_name`.

## Decisions the census raises

**D1. 16 edges target executable provider crates.** `qcore-provider-chunkio`
and `qcore-provider-libevent` are binary-only crates (`src/main.rs`, no
library target), so no crate can name them as a Cargo dependency. Cargo also
runs the other way: chunkio and libevent depend on `quirin-provider-shared`,
which depends on fluent-bit. C15.2 satisfies a dependency only when the
target crate is a declared dependency (`cx_declared_crate`). Declaring these
16 edges today would therefore hold 8 fluent-bit declarations `Unknown`
(`cx_undeclared_crate_is_unknown`), and verdicts would move. That contradicts
the brief's "no move" prediction.

* (a) **Recommended.** Separate a contract-level crate graph from Cargo. A
  dependency is satisfied when its target crate is on the same load path,
  and the manifest's contract-dependency graph is acyclic (the existing
  `QI014` cycle refusal). This changes the `cx_declared_crate` reading, its
  formulas and the Lean model. A load path without chunkio or libevent still
  holds the dependents `Unknown`.
* (b) Move the monkey `mk_list_*` and chunkio `cio_*` resource declarations
  into library crates that fluent-bit depends on. This is a large move, and
  their lock entries change owner.
* (c) Declare the edges and accept the holds. Verdicts move.
* (d) Phase the work. Apply now only what can be satisfied and is not an
  inherit edge (3 edges: `rd_calloc`, `rd_strdup`, `memset`), and register
  the rest as follow-ups.

**D2. Inheriting declarations cannot carry a clause.** Extension clauses
after `contract inherit P` are refused (`QI002`, C15), so the 11 inherit
edges cannot state a dependency line or a `link`.

* (a) **Recommended.** The inherit directive *is* the link. Elaboration
  records it as the C15.2 dependency, `contracts.py sync` records the
  parent's identity in the manifest, and the enforcement check counts the
  directive. None of the 11 children is re-pinned. A parent change already
  changes the child's elaborated text, which is a pin mismatch (`Pending`).
* (b) Admit `quant #link` after `contract inherit P` and lift `QI002` for
  that one clause.
* (c) Write the 11 contracts out. This drops inheritance and is rejected.

**D3. Body-checked edges are conditional.** The 18 body-checked edges are
required only for specifications that can discharge. Before the re-pin,
read each specification's status at `1ecaf97e9` from a measured report, and
drop the edges of any specification that never discharges.

## The linkage syntax: grammar fit (user addition, 2026-11-26)

The proposal: a `link` elaborates to exactly one C15.2 dependency, with the
same resolution, holds and cycle refusal. The target's identity
(`contract_sha256`, plus `since` for a versioned crate) is written by
`providers/contracts.py sync` into the manifest's `contract_dependencies`
entry, not into the text. `requires resource_contract_dependency(...)` would
be retired as a written form and kept only as the manifest's elaborated
record, so there is one source of truth.

The existing grammar does not settle these points:

* **G1. Clause envelope.** G01 `contract = "contract", "{", { clause }, "}"`
  admits only clauses that start with `requires`, `ensures` or `quant`, and
  each ends with `;`. A `premises { ... }` block is a new clause form, so G01,
  the structural parser and G05's `resource_clause` must all be extended.
  Under constraint 14 (no reserved word), `premises` and `link` would be
  read only in clause-head position. No contract uses either word as a clause
  head today.
* **G2. Terminator.** A trailing `.` is ambiguous. Clause bodies are
  balanced token regions in which `.` builds member paths, so
  `quant #link libc::free.` followed by `quant` lexes as `free.quant`.
  Use `;`. For blocks, there are two options:
  * (a) **Recommended.** Follow the `condition_block` precedent: entries end
    with `;` and the block ends `};`.
  * (b) Close the block with `}` and no `;`, as the user's example does.
* **G3. `#link` and `link` already appear inside formulas.**
  `cfl/intrusive_list.qv` (`#r072`, `#r136`) binds `#link: link`, with
  `link` as a sort. A clause-head `quant #link` can be told apart by
  position, because the next token is `IDENT ::` rather than `=`, `in` or
  `:`. G05g's `#outer` is the precedent for a reserved `#` word after
  `quant`. Recommendation: reserve `#link` only in clause-head position. A
  condition named `link` would be refused, and none exists.
* **G4. Spelling of `CRATE`.** Three spellings are possible:
  * `libc::free`: the derived namespace, already used by
    `contract inherit` and C15.2's `NS::NAME`, and unique per load path
    (`QI017`).
  * `"quirin-provider-libc"`: the package string used by `#include`.
  * `quirin_provider_libc::free`: the user's top-level form, used nowhere
    today.

  Recommendation: the namespace in all three forms. The manifest records the
  package and namespace beside it.
* **G5. Meaning of the module-level form.** The `#include` scanner admits
  only one string operand
  (`crates/qcore-frontend/src/contract_inclusion.rs`, `QI001`), so
  `#include NS::F as quant #link;` is a new directive form. It can be told
  apart lexically (an identifier versus a string). There are two readings:
  * (a) **Recommended.** It makes `F` linkable by bare name in the module
    and links nothing on its own. The per-declaration `quant #link F;` or
    `link { ... }` entries remain what elaboration and the enforcement check
    count. This keeps "exactly the reliances a Safe or Unsafe rests on"
    exact.
  * (b) It links every declaration in the module. This hides which
    declaration relies on what, and it adds links the rule does not require.
    Rejected.
* **G6. What `premises { ... }` means for the clauses inside it.**
  * (a) **Recommended.** A pure grouping: a clause inside it reads exactly
    as it would at contract level. The block is optional, and existing
    premises are not moved. The braces are part of the `c-tokens-v1`
    normalised text, so moving clauses would re-pin every declaration.
  * (b) A block that is required for premises, which would re-pin the whole
    corpus.

  A `link` inside a `[[local]]` condition block is refused, because links
  are unconditional.
* **G7. Inheritance.** Inheritance copies named premises verbatim, but
  C15.2 states that it does not copy a dependency clause. Combined with D2
  (a), the recommendation is that links are not inherited and the inherit
  directive is the child's link.

The other checks raise no issue:

* Q038 is not triggered, because `::` already lexes inside a `requires`
  clause, as `resource_contract_dependency(..., libc::malloc, ...)` shows.
* Digest normalisation is unaffected. A link is part of its declaration's
  text, so adding one changes the digest, which the re-pin expects.

## `linkage_contract_v1` (user addition, 2026-11-26)

The proposal: the link forms become a versioned vocabulary,
`linkage_contract_v1`, beside `resource_contract_v1`, `bounds_contract_v1` and
`outer_contract_v1`. These points need decisions:

* **L1. Where the marker goes.**
  * (a) **Recommended: on each declaration.** `requires
    linkage_contract_v1;` sits beside `requires resource_contract_v1;` on
    every declaration that states a `quant #link` or a `link { ... }` entry.
    A declaration with a link and no marker rejects the module (total or
    nothing). So does a marker with no link.
  * (b) On the module.

  The reliance rule works one declaration at a time, and the marker is part
  of the declaration's pinned text, so (a) keeps each re-pin exactly the
  declarations that rely on another crate. A module-level
  `#include NS::F as quant #link;` would need a module-level selector. Under
  G5 (a) the include only names `F` for the module, so it is admitted only
  when at least one declaration in the module carries the marker and links
  `F`. Otherwise it is an unused import, which is refused.
* **L2. Spec and grammar homes.** C15.2 would become the
  `linkage_contract_v1` section: forms, elaboration, identity in the
  manifest, resolution and holds. The grammar would gain a new section, G05l,
  beside G05g.

  For diagnostics, the `QI` band was opened as `QI001`–`QI020` and all 20
  codes are used. Options:
  * (a) Extend `QI` from `QI021`.
  * (b) **Recommended.** Open a loader-owned `QL` namespace (unused today;
    the precedents are `QI`, `QF` and `QP`). A linkage form is not an
    inclusion, and C15.2's present refusals report through `QI014`.

  Today's C15.2 holds are reported as `dependency-unresolved` and
  `dependency-changed`. They would keep those reasons.
* **L3. Versioning.** v1 fixes the three forms and their elaboration to one
  C15.2 dependency each. A new form, a new key or a changed reading is a v2
  marker, never a widening of v1.
* **L4. Composition and chain.** Each manifest `contract_dependencies` entry,
  each `measure compose` dependency record and each side's `dependency_graph`
  in `measure chain` gains `vocabulary: "linkage_contract_v1"`. A link's
  version then becomes part of its identity, and a version change refuses
  the chain link (`cx_refuse_dependency_graph`). The record schemas
  (`quirin-contract-composition`, `quirin-measure-chain`) and `qcore-schemas`
  each take a version bump.
* **L5. Lean.** `QCore.MeasureConnect` gains a theorem that the elaboration
  of a module's `linkage_contract_v1` links is exactly its C15.2 dependency
  set. `held_dependency_holds_dependents` then covers links without change.
* **L6. The legacy written form.** `requires
  resource_contract_dependency(...)`, from QUIRIN-AUDIT-CONNECT, no shipped
  contract uses it.
  * (a) **Recommended.** Retire it as a written form. It stays the
    elaborated record that the manifest carries, which keeps one source of
    truth.
  * (b) Keep it as an alternative spelling. Rejected, because it would give
    two sources of truth.

## Next steps once the decisions are made

PREDICTION.md comes next: no move with D1 (a) and D2 (a), and the chain
link refused by `cx_refuse_dependency_graph`. Then:

1. Register the terms and formulas, and extend `QCore.MeasureConnect` with
   the rule theorem and the link-to-dependency elaboration.
2. Extend G01, G05 and C15.2 with the `link` forms and implement them.
3. Add the fixtures and the enforcement check in `contracts.py check`.
4. Make the 11 edits and re-pin them, run `sync`, and write the
   integration amendments.

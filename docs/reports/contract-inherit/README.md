# CONTRACT-INHERIT: contract inclusion and contract inheritance

Card CONTRACT-INHERIT, 2026-09-24, on base `fa7ab464`. The card adds two
forms to the Quirin contract language, so that a declaration can take its
contract from another declared contract instead of restating it:

```qcore
module contract;
#include "quirin-provider-libc:libc/string.qcore";

extern fn flb_memcpy(dst: ptr<mut, u8>, src: ptr<const, u8>, n: usize) -> ptr<mut, u8>
contract inherit libc::memcpy;
```

No existing contract module uses either form, and this card adopts them in no
real contract. A later card does that; see [remaining work](#10-remaining-work).
The feature ships with its own fixture crates, tests, Lean model and parity
evidence.

**Design history.** The design was written and committed before any
implementation (`94438552`). The integrator's first amendment (cross-crate
publication and the load path) arrived before code was written and is
§1.3. The user's second amendment arrived during implementation: it requires
included parents to be written with a crate namespace (`libc::memcpy`) and
restricts unqualified parents to the module's own declarations. It is folded
into §1.2–§1.4 and adds `QI017`–`QI020`. The committed pre-implementation
text remains in `94438552`.

## Headline

* **Syntax.** `#include "CRATE:MODULE";` header directives, and
  `contract inherit [NS::]NAME;` as an `extern fn` contract. **No reserved
  word is added**: `include` and `inherit` are contextual, and both existing
  tokens are reused (`#`, `::`).
* **Semantics.** The contract loader elaborates the module before any
  consumer parses it. It resolves includes through a published load path,
  looks the parent up in a namespace-scoped way, checks signatures, and copies
  the parent's clauses under a positional renaming that refuses capture. The
  resource-v1 loader then requires the child's loaded record to **equal** the
  parent's. An inherited contract is exactly as strong as its parent, a trusted
  assumption like a hand-written one, and never a proof.
* **Provenance.** Every environment that used inclusion carries, for each
  selected module, its chains (child → parent → … → the declaration whose
  contract is written out). Each link records crate, crate version, module
  path and SHA-256. It also retains every text the elaboration read.
  `check_source_assumptions` replays the elaboration from those texts alone.
  A change to a parent's text changes the environment's identity.
* **Diagnostics.** A loader-owned `QI001`–`QI020` namespace. **No `Q` route
  code is allocated.** Every code is exercised by a negative control.
* **Lean.** `proof/QCore/ContractInherit.lean`: 38 theorems, audited by
  `AuditContractInherit.lean`, placeholder-free, no project axioms.
* **Parity.** No contract source, manifest, corpus index or pin changed. An
  environment without inclusion serializes byte for byte as before. Every
  corpus module elaborates to itself. The fresh arbiter run's verdict-bearing
  diagnostics are **byte-identical to the W43 baseline**.
* **Follow-ups 282 → 285.**

## 1. Design

### 1.1 Where the feature lives: a directive layer resolved before parsing

Inclusion and inheritance are **resolved by the contract loader against an
explicit load path, and elaborated into ordinary contract text before any
consumer parses the module**. The structural parser does not resolve them.

The alternative was to parse both forms into the shared source AST. It was
measured and rejected. Twenty-five consumer sites across `qcore-frontend`,
`qcore-contract`, `qcore-verification` and the compiler read
`SourceDeclaration::function_contract`. Several of them treat an extern
declaration without a contract block as "no clauses". An inheriting
declaration that reached one of them unresolved would fail open. Keeping the
forms out of the AST gives the opposite guarantee by construction: an
unresolved inclusion **does not parse** for any consumer, so every consumer
that has not been wired refuses it. The structural parser now refuses both
forms with a message that names the contract loader. It is still `Q000`.

The elaborated module is exactly the text a person would have written by
hand: the parent's clauses under the positional renaming (§1.4). The unchanged
resource-v1 loader loads it, and the environment retains it as its trusted
source assumption. That is the literal sense in which an inherited contract is
a trusted assumption *of the same strength* as a hand-written one: downstream,
there is no difference to observe. Provenance travels beside it (§3).

The `#` spelling fits this design. Like a C preprocessor directive, it is
consumed before parsing and never reaches the grammar.

### 1.2 Concrete syntax

```ebnf
contract_module   = "module", "contract", ";", {include_directive},
                    declaration, {declaration} ;
include_directive = "#", "include", STRING, ";" ;
(* STRING content: CRATE ":" MODULE
   CRATE  = [a-z0-9][a-z0-9_-]*              the Cargo package name
   MODULE = SEGMENT {"/" SEGMENT} ".qcore",  SEGMENT = [a-z0-9_-]+ *)
inheriting_fn     = "extern", "fn", IDENT, "(", [parameter, {",", parameter}], ")",
                    "->", TYPE, "contract", "inherit", parent, ";" ;
parent            = [NAMESPACE, "::"], IDENT ;   (* one qualifier level *)
```

* **Includes are header directives.** They are admitted only in a
  `module contract;` module, after the header and before the first
  declaration. A directive anywhere else is refused (`QI001`).
* **Includes carry the full Cargo package name.** An example is
  `"quirin-provider-libc:libc/string.qcore"`. The module half is the logical
  path the `CONTRACTS` indexes and `contract/corpus.json` already use. `:`
  splits the two halves unambiguously: Cargo package names cannot contain it,
  and the contract-bundle asset rule already rejects it inside a path. The
  module charset is exactly `providers/contracts.py`'s `[a-z0-9_/-]+\.qcore`,
  with nonempty segments. Absolute paths, `.`, `..`, backslashes, escapes and a
  second `:` are therefore unrepresentable, not merely checked (`QI003`).
* **Parents carry a namespace, not a package name.** `libc::memcpy`
  qualifies the parent by the **namespace derived from** the crate name
  (§1.3). The two spellings name the same crate in two roles. The include
  string is what the dependency rule checks. The qualifier is what the parent
  lookup scopes by.
* **One qualifier level.** `a::b::c` is refused (`QI001`).
* **No extension clauses** in this version. `contract inherit libc::memcpy { … };`
  is refused with its own code (`QI002`); §1.5 gives the reason.
* `inherit` is admitted on a plain `extern fn` only (`QI001` otherwise). An
  `unsafe extern fn` has no contract to carry. A defined `fn` has a body that
  the contract would have to be checked against, which needs a different
  consumer. The child's signature is written in full, with no effects row or
  usage annotation, because inheritance transfers the contract block, never
  the signature.

### 1.3 Cross-crate publication, the load path and namespaces

The integrator's amendment asked for the rules by which a provider crate
publishes contracts to crates that build on it. It asked for the design to
weigh a `LOADPATH` and an in-source `#path` directive against a recommended
default. The card adopts the default, with one refinement: own-crate includes
also require publication.

| Candidate | Resolution | Verdict |
|---|---|---|
| `LOADPATH` (an ordered directory list from the environment or command line) | the first directory that contains the name | **Rejected.** First-match is shadowing. Which text a module includes would depend on the invoking environment, not on anything a reviewer can read in the module or the crate graph. Two checkouts with different search orders would load different contracts under one spelling. Constraint 11's source-identity rule (two spellings are two selections, never normalised together) points the other way. |
| In-source `#path "dir";` | a module redirects its own resolution | **Rejected.** It lets a contract point its resolution at arbitrary files, which rule (4) of the amendment forbids. It would also recreate the `#[path]` coupling that QUIRIN-PROVIDER-SHARED just removed (36 attributes → 15): a cross-crate source include with no Cargo edge, invisible to the dependency graph. |
| **Derived load path** (integrator default) | the include names a crate, which must be the includer's own crate or one of its declared dependencies; the module must be in that crate's published set | **Adopted.** |

The model is `qcore_contract::contract_inclusion::ContractLoadPath`, a set of
`PublishedContractCrate { name, namespace, version, dependencies, modules, published }`.
`ContractLoadPath::new` validates it.

1. **Publication is explicit.** `published` is a declared subset of the crate's
   `modules`. Only a published module can be included, **from any crate,
   including its own**. Because own-crate includes also need publication,
   `published` is the one list a reviewer reads to learn what can be
   inherited from. An unpublished or absent module is refused (`QI005`).
2. **The load path is derived from declared dependencies.** A module in crate
   `A` may include from `A` and from each crate in `A.dependencies`, and from
   nothing else (`QI004`). Visibility is not transitive. If `fluent-bit`
   depends on `libc` and `app` depends on `fluent-bit`, `app` cannot include
   `libc` modules unless it declares `libc` too. This mirrors Cargo. A crate
   with no dependencies, which `quirin-provider-libc` is today, can include only
   its own published modules (Lean `no_dependency_includes_nothing_foreign`).
3. **Namespaces are derived and recorded** (user amendment). Every crate on a
   load path must be named `quirin-provider-NAME`. Its namespace is `NAME`
   with `-` read as `_`: `quirin-provider-libc` → `libc` and
   `quirin-provider-fluent-bit` → `fluent_bit`, and likewise `cfl`, `mpack`,
   `msgpack` and `shared`. The crate records its namespace, and the load path
   refuses a recorded namespace that differs from the derived one. It also
   refuses a crate whose name lacks the prefix or is not identifier-shaped
   after it, and two crates that derive the same namespace, such as
   `quirin-provider-fluent-bit` and `quirin-provider-fluent_bit` (`QI017`).
   A namespace names the crate that **owns** a contract, never a bundle.
   Nothing is re-exported, so a contract that `quirin-provider-shared` bundles
   is still `libc::malloc`, never `shared::malloc`.
4. **No search order, no shadowing.** Every include names its crate, so include
   resolution never searches, and there is no first-match rule to get wrong.
5. **No in-source path redirection.** The grammar cannot spell a path outside
   the named crate's `contracts/` tree (§1.2). The load path is data the caller
   builds; a module cannot extend it.
6. **Cycles are refused at two levels.** Crate dependency cycles are refused
   when the load path is built (`QI014`). Cargo already forbids them; the
   check keeps fixture load paths honest. Module include cycles, which can
   exist inside one crate, are refused during elaboration (`QI006`).

**Real-crate wiring is not in this card.** The following are unchanged
because a concurrent card edits them: the real crates' `CONTRACTS` lists and
Cargo manifests, the resource manifest, `contract/corpus.json` and the corpus
pins. Four pieces of wiring are registered as `CONTRACT-INHERIT-ADOPTION`:

* a recorded `NAMESPACE` and a `PUBLISHED` list beside each `CONTRACTS`;
* versions and dependency edges read from Cargo metadata into a load-path
  manifest that core consumers read as data (core crates may not link provider
  crates);
* a manifest schema that selects `crate:module` references;
* package builds that write include pins.

The five `qcore-provider-*` artifact crates would be refused by the prefix
rule (`QI017`). This is recorded in the same follow-up as a decision for the
adoption card.

### 1.4 Semantics of `contract inherit`

Let the child be `extern fn f(c₁: T₁, …, cₙ: Tₙ) -> R contract inherit P;` in
module `M`.

1. **Scope** (user amendment). An **unqualified** `NAME` is looked up among
   `M`'s own declarations only. A **qualified** `NS::NAME` is looked up among
   the modules `M` includes **directly** from the crate whose namespace is
   `NS`. Included modules' own includes are never visible. There is no
   implicit re-export, which matches G01's "implicit/transitive imports are not
   implemented".
2. **Lookup refusals.**
   * An unqualified name that no own declaration has, but that an included
     module declares, is refused. The refusal suggests the qualified spelling,
     for example "write `contract inherit libc::memcpy`" (`QI018`).
   * A namespace that names no directly included crate is refused (`QI019`).
     This covers an unknown namespace, a crate on the load path that this
     module does not include, and a crate reachable only through another
     module's includes.
   * No match in the scope is refused (`QI008`).
   * Two or more matches in the scope are refused, never resolved by order
     (`QI009`). After the amendment, this means two own declarations with one
     name, or two published modules of one crate that both declare the name.
   * A child named like its qualified parent, such as
     `extern fn memcpy(...) contract inherit libc::memcpy;`, is refused
     (`QI020`). Both would be one function with two contracts in one
     environment. The card does not register a follow-up for it: a wrapper
     should carry its own name, and the parent's owner already declares the
     original.
3. **The parent must be inheritable** (`QI015`). It must be a plain `extern fn`
   (not `unsafe`) whose declaration is exactly
   `extern fn p(params) -> R' contract { clauses };`, with no effects row,
   usage annotation or other signature syntax. The module that declares it
   must parse as a `module contract;` module. A parent that is itself
   `contract inherit …;` is inheritable, because it is elaborated first (§1.6).
4. **Signature agreement** (`QI010`). The child and parent have the same
   number of parameters, the same type at each position (compared as token
   sequences), and the same result type. Parameter *names* may differ, because
   binding is **by position**.
5. **Positional renaming.** Let the parent parameters be `P = [p₁ … pₙ]` and
   the child parameters `C = [c₁ … cₙ]`. The renaming σ maps the word `pᵢ` to
   `cᵢ` and every other word to itself. The child's contract block is the
   parent's clause list with σ applied to every word token of every clause
   expression. Clause order, clause kinds, exit selectors and all
   non-parameter tokens are preserved (Lean `elaboration_exact`).
6. **Capture is refused** (`QI012`). A child parameter name might occur in the
   parent's clauses as a word that is *not* a parent parameter, for example a
   child parameter named `nullable`. σ would capture it, so the declaration is
   refused. A parameter name that occurs as a selector word is refused for the
   same reason.
7. **The consumer's own reading is checked** (`QI012`). The renaming is sound
   for any consumer that resolves a clause word by looking it up in the
   parameter list (Lean `rename_preserves_meaning`). The resource-v1 loader
   instead reads some positions structurally: the nullability argument is
   compared against the words `nullable` and `nonnull` without a lookup. So a
   parent parameter that happens to be *named* `nullable` is renamed where v1
   reads a keyword. The v1 loader therefore loads both the parent declaration
   and the elaborated child, and requires the loaded records to be **equal**:
   custody modes, the result mode, the literal-argument premises (by parameter
   index), the variadic tail and the ABI projection. Any difference refuses.
   Lean `positional_reading_can_diverge` exhibits the case that makes this
   check necessary, and the negative control `qi012_…` drives it in Rust.

**Never weaker, by equality.** Admission requires the child's loaded contract
to equal the parent's. The child therefore has exactly the parent's
obligations and guarantees under the consumer that loaded it: it is neither
weaker nor stronger. Lean `inherited_is_not_weaker` states the non-weakening
direction for the name-resolving reading, and `consumerGate_sound` states it
for the gate.

### 1.5 Extension clauses: refused in this version

`contract inherit libc::memcpy { requires resource_literal_argument(n, 0); };`
is refused (`QI002`). Whether an added clause is sound depends on the
consumer, not on the syntax. Two v1 cases show the problem:

* A literal-argument premise only withdraws call sites, so adding one would be
  sound.
* An added `ensures normal` mode is a *duplicate mode* for a parameter the
  parent already covers. That is not a strengthening or a weakening but a
  contradiction, and v1 has no order on modes under which "stronger" is even
  defined.

A sound admission rule therefore has to be stated per consumer and per clause
family, with its own proof. It is registered as
`CONTRACT-INHERIT-EXTENSION-CLAUSES` rather than admitted as a partial rule.

### 1.6 Transitivity and termination

Chains are admitted. For `f inherit g`, where `g` is itself inherited, `g` is
resolved first.

* **Across modules.** An included module is elaborated completely before the
  includer sees it, so its inheriting declarations are already concrete. A
  chain can only descend the include graph, and a visiting stack refuses
  cycles in that graph (`QI006`).
* **Inside one module.** Own-module parents are resolved on demand, with an
  in-progress set over declarations. Revisiting a declaration is an
  inheritance cycle (`QI011`), including self-inheritance.

Both walks carry a visiting set over a finite node set, so each terminates.
The Lean model uses an explicit fuel bound and treats exhaustion as a refusal,
so it is total-or-refuse by construction. It proves that any cycle through the
start is refused, whatever the fuel (`walk_refuses_cycle`,
`self_inheritance_refused`).

### 1.7 Diagnostics: the `QI` namespace

The contract loader's refusals were never route codes. They were loader error
strings. The route-code space `Q000`–`Q099` is exhausted and iceboxed
(`Q-NAMESPACE-DECISION`), and the brief required a new code per refusal. This
card therefore opens a **loader-owned namespace**, `QI001`–`QI020`, on the
`QP001`–`QP016` precedent: a separate namespace owned by a component that is
not a route. It allocates **no `Q` route code** and does not settle the
iceboxed decision; the integrator may overrule the spelling. Every code starts
the loader's error string, so existing `Result<_, String>` callers carry it
unchanged. The codes are one Rust enum (`InclusionCode`), so a rename is
mechanical.

| Code | Condition | Negative control |
|---|---|---|
| `QI001` | malformed form: a directive outside a contract module's header, a non-string operand, `contract inherit` without one parent and `;`, `a::b::c`, `inherit` on anything but a plain `extern fn` | `qi001_…` (4 cases) + scanner unit tests |
| `QI002` | clauses after `contract inherit PARENT` | `qi002_…` |
| `QI003` | a target that is not `crate:module` in the admitted charset | `qi003_…` (3) + 14 scanner cases |
| `QI004` | the crate is not on the load path, or is neither the includer's crate nor a declared dependency | `qi004_…` (3, incl. libc including nothing foreign) |
| `QI005` | the module is not published by that crate (own crate included) | `qi005_…` (3) |
| `QI006` | include cycle | `qi006_…` (two-module and self) |
| `QI007` | one target included twice | `qi007_…` |
| `QI008` | no declaration in the parent's scope | `qi008_…` (qualified, unqualified) |
| `QI009` | two or more declarations in the parent's scope | `qi009_…` |
| `QI010` | arity, a parameter type or the result type differs | `qi010_…` (3) |
| `QI011` | inheritance cycle | `qi011_…` (two-step, self) |
| `QI012` | capturing renaming, or the consumer reads child and parent differently | `qi012_…` (capture, v1 divergence, non-v1 parent) |
| `QI013` | inclusion syntax at an entry point without a load path | `qi013_…` (4) |
| `QI014` | invalid load path: duplicate crate, malformed module path or version, published-not-owned, absent or self dependency, dependency cycle | `qi014_…` (7) |
| `QI015` | the parent is not inheritable | `qi015_…` (unsafe, effects row, library module) |
| `QI016` | retained evidence does not replay | `tampered_or_orphaned_evidence_is_refused_on_recheck` (5) |
| `QI017` | a crate not named `quirin-provider-*`, a recorded namespace that is not the derived one, or a namespace shared by two crates | `qi017_…` (3) |
| `QI018` | an unqualified parent that only an included module declares (the qualified spelling is suggested) | `qi018_…` |
| `QI019` | a namespace that names no directly included crate | `qi019_…` (unknown, not included, not re-exported) |
| `QI020` | a child named like its qualified parent | `qi020_…` |

The structural parser also gets two more specific parse-error **messages**
(`#include` at top level, and `contract inherit`), both still `Q000`. A
message is not a code (constraint 1). The namespace is documented in
`docs/agents/diagnostics.md` and in §L21 of the language specification.

## 2. Keyword evidence (constraint 14)

**Zero reserved words are added.**

* `include` is contextual: it is read only immediately after a `#` at a
  contract module's top level.
* `inherit` is contextual: it is read only immediately after the word
  `contract` in a declaration's contract position, where the structural
  parser admitted only `{` before this card.
* The namespace qualifier reuses the existing `ColonColon` token.

Neither word enters `qcore_frontend::frontend::is_reserved_source_word`. So
`let inherit = 5;` is still a program, and so is a parameter named `include`
(unit test `a_module_without_either_form_is_empty_and_unaffected`). This
follows the constraint's precedent that contextual words stay contextual
(`#unroll`, `lend`, `error` after `log`).

The census was run, not assumed.
[`scripts/keyword_census.py`](scripts/keyword_census.py) scans every tracked
Quirin source, and every tracked file that could embed one, on base
`fa7ab464` ([census-base.json](census-base.json)):

| Measurement | Count |
|---|---:|
| Tracked Quirin sources (`*.qcore`, `*.qc`) | 571 |
| — of which archived report probes | 174 |
| `inherit` as a word in any Quirin source | **0** |
| `#` in code position (comments stripped) in any Quirin source | **0** |
| `#include` in any Quirin source | **0** |
| `contract` followed by anything other than `{` in any Quirin source | **0** |
| Embedding files scanned (`.rs`, `.py`, `.json`, `.el`, `.lean`) | 2,481 |
| `inherit` in an embedding file | 15, all English prose in comments or reports |
| `#include` in an embedding file that also contains `module contract;` | 5, all C preprocessor lines or prose about C headers |

**No frozen report's pinned diagnostic can change.** This card changes the
outcome only for inputs that contain `#include` at top level or
`contract inherit`. The census finds none in any Quirin source, archived probe
or embedded program. Before this card, such an input was a `Q000` parse error
("expected a QCore declaration" or "function contract requires a block"). It
is still a `Q000` parse error for every consumer except the contract loader;
only the wording is new. The `record` precedent (an archived probe pinned to
`Q000`) does not recur. The census after the card ([census-after.json](census-after.json))
differs from the base only by this card's own fixtures and documentation:
578 Quirin sources (the seven fixtures added, 174 archived probes unchanged); every new `inherit`, `#`, `#include` and non-block `contract` tail is in the two inheriting fixtures, `fluent_bit/wrappers.qcore` and `app/chain.qcore`; and every new embedding-file occurrence is in a file this card added or edited.

## 3. Trust and provenance

An inherited contract is a **trusted assumption of the same strength as a
hand-written one**, not a proof. The elaborated text is what the environment
retains as its source assumption, so every consumer that rederives facts from
the retained texts sees an ordinary module. These consumers are ABI
projections, literal-argument premises, variadic tails and the extern bridge's
declared signatures. Loading the parent elsewhere is not evidence for the
child. The parent itself is only a declaration.

**Provenance format.** `ResourceContracts` gains one field,
`inclusion_evidence: BTreeMap<module key, ContractInclusionEvidence>`,
serialized as `inclusion_evidence` only when nonempty:

```text
ContractInclusionEvidence {
  module:    ContractModuleIdentity { crate_name, crate_version, module, sha256 },
  source:    the selected module's text as written (directives included),
  included:  { "crate:module" → { identity: ContractModuleIdentity, source } },  // transitive
  inherited: { child function → [ ContractInheritanceLink { declaration, module: ContractModuleIdentity } ] }
}
```

Each chain starts at the immediate parent and ends at the declaration whose
contract is written out. For example, the fixture `app_release` records
`fluent_bit/wrappers.qcore#flb_sds_destroy` →
`fluent_bit/wrappers.qcore#flb_free` → `libc/heap.qcore#free`, and each link
carries its crate, crate version and module digest.

* **Identity.** The field is part of the serialized environment, so it enters
  `resource_contracts_sha256` in the audit trust-context identity. A
  comment-only change to a parent leaves the modes and the elaborated text
  unchanged, but it changes the parent's digest, so it changes the identity
  (`a_change_to_the_parent_text_changes_the_childs_contract_identity`, and
  `crates/qcore-audit/tests/contract_inherit_identity.rs`).
* **Replay.** `check_source_assumptions` replays every elaboration from the
  retained texts alone, with no load path or file. It resolves each include by
  exact lookup in `included`. It then requires three things: that the replay
  reproduces the retained elaborated text byte for byte and the recorded
  chains, that every retained module is reached, and that every inherited
  declaration still passes the v1 agreement check. Otherwise it refuses with
  `QI016`. The replay does not re-check the publication rule, because the load
  path is not retained; the retained identities record the result of that
  check.
* **Published results.** A certificate carries the whole `ResourceContracts`.
  An audit report's assumptions block carries the same provenance under
  `contract_inclusion`, but only when it is nonempty, so every existing report
  is unchanged. No audit route can load an inherited contract yet, because the
  resource manifest selects file paths (`CONTRACT-INHERIT-ADOPTION`).
* **Packages.** A package whose contracts include another module must pin every
  module it reaches in its inventory's `includes` object: crate, version,
  module and SHA-256. It must also install the texts under
  `includes/<crate>/<module>`. `providers/package.py verify` refuses an
  unpinned directive, a changed or missing parent, a transitively reached
  parent that is not pinned, an unused pin, a stray `includes/` tree and a
  malformed directive (`scripts/tests/test_package_contract_includes.py`, 8
  tests). The pins sit in the inventory the executable embeds, so they are
  bound to the executable.

## 4. Lean correspondence

`proof/QCore/ContractInherit.lean` (registered in `QCore.lean`, audited by
`proof/AuditContractInherit.lean`, 38 theorems). Words and names are modelled
as natural numbers (interned identifiers). As constraint 28 states, the
correspondence with the Rust is maintained by review and is not an extraction
theorem.

| Rule | Rust | Lean |
|---|---|---|
| unique crate lookup; duplicates never resolve by order | `ContractLoadPath::new` (`QI014`) | `findCrate_sound`, `findCrate_duplicate_refused` |
| include only from own crate or a declared dependency, only published modules | `LoadPathResolver::resolve` (`QI004`, `QI005`) | `resolveInclude_sound`, `unpublished_is_refused`, `no_dependency_includes_nothing_foreign` |
| namespace derived from the crate name, prefix required, no `-` | `derive_contract_namespace` | `deriveNamespace_requires_prefix`, `deriveNamespace_has_no_dash`, `dashToUnderscore_ne_dash` |
| recorded = derived, namespaces distinct ⇒ a namespace names one crate | `ContractLoadPath::new` (`QI017`) | `namespacesValid_records_derived`, `namespace_names_one_crate`, `nodup_map_injective` |
| include and inheritance walks are total-or-refuse; every cycle is refused | `Elaborator::elaborate` visiting stack (`QI006`), `Resolution::resolve` in-progress set (`QI011`) | `walk`, `allOk_each`, `walk_visited`, `walk_refuses_reach_into_visiting`, `walk_refuses_cycle`, `walk_ok_is_acyclic`, `self_inheritance_refused` |
| unqualified parent: own declarations only; included-only is refused with the qualified spelling | `Resolution::resolve` (`QI018`, `QI008`, `QI009`) | `lookup_own_sound`, `unqualified_included_only_refused` |
| qualified parent: directly included modules of that namespace's crate; ambiguity and unknown namespaces refused | `Resolution::resolve` (`QI019`, `QI008`, `QI009`) | `lookup_qualified_sound`, `lookup_qualified_ambiguous`, `lookup_unknown_namespace`, `unique_sound`, `unique_ambiguous` |
| extension clauses and same-name children refused | scanner (`QI002`), `Resolution::resolve` (`QI020`) | `extension_refused`, `same_name_refused` |
| positional substitution yields exactly the parent's clauses under the renaming, with agreeing signatures | `rename_clauses`, `signatures_agree` (`QI010`) | `elaboration_exact` |
| the renaming preserves a name-resolving reading under the capture condition; chains compose | `rename_clauses` capture check (`QI012`) | `indexOf?_some`, `indexOf?_none`, `indexOf?_nodup`, `rename_word_meaning`, `rename_preserves_meaning`, `captures_each`, `rename_clauses_preserve_meaning`, `transitive_inheritance_preserves_meaning` |
| an inherited contract is never weaker than its parent | admission ⇒ equality | `inherited_is_not_weaker` |
| a structural reading can diverge, so the consumer gate is necessary, and it admits only equal readings | `inherited_declarations_agree` (`QI012`) | `positional_reading_can_diverge`, `consumerGate_sound` |

The model does not cover the following. The byte-level elaboration (blanking
directives, splicing clause blocks) and evidence replay are tested in Rust
only. The Rust namespace derivation additionally requires an identifier shape,
a refinement that only refuses more. The Lean `deriveNamespace` is stated over
`List Char` and proved generally; it is not evaluated on the five real names,
and the Rust test `recorded_namespaces_equal_the_derived_ones` does that. The
Lean walk is fuel-bounded, while the Rust walks terminate by their visiting
sets over finite node sets. The Lean refusal on fuel exhaustion is the
fail-closed image of that, and it is not a completeness theorem for acyclic
graphs.

## 5. Parity

No existing contract uses the feature, so every existing contract must load to
an identical result.

* **Sources untouched.** `git diff --stat fa7ab464 -- '*.qcore'` lists only
  this card's seven new fixture files under
  `crates/qcore-contract/tests/fixtures/contract_inherit/`. Not one byte
  changed in any of the 96 corpus modules, `contract/corpus.json`,
  `cycle19-resource-contracts.json`, any `CONTRACTS` list or the corpus pins
  (`compiler/tests/qcore_contract_corpus.rs` is unedited and passes).
* **Every corpus module elaborates to itself.** The test
  `compiler/tests/contract_inherit_parity.rs` puts every corpus module that
  uses no inclusion (all 96 today) into one load-path crate. It then requires
  each module to parse, to elaborate to itself byte for byte, and to carry no
  evidence. The count is not pinned, because a later card may adopt the
  feature.
* **Serialization unchanged.** An environment without inclusion serializes
  with exactly the two pre-card fields (`functions`, `source_assumptions`), so
  every certificate and trust-context digest over it is unchanged (tests in
  `qcore-contract` and `qcore-audit`).
* **Guard inert on the corpus.** `QI013` fires only on inclusion syntax, and the
  census and the parity test show that no corpus module has any.
* **Arbiter.** `docs/reports/w43-return-expression/scripts/run_audit.py` ran twice in this
worktree, each time with a fresh release build of
`cycle19_fluent_bit_libraries_audit`:

* **BEFORE** on base `fa7ab464` ([before-run.json](arbiter/before-run.json),
  exit 0).
* **AFTER** on `1442847b`, the tree the gate measured
  ([after-run.json](arbiter/after-run.json), exit 0).

| Artifact | AFTER vs BEFORE (same checkout) | AFTER vs W43 baseline |
|---|---|---|
| diagnostics (verdict-bearing) | **byte-identical** | **byte-identical** (`f07ba977…`) |
| chunk-contracts | byte-identical | byte-identical |
| source (`monkey-source.json`) | byte-identical | byte-identical |
| contracts | byte-identical | differs by path only: equal after the root alias and the crate-path rewrite |
| source-effects | byte-identical | differs, pre-existing at base: 53 leaves |

Records: [before-vs-after.json](arbiter/before-vs-after.json),
[after-vs-w43.json](arbiter/after-vs-w43.json) (`baseline_identity.py
--root-alias`, exit 0, `verdict_bearing_identical: true`) and
[nonverdict-parity.json](arbiter/nonverdict-parity.json).

* **contracts.** [`scripts/nonverdict_parity.py`](scripts/nonverdict_parity.py)
  applies the root alias and the QUIRIN-PROVIDER-SHARED crate-path rewrite
  (`providers/quirin-provider-*/contracts/` → `providers/shared/contracts/`).
  The result equals the W43 archive exactly. All 33 selected source texts are
  identical by key. That difference is the provider-shared move, not this
  card.
* **source-effects.** Against W43, 53 leaves differ. The differing key names
  are `build_digest`, `configuration_digest`, `task_digest`, `paths`,
  `coverage`, `kind`, `max_operations`, `obligation_id`,
  `observed_runs_only`, `schedules_explored` and
  `qcore:boolean-field-import:v1`. QUIRIN-PROVIDER-SHARED measured 41 leaves,
  all checkout-path digests. The extra leaves and key names were not
  attributed here, because they are present **identically in BEFORE**, and
  AFTER equals BEFORE byte for byte. Whatever moved them landed between the
  W43 baseline and `fa7ab464` (the Monkey callback, provider-shared and
  Windows portability merges), not in this card. This card moves **no**
  artifact byte.

AFTER vs BEFORE is the direct measure of this card, and it is byte-identical
in all five artifacts.

## 6. Positive fixtures

The fixture crates are in `crates/qcore-contract/tests/fixtures/contract_inherit/`.
The load path has four crates: `quirin-provider-libc` and `quirin-provider-cfl`
have no dependencies, `quirin-provider-fluent-bit` depends on both, and
`quirin-provider-app` depends on fluent-bit only.

* `fluent_bit/wrappers.qcore` inherits six contracts:
  * a `memcpy`-shaped contract (`libc::memcpy`);
  * a `free`-shaped contract (`libc::free`);
  * a reallocation (`libc::realloc`, which carries an index-bearing
    reallocated result);
  * a variadic tail (`libc::printf`);
  * a literal-argument premise with an owner-indexed stored mode
    (`cfl::cfl_table_put`);
  * an own-module chain (`flb_sds_destroy` → `flb_free` → `libc::free`).

  The parameter names deliberately differ from the parents'. The environment
  equals the one loaded from `hand_written.qcore`, the same contracts written
  out by hand, in every rederived fact: functions, ABI signatures, premises and
  tails. No parent function leaks into the environment.
* `app/chain.qcore` inherits across three crates through direct dependencies
  only (`app_copy` → `fluent_bit::flb_memcpy` → `libc::memcpy`).
* Selecting a parent module beside its child loads both, with equal contracts.

## 7. Negative controls

Every `QI` code has at least one control that drives it, listed in §1.7:
`crates/qcore-contract/tests/contract_inherit.rs` (29 tests), the scanner unit
tests in `crates/qcore-frontend/tests/unit/contract_inclusion.rs` (5 tests) and
the package tests. Each control was checked against its message, not only its
code prefix: a review run printed every refusal. One case is a
partial-environment control. A good module selected beside a refused one
returns nothing (`a_refused_module_leaves_no_partial_environment`).

## 8. Validation

Measured in this worktree (`agent-afca7a363739adbfc`). The core gate ran on
`1442847b`. Later commits change the report, ledger, status, increment record
and report scripts only. [`run_gate.py`](../w43-return-expression/scripts/run_gate.py)
captured every step's exit to a file, never through a pipe
(`CARGO_BUILD_JOBS=4`, `RUST_TEST_THREADS=2`). Logs are worktree-private under
`measure/`. Clang was recorded in this checkout ([clang-version.txt](clang-version.txt)).

| Command | Exit | Result | Log |
|---|---|---|---|
| `git submodule update --init --recursive` | 0 | | `measure/gate/submodules.log` |
| `clang --version` | 0 | Ubuntu clang 21.1.8 | `measure/gate/clang.log` |
| `cargo test --workspace --no-fail-fast` | 0 | **5,176 passed / 0 failed / 11 ignored**, 275 summaries | `measure/gate/test.log` |
| `cargo fmt --all -- --check` | 0 | clean | `measure/gate/fmt.log` |
| `cargo clippy --all-targets -- -D warnings` | 0 | clean | `measure/gate/clippy.log` |
| `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | | `measure/gate/probe.log` |
| `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | | `measure/gate/followup.log` |
| `python3 scripts/check_lean_audits.py --jobs 4` | 0 | **168** audits (+1, `AuditContractInherit`) | `measure/gate/lean.log` |
| `python3 -m unittest discover -s scripts/tests` (before the matrix regeneration) | 1 | 325 tests, 2 failures: the two committed-matrix consistency tests, which are expected red until the regeneration below | `measure/python-pre.log` |
| `python3 docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/contract-inherit/gate.json` | 0 | 285 follow-ups, 168 Lean audits | `measure/build-matrix.log` |
| `python3 -m unittest discover -s scripts/tests` (after the regeneration) | 0 | **325** tests (+8, `test_package_contract_includes`) | `measure/python-final.log` |
| `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline --no-fail-fast` | 0 | **109 passed / 0 failed / 0 ignored**, 39 summaries | `measure/provider-gate/test.log` |
| `cargo fmt --manifest-path providers/Cargo.toml --all -- --check` | 0 | clean | `measure/provider-gate/fmt.log` |
| `cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings` | 0 | clean | `measure/provider-gate/clippy.log` |
| `python3 providers/contracts.py check` | 0 | inventories unchanged | `measure/provider-gate/contracts-check.log` |
| `python3 providers/package.py build --output measure/provider-gate/packages` | 0 | 5 packages | `measure/provider-gate/package-build.log` |
| `python3 providers/package.py verify …/<id>` (×5) | 0 ×5 | | `measure/provider-gate/verify-<id>.log` |
| `python3 -m unittest discover -s providers/tests` | 0 | 5 tests | `measure/provider-gate/package-python.log` |
| arbiter BEFORE / AFTER (`run_audit.py`) | 0 / 0 | §5 | `measure/arbiter-before`, `measure/arbiter-after` |
| `python3 scripts/build_docs_index.py` | 0 | index regenerated | `measure/docs-index.log` |

**Where the counts come from.**

* **Core Rust.** QUIRIN-PROVIDER-SHARED recorded 5,117 / 0 / 12 at
  `aebd76a5`, and the Monkey and Windows-portability merges moved the suite
  again before `fa7ab464`. This card did not re-measure its base, so the delta
  to 5,176 is not attributed test by test. The card's own new tests are:
  * 29 in `qcore-contract/tests/contract_inherit.rs`;
  * 5 frontend scanner unit tests;
  * 2 in `qcore-audit/tests/contract_inherit_identity.rs`;
  * 1 in `compiler/tests/contract_inherit_parity.rs`.

  That is 37 in total.
* **Python.** The suite gained this card's 8 package-include tests.
* **Provider tests.** 109 equals the QUIRIN-PROVIDER-SHARED count; no provider
  crate changed.
* **Load-sensitive rows.** No `P02-FLAKE` row failed in either full run, so no
  isolated rerun was needed.

**Package identities.** Five things hold for every package:

* All five contract inventories (`contracts_manifest_sha256`) are identical to
  QUIRIN-PROVIDER-SHARED's recorded after-build.
* `Cargo.lock` is unchanged.
* No inventory gained an `includes` field.
* No package installs an `includes/` tree.
* `package.py verify`, with the new include rule, exits 0 for all five.

Executable, manifest and archive digests differ from that record. That record
came from a different checkout, and this card does not claim cross-checkout
reproducibility, as that report itself did not. The providers also link
`qcore-schemas`, which gained the `inclusion_evidence` field. A same-checkout
base package build was not run, so the executable delta is **not attributed**
here. No identity is pinned by any test.

## 9. Documentation changes

* Contract specification: new **C15**.
* Implemented grammars: new **G05a**, plus a sentence in G01 that separates
  `import` from `#include`.
* Language specification §L21: the `QI***` row.
* `docs/agents/diagnostics.md`: the `QI` namespace section. The page was
  already 32,931 bytes, over its stated 32 KiB cap, before this card; the
  section adds about 1.2 KB. The cap overrun is pre-existing and is disclosed
  here, not repaired.
* `docs/agents/providers.md`: the package include-pin amendment.
* Increment record, STATUS follow-up count, report index.

## 10. Remaining work

Registered (follow-ups 282 → 285, `dimensions.named_followups` = entry count):

* **`CONTRACT-INHERIT-ADOPTION`**: the real-crate wiring, which covers
  namespace and published lists beside `CONTRACTS`, a load-path manifest from
  Cargo metadata, a `crate:module` manifest schema, package pin building, the
  Rust `ContractBundle` accepting `includes`, and the prefix decision for the
  five `qcore-provider-*` crates. It also covers adopting the feature in at
  least one real contract, with an arbiter run.
* **`CONTRACT-INHERIT-EXTENSION-CLAUSES`**: the refused extension clauses (§1.5).
* **`CONTRACT-INHERIT-NON-V1-CONSUMERS`**: only the resource-v1 loader has a
  reading-equality gate. Observation, lifecycle and source-bound interpreters
  refuse inherited contracts, which is fail-closed, until each gets its own.

Not registered: the same-name child (`QI020`, §1.4). It stays refused, and
nothing needs it.

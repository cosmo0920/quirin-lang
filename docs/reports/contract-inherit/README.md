# CONTRACT-INHERIT: contract inclusion and contract inheritance

Card CONTRACT-INHERIT, 2026-09-24, on base `fa7ab464`. The card adds two
forms to the Quirin contract language so that a declaration can take its
contract from another declared contract instead of restating it:

```qcore
module contract;
#include "quirin-provider-libc:libc/string.qcore";

extern fn flb_memcpy(dst: ptr<mut, u8>, src: ptr<const, u8>, n: usize) -> ptr<mut, u8>
contract inherit memcpy;
```

No existing contract module uses either form, and this card does not adopt
them in any real contract (a later card does; see [remaining work](#remaining-work)).
The feature ships with its own fixtures, tests, Lean model and parity
evidence.

The design was fixed in this section before implementation. It was then
amended once by the integrator's brief (cross-crate publication and load
path, §1.3), before any code was written.

## 1. Design

### 1.1 Where the feature lives: a directive layer resolved before parsing

Inclusion and inheritance are **resolved by the contract loader against an
explicit load path and elaborated into ordinary contract text before any
consumer parses the module**. The structural parser does not resolve them.

The alternative, parsing `#include` and `contract inherit` into the shared
source AST, was measured and rejected. Twenty-five consumer sites across
`qcore-frontend`, `qcore-contract`, `qcore-verification` and the compiler read
`SourceDeclaration::function_contract`. Several of them treat an extern
declaration without a contract block as "no clauses", and one (the
observation interpreter) would evaluate an empty postcondition list as
vacuously true. An inheriting declaration reaching one of them unresolved
would be the fail-open direction. Keeping the forms out of the AST gives the
opposite guarantee by construction: an unresolved inclusion **does not parse**
for any consumer, so it is refused everywhere the elaborator has not run.

The elaborated module is exactly the text a person would have written by hand
(the parent's clauses under the positional renaming, §1.4). It is loaded by the
unchanged resource-v1 loader, and it is what the environment retains as its
trusted source assumption. That is the literal sense in which an inherited
contract is a trusted assumption *of the same strength* as a hand-written one:
downstream there is no difference to observe. Provenance travels beside it
(§3).

This is also why the directive spelling uses `#`: like a C preprocessor
directive, it is consumed before parsing and never reaches the grammar.

### 1.2 Concrete syntax

```ebnf
contract_module   = "module", "contract", ";", {include_directive},
                    declaration, {declaration} ;
include_directive = "#", "include", STRING, ";" ;
STRING            = '"', crate, ":", module, '"' ;
crate             = LOWER_ALNUM, {LOWER_ALNUM | "_" | "-"} ;
module            = segment, {"/", segment}, ".qcore" ;
segment           = (LOWER_ALNUM | "_" | "-"), {LOWER_ALNUM | "_" | "-"} ;
inheriting_fn     = "extern", "fn", IDENT, "(", [parameter, {",", parameter}], ")",
                    "->", TYPE, "contract", "inherit", IDENT, ";" ;
```

* **Includes are header directives.** They are admitted only in a
  `module contract;` module, only after the header and before the first
  declaration. A directive anywhere else is refused.
* **Includes are crate-qualified.** The string names a crate and a module
  path inside that crate's `contracts/` tree, in the same logical spelling the
  `CONTRACTS` indexes and `contract/corpus.json` already use
  (`libc/memory.qcore`). `:` separates them: Cargo package names cannot
  contain it and the contract-bundle asset rule already rejects it inside a
  path, so the split is unambiguous. The module charset is exactly
  `providers/contracts.py`'s `[a-z0-9_/-]+\.qcore`, so absolute paths, `.`,
  `..`, empty segments, backslashes, escapes and a second `:` are all
  unrepresentable, not merely checked.
* **`inherit` takes one parent name** and the declaration ends at `;`. The
  child's own signature is written out in full: inheritance transfers the
  contract block, never the signature.
* **No extension clauses** in this version. `contract inherit memcpy { … };`
  is refused with its own code. §1.5 gives the reason and the follow-up.
* `inherit` is admitted on `extern fn` only. `unsafe extern fn` has no
  contract to inherit or to carry, and a defined `fn` has a body the contract
  would have to be checked against, which is a different consumer.

### 1.3 Cross-crate publication and the load path

The integrator's amendment asked for the rules by which a provider crate
publishes contracts to crates that build on it, weighing a `LOADPATH`
and an in-source `#path` directive against a recommended default. The card
adopts the recommended default, with one refinement (own-crate includes also
require publication), for the following reasons.

| Candidate | Resolution | Verdict |
|---|---|---|
| `LOADPATH` (an ordered directory list, from the environment or command line) | first directory that contains the name | **Rejected.** First-match is shadowing: which text a module includes would depend on the invoking environment rather than on anything a reviewer can read in the module or the crate graph, and two checkouts with different search orders would load different contracts under one spelling. Constraint 11's source-identity rule (two spellings are two selections, never normalised together) points the other way. |
| In-source `#path "dir";` | a module redirects its own resolution | **Rejected.** It lets a contract point its resolution at arbitrary files, which rule (4) of the brief forbids, and it recreates the `#[path]` coupling that QUIRIN-PROVIDER-SHARED has just removed (36 attributes → 15): a cross-crate source include with no Cargo edge, invisible to the dependency graph. |
| **Derived load path** (integrator default) | the include names a crate; the crate must be the includer's own crate or one of its declared dependencies; the module must be in that crate's published set | **Adopted.** |

The model is `qcore_contract::contract_inclusion::ContractLoadPath`, a set of
`PublishedContractCrate { name, version, dependencies, modules, published }`:

1. **Publication is explicit.** `published` is a declared subset of the crate's
   `modules`. Only a published module can be included, **from any crate,
   including its own**. Requiring publication for own-crate includes too makes
   `published` the single list a reviewer reads to learn what can be
   inherited from. An unpublished or absent module is refused (`QI005`).
2. **The load path is derived from declared dependencies.** A module in crate
   `A` may include from `A` and from each crate in `A.dependencies`, and from
   nothing else (`QI004`). There is no transitive visibility: if
   `fluent-bit → libc` and `app → fluent-bit`, then `app` cannot include
   `libc` modules unless it declares `libc` too. This mirrors Cargo, where a
   crate can name only its direct dependencies. A crate with no dependencies
   — `quirin-provider-libc` today — can include only its own published
   modules.
3. **Crate-qualified names, no search order, no shadowing.** Because every
   include names its crate, include resolution never searches, so no
   first-match rule exists to be wrong. Ambiguity can still arise one level
   up, in the parent name (`inherit memcpy` when two included modules both
   declare `memcpy`), and that is refused (`QI009`), never resolved by order.
4. **No in-source path redirection.** The grammar cannot spell a path outside
   the named crate's `contracts/` tree (§1.2), and the load path is data the
   caller constructs, not something a module can extend.
5. **Cycles are refused at two levels.** Crate dependency cycles are refused
   when the load path is constructed (`QI014`; Cargo already forbids them, and
   the check keeps fixture load paths honest). Module include cycles, which
   can exist inside one crate, are refused during elaboration (`QI006`).

**Real-crate wiring is not in this card.** The five contract crates'
`CONTRACTS` lists, their Cargo manifests, the resource manifest,
`contract/corpus.json` and the corpus pins are unchanged (a concurrent card
edits them). Building a `ContractLoadPath` from the real crates — a
`PUBLISHED` list beside each `CONTRACTS`, crate versions and dependency edges
read from Cargo metadata, and a manifest schema that selects `crate:module`
references — is registered as `CONTRACT-INHERIT-ADOPTION`. The rules above are
implemented and tested against fixture crates.

### 1.4 Semantics of `contract inherit`

Let the child be `extern fn f(c₁: T₁, …, cₙ: Tₙ) -> R contract inherit p;` in
module `M`.

1. **Visible declarations** are `M`'s own declarations and the declarations of
   the modules `M` includes **directly**. Included modules' own includes are
   not visible (no implicit re-export, matching G01's "implicit/transitive
   imports are not implemented").
2. **Parent lookup** finds every visible declaration named `p`. Exactly one
   must exist: none is `QI008`, two or more is `QI009`. A child named like its
   parent in an included module therefore refuses as ambiguous rather than
   shadowing — which also keeps the two out of one environment, where they
   would be a duplicate function contract.
3. **The parent must be inheritable** (`QI015`): an `extern fn` (not `unsafe`)
   whose declaration is exactly `extern fn p(params) -> R' contract { clauses };`
   with no effects row, usage annotation or other signature syntax. A parent
   that is itself `contract inherit q;` is inheritable: it is elaborated first
   (§1.6).
4. **Signature agreement** (`QI010`): the child and parent have the same
   number of parameters, the same type at each position, compared as token
   sequences, and the same result type. The child's parameter *names* may
   differ; binding is **by position**.
5. **Positional renaming.** With parent parameters `P = [p₁ … pₙ]` and child
   parameters `C = [c₁ … cₙ]`, σ maps the word `pᵢ` to `cᵢ` and every other
   word to itself. The child's contract block is the parent's clause list with
   σ applied to every word token of every clause expression. Clause order,
   clause kinds, exit selectors and all non-parameter tokens are preserved.
6. **Capture is refused** (`QI012`). If a child parameter name occurs in the
   parent's clauses as a word that is *not* a parent parameter (for example a
   child parameter named `nullable`, or `result`), σ would capture it. Selector
   words (`error(E)`) take part in the same check.
7. **The consumer's own reading is checked** (`QI012`). The renaming is sound
   for any consumer that resolves a clause word by looking it up in the
   declaration's parameter list (Lean `rename_preserves_meaning`). The
   resource-v1 loader instead reads some positions structurally — the
   nullability argument is compared against the words `nullable`/`nonnull`
   *without* a parameter lookup — so a parent parameter that happens to be
   *named* `nullable` would be renamed where v1 reads a keyword. The v1 loader
   therefore loads both the parent declaration and the elaborated child and
   requires the loaded records to be **equal**: the custody modes, the result
   mode, the literal-argument premises (by parameter index), the variadic tail
   and the ABI projection. Any difference refuses. Lean
   `positional_reading_can_diverge` exhibits the case that makes this check
   necessary.

**Never weaker, by equality.** Admission requires the child's loaded contract
to equal the parent's, so the child has exactly the parent's obligations and
guarantees under the consumer that loaded it. It is therefore neither weaker
nor stronger. Lean `inherited_is_not_weaker` states the non-weakening
direction explicitly.

### 1.5 Extension clauses: refused in this version

`contract inherit memcpy { requires resource_literal_argument(n, 0); }` is
refused (`QI002`). Whether an added clause is sound depends on the consumer,
not on the syntax. In resource v1, a literal-argument premise only withdraws
call sites and would be sound to add, but an added `ensures normal` mode is a
*duplicate mode* for a parameter the parent already covers, which is neither
a strengthening nor a weakening but a contradiction, and v1 has no ordering
on modes under which "stronger" is even defined. A sound admission rule must
therefore be per consumer and per clause family, with its own proof.
The card records it as `CONTRACT-INHERIT-EXTENSION-CLAUSES` rather than
admitting a partial rule.

### 1.6 Transitivity and termination

Chains are admitted. `f inherit g` where `g` is itself inherited resolves `g`
first. Two cases arise:

* **Across modules.** An included module is elaborated completely before the
  includer sees it, so its inheriting declarations are already concrete. A
  chain can only descend the include graph, and the include graph is checked
  for cycles with a visiting stack (`QI006`).
* **Inside one module.** Own-module parents are resolved on demand with a
  visiting set over declaration names. Revisiting a name is an inheritance
  cycle (`QI011`), including self-inheritance.

Both walks carry a visiting set over a finite node set, so each terminates.
The Lean model uses an explicit fuel bound with exhaustion as a refusal, and
proves that any cycle reachable from the start is refused.

### 1.7 Diagnostics: the `QI` namespace

The contract loader's refusals were never route codes. They are loader error
strings, and the route-code space `Q000`–`Q099` is exhausted and iceboxed
(`Q-NAMESPACE-DECISION`). The brief requires a new diagnostic code per refusal,
so this card opens a **loader-owned namespace**, `QI001`–`QI016`, on the
`QP001`–`QP016` precedent (a separate namespace owned by a non-route
component). It allocates **no `Q` route code** and does not resolve the iceboxed
route-code decision. Every `QI` code appears at the start of the loader's
error string, so existing `Result<_, String>` callers carry it unchanged.

| Code | Condition |
|---|---|
| `QI001` | malformed inclusion syntax: a directive outside a contract module's header, a directive whose operand is not one string, `contract inherit` without exactly one name and `;`, `inherit` on anything but a plain `extern fn`, an inheriting declaration with extra signature syntax |
| `QI002` | extension clauses after `contract inherit NAME` |
| `QI003` | an include target that is not `crate:module` in the admitted charset |
| `QI004` | the named crate is not on the load path, or is neither the includer's crate nor one of its declared dependencies |
| `QI005` | the named module is not published by that crate |
| `QI006` | an include cycle |
| `QI007` | the same target included twice by one module |
| `QI008` | no visible declaration has the parent's name |
| `QI009` | more than one visible declaration has the parent's name |
| `QI010` | parameter count, a parameter type or the result type differs from the parent's |
| `QI011` | an inheritance cycle |
| `QI012` | the renaming would capture a word, or the consumer reads the renamed contract differently from the parent's |
| `QI013` | inclusion syntax reached an entry point that has no load path (the file, manifest and source-map loaders) |
| `QI014` | an invalid load path: duplicate crate, malformed name or version, a published module the crate does not own, a dependency absent from the load path, a dependency cycle |
| `QI015` | the parent is not inheritable (§1.4 item 3) |
| `QI016` | retained inclusion evidence does not reproduce the environment it accompanies |

The structural parser also gets two more specific parse-error **messages**
(`#include` at top level, `contract inherit`), both still `Q000`. A message is
not a code (constraint 1).

## 2. Keyword evidence (constraint 14)

**Zero reserved words are added.** `include` is contextual: it is read only
immediately after a `#` at a contract module's top level. `inherit` is
contextual too: it is read only immediately after the word `contract` in a
declaration's contract position, where the structural parser admitted only
`{` before this card. Neither word enters
`qcore_frontend::frontend::is_reserved_source_word`, so `let inherit = 5;` and
a parameter named `include` are still programs. This is the constraint's
"contextual words stay contextual" precedent (`#unroll`, `lend`, `error`
after `log`).

The census was run, not assumed:
[`scripts/keyword_census.py`](scripts/keyword_census.py) over every tracked
Quirin source and every tracked file that could embed one, on base
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

**No frozen report's pinned diagnostic can change.** The only inputs whose
outcome this card changes are inputs containing `#include` at top level or
`contract inherit`, and the census finds none in any Quirin source, archived
probe or embedded program. Such an input was a `Q000` parse error before this
card and is still a `Q000` parse error for every consumer except the contract
loader; only the message wording is new. The spelling `record`'s precedent
(an archived probe pinned to `Q000`) does not recur.

## 3. Trust and provenance

(Completed after implementation; see §5.)

## 4. Lean correspondence

(Completed after implementation.)

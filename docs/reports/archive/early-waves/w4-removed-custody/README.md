# W4: validated direct Removed-custody wrappers

2026-09-07. Base `6a66f99`. This bounded increment infers non-null `Removed`
formals for internal C wrappers around already modeled removal calls. It does
not infer general heap links, alias-selected removal, nullable removal
preconditions, recursive components, or cost summaries.

Before this change, `EntryCustody` supplied a removal formal as an independently
live resource. A correct `q_remove(owner, item)` wrapper therefore reported a
custody conflict instead of validating. The baseline regression records this
failure on the unchanged analyzer: a synthetic store/remove/free caller could
not pass. That earlier provisional Unsafe label was an abstraction limitation,
not evidence of a defect in an upstream library.

## Implementation and invariants

A Removed entry now binds the original formal child as contained in its distinct
non-null Owned owner formal. All parameter sites are allocated before linking
containment, so owner/child declaration order has no effect. These are stable
abstract resource sites; reassigning local parameter names does not rename the
caller's resources. Invalid owner shapes cannot construct this entry.

The existing live-entry inference runs first. A body that stores then removes
an incoming directly owned child keeps its ordinary Owned contract. Only when
that search fails does a bounded fallback inspect modeled Removed calls:
child and owner arguments must be exact formal names, the removed mode must be
non-null, all requirements must agree, and the inferred result is neutral.
Calls may use declared contracts or earlier independently validated summaries
from the acyclic call graph. This supports reordered forwarding wrappers and
multiple distinct removed children without any library-name table.

Syntax proposes only a candidate precondition. Fresh intraprocedural analysis
must be Safe and diagnostic-free, have reachable exits, and validate **every**
exit against the candidate. Each original child must be directly live and no
longer contained, each original owner must remain live, no foreign attachment
may escape, and no formal may become an untracked returned payload. Missing
removal paths, repeated removal, reassignment to another resource, reattachment,
release of the child/owner, unknown effects and incompatible ownership demands
therefore reject the candidate. Their callers retain unsupported/Unknown effects.

Declared contracts retain precedence. A validated mismatch produces a
SummaryConflict and no published inferred record for that function. Certificates
retain `InferredValidated` records and independently rederive their exact
contracts from source and the contract environment. Existing schema modes and
wire versions are unchanged.

## Regressions and formal boundary

The replaced baseline regression now validates a complete store/remove/free
caller. Eight additional gateway tests cover reordered forwarding, preserving
the old store-then-remove route, thirteen reassignment/path/effect traps, alias
and nullable discovery exclusions, seven invalid/residual caller cases, declared
precedence, certificate mode/source mutation, and other-child preservation plus
aliased multi-child removal. A detached child still leaks when the caller frees
only its former owner. Passing the wrong owner, a null child, a directly owned
child, an already removed child, or repeated aliases never becomes Safe.

The Lean `CustodySummary` model adds Removed parameter disposition and a separate
explicit entry relation. `validateFromEntry` checks stable formal-position
containment and distinct Owned owner shape before the existing all-exit
validation. Theorems establish original entry custody, live detached children
after validation, a valid direct removal, rejection of uncontained/wrong-owner
entries, and rejection when any exit retains containment. The older exit-only
`validate` relation remains explicit. These are reduced model theorems, not a
proof of arbitrary C heap extraction, Clang correspondence, caller reachability,
or the truth of a declared external removal contract.

## Pinned audit and validation

Both full 185-TU audits were freshly run from this worktree using the persistent
forced-Libevent database at
`/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/monkey-build/compile_commands.json`.
Both resource diagnostic JSON and Monkey source JSON are **byte-identical**:
**3 Safe / 98 provisional Unsafe / 80 Unknown / 4 data-only**, with no unit,
function, coverage or diagnostic delta. The pinned production contract selection
currently declares no Removed effects, so this increment's precision gain is
shown by its synthetic declared-contract wrappers, not claimed on that corpus.
Raw production units remain unpromoted.

The standing gate passes: **557 workspace tests**, formatting, clippy with
warnings denied, both reevaluation examples, Lean build and all **19** Lean audit
files. The pinned Libevent submodule was initialized before validation. Baseline
failure, fresh before/after audits, exact report hashes, complete gate logs and
exit codes are archived here; `parity.json` records the empty delta inventory.

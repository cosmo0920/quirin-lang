# Context-specific recursive custody inference

Baseline `3a2eb70`, 2026-09-07. This increment addresses actual reachable
recursive calls in the unrestricted-C gateway. Previously, a base-case exit
could not seed a usable summary: a reachable call to an unavailable SCC member
kept the entire function unvalidated. Merely repeating that analysis cannot
supply the missing scalar precondition or justify an assumed self-contract.

## Checked relation

The bounded domain separates zero and nonzero values of one immutable, explicitly
integer control formal. The zero case validates with recursive component calls
unavailable. The nonzero case may call an already validated zero case only when
the call's control argument is exactly the supported zero literal. Both contexts
must validate and agree on the complete custody contract before ordinary callers
may use it. This supports release wrappers and fresh-return wrappers, including
mutual recursion whose admitted contextual dependencies decrease.

Contextual calls have ranks zero and one, so admission never assumes a context's
own summary. The source guard accepts direct truth/negation tests (including
recognized zero comparisons) and requires every other control occurrence to
reject. All visible prototypes must agree within its restricted ABI vocabulary.
After guard selection, graph-unreachable nodes are removed and successors
reindexed; every remaining Unknown instruction rejects this route. The combined
CFG contains disjoint allocation identities and an entry covering both contexts.
It is revalidated before publication and retained for the function's own body
check, so that check never relies on its ordinary self-contract. Unsupported control, mutation, aliasing and nondecreasing recursive
arguments stay outside this domain. Ordinary inference runs first. A component
containing a declared contract stays on the existing ordinary route; contextual
inference does not replace declarations or infer new conflicts for that component.

## Proof and independent tests

`QCore.ContextualCustody` extends the existing exit-validation relation with
exhaustive integer contexts, validated lower-rank dependencies and publication
agreement. Its theorems establish well-founded contextual calls, prohibit self
and circular assumptions, and connect every integer context to nonempty checked
exits with matching custody. Typed C scalar recognition, CFG specialization and
complete call inventories remain source-import premises. This is not a theorem
that arbitrary C loops terminate.

The separate public-API test suite uses real Clang imports and independent
certificate rederivation. It covers release calls with zero, positive and negative
flags, negated guards, owned returns and caller leaks, mutual recursion, control
mutation/shadow/address escape, nested uses, unsupported scalar types/control,
nondecreasing and zero-context cycles, repeated consumption, declaration
precedence and source/summary substitution. Explicit invalid `QCORE_CLANG` fails
rather than silently skipping tests.

## Remaining scope

General arithmetic countdowns, several scalar controls, general recursive
fixed-point inference, control aliases and broader path-dependent custody remain
open. Existing typed indirect-access, general removal/out-slot aliasing and cost
summary work is also still open. This increment must not be presented as the
completion of every W4, W6 or language-conformance requirement.

## Validation and corpus comparison

The complete standing gate passes: 619 workspace tests, format/Clippy, both
reevaluation probes, Lean build and all 19 axiom audits. The excluded provider
workspace passes nine tests, and all 43 Python validator tests pass. Eight new
public-API tests cover the context feature, including 19 adversarial fixtures.
Independent subagent implementation and test/review tracks found no remaining
material issue. Logs and `validation.json` are archived beside this report.

The old `make(int n)` regression now expects a validated owned-return summary
and its caller's provisional Unsafe leak. Unconditional self recursion and
nondecreasing mutual recursion keep their old rejection expectations. An existing
ordinary proof that `flag = 0` makes recursion unreachable remains accepted.
The baseline positive-test failure and explicit-invalid-Clang failure logs are
archived as expected negative evidence, not failed final validation.

The full 185-unit audit using persistent Monkey inputs and the real ChunkIO
0.3.0 package is byte-identical to baseline in all four outputs: source report,
resource diagnostics, contract sidecar and source-effect sidecar. Raw totals
remain 2 Safe / 47 provisional Unsafe / 132 Unknown plus 4 data-only units.
The three bounded provider contracts and 93 Unknown functions are unchanged.
No raw corpus unit is promoted. `audit-parity.json` records exact hashes.

The current LC01/status text is also corrected: some syntactically recursive
functions already validate when recursion is unreachable, and the gateway's
resource projection does not generally establish program termination. Historical
audit reports retain their original provenance.

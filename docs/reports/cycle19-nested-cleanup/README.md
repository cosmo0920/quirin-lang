# Cycle 19: residual custody through nested parent teardown

The source-bound context/stream chain now composes both memory and filesystem
children. It rechecks the file dependency and requires its memory/common dispatch
certificate to equal the nested chain's certificate. Each filesystem selection
must be a nonnull, non-deleting complete chunk-close case: stream destruction
cannot substitute a deleting case or a null chunk for a ring member.

The conditional projection retains ordered stream/chunk identities, releases each
selected allocation, and carries every unresolved mapping/descriptor through
stream and context free. Total-counter equality, distinct complete identities,
checked up-counter subtraction and remaining up-counter obligations are explicit.
A conditional discharge gate rejects any residual OS identity or nonzero up
counter. Stream/context free establishes no backing deletion.

All 43 non-deleting filesystem cases are exercised as mixed memory/file source
scenarios. Seventeen conditionally discharge their selected nested obligations;
26 retain unresolved OS custody. Separate regressions preserve failures across
multiple streams and reject duplicate identities, wrong/null/deleting/out-of-range
cases, incomplete counters and up-counter underflow. These are **conditional
source scenarios**, not native runtime observations or proven caller inputs.

The 12-clause QCore policy retains component preconditions, actual ring/caller
identity and order, disjoint allocation provenance, exclusive access, saved-next
and parent frames, ABI/call binding and each child's entry/OS outcome assumptions.
No filesystem case is emitted as the core lifecycle model's unconditional
`CloseChunk`. Five new Lean theorems preserve exact residual identities, balance
the counter, reject underflow and require empty residuals/zero up count at
discharge. They do not prove the Rust projection or native C semantics.

Source reference matching also now checks ordinary record/field declarations
against explicit pointer/list types before comparing bodies. Because a reference
shares source declarations, body equality alone could otherwise accept both
bodies using a corrupted integer-owned field. Mutations reject that case; the
parent fixture now uses the pinned `void *` processed-user/group fields.
Pinned source evidence is unchanged by this stronger declaration check.

All 400 workspace tests, 43 contract modules, audit example tests, formatting,
Clippy with warnings denied, both reevaluation probes, Lean build/dependency
audit and pinned external audit pass. The full workspace run used
`RUST_TEST_THREADS=1 cargo test --workspace` after a parallel run hit the existing
process-provider fixture's intermittent `Text file busy` spawn error. No process
provider behavior was changed. Previous source/diagnostic/certificate evidence
is identical; one new nested-cleanup entry records the chain and 43 scenarios.
Aggregate results remain two Safe, 98 provisional Unsafe, 81 Unknown, four
data-only units and zero gateway failures across 185 units.

Reproduce with the [traversal commands](../cycle19-teardown-traversal/README.md)
using `/tmp/cycle19-nested-cleanup` as prefix and the serialized workspace-test
command above. Logs, compressed source evidence, parity and SHA-256 hashes are
archived here. Cycle 19 remains in progress: native constructor/caller/ring
provenance, full filesystem sync and real OS guarantees, unsupported backend
classification, aggregate source custody, remaining wrapper lifecycle paths and
full Monkey/Cycle 18 event composition still require work.

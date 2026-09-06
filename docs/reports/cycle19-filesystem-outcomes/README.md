# Cycle 19: native filesystem cleanup outcomes

Two source-derived Unix helper certificates now check all four entry/outcome
cases: null, resource absent, syscall failure and syscall success. The complete
pinned typed paths and exact native policies are retained in the sidecar. This
checks conditional C bookkeeping transitions, not actual OS resource custody.

| Entry/outcome | Return | Cleanup calls | Field changes |
| --- | ---: | ---: | --- |
| Null record | -1 | 0 | None; no record access |
| Absent descriptor/mapping | 0 | 0 | None |
| Present, syscall returns -1 | -1 | 1 | All selected fields preserved |
| Present, syscall returns 0 | 0 | 1 | Descriptor reset to -1, or mapping/size/warning reset |

The shared `cleanup_outcome.rs` interpreter uses symbolic initial fields and
pointer identities. It does not test a finite sample of fd or size integers:
comparisons outside the supported symbolic partition reject. Each case selects
exactly one complete source path, and every source path must be covered. Calls
must use the original descriptor or mapping and length, writes must follow a
successful syscall, and all reset fields must be written exactly once in total.
Unknown calls, nonterminal returns, extra writes, unsupported casts/arithmetic,
wrong types, wrong field identities and unresolved branch conditions reject.
The ChunkIO facade provides function, field, syscall and diagnostic profiles.

The contract under `contract/qcore/chunkio/filesystem_outcomes.qcore` has two
16-clause executable policies. Preconditions include live exclusive record
provenance, valid syscall argument identities, zero/-1 syscall results, returning
external functions that frame the record, and source/ABI binding. These are
caller and external-function assumptions. In particular a close error does not
prove a descriptor remains open or has been discharged. Diagnostic errno reads
and logging are explicit framed dependencies. Arbitrary caller provenance,
platform semantics and full filesystem teardown remain outside the certificate.

All 364 workspace tests and the 31-module native corpus pass. Mutations cover
wrong outcome guards/returns, premature/missing/duplicate/wrong-field resets,
wrong syscall arguments, fd-dependent unresolved branching and ABI changes.
Aliases and equivalent zero/-1 outcome guards are accepted. Every native policy
postcondition is tested with false and unknown replacements. Format, Clippy,
audit example tests, both reevaluation probes and the pinned audit pass.

Six new Lean theorems cover null/no-op behavior, failure preservation, success
reset and the distinction between failed cleanup and proven discharge. The Lean
build and Cycle 19 audit contain no placeholders or project axioms. The model is
conditional abstract semantics; Clang, the Rust source replay and external-call
frames are trusted boundaries. No new native execution is claimed in this step.

Generic diagnostics and the Monkey sidecar remain byte-identical to the previous
filesystem-attempts baseline. All 96 scalar results and prior teardown evidence
are identical. The generic inventory remains 185 units, 4,688/4,775 projections,
two Safe, 98 provisional Unsafe, 81 Unknown, four data-only and zero gateway
failures. Neither projection coverage nor provisional Unsafe is an upstream
safety/defect claim. Cycle 19 remains in progress: filesystem wrapper composition,
OS outcome binding, stream/context traversal, constructor provenance and broader
Monkey event composition still require work.

Evidence: `parity.json`, `chunk-contracts.json.gz`, validation logs and their
SHA-256 manifest in `evidence.json`. Commands match the previous report, using
`/tmp/cycle19-file-outcomes` as the artifact prefix.

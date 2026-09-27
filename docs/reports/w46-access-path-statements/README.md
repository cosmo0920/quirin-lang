# W46-ACCESS-PATH-STATEMENTS

P2 of the [unsupported-coverage plan](../../plans/unsupported-coverage/README.md),
based on `207cebe4cdb745f6216ff2b511c4b7fd134581b9`. The prediction was
frozen before implementation in [PREDICTION.md](PREDICTION.md). The earlier
[P1 prediction](../w46-access-path-census/PREDICTION.md) is unchanged.

## Implementation and boundary

`typed_access_path.rs` reads Clang's typed AST for the exact analyzed source,
dialect and frontend flags in each gateway unit. It uniquely matches a
complete statement by internal line and tokens, then lowers scalar paths and
whole pointer-valued path reads through `.`, `->`, `*` and `[]` to
the established indirect-access flow transfer. Each tracked root and loaded
pointer hop keeps the existing null/liveness obligation. A non-pointer store
retains the custody frame. A pointer read binds a target-free, possibly-null
borrowed alias and never transfers custody.

Missing or ambiguous AST facts, unmatched source, unions, bitfields,
casts within a path, volatile/atomic paths, pointer arithmetic, mixed
paths with a tracked root, pointer stores and call-bearing statements refuse. This card
does not serve conditions, returns or call arguments (P3). A narrow no-access
arm admits scalar arithmetic and declarators that Clang proves do not touch
memory; multi-pointer aliases require every source and destination to be
statically never-bound to tracked custody. Source-scope and certificate
re-derivation both pass the same original dialect and frontend flags.
Mixed embedded-record and indexed paths are admitted only when the root is
statically never-bound to a tracked identity; every pointer loaded from that
untracked graph is outside this resource projection. A tracked mixed path
still refuses because the existing field-link state cannot name its nested
places. A focused indexed path test checks both arms.
`sizeof` is admitted as no access only for a childless, non-array AST type:
Clang represents evaluated VLA bounds as children, and the held-out
`sizeof(int[(close(fd), n)])` control stays `Unknown`.

The established indexed-element, scalar-member-store, scalar-field-noop,
typed scalar, chain, sub-object-store and pointer-step shape readers stay
first. With
`QCORE_TYPED_PATH_PARITY` set to an output JSONL path, each consulted
statement records both readers' complete plans, any disagreement, and the
analyzed source identity. The corpus artifact below archives that stream.
Routes with a different instruction vocabulary record every overlap as an
unresolved structural comparison, including cases both admit; they make no
parity claim. Verdict/coverage comparison remains the retirement arbiter.
No shape route is retired before verdict and coverage parity.

The matching Lean module is `proof/QCore/TypedAccessPathStatements.lean`,
registered in `proof/AuditTypedAccessPathStatements.lean`. It proves refusal
on missing/unlisted forms, per-hop discharge, non-pointer frame preservation,
the target-free borrowed binding, and the untracked declarator boundary.

## Measured corpus and system slice

TBD after the arbiter slot: BEFORE identity, AFTER verdicts and diagnostic
counts, every moving function with attribution, P1 prediction hit/miss table,
side-by-side disagreements, and the `046accf6` Fluent Bit slice.

## Validation

TBD after the standing gate: exact commands, exits, totals and private logs.

## For the integrator

TBD after measurements: precise STATUS/matrix changes, named followups and
the increment paragraph. The integrator owns those shared files.

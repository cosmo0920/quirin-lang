# Existing-C resource access: fail-closed projection

2026-09-07. Base `f9a3009`, the merged direct Out-forwarding increment.
This repair prevents unsupported memory access from disappearing into a neutral
resource instruction. It adds no heap, liveness, bounds or general C memory-safety
model.

## Property and defect

The authoritative profile is `c.resource` in specification revision 0.3 (L18): a
Clang-gated conservative custody projection under trusted resource contracts.
`unrestricted` describes the production C front end, not complete verification
of arbitrary accepted C. The resource importer must preserve gaps as Unknown.
A syntactically valid read is not evidence that its base is live or its index valid.

The archived pre-change regression shows eleven forms incorrectly accepted as
Safe after releasing an allocation: returned and discarded dereferences,
alias-mediated dereference, returned/discarded subscript and member reads,
reads in conditions/loops, and a cast followed by dereference. These became NoOp
or unchecked null-test instructions because the parser checked only calls and
braces, or treated a compound place name as sufficient import evidence. The
borrowed dereference case already rejected and remains in the regression table.

## Implementation boundary

Return, branch, loop, switch and case expressions now reject unmodeled indirect
accesses. Null-test recognition uses the same gate, so a member or indexed
expression cannot bypass it by matching a null comparison. Simple statements
and call arguments also reject such accesses, including discarded expressions
and resource calls that would otherwise erase a borrowed argument.

The exclusion recognizes dereference forms, member selection and subscripts.
Ordinary multiplication with a plain left operand remains identifiable; a star
after a closing parenthesis may follow a typedef cast, so ambiguous forms remain
Unknown. Supported scalar arithmetic, scalar updates and simple scalar control
flow have positive regressions. Declarator stars in recognized declarations do
not themselves count as evaluated reads; array/aggregate access still requires
an importer. No unsupported cast, unevaluated-expression, VLA or object-lifetime
proof is invented from lexical spelling.

The existing direct `*out = value` store remains a separate, sanctioned Out-slot
operation, with its stable-identity and all-exit validation intact. Unmodeled
reads through an output pointer remain Unknown. Flat canonical member and
constant-index names are no longer treated as sufficient access evidence; the
former Safe aggregate fixture now explicitly expects Unknown. This deliberately
loses precision until an access importer establishes the missing object/base and
index facts. It does not silently add those facts as trusted assumptions.

The gateway's declared resource contracts retain their original scope and trust.
Specialized typed provider/source importers retain their separate support bounds;
this change neither validates their full source behavior nor establishes general
C safety. Existing provisional Unsafe diagnostics are not promoted to defects.

## Validation and Lean boundary

The gateway regressions cover direct, discarded, alias, indexed, member,
condition, loop, borrowed and cast read shapes, plus a multiplication/declarator
ambiguity. Production C11 regressions reject the original counterexamples and
certificate recheck rejects read expressions inserted into an otherwise valid
certificate. Existing Out/custody tests remain in the full suite.

The Lean model already has an unsupported instruction and the audited
`unsupported_is_never_safe` theorem. The new
`released_then_unmodeled_access_is_indeterminate` theorem records that an acquire,
release and unsupported access sequence remains indeterminate. It assumes the
importer's decision to emit unsupported; it is not a proof of lexical parsing,
Clang correspondence or arbitrary C memory access. All project axiom audits
remain placeholder-free.

## Pinned audit

The baseline is the parent's full combined-main audit at `f9a3009`, reused from
`/tmp/qcore-generalization-final-{diagnostics,source}.json`; it was independently
confirmed byte-identical to the prior Out evidence. The fresh after audit uses
the same persistent forced-Libevent compile database at
`/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/monkey-build/compile_commands.json`.
`parity.json` records every unit/function class change, diagnostic addition and
removal, coverage comparison and exact hashes. Contract sidecars preserve the
trusted input assumptions and their source paths.

Final totals: **2 Safe / 47 provisional Unsafe / 132 Unknown / 4 data-only**,
compared with **3 / 94 / 84 / 4** at the exact baseline. Forty-eight units
downgrade to Unknown (47 provisional Unsafe and one Safe); 455 functions
downgrade (206 provisional Unsafe and 249 Safe). There are no promotions.
`chunkio/cio_error.c` loses its former Safe result because its chunk-member
reads/writes lack the required object/access evidence, not because this audit
establishes a reachable production use-after-free. Coverage and source/function
inventories remain unchanged, and Monkey source JSON is byte-identical.

The raw diagnostic deltas are +7,537 unsupported, −4,472 unknown-alias,
−434 leak and −11 double-release. These large deltas record withdrawal of
unsupported projection behavior, **not proven upstream defects or repairs**.
The separate compressed diagnostic diff preserves every added and removed item.
Typed aggregate/access evidence remains an open implementation requirement for
restoring the rejected cases; the success criterion has not been redefined as
supporting only the reduced corpus. Declared-contract sidecars differ only in
worktree source-path prefixes, checked explicitly after normalization.

The full standing gate passes: **582 workspace tests**, formatting, clippy
with warnings denied, both reevaluation examples, Lean build and all **19**
audits. The pinned submodule was initialized before the gate. Shared-quota
interruptions were resolved before the successful final runs; their incomplete
builds are not counted as validation.

# Cycle 19: shared short-circuit source conditions

The shared typed C importer now expands `&&` and `||` in `if` conditions into
sequenced guard paths. It evaluates the left operand first and visits the right
operand only when required by C short-circuit semantics. Nested logical trees
and negation preserve branch truth. No eager logical operator is added to the
symbolic value format. Existing atomic-condition projections are unchanged.

The expansion caps intermediate frontiers and completed function paths at the
existing 64-path bound. Logical values in returns, initializers, arguments,
comparisons or structured scan regions remain unsupported. Unsequenced call/read
operands still reject; parsing still inspects unreachable syntax, so a skipped
unsupported goto cannot silently disappear. This is bounded branch-condition
support, not unrestricted C expression evaluation.

`cio_file_native_delete` was previously blocked by its fd-or-mapping guard. The
pinned audit now retains its four typed paths separately:

- An open-fd guard skips the mapping read and returns an error.
- The subsequent mapped guard skips the path read and unlink call.
- The remaining branch calls unlink and checks its result on both outcome paths.

The executable `filesystem_delete_attempt.qcore` policy records that unlink's
result is used in a guard and is not ignored. It does not prove the guard's
correctness, actual unlink success, caller provenance or deletion custody. The
new sidecar entry is `filesystem_delete_attempt`; no additional custody or Safe
certificate is issued, and the outer file-close deletion paths remain uncovered.

All 375 workspace tests and the 35-module native corpus pass. Tests independently
replay 27 zero/negative/positive value combinations for a nested condition and
require exactly one path with the correct result. They also check RHS skipping,
call-before-guard-before-call order, a nullable pointer guard avoiding a field
read, native-delete read/call counts, ignored-result mutations and every new
policy postcondition changed to false or unknown. Unsupported value contexts,
unsequenced calls, unreachable goto and path explosion remain Unknown.
Formatting, warnings-denied Clippy, audit example tests, both reevaluation
probes and the pinned audit pass. Six new Lean theorems prove skipped RHS effects,
left-before-right evaluation, Boolean results and independence from a skipped
RHS. The build and Cycle 19 audit pass without placeholders or project axioms.
Rust/Clang import and the symbolic replay remain trusted boundaries.

The generic diagnostic inventory and full Monkey sidecar remain byte-identical
to the file-close baseline. All 96 scalar results and every prior teardown
certificate remain identical. Aggregate counts stay at 185 units, 4,688/4,775
projections, two Safe, 98 provisional Unsafe, 81 Unknown, four data-only and zero
gateway failures. Source projection is not safety coverage, and provisional
Unsafe is not a confirmed upstream defect. No new native execution is claimed.

Evidence: `parity.json`, `chunk-contracts.json.gz`, required validation logs and
`evidence.json`. Commands match previous reports with `/tmp/cycle19-short-circuit`
as output prefix. Cycle 19 remains in progress: source-derived deletion outcomes
and full sync frames, actual OS/caller identity binding, enclosing filesystem
chunk close, stream/context traversal and full event composition remain open.

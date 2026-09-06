# Cycle 19: stream and context allocation release

The pinned `cio_stream_destroy` and `cio_destroy` bodies now match complete typed
references. Both retain their null guard and invoke the correct child traversal
before releasing parent storage. Stream teardown unlinks its primary node and
releases its name before the stream. Context teardown conditionally releases
user, group, chmod, processed user/group and root path, then the context.

The new 11-clause QCore policy keeps exclusive lifetime, disjoint owned fields,
child-call frames, live reciprocal stream membership, source/ABI and libc
bindings explicit. Matching the parent body does not establish child cleanup or
caller allocation provenance. Four Lean theorems prove that modeled parent free
releases parent fields exactly once and preserves every residual child obligation.
This abstraction does not prove the Rust matcher or C memory safety.

The shared matcher also preserves **token boundaries** in offsetof operands.
Whitespace removal could otherwise confuse `struct record` with a typedef named
`structrecord`, despite different offsets. A regression rejects that collision.
The two preceding traversal witnesses therefore change only the representation
of their offsetof operands from concatenated text to token arrays; their full
remaining AST, source references and contract texts remain identical.

All 392 workspace tests, 41 contract modules, audit example tests, formatting,
Clippy with warnings denied, both reevaluation probes, Lean build/dependency
audit and the pinned external audit pass. The generic diagnostic inventory,
Monkey sidecar, scalar results and all earlier non-traversal teardown entries
remain identical. Two parent entries are added. Aggregate verdicts remain two
Safe, 98 provisional Unsafe and 81 Unknown across 185 units, four data-only units
and zero gateway failures. No new native execution or upstream defect is claimed.

Use the [traversal reproduction commands](../cycle19-teardown-traversal/README.md)
with `/tmp/cycle19-parent` as output prefix. Logs, compressed source evidence,
parity checks and SHA-256 hashes accompany this report.

Cycle 19 remains in progress. The next source composition must bind each ring
member to the correct checked child destructor and preserve its residual cleanup
outcomes. Native constructor/caller provenance, filesystem sync/OS guarantees,
aggregate custody and full event composition remain open.

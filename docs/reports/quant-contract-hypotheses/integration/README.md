# Quant hypothesis merge integration

The user committed and pushed the merge as
`f774f54dee6c64c6449a4fca4d83dcb0e6b30d39`, combining main
`486b05a71e651c232ca93c54bf81a129f9f1d8d9` and quant branch
`647cbec1b1c44edef18c6d7f319fa9801c94fbe1`.

The six conflicts were resolved by preserving main's version-4 source-pin
metadata, refreshing typed plugin/libc pins with the official command,
regenerating inventories (including the duplicate CFL hash entry), and retaining
144 tracked corpus modules. The frontend schema dependency was aligned to 22.
The six restricted pin commands exited 0; only contract digests/review stamps
changed in 94 resource, 71 bounds and two flexible entries. All reviewed C
identities, revision ties and unrelated entries are unchanged. Provider contract
validation then exited 0. Logs: `private/quant-merge-20261027/` in main.

## First full merged gate — failed

The attached gate.json owns the actual measurement on f774f54 in main, with
Clang captured in that checkout. Formatting, Clippy and both probes exited 0;
Lean passed 386 audits. The Rust runner exited 101: 571 units, 590 summaries,
8,695 passed, 135 failed, 15 ignored, 24 filtered out. Python exited 1 over 546
tests (two failures). Sanitizer/ptrace and user-namespace restrictions affected
native tests, and real integration defects were also exercised; this run is
not successful validation.

Provider packaging exited 1 because its locked workspace omitted new dependency
edges. Package verification exited 1 (no built packages); provider Clippy exited
101 at the same lock refusal. The queued, childless provider-test waiter (PID
838028) was cancelled with SIGTERM after that cause was established, rather
than represented as an executed test suite. Provider formatting exited 0.
The full driver exited 1. Every command, exit and private log path is in the
attached record. Failure evidence remains immutable.

The resumed default-setter migration and integration repairs are subsequent
work, described in sibling reports. The all-provider migration remains open.

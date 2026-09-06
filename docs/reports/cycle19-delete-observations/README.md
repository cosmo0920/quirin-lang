# Cycle 19: controlled native deletion observations

The sanitized delete harness includes the actual pinned `cio_file_unix.c` body
and runs 16 nonnull backend scenarios. Descriptors -1, 0, 7 and INT_MAX combine
with null/non-null mapping fields and injected zero/-1 unlink results. Fourteen
executions block unlink; the two eligible executions cover failure and success.
The failure reports errno once. Record and terminated path snapshots stay
unchanged, and every executed unlink receives the original path identity.

The new `filesystem_delete_observation.qcore` has 12 executable clauses. All
16 observations pass; regression tests replace every premise and postcondition
with false or unknown facts and corrupt eligibility, call counts, argument
identity, returned status and frame facts. The checked fixture is byte-identical
to fresh output. Endpoint snapshots do not prove intermediate write absence.

The driver also reruns the original 18 close/unmap cases unchanged, making 34
controlled native executions. AddressSanitizer, UndefinedBehaviorSanitizer and
leak detection finish without diagnostics. LeakSanitizer ran outside the ptrace
sandbox with authorization. Hooks do not close/unmap resources or delete files;
this evidence does not establish actual OS effects or whole-source safety.

`metadata.json` records the pinned revision
`66910c10a4d7eafa810b84229cf2dcf7b0f26f97`, compile database identity and selected
entry, command, compiler version, included dependencies and harness/driver/
contract hashes. Tracked upstream edits reject the run. Compiler, system headers,
sanitizers and the observation producer remain trusted execution boundaries.
Runtime observations are not imported as static certificates.

All 382 workspace tests and 38 native contract modules pass, together with
formatting, Clippy with warnings denied, audit example tests, both reevaluation
probes and Lean build/audit. No new Lean semantics is introduced. The pinned
audit's complete chunk-contract sidecar, Monkey source report and generic
diagnostic inventory are byte-identical to the deleting-close baseline.
The inventory remains 185 units, two Safe, 98 provisional Unsafe, 81 Unknown,
four data-only and zero gateway failures. No upstream defect is asserted.

Reproduce executions:

```sh
python3 scripts/cycle19_cleanup_observations.py \
  --fluent-bit ../fluent-bit --output /tmp/cycle19-delete-native-execution
```

Validation commands match the previous report with
`/tmp/cycle19-delete-observations` as prefix. The evidence manifest hashes the
retained logs, observations, metadata, sidecar and parity result. Full sync
frames, actual OS/caller identity binding, enclosing chunk close, stream/context
traversal and wider event composition remain open; Cycle 19 is in progress.

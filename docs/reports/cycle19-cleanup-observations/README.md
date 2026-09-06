# Cycle 19: controlled native cleanup executions

The new sanitized harness includes the actual pinned `cio_file_unix.c` and
executes its close/unmap helpers with controlled zero/-1 syscall hooks. It
observes 18 executions across all eight helper/entry/outcome combinations.
No upstream file is changed. The hooks do not perform OS close/unmap operations.

| Helper | Null | Absent | Failure | Success |
| --- | ---: | ---: | ---: | ---: |
| `cio_file_native_close` | 1 | 1 | 3 | 3 |
| `cio_file_native_unmap` | 1 | 1 | 4 | 4 |

Close exercises descriptors 0, 7 and INT_MAX; unmap combines two lengths and two
warning states. These are valid controlled hook arguments, not real acquired OS
resources. The harness asserts argument identity, cleanup and diagnostic call
counts, return status and endpoint field values. A whole-record byte snapshot,
with only the expected fields restored, checks the remaining record frame.
Null/absent paths make no cleanup call; injected failures log once and preserve
the fields; successful calls clear the relevant bookkeeping. Endpoint snapshots
do not measure assignment counts or prove unobserved paths safe.

The new `filesystem_observation.qcore` policy evaluates these observed facts.
All 18 observations pass. Rust regression tests evaluate every postcondition
with false and unsupported replacements and reject corrupted argument, frame
and failure-preservation facts. The captured fixture is byte-identical to the
new execution output. All 365 workspace tests and the 32-module native corpus
pass, together with formatting, warnings-denied Clippy, audit example tests,
both reevaluation probes, the Lean build/audit and the pinned audit. No new Lean
semantics is introduced: the previously proved conditional outcome distinction
is unchanged. AddressSanitizer, UndefinedBehaviorSanitizer and leak detection
finish without diagnostics. LeakSanitizer requires execution outside the ptrace
sandbox; it was run there with authorization.

`metadata.json` retains the compile database entry and hash, actual command,
Clang version, non-system dependency hashes, source/harness/driver/contract
identities and scenario inventory. Returning syscall/diagnostic framing comes
from the controlled hooks. The recorded revision is
`66910c10a4d7eafa810b84229cf2dcf7b0f26f97`; tracked upstream changes reject the
run. System headers, Clang, sanitizers and the observation producer remain
trusted execution boundaries. Runtime observations are not accepted as static
certificates, and these tests do not establish actual OS behavior on failure.

The entire generic diagnostic inventory, Monkey sidecar and chunk-contract
sidecar remain byte-identical to the native-outcome baseline. Aggregate verdicts
remain two Safe, 98 provisional Unsafe, 81 Unknown, four data-only and zero
gateway failures over 185 units. No translation unit is promoted, and no
upstream defect is asserted. Cycle 19 remains in progress; wrapper failure
composition, actual OS resource binding, source iteration/provenance and full
event composition are still open.

Reproduce the new evidence:

```sh
python3 scripts/cycle19_cleanup_observations.py \
  --fluent-bit ../fluent-bit --output /tmp/cycle19-cleanup-native
```

The remaining validation commands are the same as in the filesystem-outcomes
report with `/tmp/cycle19-cleanup-native` as artifact prefix. `evidence.json`
hashes the retained logs, observations, metadata and parity report.

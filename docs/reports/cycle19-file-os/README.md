# Cycle 19: real Linux cleanup endpoint observations

Twelve executions of the pinned `cio_file_native_close`, `cio_file_native_unmap`
and `cio_file_native_delete` bodies use real syscalls in fresh, exclusive `/tmp`
fixtures. Forwarding wrappers record the exact arguments, return and errno.
`fcntl(F_GETFD)` and `mincore` probe descriptor/mapping endpoints. A hardlink
witness retains the same device/inode while the target pathname is removed;
its link count falls from two to one on successful deletion. Each fixture is
fully cleaned by the harness after the observed call.

The cases cover null/absent cleanup, successful close/unmap/delete, a stale
descriptor returning EBADF, zero-length unmap returning EINVAL with its live
mapping retained, missing-path deletion returning ENOENT, and deletion blocked
by an active descriptor or mapping. These particular failures do not generalize
to arbitrary close failures or concurrent descriptor/path/mapping reuse. The
helper's endpoint fields and all other record bytes are checked separately from
OS probes. Diagnostic interception records errno without changing cleanup results.

The independent Python adapter requires the exact ordered scenario inventory,
canonical integer fields, matching syscall identities/results, record frames,
OS probes, inode witness and fixture cleanup. Three Python tests include 31
mutations plus duplicate-JSON-key rejection. A new 13-clause QCore runtime policy
checks derived observations. Four Lean theorems describe the final abstract
identity/endpoint gate and prohibit static-safety or durability promotion; they
do not prove the C observer, Python adapter, Linux kernel or native source.

ASan, UBSan and leak checking pass with empty native stderr. LeakSanitizer's
initial ptrace restriction required rerunning outside the sandbox; sanitizer
checks were not disabled. The pinned Unix source and included headers are
sanitized; unused source sections are discarded, and system libraries retain
their existing builds. Compiler commands and hashes bind source dependencies,
observer, checker, binary, contract and records.

Reproduce from the repository root:

```sh
python3 -B scripts/cycle19_file_os_observations.py --fluent-bit ../fluent-bit --output /tmp/cycle19-file-os
python3 -B -m unittest discover -s scripts/tests -v
```

The standing Rust/Lean checks, both reevaluation probes, two audit-example tests
and pinned external audit pass: 407 workspace tests, 48 contract modules and
13 Python tests. Prior source/diagnostic/contract evidence is unchanged.
Aggregate verdicts remain 2 Safe, 98 provisional Unsafe, 81 Unknown, four
data-only units and zero gateway failures across 185 units.

Cycle 19 remains in progress. These helper observations do not establish native
filesystem constructor/parent teardown, full sync/durability, general caller or
OS failure guarantees, aggregate custody, wrapper lifecycle or full Monkey/Cycle
18 event composition. No raw unit is promoted.

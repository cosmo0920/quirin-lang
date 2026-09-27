# CT50: a reused address does not revive an old Buffer identity

Baseline `6824aa5`, 2026-09-07. CT50 is now mapped through actual source,
reference execution and native Linux storage reuse. No parser, checker, core,
backend or Lean semantics change is needed: the owned-task implementation already
uses fresh generation identities independently of allocation addresses.

The previous CT triage deliberately left this case unmapped. Its sequential
fixture could only request a duplicate model ID and observe rejection; it did not
exercise a new lifetime at the old address. The new fixture allocates generation
1 containing 7, drops it, allocates generation 2 containing 42 at the same numeric
address, reads only generation 2 and drops it. The new allocation succeeds. Two
negative source controls read or drop the old consumed binding after the second
allocation; both reject with Q600. Address reuse does not restore its authority.

## Native storage and observations

The allocation hook creates a private anonymous mapping of one system page,
checks that the actual Buffer fits, and returns that storage to the generated C.
Dropping the first Buffer invokes real `munmap`, which must succeed. The second
allocation uses `MAP_FIXED_NOREPLACE` at the saved numeric address; it must obtain
that address exactly. It never requests replacement of an occupied mapping. An
unexpected allocation/unmap failure or address conflict aborts the fixture.
The old address is saved as `uintptr_t` before the first lifetime ends; the hook
does not dereference an expired pointer to discover reuse.

Each release hook receives the actual generated Buffer's generation before
unmapping. It checks the exact currently live pointer and requires generation 1,
then generation 2. Both mappings must be released, with no live pointer at exit.
The second mapping is a new allocation after a successful unmap, not a retained
arena block relabeled while its backing allocation remains live.

The native custody log is exact:

```text
MAP 1
UNMAP 1
MAP 2
REUSED_ADDRESS
UNMAP 2
RUN_CLEAN 2
```

`REUSED_ADDRESS` is emitted only after comparing the two observed numeric
addresses. Separately, generated native stdout matches reference execution
byte-for-byte: allocation/drop of generation 1, allocation/read/drop of generation
2, then root result 42. No task is spawned in this root-only lifetime test.
The generic CT scenario runner checks both rejection controls and the native
success path through the real source pipeline.

The same generated C passes ASan, UBSan and LSan in an untraced Linux process.
LSan cannot operate under tracing, and malloc leak checking alone would not prove
mapping release: the exact successful `munmap` calls and native live-pointer
inventory establish that bounded observation. Sanitizers are not a dynamic proof
that stale raw addresses carry the wrong generation after remapping; the source
checker rejects stale bindings, while native hooks verify distinct generations.
Generated C, fixture/header hashes, compile command, executable digest and exact
stdout/stderr are archived in `native-run.json` and adjacent compressed artifacts.

```sh
cargo test -p qcore-compiler --test ct_corpus
cargo run --quiet -p qcorec -- emit -o /tmp/qcore-reused-address.c \
  compiler/tests/fixtures/ct/CT50/case.qr
clang -std=c11 -pthread -Wall -Wextra -Werror -g -O1 \
  -fsanitize=address,undefined -fno-omit-frame-pointer \
  -include compiler/tests/fixtures/ct/CT50/reused_mapping.h \
  /tmp/qcore-reused-address.c -o /tmp/qcore-reused-address-native
ASAN_OPTIONS=detect_leaks=1 /tmp/qcore-reused-address-native
```

## Scope and validation

Coverage advances from **10/50 to 11/50 mapped**, with **39 cases unmapped**.
CT50 executes parser, checker, reference, CFG and Linux native stages. Its source
proof stage remains unassessed. Existing `OwnedTaskExecution` generation
injectivity and root-ordinal lemmas remain separate model evidence; they do not
prove the source/runtime/mapping adapter refinement. Other hosts retain explicit
unexecuted native-stage status.

All **791 workspace tests**, **31 provider tests**, **43 Python tests**, formatting,
Clippy, both reevaluation probes, Lean build and all **20 Lean audits** pass.
The Rust test count is unchanged because the new case executes inside the
manifest-driven test. All 58 contract modules are unchanged.

The full 185-unit Cycle19 source, diagnostics, chunk-contract and source-effect
artifacts remain byte-identical to baseline: **2 Safe /42 provisional Unsafe /
137 Unknown**, plus four data-only units, and **3 checked /93 Unknown** source-effect
functions. No raw unit is promoted.

This is conformance for owned Buffer generations with actual mapping reuse. It
does not establish arbitrary raw-pointer capabilities, descriptor reuse, malloc
implementation behavior, failed system calls, cross-task sharing or full
source/native refinement. The broader W6 and W2–W7 goals remain active. Existing
subagents remain stopped by their reported usage limit; this work continued locally.

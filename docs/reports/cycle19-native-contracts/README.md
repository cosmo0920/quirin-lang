# Cycle 19 native contracts and entry-free QCore modules

Evaluation date: 2026-09-06. Actual Fluent Bit revision:
`66910c10a4d7eafa810b84229cf2dcf7b0f26f97`.

The actual `include/fluent-bit/flb_mem.h` passed its native QCore contracts on
**145 instrumented executions**. Contract definitions are in
[`memory.qcore`](../../../contract/qcore/fluent_bit/memory.qcore).
The header was included from the pinned Fluent Bit checkout, using the
include/define profile of its `src/flb_input_chunk.c` compilation-database
entry. No upstream source was edited or rewritten into a substitute wrapper.

| Actual function | Checked executions | Cases |
| --- | ---: | --- |
| `flb_malloc` | 10 | 0, 1, 5, 16, 64 bytes; normal and injected allocator failure |
| `flb_calloc` | 50 | All 25 combinations of those count/size values; normal and failure |
| `flb_realloc` | 40 | Null or 1, 5, 16, 64-byte input; 1, 5, 16, 64-byte output; normal and failure |
| `flb_realloc_z` | 40 | Same matrix, including growth, shrink, equal size, and null input |
| `flb_free` | 5 | Null and each positive input extent |

[Machine-readable result](flb_mem_contract_result.json),
[all memory executions](flb_mem_observation.jsonl), and
[source/build/dependency hashes](metadata.json) retain the evidence.

## Meaning of the memory result

The QCore compiler first parses and checks the contract module. The generic
`contract_observation` interpreter then evaluates each native `requires` and
`ensures normal` clause against the corresponding runtime observations. It
rejects missing ghost values, unsupported syntax/clause kinds, overflow,
postcondition violations, duplicate case IDs, and executions outside the
preconditions. Mutating a QCore predicate changes the result in regression
tests. Function-specific postconditions are not duplicated in Rust.

The bounded predicate vocabulary is integer literals and observed ghost
names, parentheses, `+`, `-`, `*`, the six comparisons, and `and`, `or`,
`not`/`!`. Nonzero means true. Arithmetic uses checked `i128`. Runtime
observations are supplied evidence, not authenticated static certificates.
The checker never labels their acceptance as a whole-C `Safe` result.

The observed behavior is:

- Zero-sized `flb_malloc` returns null without invoking malloc.
- `flb_calloc` suppresses calloc when **size** is zero. A zero **count** with
  positive size is forwarded to libc; no contract claims it must return null.
  Successful requested bytes are observed to be zero.
- With positive requested size, both realloc wrappers forward the input and
  size once. Failure leaves the input allocation available with its bytes
  intact. Success preserves the retained prefix.
- Successful `flb_realloc_z` growth calls memset once at `old_size`, for exactly
  `new_size - old_size` bytes, using zero. Failure, shrink, and equal-size
  executions do not call memset. The newly grown bytes are checked directly.
- `flb_free` forwards its argument once and leaves no tracked live allocation,
  including on null input.

The hooks use real libc allocation storage; deterministic failure injection
returns null before calling the backing allocator. Hooks record forwarded
arguments, backend result, call counts, and allocation accounting. The harness
fills input bytes with `0xa5`, inspects preserved/zeroed bytes, and observes the
actual memset range. Allocator semantics and allocator-side ownership
accounting remain trusted. Successful realloc invalidation of the old pointer
comes from libc's contract, not a newly proved C allocator implementation.

`ordinary_allocator_profile` is enforced by compile-time rejection of
`FLB_HAVE_JEMALLOC` and `FLB_HAVE_TESTS_OSSFUZZ`. `valid_input_extent` is an
assumption satisfied by harness setup: an input is null or a live allocation
from the same allocator; for `flb_realloc_z`, the supplied old size is its
known extent and null input has old size zero. `input_live`/`live_after` count
tracked allocations. `result_is_backend` compares returned pointers before
cleanup. Prefix preservation means all old bytes on failure and the retained
`min(old_size, new_size)` prefix on success.

**Boundaries:** these are recorded executions, not exhaustive source proofs.
Zero-size realloc is excluded by `requires size > 0` / `new_size > 0`:
null from zero-size realloc must not be interpreted as preserving old custody.
Huge sizes/product-overflow cases, arbitrary invalid caller pointers,
incorrect old extents, concurrent allocator use, and other allocator/build
profiles are not covered. The fuzz-only helper `flb_fuzz_get_probability` is
absent in this selected configuration and receives no checked claim.

## Actual Monkey callback observation

The native harness includes the pinned copied `mk_event.c`, which selects its
actual `mk_event_libevent.c`. Hooks wrap real Libevent entry points and invoke
the actual Monkey callback function pointers. The runner checks the copied
adapter and every included non-generated copied source/header against Fluent
Bit; it records generated-header and linked-library hashes.

The run recorded **42 events**, with `epoll` returned by
`event_base_get_method` for both created bases. One observed OS thread executes
the recorded logical dispatcher. Callback hooks assert the matching base,
thread, live event identity, and active dispatch interval before calling the
original callback. The scenarios cover:

- a ready notification channel executing `cb_event`;
- an idle timed wait executing `cb_wait_2_timeout`;
- a timer whose reader has been closed, executing `cb_timeout` and its
  failed-send teardown branch.

Every tracked event is freed and both bases are destroyed. The deliberately
closed timer reader produces the expected `write: Broken pipe` diagnostic.
See [raw callback events](monkey_observation.jsonl) and
[stderr](monkey_observation.stderr).

This observation is not a `check_monkey_composition` certificate. It lacks the
complete source-derived identity/access trace and allocation/registration
failure coverage required by that gate. Descriptor close records also do not
currently bind descriptor generations. These remain explicit `Unknown`
obligations. A normal runtime execution does not establish race freedom or
all callback argument/map lifetimes.

## Language changes

All 15 native contract modules now begin with `module contract;` and have no
dummy `main`. `module library;` also checks declarations and supported bodies
without selecting an entry. Executable compilation retains its original
behavior. Native contract compilation still does not discharge arbitrary
contract predicates. Non-executable C library emission is not implemented;
`emit` rejects that request without writing its output.

`quant<n where n := 5>` now resolves closed local definitions and preserves
dimensioned values in the typed source IR. The implemented type subset checks
forward definitions, freshness, cycles, dimensions, natural arithmetic and
constraints. See [module and index syntax](../../QCORE_MODULE_MODES.md)
for examples, multi-file rules, API changes, and remaining type boundaries.

## Reproduce by hand

From the QCore repository root:

```sh
# Compile the native contract declarations: no main required.
cargo run --quiet -p qcore-compiler --bin qcorec -- check \
  contract/qcore/fluent_bit/memory.qcore
cargo run --quiet -p qcore-compiler --bin qcorec -- check \
  contract/qcore/language/quantified_indices.qcore

# Build and execute actual upstream source under allocator/callback hooks.
python3 scripts/cycle19_native_observations.py \
  --fluent-bit /home/cosmo/Documents/GitHub/fluent-bit \
  --monkey-build /tmp/qcore-cycle19-monkey-build-codex-20260905 \
  --output /tmp/qcore-cycle19-native

# Re-evaluate the recorded runtime values against the current native clauses.
cargo run --quiet -p qcore-compiler --example check_contract_observations -- \
  contract/qcore/fluent_bit/memory.qcore \
  /tmp/qcore-cycle19-native/flb_mem_observation.jsonl

# Re-run the separate typed-source and generic C audits.
cargo run --quiet -p qcore-compiler \
  --example cycle19_fluent_bit_libraries_audit -- \
  /home/cosmo/Documents/GitHub/fluent-bit \
  /tmp/qcore-cycle19-monkey-build-codex-20260905/compile_commands.json
```

The native runner requires Clang with the sanitizer runtimes, the recorded
Fluent Bit build database, and the separate forced-Libevent Monkey build.
Omit `--monkey-build` for memory-only evaluation. The observation executables
need ordinary Unix socketpair access and a runtime where LeakSanitizer can
inspect the process; the restricted sandbox prevented these operations, so
the successful recorded native run used the approved unsandboxed execution.
No network or upstream write is needed.

## Validation and remaining Cycle 19 gates

- 280 Rust tests passed: 246 unit, 19 provider, 1 Libevent source pin,
  5 module mode, 2 native corpus, and 7 quantitative-binding tests.
- Formatting and Clippy with warnings denied passed.
- Lean build and Cycle 19 dependency audit passed. The new substitution,
  scope-preservation, and bounded-value theorems contain no placeholders or
  project axioms. These prove the elaborated core account, not the Rust parser
  or its dependency resolver.
- Native harnesses passed AddressSanitizer, UndefinedBehaviorSanitizer, and
  LeakSanitizer. The harness and textually included source are instrumented;
  linked prebuilt Monkey/Libevent archives retain their original build flags.
- Pinned external source/build checks and audit passed. The two native field
  contracts remain checked over four Boolean pre-states each. Every other
  selected lifecycle function still has an explicit `Unknown` result.
- Generic audit totals remain 185 units, 4,346/4,775 functions projected,
  2 `Safe`, 54 provisional `Unsafe`, 125 `Unknown`, 4 data-only units, and
  zero gateway failures. Provisional `Unsafe` counts are not confirmed defects.

Logs: [Rust tests](cargo-test.log), [Clippy](clippy.log),
[formatting](fmt.log), [Lean build](lean-build.log),
[Lean audit](lean-audit.log), and [external source audit](source-audit.log).

Cycle 19 remains in progress. The outstanding implementation is general
19B per-call ref/list/queue/arena identity mapping; 19C source imports through
the nested teardown chain and remaining wrapper/failure paths; and 19D
source-derived callback/access traces, descriptor identity binding, and
allocation/registration failure coverage. This run supplies actual allocator
and callback evidence without promoting those unresolved gates to complete.

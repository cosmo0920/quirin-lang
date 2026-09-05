# Cycle 19: source-derived Monkey callback access paths

On 2026-09-06 the forced-Libevent build at Fluent Bit revision
`66910c10a4d7eafa810b84229cf2dcf7b0f26f97` imported all three actual Monkey
callbacks through Clang's typed AST. This advances 19D's source-access import;
**the results are symbolic source projections, not safety certificates**.

| Callback | Retained paths | Source effects |
| --- | ---: | --- |
| `cb_event` | 4 | Both alternatives of the read/write flag tests; argument → map → context reads; fired-slot `fd`, `mask`, `data` writes; `fired_count` increment |
| `cb_timeout` | 3 | Send success/non-error alternative and both error-query alternatives; send buffer address/size; descriptor close, event deletion/free, map free calls in source order |
| `cb_wait_2_timeout` | 1 | Return with no access or call |

The flag tests use this build's actual constants: Libevent read bit 2 maps to
Monkey read bit 1; write bit 4 maps to bit 4. On this Linux build,
`evutil_socket_geterror(fd)` expands to `*__errno_location()`. The importer
retains that call and memory read rather than fabricating a descriptor query.
`cb_timeout`'s cleanup calls are preserved even though their return values are
ignored. A call to `evutil_closesocket` alone does not establish close success
or discharge descriptor custody.

## Import and identity boundary

`verifier_plugins::libevent::monkey_source::import_monkey_callbacks` accepts the
selected `C11TranslationUnit` and pinned revision assertion. The external audit
checks Git identity, tracked source cleanliness, the forced backend definition,
copied translation units, and the textually included adapter and its core
layout/macro headers. The underlying Clang transport is shared with the
existing ChunkIO field importer; its behavior is unchanged.

The bounded grammar retains declarations, pointer casts/aliases, ordinary
field and indexed accesses, integer casts and operators, fixed-scalar
`sizeof`, conditionals, supported direct calls, assignments, increments,
returns, and local storage lifetime exits. Alias values refer to callback
parameters or earlier read/call symbols, rather than variable names. Each
memory read has a fresh typed symbol, including rereads after opaque calls.
Compound assignment evaluates its lvalue once. Both branch alternatives remain
in the report, including alternatives whose feasibility has not been proved.

Every statement is parsed before path expansion. Unsupported syntax, loops,
jumps, globals, indirect or unlisted calls, attributes, volatile/atomic or
bitfield accesses, absent definitions, signature mismatches, uninitialized
local reads, and ambiguous call/read ordering remain `Unknown`. Opaque calls
and indirect writes invalidate cached address-taken locals. Size, depth,
operation, path, and symbolic-substitution limits also fail closed.

These paths do not establish pointer provenance, live/bounded storage, integer
arithmetic validity, call contracts, synchronization, or runtime access
refinement. Pure ordinary reads are serialized in AST order under an exclusive
access assumption; a call competing with another read/call is rejected. A
projected dangling access or ignored failure is not accepted as safe. Record
layout and system/build headers, Clang, preprocessing, and the AST bridge remain
explicit trusted inputs. Supplying a revision string to the library API does
not authenticate source.

The source-path report is separate from runtime observation schema v1 and
verifier protocol v1. It is not accepted by `check_monkey_composition` or the
plugin certificate firewall. Native QCore callback contracts remain separate
obligations; source import alone does not prove their predicates.

## Evidence and validation

- [Full typed paths and selected compile command](monkey_callback_source.json)
- [Pinned external audit](source-audit.log)
- [Source, build, tool and artifact hashes](evidence.json)
- [Rust test log](tests.log), [Clippy](clippy.log), [formatting](fmt.log)
- [Lean build](lean.log), [Cycle 19 dependency audit](lean-audit.log)
- [Native run metadata](native-metadata.json), [allocator result](flb_mem_contract_result.json)
- [Descriptor observations](monkey_observation.jsonl), [runtime result](monkey_identity_result.json), [native stderr](monkey_observation.stderr)

All **300 workspace Rust tests** pass. The 12 new regressions cover actual
callback bodies, all branch alternatives, alias renaming, field/call/condition
mutations, unsupported unreachable syntax, signature/layout mismatches,
unsequenced effects, escaped locals, local lifetimes, early return, single
lvalue evaluation, import limits, and invalid symbolic references.

`QCore.CallbackSource` adds seven audited theorems for prior typed producers,
forward-reference rejection, exact producer append, wrong-type/nonproducer
rejection, and retention of both branch alternatives. They formalize the
reference/guard projection, not the Rust parser, C semantics, or source/runtime
refinement. The dependency audit has no placeholders or project axioms.

The native regression still checks 145 allocator executions and the 48-record
Monkey observation with all three callbacks, four descriptor acquisitions and
closes, two reused descriptor numbers, and one injected socketpair failure.
The harness passed address, undefined-behavior, and leak sanitizers; linked
static archives retain their original instrumentation. Expected stderr is
`socketpair: Too many open files` and `write: Broken pipe`.

The pinned audit remains at **185 units, 4,346/4,775 functions projected,
2 Safe, 54 provisional Unsafe, 125 Unknown, four data-only units, and zero
gateway failures**. The three included callbacks are reported separately;
they do not increase the generic projection numerator or proved coverage.
The two existing native ChunkIO/wrapper field contracts still check.
Provisional Unsafe classifications are not confirmed upstream defects.

## Reproduction and remaining gates

```sh
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  ../fluent-bit \
  /tmp/qcore-cycle19-monkey-build-codex-20260905/compile_commands.json \
  /tmp/qcore-cycle19-callback-source.json

python3 scripts/cycle19_native_observations.py \
  --fluent-bit ../fluent-bit \
  --monkey-build /tmp/qcore-cycle19-monkey-build-codex-20260905 \
  --output /tmp/qcore-cycle19-callback-native
```

The optional third audit argument writes the callback source report. Recreate
the disposable forced-Libevent build using the cycle validation instructions
if its recorded path is unavailable. Native sanitizers and socketpair require
an execution environment that permits their operation.

Cycle 19 remains in progress. Next, bind source read/call symbols and guards to
observed payload, event-map, context, fired-slot, and descriptor identities;
check the native QCore callback predicates under those bindings; and import
allocation/registration failures. General 19B custody-call mappings and 19C
nested source teardown/failure imports also remain open. No new whole-source
or event-composition safety certificate is issued by this increment.

# Cycle 7: Cleanup-Explicit CFG and C11 Emission

**Status: Done**

Cycle 7 implements Stage 7 of the prototype roadmap for the current non-MSVC
host toolchain. It lowers a checked,
linear source-path IR into a typed control-flow graph with edge-specific
cleanup blocks, then emits standalone C11 with a tagged error ABI.

## Phase 0: warm-up prerequisite

Before Cycle 7 implementation begins:

- mark Cycle 6 as done;
- move the Cycle 6 record to `docs/cycles/`;
- update repository documentation to point at Cycle 7 as the active cycle.

## Executable Rust semantics

The Rust core provides:

- a validated error ABI with stable, nonzero numeric tags;
- typed basic blocks, instructions, explicit branch edges, and terminators;
- an independent CFG interpreter that observes values, error tags, ownership
  transfers, and acquire/release traces;
- a checked source-path IR for fallible acquisition, fallible checks, explicit
  drop, ownership transfer, and integer return;
- cleanup lowering that creates a distinct failure label for every fallible
  edge;
- path-specific cleanup instructions containing exactly the resources live
  before that edge, in reverse acquisition order;
- rejection of unknown or duplicate resources, duplicate errors, invalid C
  identifiers, normal-path leaks, operations after return, and fallible work
  after ownership transfer;
- a C11 emitter with fixed-width values, tagged error enum and payload union,
  explicit gotos, ownership-move markers, and initialization slots;
- an observation parser for translation validation;
- integration tests that use the Rust `cc` crate to compile emitted programs
  with strict C11 warnings, AddressSanitizer, and UndefinedBehaviorSanitizer;
- differential checks showing that the source interpreter, CFG interpreter,
  and compiled C agree on normal values, error tags, transferred identities,
  and resource traces.

The emitted test harness allocates a byte per abstract resource so sanitizer
runs exercise real allocation, transfer, and release behavior. LeakSanitizer is
disabled at execution because it cannot operate under ptrace-based process
supervisors; every returned allocation is nevertheless freed by the harness,
and AddressSanitizer and UndefinedBehaviorSanitizer remain active.

Native Windows/MSVC compilation is not part of Cycle 7 acceptance. The current
GNU/Clang-style diagnostic and sanitizer flags are intentionally not passed to
MSVC. Windows ABI, compiler, runtime, and generated-code conformance are
recorded as the postponed [Cycle 7.5 plan](docs/CYCLE_7_5.md).

## Surface-language maturity

Cycle 7 does not claim a complete human-facing language. The current parser
accepts the early declaration fragment for dimensions and quantitative axes.
Ownership, errors, regions, and C lowering are exposed as checked Rust IR APIs.
A full function grammar, elaboration into these passes, a command-line compiler,
and stable source syntax remain future work.

## Lean verification obligations

`QCore/Lowering.lean` defines source-path observations, cleanup-explicit CFGs,
an emitted-C abstract machine, and observable resource events. It proves:

1. source observations are preserved by cleanup-CFG lowering;
2. every error edge receives a cleanup label;
3. partial initialization produces exactly the initialized path's cleanup;
4. cleanup releases exactly the live identities;
5. cleanup order is reverse acquisition order;
6. a duplicate-free live ledger yields duplicate-free releases;
7. CFG observations are preserved by C emission;
8. source-to-C observable simulation follows by composition;
9. normal integer values and ownership transfers are preserved;
10. error tags and cleanup traces are preserved.

No theorem contains `sorry`, `admit`, or a project-defined `axiom`.

## Complete verification instructions

From the repository root:

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
if rg -n '\b(sorry|admit)\b|^[[:space:]]*axiom\b' proof \
  --glob '*.lean'; then exit 1; fi
cd proof
lake build
lake env lean AuditCycle7.lean
```

On non-MSVC hosts, the Rust test suite invokes the `cc` crate on four emitted-C
corpus programs. The compiled-C test reports the Cycle 7.5 deferral instead of
trying to pass incompatible flags when built for native MSVC.
It requests `-std=c11`, `-Wall`, `-Wextra`, `-Werror`, `-pedantic`,
`-fsanitize=address,undefined`, and `-fno-omit-frame-pointer`, then executes and
compares each binary's observation with both Rust interpreters.

`AuditCycle7.lean` prints the foundational dependencies of every Cycle 7
theorem. They are limited to Lean's standard `propext` and `Quot.sound` where
list simplification and decidable equality require them.

## Verified completion result

Completion is verified by 69 passing Rust tests, including four compiled-C
translation-validation cases, warning-free strict Clippy, a successful 12-job
Lean build, a source audit with no proof placeholders or project axioms, and a
Cycle 7 theorem-dependency audit reporting only the standard foundational
dependencies described above.

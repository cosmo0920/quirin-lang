# Independent statement execution in the ChunkIO process

2026-09-07, baseline `70fb813`. ChunkIO package 0.3.0 computes Boolean-field
execution paths from a typed statement program. The previous process received
completed paths and copied them into a proposal; requests now contain no expected
paths. Trusted admission still compares the result with the original source
import and independently evaluates the pinned contract.

## Implementation and boundary

`qcore-schemas::chunk_summary` publishes data-only expression and statement
records. The expression grammar covers i32 constants, the modeled field, logical
not, checked negation and equality/inequality. Statements cover blocks, branches,
Boolean assignments, explicit returns and independently validated helper writes.
`BooleanFieldSourceInput` carries version 2, the program and source/model metadata.
The semantic profile is `qcore.c_boolean_field_contract.v2`; the task input digest
key is also version 2. Old result-bearing input records reject. Protocol v1's
permanent Safe refusal and ordinary evidence-v2 requests without this input stay
unchanged. The witness checker version stays v1 because admission semantics do
not change.

The gateway retains the typed program inside its private, opaque source summary.
Its original raw summary JSON excludes this added field. Conversion preserves
every supported statement and expression; `parity.py` verifies 14 trusted
importer/evaluator functions against the baseline, allowing only the extra
`statement.record()` capture at summary construction. Clang import, ABI/layout
checks, helper source validation, model selection and identity binding remain
trusted gateway responsibilities.

The provider has its own interpreter and depends only on schemas and
serialization libraries. It computes false/true initial-field traces, including
ordered repeated writes, taken branches, metadata writes and early-return
behavior. It rejects missing returns, overflowing negation and non-Boolean field
stores. Structural preflight visits unreachable nodes as well: depth is limited
to 128 and program nodes/frame writes to 16,384. Execution has a shared budget of
32,768 operations across both initial states. These are conservative producer
limits, not additional source-safety claims; failure yields Unknown.

The receiver's source/build/profile/helper identity and full program digest
remain bound to the task. Proposed paths must match its independently computed
source summary exactly, after which the contract evaluator checks two source
prestates times two companion-field values. A malicious or faulty producer
cannot choose a weaker denominator or postcondition. Live borrowed storage,
exclusive access, Boolean domains, Clang import and declared profile/policy trust
remain explicit assumptions. The process does not prove arbitrary caller validity.

## Validation

The standing gate passes: 611 workspace tests, format, Clippy, both reevaluation
probes, Lean build and 19 axiom audits. All 43 Python validator tests pass. The
excluded provider workspace passes nine tests: four process/package conformance
tests and five independent-interpreter tests, plus format and Clippy.

Process conformance executes actual Clang-imported C bodies exercising every
expression constructor and branch/write/return behavior. It checks that requests
have no output paths, rejects old/changed input versions and forged programs,
rejects stale source/build tasks, and rejects invalid postconditions. An initial
fixture using a const contract for idempotent writes correctly rejected; the
final tests require a mutable borrow for those bodies and also assert that their
original const contract still rejects. Production unlock additionally exercises
validated helper metadata writes through the real process.

Existing Lean source-import/witness theorems quantify over arbitrary proposed
paths, so this producer implementation adds no trusted semantic rule. The source
import and replay meanings are unchanged; all axiom audits still pass. No proof
of the Rust producer, Clang parser or general source/runtime refinement is claimed.

Two separate target-directory builds produce identical executable, manifest and
archive hashes for all five packages. The indexes and comparison are archived.
The first build preceded a comment correction in the producer source; a final
rebuild and audit use the matching final artifacts. Earlier artifact identities
are not used as the final package evidence.

## Reproduction and remaining work

The final package set is retained at `build/cycle19/provider-packages`. On a fresh
checkout, first build it with `python3 -B providers/package.py build --output build/cycle19/provider-packages`.
The builder refuses to overwrite existing output directories.

```sh
python3 -B providers/package.py verify build/cycle19/provider-packages/qcore.chunkio
QCORE_CHUNKIO_PACKAGE="$PWD/build/cycle19/provider-packages/qcore.chunkio" \
  cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  /home/cosmo/Documents/GitHub/fluent-bit \
  build/cycle19/monkey-build/compile_commands.json \
  build/cycle19/reports/source-statement-source.json \
  build/cycle19/reports/source-statement-diagnostics.json
```

This moves finite-state path computation across the real plugin boundary. General
lifecycle import, remaining specialized provider analysis, registry-based full
corpus composition and route deletion remain open. It does not close all of W5,
Cycle 19 acceptance or full QCore r0.3 conformance. Direct trusted evaluation is
retained for replay and parity; no raw translation unit is promoted.

## Final corpus result

The final 185-unit audit retains byte-identical source diagnostics, resource
diagnostics and the existing contract sidecar: 2 Safe / 47 provisional Unsafe /
132 Unknown plus 4 data-only units. The process sidecar retains the same selected
function inventory and statuses: three checked contracts and 93 Unknowns. The
checked functions are `cio_chunk_is_locked`, `cio_chunk_unlock` and
`flb_input_chunk_release_lock`, each checked over four contract-state combinations.
Their certificates now bind the version-2 statement input/profile and package
0.3.0. `audit-parity.json` records the comparison and exact hashes.

# PROVIDER-PY-DUAL-STEM-FIXTURE-STALE

Base `3ba723201775e0a22e757e3572f00eb124481c40` (main, 2026-09-28). Test-only
repair in `providers/tests/test_contract_load_path.py`; no production code,
contract, pin or Lean change.

## Cause

On the base, `python3 -m unittest discover -s providers/tests` ran 24 tests
with one failure:
`test_dual_extension_same_stem_is_refused_before_manifest_selection`,
`ValueError not raised`. The registered diagnosis is confirmed. Before
f4018a92 ("Migrate contract sources and lock paths to .qv"), the `Tree`
fixture created `libc/memory.qcore` and the test wrote `libc/memory.qv`
beside it. The migration rewrote both paths to `.qv`. As a result, the test
overwrote the fixture's own file, and no second extension spelling of the
stem ever existed. Production still refuses a real pair:
`providers/contracts.py` `files()` raises `ambiguous contract asset stem`,
and `providers/package.py` `reject_dual_stems` does the same. This is not a
fail-closed defect.

## Fix

- The dual-stem test now asserts that the fixture owns `libc/memory.qv`. It
  then writes the legacy spelling `libc/memory.qcore` beside it, after
  asserting that the file is absent, and expects the refusal from
  `load_path_manifest`. This matches EXT-01's rule that
  [alias collisions refuse](../extension-compatibility/README.md) rather than
  selecting by order.
- Stale-pattern search of f4018a92: a script (worktree-private, not shipped)
  listed each rewritten `.qcore` literal in a test file whose `.qv` spelling
  already existed in that file before the commit. It found five. Four are in
  this file:
  - `busy_state` (fixed here): the migration collapsed
    `test_exact_owner_selector_admits_new_spelling_without_broadening_owner`
    onto one spelling. Its `.qcore` selector became `.qv`, and two of its
    assertions became identical. The test still passed, but it no longer
    exercised the legacy-selector admission branch of `contracts.owned`. The
    test now restores the `.qcore` selector, which admits its `.qv` spelling,
    and adds the converse: a `.qv` selector does not admit `.qcore`.
  - `memory.qv` in the dual-stem test: the fixed failure.
  - `quirin-provider-libc:libc/memory.qcore` and
    `quirin-provider-libc/contracts/libc/memory.qcore` in
    `test_qv_published_module_and_include_pin_keep_exact_paths`: left
    unchanged and reported. The test deletes and re-creates the fixture's
    `memory.qv`, and `CHILD.replace("memory.qcore", "memory.qv")` now changes
    nothing. It still asserts the exact `.qv` paths. It has lost only its
    original legacy-to-new transition, and `test_generated_from_metadata_and_constants`
    already covers those paths on the default fixture. This is redundant, not
    a weakened refusal.
  - `compiler/tests/extension_compatibility.rs`: not stale. The manifest now
    names `buffer_lib.qv`, and `manifest.replace(new, reference)` still builds
    the stale `.qcore` reference that the test expects to be refused.

  The fixture writes that follow the search (`memory.qv` in the stale-lock
  and changed-parent tests) overwrite the file on purpose.
  `test_package_contracts.py`'s dual-stem bundle defaults to `.qcore`, so its
  `.qv` sibling is a real pair.

## Mutation evidence

The harness is [`scripts/mutation.py`](scripts/mutation.py), and the
per-mutation logs are in [`mutation-logs/`](mutation-logs/). It rewrites
`providers/contracts.py` bytes, runs the one detecting test, restores the
bytes and asserts the round trip. It then runs a green baseline after the last
mutation. Result: exit 0, **2/2 killed**, baseline exit 0.

| Mutation | Change | Detecting test | Result |
|---|---|---|---|
| M1 | `if stem in stems:` → `if False and stem in stems:` in `files()` | dual-stem test | exit 1, `ValueError not raised` |
| M2 | removes the `.qcore` selector → `.qv` admission branch of `owned()` | owner-selector test | exit 1, `False is not true` |

The pre-fix owner-selector test would survive M2, because its selector ended
in `.qv` and matched by equality. The pre-fix dual-stem test failed with or
without M1.

## Records

[`scripts/update_ledger.py`](scripts/update_ledger.py) retires
`PROVIDER-PY-DUAL-STEM-FIXTURE-STALE` when it is registered. The script is
idempotent. On this base the entry is absent, because W47-CALLEE-CLOSURE-R2,
which registers it, is not yet on main. The run reported
`523 -> 523 ... not registered`, so the discharge is recorded here only. The
integrator reruns the script after both cards are merged. A scratch copy with
the entry injected was used to exercise the retire branch: 524 → 523, and a
second run left the count at 523.
`build_matrix.py . --gate docs/reports/runtime-source-imports/gate.json`
(the matrix's current gate provenance) leaves the matrix byte-identical.
A run without `--gate` would only rewrite `measurement_provenance` to
"carried", and was discarded.

## Gate

The following commands were run at the card tip with `CARGO_INCREMENTAL=0`.
Each exit was captured to a file under the private
`private/dual-stem-gate/`. The provider gate applies because `providers/`
changed. The core Rust gate was not run, because no non-test code outside
`providers/tests` changed.

| Command | Exit | Count |
|---|---|---|
| `python3 -m unittest discover -s providers/tests` | GATE_PU | GATE_PU_N |
| `python3 providers/contracts.py check` | GATE_CC | — |
| `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline` | GATE_CT | GATE_CT_N |
| `cargo fmt --manifest-path providers/Cargo.toml --all -- --check` | GATE_FMT | — |
| `cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings` | GATE_CLIPPY | — |
| `python3 -m unittest discover -s scripts/tests` | GATE_SU | GATE_SU_N |
| `python3 scripts/build_docs_index.py --check` | GATE_IDX | — |

Base failure, for comparison: `providers/tests` ran 24 tests with 1 failure
(exit 1).

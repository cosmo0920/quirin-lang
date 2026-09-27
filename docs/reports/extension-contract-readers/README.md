# Extension contract readers (EXT-01 delegated track)

Base: `64ddac3f60fe9b848dd789e895d67b809cce93e2`.

## Prediction before implementation

Compatibility-only acceptance should leave the existing contract corpus and
published-provider inputs byte-identical. I predict zero changes to pinned
contract counts, source identities, lock contents, verdicts, and emitted C for
existing `.qcore` inputs. New `.qv` temporary copies should load in contract
mode and retain their explicitly spelled path identities. A `.qv` file in a
runtime/library mode and a `.qr` file in contract mode should refuse. When
same-stem `.qcore` and `.qv` entries coexist in a discovery scope, the reader
should refuse rather than prefer either spelling. Explicit references and pins
should never fall back to the other extension when the named file is missing.

The reader will warn on legacy `.qcore` loads, including frozen archive inputs,
with path, parsed mode, and `.qv` recommendation. The warning is informational
and should not move a verdict. I expect no Lean change because filename
classification does not alter source semantics or proof obligations.

## Implementation and measured boundary

Contract load paths, package inventories, includes, corpus roots and audit pin
directory discovery now admit `.qv` alongside `.qcore`. Each retains the exact
spelled module path and digest. Discovery and selected source sets refuse a
same-stem legacy/current pair. No reader substitutes `.qv` for a missing
explicit `.qcore` pin or reference. Parsed module headers are checked at the
load path, bundle, corpus and source-loader boundaries; a `.qv` source must be
`module contract;`. Legacy loads emit `warning[legacy-source-extension]` with
the path, parsed mode and recommended `.qv` suffix. Existing error surfaces
are `String`/`ContractInclusionError` and contract maps, without a structured
warning field, so this track emits stderr and does not change serialized
verdicts or machine-readable result schemas.

The compatibility implementation has no Lean semantic delta: it gates file
selection by parser-derived mode, while contract text, inheritance,
declarations and proof obligations remain unchanged. No tracked sources or
frozen archives were renamed.

## Validation

The delegated worktree initialized `third_party/libevent` at
`d82464a277d0f42703702c4dfd9af6af38595a83` (exit 0). The focused first
attempt (`private/extension-contract-readers/focused.log`, exit 101) caught an
inferred unsized map key in the new corpus collision check; it was corrected.
The next focused run (`focused-2.log`, exit 0) passed four tests. The final
contract regression command was:

```sh
CARGO_TARGET_DIR=/home/cosmo/.codex/worktrees/audit-checkpoint-batches/qcore-c/target \
  CARGO_BUILD_JOBS=4 cargo test -p qcore-compiler \
  --test contract_loader --test contract_inherit_adoption \
  --test extension_contract_compatibility
```

It exited 0: 22 passed, 0 failed, 0 ignored, in
`private/extension-contract-readers/contract-regression.log`. The full standing
gate is assigned to the integrator on its warm target because a concurrent
fresh-target gate exhausted disk quota; this delegated track does not claim
the integrated gate passed.

The compatibility reader retains `.qcore` even in frozen reports. Warning
emission at a frozen path is the same path-based behavior as at any legacy
path; the integrated CLI warning tests and full standing gate belong to the
EXT-01 integrator. Follow-on EXT-02 path migration must update exact pins and
references deliberately and compare declaration/source identities.

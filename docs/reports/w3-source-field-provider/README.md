# Source-derived field contracts through the ChunkIO process provider

2026-09-07. Baseline `de5b239`. This increment connects an existing real Clang
source importer to evidence-v2 dispatch, a packaged provider and trusted replay.
The property is a bounded borrowed Boolean-field function contract. It is not
whole-unit custody, general lifecycle safety, caller provenance or synchronization.

## Trusted import and identity

The typed Boolean-field importer now resides in `qcore-c-gateway::boolean_source`.
Its library profile, revision and acceptance-policy text are explicit inputs.
The compiler facade retains the default selection, pinned revision, public
methods, model instantiation and raw JSON shape. Neither raw schema records nor
JSON can construct the opaque source/helper summaries. `parity.py` verifies
14 importer/evaluator/instantiation functions against the baseline after the
listed provenance and parameter substitutions. The Boolean execution grammar
and existing contract evaluator are unchanged.

Private, serialized-only identity snapshots bind raw and preprocessed source,
configuration (dialect, definitions, includes, scope and working directory),
actual Clang description/default target, policy text, profile/selection,
actual model owner and each independently checked helper's identity. Compiler
description changes during an import reject. Raw summary serialization excludes
this private metadata to preserve the existing report surface. Revision is
still an outer-audit assertion; the production audit retains its pin/clean-tree
checks. Clang import and declared profile/policy are explicit trusted premises.

Task derivation accepts only an opaque import, checks actual dialect/model owner,
requires a native target profile, and rejects helpers with different compiler
targets. The record-layout/signature checks remain part of import. A real
alternate-target helper regression demonstrates why matching raw records alone
cannot establish compatible source/build identity.

## Property, protocol and replay

`BooleanFieldContract` is a new, explicit function-scoped obligation. The task
binds the complete source identity, restricted projection and exact contract
text. Both Boolean source prestates are evaluated together with both unchanged
companion-field values: four contract-state combinations. Live borrowed storage,
exclusive access and both Boolean domains remain explicit caller preconditions.

The optional evidence-v2 `source_effect` request field is omitted for existing
exchanges. V1 serialization and its permanent Safe refusal are unchanged. The
host binds the source input digest to the core task before dispatch. The new
`BooleanFieldContract` witness supplies complete source paths, including branch,
return, repeated write, frame-write, helper and profile data. Replay requires the
original sealed context, exact task and complete paths, then independently
executes the pinned contract evaluator. Equality with a proposal alone cannot
certify a false postcondition. Empty/missing source inventories do not create
this task; unsupported selected functions remain explicit Unknown audit rows.

The actual `qcore.chunkio` executable at package version 0.2.0 proposes this
witness for the named C source-field profile. It copies the restricted imported
path data into a proposal; trusted import and contract evaluation remain in the
receiver. This is process admission/integration, not provider-owned source
analysis. Its normal dependencies remain schemas/serialization only. Existing
capabilities and generic v2 requests remain Unknown. The added `fluent_bit`
target uses the canonical profile library ID, distinct from the audit's display
label `fluent-bit-chunk`. The built-in manifests/source routes remain unchanged.

The production audit uses explicit `QCORE_CHUNKIO_PACKAGE` selection, verifies
that package and dispatches each checked function through the real process and
firewall. Missing, disabled, incompatible, failed or unverified proposals have
no fallback in that selected mode. The direct evaluator is retained and compared
for parity. Its existing source/contract output stays separate from the new
`.source-effects.json` sidecar, which includes checked witnesses, tasks, package
identity, bounds and all unsupported function rows. No raw unit is promoted.

## Reproduction

From the repository root, verify and use the durable package set. On a fresh
checkout, first run `python3 -B providers/package.py build --output build/cycle19/provider-packages`; the builder rejects existing output directories:

```sh
python3 -B providers/package.py verify build/cycle19/provider-packages/qcore.chunkio
QCORE_CHUNKIO_PACKAGE="$PWD/build/cycle19/provider-packages/qcore.chunkio" \
  cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  /home/cosmo/Documents/GitHub/fluent-bit \
  build/cycle19/monkey-build/compile_commands.json \
  build/cycle19/reports/source-effect-source.json \
  build/cycle19/reports/source-effect-diagnostics.json
```

The final audit used `source-effect-packages-v3`, then the verified package set
was promoted to durable `provider-packages`, after fixing an unsorted
manifest list and correcting the canonical library ID. Earlier incomplete
attempts are not counted as successful evidence. The existing persistent Monkey
source/build inputs remain in place.

## Tests and remaining scope

New tests reject forged/swapped paths, returns, branches, writes, frame effects,
helper/profile data, source/build/contract changes, target/library substitutions,
missing coverage, missing checkers/providers and false postconditions. An isolated
child uses real native and WebAssembly-target Clang imports to reject incompatible
helper targets without changing the parent test environment. Production input
fixtures retain aliases, loops, incompatible ABI/layout and unsupported functions
as Unknown. Existing v1/package-mutation tests remain, including a changed-version
mutation that stays distinct from the new 0.2.0 package version.

Lean connects exact witness agreement and the existing contract checker to both
Boolean source prestates, assuming the source statement import. It rejects changed
paths/labels and failed contracts. It does not prove Clang/contract parsing,
cryptographic identity, arbitrary caller validity or runtime source refinement.

General constructor/failure/backend/residency/close-flag lifecycle import remains
open; its unconditional incompleteness marker is preserved. Specialized provider
source analysis, remaining provider extraction and full registry-based corpus
composition remain open. This bounded field-contract claim must never replace
those deliverables or be presented as full QCore revision 0.3 conformance.

## Validated result

All standing gates pass: 611 workspace tests, 43 Python validator tests, format,
Clippy, both reevaluation probes, Lean build and 19 axiom audits. The excluded
provider workspace adds four conformance tests and passes format/Clippy.
`validation.json` and compressed logs record these checks. The read-only
subagent review found no additional material correctness issue.

The complete 185-unit audit remains 2 Safe / 47 provisional Unsafe / 132 Unknown
plus 4 data-only units. Source, diagnostic and existing contract JSON are
byte-identical to the baseline; hashes are in `audit-parity.json`. The new
sidecar contains three checked function contracts (`cio_chunk_is_locked`,
`cio_chunk_unlock`, `flb_input_chunk_release_lock`) and 93 Unknown functions.

Two fresh target-directory builds produce identical executables, manifests and
archives for all five packages. `packages.json`, `packages-repro.json` and
`package-parity.json` retain their identities. The builder's index label now
says `per_package_declared_profiles` instead of the stale `unknown_only_transport`;
this metadata correction leaves all audited package artifacts unchanged. The
corrected package set is available at `build/cycle19/provider-packages`.

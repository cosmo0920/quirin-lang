# Runtime source imports

Implementation base: `3cb923d9a5707aafe88541e6a48b2fae5db8fcdb`, after the
physical extension migration and the explicit contract-load-path CLI feature.

## Prediction recorded before implementation

Canonical standard imports become `#import std.fmt;` and retain bare standard
import compatibility. Quoted `#import "./helper";` and `#import "./helper.qr";`
resolve the same runtime module. Existing source routes, standard capabilities,
contract loading, proof obligations and native observations remain unchanged for
programs without runtime imports. No pinned C-corpus verdict or artifact change
is expected. The parent integrator owns the pinned before/after arbiter and full
standing gate; this card records focused implementation checks only.

The ordinary compile APIs resolve supplied snapshots only; an explicit disk
entry point used by the CLI loads absent dependencies. Roots are explicit in
the new bundle API. Imports contribute global declarations, not namespaces or
capabilities. Unmarked and library dependencies are admitted; explicit
executable and contract dependencies refuse. Missing, ambiguous, cyclic,
oversized or unsupported imports refuse. Source snapshots and diagnostic path
spellings remain unchanged; only derived staging blanks resolved directives
and mode headers byte-for-byte with newlines retained.

Rust controls will cover graph closure and ordering, repeated/diamond imports,
cycles, aliases, snapshot precedence, file modes, capability isolation, exact
source locations, CLI dispatch and task/epoch replay. Matching audited Lean
will describe finite bundle admission and per-file permissions. Filesystem
identity, parser extraction and Rust-to-Lean correspondence remain declared
boundaries, not consequences of the abstract model.

## Implemented boundary

Implementation commit: `d6f46d4c`; prediction commit: `6066c5ed`.

`qcore_frontend::frontend::inspect_source_imports` returns only structural
mode/header/directive metadata. The ordinary parser refuses unresolved quoted
imports, including nested directives, so no frontend semantic consumer gains a
route for ignoring a dependency. Standard directives preserve their complete
original span; bare standard imports remain compatible. The between-function
declaration boundary recognizes both spellings. One deliberate bare import
remains in the compatibility control; other usage in the owned frontend,
verification and CLI trees and the three normative specifications is canonical.
The parent integrator owns the wider current/historical syntax migration and its
original-byte attribution manifest; old measurements are not rerun by a syntax
edit or relabeled as measurements of the new bytes.

`source_imports::resolve_source_bundle` separates explicit root names from a
supplied snapshot pool. Existing compile APIs pass all their inputs as roots
and remain snapshot-only. `compile_source_bundle_with_registry` exposes the
explicit pool/loading/bounds choice; the filesystem compile wrapper is used by
CLI check/emit, after the separate contract-load-path dispatch. Supplied
snapshots take precedence and duplicate lookup identities refuse even when the
text agrees. The supplied pool is bounded even if some entries are unreachable.
All roots retain caller order; dependencies are appended in source-order DFS
first-discovery order. Completed-node order is independently checked against
every edge before publication. Diamonds and repetitions select each node once;
cycles, unsupported paths, absent targets, identity ambiguity and exhausted
limits refuse. Maximum limits are 1024 modules, 16 MiB source bytes and recursive
traversal depth 128, with caller-selected tighter bounds only.

Snapshot-only keys are lexical virtual paths, without disk or current-directory
reads. Filesystem keys resolve existing paths and parents before considering
parent steps; the actual open uses the uncollapsed candidate, preserving
`symlink/..`. An unresolved parent with `..` refuses. Loaded files must be regular
UTF-8 files and each is read once. A changed canonical key after the read
refuses, but this is not an atomic filesystem snapshot or protection against
concurrent in-place byte changes. Hard links are distinct canonical path keys;
there is no inode/hard-link identity claim. Retained evidence binds the exact
selected text and resolved edges, not filesystem or execution attestation.

Dependencies admit an unmarked or explicit library file and refuse explicit
executable/contract mode. Roots alone select combined entry intent. Ordinary
namespace, duplicate-name, type/effect, recursion and execution-profile checks
remain authoritative. Original `SourceFile` snapshots and diagnostic ranges are
retained. Only the derived checking projection blanks resolved directives and
mode headers, with byte length and CR/LF preserved. Task and epoch independent
reparse checks are unchanged; tests compare imported/direct-input emitted C and
source maps byte for byte on those routes.

Each staged file passes its own standard capability gate before concatenation.
The linked extern-name inventory is supplied to this gate so a caller cannot
borrow another file's `std.extern` import merely by calling an extern declared
there. This deliberately tightens a pre-existing direct-multifile loophole as
well as guarding runtime imports. The caller needs its own call capability;
unchecked declarations still need the declaring file's own waiver import.
Q022 keeps that existing condition. Q000 explicitly covers malformed/unresolved
runtime directives; Q014 extends to failed runtime resolution and admission;
Q403 retains filename/mode mismatches. No diagnostic code is allocated.

The `QCore.RuntimeImports` model and registered `AuditRuntimeImports.lean` cover
snapshot precedence, retained original provenance, standalone unresolved
refusal, dependency mode refusal, missing targets, all-edge checking, decreasing
completion rank, absence of cycles and file-local capability permission.
The 14 audited declarations introduce no project axioms. Filesystem identity,
token extraction, completeness of the extracted edge inventory and correspondence
between Rust and Lean remain declared boundaries. The model does not prove C
refinement, native behavior or atomic filesystem loading.

## Focused validation

Worktree: `/tmp/qcore-runtime-import-core`. Private logs and captured exit files:
`/tmp/qcore-runtime-import-core/private/runtime-source-imports/`.
The shared Cargo target was explicitly granted to this card while these commands
ran, then released to the parent after Clippy completed:
`/home/cosmo/.codex/worktrees/audit-checkpoint-batches/qcore-c/target`.

The final focused command (with that path set as `CARGO_TARGET_DIR`) was:

```sh
cargo test -p qcore-frontend --test runtime_import_syntax --test module_imports \
  -p qcore-verification --test runtime_source_imports --test generic_driver \
  -p qcorec --test runtime_imports --test contract_load_path \
  --test extension_compatibility --no-fail-fast
```

`focused-5.log` / `.exit`: **exit 0, 7 binaries, 36 passed, 0 failed, 0 ignored**.
Counts are frontend syntax 5, standard-module compatibility 3, generic driver 5,
runtime resolver 16, contract CLI 4, extension compatibility 2 and runtime CLI 1.
The controls exercise recursive extension forms, direct/imported scalar C and
observation parity, task/epoch replay parity, cycles/ambiguity, explicit roots,
unreachable pools, wrong modes, standard/unchecked-extern capability isolation,
foreign extern declarations, CRLF/Unicode original locations, missing files,
invalid UTF-8, directory refusal, snapshot precedence, explicit disk opt-in,
module/byte/depth bounds and Unix symlink-parent/cycle behavior.

```sh
cargo clippy -p qcore-frontend -p qcore-verification -p qcorec --all-targets -- -D warnings
cargo fmt --all -- --check
git submodule update --init --recursive
```

Clippy `clippy-3.log` / `.exit`: **exit 0**. Format `fmt.log` / `.exit`:
**exit 0**; final formatting after test-slice lint cleanup made no semantic
change. Submodule initialization: **exit 0**, pinned libevent
`d82464a277d0f42703702c4dfd9af6af38595a83`. `git diff --check`: **exit 0**.

Lean commands ran from `proof/` under its pinned **Lean 4.34.0**, not the default
outside that directory. The unchanged ModuleImports/SourceExtensions dependency
oleans were borrowed from the shared worktree's `proof/.lake/build/lib/lean`;
only this card's model olean was emitted into its private directory.

```sh
LEAN_PATH=/home/cosmo/.codex/worktrees/audit-checkpoint-batches/qcore-c/proof/.lake/build/lib/lean \
  lean -o ../private/runtime-source-imports/lean/QCore/RuntimeImports.olean QCore/RuntimeImports.lean
LEAN_PATH=../private/runtime-source-imports/lean lean AuditRuntimeImports.lean
```

`lean-model-4.log` / `.exit`: **exit 0**. `lean-audit-3.log` / `.exit`:
**exit 0, 14 declarations**, closure uses only permitted `propext` and
`Quot.sound` where nonempty. The private search root contains symlinks to the two
unchanged dependency oleans, not copies of a new full build cache.

Earlier attempts are retained, not hidden:

| Attempt | Exit | Counts / finding and correction |
|---|---:|---|
| focused-1 | 101 | 2 binaries, 15 passed / 2 failed: nested directive slipped through structural parsing; reject directives outside top-level inventories. The declared-extern test also omitted its mandatory semicolon; corrected fixture syntax. |
| focused-2 | 101 | 4 binaries, 22 passed / 1 failed: function-body boundary did not recognize following `#import`; added it to declaration-start recognition plus a standalone boundary control. |
| focused-3 | 101 | 7 binaries, 33 passed / 1 failed: malformed legacy root lost its pre-refusal naming warning; emit root warnings before resolution, retaining dependency warnings and structured success warnings. |
| focused-4 | 101 | 7 binaries, 35 passed / 1 failed: Unicode control expected the unknown identifier column, but Q013 uses its existing statement span; assert the original `return` column without changing diagnostics. |
| clippy-1 | 101 | Redundant error-mapping closure; replaced with the closure reference. |
| clippy-2 | 101 | Six cloned singleton test slices; replaced with borrowed slices. Clippy-3 compiled the final tests successfully. |
| lean-model-1 | 1 | Continuation indentation in the path-rank proof; corrected. |
| lean-audit-1 | 1 | Private search root shadowed the dependency namespace; added dependency olean symlinks. |
| lean-model-3 | 1 | Extra tactic after simplification had closed the missing-target goal; removed the redundant tactic. |

The focused-1 command selected runtime syntax/resolver only; focused-2 added
runtime/contract CLI; focused-3 through focused-5 used the final command above.
`lean-model-2` and `lean-audit-2` passed the initial 12-declaration version; the
final model adds generic capability and missing-target theorems. These are
focused checks, not a claim that this isolated card ran the standing gate.

## Integrator handoff

The parent owns the combined source-spelling and pinned-corpus before/after
comparisons, full standing gate (including complete Lean AuditAll closure),
STATUS/increment/acceptance attribution, historical source-migration manifest,
and main integration/push. The parent also applies the requested diagnostic
and source-route rule amendments. No standalone duplicate full gate or separate
large target cache was created. The original prediction expects no pinned
C-corpus verdict movement; any direct-multifile extern refusal change must be
attributed to the explicit Q022 tightening rather than silently counted as
syntax-only parity.

## Integrated source migration

The integrator changed 351 maintained input/document files with 1,072 canonical
spelling substitutions at `19698477`; [attribution](spelling-attribution.json)
records each original and rewritten SHA-256. This includes 240 `.qr` sources.
Archived inputs and runnable snippets are maintained under the user’s explicit
instruction; notices distinguish their original measurements from current source
bytes. Core parser/driver/CLI and normative-spec changes belong to the implementation
commit rather than that mechanical census. One bare standard import is retained
as an explicit parser-compatibility control.

The tracked `runtime_import_spelling_inventory` example compared all 421 runtime
inputs at `b7c33df5` and `9271b204`: 168 accepted / 253 refused at both revisions,
no added or removed inputs, and identical emitted C and selected compile metadata
for every accepted source. Exactly 58 refused outputs change only the spelling
`import std.` to `#import std.` in their diagnostic message; codes, passes and
remaining messages are unchanged. The [exact comparison](source-comparison.json)
retains those deltas. This is default snapshot compilation evidence, not native
execution or a C-corpus safety claim. Private originals: `private/runtime-source-imports/`
`spelling-before.json`, `spelling-after.json`, their logs and comparison.

A pre-comparison formatting check exited 1 because the longer canonical spelling
wrapped one test string replacement; `9271b204` applies rustfmt to that statement.
The subsequent inventory exits 0. The integrated standing gate remains pending.

The pinned post-change audit also exits 0. All five JSON result artifacts are
byte-identical to the pre-change run, including resource diagnostics and the
retained contract/source-effect sidecars. [Audit comparison](audit-comparison.json)
records exact artifact digests and both run commands/source identities. The
inputs reuse the prepared Clang tree; no C revision, assumption pin or coverage
claim changes. Both runs use four workers; this card makes no throughput claim.

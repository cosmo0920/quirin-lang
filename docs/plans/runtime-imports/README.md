# Runtime source imports (`#import`)

> [!NOTE]
> **Current status (completed).** Implemented after the `.qv`/`.qr`/`.qrlib` extension migration and validated under the full standing gate ([implementation and validation report](../../reports/archive/language-and-tooling/runtime-source-imports/README.md)). Normative language rules live in [`docs/QCORE_SPEC_r0_3.md`](../../QCORE_SPEC_r0_3.md#runtime-source-imports) and [`docs/QCORE_MODULE_MODES.md`](../../QCORE_MODULE_MODES.md), and exact syntax lives in [`docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md`](../../QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g01a-runtime-import-bundle).

## 1. Implemented syntax and module roles

Runtime and library `.qr` files use `#import` for both bundled standard capabilities and relative `.qr` source dependencies (superseding bare `import std.*;` and keeping `.qv` `#include` completely separate):

- **Bundled standard capabilities**: `#import std.fmt;`, `#import std.io;`, `#import std.logger;`, `#import std.string;`, `#import std.buffer;`, `#import std.event;`, `#import std.socket;`, `#import std.http;`, `#import std.concurrency;`, `#import std.extern;`, `#import std.extern.unsafe;`, `#import std.ffi.unsafe;`. Dotted identifiers match the closed standard-module registry (no wildcards). Bare `import std.*;` remains a parser compatibility alias only; maintained and archived runnable examples use `#import`.
- **Relative `.qr` source files**: `#import "./helper";` and `#import "../shared/helper.qr";` resolve relative to the importing file, appending `.qr` when omitted. There is no legacy-suffix fallback, directory-module implicit index, or environment search path, and runtime `#import` never loads `.qv` contracts or `.qrlib` native manifests.

## 2. Resolution, staging, and capability-isolation rules

1. **Snapshot-first API, explicit CLI disk loading**: Library compilation APIs remain snapshot-only by default; filesystem loading requires an explicit API opted into by the `quirin` CLI, with supplied snapshots taking precedence at matching lookup identities.
2. **Canonical identity & fail-closed bounds**: Lookup identities stay separate from diagnostic display spans; filesystem `..` is never collapsed across symlinks prior to lookup; missing files, ambiguous aliases, cycles, and depth/module/byte-limit violations refuse (`Q014`). Repeated and diamond imports reuse one loaded snapshot in deterministic source-traversal order.
3. **Allowed dependency modes**: Imported `.qr` dependencies may be unmarked or declare `module library;`. Explicit `module executable;` and `module contract;` dependencies refuse. Root inputs alone determine compilation mode.
4. **Per-file capability isolation & span-preserving staging**: Unresolved runtime imports refuse at the standalone parser (`Q000`). After full bundle resolution, directives and mode headers are replaced with equal-length spaces (preserving exact byte offsets, line numbers, UTF-8/CRLF positions, and diagnostic spans), and standard capability gates run **independently per staged file** before combining declarations—so a helper's `#import std.io;`, `std.concurrency`, or `std.extern.unsafe` never grants authority to its importer (`QCore.SourceImports`).

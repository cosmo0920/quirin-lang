# Windows Clang fixture type and line-ending repairs

Date: 2026-09-08. Starting checkpoint: `5065edc`; rebased onto fetched
`origin/main` at `93f071c` during validation, at the user's request. The
rebase and restoration of the fixture fixes were conflict-free.

## Reproduction and changes

Visual Studio 2026 Clang 22.1.3, targeting `x86_64-pc-windows-msvc`,
reproduced seven failures in `qcore-c-gateway --test o04_constructor_caller`:

- The reduced C prelude declared `size_t` as `unsigned long`, conflicting
  with Clang's Windows x64 type. The prelude now uses `__SIZE_TYPE__`,
  `__PTRDIFF_TYPE__` and `__UINT64_TYPE__` for its size, signed-size and
  64-bit integer declarations. It needs no target libc headers and retains
  the Linux x64 widths. The remaining reduced declarations are still fixture
  assumptions, not a full platform ABI import.
- The generic consuming sink took `struct g_handle *`, but two rejection
  cases supplied `struct g_ctx *` and `struct g_conf *`. It now takes
  `void *`, like a generic object deallocator. The unchanged consuming
  contract and all five ambiguity assertions still run through the gateway;
  invalid transfer, leak and certificate-substitution controls remain active.
  No Clang diagnostic is suppressed.
- Git's `core.autocrlf=true` changed the four upstream excerpts' bytes. A
  fixture-local `.gitattributes` now preserves their LF endings. Their
  contents and all recorded SHA-256 hashes remain unchanged; the raw-byte
  provenance test still requires an exact match.
- The destructor-contract mutation expected LF text in a CRLF checkout.
  It now normalizes its test-local baseline before substitution and compares
  against that same baseline, so a failed substitution cannot pass merely
  because line endings changed. The same repair fixes the remaining
  `qcore-verification --test counter_source` false-contract control.

Windows CI also runs both targets through Visual Studio discovery with
Clang absent from PATH; the full CI suite already uses explicit `QCORE_CLANG`.

## Validation

The O04 assertion target passes all 14 tests through native Windows Clang
discovery and explicit `QCORE_CLANG`. The counter-source target passes all
six tests with explicit `QCORE_CLANG`. O04's one ignored test remains an
opt-in ledger reporting aid.
The concatenated real C fixture passes `-std=c11 -Werror -fsyntax-only`
for both the native Windows x64 target and `x86_64-unknown-linux-gnu`, with
additional static assertions that `size_t` matches pointer width, `ssize_t`
matches `size_t`, and `uint64_t` is eight bytes. The Linux check is a syntax
and type-width check, not native Linux execution.

Formatting, workspace Clippy with warnings denied, and both reevaluation
examples pass before and after the rebase. The pinned Lean build and all 31
audits also pass before and after the rebase (75 build jobs). Submodule initialization
completed and Libevent matches gitlink `d82464a277d0f42703702c4dfd9af6af38595a83`.

The first workspace run was stopped for the requested rebase, with no reported
test failure before interruption. At the user's subsequent commit/push request,
the restarted workspace suite is still running; the incoming commit's changed
Lean build and audit runner subsequently passed. This record does not claim a completed
aggregate gate for the rebased tree. Logs are in
`target/windows-clang-fixtures-rebased-{workspace,lean,clippy,probe,followup}.log`;
the focused post-rebase logs use the same prefix with `o04` and `counter`.

## Scope

Only test fixtures, test setup, CI coverage and documentation change.
Production checker code, contracts, Lean semantics and verdict admission rules
are unchanged. The real O04 constructor and caller still have function-level
`Unknown` status, with the same bounded per-exit assertions. No raw production
audit is re-measured or promoted, and CT mappings and contract counts do not
change.

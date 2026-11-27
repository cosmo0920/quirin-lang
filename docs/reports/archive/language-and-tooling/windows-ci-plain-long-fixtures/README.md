# Windows CI: target-portable fixture spellings

Windows CI on main (`x86_64-pc-windows-msvc`) failed 14 gateway tests in four
targets: `ag1_indirect_member_access` (3), `ag2_unresolved_alias` (5),
`callback_literal_premise` (3) and `openssl_undeclared_handshake_callbacks` (3).
Test-only change; no gateway, contract, rule or Lean change.

## Cause

Not an analyser defect. Since CG20-NUMERIC-DOMAIN (rule 11q) the declared-ABI
reader reads plain `long` (and a typedef of it) only when the analysing Clang's
target is LP64; under LLP64 it is unreadable and the contract is withdrawn with
`abi-unreadable-declaration` (`legacy_declared_abi.rs`, fail-closed by design).
The UCRT's `<stdlib.h>` is also unread there (`member_effect_summary.rs` records
the same boundary for `malloc`). The four fixtures spelled a size or a
`long`-typed callback with plain `long`, or took `calloc` from the host headers:

| target | fixture spelling | Windows result |
|---|---|---|
| `ag1_indirect_member_access` | host `<stdlib.h>`/`<string.h>` for `calloc` | `calloc` withdrawn |
| `ag2_unresolved_alias` | `typedef unsigned long x_size;` | `x_len`/`x_fresh` withdrawn |
| `callback_literal_premise` | `long size` on `isize` callbacks | candidates `callback-uncontracted` |
| `openssl_undeclared_handshake_callbacks` | `long` in `bio_cb_ok`'s signature (`i64`) | candidate `callback-uncontracted` |

## Fix

The synthetic declarations now spell sizes `__SIZE_TYPE__`, the signed
pointer-width size `__PTRDIFF_TYPE__`, and 64-bit `long long`; ag1 declares
`calloc`, `free` and `NULL` itself instead of including host headers. Each
fixture carries a comment saying why. Every assertion is unchanged.

## Evidence (private logs)

* Linux, system Clang 21: the four targets pass, 6 + 14 + 11 + 29
  (`private/fix-windows-ci-linux.log`).
* Simulated Windows target (`QCORE_CLANG` wrapper running Clang 21 with
  `--target=x86_64-pc-windows-msvc`): with the fix, all four pass
  (`private/fix-windows-ci-llp64.log`); with the original fixtures the same
  run fails 6, 5, 3 and 3 (`private/fix-windows-ci-llp64-orig.log`): the three
  non-header targets match CI exactly, and ag1 fails all six only because Linux
  has no UCRT headers.
* Windows CI itself has not been re-run here; the next CI run on main is the
  confirmation.

# Windows types in fixtures (LLP64 notice)

Notice for anyone writing a test fixture, a synthetic C declaration or a card
test. Routed by [AGENTS.md](../../AGENTS.md#windows-types-notice). It states
no rule of its own: the authorities are gateway rules
[11q](../agents/gateway.md#constraint-11q) (integer widths are the analysing
Clang's, and plain `long` is read only under an LP64 target) and
[12u](../agents/gateway.md#constraint-12u) (an enumeration is the integer type
Clang states for it). This page says what those rules mean for a fixture that
must pass on Windows CI as well as on Linux.

## Why it matters

Windows CI analyses with Clang targeting `x86_64-pc-windows-msvc`, which is
LLP64. There, `long` and `unsigned long` are 32 bits, `size_t` is
`unsigned long long`, and an enumeration is `int`. Linux (LP64) gives `long`
64 bits and may give an enumeration `unsigned int`. A fixture spelled for
Linux can therefore bind a contract position on Linux and be withdrawn on
Windows. The withdrawal is fail-closed, so the test sees `Unknown` (with
`abi-unreadable-declaration` or `abi-parameter-type-disagreement`) and fails
only on Windows. Two rounds of Windows CI failures had exactly this cause:
[round 1](../reports/archive/language-and-tooling/windows-ci-plain-long-fixtures/README.md) (r42, 14 tests)
and [round 2](../reports/archive/language-and-tooling/windows-ci-llp64-fixtures-2/README.md) (r73, 8 tests).

## Spell these types this way

| C type in a fixture | At a contract position of | Spell it | Do not spell it |
|---|---|---|---|
| `size_t` | `usize` | `__SIZE_TYPE__`, or `host_abi::size_t_declaration!()` where the test already uses it | `unsigned long`, `typedef unsigned long size_t;` |
| `ssize_t`, `ptrdiff_t` | `isize` | `__PTRDIFF_TYPE__` | `long` |
| a 64-bit integer | `i64` / `u64` | `long long` / `unsigned long long` | `long` / `unsigned long` |
| an enumeration | `u32` | `typedef unsigned int E;` beside an untagged `enum { … };` holding the constants | `typedef enum { … } E;` (Clang gives it `int` on MSVC). A C23 `enum : unsigned int` is not read by the declared-ABI reader on either target |
| a libc callee (`calloc`, `free`, …) | any | declare it in the fixture text | a hosted `#include <stdlib.h>`; the UCRT headers are not read there |

`__SIZE_TYPE__` and `__PTRDIFF_TYPE__` are Clang's predefined macros, so the
preprocessed text the gateway reads carries the target's own type.
`crates/qcore-c-gateway/tests/support/host_abi.rs`'s `size_t_declaration!`
picks the spelling with `cfg(windows)` when the test is compiled. Plain `long`
is fine where no contract reads it, such as a local, a field the contract
reads only as bytes, or a subject function's own parameter; when in doubt,
use the spellings above. Each fixed fixture carries a one-line comment saying
why, and so should a new one.

## Check before handing back or landing

```sh
git diff --name-only <base>...HEAD -- '*tests*' \
  | python3 docs/reports/windows-ci-llp64-fixtures-2/scripts/llp64_fixture_scan.py --stdin
```

Exit 0 means nothing was counted. Exit 1 lists each hazard as
`path:line: [hazard] text (why)`. Callback literals need `--strict`. A line
carrying `llp64-scan: allow` suppresses a reviewed hit. The integrator runs the
same scan over each card's tests at landing.

A simulated Windows run (`QCORE_CLANG` pointing at Clang with
`--target=x86_64-pc-windows-msvc`) reproduces these withdrawals on Linux; the
round 2 report describes it and the artifacts it cannot avoid. Only Windows CI
on main confirms a fix, and no report claims a Windows pass it did not observe.

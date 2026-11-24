# Windows CI: target-portable fixture spellings, round 2

Card WINDOWS-CI-LLP64-FIXTURES-2 (USER DECISION 2026-11-24, "WE need to
address those of errors"). Windows CI on main (`x86_64-pc-windows-msvc`, run
20261124T075934Z: 689 units, 9530 passed, 8 failed) failed 7 of 9 tests in
`in_place_regex_certified` (added by r64) and
`round_4_xxh_finalize_reads_a_decayed_member` in
`cg10_sole_residual_contracts`. This is a test-only change made by the method
of [round 1](../windows-ci-plain-long-fixtures/README.md) (r42). No gateway,
contract, provider, rule or Lean file changes, and no assertion changes. Base
`69c64255f` (origin/main).

## Cause

Neither failure is an analyser defect. Both are fail-closed withdrawals of a
declared contract against a fixture's C declaration:

| target | fixture spelling | contract | Windows reading |
|---|---|---|---|
| `in_place_regex_certified` (7 tests) | `unsigned long nmatch`, `unsigned long regerror(…, unsigned long errbuf_size)` | `regex.qv`: `usize` | plain `long` is unreadable unless the analysing Clang is LP64 (rule 11q, `legacy_declared_abi.rs`): `regexec` ×6 and `regerror` ×1 withdrawn, `abi-unreadable-declaration` |
| `cg10_sole_residual_contracts` (1 test) | `typedef enum { XXH_bigEndian=0, … } XXH_endianess;` (and `XXH_alignment`) at `endian`/`align` | `lz4_stream.qv`: `u32` | rule 12u reads an enumeration through the integer type Clang states for it. Clang 21 gives this enum `unsigned int` on `x86_64-pc-linux-gnu` and `int` on `x86_64-pc-windows-msvc`, checked with `_Static_assert(_Generic((E)0, unsigned int: 1, default: 0))`, which fails only on the MSVC target. `XXH32_finalize` is withdrawn with `abi-parameter-type-disagreement` |

`regex_t.re_nsub` was also spelled `unsigned long`. `regex.qv` reads `regex_t`
only as `ptr<const, u8>`, so that field does not cause the failure. It is
spelled `__SIZE_TYPE__` anyway, as POSIX's `size_t`.

## Fix

| file | change |
|---|---|
| `crates/qcore-c-gateway/tests/in_place_regex_certified.rs` | `DECLARATIONS` only: `re_nsub`, `nmatch`, `regerror`'s result and `errbuf_size` are spelled `__SIZE_TYPE__`. `pmatch` stays `void *`. A comment says why |
| `crates/qcore-c-gateway/tests/cg10_sole_residual_contracts.rs` | in `XXH` and in the rule 66 test's `head`, each enum typedef became an untagged `enum { … };` holding the same constants beside `typedef unsigned int XXH_endianess;` (`XXH_alignment`). Comments say why |

Choosing the XXH spelling: a C23 fixed underlying type, `typedef enum :
unsigned int { … } XXH_endianess;`, does compile under Clang 21 in the
gateway's mode. The declared-ABI reader cannot read it on either target,
though, so the test stayed `Unknown` with `abi-unreadable-declaration`
(`private/llp64/tryB-win.log`). A typedef of `unsigned int` is a builtin scalar
on both targets. The decayed-member read, `state->mem32` passed as `const void
*`, and the `XXH_aligned` constant actual are unchanged.

The rule 66 refusal test (`round_4_rule_66_keeps_a_non_scalar_typedef_element_refused`)
expects `Unknown` and passed on Windows, but for the wrong reason. A probe
printed its diagnostics. With the original spelling, the simulated Windows run
gave `XXH32_finalize … withdrawn (abi-parameter-type-disagreement)` for all
three bodies. With the fix, it gives `indirect memory access is outside the
resource projection`, the rule 66 refusal Linux already gave (logs
`private/llp64/r66-{orig,fixed}-{linux,win}.log`; the probe copy was rewritten
byte for byte afterwards and compared with `cmp`). Because of that, its `head`
gets the same spelling.

## Sweep

* **The scan** (below) was run over `crates/*/tests compiler/tests
  providers/*/tests`. It found 1,180 candidate spellings and counted 253 that
  bind a hazardous contract position: 38 plain-long, 8 enum-type and 207
  host-include. All of them predate this card, and Windows CI on main passes
  every one of them. CI failed only the 8 tests above, so none needs a fix. Of
  the files the scan counted, 22 branch on a Windows or Unix `cfg`, and many
  others take `size_t` from `support/host_abi.rs`, whose
  `size_t_declaration!` picks `unsigned long long` under `cfg(windows)`.
  Others expect `Unknown` anyway, or are native, non-analysed C. The scan
  lists; CI's result decides.
* **Simulated Windows, whole workspace.** `run_test_binaries.py` ran with
  `QCORE_CLANG` pointed at Clang 21 with `--target=x86_64-pc-windows-msvc` and
  the xwin MSVC CRT/SDK headers (`private/wine-toolchain/clang-win-hdr`). The
  result is in the Evidence section. A Linux-host build cannot stand in for
  CI's Windows host build in three ways. `cfg!(windows)` and the
  `host_abi::size_t_declaration!` macro are decided when the test is compiled,
  on Linux, so they spell `unsigned long` (for example `retry_lifetime`,
  `sr5_soundness` and `refuted_contract_withdrawal` withdraw `p_malloc` and
  `w_malloc` there). The compiler and verification tests build and run native
  Windows-target objects, which Linux cannot do. And some header readings
  differ, such as `cg26_variadics`' `va_list`. These failures are simulation
  artifacts, because CI passes them. No fixture beyond the two above is
  changed.
* **Queued cards.** I scanned the added and modified test files of the 14
  unlanded agent branches, read from git objects only. The counted hits are
  all in `CG25-IN-PLACE-REGEX-REGERROR-FAILURE-EDGE` (r73, tip `25c0293c8`),
  whose `in_place_regex_certified.rs` and new `in_place_regex_failure_use.rs`
  carry the same `unsigned long` `DECLARATIONS`: `regexec` parameter 3, and
  `regerror`'s result and parameter 4. The other counted hit,
  `clang_gap_phase4.rs:157`'s `<setjmp.h>`, is already on main. That file is
  in the Onigmo branch's diff, but the hit is not on a new line.

## The scan script

`scripts/llp64_fixture_scan.py PATH... [--stdin] [--strict] [--all]
[--contracts PATH]` takes Rust test files and C fixtures. It walks
directories, and with `--stdin` it reads a path list, for example from `git
diff --name-only BASE`. It lists hazards and never edits anything. Each line
reads `path:line: [hazard] text (why)`. Exit 0 means nothing was counted, 1
means hazards were counted, and 2 means an input could not be read.

* `plain-long` counts plain `long` or `unsigned long` at a `usize`, `isize`,
  `i64` or `u64` contract position.
* `enum-type` counts an enumeration type at a `u32` position.
* `host-include` counts a hosted `#include` while a contracted callee is
  declared by no fixture text. Clang's freestanding headers are exempt.

For a prototype, the C position is matched to the contract signature: the
result or parameter *k*. A typedef counts when a contracted prototype in the
same file uses it at such a position. Signatures come first from the file's
own contract text and its `include_str!`'d `.qv` files. A file with neither
uses every provider and gateway-fixture contract in the repository.
Definitions, which include callback literals, appear only with `--strict` or
`--all`, along with locals and fields. Callback fixtures therefore need a
`--strict` review. A line containing `llp64-scan: allow`, or the line above
the literal, suppresses a hit. On the original fixtures the scan counts the 3
regex positions and 4 XXH positions that CI failed on. On the fixed files it
counts 0.

For the integrator, at landing:

```sh
git diff --name-only <main>...<card tip> -- '*tests*' \
  | python3 docs/reports/windows-ci-llp64-fixtures-2/scripts/llp64_fixture_scan.py --stdin
```

## Evidence (private logs, `private/llp64/`)

* **Linux, system Clang 21.** The two targets pass, 9 + 9 (`fix-linux.log`).
  They also passed before the fix (`orig-linux.log`).
* **Simulated Windows** (`QCORE_CLANG=private/llp64/clang-win`, Clang 21 with
  `--target=x86_64-pc-windows-msvc`). With the fix, both pass 9 + 9
  (`fix-win.log`). With the original fixtures, the same run fails 7 in
  `in_place_regex_certified` and 1 in `cg10_sole_residual_contracts`, exactly
  CI's 8 tests with CI's diagnostics: `regexec` ×6 and `regerror` ×1
  `abi-unreadable-declaration`, and `XXH32_finalize`
  `abi-parameter-type-disagreement` (`orig-win.log`).
* **Simulated Windows, whole workspace with xwin headers:** 688 units, 9,402 passed, 559 failed in 147 units (qcore-c-gateway 32, qcore-compiler 77, qcore-verification 31, qcore-audit 4, others 3); both fixed targets pass 9 + 9 there. Every failing unit is outside CI's failure list, so each is a simulation artifact of the kinds named under Sweep, not a Windows failure
  (`sweep-win.out`, `sweep-win/`, `sweep-failures.txt`).
* **Scoped gate:** `sh private/toolchain/env.sh python3 scripts/gate_scope.py --base 69c64255f --out private/llp64/gate` at `3863195ed`, all 9 steps exit 0. Those steps were submodules, fmt, 5 scoped Clippy runs, the Rust suite (11 targets, 67 passed, 0 failed) and the Python suite (807 tests). There was no Lean step and no provider step (`private/llp64/gate/gate.json`).
* Windows CI itself cannot run here. The next Windows CI run on main confirms
  the fix.

## Boundary noted, not registered

The production librdkafka `rdxxhash.c` declares the same plain enumerations.
Analysed by an MSVC-target Clang, `XXH32_finalize` and `XXH64_finalize` would
be withdrawn there as well, which is fail-closed. That is a property of the
contract under a Windows build of librdkafka, not a test hazard. The audited
corpus is analysed on Linux. No ledger entry is registered.

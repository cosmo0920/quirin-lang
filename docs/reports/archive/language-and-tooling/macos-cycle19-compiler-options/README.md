# macOS Cycle19 compiler-option transport — 2026-09-08

The current-revision Cycle19 preparation completed on macOS, but its audit
initially refused every selected compilation command before resource analysis.
The 152 Fluent Bit configurations first reported
`-Wundef-prefix=TARGET_OS_`; the 29 forced-Libevent Monkey configurations first
reported the two-word `-arch arm64` option. The refusal was fail-closed and
published no optimistic result.

## Change

Both option forms are now ordered, typed `CFrontendFlag` values rather than
discarded command text. `-Wundef-prefix=<prefix>` retains its exact prefix and
`-arch <architecture>` retains its exact architecture and two-argument form.
The gateway forwards both to original-file preprocessing, syntax checking and
typed-AST import. The complete ordered compile command and these typed options
remain in `CompileTaskIdentity`; selecting an architecture therefore retires an
identity instead of becoming an ambient host assumption.

Malformed empty prefixes and missing, empty or option-looking architecture
arguments reject before Clang runs. As with the previously admitted frontend
flags, either option requires an explicit supported `-std=` dialect and the
configured Clang C driver. Other compiler options and compiler substitutions
remain rejected. An architecture unsupported by the configured Clang fails in
the Clang gateway; it is never ignored or rewritten.

## macOS replay

The prepared command from the user was replayed with Apple Clang 21.0.0 against
Fluent Bit commit `11651d47a324585c218594827468a29623f62672` and its separately
prepared forced-Libevent Monkey database. It completed all selected work:

**180 source files / 181 configurations: 2 Safe / 41 provisional Unsafe /
134 Unknown / 4 data-only / 0 gateway failures.**

There were zero compiler-command refusals. Historical source, native and
provider evidence was not selected and remained Unknown. This is a raw resource
projection for this revision, compiler and build configuration. It does not
replace the pinned 185-unit ledger, promote either Safe unit, turn a provisional
Unsafe into an upstream defect, or establish full Fluent Bit build equivalence.

## Validation

`compile_options` exercises both options independently and together against
direct Clang preprocessing and the typed-AST path. It also exercises `arm64`
and `x86_64` target macro selection on Apple Clang and rejects malformed forms.
The focused 12 compile-option tests and seven compile-task-identity tests pass.
`QCORE_CLANG=/usr/bin/clang cargo test --workspace`, formatting, Clippy with
warnings denied, both reevaluation probes and all 34 Lean audits pass.

No QCore language/resource rule or Lean definition changes in this increment.

# Cycle19 current-command checkpoint — 2026-09-07

The existing `cycle19_fluent_bit_libraries_audit` example now defaults to the
current checkout. `--pinned-evidence`, before positional arguments, explicitly
selects the previous source/native evidence path and retains its revision gate.
The current path records source/build identity for every distinct compilation
configuration, verifies selected source bytes against the actual Git commit,
and retains unsupported configurations as Unknown. Identical commands deduplicate;
different commands for one source remain separate obligations. Old source-effect,
chunk-contract, Monkey and binding output sidecars are replaced with explicit
Unknown records rather than left adjacent to a new-revision report.

The Monkey preparation script now accepts the current clean source revision.
It continues to reject reuse of a managed directory with different input identity;
use a fresh output directory when updating prepared inputs.

## Execution and remaining work

The user's exact command completes at
`d1ab8abff6cbaee89cee5efeeeb2667d1910ac52`:

```sh
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  /home/cosmo/Documents/GitHub/fluent-bit \
  build/cycle19/monkey-build/compile_commands.json \
  build/cycle19/reports/monkey-source.json \
  build/cycle19/reports/resource-diagnostics.json
```

**185 source files / 186 configurations: 186 Unknown, zero gateway failures.**
This is an executable inventory checkpoint, **not successful semantic auditing**
of the production corpus. All configurations stop before resource analysis:
140 need supported handling of the declared compiler/dialect and frontend flags;
17 contain an unexpanded CTraces option; 29 hit unsupported `-Wextra` in the
Monkey build. The next required work is faithful ingestion of those actual build
commands, including the filename macro expression, compiler identity/default
language mode and flag effects. Do not replace these checks with dropped options
or claim that command success establishes the requested audit result.

The explicit historical option still refuses this newer checkout. Fresh Monkey
preparation at this revision succeeds. No source-specific provider or native
certificate has been promoted to the new revision.

## Validation

All **856 workspace tests** pass, including two current-inventory regressions
and a Cycle19-route regression. They exercise accepted/Unknown configurations,
exact duplicate deduplication, distinct-command preservation, new-commit identity,
changed and dirty source, stale-sidecar replacement and historical rejection.
Fmt, Clippy with warnings denied, both reevaluation examples, Lean build and all
20 Lean audits pass. Input/provenance routing changes do not alter the Lean
language/resource semantics. Compressed logs and the actual command reports are
archived alongside this document. The former pinned corpus was not remeasured
or relabeled; its previous verdict totals do not describe this checkpoint.

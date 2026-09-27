# Pinned replay CLI relocation

Base: `2e44e963cfed0320a9d1c93ea09c9e70eeb1ff99`.

## Prediction (recorded before implementation)

This is a nonsemantic relocation of the Cycle 19 pinned Fluent Bit replay from the compiler example to `quirin-audit`. Expected changes to every pinned verdict, contract count, semantic profile, assumption, evidence version, and corpus coverage are **zero**. The replacement command should reproduce the old command's pinned output, modulo the command name or intentionally documented presentation metadata. Any variance requires investigation before deleting the legacy route.

## Ownership and command

`quirin-audit pinned [--jobs N] [FLUENT_BIT_ROOT [MONKEY_DATABASE [MONKEY_SOURCE_REPORT [DIAGNOSTICS [BINDINGS [MANIFEST]]]]]]` is the explicit built-in historical replay. Its ordered audit fold and sidecar assembly moved from the compiler example to `qcore-audit::pinned_route`; the new `pinned_cli` parses before starting source work. `pinned` is reserved in the external-subcommand built-in table, and the plugin shadowing regression covers it. The configured audit remains `quirin-audit current`, with its current-revision test now in the owning audit crate. The live runner is `scripts/run_pinned_audit.py`; the frozen W43 runner remains historical evidence.

`quirin-source-analysis` owns the formerly compiler-resident ChunkIO and Libevent source adapters, their source-profile locator, typed match and chunk contract boundaries, unit tests and needed fixtures. `qcore-compiler` uses facade reexports for source compatibility; `qcore-audit` uses the shared crate and lower gateway, contract and verification crates directly. The gateway's generic typed C importer, compiler's separate `pinned_lifecycle` adapter, verification models and the compiler's first-party provider construction stay in their existing owners. Opaque `ChunkSourceSummary`, `ChunkContractCertificate` and `SourceMatch` constructors remain private. This extraction changes no inference, witness admission, resource contract or pin. There is no new Lean semantic obligation; the standing Lean audit is still run as a gate.

The old profile locator produced a non-canonical repository path through `compiler/../contract/profiles`. Contract sidecars include that spelling in raw source keys (evidence constraint 22), so the shared locator deliberately reconstructs the same path from its new crate root. The first new replay attempt produced different raw keys; it was interrupted (exit 130) and is **not** parity evidence. The first old single-job attempt was also interrupted (exit 130) under shared-host load. Both attempts remain in `private/audit-pinned-replay-cli/{new,baseline}/replay/` with their logs and `INTERRUPTED.txt`; fresh four-job paths are used for parity.

The compilation environment still comes from the pinned compile database, with the same `HistoricalStdin` transport; no selected unit, source path, assumption, observed evidence or certificate check changed. The package and profile pins remain the same. Source summary and result formatting follow the original code apart from import ownership and the command name.

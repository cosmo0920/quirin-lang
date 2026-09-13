# Wave 21 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

<!-- migrated-section:start -->
Wave-21 close increment (2026-09-11): the two remaining cards merged under
the failure-gated chain, which caught one composition seam (the example
suite's rename-proof driver constant failed to compile once only `quirin`
existed — `env!` resolves in the arm a match never takes). Wave 21 is
corpus-neutral by construction; the baseline stays
`docs/reports/wave20-corpus/`.
- Source logging in the thread examples (COMPOSE-CONCURRENCY): the new
  route `source.std-io-tasks.v1` composes the checked task route with
  `std.io`/`std.logger` — the task half is the unedited task lowering and
  checker, the line half is each std-io definition extracted and shared,
  so no second lowering or backend exists. The v1 boundary is measured,
  not chosen: every write an emitted task program performs happens on the
  parent's thread, so any parent-body line is join-ordered and admitted;
  a worker-body line refuses at `Q050` (new `ComposedTasks` pass,
  standing constraint 15a). TRACE-STDIO's impossibility pin is rewritten
  into three tests; one STD-IO-COMPOSE route exclusion is lifted and the
  other two re-pinned; `STD-FMT-BUFFER-READ` lands
  (`value |> log info dec` over `qcore_buffer_read`). All four
  `examples/threads` programs gain source logging, sanitized and
  per-stream byte-compared; no synthesized trace byte moved. Evidence:
  `docs/reports/wave21-compose-concurrency/README.md`.
- The example suite (EXAMPLES-SUITE): five families under `examples/` —
  stdio (both renderings, all three severities with the `fatal` collision
  asserted, `flush`'s no-durability boundary, the composed
  `value |> render |> log` chain), pipes, branching, extern, and
  unsafe-linkage — 14 programs, 32 tests, with refused programs and
  quoted diagnostics extracted from the READMEs and compiled, so no
  documented refusal can drift. Evidence:
  `docs/reports/wave21-examples-suite/README.md`.
Merged gate at this checkpoint: 3,343 workspace tests (8 ignored),
122 provider tests, 193 Python tests, 90 Lean audits, both reevaluation
examples exit 0, fmt and clippy clean. Follow-on accounting lives in
`docs/reviews/r0_3/acceptance_matrix.json` (`named_followups`, updated
for this wave).


Wave-21 partial increment (2026-09-11): QUIRIN-RENAME landed through a
delegated worktree — the language presents as **Quirin**, the CLI binaries
rename to `quirin` and `quirin-audit` via `[[bin]] name` alone, and the
`.qc` extension is retired (385 `.qcore`, zero `.qc`, Emacs registration
dropped with a new negative ERT test). The user rule is load-bearing and
kept: every `qcore-*` crate, package, library and namespace name, the
emitted-C `QCORE_*`/`qcore_*` surface and the spec/doc filenames stay —
the core component remains seated as "qcore". Census: 44 of 170 live
occurrences named a binary; no CI edit was needed (every automated
invocation selects the package, not the binary); no transitional alias,
because a second `[[bin]]` would make `cargo run -p qcorec --` ambiguous.
Behaviour neutrality byte-pinned. Evidence:
`docs/reports/wave21-quirin-rename/README.md`.


<!-- migrated-section:end -->

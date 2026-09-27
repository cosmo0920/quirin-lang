# D01 — Diagnostic ownership and actionable `Unknown` reasons

> Historical checkpoint: source filenames, runnable references or discovery scripts in this archive were updated by the [extension migration](../extension-physical-migration/README.md). Recorded results, hashes and gate counts describe the original revision, not a new run of the maintained files. Use [current status](../../STATUS.md) for present measurements.


**Date:** 2026-09-07. **Base:** `bfbad21` (*Merge P02 enforced process resource
limits*). **Card type:** implementation (W7).

This card repairs the two real diagnostic-attribution defects recorded by V04
and V07, makes the diagnostic ownership table canonical, reconciles the
`Q700`–`Q703` spec drift, records an ownership decision for the `Q020`/`Q021`
route split, and specifies — but deliberately does not implement — a closed
reason-code vocabulary for uncovered obligations.

**No diagnostic code changed.** Every input that was rejected before is still
rejected, with the same code, the same message and the same exit status. Only
the *attribution* moved: which file, which line, which column, which byte
offsets. No gate was weakened, no pinned expectation was deleted, and no raw
Cycle 19 unit was promoted.

## 1. Defects repaired

### D01-a — multi-file diagnostics misattributed at a file boundary

Reported by `docs/reports/remaining-v07/README.md` §"D01". A diagnostic whose
span started at byte 0 of a non-first input file was attributed to the
**previous** file, at a line one past that file's end.

Root cause, `crates/qcore-verification/src/driver.rs`:

```rust
.find(|range| range.start <= offset && offset <= range.end)
```

`FileRange::end` is the *exclusive* end of a file in the combined buffer, and
`combine_files` starts the next file at exactly that offset whenever the
previous file already ends with a newline. Byte 0 of file *n+1* therefore
satisfied file *n*'s predicate, and `find` returned the earlier file.

Repair: select the range that actually contains the offset, and fall back to
the last range starting at or before it. The fallback is not decoration — an
offset can legitimately belong to no file at all, namely the separator newline
`combine_files` appends after a file that did not end with one, and the very
end of the combined buffer. Without it, an end-of-input span would have
regressed to `ranges[0]`.

```rust
let index = ranges
    .iter()
    .position(|range| range.start <= offset && offset < range.end)
    .or_else(|| ranges.iter().rposition(|range| range.start <= offset))
    .unwrap_or(0);
```

All three occurrences V07 confirmed are fixed, replayed against **V07's own
archived probe sources**:

| Reproducer (V07 probes) | Before | After |
|---|---|---|
| `d01_boundary_first` + `d01_boundary_second` | `d01_boundary_first.qcore:2:1 Q010` | `d01_boundary_second.qcore:1:1 Q010` |
| `a15_task_second_file_main` + `…_bound` | `…_main.qcore:11:1 Q020` | `…_bound.qcore:1:1 Q020` |
| `a15_task_second_file_main` + `d02_boundary_record` | `…_main.qcore:11:1 Q000` | `d02_boundary_record.qcore:1:1 Q000` |

Both of V07's controls are unchanged: `a04` still reports `Q021` at the clause
span `a04_second_bound.qcore:1:29`, and `b10` still reports `Q013` at
`b10_cross_file_second.qcore:2:3`.

### D01-b — the unconditional declaration check reported a degenerate span

Reported by `docs/reports/remaining-v04/README.md` §F1. `elaborate_source`
validates dimension/axis declarations unconditionally by slicing them out of
the source, joining the slices and reparsing. Every failure of that pass was
emitted with `Span { start: 0, end: 0 }`, which renders as line 1 column 1 of
the first input file.

Repair: retain, per slice, its placement in the reconstructed buffer and its
span in the caller's own source. The slices are verbatim copies, so the mapping
is exact byte for byte. `StaticEnvironment::check_located` is added alongside
`check` and additionally returns the index of the declaration that failed;
`check` delegates to it and discards the index, so the accepted/rejected set and
the returned environment are untouched.

### D01-c — V04-F2, byte offsets into the reconstructed source

Reported by `docs/reports/remaining-v04/README.md` §F2 and left open by the card
("fix if the mapping is straightforwardly recoverable"). It is recoverable:
`ParseError` already carries a `Span`, so the same slice map that fixes F1 also
translates a parse failure's offsets back to the caller's file. **Fixed**, not
documented as a boundary. The message shape (`… at bytes N..M`) is preserved;
only the numbers become real.

### V04-F3 — first-error truncation

`StaticEnvironment::check` still returns on the first `CheckError`, so a source
with two bad axes names only the first. This is fail-closed and consistent with
the validator's `Result` signature. **Documented, not changed**, per the card.

### Probe matrix

`probes/run_probes.sh target/debug/qcorec` reproduces all of it. Both logs are
archived: [`probes/probe_log_before.txt`](probes/probe_log_before.txt) (built
from the same tree with the repair reverted) and
[`probes/probe_log_after.txt`](probes/probe_log_after.txt).

| Probe | Before | After | Note |
|---|---|---|---|
| `d01_boundary_*` | `first.qcore:2:1` | `second.qcore:1:1` | file 1 has no line 2 |
| `P21_span_line4` | `1:1`, caret under line 1 | `4:1`, caret under the bad axis | V04 F1 minimal case |
| `P22_span_dupaxis` | `Q010 @ 4:1`, `Q016 @ 1:1` | both at `4:1` | the two passes now agree |
| `P28_byte_offsets` | `1:1`, `bytes 17..24` | `3:18`, `bytes 48..55` | V04 F2; `missing` really is at byte 48 |
| `P25` + `P26` | blamed `P25` (file 1) | blames `P26` | V04 F1 cross-file |
| `P26` + `P25` reversed | blamed `P26` | blames `P26` | blame no longer follows argument order |
| `P25` + `P27` | accepted | accepted | control, unchanged |

Every rejecting row exits 1 before and after.

### Tests

Three new tests, all of which fail on the unrepaired tree:

| Test | Pins |
|---|---|
| `compiler/tests/unit/driver.rs` `attributes_diagnostics_at_a_file_boundary_to_the_offending_file` | V07's reproducer shape (offending span at byte 0 of file 2), with and without a trailing newline on file 1, plus a three-file case for the `Q016` path. Asserts path, line, column and `line_source`. |
| `crates/qcore-frontend/tests/unit/frontend.rs` `unconditional_declaration_checks_report_the_offending_span` | V04's P21/P22/P28 shapes: the `Q016` span slices back to the offending declaration text; `Q010` and `Q016` agree on a duplicate; a parse failure reports the caller's byte offsets. |
| `crates/qcore-frontend/tests/unit/check.rs` `locates_the_declaration_that_failed_validation` | `check_located` indices, and that it accepts/rejects exactly what `check` does. Also gives `DuplicateDimension` and `DuplicateAxis` direct cases, a coverage gap V04 named. |

The existing `refuses_unchecked_source_contracts_in_every_input_file` is
unaffected: its `Q021` span is a clause span strictly inside file 1.

## 2. Diagnostic ownership

`docs/QCORE_SPEC_r0_3.md` §L21 now carries a per-code table with one canonical
owner each. The rule it states is the one the code already follows: **a semantic
pass owns a range, `qcore_frontend::frontend::source_diagnostic` is the
authoritative pass → code map, and any crate running that pass emits from that
range.** Within a range, an individual code names one condition at its emission
site. `AGENTS.md` points at that table rather than duplicating it.

Ownership was read out of the code, not out of prior prose. Every code in the
table was located by grep over `crates/` and `compiler/`, excluding tests:

| Namespace | Owning pass | Emitted by |
|---|---|---|
| `Q000` | `NameResolution` (per input file) | `qcore-verification::driver` |
| `Q001`, `Q016` | `Index` | `qcore-frontend` |
| `Q010`–`Q013` | `NameResolution` | `qcore-frontend` |
| `Q020`, `Q021` | `OrdinaryTypes` | `qcore-frontend`, `qcore-verification::driver` |
| `Q100`, `Q103` | `Usage`, `Ownership` | `qcore-frontend` |
| `Q200`, `Q202` | `ResourceFlow`, `CleanupLowering` | `qcore-frontend`, `qcore-verification::driver` |
| `Q300`, `Q302` | `TypedErrors`, `ErrorRegion` | `qcore-frontend` |
| `Q400`, `Q401`, `Q403` | driver invocation | `qcore-verification::driver` |
| `Q500` | `CEmission` | `qcore-frontend`, `qcore-verification::driver` |
| `Q600` | `Concurrency` | `qcore-frontend`, `qcore-verification::driver` |
| `Q700`–`Q703` | `PluginVerification` | `qcore-verification` |
| `QP001`–`QP016` | provider host | `qcore-verifier-host`, `qcore-verification` |
| `<plugin-id>.<local>` | — | providers |

No historical report was rewritten.

### Decision — `Q700`–`Q703` drift (R09 residual, B02)

`docs/reports/remaining-b02/README.md` recorded that `AGENTS.md` named
`Q700`–`Q703` while `docs/QCORE_SPEC_r0_3.md:418` named only `Q700` and `Q701`.
**The code is the authority**, and it has all four: `Q700` and `Q701` in
`crates/qcore-verification/src/driver.rs` and `src/verify_dispatch.rs`, `Q702`
(conflicting provider evidence or assumptions) at `verify_dispatch.rs:318`, and
`Q703` (a provider allegation the core cannot validate) at
`verify_dispatch.rs:268`. The **spec row was stale and is updated**; `AGENTS.md`
was already correct and keeps its meaning. This is a documentation reconcilation
— no code, no diagnostic and no wire version changed.

### Decision — `Q020` (task route) vs `Q021` (sequential route)

V07 recorded that the task route rejects an unenforced source `contract { … }`
with `Q020` while the sequential route uses `Q021`, because
`compile_task_source` structurally never calls `check_whole_program`, where the
`Q021` gate lives.

**Decision: keep both codes, do not unify, until a source-contract consumer
lands.** Recorded in §L21 as intentional and route-specific. The reason is not
tidiness: the two gates do not have the same rejection coverage. The task gate
also refuses an empty `contract { }` (V07 c07), which the sequential route
accepts (V07 a07). Collapsing them onto one code would change what is rejected
on one of the two routes, and the card's rule is explicit that a code change is
semantic. Both directions are conservative today, so nothing is admitted while
the split stands.

The span defect that made the split visible in a multi-file build *is* fixed:
V07's a15 now names the file that actually carries the clause.

## 3. Reason codes for uncovered obligations — specified, not implemented

`docs/QCORE_SPEC_r0_3.md` §L21.1 adds a closed seven-category vocabulary —
`unsupported-syntax`, `missing-premise`, `incomplete-obligation-coverage`,
`stale-evidence`, `provider-failure`, `residual-custody`,
`environment-unavailable` — with the free-text reasons in force today mapped
onto it. The mapping was built from the real strings, not invented: the seven
`DerivationIncomplete` sites in
`crates/qcore-verification/src/obligation_derivation.rs`, the `unmodeled …` /
`unsupported …` family in the gateway importers (the reasons E01 measured, e.g.
`unmodeled expression CallExpr`), and the per-unit `"reason"` field written by
`crates/qcore-audit/src/report.rs`.

`environment-unavailable` is a separate category by design, so that the card's
hard rule holds by construction: **no environment failure may be relabeled as a
source defect.** A missing Clang, an absent build tree, a mismatched compile
database or an absent submodule cannot be folded into `unsupported-syntax` or
`residual-custody`, and cannot contribute to a provisional `Unsafe`. A live
example was produced by this card's own gate run — see §5.

**Not implemented, and the reason is structural rather than editorial.** The
card permits implementation only if additive and low-risk in `qcore-schemas`.
It is neither, on either candidate carrier:

- `DerivationIncomplete` is a field of `VerificationTask`, whose SHA-256 over
  canonical JSON travels in every evidence-v2 request and every witness, and the
  record is `deny_unknown_fields`. Adding a category field changes **every** task
  digest. `crates/qcore-schemas/src/obligation.rs:440-454` and
  `crates/qcore-schemas/tests/schema_canonicalization.rs:107` pin digest
  behaviour, and every shipped provider package (ChunkIO 0.4.0, CMetrics 0.2.0)
  and recorded witness would have to be regenerated in the same change.
- The audit report's per-unit `"reason"` is not digest-bearing, but adding a key
  there changes all four pinned 185-unit artifacts byte for byte.

Either belongs to its own increment that re-derives the pinned artifacts
deliberately and says so. §L21.1 records the two invariants that increment must
satisfy: environment failures stay their own category, and categories refine
rather than replace the verdict contract — source spans, `property`, semantic
profile, scope, bounds, assumptions and obligation IDs must be shown unchanged
across the migration.

This card's own changes satisfy the second invariant trivially: nothing in
`qcore-schemas` was touched, and no obligation ID, property or profile is
reachable from the two repaired sites.

## 4. Cycle 19 audit parity

The full 185-unit audit was **not re-run**, and could not be: this worktree has
no prepared Clang build tree, so the audit fails closed with

```
audit requires the configured Clang executable (/usr/lib/llvm-21/bin/clang);
compilation database selects /usr/bin/cc. … GCC compiler equivalence is unproven
```

which is the gate behaving correctly, not a regression.

Parity is instead established mechanically, by disjointness. `locate` is private
to `qcore-verification::driver` and has exactly three callers, all inside the
QCore source-language compilation route (`driver.rs:389`, `:1193`, `:1278`).
`crates/qcore-audit/src`, `compiler/examples/support/cycle19_compatibility.rs`
and `crates/qcore-c-gateway/src` reference none of `compile_files`,
`elaborate_source`, `StaticEnvironment` or `check_located` — verified by grep.
The C audit cannot reach either repaired site, so its artifacts cannot change.

Contract corpus counts, the pinned submodule revision and CT mapping are
untouched.

## 5. Standing gate

Run on `bfbad21` plus this card's changes.

| Step | Result |
|---|---|
| `git -c protocol.file.allow=always submodule update --init --recursive` | exit 0 |
| `cargo test --workspace` | exit 0 — **890 passed, 0 failed, 0 ignored** |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `cd proof && lake build` | exit 0 — 62 jobs |

Test delta explained per case:

- **+3** new tests, listed in §1. No existing test changed expectations.
- **+6** tests in `crates/qcore-verification/tests/source_effect_obligations.rs`
  became runnable at all. See the finding below.

### Out-of-scope repair, flagged

**`bfbad21` does not compile.** `cargo test --workspace` fails on clean `HEAD`
with

```
error[E0063]: missing field `additional` in initializer of `qcore_schemas::PluginLimits`
   --> crates/qcore-verification/tests/source_effect_obligations.rs:542:17
```

This is a merge defect between two cards that were merged independently and
never rebuilt against each other: E01 (`ffb768a`) added the initializer, and P02
(`bfbad21`) added the `PluginLimits::additional` field. It is not caused by this
card, and it blocks the entire standing gate.

Repaired minimally as `additional: BTreeMap::new()`, which is exactly what the
fixture meant before the field existed — it declares no additional limit kinds,
and the field is documented as empty by default and skipped when serializing.
No limit is weakened and no assertion changed. Called out here rather than
folded in silently, because it is outside D01's scope.

## 6. What this does not establish

The repairs are to diagnostic attribution only. They do not change what is
accepted or rejected, do not promote any raw Cycle 19 unit, do not change any CT
mapping, and do not touch the plugin protocol, evidence, schema or package
versions. `Q021` remains a refusal to enforce source contracts, not an
implementation of them. The reason-code vocabulary is specified prose with a
mapping onto today's strings; no field carries it yet, so no consumer can filter
on it. V04-F3 (first-error truncation) remains as it was, by decision.

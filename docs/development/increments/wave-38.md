# Wave 38 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

**`STABILITY-REEVAL-3` (2026-09-18).** The cadence's third firing, over waves
36–37, base `e69083a6`, an audit: no gate weakened, no frozen archive edited, no
verdict rule changed, no semantic change; one test landed and **fourteen**
residuals registered with discharge conditions, `named_followups` **215 →
229**, **zero retirements** (named explicitly: `GATE-ROW-FIGURE-ATTRIBUTION-DRIFT`
deliberately kept as partial, its second arm unlanded). **Step 0**, firing #2's
47 residuals: **10 discharged / 14 partial / 23 open**, each verdict re-checked
at its cited line; of the 18 that need a mechanism **none** was discharged in
two waves, and the only discharges that did land were items also on firing
#2's integrator list — the sweep discharges, the tracker does not. One of the
open 23 escalated from hazard to live defect: `F02-DIGEST-PIN-DUPLICATED`'s
predicted drift has happened — `windows_threads.rs:31,36` still pins the
fourth-move digests while `f02_trace_relation.rs:96-97` pins the fifth, the
same FNV-1a over the same LF bytes, so the Windows F02 row is **red on every
Windows host since `fa48211a`** and no Linux gate can see it
(`F02-WINDOWS-PIN-STALE-SINCE-WAVE-36`; not re-pinned here, that being
`37bc9004`'s shape). **Step 1**, the waves' fail-closed arms: the scalar
card's retained-exit arm is driven end to end with harness and log shipped,
and its back edge is now exactly the shape firing #2 found the exit edge in —
value-route deletion under the unwritten `volatile` premise, false-`Safe`
witness measured once and pinned nowhere — but named and registered; the
round-bound card's bound is **proved** of an abstract sweep under a descent
hypothesis that is **observed** of the Rust transfer, its declining arm driven
directly at `iterate` and unreachable through `derive` by the theorem, and
its mutation harness on a gitignored path; `W37-CORPUS-CONTRACTS-4`'s
withdrawal arm moved from fixture-verified (`loader_abi.rs:500`, since
`5f3c3cf7`) to corpus-observed, its leaf-neutrality true but supported by a
counting script rather than the set equality it claims; the two TSan tests
share no compilation, executable or binary — the unbound rows are evidence
about the toolchain, the bound rows about a pthread-substituted program, and
the wave-36 row has a silent arm and a tautological one; CT07's byte-identical
trace is the parent's order by construction in both halves; and
`CT-FIXTURE-EXTERNAL-READERS`' mechanism runs the **other** way — the harness
refuses a manifest re-point, while the proof stage was credited to a
`file::test` string nobody read, which this card closes with
`proof_gated_cases_name_a_live_test_that_reads_the_cases_own_fixture`
(`ct_corpus.rs`), three of three mutations red, restore sha256-equal. Wave
36's guard is tested but its pack-refs refusal is pinned by nothing and is
two transactions, not one; the `--check` content arm is still 8 of 8 red by
mutation at this base; the `lean_audits` key is red by mutation and
**optional**. Directly measured: 67 of 215 tracker entries carry no
`recorded_by` (28 of them registered in waves 36–37); four of the two waves'
cards state mutation counts whose harness lives under `measure/` or
`private/`; the Clang early-return guard is a **750**-guard family over 79
files and no `gate.json` records Clang presence (this card's does). **Step
3**: `797661f3`'s `loop_exit_edge: None,` is discharged with no residual on
the measured ground that `legacy_joined_provenance.rs` reads only `successors`
— the field is inert for the test — with the fixture-spelling hazard behind
it registered; the gate-row Lean drift's discharge landed one of its two
arms; `--uncarded` over the range lists 19 commits, every one a card's own or
the close's, zero integrator commits outside merges. **Step 4**, one paragraph
each with the load-bearing claim named: `CT42-CELL-ROUTE` reaches an
*admitting* fallback arm the frozen report calls refusing;
`AGGREGATE-OWNED-FIELD` breaks the name-keyed owner vocabulary before any of
15k's four rules; `CT07-TWO-THREAD-PROOF-LEG` cannot bind on the
CT01/CT37/CT03 pattern because `reference_trace` refuses CT07's run by 15n's
decided boundary; `TSAN-ROWS-2` is cheap on the task route through
`QCORE_TASK_CREATE` and impossible by construction on CT46;
`LOOPS-ON-COMPOSED-ROUTES` falsifies a top-level-index window computation and
two of its three target names reach no tracker. Gate detached at `6368cc7b`:
**4,637 / 0 / 8** over 233 summaries, fmt clean, clippy **101 in the detached
run on this card's own unrepaired test** then 0 after a one-line repair, both
probes 0, **137** Lean, **295** Python before and after the regeneration with
`--gate docs/reports/wave37-close/gate.json`, Clang 21.1.8 recorded. Nothing
promoted. `docs/reports/stability-reeval-3/README.md`.


# W45-AUDIT-CHECKPOINTS: per-group `.qo` checkpoints, resume and streaming composition

Card W45-AUDIT-CHECKPOINTS, 2026-09-26, on base `84ebad57` ("Record the
w45-fb-retry-lifetime integration and archive the combined baseline"), with
`main` at `2f9131a8` (W45-CONTRACT-REVISION-MODE) merged mid-card.

The user's decisions (2026-09-26): crates.io dependencies are allowed, the
decomposer writes both YAML and TOML, and checkpoints serve both **resume** and
**composition**. Two amendments followed: the files are `.qo` ("Quirin-audit
Object") with a `QO` magic, and the purpose is **memory** — peak memory bounded
by one group, not the corpus — so a checkpointed run processes one group at a
time and drops it, composition streams, and every `.qo` records its group's
peak memory and wall time as provenance. The coordinator added: key the
identity on every `contracts.lock` in force, and keep the composition's verdict
fold in one small function (`compose::fold_verdicts`) so the four-valued
verdict semigroup of W45-USER-CONTRACT-PLUGINS can replace it.

## Headline

* **Checkpoints.** `quirin-audit current --checkpoint-dir D` analyses one
  library group at a time, seals it into `D/<label>.qo` (a `QO` magic, format
  version 1, a postcard header of ten SHA-256-digested sections) and drops it;
  the report is composed from the checkpoints. `quirin-audit checkpoint
  verify | compose | decompose | recompose` inspect, compose and convert them.
* **Composition is the whole run.** On the 185-unit corpus inputs (186
  configurations in 6 groups) every checkpointed run — 1, 4 and 16 jobs, a
  no-change resume, a one-group-change resume — and a standalone composition
  write all five artifacts byte-identical to the whole run's
  (`baseline_identity.py` against the whole run's archive), and the same
  console transcript apart from the measured `preparation:` line.
* **Resume.** A no-change rerun at 16 jobs takes 5.1 s instead of 130.8 s,
  reusing all six groups; changing one group's 13 compile commands recomputes
  only that group, in 7.1 s against the whole run's 98.8 s, byte-identical to
  a whole run over the changed database. Reuse needs every identity field equal,
  including the contract locks and the pin evaluation.
* **Determinism.** Every section but `provenance` is identical at 1, 4 and 16
  jobs (equal content digests); every checkpoint round-trips through YAML and
  TOML to its original bytes; truncated, tampered and version-mismatched files
  are refused whole.
* **Memory.** At one job a checkpointed run peaks at 111 MiB against the whole
  run's 172 MiB (−35 %), exactly the largest group's peak; at 4 and 16 jobs
  331 against 392 and 872 against 928 MiB, because the analyses in flight
  inside librdkafka set the peak there. Reaching this needed the pin
  evaluation's source read to stream: it had held the checkout's 85 MiB of
  C twice and set a 182 MiB setup peak (§7). Composition peaks at 31 MiB.
* **No verdict change.** Off by default; the corpus arbiter's diagnostics are
  byte-identical to the W45-FB-RETRY-LIFETIME baseline (functions 389 / 12 /
  4,433 unchanged).
* **Lean.** `QCore.Checkpoints`: 14 theorems on identity-guarded reuse,
  composition and admission, no placeholders, no project axioms.

## 1. What a user runs

```sh
quirin-audit current --checkout … --profile-directory contract/profiles/fluent-bit \
  --monkey-database … --report out/resource-diagnostics.json --jobs 16 \
  --checkpoint-dir out/qo            # one .qo per group; --resume reuses
quirin-audit checkpoint verify out/qo/*.qo
quirin-audit checkpoint compose out/qo/*.qo --report again/resource-diagnostics.json
quirin-audit checkpoint decompose out/qo/chunkio.qo --format toml --output chunkio.toml
quirin-audit checkpoint recompose chunkio.toml --format toml --output chunkio.qo
```

Both forms of `current` take the settings (`--checkpoint-dir`/`--resume`, or
`checkpoint_dir`/`resume` in the JSON configuration, which must agree).
Checkpointing is off by default. The invocation record names the checkpoints
only when they are used, so every other run's record is unchanged.

## 2. Format and dependencies

The format is specified in
[docs/development/audit-checkpoints.md](../../development/audit-checkpoints.md):
the `QO` magic, a big-endian format version (1), a guard, a postcard header
with the schema version (1) and the ten-section manifest (name, length,
SHA-256), the header's own SHA-256, and the postcard-encoded sections. The
crate README summarises it.

| Crate | Version | Licence | Why |
|---|---|---|---|
| `postcard` | 1.1.3 | MIT OR Apache-2.0 | The binary encoding: a serde format with a published, stable wire specification (1.0), deterministic for a given value, compact varints. Built with `default-features = false, features = ["use-std"]`. |
| `serde_norway` | 0.9.42 | MIT OR Apache-2.0 | YAML. `serde_yaml` is deprecated upstream; this is its maintained fork with the same API. |
| `toml` | 1.1.6+spec-1.1.0 | MIT OR Apache-2.0 | TOML 1.1, the toml-rs project's serde implementation. |

They bring in, transitively: `cobs` 0.3.0 and `thiserror`/`thiserror-impl`
2.0.21 (MIT OR Apache-2.0) for postcard; `unsafe-libyaml-norway` 0.2.15 (MIT),
`indexmap` 2.14.2, `hashbrown` 0.17.1, `equivalent` 1.0.2 (MIT OR Apache-2.0)
and `ryu` 1.0.23 (Apache-2.0 OR BSL-1.0) for YAML; `toml_parser`,
`toml_writer`, `toml_datetime`, `serde_spanned` (MIT OR Apache-2.0) and
`winnow` 1.0.4 (MIT) for TOML. `embedded-io` 0.4.0/0.6.1 appear in the lock
as postcard's optional dependencies and are not built. Both `Cargo.lock` and
`providers/Cargo.lock` gained the same 18 packages, because the provider
workspace's conformance crate depends on `qcore-audit`; no provider crate uses
them. The standing gate's offline builds use the local cache.

Disclosure: while choosing, a throwaway `cargo tree` over the candidate YAML
crates downloaded `serde-saphyr` 1.3.0 and `serde_yaml_ng` 0.10.0 with their
dependencies into the local Cargo cache. Nothing of them is used or locked.

## 3. Groups and steps

Groups are the route's groups: `audit.json`'s libraries under their labels
(`cmetrics`, `ctraces`, `librdkafka`, `chunkio`, `fluent-bit-chunk`), or an
explicit `--selection`'s groups, then `monkey-libevent`. The steps are the ten
sections (identity, plan, global, preparation, lowering, verdicts,
certificates, contracts, result, provenance), each digested separately; the
specification tabulates their contents. On the corpus the six checkpoints
hold 31, 17, 94, 13, 2 and 29 configurations (186 in all, 185 source files).

Two steps are narrower than the brief's list, because the route publishes no
more. The route publishes each function's summary as one row (verdict,
diagnostics, counted-user clauses), so the per-function summary and the
per-function verdict and diagnostics are one step (`verdicts`). The
`certificates` step records what the rows establish about each `Safe`
certificate (bound to the unit's compile-task identity, binding refused, or
none), not the certificate body, which is not a serde type
(`W45-CHECKPOINT-CERTIFICATE-BODIES`).

## 4. Independence audit

Composition is sound only if no group reads another's results. Every
cross-unit and cross-group input of `quirin-audit current`:

| Input | What it is | Handling |
|---|---|---|
| The audit's shared state (`CurrentAudit`: checkout, `HEAD`, Clang identity, contracts after the pin evaluation, dependency digests, profile digest) | read-only, computed once | the run identity; every checkpoint must agree |
| A unit's source, headers, command, preprocessed text | the unit's own | the unit fingerprint |
| The gateway's interprocedural (W4) callee summaries | built inside one translation unit's analysis | none needed: no table outlives a unit |
| The gateway's per-thread member-release memo (`legacy_member_release.rs`) | a one-entry cache per thread, keyed by a 64-bit hash of the token texts | a deterministic function of the tokens except under a hash collision; the one state that can cross units; not composition-specific (the job-count identity rests on it too); follow-up `W45-GATEWAY-MEMBER-RELEASE-MEMO-KEY` |
| Census sinks | process-global streams in analysis order, numbered by global counters | checkpoints refuse a census run |
| The Monkey source copy check | decides the group-wide refusal of every Monkey unit | the Monkey group's identity (`unresolved`) |
| The Monkey source report, chunk contracts, source effects | the route writes explicit Unknown placeholders, a function of the revision and the historical reference revision | the `global` section; the composer writes them |
| The report's tallies (`diagnostic_counts`), the unit counts | sums over units | summed per group, then over groups |
| `source_file_count` | the number of distinct canonical sources | per-group source lists, united |
| `pending_human_review` (W45-CONTRACT-DEFERRED-REVIEW) | markers from the contracts; held calls from every row | markers in `global`; per-group held calls, united and summed; `pending_review::render` |
| `contract_revision` (W45-CONTRACT-REVISION-MODE) | a template from the contracts and pin evaluation; analysed functions and pin-held calls from every row | template in `global` (and its digest in the identity); per-group facts, united; `contract_revision::render` |
| The contract locks and pin evaluation | read once, before any unit | lock digests and the template digest in the identity |
| The checkout `HEAD` check at the end | the whole run refuses if `HEAD` moved | the checkpointed run checks it before composing |
| Console lines | the preamble, each group's header and unit lines, the totals | preamble in `global`, the rest in `result`; the composer reprints them |

Two report-wide sections, `pending_human_review` and `contract_revision`,
previously read the finished report whole. Each was split, with its behaviour
unchanged, into a report-independent part and a per-row fold
(`pending_review::{markers, HeldCalls, render}`,
`contract_revision::{template, ReportFacts, render}`), and the whole run now
calls the same functions, so its sections and the composed ones come from one
renderer.

The corpus arbiter's own route, the pinned replay in the compiler example, is
not checkpointed (`W45-CHECKPOINT-PINNED-REPLAY-ROUTE`). Audited the same way,
its extra dependencies are all composable. The ChunkIO library prelude
(frame-helper summaries, the memory-backend and filesystem certificates) and
postlude (the nested-memory chain over the captures of several units) are
group-local. The native and teardown results feeding the chunk-contracts
sidecar are an ordered concatenation across the ChunkIO and
`fluent-bit-chunk` groups. The Monkey gate (the libevent contract check, the
source-copy check and the callback source report) runs between the Fluent Bit
groups and Monkey, and belongs to the Monkey group. None of it is serializable
today, being typed certificates held in memory.

## 5. Identity and resume

The identity is specified in the format document: the run part (analyzer
build commit and binary SHA-256, mode, checkout and `HEAD`, profile digest,
contract manifest and sources digests, provider and evidence records, every
`contracts.lock` read with its SHA-256, the pin evaluation's template digest,
Clang identity, `QCORE_*` environment) and the group part (label, group-wide
refusal, and per unit the compile-configuration digest and preparation facts).
The fingerprint is taken from the row the analysis published and the
preparation it ran, so it is exactly what was analysed; `--resume` recomputes
it by preparing each unit (three Clang runs) without analysing it.

Measured once each on the corpus inputs with the release binary
(`scripts/run_current.py`, `scripts/measure.sh`, sequentially on a shared
host; wall times are noisy):

| Run | Wall | Peak RSS | Groups |
|---|---|---|---|
| whole run, 16 jobs | 130.8 s | 930 MiB | — |
| checkpointed, 16 jobs | 115.8 s | 912 MiB | 6 recomputed (`resume not requested`) |
| resume, no change, 16 jobs | **5.1 s** | 483 MiB | 6 reused (`identity equal`) |
| resume, ChunkIO's 13 commands changed, 16 jobs | **7.1 s** | 494 MiB | `chunkio` recomputed (`unit 0 (lib/chunkio/src/chunkio.c): compile configuration differs`), 5 reused |
| whole run over the changed database, 16 jobs | 98.8 s | 922 MiB | — |
| whole run, 1 job | 873.2 s | 182 MiB | — |
| resume, no change, 1 job | **26.1 s** | 182 MiB | 6 reused |

A no-change rerun saves 96 % of the wall time at 16 jobs and 97 % at one; a
one-group change saves 93 %. What remains is the identity check: reading the
analyzer binary, and preparing every unit. The changed-database run is
`scripts/probe_database.py`: `-DQCORE_CHECKPOINT_PROBE=1` added to the 13
ChunkIO commands. Because the plan records every unit's command digest, the
five reused groups were re-sealed with the new plan, their step sections
unchanged, so their content digests differ from the fresh run's (the plan is
content). Per-group records are in [evidence/checkpoints/](evidence/checkpoints/).

The fixture test `resume_reuses_exactly_the_groups_whose_identity_is_unchanged`
pins the same behaviour and more: an uncommitted source change recomputes
only its group, a tampered checkpoint is recomputed (`checkpoint refused: …`),
a changed contract recomputes every group (`contract set differs`), and a
fresh run seals the same content as the resumed one.
`a_re_pin_invalidates_every_checkpoint` runs `quirin-audit pin` between two
runs: the new `contracts.lock` enters the identity and every group is
recomputed.

## 6. Composition identity on the 185-unit corpus

The reference is the whole `quirin-audit current` run at 16 jobs over the
arbiter's corpus inputs, archived as [artifacts/current-whole/](artifacts/current-whole/)
by the arbiter's own `archive.py` (report `3b92dac1…`, contract sidecar
`a1d98b6a…`, and the three Unknown sidecars `9fe99367…`). Each run below was
compared with it by `baseline_identity.py` (all exits 0,
[evidence/identity/](evidence/identity/)):

| Run | Report | `.contracts.json` | chunk-contracts | source-effects | monkey-source |
|---|---|---|---|---|---|
| whole run, 4 and 1 jobs | identical | identical | identical | identical | identical |
| checkpointed, 16, 4 and 1 jobs | identical | identical | identical | identical | identical |
| `checkpoint compose qo-j16/*.qo` | identical | identical | identical | identical | identical |
| resume, no change | identical | identical | identical | identical | identical |
| resume after the ChunkIO change, against the whole run over the changed database | identical | identical | identical | identical | identical |

Byte-identical, not "after an alias": the report's `.invocation.json`
sidecar, provenance by definition, is the only output that differs. The
console transcripts of the checkpointed, resumed and composed runs equal the
whole run's once the `checkpoint …` status lines (standard error) and the
measured `preparation:` line are set aside.

The standalone composer's refusals are pinned by
`composition_is_the_whole_run_and_refuses_an_incomplete_or_mixed_set`: a
missing group (`missing: ["beta-group"]`), a group given twice, checkpoints of
two contract sets (`disagree on the contract set`) and a truncated file are
refused, and no report is left behind.

## 7. Memory

### The setup read the whole checkout twice; it now streams

At the first measurement, a checkpointed run at one job peaked at the same
182 MiB as the whole run, although no group's sampled peak exceeded 115 MiB,
and a no-change resume, which analyses nothing, peaked at 182 MiB too. The
peak was in the route's setup, before the first group: W45-CONTRACT-REVISION-
MODE's pin evaluation (`contract_revision::apply_pins`) read every `.c` and
`.h` blob of the checkout's `HEAD` through `git cat-file --batch` into one
buffer, then decoded it into a second map of strings, and only then scanned
it for the pinned functions' sites. On the corpus checkout that is 4,926
files, 85 MiB, held twice.

After the integration review asked for it (item 4), the evaluation reads the
sites one file at a time (`c_identity::SourceTree::read_sites`: it streams
`git cat-file`, or walks the directory, and scans and drops each file). The
sites are sorted per function, so the order does not matter, and the unit test
`streamed_sites_equal_the_whole_tree_sites` checks the streamed sites equal the
whole-tree read's, on a Git checkout whose working tree differs from `HEAD`
and on a plain directory. `quirin-audit pin`, which writes locks, still reads
the whole tree. No other setup step reads the corpus whole: the rest loads the
contracts, the profile and the compile databases. The arbiter's contract and
source-effects sidecars are byte-identical before and after the change.

### Measurements

Measured once each at `3df36e86` (`scripts/measure_memory.sh`,
[evidence/memory/](evidence/memory/)), one run at a time on a shared host.
Peak resident set size is from `/usr/bin/time -v`. Each group's peak is sampled
every 10 ms from `VmRSS` and recorded in the `.qo` provenance and in
`checkpoint-run.json`. The sampler never resets the kernel's high-water mark,
so `/usr/bin/time` still measures the whole process.

| Jobs | Whole run | Checkpointed | Change | Largest group's sampled peak |
|---|---|---|---|---|
| 1 | 172 MiB | **111 MiB** | −35 % | 111.5 MiB (librdkafka) |
| 4 | 392 MiB | 331 MiB | −15 % | 316 MiB (librdkafka) |
| 16 | 928 MiB | 872 MiB | −6 % | 872 MiB (librdkafka) |
| compose | | 31 MiB | | one group's step sections |

* **One job: the peak is the largest group's.** The checkpointed process peak
  (111.7 MiB in `checkpoint-run.json`) equals librdkafka's sampled peak. The
  first group, cmetrics, runs at 41 MiB, so the setup is now below every
  large group. A no-change resume at one job peaks at 56 MiB in the process
  (the 103 MiB `/usr/bin/time` reports for that run is its largest child,
  `git cat-file` or a Clang preparation; `ru_maxrss` of waited children is
  the largest descendant's, not a sum).
* **Before the streaming change** the pairs were 182/182, 397/406 and
  930/912 MiB (whole/checkpointed, at 1/4/16 jobs,
  [evidence/runs/](evidence/runs/)), so the setup had erased the one-job
  saving. Before the W45-CONTRACT-REVISION-MODE merge they were 168/110 MiB at
  one job (measured once, logs private).
* **`--jobs` multiplies memory.** Inside a group every job holds one unit's
  analysis and Clang output, and librdkafka (94 units) sets the peak in both
  modes. Checkpoints bound what the run *retains* (the rows: about 60 MiB, the
  one-job difference), not what it *computes*; `--jobs` is the working-set control.
  No separate memory-aware limit is added (`W45-CHECKPOINT-JOBS-MEMORY`).
* **Composition streams.** Composing the six checkpoints (18.4 MB of `.qo`)
  into the 17 MB report takes 0.15 s and 31 MiB.

A group's sampled peak includes memory the allocator kept from earlier groups,
because RSS does not fall when state is freed. A later, smaller group can
therefore show a higher sampled peak than its own work needs.

Every run of this series is byte-identical to the whole-run archive in all
five artifacts (`baseline_identity.py`, exit 0 for all eight), and
`checkpoint verify` passes on all 18 checkpoints.

**Wall time** is noisy on this host: other agents' gates ran at the same time.
The first series measured the checkpointed run at 115.8 s against 130.8 s (16
jobs) and 288.1 s against 141.7 s (4 jobs). This series measured 286.6 s
against 126.9 s (16 jobs) and 286.0 s against 279.1 s (4 jobs). One group in
flight lets a group's longest unit idle the other jobs (librdkafka took
213–230 s in the slow runs); the cost is real but its size is not settled here
(`W45-CHECKPOINT-GROUPS-IN-FLIGHT`).

## 8. Determinism and round trips

* **Job counts.** The six checkpoints of the 1-, 4- and 16-job runs have equal
  content digests group by group (`chunkio` `3da1df16…`, `cmetrics`
  `93de1115…`, `ctraces` `6ffcc617…`, `fluent-bit-chunk` `02280462…`,
  `librdkafka` `3d9a4bce…`, `monkey-libevent` `2e4f755f…`), and so does every
  group of the no-change resume; `checkpoint verify` exits 0 on all
  ([evidence/identity/verify-*.txt](evidence/identity/)). Whole files differ only
  in `provenance`.
* **Text round trips on the corpus.** Each of the six corpus checkpoints was
  decomposed to YAML and to TOML and recomposed: 12 of 12 byte-identical to
  the original (texts of 0.9–13.7 MB); a YAML text with one identifier edited
  is refused (`the text's contents do not match its recorded manifest`)
  ([evidence/roundtrip.txt](evidence/roundtrip.txt), `scripts/roundtrip.sh`).
* **Unit tests** (`checkpoint.rs`, 9): a row splits into its step members and
  renders back exactly as serde_json prints it at depths 0–3; an unknown row key
  stays in the verdict step; the binary form round-trips and is deterministic,
  and provenance alone is outside the content digest; truncation at six
  offsets, one flipped byte at every seventh offset, a trailing byte, another
  format version, a bad magic and a future schema version are each refused
  whole; both text forms round-trip to the same checkpoint and bytes; an
  edited text is refused; the section reader checks each section on read and
  refuses a truncated file; the report frame splices rows byte for byte.
* **Integration tests** (`tests/checkpoint_cli.rs`, 6, on a two-group Git
  fixture with pin-withheld contracts): one checkpoint per group and the whole
  run's five outputs at 1 and 4 jobs, with equal content digests; resume and
  its reasons (§5); a selection with a file that has no compile record (an
  absent unit) resumes and composes; composition and its refusals (§6); a
  re-pin invalidates every checkpoint; and the YAML and TOML round trip through
  the command line on real checkpoints.

## 9. Lean

`QCore.Checkpoints` (`proof/QCore/Checkpoints.lean`), audited by
`proof/AuditCheckpoints.lean`: 14 theorems, no placeholders, no project
axioms.

- **Identity-guarded reuse.** `reused_eq_recomputed`: under `Determined` (a
  unit's row is a function of the run identity and its fingerprint), a sealed
  checkpoint whose identity equals the recomputed one yields exactly the
  recomputed rows, whatever environment and units it was sealed from;
  `resume_eq_recompute` extends it to every stored state (none, or sealed);
  `rows_eq_of_fingerprints_eq` is the induction under it;
  `reuse_takes_the_stored_rows` records why the premise matters.
- **Composition.** The publication fold (rows appended, tallies added, sources
  united) is a monoid (`Publication.join_assoc`, `empty_join`, `join_empty`);
  `publish_append` makes the fold a homomorphism, so `compose_eq_whole` holds
  for any split into consecutive groups; `group_rows_flatten` is the
  independence premise's consequence; `resume_then_compose_eq_whole` joins the
  two.
- **Admission.** `compose_admits_one_run`, `compose_covers_the_plan`, and
  `one_run_composes_to_the_whole` (one run's sealed checkpoints are admitted and
  compose to the whole run).

The Rust boundary — that the fingerprint captures every input (§4), that every
digest is checked, that the renderer prints the fold byte for byte — is pinned
by the tests in §8 and the corpus comparison in §6.

## 10. No verdict change

Checkpointing is off unless `--checkpoint-dir` is given, and no analysis code
changed: the whole-run route only gained a preparation-facts field on each
configuration's products and now renders its two report-wide sections through
the split functions of §4. The corpus arbiter (`run_audit.py`, 16 jobs, this
card's release build at `17413f6a`) against the combined baseline
`docs/reports/w45-fb-retry-lifetime/integration/artifacts/`:

| Artifact | Result |
|---|---|
| diagnostics (verdict-bearing) | **byte-identical** (`baseline_identity.py` exit 0) — so units, functions 389 / 12 / 4,433 and every leaf are unchanged |
| chunk-contracts, monkey-source | byte-identical |
| contracts | W45-CONTRACT-REVISION-MODE's own edits and nothing else (its `contracts_difference.py`, exit 0: the additive `contract_revision` key, 16 libc modules, two included libc parents) |
| source-effects | identical after the checkout alias and masking the path-bound digests (10 per side, as every W45 card discloses; `scripts/mask_digests.py`) |

The whole `quirin-audit current` run's report is also unchanged across the
card: the pre-merge base binary and this card's first build wrote the same
bytes (`0bc5ff04…`, measured once, private).

## 11. Gate

Run at `182d4b21` in this worktree, `CARGO_INCREMENTAL=0`, each exit to a
file (`private/gate-final/`), after the corpus measurements and never beside
them. Recorded in [gate/gate.json](gate/gate.json), with Clang beside it.

| Command | Exit | Count |
|---|---|---|
| `git submodule update --init --recursive` | 0 | |
| `python3 scripts/run_test_binaries.py` | 0 | 303 units, 313 summaries; 5,855 passed, 0 failed, 11 ignored (`private/test-binaries/20260926T091617Z-1386047/`) |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| `python3 scripts/check_lean_audits.py` | 0 | 199 Lean audits |
| `python3 -m unittest discover -s scripts/tests` | **1**, then 0 | 358 tests; the first run failed `test_the_derived_facts_are_recomputed_rather_than_carried` because the committed matrix counted 198 Lean audits against 199 on disk; the matrix was regenerated with `build_matrix.py . --gate gate/gate.json` and the rerun passed |
| `providers/package.py build`, `verify qcore.libevent` | 0, 0 | |
| `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline` | 0 | 124 passed, 0 failed, 0 ignored over 39 summaries |
| provider `fmt --check`, `clippy -D warnings` | 0, 0 | |
| `python3 -m unittest discover -s providers/tests` | 0 | 15 tests |
| `cargo build --workspace --locked --offline` | 0 | |
| `cargo clean --profile dev` | 0 | |
| `scripts/build_docs_index.py --check` | 0 | |

The provider gate ran because `providers/Cargo.lock` changed (the new
crates reach it through the conformance crate's `qcore-audit` dependency).

## 12. Follow-ups

Registered in the acceptance matrix's `named_followups`:

- `W45-CHECKPOINT-PINNED-REPLAY-ROUTE` — the corpus arbiter's own route is not
  checkpointed (§4).
- `W45-CHECKPOINT-CONTRACTS-AND-CONFIGURED-ROUTES` — `quirin-audit contracts`
  and `quirin-audit CONFIG.json` have no `--checkpoint-dir` (their files were
  being edited by W45-CONTRACT-REVISION-MODE).
- `W45-CHECKPOINT-CERTIFICATE-BODIES` — certificate bodies and callee summaries
  are not serde types and are not recorded (§3).
- `W45-CHECKPOINT-JOBS-MEMORY` — inside a group, memory scales with `--jobs`
  (§7).
- `W45-CHECKPOINT-RESUME-PREPARATION-DUPLICATE` — the resume fingerprint
  repeats the route's preparation steps; a recomputed group prepares twice.
- `W45-GATEWAY-MEMBER-RELEASE-MEMO-KEY` — the per-thread memo keyed by a
  64-bit hash (§4).

Not a follow-up but a boundary: composition folds unit verdicts by summing
tallies (`compose::fold_verdicts`); the per-group four-valued join belongs to
W45-USER-CONTRACT-PLUGINS's registered follow-up.

## Files

- `crates/qcore-audit/src/checkpoint.rs` — the format, the sections, row
  splitting and rendering, the YAML/TOML text form.
- `crates/qcore-audit/src/checkpoint_run.rs` — the group-by-group run, identity,
  resume, RSS sampling.
- `crates/qcore-audit/src/compose.rs` — streaming composition.
- `crates/qcore-audit/src/checkpoint_cli.rs` — `quirin-audit checkpoint`.
- `crates/qcore-audit/build.rs` — the analyzer build commit.
- Hooks: `current_route.rs` (the branch, shared task evaluation and Unknown
  sidecars), `current_cli.rs` (flags and configuration), `current.rs` (the
  preparation facts on each configuration's products, read-only accessors),
  `report.rs` (the report document and skeleton), `pending_review.rs` and
  `contract_revision.rs` (the per-row folds, and the streamed site read),
  `c_identity.rs` (`SourceTree::read_sites`), `main.rs`, `lib.rs`.
- `crates/qcore-audit/tests/checkpoint_cli.rs`, `proof/QCore/Checkpoints.lean`,
  `proof/AuditCheckpoints.lean`.
- `docs/development/audit-checkpoints.md`; `scripts/run_current.py` and
  `scripts/probe_database.py` here; `artifacts/current-whole/`, the whole-run
  reference archive.

# MEASURE-TOOL

Card MEASURE-TOOL builds seam F of the
[pluggable-cards plan](../../plans/pluggable-cards/README.md) (phase P0):
measurement as the `quirin-audit measure` subcommand plugin (user decision 8
of 2026-10-25), behind the same out-of-process firewall and receipts as every
other subcommand (contract specification C11.2). It replaces the copied
`measure.py`, `compare.py`, `mirror_inputs.py` and `gate_job.sh` scripts. It
changes no analyzer, contract, route or theorem, so it needs no identity run
and adds no Lean (no new semantics). Base: local main `373e41ee8` (AUDIT-COMPOSITION
merged, `qcore-schemas` 21.0.0); the branch merged origin/main `e37e6703d`
(CONTRACT-LOCKS, AUDIT-COMPOSITION) and `8a015cd83` (CFG-RELEASE,
EVALUATION-PROFILES).

## What changed

**Crate `crates/quirin-audit-plugin-measure`**, binary `quirin-audit-measure`,
wired exactly as `quirin-audit-plugin-libevent`: `main` calls
`quirin_audit_plugin::run`, the workspace lists the crate, and the layering
pins (`compiler/tests/workspace_layering.rs`,
`crates/quirin-audit-plugin/tests/dependencies.rs`) allow it the SDK and
`qcore-schemas` only. It links no analysis crate: it reads results only
through `qcore_schemas::composition`. Usage is documented once, in
[the plugin page](../../development/audit-subcommand-plugins.md#the-measure-plugin);
`docs/agents/operations.md`'s card-brief evidence item now points there.

| Module | Role |
|---|---|
| `inputs` | Routes: the live evaluation profiles `contract/evaluations/<name>/evaluation.json` (`--profile`; every recorded untracked digest verified, every prepared directory required, frozen profiles refused) and the explicit manifest `quirin-measure-inputs/1` (`--inputs`; [`inputs/live.json`](../../../crates/quirin-audit-plugin-measure/inputs/live.json) holds the same three routes) |
| `sides` | `--side NAME=SPEC`: `rev:` snapshots, checkouts, binaries, `baseline:<sha>`, `run:<dir>`; builds and route runs (the copied driver's environment and layout, inputs before and after, exit 101 on change), each route under a `gate_slot.py audit` slot |
| `mirror` | The re-spelled, digest-manifested private mirror (`--mirror auto\|always\|never`) |
| `rows` | A route's rows read through its component sidecar, validated, every unit row derived by `Contribution::derive` and checked equal to the report's row; `--legacy-rows` for pre-composition reports |
| `compare` | Row-by-row comparison, totals from each report, the `⊕` union of both sides with conflicts named, byte comparison of artifacts with the checked clock-key substitution, interactions |
| `declared` | The `measure-prediction` block of `PREDICTION.md` and the declared-versus-measured check |
| `baseline` | `measure publish` and identity verification before reuse |

**The record.** `native-comparison.json`, schema `quirin-native-comparison/1`:
per side the run metadata (spec, revision, build commit, binary digest,
contract-bundle digests, per-route inputs, its `run.json` with digest); per
comparison and route the totals counted from each side's own report, each
side's figures as `scripts/evaluation_profiles.py summarize` computes them
and its `compare` verdict against the profile's baseline, every
moved, content-changed and unmatched function row keyed by its unit's
component key, environment-only and identical counts, unit-row verdict moves,
the composition union, the artifact byte comparison; the named interactions
(rows moved in more than one comparison, never summed); the declared result
naming every unpredicted and missing row; and a reserved per-rule
`contribution` slot. An unmatched row stays in its side's denominator and
marks the route incomplete; a composition conflict does too.

**Fail-closed.** A side that fails to build, a route that exits nonzero or
changes an input, a sidecar that disagrees with its report, a report without a
sidecar (unless `--legacy-rows`), a baseline that fails verification, or a
declared-versus-measured mismatch fails the run: the host records it `Unknown`
with its receipt, and the record stays on disk naming the cause. The tool
claims no verdict.

**`--rules-off IDS`** is parsed and wired (an extra `<tip>-off` side from the
same binary with `QCORE_GATEWAY_RULES_OFF`, compared with `<tip>` into the
contribution slot). Until RULE-SWITCHES lands it is refused before any route
runs: "not yet supported … does not advertise the run-time rule switch". A
binary that does name the variable is checked again after its run: its
components must record exactly the requested off-list, so a switch the
analyzer ignored is never reported as applied (tested with stub analyzers).

**Snapshot hazards** (MEASURE-SNAPSHOT-HAZARDS): `rev:` sides clone every
gitlink at its pinned commit; each route runs in a private working directory
(`sides/<name>/work-<route>`); the input identity excludes VCS metadata and
`target/`, `private/`, `measure/`; `QCORE_AUDIT_BUILD_COMMIT` is the measured
revision (or `--build-commit`); snapshots and the mirror are bound in a
private mount namespace at one fixed path (`/tmp/quirin-measure/{src,mirror}`),
so sides differ only in source. A checkout counts as a revision only when it
is its repository's top level and clean.

**Clock key** (LIFETIME-SOURCE-IDENTITY-TIME-MACROS, PREPASS-CLOCK-DEPENDENT-KEY):
every route of every side runs with one fixed `SOURCE_DATE_EPOCH` (default
`1767225600`, `--source-date-epoch`, recorded and part of a baseline's
inputs), as the coordinator relayed from CFG-RELEASE; live-4 below shows the
contracts sidecar then byte-identical. For sides measured without it,
`compare::time_key` ports AUDIT-COMPOSITION's check: a differing
`*.contracts.json` is accepted only if exactly one lifetime-source summary key
differs, neither key occurs in the other file, and swapping it turns one file
into the other as JSON and as a line multiset. Outside the tool the analyzer
still embeds the clock.

**Baselines.** `measure publish --from <out>/sides/<name>` with the output
`private/baselines/<sha>` writes `<profile>/` (route reports, `run.json`,
`baseline.json` with every file's digest, the binary, inputs and bundle
digests). The integrator's landing gate, after measuring the landed revision:

```sh
target/release/quirin-audit --plugin-path target/release --output private/measure/landing-<sha> \
  --timeout-ms 36000000 measure --inputs crates/quirin-audit-plugin-measure/inputs/live.json \
  --inputs-base /media/Data/GitHub/quirin-lang --side prev=baseline:<previous-sha> --side tip=rev:<sha>
target/release/quirin-audit --plugin-path target/release --output private/baselines/<sha> \
  measure publish --from private/measure/landing-<sha>/sides/tip
```

(For the first landing there is no previous baseline; use `--side prev=rev:<previous-sha>`.)
`baseline:<sha>` is reused only after every published file, the route
definition, the inputs as spelled now and the contract-bundle digests
recomputed from `<sha>`'s git tree match.

**Superseded scripts.** [`superseded-scripts.json`](superseded-scripts.json)
lists twenty copied scripts with their digests and the function that replaces
each. They are not edited or retired: their reports cite them and the drivers
record their own digests; the crate's test checks every digest.

## Parity

**Result: the tool reproduces rule 33's recorded verdict moves exactly.**
Rule 33's DETACH comparison (card DETACH-RESULT-WRAPPER-COMPOSITION, pinned
corpus, BASE `e282aaccc`, TIP `9ecc941d5`) was replayed end to end through the
host: both sides built by the tool from their revisions (`rev:` sides, release
builds of 3.5 minutes each at `/tmp/quirin-measure/src`), the pinned inputs
re-spelled into the mirror (both revisions predate AUDIT-CANONICAL-PATHS), each
corpus route under an `audit` slot with `--jobs 3`, the pre-composition reports
read with `--legacy-rows`, and the declared moves of
[parity/PREDICTION.md](parity/PREDICTION.md) (rule 33's recorded outcome,
transcribed before the replay). Script: [scripts/parity.sh](scripts/parity.sh);
records: [parity/](parity/) (`native-comparison.json`, both `run.json`, the
mirror manifest and the host report); run directory
`/media/Data/GitHub/quirin-lang/private/measure-tool/parity-1/`.

| | Recorded (rule 33) | Replayed by the tool |
|---|---|---|
| BASE Safe / Unknown / prov. Unsafe | 975 / 3,850 / 9 | 975 / 3,850 / 9 |
| TIP | 977 / 3,848 / 9 | 977 / 3,848 / 9 |
| Rows changing status | `cJSON_DetachItemFromArray`, `cJSON_DeleteItemFromArray`, Unknown → Safe | the same two, Unknown → Safe; 0 unmatched |
| Leaf trade | each loses its single refusal leaf (nested `get_array_item`, nested `cJSON_DetachItemFromArray`) | the same leaves (`cJSON.c:2286`, `cJSON.c:2291`), nothing added |
| `cJSON_DeleteItemFromArray` dependencies | declarations `cJSON_Delete`, `cJSON_DetachItemViaPointer` (the `detach_*` hypotheses), summary `get_array_item`, null premise `null-deref:get_array_item:2770:current_child` | the same |
| Document `diagnostic_counts` | `unsupported` 27,710 → 27,708, others equal | the same |
| cJSON unit fields | `cross_unit_link_resolution`, `diagnostic_counts`, `reason` | `diagnostic_counts`, `reason` |
| Declared-versus-measured | (prose) | `match`: no unpredicted, no missing row, deltas Safe +2, Unknown −2, Unsafe 0 |

The one difference is expected and explained: the recorded run measured its
two snapshots at two directories, so `cross_unit_link_resolution` and the
environment digests of all 185 units differed by a path substitution its field
review had to prove. The tool binds both sides at one fixed path, so no unit
field moves beyond the cJSON unit's two, and the other 4,832 rows differ only
in environment digests (the two sides' build commits enter every evidence
version), which the tool classifies as `environment_only`. Both routes exited
0 and their input identity was unchanged before and after (25,496 and 25,506
files; the libevent clone's `.git` is excluded, so the audit's own index
refresh no longer fails the route as it did for AUDIT-COMPOSITION's run).

**Live routes.** Two further runs exercise the configured routes, the sidecars, the
fixed `SOURCE_DATE_EPOCH`, publication and reuse on real data
([scripts/live_identity.sh](scripts/live_identity.sh); records in [live/](live/)):

- **live-4** (`BASE_REV=e37e6703d`, TIP `7d4aa36bc`, `--build-commit
  measure-tool-identity`, system and slice): both sides built by the tool at
  the fixed path produced the same binary (`1c86e628…`), and every report
  artifact of both routes is **byte-identical**, the contracts sidecar
  included: 404 and 448 function rows identical, 0 moved, the union of the two
  sides' 16 and 15 components deduplicating to 16 and 15, no conflict. With one
  fixed `SOURCE_DATE_EPOCH` the clock-dependent lifetime-source key no longer
  differs at all. BASE was then published (`measure publish`, two immutable
  profile directories) and reused as `--side base=baseline:e37e6703d` against
  TIP's recorded run: identity verification passed and the comparison is again
  byte-identical (`live/reuse-native-comparison.json`).
- **live-2** (BASE `373e41ee8`, TIP `980bd2470`, each side's own build commit):
  0 moved rows, every row `environment_only` (the build commits enter every
  evidence version), totals system 101 / 0 / 343 / 4 and slice 57 / 4 / 342 / 1.
  Re-read through the committed live profiles (`--profile fluent-bit-system
  --profile fluent-bit-tail-pause`, [scripts/profile_figures.sh](scripts/profile_figures.sh)),
  both sides' `evaluation_profiles.py summarize` figures reproduce the
  published profile baselines: `compare` exits 0 for both profiles on both
  sides (`live/profile-figures-native-comparison.json`). Two routes of one side
  ran concurrently on one snapshot without tripping each other's input check;
  the retained products landed in each route's `work-<route>/`.

Retained, not evidence: live-1 (every `current` route exited 1 because
`gate_slot.py` inserted `--memory-budget-mib=7168`, a spelling the CLI
refuses; fixed in the tool, follow-up GATE-SLOT-CURRENT-BUDGET-SPELLING) and
live-3 (stopped by its task handle when the epoch option superseded it).

## Tests

`cargo test -p quirin-audit-plugin-measure`: 19 unit tests, and 15
integration tests in `tests/measure.rs` (SDK harness), `tests/host.rs` (the
real host: the record is a verified artifact, no claim and no verdict; a
failed measurement is an `Unknown` run with its invocation receipt) and
`tests/units.rs` (the mirror reproduces `mirror_inputs.py`'s re-spelling on a
moved fixture checkout; the profile adapter reads the committed
`contract/evaluations` records into the same databases and digests as
`live.json` and refuses a frozen profile; bundle digests of a `git archive` snapshot equal those read from the
git tree; the 14 shared vectors of `proof/QCore/AuditComposition.lean` hold
through the tool's union, split between two sides in both orders; the
superseded registry's digests).

## Gate

The scoped card gate ran on `934d6bc4e` (the code of this report; later
commits add report text only), after merging origin/main `8a015cd83`:
`python3 scripts/gate_scope.py --base 8a015cd83 --out private/gate-4` under
slots ([gate.json](gate.json); logs in the card worktree's `private/gate-4/`,
console `/media/Data/GitHub/quirin-lang/private/measure-tool/gate-4.console.log`;
Clang recorded in [clang-version.txt](clang-version.txt)). It selected full
mode: adding a workspace member changes `Cargo.toml` and `Cargo.lock`.

| Step | Exit | Count |
|---|---|---|
| submodules | 0 | 1 at its pin |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| Rust suite (`run_test_binaries.py`, rust-suite slot; 7,120 s including the slot wait) | 0 | 560 units, 574 summaries; 8,722 passed, 0 failed, 15 ignored |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| `python3 -m unittest discover -s scripts/tests` | 0 | 492 tests |
| Lean, provider gate | not selected | no `proof/` or `providers/` change |

Earlier attempts, retained and not evidence: `private/gate` (gate-1, base
`373e41ee8`, stopped by its task handle to merge origin/main), gate-2 (refused:
its output directory existed), and `private/gate-3` on `89466a641` (base
`e37e6703d`): every step 0 (Rust 560 units, 8,722 passed, 0 failed, 15 ignored)
except the Python suite, exit 1 on one test, because the committed
`integration-amendments.json` was still a placeholder.

## Boundaries

- The comparison keys function rows by their unit row's component; the
  property rows (null, outer projection) are compared only as artifacts, by
  bytes (COMPOSITION-PROPERTY-AND-CHECKPOINT-ROWS).
- Environment-only differences are those of the copied `native_compare.py`
  list (`assumptions`, `build_identity`, `environment_sha256`,
  `resource_contracts_sha256`, `id` and the evidence version's build commit);
  a difference outside it is a content change and is listed.
- A `binary` or checkout side runs at its own path, so its environment digests
  differ from a fixed-path side's; only `rev:` and `baseline:` sides are
  byte-comparable.
- The contribution slot is reserved until RULE-SWITCHES and
  CONTRIBUTION-LEDGERS land (MEASURE-TOOL-RULES-OFF-LIVE); no landing has
  published a baseline yet (MEASURE-TOOL-LANDING-PUBLISH).
- The corpus process peaks above the 7 GiB slot budget at `--jobs 3`
  (CFG-RELEASE's CORPUS-PEAK-ABOVE-AUDIT-SLOT-BUDGET); the tool takes the
  slot's default budget and does not resolve that.
- The mount namespace needs unprivileged user namespaces (`unshare -rm`).

## Follow-ups

Registered in [integration-amendments.json](integration-amendments.json):
MEASURE-TOOL-RULES-OFF-LIVE, MEASURE-TOOL-LANDING-PUBLISH,
GATE-SLOT-CURRENT-BUDGET-SPELLING. Retired: MEASURE-SNAPSHOT-HAZARDS,
MEASURE-TOOL-COMPOSITION-CONSUMER, EVALUATION-PROFILES-MEASURE-READER.
Amended (the tool's side handled, the analyzer's root cause open):
LIFETIME-SOURCE-IDENTITY-TIME-MACROS, PREPASS-CLOCK-DEPENDENT-KEY.

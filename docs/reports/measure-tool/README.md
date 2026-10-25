# MEASURE-TOOL

Card MEASURE-TOOL builds seam F of the
[pluggable-cards plan](../../plans/pluggable-cards/README.md) (phase P0):
measurement as the `quirin-audit measure` subcommand plugin (user decision 8
of 2026-10-25), behind the same out-of-process firewall and receipts as every
other subcommand (contract specification C11.2). It replaces the copied
`measure.py`, `compare.py`, `mirror_inputs.py` and `gate_job.sh` scripts. It
changes no analyzer, contract, route or theorem, so it needs no identity run
and adds no Lean (no new semantics). Base: local main `373e41ee8` (AUDIT-COMPOSITION
merged, `qcore-schemas` 21.0.0).

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
| `inputs` | Routes: the explicit manifest `quirin-measure-inputs/1` (`--inputs`; [`inputs/live.json`](../../../crates/quirin-audit-plugin-measure/inputs/live.json) holds the corpus, system and slice routes) and the one adapter to EVALUATION-PROFILES' draft `evaluation.json` (`--profile`; frozen profiles refused) |
| `sides` | `--side NAME=SPEC`: `rev:` snapshots, checkouts, binaries, `baseline:<sha>`, `run:<dir>`; builds and route runs (the copied driver's environment and layout, inputs before and after, exit 101 on change), each route under a `gate_slot.py audit` slot |
| `mirror` | The re-spelled, digest-manifested private mirror (`--mirror auto\|always\|never`) |
| `rows` | A route's rows read through its component sidecar, validated, every unit row derived by `Contribution::derive` and checked equal to the report's row; `--legacy-rows` for pre-composition reports |
| `compare` | Row-by-row comparison, totals from each report, the `⊕` union of both sides with conflicts named, byte comparison of artifacts with the checked clock-key substitution, interactions |
| `declared` | The `measure-prediction` block of `PREDICTION.md` and the declared-versus-measured check |
| `baseline` | `measure publish` and identity verification before reuse |

**The record.** `native-comparison.json`, schema `quirin-native-comparison/1`:
per side the run metadata (spec, revision, build commit, binary digest,
contract-bundle digests, per-route inputs, its `run.json` with digest); per
comparison and route the totals counted from each side's own report, every
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
`compare::time_key` ports AUDIT-COMPOSITION's check: a differing
`*.contracts.json` is accepted only if exactly one lifetime-source summary key
differs, neither key occurs in the other file, and swapping it turns one file
into the other as JSON and as a line multiset. The analyzer still embeds the
clock; the tool names the substitution.

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

PARITY-PLACEHOLDER

## Tests

`cargo test -p quirin-audit-plugin-measure`: 19 unit tests, and 15
integration tests in `tests/measure.rs` (SDK harness), `tests/host.rs` (the
real host: the record is a verified artifact, no claim and no verdict; a
failed measurement is an `Unknown` run with its invocation receipt) and
`tests/units.rs` (the mirror reproduces `mirror_inputs.py`'s re-spelling on a
moved fixture checkout; the profile adapter reads EVALUATION-PROFILES' draft
records into the same databases and digests as `live.json` and refuses a frozen
profile; bundle digests of a `git archive` snapshot equal those read from the
git tree; the 14 shared vectors of `proof/QCore/AuditComposition.lean` hold
through the tool's union, split between two sides in both orders; the
superseded registry's digests).

## Gate

GATE-PLACEHOLDER

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
- The profile adapter reads a draft format (follow-up
  MEASURE-TOOL-PROFILE-ADAPTER); the contribution slot is reserved until
  RULE-SWITCHES and CONTRIBUTION-LEDGERS land.
- The mount namespace needs unprivileged user namespaces (`unshare -rm`).

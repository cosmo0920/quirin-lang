# Audit subcommand plugins

`quirin-audit` runs external subcommands the way `cargo` does: for a name that
is not built in, `quirin-audit NAME ARGS…` runs an executable named
`quirin-audit-NAME`. The SDK for writing one is the crate
[`quirin-audit-plugin`](../../crates/quirin-audit-plugin/src/lib.rs); the
first plugin is [`quirin-audit-plugin-libevent`](../../crates/quirin-audit-plugin-libevent/src/lib.rs)
(`quirin-audit libevent prepare`). The normative text is contract
specification [C11.2](../QCORE_CONTRACT_SPEC_r0_3.md#c112-audit-subcommand-plugins);
the host is `crates/qcore-audit/src/subcommand_plugins.rs`; the evidence is
[W45-AUDIT-SUBCOMMAND-PLUGINS](../reports/w45-audit-subcommand-plugins/README.md).

## Running one

```sh
quirin-audit [--plugin-path DIR]... --output DIR [HOST-OPTIONS] NAME [ARGUMENTS...]
quirin-audit [--plugin-path DIR]... --list-plugins
```

Host options come before `NAME`; everything after it goes to the plugin
verbatim.

| Option | Meaning |
|---|---|
| `--plugin-path DIR` | A directory to search, in order; repeatable |
| `--output DIR` | **Required.** The plugin's working directory and the only place its artifacts may lie; created if missing |
| `--report FILE` | The host's report (default `<output>/quirin-audit-NAME.report.json`); `<report>.invocation.json` and `<report>.stderr.log` go beside it |
| `--audit-report FILE` | An existing audit report whose unit statuses are the host's verdicts (below) |
| `--qo FILE` | A `.qo` checkpoint, verified whole and passed by content digest; all must come from one run; repeatable |
| `--load-path FILE`, `--provider NAME` | A contract load path, validated, and crates on it; passed to the plugin |
| `--timeout-ms N` | Wall-time quota (default 30 minutes), counting analysis time: declared gate-slot waits are excluded (below) |
| `--slot-wait-limit-ms N` | The most declared slot-wait time the quota excludes (default: the overall wall bound, so a long queue is not capped at the quota; `0` counts every wait) |
| `--wall-limit-ms N` | The overall wall bound on the whole run, analysis and declared waits together (default seven days, never below `--timeout-ms`; a smaller value is refused). A run that reaches it is stopped with `plugin-wall-limit` |
| `--stdout-limit-bytes N`, `--stderr-limit-bytes N` | Output quotas (default 16 MiB each) |
| `--plugin-env NAME` | Pass one more environment variable by name; repeatable |

**Search.** Only the declared path is searched: every `--plugin-path`, then
the directories in `QUIRIN_AUDIT_PLUGIN_PATH`. `PATH` is never searched, so a
program that happens to be called `quirin-audit-something` is never run by
accident. A built-in name (`current`, `prepare`, `plugin`, …) always runs the
built-in; an external executable of that name is never run, and
`--list-plugins` marks it as shadowed. An unknown name is a usage error that
lists the search path. A configuration path — the other form of the first
argument, `quirin-audit CONFIG.json` — contains a `.` or a `/`.

**Environment.** The plugin's environment is cleared except `PATH`, `HOME`,
`TMPDIR`, `LANG`, `LC_ALL`, `LC_CTYPE` and `TZ` (when set) and each
`--plugin-env NAME`. `PATH` is passed so a plugin can run `git`, `cmake` or
`clang`; it is not used to find the plugin. A relative `PATH` entry resolves
against the plugin's working directory, the output.

**Quotas and confinement.** The plugin runs in its own process group. A
wall-time breach, a response larger than the standard-output quota or
standard error (captured to `<report>.stderr.log`) larger than its quota kills
the whole group; so does the end of the run, so nothing the plugin started
outlives it. Every artifact must resolve inside the output and have the
SHA-256 it declares. This is containment of a process group and of what the
host accepts, **not an operating-system sandbox**: nothing stops a plugin
writing elsewhere, and a descendant that leaves the group escapes the kill
(standing constraint 26).

**Slot waits outside the quota** (card MEASURE-QUOTA-AND-INCREMENTAL,
requirement R7 of [gate-time reduction](../plans/gate-time-reduction/README.md)).
While the file `.quirin-audit-slot-wait` (`quirin_audit_plugin::SLOT_WAIT_MARKER`)
exists in the output, the plugin declares that all its work waits for a gate
slot (`scripts/gate_slot.py`); the host polls for it and does not count that
time against `--timeout-ms`, up to `--slot-wait-limit-ms`, which defaults to
the overall wall bound and not to the quota (card GATE-SLOT-FIFO-ORDER: a
queue of 15 h killed a measurement whose quota was 10 h). The run record
(`run` in the report) gives `wall_time_ms`, `slot_wait_ms`,
`analysis_time_ms` and the limits (`wall_time_quota_ms`, `slot_wait_limit_ms`,
`wall_limit_ms`); the invocation record and the request's `quotas` carry them
too, so the plugin sees what it runs under. The declaration is the plugin's
own, so the overall wall bound (`--wall-limit-ms`, default seven days) is what
bounds it: the run is stopped at that elapsed time whatever it declares, with
`plugin-wall-limit`. The host removes a marker left in the output before it
spawns the plugin.

**Exit status.** 0: the plugin completed and the report is written. 1: the run
failed (the report is written, and its verdict is `Unknown`). 2: a usage error
or a refused input; nothing ran.

## Verdicts

A plugin may *claim* verdicts. A claim is the full tuple of hard invariant 2 —
artifact, property, semantic profile, scope, bounds, assumptions and evidence
version — with a proposed `Safe`, `Unsafe` or `Unknown`, a reason and any
certificate references. The host classifies each claim with the checker-plugin
firewall (contract spec C11.1):

| Claim | Verdict |
|---|---|
| `Safe` with a certificate the audit's own checker accepts | `Safe` — unreachable today: no checker accepts a subcommand plugin's certificate |
| `Safe` without one, or with references no checker reads (`QP008`, `QP016`) | `Pending` (`plugin-claim-uncertified`) |
| `Unsafe` without a witness the audit validates (`QP010`) | `Pending` (`plugin-claim-uncertified`) |
| `Unknown` | `Unknown` |

A spawn failure, a timeout, a quota breach, a nonzero exit, a malformed
response, a protocol-version mismatch (`protocol-version-mismatch`), a failure
the plugin reports, or an artifact that does not verify makes the **run**
`Unknown`, and a failed run's claims are recorded but joined nowhere.

**The join.** Each row of the report is one artifact: the join
(`Safe < Pending < Unknown < Unsafe`) of the host's verdict for it (the
`--audit-report` unit row whose `file` is the artifact), the run's verdict and
the classified claims about it. The report's `verdict` is the join of every row
and the run, or `null` when a completed run has no row (a plugin that claims
nothing publishes no verdict). The join never lowers a host verdict: an
`Unsafe` row stays `Unsafe`, and a failed plugin makes a `Safe` row `Unknown`.
`QCore.SubcommandPlugins` proves these properties of the model.

## The protocol

Version 1 of `quirin-audit-plugin`: one JSON request on the plugin's standard
input, one JSON response on its standard output, nothing else on standard
output. Both carry `"protocol": "quirin-audit-plugin"` and
`"protocol_version": 1`; the host checks them before the shape, so another
version is a named mismatch. Unknown fields are refused.

The **request** carries the subcommand and its arguments; the host's working
directory (`invocation_directory`, against which a relative path argument is
meant); `output`; `report`; the load path and providers; the audit identity
(revision, contract-set digest and refutation-table digest, from the `.qo`
checkpoints); the audit report and checkpoints, each with its digest; and the
quotas.

The **response** carries `outcome` (`"completed"` or `{"failed": REASON}`),
`artifacts` (path, SHA-256, kind), `diagnostics` (severity, code, message) and
`claims`.

## Writing one in Rust

```toml
[dependencies]
quirin-audit-plugin = { path = "…/crates/quirin-audit-plugin" }

[[bin]]
name = "quirin-audit-mytool"
path = "src/main.rs"
```

```rust
use quirin_audit_plugin::{Request, Response, run};

fn answer(request: &Request) -> Result<Response, String> {
    let output = request.output_directory();
    std::fs::write(output.join("result.txt"), "done\n").map_err(|e| e.to_string())?;
    let mut response = Response::completed();
    response.push_artifact(&output, std::path::Path::new("result.txt"), "result")?;
    Ok(response)
}

fn main() -> std::process::ExitCode {
    run(answer)
}
```

`run` reads the request, refuses another protocol version, and writes the
response; an `Err` becomes a failed response and exit 1. Write diagnostics for
a person to standard error. `quirin_audit_plugin::testing::invoke` runs a built
plugin binary on a request as the host would (cleared environment, working
directory the output) and `Invocation::verified` checks its response and
artifacts; `quirin_audit_plugin::json_text` (`JsonText`) writes ordered,
indented JSON text with ASCII escapes, for records compared byte for byte. The crate's own example,
`quirin-audit-example`, is a complete plugin:

```sh
cargo build -p qcore-audit -p quirin-audit-plugin
target/debug/quirin-audit --plugin-path target/debug --output /tmp/out example --claim safe
```

Its report records the artifact and the claim, classified `Pending`.

## The libevent plugin

```sh
cargo build -p qcore-audit -p quirin-audit-plugin-libevent
target/debug/quirin-audit --plugin-path target/debug --output ../out/libevent \
  libevent prepare --clang clang --clangxx clang++
```

It prepares the pinned `libevent-core` configured-audit inputs from the
repository's `third_party/libevent` (`--repository DIR`, else the load path's
repository, else the working directory): CMake with Clang, the
`event_core_static` build, and `libevent-clang-build/` holding CMake's
unmodified `compile_commands.json`, `libevent-core-audit.json.config` and
`libevent-build-provenance.json`, byte for byte as the retired script wrote
them. It refuses an output inside `build/cycle19` or `third_party`, and
`--write-profile` writes the regenerated audited-file table to
`libevent-clang-build/profile.json` for a person to review and copy, rather
than editing the committed profile. It claims no verdict.

## The measure plugin

`quirin-audit measure` (crate
[`quirin-audit-plugin-measure`](../../crates/quirin-audit-plugin-measure/src/lib.rs),
card [MEASURE-TOOL](../reports/measure-tool/README.md)) measures audit routes
on two or more sides and compares them row by row. It replaces the
`measure.py`, `compare.py`, `mirror_inputs.py` and `gate_job.sh` copies in the
report directories ([registry](../reports/measure-tool/superseded-scripts.json)).
It claims no verdict; a failed side, a refused baseline, a route that cannot be
compared or a declared-versus-measured mismatch fails the run (exit 1,
`Unknown`). Measurements take hours, so raise the host's wall-time quota, and
do not wrap the command in `gate_slot.py`: each route takes its own `audit`
slot. Each route's slot script keeps a live wait record
(`sides/<side>/<route>.slot-wait.json`, `gate_slot.py --wait-record`); while
every route in flight waits for its slot, the measurement declares the
host's slot-wait marker, so the quota counts analysis time only (R7). The
record's `timing` slot gives the measurement's `wall_time_ms`, the declared
`slot_wait_ms` and the `analysis_time_ms`, the `limits` it ran under
(`wall_time_quota_ms`, `slot_wait_limit_ms`, `overall_wall_limit_ms`), and per side and route (with its
profile) the route's own wait and run time, which each route's `run.json`
entry also carries. One route waiting while another runs is not a wait: the
measurement is making progress. Slot variables (`QUIRIN_GATE_DIR`,
`QUIRIN_GATE_LEGACY_LOCK`, `QUIRIN_GATE_MEMINFO`) reach the routes only when
the host passes them with `--plugin-env`.

```sh
cargo build --release -p qcore-audit -p quirin-audit-plugin-measure
target/release/quirin-audit --plugin-path target/release --output ../measure/card-x \
  --timeout-ms 36000000 measure \
  --inputs crates/quirin-audit-plugin-measure/inputs/live.json \
  --inputs-base /media/Data/GitHub/quirin-lang \
  --side base=baseline:<sha> --side tip=rev:HEAD \
  --declared docs/reports/<card>/PREDICTION.md
```

| Option | Meaning |
|---|---|
| `--side NAME=SPEC` | Two or more. `rev:<rev>`: a `git archive` snapshot with every submodule cloned at its pinned commit, built (`cargo build --release --locked -p qcore-audit`, `QCORE_AUDIT_BUILD_COMMIT` = the revision) and run in a private mount namespace at one fixed path; a directory: that checkout, built in place; a file: a `quirin-audit` binary at `<checkout>/target/release/`; `baseline:<sha>`: a published baseline; `run:<dir>`: a side directory an earlier measurement wrote |
| `--compare A B` | The comparisons, repeatable (default: the first two sides) |
| `--inputs FILE`, `--routes R,...` | The explicit inputs (`quirin-measure-inputs/1`): per route a compile database (or the pinned corpus inputs root) and a unit list; [`inputs/live.json`](../../crates/quirin-audit-plugin-measure/inputs/live.json) holds the three live routes |
| `--profile NAME` | A live evaluation profile, `contract/evaluations/<NAME>/evaluation.json` (`eight-home`, `fluent-bit-system`, `fluent-bit-tail-pause`; schema in `scripts/evaluation_profiles.py`), repeatable. Every untracked input digest it records is verified and every prepared directory must exist before anything runs; a frozen profile is refused. The record then carries each side's figures as `evaluation_profiles.py summarize` computes them and its `compare` verdict against the profile's baseline |
| `--declared FILE` | Check the measured moves of the first comparison against the file's `measure-prediction` block (below) |
| `--rules-off IDS` | Also measure the first comparison's after side `<tip>` with `QCORE_GATEWAY_RULES_OFF=IDS` as side `<tip>-off` and compare it with `<tip>` (the per-rule contribution, below). Ids are distinct. Refused, "not yet supported", unless the binary advertises the switch (every `quirin-audit` built since card GATEWAY-RULE-API does); after the run every switched side's components must record exactly its off-list. The ids are those of `crates/gateway-rules/registry.toml`; the analyzer refuses to run under an unknown id, a hold or a malformed list. Set by hand, the variable is honoured only where it is recorded (`current` and `contracts` with a report, `pinned` with its diagnostic report, a configuration run, `prepare --run`); every other `quirin-audit` command line, `quirin-dump-checkpoint`, `quirin` and every example linking the gateway refuses a non-empty list, and so does the host before any subcommand plugin, whose environment it clears ([RULE-SWITCH-ENTRY-POINTS](../reports/rule-switch-entry-points/README.md)) |
| `--per-rule` | With two or more `--rules-off` ids: also measure one side `<tip>-off-<id>` per id with only that rule off, so each rule's rows are separated from the rows the rules move together (named interactions) |
| `--repository DIR`, `--inputs-base DIR` | The repository (revisions, link map, `scripts/gate_slot.py`) and the root of the untracked prepared inputs (default: the repository) |
| `--mirror auto\|always\|never` | Re-spell the compile databases into a digest-manifested mirror when they spell a non-canonical path (default `auto`); needed for analyzers built before AUDIT-CANONICAL-PATHS |
| `--source-date-epoch SECONDS\|none` | `SOURCE_DATE_EPOCH` for every route of every side (default `1767225600`), so Clang expands Monkey's `__DATE__`/`__TIME__` identically and the lifetime-source pre-pass key does not move with the clock; recorded and part of a baseline's inputs |
| `--legacy-rows` | Read reports without a component sidecar (written before AUDIT-COMPOSITION) by their unit rows |
| `--link-resolution NAME` | Repeatable: side `NAME` (one that runs) runs every route under the route's rule 40 link-resolution artifact in place of the stated map (below) |
| `--reuse-contract-only`, `--verify-reuse` | Measure the first comparison's tip with the base side's analyzer, resuming the base side's pinned group checkpoints, when the diff is classified contract-only (below); `--verify-reuse` also measures `<tip>-full` without checkpoints and fails unless the two are byte-identical |
| `--jobs N`, `--build-jobs N`, `--no-slots`, `--mount-root DIR`, `--build-commit TEXT`, `--baselines DIR` | Audit workers (3, the 7 GiB slot's cap), build jobs (6), run without slots, the fixed path (`/tmp/quirin-measure`), an explicit build commit for identity runs, the baseline root (`<repository>/private/baselines`) |

**What it writes.** Under the output: `sides/<name>/` (the snapshot `src/`,
`build.log`, `run.json` and one directory per route in the copied drivers'
layout, with `invocation.json`, the inputs digests before and after, and exit
101 when an input or the binary changed), `mirror/manifest.json`, and
`native-comparison.json`. Per comparison and route that record holds the totals
counted from each side's own report, every moved, content-changed,
environment-only and unmatched function row keyed by its unit's composition
component (`library::file::unit#n::property::profile::function#k`), the unit
rows' derived verdicts, the union of the two sides' components with any
`composition-conflict` named, and the byte comparison of the report artifacts,
where the contract sidecar's one clock-dependent lifetime-source key (Monkey's
`__DATE__`/`__TIME__`) is accepted only as a checked one-key substitution. Rows
moved in more than one comparison are listed as named interactions; nothing is
summed. An unmatched row stays in its side's denominator and marks the route
incomplete.

**Working directory.** Every route of a snapshot side runs from one fixed
directory, `<mount root>/work`, with the route's own private directory
(`sides/<side>/work-<route>`) bound there, so every side of every measurement
spells the same working directory. The pinned route digests the process
working directory into its rows' `hypothesis_context.build_identity` and its
checkpoint identity (`PINNED-ROWS-BIND-WORKING-DIRECTORY`); `invocation.json`
records both spellings (`working_directory`, `working_directory_as_run`).

**Link-resolution sides** (card
[SELECTION-ROUTES-SPLIT-CALLER-AND-HOME](../reports/selection-routes-split-caller-and-home/README.md),
`--link-resolution NAME`, [`link.rs`](../../crates/quirin-audit-plugin-measure/src/link.rs)).
A configured route of the inputs manifest may name its rule 40 artifact,
`"link_resolution": {"path": ..., "sha256": ...}` (relative to the inputs base,
digest required; a pinned route refuses one; a route without one keeps its
canonical JSON and digest). A side named by `--link-resolution` runs every route
with `QCORE_CROSS_UNIT_LINK_RESOLUTION` set to that artifact and without
`QCORE_CROSS_UNIT_LINK_MAP`; every other side runs the stated map, so two
sides of one binary compare the stated map with the derived resolution. It is
fail-closed and never falls back to the map: before anything runs, every route
must declare an artifact whose file has the declared digest and schema and binds
the compile database the route reads as spelled (rule 40's condition 1; a
re-spelled mirror refuses); a side that does not run (`baseline:`, `run:`) is
refused; after the build, the side's binary must name the variable; after each
route, the analyzer must have printed that it derived its resolution from that
artifact's digest and its contract sidecar must record no whole-run refusal, or the route
fails with exit 102 (`link_resolution_error` in its `invocation.json`). The
side's `run.json` and the record's `sides` slot carry `link_resolution: true`,
each route's `invocation.json` the artifact (`link_resolution`) and what the run
derived (`link_resolution_run`), and `measure publish` refuses such a side: a
profile's baseline is its route under the stated map.

**Contract-only reuse** (card
[MEASURE-CONTRACT-ONLY-SIDE-WIRING](../reports/measure-contract-only-side-wiring/README.md),
`--reuse-contract-only`). When the first comparison is two `rev:` sides, a
pinned corpus route is measured, and neither `--rules-off` nor
`--build-commit` is given, the base side is built first; the tip is then
checked out (a shared clone, so a run can check its `HEAD`) and the base
side's binary classifies `BASE..TIP` (`quirin-audit pinned --classify-only`,
[audit checkpoints](audit-checkpoints.md#the-pinned-corpus-route)). On
acceptance the base binary is copied into the tip tree instead of building
it, so the tip side's binary digest and build commit are the base side's;
the base side's pinned route seals its groups into `checkpoints/<route>`
under the output, and the tip side's waits for it, then resumes from that
directory with `--accept-base-analyzer BASE --repository <tip as run>`. Any
refusal (a mixed or unclassifiable diff, a tree whose `run_pinned_audit.py`
does not forward the acceptance, an older binary without `--classify-only`)
builds the tip as a separate side, exactly as without the option. The
record's `contract_only_reuse` slot says whether the path was taken and why
not, the classification, and the checkpoint directories; each side keeps the
checkpoint run record it produced (`sides/<side>/<route>.checkpoint-run.json`,
summarised per stage in the route's `checkpoint_run`). The tip's pinned route
starts only after the base side's, so the reuse saves wall time only when
groups are reused: while every corpus group seals `whole`
(`GATEWAY-WHOLE-TABLE-READS`), a diff that changes any loaded contract text
recomputes every group, and only a diff the route does not read (identity
unchanged) reuses them. Not for landing baselines: the tip is measured by the
base build.

**Contribution ledgers** (card CONTRIBUTION-LEDGERS, plan seam A). With
`--rules-off`, the record's `contribution` slot holds a
`quirin-contribution-ledger/1` ledger per switched rule and profile, built
from the switched comparisons of one binary (`ledger.rs`): the rows moved, each
with the named premises its TIP row rests on (resolved through the route's
contract sidecar), the leaves traded on rows whose status does not move, and
the touched rows that stay Unknown with their remaining leaves. One rule
switched alone owns every row its switch moves, and its ledger says its
interactions were not measured. With `--per-rule`, a row is a rule's only when
that rule's switch alone moves it, no other single switch does, and the joint
switch moves it the same way; every other moved row is an interaction ledger
(`33+35`), never attributed to either rule. The group's base (every switched
rule off) plus all its ledgers' deltas must equal the TIP totals, or the run
fails; nothing is repaired. Rows of the per-rule comparisons are not listed as
comparison interactions. Commit the record (for example under
`docs/reports/<card>/`) and write the ledgers into the tree with
`python3 scripts/contribution_ledgers.py write --record <committed record>`,
after the TIP run is the profile's baseline; that script owns the schema, the
layout (`contract/contributions/<rule-id>/<profile>.json`,
`contract/contributions/interactions/<key>/<profile>.json`), the profile
consistency check and the generated [contributions page](contributions.md).

**The prediction block.** One fenced block in `PREDICTION.md`:

````text
```measure-prediction
route corpus
move cJSON_DetachItemFromArray Unknown -> Safe
move librdkafka::cJSON.c::cJSON_DeleteItemFromArray Unknown -> Safe
delta Safe +2
route system
none
```
````

A selector is `function`, `file::function` or `library::file::function`; a
status may be `*` or `absent`. Every measured move no line covers is named as
unpredicted and every declared move that did not happen as missing.

**Publishing a baseline** (the integrator's landing gate, after measuring the
landed revision `<sha>` with a `rev:<sha>` side named `tip`):

```sh
target/release/quirin-audit --plugin-path target/release \
  --output private/baselines/<sha> measure publish --from <measure-output>/sides/tip
```

It writes `private/baselines/<sha>/<profile>/` (the route's reports, the
side's `run.json` and `baseline.json` with the binary, inputs and
contract-bundle digests and the SHA-256 of every file), refuses a failed or
unstable route, a side without a revision and an existing profile directory.
`--side base=baseline:<sha>` reuses it only after verifying every file digest,
the route definition, the inputs as this measurement spells them, and the
contract-bundle digests recomputed from `<sha>`'s git tree.

The integrator runs `python3 scripts/land.py publish` after the full gate and
the records commit. It finds the nearest published first-parent ancestor,
compares that verified baseline with `rev:<landed-sha>` on all three live routes,
and publishes the tip only when every live profile's landed figures reproduce
the baseline committed in the landing (`evaluation_profiles.compare`) and the
profile history policy holds; otherwise it keeps the measurement and refuses
(card LANDING-PUBLISH-COMPLETION; `scripts/land.py` names each case). The first publication uses `--bootstrap` to measure the
previous revision as `rev:` because no baseline exists yet. Each attempt keeps
its own `private/measure/landing-<sha>-<stamp>/` output; an existing immutable
`private/baselines/<sha>/` refuses another publication. `--inputs-base DIR`
selects the prepared input root, and `--previous SHA` selects a particular
previous landing when needed. The command requires a clean checkout and the
release audit host and `quirin-audit-measure` binary.

# W47 round 9 integration

Integration branch `integrate/w47-r9`, tip `6cacfb1b` (formatting fix on top:
`2aa377e5`). This report is the combined gate, pin check, result-identity and
census re-derivation for that tip. It does not implement or fix anything; it
records what the gate measured.

## What was composed

Base = local main `2101449b` (the requested-branch integration of nine codex
branches; unpushed; `origin/main` is `fe603d49`). On top, four cards, each
gated alone at base `fe603d49`:

1. W47-PREMISES-CORO-PLUGIN (`docs/reports/w47-premises-coro-plugin/`)
2. W47-PREMISES-INSTANCE (`docs/reports/w47-premises-instance/`)
3. W47-FLB-LIB-STRICT-PAIRING (`docs/reports/w47-flb-lib-strict-pairing/`)
4. W47-PREMISES-HOLDER-REGION (`docs/reports/w47-premises-holder-region/`)

Integration edits (`git log fe603d49..6cacfb1b --merges`): the `CensusReading`
field `premises` from coro-plugin was renamed `coro_plugin_premises` (the
JSON key `typed.premises` is unchanged); holder-region's `copy_literal` is
set before instance's premised early return; the second StepB view gained
holder-region's fields; follow-ups 580; Lean `LoopOwner` audits 138.

## 1. Combined gate at 6cacfb1b

`docs/reports/w47-callee-closure-r2/scripts/gate.sh` ran the standing gate
plus the provider gate into `private/w47-int-l/gate-6cacfb1b/gate/`
(`steps.txt`, one `NAME EXIT` line per step):

| Step | Exit | Notes |
|---|---|---|
| submodule | 0 | |
| tests (`run_test_binaries.py`) | 0 | 358 units, 371 summaries; **6417 passed, 0 failed, 11 ignored**. The strict `^(ok\|FAILED) ` line sums (6417/0/11) match the runner's own totals line exactly. No load-sensitive timing failure: `checkpoint_cli` 66.69s, `cycle20_verifier_plugins` 1.35s, both well inside their timeouts; no rerun needed. |
| clean-dev | 0 | |
| fmt | **1 (FAIL)** | see §2 |
| clippy | 0 | |
| reevaluation-probe | 0 | |
| reevaluation-followup | 0 | |
| lean (`check_lean_audits.py`) | 0 | 224 Lean audits passed |
| python (`unittest discover scripts/tests`) | 0 | Ran 365 tests, OK |
| docs-index (`--check`) | **1 (FAIL)** | see §2 |
| provider-tests | 0 | 43 `^test result: ` lines; sums **129 passed, 0 failed, 1 ignored**, matching the individual line sums |
| provider-clean-dev | 0 | |
| provider-fmt | **1 (FAIL)** | see §2 |
| provider-clippy | 0 | |
| provider-contracts-check | 0 | |
| package-build | 0 | |
| package-verify-qcore.chunkio | 0 | |
| package-verify-qcore.cmetrics | 0 | |
| package-verify-qcore.ctraces | 0 | |
| package-verify-qcore.libevent | 0 | |
| package-verify-qcore.librdkafka | 0 | |
| provider-python | 0 | Ran 24 tests, OK |
| clang-version | 0 | Ubuntu clang version 21.1.8 (6ubuntu1), `/usr/lib/llvm-21/bin` |

Outside `gate.sh`, the same script built the release `quirin-audit`
(exit 0) and ran the three arbiter routes:

| Route | Exit | Result |
|---|---|---|
| corpus (`run_pinned_audit.py`, 66910c10) | 0 | `resource-diagnostics.json` sha256 `ae17152b05516be7…`, **byte-identical** to the archived baseline |
| system (`run_system.py`, 046accf6) | 0 | see §3 |
| slice (`run-current.sh`, 90978ac6, tail-pause selection) | 0 | see §3 |

Private logs: `private/w47-int-l/gate-6cacfb1b/` (`gate/` for the standing
and provider gate; `corpus/`, `system/`, `slice/` for the three arbiter
routes; `release-build.log`).

## 2. The three failing exits, and the fix

`fmt`, `provider-fmt` and `docs-index` were the only non-zero exits at
`6cacfb1b`. The integrator diagnosed and fixed both causes in commit
`2aa377e5` on `integrate/w47-r9` (on top of this gate's head), formatting
only:

- `crates/quirin-source-analysis/tests/loop_owner_coro_plugin.rs`: the
  `premises` -> `coro_plugin_premises` rename made one
  `panic!("no premise {name}: {:#?}", …)` call's line too long; rustfmt
  wraps it across five lines.
- `docs/reports/INDEX.md`: the hand-merged `w47-premises-coro-plugin`,
  `w47-premises-holder-region` and `w47-premises-instance` rows were missing
  the generator's blank-line row separators.

`git diff 6cacfb1b 2aa377e5 -- crates/quirin-source-analysis/tests/loop_owner_coro_plugin.rs docs/reports/INDEX.md`
confirms both hunks are formatting only (no code, test assertion or record
content changed). Re-run at `2aa377e5` in this worktree:

```
cargo fmt --all -- --check                                    exit 0
cargo fmt --manifest-path providers/Cargo.toml --all -- --check   exit 0
python3 scripts/build_docs_index.py --check                   exit 0
```

Because this is a formatting-only change, the test and arbiter results
measured at `6cacfb1b` stand; they were not re-run at `2aa377e5`.

## 3. Result identity (corpus, system, slice)

Compared with `docs/reports/w47-flb-lib-strict-pairing/artifacts/after/`
(system, slice — the newest archived baseline) and the premises cards'
archived corpus digest (`ae17152b…`, `docs/reports/w47-premises-instance/artifacts/arbiters/corpus.identity`),
using `docs/reports/w47-flb-lib-strict-pairing/scripts/identity.py` (the
brief's `baseline_identity.py` raises `KeyError` on these manifests, as the
cards already note):

| Route | Functions (Safe/Unsafe/Unknown[/Pending]) | Identity |
|---|---|---|
| corpus (185 units, 66910c10) | **526 / 10 / 4298** | sha256 `ae17152b05516be7…` on both sides: **byte-identical** |
| system (046accf6) | **31 / 2 / 201** | 0 status/leaf moves; differs only in 15 `resource_contract_sources`/`inventory_sha256` checkout-root lines and 8 `review_holds[].module` lines, which embed the same checkout-root path (`.../implementation-wave-47-681db5/...` vs the archive's own worktree) — the same class of environment identity as the hash lines |
| slice (90978ac6) | **24 / 1 / 244 / 1 Pending** | 0 status/leaf moves; 16 `resource_contract_sources`/`inventory_sha256` lines and 14 `review_holds[].module` lines, same environment-identity class |

`moved_functions: []` and `moved_units: []` on both system and slice — **0
mismatches** on every route, matching the expected composed figures exactly.

## 4. Census re-derivation on the composed tree

Fresh builds in this worktree: `loop_owner_census`, `typed_dispatch_facts`,
`predefined_macro_facts`, `region_macro_facts` (all `cargo build -p
quirin-source-analysis --example …`, exit 0).

**Condvar/lifecycle census** (`docs/reports/w47-flb-lib-strict-pairing/scripts/census.sh`,
fixed/previous/system/corpus, all 5 exits 0): byte-identical to
`docs/reports/w47-flb-lib-strict-pairing/artifacts/census/{fixed,previous,system,corpus}.json.gz`
except the worktree-root `module` path line. The fixed test's 3 sites
(`in_tail.c:2483`, `:2492`, `:2494`) are discharged (confirmed independently
in the holder-region typed census's `fixed-configured` reading, all three
`discharged`); the old test's 8 sites and C5 are not discharged, exactly as
archived (implied by the byte-identical match).

**Typed/predefined/region facts**, freshly derived against the 90978ac6
export and the previous test (all exits 0):
`docs/reports/w47-loop-owner-typed-dispatch/scripts/facts.py`,
`docs/reports/w47-loop-owner-predefined-macro-arms/scripts/predefined_facts.py`,
`docs/reports/w47-premises-holder-region/scripts/region_facts.py`. These
regenerate with **more** data than the older `w47-loop-owner-typed-dispatch`
archive (populated `literal_calls`/`named_objects`, ~40 extra
`plugins/custom_calyptia/calyptia.c` entries) — expected, since the composed
tree includes coro-plugin's own later enhancement to the same derivation;
not a discrepancy against that older archive, and not what §4 compares.

**Premised readings**, each card's own `census.py` against the composed
tree, compared against **that card's own archived AFTER** output
(verdict, first-reason classification, instance-flag derivation, function
and seed counts; not raw reason text):

- **W47-PREMISES-CORO-PLUGIN** (`fixed-configured`, `previous-configured`):
  0 verdict moves, 0 flag moves, edges/functions/seeds unchanged —
  **11/130/7** (fixed) and **8/136/7** (previous), matching the card exactly.
- **W47-PREMISES-INSTANCE** (`fixed-premised`, `fixed-configured-premised`,
  `previous-premised`, `previous-configured-premised`): 0 verdict moves, 0
  flag moves, functions and seeds unchanged on all four. The two
  `-configured-premised` readings' edge counts (33556, 33530) match the
  card's archive exactly. The two unconfigured `-premised` readings show
  edges **33493 -> 33520** and **33466 -> 33493** (+27 each) against the
  card's own archive.
- **W47-PREMISES-HOLDER-REGION** (`fixed`, `fixed-configured-control`,
  `fixed-configured-predefined`, `fixed-configured`,
  `previous-configured`): 0 verdict moves, 0 flag moves, functions and
  seeds unchanged on all five. `fixed-configured-predefined` and
  `fixed-configured` (33556) and `previous-configured` (33530) match the
  card's archive exactly. `fixed` and `fixed-configured-control` show edges
  **33493 -> 33520** and **33573 -> 33600** (+27 each) against the card's
  own archive.

The uniform **+27** edge delta on exactly the readings that do not already
carry coro-plugin's configured facts is coro-plugin's own documented,
predicted per-card addition (its README: "both controls (configured without
the predefined facts) \| unchanged \| +27 each"). Composing all three
premise cards together, every reading that previously lacked coro-plugin's
contribution now legitimately carries it. No site's verdict moved, no
instance flag's derivation moved, and no function or seed count moved on
any of the 11 compared readings; this is reported as an expected
composition effect, not treated as a failure.

## 5. Pin checks

Release binary `target/release/quirin-audit`, crate
`quirin-provider-fluent-bit`:

| Check | Root | Exit | Notes |
|---|---|---|---|
| whole-crate `pin --check` | `build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source` (66910c10) | 0 | |
| whole-crate `pin --check` | `private/w45-fb/w45-fb-engine-progress/attempt-1/fluent-bit-clang-source` (046accf6) | 0 | |
| whole-crate `pin --check` | `private/w47-tail/attempt-1/prep/fluent-bit-clang-source` (90978ac6 export, synthetic git `34155955`) | **1** | known **W47-FB-PIN-EXPORT-IDENTITIES** condition: 0 not in force, 0 stale, 51 in force through another revision |
| three-function `pin --check` (`flb_start`, `flb_stop`, `flb_destroy`) | all three roots | 0 / 0 / 0 | |
| `pin --verify-contracts` | (repo) | 0 | every provider crate's lock lists every declaration |

The 046accf6 root was read from
`private/w45-fb/w45-fb-engine-progress/attempt-1/fluent-bit-clang-source`
(the brief's path); it is the same git commit
(`046accf6aded7906f66a1baf0202da349e521736`) as
`private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source`,
which `docs/reports/w47-flb-lib-strict-pairing/scripts/pin.sh` and the
combined gate's `system` route use.

## Headline

**PASS.** The only non-zero exits (`fmt`, `provider-fmt`, `docs-index`) were
a formatting-only integration slip, fixed in `2aa377e5`; re-run there, all
three exit 0. The known `W47-FB-PIN-EXPORT-IDENTITIES` pin condition is the
only other non-zero exit anywhere in this report, and it is the documented
expected condition. Corpus is byte-identical; system and slice move no
status or leaf, only checkout-root/environment identity lines. Every
premise card's census reproduces its own archived verdicts, flag
derivations, function and seed counts exactly on the composed tree; the
observed edge-count increases are coro-plugin's own already-documented
per-card delta, now composed into readings that did not carry it before.

Commit on `integrate/w47-r9`: see the commit that adds this report,
`docs/reports/INDEX.md`'s regenerated row and the acceptance matrix
`validation` block update.

Measurement provenance: `docs/reports/w47-round9-integration/gate.json`.

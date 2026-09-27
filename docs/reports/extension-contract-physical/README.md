# EXT-02 contract source migration

Base `e7341b1debb75002e65645ff713bdc3ebda52fb7` includes the integrated
compatibility readers and the [physical migration prediction](../extension-physical-migration/prediction.md).
This card's physical edits are held until the integrator reports the EXT-01
standing gate passed. Its preparation does not alter a source, reference or
lock.

## User-authorized archive policy change (before mutation)

The user subsequently directed outdated frozen historical files to be changed
or removed to prevent confusion. This instruction overrides the repository's
general frozen-file preservation rule for the extension migration. The contract
rename set now includes all **195** mapped `.qv` targets: the original 177 live
sources and 18 files under `docs/reports/`. Sixteen archived files parse as
contract mode; two are deliberately malformed attribution fragments with a
contract header and remain malformed after their path changes. The original
source bytes and original measurement labels/digests will be recorded beside
the migrated names. Historical reports must not claim a past run used a new
path or digest. Executable historical commands and links that target one of
these sources should point to the new path with explicit provenance for their
original spelling. No report directory is removed wholesale.

## Before-change evidence

The [parser-derived mapping](../extension-compatibility/mapping.tsv) names
195 `.qcore` contract files proposed for `.qv`: 177 live sources (123 under
`providers/`, 41 under `crates/`, 11 under `compiler/`, one under `contract/`,
and one under `examples/`) and 18 archived sources. Of these, 187 parse
directly as contract mode, six use recognized include syntax, and two archived
contract-header fragments are intentionally malformed.
The [baseline](baseline.json) records every old/new path, Git index blob and
source SHA-256. Six sources include nine `#include` targets that must change
from `.qcore` to `.qv`; the baseline also records the expected SHA-256 after
exactly those nine edits. It records all ten provider revision locks, with 125 pinned
and 105 flexible entries. Exactly 230 lock module fields point to mapped
paths. Every entry's declaration digest and, for pinned entries, reviewed
source identities are captured for a field-by-field comparison. The
[verification tool](verify_paths.py) refuses any other changed source byte, a missing
or added rename, a changed lock count, digest, reviewed identity or metadata.

The [reference candidates](reference-candidates.tsv) list 222 tracked,
non-frozen files containing an exact mapped source path or a provider module
name. This is a review queue, not permission for global string replacement:
some rows retain intentional legacy fixtures, historical commentary or
virtual names, and two logical module names appear in more than one source
scope. The migration must update active includes, provider exports, corpus
selection, package/load-path inventories, revision-lock module fields, and
tests/current code references according to each consumer's actual target.
The [historical candidates](historical-reference-candidates.tsv) record 147
possible references to the 18 archived sources across 35 report files. Some
are historical measurements or the original EXT-01 inventory and must retain
their original identity claims; executable commands and live links to the
renamed source need targeted edits and a provenance note.

## Required sequence after the compatibility gate

1. Record a pre-change `quirin-audit pin --verify-contracts` result on the
   old load path and archive its log. Then use the [guarded source migrator](migrate_sources.py)
   to rename exactly the mapped files;
   preserve bytes except the nine mapped include target suffixes.
2. Rewrite includes and active exact references in the appropriate scope.
   Use [the guarded lock module migrator](migrate_lock_modules.py) to change
   each lock's `module` field by its owner crate's mapping only; keep
   all other fields, including declaration digests and reviewed identities,
   byte-equivalent as data. Do not regenerate or re-pin the lock.
3. Run `python3 providers/contracts.py sync`, then `check`, to regenerate
   declared package inventories and the load-path manifest from updated
   provider exports. Attribute every package digest change to the renamed
   path or changed include text.
4. Run `verify_paths.py verify`, the post-change pin verification, focused
   contract and provider tests, and the standing and provider gates in the
   integration sequence. Compare corpus and audit outcomes before claiming
   parity. A failing pin or Unknown-to-Safe movement blocks the card.

Preparation commands: `git submodule update --init --recursive` exited 0
at libevent `d82464a277d0f42703702c4dfd9af6af38595a83`;
`python3 docs/reports/extension-contract-physical/verify_paths.py baseline`
exited 0 and recorded 195 sources and ten locks. No cargo gate has run on this
preparation worktree; the integrator coordinates the warm target and physical
change start.

## Measured migration

The integrator reported the EXT-01 standing gate passed before source mutation.
The guarded migration renamed all 195 mapped sources, including the 18
historical sources authorized by the user. Nine include operands changed to
their `.qv` targets. `migrate_lock_modules.py --apply` changed 230 `module`
fields in ten locks, without repinning declaration digests or reviewed source
identities. `verify_paths.py verify` exited 0 after all edits: every renamed
source has its expected content SHA-256, and every lock entry retains its
non-path fields. `providers/contracts.py sync` and `check` both exited 0;
`quirin-audit pin --verify-contracts` exited 0 before and after the migration.
Logs are under `private/extension-contract-physical/` (`providers-sync.log`,
`providers-check.log`, `pin-before.log`, `pin-after.log`, and
`verify-paths-final.log`).

The release pinned arbiter ran on the same Cycle 19 inputs with baseline source
`e7341b1debb75002e65645ff713bdc3ebda52fb7` (`--jobs 1`, exit 0) and
migrated implementation `f4018a92` (`--jobs 4`, exit 0). The job-count difference
is allowed for verdict parity because the arbiter folds unit results in input
order; no runtime comparison is claimed. The four result artifacts below are
byte-identical before and after, including all 185 diagnostic units and both
96-row sidecars:

| Artifact | SHA-256 before and after |
| --- | --- |
| `resource-diagnostics.json` | `caf7ed01c51a702c78aee6664f0f156cde2b26f24f353569c41494b0e67ed965` |
| `resource-diagnostics.chunk-contracts.json` | `cbbcfd9c176005146c1946d5fcd4dc83cc9ec041a3397b10b1f0ab16ae6c0484` |
| `resource-diagnostics.source-effects.json` | `90194b1f11f941e4ca485dd84d1e799e76623b91a68f47ea448de0588f22a834` |
| `monkey-source.json` | `57acfc58c4f7dddf2623efb3c9dc3e8f2dc276835e15cd18a157414696bb2f11` |

The contract sidecar intentionally records new `.qv` module paths and source
text; its pin counts remain 125 pinned in force and 105 standing libc, with
zero pin mismatches or held functions/call sites. The run metadata also records
the new source revision, release binary digest, and job count. Private outputs
are `pinned-before/`, `pinned-after/`, `pinned-before-run.log`, and
`pinned-after.log`. This comparison establishes no changed verdict or
coverage row and no `Unknown`-to-`Safe` movement in the pinned corpus.

The first full Rust run (log `rust-tests.log`) found 58 stale fixture-path
failures across nine binaries; that failed attempt is retained. After updating
the fixture keys and the cmetrics binding's exact regenerated inventory
digest, focused reruns passed and the complete `run_test_binaries.py` gate
exited 0: 335 units, 348 summaries, 6,096 passed, zero failed, 11 ignored
(`rust-tests-2.log`, runner summary at
`private/test-binaries/20260927T130148Z-3371889/summary.json`).
`cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
both reevaluation examples, and `scripts/check_lean_audits.py` exited 0; Lean
passed 215/215 audits. The first Python run found a `.qcore`-only corpus walk
and accidentally modernized historical report expectations. After preserving
historical keys while admitting `.qv` in live discovery, the Python suite
exited 0 with 365/365 tests (`python-tests-4.log`).

The historical corpus diff JSON remains a record of old `.qcore` identities;
its tests still expect those original keys. Current executable consumers use
the new paths. The two archived malformed contract-header fragments remain
malformed with unchanged refusal status and no declaration digest claim.

## Provider validation

The first provider workspace `cargo test --manifest-path providers/Cargo.toml
--workspace --no-fail-fast` run failed only in `quirin-provider-plugin --test
sdk` (two failing tests). Its user-crate reader accepted only `.qcore`, while
the migrated example user crate owns `acme/pool.qv`. The SDK reader now accepts
both contract extensions and refuses simultaneous same-stem aliases. Its new
crate template emits `.qv`; the focused SDK suite passed 4/4 after the fix.
The failed full attempt and focused rerun remain at `provider-tests.log` and
`provider-sdk-fix.log`. Provider `cargo fmt --all -- --check` and
`cargo clippy --workspace --all-targets -- -D warnings` both exited 0
(`provider-fmt-check.log`, `provider-clippy.log`).

`providers/package.py build` exited 0 and wrote five packages to
`private/extension-contract-physical/provider-packages/` using the provider
warm target (`provider-package-build.log`). `providers/package.py verify` on
each of the five package **directories** exited 0 with no failures
(`provider-package-verify-3.log`). An initial verification invocation passed
archive paths instead of directories and failed; a second run reached the
source-mode probe without the writable target setting and failed on its Cargo
lock. Both attempts are retained as `provider-package-verify.log` and
`provider-package-verify-2.log`; neither was a package-identity refusal.

The full provider workspace test rerun after the SDK fix exited 0: 43
summaries, 129 passed, zero failed, one ignored (`provider-tests-2.log`). It
includes the four SDK tests. The first failed attempt remains distinct from
this complete green run.

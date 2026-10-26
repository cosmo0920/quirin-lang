# MEASURE-TOOL-LANDING-PUBLISH

Base `9c58892cfd0ab776bd7512fe8ef65516ebafbe84`. This card connects the
existing `quirin-audit measure` baseline publisher to the integrator's landing
sequence. It changes landing orchestration, not analyzer semantics, route
definitions, contracts, or Lean theorems.

`python3 scripts/land.py publish` runs after the full landing gate and records
commit. It requires a clean tree and release host/plugin binaries, measures the
current committed SHA as `tip=rev:<sha>` on the three live routes, and publishes
`private/baselines/<sha>/<profile>/` only after the comparison succeeds. It
selects the nearest first-parent ancestor with a published baseline and passes
that side as `prev=baseline:<sha>`; the plugin verifies its file, input, route,
and contract-bundle identity. The first publication requires `--bootstrap`,
which uses `prev=rev:<sha>` explicitly. `--previous` selects a particular
ancestor, and `--inputs-base` selects the prepared-input root. Each attempt has
a fresh private measurement path. An existing baseline refuses a second
publication.

The command-level Python test uses a temporary Git repository and a stub audit
host to exercise bootstrap followed by real baseline-side selection; another
test verifies that a missing baseline without `--bootstrap` refuses before a
measurement. The plugin's own tests cover baseline identity and publish
validation. This card's native runs and gate are recorded below.

## Native attempts

The first bootstrap attempt returned exit 1 before any route: sandboxed
`unshare` refused the plugin's fixed-path mount namespace (`Operation not
permitted`). Private log:
`private/measure-tool-landing-publish/bootstrap.log`; plugin build log:
`private/measure/landing-b0b63da7dcdd7c3d289259caf4067b72ad45c757-20261026T084715133585Z/sides/prev/build.log`.

The retry uses the same committed revisions and a fresh private output path
with mount-namespace permission. Its log is
`private/measure-tool-landing-publish/bootstrap-escalated.log`.

## Boundary

This branch is a card tip, so publishing its SHA proves the workflow but does
not claim a combined-main baseline. The integrator runs the full gate on the
landed commit, then `land.py publish` on the committed landing SHA. The open
`MEASURE-TOOL-LANDING-PUBLISH` follow-up retires only after a landed baseline
and the next card's real verified reuse.

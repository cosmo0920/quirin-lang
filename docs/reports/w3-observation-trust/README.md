# W3 observation trust preservation

Explicit caller `observations_trusted=false` previously became `true` during derivation. The baseline negative controls in `baseline-regression.log` and `baseline-replay.log` reproduce that override and acceptance by direct replay without a declared observation assumption.

Derivation now preserves caller assumptions and marks explicit refusal incomplete. Absent trust flags continue to name the approved conditional `observations_trusted=true` assumption. Replay independently requires true trust, observed-run bounds, and agreement between task and obligation assumptions. An incomplete task never dispatches; hostile direct witnesses cannot bypass these checks. Other production assumption insertion/extension sites were reviewed; no additional silent overrides were found.

These certificates express conditional agreement with supplied observations. They do not authenticate native provenance, establish production reachability, or generalize beyond observed runs. The positive schema-only external process fixture still passes.

Lean mirrors the trust default, explicit-refusal preservation and rejection, and necessary trust/bounds for acceptance. The Cycle 20 axiom audit remains free of project axioms.

All standing checks passed: 513 workspace tests, formatting, Clippy with warnings denied, both reevaluation probes, and `lake build`; see `gate-results.json`. This checkout's Libevent submodule was already initialized at the pinned revision and the workspace pin test passes. Baseline revision: `a664059`.

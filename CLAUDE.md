# Claude Code project notes

@AGENTS.md

Claude-specific additions:

- Before starting checker/architecture work, read `docs/GENERALIZATION_PLAN.md`
  and land code according to its target crate layering (§3) and invariants
  (§2); do not add new code to the monolithic grab-bag once a target crate
  exists.
- Never weaken a fail-closed gate, delete a pinned expectation, or downgrade
  a diagnostic to make a test pass. If a pinned table (contract corpus counts,
  submodule revision, archived report) conflicts with your change, the change
  must update the pin deliberately and say so.
- Status language matters in this project: keep the Safe/Unsafe/Unknown
  verdict vocabulary and the "unpromoted" phrasing exactly as defined in
  AGENTS.md when editing docs, reports, or commit messages.
- Run the full standing per-change gate from AGENTS.md before reporting a
  task complete; report probe/example exit codes honestly (a nonzero probe
  exit is a finding, not a build failure).

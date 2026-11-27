# Generalization integration checkpoint — 2026-09-07

Code revision `f9a3009` combines validated Removed and Out forwarding, sequential
and concurrent kernel consolidation, chunk schema/contract separation, whole-program
frontend relocation, generic resource accounting, trusted verification adapters,
five reproducible process-provider packages and explicit CLI package lifecycle.
Current revision0.3 requirements remain in `docs/QCORE_SPEC_r0_3.md`; obsolete
specifications are indexed under `docs/obsolete/README.md`.

The integrated main workspace passes **579 tests**, formatting, Clippy with
warnings denied, both reevaluation probes, Lean build and **19 axiom audits**.
The separate provider workspace's process conformance, malformed-request rejection
and two schema0.2.0 clean reproducible builds are recorded in
`../w5-provider-packages/`. Those executables remain Unknown-only. The prior
Python validator run passed43 tests (`../w5-resource-accounting/`); this checkpoint
does not claim a second run of the unchanged Python suite.

A fresh full185 audit uses the persistent forced-Libevent compile database at
`build/cycle19/monkey-build/compile_commands.json`. Resource diagnostics and source
JSON are byte-identical to the merged W4 forwarding report: **3 Safe /94 provisional
Unsafe /84 Unknown /4 data-only**. Relative to the pre-forwarding baseline, four
units and25 functions conservatively downgrade to Unknown; nothing is promoted.
Exact deltas and the reasons remain in `../w4-out-forwarding/`. Projection and
source inventories are unchanged. Raw production units remain unpromoted.

## Remaining scope

This is a verified integration checkpoint, not completion of W2–W7 or Cycle19.
Specialized provider model extraction, generic reactor/descriptor separation,
registry-based Fluent Bit composition and old-route removal remain open. W3
production lifecycle ABI/effect import, W4 general aliases/recursion/cost, broader
source result/concurrency/contract support,45 unmapped CT cases and the production
caller/observation/composition gates remain open.

A separate synthetic gateway probe on pre-forwarding main found that
`int *p=q_alloc(); q_free(p); return *p;` and a discarded `*p;` expression receive
Safe, while a volatile-read assignment is Unknown. Full returned records are
archived in `indirect-read-probe.log.gz`. The follow-on must reconcile those
unmodeled accesses with the actual custody verdict/support boundary and repair
optimistic acceptance without claiming a general C memory model. It is active
independently of the completed forwarding change; this checkpoint does not
claim it repaired or use green tests to override the probe.

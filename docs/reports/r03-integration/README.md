# Revision 0.3 integration and persistent audit inputs

The six inherited work tracks are merged onto main. Further W2–W7 integration
extracts the gateway, generic provider host and configured audit runner, moves
remaining bounded Monkey/Boolean/registration tables into declared profiles,
repairs source contract/binder gates, and adds Lean/diagnostic hygiene. These are
bounded workstream results; full kernel/provider consolidation and general
source language support remain open.

The supplied v0.3 review is adopted and reconciled against current implementation.
Its five original files are archived byte-identically under
`docs/reviews/r0_3/supplied`; no missing archive/probe package is represented as
executed. Current specifications are `docs/QCORE_SPEC_r0_3.md` and companions;
superseded revision0.1/0.2 specifications and their historical validation records
move to `docs/obsolete`. Active module/contract commands and navigation are fixed.
The machine-readable `docs/reviews/r0_3/acceptance.json` distinguishes repaired
model boundaries from partial typing and explicit trust premises.

R01/R02 lifetime state, R05 axis validation, R06 exact versus maximum custody and
R07 literal bounds are repaired with regressions and reduced Lean evidence.
R04 enforces ordinary i32/unit scalar compatibility while preserving an explicit
separate custody payload; full owned-result typing remains open. R03 explicitly
retains guard/synchronization premises; checked source invariant authority remains
open. R08/R10 replay now rejects scope/module/bounds substitutions without
claiming authenticated native provenance or whole-program refinement.

## Persistent Monkey build setup

The reported 20260906 temporary compile database was zero bytes. A JSON-only
copy would also retain temporary source, working-directory and generated-header
paths. `scripts/prepare_cycle19_monkey.py` now prepares all those inputs under
ignored, persistent `build/cycle19`, and checks the declared Fluent Bit pin.
Tracked source hashes and generated source files are recorded separately, so a
fresh-checkout setup remains repeatable after CMake generates its header/service
file. Every configuration attempt clears a previous success claim. An invalid
supplied Monkey database now rejects with its path before the expensive audit.

The setup was exercised from a fresh output directory, repeated, deliberately
failed through a false CMake command, and recovered. The default persistent
database has 142 entries and the required forced-Libevent mk_event.c command.
Configuration alone is not a native/sanitizer build claim. See
`docs/CYCLE19_BUILD_INPUTS.md` for the durable reproduction commands.

## Combined validation

All 549 workspace tests, both reevaluation probes, formatting, Clippy with
warnings denied, Lean build and 19 axiom audits passed after the review repairs
were combined. The Python evidence-validator suite passed 43 tests. These are
separate suites, not additional CT mappings. Five CT cases remain mapped and 45
unmapped. No project axioms were admitted by the Lean audit.

A fresh complete audit using the persistent database retains 185 units:
3 Safe / 98 provisional Unsafe / 80 Unknown / 4 data-only. Resource diagnostics
are byte-identical to the W4 extension baseline, with the hash in validation.json.
Raw units remain unpromoted; provisional warnings are not confirmed upstream
defects. The final audit's logs and reports are compressed here. Source report
paths reflect the new persistent build, and profile assumptions remain explicit.

Markdown links and fence structure were checked for the current spec, ledger,
validation/build guides and obsolete index. The design BNF has 175 defined
nonterminals and no unresolved references beyond its declared lexical terminals;
this checks reference coverage, not parser conformance or unambiguity. Historical
supplied documents retain their original unsupported references as provenance.

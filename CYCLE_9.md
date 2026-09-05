# Cycle 9: Conservative Legacy-C Resource Analyzer

**Status: Done**

Cycle 9 implements Stage 9 of the prototype roadmap. It adds a conservative,
path-sensitive resource analyzer for an explicitly supported annotated C
subset. The analyzer reports `Unknown` whenever parsing or alias semantics
leave that subset, rather than treating unmodeled C behavior as safe.

Cycle 7.5 remains postponed. No native Windows/MSVC work is required by or
included in Cycle 9, and `docs/CYCLE_7_5.md` remains unchanged.

## Rust implementation

`compiler/src/legacy_c.rs` provides:

- a C tokenizer and function-body parser integration for declarations, simple
  assignments, direct calls, compound blocks, `if`/`else`, `while`, and
  `return`;
- explicit resource API contracts classifying direct functions as acquire,
  release, or ownership escape operations;
- CFG import with one stable allocation-site identity per syntactic acquire;
- SSA-like strong pointer updates for direct assignments and null clearing;
- a conservative may-alias abstraction for direct pointer copies;
- a finite resource lattice recording possible live, released, and escaped
  states at every allocation site;
- a work-list fixed point across branches and loop back edges;
- path-sensitive exit checks for normal and early/error returns;
- diagnostics carrying both operation lines and acquisition lines;
- `Safe`, `Unsafe`, and `Unknown` summaries, with concrete safety defects taking
  priority over concurrent imprecision.

The supported annotated subset uses direct identifier assignments and direct
resource calls whose behavior is supplied through `ResourceContracts`.
Preprocessor-driven semantics, computed calls, fields, arrays, pointer
arithmetic, uncontracted calls, calls in conditions or returns, `goto`, `for`,
`do`, and `switch` are outside the current model. Encountering one produces
`Unknown` (or `Unsafe` if a definite/may safety defect is also found). This is
the precise scope of the no-false-negative claim; Cycle 9 does not claim sound
analysis of unrestricted C.

The executable corpus covers safe release, normal and error-path leaks, double
free through an alias, conditional cleanup, loop convergence, ownership
escape, unresolved behavior, compound blocks, and malformed input.

## Lean verification

`proof/QCore/LegacyCAnalyzer.lean` defines corresponding resource states,
instructions, diagnostics, transfer and exit semantics, verdicts, may-state
joins, and alias joins. It proves:

1. `safe_report_is_sound`: safety implies precision, no possibly-live exit
   resource, and no modeled safety defect;
2. no safe report contains a double-release or release-after-escape diagnostic;
3. imprecise states and unsupported instructions are never safe;
4. seeded leaks and double releases classify as defects;
5. acquire followed by release classifies as safe; and
6. CFG and alias joins retain live and unknown possibilities from either
   predecessor.

No proof contains a placeholder or project-defined axiom.

## Complete verification instructions

From the repository root:

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
if rg -n '\b(sorry|admit)\b|^[[:space:]]*axiom\b' proof \
  --glob '*.lean'; then exit 1; fi
cd proof
lake build
lake env lean AuditCycle9.lean
```

The theorem-dependency audit is expected to report only Lean's standard
`propext` and `Quot.sound` foundations where used by simplification and list
membership.

## Verified completion result

Completion is verified by 90 passing Rust tests, including 13 legacy-C
analyzer cases and the existing compiled-C sanitizer corpus, warning-free
formatting and strict Clippy, a successful 14-job Lean build, a source audit
with no proof placeholders or project axioms, and a Cycle 9 dependency audit
limited to the standard foundations above.

## Remaining work

Stage 9 is complete for its stated subset. The overall language remains
unfinished; the sequenced remaining milestones are recorded in
[`docs/ROADMAP_AFTER_CYCLE_9.md`](docs/ROADMAP_AFTER_CYCLE_9.md).

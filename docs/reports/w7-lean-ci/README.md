# W7 Lean CI hygiene

Date: 2026-09-07. The missing Lean CI job (EV01) and follow-up reevaluation
checks are implemented. Hosted CI has **not run**; this report records local
validation of the same commands.

The new Ubuntu Lean job uses `leanprover/lean-action@v1`, the `proof` package
and its existing `lean-toolchain`, with explicit build configuration. These
inputs are documented in the [official action definition](https://github.com/leanprover/lean-action/blob/v1/action.yml).
After building, `scripts/check_lean_audits.py` runs every `Audit*.lean` file with
warnings treated as errors. Missing `AuditAll.lean` fails rather than succeeding
with an empty audit inventory.

`AuditAll.lean` examines the compiled environment and computes the transitive
axiom closure of every public or private declaration in the QCore modules.
Only `propext`, `Classical.choice` and `Quot.sound` are allowed. Thus a project
axiom, a `sorry` placeholder, or a theorem depending on either fails even if
no cycle-specific print audit names it. Existing cycle audits remain in the
run and fail on stale theorem names. The current QCore root imports all 49
model modules. This is an audit of Lean declarations, not evidence that Rust
or arbitrary imported C implements every model theorem.

Linux and Clang-enabled Windows Rust jobs now execute `reevaluation_followup`
alongside `reevaluation_probe`. Windows filtered library tests now include
`--workspace`, ensuring the moved gateway's Clang and legacy-C tests actually
run instead of a successful zero-test filter.

## Local evidence

- Full standing gate passed: 473 workspace tests, formatting, clippy, both
  reevaluation probes and the 52-job Lean build.
- All 17 audit files passed. `AuditAll` examined 7,471 declarations with only
  the allowed logical axioms; [audits.log](audits.log) preserves the output.
- Injecting a QCore project axiom and injecting a theorem using `sorry` both
  failed the actual Lean audit. [negative-controls.json](negative-controls.json)
  records those outputs. Temporary injected files are not part of the proof.
- No semantic/model definition changed. Generic model naming cleanup remains
  a separate W7 step.

Run locally after `lake build` with the pinned toolchain on PATH:

```sh
python3 scripts/check_lean_audits.py
```

[validation.json](validation.json) records the local gate and explicitly retains
the pending hosted-CI status.

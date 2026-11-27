#!/usr/bin/env python3
"""CG26-TAIL-RESIDUAL-ACTUALS: write the gate section of the report and the
note of gate.json from the g2 run. Idempotent."""
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[1]

gate_path = ROOT / "gate.json"
gate = json.loads(gate_path.read_text())
gate["note"] = gate["note"].replace(
    "rerun after build_matrix.py . --gate this file (09a, 09b);",
    "rerun after build_matrix.py . --gate this file (09a 0, 09b 0: 365 OK; a first finish-gate run had "
    "refused this file for its missing provider_tests field, 09a 1, so 09b was still 1);")
gate_path.write_text(json.dumps(gate, indent=2, ensure_ascii=False) + "\n")

SECTION = """The final gate ran at `ceb599cc5` on base `cb3085365`. Commits after it
touch only records and gate files. It used
[`scripts/gate.sh`](scripts/gate.sh) and
[`scripts/finish-gate.sh`](scripts/finish-gate.sh), each step under the
shared heavy lock. Logs are in this worktree's `private/cg26r/gate-g2`, exits
in [`gate/`](gate/), and the summary in [`gate.json`](gate.json).

| Step | Command | Exit | Count / note |
|---|---|---|---|
| check | `cargo check --workspace --all-targets` | 0 | |
| submodule | `git submodule update --init --recursive` | 0 | |
| fmt | `cargo fmt --all -- --check` | 0 | |
| clippy | `cargo clippy --locked --all-targets -- -D warnings` | 0 | |
| tests | `python3 scripts/run_test_binaries.py` | 0 | 437 units, 450 summaries, **7,517 passed**, 0 failed, 11 ignored (`private/test-binaries/20261003T025140Z-2852012`) |
| probes | `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| Lean | `python3 scripts/check_lean_audits.py` | 0 | **283** audits (282 + `AuditVariadicTailActuals`) |
| Python | `python3 -m unittest discover -s scripts/tests` | 1, then **0** after `build_matrix.py . --gate gate.json` | 365. Before regeneration, `test_build_matrix_sections` fails: the committed matrix's derived `lean_audits` is 282 and the new audit makes it 283, as at every Lean-adding card |
| docs index | `python3 scripts/build_docs_index.py --check` | 0 | |
| provider tests | `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline` | 0 | 129 passed. No provider path changed, so the provider gate is not owed; this ran only for the matrix's `provider_tests` field |

**Earlier attempts.** They are retained, and they are not evidence of a pass:

- **Gate g1** (`e0d6c8a0a`, code steps only, `private/cg26r/gate-g1`) failed
  clippy once. The lint was `type_complexity` on the test-only
  `READ_VECTORS` constant. Check, fmt, tests (7,517 passed), the probes and
  the 283 Lean audits passed. `ceb599cc5` names the row type.
- **The first `finish-gate.sh` run** refused `gate.json` for its missing
  `provider_tests` field (09a exit 1).

The corpus binary is unaffected by both: the alias is `#[cfg(test)]`."""

readme = ROOT / "README.md"
text = readme.read_text()
if "GATE_SECTION" in text:
    readme.write_text(text.replace("GATE_SECTION", SECTION))
print("ok")

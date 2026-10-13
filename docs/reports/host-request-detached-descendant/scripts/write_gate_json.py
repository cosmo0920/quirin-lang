#!/usr/bin/env python3
"""write_gate_json.py NAME -- HOST-REQUEST-READ-BY-DETACHED-DESCENDANT (from
HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING's): gate.json from the standing
gate's logs in private/hdd/gate-NAME (exits, the runner's totals line, the Lean
audit count, the provider gate's counts) and the Clang recording.

Every exit must be 0 except one accounted step: the clippy step (03), red on a
lint that the host's rustc/clippy 1.99.0, installed during the card, raises in
a line this card does not touch. It is accounted only when that lint at that
line is the log's sole finding, and when the same command with only that lint
allowed (private/hdd/clippy-diagnostic) exited 0; the record then says the
gate is incomplete. Any other nonzero exit fails this script, so a red gate
never yields a green summary."""
import json
import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]
REPORT = REPO / "docs/reports/host-request-detached-descendant"
BASE = "b565f79ab"
LINT = "needless_borrows_for_generic_args"
SITE = "crates/qcore-verification/src/source_imports.rs:186:63"


def clippy_accounted(logs: pathlib.Path) -> bool:
    log = (logs / "03-clippy.log").read_text()
    sites = re.findall(r"^\s+--> (\S+)", log, flags=re.M)
    diagnostic = REPO / "private/hdd/clippy-diagnostic.exit"
    return (
        sites == [SITE]
        and LINT.replace("_", "-") in log
        and diagnostic.is_file()
        and diagnostic.read_text().strip() == "0"
    )


def main() -> int:
    name = sys.argv[1]
    logs = REPO / "private/hdd" / f"gate-{name}"
    exits = {path.stem: path.read_text().strip() for path in sorted(logs.glob("*.exit"))}
    bad = {k: v for k, v in exits.items() if v != "0"}
    clippy_red = bad.pop("03-clippy", None) is not None
    if clippy_red and not clippy_accounted(logs):
        print("clippy failed for a reason other than the accounted lint")
        return 1
    if bad:
        print("nonzero exits:", bad)
        return 1
    totals = re.search(
        r"test binaries: (\d+) units, (\d+) summaries; (\d+) passed; (\d+) failed; (\d+) ignored",
        (logs / "04-tests.log").read_text(),
    )
    lean = re.search(r"Passed (\d+) Lean audits", (logs / "07-lean.log").read_text())
    provider = re.findall(
        r"test result: ok\. (\d+) passed; (\d+) failed", (logs / "12-provider-tests.log").read_text()
    )
    provider_passed = sum(int(p) for p, _ in provider)
    provider_failed = sum(int(f) for _, f in provider)
    provider_python = re.search(r"Ran (\d+) tests", (logs / "13-provider-python.log").read_text())
    clang = (logs / "clang-version.txt").read_text().splitlines()[0]
    head = (logs / "head").read_text().strip()
    clippy_text = (
        f"clippy --locked {exits['03-clippy']}: INCOMPLETE. The host's stable toolchain became "
        f"rustc/clippy 1.99.0 during the card, and its {LINT} lint fires at {SITE} "
        "(`map_err(&fail)`), a line this card does not touch; the same command with only that "
        "lint allowed exits 0 over the whole workspace (private/hdd/clippy-diagnostic.log). "
        "Not fixed here (out of scope), owed to the integrator; "
        if clippy_red
        else "clippy --locked 0; "
    )
    note = (
        f"HOST-REQUEST-READ-BY-DETACHED-DESCENDANT card gate on base {BASE}, every code step at "
        f"{head[:9]}, scripts docs/reports/host-request-detached-descendant/scripts/gate.sh, "
        f"logs private/hdd/gate-{name} of this worktree. cargo check --workspace --all-targets 0; "
        "submodule 0; cargo fmt --check 0; " + clippy_text + "run_test_binaries.py "
        f"{totals.group(1)} units, {totals.group(2)} summaries, {int(totals.group(3)):,} passed, "
        f"{totals.group(4)} failed, {totals.group(5)} ignored, exit {exits['04-tests']} "
        f"(private/hdd/tests-{name}), with no rerun; both reevaluation probes exit 0; "
        f"check_lean_audits.py {lean.group(1)} audits passed (AuditRequestConsumption extended, "
        "no audit added); provider gate: providers/package.py build and verify qcore.libevent "
        f"0 / 0, cargo test in providers/ {provider_passed} passed / {provider_failed} failed, "
        f"providers/tests {provider_python.group(1)} OK; the python suite and the docs index "
        "check per gate/ (08, 09)."
        + (" The matrix was not regenerated with this file, because the gate is incomplete."
           if clippy_red else "")
    )
    gate = {
        "note": note,
        "workspace_tests": int(totals.group(3)),
        "provider_tests": provider_passed,
        "python_evidence_validator_tests": None,
        "reevaluation_probe_exit": int(exits["05-probe"]),
        "reevaluation_followup_exit": int(exits["06-followup"]),
        "cargo_fmt": "clean",
        "cargo_clippy": (f"red: one {LINT} finding at {SITE}, a line this card does not touch; "
                         "clean with that lint allowed") if clippy_red else "clean",
        "standing_gate": "incomplete (clippy)" if clippy_red else "complete",
        "lean_audits": int(lean.group(1)),
        "clang": f"present: {clang}",
    }
    python_log = logs / "08-python.log"
    if python_log.exists():
        ran = re.search(r"Ran (\d+) tests", python_log.read_text())
        gate["python_evidence_validator_tests"] = int(ran.group(1)) if ran else None
    (REPORT / "gate.json").write_text(json.dumps(gate, indent=2, ensure_ascii=False) + "\n")
    print(json.dumps(gate, indent=2)[:2000])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

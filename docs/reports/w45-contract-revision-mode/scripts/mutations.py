#!/usr/bin/env python3
"""Controlled-mutation harness of card W45-CONTRACT-REVISION-MODE.

usage: mutations.py BINARY WORK_DIR LOG_DIR      (from the repository root)

Reads the generic held-out fixture `crates/qcore-audit/tests/fixtures/
revision_mode/` (queue.c, queue.qcore, mutations.json). For each mode:

* flexible: the contracts as written (every declaration `revision flexible`);
* pinned: the same contracts with every `requires resource_revision(flexible);`
  removed (pinned by default), locked by `quirin-audit pin --modules` against
  the unmutated fixture;

and for each mutation it writes the mutated source to a fresh directory under
WORK_DIR (which must not exist), runs `quirin-audit contracts` on it, and
classifies the mutated function and every function that relies on its
contract: `accepted` (Safe), `violation-detected` (Unsafe) or `coverage-lost`
(Unknown, with the reason codes of its refusals). Per-mutation logs (console,
report rows, contract_revision section) go to LOG_DIR/<mode>/<mutation>/, and
the table to LOG_DIR/table.json. Exit 0 when every run completed and every
harmless mutation stayed accepted under flexible, every defect was caught or
reported as lost coverage (never accepted) under flexible, and every body
change left the mutated contract out of force under pinned.
"""
import json
import os
import pathlib
import shutil
import subprocess
import sys

FIXTURE = pathlib.Path("crates/qcore-audit/tests/fixtures/revision_mode")
FLEXIBLE_MARKER = "    requires resource_revision(flexible);\n"
CLANG = os.environ.get("QCORE_CLANG", "/usr/lib/llvm-21/bin/clang")
CALLERS = {"box_drop": ["user_drop"], "box_keep": ["user_keep"], "box_count": [], "box_two": []}


def mutate(text, edits):
    for old, new in edits:
        if text.count(old) != 1:
            raise SystemExit(f"edit does not match exactly once: {old!r}")
        text = text.replace(old, new)
    return text


def reasons(function):
    found = set()
    for diagnostic in function["diagnostics"]:
        description = diagnostic["description"]
        if "(contract-pin-mismatch)" in description:
            found.add("contract-pin-mismatch")
        else:
            found.add(diagnostic["category"])
    return sorted(found)


def classify(function):
    status = function["status"]
    if status == "Safe":
        return "accepted"
    if status == "Unsafe":
        return "violation-detected"
    return "coverage-lost"


def run(binary, arguments, log):
    completed = subprocess.run([binary, *arguments], capture_output=True, text=True,
                               env={**os.environ, "QCORE_CLANG": CLANG})
    log.write_text(completed.stdout + completed.stderr + f"\nexit {completed.returncode}\n")
    return completed.returncode


def main():
    binary = str(pathlib.Path(sys.argv[1]).resolve())
    work = pathlib.Path(sys.argv[2]).resolve()
    logs = pathlib.Path(sys.argv[3])
    work.mkdir(parents=True, exist_ok=False)
    if logs.exists():
        shutil.rmtree(logs)
    logs.mkdir(parents=True)
    spec = json.loads((FIXTURE / "mutations.json").read_text())
    source = (FIXTURE / spec["fixture"]).read_text()
    contracts = (FIXTURE / spec["contracts"]).read_text()
    variants = {"flexible": contracts, "pinned": contracts.replace(FLEXIBLE_MARKER, "")}
    rows = []
    ok = True
    for mode, text in variants.items():
        # The contracts and the lock, pinned against the unmutated fixture.
        contract_dir = work / mode / "contracts"
        contract_dir.mkdir(parents=True)
        (contract_dir / "queue.qcore").write_text(text)
        (work / mode / "manifest.json").write_text(
            json.dumps({"schema_version": 1, "modules": ["contracts/queue.qcore"]}) + "\n")
        base_source = work / mode / "base-source"
        base_source.mkdir()
        (base_source / "queue.c").write_text(source)
        (logs / mode).mkdir()
        pin_exit = run(binary, ["pin", "--source-root", str(base_source), "--modules",
                                str(contract_dir)], logs / mode / "pin.log")
        if pin_exit != 0:
            ok = False
        for mutation in spec["mutations"]:
            name = mutation["name"]
            directory = work / mode / "runs" / name
            (directory / "src").mkdir(parents=True)
            (directory / "src" / "queue.c").write_text(mutate(source, mutation["edits"]))
            log_dir = logs / mode / name
            log_dir.mkdir()
            exit_code = run(binary, ["contracts", "--source-root", str(directory / "src"),
                                     "--contract-manifest", str(work / mode / "manifest.json"),
                                     "--report", str(directory / "report.json")],
                            log_dir / "console.log")
            if exit_code != 0:
                ok = False
                rows.append({"mode": mode, "mutation": name, "exit": exit_code})
                continue
            report = json.loads((directory / "report.json").read_text())
            sidecar = json.loads((directory / "report.contracts.json").read_text())
            section = sidecar["contract_revision"]
            functions = {f["function"]: f for u in report["units"] for f in u["functions"]}
            (log_dir / "functions.json").write_text(json.dumps(
                [{"function": n, "status": f["status"], "diagnostics": f["diagnostics"]}
                 for n, f in sorted(functions.items())], indent=2) + "\n")
            (log_dir / "contract_revision.json").write_text(json.dumps(
                {key: section[key] for key in ("counts", "pin_mismatches", "contracts")},
                indent=2) + "\n")
            mutated = mutation["function"]
            watched = [mutated, *CALLERS.get(mutated, [])] if mutated else sorted(functions)
            outcome = {
                name_: {"status": functions[name_]["status"], "class": classify(functions[name_]),
                        "reasons": reasons(functions[name_])}
                for name_ in watched
            }
            row = {
                "mode": mode, "mutation": name, "kind": mutation["kind"], "function": mutated,
                "outcome": outcome,
                "not_in_force": sorted(m["function"] for m in section["pin_mismatches"]),
            }
            if mode == "flexible" and mutation["kind"] == "harmless":
                row["meets"] = outcome[mutated]["class"] == "accepted"
            elif mode == "flexible" and mutation["kind"] == "defect":
                row["meets"] = any(o["class"] != "accepted" for o in outcome.values())
            elif mode == "pinned" and mutated:
                row["meets"] = mutated in row["not_in_force"]
            else:
                row["meets"] = all(o["class"] == "accepted" for o in outcome.values()) and \
                    not row["not_in_force"]
            ok = ok and row["meets"]
            rows.append(row)
    table = {"schema_version": 1, "harness": "docs/reports/w45-contract-revision-mode/scripts/mutations.py",
             "fixture": str(FIXTURE), "binary_sha256": subprocess.run(
                 ["sha256sum", binary], capture_output=True, text=True).stdout.split()[0],
             "rows": rows, "all_met": ok}
    (logs / "table.json").write_text(json.dumps(table, indent=2) + "\n")
    for row in rows:
        summary = {k: f'{v["status"]}{"/" + ",".join(v["reasons"]) if v["reasons"] else ""}'
                   for k, v in row.get("outcome", {}).items()}
        print(f'{row["mode"]:8} {row["mutation"]:25} meets={row.get("meets")} '
              f'not_in_force={row.get("not_in_force")} {summary}')
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())

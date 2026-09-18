#!/usr/bin/env python3
"""Card SCALAR-ROUTE-EXIT-EDGE: falsify every arm the card added.

Each mutation edits `legacy_c.rs`, runs the card's own test binary, records
which tests went red, then restores the pristine file **with its mtime
advanced** so the next build is not served a stale artifact. The run ends by
re-checking that the restored tree is byte-identical to the pristine one and
green.

A mutation that leaves the suite green is a finding: it means the arm it
removed is not driven by any test.

Usage: python3 mutate.py
"""
import hashlib
import pathlib
import subprocess
import sys
import time

WT = pathlib.Path(__file__).resolve().parents[2]
M = WT / "measure" / "scalar-route-exit-edge"
SRC = WT / "crates" / "qcore-c-gateway" / "src" / "legacy_c.rs"
TEST = "scalar_route_exit_edge"

RETAIN = """                    PointerRefinement::Infeasible if node.loop_exit_edge == Some(edge) => {
                        edge_output = output.clone();
                        edge_output.unmodeled = true;
                    }
"""

MUTATIONS = [
    (
        "M1-prune-the-exit-edge-again",
        "the whole restriction: the exit edge is deleted as before",
        [
            (
                RETAIN,
                """                    PointerRefinement::Infeasible if node.loop_exit_edge == Some(edge) => {
                        let _ = &output;
                        continue;
                    }
""",
            ),
            ("    report_retained_loop_exit_edges(&cfg, &incoming, &mut diagnostics);\n", ""),
            (
                "fn report_retained_loop_exit_edges(",
                "#[allow(dead_code)]\nfn report_retained_loop_exit_edges(",
            ),
        ],
    ),
    (
        "M2-drop-the-settled-report",
        "the edge is retained but the refusal is never reported",
        [
            ("    report_retained_loop_exit_edges(&cfg, &incoming, &mut diagnostics);\n", ""),
            (
                "fn report_retained_loop_exit_edges(",
                "#[allow(dead_code)]\nfn report_retained_loop_exit_edges(",
            ),
        ],
    ),
    (
        "M3-while-header-forgets-its-exit-edge",
        "the `while` header records no loop exit edge, so the rule cannot see it",
        [
            (
                "                self.nodes[header].loop_exit_edge = Some(1);\n",
                "                self.nodes[header].loop_exit_edge = None;\n",
            )
        ],
    ),
    (
        "M4-protect-every-edge",
        "the rule fires on branches too, destroying ordinary path sensitivity",
        [
            (
                "PointerRefinement::Infeasible if node.loop_exit_edge == Some(edge) => {",
                "PointerRefinement::Infeasible if node.loop_exit_edge == Some(edge) || true => {",
            )
        ],
    ),
    (
        "M5-protect-the-body-edge-instead",
        "the protected index is the body edge, not the leaving one",
        [
            (
                "                self.nodes[header].loop_exit_edge = Some(1);\n",
                "                self.nodes[header].loop_exit_edge = Some(0);\n",
            )
        ],
    ),
    (
        "M6-report-inside-the-work-list",
        "the refusal is reported on intermediate iterates, not settled states",
        [
            (
                RETAIN,
                """                    PointerRefinement::Infeasible if node.loop_exit_edge == Some(edge) => {
                        edge_output = output.clone();
                        edge_output.unmodeled = true;
                        diagnostics.insert(LegacyDiagnostic {
                            function: cfg.function.clone(),
                            line: node.line,
                            kind: LegacyDiagnosticKind::Unsupported {
                                description: LOOP_EXIT_EDGE_RETAINED.to_owned(),
                            },
                        });
                    }
""",
            )
        ],
    ),
]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run_tests():
    done = subprocess.run(
        ["cargo", "test", "-p", "qcore-c-gateway", "--test", TEST],
        cwd=WT,
        capture_output=True,
        text=True,
    )
    out = done.stdout + done.stderr
    red = sorted(
        line.split()[1]
        for line in out.splitlines()
        if line.startswith("test ") and line.rstrip().endswith("FAILED")
    )
    return done.returncode, red, out


def main():
    pristine = SRC.read_text()
    before = digest(SRC)
    (M / "mutations.log").write_text("")
    log = (M / "mutations.log").open("a")

    code, red, _ = run_tests()
    print(f"baseline: exit={code} red={red}")
    log.write(f"baseline: exit={code} red={red}\n")
    if code != 0:
        print("baseline is not green; stopping", file=sys.stderr)
        return 1

    failures = []
    for name, what, edits in MUTATIONS:
        text = pristine
        ok = True
        for old, new in edits:
            if text.count(old) != 1:
                print(f"{name}: anchor not unique ({text.count(old)}) for {old[:50]!r}")
                log.write(f"{name}: ANCHOR NOT UNIQUE\n")
                ok = False
                break
            text = text.replace(old, new)
        if not ok:
            failures.append(name)
            continue
        SRC.write_text(text)
        time.sleep(1.1)
        code, red, out = run_tests()
        verdict = "RED (arm is driven)" if code != 0 else "GREEN — VACUOUS ARM"
        print(f"{name}: exit={code} {verdict}\n    {what}\n    red: {red}")
        log.write(f"{name}: exit={code} {verdict}\n  {what}\n  red: {red}\n")
        if code == 0:
            failures.append(name)
            (M / f"mutation-{name}.out").write_text(out)
        # restore, mtime advanced
        SRC.write_text(pristine)
        time.sleep(1.1)
        SRC.touch()

    SRC.write_text(pristine)
    SRC.touch()
    after = digest(SRC)
    code, red, _ = run_tests()
    print(f"\nrestored: sha256 {'EQUAL' if before == after else 'DIFFERS'}  tests exit={code}")
    log.write(f"restored: sha256 {'EQUAL' if before == after else 'DIFFERS'} exit={code}\n")
    log.close()
    if failures:
        print(f"VACUOUS OR BROKEN: {failures}")
        return 1
    return 0 if before == after and code == 0 else 1


if __name__ == "__main__":
    sys.exit(main())

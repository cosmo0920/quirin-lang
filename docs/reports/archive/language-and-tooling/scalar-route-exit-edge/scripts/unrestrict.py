#!/usr/bin/env python3
"""Card SCALAR-ROUTE-EXIT-EDGE: undo this card's restriction, keeping the field.

Turns the retaining arm back into the pruning `continue` and drops the settled
report, so the binary reproduces the BASE analyzer's verdicts exactly while
`LegacyCfgNode::loop_exit_edge` is still present for the census to read a
role off. Combined with `instrument_census.py` this is the BEFORE census
build: base behaviour, instrumented.

It is also this card's **mutation harness**: applied on its own (no census) it
must turn `scalar_route_exit_edge.rs`'s withdrawing half red.

Usage: python3 unrestrict.py <legacy_c.rs> <out.rs>
"""
import pathlib
import sys

RETAIN = """                    PointerRefinement::Infeasible if node.loop_exit_edge == Some(edge) => {
                        edge_output = output.clone();
                        edge_output.unmodeled = true;
                    }
"""

PRUNE = """                    PointerRefinement::Infeasible if node.loop_exit_edge == Some(edge) => {
                        let _ = &output;
                        continue;
                    }
"""

REPORT = "    report_retained_loop_exit_edges(&cfg, &incoming, &mut diagnostics);\n"


def main():
    if len(sys.argv) != 3:
        print("usage: unrestrict.py <legacy_c.rs> <out.rs>", file=sys.stderr)
        return 2
    source = pathlib.Path(sys.argv[1]).read_text()
    for label, needle in (("retain arm", RETAIN), ("settled report", REPORT)):
        if source.count(needle) != 1:
            print(f"{label} not unique ({source.count(needle)})", file=sys.stderr)
            return 1
    patched = source.replace(RETAIN, PRUNE).replace(REPORT, "")
    # The settled report is now dead; keep it compiling.
    patched = patched.replace(
        "fn report_retained_loop_exit_edges(",
        "#[allow(dead_code)]\nfn report_retained_loop_exit_edges(",
    )
    pathlib.Path(sys.argv[2]).write_text(patched)
    print(f"unrestricted -> {sys.argv[2]}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""OUTER-CONTRACT-DESIGN: which runtime-loading units each route compiles.

For every compile database a route reads (corpus pinned input, system
prepared attempt, tail-pause slice prepared attempt), list the units that
implement or reach a runtime-loading mechanism (dlopen-based plugin loaders,
the Go proxy, librdkafka's plugin and interceptor code, Monkey's plugin
loader, the library-mode API, and the Lua / WASM / NVML hosts), and whether
the route's selection (system, slice) analyses them. Read-only; prints JSON.
"""

import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
QC = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c")
DBS = {
    "corpus": QC / "build/cycle19/c-gateway-wave-pin/fluent-bit-clang-production/compile_commands.json",
    "system": QC / "private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-production/compile_commands.json",
    "slice": QC / "private/w47-tail/attempt-1/prep/fluent-bit-clang-production/compile_commands.json",
}
SELECTIONS = {
    "system": ROOT / "contract/profiles/fluent-bit/system-selection.json",
    "slice": ROOT / "contract/profiles/fluent-bit/tail-pause-selection.json",
}
PATTERN = re.compile(
    r"(src/flb_plugin\.c|src/flb_plugin_proxy\.c|src/proxy/go/go\.c|"
    r"rdkafka_plugin\.c|rddl\.c|rdkafka_interceptor\.c|mk_server/mk_plugin\.c|"
    r"src/flb_lib\.c|plugins/out_lib/.*\.c|plugins/in_lib/.*\.c|lj_clib\.c|"
    r"lib_package\.c|nvml_gpu\.c|plugins/filter_lua/.*\.c|src/wasm/.*\.c|"
    r"wasi_nn\.c|sqlite3\.c|src/flb_api\.c|fluent-bit\.c|src/flb_config\.c)$"
)


def selected(route):
    path = SELECTIONS.get(route)
    if path is None or not path.exists():
        return None
    data = json.loads(path.read_text())
    text = json.dumps(data)
    return text


def main() -> int:
    result = {}
    for route, db in DBS.items():
        if not db.exists():
            result[route] = {"compile_database": str(db), "missing": True}
            continue
        entries = json.loads(db.read_text())
        files = sorted({e["file"] for e in entries})
        sel = selected(route)
        hits = []
        for f in files:
            if PATTERN.search(f):
                rel = re.sub(r"^.*?/fluent-bit-clang-source/", "", f)
                hits.append({
                    "unit": rel,
                    "in_selection": None if sel is None else (rel in sel),
                })
        result[route] = {
            "compile_database": str(db),
            "units": len(files),
            "loader_units": hits,
        }
    json.dump(result, sys.stdout, indent=1)
    print()
    return 0


if __name__ == "__main__":
    sys.exit(main())

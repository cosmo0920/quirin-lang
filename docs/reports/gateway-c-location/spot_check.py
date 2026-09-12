"""Hand-verifiable sample: read a presented coordinate back out of the pinned
source and print the line it names.

Card GATEWAY-C-LOCATION. The map is derived from Clang's markers; this is the
independent check that the coordinate it publishes is the line a person would
open. For each sampled diagnostic it prints the internal preprocessed line, the
presented coordinate, and the **text of that line in the pinned checkout**, so a
reader can compare the two by eye.

    python3 -B measure/spot_check.py measure/runs/after
"""
import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
PIN = ROOT / "build/cycle19/clang-pin/fluent-bit-clang-source"
RUN = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "measure/runs/after"

# Where each audited library's sources live inside the pinned checkout.
ROOTS = {
    "fluent-bit-chunk": ["src"],
    "cmetrics": ["lib/cmetrics/src"],
    "ctraces": ["lib/ctraces/src"],
    "librdkafka": ["lib/librdkafka-2.12.1/src"],
    "chunkio": ["lib/chunkio/src"],
    "monkey-libevent": ["lib/monkey/mk_core", "lib/monkey/mk_server", "lib/monkey"],
}

#: (library, file, preprocessed line) samples, chosen to cover the leaf families
#: the corpus record names plus one ordinary unit from another library.
SAMPLES = [
    ("fluent-bit-chunk", "flb_input_chunk.c", 19357),
    ("fluent-bit-chunk", "flb_input_chunk.c", 19384),
    ("fluent-bit-chunk", "flb_input_chunk.c", 19388),
    ("fluent-bit-chunk", "flb_input_chunk.c", 19399),
    ("fluent-bit-chunk", "flb_input_chunk.c", 19544),
    ("fluent-bit-chunk", "flb_input_chunk.c", 19849),
    ("cmetrics", "cmetrics.c", 5588),
    ("cmetrics", "cmetrics.c", 5595),
]


def find_source(library, name):
    for root in ROOTS.get(library, []):
        for path in (PIN / root).rglob(name):
            return path
    for path in PIN.rglob(name):
        return path
    return None


report = json.loads((RUN / "resource-diagnostics.json").read_text())
index = {}
for unit in report["units"]:
    for function in unit.get("functions", []):
        for row in function.get("diagnostics", []):
            index.setdefault((unit["library"], unit["file"], row["line"]), []).append(
                (function["function"], row)
            )

SITE = re.compile(r"([^\s]+):(\d+)")


def source_line(library, spelled, number):
    path = find_source(library, pathlib.PurePath(spelled).name)
    if path is None:
        return ""
    lines = path.read_text(errors="replace").split("\n")
    return lines[number - 1].strip() if 1 <= number <= len(lines) else ""


print("A. Sampled diagnostic rows: the internal position, the presented coordinate,")
print("   and the text of that line in the pinned checkout.")
print()
print(f"{'unit':40s} {'L':>7s}  {'presented':30s} source line")
print("-" * 128)
for library, name, line in SAMPLES:
    rows = index.get((library, name, line))
    if not rows:
        print(f"{library + '/' + name:40s} {line:7d}  (no diagnostic row at this line)")
        continue
    function, row = rows[0]
    origin = row.get("origin")
    text = ""
    if origin:
        spelled, _, number = origin.rpartition(":")
        text = source_line(library, spelled, int(number))
    print(
        f"{library + '/' + name:40s} {line:7d}  {str(origin):30s} {text[:52]}"
        f"   [{function}] {row['category']}"
    )

print()
print("B. Every leak of the seven-leaf family the corpus record names. A leak row's")
print("   own position is its exit; its description names the ACQUISITION, which is")
print("   the coordinate the previous records wrote as L19357 / L19388.")
print()
FAMILY = ("fluent-bit-chunk", "flb_input_chunk.c", "flb_input_chunk_get_direct_routes")
print(f"{'exit L':>7s} {'exit origin':26s} {'acquisition (from the text)':32s} source line")
print("-" * 128)
for unit in report["units"]:
    if (unit["library"], unit["file"]) != FAMILY[:2]:
        continue
    for function in unit.get("functions", []):
        if function["function"] != FAMILY[2]:
            continue
        for row in function["diagnostics"]:
            if row["category"] != "leak":
                continue
            match = SITE.search(row["description"])
            acquired = match.group(0) if match else "(unattributed)"
            text = source_line(FAMILY[0], match.group(1), int(match.group(2))) if match else ""
            print(
                f"{row['line']:7d} {str(row.get('origin')):26s} {acquired:32s} {text[:54]}"
            )

print()
print("C. The two hand-noted oracles of the wave-22/23 records, read straight out of")
print("   the pinned source: the declaration lines of the two destructors.")
path = find_source("fluent-bit-chunk", "flb_input_chunk.c")
if path is not None:
    lines = path.read_text(errors="replace").split("\n")
    for number in (2483, 2623):
        print(f"   flb_input_chunk.c:{number}  {lines[number - 1].strip()[:80]}")

print()
print("D. A deterministic cross-library sample: every 4,001st diagnostic row of the")
print("   run, with the pinned source line its coordinate names. A coordinate that")
print("   did not land would show as a blank or an unrelated line here.")
print()
print(f"{'unit':40s} {'L':>7s}  {'presented':30s} source line")
print("-" * 128)
ordinal = 0
for unit in report["units"]:
    for function in unit.get("functions", []):
        for row in function.get("diagnostics", []):
            ordinal += 1
            if ordinal % 4001:
                continue
            origin = row.get("origin")
            text = ""
            if origin:
                spelled, _, number = origin.rpartition(":")
                text = source_line(unit["library"], spelled, int(number))
            print(
                f"{unit['library'] + '/' + unit['file']:40s} {row['line']:7d}  "
                f"{str(origin):30s} {text[:52]}   [{row['category']}]"
            )

#!/usr/bin/env python3
"""preprocess.py OUT_DIR [ROOT_TAG] -- copied from LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED
(docs/reports/librdkafka-plugin-premise/scripts/) for LIBRDKAFKA-HEADER-INLINES,
with the checkout re-spelled canonically; otherwise unchanged. Run
`clang -E` over every librdkafka C unit of the pinned production compile
database (`66910c10`'s build), each with its own recorded command, so the
census's call graph (`reach.py`) sees every macro expanded (`rd_kafka_dbg`,
`rd_kafka_log`, `rd_rkb_dbg`, `rd_kafka_assert`, ...). The command's `-o`
and `-c` are dropped; nothing is written beside the read-only build. With
ROOT_TAG `046accf6`, the source path is rewritten to the prepared system
tree (`046accf6`) so the same units are preprocessed at that revision.
Exits per unit go to OUT_DIR/exits.json; nothing is piped. A reading aid for
the census, never evidence for a clause: the gateway reads Clang's AST."""
import concurrent.futures
import json
import pathlib
import shlex
import subprocess
import sys

BUILD = pathlib.Path("/media/Data/GitHub/quirin-lang/build/cycle19/c-gateway-wave-pin")
# The compile database spells the checkout through a symlink; every path is
# re-spelled to the canonical checkout before Clang runs (never the symlink).
SYMLINK = "/home/cosmo/Documents/GitHub/qcore-c"
CANONICAL = "/media/Data/GitHub/quirin-lang"
TREES = {
    "66910c10": "/media/Data/GitHub/quirin-lang/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source",
    "046accf6": "/media/Data/GitHub/quirin-lang/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source",
}


def command(entry, tag):
    args = shlex.split(entry["command"]) if "command" in entry else list(entry["arguments"])
    out, skip = [], False
    for arg in args:
        if skip:
            skip = False
            continue
        if arg in ("-o", "-MF", "-MT", "-MQ"):
            skip = True
            continue
        if arg in ("-c", "-MD", "-MMD"):
            continue
        out.append(arg.replace(SYMLINK, CANONICAL).replace(TREES["66910c10"], TREES[tag]))
    return out + ["-E"]


def main():
    out = pathlib.Path(sys.argv[1])
    tag = sys.argv[2] if len(sys.argv) > 2 else "66910c10"
    out.mkdir(parents=True, exist_ok=True)
    db = json.loads((BUILD / "fluent-bit-clang-production/compile_commands.json").read_text())
    units = [e for e in db if "/lib/librdkafka-2.15.0/src/" in e["file"] and e["file"].endswith(".c")]

    def run(entry):
        name = pathlib.Path(entry["file"]).name
        with open(out / (name + ".i"), "wb") as stdout, open(out / (name + ".err"), "wb") as stderr:
            rc = subprocess.run(command(entry, tag), cwd=entry["directory"].replace(SYMLINK, CANONICAL), stdout=stdout, stderr=stderr).returncode
        return name, rc

    with concurrent.futures.ThreadPoolExecutor(4) as pool:
        exits = dict(sorted(pool.map(run, units)))
    (out / "exits.json").write_text(json.dumps({"tag": tag, "units": len(units), "exits": exits}, indent=1) + "\n")
    bad = {k: v for k, v in exits.items() if v}
    print(f"{len(units)} units, {len(bad)} nonzero: {bad}")
    sys.exit(1 if bad else 0)


main()

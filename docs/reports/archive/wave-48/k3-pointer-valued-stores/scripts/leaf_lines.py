#!/usr/bin/env python3
"""Card K3: write the analysed `-E -P` text of every unit holding a re-census
candidate and print each candidate's IND/COND/RET leaf lines.

usage: leaf_lines.py REPORT_JSON COMPILE_COMMANDS RECENSUS_JSON OUT_DIR

Takes the compile record whose file ends with the unit's file, drops `-c`,
`-o X` and the dependency-file flags (`-MD`, `-MMD`, `-MF X`, `-MT X`, `-MQ X`)
so nothing is written beside the build, inserts `-E -P`, writes the text to
OUT_DIR, and compares its SHA-256 with the unit's source identity when the
report carries one. Read-only on the user's tree.
"""
import hashlib
import json
import pathlib
import shlex
import subprocess
import sys

SKIP_WITH_VALUE = {"-o", "-MF", "-MT", "-MQ"}
SKIP = {"-c", "-MD", "-MMD", "-MP"}


def preprocess(database, suffix, out):
    for entry in database:
        if not entry["file"].endswith(suffix):
            continue
        words = shlex.split(entry["command"]) if "command" in entry else list(entry["arguments"])
        argv, skip = [], False
        for word in words:
            if skip:
                skip = False
                continue
            if word in SKIP_WITH_VALUE:
                skip = True
                continue
            if word in SKIP:
                continue
            argv.append(word)
        argv[1:1] = ["-E", "-P"]
        result = subprocess.run(argv, cwd=entry["directory"], capture_output=True, text=True)
        out.write_text(result.stdout)
        return result.returncode
    return None


def main():
    report = json.load(open(sys.argv[1]))
    database = json.load(open(sys.argv[2]))
    recensus = json.load(open(sys.argv[3]))
    out_dir = pathlib.Path(sys.argv[4])
    out_dir.mkdir(parents=True, exist_ok=True)
    wanted = set(recensus["freed_alone"]) | {f["key"] for f in recensus["frozen_1107"]}
    wanted_short = {k.split("::", 1)[1] for k in wanted}
    texts = {}
    for unit in report["units"]:
        for function in unit.get("functions", []):
            short = f"{unit['file']}::{function['function']}"
            if short not in wanted_short or function["status"] != "Unknown":
                continue
            path = out_dir / (unit["file"].replace("/", "__") + ".i")
            if unit["file"] not in texts:
                code = preprocess(database, unit["file"], path)
                lines = path.read_text().splitlines() if code == 0 else []
                digest = hashlib.sha256(path.read_bytes()).hexdigest() if code == 0 else None
                texts[unit["file"]] = lines
                print(f"## unit {unit['file']} exit={code} sha256={digest} identity={unit.get('source_identity')}")
            lines = texts[unit["file"]]
            print(f"# {short}")
            for d in function["diagnostics"]:
                text = lines[d["line"] - 1].strip() if 0 < d["line"] <= len(lines) else "?"
                print(f"   {d['line']} [{d['description'][:40]}] {d.get('origin')} | {text[:160]}")


if __name__ == "__main__":
    main()

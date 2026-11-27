#!/usr/bin/env python3
"""Card LIBCURL-PROVIDER-PLUGIN: the probe's compile databases for the
opt-in run, cut from the prepared Fluent Bit build's database.

    python3 probe_database.py PREPARED-DATABASE OUT-DIR UNIT...

Writes OUT-DIR/selected.json (the commands of the named units only, each
naming its file by a path suffix) and OUT-DIR/summary.json (how many
commands the whole database has, how many pass the libcurl include
directory, and which units were selected). Nothing is rewritten: the
selected commands are the prepared ones, byte for byte as JSON values.
"""
import json
import sys
from pathlib import Path

database, out, units = Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3:]
entries = json.loads(database.read_text())


def text(entry):
    return entry.get("command") or " ".join(entry.get("arguments", []))


# The libcurl include directory CMake found (`CURL_INCLUDE_DIR`), passed as
# `-I DIR`, `-IDIR` or `-isystem DIR`.
curl_dirs = sorted({part.removeprefix("-I") for entry in entries for part in text(entry).split()
                    if "libcurl-dev" in part and "/include" in part})
selected = [entry for entry in entries if any(entry["file"].endswith(unit) for unit in units)]
out.mkdir(parents=True, exist_ok=True)
(out / "selected.json").write_text(json.dumps(selected, indent=1) + "\n")
summary = {
    "database": str(database),
    "commands": len(entries),
    "commands_passing_libcurl_include": sum(1 for entry in entries
                                            if any(d in text(entry) for d in curl_dirs)),
    "libcurl_include_directories": curl_dirs,
    "selected_units": sorted(entry["file"] for entry in selected),
}
(out / "summary.json").write_text(json.dumps(summary, indent=1) + "\n")
print(json.dumps(summary, indent=1))

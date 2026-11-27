#!/usr/bin/env python3
"""preprocess.py OUT_DIR SUFFIX... -- W43-ABI-ENUM-AND-CONST-PP: preprocess the
corpus units whose source path ends with one of SUFFIX (the production
compile database of the pinned `66910c10` build, read only) into OUT_DIR as
`.i` files, for `declared_abi_census` (a reading aid for the declarations'
C side; the arbiter is the measurement). Each command is the database's,
with `-c`/`-o OUTPUT` replaced by `-E -o OUT_DIR/NAME.i`; nothing under the
read-only build tree is written."""
import json
import pathlib
import shlex
import subprocess
import sys

DATABASE = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/"
                        "fluent-bit-clang-production/compile_commands.json")


def main() -> int:
    out = pathlib.Path(sys.argv[1]).resolve()
    out.mkdir(parents=True, exist_ok=True)
    wanted = sys.argv[2:]
    status = 0
    for entry in json.loads(DATABASE.read_text()):
        if not any(entry["file"].endswith(suffix) for suffix in wanted):
            continue
        words = shlex.split(entry["command"])
        command, skip = [], False
        for word in words:
            if skip:
                skip = False
                continue
            if word == "-o":
                skip = True
                continue
            if word == "-c":
                continue
            command.append(word)
        target = out / (pathlib.Path(entry["file"]).name + ".i")
        command += ["-E", "-o", str(target)]
        result = subprocess.run(command, cwd=entry["directory"], capture_output=True, text=True)
        print(entry["file"], result.returncode)
        status |= result.returncode
    return status


if __name__ == "__main__":
    sys.exit(main())

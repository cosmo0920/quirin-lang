#!/usr/bin/env python3
"""Card CURRENT-ACCESS-ALIAS-CENSUS: one census run of the user's `current` command.

Usage: run_census.py OUT_DIR SOURCE_COMMIT BINARY [plain]

W47-ALIAS-RESIDUALS-CENSUS's runner, moved from the 15-unit system selection
to the user's own 2026-11-06 `quirin-audit current` command: the whole
production compile database at fluent-bit 046accf6, no `--selection`, the
Monkey database and source report. BINARY is the `indirect-census` build; with
the fourth argument `plain` no census stream is switched on (the inertness
run, any build). Otherwise two streams are on: the access-path unit stream
(QCORE_ACCESS_PATH_CENSUS) and the typed-path parity stream
(QCORE_TYPED_PATH_PARITY). A census stream forces one unit at a time. The
output directory must not exist. Nothing under build/ is written: the report,
the Monkey source report and the logs go under OUT_DIR.
"""
import datetime
import hashlib
import json
import os
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
MAIN = pathlib.Path("/media/Data/GitHub/quirin-lang")
RUN = MAIN / "build/cycle19/clang/046accf6aded7906f66a1baf0202da349e521736/fluent-bit-clang-source"


def main() -> int:
    out = pathlib.Path(sys.argv[1]).resolve()
    commit, binary = sys.argv[2], pathlib.Path(sys.argv[3]).resolve()
    plain = len(sys.argv) > 4 and sys.argv[4] == "plain"
    out.mkdir(parents=True, exist_ok=False)
    checkout = RUN / "fluent-bit-clang-source"
    revision = subprocess.check_output(["git", "-C", str(checkout), "rev-parse", "HEAD"], text=True).strip()
    if revision != "046accf6aded7906f66a1baf0202da349e521736":
        raise SystemExit("prepared checkout revision mismatch")
    command = [str(binary), "current", "--checkout", str(checkout),
               "--profile-directory", str(ROOT / "contract/profiles/fluent-bit"),
               "--compile-database", str(RUN / "fluent-bit-clang-production/compile_commands.json"),
               "--monkey-database", str(RUN / "monkey/monkey-build/compile_commands.json"),
               "--monkey-source-report", str(out / "monkey-source.json"),
               "--report", str(out / "resource-diagnostics.json"),
               "--jobs", "8"]
    environment = {k: v for k, v in os.environ.items() if not k.startswith("QCORE_")}
    environment["QCORE_CLANG"] = "/usr/lib/llvm-21/bin/clang"
    if not plain:
        environment["QCORE_ACCESS_PATH_CENSUS"] = str(out / "access-path-units")
        environment["QCORE_TYPED_PATH_PARITY"] = str(out / "parity.jsonl")
    record = {"command": command, "source_commit": commit, "plain": plain,
              "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
              "environment": {k: v for k, v in environment.items() if k.startswith("QCORE_")},
              "start": datetime.datetime.now(datetime.timezone.utc).isoformat()}
    with open(out / "audit.log", "wb") as log:
        code = subprocess.run(command, cwd=ROOT, env=environment, stdout=log, stderr=log).returncode
    record["exit"] = code
    record["end"] = datetime.datetime.now(datetime.timezone.utc).isoformat()
    (out / "run.json").write_text(json.dumps(record, indent=2) + "\n")
    print(json.dumps(record, indent=2))
    return code


if __name__ == "__main__":
    sys.exit(main())

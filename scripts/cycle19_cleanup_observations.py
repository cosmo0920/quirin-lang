#!/usr/bin/env python3
"""Execute real pinned ChunkIO cleanup helpers with controlled syscall hooks.

No upstream file is modified and no OS descriptor/mapping is released. Sanitized
execution evidence covers only the listed scenarios, not whole-source safety.
"""
import argparse
import json
from pathlib import Path
import sys
sys.dont_write_bytecode = True
from cycle19_native_observations import PIN, REPO, build_and_run, profile, run, sha


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fluent-bit", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    root, output = args.fluent_bit.resolve(), args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    revision = run(["git", "-C", str(root), "rev-parse", "HEAD"], capture_output=True).stdout.strip()
    if revision != PIN:
        raise RuntimeError(f"expected {PIN}, found {revision}")
    # Check all tracked sources, not just the directly included helper body.
    if run(["git", "-C", str(root), "status", "--porcelain", "--untracked-files=no"], capture_output=True).stdout:
        raise RuntimeError("pinned source checkout has tracked modifications")
    database = root / "build/compile_commands.json"
    source = root / "lib/chunkio/src/cio_file_unix.c"
    entries = json.loads(database.read_text())
    entry = next(e for e in entries if Path(e["file"]).resolve() == source)
    flags = profile(entry) + [f'-DQCORE_FILE_CLEANUP_SOURCE="{source}"',
                              "-ffunction-sections", "-fdata-sections", "-Wl,--gc-sections"]
    artifact = build_and_run("file_cleanup_observation", entry, flags, [], output)
    if str(source) not in artifact["dependencies_sha256"]:
        raise RuntimeError("compiled harness did not include the actual selected source")
    observations = output / "file_cleanup_observation.jsonl"
    rows = [json.loads(line) for line in observations.read_text().splitlines()]
    expected = {"cio_file_native_close": {0: 1, 1: 1, 2: 3, 3: 3},
                "cio_file_native_unmap": {0: 1, 1: 1, 2: 4, 3: 4}}
    actual = {}
    for row in rows:
        counts = actual.setdefault(row["helper"], {})
        counts[row["mode"]] = counts.get(row["mode"], 0) + 1
    if actual != expected or len({row["case"] for row in rows}) != len(rows):
        raise RuntimeError(f"native scenario coverage mismatch: {actual}")
    contract = REPO / "contract/qcore/chunkio/filesystem_observation.qcore"
    checked = run(["cargo", "run", "--quiet", "-p", "qcore-compiler", "--example",
                   "check_contract_observations", "--", str(contract), str(observations)],
                  cwd=REPO, capture_output=True)
    (output / "contract_result.json").write_text(checked.stdout)
    metadata = {"fluent_bit_revision": revision, "database_sha256": sha(database),
                "clang": run(["clang", "--version"], capture_output=True).stdout,
                "native_execution": artifact, "contract_sha256": sha(contract),
                "harness_sha256": sha(REPO / "compiler/examples/file_cleanup_observation.c"),
                "driver_sha256": sha(Path(__file__)),
                "build_helper_sha256": sha(REPO / "scripts/cycle19_native_observations.py"),
                "scenario_counts": actual, "static_certificate": False,
                "os_cleanup_performed": False,
                "outcome_binding": "controlled close/munmap hooks return injected zero or minus one; diagnostic hook records errno"}
    (output / "metadata.json").write_text(json.dumps(metadata, indent=2) + "\n")
    print(checked.stdout.strip())


if __name__ == "__main__":
    main()

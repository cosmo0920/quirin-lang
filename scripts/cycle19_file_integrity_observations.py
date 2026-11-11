#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""Card O01: run the bounded ChunkIO reopen corruption/checksum fixture family.

This is preparation only. It extracts the pinned ChunkIO sources at the pinned
Fluent Bit commit into an exclusive tree, configures ChunkIO's own CMake with
Clang, rebuilds every selected translation unit under ASan/UBSan with libc
allocation and syscall wrappers, seeds eight exclusive fixtures and runs one
process per fixture.

The classification, custody accounting and refusals are NOT decided here: the
native observer writes canonical records and `qcore_core::chunk_integrity`
validates them. That validator is invoked through the
`validate_integrity_observations` example, whose output is then evaluated
against the declared `.qcore` observation policy.

The build configuration is ChunkIO's own standalone CMake, not Fluent Bit's;
that is a recorded premise, and the compiled source bytes are the pinned
upstream blobs, verified against the constants below.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import zlib

sys.dont_write_bytecode = True
from cycle19_native_observations import PIN, REPO, profile, run, sha

# Selected translation units. `tools/cio.c` is a command-line program and is
# deliberately excluded; every other unit ChunkIO's own build compiles is here.
UNITS = (
    "src/chunkio.c", "src/cio_chunk.c", "src/cio_error.c", "src/cio_file.c",
    "src/cio_file_unix.c", "src/cio_log.c", "src/cio_memfs.c", "src/cio_meta.c",
    "src/cio_os.c", "src/cio_scan.c", "src/cio_stats.c", "src/cio_stream.c",
    "src/cio_utils.c", "deps/crc32/crc32.c",
)

# Load-bearing pinned identities. A substituted checkout is refused, never
# compiled: `cio_file.c` owns the format check and its crc32 comparison,
# `crc32.c` owns the checksum itself, and `cio_file_st.h` owns the layout the
# fixtures are built from.
PINNED = {
    "src/cio_file.c": "1c3eb285307108deb2b22c68cf282c0121619a35bbe36adc216f276d51925257",
    "deps/crc32/crc32.c": "ce628f5a1a3f63bb17d144f77ee0595ff7869047fc70f59cd0a48853e00e0803",
    "include/chunkio/cio_file_st.h": "02eb39090bf82c246c1fc3b29f3b121259a066656956c8ac5fb2f4cbc13d9133",
}

SANITIZER = ("-UNDEBUG", "-g", "-O0", "-fsanitize=address,undefined",
             "-fno-omit-frame-pointer", "-fno-builtin",
             "-Werror=implicit-function-declaration")
WRAPPED = ("malloc", "calloc", "strdup", "realloc", "free",
           "open", "close", "chmod", "mmap", "munmap", "msync")
SCENARIOS = 8


def fixture_bytes(scenario):
    """Seed image for one fixture, built from the pinned 24-byte layout.

    Header: 0xC1 0x00, crc32(content) big endian at offset 2, content length
    big endian at offset 10, metadata length at offset 22. Content is the two
    metadata-length bytes plus the four payload bytes.
    """
    data = bytearray(4096)
    data[0] = 0xC1
    data[1] = 0x00
    data[10:14] = (4).to_bytes(4, "big")
    data[22:24] = (0).to_bytes(2, "big")
    data[24:28] = b"seed"
    data[2:6] = (zlib.crc32(bytes(data[22:28])) & 0xFFFFFFFF).to_bytes(4, "big")
    if scenario in (1, 6):
        data[24] = ord("S")            # payload edited, stored checksum stale
    if scenario == 2:
        data[0] = 0x00                 # magic byte destroyed
    if scenario == 3:
        return bytes(data[:8])         # header shorter than the 24-byte minimum
    if scenario == 4:
        data[10:14] = (4096).to_bytes(4, "big")  # declared content exceeds file
    return bytes(data)


def extract(root, output):
    """Copy the pinned ChunkIO subtree out of the checkout; never edit upstream."""
    source = output / "chunkio"
    if source.exists():
        shutil.rmtree(source)
    source.mkdir(parents=True)
    archive = subprocess.run(["git", "-C", str(root), "archive", PIN, "lib/chunkio"],
                             check=True, stdout=subprocess.PIPE)
    subprocess.run(["tar", "-x", "-C", str(source)], check=True, input=archive.stdout)
    source = source / "lib/chunkio"
    for relative, digest in PINNED.items():
        observed = sha(source / relative)
        if observed != digest:
            raise RuntimeError(f"pinned source mismatch for {relative}: {observed}")
    identities = {relative: sha(source / relative) for relative in UNITS}
    identities.update(PINNED)
    return source, identities


def configure(source, output):
    """ChunkIO's own standalone CMake, which generates `cio_info.h`."""
    build = output / "cmake"
    if build.exists():
        shutil.rmtree(build)
    log = run(["cmake", "-S", str(source), "-B", str(build), "-DCMAKE_C_COMPILER=clang",
               "-DCMAKE_EXPORT_COMPILE_COMMANDS=ON"], capture_output=True)
    (output / "configure.log").write_text(log.stdout + log.stderr)
    database = build / "compile_commands.json"
    entries = {Path(e["file"]).resolve(): e for e in json.loads(database.read_text())}
    missing = [u for u in UNITS if (source / u).resolve() not in entries]
    if missing:
        raise RuntimeError(f"ChunkIO build does not compile {missing}")
    return build, database, entries


def build(source, output):
    """Rebuild every selected unit with ChunkIO's own preprocessor inputs.

    Only the preprocessor arguments the recorded configure produced are reused;
    the sanitizer, optimization and instrumentation arguments are the harness's
    and are recorded as such.
    """
    build_dir, database, entries = configure(source, output)
    commands, objects = [], []
    for unit in UNITS:
        entry = entries[(source / unit).resolve()]
        obj = output / (Path(unit).stem + ".o")
        command = ["clang", *profile(entry), *SANITIZER, "-finstrument-functions",
                   "-c", str(source / unit), "-o", str(obj)]
        run(command, cwd=entry["directory"], capture_output=True)
        commands.append(command)
        objects.append(obj)
    observer = REPO / "compiler/examples/chunk_file_integrity_observation.c"
    binary = output / "chunk_file_integrity_observation"
    entry = entries[(source / "src/cio_file.c").resolve()]
    command = ["clang", *profile(entry), *SANITIZER, str(observer), *map(str, objects),
               "-Wl,--wrap=" + ",--wrap=".join(WRAPPED), "-lpthread", "-o", str(binary)]
    run(command, cwd=entry["directory"], capture_output=True)
    commands.append(command)
    return binary, database, commands


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fluent-bit", type=Path, required=True,
                        help="a checkout that contains the pinned commit")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if sys.platform != "linux":
        raise RuntimeError("requires the recorded Linux ABI")
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    source, identities = extract(args.fluent_bit.resolve(), output)
    binary, database, commands = build(source, output)

    environment = dict(os.environ, LC_ALL="C",
                       ASAN_OPTIONS="detect_leaks=1:halt_on_error=1",
                       UBSAN_OPTIONS="halt_on_error=1:print_stacktrace=1")
    lines = ["schema chunk_integrity_observation_v1",
             "identity revision=%s observer=%s binary=%s" % (PIN, sha(REPO / "compiler/examples/chunk_file_integrity_observation.c"), sha(binary))]
    processes = []
    for scenario in range(SCENARIOS):
        root = Path(tempfile.mkdtemp(prefix="qcore-profile-", dir="/tmp"))
        try:
            stream = root / "stream-0"
            stream.mkdir()
            path = stream / "chunk-0"
            path.write_bytes(fixture_bytes(scenario))
            path.chmod(0o600)
            before = path.stat()
            record = output / f"case-{scenario}.records"
            result = subprocess.run([str(binary), str(scenario), str(root), str(record)],
                                    capture_output=True, text=True, env=environment, timeout=120)
            (output / f"case-{scenario}.jsonl").write_text(result.stdout)
            (output / f"case-{scenario}.stderr").write_text(result.stderr)
            if result.returncode != 0:
                raise RuntimeError(f"scenario {scenario} exited {result.returncode}:\n{result.stderr}")
            lines.extend(record.read_text().splitlines())
            after = path.stat()
            processes.append(dict(case=400 + scenario, returncode=result.returncode,
                                  device=before.st_dev, inode=before.st_ino,
                                  inode_after=after.st_ino, size=before.st_size,
                                  seed_sha256=hashlib.sha256(fixture_bytes(scenario)).hexdigest(),
                                  final_sha256=hashlib.sha256(path.read_bytes()).hexdigest(),
                                  stderr=result.stderr))
        finally:
            shutil.rmtree(root, ignore_errors=True)
        processes[-1]["fixture_cleanup_complete"] = int(not root.exists())

    records = output / "records.txt"
    records.write_text("".join(line + "\n" for line in lines))
    (output / "processes.jsonl").write_text("".join(json.dumps(row) + "\n" for row in processes))

    observations = output / "observations.jsonl"
    validated = run(["cargo", "run", "--quiet", "-p", "qcore-compiler", "--example",
                     "validate_integrity_observations", "--", str(records), str(observations)],
                    cwd=REPO, capture_output=True)
    (output / "validation.json").write_text(validated.stdout)
    contract = REPO / "providers/qcore-provider-chunkio/contracts/chunkio/filesystem_integrity_observation.qv"
    checked = run(["cargo", "run", "--quiet", "-p", "qcore-compiler", "--example",
                   "check_contract_observations", "--", str(contract), str(observations)],
                  cwd=REPO, capture_output=True)
    (output / "contract_result.json").write_text(checked.stdout)

    metadata = {
        "schema_version": 1,
        "fluent_bit_revision": PIN,
        "pinned_source_sha256": identities,
        "build_configuration": "ChunkIO standalone CMake, Clang, export compile commands",
        "database_sha256": sha(database),
        "commands": commands,
        "clang": run(["clang", "--version"], capture_output=True).stdout,
        "cmake": run(["cmake", "--version"], capture_output=True).stdout.splitlines()[0],
        "observer_sha256": sha(REPO / "compiler/examples/chunk_file_integrity_observation.c"),
        "shared_observer_sha256": sha(REPO / "compiler/examples/native_file_profile_observer.h"),
        "memory_observer_sha256": sha(REPO / "compiler/examples/chunk_teardown_observation.c"),
        "driver_sha256": sha(Path(__file__)),
        "validator_sha256": sha(REPO / "crates/qcore-core/src/chunk_integrity.rs"),
        "validator_example_sha256": sha(REPO / "compiler/examples/validate_integrity_observations.rs"),
        "contract_sha256": sha(contract),
        "binary_sha256": sha(binary),
        "records_sha256": sha(records),
        "observations_sha256": sha(observations),
        "contract_result_sha256": sha(output / "contract_result.json"),
        "sanitizer_environment": {k: environment[k] for k in ("LC_ALL", "ASAN_OPTIONS", "UBSAN_OPTIONS")},
        "scope": "eight exclusive one-stream/one-chunk reopen fixtures under CIO_CHECKSUM; "
                 "four corrupted inputs, three injected operation failures, one clean control",
        "static_certificate": False,
        "durability_proved": False,
        "filesystem_exercised": True,
    }
    (output / "metadata.json").write_text(json.dumps(metadata, indent=2) + "\n")
    print(checked.stdout.strip())


if __name__ == "__main__":
    main()

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

"""Build native instrumentation around pinned Fluent Bit sources and evaluate QCore.

The checks are execution evidence, not whole-source safety certificates. No
upstream files are modified. Run from any directory; output must be disposable.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shlex
import subprocess

REPO = Path(__file__).resolve().parents[1]
PIN = "66910c10a4d7eafa810b84229cf2dcf7b0f26f97"


def run(command, **kwargs):
    return subprocess.run(command, check=True, text=True, **kwargs)


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def profile(entry):
    """Reuse preprocessor inputs, not source/output/optimization arguments."""
    args = shlex.split(entry["command"])
    flags = []
    i = 1
    while i < len(args):
        arg = args[i]
        if arg in ("-I", "-isystem", "-D", "-U", "-include", "-imacros", "-iquote"):
            value = args[i + 1]
            if "$" not in value:
                flags.extend((arg, value))
            i += 2
            continue
        if arg.startswith(("-I", "-D", "-U", "-std=")) and "$" not in arg:
            flags.append(arg)
        i += 1
    return flags


def build_and_run(name, entry, flags, libraries, output):
    binary = output / name
    command = ["clang", "-std=gnu99", *flags, "-UNDEBUG", "-g", "-O1",
               "-fsanitize=address,undefined", "-fno-omit-frame-pointer",
               "-Werror=implicit-function-declaration", "-MMD", "-MF", str(binary) + ".d",
               str(REPO / "compiler/examples" / (name + ".c")), *map(str, libraries),
               "-lpthread", "-o", str(binary)]
    run(command, cwd=entry["directory"])
    environment = dict(os.environ, ASAN_OPTIONS="detect_leaks=1:halt_on_error=1",
                       UBSAN_OPTIONS="halt_on_error=1:print_stacktrace=1",
                       QCORE_MONKEY_BINDINGS=str(output / "monkey_bindings.jsonl"))
    with (output / (name + ".jsonl")).open("w") as stdout, (output / (name + ".stderr")).open("w") as stderr:
        run([str(binary)], stdout=stdout, stderr=stderr, env=environment)
    # Clang depfiles escape spaces and continued lines; this checkout's paths
    # are parsed with shlex so generated and included header identities persist.
    deptext = Path(str(binary) + ".d").read_text().replace("\\\n", "")
    dependencies = shlex.split(deptext.split(":", 1)[1])
    hashes = {}
    for value in dependencies:
        path = Path(value)
        if not path.is_absolute():
            path = Path(entry["directory"]) / path
        hashes[str(path.resolve())] = sha(path)
    return {"compile_entry": entry, "command": command, "dependencies_sha256": hashes,
            "libraries_sha256": {str(p): sha(p) for p in libraries},
            "observations_sha256": sha(output / (name + ".jsonl")),
            "instrumentation": "sanitized harness and included source; linked static libraries retain their original build"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fluent-bit", type=Path, required=True)
    parser.add_argument("--monkey-build", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    root, output = args.fluent_bit.resolve(), args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    revision = run(["git", "-C", str(root), "rev-parse", "HEAD"], capture_output=True).stdout.strip()
    if revision != PIN:
        raise RuntimeError(f"expected {PIN}, found {revision}")
    run(["git", "-C", str(root), "diff", "--exit-code", "HEAD", "--",
         "include/fluent-bit/flb_mem.h", "include/fluent-bit/flb_macros.h", "lib/monkey"])
    database = root / "build/compile_commands.json"
    entries = json.loads(database.read_text())
    entry = next(e for e in entries if Path(e["file"]) == root / "src/flb_input_chunk.c")
    metadata = {"fluent_bit_revision": revision, "database_sha256": sha(database),
                "clang": run(["clang", "--version"], capture_output=True).stdout}
    metadata["flb_mem"] = build_and_run("flb_mem_observation", entry, profile(entry), [], output)
    expected_header = str(root / "include/fluent-bit/flb_mem.h")
    if expected_header not in metadata["flb_mem"]["dependencies_sha256"]:
        raise RuntimeError("compiler did not include the selected actual flb_mem.h")
    observations = output / "flb_mem_observation.jsonl"
    rows = [json.loads(line) for line in observations.read_text().splitlines()]
    counts = {name: sum(row["function"] == name for row in rows) for name in
              ("flb_malloc", "flb_calloc", "flb_realloc", "flb_realloc_z", "flb_free")}
    if list(counts.values()) != [10, 50, 40, 40, 5]:
        raise RuntimeError(f"missing native scenarios: {counts}")
    contract = REPO / "providers/quirin-provider-fluent-bit/contracts/fluent_bit/memory.qv"
    metadata["contract_sha256"] = sha(contract)
    checked = run(["cargo", "run", "--quiet", "-p", "qcore-compiler", "--example",
                   "check_contract_observations", "--", str(contract), str(observations)],
                  cwd=REPO, capture_output=True)
    (output / "flb_mem_contract_result.json").write_text(checked.stdout)
    print(checked.stdout.strip())
    if args.monkey_build:
        build = args.monkey_build.resolve()
        db = build / "compile_commands.json"
        monkey = next(e for e in json.loads(db.read_text()) if e["file"].endswith("/mk_core/mk_event.c"))
        source = Path(monkey["file"]).parent.parent
        # Verify every copied non-generated source/header used by the harness
        # after compilation as well as the adapter before execution.
        for file in ("mk_event.c", "mk_event_libevent.c"):
            if sha(source / "mk_core" / file) != sha(root / "lib/monkey/mk_core" / file):
                raise RuntimeError(f"Monkey copied source mismatch: {file}")
        flags = profile(monkey) + ['-DQCORE_MONKEY_SOURCE="' + monkey["file"] + '"']
        metadata["monkey"] = build_and_run("monkey_observation", monkey, flags,
                  [build / "library/libmk_core.a", build / "library/libevent.a"], output)
        for path in metadata["monkey"]["dependencies_sha256"]:
            path = Path(path)
            if path.is_relative_to(source):
                upstream = root / "lib/monkey" / path.relative_to(source)
                if not upstream.is_file() or sha(path) != sha(upstream):
                    raise RuntimeError(f"Monkey dependency source mismatch: {path}")
        observation_path = output / "monkey_observation.jsonl"
        checked = run(["cargo", "run", "--quiet", "-p", "qcore-compiler", "--example",
                       "check_monkey_observations", "--", str(observation_path)],
                      cwd=REPO, capture_output=True)
        (output / "monkey_identity_result.json").write_text(checked.stdout)
        report = json.loads(checked.stdout)["checked"]
        if report["callbacks"] != ["cb_event", "cb_timeout", "cb_wait_2_timeout"]:
            raise RuntimeError(f"unobserved callbacks: {report['callbacks']}")
        if report["socketpair_failures"] != 1 or report["descriptor_acquisitions"] != 6:
            raise RuntimeError(f"missing descriptor creation/failure scenarios: {report}")
        # Import fresh private source summaries under the same checked build;
        # serialized symbolic reports are never accepted as trusted summaries.
        source_report = output / "monkey_callback_source.json"
        diagnostics = output / "resource_diagnostics.json"
        with (output / "source-audit.log").open("w") as audit:
            run(["cargo", "run", "--quiet", "-p", "qcore-audit", "--",
                 "current", "--checkout", str(root), "--profile-directory",
                 str(REPO / "contract/profiles/fluent-bit"), "--monkey-database", str(db),
                 "--monkey-source-report", str(source_report), "--report", str(diagnostics),
                 "--binding-directory", str(output)],
                cwd=REPO, stdout=audit, stderr=subprocess.STDOUT)
        binding_result = json.loads((output / "monkey_binding_result.json").read_text())
        if (any(row["checked_executions"] < 1 or row["unknown_paths"] or row["unknown_reasons"]
                for row in binding_result["callbacks"])
                or sum(row["injected_executions"] for row in binding_result["callbacks"]) != 1):
            raise RuntimeError(f"callback binding regression: {binding_result}")
        metadata["monkey_binding"] = {
            "schema_version": 1,
            "replay_contract_sha256": sha(REPO / "providers/qcore-provider-libevent/contracts/monkey/callback_binding.qv"),
            "identity_contract_sha256": sha(REPO / "providers/qcore-provider-libevent/contracts/monkey/descriptor_observation.qv"),
            "controlled_scenarios": ["event_active flags: write, read|write, timeout", "one injected send failure with ECONNABORTED"],
            "database_sha256": sha(db),
            "contract_sha256": sha(REPO / "providers/qcore-provider-libevent/contracts/monkey/composition.qv"),
            "bindings_sha256": sha(output / "monkey_bindings.jsonl"),
            "source_report_sha256": sha(source_report),
            "result_sha256": sha(output / "monkey_binding_result.json")}
        metadata["monkey_result"] = {"identity_projection": report,
            "callback_binding": binding_result, "static_certificate": False}
        print(json.dumps(metadata["monkey_result"]))
    (output / "metadata.json").write_text(json.dumps(metadata, indent=2) + "\n")


if __name__ == "__main__":
    main()

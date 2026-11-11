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

"""Observe eleven native filesystem allocation failures plus a control on Linux."""
import argparse
import json
import os
from pathlib import Path
import shutil
import stat
import subprocess
import sys
import tempfile
sys.dont_write_bytecode = True
from cycle19_native_observations import REPO, run, sha
from cycle19_teardown_observations import build
from check_file_constructor_failure_trace import check_trace, load_trace


def fixture_identity(path):
    info = path.lstat()
    if not stat.S_ISDIR(info.st_mode): raise RuntimeError("fixture root is no longer a directory")
    return [info.st_dev,info.st_ino]


def fixture_entries(root):
    entries = []
    for path in sorted(root.rglob("*")):
        info = path.lstat()
        kind = "directory" if stat.S_ISDIR(info.st_mode) else "file" if stat.S_ISREG(info.st_mode) else "unsupported"
        entries.append(dict(relative=str(path.relative_to(root)),kind=kind,device=info.st_dev,inode=info.st_ino))
    return entries


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fluent-bit",type=Path,required=True)
    parser.add_argument("--output",type=Path,required=True)
    args = parser.parse_args()
    if sys.platform != "linux": raise RuntimeError("requires the recorded Linux ABI")
    output = args.output.resolve()
    output.mkdir(parents=True,exist_ok=True)
    binary,metadata = build(args.fluent_bit.resolve(),output,"chunk_file_constructor_failure_observation",
                            ("close","munmap","unlink","msync"))
    environment = dict(os.environ,LC_ALL="C",ASAN_OPTIONS="detect_leaks=1:halt_on_error=1",
                       UBSAN_OPTIONS="halt_on_error=1:print_stacktrace=1")
    records,processes = [],[]
    for fail in range(12):
        # Fresh root per process. Prefix fixes allocation sizes in the ABI profile.
        root = Path(tempfile.mkdtemp(prefix="qcore-failure-",dir="/tmp"))
        try:
            if len(str(root)) != 27: raise RuntimeError("unexpected fixture root length")
            before = fixture_identity(root)
            completed = subprocess.run([str(binary),str(fail),str(root)],capture_output=True,text=True,
                                       env=environment,timeout=30)
            trace = output / f"case-{fail}.jsonl"
            trace.write_text(completed.stdout)
            (output/f"case-{fail}.stderr").write_text(completed.stderr)
            process = dict(case=100+fail,returncode=completed.returncode,stderr=completed.stderr,
                           root_before=before,root_after=fixture_identity(root),entries=fixture_entries(root))
            records.extend(load_trace(trace))
        finally:
            # Harness-owned fixtures only. Process termination is never recorded
            # as an observed C release of its outstanding allocation generations.
            shutil.rmtree(root)
        process["fixture_cleanup_complete"] = int(not root.exists())
        processes.append(process)
    trace = output/"trace.jsonl"
    trace.write_text("".join(json.dumps(row)+"\n" for row in records))
    process_path = output/"processes.jsonl"
    process_path.write_text("".join(json.dumps(row)+"\n" for row in processes))
    observations = check_trace(records,processes)
    for row in observations: row["values"]["compiled_source_and_build_recorded"] = 1
    observations_path = output/"observations.jsonl"
    observations_path.write_text("".join(json.dumps(row)+"\n" for row in observations))
    contract = REPO/"providers/qcore-provider-chunkio/contracts/chunkio/filesystem_constructor_failure_observation.qv"
    checked = run(["cargo","run","--quiet","-p","qcore-compiler","--example","check_contract_observations",
                   "--",str(contract),str(observations_path)],cwd=REPO,capture_output=True)
    (output/"contract_result.json").write_text(checked.stdout)
    for key,path in {"failure_driver":Path(__file__),"trace_checker":REPO/"scripts/check_file_constructor_failure_trace.py",
                     "layout_profile":REPO/"scripts/check_file_teardown_trace.py",
                     "trace_loader":REPO/"scripts/check_teardown_trace.py","contract":contract,
                     "trace":trace,"processes":process_path,"observations":observations_path,
                     "contract_result":output/"contract_result.json"}.items():
        metadata[key+"_sha256"] = sha(path)
    metadata.update(filesystem_exercised=True,durability_proved=False,
                    sanitizer_environment={k:environment[k] for k in ("LC_ALL","ASAN_OPTIONS","UBSAN_OPTIONS")},
                    scope="one ENOMEM per constructor allocation on an empty one-stream/one-file Linux fixture, plus control; cases 3 and 9 terminate under UBSan and retain incomplete prefix ledgers",
                    completed_executions=10,interrupted_executions=2,
                    leak_check_boundary="ten returning cases complete ASan/UBSan/LSan; no leak-check or teardown completion claim for two UBSan-terminated processes")
    (output/"metadata.json").write_text(json.dumps(metadata,indent=2)+"\n")
    print(checked.stdout.strip())


if __name__ == "__main__": main()

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

"""Observe bounded constructor OS failures and rollback descriptor custody."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
sys.dont_write_bytecode = True
from cycle19_native_observations import REPO, run, sha
from cycle19_teardown_observations import build
from cycle19_file_constructor_failure_observations import fixture_identity, fixture_entries
from check_teardown_trace import load_trace

WRAPPERS = ("open","fstat","stat","chmod","posix_fallocate","ftruncate","mmap","close","munmap","unlink","msync")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fluent-bit",type=Path,required=True)
    parser.add_argument("--output",type=Path,required=True)
    args = parser.parse_args()
    if sys.platform != "linux": raise RuntimeError("requires the recorded Linux ABI")
    output = args.output.resolve(); output.mkdir(parents=True,exist_ok=True)
    binary,metadata = build(args.fluent_bit.resolve(),output,"chunk_file_os_constructor_observation",WRAPPERS)
    environment = dict(os.environ,LC_ALL="C",ASAN_OPTIONS="detect_leaks=1:halt_on_error=1",
                       UBSAN_OPTIONS="halt_on_error=1:print_stacktrace=1")
    records,processes = [],[]
    for scenario in range(8):
        root = Path(tempfile.mkdtemp(prefix="qcore-os-open-",dir="/tmp"))
        try:
            if len(str(root)) != 27: raise RuntimeError("unexpected root length")
            before = fixture_identity(root)
            result = subprocess.run([str(binary),str(scenario),str(root)],capture_output=True,text=True,
                                    env=environment,timeout=30)
            trace = output/f"case-{scenario}.jsonl"; trace.write_text(result.stdout)
            (output/f"case-{scenario}.stderr").write_text(result.stderr)
            process = dict(case=200+scenario,returncode=result.returncode,stderr=result.stderr,
                root_before=before,root_after=fixture_identity(root),entries=fixture_entries(root))
            records.extend(load_trace(trace))
        finally: shutil.rmtree(root)
        process["fixture_cleanup_complete"] = int(not root.exists())
        processes.append(process)
    for name,rows in (("trace",records),("processes",processes)):
        (output/(name+".jsonl")).write_text("".join(json.dumps(row)+"\n" for row in rows))
    from check_file_os_constructor_trace import check_trace
    observations = check_trace(records,processes)
    for row in observations: row["values"]["compiled_source_and_build_recorded"] = 1
    observed = output/"observations.jsonl"
    observed.write_text("".join(json.dumps(row)+"\n" for row in observations))
    contract = REPO/"providers/qcore-provider-chunkio/contracts/chunkio/filesystem_os_constructor_observation.qv"
    checked = run(["cargo","run","--quiet","-p","qcore-compiler","--example","check_contract_observations",
                   "--",str(contract),str(observed)],cwd=REPO,capture_output=True)
    (output/"contract_result.json").write_text(checked.stdout)
    for key,path in {"os_driver":Path(__file__),"trace_checker":REPO/"scripts/check_file_os_constructor_trace.py",
                     "shared_trace_checker":REPO/"scripts/check_file_constructor_failure_trace.py",
                     "fixture_driver":REPO/"scripts/cycle19_file_constructor_failure_observations.py",
                     "layout_profile":REPO/"scripts/check_file_teardown_trace.py",
                     "trace_loader":REPO/"scripts/check_teardown_trace.py","contract":contract,
                     "trace":output/"trace.jsonl","processes":output/"processes.jsonl",
                     "observations":observed,"contract_result":output/"contract_result.json"}.items():
        metadata[key+"_sha256"] = sha(path)
    metadata.update(filesystem_exercised=True,durability_proved=False,
        sanitizer_environment={k:environment[k] for k in ("LC_ALL","ASAN_OPTIONS","UBSAN_OPTIONS")},
        scope="eight empty one-file Linux constructor executions; suppressed open, size, resize and map calls; one map failure followed by suppressed rollback close; original OS identities probed before/after parent release",
        failure_boundary="suppressed syscall failures establish only injected post-states, not general kernel failure semantics; no static certificate")
    (output/"metadata.json").write_text(json.dumps(metadata,indent=2)+"\n")
    print(checked.stdout.strip())


if __name__ == "__main__": main()

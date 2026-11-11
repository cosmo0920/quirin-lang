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

"""Observe bounded native unmap failures and recovery on existing fixtures."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
sys.dont_write_bytecode=True
from cycle19_native_observations import REPO,run,sha
from cycle19_teardown_observations import build
from cycle19_file_constructor_failure_observations import fixture_identity,fixture_entries
from check_teardown_trace import load_trace


def seed_bytes(scenario):
    data=bytearray(4096);data[0]=0xc1;data[13]=4;data[24:28]=b"seed"
    return bytes(data)


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--fluent-bit',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    if sys.platform!='linux':raise RuntimeError('requires Linux ABI')
    out=args.output.resolve();out.mkdir(parents=True,exist_ok=True)
    binary,metadata=build(args.fluent_bit.resolve(),out,'chunk_file_unmap_failure_observation',('open','close','chmod','mmap','munmap','msync'))
    env=dict(os.environ,LC_ALL='C',ASAN_OPTIONS='detect_leaks=1:halt_on_error=1',UBSAN_OPTIONS='halt_on_error=1:print_stacktrace=1')
    records=[];processes=[]
    for scenario in range(4):
        root=Path(tempfile.mkdtemp(prefix='qcore-profile-',dir='/tmp'))
        try:
            stream=root/'stream-0';stream.mkdir();path=stream/'chunk-0'
            path.write_bytes(seed_bytes(0));path.chmod(0o600)
            before=fixture_identity(root);initial=path.stat()
            result=subprocess.run([str(binary),str(scenario),str(root)],capture_output=True,text=True,env=env,timeout=30)
            trace=out/f'case-{scenario}.jsonl';trace.write_text(result.stdout)
            (out/f'case-{scenario}.stderr').write_text(result.stderr)
            records.extend(load_trace(trace))
            process=dict(case=600+scenario,returncode=result.returncode,stderr=result.stderr,root_before=before,
                root_after=fixture_identity(root),entries=fixture_entries(root),initial_device=initial.st_dev,
                initial_inode=initial.st_ino,initial_hex=seed_bytes(0).hex(),final_hex=path.read_bytes().hex())
        finally:shutil.rmtree(root)
        process['fixture_cleanup_complete']=int(not root.exists());processes.append(process)
    for name,rows in [('trace',records),('processes',processes)]:
        (out/(name+'.jsonl')).write_text(''.join(json.dumps(row)+'\n' for row in rows))
    from check_file_unmap_failure_trace import check_trace
    observations=check_trace(records,processes)
    for row in observations:row['values']['compiled_source_and_build_recorded']=1
    obs=out/'observations.jsonl';obs.write_text(''.join(json.dumps(row)+'\n' for row in observations))
    contract=REPO/'providers/qcore-provider-chunkio/contracts/chunkio/filesystem_unmap_failure_observation.qv'
    checked=run(['cargo','run','--quiet','-p','qcore-compiler','--example','check_contract_observations','--',str(contract),str(obs)],cwd=REPO,capture_output=True)
    (out/'contract_result.json').write_text(checked.stdout)
    for key,path in {'profile_driver':Path(__file__),'trace_checker':REPO/'scripts/check_file_unmap_failure_trace.py',
                     'shared_checker':REPO/'scripts/native_file_profile_trace.py',
                     'existing_process_checker':REPO/'scripts/check_file_existing_trace.py',
                     'fixture_driver':REPO/'scripts/cycle19_file_constructor_failure_observations.py',
                     'trace_loader':REPO/'scripts/check_teardown_trace.py','contract':contract,
                     'trace':out/'trace.jsonl','processes':out/'processes.jsonl','observations':obs,
                     'contract_result':out/'contract_result.json'}.items():metadata[key+'_sha256']=sha(path)
    metadata.update(filesystem_exercised=True,static_certificate=False,durability_proved=False,
        scope='four fixed existing-file failed-down sequences; one suppressed munmap EIO each, two with suppressed msync EIO',
        sanitizer_environment={k:env[k] for k in ('LC_ALL','ASAN_OPTIONS','UBSAN_OPTIONS')})
    (out/'metadata.json').write_text(json.dumps(metadata,indent=2)+'\n');print(checked.stdout.strip())


if __name__=='__main__':main()

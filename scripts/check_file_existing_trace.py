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

"""Replay seven pinned existing-file profiles; corruption and OS custody stay distinct."""
import errno
import re
from check_teardown_trace import require, load_trace
from check_file_constructor_failure_trace import canonical
from native_file_profile_trace import Replay


def fixture_bytes(scenario):
    # Independent specification of the 24-byte header plus four-byte payload.
    header = bytes([0xc1 if scenario != 3 else 0]) + bytes(12) + bytes([4]) + bytes(10)
    data = header + b'seed' + bytes(4068)
    return b'' if scenario == 5 else data[:8] if scenario == 4 else data


def check_process(p, scenario):
    require(set(p) == {'case','returncode','stderr','root_before','root_after','entries',
            'initial_device','initial_inode','initial_hex','final_hex','fixture_cleanup_complete'}, 'process schema')
    require(p['case'] == 300+scenario and p['returncode'] == 0 and p['fixture_cleanup_complete'] == 1,
            'process identity/completion mismatch')
    before = p['root_before']
    require(len(before) == 2 and all(0 < v < 2**64 for v in before) and before == p['root_after'], 'root identity')
    entries = p['entries']
    require([(e['relative'],e['kind']) for e in entries] == [('stream-0','directory'),('stream-0/chunk-0','file')],
            'fixture tree mismatch')
    identities = [tuple(before)]
    for e in entries:
        require(set(e) == {'relative','kind','device','inode'}, 'entry schema')
        ident = (e['device'],e['inode'])
        require(ident[0] == before[0] and 0 < ident[1] < 2**64 and ident not in identities, 'aliased inode')
        identities.append(ident)
    require((p['initial_device'],p['initial_inode']) == identities[-1], 'file replaced')
    require(p['initial_hex'] == p['final_hex'] == fixture_bytes(scenario).hex(), 'file content changed')
    expected = [(305,1,'Operation not permitted')] if scenario in (1,2) else []
    if scenario == 2: expected.append((431,5,'Input/output error'))
    lines = p['stderr'].splitlines()
    require(len(lines) == len(expected), 'unexpected native/sanitizer diagnostics')
    for line,(number,code,message) in zip(lines,expected):
        require(re.fullmatch(r'\[/[^\n]+/lib/chunkio/src/'+re.escape(f'cio_file_unix.c:{number} errno={code}] {message}'),line),
                'native diagnostic mismatch')
    return identities[-1]


def check_case(records, process, scenario):
    device,inode = check_process(process,scenario)
    r = Replay(records,300+scenario)
    success, mapped = scenario in (0,6), scenario in (0,3,4,6)
    size = len(fixture_bytes(scenario))
    r.consume('begin_existing',schema_version=1,root_length=27,device=device,inode=inode)
    r.allocations()
    fd = r.peek('result')
    require(type(fd) is int and 0 <= fd < 2**31, 'invalid descriptor')
    r.consume('open',phase='constructor',path=9,flags=0 if scenario >= 5 else 66,
              mode=0 if scenario >= 5 else 384,result=fd,errno=0,epoch=1)
    r.consume('chmod',phase='constructor',path=9,mode=448,result=-1 if scenario in (1,2) else 0,
              errno=errno.EPERM if scenario in (1,2) else 0,injected=int(scenario in (1,2)))
    mapping = 0
    if mapped:
        mapping = r.peek('result'); r.separated(mapping,size)
        require(mapping % 4096 == 0, 'unaligned mapping')
        r.consume('map',phase='constructor',address=0,size=size,prot=1 if scenario == 6 else 3,
                  flags=1,fd=fd,fd_epoch=1,offset=0,result=mapping,errno=0,epoch=1)
    if not success:
        if mapped: r.unmap(mapping,size,'constructor')
        r.close(fd,'constructor',scenario == 2)
        r.backend_release(fd if scenario == 2 else -1,size)
        r.release(8)
        r.consume('rollback_chunk',id=7,error={3:-11,4:-13,5:-12}.get(scenario,0))
        r.release(7)
    r.consume('constructor_result',chunk=7 if success else 0,error=0 if success else -1 if scenario in (1,2) else -3)
    def probe(label):
        present = scenario == 2 or (success and label == 'returned')
        map_present = success and label == 'returned'
        r.consume('probe',label=label,fd=fd,fd_epoch=1,fd_result=0 if present else -1,
            fd_errno=0 if present else errno.EBADF,fd_device=device if present else 0,fd_inode=inode if present else 0,
            map=mapping,map_epoch=int(mapped),map_result=0 if map_present else -1 if mapped else -999,
            map_errno=errno.ENOMEM if mapped and not map_present else 0,size=size,
            mode=384 if scenario in (1,2) else 448,device=device,inode=inode)
    probe('returned')
    chunk = dict(id=7,name=8,context=1,stream=5,locked=0,transaction=0,tx_length=0,error=0,
        backend=10,path=9,fd=fd,fd_epoch=1,map=mapping,map_epoch=1,size=4096,fs_size=4096,
        data_size=4,synced=1,header_length=4,content=b'seed'.hex()) if success else None
    r.snapshot(chunk)
    def cleanup():
        r.enter('cio_file_native_unmap'); r.unmap(mapping,size,'teardown'); r.leave('cio_file_native_unmap')
        r.enter('cio_chunk_counter_total_up_sub'); r.leave('cio_chunk_counter_total_up_sub')
        r.enter('cio_file_native_close'); r.close(fd,'teardown'); r.leave('cio_file_native_close')
        r.backend_release(-1,size)
    r.parents(cleanup if success else None)
    probe('parents_released'); r.consume('end',live_allocations=0)
    if scenario == 2:
        r.consume('harness_close',fd=fd,epoch=1,result=0,errno=0,fd_result=-1,fd_errno=errno.EBADF)
    r.finish()
    return {'function':'native_file_existing_observation','case':300+scenario,'values':{
        'compiled_source_and_build_recorded':0,'exclusive_instrumented_fixture':1,
        'exact_constructor_and_rollback':1,'exact_parent_teardown':1,'os_identities_bound':1,
        'allocations':10,'releases':10,'transient_releases':1,'rollback_releases':0 if success else 4,
        'teardown_releases':9 if success else 5,'chunk_null':int(not success),
        'injected_failures':2 if scenario == 2 else int(scenario == 1),'acl_failed':int(scenario in (1,2)),
        'corruption_result':int(scenario in (3,4,5)), 'readonly':int(scenario >= 5),
        'descriptor_residual_after_return':int(scenario == 2),'descriptor_residual_after_parents':int(scenario == 2),
        'harness_descriptor_reclaimed':int(scenario == 2),'original_inode_and_bytes_retained':1,
        'fixture_cleanup_complete':1,'static_certificate':0,'durability_proved':0}}


def check_trace(records,processes):
    canonical(records); canonical(processes)
    require(len(processes) == 7, 'incomplete process matrix')
    groups, last = [[] for _ in range(7)], 300
    for row in records:
        case = row.get('case')
        require(type(case) is int and last <= case < 307, 'unknown/out-of-order case')
        groups[case-300].append(row); last = case
    return [check_case(rows,p,n) for n,(rows,p) in enumerate(zip(groups,processes))]

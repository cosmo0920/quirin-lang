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

"""Replay failed down operations while preserving live mappings, rings and descriptors."""
import errno
import re
from check_teardown_trace import require, load_trace
from check_file_constructor_failure_trace import canonical
from check_file_existing_trace import check_process as check_existing_process, fixture_bytes
from native_file_profile_trace import Replay


def check_process(p, scenario):
    require(p.get('case') == 600+scenario, 'transition process identity')
    expected = bytearray(fixture_bytes(0))
    if scenario != 0:
        expected[13] = 7; expected[28:31] = b'add'
    require(p.get('final_hex') == expected.hex(), 'final native bytes mismatch')
    # Reuse the strict unchanged-fixture schema/identity/diagnostics checks after
    # separately validating this profile's case number and final bytes above.
    lines = p.get('stderr', '').splitlines()
    expected_lines = [491, 62] if scenario >= 2 else [62]
    require(len(lines) == len(expected_lines), 'unexpected native/sanitizer diagnostics')
    for line, number in zip(lines, expected_lines):
        require(re.fullmatch(r'\[/[^\n]+/lib/chunkio/src/cio_file_unix.c:' +
                str(number) + r' errno=5] Input/output error', line), 'native diagnostic mismatch')
    return check_existing_process({**p,'case':300,'final_hex':p.get('initial_hex'),'stderr':''},0)


def check_case(records, process, scenario):
    device,inode = check_process(process,scenario)
    r = Replay(records,600+scenario)
    r.consume('begin_unmap_failure',schema_version=1,root_length=27,device=device,inode=inode)
    r.allocations()
    fd, mapping, epoch = -1, 0, 0
    up, length, header, synced, transaction, tx_length, locked = True, 4, 4, 1, 0, 0, 0
    def open_map(phase):
        nonlocal fd,mapping,epoch
        epoch += 1
        fd = r.peek('result')
        require(type(fd) is int and 0 <= fd < 2**31, 'invalid descriptor')
        r.consume('open',phase=phase,path=9,flags=66,mode=384,result=fd,errno=0,epoch=epoch)
        if phase == 'constructor':
            r.consume('chmod',phase=phase,path=9,mode=448,result=0,errno=0,injected=0)
        mapping = r.peek('result'); r.separated(mapping,4096)
        require(mapping % 4096 == 0, 'unaligned mapping')
        r.consume('map',phase=phase,address=0,size=4096,prot=3,flags=1,fd=fd,fd_epoch=epoch,
                  offset=0,result=mapping,errno=0,epoch=epoch)
    def probe(label,present):
        r.consume('probe',label=label,fd=fd,fd_epoch=epoch,fd_result=0 if present else -1,
            fd_errno=0 if present else errno.EBADF,fd_device=device if present else 0,fd_inode=inode if present else 0,
            map=mapping,map_epoch=epoch,map_result=0 if present else -1,map_errno=0 if present else errno.ENOMEM,
            size=4096,mode=448,device=device,inode=inode)
    def snapshot():
        r.snapshot(dict(id=7,name=8,context=1,stream=5,locked=locked,transaction=transaction,
            tx_length=tx_length,error=0,backend=10,path=9,fd=fd if up else -1,fd_epoch=epoch,
            map=mapping if up else 0,map_epoch=epoch,size=4096 if up else 0,fs_size=4096,
            data_size=length if up else 0,synced=synced,header_length=header if up else 0,
            content=b'seedadd'[:length].hex() if up else ''),up)
    def operation(name,result=0):
        r.consume('operation',name=name,result=result); probe(name,up); snapshot()
    def sync(phase, failed=False):
        nonlocal synced
        if failed: r.consume('suppressed_sync',phase=phase,epoch=epoch,remaining=0)
        r.consume('sync',phase=phase,map=mapping,size=4096,epoch=epoch,flags=1,
            result=-1 if failed else 0,errno=errno.EIO if failed else 0)
        if not failed: synced = 1
    def down():
        nonlocal up,length
        if not synced: sync('transition')
        r.unmap(mapping,4096,'transition',epoch); r.close(fd,'transition',epoch=epoch)
        up = False; length = 0
        operation('down_retry')
    def restore():
        nonlocal up,length,synced
        open_map('transition'); up = True; length = header; synced = 1
        operation('up')
    open_map('constructor')
    r.consume('constructor_result',chunk=7,error=0); probe('returned',True); snapshot()
    if scenario != 0:
        length = header = 7; synced = 0; operation('append')
    r.consume('arm_unmap_failure',unmap_count=1,sync_count=int(scenario >= 2))
    if not synced: sync('transition',scenario >= 2)
    r.consume('suppressed_unmap',phase='transition',epoch=epoch,remaining=0)
    r.consume('unmap',phase='transition',map=mapping,size=4096,epoch=epoch,result=-1,errno=errno.EIO)
    operation('down_failed',-1); operation('up_still_mapped',-1)
    if scenario != 3: down(); restore()
    def cleanup():
        if not synced:
            r.enter('cio_file_sync'); r.enter('cio_file_native_sync'); sync('teardown')
            r.leave('cio_file_native_sync'); r.leave('cio_file_sync')
        r.enter('cio_file_native_unmap'); r.unmap(mapping,4096,'teardown',epoch); r.leave('cio_file_native_unmap')
        r.enter('cio_chunk_counter_total_up_sub'); r.leave('cio_chunk_counter_total_up_sub')
        r.enter('cio_file_native_close'); r.close(fd,'teardown',epoch=epoch); r.leave('cio_file_native_close')
        r.backend_release(-1,4096)
    r.parents(cleanup); probe('parents_released',False); r.consume('end',live_allocations=0); r.finish()
    return {'function':'native_file_unmap_failure_observation','case':600+scenario,'values':{
        'compiled_source_and_build_recorded':0,'exclusive_instrumented_fixture':1,
        'exact_native_transitions':1,'exact_parent_teardown':1,'os_epochs_and_rings_bound':1,
        'allocations':10,'releases':10,'transient_releases':1,'teardown_releases':9,
        'open_epochs':epoch,'mapping_epochs':epoch,'suppressed_unmap_calls':1,
        'suppressed_sync_calls':int(scenario >= 2),'failed_down_retains_mapping_and_descriptor':1,
        'failed_down_retains_up_ring_and_counter':1,'up_while_still_mapped_rejected':1,
        'successful_sync_before_failed_unmap':int(scenario == 1),
        'retry_down_releases_original_epoch':int(scenario != 3),
        'parent_teardown_recovers_original_epoch':int(scenario == 3),
        'original_inode_retained':1,'fixture_cleanup_complete':1,
        'static_certificate':0,'durability_proved':0}}


def check_trace(records,processes):
    canonical(records); canonical(processes)
    require(len(processes) == 4, 'incomplete unmap-failure process matrix')
    groups, last = [[] for _ in range(4)], 600
    for row in records:
        case = row.get('case')
        require(type(case) is int and last <= case < 604, 'unknown/out-of-order case')
        groups[case-600].append(row); last = case
    return [check_case(rows,p,n) for n,(rows,p) in enumerate(zip(groups,processes))]

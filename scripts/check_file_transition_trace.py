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

"""Replay fixed native transitions without equating logical and mapped lengths."""
import errno
from check_teardown_trace import require, load_trace
from check_file_constructor_failure_trace import canonical
from check_file_existing_trace import check_process as check_existing_process, fixture_bytes
from native_file_profile_trace import Replay


def check_process(p, scenario):
    require(p.get('case') == 400+scenario, 'transition process identity')
    expected = bytearray(fixture_bytes(0))
    if scenario != 0:
        expected[13] = 7; expected[28:31] = b'add'
    require(p.get('final_hex') == expected.hex(), 'final native bytes mismatch')
    # Reuse the strict unchanged-fixture schema/identity/diagnostics checks after
    # separately validating this profile's case number and final bytes above.
    return check_existing_process({**p,'case':300,'final_hex':p.get('initial_hex')},0)


def check_case(records, process, scenario):
    device,inode = check_process(process,scenario)
    r = Replay(records,400+scenario)
    r.consume('begin_transition',schema_version=1,root_length=27,device=device,inode=inode)
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
    def sync(phase):
        nonlocal synced
        r.consume('sync',phase=phase,map=mapping,size=4096,epoch=epoch,flags=1,result=0,errno=0)
        synced = 1
    def down():
        nonlocal up,length
        if not synced: sync('transition')
        r.unmap(mapping,4096,'transition',epoch); r.close(fd,'transition',epoch=epoch)
        up = False; length = 0
        operation('down')
    def restore():
        nonlocal up,length,synced
        open_map('transition'); up = True; length = header; synced = 1
        operation('up')
    open_map('constructor')
    r.consume('constructor_result',chunk=7,error=0); probe('returned',True); snapshot()
    if scenario == 0:
        down(); operation('down_again',-1); restore(); operation('up_again',-1)
    elif scenario == 3:
        locked = 1; operation('lock'); operation('lock_again',-1)
        operation('begin_locked',-2)
        length = header = 7; synced = 0
        operation('write_locked')
        locked = 0; operation('unlock'); operation('unlock_again',-1)
    else:
        transaction = 1; tx_length = 4
        operation('begin'); operation('begin_again')
        length = header = 7; synced = 0; operation('append')
        if scenario == 2:
            sync('transition'); transaction = 0; operation('commit')
        else:
            transaction = 0; length = tx_length
            operation('rollback'); operation('rollback_again',-1)
        if scenario == 4: down(); restore()
    def cleanup():
        if not synced:
            r.enter('cio_file_sync'); r.enter('cio_file_native_sync'); sync('teardown')
            r.leave('cio_file_native_sync'); r.leave('cio_file_sync')
        r.enter('cio_file_native_unmap'); r.unmap(mapping,4096,'teardown',epoch); r.leave('cio_file_native_unmap')
        r.enter('cio_chunk_counter_total_up_sub'); r.leave('cio_chunk_counter_total_up_sub')
        r.enter('cio_file_native_close'); r.close(fd,'teardown',epoch=epoch); r.leave('cio_file_native_close')
        r.backend_release(-1,4096)
    r.parents(cleanup); probe('parents_released',False); r.consume('end',live_allocations=0); r.finish()
    return {'function':'native_file_transition_observation','case':400+scenario,'values':{
        'compiled_source_and_build_recorded':0,'exclusive_instrumented_fixture':1,
        'exact_native_transitions':1,'exact_parent_teardown':1,'os_epochs_and_rings_bound':1,
        'allocations':10,'releases':10,'transient_releases':1,'teardown_releases':9,
        'open_epochs':epoch,'mapping_epochs':epoch,'logical_rollback':int(scenario in (1,4)),
        'rollback_header_retained':int(scenario in (1,4)),'reloaded_appended_bytes':int(scenario == 4),
        'locked_begin_retry':int(scenario == 3),'locked_write_succeeded':int(scenario == 3),
        'committed':int(scenario == 2),'final_header_length':header,'original_inode_retained':1,
        'fixture_cleanup_complete':1,'static_certificate':0,'durability_proved':0}}


def check_trace(records,processes):
    canonical(records); canonical(processes)
    require(len(processes) == 5, 'incomplete transition process matrix')
    groups, last = [[] for _ in range(5)], 400
    for row in records:
        case = row.get('case')
        require(type(case) is int and last <= case < 405, 'unknown/out-of-order case')
        groups[case-400].append(row); last = case
    return [check_case(rows,p,n) for n,(rows,p) in enumerate(zip(groups,processes))]

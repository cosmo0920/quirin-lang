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

"""Bind down-ring transitions to descriptor custody, including parent residuals.

Suppressed calls do not execute the syscall. This is not a model of arbitrary
kernel close failures, and harness reclamation never discharges library custody.
"""
import errno
import re
from check_teardown_trace import require, load_trace
from check_file_constructor_failure_trace import canonical
from check_file_existing_trace import check_process as check_existing_process
from native_file_profile_trace import Replay


def check_process(p, scenario):
    require(p.get('case') == 700+scenario, 'descriptor process identity')
    lines = p.get('stderr', '').splitlines()
    expected = [431, 954] + ([431] if scenario == 1 else []) if scenario < 2 else [411]
    require(len(lines) == len(expected), 'unexpected native/sanitizer diagnostics')
    for line, number in zip(lines, expected):
        filename = 'cio_file.c' if number == 954 else 'cio_file_unix.c'
        require(re.fullmatch(r'\[/[^\n]+/lib/chunkio/src/' + re.escape(filename) + ':' +
                str(number) + r' errno=5] Input/output error', line), 'native diagnostic mismatch')
    return check_existing_process({**p, 'case':300, 'stderr':''}, 0)


def check_case(records, process, scenario):
    device, inode = check_process(process, scenario)
    r = Replay(records, 700+scenario)
    r.consume('begin_descriptor_failure', schema_version=1, root_length=27, device=device, inode=inode)
    r.allocations()
    fd, mapping, epoch = -1, 0, 0
    up, descriptor_live = True, True

    def open_map(phase):
        nonlocal fd, mapping, epoch
        epoch += 1
        fd = r.peek('result')
        require(type(fd) is int and 0 <= fd < 2**31, 'invalid descriptor')
        r.consume('open', phase=phase, path=9, flags=66, mode=384, result=fd, errno=0, epoch=epoch)
        if phase == 'constructor':
            r.consume('chmod', phase=phase, path=9, mode=448, result=0, errno=0, injected=0)
        mapping = r.peek('result'); r.separated(mapping, 4096)
        require(mapping % 4096 == 0, 'unaligned mapping')
        r.consume('map', phase=phase, address=0, size=4096, prot=3, flags=1, fd=fd,
                  fd_epoch=epoch, offset=0, result=mapping, errno=0, epoch=epoch)

    def probe(label, mapped, held):
        r.consume('probe', label=label, fd=fd, fd_epoch=epoch,
            fd_result=0 if held else -1, fd_errno=0 if held else errno.EBADF,
            fd_device=device if held else 0, fd_inode=inode if held else 0,
            map=mapping, map_epoch=epoch, map_result=0 if mapped else -1,
            map_errno=0 if mapped else errno.ENOMEM, size=4096, mode=448, device=device, inode=inode)

    def snapshot():
        r.snapshot(dict(id=7, name=8, context=1, stream=5, locked=0, transaction=0,
            tx_length=0, error=0, backend=10, path=9, fd=fd if descriptor_live else -1,
            fd_epoch=epoch, map=mapping if up else 0, map_epoch=epoch, size=4096 if up else 0,
            fs_size=4096, data_size=4 if up else 0, synced=1, header_length=4 if up else 0,
            content='73656564' if up else ''), up)

    def operation(name, result):
        r.consume('operation', name=name, result=result); probe(name, up, descriptor_live); snapshot()

    def close(phase, suppressed=False, remaining=0):
        nonlocal descriptor_live
        if suppressed:
            r.consume('suppressed_close', phase=phase, epoch=epoch, remaining=remaining)
        r.consume('close', phase=phase, fd=fd, epoch=epoch, result=-1 if suppressed else 0,
                  errno=errno.EIO if suppressed else 0, injected=0)
        if not suppressed: descriptor_live = False

    open_map('constructor')
    r.consume('constructor_result', chunk=7, error=0); probe('returned', True, True); snapshot()
    if scenario < 2:
        r.consume('arm_descriptor_failure', close_count=scenario+1, open_count=0)
        r.unmap(mapping, 4096, 'transition', epoch); up = False
        close('transition', True, scenario)
        operation('down_failed_close', -1)
        operation('down_retry_unmapped', -1)
        operation('up_descriptor_present', -1)
    else:
        r.unmap(mapping, 4096, 'transition', epoch); up = False
        close('transition'); operation('down', 0)
        r.consume('arm_descriptor_failure', close_count=0, open_count=1)
        r.consume('suppressed_open', phase='transition', epoch=epoch, remaining=0)
        r.consume('open', phase='transition', path=9, flags=66, mode=384, result=-1,
                  errno=errno.EIO, epoch=epoch)
        operation('up_failed_open', -1)
        if scenario == 2:
            open_map('transition'); up = descriptor_live = True
            operation('up_retry', 0)

    def cleanup():
        nonlocal up
        if up:
            r.enter('cio_file_native_unmap'); r.unmap(mapping, 4096, 'teardown', epoch)
            r.leave('cio_file_native_unmap')
            r.enter('cio_chunk_counter_total_up_sub'); r.leave('cio_chunk_counter_total_up_sub')
            up = False
        r.enter('cio_file_native_close')
        if descriptor_live: close('teardown', scenario == 1)
        r.leave('cio_file_native_close')
        r.backend_release(fd if descriptor_live else -1, 4096)
    r.parents(cleanup); probe('parents_released', False, scenario == 1)
    r.consume('end', live_allocations=0)
    if scenario == 1:
        r.consume('harness_close', fd=fd, epoch=epoch, result=0, errno=0)
        probe('harness_reclaimed', False, False)
    r.consume('complete', pending_close=0, pending_open=0); r.finish()
    return {'function':'native_file_descriptor_failure_observation', 'case':700+scenario, 'values':{
        'compiled_source_and_build_recorded':0, 'exclusive_instrumented_fixture':1,
        'exact_native_transitions':1, 'exact_parent_teardown':1, 'os_epochs_and_rings_bound':1,
        'allocations':10, 'releases':10, 'transient_releases':1, 'teardown_releases':9,
        'open_epochs':epoch, 'mapping_epochs':epoch, 'suppressed_close_calls':scenario+1 if scenario < 2 else 0,
        'suppressed_open_calls':int(scenario >= 2),
        'failed_close_retains_descriptor_in_down_ring':int(scenario < 2),
        'retry_down_and_up_reject_without_closing':int(scenario < 2),
        'failed_open_preserves_down_state_without_new_epoch':int(scenario >= 2),
        'reopen_retry_acquires_new_epoch':int(scenario == 2),
        'library_descriptor_releases':epoch-int(scenario == 1),
        'parent_descriptor_residuals':int(scenario == 1), 'harness_descriptor_releases':int(scenario == 1),
        'original_inode_retained':1, 'fixture_cleanup_complete':1, 'static_certificate':0, 'durability_proved':0}}


def check_trace(records, processes):
    canonical(records); canonical(processes)
    require(len(processes) == 4, 'incomplete descriptor-failure process matrix')
    groups, last = [[] for _ in range(4)], 700
    for row in records:
        case = row.get('case')
        require(type(case) is int and last <= case < 704, 'unknown/out-of-order case')
        groups[case-700].append(row); last = case
    return [check_case(rows, p, n) for n, (rows, p) in enumerate(zip(groups, processes))]

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

"""Bind reopen size-lookup and mapping failures to descriptor and mapping custody.

Suppressed fstat/stat/mmap calls never reach the kernel; every other selected
call is forwarded. These are five fixed sequences, not a model of arbitrary
kernel failures or of production callers. A failed mapping leaves the backend
field holding MAP_FAILED, which the library's own predicate still reads as
"mapped"; that is recorded as an observed state, never repaired here. Harness
reclamation never discharges library custody.
"""
import errno
import re
from check_teardown_trace import require, load_trace
from check_file_constructor_failure_trace import canonical
from check_file_existing_trace import check_process as check_existing_process
from native_file_profile_trace import Replay

CASES = 5
MAP_FAILED = 2**64 - 1
SIZE_FAILURE = (1, 2)
MAP_FAILURE = (3, 4)
SUPPRESSED_PARENT_CLOSE = (2, 4)
CONTENT = b'seed'.hex()


def diagnostics(scenario):
    """Ordered native os-error reports expected on stderr for one process."""
    lines = []
    if scenario in MAP_FAILURE:
        # Suppressed mmap, then the invalid-argument unmap of MAP_FAILED that
        # the down retry and the parent teardown each attempt once.
        lines = [(103, 5, 'Input/output error'), (62, 22, 'Invalid argument'),
                 (62, 22, 'Invalid argument')]
    if scenario in SUPPRESSED_PARENT_CLOSE:
        lines.append((431, 5, 'Input/output error'))
    return lines


def check_process(p, scenario):
    require(p.get('case') == 800+scenario, 'reopen process identity')
    expected = diagnostics(scenario)
    lines = p.get('stderr', '').splitlines()
    require(len(lines) == len(expected), 'unexpected native/sanitizer diagnostics')
    for line, (number, code, message) in zip(lines, expected):
        require(re.fullmatch(r'\[/[^\n]+/lib/chunkio/src/' +
                re.escape(f'cio_file_unix.c:{number} errno={code}] {message}'), line),
                'native diagnostic mismatch')
    return check_existing_process({**p, 'case':300, 'stderr':''}, 0)


def check_case(records, process, scenario):
    device, inode = check_process(process, scenario)
    r = Replay(records, 800+scenario)
    r.consume('begin_reopen_failure', schema_version=1, root_length=27, device=device, inode=inode)
    r.allocations()
    # held/live are OS endpoint facts; the cf_* fields are backend storage.
    s = dict(fd=-1, epoch=0, mapping=0, map_epoch=0, held=False, live=False,
             cf_fd=-1, cf_map=0, cf_size=0, cf_fs=0, cf_data=0,
             header=0, content='', up=False, counter=0)

    def probe(label):
        r.consume('probe', label=label, fd=s['fd'], fd_epoch=s['epoch'],
            fd_result=0 if s['held'] else -1, fd_errno=0 if s['held'] else errno.EBADF,
            fd_device=device if s['held'] else 0, fd_inode=inode if s['held'] else 0,
            map=s['mapping'], map_epoch=s['map_epoch'],
            map_result=0 if s['live'] else -1, map_errno=0 if s['live'] else errno.ENOMEM,
            size=4096, mode=448, device=device, inode=inode)

    def snapshot():
        r.snapshot(dict(id=7, name=8, context=1, stream=5, locked=0, transaction=0,
            tx_length=0, error=0, backend=10, path=9, fd=s['cf_fd'], fd_epoch=s['epoch'],
            map=s['cf_map'], map_epoch=s['map_epoch'], size=s['cf_size'], fs_size=s['cf_fs'],
            data_size=s['cf_data'], synced=1, header_length=s['header'], content=s['content']),
            s['up'], s['counter'])

    def operation(name, result):
        r.consume('operation', name=name, result=result); probe(name); snapshot()

    def open_epoch(phase, number):
        fd = r.peek('result')
        require(type(fd) is int and 0 <= fd < 2**31, 'invalid descriptor')
        r.consume('open', phase=phase, path=9, flags=66, mode=384, result=fd, errno=0, epoch=number)
        s.update(fd=fd, epoch=number, held=True, cf_fd=fd)

    def map_epoch(phase, number):
        mapping = r.peek('result'); r.separated(mapping, 4096)
        require(mapping % 4096 == 0, 'unaligned mapping')
        r.consume('map', phase=phase, address=0, size=4096, prot=3, flags=1, fd=s['fd'],
                  fd_epoch=s['epoch'], offset=0, result=mapping, errno=0, epoch=number)
        s.update(mapping=mapping, map_epoch=number, live=True, cf_map=mapping, cf_size=4096,
                 cf_fs=4096, cf_data=4, header=4, content=CONTENT, up=True, counter=1)

    def size_lookup(call, phase, epoch, injected):
        r.consume('size_lookup', phase=phase, call=call, epoch=epoch, result=-1 if injected else 0,
                  errno=errno.EIO if injected else 0, injected=int(injected),
                  size=-1 if injected else 4096)

    open_epoch('constructor', 1)
    size_lookup('fstat', 'constructor', 1, False)
    r.consume('chmod', phase='constructor', path=9, mode=448, result=0, errno=0, injected=0)
    map_epoch('constructor', 1)
    r.consume('constructor_result', chunk=7, error=0)
    probe('returned'); snapshot()

    r.unmap(s['mapping'], 4096, 'transition', 1)
    size_lookup('fstat', 'transition', 1, False)
    r.close(s['fd'], 'transition', False, 1)
    s.update(held=False, live=False, cf_fd=-1, cf_map=0, cf_size=0, cf_fs=4096,
             cf_data=0, header=0, content='', up=False, counter=0)
    operation('down', 0)

    r.consume('arm_reopen_failure', fstat_count=int(scenario < 3),
              stat_count=int(scenario in SIZE_FAILURE), map_count=int(scenario in MAP_FAILURE))
    open_epoch('transition', 2)
    invalid_unmaps = 0
    if scenario < 3:
        size_lookup('fstat', 'transition', 2, True)
        size_lookup('stat', 'transition', 2, scenario in SIZE_FAILURE)
    else:
        size_lookup('fstat', 'transition', 2, False)
    if scenario == 0:
        map_epoch('transition', 2)
        operation('up_stat_fallback', 0)
    elif scenario in SIZE_FAILURE:
        # cio_file_update_size zeroes the recorded size when both lookups fail.
        s['cf_fs'] = 0
        operation('up_failed_size_lookup', -1)
        operation('down_retry_unmapped', -1)
        operation('up_retry_descriptor_present', -1)
    else:
        r.consume('suppressed_map', phase='transition', fd=s['fd'], epoch=2, size=4096, remaining=0)
        r.consume('map', phase='transition', address=0, size=4096, prot=3, flags=1, fd=s['fd'],
                  fd_epoch=2, offset=0, result=MAP_FAILED, errno=errno.EIO, epoch=s['map_epoch'])
        # No mapping epoch is acquired, yet the library's is_mapped predicate
        # now holds, so state-ring membership moves up without a counter add.
        s.update(cf_map=MAP_FAILED, cf_size=4096, cf_fs=4096, up=True, counter=0)
        operation('up_failed_map', -1)
        if scenario == 3: operation('up_retry_already_mapped', -1)
        else: operation('sync_after_failed_map', 0)
        r.unmap(MAP_FAILED, 4096, 'transition', s['map_epoch'], -1, errno.EINVAL)
        invalid_unmaps += 1
        operation('down_retry_invalid_map', -1)

    suppressed = scenario in SUPPRESSED_PARENT_CLOSE

    def cleanup():
        nonlocal invalid_unmaps
        if s['live'] or s['cf_map'] == MAP_FAILED:
            r.enter('cio_file_native_unmap')
            if s['live']:
                r.unmap(s['mapping'], 4096, 'teardown', s['map_epoch'])
            else:
                r.unmap(MAP_FAILED, 4096, 'teardown', s['map_epoch'], -1, errno.EINVAL)
                invalid_unmaps += 1
            r.leave('cio_file_native_unmap')
            if s['live']:
                r.enter('cio_chunk_counter_total_up_sub'); r.leave('cio_chunk_counter_total_up_sub')
                s.update(live=False, cf_map=0, cf_size=0, cf_data=0)
        r.enter('cio_file_native_close')
        r.close(s['fd'], 'teardown', suppressed, s['epoch'])
        if not suppressed: s.update(held=False, cf_fd=-1)
        r.leave('cio_file_native_close')
        r.backend_release(s['cf_fd'], s['cf_fs'], s['cf_map'], s['cf_size'], s['cf_data'], 0)

    r.parents(cleanup)
    probe('parents_released')
    r.consume('end', live_allocations=0)
    if suppressed:
        r.consume('harness_close', fd=s['fd'], epoch=s['epoch'], result=0, errno=0)
        s['held'] = False
        probe('harness_reclaimed')
    r.consume('complete', pending_fstat=0, pending_stat=0, pending_map=0)
    r.finish()
    mappings = 2 if scenario == 0 else 1
    return {'function':'native_file_reopen_failure_observation', 'case':800+scenario, 'values':{
        'compiled_source_and_build_recorded':0, 'exclusive_instrumented_fixture':1,
        'exact_native_transitions':1, 'exact_parent_teardown':1, 'os_epochs_and_rings_bound':1,
        'allocations':10, 'releases':10, 'transient_releases':1, 'teardown_releases':9,
        'open_epochs':2, 'mapping_epochs':mappings, 'library_mapping_releases':mappings,
        'library_descriptor_releases':2-int(suppressed),
        'parent_descriptor_residuals':int(suppressed), 'harness_descriptor_releases':int(suppressed),
        'suppressed_size_lookups':(1 if scenario == 0 else 2) if scenario < 3 else 0,
        'suppressed_map_calls':int(scenario in MAP_FAILURE),
        'stat_fallback_completes_reopen':int(scenario == 0),
        'failed_size_lookup_retains_descriptor':int(scenario in SIZE_FAILURE),
        'failed_map_retains_descriptor':int(scenario in MAP_FAILURE),
        'reopen_failure_retains_new_descriptor':int(scenario != 0),
        'failed_map_reports_up_ring_without_counter':int(scenario in MAP_FAILURE),
        'down_retry_rejects_without_new_epoch':int(scenario != 0),
        'up_retry_rejects_without_new_epoch':int(scenario in (1, 2, 3)),
        'invalid_map_unmap_failures':invalid_unmaps,
        'sync_succeeds_without_mapping':int(scenario == 4),
        'original_inode_retained':1, 'fixture_cleanup_complete':1,
        'static_certificate':0, 'durability_proved':0}}


def check_trace(records, processes):
    canonical(records); canonical(processes)
    require(len(processes) == CASES, 'incomplete reopen-failure process matrix')
    groups, last = [[] for _ in range(CASES)], 800
    for row in records:
        case = row.get('case')
        require(type(case) is int and last <= case < 800+CASES, 'unknown/out-of-order case')
        groups[case-800].append(row); last = case
    return [check_case(rows, p, n) for n, (rows, p) in enumerate(zip(groups, processes))]

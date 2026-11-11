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

"""Replay eight pinned Linux constructor OS-failure traces without promotion.

Compiler, complete native instrumentation and exclusive process/fixture records
are trusted. Suppressed calls establish only the injected post-state; libc error
conventions, allocation custody and descriptor custody are checked separately.
"""
import errno
import re
from check_teardown_trace import require, load_trace
from check_file_teardown_trace import LAYOUT
from check_file_constructor_failure_trace import canonical, PLAN

SUCCESS = (0,2)
MAP_FAILED = 2**64-1
FAILURES = (0,1,1,2,1,1,2,2)
# The records are Linux ABI observations. Python's errno module describes the
# host running the replay (macOS uses a different EOPNOTSUPP number), so pin the
# observed Linux value instead of silently reinterpreting the trace.
LINUX_EOPNOTSUPP = 95


def check_process(process, scenario):
    require(set(process)=={"case","returncode","stderr","root_before","root_after","entries","fixture_cleanup_complete"},
            "unexpected process schema")
    require(process["case"]==200+scenario and process["returncode"]==0
            and process["fixture_cleanup_complete"]==1,"process identity/completion mismatch")
    before=process["root_before"]
    require(len(before)==2 and all(0<v<2**64 for v in before) and before==process["root_after"],
            "fixture root identity changed")
    entries=process["entries"]
    expected=[("stream-0","directory")]
    if scenario!=1: expected.append(("stream-0/chunk-0","file"))
    require([(e["relative"],e["kind"]) for e in entries]==expected,"unexpected surviving fixture tree")
    identities=[tuple(before)]
    for entry in entries:
        require(set(entry)=={"relative","kind","device","inode"},"fixture entry schema")
        ident=(entry["device"],entry["inode"])
        require(ident[0]==before[0] and 0<ident[1]<2**64 and ident not in identities,"aliased/invalid inode")
        identities.append(ident)
    # Diagnostics bind the pinned source locations as well as error convention.
    expected_errors={1:[("cio_file_unix.c",411,13,"Permission denied")],
        4:[("cio_file_unix.c",564,95,"Operation not supported"),("cio_file.c",1275,95,"Operation not supported")],
        5:[("cio_file_unix.c",103,12,"Cannot allocate memory")],
        6:[("cio_file_unix.c",103,12,"Cannot allocate memory"),("cio_file_unix.c",431,5,"Input/output error")],
        7:[("cio_file.c",377,5,"Input/output error")]}.get(scenario,[])
    lines=process["stderr"].splitlines()
    require(len(lines)==len(expected_errors),"unexpected native/sanitizer diagnostics")
    for line,(source,number,code,message) in zip(lines,expected_errors):
        require(re.fullmatch(r"\[/[^\n]+/lib/chunkio/src/"+re.escape(f"{source}:{number} errno={code}] {message}"),line),
                "native diagnostic mismatch")
    return (0,0) if scenario==1 else (entries[1]["device"],entries[1]["inode"])


def check_case(records,process,scenario):
    case,cursor=200+scenario,0
    live={}
    releases=[]
    device,inode=check_process(process,scenario)
    success=scenario in SUCCESS
    def consume(kind,**fields):
        nonlocal cursor
        require(cursor<len(records),"truncated trace")
        require(records[cursor]==dict(case=case,sequence=cursor,kind=kind,**fields),
                f"case {case}: unexpected {kind} at {cursor}")
        cursor+=1
    def enter(function): consume("enter",function=function)
    def leave(function): consume("exit",function=function)
    def release(ident):
        require(ident in live,"release of non-live generation")
        consume("free",id=ident);del live[ident];releases.append(ident)
    consume("begin_os_constructor",schema_version=1,scenario=scenario,root_length=27)
    for ident,(allocator,size) in enumerate(PLAN,1):
        require(cursor<len(records),"missing allocation")
        address=records[cursor].get("address",0)
        require(type(address) is int and 0<address and address+size<=2**64,"invalid allocation storage")
        require(all(address+size<=a or a+n<=address for a,n in live.values()),"overlapping live allocations")
        consume("allocation",id=ident,address=address,size=size,allocator=allocator)
        live[ident]=(address,size)
        if ident==5: release(5);release(4)
    require(cursor<len(records),"missing native open")
    fd=records[cursor].get("result")
    require(type(fd) is int and (fd==-1 if scenario==1 else 0<=fd<2**31),"invalid descriptor")
    consume("constructor_open",path=10,flags=66,mode=384,result=fd,errno=errno.EACCES if scenario==1 else 0,
            injected=int(scenario==1),device=device,inode=inode)
    mapping=0
    def size_lookup(ordinal,fail_fd,fail_path):
        consume("constructor_fstat",fd=fd,ordinal=ordinal,result=-1 if fail_fd else 0,
            errno=errno.EIO if fail_fd else 0,injected=int(fail_fd),size=-1 if fail_fd else 0,
            device=0 if fail_fd else device,inode=0 if fail_fd else inode)
        if fail_fd:
            consume("constructor_stat",path=10,result=-1 if fail_path else 0,
                errno=errno.EIO if fail_path else 0,injected=int(fail_path),size=-1 if fail_path else 0,
                device=0 if fail_path else device,inode=0 if fail_path else inode)
    if scenario!=1:
        size_lookup(1,scenario in (2,3),scenario==3)
        if scenario!=3:
            consume("constructor_chmod",path=10,mode=448,result=0,errno=0)
            size_lookup(2,scenario==7,scenario==7)
            if scenario!=7:
                # Pinned build falls through the compile-time unsupported Linux
                # strategy to POSIX allocation. A positive result is an error;
                # errno retains EOPNOTSUPP from the native fallback selection.
                consume("constructor_resize",fd=fd,offset=0,length=4096,
                    result=errno.ENOSPC if scenario==4 else 0,errno_before=LINUX_EOPNOTSUPP,
                    errno=LINUX_EOPNOTSUPP,injected=int(scenario==4))
                if scenario!=4:
                    require(cursor<len(records),"missing native map")
                    mapping=records[cursor].get("result",0)
                    if success:
                        require(type(mapping) is int and 0<mapping and mapping%4096==0 and mapping+4096<=2**64,
                                "invalid successful mapping")
                        require(all(mapping+4096<=a or a+n<=mapping for a,n in live.values()),"mapping overlaps allocation")
                    else: require(mapping==MAP_FAILED,"failed mmap must retain MAP_FAILED sentinel")
                    consume("constructor_map",address=0,length=4096,prot=3,flags=1,fd=fd,offset=0,
                        result=mapping,errno=0 if success else errno.ENOMEM,injected=int(not success))
        if not success:
            consume("constructor_close",fd=fd,result=-1 if scenario==6 else 0,
                errno=errno.EIO if scenario==6 else 0,injected=int(scenario==6))
    if not success:
        release(10)
        consume("rollback_backend",id=11,fd=fd if scenario==6 else -1,map=mapping,
            size=4096 if scenario in (5,6) else 0,fs_size=4096 if scenario in (5,6) else 0,up=0)
        release(11);release(9);release(8)
    consume("constructor_result",context=1,stream=6,chunk=8 if success else 0,error=0 if success else -1)
    def probe(phase):
        present=(success and phase=="returned") or scenario==6
        consume("constructor_probe",phase=phase,fd=fd,fd_result=0 if present else -1,
            fd_errno=0 if present else errno.EBADF,fd_device=device if present else 0,fd_inode=inode if present else 0,
            path_result=-1 if scenario==1 else 0,path_errno=errno.ENOENT if scenario==1 else 0,
            path_size=-1 if scenario==1 else 4096 if scenario in (0,2,5,6) else 0,device=device,inode=inode)
    probe("returned")
    def ring(parent,offset,owner=0,node_offset=""):
        sentinel=live[parent][0]+LAYOUT[offset]
        address=live[owner][0]+LAYOUT[node_offset] if owner else sentinel
        return dict(sentinel=sentinel,next=address,prev=address,
            nodes=[dict(owner=owner,address=address,next=sentinel,prev=sentinel)] if owner else [])
    chunks=[dict(id=8,name=9,context=1,stream=6,locked=0,transaction=0,backend=11,path=10,backend_context=1,
        fd=fd,map=mapping,size=4096,synced=0,data_size=0,device=device,inode=inode,links=1,path_device=device,path_inode=inode)] if success else []
    stream=dict(id=6,name=7,context=1,memory=0,ring=ring(6,"stream_ring",8 if success else 0,"chunk_node"),
        up_ring=ring(6,"up_ring",8 if success else 0,"state_node"),down_ring=ring(6,"down_ring"),chunks=chunks)
    consume("snapshot",context=dict(id=1,total=int(success),up=int(success),fields=[0,0,2,0,0,3],
        ring=ring(1,"context_ring",6,"stream_node"),streams=[stream]))
    for name in ("cio_destroy","cio_stream_destroy_all","cio_stream_destroy","cio_chunk_close_stream"): enter(name)
    if success:
        for name in ("cio_chunk_close","cio_file_close","cio_file_sync","cio_file_native_sync"): enter(name)
        consume("os_sync",map=mapping,size=4096,flags=1,result=0,errno=0,injected=0)
        leave("cio_file_native_sync");leave("cio_file_sync")
        enter("cio_file_native_unmap")
        consume("os_unmap",map=mapping,size=4096,result=0,errno=0,injected=0)
        leave("cio_file_native_unmap")
        enter("cio_chunk_counter_total_up_sub");leave("cio_chunk_counter_total_up_sub")
        enter("cio_file_native_close")
        consume("os_close",fd=fd,result=0,errno=0,injected=0)
        leave("cio_file_native_close");release(10)
        consume("backend_state",id=11,fd=-1,map=0,size=0,data_size=0,up=0,synced=1)
        release(11);leave("cio_file_close");release(9)
        consume("chunk_state",id=8,primary_cleared=1,state_cleared=1,total=1)
        release(8)
        enter("cio_chunk_counter_total_sub");leave("cio_chunk_counter_total_sub")
        consume("counter",total=0,up=0);leave("cio_chunk_close")
    leave("cio_chunk_close_stream");release(7)
    consume("stream_state",id=6,primary_cleared=1,chunks_empty=1,up_empty=1,down_empty=1)
    release(6);leave("cio_stream_destroy");leave("cio_stream_destroy_all")
    release(2);release(3)
    consume("context_state",id=1,total=0,up=0,streams_empty=1)
    release(1);leave("cio_destroy")
    probe("parents_released")
    if success: consume("terminal_mapping",map=mapping,size=4096,result=-1,errno=errno.ENOMEM)
    consume("end",live_allocations=0)
    if scenario==6:
        consume("harness_descriptor_cleanup",fd=fd,result=0,errno=0,fd_result=-1,fd_errno=errno.EBADF)
    require(cursor==len(records) and not live and sorted(releases)==list(range(1,12)),"terminal allocation/trace mismatch")
    return {"function":"native_file_os_constructor_observation","case":case,"values":{
        "compiled_source_and_build_recorded":0,"exclusive_instrumented_fixture":1,
        "exact_constructor_and_rollback":1,"exact_parent_teardown":1,"os_identities_bound":1,
        "allocations":11,"releases":11,"transient_releases":2,"rollback_releases":0 if success else 4,
        "teardown_releases":9 if success else 5,"injected_failures":FAILURES[scenario],
        "chunk_null":int(not success),"fallback_stat_succeeded":int(scenario==2),
        "posix_resize_error":int(scenario==4),"posix_error_distinct_from_errno":int(scenario==4),
        "failed_map_sentinel":int(scenario in (5,6)),"descriptor_residual_after_return":int(scenario==6),
        "descriptor_residual_after_parents":int(scenario==6),"path_retained":int(scenario!=1),
        "harness_descriptor_reclaimed":int(scenario==6),"fixture_cleanup_complete":1,
        "static_certificate":0,"durability_proved":0}}


def check_trace(records,processes):
    canonical(records);canonical(processes)
    require(len(processes)==8,"incomplete process matrix")
    groups=[[] for _ in range(8)];last=200
    for record in records:
        case=record.get("case")
        require(type(case) is int and last<=case<208,"unknown/out-of-order case")
        groups[case-200].append(record);last=case
    return [check_case(rows,process,scenario) for scenario,(rows,process) in enumerate(zip(groups,processes))]

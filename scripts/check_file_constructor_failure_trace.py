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

"""Check a fixed Linux ENOMEM matrix, including two interrupted prefixes.

The native observer/compiler and external process/fixture records are trusted.
A sanitizer termination has an unreleased prefix ledger, never a completed
teardown or a static Unsafe certificate. No source verdict is promoted.
"""
import errno
import re
from check_teardown_trace import require, load_trace
from check_file_teardown_trace import LAYOUT

PLAN = [("calloc",136), ("strdup",5), ("strdup",28), ("malloc",38),
        ("strdup",37), ("malloc",88), ("strdup",9), ("malloc",96),
        ("strdup",8), ("malloc",50), ("calloc",120)]
ATTEMPTS = [11,1,11,3,4,11,6,7,8,9,10,11]
INTERRUPTED = {3: ("cio_stream.c",65,"check_stream_path"),
               9: ("cio_file_unix.c",382,"cio_file_native_filename_check")}


def canonical(value):
    if isinstance(value, dict):
        for item in value.values(): canonical(item)
    elif isinstance(value, list):
        for item in value: canonical(item)
    else:
        require(type(value) in (int,str), "noncanonical scalar")


def check_process(process, fail):
    require(set(process) == {"case","returncode","stderr","root_before","root_after",
                             "entries","fixture_cleanup_complete"}, "process schema mismatch")
    require(process["case"] == 100 + fail and process["fixture_cleanup_complete"] == 1,
            "process identity/cleanup mismatch")
    before = process["root_before"]
    require(len(before) == 2 and all(0 < v < 2**64 for v in before)
            and before == process["root_after"], "fixture root identity changed")
    entries = process["entries"]
    names = [] if fail in (1,3,4,5) else [("stream-0","directory")]
    if fail in (0,2): names.append(("stream-0/chunk-0","file"))
    require([(e["relative"],e["kind"]) for e in entries] == names,
            "unexpected surviving fixture paths")
    identities = [tuple(before)]
    for entry in entries:
        require(set(entry) == {"relative","kind","device","inode"}, "fixture entry schema")
        ident = (entry["device"],entry["inode"])
        require(ident[0] == before[0] and 0 < ident[1] < 2**64 and ident not in identities,
                "aliased/invalid fixture inode")
        identities.append(ident)
    stderr = process["stderr"]
    if fail in INTERRUPTED:
        source,line,function = INTERRUPTED[fail]
        location = r"(/[^\n]+/lib/chunkio/src/)" + re.escape(f"{source}:{line}:18")
        require(process["returncode"] == 1, "expected sanitizer exit code")
        diagnostic = re.match(location + r": runtime error: null pointer passed as argument 1, which is declared to never be null\n",stderr)
        require(diagnostic, "wrong sanitizer diagnostic")
        source_root = diagnostic.group(1)
        location = re.escape(source_root + f"{source}:{line}:18")
        require(re.search(r"SUMMARY: UndefinedBehaviorSanitizer: undefined-behavior " + location + r"\s*\Z",stderr),
                "missing sanitizer summary")
        lines = stderr.splitlines()
        frames = [(function,f"{source}:{line}:11")]
        frames += ([("cio_stream_create","cio_stream.c:151:15")] if fail==3 else
                   [("cio_file_open","cio_file.c:650:11"),("cio_chunk_open","cio_chunk.c:81:19")])
        require(len(lines)==len(frames)+8 and lines[-2]=="", "truncated/extra sanitizer diagnostic lines")
        require(re.fullmatch(r"/[^\n]+/string\.h:[1-9][0-9]*:[1-9][0-9]*: note: nonnull attribute specified here",lines[1]),
                "unexpected nonnull declaration")
        for index,(name,site) in enumerate(frames):
            require(re.fullmatch(rf"    #{index} 0x[0-9a-f]+ in {name} " + re.escape(source_root + site),lines[2+index]),
                    "wrong sanitizer call chain")
        index = len(frames)
        require(re.fullmatch(rf"    #{index} 0x[0-9a-f]+ in main /[^\n]+/compiler/examples/chunk_file_constructor_failure_observation\.c:[1-9][0-9]*:[1-9][0-9]*",lines[2+index]),
                "missing native caller frame")
        for offset,name in enumerate(("__libc_start_call_main","__libc_start_main","_start"),1):
            require(re.fullmatch(rf"    #{index+offset} 0x[0-9a-f]+ in {name} [^\n]+",lines[2+index+offset]),
                    "unexpected runtime stack frame")
    else:
        require(process["returncode"] == 0, "native execution did not finish cleanly")
        diagnostic = {1:r"calloc: Cannot allocate memory\n",
            4:r"cio_stream.c:68 errno=12",5:r"cio_file_unix.c:411 errno=2",
            6:r"cio_stream.c:159 errno=12",7:r"cio_stream.c:165 errno=12",
            8:r"cio_chunk.c:63 errno=12",10:r"cio_file_unix.c:359 errno=12",
            11:r"cio_file.c:665 errno=12"}.get(fail)
        if fail == 1:
            require(re.fullmatch(diagnostic,stderr), "wrong allocator diagnostic")
        elif diagnostic:
            message = "No such file or directory" if fail == 5 else "Cannot allocate memory"
            require(re.fullmatch(r"\[/[^\n]+/lib/chunkio/src/" + re.escape(diagnostic) + r"\] " + message + r"\n",stderr),
                    "unexpected native diagnostic")
        else: require(stderr == "", "unexpected native stderr")


def check_case(records, process, fail):
    case, cursor, generation = 100 + fail, 0, 0
    live, ids, acquired = {}, {}, []
    transient, rollback = [], []
    def consume(kind, **fields):
        nonlocal cursor
        require(cursor < len(records), "truncated trace")
        require(records[cursor] == dict(case=case,sequence=cursor,kind=kind,**fields),
                f"case {case}: unexpected {kind} at {cursor}")
        cursor += 1
    def enter(name): consume("enter",function=name)
    def leave(name): consume("exit",function=name)
    def release(ordinal, ledger):
        ident = ids[ordinal]
        require(ident in live, "release of non-live generation")
        consume("free",id=ident)
        del live[ident]
        ledger.append(ident)
    consume("begin_file_failure",schema_version=1,fail_at=fail,root_length=27)
    for ordinal,(allocator,size) in enumerate(PLAN[:ATTEMPTS[fail]],1):
        consume("attempt",ordinal=ordinal,allocator=allocator,size=size,failed=int(ordinal==fail))
        if ordinal != fail:
            require(cursor < len(records), "missing allocation")
            address = records[cursor].get("address",0)
            require(type(address) is int and 0 < address and address + size <= 2**64,
                    "invalid native allocation")
            require(all(address + size <= a or a + n <= address for a,n in live.values()),
                    "overlapping live allocations")
            generation += 1
            consume("allocation",id=generation,address=address,size=size,allocator=allocator)
            live[generation] = (address,size)
            ids[ordinal] = generation
            acquired.append(generation)
        if ordinal == 3 or fail == 1:
            consume("context_result",context=ids.get(1,0),chmod=ids.get(2,0),root=ids.get(3,0))
        if ordinal == 5:
            if fail != 5: release(5,transient)
            release(4,transient)
        if ordinal == 7 and fail == 7: release(6,rollback)
        if ordinal == 7 or (ordinal == fail and fail in (4,6)):
            consume("stream_result",stream=ids.get(6,0) if fail!=7 else 0,name=ids.get(7,0))
    # The interrupted native calls have no constructor result, snapshot, teardown
    # or terminal allocation census. Keep every outstanding prefix generation.
    if fail in INTERRUPTED:
        require(cursor == len(records), "events after sanitizer termination")
        return observation(fail,acquired,transient,rollback,[],list(live),False)
    for ordinal in {5:[10,11,9,8],10:[9,8],11:[10,9,8]}.get(fail,[]):
        release(ordinal,rollback)
    ctx = ids.get(1,0)
    stream = ids.get(6,0) if fail != 7 else 0
    chunk = ids.get(8,0) if fail in (0,2) else 0
    error = 0 if chunk else (-1 if fail == 5 else -999)
    consume("constructor_result",context=ctx,stream=stream,chunk=chunk,error=error,attempts=ATTEMPTS[fail])
    def ring(parent,offset,owner=0,node_offset=""):
        sentinel = live[parent][0] + LAYOUT[offset]
        address = live[owner][0] + LAYOUT[node_offset] if owner else sentinel
        return dict(sentinel=sentinel,next=address,prev=address,
                    nodes=[dict(owner=owner,address=address,next=sentinel,prev=sentinel)] if owner else [])
    fd = mapping = size = device = inode = 0
    if ctx:
        child = []
        if chunk:
            require(cursor < len(records), "missing filesystem snapshot")
            try: recorded = records[cursor]["context"]["streams"][0]["chunks"][0]
            except (KeyError,IndexError,TypeError) as e: raise ValueError("missing chunk snapshot") from e
            fd,mapping,size,device,inode = [recorded[k] for k in ("fd","map","size","device","inode")]
            require(0 <= fd < 2**31 and size==4096 and 0 < mapping and mapping%size==0
                    and mapping+size<=2**64 and 0<device<2**64 and 0<inode<2**64,
                    "invalid filesystem identities")
            require(all(mapping+size<=a or a+n<=mapping for a,n in live.values()),
                    "mapping overlaps live allocation")
            file_entry = process["entries"][1]
            require((device,inode)==(file_entry["device"],file_entry["inode"]),
                    "post-process pathname identity changed")
            child = [dict(id=chunk,name=ids[9],context=ctx,stream=stream,locked=0,transaction=0,
                backend=ids[11],path=ids[10],backend_context=ctx,fd=fd,map=mapping,size=size,
                synced=0,data_size=0,device=device,inode=inode,links=1,path_device=device,path_inode=inode)]
        streams = [dict(id=stream,name=ids[7],context=ctx,memory=0,
            ring=ring(stream,"stream_ring",chunk,"chunk_node"),
            up_ring=ring(stream,"up_ring",chunk,"state_node"),down_ring=ring(stream,"down_ring"),
            chunks=child)] if stream else []
        consume("snapshot",context=dict(id=ctx,total=int(bool(chunk)),up=int(bool(chunk)),
            fields=[0,0,ids.get(2,0),0,0,ids[3]],ring=ring(ctx,"context_ring",stream,"stream_node"),streams=streams))
    teardown = []
    enter("cio_destroy")
    if ctx:
        enter("cio_stream_destroy_all")
        if stream:
            enter("cio_stream_destroy")
            enter("cio_chunk_close_stream")
            if chunk:
                for name in ("cio_chunk_close","cio_file_close","cio_file_sync","cio_file_native_sync"): enter(name)
                consume("os_sync",map=mapping,size=size,flags=1,result=0,errno=0,injected=0)
                leave("cio_file_native_sync"); leave("cio_file_sync")
                enter("cio_file_native_unmap")
                consume("os_unmap",map=mapping,size=size,result=0,errno=0,injected=0)
                leave("cio_file_native_unmap")
                enter("cio_chunk_counter_total_up_sub"); leave("cio_chunk_counter_total_up_sub")
                enter("cio_file_native_close")
                consume("os_close",fd=fd,result=0,errno=0,injected=0)
                leave("cio_file_native_close")
                release(10,teardown)
                consume("backend_state",id=ids[11],fd=-1,map=0,size=0,data_size=0,up=0,synced=1)
                release(11,teardown); leave("cio_file_close")
                release(9,teardown)
                consume("chunk_state",id=chunk,primary_cleared=1,state_cleared=1,total=1)
                release(8,teardown)
                enter("cio_chunk_counter_total_sub"); leave("cio_chunk_counter_total_sub")
                consume("counter",total=0,up=0); leave("cio_chunk_close")
            leave("cio_chunk_close_stream")
            release(7,teardown)
            consume("stream_state",id=stream,primary_cleared=1,chunks_empty=1,up_empty=1,down_empty=1)
            release(6,teardown); leave("cio_stream_destroy")
        leave("cio_stream_destroy_all")
        if 2 in ids: release(2,teardown)
        release(3,teardown)
        consume("context_state",id=ctx,total=0,up=0,streams_empty=1)
        release(1,teardown)
    leave("cio_destroy")
    if chunk:
        consume("os_terminal",fd=fd,map=mapping,size=size,fd_result=-1,fd_errno=errno.EBADF,
            map_result=-1,map_errno=errno.ENOMEM,path_result=0,path_errno=0,device=device,inode=inode)
    consume("end",live_allocations=0)
    require(cursor == len(records) and not live, "trailing events/unreleased allocations")
    return observation(fail,acquired,transient,rollback,teardown,[],True)


def observation(fail,acquired,transient,rollback,teardown,outstanding,completed):
    released = transient + rollback + teardown
    require(len(set(released+outstanding)) == len(acquired)
            and set(released+outstanding) == set(acquired), "allocation partition mismatch")
    return {"function":"native_file_constructor_failure_observation","case":100+fail,"values":{
        "compiled_source_and_build_recorded":0,"exclusive_instrumented_fixture":1,
        "exact_constructor_prefix":1,"attempts":ATTEMPTS[fail],"expected_attempts":ATTEMPTS[fail],
        "injected_failures":int(fail!=0),"expected_failures":int(fail!=0),
        "allocations":len(acquired),"releases":len(released),"transient_releases":len(transient),
        "rollback_releases":len(rollback),"teardown_releases":len(teardown),
        "prefix_outstanding":len(outstanding),"sanitizer_interrupted":int(not completed),
        "teardown_completed":int(completed),"native_leak_check_completed":int(completed),
        "nullable_chmod":int(fail==2),"stream_without_directory":int(fail==5),
        "exact_native_outcome":1,"fixture_cleanup_complete":1,"static_certificate":0,
        "durability_proved":0},"prefix_outstanding_generations":outstanding}


def check_trace(records, processes):
    canonical(records); canonical(processes)
    require(len(processes)==12, "incomplete process matrix")
    groups = [[] for _ in range(12)]
    last = 100
    for record in records:
        case = record.get("case")
        require(type(case) is int and last <= case < 112, "unknown/out-of-order case")
        groups[case-100].append(record)
        last = case
    results = []
    for fail,(rows,process) in enumerate(zip(groups,processes)):
        check_process(process,fail)
        results.append(check_case(rows,process,fail))
    return results

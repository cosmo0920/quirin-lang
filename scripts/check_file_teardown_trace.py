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

"""Check six native filesystem constructor/teardown traces, preserving residuals.

This fixed Linux profile trusts the compiler/observer, complete instrumentation,
exclusive fixtures and injected failures. It is not a general C custody proof.
"""
import errno
from check_teardown_trace import load_trace, require

LAYOUT = dict(context_size=136, stream_size=88, chunk_size=96, backend_size=120,
              context_ring=112, stream_node=16, stream_ring=32, up_ring=48,
              down_ring=64, chunk_node=80, state_node=64)


def check_trace(records):
    def canonical(value):
        if isinstance(value, dict):
            for item in value.values(): canonical(item)
        elif isinstance(value, list):
            for item in value: canonical(item)
        else:
            require(type(value) in (int, str), "noncanonical scalar")
    canonical(records)
    groups = [[] for _ in range(6)]
    last = 0
    for record in records:
        case = record.get("case")
        require(type(case) is int and last <= case < 6, "case/order mismatch")
        groups[case].append(record)
        last = case
    return [check_case(group, case) for case, group in enumerate(groups)]


def check_case(records, case):
    cursor, generation = 0, 0
    live, allocated = {}, {}
    def consume(kind, **fields):
        nonlocal cursor
        require(cursor < len(records), "truncated trace")
        require(records[cursor] == dict(case=case, sequence=cursor, kind=kind, **fields),
                "unexpected event at " + str(cursor) + ": " + kind)
        cursor += 1
    consume("begin_file", schema_version=1, delete=int(case == 2), write=int(case != 0),
            full_sync=int(case != 0), layout=LAYOUT)
    # Fixed constructor allocation plan includes two temporary stream-path
    # allocations which are released before returning the constructed stream.
    plan = [("calloc",136), ("strdup",5), ("strdup",28), ("malloc",38), ("strdup",37),
            ("malloc",88), ("strdup",9), ("malloc",96), ("strdup",8), ("malloc",50), ("calloc",120)]
    for allocator, size in plan:
        generation += 1
        require(cursor < len(records), "missing allocation")
        address = records[cursor].get("address")
        require(type(address) is int and 0 < address and address + size <= 2**64, "invalid storage")
        require(all(address + size <= a["address"] or a["address"] + a["size"] <= address
                    for a in live.values()), "overlapping live storage")
        consume("allocation", id=generation, address=address, size=size, allocator=allocator)
        allocated[generation] = live[generation] = dict(address=address,size=size)
        if generation == 5:
            for ident in (5, 4):
                consume("free", id=ident)
                del live[ident]
    consume("constructor_result", context=1, stream=6, chunk=8, error=0,
            write_result=-999 if case == 0 else 0, full_sync=int(case != 0))
    require(cursor < len(records), "missing snapshot")
    snapshot = records[cursor].get("context", {})
    streams = snapshot.get("streams", [])
    require(len(streams) == 1 and len(streams[0].get("chunks", [])) == 1, "wrong tree shape")
    chunk = streams[0]["chunks"][0]
    fd, mapping, size = chunk["fd"], chunk["map"], chunk["size"]
    device, inode = chunk["device"], chunk["inode"]
    require(type(fd) is int and 0 <= fd < 2**31 and type(mapping) is int and mapping > 0
            and size == 4096 and mapping % size == 0 and mapping + size <= 2**64,
            "invalid native descriptor/mapping")
    require(device > 0 and inode > 0 and device < 2**64 and inode < 2**64, "invalid inode")
    require(all(mapping + size <= a["address"] or a["address"] + a["size"] <= mapping
                for a in live.values()), "mapping overlaps allocation storage")
    def ring(parent, offset, owner=None, node_offset=None):
        sentinel = live[parent]["address"] + LAYOUT[offset]
        nodes = []
        address = sentinel
        if owner:
            address = live[owner]["address"] + LAYOUT[node_offset]
            nodes = [dict(owner=owner,address=address,next=sentinel,prev=sentinel)]
        return dict(sentinel=sentinel,next=address,prev=address,nodes=nodes)
    expected_chunk = dict(id=8,name=9,context=1,stream=6,locked=0,transaction=0,backend=11,path=10,
                          backend_context=1,fd=fd,map=mapping,size=size,synced=0,data_size=0 if case==0 else 7,
                          device=device,inode=inode,links=1,path_device=device,path_inode=inode)
    expected_stream = dict(id=6,name=7,context=1,memory=0,
                           ring=ring(6,"stream_ring",8,"chunk_node"),
                           up_ring=ring(6,"up_ring",8,"state_node"),down_ring=ring(6,"down_ring"),
                           chunks=[expected_chunk])
    consume("snapshot", context=dict(id=1,total=1,up=1,fields=[0,0,2,0,0,3],
            ring=ring(1,"context_ring",6,"stream_node"),streams=[expected_stream]))
    require(set(live) == {1,2,3,6,7,8,9,10,11}, "constructor orphan")
    def enter(name): consume("enter", function=name)
    def leave(name): consume("exit", function=name)
    def release(ident):
        require(ident in live, "release without live generation")
        consume("free", id=ident)
        del live[ident]
    def parents_enter():
        for name in ("cio_destroy","cio_stream_destroy_all","cio_stream_destroy","cio_chunk_close_stream"):
            enter(name)
    if case != 2: parents_enter()
    enter("cio_chunk_close")
    enter("cio_file_close")
    enter("cio_file_sync")
    enter("cio_file_native_sync")
    consume("os_sync", map=mapping,size=size,flags=1 if case==0 else 4,
            result=-1 if case==5 else 0,errno=errno.EIO if case==5 else 0,injected=int(case==5))
    leave("cio_file_native_sync")
    leave("cio_file_sync")
    enter("cio_file_native_unmap")
    consume("os_unmap",map=mapping,size=size,result=-1 if case==3 else 0,
            errno=errno.EIO if case==3 else 0,injected=int(case==3))
    leave("cio_file_native_unmap")
    if case != 3:
        enter("cio_chunk_counter_total_up_sub")
        leave("cio_chunk_counter_total_up_sub")
    enter("cio_file_native_close")
    consume("os_close",fd=fd,result=-1 if case==4 else 0,errno=errno.EIO if case==4 else 0,injected=int(case==4))
    leave("cio_file_native_close")
    if case == 2:
        enter("cio_file_native_delete")
        consume("os_delete",path=10,result=0,errno=0)
        leave("cio_file_native_delete")
    release(10)
    consume("backend_state",id=11,fd=fd if case==4 else -1,map=mapping if case==3 else 0,
            size=size if case==3 else 0,data_size=7 if case==3 else 0,up=int(case==3),synced=int(case!=5))
    release(11)
    leave("cio_file_close")
    release(9)
    consume("chunk_state",id=8,primary_cleared=1,state_cleared=1,total=1)
    release(8)
    enter("cio_chunk_counter_total_sub")
    leave("cio_chunk_counter_total_sub")
    consume("counter",total=0,up=int(case==3))
    leave("cio_chunk_close")
    if case == 2: parents_enter()
    leave("cio_chunk_close_stream")
    release(7)
    consume("stream_state",id=6,primary_cleared=1,chunks_empty=1,up_empty=1,down_empty=1)
    release(6)
    leave("cio_stream_destroy")
    leave("cio_stream_destroy_all")
    release(2)
    release(3)
    consume("context_state",id=1,total=0,up=int(case==3),streams_empty=1)
    release(1)
    leave("cio_destroy")
    consume("os_terminal",fd=fd,map=mapping,size=size,fd_result=0 if case==4 else -1,fd_errno=0 if case==4 else errno.EBADF,
            map_result=0 if case==3 else -1,map_errno=0 if case==3 else errno.ENOMEM,
            path_result=-1 if case==2 else 0,path_errno=errno.ENOENT if case==2 else 0,
            device=0 if case==2 else device,inode=0 if case==2 else inode)
    consume("end",live_allocations=0,fixture_cleanup_complete=1)
    require(cursor == len(records) and not live, "trailing events/live allocations")
    return {"function":"native_file_teardown_observation","case":case,"values": {
        "compiled_source_and_build_recorded":0,"exclusive_instrumented_fixture":1,
        "constructor_generations_and_rings_bound":1,"exact_nested_release_sequence":1,
        "allocations":len(allocated),"releases":len(allocated),"transient_releases":2,
        "injected_failures":int(case>=3),"mapping_residual":int(case==3),"descriptor_residual":int(case==4),
        "up_residual":int(case==3),"sync_failed":int(case==5),"delete_succeeded":int(case==2),
        "parent_allocations_released":1,"os_endpoints_bound":1,"fixture_cleanup_complete":1,
        "static_certificate":0,"durability_proved":0}}

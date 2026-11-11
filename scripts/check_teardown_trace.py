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

"""Check bounded native memory teardown traces, never issue static certificates.

The pinned observer/compiler/allocator instrumentation and exclusive execution
are trusted. This checks their records; it is not a proof of instrumentation
completeness or arbitrary C callers. Addresses alone never establish identity.
"""
import json

SCENARIOS = [(0, 0, 0), (2, 0, 0), (1, 1, 0), (2, 3, 0), (2, 3, 1)]


def require(condition, message):
    if not condition:
        raise ValueError(message)


def load_trace(path):
    def unique(pairs):
        result = {}
        for key, value in pairs:
            require(key not in result, "duplicate JSON key")
            result[key] = value
        return result
    return [json.loads(line, object_pairs_hook=unique) for line in path.read_text().splitlines()]


def check_case(records, case, scenario):
    require(bool(records), "missing case")
    for index, record in enumerate(records):
        require(record.get("case") == case and record.get("sequence") == index, "case/sequence mismatch")
    begin = records[0]
    ns, nc, metadata = scenario
    require(begin["kind"] == "begin" and begin["schema_version"] == 1, "invalid begin")
    require((begin["streams"], begin["chunks_per_stream"], begin["metadata"]) == scenario, "unexpected scenario")
    layout = begin["layout"]
    require(all(type(value) is int and value >= 0 for value in layout.values()), "invalid layout")
    allocations = {}
    cursor = 1
    while cursor < len(records) and records[cursor]["kind"] == "allocation":
        record = records[cursor]
        ident, address, size = record["id"], record["address"], record["size"]
        require(type(ident) is int and ident == len(allocations) + 1, "nonfresh allocation generation")
        require(type(address) is int and type(size) is int and address > 0 and size > 0, "invalid allocation")
        require(record["allocator"] in ("malloc", "calloc", "strdup"), "unsupported allocator")
        for old in allocations.values():
            require(address + size <= old["address"] or old["address"] + old["size"] <= address, "overlapping live storage")
        allocations[ident] = record
        cursor += 1
    require(cursor < len(records) and records[cursor]["kind"] == "snapshot", "missing constructor snapshot")
    return check_tree_and_teardown(records, cursor, allocations, case, scenario, layout)


def check_tree_and_teardown(records, cursor, allocations, case, scenario, layout,
                            nullable_chmod=False, nullable_chunk_name=False, stream_memory=1):
    ns, nc, metadata = scenario
    require(type(stream_memory) is int and stream_memory in (0, 1), "invalid stream memory flag")
    require(stream_memory == 1 or nc == 0, "non-memory stream must have no chunk/backend interpretation")
    context = records[cursor]["context"]
    owned = set()

    def claim(ident, role, nullable=False):
        if nullable and ident == 0:
            return
        require(type(ident) is int and ident in allocations and ident not in owned, "missing/aliased owned allocation: " + role)
        owned.add(ident)
        if role + "_size" in layout:
            require(allocations[ident]["size"] == layout[role + "_size"], "allocation layout mismatch")

    def ring(data, parent, offset, owners, node_offset):
        sentinel = allocations[parent]["address"] + layout[offset]
        require(layout[offset] + 16 <= allocations[parent]["size"], "sentinel outside owner")
        require(data["sentinel"] == sentinel, "wrong ring sentinel")
        require([node["owner"] for node in data["nodes"]] == owners, "ring membership/order mismatch")
        addresses = [allocations[owner]["address"] + layout[node_offset] for owner in owners]
        require(data["next"] == (addresses[0] if addresses else sentinel) and data["prev"] == (addresses[-1] if addresses else sentinel), "sentinel links mismatch")
        for index, node in enumerate(data["nodes"]):
            require(layout[node_offset] + 16 <= allocations[owners[index]]["size"], "node outside allocation")
            require(node["address"] == addresses[index], "node identity mismatch")
            require(node["prev"] == (addresses[index-1] if index else sentinel) and node["next"] == (addresses[index+1] if index+1 < len(addresses) else sentinel), "nonreciprocal ring")

    claim(context["id"], "context")
    require(len(context["fields"]) == 6 and context["fields"][:2] == [0, 0] and (context["fields"][2] == 0) == nullable_chmod and context["fields"][3:] == [0, 0, 0], "unexpected context options")
    for ident in context["fields"]:
        claim(ident, "field", True)
    streams = context["streams"]
    require(len(streams) == ns and context["total"] == ns * nc and context["up"] == 0, "constructor counts mismatch")
    for stream in streams:
        claim(stream["id"], "stream")
        claim(stream["name"], "name")
        require(stream["context"] == context["id"] and stream["memory"] == stream_memory, "wrong stream parent/backend")
        require(len(stream["chunks"]) == nc, "wrong chunk count")
        for chunk in stream["chunks"]:
            claim(chunk["id"], "chunk")
            claim(chunk["name"], "name", nullable_chunk_name)
            require((chunk["name"] == 0) == nullable_chunk_name, "unexpected null chunk name")
            claim(chunk["backend"], "backend")
            require(chunk["context"] == context["id"] and chunk["stream"] == stream["id"], "wrong chunk parent")
            require(chunk["locked"] == 0 and chunk["transaction"] == 0, "unsupported chunk state")
            require(len(chunk["fields"]) == 3 and chunk["fields"][0] == 0 and chunk["fields"][1] != 0 and bool(chunk["fields"][2]) == bool(metadata), "wrong backend fields")
            for ident in chunk["fields"]:
                claim(ident, "field", True)
    require(owned == set(allocations), "constructor orphan allocation")
    ring(context["ring"], context["id"], "context_ring", [s["id"] for s in streams], "stream_node")
    for stream in streams:
        ids = [ch["id"] for ch in stream["chunks"]]
        ring(stream["ring"], stream["id"], "stream_ring", ids, "chunk_node")
        ring(stream["up_ring"], stream["id"], "up_ring", ids, "state_node")
        ring(stream["down_ring"], stream["id"], "down_ring", [], "state_node")

    # Derive the entire observable teardown sequence from the pre-state. Exact
    # matching rejects missing/extra calls, wrong nesting, early or repeated frees,
    # stale generations, inconsistent counters and releases before unlinking.
    expected = []
    def event(kind, **fields):
        expected.append(dict(case=case, sequence=cursor + 1 + len(expected), kind=kind, **fields))
    def enter(function):
        event("enter", function=function)
    def leave(function):
        event("exit", function=function)
    def release(ident):
        if ident:
            event("free", id=ident)
    total = ns * nc
    enter("cio_destroy")
    enter("cio_stream_destroy_all")
    for stream in streams:
        enter("cio_stream_destroy")
        enter("cio_chunk_close_stream")
        for chunk in stream["chunks"]:
            enter("cio_chunk_close")
            enter("cio_memfs_close")
            for ident in chunk["fields"]:
                release(ident)
            release(chunk["backend"])
            leave("cio_memfs_close")
            release(chunk["name"])
            event("chunk_state", id=chunk["id"], primary_cleared=1, state_cleared=1, total=total)
            release(chunk["id"])
            enter("cio_chunk_counter_total_sub")
            leave("cio_chunk_counter_total_sub")
            total -= 1
            event("counter", total=total, up=0)
            leave("cio_chunk_close")
        leave("cio_chunk_close_stream")
        release(stream["name"])
        event("stream_state", id=stream["id"], primary_cleared=1, chunks_empty=1, up_empty=1, down_empty=1)
        release(stream["id"])
        leave("cio_stream_destroy")
    leave("cio_stream_destroy_all")
    for ident in context["fields"]:
        release(ident)
    event("context_state", id=context["id"], total=0, up=0, streams_empty=1)
    release(context["id"])
    leave("cio_destroy")
    event("end", live_allocations=0)
    require(records[cursor+1:] == expected, "teardown event sequence differs from constructor ownership tree")
    freed = [record["id"] for record in expected if record["kind"] == "free"]
    require(len(freed) == len(set(freed)) and set(freed) == owned, "release ledger mismatch")
    return {"function": "native_teardown_observation", "case": case, "values": {
        "observed_memory_profile": stream_memory, "constructor_graph_bound": 1,
        "reciprocal_rings_bound": 1, "exact_nested_release_sequence": 1,
        "allocations": len(allocations), "releases": len(freed),
        "chunks": ns * nc, "counter_steps": sum(r["kind"] == "counter" for r in expected),
        "terminal_live": 0, "terminal_total": total,
        "static_certificate": 0,
    }}


def check_trace(records):
    require(bool(records), "empty trace")
    grouped = [[] for _ in SCENARIOS]
    last = 0
    for record in records:
        case = record.get("case")
        require(type(case) is int and last <= case < len(SCENARIOS), "unknown/out-of-order case")
        grouped[case].append(record)
        last = case
    return [check_case(group, case, scenario) for case, (group, scenario) in enumerate(zip(grouped, SCENARIOS))]

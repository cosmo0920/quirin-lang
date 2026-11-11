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

"""Replay the bounded native allocation-failure matrix without upgrading verdicts."""
from check_teardown_trace import require, check_tree_and_teardown

ATTEMPTS = [9, 1, 9, 3, 4, 5, 9, 7, 8, 9]


def check_case(records, case):
    require(bool(records), "missing failure case")
    for index, record in enumerate(records):
        require(record.get("case") == case and record.get("sequence") == index, "case/sequence mismatch")
    begin = records[0]
    require(begin["kind"] == "begin_failure" and begin["schema_version"] == 1 and begin["fail_at"] == case, "invalid failure scenario")
    layout = begin["layout"]
    require(all(type(value) is int and value >= 0 for value in layout.values()), "invalid layout")
    plan = [("calloc", layout["context_size"]), ("strdup", 5),
            ("malloc", layout["stream_size"]), ("strdup", 9),
            ("malloc", layout["chunk_size"]), ("strdup", 8),
            ("calloc", layout["backend_size"]), ("malloc", 64), ("malloc", 4)]
    cursor, generation = 1, 0
    live, acquired, ordinal_ids = {}, [], {}

    def consume(kind, **fields):
        nonlocal cursor
        require(cursor < len(records), "truncated trace")
        expected = dict(case=case, sequence=cursor, kind=kind, **fields)
        require(records[cursor] == expected, "constructor event mismatch")
        cursor += 1

    for ordinal, (allocator, size) in enumerate(plan[:ATTEMPTS[case]], 1):
        failed = int(case == ordinal)
        consume("attempt", ordinal=ordinal, allocator=allocator, size=size, failed=failed)
        if failed:
            continue
        require(cursor < len(records), "missing acquired allocation")
        address = records[cursor].get("address")
        require(type(address) is int and address > 0 and size > 0, "invalid allocation")
        for old in live.values():
            require(address + size <= old["address"] or old["address"] + old["size"] <= address, "overlapping live storage")
        generation += 1
        consume("allocation", id=generation, address=address, size=size, allocator=allocator)
        live[generation] = records[cursor-1]
        acquired.append(generation)
        ordinal_ids[ordinal] = generation
    rollback = {4: [3], 7: [6, 5], 8: [7, 6, 5]}.get(case, [])
    for ordinal in rollback:
        ident = ordinal_ids[ordinal]
        consume("free", id=ident)
        del live[ident]
    context = ordinal_ids.get(1, 0)
    stream = ordinal_ids.get(3, 0) if case != 4 else 0
    chunk = ordinal_ids.get(5, 0) if case not in (7, 8) else 0
    error = 0 if ATTEMPTS[case] >= 6 else -999
    meta_result = (-1 if case == 9 else 0) if chunk else -999
    consume("constructor_result", context=context, stream=stream, chunk=chunk,
            error=error, metadata_result=meta_result, attempts=ATTEMPTS[case])
    if context:
        require(cursor < len(records) and records[cursor]["kind"] == "snapshot", "missing snapshot")
        snapshot = records[cursor]["context"]
        require(snapshot["id"] == context and [s["id"] for s in snapshot["streams"]] == ([stream] if stream else []), "returned ownership mismatch")
        require([ch["id"] for s in snapshot["streams"] for ch in s["chunks"]] == ([chunk] if chunk else []), "returned chunk mismatch")
        outcome = check_tree_and_teardown(records, cursor, live, case,
            (int(bool(stream)), int(bool(chunk)), int(bool(chunk) and case != 9)), layout,
            nullable_chmod=case == 2, nullable_chunk_name=case == 6)
        released = outcome["values"]["releases"] + len(rollback)
    else:
        require(not live, "null context retained storage")
        consume("enter", function="cio_destroy")
        consume("exit", function="cio_destroy")
        consume("end", live_allocations=0)
        require(cursor == len(records), "unexpected terminal events")
        released = 0
    require(len(acquired) == released, "rollback plus teardown ledger mismatch")
    return {"function": "native_constructor_failure_observation", "case": case, "values": {
        "controlled_allocation_failure": 1, "attempts": ATTEMPTS[case],
        "expected_attempts": ATTEMPTS[case], "injected_failures": int(case != 0),
        "expected_failures": int(case != 0), "allocations": len(acquired), "releases": released,
        "rollback_releases": len(rollback), "nullable_chmod": int(case == 2),
        "nullable_chunk_name": int(case == 6), "backend_allocation_failure": int(case in (7, 8)),
        "chunk_null": int(not chunk), "error_zero": int(error == 0),
        "exact_constructor_outcome": 1, "remaining_tree_and_teardown_checked": 1,
        "terminal_live": 0, "static_certificate": 0,
    }}


def check_trace(records):
    require(bool(records), "empty trace")
    grouped = [[] for _ in ATTEMPTS]
    last = 0
    for record in records:
        case = record.get("case")
        require(type(case) is int and last <= case < len(ATTEMPTS), "unknown/out-of-order failure case")
        grouped[case].append(record)
        last = case
    return [check_case(group, case) for case, group in enumerate(grouped)]

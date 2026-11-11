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

"""Exact replay of four native unsupported-type constructor executions."""
from check_teardown_trace import require, check_tree_and_teardown

KINDS = [-2147483648, -1, 2, 2147483647]


def check_case(records, case):
    require(bool(records), "missing unsupported constructor case")
    for index, record in enumerate(records):
        require(record.get("case") == case and record.get("sequence") == index, "case/sequence mismatch")
    begin = records[0]
    require(begin["kind"] == "begin_unsupported" and begin["schema_version"] == 1, "invalid constructor profile")
    require(begin["backend_kind"] == KINDS[case] and begin["memory_kind"] == 1 and begin["filesystem_kind"] == 0, "backend discriminant mismatch")
    layout = begin["layout"]
    require(all(type(value) is int and value >= 0 for value in layout.values()), "invalid layout")
    plan = [("calloc", layout["context_size"]), ("strdup", 5),
            ("malloc", layout["stream_size"]), ("strdup", 9),
            ("malloc", layout["chunk_size"]), ("strdup", 8)]
    live, cursor = {}, 1

    def consume(kind, **fields):
        nonlocal cursor
        require(cursor < len(records), "truncated constructor trace")
        require(records[cursor] == dict(case=case, sequence=cursor, kind=kind, **fields), "constructor event mismatch")
        cursor += 1

    for ident, (allocator, size) in enumerate(plan, 1):
        require(cursor < len(records), "missing allocation")
        address = records[cursor].get("address")
        require(type(address) is int and address > 0 and size > 0 and address + size <= 2**64, "invalid allocation address/size")
        for old in live.values():
            require(address + size <= old["address"] or old["address"] + old["size"] <= address, "overlapping allocations")
        consume("allocation", id=ident, address=address, size=size, allocator=allocator)
        live[ident] = records[cursor-1]
    for ident in [6, 5]:
        consume("free", id=ident)
        del live[ident]
    consume("constructor_result", context=1, stream=3, chunk=0, stream_kind=KINDS[case], error=-999)
    require(cursor < len(records) and records[cursor]["kind"] == "snapshot", "missing surviving tree")
    tree = records[cursor]["context"]
    require(tree["id"] == 1 and [s["id"] for s in tree["streams"]] == [3], "wrong returned identities")
    outcome = check_tree_and_teardown(records, cursor, live, case, (1, 0, 0), layout, stream_memory=0)
    require(outcome["values"]["releases"] == 4, "surviving teardown release mismatch")
    return {"function": "native_unsupported_constructor", "case": case, "values": {
        "selected_unsupported_int_case": 1, "stream_preserves_requested_kind": 1,
        "chunk_null": 1, "error_sentinel_unchanged": 1, "allocations": 6,
        "rollback_releases": 2, "teardown_releases": 4,
        "empty_reciprocal_surviving_tree": 1, "terminal_live": 0,
        "static_certificate": 0,
    }}


def check_trace(records):
    require(bool(records), "empty trace")
    def typed(value):
        require(type(value) in (dict, list, str, int), "invalid trace scalar type")
        if type(value) is dict:
            for child in value.values(): typed(child)
        elif type(value) is list:
            for child in value: typed(child)
    typed(records)
    grouped, last = [[] for _ in KINDS], 0
    for record in records:
        case = record.get("case")
        require(type(case) is int and last <= case < len(KINDS), "unknown/out-of-order case")
        grouped[case].append(record)
        last = case
    return [check_case(group, case) for case, group in enumerate(grouped)]

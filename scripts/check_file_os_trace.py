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

"""Validate twelve exclusive Linux cleanup observations, never static safety.

The observer, native compiler, Linux probes and lack of concurrent descriptor,
mapping or pathname replacement remain explicit trusted premises.
"""
import errno
from check_teardown_trace import load_trace, require


def check_trace(records):
    require(len(records) == 12, "missing/extra OS scenario")
    return [check_case(record, case) for case, record in enumerate(records)]


def check_case(record, case):
    def scalar_types(value):
        if isinstance(value, dict):
            for item in value.values():
                scalar_types(item)
        else:
            require(type(value) in (str, int), "noncanonical scalar")
    scalar_types(record)
    require(set(record) == {"case", "schema_version", "helper", "was_null", "fixture",
            "before", "after", "os_before", "os_after", "returned", "call", "diagnostics",
            "reported_errno", "other_fields_preserved", "path_bytes_preserved",
            "fixture_cleanup_complete"}, "unexpected record shape")
    require(record["case"] == case and record["schema_version"] == 1, "case/schema mismatch")
    helper = "close" if case < 4 else "unmap" if case < 8 else "delete"
    require(record["helper"] == helper and record["was_null"] == int(case in (0, 4)), "wrong helper/null input")
    fixture = record["fixture"]
    require(set(fixture) == {"fd", "map", "size"}, "invalid fixture")
    fd, address, size = fixture["fd"], fixture["map"], fixture["size"]
    require(type(fd) is int and 0 <= fd < 2**31 and type(address) is int and address > 0
            and type(size) is int and size in (4096, 8192, 16384, 32768, 65536)
            and address % size == 0 and address + size <= 2**64, "invalid fixture acquisition")
    before = record["before"]
    path = before["path"]
    require(type(path) is int and 0 < path < 2**64, "missing path identity")
    expected = {"fd": fd if case in (0, 2, 3, 8) else -1,
                "map": address if case in (4, 6, 7, 9) else 0,
                "size": size if case in (4, 5, 6, 9) else 0,
                "warned": int(4 <= case < 8), "path": path}
    require(before == expected, "wrong entry fields")
    after = dict(expected)
    if case == 2:
        after["fd"] = -1
    if case == 6:
        after.update(map=0, size=0, warned=0)
    require(record["after"] == after, "wrong endpoint fields")
    called = case in (2, 3, 6, 7, 10, 11)
    failed = case in (3, 7, 10)
    error = {3: errno.EBADF, 7: errno.EINVAL, 10: errno.ENOENT}.get(case, 0)
    call = {"count": int(called), "operation": {"close": "close", "unmap": "munmap", "delete": "unlink"}[helper] if called else "none",
            "fd": fd if case in (2, 3) else -1,
            "map": address if case in (6, 7) else 0,
            "length": size if case == 6 else 0,
            "path": path if case in (10, 11) else 0,
            "result": -int(failed), "errno": error}
    require(record["call"] == call, "syscall argument/result mismatch")
    require(record["diagnostics"] == int(failed) and record["reported_errno"] == error, "diagnostic mismatch")
    require(record["returned"] == (-1 if case in (0, 3, 4, 7, 8, 9, 10) else 0), "helper return mismatch")
    os_before = record["os_before"]
    device, inode = os_before["witness_device"], os_before["witness_inode"]
    require(type(device) is int and device > 0 and type(inode) is int and inode > 0, "missing inode witness")
    for phase in ("before", "after"):
        descriptor_gone = case == 3 or (phase == "after" and case == 2)
        mapping_gone = phase == "after" and case == 6
        path_gone = case == 10 or (phase == "after" and case == 11)
        probe = {"descriptor": -int(descriptor_gone), "descriptor_errno": errno.EBADF if descriptor_gone else 0,
                 "mapping": -int(mapping_gone), "mapping_errno": errno.ENOMEM if mapping_gone else 0,
                 "path": -int(path_gone), "path_errno": errno.ENOENT if path_gone else 0,
                 "device": 0 if path_gone else device, "inode": 0 if path_gone else inode,
                 "links": 0 if path_gone else 2, "witness_device": device, "witness_inode": inode,
                 "witness_links": 1 if path_gone else 2}
        require(record["os_" + phase] == probe, "OS endpoint/identity mismatch")
    require(record["other_fields_preserved"] == record["path_bytes_preserved"]
            == record["fixture_cleanup_complete"] == 1, "frame or fixture cleanup mismatch")
    return {"function": "native_os_cleanup_observation", "case": case, "values": {
        "exclusive_linux_fixture": 1, "forwarded_syscalls": 1, "compiled_source_and_build_recorded": 0,
        "exact_arguments_and_outcomes": 1, "os_endpoints_and_inode_bound": 1,
        "record_frame_preserved": 1, "fixture_cleanup_complete": 1,
        "observed_close_success": int(case == 2), "descriptor_absent_after": int(case in (2, 3)),
        "observed_unmap_success": int(case == 6), "mapping_absent_after": int(case == 6),
        "observed_delete_success": int(case == 11), "path_absent_after": int(case in (10, 11)),
        "observed_unmap_failure": int(case == 7), "mapping_live_after": int(case != 6),
        "static_certificate": 0, "durability_proved": 0}}

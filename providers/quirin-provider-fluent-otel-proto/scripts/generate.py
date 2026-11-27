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

"""Card OTEL-PROTO-PHASE-B1: write this crate's contract modules and its
protocol identity from the vendored fluent-otel-proto tree (read-only).

    generate.py FLUENT_BIT_TREE PROTO_CLONE CENSUS_JSON [--check]

The declared set is every generated `opentelemetry__proto__*__{init,
get_packed_size,pack,unpack,free_unpacked}` function an OpenTelemetry
consumer unit of the tree calls, plus the four protobuf-c runtime entry
points those wrappers reach. Each function's prototype is read from its
generated header and its body from its generated source, and both must have
exactly the protoc-c 1.5 shape its family states (below), or the run is
refused: a declaration is written only for a body read. The protocol
identity (`protocol-identity.json`) is the census's pinned opentelemetry-proto
identity (re-checked against the clone) plus the sha256 of every compiled
`.pb-c.c`/`.pb-c.h` and of `protobuf-c.c`/`protobuf-c.h`, on the vendored
basis. `--check` rewrites nothing and exits 1 on any difference.
"""

from __future__ import annotations

import glob
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

CRATE = Path(__file__).resolve().parents[1]
CONSUMERS = [
    "lib/cmetrics/src/cmt_*opentelemetry*.c", "lib/ctraces/src/ctr_*opentelemetry.c",
    "src/opentelemetry/*.c", "plugins/in_opentelemetry/*.c", "plugins/out_opentelemetry/*.c",
    "plugins/processor_opentelemetry_envelope/*.c", "lib/cprofiles/cprof_*opentelemetry.c",
]
PBC = "lib/fluent-otel-proto/proto_c"
STALE = "opentelemetry/proto/profiles/v1development/pprofextended.pb-c"
FAMILIES = ("init", "get_packed_size", "pack", "unpack", "free_unpacked")
CALL = re.compile(r"\b(opentelemetry__proto__\w+?__(?:" + "|".join(FAMILIES) + r"))\s*\(")
RUNTIME = ("protobuf_c_message_free_unpacked", "protobuf_c_message_get_packed_size",
           "protobuf_c_message_pack", "protobuf_c_message_unpack")

# The C declaration shape of each family (whitespace-normalised), with the
# extern declaration and the custody clauses it is declared with.
SHAPES = {
    "init": (r"void {f} \((?P<T>\w+) \*message\)",
             "message: ptr<mut, u8>", "unit",
             ["resource_borrowed(message)", "resource_return_none()"]),
    "get_packed_size": (r"size_t {f} \(const (?P<T>\w+) \*message\)",
                        "message: ptr<const, u8>", "usize",
                        ["resource_borrowed(message)", "resource_return_none()"]),
    "pack": (r"size_t {f} \(const (?P<T>\w+) \*message, uint8_t \*out\)",
             "message: ptr<const, u8>, out: ptr<mut, u8>", "usize",
             ["resource_borrowed(message)", "resource_borrowed(out)", "resource_return_none()"]),
    "unpack": (r"(?P<T>\w+) \* {f} \(ProtobufCAllocator \*allocator, size_t len, const uint8_t \*data\)",
               "allocator: ptr<mut, u8>, len: usize, data: ptr<const, u8>", "ptr<mut, u8>",
               ["resource_borrowed(allocator)", "resource_borrowed(len)", "resource_borrowed(data)",
                "resource_return_owned(nullable)"]),
    "free_unpacked": (r"void {f} \((?P<T>\w+) \*message, ProtobufCAllocator \*allocator\)",
                      "message: ptr<mut, u8>, allocator: ptr<mut, u8>", "unit",
                      ["resource_released(message, nullable)", "resource_borrowed(allocator)",
                       "resource_return_none()"]),
}
# The body each family must have (whitespace-normalised; D the descriptor).
BODIES = {
    "init": r"\{ static const {T} init_value = \w+__INIT; \*message = init_value; \}",
    "get_packed_size": (r"\{ assert\(message->base\.descriptor == &{D}\); return "
                        r"protobuf_c_message_get_packed_size \(\(const ProtobufCMessage\*\)\(message\)\); \}"),
    "pack": (r"\{ assert\(message->base\.descriptor == &{D}\); return "
             r"protobuf_c_message_pack \(\(const ProtobufCMessage\*\)message, out\); \}"),
    "unpack": r"\{ return \({T} \*\) protobuf_c_message_unpack \(&{D}, allocator, len, data\); \}",
    "free_unpacked": (r"\{ if\(!message\) return; assert\(message->base\.descriptor == &{D}\); "
                      r"protobuf_c_message_free_unpacked \(\(ProtobufCMessage\*\)message, allocator\); \}"),
}
RUNTIME_SHAPES = {
    "protobuf_c_message_unpack": (
        r"ProtobufCMessage \* protobuf_c_message_unpack\(const ProtobufCMessageDescriptor \*descriptor, "
        r"ProtobufCAllocator \*allocator, size_t len, const uint8_t \*data\);",
        "descriptor: ptr<const, u8>, allocator: ptr<mut, u8>, len: usize, data: ptr<const, u8>",
        "ptr<mut, u8>",
        ["resource_borrowed(descriptor)", "resource_borrowed(allocator)", "resource_borrowed(len)",
         "resource_borrowed(data)", "resource_return_owned(nullable)"]),
    "protobuf_c_message_free_unpacked": (
        r"void protobuf_c_message_free_unpacked\(ProtobufCMessage \*message, ProtobufCAllocator \*allocator\);",
        "message: ptr<mut, u8>, allocator: ptr<mut, u8>", "unit",
        ["resource_released(message, nullable)", "resource_borrowed(allocator)", "resource_return_none()"]),
    "protobuf_c_message_pack": (
        r"size_t protobuf_c_message_pack\(const ProtobufCMessage \*message, uint8_t \*out\);",
        "message: ptr<const, u8>, out: ptr<mut, u8>", "usize",
        ["resource_borrowed(message)", "resource_borrowed(out)", "resource_return_none()"]),
    "protobuf_c_message_get_packed_size": (
        r"size_t protobuf_c_message_get_packed_size\(const ProtobufCMessage \*message\);",
        "message: ptr<const, u8>", "usize",
        ["resource_borrowed(message)", "resource_return_none()"]),
}

LICENSE = """// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
"""


class Refused(ValueError):
    pass


def norm(text: str) -> str:
    text = re.sub(r"/\*.*?\*/", " ", text, flags=re.S)
    text = re.sub(r"\s+", " ", text).strip()
    text = re.sub(r"\(\s+", "(", text)
    text = re.sub(r"\s+\)", ")", text)
    text = re.sub(r"\s*,\s*", ", ", text)
    return text


def consumer_calls(tree: Path) -> list[str]:
    names = set()
    for pattern in CONSUMERS:
        for path in sorted(glob.glob(str(tree / pattern))):
            names.update(CALL.findall(Path(path).read_text(errors="replace")))
    return sorted(names)


def generated_sources(tree: Path) -> dict[str, str]:
    root = tree / PBC
    return {str(p.relative_to(root)): p.read_text()
            for p in sorted(root.rglob("*.pb-c.[ch]")) if STALE not in str(p)}


def read_function(name: str, family: str, sources: dict[str, str]) -> dict:
    proto_hits, body_hits = [], []
    for path, text in sources.items():
        if path.endswith(".h"):
            for m in re.finditer(r"[^;{}]*\b" + re.escape(name) + r"\s*\([^;{}]*\);", text):
                proto_hits.append((path, norm(m.group(0))))
        else:
            for m in re.finditer(r"[^;{}]*\b" + re.escape(name) + r"\s*\([^;{}]*\)\s*\{.*?\n\}", text, re.S):
                body_hits.append((path, norm(m.group(0))))
    if len(proto_hits) != 1 or len(body_hits) != 1:
        raise Refused(f"{name}: {len(proto_hits)} prototypes, {len(body_hits)} definitions")
    shape, params, result, clauses = SHAPES[family]
    proto = proto_hits[0][1].rstrip(";")
    m = re.fullmatch(shape.format(f=re.escape(name)), proto)
    if not m:
        raise Refused(f"{name}: prototype `{proto}` is not the {family} shape")
    ctype = m.group("T")
    descriptor = name[: -len(family) - 2] + "__descriptor"
    head, _, body = body_hits[0][1].partition(") {")
    # the match may start at the preceding preprocessor lines: the definition
    # head must end with the prototype
    if not norm(head + ")").endswith(proto):
        raise Refused(f"{name}: definition head differs from its prototype")
    want = BODIES[family].replace("{T}", re.escape(ctype)).replace("{D}", re.escape(descriptor))
    if not re.fullmatch(want, "{" + body):
        raise Refused(f"{name}: body is not the protoc-c 1.5 {family} body: {{{body}")
    return {"function": name, "family": family, "type": ctype, "header": proto_hits[0][0],
            "source": body_hits[0][0], "params": params, "result": result, "clauses": clauses}


def declaration(name: str, params: str, result: str, clauses: list[str]) -> str:
    lines = [f"extern fn {name}({params}) -> {result}", "contract {", "    requires resource_contract_v1;"]
    lines += [f"    ensures normal {c};" for c in clauses]
    lines.append("};")
    return "\n".join(lines)


GENERATED_HEADER = LICENSE + """
module contract;

// fluent-otel-proto generated functions — resource-projection vocabulary
// version 1. Card OTEL-PROTO-PHASE-B1 (USER DECISIONS 2026-11-27, "New
// provider crate", "Phase it, accept 7"). Written by
// `scripts/generate.py`; never edit by hand.
//
// {count} TRUSTED ASSUMPTIONS, one per generated function an OpenTelemetry
// consumer unit of Fluent Bit calls ({families}). The C bodies are not
// analysed by this loader; each declaration is pinned (contract spec C06.2)
// to its definition in `lib/fluent-otel-proto/proto_c/` and its prototype in
// the generated header, so a regenerated file holds every declaration whose
// site changed (`contract-pin-mismatch`, Unknown). The basis is the vendored
// descriptors, which equal opentelemetry-proto v1.10.0
// (`protocol-identity.json`, docs/reports/otel-proto-audit/README.md §3).
//
// ## The bodies, read before declaring (protoc-c 1.5.2; every one checked)
//
//     void   T__init (T *message)
//     {{ static const T init_value = T__INIT; *message = init_value; }}
//     size_t T__get_packed_size (const T *message)
//     {{ assert(...); return protobuf_c_message_get_packed_size ((const ProtobufCMessage*)(message)); }}
//     size_t T__pack (const T *message, uint8_t *out)
//     {{ assert(...); return protobuf_c_message_pack ((const ProtobufCMessage*)message, out); }}
//     T *    T__unpack (ProtobufCAllocator *allocator, size_t len, const uint8_t *data)
//     {{ return (T *) protobuf_c_message_unpack (&T__descriptor, allocator, len, data); }}
//     void   T__free_unpacked (T *message, ProtobufCAllocator *allocator)
//     {{ if(!message) return; assert(...); protobuf_c_message_free_unpacked ((ProtobufCMessage*)message, allocator); }}
//
// * `__init` copies a static initializer into memory the caller holds:
//   `message` borrowed, no custody returned.
// * `__get_packed_size` and `__pack` read the message (and `__pack` writes the
//   caller's buffer): every parameter borrowed, no custody returned.
// * `__unpack` returns a fresh message allocated through `allocator` (the
//   system allocator when NULL), or NULL when the bytes do not decode or an
//   allocation fails (`protobuf_c_message_unpack`, `protobuf-c.c:3028`): an
//   owned, nullable result. protobuf-c.h:647-651: "calls to functions like
//   protobuf_c_message_unpack() which return a ProtobufCMessage must be paired
//   with a call to a free function".
// * `__free_unpacked` returns at once on NULL and otherwise releases the
//   message and everything it owns through the same allocator: a nullable
//   release of `message`, the allocator borrowed.
//
// ## ABI (constraint 8)
//
// A pointer to const is `ptr<const, u8>`, any other pointer `ptr<mut, u8>`,
// `size_t` `usize` (LP64), `void` `unit`.
//
// ## NOT CLAIMED
//
// * The protocol facts of an unpacked message (an absent sub-message is NULL,
//   an element below the count is non-NULL, the selected oneof member is
//   valid): they are the OTP premises of
//   docs/reports/otel-proto-audit/formulas.json, which no verdict reads yet
//   (OTEL-PROTO-NULL-CONDITIONAL-READING).
// * That an `__unpack` result is released by the `__free_unpacked` of its own
//   type, or that the allocator passed to both is the same: the custody model
//   keys a release on the value, not on the message type.
// * Anything about `message` being non-NULL where a function asserts it.
"""

RUNTIME_HEADER = LICENSE + """
module contract;

// protobuf-c runtime entry points — resource-projection vocabulary version 1.
// Card OTEL-PROTO-PHASE-B1. Written by `scripts/generate.py`; never edit by
// hand.
//
// FOUR TRUSTED ASSUMPTIONS: the runtime functions the generated wrappers of
// `generated.qv` reach, read from the vendored protobuf-c 1.5.0
// (`lib/fluent-otel-proto/proto_c/protobuf-c/protobuf-c.c`, prototypes in
// `protobuf-c.h`). The runtime unit is on no measured route; these are
// declared so the wrappers' callees are contracted wherever a route reads them.
//
// * `protobuf_c_message_unpack` (:3028): a message allocated through
//   `allocator`, or NULL on a decode or allocation failure (every failure
//   path frees what it allocated and returns NULL): owned, nullable.
// * `protobuf_c_message_free_unpacked` (:3324): `if (message == NULL) return;`
//   then frees the message and its members: a nullable release.
// * `protobuf_c_message_pack` (:1472), `protobuf_c_message_get_packed_size`:
//   read the message (and write the caller's buffer): borrowed.
//
// No libc name is declared here: allocation goes through the allocator, and
// libc's own declarations stay in quirin-provider-libc.
"""


def protocol_identity(tree: Path, clone: Path, census: Path) -> dict:
    pinned = json.loads(census.read_text())["protocol_identity"]
    commit = subprocess.run(["git", "-C", str(clone), "rev-parse", "HEAD"], check=True,
                            capture_output=True, text=True).stdout.strip()
    files = {str(p.relative_to(clone)): hashlib.sha256(p.read_bytes()).hexdigest()
             for p in sorted((clone / "opentelemetry").rglob("*.proto"))}
    if commit != pinned["commit"] or files != pinned["files"]:
        raise Refused("the protocol clone differs from the census's pinned identity")
    root = tree / PBC
    vendored = {}
    for p in sorted(root.rglob("*")):
        rel = str(p.relative_to(root))
        if p.is_file() and (rel.endswith((".pb-c.c", ".pb-c.h")) and STALE not in rel
                            or rel in ("protobuf-c/protobuf-c.c", "protobuf-c/protobuf-c.h")):
            vendored[rel] = hashlib.sha256(p.read_bytes()).hexdigest()
    return {
        "schema": "quirin-protocol-identity/1",
        "basis": "vendored fluent-otel-proto descriptors = opentelemetry-proto v1.10.0 "
                 "(adopted by the user and the integrator, 2026-11-27)",
        "protocol": {"repository": "open-telemetry/opentelemetry-proto", "commit": pinned["commit"],
                     "files": pinned["files"]},
        "vendored_root": PBC,
        "vendored": vendored,
        "excluded": [STALE + ".c", STALE + ".h"],
    }


def build(tree: Path, clone: Path, census: Path) -> dict[str, str]:
    sources = generated_sources(tree)
    functions = []
    for name in consumer_calls(tree):
        family = next(f for f in sorted(FAMILIES, key=len, reverse=True) if name.endswith("__" + f))
        functions.append(read_function(name, family, sources))
    runtime_h = norm((tree / PBC / "protobuf-c/protobuf-c.h").read_text())
    runtime = []
    for name in RUNTIME:
        shape, params, result, clauses = RUNTIME_SHAPES[name]
        if not re.search(shape.replace(" ", r"\s*"), runtime_h.replace("PROTOBUF_C__API ", "")):
            raise Refused(f"{name}: prototype is not the protobuf-c 1.5 shape")
        runtime.append((name, params, result, clauses))
    counts = {f: sum(1 for x in functions if x["family"] == f) for f in FAMILIES}
    families = ", ".join(f"{counts[f]} `__{f}`" for f in FAMILIES)
    generated = GENERATED_HEADER.format(count=len(functions), families=families)
    for x in functions:
        generated += f"\n// {x['header']} / {x['source']}\n"
        generated += declaration(x["function"], x["params"], x["result"], x["clauses"]) + "\n"
    runtime_text = RUNTIME_HEADER
    for name, params, result, clauses in runtime:
        runtime_text += "\n" + declaration(name, params, result, clauses) + "\n"
    identity = protocol_identity(tree, clone, census)
    identity["declared"] = {"generated": [x["function"] for x in functions], "runtime": list(RUNTIME)}
    return {
        "contracts/fluent_otel_proto/generated.qv": generated,
        "contracts/fluent_otel_proto/runtime.qv": runtime_text,
        "protocol-identity.json": json.dumps(identity, indent=1, sort_keys=True) + "\n",
    }


def main(argv: list[str]) -> int:
    check = "--check" in argv
    args = [a for a in argv if a != "--check"]
    if len(args) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    try:
        out = build(Path(args[0]), Path(args[1]), Path(args[2]))
    except Refused as error:
        print(f"refused: {error}", file=sys.stderr)
        return 1
    status = 0
    for rel, text in out.items():
        path = CRATE / rel
        if check:
            if not path.exists() or path.read_text() != text:
                print(f"stale: {rel}", file=sys.stderr)
                status = 1
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text)
    return status


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

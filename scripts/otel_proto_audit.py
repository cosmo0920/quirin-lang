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

"""Card OTEL-PROTO-AUDIT: the protocol identity, the conformance fact table and
the route census of docs/reports/otel-proto-audit/.

Subcommands (every input read-only):

* ``identity PROTO_ROOT`` prints the protocol identity (the clone's commit and
  each ``.proto`` file's sha256); ``identity PROTO_ROOT --check CENSUS`` refuses
  (exit 1) unless the identity equals the one pinned in CENSUS. A later use of
  the pinned protocol rests on that check: a differing identity is refused,
  never reconciled.
* ``conformance PROTO_ROOT PBC_ROOT`` compares the ``.proto`` declarations with
  the protobuf-c generated descriptors (``*.pb-c.c``) and prints the fact table:
  messages, enums and values present on one side only, and per field its
  number, protobuf-c type, label, oneof, packing and deprecation.
* ``census MEASURE_TIP`` joins the four pinned routes' rows (corpus,
  twelve-home, system, slice) for the OTel modules' functions.

The parsers are total-or-reject: a construct outside the subset the pinned
files use (``map<>``, ``group``, ``extend``, proto2) is refused, never skipped.
"""

from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import re
import subprocess
import sys
from collections import Counter
from pathlib import Path

SCALARS = {
    "double", "float", "int32", "int64", "uint32", "uint64", "sint32", "sint64",
    "fixed32", "fixed64", "sfixed32", "sfixed64", "bool", "string", "bytes",
}
# Scalar types whose repeated form proto3 packs by default.
PACKABLE = SCALARS - {"string", "bytes"}


class Refused(ValueError):
    """A construct outside the parsed subset, or an inconsistent input."""


# --------------------------------------------------------------------------
# Protocol identity


def proto_files(root: Path) -> list[Path]:
    return sorted(p for p in (root / "opentelemetry").rglob("*.proto"))


def identity(root: Path, commit: str | None = None) -> dict:
    if commit is None:
        commit = subprocess.run(
            ["git", "-C", str(root), "rev-parse", "HEAD"],
            check=True, capture_output=True, text=True,
        ).stdout.strip()
    files = {
        str(p.relative_to(root)): hashlib.sha256(p.read_bytes()).hexdigest()
        for p in proto_files(root)
    }
    if not files:
        raise Refused(f"{root}: no .proto files")
    return {"commit": commit, "files": files}


def check_identity(pinned: dict, actual: dict) -> list[str]:
    """Every difference between the pinned and the actual identity; empty
    only when they are equal (commit, file set and every digest)."""
    problems = []
    if pinned.get("commit") != actual.get("commit"):
        problems.append(f"commit {actual.get('commit')} != pinned {pinned.get('commit')}")
    p, a = pinned.get("files", {}), actual.get("files", {})
    for name in sorted(set(p) | set(a)):
        if name not in a:
            problems.append(f"{name}: pinned, absent")
        elif name not in p:
            problems.append(f"{name}: present, not pinned")
        elif p[name] != a[name]:
            problems.append(f"{name}: sha256 {a[name]} != pinned {p[name]}")
    if not p:
        problems.append("no pinned files")
    return problems


# --------------------------------------------------------------------------
# .proto reader (the proto3 subset the pinned files use)

TOKEN = re.compile(r'\s+|//[^\n]*|/\*.*?\*/|"(?:[^"\\]|\\.)*"|[A-Za-z_][A-Za-z0-9_.]*|-?0[xX][0-9A-Fa-f]+|-?\d+|[{}\[\]()<>;=,]', re.S)


def tokens(text: str) -> list[str]:
    out, pos = [], 0
    while pos < len(text):
        m = TOKEN.match(text, pos)
        if not m:
            raise Refused(f"unlexable input at offset {pos}: {text[pos:pos + 20]!r}")
        t = m.group(0)
        pos = m.end()
        if t.isspace() or t.startswith("//") or t.startswith("/*"):
            continue
        out.append(t)
    return out


class _Reader:
    def __init__(self, toks: list[str]):
        self.t, self.i = toks, 0

    def peek(self) -> str | None:
        return self.t[self.i] if self.i < len(self.t) else None

    def take(self, want: str | None = None) -> str:
        if self.i >= len(self.t):
            raise Refused("unexpected end of input")
        tok = self.t[self.i]
        if want is not None and tok != want:
            raise Refused(f"expected {want!r}, found {tok!r}")
        self.i += 1
        return tok

    def skip_statement(self) -> None:
        depth = 0
        while True:
            tok = self.take()
            if tok in "{[(":
                depth += 1
            elif tok in "}])":
                depth -= 1
            elif tok == ";" and depth == 0:
                return


def _options(r: _Reader) -> dict:
    opts = {}
    if r.peek() != "[":
        return opts
    r.take("[")
    while True:
        name = r.take()
        r.take("=")
        opts[name] = r.take()
        if r.peek() == ",":
            r.take(",")
            continue
        r.take("]")
        return opts


def parse_proto(text: str) -> dict:
    """One file: package, messages and enums with unresolved field types."""
    r = _Reader(tokens(text))
    out = {"package": "", "syntax": None, "messages": {}, "enums": {}}
    while r.peek() is not None:
        tok = r.take()
        if tok == ";":
            continue
        if tok == "syntax":
            r.take("=")
            out["syntax"] = r.take().strip('"')
            r.take(";")
        elif tok == "package":
            out["package"] = r.take()
            r.take(";")
        elif tok in ("import", "option"):
            r.skip_statement()
        elif tok == "message":
            _message(r, out["package"], out)
        elif tok == "enum":
            _enum(r, out["package"], out)
        elif tok == "service":
            name = r.take()
            r.take("{")
            depth = 1
            while depth:
                t = r.take()
                depth += {"{": 1, "}": -1}.get(t, 0)
            out.setdefault("services", []).append(name)
        else:
            raise Refused(f"top-level {tok!r} is outside the parsed subset")
    if out["syntax"] != "proto3":
        raise Refused(f"syntax {out['syntax']!r}: only proto3 is parsed")
    return out


def _enum(r: _Reader, scope: str, out: dict) -> None:
    name = r.take()
    full = f"{scope}.{name}" if scope else name
    r.take("{")
    values = {}
    while r.peek() != "}":
        tok = r.take()
        if tok in ("option", "reserved"):
            r.skip_statement()
            continue
        r.take("=")
        number = int(r.take(), 0)
        opts = _options(r)
        r.take(";")
        values[tok] = {"number": number, "deprecated": opts.get("deprecated") == "true"}
    r.take("}")
    out["enums"][full] = {"values": values}


def _message(r: _Reader, scope: str, out: dict) -> None:
    name = r.take()
    full = f"{scope}.{name}" if scope else name
    r.take("{")
    msg = {"fields": [], "scope": full, "reserved": []}
    out["messages"][full] = msg
    oneof = None
    while True:
        tok = r.take()
        if tok == "}":
            if oneof is not None:
                oneof = None
                continue
            return
        if tok == ";":
            continue  # an empty statement (`};` after a nested declaration)
        if tok == "message":
            _message(r, full, out)
        elif tok == "enum":
            _enum(r, full, out)
        elif tok == "oneof":
            if oneof is not None:
                raise Refused("nested oneof")
            oneof = r.take()
            r.take("{")
        elif tok == "option":
            r.skip_statement()
        elif tok == "reserved":
            start = r.i
            r.skip_statement()
            msg["reserved"].append(" ".join(r.t[start:r.i - 1]))
        elif tok in ("map", "group", "extend", "extensions", "required"):
            raise Refused(f"{full}: {tok!r} is outside the parsed subset")
        else:
            label = "none"
            if tok in ("optional", "repeated"):
                if oneof is not None:
                    raise Refused(f"{full}: label inside oneof")
                label = tok
                tok = r.take()
            ftype = tok
            fname = r.take()
            r.take("=")
            number = int(r.take(), 0)
            opts = _options(r)
            r.take(";")
            msg["fields"].append({
                "name": fname, "number": number, "label": label, "type": ftype,
                "oneof": oneof, "deprecated": opts.get("deprecated") == "true",
                "packed_option": opts.get("packed"),
            })


def _presence(label: str, kind: str, oneof: str | None) -> str:
    """proto3 field presence: `repeated`, `explicit` (a singular message, a
    oneof member or an `optional` field) or `implicit` (a singular scalar:
    zero is absence)."""
    if label == "repeated":
        return "repeated"
    if label == "optional" or kind == "message" or oneof is not None:
        return "explicit"
    return "implicit"


def resolve_protos(files: dict[str, str]) -> dict:
    """All files: every field type resolved by protobuf scoping (innermost
    scope outward, then fully qualified); an unresolved name is refused."""
    messages, enums, origin = {}, {}, {}
    for path, text in sorted(files.items()):
        try:
            parsed = parse_proto(text)
        except Refused as error:
            raise Refused(f"{path}: {error}") from None
        for k, v in parsed["messages"].items():
            if k in messages:
                raise Refused(f"{k}: declared twice")
            messages[k] = v
            origin[k] = path
        for k, v in parsed["enums"].items():
            enums[k] = v
            origin[k] = path

    def lookup(name: str, scope: str) -> tuple[str, str]:
        if name.startswith("."):
            cands = [name[1:]]
        else:
            parts = scope.split(".")
            cands = [".".join(parts[:i] + [name]) for i in range(len(parts), -1, -1)]
        for c in cands:
            if c in messages:
                return "message", c
            if c in enums:
                return "enum", c
        raise Refused(f"type {name!r} in {scope} does not resolve")

    for full, msg in messages.items():
        for f in msg["fields"]:
            if f["type"] in SCALARS:
                f["kind"], f["type_name"] = f["type"], None
            else:
                f["kind"], f["type_name"] = lookup(f["type"], full)
            f["presence"] = _presence(f["label"], f["kind"], f["oneof"])
            packed = f["packed_option"]
            f["packed"] = (f["label"] == "repeated" and f["kind"] in PACKABLE | {"enum"}
                           and packed != "false")
    return {"messages": messages, "enums": enums, "origin": origin}


# --------------------------------------------------------------------------
# protobuf-c generated descriptors (*.pb-c.c)

FIELD_ARRAY = re.compile(
    r"static const ProtobufCFieldDescriptor (\w+)__field_descriptors\[(\d+)\]\s*=\s*\{(.*?)\n\};", re.S)
FIELD_ENTRY = re.compile(
    r"\{\s*\"(\w+)\",\s*(\d+),\s*PROTOBUF_C_LABEL_(\w+),\s*PROTOBUF_C_TYPE_(\w+),\s*"
    r"(?:offsetof\(\w+,\s*(\w+)\)|0),\s*(?:/\*[^*]*\*/)?\s*offsetof\(\w+,\s*(\w+)\),\s*"
    r"(NULL|&(\w+)),\s*([^,]+),\s*([^,/]+?)\s*,?\s*/\* flags \*/", re.S)
MESSAGE_DESC = re.compile(
    r"const ProtobufCMessageDescriptor (\w+)__descriptor\s*=\s*\{\s*PROTOBUF_C__MESSAGE_DESCRIPTOR_MAGIC,\s*"
    r"\"([\w.]+)\",\s*\"(\w+)\",\s*\"(\w+)\",\s*\"([\w.]*)\",\s*sizeof\(\w+\),\s*(\d+),", re.S)
ENUM_VALUES = re.compile(
    r"static const ProtobufCEnumValue (\w+)__enum_values_by_number\[(\d+)\]\s*=\s*\{(.*?)\n\};", re.S)
ENUM_ENTRY = re.compile(r"\{\s*\"(\w+)\",\s*\"(\w+)\",\s*(-?\d+)\s*\}")
ENUM_DESC = re.compile(
    r"const ProtobufCEnumDescriptor (\w+)__descriptor\s*=\s*\{\s*PROTOBUF_C__ENUM_DESCRIPTOR_MAGIC,\s*"
    r"\"([\w.]+)\",", re.S)
PBC_TYPES = {
    "INT32": "int32", "SINT32": "sint32", "SFIXED32": "sfixed32", "INT64": "int64",
    "SINT64": "sint64", "SFIXED64": "sfixed64", "UINT32": "uint32", "FIXED32": "fixed32",
    "UINT64": "uint64", "FIXED64": "fixed64", "FLOAT": "float", "DOUBLE": "double",
    "BOOL": "bool", "ENUM": "enum", "STRING": "string", "BYTES": "bytes", "MESSAGE": "message",
}
COMPILER = re.compile(r"#elif (\d+) < PROTOBUF_C_MIN_COMPILER_VERSION")
GENERATED_FROM = re.compile(r"/\* Generated from: (\S+) \*/")


def parse_pbc(files: dict[str, str]) -> dict:
    """Descriptors of every generated file, keyed by full proto name; field
    type references resolved through the descriptor symbols."""
    symbol_full: dict[str, str] = {}
    raw_messages, enums, meta = {}, {}, {}
    for path, text in sorted(files.items()):
        gen = GENERATED_FROM.search(text)
        meta[path] = {"generated_from": gen.group(1) if gen else None}
        for m in MESSAGE_DESC.finditer(text):
            if m.group(2) in raw_messages:
                raise Refused(f"{path}: {m.group(2)} is also generated in "
                              f"{raw_messages[m.group(2)]['file']} (exclude the file not compiled)")
            symbol_full[m.group(1) + "__descriptor"] = m.group(2)
            raw_messages[m.group(2)] = {"symbol": m.group(1), "n_fields": int(m.group(6)),
                                        "fields": [], "file": path}
        for m in ENUM_DESC.finditer(text):
            if m.group(1) + "__descriptor" in symbol_full:
                raise Refused(f"{path}: enum {m.group(2)} is generated twice")
            symbol_full[m.group(1) + "__descriptor"] = m.group(2)
        by_symbol = {v["symbol"]: k for k, v in raw_messages.items() if v["file"] == path}
        for m in FIELD_ARRAY.finditer(text):
            full = by_symbol.get(m.group(1))
            if full is None:
                raise Refused(f"{path}: field array {m.group(1)} without a message descriptor")
            entries = FIELD_ENTRY.findall(m.group(3))
            if len(entries) != int(m.group(2)):
                raise Refused(f"{path}: {m.group(1)}: read {len(entries)} of {m.group(2)} fields")
            raw_messages[full]["raw"] = entries
        enum_symbol = {}
        for m in ENUM_DESC.finditer(text):
            enum_symbol[m.group(1)] = m.group(2)
        for m in ENUM_VALUES.finditer(text):
            full = enum_symbol.get(m.group(1))
            if full is None:
                raise Refused(f"{path}: enum values {m.group(1)} without an enum descriptor")
            entries = ENUM_ENTRY.findall(m.group(3))
            if len(entries) != int(m.group(2)):
                raise Refused(f"{path}: {m.group(1)}: read {len(entries)} of {m.group(2)} values")
            enums[full] = {"values": {n: {"number": int(v), "deprecated": False} for n, _c, v in entries}}
    messages = {}
    for full, msg in raw_messages.items():
        if "raw" not in msg and msg["n_fields"]:
            raise Refused(f"{full}: no field array")
        fields = []
        for (name, number, label, ptype, quant, _member, ref, refsym, _default, flags) in msg.get("raw", []):
            flagset = {f.strip() for f in flags.split("|")} - {"0", ""}
            oneof = None
            if "PROTOBUF_C_FIELD_FLAG_ONEOF" in flagset:
                if not quant or not quant.endswith("_case"):
                    raise Refused(f"{full}.{name}: oneof flag without a case quantifier")
                oneof = quant[: -len("_case")]
            kind = PBC_TYPES.get(ptype)
            if kind is None:
                raise Refused(f"{full}.{name}: protobuf-c type {ptype}")
            type_name = None
            if kind in ("message", "enum"):
                if ref == "NULL" or refsym not in symbol_full:
                    raise Refused(f"{full}.{name}: unresolved descriptor {ref}")
                type_name = symbol_full[refsym]
            lab = {"NONE": "none", "OPTIONAL": "optional", "REPEATED": "repeated"}.get(label)
            if lab is None:
                raise Refused(f"{full}.{name}: label {label} (proto2 required is not parsed)")
            # protobuf-c spells a singular message field and a oneof member
            # PROTOBUF_C_LABEL_OPTIONAL: the label read back is the .proto's.
            pbc_label = lab
            if oneof is not None or (lab == "optional" and kind == "message" and not quant):
                lab = "none"
            fields.append({
                "pbc_label": pbc_label, "presence": _presence(lab, kind, oneof),
                "name": name, "number": int(number), "label": lab, "kind": kind,
                "type_name": type_name, "oneof": oneof,
                "deprecated": "PROTOBUF_C_FIELD_FLAG_DEPRECATED" in flagset,
                "packed": "PROTOBUF_C_FIELD_FLAG_PACKED" in flagset,
            })
        messages[full] = {"fields": fields, "file": msg["file"]}
    return {"messages": messages, "enums": enums, "files": meta}


def compiler_versions(headers: dict[str, str]) -> dict[str, int | None]:
    out = {}
    for path, text in sorted(headers.items()):
        m = COMPILER.search(text)
        out[path] = int(m.group(1)) if m else None
    return out


# --------------------------------------------------------------------------
# Comparison

FIELD_KEYS = ("name", "kind", "type_name", "label", "presence", "oneof", "packed", "deprecated")


def compare(upstream: dict, vendored: dict) -> list[dict]:
    """The fact table: one record per difference. `side` says where the
    declaration exists; `aspect` names what differs."""
    diffs = []
    um, vm = upstream["messages"], vendored["messages"]
    for full in sorted(set(um) | set(vm)):
        if full not in vm:
            diffs.append({"message": full, "aspect": "message-absent-in-vendored",
                          "upstream_fields": len(um[full]["fields"])})
            continue
        if full not in um:
            diffs.append({"message": full, "aspect": "message-absent-upstream",
                          "vendored_fields": len(vm[full]["fields"])})
            continue
        uf = {f["number"]: f for f in um[full]["fields"]}
        vf = {f["number"]: f for f in vm[full]["fields"]}
        for number in sorted(set(uf) | set(vf)):
            if number not in vf:
                f = uf[number]
                diffs.append({"message": full, "field": f["name"], "number": number,
                              "aspect": "field-absent-in-vendored",
                              "upstream": {k: f[k] for k in FIELD_KEYS}})
            elif number not in uf:
                f = vf[number]
                diffs.append({"message": full, "field": f["name"], "number": number,
                              "aspect": "field-absent-upstream",
                              "vendored": {k: f[k] for k in FIELD_KEYS},
                              "reserved_upstream": um[full].get("reserved", [])})
            else:
                for key in FIELD_KEYS:
                    if uf[number][key] != vf[number][key]:
                        diffs.append({"message": full, "field": uf[number]["name"], "number": number,
                                      "aspect": f"field-{key}",
                                      "upstream": uf[number][key], "vendored": vf[number][key]})
    ue, ve = upstream["enums"], vendored["enums"]
    for full in sorted(set(ue) | set(ve)):
        if full not in ve:
            diffs.append({"enum": full, "aspect": "enum-absent-in-vendored"})
            continue
        if full not in ue:
            diffs.append({"enum": full, "aspect": "enum-absent-upstream"})
            continue
        uv = {v["number"]: n for n, v in ue[full]["values"].items()}
        vv = {v["number"]: n for n, v in ve[full]["values"].items()}
        for number in sorted(set(uv) | set(vv)):
            if number not in vv:
                diffs.append({"enum": full, "value": uv[number], "number": number,
                              "aspect": "enum-value-absent-in-vendored"})
            elif number not in uv:
                diffs.append({"enum": full, "value": vv[number], "number": number,
                              "aspect": "enum-value-absent-upstream"})
            elif uv[number] != vv[number]:
                diffs.append({"enum": full, "number": number, "aspect": "enum-value-name",
                              "upstream": uv[number], "vendored": vv[number]})
    return diffs


def deprecated_upstream(upstream: dict) -> list[dict]:
    out = []
    for full, msg in sorted(upstream["messages"].items()):
        for f in msg["fields"]:
            if f["deprecated"]:
                out.append({"message": full, "field": f["name"], "number": f["number"]})
    for full, en in sorted(upstream["enums"].items()):
        for n, v in en["values"].items():
            if v["deprecated"]:
                out.append({"enum": full, "value": n, "number": v["number"]})
    return out


def read_tree(root: Path, suffix: str) -> dict[str, str]:
    return {str(p.relative_to(root)): p.read_text() for p in sorted(root.rglob("*" + suffix))
            if "opentelemetry" in p.parts}


def git_tree(root: Path, rev: str) -> dict[str, str]:
    names = subprocess.run(["git", "-C", str(root), "ls-tree", "-r", "--name-only", rev, "opentelemetry"],
                           check=True, capture_output=True, text=True).stdout.split()
    return {n: subprocess.run(["git", "-C", str(root), "show", f"{rev}:{n}"], check=True,
                              capture_output=True, text=True).stdout
            for n in names if n.endswith(".proto")}


def conformance(proto_root: Path, pbc_root: Path, revs: list[str] | None = None,
                exclude: list[str] | None = None) -> dict:
    exclude = sorted(exclude or [])
    generated = read_tree(pbc_root, ".pb-c.c")
    missing = [e for e in exclude if e not in generated]
    if missing:
        raise Refused(f"excluded file(s) not present: {missing}")
    upstream = resolve_protos(read_tree(proto_root, ".proto"))
    vendored = parse_pbc({k: v for k, v in generated.items() if k not in exclude})
    diffs = compare(upstream, vendored)
    result = {
        "excluded_generated_files": exclude,
        "vendored_files": vendored["files"],
        "compiler_versions": compiler_versions(read_tree(pbc_root, ".pb-c.h")),
        "counts": {
            "upstream_messages": len(upstream["messages"]),
            "upstream_enums": len(upstream["enums"]),
            "vendored_messages": len(vendored["messages"]),
            "vendored_enums": len(vendored["enums"]),
            "by_aspect": dict(sorted(Counter(d["aspect"] for d in diffs).items())),
        },
        "differences": diffs,
        "deprecated_upstream": deprecated_upstream(upstream),
    }
    if revs:
        closest = []
        for rev in revs:
            try:
                tagged = resolve_protos(git_tree(proto_root, rev))
            except Refused as error:
                closest.append({"rev": rev, "refused": str(error)})
                continue
            d = compare(tagged, vendored)
            closest.append({"rev": rev, "differences": len(d),
                            "by_aspect": dict(sorted(Counter(x["aspect"] for x in d).items()))})
        result["by_revision"] = closest
    return result


# --------------------------------------------------------------------------
# Route census

ROUTES = ("corpus", "twelve-home", "system", "slice")
REPORT = {
    "corpus": "corpus/corpus/resource-diagnostics.json",
    "twelve-home": "twelve-home/corpus/resource-diagnostics.json",
    "system": "system/resource-diagnostics.json",
    "slice": "slice/resource-diagnostics.json",
}


def reason_class(diagnostic: dict) -> str:
    """The closed reason class of one custody diagnostic (fail-closed: an
    unrecognised text is `other`, never dropped)."""
    text = diagnostic.get("description", "")
    category = diagnostic.get("category", "")
    if text.startswith("call to uncontracted function"):
        return "missing-contract"
    if "awaiting human review" in text or "held" in text and "review" in text:
        return "held-review"
    if category == "unknown_alias":
        return "unresolved-alias"
    if "indirect call" in text or "function pointer" in text:
        return "unproven-call-effect"
    if category == "unsupported":
        return "unsupported-construct"
    if category in ("leak", "counted_user", "double_release", "use_after_release"):
        return f"finding:{category}"
    return f"other:{category}"


def null_reason_class(check: dict) -> str:
    sources = check.get("sources") or ["unattributed"]
    kinds = sorted({s.split(":", 1)[0] for s in sources})
    return "null-candidate:" + "+".join(kinds)


def census(measure_tip: Path, files: set[str]) -> dict:
    """Rows per route for the units whose report file name is in `files`."""
    out = {}
    for route in ROUTES:
        report = json.loads((measure_tip / REPORT[route]).read_text())
        null_path = measure_tip / route / f"{route}-null.jsonl"
        nulls = {}
        for line in null_path.read_text().splitlines():
            row = json.loads(line)
            if row.get("file") in files:
                nulls[(row["file"], row["function"])] = row
        rows = []
        for unit in report["units"]:
            if unit["file"] not in files:
                continue
            for fn in unit["functions"]:
                key = (unit["file"], fn["function"])
                n = nulls.get(key)
                custody_reasons = Counter(reason_class(d) for d in fn.get("diagnostics", []))
                null_reasons = Counter()
                null_verdict = None
                checks = []
                if n is not None and isinstance(n.get("null"), dict):
                    null_verdict = n["null"].get("verdict")
                    for c in n["null"].get("checks", []):
                        if c.get("outcome") in ("candidate", "finding"):
                            null_reasons[null_reason_class(c) if c["outcome"] == "candidate"
                                         else "null-finding"] += 1
                            origin = c.get("origin", "")
                            checks.append({
                                "outcome": c["outcome"], "kind": c.get("kind"),
                                "line": int(origin.rsplit(":", 1)[1]) if ":" in origin else None,
                                "subject": c.get("subject"), "sources": c.get("sources", []),
                            })
                rows.append({
                    "file": unit["file"], "library": unit["library"], "function": fn["function"],
                    "unit_status": unit["status"], "custody": fn["status"],
                    "null": null_verdict, "combined": n.get("combined") if n else None,
                    "custody_reasons": dict(sorted(custody_reasons.items())),
                    "null_reasons": dict(sorted(null_reasons.items())),
                    "null_checks": checks,
                })
        out[route] = rows
    return out


# --------------------------------------------------------------------------
# The committed census: route rows, inventories, sites, conformance, identity

# Module groups of the brief (scope `brief`) and the OTel units next to them
# that the brief does not name (scope `adjacent`), as Fluent Bit tree paths.
MODULES = [
    ("fluent-otel-proto generated descriptors", "brief", r"^lib/fluent-otel-proto/proto_c/opentelemetry/.*\.pb-c\.c$"),
    ("fluent-otel-proto protobuf-c runtime", "brief", r"^lib/fluent-otel-proto/proto_c/protobuf-c/protobuf-c\.c$"),
    ("fluent-otel-proto helpers", "brief", r"^lib/fluent-otel-proto/src/.*\.c$"),
    ("cmetrics OTLP codec", "brief", r"^lib/cmetrics/src/cmt_(encode|decode)_opentelemetry.*\.c$"),
    ("ctraces OTLP codec", "brief", r"^lib/ctraces/src/ctr_(encode|decode)_opentelemetry\.c$"),
    ("src/opentelemetry", "brief", r"^src/opentelemetry/.*\.c$"),
    ("in_opentelemetry", "brief", r"^plugins/in_opentelemetry/.*\.c$"),
    ("out_opentelemetry", "brief", r"^plugins/out_opentelemetry/.*\.c$"),
    ("processor_opentelemetry_envelope", "brief", r"^plugins/processor_opentelemetry_envelope/.*\.c$"),
    ("cprofiles OTLP codec", "adjacent", r"^lib/cprofiles/cprof_(encode|decode)_opentelemetry\.c$"),
    ("processor_content_modifier OTel", "adjacent", r"^plugins/processor_content_modifier/cm_opentelemetry\.c$"),
]
# The formulas (docs/reports/otel-proto-audit/formulas.json) each site class
# and obligation is stated by.
OBLIGATION_FORMULAS = {
    "O0": "otp_unpack_result_tested",
    "O1": "otp_deref_needs_nonnull",
    "O2": "otp_subscript_needs_bound",
    "O3": "otp_member_needs_case",
    "O4": "otp_data_needs_length",
    "O5": "otp_enum_index_needs_name",
}
CLASSIFICATION_FORMULAS = [
    "otp_classes_partition", "otp_candidate_is_wire_and_unguarded", "otp_unguarded_off_wire_undetermined",
    "otp_guard_is_cited", "otp_undetermined_names_reason", "otp_no_unsafe_claimed",
]
PREMISE_FORMULAS = [
    "otp_absent_message_is_null", "otp_repeated_elements_nonnull", "otp_repeated_array_nonnull",
    "otp_selected_member_nonnull", "otp_string_nonnull", "otp_bytes_data_iff_length",
    "otp_bounded_element_nonnull", "otp_selected_member_guarded_nonnull",
]
SITE_CLASSES = ("guarded", "candidate", "not-determinable")


def check_sites(sites: list[dict]) -> list[str]:
    """The classification formulas, checked on the census rows: the classes
    partition the sites, a candidate is on the wire, a guarded site cites its
    guard, an undetermined site names its reason (no class is an Unsafe)."""
    problems = []
    for s in sites:
        where = f"{s.get('file')}:{s.get('line')} {s.get('obligation')}"
        if s.get("class") not in SITE_CLASSES:
            problems.append(f"{where}: class {s.get('class')!r}")
        if s.get("obligation") not in OBLIGATION_FORMULAS:
            problems.append(f"{where}: obligation without a formula")
        if s.get("class") == "candidate" and not s.get("wire_reachable"):
            problems.append(f"{where}: a candidate off the wire")
        if s.get("class") == "guarded" and not s.get("detail"):
            problems.append(f"{where}: a guard without a citation")
        if s.get("class") == "not-determinable" and not s.get("reason"):
            problems.append(f"{where}: no reason named")
        if s.get("class") != "not-determinable" and s.get("reason"):
            problems.append(f"{where}: a reason on a decided site")
    return problems


# Generated-code and runtime units carry no decoder/encoder site of their own.
NO_SITE_MODULES = {"fluent-otel-proto generated descriptors", "fluent-otel-proto protobuf-c runtime"}


def module_of(path: str) -> tuple[str, str]:
    hits = [(name, scope) for name, scope, pat in MODULES if re.search(pat, path)]
    if len(hits) != 1:
        raise Refused(f"{path}: matches {len(hits)} module groups")
    return hits[0]


def route_unit_path(unit_file: str, library: str, sources: list[str]) -> str | None:
    """The Fluent Bit tree path of a route's unit, or None when it is no OTel
    unit; two matches are refused."""
    if "/" in unit_file and unit_file in sources:
        return unit_file
    hits = [s for s in sources if s.endswith("/" + unit_file) and (library in s or "/" not in unit_file)]
    if len(hits) > 1:
        raise Refused(f"{unit_file} ({library}): ambiguous {hits}")
    return hits[0] if hits else None


def _norm_subject(s: str) -> str:
    return (s or "").replace("->.", "->").replace(" ", "")


def protocol_shape(check: dict, sites: list[dict], params: dict, wire: bool) -> str:
    """How a null-property candidate relates to the protocol: `site` when a
    site of the same function and line reads the same field path, `formal`
    when it dereferences a formal of an OpenTelemetry message type, else
    `other`. Off the wire (an encoder's own messages) the unpack facts do not
    apply: `encoder-site` / `encoder-formal`."""
    raw = check.get("subject") or ""
    subj = _norm_subject(raw.split(" at ", 1)[0])
    for s in sites:
        if s["line"] == check["line"] and s["subject"] and \
                (subj.startswith(_norm_subject(s["subject"])) or _norm_subject(s["subject"]).startswith(subj)):
            return "site" if wire else "encoder-site"
    # the analyser's own provenance of the dereferenced value: a field path
    # (an element of a subscripted repeated field) or a formal
    origins = [_norm_subject(src.split(":", 1)[1]) for src in check.get("sources", [])
               if src.split(":", 1)[0] in ("memory-field", "formal") and ":" in src]
    for o in [subj] + origins:
        base = o.split("[", 1)[0]
        if o != subj and any(s["subject"] and _norm_subject(s["subject"]) == base and s["obligation"] == "O2"
                             for s in sites):
            return "site" if wire else "encoder-site"
        root = re.split(r"->|\.|\[", o, maxsplit=1)[0].lstrip("&*(")
        t = params.get(root)
        if t and "Opentelemetry__Proto__" in t:
            return "formal" if wire else "encoder-formal"
    return "other"


def build_report(measure_tip: Path, sites_doc: dict, conformance_doc: dict, ident: dict,
                 tree_root: str, inputs: dict) -> dict:
    units = {}
    for src, u in sites_doc["units"].items():
        if not src.startswith(tree_root):
            raise Refused(f"{src}: outside the pinned tree {tree_root}")
        units[src[len(tree_root):]] = u
    sources = sorted(units)
    files = {Path(s).name for s in sources}
    rows = census(measure_tip, files)
    report_units = {}
    for route in ROUTES:
        report = json.loads((measure_tip / REPORT[route]).read_text())
        for unit in report["units"]:
            path = route_unit_path(unit["file"], unit["library"], sources)
            if path is not None:
                report_units.setdefault(path, {})[route] = sorted(unit.get("source_functions", []))
    functions = []
    for path in sources:
        module, scope = module_of(path)
        inv = units[path]["functions"]
        measured = report_units.get(path, {})
        for route, listed in measured.items():
            if sorted(inv) != listed:
                raise Refused(f"{path}: {route} lists {len(listed)} definitions, the AST {len(inv)}")
        site_list = [s for s in units[path]["sites"] if s["function"]]
        for name, info in sorted(inv.items()):
            per_route = {}
            for route in ROUTES:
                hit = [r for r in rows[route] if r["function"] == name
                       and route_unit_path(r["file"], r["library"], sources) == path]
                if len(hit) > 1:
                    raise Refused(f"{path}#{name}: {len(hit)} rows on {route}")
                if hit:
                    r = hit[0]
                    params = {p: t for p, t in info["params"]}
                    fsites = [s for s in site_list if s["function"] == name]
                    wire = name in units[path]["wire_reachable"]
                    shapes = Counter(protocol_shape(c, fsites, params, wire) for c in r["null_checks"]
                                     if c["outcome"] == "candidate")
                    per_route[route] = {
                        "custody": r["custody"], "null": r["null"], "combined": r["combined"],
                        "custody_reasons": r["custody_reasons"], "null_reasons": r["null_reasons"],
                        "null_candidate_shapes": dict(sorted(shapes.items())),
                        "null_findings": sum(1 for c in r["null_checks"] if c["outcome"] == "finding"),
                    }
            functions.append({"module": module, "scope": scope, "file": path, "function": name,
                              "line": info["line"], "routes": per_route, "measured": bool(per_route)})
    # Sites (decoder/encoder modules only)
    sites = []
    for path in sources:
        module, scope = module_of(path)
        if module in NO_SITE_MODULES:
            continue
        for s in units[path]["sites"]:
            sites.append(dict(s, file=path, module=module, scope=scope))
    problems = check_sites(sites)
    if problems:
        raise Refused("; ".join(problems[:10]))
    return {
        "schema": "otel-proto-audit/1",
        "protocol_identity": ident,
        "identity_formulas": ["otp_identity_refused_unless_pinned", "otp_pinned_identity_admitted"],
        "obligation_formulas": OBLIGATION_FORMULAS,
        "classification_formulas": CLASSIFICATION_FORMULAS,
        "premise_formulas": PREMISE_FORMULAS,
        "inputs": inputs,
        "functions": functions,
        "coverage": coverage_counts(functions),
        "conformance": conformance_doc,
        "sites": sites,
        "site_counts": site_counts(sites),
        "wire_reachable": {p: units[p]["wire_reachable"] for p in sources if units[p]["wire_reachable"]},
        "phase_b_effect": phase_b_effect(functions),
    }


def coverage_counts(functions: list[dict]) -> dict:
    out = {}
    for f in functions:
        m = out.setdefault(f["module"], {"scope": f["scope"], "functions": 0, "unmeasured": 0, "routes": {}})
        m["functions"] += 1
        if not f["measured"]:
            m["unmeasured"] += 1
        for route in ROUTES:
            r = f["routes"].get(route)
            rr = m["routes"].setdefault(route, {"rows": 0, "custody": Counter(), "null": Counter(),
                                                "combined": Counter(), "custody_unknown_reasons": Counter(),
                                                "null_unknown_reasons": Counter()})
            if r is None:
                continue
            rr["rows"] += 1
            for prop in ("custody", "null", "combined"):
                rr[prop][str(r[prop])] += 1
            if r["custody"] != "Safe":
                for k in r["custody_reasons"]:
                    rr["custody_unknown_reasons"][k] += 1
            if r["null"] != "Safe":
                for k in r["null_reasons"]:
                    rr["null_unknown_reasons"][k] += 1
    for m in out.values():
        for rr in m["routes"].values():
            for k, v in rr.items():
                if isinstance(v, Counter):
                    rr[k] = dict(sorted(v.items()))
    return out


def site_counts(sites: list[dict]) -> dict:
    by = {}
    for s in sites:
        m = by.setdefault(s["module"], Counter())
        m[f"{s['obligation']}:{s['class']}"] += 1
    total = Counter()
    for s in sites:
        total[s["class"]] += 1
    nd = Counter((s["reason"] or "").split(" ")[0] for s in sites if s["class"] == "not-determinable")
    return {"by_module": {k: dict(sorted(v.items())) for k, v in sorted(by.items())},
            "by_class": dict(sorted(total.items())),
            "not_determinable_reasons": dict(sorted(nd.items())),
            "wire_reachable": dict(sorted(Counter(s["class"] for s in sites if s["wire_reachable"]).items()))}


def phase_b_effect(functions: list[dict]) -> dict:
    """Upper bound of the rows a proto-derived contract could move: a null
    Unknown whose every candidate is protocol-shaped (`site` or `formal`) and
    which carries no finding; on the combined verdict only when custody is
    already Safe. A bound, never a prediction of Safe."""
    out = {}
    for route in ROUTES:
        null_rows, combined_rows = [], []
        for f in functions:
            r = f["routes"].get(route)
            if r is None or r["null"] != "Unknown" or r["null_findings"]:
                continue
            shapes = r["null_candidate_shapes"]
            if shapes and set(shapes) <= {"site", "formal"}:
                null_rows.append(f"{f['file']}#{f['function']}")
                if r["custody"] == "Safe":
                    combined_rows.append(f"{f['file']}#{f['function']}")
        out[route] = {"null_rows": null_rows, "combined_rows": combined_rows}
    return out


# The per-function and per-site rows are bulk; the committed report keeps them
# gzipped beside the compact summary (report-artifact ceiling, check_report_hygiene).
ROW_KEYS = ("functions", "sites")


def rows_path(summary: Path) -> Path:
    """The gzipped rows file that accompanies a summary `census.json`."""
    return summary.with_name(summary.stem + "-rows.json.gz")


def split_rows(report: dict) -> tuple[dict, dict]:
    """The report without its bulk rows, and the rows."""
    return ({k: v for k, v in report.items() if k not in ROW_KEYS},
            {k: report[k] for k in ROW_KEYS if k in report})


def write_census(report: dict, summary: Path) -> None:
    """Write the compact summary and its gzipped rows, byte-stable (gzip mtime 0)."""
    head, rows = split_rows(report)
    summary.write_text(json.dumps(head, indent=1, sort_keys=True) + "\n")
    data = (json.dumps(rows, indent=1, sort_keys=True) + "\n").encode()
    rows_path(summary).write_bytes(gzip.compress(data, mtime=0))


def load_census(summary: Path) -> dict:
    """The full report: the summary merged with its rows, when they exist."""
    report = json.loads(summary.read_text())
    if any(k in report for k in ROW_KEYS):
        raise Refused(f"{summary}: rows belong in {rows_path(summary).name}")
    rows = rows_path(summary)
    if rows.exists():
        report.update(json.loads(gzip.decompress(rows.read_bytes())))
    return report


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    a = sub.add_parser("identity")
    a.add_argument("proto_root", type=Path)
    a.add_argument("--check", type=Path, help="census JSON holding the pinned `protocol_identity`")
    c = sub.add_parser("conformance")
    c.add_argument("proto_root", type=Path)
    c.add_argument("pbc_root", type=Path)
    c.add_argument("--rev", action="append", default=[], help="also compare a revision of the clone")
    c.add_argument("--exclude", action="append", default=[],
                   help="a generated .pb-c.c (relative to PBC_ROOT) the build does not compile")
    s = sub.add_parser("census")
    s.add_argument("measure_tip", type=Path)
    s.add_argument("--file", action="append", required=True, help="report file name of an OTel unit")
    r = sub.add_parser("report")
    r.add_argument("--measure-tip", type=Path, required=True)
    r.add_argument("--sites", type=Path, required=True, help="otel_proto_sites.py output")
    r.add_argument("--conformance", type=Path, required=True, help="the conformance subcommand's output")
    r.add_argument("--proto-root", type=Path, required=True)
    r.add_argument("--tree-root", required=True, help="the pinned Fluent Bit tree, with a trailing /")
    r.add_argument("--compile-db", type=Path, required=True)
    r.add_argument("--inputs-base", default="", help="a prefix stripped from recorded input paths")
    for p in (a, c, s, r):
        p.add_argument("--output", type=Path, help="write the JSON here instead of stdout")
    args = ap.parse_args(argv)

    def emit(value: dict) -> None:
        text = json.dumps(value, indent=1, sort_keys=True) + "\n"
        if args.output:
            args.output.write_text(text)
        else:
            sys.stdout.write(text)

    try:
        if args.cmd == "identity":
            actual = identity(args.proto_root)
            if args.check:
                pinned = json.loads(args.check.read_text())["protocol_identity"]
                problems = check_identity(pinned, actual)
                for p in problems:
                    print(f"refused: {p}", file=sys.stderr)
                return 1 if problems else 0
            emit(actual)
        elif args.cmd == "conformance":
            emit(conformance(args.proto_root, args.pbc_root, args.rev, args.exclude))
        elif args.cmd == "census":
            emit(census(args.measure_tip, set(args.file)))
        else:
            run = json.loads((args.measure_tip / "run.json").read_text())
            head = (Path(args.tree_root) / ".git" / "HEAD").read_text().strip()
            def rel(p: str) -> str:
                return p[len(args.inputs_base):] if args.inputs_base and p.startswith(args.inputs_base) else p

            inputs = {
                "measurement": rel(str(args.measure_tip)),
                "measurement_build_commit": run.get("build_commit"),
                "fluent_bit_tree": rel(args.tree_root),
                "fluent_bit_head": head,
                "compile_db_sha256": hashlib.sha256(args.compile_db.read_bytes()).hexdigest(),
                "describe": subprocess.run(["git", "-C", str(args.proto_root), "describe", "--tags"],
                                           check=True, capture_output=True, text=True).stdout.strip(),
            }
            report = build_report(args.measure_tip, json.loads(args.sites.read_text()),
                                  json.loads(args.conformance.read_text()), identity(args.proto_root),
                                  args.tree_root, inputs)
            if args.output:
                write_census(report, args.output)
            else:
                emit(report)
    except Refused as error:
        print(f"refused: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())

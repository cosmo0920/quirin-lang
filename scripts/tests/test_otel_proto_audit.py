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

"""Card OTEL-PROTO-AUDIT: the protocol identity check refuses any difference,
the .proto and protobuf-c descriptor readers are total-or-reject and compare
field by field, the site reader cites a guard for every guarded site and
never guards an unguarded one (fixture `fixtures/otel_proto/otel_sites.c`,
its Clang AST committed beside it), and the committed census meets the
classification formulas."""
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
import otel_proto_audit as audit  # noqa: E402
import otel_proto_sites as sites  # noqa: E402

FIXTURES = Path(__file__).resolve().parent / "fixtures" / "otel_proto"
REPORT = ROOT / "docs" / "reports" / "otel-proto-audit"

PROTO = """
syntax = "proto3";
package demo.v1;
option go_package = "x";
// a comment
message Outer {
  enum Kind { KIND_UNSPECIFIED = 0; KIND_MASK = 0x0F; }
  reserved 4;
  string name = 1;
  repeated Inner items = 2;
  Inner single = 3;
  oneof value {
    string text = 5;
    Inner nested = 6;
  }
  optional int32 maybe = 7;
  repeated fixed64 counts = 8;
  Kind kind = 9 [deprecated = true];
};
message Inner { bytes data = 1; }
"""

PBC = """
/* Generated from: demo/v1/demo.proto */
static const ProtobufCFieldDescriptor demo__v1__outer__field_descriptors[8] =
{
  { "name", 1, PROTOBUF_C_LABEL_NONE, PROTOBUF_C_TYPE_STRING, 0,   /* quantifier_offset */
    offsetof(Demo__V1__Outer, name), NULL, &protobuf_c_empty_string, 0,             /* flags */
    0,NULL,NULL },
  { "items", 2, PROTOBUF_C_LABEL_REPEATED, PROTOBUF_C_TYPE_MESSAGE, offsetof(Demo__V1__Outer, n_items),
    offsetof(Demo__V1__Outer, items), &demo__v1__inner__descriptor, NULL, 0,             /* flags */
    0,NULL,NULL },
  { "single", 3, PROTOBUF_C_LABEL_OPTIONAL, PROTOBUF_C_TYPE_MESSAGE, 0,   /* quantifier_offset */
    offsetof(Demo__V1__Outer, single), &demo__v1__inner__descriptor, NULL, 0,             /* flags */
    0,NULL,NULL },
  { "text", 5, PROTOBUF_C_LABEL_OPTIONAL, PROTOBUF_C_TYPE_STRING, offsetof(Demo__V1__Outer, value_case),
    offsetof(Demo__V1__Outer, text), NULL, &protobuf_c_empty_string, PROTOBUF_C_FIELD_FLAG_ONEOF,             /* flags */
    0,NULL,NULL },
  { "nested", 6, PROTOBUF_C_LABEL_OPTIONAL, PROTOBUF_C_TYPE_MESSAGE, offsetof(Demo__V1__Outer, value_case),
    offsetof(Demo__V1__Outer, nested), &demo__v1__inner__descriptor, NULL, PROTOBUF_C_FIELD_FLAG_ONEOF,             /* flags */
    0,NULL,NULL },
  { "maybe", 7, PROTOBUF_C_LABEL_OPTIONAL, PROTOBUF_C_TYPE_INT32, offsetof(Demo__V1__Outer, has_maybe),
    offsetof(Demo__V1__Outer, maybe), NULL, NULL, 0,             /* flags */
    0,NULL,NULL },
  { "counts", 8, PROTOBUF_C_LABEL_REPEATED, PROTOBUF_C_TYPE_FIXED64, offsetof(Demo__V1__Outer, n_counts),
    offsetof(Demo__V1__Outer, counts), NULL, NULL, 0 | PROTOBUF_C_FIELD_FLAG_PACKED,             /* flags */
    0,NULL,NULL },
  { "kind", 9, PROTOBUF_C_LABEL_NONE, PROTOBUF_C_TYPE_ENUM, 0,   /* quantifier_offset */
    offsetof(Demo__V1__Outer, kind), &demo__v1__outer__kind__descriptor, NULL, 0 | PROTOBUF_C_FIELD_FLAG_DEPRECATED,             /* flags */
    0,NULL,NULL },
};
const ProtobufCMessageDescriptor demo__v1__outer__descriptor =
{
  PROTOBUF_C__MESSAGE_DESCRIPTOR_MAGIC,
  "demo.v1.Outer",
  "Outer",
  "Demo__V1__Outer",
  "demo.v1",
  sizeof(Demo__V1__Outer),
  8,
};
static const ProtobufCFieldDescriptor demo__v1__inner__field_descriptors[1] =
{
  { "data", 1, PROTOBUF_C_LABEL_NONE, PROTOBUF_C_TYPE_BYTES, 0,   /* quantifier_offset */
    offsetof(Demo__V1__Inner, data), NULL, NULL, 0,             /* flags */
    0,NULL,NULL },
};
const ProtobufCMessageDescriptor demo__v1__inner__descriptor =
{
  PROTOBUF_C__MESSAGE_DESCRIPTOR_MAGIC,
  "demo.v1.Inner",
  "Inner",
  "Demo__V1__Inner",
  "demo.v1",
  sizeof(Demo__V1__Inner),
  1,
};
static const ProtobufCEnumValue demo__v1__outer__kind__enum_values_by_number[2] =
{
  { "KIND_UNSPECIFIED", "DEMO__V1__OUTER__KIND__KIND_UNSPECIFIED", 0 },
  { "KIND_MASK", "DEMO__V1__OUTER__KIND__KIND_MASK", 15 },
};
const ProtobufCEnumDescriptor demo__v1__outer__kind__descriptor =
{
  PROTOBUF_C__ENUM_DESCRIPTOR_MAGIC,
  "demo.v1.Outer.Kind",
};
"""


class ProtoReader(unittest.TestCase):
    def test_reads_the_subset_and_presence(self):
        u = audit.resolve_protos({"demo.proto": PROTO})
        outer = {f["name"]: f for f in u["messages"]["demo.v1.Outer"]["fields"]}
        self.assertEqual(outer["items"]["type_name"], "demo.v1.Inner")
        self.assertEqual(outer["items"]["presence"], "repeated")
        self.assertEqual(outer["single"]["presence"], "explicit")
        self.assertEqual(outer["text"]["oneof"], "value")
        self.assertEqual(outer["text"]["presence"], "explicit")
        self.assertEqual(outer["name"]["presence"], "implicit")
        self.assertEqual(outer["maybe"]["label"], "optional")
        self.assertTrue(outer["counts"]["packed"])
        self.assertTrue(outer["kind"]["deprecated"])
        self.assertEqual(outer["kind"]["type_name"], "demo.v1.Outer.Kind")
        self.assertEqual(u["enums"]["demo.v1.Outer.Kind"]["values"]["KIND_MASK"]["number"], 15)
        self.assertEqual(audit.deprecated_upstream(u),
                         [{"message": "demo.v1.Outer", "field": "kind", "number": 9}])

    def test_refuses_what_it_does_not_read(self):
        for text in ('syntax = "proto3"; message M { map<string, string> m = 1; }',
                     'syntax = "proto2"; message M { optional int32 a = 1; }',
                     'syntax = "proto3"; message M { Unknown u = 1; }',
                     'syntax = "proto3"; extend Foo { int32 a = 1; }'):
            with self.assertRaises(audit.Refused, msg=text):
                audit.resolve_protos({"x.proto": text})


class DescriptorComparison(unittest.TestCase):
    def test_equal_declarations_have_no_difference(self):
        u = audit.resolve_protos({"demo.proto": PROTO})
        v = audit.parse_pbc({"demo.pb-c.c": PBC})
        self.assertEqual(audit.compare(u, v), [])
        nested = {f["name"]: f for f in v["messages"]["demo.v1.Outer"]["fields"]}["nested"]
        self.assertEqual((nested["pbc_label"], nested["label"], nested["oneof"]), ("optional", "none", "value"))

    def test_every_difference_is_listed(self):
        u = audit.resolve_protos({"demo.proto": PROTO.replace("Kind kind = 9", "Kind kind = 10")
                                  .replace("bytes data = 1;", "bytes data = 1; string extra = 2;")})
        v = audit.parse_pbc({"demo.pb-c.c": PBC})
        aspects = sorted(d["aspect"] for d in audit.compare(u, v))
        self.assertEqual(aspects, ["field-absent-in-vendored", "field-absent-in-vendored", "field-absent-upstream"])

    def test_a_field_count_mismatch_and_a_duplicate_are_refused(self):
        with self.assertRaises(audit.Refused):
            audit.parse_pbc({"demo.pb-c.c": PBC.replace("field_descriptors[8]", "field_descriptors[9]")})
        with self.assertRaises(audit.Refused):
            audit.parse_pbc({"a.pb-c.c": PBC, "b.pb-c.c": PBC})


class Identity(unittest.TestCase):
    def test_any_difference_is_refused(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "opentelemetry" / "a").mkdir(parents=True)
            (root / "opentelemetry" / "a" / "x.proto").write_text("syntax = \"proto3\";\n")
            pinned = audit.identity(root, commit="c0ffee")
            self.assertEqual(audit.check_identity(pinned, audit.identity(root, commit="c0ffee")), [])
            self.assertTrue(audit.check_identity(pinned, audit.identity(root, commit="beef")))
            (root / "opentelemetry" / "a" / "x.proto").write_text("syntax = \"proto3\"; // edited\n")
            self.assertTrue(audit.check_identity(pinned, audit.identity(root, commit="c0ffee")))
            (root / "opentelemetry" / "a" / "y.proto").write_text("syntax = \"proto3\";\n")
            problems = audit.check_identity(pinned, audit.identity(root, commit="c0ffee"))
            self.assertIn("opentelemetry/a/y.proto: present, not pinned", problems)
            self.assertTrue(audit.check_identity({}, pinned))

    def test_the_cli_refuses_a_stale_identity(self):
        census = json.loads((REPORT / "census.json").read_text())
        pinned = census["protocol_identity"]
        stale = dict(pinned, commit="0" * 40)
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "census.json"
            path.write_text(json.dumps({"protocol_identity": pinned}))
            with mock.patch.object(audit, "identity", return_value=stale):
                self.assertEqual(audit.main(["identity", tmp, "--check", str(path)]), 1)
            with mock.patch.object(audit, "identity", return_value=pinned):
                self.assertEqual(audit.main(["identity", tmp, "--check", str(path)]), 0)


class ReasonClasses(unittest.TestCase):
    def test_closed_classes(self):
        self.assertEqual(audit.reason_class({"category": "unsupported",
                                             "description": "call to uncontracted function `f`"}),
                         "missing-contract")
        self.assertEqual(audit.reason_class({"category": "unknown_alias", "description": "x"}), "unresolved-alias")
        self.assertEqual(audit.reason_class({"category": "unsupported",
                                             "description": "indirect memory access is outside"}),
                         "unsupported-construct")
        self.assertEqual(audit.reason_class({"category": "weird", "description": "x"}), "other:weird")
        self.assertEqual(audit.null_reason_class({"sources": ["memory-field:a->b", "formal:a"]}),
                         "null-candidate:formal+memory-field")


X = "Opentelemetry__Proto__T__X"
KINDS = {
    "members": {(X, "sub"): ("message", "message", "sub"), (X, "e"): ("repeated", "message", "e"),
                (X, "n_e"): ("count", "message", "e"), (X, "s"): ("oneof", "string", "v", 1),
                (X, "m"): ("oneof", "message", "v", 2), (X, "v_case"): ("case", "enum", "v"),
                (X, "b"): ("scalar", "bytes", "b")},
    "case_constants": {"OPENTELEMETRY__PROTO__T__X__V_S": 1, "OPENTELEMETRY__PROTO__T__X__V_M": 2},
}


class SiteReader(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        tree = json.loads((FIXTURES / "otel_sites.ast.json").read_text())
        cls.result = sites.analyse(tree, "otel_sites.c", KINDS)
        cls.by = {}
        for s in cls.result["sites"]:
            cls.by.setdefault((s["line"], s["obligation"]), []).append(s)

    def cls_at(self, line, ob):
        got = {s["class"] for s in self.by.get((line, ob), [])}
        self.assertEqual(len(got), 1, (line, ob, got))
        return got.pop()

    def test_wire_reachability(self):
        self.assertEqual(self.result["wire_reachable"], ["decode", "each", "take"])

    def test_guards_are_found_and_cited(self):
        for line, ob in ((53, "O1"), (57, "O2"), (62, "O3"), (62, "O1"), (68, "O1"), (70, "O2"),
                         (72, "O4"), (78, "O1"), (52, "O0")):
            self.assertEqual(self.cls_at(line, ob), "guarded", (line, ob))
            self.assertTrue(all(s["detail"] for s in self.by[(line, ob)]))

    def test_unguarded_wire_sites_are_candidates(self):
        # x->sub->x after a test whose branch does not exit; x->e[0] with no
        # count test; x->m->x outside the switch on x->v_case
        for line, ob in ((55, "O1"), (59, "O2"), (67, "O3"), (67, "O1")):
            self.assertEqual(self.cls_at(line, ob), "candidate", (line, ob))

    def test_undecided_sites_name_their_reason(self):
        self.assertEqual(self.by[(69, "O1")][0]["reason"], "callee-outside-unit")
        encode = [s for s in self.result["sites"] if s["function"] == "encode"]
        self.assertEqual([(s["class"], s["reason"]) for s in encode],
                         [("not-determinable", "encoder-constructed-value")])

    def test_the_classification_formulas_hold(self):
        self.assertEqual(audit.check_sites(self.result["sites"]), [])

    def test_facts(self):
        ref = lambda n: {"kind": "DeclRefExpr", "referencedDecl": {"name": n},  # noqa: E731
                         "type": {"qualType": "T *"}}
        null = {"kind": "IntegerLiteral", "value": "0"}
        ne = {"kind": "BinaryOperator", "opcode": "!=", "inner": [ref("p"), null]}
        eq = {"kind": "BinaryOperator", "opcode": "==", "inner": [ref("q"), null]}
        both = {"kind": "BinaryOperator", "opcode": "&&", "inner": [ne, ref("r")]}
        either = {"kind": "BinaryOperator", "opcode": "||", "inner": [eq, ref("r")]}
        self.assertEqual(sites.nonnull_facts(ne, True), {"p"})
        self.assertEqual(sites.nonnull_facts(ne, False), set())
        self.assertEqual(sites.nonnull_facts(eq, False), {"q"})
        self.assertEqual(sites.nonnull_facts(both, True), {"p", "r"})
        self.assertEqual(sites.nonnull_facts(both, False), set())
        self.assertEqual(sites.nonnull_facts(either, True), set())
        lt = {"kind": "BinaryOperator", "opcode": "<", "inner": [ref("i"), ref("n")]}
        self.assertEqual(sites.bound_facts(lt, True), {("i", "n")})
        self.assertEqual(sites.bound_facts(lt, False), set())


class ProtocolShape(unittest.TestCase):
    def test_join(self):
        site = [{"line": 10, "subject": "m->values", "obligation": "O2"}]
        params = {"src": "Opentelemetry__Proto__Common__V1__ArrayValue *", "map": "struct cmt_map *"}
        self.assertEqual(audit.protocol_shape({"line": 10, "subject": "m->values"}, site, params, True), "site")
        self.assertEqual(audit.protocol_shape({"line": 10, "subject": "m->values"}, site, params, False),
                         "encoder-site")
        self.assertEqual(audit.protocol_shape({"line": 3, "subject": "src"}, site, params, True), "formal")
        self.assertEqual(audit.protocol_shape({"line": 3, "subject": "kv",
                                               "sources": ["memory-field:m->values[i]"]}, site, params, True),
                         "site")
        self.assertEqual(audit.protocol_shape({"line": 3, "subject": "map"}, site, params, True), "other")


class CommittedCensus(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.census = json.loads((REPORT / "census.json").read_text())
        rows = json.loads((REPORT / "formulas.json").read_text())["formulas"]
        cls.formulas = {i for i, _ in rows}

    def test_every_cited_formula_exists(self):
        c = self.census
        cited = (list(c["obligation_formulas"].values()) + c["classification_formulas"]
                 + c["premise_formulas"] + c["identity_formulas"])
        self.assertEqual(sorted(set(cited) - self.formulas), [])
        self.assertEqual(sorted(self.formulas - set(cited)), [])

    def test_sites_meet_the_classification_formulas(self):
        self.assertEqual(audit.check_sites(self.census["sites"]), [])

    def test_unmeasured_units_are_never_counted_as_covered(self):
        for f in self.census["functions"]:
            self.assertEqual(f["measured"], bool(f["routes"]), f["function"])
        for m in self.census["coverage"].values():
            covered = sum(1 for r in m["routes"].values() if r["rows"])
            if m["unmeasured"] == m["functions"]:
                self.assertEqual(covered, 0)

    def test_identity_is_pinned(self):
        ident = self.census["protocol_identity"]
        self.assertEqual(ident["commit"], "3a6e438003e673112efd4a00e9836c00672eae3c")
        self.assertEqual(len(ident["files"]), 11)


if __name__ == "__main__":
    unittest.main()

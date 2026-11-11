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

import pathlib
import runpy
import shutil
import tempfile
import unittest


MODULE = runpy.run_path(
    pathlib.Path(__file__).parents[1] / "w42_unprojected_census.py",
    run_name="w42_unprojected_census_test",
)


class ConditionalEvidenceTests(unittest.TestCase):
    def test_instrumentation_marks_every_branch_and_restores_lines(self):
        lines = ["#if FEATURE\n", "int selected;\n", "#else\n", "int other;\n", "#endif\n"]
        groups = MODULE["conditional_groups"](lines)
        text = MODULE["instrument_source"](lines, groups, pathlib.Path("<stdin>"))
        self.assertIn("#warning W42_BRANCH_1_0", text)
        self.assertIn("#warning W42_BRANCH_1_1", text)
        self.assertIn('#line 6 "<stdin>"', text)

    def test_no_else_gets_a_positive_synthetic_branch(self):
        lines = ["#ifdef WINDOWS\n", "int windows_only;\n", "#endif\n"]
        groups = MODULE["conditional_groups"](lines)
        text = MODULE["instrument_source"](lines, groups, pathlib.Path("<stdin>"))
        self.assertIn("#else\n#warning W42_BRANCH_1_1", text)

    def test_continued_condition_is_rejected(self):
        lines = ["#if ONE || \\\n", "    TWO\n", "int value;\n", "#endif\n"]
        groups = MODULE["conditional_groups"](lines)
        with self.assertRaisesRegex(ValueError, "continued conditional"):
            MODULE["instrument_source"](lines, groups, pathlib.Path("<stdin>"))


class DefinitionAnchorTests(unittest.TestCase):
    def test_real_multiline_definition_is_verified(self):
        source = "int helper(\n    int value)\n{\n    return value;\n}\n"
        self.assertEqual(MODULE["verify_definition_site"](source, "helper", 1), 3)

    def test_call_and_declaration_are_not_definitions(self):
        with self.assertRaisesRegex(ValueError, "not a function definition"):
            MODULE["verify_definition_site"]("int helper(int);\n", "helper", 1)
        with self.assertRaisesRegex(ValueError, "not a function definition"):
            MODULE["verify_definition_site"]("helper(value);\n", "helper", 1)


class CompileIdentityTests(unittest.TestCase):
    def test_historical_arguments_match_the_reduced_route(self):
        with tempfile.TemporaryDirectory() as raw:
            root = pathlib.Path(raw)
            source = root / "src" / "unit.c"
            source.parent.mkdir()
            source.write_text("int unit(void) { return 0; }\n")
            entry = {
                "directory": str(root),
                "file": str(source),
                "command": (
                    f"clang -DFEATURE=1 -D'DOLLAR=$bad' -I include "
                    f"-isystem system -std=gnu11 -Wall -c {source} -o unit.o"
                ),
            }
            arguments = MODULE["historical_arguments"](entry, source)
            self.assertEqual(arguments[:4], ["clang", "-x", "c", "-std=gnu11"])
            self.assertIn(f"-I{source.parent}", arguments)
            self.assertIn(f"-I{root / 'include'}", arguments)
            self.assertIn(f"-I{root / 'system'}", arguments)
            self.assertIn("-DFEATURE=1", arguments)
            self.assertNotIn("-DDOLLAR=$bad", arguments)
            self.assertNotIn("-Wall", arguments)

    def test_ast_definition_requires_a_compound_statement(self):
        dump = """|-FunctionDecl 0x1 <line:1:1> helper 'int (void)'\n| `-CompoundStmt 0x2 <col:18, col:29>\n`-FunctionDecl 0x3 <line:2:1> declared 'int (void)'\n"""
        with tempfile.TemporaryDirectory() as raw:
            path = pathlib.Path(raw) / "ast.txt"
            path.write_text(dump)
            found = MODULE["ast_definitions"](path, {"helper", "declared"})
        self.assertEqual(len(found["helper"]), 1)
        self.assertEqual(found["declared"], [])

    def test_real_clang_proves_marker_token_equivalence_and_rejects_a_change(self):
        clang = shutil.which("clang")
        if clang is None:
            self.skipTest("Clang is unavailable")
        source_text = """#if FEATURE
int hidden(void) { return 1; }
#endif
const char *origin = __FILE__;
int source_line = __LINE__;
"""
        with tempfile.TemporaryDirectory() as raw:
            root = pathlib.Path(raw)
            source = root / "unit.c"
            source.write_text(source_text)
            entry = {
                "_database_index": 0,
                "directory": str(root),
                "file": str(source),
                "command": f"{clang} -DFEATURE=0 -std=gnu17 -c {source} -o unit.o",
                "output": "unit.o",
            }
            lines = source_text.splitlines(keepends=True)
            groups = MODULE["conditional_groups"](lines)
            instrumented = MODULE["instrument_source"](
                lines, groups, pathlib.Path("<stdin>")
            )
            evidence = MODULE["compile_evidence"](
                entry, source, instrumented, groups, {"hidden"}
            )
            self.assertTrue(
                evidence["preprocess"][
                    "instrumented_token_stream_equals_uninstrumented"
                ]
            )
            self.assertEqual(evidence["ast"]["definitions"]["hidden"], [])

            changed = instrumented.replace(
                "int source_line = __LINE__;", "int source_line = 999;"
            )
            with self.assertRaisesRegex(RuntimeError, "changed the token stream"):
                MODULE["compile_evidence"](
                    entry, source, changed, groups, {"hidden"}
                )


if __name__ == "__main__":
    unittest.main()

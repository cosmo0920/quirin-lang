"""A library's #[cfg(test)] code selects its unit-test executable alone.

Card GATE-SCOPE-LIB-UNIT-TEST-MODULES.  A module a library declares only under
a test-only ``#[cfg]`` (``#[cfg(test)] #[path = "../tests/unit/x.rs"] mod
tests;``), and an include or a literal inside a ``#[cfg(test)]`` item, is
compiled into ``PKG::lib::NAME`` only: dependents and doc-tests build the
library without ``cfg(test)``.  Such a change selects that unit, without the
crate's other targets or its reverse dependencies.  A file also reached by an
ordinary edge, and anything the scanner cannot classify (an inner
``#![cfg(test)]``, an unparsed predicate, an attribute on something that is not
an item, a crate with opaque module structure, a binary's cfg(test) code, a
library cargo does not test) keeps the old, crate-wide selection.
"""
from pathlib import Path
import shutil
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import gate_scope as scope_mod  # noqa: E402
import run_test_binaries as runner_mod  # noqa: E402
from gate_scope import Crate, Model, Target  # noqa: E402

LIB = "qcore-contract::lib::qcore_contract"


def spans_text(text):
    return [text[start:end] for start, end in scope_mod.cfg_test_spans(text)]


def offset_in_span(text, needle):
    offset = text.index(needle)
    return any(start <= offset < end for start, end in scope_mod.cfg_test_spans(text))


class CfgTestSpans(unittest.TestCase):
    def test_a_declared_test_module_is_one_span(self):
        text = ('pub mod real;\n#[cfg(test)]\n#[path = "../tests/unit/t.rs"]\n'
                '#[cfg_attr(windows, allow(dead_code))]\nmod tests;\npub fn after() {}\n')
        self.assertEqual(spans_text(text), [
            '#[cfg(test)]\n#[path = "../tests/unit/t.rs"]\n'
            '#[cfg_attr(windows, allow(dead_code))]\nmod tests;'])

    def test_attributes_before_the_cfg_belong_to_the_span(self):
        text = '#[allow(dead_code)]\n#[cfg(test)]\nfn helper() {}\nfn after() {}\n'
        self.assertEqual(spans_text(text), ['#[allow(dead_code)]\n#[cfg(test)]\nfn helper() {}'])

    def test_test_only_predicates(self):
        for predicate in ("test", "all(test, unix)", "all(unix, test)",
                          "all(test, not(target_os = \"windows\"))",
                          "any(test, all(test, unix))", "all(all(test, unix), feature = \"x\")"):
            self.assertTrue(spans_text(f"#[cfg({predicate})]\nfn f() {{}}\n"), predicate)
        for predicate in ("not(test)", "any(windows, test)", "any(target_os = \"linux\", test)",
                          "unix", "feature = \"test\"", "all(unix, not(test))", "any()",
                          "test = \"x\""):
            self.assertEqual(spans_text(f"#[cfg({predicate})]\nfn f() {{}}\n"), [], predicate)

    def test_an_inner_cfg_test_is_not_honoured(self):
        # Adding `#![cfg(test)]` to a module removes it from dependents' build:
        # the change must keep reaching them, so the file stays ordinary.
        self.assertEqual(spans_text('#![cfg(test)]\nconst A: &str = include_str!("a");\n'), [])

    def test_strings_comments_and_characters_do_not_end_the_item(self):
        text = ('#[cfg(test)]\nmod t {\n'
                '    const S: &str = "}";\n'
                "    const C: char = '}';\n"
                "    const B: u8 = b'}';\n"
                '    // }\n    /* } /* nested } */ */\n'
                '    const R: &str = r##"} "# still raw"##;\n'
                "    fn f<'a>(x: &'a str) -> &'a str { x }\n"
                "    const E: char = '\\'';\n"
                '}\nconst AFTER: &str = include_str!("after.txt");\n')
        self.assertTrue(offset_in_span(text, "fn f<'a>"))
        self.assertFalse(offset_in_span(text, "const AFTER"))
        self.assertTrue(spans_text(text)[0].endswith("x }\n    const E: char = '\\'';\n}"))

    def test_semicolon_items_end_at_their_semicolon(self):
        text = ('#[cfg(test)]\nconst X: S = S { a: 1 };\nconst Y: &str = include_str!("y");\n'
                '#[cfg(test)]\nlet v = if a { include_str!("a") } else { include_str!("b") };\n'
                'let w = include_str!("w");\n'
                '#[cfg(test)]\nuse a::{b, c};\nconst Z: &str = include_str!("z");\n')
        self.assertFalse(offset_in_span(text, "const Y"))
        self.assertTrue(offset_in_span(text, 'include_str!("b")'))
        self.assertFalse(offset_in_span(text, "let w"))
        self.assertFalse(offset_in_span(text, "const Z"))

    def test_const_fn_and_generic_items_end_at_their_body(self):
        text = ('#[cfg(test)]\npub(crate) const fn f() -> u8 { 1 }\nconst A: u8 = 2;\n'
                '#[cfg(test)]\nimpl<A, B> T for X<A, B> where A: Y { fn g() {} }\nconst B: u8 = 3;\n')
        self.assertTrue(offset_in_span(text, "-> u8 { 1 }"))
        self.assertFalse(offset_in_span(text, "const A"))
        self.assertTrue(offset_in_span(text, "fn g()"))
        self.assertFalse(offset_in_span(text, "const B"))

    def test_attributes_on_non_items_give_no_span(self):
        # A match arm, a struct field, a struct-expression field, a parameter:
        # the following code is not inside a test-only item.
        for text in ('match x {\n    #[cfg(test)]\n    A => include_str!("a"),\n'
                     '    B => { include_str!("b") }\n}\n',
                     'struct S {\n    #[cfg(test)]\n    a: u8,\n    b: u8,\n}\n',
                     'let s = S {\n    #[cfg(test)]\n    a: 1,\n    b: X { c: 2 },\n};\n',
                     'fn f(#[cfg(test)] a: u8, b: u8) { include_str!("b"); }\n',
                     '#[cfg(test)]\n{ include_str!("block"); }\n'):
            self.assertEqual(spans_text(text), [], text)

    def test_an_unterminated_item_gives_no_span(self):
        self.assertEqual(spans_text('#[cfg(test)]\nmod t {\n    fn f() {}\n'), [])
        self.assertEqual(spans_text('#[cfg(test)]\nmod t { const S: &str = "open; }\n'), [])

    def test_nested_items_inside_a_test_module(self):
        text = '#[cfg(test)]\nmod t {\n    #[cfg(all(test, unix))]\n    fn u() {}\n}\nfn o() {}\n'
        self.assertEqual(len(spans_text(text)), 2)
        self.assertFalse(offset_in_span(text, "fn o"))


def contract_crate(**kwargs):
    return Crate("qcore-contract", "root", "crates/qcore-contract", set(), **kwargs)


CRATES = [
    contract_crate(),
    Crate("user", "root", "crates/user", {"qcore-contract"}),
    Crate("qcore-c-gateway", "root", "crates/qcore-c-gateway", set()),
]
CONTRACT_LIB = (
    "pub mod real;\n"
    "pub mod both_plain_parent;\n"
    "#[cfg(test)]\n"
    '#[path = "../tests/unit/lib_tests.rs"]\n'
    "mod lib_tests;\n"
    "#[cfg(test)]\n"
    "mod test_support;\n"
    "#[cfg(test)]\n"
    '#[path = "../tests/unit/both.rs"]\n'
    "mod both_test;\n"
    'pub const PROD: &str = include_str!("../data/prod.txt");\n'
    "#[cfg(test)]\n"
    "mod inline {\n"
    '    const FIX: &str = include_str!("../tests/fixtures/fix.qv");\n'
    '    fn run() -> &\'static str { "tests/fixtures/run.json" }\n'
    '    fn shared() -> &\'static str { "tests/fixtures/shared.json" }\n'
    '    const LEAN: &str = include_str!("../../../proof/QCore/Unit.lean");\n'
    "}\n"
    # The same literal in ordinary code, after the test-only one.
    'pub fn shared() -> &\'static str { "tests/fixtures/shared.json" }\n'
)
FILES = {
    "crates/qcore-contract/Cargo.toml": "[package]\n",
    "crates/qcore-contract/src/lib.rs": CONTRACT_LIB,
    "crates/qcore-contract/src/real.rs": "pub fn r() {}\n",
    "crates/qcore-contract/src/both_plain_parent.rs":
        '#[path = "../tests/unit/both.rs"]\nmod both_plain;\n',
    "crates/qcore-contract/src/test_support.rs": "pub fn s() {}\n",
    "crates/qcore-contract/tests/unit/lib_tests.rs": (
        "mod nested;\n"
        'const A: &str = include_str!("../fixtures/unit.qv");\n'),
    "crates/qcore-contract/tests/unit/lib_tests/nested.rs": "fn n() {}\n",
    "crates/qcore-contract/tests/unit/both.rs": "fn b() {}\n",
    "crates/qcore-contract/tests/it.rs": '#[path = "../src/real.rs"]\nmod real;\n',
    "crates/qcore-contract/tests/fixtures/fix.qv": "fn f();\n",
    "crates/qcore-contract/tests/fixtures/unit.qv": "fn u();\n",
    "crates/qcore-contract/tests/fixtures/run.json": "{}\n",
    "crates/qcore-contract/tests/fixtures/shared.json": "{}\n",
    "crates/qcore-contract/data/prod.txt": "p\n",
    "crates/user/src/lib.rs": "pub fn u() {}\n",
    "crates/qcore-c-gateway/src/lib.rs": "pub fn g() {}\n",
    "proof/QCore/Unit.lean": "theorem t : True := trivial\n",
}


def model(files=FILES, crates=CRATES):
    return Model(crates, sorted(files), read_text=lambda path: files[path])


def plan(paths, files=FILES, crates=CRATES):
    return scope_mod.plan(paths, model(files, crates), exists=lambda p: p in files,
                          read_text=lambda p: files[p])


class LibraryUnitSelection(unittest.TestCase):
    def assert_lib_unit_only(self, path, files=FILES, crates=CRATES):
        scope = plan([path], files, crates)
        self.assertEqual(scope["mode"], "scoped", (path, scope["full_reasons"]))
        self.assertEqual(scope["rust_targets"], [LIB], path)
        self.assertEqual(scope["rust_packages"], [], path)
        self.assertNotIn("user", scope["affected"], "no reverse dependency joins")
        return scope

    def test_a_path_module_declared_under_cfg_test_selects_the_lib_unit(self):
        self.assert_lib_unit_only("crates/qcore-contract/tests/unit/lib_tests.rs")

    def test_its_children_are_test_only_too(self):
        self.assert_lib_unit_only("crates/qcore-contract/tests/unit/lib_tests/nested.rs")

    def test_a_src_module_declared_under_cfg_test_selects_the_lib_unit(self):
        self.assert_lib_unit_only("crates/qcore-contract/src/test_support.rs")

    def test_includes_and_literals_in_cfg_test_items_select_the_lib_unit(self):
        self.assert_lib_unit_only("crates/qcore-contract/tests/fixtures/fix.qv")
        self.assert_lib_unit_only("crates/qcore-contract/tests/fixtures/unit.qv")
        self.assert_lib_unit_only("crates/qcore-contract/tests/fixtures/run.json")

    def test_a_read_outside_the_crate_selects_the_reader_lib_unit(self):
        scope = plan(["proof/QCore/Unit.lean"])
        self.assertEqual(scope["mode"], "scoped", scope["full_reasons"])
        self.assertEqual(scope["rust_targets"], [LIB])
        self.assertEqual(scope["lean"]["mode"], "modules")

    def test_a_literal_also_in_ordinary_code_keeps_the_crate(self):
        scope = plan(["crates/qcore-contract/tests/fixtures/shared.json"])
        self.assertEqual(scope["mode"], "full")
        self.assertIn("the diff reaches qcore-contract", scope["full_reasons"])

    def test_a_file_also_reached_by_an_ordinary_edge_keeps_the_crate(self):
        scope = plan(["crates/qcore-contract/tests/unit/both.rs"])
        self.assertEqual(scope["mode"], "full")

    def test_ordinary_code_keeps_the_crate(self):
        for path in ("crates/qcore-contract/src/lib.rs", "crates/qcore-contract/src/real.rs",
                     "crates/qcore-contract/data/prod.txt"):
            scope = plan([path])
            self.assertEqual(scope["mode"], "full", path)
            self.assertIn("user", scope["affected"], path)

    def test_an_integration_test_and_the_lib_unit_together(self):
        files = {**FILES, "crates/qcore-contract/tests/it.rs":
                 '#[path = "../src/test_support.rs"]\nmod support;\n'}
        scope = plan(["crates/qcore-contract/src/test_support.rs"], files)
        self.assertEqual(scope["mode"], "scoped", scope["full_reasons"])
        self.assertEqual(scope["rust_targets"], [LIB, "qcore-contract::test::it"])

    def test_cfg_test_code_of_a_file_an_integration_test_compiles_selects_both(self):
        files = {**FILES,
                 "crates/qcore-contract/src/real.rs": (
                     "pub fn r() {}\n#[cfg(test)]\nmod t {\n"
                     '    const X: &str = include_str!("../tests/fixtures/fix.qv");\n}\n')}
        scope = plan(["crates/qcore-contract/tests/fixtures/fix.qv"], files)
        self.assertEqual(scope["mode"], "scoped", scope["full_reasons"])
        self.assertEqual(scope["rust_targets"], [LIB, "qcore-contract::test::it"])

    def test_steps_select_the_lib_unit_and_lint_it_in_test_mode(self):
        scope = self.assert_lib_unit_only("crates/qcore-contract/tests/unit/lib_tests.rs")
        steps = {s.name: s for s in scope_mod.steps_for(scope, Path("/o"))}
        self.assertEqual(steps["clippy-qcore-contract-lib"].argv,
                         ["cargo", "clippy", "-p", "qcore-contract", "--lib", "--profile", "test",
                          "--", "-D", "warnings"])
        self.assertNotIn("clippy-qcore-contract", steps)
        suite = steps["rust-suite"].argv
        self.assertEqual(suite[suite.index("--targets") + 1], LIB)
        self.assertNotIn("--packages", suite)


class FailClosed(unittest.TestCase):
    """What the scanner cannot classify selects as before: the crate whole."""

    def assert_crate_wide(self, path, files=FILES, crates=CRATES):
        scope = plan([path], files, crates)
        self.assertEqual(scope["mode"], "full", path)
        self.assertIn("the diff reaches qcore-contract", scope["full_reasons"], path)
        self.assertIn("user", scope["affected"])

    def test_an_opaque_module_structure_disables_narrowing(self):
        for marker in ('include!(concat!(env!("OUT_DIR"), "/generated.rs"));\n',
                       'macro_rules! m { ($n:ident) => { mod $n; } }\n',
                       '#[cfg_attr(feature = "x", path = "other.rs")]\nmod other;\n'):
            files = {**FILES, "crates/qcore-contract/src/real.rs": marker}
            self.assertIn("qcore-contract", model(files).opaque, marker)
            for path in ("crates/qcore-contract/tests/unit/lib_tests.rs",
                         "crates/qcore-contract/tests/fixtures/fix.qv",
                         "crates/qcore-contract/src/test_support.rs"):
                self.assert_crate_wide(path, files)

    def test_an_inner_cfg_test_does_not_narrow(self):
        files = {**FILES, "crates/qcore-contract/src/lib.rs": CONTRACT_LIB + "mod inner_only;\n",
                 "crates/qcore-contract/src/inner_only.rs":
                     '#![cfg(test)]\nconst X: &str = include_str!("../tests/fixtures/unit.qv");\n'}
        self.assert_crate_wide("crates/qcore-contract/src/inner_only.rs", files)

    def test_an_unparsed_predicate_does_not_narrow(self):
        text = CONTRACT_LIB.replace("#[cfg(test)]\nmod test_support;",
                                    "#[cfg(any(test, feature = \"support\"))]\nmod test_support;")
        files = {**FILES, "crates/qcore-contract/src/lib.rs": text}
        self.assert_crate_wide("crates/qcore-contract/src/test_support.rs", files)

    def test_a_library_cargo_does_not_test_is_not_narrowed(self):
        lib = Target("qcore_contract", ("lib",), "crates/qcore-contract/src/lib.rs", test=False)
        crates = [contract_crate(targets=[lib]), *CRATES[1:]]
        self.assert_crate_wide("crates/qcore-contract/tests/unit/lib_tests.rs", crates=crates)

    def test_cfg_test_code_of_a_binary_is_not_narrowed(self):
        files = {**FILES, "crates/qcore-contract/src/main.rs":
                 '#[cfg(test)]\n#[path = "../tests/unit/lib_tests.rs"]\nmod t;\n'}
        self.assert_crate_wide("crates/qcore-contract/tests/unit/lib_tests.rs", files)

    def test_a_module_no_target_reaches_stays_crate_wide(self):
        files = {**FILES, "crates/qcore-contract/src/orphan.rs": "fn o() {}\n"}
        self.assert_crate_wide("crates/qcore-contract/src/orphan.rs", files)


class RunnerLibUnits(unittest.TestCase):
    """run_test_binaries.py --targets PKG::lib::NAME."""

    DOCUMENT = {"packages": [
        {"name": "a", "targets": [{"kind": ["lib"], "name": "a"}, {"kind": ["test"], "name": "x"}]},
        {"name": "d", "targets": [{"kind": ["lib"], "name": "d", "test": False}]},
        {"name": "m", "targets": [{"kind": ["proc-macro"], "name": "m"}]},
    ]}

    def test_a_library_unit_is_selected_with_cargo_lib_filter(self):
        selected = runner_mod.parse_targets(["a::test::x,a::lib::a"], self.DOCUMENT, None)
        self.assertEqual(selected, {"a": [("lib", "a"), ("test", "x")]})
        self.assertEqual(runner_mod.target_filters(selected["a"]), ["--lib", "--test", "x"])
        self.assertEqual(runner_mod.parse_targets(["m::proc-macro::m"], self.DOCUMENT, None),
                         {"m": [("proc-macro", "m")]})

    def test_a_library_cargo_does_not_test_refuses(self):
        with self.assertRaises(ValueError):
            runner_mod.parse_targets(["d::lib::d"], self.DOCUMENT, None)

    def test_the_scoper_and_the_runner_name_units_alike(self):
        target = Target("qcore_contract", ("lib",), "crates/qcore-contract/src/lib.rs")
        self.assertEqual(scope_mod.unit_uid("qcore-contract", target), LIB)
        self.assertEqual(runner_mod.unit_id("qcore-contract", {"kind": ["lib"],
                                                               "name": "qcore_contract"}), LIB)

    @unittest.skipIf(shutil.which("cargo") is None, "cargo is not installed")
    def test_a_wrong_library_unit_refuses_before_building(self):
        import os
        import tempfile
        for uid in ("qcore-contract::lib::wrong", "qcorec::bin::qcorec"):
            with tempfile.TemporaryDirectory() as directory:
                log_dir = Path(directory) / "logs"
                with open(os.devnull, "w") as sink:
                    stderr, sys.stderr = sys.stderr, sink
                    try:
                        code = runner_mod.main(["--log-dir", str(log_dir), "--targets", uid])
                    finally:
                        sys.stderr = stderr
                self.assertEqual(code, 2, uid)
                self.assertFalse(log_dir.exists(), "nothing was built or run")


@unittest.skipIf(shutil.which("cargo") is None or shutil.which("git") is None,
                 "cargo or git is not installed")
class RealRepository(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.model = scope_mod.real_model()

    def test_no_crate_of_this_tree_is_opaque(self):
        self.assertEqual(self.model.opaque, {})

    def test_contract_unit_modules_select_the_contract_lib_unit(self):
        for path in ("crates/qcore-contract/tests/unit/contract_observation.rs",
                     "crates/qcore-c-gateway/tests/unit/legacy_c.rs",
                     "crates/qcore-verification/tests/unit/cycle18_callback_obligations.rs"):
            scope = scope_mod.plan([path], self.model, lean=False)
            self.assertEqual(scope["mode"], "scoped", (path, scope["full_reasons"]))
            self.assertEqual(scope["rust_packages"], [], path)
            owner = self.model.owner(path)
            self.assertNotIn(owner, scope["affected"], path)
            # The owner's library unit, not the crate; walkers of the tree
            # (qcore-verification's composition_bounds walks crates/) still
            # claim their own test targets.
            self.assertIn(f"{owner}::lib::{owner.replace('-', '_')}", scope["targets"][owner],
                          path)


if __name__ == "__main__":
    unittest.main()

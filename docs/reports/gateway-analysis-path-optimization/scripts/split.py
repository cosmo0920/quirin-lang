#!/usr/bin/env python3
"""GATEWAY-ANALYSIS-PATH-OPTIMIZATION, step 5: split `legacy_c.rs` into child
modules along the route map, **move-only**.

Usage: split.py [--check]   (run from the repository root)

Each segment is found by anchors (the first line of its first item and the
first line of the item that follows it), widened upward over the doc
comments, attributes and comments directly above each anchor, cut out of
`legacy_c.rs` verbatim and written to its own file under `#[path]`. The only
edits to moved text are visibility: a private top-level item, a private
method of a non-trait `impl` and a private named field of a moved struct gain
`pub(super)`, so the parent and the sibling modules still reach what they
reached. The parent declares each module at the end of the file (no macro
is defined in it, so position changes no name resolution) and imports its
items with `pub use self::NAME::*;` (a glob import binds each item at the
lesser of the two visibilities, so the public API is unchanged). Methods of
`impl Parser` are moved inside a fresh `impl Parser<'_>` block.

Re-runnable after a rebase: anchors are item signatures, not line numbers.
With --check it only reports the segments' line ranges. `scripts/move_check.py`
proves the result is a move.
"""
import pathlib
import re
import sys

SRC = pathlib.Path("crates/qcore-c-gateway/src")
PARENT = SRC / "legacy_c.rs"

# (module, file, title, kind, cfg, start anchor, end anchor)
SEGMENTS = [
    ("tokens", "legacy_tokens.rs",
     "the token stream: the tokenizer, conditional-directive scan, raw "
     "definition scan, body and parameter readers and bracket matching",
     "top", None, r"^pub\(crate\) struct Token \{$", r"^enum Statement \{$"),
    ("parser_census", "legacy_parser_census.rs",
     "the parser's measurement-only census hooks (feature `indirect-census`)",
     "impl", "indirect-census", r"^    fn census_indirect\($", r"^    fn expression_writes\("),
    ("condition_routes", "legacy_condition_routes.rs",
     "the expression and condition readers the statement and condition routes "
     "ask (ROUTE-MAP sections 3 and 6): writes, scalar effects, the supported "
     "test, the condition call, access, null and distinct routes, alias reads",
     "impl", None, r"^    fn expression_writes\(", r"^    fn parse_block\("),
    ("return_routes", "legacy_return_routes.rs",
     "the return-expression routes (ROUTE-MAP section 4, step 2): W4 return "
     "call, B01, W43 and W45 families, 10k, 11h",
     "impl", None, r"^    fn return_call_instruction\($", r"^    fn classify_simple\("),
    ("classify", "legacy_classify.rs",
     "the statement classifier (ROUTE-MAP section 2.2): `classify_simple` and "
     "its guards, the typed fallbacks of `classify_simple_unguarded`, the "
     "element, sub-object and stepped routes and `classify_simple_base`",
     "impl", None, r"^    fn classify_simple\(", r"^    fn text\(&self\)"),
    ("token_shapes", "legacy_token_shapes.rs",
     "lexical shape readers over token texts: calls, arguments, conditions, "
     "places and declarators",
     "top", None, r"^fn is_leading_cast\(", r"^struct CfgBuilder \{$"),
    ("cfg_builder", "legacy_cfg_builder.rs",
     "the CFG builder: parsed statements compiled to `LegacyCfg` nodes, with "
     "each condition's compile precedence (ROUTE-MAP section 3)",
     "top", None, r"^struct CfgBuilder \{$", r"^struct AbstractPointer \{$"),
    ("transfer", "legacy_transfer.rs",
     "the abstract state and the transfer functions: pointers, derivations, "
     "entry custody, exit observations and the flow analysis over a CFG",
     "top", None, r"^struct AbstractPointer \{$", r'^#\[path = "\.\./tests/unit/legacy_c\.rs"\]$'),
]

DECORATION = re.compile(r"^\s*(///|//|#\[|#!\[)")
TOP_ITEM = re.compile(r"^(fn|struct|enum|type|const|static|trait|unsafe fn|const fn) ")
METHOD = re.compile(r"^    (fn|const fn|unsafe fn) ")
FIELD = re.compile(r"^    (r#)?[a-z_][a-z0-9_]*: ")


def widen(lines, index):
    """Move an anchor up over the decoration directly above it."""
    while index > 0 and DECORATION.match(lines[index - 1]) and lines[index - 1].strip():
        index -= 1
    return index


def find(lines, pattern, start=0):
    expression = re.compile(pattern)
    for index in range(start, len(lines)):
        if expression.search(lines[index]):
            return index
    raise SystemExit(f"anchor not found: {pattern}")


def widen_visibility(block, kind):
    out = []
    in_impl = in_trait_impl = in_struct = False
    for line in block:
        if kind == "impl":
            if METHOD.match(line):
                line = "    pub(super) " + line[4:]
            out.append(line)
            continue
        if line.startswith("impl"):
            in_impl = True
            in_trait_impl = " for " in line.split("{")[0]
        elif TOP_ITEM.match(line):
            in_struct = line.startswith("struct ") and line.rstrip().endswith("{")
            line = "pub(super) " + line
        elif line.startswith("}"):
            in_impl = in_struct = False
        elif in_impl and not in_trait_impl and METHOD.match(line):
            line = "    pub(super) " + line[4:]
        elif in_struct and FIELD.match(line):
            line = "    pub(super) " + line[4:]
        if line.startswith("pub(crate) struct") or line.startswith("pub struct"):
            in_struct = line.rstrip().endswith("{")
        out.append(line)
    return out


def main():
    check = "--check" in sys.argv
    lines = PARENT.read_text().split("\n")
    cuts = []
    for module, file, title, kind, cfg, start, end in SEGMENTS:
        first = widen(lines, find(lines, start))
        last = widen(lines, find(lines, end, first + 1))
        cuts.append((first, last, module, file, title, kind, cfg))
    cuts.sort()
    for (a, b, *_), (c, d, *_) in zip(cuts, cuts[1:]):
        if b > c:
            raise SystemExit("segments overlap")
    for first, last, module, file, *_ in cuts:
        print(f"{module}: lines {first + 1}-{last} ({last - first}) -> {file}")
    if check:
        return
    tail = []
    for first, last, module, file, title, kind, cfg in reversed(cuts):
        block = lines[first:last]
        while block and not block[-1].strip():
            block.pop()
        moved = widen_visibility(block, kind)
        header = [
            f"//! Card GATEWAY-ANALYSIS-PATH-OPTIMIZATION: {title}.",
            "//!",
            "//! Moved verbatim from `legacy_c.rs` (only visibility widened to",
            "//! `pub(super)` where the parent or a sibling module reads an item).",
            "",
            "use super::*;",
            "",
        ]
        body = ["impl Parser<'_> {"] + moved + ["}"] if kind == "impl" else moved
        (SRC / file).write_text("\n".join(header + body) + "\n")
        del lines[first:last]
        declaration = [
            f"// Card GATEWAY-ANALYSIS-PATH-OPTIMIZATION: {title}.",
        ]
        if cfg:
            declaration.append(f'#[cfg(feature = "{cfg}")]')
        declaration += [f'#[path = "{file}"]', f"mod {module};"]
        if kind == "top":
            declaration.append(f"pub use self::{module}::*;")
        tail = declaration + [""] + tail
    test = find(lines, r'^#\[path = "\.\./tests/unit/legacy_c\.rs"\]$') - 1
    lines[test:test] = tail
    PARENT.write_text("\n".join(lines))


if __name__ == "__main__":
    main()

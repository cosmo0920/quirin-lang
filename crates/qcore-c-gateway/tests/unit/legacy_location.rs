// SPDX-License-Identifier: Apache-2.0
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

//! Reader-level matrix for the original-coordinate map (card
//! **GATEWAY-C-LOCATION**). These drive [`super::walk_marked`] and
//! [`super::map_marked_preprocessing`] on hand-written marker streams, so every branch
//! is reachable without a Clang run; `tests/gateway_c_location.rs` pins the
//! same rules against real preprocessings.

use super::{LocationRefusal, OriginFile, map_marked_preprocessing, unlocated_site, walk_marked};

/// A marked stream and the `-E -P` text it must pair with.
fn map(marked: &str, plain: &str) -> super::SourceLineMap {
    map_marked_preprocessing(marked, plain)
}

#[test]
fn a_primary_line_is_named_by_the_caller_not_by_the_marker() {
    // The opening marker spells `<stdin>`; nothing presents that spelling.
    let marked = "# 1 \"<stdin>\"\nint a(void);\nint b(void);\n";
    let plain = "int a(void);\nint b(void);\n";
    let map = map(marked, plain);
    assert!(map.is_established());
    let first = map.original(1).unwrap();
    assert_eq!(first.depth, 0);
    assert_eq!(first.file, OriginFile::Primary);
    assert_eq!(first.line, 1);
    assert_eq!(map.present(1, "src/unit.c").unwrap(), "src/unit.c:1");
    assert_eq!(map.present(2, "src/unit.c").unwrap(), "src/unit.c:2");
}

#[test]
fn an_entered_file_is_named_by_its_own_entering_marker() {
    let marked =
        "# 1 \"<stdin>\"\n# 1 \"inc/h.h\" 1\nint h(void);\n# 4 \"<stdin>\" 2\nint a(void);\n";
    let plain = "int h(void);\nint a(void);\n";
    let map = map(marked, plain);
    let header = map.original(1).unwrap();
    assert_eq!(header.depth, 1);
    assert_eq!(header.file, OriginFile::Included("inc/h.h".into()));
    assert_eq!(map.present(1, "src/unit.c").unwrap(), "inc/h.h:1");
    // …and the return puts the primary frame back on the line it declares.
    assert_eq!(map.present(2, "src/unit.c").unwrap(), "src/unit.c:4");
}

#[test]
fn a_flagless_marker_that_agrees_with_its_frame_only_resynchronises() {
    // Clang's own resynchronisation after a skipped region, and the shape a
    // bare `#line 500` is re-emitted as. It carries the frame's own name, so
    // the frame keeps its identity and only the counter moves.
    let marked = "# 1 \"<stdin>\"\nint a(void);\n# 500 \"<stdin>\"\nint b(void);\n";
    let plain = "int a(void);\nint b(void);\n";
    let map = map(marked, plain);
    assert!(map.is_established());
    assert_eq!(map.relabelled_lines(), 0);
    assert_eq!(map.present(1, "u.c").unwrap(), "u.c:1");
    assert_eq!(map.present(2, "u.c").unwrap(), "u.c:500");
}

#[test]
fn a_flagless_marker_naming_another_file_withdraws_the_coordinate() {
    // `#line 500 "elsewhere.h"` written in the main source. The name is never
    // adopted, and the lines it governs carry no coordinate at all.
    let marked =
        "# 1 \"<stdin>\"\nint a(void);\n# 500 \"elsewhere.h\"\nint b(void);\nint c(void);\n";
    let plain = "int a(void);\nint b(void);\nint c(void);\n";
    let map = map(marked, plain);
    assert!(map.is_established());
    assert_eq!(map.present(1, "u.c").unwrap(), "u.c:1");
    assert_eq!(map.original(2), None);
    assert_eq!(map.original(3), None);
    assert_eq!(map.relabelled_lines(), 2);
    // The fallback names the internal position as internal.
    assert_eq!(map.site(2, "u.c"), unlocated_site(2));
    assert_eq!(map.site(2, "u.c"), "preprocessed line 2");
}

#[test]
fn a_header_that_writes_the_primary_spelling_cannot_present_as_the_primary() {
    // The measured E01-TAIL direction, seen from the presentation side: a
    // `#line 100 "<stdin>"` inside a header. Depth is unmoved (flag 1 entered
    // the header and only flag 2 leaves it), and the disagreeing flagless
    // marker withdraws the coordinate rather than handing it the main file.
    let marked = "# 1 \"<stdin>\"\n# 1 \"inc/h.h\" 1\n# 100 \"<stdin>\"\nint h(void);\n# 3 \"<stdin>\" 2\nint a(void);\n";
    let plain = "int h(void);\nint a(void);\n";
    let map = map(marked, plain);
    assert_eq!(map.original(1), None, "the header line claims no file");
    let main = map.original(2).unwrap();
    assert_eq!(main.depth, 0);
    assert_eq!(main.file, OriginFile::Primary);
}

#[test]
fn a_macro_expansion_presents_its_expansion_site() {
    // Clang writes an expansion out at the invocation's own line and emits no
    // marker for it, so the reading is the site's.
    let marked = "# 1 \"<stdin>\"\nint a(void);\n\n\nint b(void) { return 0; }\n";
    let plain = "int a(void);\nint b(void) { return 0; }\n";
    let map = map(marked, plain);
    assert_eq!(map.present(2, "u.c").unwrap(), "u.c:4");
}

#[test]
fn an_opening_marker_is_required() {
    let map = map("int a(void);\n", "int a(void);\n");
    assert!(!map.is_established());
    assert_eq!(
        map.refusals(),
        [LocationRefusal::PrimaryInputUnidentified].as_slice()
    );
    assert_eq!(map.original(1), None);
}

#[test]
fn an_underflowing_include_stack_is_refused() {
    let marked = "# 1 \"<stdin>\"\n# 1 \"<stdin>\" 2\nint a(void);\n";
    let map = map(marked, "int a(void);\n");
    assert_eq!(
        map.refusals(),
        [LocationRefusal::UnbalancedIncludeMarkers].as_slice()
    );
}

#[test]
fn an_unclosed_include_stack_is_refused() {
    let marked = "# 1 \"<stdin>\"\n# 1 \"h.h\" 1\nint h(void);\n";
    let map = map(marked, "int h(void);\n");
    assert_eq!(
        map.refusals(),
        [LocationRefusal::UnbalancedIncludeMarkers].as_slice()
    );
}

#[test]
fn content_that_does_not_pair_is_refused_rather_than_approximated() {
    // Different counts.
    let counts = map(
        "# 1 \"<stdin>\"\nint a(void);\n",
        "int a(void);\nint b(void);\n",
    );
    assert_eq!(
        counts.refusals(),
        [LocationRefusal::ContentDisagreement].as_slice()
    );
    // Same count, different text.
    let text = map("# 1 \"<stdin>\"\nint a(void);\n", "int b(void);\n");
    assert_eq!(
        text.refusals(),
        [LocationRefusal::ContentDisagreement].as_slice()
    );
}

#[test]
fn leading_whitespace_is_not_a_disagreement() {
    // `-E` keeps a line's original indentation; `-P` collapses it. That is the
    // one measured difference between the two outputs' content lines.
    let map = map("# 1 \"<stdin>\"\n    int a(void);\n", " int a(void);\n");
    assert!(map.is_established());
    assert_eq!(map.present(1, "u.c").unwrap(), "u.c:1");
}

#[test]
fn blank_lines_carry_no_coordinate_and_shift_nothing() {
    let marked = "# 1 \"<stdin>\"\nint a(void);\n\nint b(void);\n";
    let plain = "int a(void);\n\n\n\nint b(void);\n";
    let map = map(marked, plain);
    assert!(map.is_established());
    assert_eq!(map.present(1, "u.c").unwrap(), "u.c:1");
    assert_eq!(map.original(2), None);
    assert_eq!(map.original(4), None);
    assert_eq!(map.present(5, "u.c").unwrap(), "u.c:3");
}

#[test]
fn a_pragma_is_not_a_marker() {
    let marked = "# 1 \"<stdin>\"\n#pragma once\nint a(void);\n";
    let plain = "#pragma once\nint a(void);\n";
    let map = map(marked, plain);
    assert!(map.is_established());
    // The pragma is an ordinary content line of the primary input.
    assert_eq!(map.present(1, "u.c").unwrap(), "u.c:1");
    assert_eq!(map.present(2, "u.c").unwrap(), "u.c:2");
}

#[test]
fn the_preamble_sits_out_of_the_primary_frame() {
    // `<built-in>` and `<command line>`, exactly as Clang emits them.
    let marked = "# 1 \"<stdin>\"\n# 1 \"<built-in>\" 1\n# 1 \"<built-in>\" 3\n# 410 \"<built-in>\" 3\n# 1 \"<command line>\" 1\n# 1 \"<built-in>\" 2\n# 1 \"<stdin>\" 2\nint a(void);\n";
    let plain = "int a(void);\n";
    let map = map(marked, plain);
    assert!(map.is_established());
    let reading = map.original(1).unwrap();
    assert_eq!(reading.depth, 0);
    assert_eq!(reading.file, OriginFile::Primary);
    assert_eq!(reading.line, 1);
}

#[test]
fn a_marker_free_stream_gives_every_line_a_reading() {
    // Sanity on the walker itself: content lines advance the frame by one each.
    let walk = walk_marked("# 1 \"<stdin>\"\na\nb\nc\n");
    let readings = walk
        .lines
        .iter()
        .filter(|(text, _)| !text.trim().is_empty())
        .map(|(_, reading)| reading.as_ref().unwrap().line)
        .collect::<Vec<_>>();
    assert_eq!(readings, vec![1, 2, 3]);
}

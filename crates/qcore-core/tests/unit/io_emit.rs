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

//! `qcore_io` — the one printing route (card `TRACE-STDIO`).

use super::*;

#[test]
fn a_write_is_always_exactly_one_line() {
    // The newline is the helper's, not the caller's: there is no argument a
    // caller could pass that produces a partial line, which is why every
    // observation a backend emits is line-structured by construction.
    assert_eq!(
        write_text(Stream::Out, "return"),
        "QCORE_IO_WRITE(stdout, \"return\\n\");"
    );
    assert_eq!(
        write_line(Stream::Out, "OK %\" PRId32 \"", &["q_result.value"]),
        "QCORE_IO_WRITE(stdout, \"OK %\" PRId32 \"\\n\", q_result.value);"
    );
}

#[test]
fn the_stream_comes_from_the_std_io_vocabulary() {
    // `Stream` is `source.std-io.v1`'s own type. A backend cannot name a
    // stream this route does not model, because there is no other spelling.
    assert!(write_text(Stream::Out, "x").contains("(stdout, "));
    assert!(write_text(Stream::Err, "x").contains("(stderr, "));
    assert_eq!(flush(Stream::Err), "QCORE_IO_FLUSH(stderr);");
}

#[test]
fn several_conversions_keep_their_argument_order() {
    assert_eq!(
        write_line(
            Stream::Out,
            "pair %\" PRIu64 \" %\" PRIu64 \"",
            &["left", "right"],
        ),
        "QCORE_IO_WRITE(stdout, \"pair %\" PRIu64 \" %\" PRIu64 \"\\n\", left, right);"
    );
}

#[test]
fn the_prelude_defines_the_route_and_nothing_else() {
    // Self-containedness matters: a backend inserts this after its own
    // `<stdio.h>` and must not acquire a new header dependency or an object.
    assert!(IO_PRELUDE.contains("#define QCORE_IO_WRITE(stream, ...)"));
    assert!(IO_PRELUDE.contains("#define QCORE_IO_FLUSH(stream)"));
    assert!(!IO_PRELUDE.contains("#include"));
    assert!(!IO_PRELUDE.contains("static "));
}

#[test]
fn the_prelude_is_the_only_exempt_text() {
    // A backend cannot smuggle a direct call past the single-route check by
    // naming something similarly: the exemption is whole-line identity with
    // the prelude, not a substring.
    let smuggled = "#define QCORE_IO_WRITER(s, ...) fprintf(s, __VA_ARGS__)\n";
    assert_eq!(
        direct_stdio_calls(smuggled),
        vec![(1, "fprintf")],
        "a look-alike macro must not be exempt"
    );
    assert!(direct_stdio_calls(IO_PRELUDE).is_empty());
}

#[test]
fn every_direct_output_spelling_is_reported() {
    // The scan is the structural half of "one route"; if it silently missed a
    // spelling, a backend could reintroduce a second route unnoticed.
    for spelling in [
        "printf", "fprintf", "vfprintf", "vprintf", "fputs", "puts", "fwrite",
    ] {
        let line = format!("    {spelling}(stdout, \"x\");\n");
        assert_eq!(
            direct_stdio_calls(&line).len(),
            1,
            "unreported direct output spelling: {spelling}"
        );
    }
    assert!(direct_stdio_calls("    QCORE_IO_WRITE(stdout, \"x\\n\");\n").is_empty());
}

/// The two-sided tie with `QCore.SingleEmissionRoute`. Both sides walk the same
/// eleven lines and report the same outcome vector — the reported spelling's
/// length, or `0` for an exempt or call-free line — each computing it from its
/// own procedure. The `fputs` entry is `5`, not the `4` a naive scan would give
/// for the `puts` it also contains: that entry is the maximality property made
/// observable, and it is the one this implementation originally got wrong.
///
/// The theorem is
/// `QCore.SingleEmissionRoute.the_domain_outcomes_are_pinned`; changing either
/// side alone fails here.
#[test]
fn the_domain_outcomes_agree_with_the_lean_model() {
    let exempt: Vec<String> = IO_PRELUDE
        .lines()
        .filter(|line| line.starts_with("#define QCORE_IO_"))
        .map(str::to_owned)
        .collect();
    assert_eq!(exempt.len(), 2, "the prelude defines exactly two macros");

    let mut domain: Vec<String> = [
        "vfprintf", "fprintf", "vprintf", "printf", "fwrite", "fputs", "puts",
    ]
    .into_iter()
    .map(|spelling| format!("    {spelling}(stdout, \"x\");"))
    .collect();
    domain.push("    QCORE_IO_WRITE(stdout, \"x\\n\");".to_owned());
    domain.push("    QCORE_IO_FLUSH(stderr);".to_owned());
    domain.extend(exempt);
    assert_eq!(domain.len(), 11);

    let outcomes: Vec<usize> = domain
        .iter()
        .map(|line| match direct_stdio_calls(line).first() {
            None => 0,
            Some((_, spelling)) => spelling.len(),
        })
        .collect();
    assert_eq!(outcomes, vec![8, 7, 7, 6, 6, 5, 4, 0, 0, 0, 0]);
}

#[test]
fn the_longest_matching_spelling_is_the_one_reported() {
    assert_eq!(
        direct_stdio_calls("    fprintf(stderr, \"x\");\n"),
        vec![(1, "fprintf")]
    );
    assert_eq!(
        direct_stdio_calls("    vfprintf(stderr, f, a);\n"),
        vec![(1, "vfprintf")]
    );
}

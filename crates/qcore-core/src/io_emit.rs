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

//! `qcore_io` — the source-authored I/O route in an emitted QCore program.
//!
//! # What this module is
//!
//! This module is exclusively the lowering target for source
//! `print`/`log`/`flush` statements. Compiler observations use the typed
//! diagnostic logger and cannot become program-stream bytes through here.
//!
//! * [`IO_PRELUDE`] is inserted only when source-authored I/O exists. It
//!   defines `QCORE_IO_WRITE` and `QCORE_IO_FLUSH`.
//! * [`TEXT_PRELUDE`] is inserted only when a backend renders **text** — a byte
//!   range whose length is known and which may contain any byte, `0` included
//!   (card `STRING-SLICE-IO`). It defines `QCORE_IO_WRITE_TEXT` and is a
//!   *separate* fragment precisely so that every program that renders no text is
//!   byte-identical to what it was before that card.
//! * [`write_line`], [`write_text`], [`write_text_bytes`] and [`flush`] are the
//!   Rust-side constructors for the sites a backend builds programmatically.
//! * [`direct_stdio_calls`] checks that no direct standard-output call bypasses
//!   this source route.
//!
//! # Why text needs a second spelling, measured rather than assumed
//!
//! Every conversion `fprintf` offers for a byte range stops at the first `0`:
//! `%s` scans to a terminator and `%.*s` takes a precision but still stops
//! early. A Quirin `string` is a `{ptr, len}` view whose payload may contain an
//! embedded `0` as an ordinary byte — `"ab\0cd"` has length 5 where `strlen`
//! would say 2, which `QCore.StdString.an_embedded_nul_is_inside_the_length`
//! states as a theorem — so rendering one through any `printf` conversion would
//! write a *different string* than the one the checker admitted and the
//! reference execution predicted. [`TEXT_PRELUDE`] therefore writes the payload
//! with a length-bounded `fwrite`, and checks its return so that a short write
//! is a failed run rather than a silently truncated line.
//!
//! # Why a macro and not a variadic function
//!
//! A `static void qcore_io_write(FILE *, const char *, ...)` wrapper would be
//! one route too, and it would read more like a helper. It was rejected by
//! measurement of what it costs: these programs are built with
//! `-Wall -Wextra -Werror` on Unix and `/W4 /WX` on MSVC, and a wrapper hides
//! every call site's format string from the compiler, silently dropping the
//! `-Wformat` / C4477 checking that today catches a conversion/argument
//! mismatch in generated code. The portable way to keep that checking is for
//! the call site to remain a literal `fprintf` invocation, which is what a
//! macro gives. `__attribute__((format(printf, ...)))` would restore it for
//! GCC and Clang only, and the Windows subset must compile under `cl`.
//!
//! So the route is single in the sense that matters here: an emitted program
//! contains exactly one `fprintf` spelling, in [`IO_PRELUDE`], and every line
//! it writes goes through it.
//!
//! # What is not claimed
//!
//! * `QCORE_IO_FLUSH` is sink handoff, not durability — no `fsync`, no device
//!   write, no crash survival. That is the same boundary `source.std-io.v1`
//!   already stated for `flush`.
//! * No order between the two streams is modeled. A backend that writes to
//!   both writes two independent observations.
//! * Routing does not make a trace line *checked*. Each backend remains
//!   responsible for the line it asks this module to write; this module
//!   decides only how the line reaches a stream.

use crate::c_program::{Fragment, Section};
use crate::std_io::Stream;

/// The deduplication key of [`IO_PRELUDE`] in an assembled translation unit.
pub const IO_PRELUDE_KEY: &str = "qcore_io.prelude";

/// The deduplication key of [`TEXT_PRELUDE`].
pub const TEXT_PRELUDE_KEY: &str = "qcore_io.text_prelude";

/// The deduplication key of the `<stdio.h>` include this route needs.
pub const STDIO_INCLUDE_KEY: &str = "include.stdio";

/// [`IO_PRELUDE`] as an assembler fragment (card `C-EMIT-MAIN-UNIFY`).
///
/// Every backend that installs the route names it through this constructor, so
/// two routes composed into one program contribute the same key with the same
/// text and the assembler gives the program one copy. That is the mechanical
/// form of what `TRACE-STDIO` stated in prose: the prelude is installed **once
/// per emitted program**.
#[must_use]
pub fn io_prelude_fragment() -> Fragment {
    Fragment::new(Section::Prelude, IO_PRELUDE_KEY, IO_PRELUDE)
}

/// [`TEXT_PRELUDE`] as an assembler fragment.
///
/// A backend contributes this **only** for a program that renders text, which
/// is what keeps every program that renders none byte-identical to what it was
/// before card `STRING-SLICE-IO`. The conditional stays the contributor's: the
/// assembler emits what it is handed.
#[must_use]
pub fn text_prelude_fragment() -> Fragment {
    Fragment::new(Section::Prelude, TEXT_PRELUDE_KEY, TEXT_PRELUDE)
}

/// The `<stdio.h>` include, for a backend whose own include block omits it and
/// adds it only when the program writes.
#[must_use]
pub fn stdio_include_fragment() -> Fragment {
    Fragment::new(Section::Includes, STDIO_INCLUDE_KEY, "#include <stdio.h>\n")
}

/// The C fragment defining the single emission route. A backend inserts this
/// after its `<stdio.h>` include and before any code that writes.
///
/// It is self-contained: it declares no object, allocates nothing, and needs
/// no header beyond `<stdio.h>`.
pub const IO_PRELUDE: &str = r#"
/* qcore_io: the source-authored I/O route in this program.
   `std.io`'s `print` and `std.logger`'s `log` expand to QCORE_IO_WRITE;
   compiler diagnostic events use a separate typed sink. The only `fprintf`
   spelling in this file is the one below. A macro rather than a variadic
   wrapper, so each call site keeps its -Wformat / C4477 checking under
   `-Wall -Wextra -Werror` and `/W4 /WX`. QCORE_IO_FLUSH is sink handoff, not
   durability: no fsync is performed and no crash survival is claimed. */
#define QCORE_IO_WRITE(stream, ...) ((void) fprintf((stream), __VA_ARGS__))
#define QCORE_IO_FLUSH(stream) ((void) fflush((stream)))
"#;

/// The C fragment defining the **text** write (card `STRING-SLICE-IO`).
///
/// Inserted only by a backend that renders a byte range, and only into a
/// program that contains one, so a program without text output is byte-identical
/// to what it was before this card. It needs `<stdio.h>` and `<stdlib.h>`.
///
/// `fwrite` rather than any `printf` conversion, because the payload may contain
/// an embedded `0` and every conversion stops there. The return value is
/// compared rather than discarded: a short write means the bytes the reference
/// execution predicted did not reach the stream, and a run that quietly wrote a
/// prefix of a line would falsify the byte comparison this route's evidence
/// rests on. It is a real check and not a witness — unlike the slice helper's
/// bounds comparison, nothing in the checker rules a short write out.
pub const TEXT_PRELUDE: &str = r#"
/* qcore_io text: rendering a {ptr, len} byte range (card STRING-SLICE-IO).
   `fwrite` and not a printf conversion, because a Quirin string may contain an
   embedded NUL as an ordinary byte inside its length, and `%s` and `%.*s` both
   stop there: rendering through one would write a different string than the one
   the checker admitted. The prefix is the sink's own line framing (`[INFO] `,
   or empty for `print`) and the trailing newline is that framing's, so one
   source line is one written line. The fwrite result is checked, not discarded:
   a short write is a failed run, never a silently truncated line. */
#define QCORE_IO_WRITE_TEXT(stream, prefix, ptr, len) \
    do { \
        QCORE_IO_WRITE((stream), "%s", (prefix)); \
        if (fwrite((ptr), 1u, (size_t) (len), (stream)) != (size_t) (len)) { \
            abort(); \
        } \
        QCORE_IO_WRITE((stream), "\n"); \
    } while (0)
"#;

/// One emitted write, as a C statement without indentation or a trailing
/// newline in the *generated text*.
///
/// `body` is the body of the C string literal **without** its terminating
/// `\n`, which this function appends: a caller cannot ask for a partial line.
/// The body may splice `<inttypes.h>` conversion macros in the ordinary C way
/// (`QCORE_TASK_JOIN %" PRIu64 "`), because it is pasted between the literal's
/// outer quotes exactly as written.
///
/// `arguments` are the C expressions the conversions consume, in order.
pub fn write_line(stream: Stream, body: &str, arguments: &[&str]) -> String {
    let mut call = format!("QCORE_IO_WRITE({}, \"{body}\\n\"", stream.c_handle());
    for argument in arguments {
        call.push_str(", ");
        call.push_str(argument);
    }
    call.push_str(");");
    call
}

/// One emitted write of a fixed line with no conversions.
pub fn write_text(stream: Stream, body: &str) -> String {
    write_line(stream, body, &[])
}

/// One emitted line whose body is a **byte range of known length** (card
/// `STRING-SLICE-IO`).
///
/// `prefix` is the sink's own framing, written verbatim into a C string literal
/// — `""` for `print`, `"[INFO] "` for `log info`. `pointer` and `length` are C
/// expressions denoting the view's two fields. The newline is the framing's, so
/// one source line is one written line, exactly as [`write_line`] guarantees for
/// a rendered scalar.
///
/// The bytes between the prefix and the newline are the view's own, unchanged:
/// no re-encoding, no escaping, no terminator. A caller that wanted to escape
/// them would be writing a different observation than the one the checker
/// admitted.
pub fn write_text_bytes(stream: Stream, prefix: &str, pointer: &str, length: &str) -> String {
    format!(
        "QCORE_IO_WRITE_TEXT({}, \"{prefix}\", {pointer}, {length});",
        stream.c_handle()
    )
}

/// Hand a stream to its sink. Sink handoff, not durability.
pub fn flush(stream: Stream) -> String {
    format!("QCORE_IO_FLUSH({});", stream.c_handle())
}

/// Every standard-output call in `source` that is **not** part of
/// [`IO_PRELUDE`], as one-based line numbers with the offending spelling.
///
/// This is the structural statement that the route is single: for a program
/// emitted by any backend the result must be empty. It is deliberately a
/// lexical scan rather than a parse — it is checking that a spelling does not
/// occur, and a spelling is exactly what a scan decides.
///
/// The preludes' own lines are the only exemption — [`IO_PRELUDE`]'s two
/// `#define`s and [`TEXT_PRELUDE`]'s one, plus the comments above them, which
/// name the spellings they are replacing. They are recognized by whole-line
/// identity with those constants rather than by a substring, so a backend cannot
/// smuggle a direct call past this check by naming a macro similarly.
///
/// [`TEXT_PRELUDE`] joins the exemption rather than weakening the claim: the
/// route is still single in the sense that matters, because an emitted program
/// contains exactly one `fprintf` spelling and exactly one `fwrite` spelling,
/// both in a prelude, and every line it writes goes through one of them.
pub fn direct_stdio_calls(source: &str) -> Vec<(usize, &'static str)> {
    // Longest first: `fprintf` contains `printf` and `fputs` contains `puts`,
    // so this order is what makes the reported spelling the real one.
    const SPELLINGS: [&str; 7] = [
        "vfprintf", "fprintf", "vprintf", "printf", "fwrite", "fputs", "puts",
    ];
    let exempt: Vec<&str> = IO_PRELUDE.lines().chain(TEXT_PRELUDE.lines()).collect();
    let mut found = Vec::new();
    for (index, line) in source.lines().enumerate() {
        if exempt.contains(&line) {
            continue;
        }
        if let Some(spelling) = SPELLINGS.iter().find(|spelling| line.contains(*spelling)) {
            found.push((index + 1, *spelling));
        }
    }
    found
}

#[cfg(test)]
#[path = "../tests/unit/io_emit.rs"]
mod tests;

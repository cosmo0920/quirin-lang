//! `qcore_io` — the one printing route in an emitted QCore program
//! (card `TRACE-STDIO`).
//!
//! # What this module is
//!
//! Before this card every C backend synthesized its own `printf`/`fprintf`
//! calls. There were therefore two unrelated printing mechanisms in emitted
//! programs: the one `source.std-io.v1` lowers a source `print`/`log`/`flush`
//! onto, and four backends' ad-hoc trace instrumentation. This module is the
//! single route both now travel:
//!
//! * [`IO_PRELUDE`] is the C fragment every emitting backend inserts after its
//!   `<stdio.h>` include. It defines `QCORE_IO_WRITE` and `QCORE_IO_FLUSH`.
//! * [`write_line`], [`write_text`] and [`flush`] are the Rust-side
//!   constructors for the sites a backend builds programmatically.
//! * [`direct_stdio_calls`] is the structural check that the route is single:
//!   an emitted program must contain no standard-output call outside the
//!   prelude's own definitions.
//!
//! # What did not change
//!
//! **The bytes on the wire.** Every trace line an emitted program writes is
//! byte-identical to the line it wrote before this card; the mechanism moved
//! and the rendering did not. Where a reader pins those bytes — the F02 trace
//! relation's observer, `parse_c_observation`, the mutex and extern reference
//! comparisons, `examples/threads/README.md` — the pin is unchanged. What did
//! move is the *emitted C text*, and therefore the digests taken over it.
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

use crate::std_io::Stream;

/// The C fragment defining the single emission route. A backend inserts this
/// after its `<stdio.h>` include and before any code that writes.
///
/// It is self-contained: it declares no object, allocates nothing, and needs
/// no header beyond `<stdio.h>`.
pub const IO_PRELUDE: &str = r#"
/* qcore_io (card TRACE-STDIO): the one printing route in this program.
   `std.io`'s `print`, `std.logger`'s `log` and every checked trace line the
   compiler synthesizes expand to QCORE_IO_WRITE, so the only `fprintf`
   spelling in this file is the one below. A macro rather than a variadic
   wrapper, so each call site keeps its -Wformat / C4477 checking under
   `-Wall -Wextra -Werror` and `/W4 /WX`. QCORE_IO_FLUSH is sink handoff, not
   durability: no fsync is performed and no crash survival is claimed. */
#define QCORE_IO_WRITE(stream, ...) ((void) fprintf((stream), __VA_ARGS__))
#define QCORE_IO_FLUSH(stream) ((void) fflush((stream)))
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
/// The prelude's own lines are the only exemption — its two `#define`s and the
/// comment above them, which names the spelling it is replacing. They are
/// recognized by whole-line identity with [`IO_PRELUDE`] rather than by a
/// substring, so a backend cannot smuggle a direct call past this check by
/// naming a macro similarly.
pub fn direct_stdio_calls(source: &str) -> Vec<(usize, &'static str)> {
    // Longest first: `fprintf` contains `printf` and `fputs` contains `puts`,
    // so this order is what makes the reported spelling the real one.
    const SPELLINGS: [&str; 7] = [
        "vfprintf", "fprintf", "vprintf", "printf", "fwrite", "fputs", "puts",
    ];
    let exempt: Vec<&str> = IO_PRELUDE.lines().collect();
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

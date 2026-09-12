#include <inttypes.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

/* qcore_io: the source-authored I/O route in this program.
   `std.io`'s `print` and `std.logger`'s `log` expand to QCORE_IO_WRITE;
   compiler diagnostic events use a separate typed sink. The only `fprintf`
   spelling in this file is the one below. A macro rather than a variadic
   wrapper, so each call site keeps its -Wformat / C4477 checking under
   `-Wall -Wextra -Werror` and `/W4 /WX`. QCORE_IO_FLUSH is sink handoff, not
   durability: no fsync is performed and no crash survival is claimed. */
#define QCORE_IO_WRITE(stream, ...) ((void) fprintf((stream), __VA_ARGS__))
#define QCORE_IO_FLUSH(stream) ((void) fflush((stream)))

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

/* std.string / source.string.v1 (card STRING-SLICE): the whole mechanism of an
   immutable UTF-8 string view, defined here rather than linked.

   A view is `{ ptr, len }` and nothing else. There is deliberately no `cap`:
   an immutable string has no append operation, so capacity would grow the
   value and its ABI size without enabling anything. There is deliberately no
   allocation and no copy: a literal is static storage and a slice is pointer
   addition plus length subtraction, so `len` and slicing are O(1) in the
   number of bytes.

   `len` is a field, never a scan. `strlen` appears nowhere in this program and
   could not be correct if it did: a payload may contain an embedded NUL, which
   is inside `len` and is an ordinary byte of the text. Each literal array does
   carry a trailing zero after its payload, for toolchain convenience only; it
   is outside `len` and outside the language invariant.

   The bounds comparison in the slice helper is a WITNESS, not a check. Every
   bound in an admitted program is a check-time constant the checker already
   proved satisfies 0 <= low <= high <= len, so the comparison is provably never
   taken; if it ever fired, that would be a defect in the checker. */
typedef struct {
    const unsigned char *ptr;
    size_t len;
} qcore_string_view;

/* std.buffer / source.buffer.v1 (card STD-BUFFER): the whole mechanism of an
   owned byte buffer, defined here rather than linked. `calloc` and not
   `malloc`, because the model's buffer is zero at acquisition and an
   indeterminate byte would be undefined behaviour to read.

   The bounds comparison and the allocation check are WITNESSES, not checks.
   Every index in an admitted program is a check-time constant the checker
   already proved in range, so the comparisons below are provably never taken;
   if one ever fired, that would be a defect in the checker. `abort()` is not
   catchable and yields no QCore error value: a failing run produces no
   observation, which is why every claim on this route is a partial-correctness
   statement.

   Only the accessors this program uses are emitted. These are file-scope
   `static` functions and the emitted C is built with -Wall -Wextra -Werror
   (and /W4 /WX on MSVC), where an unused one is an error — so a program that
   never reads a byte must not carry a reader. */
static unsigned char *qcore_bytes_new(size_t extent)
{
    unsigned char *bytes = (unsigned char *) calloc(extent, 1u);
    if (bytes == NULL) {
        abort();
    }
    return bytes;
}

static void qcore_bytes_drop(unsigned char *bytes)
{
    free(bytes);
}

static void qcore_bytes_store(unsigned char *bytes, size_t extent, size_t index, int32_t value)
{
    if (index >= extent) {
        abort();
    }
    bytes[index] = (unsigned char) value;
}


int main(void)
{
    unsigned char *b_store = qcore_bytes_new((size_t) 6u);
    qcore_bytes_store(b_store, (size_t) 6u, (size_t) (INT32_C(0)), INT32_C(97));
    qcore_bytes_store(b_store, (size_t) 6u, (size_t) (INT32_C(1)), INT32_C(98));
    qcore_bytes_store(b_store, (size_t) 6u, (size_t) (INT32_C(2)), INT32_C(99));
    qcore_bytes_store(b_store, (size_t) 6u, (size_t) (INT32_C(3)), INT32_C(100));
    qcore_bytes_store(b_store, (size_t) 6u, (size_t) (INT32_C(4)), INT32_C(101));
    qcore_bytes_store(b_store, (size_t) 6u, (size_t) (INT32_C(5)), INT32_C(102));
    qcore_string_view v_head = { b_store, (size_t) 2u };
    qcore_string_view v_tail = { b_store + 4u, (size_t) 2u };
    qcore_bytes_store(b_store, (size_t) 6u, (size_t) (INT32_C(2)), INT32_C(88));
    qcore_bytes_store(b_store, (size_t) 6u, (size_t) (INT32_C(3)), INT32_C(89));
    QCORE_IO_WRITE_TEXT(stdout, "", v_head.ptr, v_head.len);
    QCORE_IO_WRITE_TEXT(stdout, "", v_tail.ptr, v_tail.len);
    qcore_string_view v_middle = { b_store + 2u, (size_t) 2u };
    QCORE_IO_WRITE_TEXT(stdout, "", v_middle.ptr, v_middle.len);
    qcore_bytes_drop(b_store);
    return 0;
}

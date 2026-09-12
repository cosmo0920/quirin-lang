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

static qcore_string_view qcore_string_slice(qcore_string_view s, size_t low, size_t high)
{
    qcore_string_view derived;
    if (!(low <= high && high <= s.len)) {
        abort();
    }
    /* { ptr = p + low, len = high - low }. `s.ptr + low` is in range for every
       low <= s.len, one-past-the-end included, and is never dereferenced when
       the derived length is zero. */
    derived.ptr = s.ptr + low;
    derived.len = high - low;
    return derived;
}

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

static const unsigned char qcore_string_literal_2[13] = { 0x68, 0x65, 0x6c, 0x6c, 0x6f, 0x2c, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64, 0x00 };
static const unsigned char qcore_string_composed_0[16] = { 0x48, 0x54, 0x54, 0x50, 0x2f, 0x31, 0x2e, 0x31, 0x20, 0x32, 0x30, 0x30, 0x20, 0x4f, 0x4b, 0x00 };
static const unsigned char qcore_string_composed_1[18] = { 0x68, 0x65, 0x6c, 0x6c, 0x6f, 0x2c, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64, 0x20, 0x28, 0x31, 0x35, 0x29, 0x00 };
static const unsigned char qcore_string_composed_2[6] = { 0x5b, 0x6f, 0x6b, 0x21, 0x5d, 0x00 };

int main(void)
{
    qcore_string_view v_head = { qcore_string_composed_0, (size_t) 15u };
    QCORE_IO_WRITE_TEXT(stdout, "", v_head.ptr, v_head.len);
    int32_t s_n = (int32_t) v_head.len;
    QCORE_IO_WRITE(stdout, "%" PRId32 "\n", s_n);
    qcore_string_view v_subject = { qcore_string_literal_2, (size_t) 12u };
    (void) v_subject;
    qcore_string_view v_mixed = { qcore_string_composed_1, (size_t) 17u };
    QCORE_IO_WRITE_TEXT(stdout, "", v_mixed.ptr, v_mixed.len);
    qcore_string_view qcore_text_7 = qcore_string_slice(v_mixed, (size_t) (INT32_C(0)), (size_t) (INT32_C(5)));
    QCORE_IO_WRITE_TEXT(stdout, "", qcore_text_7.ptr, qcore_text_7.len);
    unsigned char *b_body = qcore_bytes_new((size_t) 3u);
    qcore_bytes_store(b_body, (size_t) 3u, (size_t) (INT32_C(0)), INT32_C(111));
    qcore_bytes_store(b_body, (size_t) 3u, (size_t) (INT32_C(1)), INT32_C(107));
    qcore_bytes_store(b_body, (size_t) 3u, (size_t) (INT32_C(2)), INT32_C(33));
    qcore_string_view v_text = { b_body, (size_t) 3u };
    QCORE_IO_WRITE_TEXT(stdout, "", v_text.ptr, v_text.len);
    qcore_string_view qcore_text_14 = qcore_string_slice(v_text, (size_t) (INT32_C(0)), (size_t) (INT32_C(2)));
    QCORE_IO_WRITE_TEXT(stderr, "[INFO] ", qcore_text_14.ptr, qcore_text_14.len);
    int32_t s_held = (int32_t) v_text.len;
    QCORE_IO_WRITE(stdout, "%" PRId32 "\n", s_held);
    qcore_string_view v_framed = { qcore_string_composed_2, (size_t) 5u };
    qcore_bytes_drop(b_body);
    QCORE_IO_WRITE_TEXT(stdout, "", v_framed.ptr, v_framed.len);
    QCORE_IO_FLUSH(stderr);
    return 0;
}

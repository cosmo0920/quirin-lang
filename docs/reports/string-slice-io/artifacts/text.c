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

static const unsigned char qcore_string_literal_0[16] = { 0x48, 0x54, 0x54, 0x50, 0x2f, 0x31, 0x2e, 0x31, 0x20, 0x32, 0x30, 0x30, 0x20, 0x4f, 0x4b, 0x00 };
static const unsigned char qcore_string_literal_1[25] = { 0x43, 0x6f, 0x6e, 0x74, 0x65, 0x6e, 0x74, 0x2d, 0x54, 0x79, 0x70, 0x65, 0x3a, 0x20, 0x74, 0x65, 0x78, 0x74, 0x2f, 0x70, 0x6c, 0x61, 0x69, 0x6e, 0x00 };
static const unsigned char qcore_string_literal_2[13] = { 0x68, 0x65, 0x6c, 0x6c, 0x6f, 0x2c, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64, 0x00 };
static const unsigned char qcore_string_literal_3[5] = { 0xc3, 0xa9, 0x6f, 0x21, 0x00 };
static const unsigned char qcore_string_literal_4[6] = { 0x61, 0x62, 0x00, 0x63, 0x64, 0x00 };
static const unsigned char qcore_string_literal_5[1] = { 0x00 };

int main(void)
{
    QCORE_IO_WRITE_TEXT(stdout, "", qcore_string_literal_0, (size_t) 15u);
    QCORE_IO_WRITE_TEXT(stdout, "", qcore_string_literal_1, (size_t) 24u);
    qcore_string_view v_body = { qcore_string_literal_2, (size_t) 12u };
    QCORE_IO_WRITE_TEXT(stdout, "", v_body.ptr, v_body.len);
    qcore_string_view v_greeting = qcore_string_slice(v_body, (size_t) (INT32_C(0)), (size_t) (INT32_C(5)));
    QCORE_IO_WRITE_TEXT(stdout, "", v_greeting.ptr, v_greeting.len);
    qcore_string_view qcore_text_6 = qcore_string_slice(v_body, (size_t) (INT32_C(7)), (size_t) (INT32_C(12)));
    QCORE_IO_WRITE_TEXT(stderr, "[INFO] ", qcore_text_6.ptr, qcore_text_6.len);
    qcore_string_view v_accented = { qcore_string_literal_3, (size_t) 4u };
    QCORE_IO_WRITE_TEXT(stdout, "", v_accented.ptr, v_accented.len);
    qcore_string_view qcore_text_9 = qcore_string_slice(v_accented, (size_t) (INT32_C(2)), (size_t) (INT32_C(4)));
    QCORE_IO_WRITE_TEXT(stderr, "[INFO] ", qcore_text_9.ptr, qcore_text_9.len);
    QCORE_IO_WRITE_TEXT(stderr, "[WARN] ", qcore_string_literal_4, (size_t) 5u);
    QCORE_IO_WRITE_TEXT(stdout, "", qcore_string_literal_5, (size_t) 0u);
    int32_t s_n = (int32_t) v_body.len;
    QCORE_IO_WRITE(stdout, "%" PRId32 "\n", s_n);
    QCORE_IO_FLUSH(stderr);
    return 0;
}

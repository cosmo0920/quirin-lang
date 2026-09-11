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

static const unsigned char qcore_string_literal_0[6] = { 0x68, 0x65, 0x6c, 0x6c, 0x6f, 0x00 };
static const unsigned char qcore_string_literal_1[5] = { 0xc3, 0xa9, 0x6f, 0x21, 0x00 };
static const unsigned char qcore_string_literal_2[6] = { 0x61, 0x62, 0x00, 0x63, 0x64, 0x00 };

int main(void)
{
    qcore_string_view v_greeting = { qcore_string_literal_0, (size_t) 5u };
    int32_t s_n = (int32_t) v_greeting.len;
    QCORE_IO_WRITE(stdout, "%" PRId32 "\n", s_n);
    qcore_string_view v_ell = qcore_string_slice(v_greeting, (size_t) (INT32_C(1)), (size_t) (INT32_C(4)));
    int32_t s_m = (int32_t) v_ell.len;
    QCORE_IO_WRITE(stdout, "%" PRId32 "\n", s_m);
    qcore_string_view v_inner = qcore_string_slice(v_ell, (size_t) (INT32_C(1)), (size_t) (INT32_C(2)));
    int32_t s_k = (int32_t) v_inner.len;
    QCORE_IO_WRITE(stdout, "%" PRId32 "\n", s_k);
    qcore_string_view v_empty = qcore_string_slice(v_greeting, (size_t) (INT32_C(5)), (size_t) (INT32_C(5)));
    int32_t s_zero = (int32_t) v_empty.len;
    QCORE_IO_WRITE(stdout, "%" PRId32 "\n", s_zero);
    qcore_string_view v_accented = { qcore_string_literal_1, (size_t) 4u };
    int32_t s_accented_len = (int32_t) v_accented.len;
    QCORE_IO_WRITE(stdout, "%" PRId32 "\n", s_accented_len);
    qcore_string_view v_tail = qcore_string_slice(v_accented, (size_t) (INT32_C(2)), (size_t) (INT32_C(4)));
    int32_t s_tail_len = (int32_t) v_tail.len;
    QCORE_IO_WRITE(stdout, "%" PRId32 "\n", s_tail_len);
    qcore_string_view v_with_nul = { qcore_string_literal_2, (size_t) 5u };
    int32_t s_nul_len = (int32_t) v_with_nul.len;
    QCORE_IO_WRITE(stderr, "[WARN] %" PRId32 "\n", s_nul_len);
    QCORE_IO_FLUSH(stderr);
    return 0;
}

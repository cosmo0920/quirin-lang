#include <inttypes.h>
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

static int32_t qcore_bytes_load(const unsigned char *bytes, size_t extent, size_t index)
{
    if (index >= extent) {
        abort();
    }
    return (int32_t) bytes[index];
}

int main(void)
{
    unsigned char *b_store = qcore_bytes_new((size_t) 8u);
    unsigned char *const l_left = b_store;
    unsigned char *const l_right = b_store + 4u;
    qcore_bytes_store(l_left, (size_t) 4u, (size_t) (INT32_C(0)), INT32_C(11));
    qcore_bytes_store(l_right, (size_t) 4u, (size_t) (INT32_C(0)), INT32_C(55));
    qcore_bytes_store(l_left, (size_t) 4u, (size_t) (INT32_C(1)), INT32_C(22));
    qcore_bytes_store(l_right, (size_t) 4u, (size_t) (INT32_C(1)), INT32_C(66));
    int32_t s_b = qcore_bytes_load(l_right, (size_t) 4u, (size_t) (INT32_C(1)));
    int32_t s_a = qcore_bytes_load(l_left, (size_t) 4u, (size_t) (INT32_C(1)));
    int32_t s_x = qcore_bytes_load(b_store, (size_t) 8u, (size_t) (INT32_C(0)));
    int32_t s_y = qcore_bytes_load(b_store, (size_t) 8u, (size_t) (INT32_C(4)));
    QCORE_IO_WRITE(stdout, "%" PRId32 "\n", s_a);
    QCORE_IO_WRITE(stdout, "%" PRId32 "\n", s_b);
    QCORE_IO_WRITE(stdout, "%" PRId32 "\n", s_x);
    QCORE_IO_WRITE(stdout, "%" PRId32 "\n", s_y);
    qcore_bytes_drop(b_store);
    return 0;
}

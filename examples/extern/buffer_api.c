/* The C behind `contracts/buffer_api.qcore`.
 *
 * Nothing in this file is checked by QCore. It exists so the emitted programs
 * in this directory (and in `../stdio` and `../pipes`) perform real calls across
 * a real link boundary, allocate and free real heap bytes, and can be observed
 * by ASan/UBSan/LSan.
 *
 * Its agreement with the declared contract beside it is exactly the trusted
 * premise the certificate records. A sanitized run can *refute* that premise for
 * these programs — a lost release shows up as a leak, a double release as a
 * double free — but no run establishes it in general.
 */
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

/* `resource_return_owned(nullable)`: may return NULL, and the caller's
 * `succeeds true else raise` arm is what handles that. */
uint8_t *demo_alloc(uintptr_t size)
{
    return (uint8_t *) calloc((size_t) size, 1u);
}

/* `resource_borrowed(buffer)` through a `mut` pointer. The eight bytes are this
 * implementation's own choice: no clause transports an extent. */
void demo_fill(uint8_t *buffer, uint32_t value)
{
    memset(buffer, (int) (value & 0xffu), 8u);
}

/* `resource_borrowed(buffer)` through a `const` pointer, returning a scalar. */
int32_t demo_sum(const uint8_t *buffer, uint32_t count)
{
    int32_t total = 0;
    for (uint32_t index = 0; index < count; index += 1u) {
        total += (int32_t) buffer[index];
    }
    return total;
}

/* Scalar in, scalar out. It touches no memory, so a sanitizer says nothing
 * about it beyond "it was called with these arguments" — which is the whole
 * content of "a scalar carries data and no custody". */
int32_t demo_scale(int32_t value, int32_t factor)
{
    return value * factor;
}

/* `resource_consumed(buffer, nonnull)`. This implementation really does free,
 * which is what makes the leak check in the tests a positive control. */
void demo_release(uint8_t *buffer)
{
    free(buffer);
}

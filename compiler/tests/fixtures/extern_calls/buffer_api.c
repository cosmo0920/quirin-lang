/* A real C implementation of the callees declared in
 * `contracts/buffer_api.qcore`, linked against the emitted translation unit for
 * card `source.extern-call.v1`.
 *
 * Nothing here is verified. This file exists so the emitted program performs
 * actual calls across a link boundary, allocates and frees real heap bytes, and
 * can be observed by ASan/UBSan/LSan. Its agreement with the declared contract
 * is exactly the trusted premise the certificate records; a leak checker can
 * refute that premise for these fixtures, but no run establishes it in general.
 */
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

/* `resource_return_owned(nullable)` — succeeds for the sizes these fixtures
 * request, so the emitted declared-outcome check observes `ok`. */
uint8_t *qcore_extern_alloc(uintptr_t size)
{
    uint8_t *buffer = (uint8_t *) calloc((size_t) size, 1u);
    return buffer;
}

/* The same declared shape, always refusing: the fixture that declares
 * `succeeds false` links against this one so the null arm is real. */
uint8_t *qcore_extern_alloc_failing(uintptr_t size)
{
    (void) size;
    return NULL;
}

/* `resource_borrowed(buffer)` with a `mut` pointer: the derived row carries
 * `Mut`, and the callee writes only inside bytes the caller allocated. Extents
 * are not transported by the contract; this fixture's agreement with the size
 * it was given is a property of the fixture, not of the declaration. */
void qcore_extern_fill(uint8_t *buffer, uint32_t value)
{
    memset(buffer, (int) (value & 0xffu), 8u);
}

/* `resource_borrowed(buffer)` on a `const` pointer: no derived `Mut`. */
int32_t qcore_extern_sum(const uint8_t *buffer, uint32_t count)
{
    int32_t total = 0;
    for (uint32_t index = 0; index < count; index += 1u) {
        total += (int32_t) buffer[index];
    }
    return total;
}

/* `resource_consumed(buffer, nonnull)`. The declaration says the caller's
 * obligation moves to the callee, not that the callee destroys the resource;
 * this particular implementation does destroy it, which is what lets LSan see
 * a leak if the checked source ever stops moving the buffer here. */
void qcore_extern_release(uint8_t *buffer)
{
    free(buffer);
}

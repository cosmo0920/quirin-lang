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

/* --- Card EXTERN-F1: the `resource_out` and `resource_stored` callees. --- */

/* `resource_out(slot, nullable)` with `resource_return_none()` and an i32
 * status. The emitted caller initializes the slot to NULL, so a run in which
 * this function forgot to write it would be observed as the null arm and
 * refuted by the emitted declared-outcome check. */
int32_t qcore_extern_open(uint8_t **slot, uintptr_t size)
{
    uint8_t *buffer = (uint8_t *) calloc((size_t) size, 1u);
    *slot = buffer;
    return buffer == NULL ? -1 : 0;
}

/* The same declared shape, always refusing: it writes NULL and reports the
 * failure, so the checked `succeeds false` arm is a real observation. */
int32_t qcore_extern_open_failing(uint8_t **slot, uintptr_t size)
{
    (void) size;
    *slot = NULL;
    return -1;
}

/* `resource_stored(child, owner, nonnull)`. The declaration says the child's
 * obligation becomes indexed beneath the owner, not that this implementation
 * is the only one that could satisfy it; storing the pointer in the owner's
 * first bytes is this fixture's choice. Whether the owner ever releases the
 * child is exactly the premise the certificate records and nothing here
 * discharges. */
void qcore_extern_attach(uint8_t *owner, uint8_t *child)
{
    memcpy(owner, &child, sizeof child);
}

/* --- Card `EXTERN-ERR-REL`: releasing a declared store owner. --- */

/* `resource_consumed(owner, nonnull)`, declared exactly like
 * `qcore_extern_release`. This implementation additionally releases whatever
 * `qcore_extern_attach` stored beneath the owner, which is what lets a leak
 * checker account for both allocations of the store fixtures. That is this
 * fixture satisfying the `StoredCustodyUnderDeclaredOwner` premise for its own
 * runs; no run establishes it in general. An owner that was never attached to
 * still reads the zeroed slot `calloc` gave it, so the child release is a
 * `free(NULL)`. */
void qcore_extern_release_owner(uint8_t *owner)
{
    uint8_t *child = NULL;
    memcpy(&child, owner, sizeof child);
    free(child);
    free(owner);
}

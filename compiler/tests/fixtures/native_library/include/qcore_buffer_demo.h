/* The exported header of the fixture native library `qcore_buffer_demo`
 * (card `NATIVE-LIBRARY-HEADER`).
 *
 * This is a **real** `.h`, not a transcription of one: the manifest
 * `fixtures/native_library/buffer_lib_header.qrlib` names it and its digest,
 * and every declaration below is read from this text and compared against that
 * manifest's own entry spellings. It is written inside the v1 subset the design
 * note defines — an include guard, comments, `#include`s that are never
 * followed, and declarations whose types are C11's own fixed-width typedefs.
 *
 * The implementation these declare is `fixtures/extern_calls/buffer_api.c`,
 * which is what the sanitizer harness links. Nothing here is verified: the
 * declarations state an ABI, and what the code does is the declared contract's
 * business.
 */
#ifndef QCORE_BUFFER_DEMO_H
#define QCORE_BUFFER_DEMO_H

/* Never followed by this reader. `uint8_t` and friends are read through the
 * C11-fixed prelude instead, and a type this header used from anywhere else
 * would refuse by name rather than being guessed. */
#include <stdint.h>

/* Acquire: a nullable owned result. */
uint8_t *qcore_extern_alloc(uintptr_t size);

/* Borrow, mutably. */
void qcore_extern_fill(uint8_t *buffer, uint32_t value);

/* Borrow, immutably — the `const` is load-bearing, and a manifest that dropped
 * it disagrees with this text. */
int32_t qcore_extern_sum(const uint8_t *buffer, uint32_t count);

/* Consume. */
void qcore_extern_release(uint8_t *buffer);

#endif /* QCORE_BUFFER_DEMO_H */

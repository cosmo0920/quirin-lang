/* The same exported surface as `qcore_buffer_demo.h`, written the way a real
 * third-party library writes one: every declaration is decorated with an
 * export-visibility macro whose `#define` lives in a header this reader does not
 * follow (card `NATIVE-LIBRARY-HEADER`).
 *
 * This is the shape the census measured across the whole of Libevent's
 * `event2/` surface, reproduced here so the refusal it produces is pinned in
 * this repository rather than only in a submodule. The declaration is not
 * *wrong*; it is outside the v1 subset, and the refusal names the token that
 * puts it there. Widening to it is follow-on
 * `NATIVE-LIBRARY-HEADER-DECORATION`.
 */
#ifndef QCORE_BUFFER_DEMO_H
#define QCORE_BUFFER_DEMO_H

#include <stdint.h>
#include "qcore_buffer_demo_visibility.h"

QCORE_BUFFER_DEMO_EXPORT
uint8_t *qcore_extern_alloc(uintptr_t size);

QCORE_BUFFER_DEMO_EXPORT
void qcore_extern_fill(uint8_t *buffer, uint32_t value);

QCORE_BUFFER_DEMO_EXPORT
int32_t qcore_extern_sum(const uint8_t *buffer, uint32_t count);

QCORE_BUFFER_DEMO_EXPORT
void qcore_extern_release(uint8_t *buffer);

#endif /* QCORE_BUFFER_DEMO_H */

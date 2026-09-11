/* The same exported surface as `qcore_buffer_demo.h` with **one** projection
 * moved: `qcore_extern_sum` takes a mutable pointer here and a `const` one
 * there (card `NATIVE-LIBRARY-HEADER`).
 *
 * That is the whole fixture. A manifest transcribing the `const` declaration
 * and naming this header disagrees with it, and the disagreement **refuses**
 * rather than withdrawing to the manifest-without-a-header reading — which is
 * accepted, and which is exhibited beside the refusal in the suite.
 */
#ifndef QCORE_BUFFER_DEMO_H
#define QCORE_BUFFER_DEMO_H

#include <stdint.h>

uint8_t *qcore_extern_alloc(uintptr_t size);
void qcore_extern_fill(uint8_t *buffer, uint32_t value);
int32_t qcore_extern_sum(uint8_t *buffer, uint32_t count);
void qcore_extern_release(uint8_t *buffer);

#endif /* QCORE_BUFFER_DEMO_H */

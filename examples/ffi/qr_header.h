// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

/* A small real C library for examples/ffi (card W48-QR-C-STRUCT-IMPORT).
 *
 * `struct qr_header` is imported by `header_sum.qr`, which lists four of its
 * members. The other two — `reserved`, a `char` array, and `label`, a pointer —
 * have no fixed-width scalar equivalent, so they stay unlisted: opaque bytes the
 * program can neither read nor write. */
#ifndef QR_HEADER_H
#define QR_HEADER_H

#include <stdbool.h>
#include <stdint.h>

struct qr_header {
    uint32_t len;
    uint8_t flags;
    char reserved[3];
    int64_t id;
    bool valid;
    const char *label;
};

/* `len + flags`, read through a `const` pointer: the callee neither writes the
 * header nor keeps the pointer. */
uint32_t qr_header_total(const struct qr_header *header);

/* A header built by C and returned by value: `id` is `len * 3`, `valid` is
 * true, and `label` points at a static string. */
struct qr_header qr_header_make(uint32_t len, uint8_t flags);

#endif

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

/* A small real C library for examples/ffi/record_passing.qr (card
 * W48-QR-USABLE-RECORDS).
 *
 * `struct qr_point` is imported with three of its members listed; `note`, a
 * `char` array, has no fixed-width scalar equivalent and stays unlisted:
 * opaque bytes the program can neither read nor write. */
#ifndef QR_POINT_H
#define QR_POINT_H

#include <stdint.h>

struct qr_point {
    int32_t x;
    int32_t y;
    uint16_t weight;
    char note[6];
};

/* `(|x| + |y|) * weight`, read through a `const` pointer: the callee neither
 * writes the point nor keeps the pointer. */
int32_t qr_point_norm(const struct qr_point *point);

/* A copy of `point` moved by `(dx, dy)`, returned by value; `weight` and
 * `note` are copied unchanged. */
struct qr_point qr_point_moved(struct qr_point point, int32_t dx, int32_t dy);

#endif

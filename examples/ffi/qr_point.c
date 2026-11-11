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

/* The C side of examples/ffi/record_passing.qr (card W48-QR-USABLE-RECORDS). */
#include "qr_point.h"

static int32_t magnitude(int32_t value)
{
    return value < 0 ? -value : value;
}

int32_t qr_point_norm(const struct qr_point *point)
{
    return (magnitude(point->x) + magnitude(point->y)) * (int32_t) point->weight;
}

struct qr_point qr_point_moved(struct qr_point point, int32_t dx, int32_t dy)
{
    point.x += dx;
    point.y += dy;
    return point;
}

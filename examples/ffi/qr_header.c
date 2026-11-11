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

/* The implementation behind examples/ffi/qr_header.h. The emitted C of
 * `header_sum.qr` is linked with this object; nothing here is generated. */
#include "qr_header.h"

uint32_t qr_header_total(const struct qr_header *header)
{
    return header->len + (uint32_t) header->flags;
}

struct qr_header qr_header_make(uint32_t len, uint8_t flags)
{
    struct qr_header header = {0};
    header.len = len;
    header.flags = flags;
    header.id = (int64_t) len * 3;
    header.valid = true;
    header.label = "made";
    return header;
}

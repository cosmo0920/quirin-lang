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

/* A real C library with **no QCore contract whatsoever**.
 *
 * That is the point. Nothing in `contract/qcore/` and nothing in any selected
 * environment describes these three functions. The QCore module beside this file
 * declares only their ABI signatures, through the explicit `unsafe extern fn`
 * surface, and its certificate carries each of them by name as an
 * unchecked-linkage premise the driver prints rather than discharges.
 *
 * `demo_hidden_effect` deliberately mutates a private global, so "nothing
 * about its behavior is checked" is not a formality: this callee really does
 * something its caller cannot see, and no part of the toolchain notices.
 */
#include <stdint.h>
#include <stdlib.h>

static int32_t demo_hidden_state = 0;

/* A pure scalar function: handed values, returns a value. Custody-free by
 * construction, which is the whole soundness argument for the scalar-only v1
 * surface. */
int32_t demo_unchecked_add(int32_t a, int32_t b)
{
    return a + b;
}

/* A `void` result. The caller binds nothing and cannot see the state this
 * changes. */
void demo_hidden_effect(uint32_t tag)
{
    demo_hidden_state += (int32_t) tag;
}

/* Allocates and frees behind the caller's back. The caller's ledger is
 * untouched: it was handed nothing and given nothing back. */
uint64_t demo_measure(uint64_t count)
{
    void *scratch = malloc((size_t) count);
    uint64_t answer = scratch == NULL ? 0u : count;
    free(scratch);
    return answer;
}

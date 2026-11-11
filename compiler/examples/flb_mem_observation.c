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

/* Compile against the actual Fluent Bit header and build include paths.
 * Preinclude dependencies before instrumenting allocator calls. The hooks use
 * real libc storage, with deterministic failure injection. No upstream edits.
 */
#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <fluent-bit/flb_info.h>
#include <fluent-bit/flb_macros.h>
#if defined(FLB_HAVE_JEMALLOC) || defined(FLB_HAVE_TESTS_OSSFUZZ)
#error Unsupported allocator profile: requires a separate contract evaluation
#endif
#ifdef FLB_MEM_H
#error flb_mem.h was included before the observation hooks
#endif

static int fail_allocation, backend_calls, live_after, input_live;
static int forwarded_input, memset_calls, zero_value;
static size_t forwarded_size, forwarded_n, zero_offset, zero_length;
static void *input_pointer, *backend_result;
static size_t result_extent;

static void *observe_malloc(size_t size)
{
    backend_calls++;
    forwarded_size = size;
    backend_result = fail_allocation ? NULL : malloc(size);
    if (backend_result) { live_after++; }
    return backend_result;
}
static void *observe_calloc(size_t n, size_t size)
{
    backend_calls++;
    forwarded_size = size;
    forwarded_n = n;
    backend_result = fail_allocation ? NULL : calloc(n, size);
    if (backend_result) { live_after++; }
    return backend_result;
}
static void *observe_realloc(void *ptr, size_t size)
{
    backend_calls++;
    forwarded_size = size;
    forwarded_input = ptr == input_pointer;
    /* Zero-sized realloc is intentionally outside the preserve-on-failure
     * contract. Do not invoke it here or assume its platform-specific result. */
    assert(size > 0);
    backend_result = fail_allocation ? NULL : realloc(ptr, size);
    if (backend_result) { live_after = 1; }
    result_extent = size;
    return backend_result;
}
static void observe_free(void *ptr)
{
    backend_calls++;
    forwarded_input = ptr == input_pointer;
    if (ptr) { live_after--; }
    free(ptr);
}
static void *observe_memset(void *ptr, int value, size_t size)
{
    assert(backend_result != NULL);
    assert((unsigned char *)ptr >= (unsigned char *)backend_result);
    zero_offset = (unsigned char *)ptr - (unsigned char *)backend_result;
    assert(zero_offset <= result_extent && size <= result_extent - zero_offset);
    memset_calls++;
    zero_length = size;
    zero_value = value;
    return memset(ptr, value, size);
}
#define malloc observe_malloc
#define calloc observe_calloc
#define realloc observe_realloc
#define free observe_free
#define memset observe_memset
#include <fluent-bit/flb_mem.h>
#undef malloc
#undef calloc
#undef realloc
#undef free
#undef memset

static void reset(int failure, size_t old_size)
{
    fail_allocation = failure;
    backend_calls = memset_calls = zero_value = 0;
    forwarded_size = forwarded_n = zero_offset = zero_length = result_extent = 0;
    forwarded_input = 0;
    backend_result = NULL;
    input_pointer = old_size ? malloc(old_size) : NULL;
    assert(old_size == 0 || input_pointer != NULL);
    if (input_pointer) { memset(input_pointer, 0xa5, old_size); }
    input_live = live_after = input_pointer != NULL;
}
static int bytes_equal(void *pointer, size_t offset, size_t count, unsigned char value)
{
    size_t i;
    for (i = offset; i < offset + count; i++) {
        if (((unsigned char *)pointer)[i] != value) { return 0; }
    }
    return 1;
}
static unsigned int sequence;
static void record(const char *function, size_t n, size_t size, size_t old_size,
                   void *result, int zeroed, int prefix, int growth)
{
    printf("{\"function\":\"%s\",\"case\":%u,\"values\":{"
           "\"ordinary_allocator_profile\":1,\"valid_input_extent\":1,"
           "\"n\":%zu,\"size\":%zu,\"new_size\":%zu,\"old_size\":%zu,"
           "\"failure_injected\":%d,\"input_live\":%d,\"live_after\":%d,"
           "\"result_null\":%d,\"result_is_backend\":%d,\"backend_calls\":%d,"
           "\"forwarded_size\":%zu,\"forwarded_n\":%zu,\"forwarded_input\":%d,"
           "\"zeroed\":%d,\"prefix_preserved\":%d,\"growth_zeroed\":%d,"
           "\"memset_calls\":%d,\"zero_offset\":%zu,\"zero_length\":%zu,\"zero_value\":%d}}\n",
           function, ++sequence, n, size, size, old_size, fail_allocation,
           input_live, live_after, result == NULL, result == backend_result,
           backend_calls, forwarded_size, forwarded_n, forwarded_input,
           zeroed, prefix, growth, memset_calls, zero_offset, zero_length, zero_value);
}
int main(void)
{
    const size_t sizes[] = {0, 1, 5, 16, 64};
    size_t a, b, old_size, size, kept;
    int failure, zeroing, prefix, growth;
    void *result;
    for (failure = 0; failure <= 1; failure++) {
        for (a = 0; a < 5; a++) {
            size = sizes[a];
            reset(failure, 0);
            result = flb_malloc(size);
            record("flb_malloc", 0, size, 0, result, 0, 1, 0);
            free(result);
            for (b = 0; b < 5; b++) {
                reset(failure, 0);
                result = flb_calloc(sizes[b], size);
                record("flb_calloc", sizes[b], size, 0, result,
                       !result || bytes_equal(result, 0, sizes[b] * size, 0), 1, 0);
                free(result);
            }
        }
        for (zeroing = 0; zeroing <= 1; zeroing++) {
            for (a = 0; a < 5; a++) {
                old_size = sizes[a];
                for (b = 1; b < 5; b++) {
                    size = sizes[b];
                    reset(failure, old_size);
                    result = zeroing ? flb_realloc_z(input_pointer, old_size, size)
                                     : flb_realloc(input_pointer, size);
                    kept = result ? (old_size < size ? old_size : size) : old_size;
                    prefix = bytes_equal(result ? result : input_pointer, 0, kept, 0xa5);
                    growth = !zeroing || !result || size <= old_size ||
                        bytes_equal(result, old_size, size - old_size, 0);
                    record(zeroing ? "flb_realloc_z" : "flb_realloc", 0,
                           size, old_size, result, 0, prefix, growth);
                    free(result ? result : input_pointer);
                }
            }
        }
    }
    for (a = 0; a < 5; a++) {
        reset(0, sizes[a]);
        flb_free(input_pointer);
        record("flb_free", 0, 0, sizes[a], NULL, 0, 1, 0);
    }
    return 0;
}

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

/* Observe the actual pinned helper bodies with controlled syscall results.
 * These hooks perform no OS close/unmap. ASan/UBSan cover the compiled source.
 * Endpoint snapshots observe fields, not the number of assignments performed.
 */
#define _GNU_SOURCE
#include <assert.h>
#include <errno.h>
#include <limits.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>
#include <sys/mman.h>
#include <chunkio/chunkio.h>
#include <chunkio/cio_file.h>
#include <chunkio/cio_log.h>

static int injected_result, calls, reports, arguments_match;
static int expected_fd;
static void *expected_map;
static size_t expected_size;
static int unmapping;

static int observe_close(int fd)
{
    calls++;
    arguments_match = arguments_match && !unmapping && fd == expected_fd;
    if (injected_result == -1) { errno = EIO; }
    return injected_result;
}
static int observe_unmap(void *map, size_t length)
{
    calls++;
    arguments_match = arguments_match && unmapping && map == expected_map && length == expected_size;
    if (injected_result == -1) { errno = EIO; }
    return injected_result;
}
static int observe_errno_print(int error, const char *file, int line)
{
    assert(error == EIO && file && line > 0);
    reports++;
    return 0;
}

#define close observe_close
#define munmap observe_unmap
#define cio_errno_print observe_errno_print
#include QCORE_FILE_CLEANUP_SOURCE
#undef close
#undef munmap
#undef cio_errno_print

static unsigned case_id;
static char mapping[16];
static char path[] = "controlled-file";

static void observe(int is_unmap, int mode, int fd, size_t size, int warned)
{
    struct cio_file cf, before, frame;
    int result, present, success, preserved, reset, other_preserved;
    memset(&cf, 0, sizeof(cf));
    cf.fd = fd;
    cf.map = mode == 1 && is_unmap ? NULL : mapping;
    cf.alloc_size = size;
    cf.map_truncated_warned = warned;
    cf.path = path;
    cf.flags = 13;
    cf.data_size = 9;
    cf.page_size = 4096;
    cf.crc_cur = 71;
    cf.auto_remap_warned = 1;
    if (mode == 1 && !is_unmap) { cf.fd = -1; }
    memcpy(&before, &cf, sizeof(cf));
    expected_fd = cf.fd;
    expected_map = cf.map;
    expected_size = cf.alloc_size;
    injected_result = mode == 2 ? -1 : 0;
    unmapping = is_unmap;
    calls = reports = 0;
    arguments_match = 1;
    result = is_unmap ? cio_file_native_unmap(mode == 0 ? NULL : &cf)
                      : cio_file_native_close(mode == 0 ? NULL : &cf);
    present = mode >= 2;
    success = mode == 3;
    preserved = memcmp(&cf, &before, sizeof(cf)) == 0;
    reset = is_unmap ? cf.map == NULL && cf.alloc_size == 0 && cf.map_truncated_warned == 0
                     : cf.fd == -1;
    memcpy(&frame, &cf, sizeof(cf));
    if (is_unmap) {
        frame.map = before.map;
        frame.alloc_size = before.alloc_size;
        frame.map_truncated_warned = before.map_truncated_warned;
    }
    else { frame.fd = before.fd; }
    other_preserved = memcmp(&frame, &before, sizeof(cf)) == 0;
    assert(calls == present && arguments_match);
    assert(reports == (mode == 2));
    assert(result == (mode == 0 || mode == 2 ? CIO_ERROR : CIO_OK));
    assert(success ? reset : preserved);
    assert(other_preserved);
    printf("{\"function\":\"native_cleanup_observation\",\"case\":%u,"
           "\"helper\":\"%s\",\"mode\":%d,\"fd\":%d,\"size\":%zu,\"warned\":%d,\"values\":{"
           "\"controlled_returning_syscall_hooks\":1,\"exclusive_live_record_or_null\":1,"
           "\"compiled_source_and_build_recorded\":1,\"diagnostic_hook_frames_record\":1,"
           "\"was_null\":%d,\"was_present\":%d,\"injected_success\":%d,\"injected_failure\":%d,"
           "\"returned\":%d,\"cleanup_calls\":%d,\"diagnostic_calls\":%d,"
           "\"arguments_match\":%d,\"fields_preserved\":%d,\"fields_reset\":%d,"
           "\"other_fields_preserved\":%d}}\n",
           case_id++, is_unmap ? "cio_file_native_unmap" : "cio_file_native_close",
           mode, before.fd, size, warned, mode == 0, present, success, mode == 2,
           result, calls, reports, arguments_match, preserved, reset, other_preserved);
}
int main(void)
{
    int descriptors[] = {0, 7, INT_MAX};
    size_t sizes[] = {1, sizeof(mapping)};
    for (int unmap = 0; unmap <= 1; unmap++) {
        observe(unmap, 0, 7, sizeof(mapping), 1);
        observe(unmap, 1, 7, sizeof(mapping), 1);
    }
    for (size_t i = 0; i < sizeof(descriptors) / sizeof(descriptors[0]); i++) {
        observe(0, 2, descriptors[i], sizeof(mapping), 1);
        observe(0, 3, descriptors[i], sizeof(mapping), 1);
    }
    for (size_t i = 0; i < sizeof(sizes) / sizeof(sizes[0]); i++) {
        for (int warned = 0; warned <= 1; warned++) {
            observe(1, 2, 7, sizes[i], warned);
            observe(1, 3, 7, sizes[i], warned);
        }
    }
    return 0;
}

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

/* Execute the pinned native-delete body with no filesystem side effects.
 * Endpoint snapshots do not prove the absence of intermediate assignments.
 */
#define _GNU_SOURCE
#include <assert.h>
#include <errno.h>
#include <limits.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>
#include <chunkio/chunkio.h>
#include <chunkio/cio_file.h>
#include <chunkio/cio_log.h>

static int injected_result, calls, reports, arguments_match;
static const char *expected_path;

static int observe_unlink(const char *path)
{
    calls++;
    arguments_match = arguments_match && path == expected_path;
    if (injected_result == -1) { errno = EIO; }
    return injected_result;
}
static int observe_errno_print(int error, const char *file, int line)
{
    assert(error == EIO && file && line > 0);
    reports++;
    return 0;
}

#define unlink observe_unlink
#define cio_errno_print observe_errno_print
#include QCORE_FILE_CLEANUP_SOURCE
#undef unlink
#undef cio_errno_print

static unsigned case_id;
static char mapping[16];

static void observe(int fd, int mapped, int outcome)
{
    struct cio_file cf, before;
    char path[] = "controlled-delete-file";
    char path_before[sizeof(path)];
    int result, eligible, mode, record_preserved, path_preserved;
    memset(&cf, 0, sizeof(cf));
    cf.fd = fd;
    cf.map = mapped ? mapping : NULL;
    cf.alloc_size = sizeof(mapping);
    cf.path = path;
    cf.flags = 13;
    cf.data_size = 9;
    cf.page_size = 4096;
    cf.crc_cur = 71;
    cf.auto_remap_warned = 1;
    cf.map_truncated_warned = 1;
    memcpy(&before, &cf, sizeof(cf));
    memcpy(path_before, path, sizeof(path));
    expected_path = path;
    injected_result = outcome;
    calls = reports = 0;
    arguments_match = 1;
    eligible = fd == -1 && !mapped;
    mode = fd != -1 ? mapped : mapped ? 2 : outcome == -1 ? 3 : 4;
    result = cio_file_native_delete(&cf);
    record_preserved = memcmp(&before, &cf, sizeof(cf)) == 0;
    path_preserved = memcmp(path_before, path, sizeof(path)) == 0;
    assert(calls == eligible && arguments_match);
    assert(result == (eligible ? outcome : -1));
    assert(reports == (eligible && outcome == -1));
    assert(record_preserved && path_preserved);
    printf("{\"function\":\"native_delete_observation\",\"case\":%u,"
           "\"helper\":\"cio_file_native_delete\",\"mode\":%d,\"fd\":%d,\"values\":{"
           "\"exclusive_nonnull_live_record_and_terminated_path\":1,"
           "\"controlled_returning_unlink_hook\":1,"
           "\"diagnostic_hook_frames_record_and_path\":1,"
           "\"compiled_source_and_build_recorded\":1,"
           "\"fd_active\":%d,\"mapping_active\":%d,\"eligible\":%d,"
           "\"injected_result\":%d,\"returned\":%d,\"unlink_calls\":%d,"
           "\"diagnostic_calls\":%d,\"arguments_match\":%d,"
           "\"record_preserved\":%d,\"path_preserved\":%d}}\n",
           case_id++, mode, fd, fd != -1, mapped, eligible, outcome, result,
           calls, reports, arguments_match, record_preserved, path_preserved);
}

int main(void)
{
    int descriptors[] = {-1, 0, 7, INT_MAX};
    for (size_t i = 0; i < sizeof(descriptors) / sizeof(descriptors[0]); i++) {
        for (int mapped = 0; mapped <= 1; mapped++) {
            observe(descriptors[i], mapped, -1);
            observe(descriptors[i], mapped, 0);
        }
    }
    return 0;
}

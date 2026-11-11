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

/* One ENOMEM at each constructor allocation, in a fresh process/fixture.
 * Cases 3 and 9 deliberately continue through native unchecked null uses;
 * their sanitizer termination is recorded separately by the driver. */
#define QCORE_FILE_OBSERVER_NO_MAIN
#include "chunk_file_teardown_observation.c"

static unsigned attempt, fail_at;
static int inject(const char *allocator, size_t size) {
    attempt++;
    int failed = attempt == fail_at;
    prefix("attempt");
    printf(",\"ordinal\":%u,\"allocator\":\"%s\",\"size\":%zu,\"failed\":%d}\n",
           attempt, allocator, size, failed);
    if (failed) errno = ENOMEM;
    return failed;
}

int main(int argc, char **argv) {
    assert(argc == 3);
    fail_at = (unsigned) strtoul(argv[1], NULL, 10);
    assert(fail_at <= 11);
    /* The driver owns a fresh root with the same length as the prior profile. */
    const char *root = argv[2];
    assert(strlen(root) == 27);
    case_id = 100 + fail_at;
    setvbuf(stdout, NULL, _IONBF, 0);
    prefix("begin_file_failure");
    printf(",\"schema_version\":1,\"fail_at\":%u,\"root_length\":%zu}\n", fail_at, strlen(root));
    tracking = 1;
    backend_snapshot_hook = file_snapshot;
    before_free_hook = file_before_free;
    extra_function_name = file_function;
    allocation_failure_hook = inject;
    struct cio_options options;
    cio_options_init(&options);
    options.root_path = root;
    options.chmod = "0700";
    context = cio_create(&options);
    prefix("context_result");
    printf(",\"context\":%u,\"chmod\":%u,\"root\":%u}\n", identity(context),
           context ? identity(context->options.chmod) : 0,
           context ? identity(context->options.root_path) : 0);
    struct cio_stream *st = NULL;
    struct cio_chunk *ch = NULL;
    int error = -999;
    if (context) {
        st = cio_stream_create(context, "stream-0", CIO_STORE_FS);
        prefix("stream_result");
        printf(",\"stream\":%u,\"name\":%u}\n", identity(st), st ? identity(st->name) : 0);
        if (st) {
            streams[stream_count++] = st;
            ch = cio_chunk_open(context, st, "chunk-0", CIO_OPEN_RW, 64, &error);
            if (ch) chunks[chunk_count++] = ch;
        }
    }
    allocation_failure_hook = NULL;
    prefix("constructor_result");
    printf(",\"context\":%u,\"stream\":%u,\"chunk\":%u,\"error\":%d,\"attempts\":%u}\n",
           identity(context), identity(st), identity(ch), error, attempt);
    if (context) snapshot();
    destroying = 1;
    cio_destroy(context);
    destroying = 0;
    if (ch) {
        unsigned char vector;
        struct stat remaining;
        errno = 0; int fd_result = fcntl(saved_fd, F_GETFD); int fd_error = errno;
        errno = 0; int map_result = mincore(saved_map, saved_size, &vector); int map_error = errno;
        errno = 0; int path_result = lstat(saved_path, &remaining); int path_error = errno;
        prefix("os_terminal");
        printf(",\"fd\":%d,\"map\":%llu,\"size\":%zu,\"fd_result\":%d,\"fd_errno\":%d,\"map_result\":%d,\"map_errno\":%d,\"path_result\":%d,\"path_errno\":%d,\"device\":%llu,\"inode\":%llu}\n",
               saved_fd, (unsigned long long)(uintptr_t)saved_map, saved_size,
               fd_result, fd_error, map_result, map_error, path_result, path_error,
               path_result == 0 ? (unsigned long long)remaining.st_dev : 0,
               path_result == 0 ? (unsigned long long)remaining.st_ino : 0);
    }
    unsigned live = 0;
    for (unsigned i = 0; i < used; i++) live += allocations[i].live;
    prefix("end"); printf(",\"live_allocations\":%u}\n", live);
    assert(live == 0);
    tracking = 0;
    return 0;
}

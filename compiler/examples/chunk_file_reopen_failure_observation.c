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

/* Research-only reopen size-lookup/mapping suppression on an exclusive fixture.
 * Pinned ChunkIO source bodies are compiled unchanged; only selected libc
 * entry points are redirected. Suppressed calls never reach the kernel.
 *
 * The observer's own fstat calls are routed straight to the real symbol so a
 * suppression window can only be consumed by the library under observation.
 */
#define _GNU_SOURCE
#include <sys/stat.h>
static int observer_fstat(int, struct stat *);
#define fstat observer_fstat
#define __real_mmap profile_mmap
#include "native_file_profile_observer.h"
#undef fstat
#undef __real_mmap

int __real_fstat(int, struct stat *);
int __real_stat(const char *, struct stat *);
void *__real_mmap(void *, size_t, int, int, int, off_t);
static int observer_fstat(int fd, struct stat *st) { return __real_fstat(fd, st); }
static unsigned suppress_fstat, suppress_stat, suppress_map;

static void size_lookup(const char *call, int injected, int result, int error, off_t size) {
    os_prefix("size_lookup");
    printf(",\"call\":\"%s\",\"epoch\":%u,\"result\":%d,\"errno\":%d,\"injected\":%d,\"size\":%lld}\n",
           call, fd_epoch, result, error, injected, result == 0 ? (long long)size : -1);
}

int __wrap_fstat(int fd, struct stat *st) {
    if (!tracking || fd != opened_fd) return __real_fstat(fd, st);
    int injected = suppress_fstat != 0;
    if (injected) suppress_fstat--;
    errno = 0;int result = injected ? (errno = EIO, -1) : __real_fstat(fd, st);int error = errno;
    size_lookup("fstat", injected, result, error, result == 0 ? st->st_size : 0);
    errno = error;return result;
}

int __wrap_stat(const char *path, struct stat *st) {
    if (!tracking || strcmp(path, file_path) != 0) return __real_stat(path, st);
    int injected = suppress_stat != 0;
    if (injected) suppress_stat--;
    errno = 0;int result = injected ? (errno = EIO, -1) : __real_stat(path, st);int error = errno;
    size_lookup("stat", injected, result, error, result == 0 ? st->st_size : 0);
    errno = error;return result;
}

void *profile_mmap(void *address, size_t size, int prot, int flags, int fd, off_t offset) {
    if (tracking && suppress_map) {
        assert(fd == opened_fd);
        suppress_map--;
        os_prefix("suppressed_map");
        printf(",\"fd\":%d,\"epoch\":%u,\"size\":%zu,\"remaining\":%u}\n", fd, fd_epoch, size, suppress_map);
        errno = EIO;return MAP_FAILED;
    }
    return __real_mmap(address, size, prot, flags, fd, offset);
}

static void operation(const char *name, int result) {
    prefix("operation");printf(",\"name\":\"%s\",\"result\":%d}\n", name, result);
    fixture_probe(name);snapshot();
}

int main(int argc, char **argv) {
    assert(argc == 3);
    unsigned scenario = (unsigned)strtoul(argv[1], NULL, 10);
    assert(scenario < 5);
    profile_init(argv[2], 800 + scenario, "begin_reopen_failure");
    int error = -999;
    struct cio_chunk *ch = cio_chunk_open(context, streams[0], "chunk-0", CIO_OPEN_RW, 64, &error);
    assert(ch);chunks[chunk_count++] = ch;
    prefix("constructor_result");printf(",\"chunk\":%u,\"error\":%d}\n", identity(ch), error);
    fixture_probe("returned");snapshot();phase = "transition";
    operation("down", cio_chunk_down(ch));
    if (scenario < 3) {
        suppress_fstat = 1;suppress_stat = scenario != 0;
        prefix("arm_reopen_failure");
        printf(",\"fstat_count\":%u,\"stat_count\":%u,\"map_count\":0}\n", suppress_fstat, suppress_stat);
        if (scenario == 0) operation("up_stat_fallback", cio_chunk_up(ch));
        else {
            operation("up_failed_size_lookup", cio_chunk_up(ch));
            operation("down_retry_unmapped", cio_chunk_down(ch));
            operation("up_retry_descriptor_present", cio_chunk_up(ch));
        }
    }
    else {
        suppress_map = 1;
        prefix("arm_reopen_failure");printf(",\"fstat_count\":0,\"stat_count\":0,\"map_count\":1}\n");
        operation("up_failed_map", cio_chunk_up(ch));
        if (scenario == 3) operation("up_retry_already_mapped", cio_chunk_up(ch));
        else operation("sync_after_failed_map", cio_chunk_sync(ch));
        operation("down_retry_invalid_map", cio_chunk_down(ch));
    }
    fail_close = (scenario == 2 || scenario == 4);
    profile_destroy();
    fail_close = 0;
    assert(suppress_fstat == 0 && suppress_stat == 0 && suppress_map == 0);
    /* Only the two suppressed-close fixtures are reclaimed by the harness. This
     * happens after the parent-release probe and never counts as library cleanup. */
    if (scenario == 2 || scenario == 4) {
        errno = 0;int result = __real_close(opened_fd);int err = errno;
        prefix("harness_close");
        printf(",\"fd\":%d,\"epoch\":%u,\"result\":%d,\"errno\":%d}\n", opened_fd, fd_epoch, result, err);
        assert(result == 0);fixture_probe("harness_reclaimed");
    }
    prefix("complete");
    printf(",\"pending_fstat\":%u,\"pending_stat\":%u,\"pending_map\":%u}\n",
           suppress_fstat, suppress_stat, suppress_map);
    return 0;
}

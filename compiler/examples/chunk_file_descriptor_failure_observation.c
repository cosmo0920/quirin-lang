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

/* Research-only syscall suppression; pinned source bodies remain unchanged. */
#define __real_close profile_close
#define __real_open profile_open
#include "native_file_profile_observer.h"
#undef __real_close
#undef __real_open

int __real_close(int);
int __real_open(const char *, int, ...);
static unsigned suppress_close, suppress_open;

int profile_close(int fd) {
    if (tracking && suppress_close) {
        assert(fd == opened_fd);
        suppress_close--;
        os_prefix("suppressed_close");
        printf(",\"epoch\":%u,\"remaining\":%u}\n", fd_epoch, suppress_close);
        errno = EIO;return -1;
    }
    return __real_close(fd);
}

int profile_open(const char *path, int flags, ...) {
    mode_t mode = 0;
    if (flags & O_CREAT) {
        va_list args;va_start(args, flags);mode = (mode_t)va_arg(args, int);va_end(args);
    }
    if (tracking && suppress_open) {
        assert(strcmp(path, file_path) == 0);
        suppress_open--;
        os_prefix("suppressed_open");
        printf(",\"epoch\":%u,\"remaining\":%u}\n", fd_epoch, suppress_open);
        errno = EIO;return -1;
    }
    return __real_open(path, flags, mode);
}

static void operation(const char *name, int result) {
    prefix("operation");printf(",\"name\":\"%s\",\"result\":%d}\n", name, result);
    fixture_probe(name);snapshot();
}

int main(int argc, char **argv) {
    assert(argc == 3);
    unsigned scenario = (unsigned)strtoul(argv[1], NULL, 10);
    assert(scenario < 4);
    profile_init(argv[2], 700 + scenario, "begin_descriptor_failure");
    int error = -999;
    struct cio_chunk *ch = cio_chunk_open(context, streams[0], "chunk-0", CIO_OPEN_RW, 64, &error);
    assert(ch);chunks[chunk_count++] = ch;
    prefix("constructor_result");printf(",\"chunk\":%u,\"error\":%d}\n", identity(ch), error);
    fixture_probe("returned");snapshot();phase = "transition";
    if (scenario < 2) {
        suppress_close = scenario + 1;
        prefix("arm_descriptor_failure");printf(",\"close_count\":%u,\"open_count\":0}\n", suppress_close);
        operation("down_failed_close", cio_chunk_down(ch));
        operation("down_retry_unmapped", cio_chunk_down(ch));
        operation("up_descriptor_present", cio_chunk_up(ch));
    }
    else {
        operation("down", cio_chunk_down(ch));
        suppress_open = 1;
        prefix("arm_descriptor_failure");printf(",\"close_count\":0,\"open_count\":1}\n");
        operation("up_failed_open", cio_chunk_up(ch));
        if (scenario == 2) operation("up_retry", cio_chunk_up(ch));
    }
    profile_destroy();
    assert(suppress_close == 0 && suppress_open == 0);
    /* Only the exclusive suppressed-close fixture is reclaimed by the harness.
     * This is after the parent-release probe, never counted as library cleanup. */
    if (scenario == 1) {
        errno = 0;int result = __real_close(opened_fd);int error = errno;
        prefix("harness_close");printf(",\"fd\":%d,\"epoch\":%u,\"result\":%d,\"errno\":%d}\n", opened_fd, fd_epoch, result, error);
        assert(result == 0);fixture_probe("harness_reclaimed");
    }
    prefix("complete");printf(",\"pending_close\":%u,\"pending_open\":%u}\n", suppress_close, suppress_open);
    return 0;
}

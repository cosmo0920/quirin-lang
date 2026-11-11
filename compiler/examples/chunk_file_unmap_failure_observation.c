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

/* Research-only one-shot suppression; pinned source bodies are unchanged. */
#define __real_munmap profile_munmap
#define __real_msync profile_msync
#include "native_file_profile_observer.h"
#undef __real_munmap
#undef __real_msync

int __real_munmap(void *, size_t);
int __real_msync(void *, size_t, int);
static unsigned suppress_unmap, suppress_sync;

int profile_munmap(void *map, size_t size) {
    if (tracking && suppress_unmap) {
        assert(map_live && map == last_map && size == last_map_size);
        suppress_unmap--;
        os_prefix("suppressed_unmap");
        printf(",\"epoch\":%u,\"remaining\":%u}\n", map_epoch, suppress_unmap);
        errno = EIO;return -1;
    }
    return __real_munmap(map, size);
}

int profile_msync(void *map, size_t size, int flags) {
    if (tracking && suppress_sync) {
        assert(map_live && map == last_map && size == last_map_size);
        suppress_sync--;
        os_prefix("suppressed_sync");
        printf(",\"epoch\":%u,\"remaining\":%u}\n", map_epoch, suppress_sync);
        errno = EIO;return -1;
    }
    return __real_msync(map, size, flags);
}

static void operation(const char *name, int result) {
    prefix("operation");printf(",\"name\":\"%s\",\"result\":%d}\n",name,result);
    fixture_probe(name);snapshot();
}

int main(int argc, char **argv) {
    assert(argc == 3);
    unsigned scenario = (unsigned)strtoul(argv[1], NULL, 10);
    assert(scenario < 4);
    profile_init(argv[2], 600 + scenario, "begin_unmap_failure");
    int error = -999;
    struct cio_chunk *ch = cio_chunk_open(context, streams[0], "chunk-0", CIO_OPEN_RW, 64, &error);
    assert(ch);chunks[chunk_count++] = ch;
    prefix("constructor_result");printf(",\"chunk\":%u,\"error\":%d}\n",identity(ch),error);
    fixture_probe("returned");snapshot();phase = "transition";
    if (scenario != 0) operation("append", cio_chunk_write(ch, "add", 3));
    suppress_unmap = 1;suppress_sync = scenario >= 2;
    prefix("arm_unmap_failure");printf(",\"unmap_count\":1,\"sync_count\":%u}\n",suppress_sync);
    operation("down_failed", cio_chunk_down(ch));
    operation("up_still_mapped", cio_chunk_up(ch));
    if (scenario != 3) {
        operation("down_retry", cio_chunk_down(ch));
        operation("up", cio_chunk_up(ch));
    }
    assert(suppress_unmap == 0 && suppress_sync == 0);
    profile_destroy();return 0;
}

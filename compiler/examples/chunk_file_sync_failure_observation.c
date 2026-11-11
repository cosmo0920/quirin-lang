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

/* Research-only one-shot suppression; the pinned ChunkIO sources are unchanged. */
#define __real_msync profile_msync
#include "native_file_profile_observer.h"
#undef __real_msync

int __real_msync(void *, size_t, int);
static unsigned suppress_sync;

int profile_msync(void *map, size_t size, int flags) {
    if (tracking && suppress_sync) {
        assert(map_live && map == last_map && size == last_map_size);
        suppress_sync--;
        os_prefix("suppressed_sync");
        printf(",\"epoch\":%u,\"remaining\":%u}\n", map_epoch, suppress_sync);
        errno = EIO;
        return -1;
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
    assert(scenario < 3);
    profile_init(argv[2], 500 + scenario, "begin_sync_failure");
    int error = -999;
    struct cio_chunk *ch = cio_chunk_open(context, streams[0], "chunk-0", CIO_OPEN_RW, 64, &error);
    assert(ch);chunks[chunk_count++] = ch;
    prefix("constructor_result");printf(",\"chunk\":%u,\"error\":%d}\n",identity(ch),error);
    fixture_probe("returned");snapshot();phase = "transition";
    operation("begin", cio_chunk_tx_begin(ch));
    operation("append", cio_chunk_write(ch, "add", 3));
    suppress_sync = 1;
    prefix("arm_sync_failure");printf(",\"count\":1}\n");
    if (scenario == 2) {
        operation("down_sync_failed", cio_chunk_down(ch));
        operation("commit_down", cio_chunk_tx_commit(ch));
        operation("up", cio_chunk_up(ch));
    }
    else {
        operation("commit_failed", cio_chunk_tx_commit(ch));
        if (scenario == 0) {
            operation("commit_retry", cio_chunk_tx_commit(ch));
            operation("commit_again", cio_chunk_tx_commit(ch));
        }
        else {
            operation("rollback", cio_chunk_tx_rollback(ch));
            operation("commit_after_rollback", cio_chunk_tx_commit(ch));
        }
    }
    assert(suppress_sync == 0);
    profile_destroy();return 0;
}

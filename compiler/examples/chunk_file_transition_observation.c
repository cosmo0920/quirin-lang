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

#include "native_file_profile_observer.h"

static void operation(const char *name, int result) {
    prefix("operation");printf(",\"name\":\"%s\",\"result\":%d}\n",name,result);
    fixture_probe(name);snapshot();
}
int main(int argc, char **argv) {
    assert(argc==3);
    unsigned scenario=(unsigned)strtoul(argv[1],NULL,10);assert(scenario<5);
    profile_init(argv[2],400+scenario,"begin_transition");
    int error=-999;
    struct cio_chunk *ch=cio_chunk_open(context,streams[0],"chunk-0",CIO_OPEN_RW,64,&error);
    assert(ch);chunks[chunk_count++]=ch;
    prefix("constructor_result");printf(",\"chunk\":%u,\"error\":%d}\n",identity(ch),error);
    fixture_probe("returned");snapshot();phase="transition";
    if(scenario==0) {
        operation("down",cio_chunk_down(ch));
        operation("down_again",cio_chunk_down(ch));
        operation("up",cio_chunk_up(ch));
        operation("up_again",cio_chunk_up(ch));
    }
    else if(scenario==3) {
        operation("lock",cio_chunk_lock(ch));
        operation("lock_again",cio_chunk_lock(ch));
        operation("begin_locked",cio_chunk_tx_begin(ch));
        operation("write_locked",cio_chunk_write(ch,"add",3));
        operation("unlock",cio_chunk_unlock(ch));
        operation("unlock_again",cio_chunk_unlock(ch));
    }
    else {
        operation("begin",cio_chunk_tx_begin(ch));
        operation("begin_again",cio_chunk_tx_begin(ch));
        operation("append",cio_chunk_write(ch,"add",3));
        if(scenario==2) operation("commit",cio_chunk_tx_commit(ch));
        else {
            operation("rollback",cio_chunk_tx_rollback(ch));
            operation("rollback_again",cio_chunk_tx_rollback(ch));
        }
        if(scenario==4) {
            operation("down",cio_chunk_down(ch));
            operation("up",cio_chunk_up(ch));
        }
    }
    profile_destroy();return 0;
}

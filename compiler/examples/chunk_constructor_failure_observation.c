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

/* Single injected allocation failure at each position of the same native
 * context/stream/chunk/metadata constructor path. Upstream bodies are unchanged.
 */
#define main positive_teardown_main
#include "chunk_teardown_observation.c"
#undef main
#include <errno.h>

static unsigned attempt, fail_at;
static int inject(const char *allocator,size_t size) {
    attempt++;
    int failed=attempt==fail_at;
    prefix("attempt");printf(",\"ordinal\":%u,\"allocator\":\"%s\",\"size\":%zu,\"failed\":%d}\n",attempt,allocator,size,failed);
    if(failed)errno=ENOMEM;
    return failed;
}
static void failure_case(unsigned failure) {
    used=0;sequence=0;case_id=failure;stream_count=0;chunk_count=0;
    attempt=0;fail_at=failure;context=NULL;tracking=1;
    prefix("begin_failure");printf(",\"schema_version\":1,\"fail_at\":%u,\"layout\":{\"context_size\":%zu,\"stream_size\":%zu,\"chunk_size\":%zu,\"backend_size\":%zu,\"context_ring\":%zu,\"stream_node\":%zu,\"stream_ring\":%zu,\"up_ring\":%zu,\"down_ring\":%zu,\"chunk_node\":%zu,\"state_node\":%zu}}\n",failure,sizeof(struct cio_ctx),sizeof(struct cio_stream),sizeof(struct cio_chunk),sizeof(struct cio_memfs),offsetof(struct cio_ctx,streams),offsetof(struct cio_stream,_head),offsetof(struct cio_stream,chunks),offsetof(struct cio_stream,chunks_up),offsetof(struct cio_stream,chunks_down),offsetof(struct cio_chunk,_head),offsetof(struct cio_chunk,_state_head));
    allocation_failure_hook=inject;
    struct cio_options options;cio_options_init(&options);options.chmod="0700";
    context=cio_create(&options);
    struct cio_stream *st=NULL;struct cio_chunk *ch=NULL;
    int err=-999,meta_result=-999;
    if(context) {
        st=cio_stream_create(context,"stream-0",CIO_STORE_MEM);
        if(st) {
            streams[stream_count++]=st;
            ch=cio_chunk_open(context,st,"chunk-0",CIO_OPEN_RW,64,&err);
            if(ch) { chunks[chunk_count++]=ch;meta_result=cio_meta_write(ch,"meta",4); }
        }
    }
    allocation_failure_hook=NULL;
    prefix("constructor_result");printf(",\"context\":%u,\"stream\":%u,\"chunk\":%u,\"error\":%d,\"metadata_result\":%d,\"attempts\":%u}\n",identity(context),identity(st),identity(ch),err,meta_result,attempt);
    if(context)snapshot();
    destroying=1;cio_destroy(context);destroying=0;
    unsigned live=0;for(unsigned i=0;i<used;i++)live+=allocations[i].live;
    prefix("end");printf(",\"live_allocations\":%u}\n",live);assert(live==0);tracking=0;
}
int main(void) {
    for(unsigned failure=0;failure<=9;failure++)failure_case(failure);
    return 0;
}

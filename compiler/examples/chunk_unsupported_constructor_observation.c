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

/* Real constructor outcomes for four unsupported stream types. */
#define main positive_teardown_main
#include "chunk_teardown_observation.c"
#undef main
#include <limits.h>

static void unsupported_case(unsigned number, int kind) {
    used=0;sequence=0;case_id=number;stream_count=0;chunk_count=0;
    context=NULL;tracking=1;
    prefix("begin_unsupported");
    printf(",\"schema_version\":1,\"backend_kind\":%d,\"memory_kind\":%d,\"filesystem_kind\":%d,\"layout\":{\"context_size\":%zu,\"stream_size\":%zu,\"chunk_size\":%zu,\"backend_size\":%zu,\"context_ring\":%zu,\"stream_node\":%zu,\"stream_ring\":%zu,\"up_ring\":%zu,\"down_ring\":%zu,\"chunk_node\":%zu,\"state_node\":%zu}}\n",kind,CIO_STORE_MEM,CIO_STORE_FS,sizeof(struct cio_ctx),sizeof(struct cio_stream),sizeof(struct cio_chunk),sizeof(struct cio_memfs),offsetof(struct cio_ctx,streams),offsetof(struct cio_stream,_head),offsetof(struct cio_stream,chunks),offsetof(struct cio_stream,chunks_up),offsetof(struct cio_stream,chunks_down),offsetof(struct cio_chunk,_head),offsetof(struct cio_chunk,_state_head));
    struct cio_options options;cio_options_init(&options);options.chmod="0700";
    context=cio_create(&options);assert(context);
    struct cio_stream *st=cio_stream_create(context,"stream-0",kind);assert(st);
    streams[stream_count++]=st;
    int err=-999;
    struct cio_chunk *ch=cio_chunk_open(context,st,"chunk-0",CIO_OPEN_RW,64,&err);
    prefix("constructor_result");
    printf(",\"context\":%u,\"stream\":%u,\"chunk\":%u,\"stream_kind\":%d,\"error\":%d}\n",identity(context),identity(st),identity(ch),st->type,err);
    assert(!ch); /* The shared snapshot interprets only empty unsupported streams. */
    snapshot();
    destroying=1;cio_destroy(context);destroying=0;
    unsigned live=0;for(unsigned i=0;i<used;i++)live+=allocations[i].live;
    prefix("end");printf(",\"live_allocations\":%u}\n",live);assert(live==0);tracking=0;
}
int main(void) {
    const int kinds[]={INT_MIN,-1,2,INT_MAX};
    _Static_assert(INT_MIN == -2147483647-1 && INT_MAX == 2147483647,"32-bit int profile");
    for(unsigned i=0;i<4;i++)unsupported_case(i,kinds[i]);
    return 0;
}

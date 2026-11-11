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

/* Native constructor/ring/teardown observer. No source bodies are rewritten.
 * Only ChunkIO translation units get -finstrument-functions. Allocator wrappers
 * preserve libc semantics; this harness performs single-threaded memory cases.
 */
#define _GNU_SOURCE
#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <chunkio/chunkio.h>
#include <chunkio/cio_chunk.h>
#include <chunkio/cio_stream.h>
#include <chunkio/cio_memfs.h>

_Static_assert(sizeof(void *) == 8 && sizeof(struct mk_list) == 16,
               "observer requires the recorded 64-bit Linux list ABI");

struct allocation { void *ptr; size_t size; unsigned id; int live; };
static struct allocation allocations[2048];
static unsigned used, sequence, case_id;
static int tracking, destroying;
static int (*allocation_failure_hook)(const char *, size_t);
static void (*backend_snapshot_hook)(struct cio_chunk *);
static void (*before_free_hook)(void *);
static const char *(*extra_function_name)(void *);
static struct cio_ctx *context;
static struct cio_stream *streams[16];
static struct cio_chunk *chunks[64];
static unsigned stream_count, chunk_count;

void *__real_malloc(size_t);
void *__real_calloc(size_t,size_t);
void *__real_realloc(void *,size_t);
char *__real_strdup(const char *);
void __real_free(void *);
static void prefix(const char *kind) {
    printf("{\"case\":%u,\"sequence\":%u,\"kind\":\"%s\"",case_id,sequence++,kind);
}
static unsigned identity(const void *ptr) {
    if (!ptr) return 0;
    for (unsigned i=used;i>0;i--) if (allocations[i-1].ptr==ptr && allocations[i-1].live) return i;
    fprintf(stderr,"unregistered live pointer %p\n",ptr);abort();
}
static void acquired(void *ptr,size_t size,const char *allocator) {
    if (!tracking || !ptr) return;
    assert(used<2048);
    allocations[used]=(struct allocation){ptr,size,used+1,1};used++;
    prefix("allocation");printf(",\"id\":%u,\"address\":%llu,\"size\":%zu,\"allocator\":\"%s\"}\n",used,(unsigned long long)(uintptr_t)ptr,size,allocator);
}
void *__wrap_malloc(size_t size) { if (allocation_failure_hook && allocation_failure_hook("malloc",size)) return NULL; void *p=__real_malloc(size);acquired(p,size,"malloc");return p; }
void *__wrap_calloc(size_t n,size_t size) { if (allocation_failure_hook && allocation_failure_hook("calloc",n*size)) return NULL; void *p=__real_calloc(n,size);assert(!n || size<=SIZE_MAX/n);acquired(p,n*size,"calloc");return p; }
char *__wrap_strdup(const char *s) { if (allocation_failure_hook && allocation_failure_hook("strdup",strlen(s)+1)) return NULL; char *p=__real_strdup(s);acquired(p,strlen(s)+1,"strdup");return p; }
void *__wrap_realloc(void *old,size_t size) {
    if (allocation_failure_hook && allocation_failure_hook("realloc",size)) return NULL;
    unsigned id=tracking ? identity(old):0;
    assert(size>0);
    void *p=__real_realloc(old,size);
    if (tracking && p) {
        if (id) { allocations[id-1].live=0;prefix("replace");printf(",\"id\":%u}\n",id); }
        acquired(p,size,"realloc");
    }
    return p;
}
static int empty(struct mk_list *head) { return head->next==head && head->prev==head; }
void __wrap_free(void *p) {
    if (tracking && p) {
        unsigned id=identity(p);
        if (before_free_hook) before_free_hook(p);
        if (destroying) {
            for (unsigned i=0;i<chunk_count;i++) if (chunks[i]==p) {
                struct cio_chunk *ch=p;
                prefix("chunk_state");printf(",\"id\":%u,\"primary_cleared\":%d,\"state_cleared\":%d,\"total\":%zu}\n",id,ch->_head.next==NULL && ch->_head.prev==NULL,ch->_state_head.next==NULL && ch->_state_head.prev==NULL,context->total_chunks);
                chunks[i]=NULL;
            }
            for (unsigned i=0;i<stream_count;i++) if (streams[i]==p) {
                struct cio_stream *st=p;
                prefix("stream_state");printf(",\"id\":%u,\"primary_cleared\":%d,\"chunks_empty\":%d,\"up_empty\":%d,\"down_empty\":%d}\n",id,st->_head.next==NULL && st->_head.prev==NULL,empty(&st->chunks),empty(&st->chunks_up),empty(&st->chunks_down));
                streams[i]=NULL;
            }
            if (context==p) {
                prefix("context_state");printf(",\"id\":%u,\"total\":%zu,\"up\":%zu,\"streams_empty\":%d}\n",id,context->total_chunks,context->total_chunks_up,empty(&context->streams));context=NULL;
            }
        }
        allocations[id-1].live=0;prefix("free");printf(",\"id\":%u}\n",id);
    }
    __real_free(p);
}
static const char *function_name(void *fn) {
#define NAME(f) if (fn==(void *)(uintptr_t)&f) return #f
    NAME(cio_destroy);NAME(cio_stream_destroy_all);NAME(cio_stream_destroy);
    NAME(cio_chunk_close_stream);NAME(cio_chunk_close);NAME(cio_memfs_close);
    NAME(cio_chunk_counter_total_sub);
#undef NAME
    return extra_function_name ? extra_function_name(fn) : NULL;
}
void __cyg_profile_func_enter(void *fn,void *caller) {
    (void)caller;
    if (destroying) { const char *name=function_name(fn);if (name) {prefix("enter");printf(",\"function\":\"%s\"}\n",name);} }
}
void __cyg_profile_func_exit(void *fn,void *caller) {
    (void)caller;
    if (destroying) {
        const char *name=function_name(fn);
        if (name) {prefix("exit");printf(",\"function\":\"%s\"}\n",name);}
        if (fn==(void *)(uintptr_t)&cio_chunk_counter_total_sub) {assert(context);prefix("counter");printf(",\"total\":%zu,\"up\":%zu}\n",context->total_chunks,context->total_chunks_up);}
    }
}
static void ring(struct mk_list *head,size_t offset) {
    printf("{\"sentinel\":%llu,\"next\":%llu,\"prev\":%llu,\"nodes\":[",(unsigned long long)(uintptr_t)head,(unsigned long long)(uintptr_t)head->next,(unsigned long long)(uintptr_t)head->prev);
    unsigned count=0;
    for (struct mk_list *n=head->next;n!=head;n=n->next) {
        assert(count<64);if(count++)printf(",");
        void *owner=(char*)n-offset;
        printf("{\"owner\":%u,\"address\":%llu,\"next\":%llu,\"prev\":%llu}",identity(owner),(unsigned long long)(uintptr_t)n,(unsigned long long)(uintptr_t)n->next,(unsigned long long)(uintptr_t)n->prev);
    }
    printf("]}");
}
static void snapshot(void) {
    prefix("snapshot");printf(",\"context\":{\"id\":%u,\"total\":%zu,\"up\":%zu,\"fields\":[%u,%u,%u,%u,%u,%u],\"ring\":",identity(context),context->total_chunks,context->total_chunks_up,identity(context->options.user),identity(context->options.group),identity(context->options.chmod),identity(context->processed_user),identity(context->processed_group),identity(context->options.root_path));
    ring(&context->streams,offsetof(struct cio_stream,_head));
    printf(",\"streams\":[");unsigned si=0;
    for (struct mk_list *n=context->streams.next;n!=&context->streams;n=n->next) {
        struct cio_stream *st=mk_list_entry(n,struct cio_stream,_head);
        if(si++)printf(",");
        printf("{\"id\":%u,\"name\":%u,\"context\":%u,\"memory\":%d,\"ring\":",identity(st),identity(st->name),identity(st->parent),st->type==CIO_STORE_MEM);
        ring(&st->chunks,offsetof(struct cio_chunk,_head));printf(",\"up_ring\":");ring(&st->chunks_up,offsetof(struct cio_chunk,_state_head));printf(",\"down_ring\":");ring(&st->chunks_down,offsetof(struct cio_chunk,_state_head));
        printf(",\"chunks\":[");unsigned ci=0;
        for(struct mk_list *h=st->chunks.next;h!=&st->chunks;h=h->next) {
            struct cio_chunk *ch=mk_list_entry(h,struct cio_chunk,_head);struct cio_memfs *mf=ch->backend;
            if(ci++)printf(",");
            if (backend_snapshot_hook) { backend_snapshot_hook(ch); continue; }
            printf("{\"id\":%u,\"name\":%u,\"context\":%u,\"stream\":%u,\"locked\":%d,\"transaction\":%d,\"backend\":%u,\"fields\":[%u,%u,%u]}",identity(ch),identity(ch->name),identity(ch->ctx),identity(ch->st),ch->lock,ch->tx_active,identity(mf),identity(mf->name),identity(mf->buf_data),identity(mf->meta_data));
        }
        printf("]}");
    }
    printf("]}}\n");
}
static void observe(unsigned number,unsigned ns,unsigned nc,int metadata) {
    case_id=number;used=0;stream_count=0;chunk_count=0;sequence=0;
    prefix("begin");printf(",\"schema_version\":1,\"streams\":%u,\"chunks_per_stream\":%u,\"metadata\":%d,\"layout\":{\"context_size\":%zu,\"stream_size\":%zu,\"chunk_size\":%zu,\"backend_size\":%zu,\"context_ring\":%zu,\"stream_node\":%zu,\"stream_ring\":%zu,\"up_ring\":%zu,\"down_ring\":%zu,\"chunk_node\":%zu,\"state_node\":%zu}}\n",ns,nc,metadata,sizeof(struct cio_ctx),sizeof(struct cio_stream),sizeof(struct cio_chunk),sizeof(struct cio_memfs),offsetof(struct cio_ctx,streams),offsetof(struct cio_stream,_head),offsetof(struct cio_stream,chunks),offsetof(struct cio_stream,chunks_up),offsetof(struct cio_stream,chunks_down),offsetof(struct cio_chunk,_head),offsetof(struct cio_chunk,_state_head));
    tracking=1;
    struct cio_options options;cio_options_init(&options);options.chmod="0700";
    context=cio_create(&options);assert(context);
    for(unsigned i=0;i<ns;i++) {
        char name[32];snprintf(name,sizeof(name),"stream-%u",i);
        struct cio_stream *st=cio_stream_create(context,name,CIO_STORE_MEM);assert(st);streams[stream_count++]=st;
        for(unsigned j=0;j<nc;j++) {
            int err=-999;snprintf(name,sizeof(name),"chunk-%u",j);
            struct cio_chunk *ch=cio_chunk_open(context,st,name,CIO_OPEN_RW,64,&err);assert(ch && err==CIO_OK);chunks[chunk_count++]=ch;
            if(metadata)assert(cio_meta_write(ch,"meta",4)==0);
        }
    }
    snapshot();destroying=1;cio_destroy(context);destroying=0;
    unsigned live=0;for(unsigned i=0;i<used;i++)live+=allocations[i].live;
    prefix("end");printf(",\"live_allocations\":%u}\n",live);assert(live==0);tracking=0;
}
int main(void) {
    observe(0,0,0,0);observe(1,2,0,0);observe(2,1,1,0);observe(3,2,3,0);observe(4,2,3,1);
    return 0;
}

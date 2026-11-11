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

/* Real filesystem constructor -> teardown, with unmodified source units.
 * Shared allocation generations/ring observer; exclusive Linux fixtures. */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <unistd.h>
#include <chunkio/chunkio.h>
#include <chunkio/cio_file.h>
#include <chunkio/cio_file_native.h>
#define main memory_observer_main
#include "chunk_teardown_observation.c"
#undef main

static struct cio_file *file_backend;
static unsigned backend_id, path_id;
static int saved_fd;
static void *saved_map;
static size_t saved_size;
static char saved_path[256];
static dev_t saved_device;
static ino_t saved_inode;
int __real_close(int);
int __real_munmap(void *,size_t);
int __real_unlink(const char *);
int __real_msync(void *,size_t,int);

int __wrap_close(int fd) {
    int injected=destroying && case_id==4;
    errno=0;int result=injected ? (errno=EIO,-1) : __real_close(fd);int error=errno;
    if (destroying) {prefix("os_close");printf(",\"fd\":%d,\"result\":%d,\"errno\":%d,\"injected\":%d}\n",fd,result,error,injected);}
    errno=error;return result;
}
int __wrap_munmap(void *map,size_t size) {
    int injected=destroying && case_id==3;
    errno=0;int result=injected ? (errno=EIO,-1) : __real_munmap(map,size);int error=errno;
    if (destroying) {prefix("os_unmap");printf(",\"map\":%llu,\"size\":%zu,\"result\":%d,\"errno\":%d,\"injected\":%d}\n",(unsigned long long)(uintptr_t)map,size,result,error,injected);}
    errno=error;return result;
}
int __wrap_unlink(const char *path) {
    unsigned id=tracking ? identity(path):0;
    errno=0;int result=__real_unlink(path);int error=errno;
    if (destroying) {prefix("os_delete");printf(",\"path\":%u,\"result\":%d,\"errno\":%d}\n",id,result,error);}
    errno=error;return result;
}
int __wrap_msync(void *map,size_t size,int flags) {
    int injected=destroying && case_id==5;
    errno=0;int result=injected ? (errno=EIO,-1) : __real_msync(map,size,flags);int error=errno;
    if (destroying) {prefix("os_sync");printf(",\"map\":%llu,\"size\":%zu,\"flags\":%d,\"result\":%d,\"errno\":%d,\"injected\":%d}\n",(unsigned long long)(uintptr_t)map,size,flags,result,error,injected);}
    errno=error;return result;
}
static const char *file_function(void *fn) {
#define NAME(f) if(fn==(void *)(uintptr_t)&f) return #f
    NAME(cio_file_close);NAME(cio_file_sync);NAME(cio_file_native_sync);
    NAME(cio_file_native_unmap);NAME(cio_file_native_close);NAME(cio_file_native_delete);
    NAME(cio_chunk_counter_total_up_sub);
#undef NAME
    return NULL;
}
static void file_snapshot(struct cio_chunk *ch) {
    struct cio_file *cf=ch->backend;
    struct stat st, path_st;unsigned char vector;
    assert(cf->alloc_size==(size_t)sysconf(_SC_PAGESIZE));
    assert(fstat(cf->fd,&st)==0 && mincore(cf->map,cf->alloc_size,&vector)==0);
    assert(lstat(cf->path,&path_st)==0 && st.st_dev==path_st.st_dev && st.st_ino==path_st.st_ino);
    file_backend=cf;backend_id=identity(cf);path_id=identity(cf->path);
    saved_fd=cf->fd;saved_map=cf->map;saved_size=cf->alloc_size;
    assert(strlen(cf->path)<sizeof(saved_path));strcpy(saved_path,cf->path);
    saved_device=st.st_dev;saved_inode=st.st_ino;
    printf("{\"id\":%u,\"name\":%u,\"context\":%u,\"stream\":%u,\"locked\":%d,\"transaction\":%d,"
           "\"backend\":%u,\"path\":%u,\"backend_context\":%u,\"fd\":%d,\"map\":%llu,\"size\":%zu,"
           "\"synced\":%d,\"data_size\":%zu,\"device\":%llu,\"inode\":%llu,\"links\":%llu,\"path_device\":%llu,\"path_inode\":%llu}",
           identity(ch),identity(ch->name),identity(ch->ctx),identity(ch->st),ch->lock,ch->tx_active,
           backend_id,path_id,identity(cf->ctx),cf->fd,(unsigned long long)(uintptr_t)cf->map,
           cf->alloc_size,cf->synced,cf->data_size,(unsigned long long)st.st_dev,(unsigned long long)st.st_ino,(unsigned long long)st.st_nlink,
           (unsigned long long)path_st.st_dev,(unsigned long long)path_st.st_ino);
}
static void file_before_free(void *ptr) {
    if (destroying && ptr==file_backend) {
        prefix("backend_state");printf(",\"id\":%u,\"fd\":%d,\"map\":%llu,\"size\":%zu,\"data_size\":%zu,\"up\":%zu,\"synced\":%d}\n",
                backend_id,file_backend->fd,(unsigned long long)(uintptr_t)file_backend->map,
                file_backend->alloc_size,file_backend->data_size,context->total_chunks_up,file_backend->synced);
        file_backend=NULL;
    }
}
static void observe_file(unsigned number) {
    char root[]="/tmp/qcore-file-tree-XXXXXX", stream_path[256];
    assert(mkdtemp(root));
    case_id=number;used=stream_count=chunk_count=sequence=0;
    prefix("begin_file");printf(",\"schema_version\":1,\"delete\":%u,\"write\":%u,\"full_sync\":%u,\"layout\":{\"context_size\":%zu,\"stream_size\":%zu,\"chunk_size\":%zu,\"backend_size\":%zu,\"context_ring\":%zu,\"stream_node\":%zu,\"stream_ring\":%zu,\"up_ring\":%zu,\"down_ring\":%zu,\"chunk_node\":%zu,\"state_node\":%zu}}\n",
        number==2,number!=0,number!=0,sizeof(struct cio_ctx),sizeof(struct cio_stream),sizeof(struct cio_chunk),sizeof(struct cio_file),offsetof(struct cio_ctx,streams),offsetof(struct cio_stream,_head),offsetof(struct cio_stream,chunks),offsetof(struct cio_stream,chunks_up),offsetof(struct cio_stream,chunks_down),offsetof(struct cio_chunk,_head),offsetof(struct cio_chunk,_state_head));
    tracking=1;
    struct cio_options options;cio_options_init(&options);options.root_path=root;options.chmod="0700";
    if(number!=0) options.flags|=CIO_FULL_SYNC;
    context=cio_create(&options);assert(context);
    struct cio_stream *st=cio_stream_create(context,"stream-0",CIO_STORE_FS);assert(st);streams[stream_count++]=st;
    int error=-999;
    struct cio_chunk *ch=cio_chunk_open(context,st,"chunk-0",CIO_OPEN_RW,64,&error);assert(ch && error==CIO_OK);chunks[chunk_count++]=ch;
    int write_result=number!=0 ? cio_chunk_write(ch,"payload",7) : -999;
    assert(number==0 || write_result==0);
    prefix("constructor_result");printf(",\"context\":%u,\"stream\":%u,\"chunk\":%u,\"error\":%d,\"write_result\":%d,\"full_sync\":%d}\n",
        identity(context),identity(st),identity(ch),error,write_result,(context->options.flags & CIO_FULL_SYNC)!=0);
    snapshot();destroying=1;
    if(number==2) cio_chunk_close(ch,CIO_TRUE);
    cio_destroy(context);destroying=0;
    unsigned char vector;struct stat remaining;
    errno=0;int fd_result=fcntl(saved_fd,F_GETFD);int fd_error=errno;
    errno=0;int map_result=mincore(saved_map,saved_size,&vector);int map_error=errno;
    errno=0;int path_result=lstat(saved_path,&remaining);int path_error=errno;
    prefix("os_terminal");printf(",\"fd\":%d,\"map\":%llu,\"size\":%zu,\"fd_result\":%d,\"fd_errno\":%d,\"map_result\":%d,\"map_errno\":%d,\"path_result\":%d,\"path_errno\":%d,\"device\":%llu,\"inode\":%llu}\n",
        saved_fd,(unsigned long long)(uintptr_t)saved_map,saved_size,fd_result,fd_error,map_result,map_error,path_result,path_error,
        path_result==0?(unsigned long long)remaining.st_dev:0,path_result==0?(unsigned long long)remaining.st_ino:0);
    assert(number==4 ? fd_result==0 && fd_error==0 : fd_result==-1 && fd_error==EBADF);
    assert(number==3 ? map_result==0 && map_error==0 : map_result==-1 && map_error==ENOMEM);
    assert(number==2 ? path_result==-1 && path_error==ENOENT : path_result==0 && remaining.st_dev==saved_device && remaining.st_ino==saved_inode);
    unsigned live=0;for(unsigned i=0;i<used;i++)live+=allocations[i].live;
    assert(live==0);tracking=0;
    if(number==3) assert(munmap(saved_map,saved_size)==0);
    if(number==4) assert(close(saved_fd)==0);
    if(number!=2) assert(unlink(saved_path)==0);
    snprintf(stream_path,sizeof(stream_path),"%s/stream-0",root);
    assert(rmdir(stream_path)==0 && rmdir(root)==0);
    prefix("end");printf(",\"live_allocations\":%u,\"fixture_cleanup_complete\":1}\n",live);
}
#ifndef QCORE_FILE_OBSERVER_NO_MAIN
int main(void) {
    backend_snapshot_hook=file_snapshot;before_free_hook=file_before_free;extra_function_name=file_function;
    for(unsigned i=0;i<6;i++) observe_file(i);
    return 0;
}
#endif

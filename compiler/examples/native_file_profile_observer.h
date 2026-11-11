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

/* Shared bounded Linux observation support. All fixtures are harness-owned.
 * Allocation generation tracking and reciprocal-ring snapshots come from the
 * existing memory observer; source bodies are compiled without rewriting. */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <stdarg.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <unistd.h>
#include <chunkio/chunkio.h>
#include <chunkio/cio_file.h>
#include <chunkio/cio_file_st.h>
#include <chunkio/cio_file_native.h>
#include <chunkio/cio_error.h>
#define main memory_observer_main
#include "chunk_teardown_observation.c"
#undef main

static unsigned profile_case, fd_epoch, map_epoch;
/* Endpoint call tallies, kept separate per operation and per outcome so a
 * library release is never counted as a harness reclamation. Existing profiles
 * ignore them; they default to zero and change no observed behaviour. */
static unsigned os_close_ok, os_close_failed, os_unmap_ok, os_unmap_failed, os_map_ok, os_map_failed;
static int profile_flags, fail_mmap;
static int opened_fd=-1, map_live, fail_chmod, fail_close;
static void *last_map;
static size_t last_map_size;
static dev_t file_device;
static ino_t file_inode;
static char file_path[256];
static const char *phase="constructor";
int __real_open(const char *,int,...);
int __real_close(int);
int __real_chmod(const char *,mode_t);
void *__real_mmap(void *,size_t,int,int,int,off_t);
int __real_munmap(void *,size_t);
int __real_msync(void *,size_t,int);
static void os_prefix(const char *kind) { prefix(kind);printf(",\"phase\":\"%s\"",phase); }
int __wrap_open(const char *path,int flags,...) {
    mode_t mode=0;
    if(flags&O_CREAT) {va_list args;va_start(args,flags);mode=(mode_t)va_arg(args,int);va_end(args);}
    if(tracking) assert(strcmp(path,file_path)==0);
    errno=0;int result=__real_open(path,flags,mode);int error=errno;
    if(tracking) {
        if(result>=0) {
            struct stat st;assert(fstat(result,&st)==0 && st.st_dev==file_device && st.st_ino==file_inode);
            opened_fd=result;fd_epoch++;
        }
        os_prefix("open");printf(",\"path\":%u,\"flags\":%d,\"mode\":%u,\"result\":%d,\"errno\":%d,\"epoch\":%u}\n",
            identity(path),flags,(unsigned)mode,result,error,fd_epoch);
    }
    errno=error;return result;
}
int __wrap_chmod(const char *path,mode_t mode) {
    int injected=tracking && fail_chmod;
    errno=0;int result=injected?(errno=EPERM,-1):__real_chmod(path,mode);int error=errno;
    if(tracking) {os_prefix("chmod");printf(",\"path\":%u,\"mode\":%u,\"result\":%d,\"errno\":%d,\"injected\":%d}\n",identity(path),(unsigned)mode,result,error,injected);}
    errno=error;return result;
}
int __wrap_close(int fd) {
    int injected=tracking && fail_close;
    errno=0;int result=injected?(errno=EIO,-1):__real_close(fd);int error=errno;
    if(tracking) {
        if(result==0)os_close_ok++;else os_close_failed++;
        os_prefix("close");printf(",\"fd\":%d,\"epoch\":%u,\"result\":%d,\"errno\":%d,\"injected\":%d}\n",fd,fd_epoch,result,error,injected);}
    errno=error;return result;
}
void *__wrap_mmap(void *address,size_t size,int prot,int flags,int fd,off_t offset) {
    int injected=tracking && fail_mmap;
    errno=0;void *result=injected?(errno=EIO,MAP_FAILED):__real_mmap(address,size,prot,flags,fd,offset);int error=errno;
    if(tracking) {
        if(result!=MAP_FAILED) {assert(!map_live);map_epoch++;map_live=1;last_map=result;last_map_size=size;os_map_ok++;}
        else os_map_failed++;
        os_prefix("map");printf(",\"address\":%llu,\"size\":%zu,\"prot\":%d,\"flags\":%d,\"fd\":%d,\"fd_epoch\":%u,\"offset\":%lld,\"result\":%llu,\"errno\":%d,\"epoch\":%u}\n",
            (unsigned long long)(uintptr_t)address,size,prot,flags,fd,fd_epoch,(long long)offset,(unsigned long long)(uintptr_t)result,error,map_epoch);
    }
    errno=error;return result;
}
int __wrap_munmap(void *map,size_t size) {
    /* A library that stored a failed mmap result may hand back MAP_FAILED; that
     * argument is recorded and forwarded, never treated as a tracked mapping. */
    if(tracking && map!=MAP_FAILED) assert(map_live && map==last_map && size==last_map_size);
    errno=0;int result=__real_munmap(map,size);int error=errno;
    if(tracking) {
        if(result==0){map_live=0;os_unmap_ok++;}else os_unmap_failed++;
        os_prefix("unmap");printf(",\"map\":%llu,\"size\":%zu,\"epoch\":%u,\"result\":%d,\"errno\":%d}\n",(unsigned long long)(uintptr_t)map,size,map_epoch,result,error);
    }
    errno=error;return result;
}
int __wrap_msync(void *map,size_t size,int flags) {
    errno=0;int result=__real_msync(map,size,flags);int error=errno;
    if(tracking) {os_prefix("sync");printf(",\"map\":%llu,\"size\":%zu,\"epoch\":%u,\"flags\":%d,\"result\":%d,\"errno\":%d}\n",(unsigned long long)(uintptr_t)map,size,map_epoch,flags,result,error);}
    errno=error;return result;
}
static const char *profile_function(void *fn) {
#define NAME(f) if(fn==(void *)(uintptr_t)&f)return #f
    NAME(cio_file_close);NAME(cio_file_sync);NAME(cio_file_native_sync);
    NAME(cio_file_native_unmap);NAME(cio_file_native_close);NAME(cio_chunk_counter_total_up_sub);
#undef NAME
    return NULL;
}
static void hex_bytes(const unsigned char *data,size_t size) {for(size_t i=0;i<size;i++)printf("%02x",data[i]);}
static void profile_backend(struct cio_chunk *ch) {
    struct cio_file *cf=ch->backend;
    int mapped=cf->map!=NULL && cf->map!=MAP_FAILED;
    assert(!mapped || (map_live && cf->map==last_map && cf->alloc_size==last_map_size && cf->data_size<=16));
    printf("{\"id\":%u,\"name\":%u,\"context\":%u,\"stream\":%u,\"locked\":%d,\"transaction\":%d,\"tx_length\":%zu,\"error\":%d,\"backend\":%u,\"path\":%u,\"fd\":%d,\"fd_epoch\":%u,\"map\":%llu,\"map_epoch\":%u,\"size\":%zu,\"fs_size\":%zu,\"data_size\":%zu,\"synced\":%d,\"header_length\":%u,\"content\":\"",
        identity(ch),identity(ch->name),identity(ch->ctx),identity(ch->st),ch->lock,ch->tx_active,ch->tx_content_length,cio_error_get(ch),identity(cf),identity(cf->path),cf->fd,fd_epoch,
        (unsigned long long)(uintptr_t)cf->map,map_epoch,cf->alloc_size,cf->fs_size,cf->data_size,cf->synced,
        mapped?((unsigned)(unsigned char)cf->map[10]<<24)|((unsigned)(unsigned char)cf->map[11]<<16)|((unsigned)(unsigned char)cf->map[12]<<8)|(unsigned char)cf->map[13]:0);
    if(mapped)hex_bytes((unsigned char *)cio_file_st_get_content(cf->map),cf->data_size);
    printf("\"}");
}
static void profile_before_free(void *ptr) {
    unsigned id=identity(ptr);
    if(id==10) {
        struct cio_file *cf=ptr;
        prefix("backend_release");printf(",\"id\":%u,\"fd\":%d,\"map\":%llu,\"size\":%zu,\"fs_size\":%zu,\"data_size\":%zu,\"up\":%zu}\n",id,cf->fd,(unsigned long long)(uintptr_t)cf->map,cf->alloc_size,cf->fs_size,cf->data_size,context->total_chunks_up);
    }
    if(id==7 && !destroying) {prefix("rollback_chunk");printf(",\"id\":7,\"error\":%d}\n",cio_error_get(ptr));}
}
static void fixture_probe(const char *label) {
    struct stat st,fd_st;unsigned char vec;
    errno=0;int fd_result=fcntl(opened_fd,F_GETFD);int fd_error=errno;
    assert(lstat(file_path,&st)==0 && st.st_dev==file_device && st.st_ino==file_inode);
    int valid=fd_result>=0 && fstat(opened_fd,&fd_st)==0;
    errno=0;int map_result=map_epoch?mincore(last_map,last_map_size,&vec):-999;int map_error=errno;
    prefix("probe");printf(",\"label\":\"%s\",\"fd\":%d,\"fd_epoch\":%u,\"fd_result\":%d,\"fd_errno\":%d,\"fd_device\":%llu,\"fd_inode\":%llu,\"map\":%llu,\"map_epoch\":%u,\"map_result\":%d,\"map_errno\":%d,\"size\":%lld,\"mode\":%u,\"device\":%llu,\"inode\":%llu}\n",
        label,opened_fd,fd_epoch,fd_result,fd_error,valid?(unsigned long long)fd_st.st_dev:0,valid?(unsigned long long)fd_st.st_ino:0,(unsigned long long)(uintptr_t)last_map,map_epoch,map_result,map_error,(long long)st.st_size,(unsigned)(st.st_mode&0777),(unsigned long long)st.st_dev,(unsigned long long)st.st_ino);
}
static void profile_init(char *root,unsigned number,const char *kind) {
    assert(strlen(root)==27);profile_case=number;case_id=number;
    setvbuf(stdout,NULL,_IONBF,0);snprintf(file_path,sizeof(file_path),"%s/stream-0/chunk-0",root);
    struct stat st;assert(lstat(file_path,&st)==0);file_device=st.st_dev;file_inode=st.st_ino;
    prefix(kind);printf(",\"schema_version\":1,\"root_length\":27,\"device\":%llu,\"inode\":%llu}\n",(unsigned long long)file_device,(unsigned long long)file_inode);
    backend_snapshot_hook=profile_backend;before_free_hook=profile_before_free;extra_function_name=profile_function;
    tracking=1;
    struct cio_options options;cio_options_init(&options);options.root_path=root;options.chmod="0700";
    options.flags|=profile_flags;
    context=cio_create(&options);assert(context);
    struct cio_stream *stp=cio_stream_create(context,"stream-0",CIO_STORE_FS);assert(stp);streams[stream_count++]=stp;
}
static void profile_destroy(void) {
    phase="teardown";destroying=1;cio_destroy(context);destroying=0;
    fixture_probe("parents_released");
    unsigned live=0;for(unsigned i=0;i<used;i++)live+=allocations[i].live;
    prefix("end");printf(",\"live_allocations\":%u}\n",live);assert(live==0);tracking=0;
}

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

/* Execute pinned Unix cleanup bodies against exclusively owned temporary OS
 * resources. Wrappers forward calls unchanged and preserve their errno. This
 * is Linux execution evidence, not a native-source or durability proof. */
#define _GNU_SOURCE
#include <assert.h>
#include <errno.h>
#include <fcntl.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <chunkio/chunkio.h>
#include <chunkio/cio_file.h>
#include <chunkio/cio_log.h>

static int calls, reports, call_result, call_errno, reported_errno;
static int argument_fd;
static uintptr_t argument_map, argument_path;
static size_t argument_length;
static const char *operation;

static int forward_close(int fd) {
    calls++; operation="close"; argument_fd=fd;
    errno=0; call_result=close(fd); call_errno=errno;
    return call_result;
}
static int forward_unmap(void *map, size_t size) {
    calls++; operation="munmap"; argument_map=(uintptr_t)map; argument_length=size;
    errno=0; call_result=munmap(map,size); call_errno=errno;
    return call_result;
}
static int forward_unlink(const char *path) {
    calls++; operation="unlink"; argument_path=(uintptr_t)path;
    errno=0; call_result=unlink(path); call_errno=errno;
    return call_result;
}
static int observe_error(int error, const char *file, int line) {
    assert(file && line>0); reports++; reported_errno=error; return 0;
}
#define close forward_close
#define munmap forward_unmap
#define unlink forward_unlink
#define cio_errno_print observe_error
#include QCORE_FILE_CLEANUP_SOURCE
#undef close
#undef munmap
#undef unlink
#undef cio_errno_print

static void fields(const struct cio_file *cf) {
    printf("{\"fd\":%d,\"map\":%llu,\"size\":%zu,\"warned\":%d,\"path\":%llu}",
           cf->fd,(unsigned long long)(uintptr_t)cf->map,cf->alloc_size,
           cf->map_truncated_warned,(unsigned long long)(uintptr_t)cf->path);
}
static void probes(int fd, void *map, size_t size, const char *path, const char *witness) {
    unsigned char vector=0;
    errno=0; int descriptor=fcntl(fd,F_GETFD); int descriptor_errno=errno;
    errno=0; int mapped=mincore(map,size,&vector); int mapping_errno=errno;
    struct stat target={0}, anchor={0};
    errno=0; int present=lstat(path,&target); int path_errno=errno;
    assert(lstat(witness,&anchor)==0);
    printf("{\"descriptor\":%d,\"descriptor_errno\":%d,\"mapping\":%d,\"mapping_errno\":%d,"
           "\"path\":%d,\"path_errno\":%d,\"device\":%llu,\"inode\":%llu,\"links\":%llu,"
           "\"witness_device\":%llu,\"witness_inode\":%llu,\"witness_links\":%llu}",
           descriptor,descriptor_errno,mapped,mapping_errno,present,path_errno,
           (unsigned long long)target.st_dev,(unsigned long long)target.st_ino,
           (unsigned long long)target.st_nlink,(unsigned long long)anchor.st_dev,
           (unsigned long long)anchor.st_ino,(unsigned long long)anchor.st_nlink);
}
static void observe(unsigned id) {
    char directory[]="/tmp/qcore-os-XXXXXX", path[128], witness[128], saved_path[128];
    assert(mkdtemp(directory));
    assert(snprintf(path,sizeof(path),"%s/chunk",directory)>0);
    assert(snprintf(witness,sizeof(witness),"%s/witness",directory)>0);
    strcpy(saved_path,path);
    int fd=open(path,O_CREAT|O_EXCL|O_RDWR,0600); assert(fd>=0);
    size_t page=(size_t)sysconf(_SC_PAGESIZE); assert(page>0 && page<=65536);
    assert(ftruncate(fd,(off_t)page)==0 && link(path,witness)==0);
    void *map=mmap(NULL,page,PROT_READ|PROT_WRITE,MAP_SHARED,fd,0); assert(map!=MAP_FAILED);
    memset(map,0x5a,page);
    struct cio_file cf, before, frame;
    memset(&cf,0,sizeof(cf));
    cf.fd=-1; cf.map=NULL; cf.path=path; cf.flags=CIO_OPEN_RW;
    cf.page_size=page; cf.data_size=9; cf.crc_cur=71;
    if (id<4) cf.fd=id==1 ? -1 : fd;
    if (id>=4 && id<8) { cf.map=id==5 ? NULL : map; cf.alloc_size=id==7 ? 0 : page; cf.map_truncated_warned=1; }
    if (id==8) cf.fd=fd;
    if (id==9) { cf.map=map; cf.alloc_size=page; }
    if (id==3) assert(close(fd)==0);
    if (id==10) assert(unlink(path)==0);
    memcpy(&before,&cf,sizeof(cf));
    printf("{\"case\":%u,\"schema_version\":1,\"helper\":\"%s\",\"was_null\":%d,"
           "\"fixture\":{\"fd\":%d,\"map\":%llu,\"size\":%zu},\"before\":",
           id,id<4 ? "close" : id<8 ? "unmap" : "delete",id==0 || id==4,
           fd,(unsigned long long)(uintptr_t)map,page);
    fields(&before); printf(",\"os_before\":"); probes(fd,map,page,path,witness);
    calls=reports=call_result=call_errno=reported_errno=0;
    argument_fd=-1;argument_map=argument_path=argument_length=0;operation="none";
    int result=id<4 ? cio_file_native_close(id==0 ? NULL : &cf) :
               id<8 ? cio_file_native_unmap(id==4 ? NULL : &cf) : cio_file_native_delete(&cf);
    printf(",\"returned\":%d,\"call\":{\"count\":%d,\"operation\":\"%s\",\"fd\":%d,"
           "\"map\":%llu,\"length\":%zu,\"path\":%llu,\"result\":%d,\"errno\":%d},"
           "\"diagnostics\":%d,\"reported_errno\":%d,\"after\":",
           result,calls,operation,argument_fd,(unsigned long long)argument_map,argument_length,
           (unsigned long long)argument_path,call_result,call_errno,reports,reported_errno);
    fields(&cf); printf(",\"os_after\":"); probes(fd,map,page,path,witness);
    memcpy(&frame,&cf,sizeof(cf));
    if (id<4) frame.fd=before.fd;
    if (id>=4 && id<8) { frame.map=before.map; frame.alloc_size=before.alloc_size; frame.map_truncated_warned=before.map_truncated_warned; }
    printf(",\"other_fields_preserved\":%d,\"path_bytes_preserved\":%d",
           memcmp(&frame,&before,sizeof(cf))==0,strcmp(saved_path,cf.path)==0);
    /* Cleanup belongs to the harness and is outside the observed helper call.
     * An invalid-length munmap failure retains a real mapping to release here. */
    if (id!=6) assert(munmap(map,page)==0);
    if (id!=2 && id!=3) assert(close(fd)==0);
    if (id!=10 && id!=11) assert(unlink(path)==0);
    assert(unlink(witness)==0 && rmdir(directory)==0);
    printf(",\"fixture_cleanup_complete\":1}\n");
}
int main(void) { for (unsigned id=0;id<12;id++) observe(id); return 0; }

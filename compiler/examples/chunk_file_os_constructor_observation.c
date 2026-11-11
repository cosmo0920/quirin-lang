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

/* Native constructor OS failures and rollback, on exclusive Linux fixtures.
 * Wrappers suppress only selected calls; all other calls reach the real kernel.
 * Injections describe harness post-states, not arbitrary kernel failure rules. */
#define QCORE_FILE_OBSERVER_NO_MAIN
#define __wrap_close prior_wrap_close
#include "chunk_file_teardown_observation.c"
#undef __wrap_close
#include <stdarg.h>

static int opening;
static unsigned scenario, size_calls;
static int constructor_fd = -1;
static dev_t constructor_device;
static ino_t constructor_inode;
static char expected_path[256];
int __real_open(const char *, int, ...);
int __real_fstat(int, struct stat *);
int __real_stat(const char *, struct stat *);
int __real_chmod(const char *, mode_t);
int __real_posix_fallocate(int, off_t, off_t);
int __real_ftruncate(int, off_t);
void *__real_mmap(void *, size_t, int, int, int, off_t);

int __wrap_open(const char *path, int flags, ...) {
    mode_t mode = 0;
    if (flags & O_CREAT) {
        va_list args; va_start(args, flags); mode = (mode_t)va_arg(args, int); va_end(args);
    }
    int injected = opening && scenario == 1;
    errno = 0;
    int result = injected ? (errno = EACCES, -1) : __real_open(path, flags, mode);
    int error = errno;
    if (opening) {
        assert(strcmp(path, expected_path) == 0);
        if (result >= 0) {
            struct stat st, path_st;
            assert(__real_fstat(result, &st) == 0 && lstat(path, &path_st) == 0);
            assert(st.st_dev == path_st.st_dev && st.st_ino == path_st.st_ino && st.st_size == 0);
            constructor_fd = result; constructor_device = st.st_dev; constructor_inode = st.st_ino;
        }
        prefix("constructor_open");
        printf(",\"path\":%u,\"flags\":%d,\"mode\":%u,\"result\":%d,\"errno\":%d,\"injected\":%d,\"device\":%llu,\"inode\":%llu}\n",
               identity(path),flags,(unsigned)mode,result,error,injected,
               (unsigned long long)constructor_device,(unsigned long long)constructor_inode);
    }
    errno = error; return result;
}
int __wrap_fstat(int fd, struct stat *st) {
    if (!opening) return __real_fstat(fd, st);
    size_calls++;
    int injected = ((scenario == 2 || scenario == 3) && size_calls == 1) ||
                   (scenario == 7 && size_calls == 2);
    errno = 0;
    int result = injected ? (errno = EIO, -1) : __real_fstat(fd, st);
    int error = errno;
    prefix("constructor_fstat");
    printf(",\"fd\":%d,\"ordinal\":%u,\"result\":%d,\"errno\":%d,\"injected\":%d,\"size\":%lld,\"device\":%llu,\"inode\":%llu}\n",
           fd,size_calls,result,error,injected,result == 0 ? (long long)st->st_size : -1,
           result == 0 ? (unsigned long long)st->st_dev : 0,result == 0 ? (unsigned long long)st->st_ino : 0);
    errno = error; return result;
}
int __wrap_stat(const char *path, struct stat *st) {
    if (!opening) return __real_stat(path, st);
    int injected = scenario == 3 || scenario == 7;
    errno = 0;
    int result = injected ? (errno = EIO, -1) : __real_stat(path, st);
    int error = errno;
    prefix("constructor_stat");
    printf(",\"path\":%u,\"result\":%d,\"errno\":%d,\"injected\":%d,\"size\":%lld,\"device\":%llu,\"inode\":%llu}\n",
           identity(path),result,error,injected,result == 0 ? (long long)st->st_size : -1,
           result == 0 ? (unsigned long long)st->st_dev : 0,result == 0 ? (unsigned long long)st->st_ino : 0);
    errno = error; return result;
}
int __wrap_chmod(const char *path, mode_t mode) {
    errno = 0; int result = __real_chmod(path, mode); int error = errno;
    if (opening) {
        prefix("constructor_chmod");
        printf(",\"path\":%u,\"mode\":%u,\"result\":%d,\"errno\":%d}\n",identity(path),(unsigned)mode,result,error);
    }
    errno = error; return result;
}
int __wrap_posix_fallocate(int fd, off_t offset, off_t length) {
    int injected = opening && scenario == 4;
    /* POSIX returns an error number directly; injection leaves errno alone. */
    int before = errno;
    int result = injected ? ENOSPC : __real_posix_fallocate(fd,offset,length);
    int error = errno;
    if (opening) {
        prefix("constructor_resize");
        printf(",\"fd\":%d,\"offset\":%lld,\"length\":%lld,\"result\":%d,\"errno_before\":%d,\"errno\":%d,\"injected\":%d}\n",
               fd,(long long)offset,(long long)length,result,before,error,injected);
    }
    errno = error; return result;
}
int __wrap_ftruncate(int fd, off_t length) {
    errno = 0; int result = __real_ftruncate(fd,length); int error = errno;
    if (opening) {
        prefix("constructor_truncate");
        printf(",\"fd\":%d,\"length\":%lld,\"result\":%d,\"errno\":%d}\n",fd,(long long)length,result,error);
    }
    errno = error; return result;
}
void *__wrap_mmap(void *address, size_t length, int prot, int flags, int fd, off_t offset) {
    int injected = opening && (scenario == 5 || scenario == 6);
    errno = 0; void *result = injected ? (errno = ENOMEM, MAP_FAILED) : __real_mmap(address,length,prot,flags,fd,offset);
    int error = errno;
    if (opening) {
        prefix("constructor_map");
        printf(",\"address\":%llu,\"length\":%zu,\"prot\":%d,\"flags\":%d,\"fd\":%d,\"offset\":%lld,\"result\":%llu,\"errno\":%d,\"injected\":%d}\n",
               (unsigned long long)(uintptr_t)address,length,prot,flags,fd,(long long)offset,
               (unsigned long long)(uintptr_t)result,error,injected);
    }
    errno = error; return result;
}
int __wrap_close(int fd) {
    if (!opening) return prior_wrap_close(fd);
    int injected = scenario == 6;
    errno = 0; int result = injected ? (errno = EIO, -1) : __real_close(fd); int error = errno;
    prefix("constructor_close");
    printf(",\"fd\":%d,\"result\":%d,\"errno\":%d,\"injected\":%d}\n",fd,result,error,injected);
    errno = error; return result;
}
static void constructor_before_free(void *ptr) {
    if (opening && identity(ptr) == 11) {
        struct cio_file *cf = ptr;
        prefix("rollback_backend");
        printf(",\"id\":11,\"fd\":%d,\"map\":%llu,\"size\":%zu,\"fs_size\":%zu,\"up\":%zu}\n",
               cf->fd,(unsigned long long)(uintptr_t)cf->map,cf->alloc_size,cf->fs_size,context->total_chunks_up);
    }
    file_before_free(ptr);
}
static void probe(const char *phase) {
    struct stat path_st, fd_st;
    errno = 0; int fd_result = fcntl(constructor_fd,F_GETFD); int fd_error = errno;
    errno = 0; int path_result = lstat(expected_path,&path_st); int path_error = errno;
    int fd_stat = fd_result == -1 ? -1 : __real_fstat(constructor_fd,&fd_st);
    prefix("constructor_probe");
    printf(",\"phase\":\"%s\",\"fd\":%d,\"fd_result\":%d,\"fd_errno\":%d,\"fd_device\":%llu,\"fd_inode\":%llu,\"path_result\":%d,\"path_errno\":%d,\"path_size\":%lld,\"device\":%llu,\"inode\":%llu}\n",
           phase,constructor_fd,fd_result,fd_error,
           fd_stat == 0 ? (unsigned long long)fd_st.st_dev : 0,fd_stat == 0 ? (unsigned long long)fd_st.st_ino : 0,
           path_result,path_error,path_result == 0 ? (long long)path_st.st_size : -1,
           path_result == 0 ? (unsigned long long)path_st.st_dev : 0,path_result == 0 ? (unsigned long long)path_st.st_ino : 0);
}
int main(int argc, char **argv) {
    assert(argc == 3);
    scenario = (unsigned)strtoul(argv[1],NULL,10); assert(scenario < 8);
    const char *root = argv[2]; assert(strlen(root) == 27);
    snprintf(expected_path,sizeof(expected_path),"%s/stream-0/chunk-0",root);
    setvbuf(stdout,NULL,_IONBF,0);
    case_id = 200 + scenario;
    prefix("begin_os_constructor"); printf(",\"schema_version\":1,\"scenario\":%u,\"root_length\":27}\n",scenario);
    tracking = 1;
    backend_snapshot_hook = file_snapshot; before_free_hook = constructor_before_free; extra_function_name = file_function;
    struct cio_options options; cio_options_init(&options); options.root_path = root; options.chmod = "0700";
    context = cio_create(&options); assert(context);
    struct cio_stream *st = cio_stream_create(context,"stream-0",CIO_STORE_FS); assert(st);
    streams[stream_count++] = st;
    int error = -999;
    opening = 1;
    struct cio_chunk *ch = cio_chunk_open(context,st,"chunk-0",CIO_OPEN_RW,64,&error);
    opening = 0;
    if (ch) chunks[chunk_count++] = ch;
    prefix("constructor_result");
    printf(",\"context\":%u,\"stream\":%u,\"chunk\":%u,\"error\":%d}\n",identity(context),identity(st),identity(ch),error);
    probe("returned"); snapshot();
    destroying = 1; cio_destroy(context); destroying = 0;
    probe("parents_released");
    if (ch) {
        unsigned char vector;
        errno = 0; int result = mincore(saved_map,saved_size,&vector); int map_error = errno;
        prefix("terminal_mapping");
        printf(",\"map\":%llu,\"size\":%zu,\"result\":%d,\"errno\":%d}\n",
               (unsigned long long)(uintptr_t)saved_map,saved_size,result,map_error);
    }
    unsigned live = 0; for (unsigned i=0;i<used;i++) live += allocations[i].live;
    prefix("end"); printf(",\"live_allocations\":%u}\n",live); assert(live == 0);
    tracking = 0;
    /* Reclaim the observed residual only after recording all parent releases. */
    if (scenario == 6) {
        errno = 0; int result = __real_close(constructor_fd); int error = errno;
        errno = 0; int fd_result = fcntl(constructor_fd,F_GETFD); int fd_error = errno;
        prefix("harness_descriptor_cleanup");
        printf(",\"fd\":%d,\"result\":%d,\"errno\":%d,\"fd_result\":%d,\"fd_errno\":%d}\n",
               constructor_fd,result,error,fd_result,fd_error);
        assert(result == 0 && fd_result == -1 && fd_error == EBADF);
    }
    return 0;
}

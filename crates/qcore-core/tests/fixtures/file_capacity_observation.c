/*
 * Card O02 — one capacity boundary family, observed natively.
 *
 * Six fixed sequences grow one file-backed chunk in an exclusive, disposable
 * directory and record every acquisition epoch and state transition, including
 * retries and teardown. The capacity refusal is a real kernel refusal obtained
 * with RLIMIT_FSIZE (EFBIG), not a suppressed wrapper: every syscall here
 * reaches the kernel.
 *
 * Usage: file_capacity_observation <scenario 0..5> <existing empty directory>
 *
 * The trace is written to stdout, one `QCORE_CAP` record per line, and is
 * validated by `qcore_core::capacity_boundary`. Nothing here concludes
 * anything on its own.
 */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <signal.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/resource.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <unistd.h>

#define SEED_LEN 4096u
#define GROWN_LEN 8192u

static unsigned long long map_generation;
static unsigned long long fd_generation;
static unsigned long long live_maps;
static unsigned long long live_fds;

static void emit(const char *format, ...) {
  va_list arguments;
  va_start(arguments, format);
  fputs("QCORE_CAP ", stdout);
  vprintf(format, arguments);
  fputc('\n', stdout);
  va_end(arguments);
  fflush(stdout);
}

static void fail(const char *what) {
  fprintf(stderr, "fixture failure: %s: %s\n", what, strerror(errno));
  exit(2);
}

/* Read the two capacity facts: the length, and how much of it is backed. */
static void measure(int fd, unsigned long long *length, unsigned long long *blocks) {
  struct stat status;
  if (fstat(fd, &status) != 0) {
    fail("fstat");
  }
  *length = (unsigned long long)status.st_size;
  *blocks = (unsigned long long)status.st_blocks;
}

static void set_file_limit(rlim_t soft) {
  struct rlimit before;
  struct rlimit after;
  if (getrlimit(RLIMIT_FSIZE, &before) != 0) {
    fail("getrlimit");
  }
  after = before;
  after.rlim_cur = soft;
  if (setrlimit(RLIMIT_FSIZE, &after) != 0) {
    fail("setrlimit");
  }
  if (before.rlim_cur == RLIM_INFINITY) {
    emit("LIMIT soft_before=inf soft_after=%llu", (unsigned long long)soft);
  } else {
    emit("LIMIT soft_before=%llu soft_after=%llu", (unsigned long long)before.rlim_cur,
         (unsigned long long)soft);
  }
}

/* Create the exclusive fixture file and fill it with real, backed bytes. */
static int open_fixture(const char *directory, char *path, size_t path_size) {
  unsigned char *seed;
  unsigned long long length;
  unsigned long long blocks;
  struct stat status;
  int fd;

  if ((size_t)snprintf(path, path_size, "%s/chunk-0", directory) >= path_size) {
    fail("fixture path");
  }
  fd = open(path, O_RDWR | O_CREAT | O_EXCL, 0600);
  if (fd < 0) {
    fail("open");
  }
  seed = calloc(1, SEED_LEN);
  if (seed == NULL) {
    fail("calloc");
  }
  memcpy(seed, "qcore-o02", 9);
  if (write(fd, seed, SEED_LEN) != (ssize_t)SEED_LEN) {
    fail("seed write");
  }
  free(seed);
  if (fsync(fd) != 0) {
    fail("fsync");
  }
  if (fstat(fd, &status) != 0) {
    fail("fstat");
  }
  measure(fd, &length, &blocks);
  fd_generation += 1;
  live_fds += 1;
  emit("OPEN gen=%llu inode=%llu file_len=%llu blocks=%llu", fd_generation,
       (unsigned long long)status.st_ino, length, blocks);
  return fd;
}

static void *map_file(int fd, size_t length) {
  unsigned long long file_length;
  unsigned long long blocks;
  void *address = mmap(NULL, length, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
  if (address == MAP_FAILED) {
    fail("mmap");
  }
  measure(fd, &file_length, &blocks);
  map_generation += 1;
  live_maps += 1;
  emit("MAP gen=%llu addr=0x%llx len=%zu file_len=%llu", map_generation,
       (unsigned long long)(uintptr_t)address, length, file_length);
  return address;
}

static void unmap(void *address, size_t length, unsigned long long generation) {
  int result = munmap(address, length);
  emit("UNMAP gen=%llu len=%zu ret=%d", generation, length, result);
  if (result == 0) {
    live_maps -= 1;
  }
}

static void close_fixture(int fd, unsigned long long generation) {
  int result = close(fd);
  emit("CLOSE gen=%llu ret=%d", generation, result);
  if (result == 0) {
    live_fds -= 1;
  }
}

/* One grow attempt, with the length and backing measured on both sides. */
static int grow(int fd, const char *call, unsigned long long request, int errno_read) {
  unsigned long long before_len;
  unsigned long long before_blocks;
  unsigned long long after_len;
  unsigned long long after_blocks;
  long long returned;
  int observed_errno;

  measure(fd, &before_len, &before_blocks);
  errno = 0;
  if (strcmp(call, "ftruncate") == 0) {
    returned = ftruncate(fd, (off_t)request);
    observed_errno = errno;
  } else if (strcmp(call, "posix_fallocate") == 0) {
    /* posix_fallocate reports the error as its return value and leaves errno
     * untouched; the fixture records that separation rather than hiding it. */
    returned = posix_fallocate(fd, 0, (off_t)request);
    observed_errno = errno;
  } else {
    fail("unknown grow call");
    return -1;
  }
  measure(fd, &after_len, &after_blocks);
  if (returned == 0) {
    emit("GROW call=%s request=%llu ret=0 error=none errno_read=%d len_before=%llu "
         "len_after=%llu blocks_before=%llu blocks_after=%llu transferred=0",
         call, request, errno_read, before_len, after_len, before_blocks, after_blocks);
    return 0;
  }
  if (strcmp(call, "posix_fallocate") == 0) {
    /* The return value *is* the error number here. */
    emit("GROW call=%s request=%llu ret=%lld error=%lld errno_read=0 len_before=%llu "
         "len_after=%llu blocks_before=%llu blocks_after=%llu transferred=0",
         call, request, returned, returned, before_len, after_len, before_blocks, after_blocks);
    if (observed_errno != 0) {
      fprintf(stderr, "fixture note: posix_fallocate also set errno=%d\n", observed_errno);
    }
    return -1;
  }
  emit("GROW call=%s request=%llu ret=-1 error=%d errno_read=1 len_before=%llu "
       "len_after=%llu blocks_before=%llu blocks_after=%llu transferred=0",
       call, request, observed_errno, before_len, after_len, before_blocks, after_blocks);
  return -1;
}

/* One extending write at the boundary, which may transfer part of its request. */
static void extend_by_write(int fd, unsigned long long offset, unsigned long long count) {
  unsigned char payload[16];
  unsigned long long before_len;
  unsigned long long before_blocks;
  unsigned long long after_len;
  unsigned long long after_blocks;
  ssize_t moved;
  int observed_errno;

  if (count > sizeof(payload)) {
    fail("payload too large");
  }
  memset(payload, 0x5a, sizeof(payload));
  measure(fd, &before_len, &before_blocks);
  errno = 0;
  moved = pwrite(fd, payload, (size_t)count, (off_t)offset);
  observed_errno = errno;
  measure(fd, &after_len, &after_blocks);
  if (moved < 0) {
    emit("GROW call=write request=%llu ret=-1 error=%d errno_read=1 len_before=%llu "
         "len_after=%llu blocks_before=%llu blocks_after=%llu transferred=0",
         offset + count, observed_errno, before_len, after_len, before_blocks, after_blocks);
    return;
  }
  emit("GROW call=write request=%llu ret=%zd error=none errno_read=1 len_before=%llu "
       "len_after=%llu blocks_before=%llu blocks_after=%llu transferred=%zd",
       offset + count, moved, before_len, after_len, before_blocks, after_blocks, moved);
}

static void *remap(int fd, void *address, size_t from_len, size_t to_len,
                   unsigned long long from_generation) {
  unsigned long long file_length;
  unsigned long long blocks;
  void *moved = mremap(address, from_len, to_len, MREMAP_MAYMOVE);
  if (moved == MAP_FAILED) {
    fail("mremap");
  }
  measure(fd, &file_length, &blocks);
  map_generation += 1;
  emit("REMAP from=%llu to=%llu addr=0x%llx len=%zu file_len=%llu", from_generation,
       map_generation, (unsigned long long)(uintptr_t)moved, to_len, file_length);
  return moved;
}

int main(int argc, char **argv) {
  char path[4096];
  void *address;
  unsigned long long scenario;
  unsigned long long mapping;
  unsigned long long descriptor;
  int fd;

  if (argc != 3) {
    fprintf(stderr, "usage: %s <scenario> <directory>\n", argv[0]);
    return 2;
  }
  scenario = strtoull(argv[1], NULL, 10);
  /* Without this the kernel terminates the process at the boundary and no
   * teardown runs at all, so no post-state would be observable. */
  if (signal(SIGXFSZ, SIG_IGN) == SIG_ERR) {
    fail("signal");
  }
  emit("FIXTURE scenario=%llu", scenario);
  emit("SIGNAL handled=1");
  fd = open_fixture(argv[2], path, sizeof(path));
  descriptor = fd_generation;

  switch (scenario) {
  case 0: {
    /* A length-only grant: the file grows, nothing is reserved, and the
     * mapping must be remapped before the new range is touched. */
    address = map_file(fd, SEED_LEN);
    mapping = map_generation;
    if (grow(fd, "ftruncate", GROWN_LEN, 0) != 0) {
      fail("scenario 0 expected the grant");
    }
    address = remap(fd, address, SEED_LEN, GROWN_LEN, mapping);
    mapping = map_generation;
    /* The content length stays where the backed bytes end. */
    emit("LOGICAL len=%u", SEED_LEN);
    unmap(address, GROWN_LEN, mapping);
    break;
  }
  case 1: {
    /* A reserved grant: the allocating call reserves the range, so the new
     * bytes are writable authority. */
    address = map_file(fd, SEED_LEN);
    mapping = map_generation;
    if (grow(fd, "posix_fallocate", GROWN_LEN, 0) != 0) {
      fail("scenario 1 expected the grant");
    }
    address = remap(fd, address, SEED_LEN, GROWN_LEN, mapping);
    mapping = map_generation;
    ((unsigned char *)address)[GROWN_LEN - 1] = 0x2a;
    emit("LOGICAL len=%u", GROWN_LEN);
    unmap(address, GROWN_LEN, mapping);
    break;
  }
  case 2: {
    /* A capacity refusal, a retry with nothing changed, and a grant only
     * after the enforced capacity changed. */
    address = map_file(fd, SEED_LEN);
    mapping = map_generation;
    set_file_limit((rlim_t)SEED_LEN);
    if (grow(fd, "ftruncate", GROWN_LEN, 1) == 0) {
      fail("scenario 2 expected the refusal");
    }
    if (grow(fd, "ftruncate", GROWN_LEN, 1) == 0) {
      fail("scenario 2 expected the repeated refusal");
    }
    set_file_limit((rlim_t)(GROWN_LEN * 4));
    if (grow(fd, "ftruncate", GROWN_LEN, 1) != 0) {
      fail("scenario 2 expected the grant after the limit changed");
    }
    address = remap(fd, address, SEED_LEN, GROWN_LEN, mapping);
    mapping = map_generation;
    emit("LOGICAL len=%u", SEED_LEN);
    unmap(address, GROWN_LEN, mapping);
    break;
  }
  case 3: {
    /* The allocating call reports its error as a return value and does not
     * set errno; the mapping and the descriptor survive untouched. */
    address = map_file(fd, SEED_LEN);
    mapping = map_generation;
    set_file_limit((rlim_t)SEED_LEN);
    if (grow(fd, "posix_fallocate", GROWN_LEN, 0) == 0) {
      fail("scenario 3 expected the refusal");
    }
    set_file_limit((rlim_t)(GROWN_LEN * 4));
    emit("LOGICAL len=%u", SEED_LEN);
    unmap(address, SEED_LEN, mapping);
    break;
  }
  case 4: {
    /* A transfer that moves part of its request at the boundary. */
    address = map_file(fd, SEED_LEN);
    mapping = map_generation;
    emit("LOGICAL len=%u", SEED_LEN);
    set_file_limit((rlim_t)(SEED_LEN + 4));
    extend_by_write(fd, SEED_LEN, 8);
    emit("LOGICAL len=%u", SEED_LEN + 4);
    extend_by_write(fd, SEED_LEN + 4, 1);
    set_file_limit((rlim_t)(GROWN_LEN * 4));
    unmap(address, SEED_LEN, mapping);
    break;
  }
  case 5: {
    /* A mapping longer than the file. It is live, and it witnesses no
     * capacity: the range past the end faults on first touch, so the fixture
     * does not touch it. */
    address = map_file(fd, SEED_LEN);
    mapping = map_generation;
    unmap(address, SEED_LEN, mapping);
    address = map_file(fd, GROWN_LEN);
    mapping = map_generation;
    if (((unsigned char *)address)[0] != 'q') {
      fail("scenario 5 expected the original first byte");
    }
    emit("LOGICAL len=%u", SEED_LEN);
    unmap(address, GROWN_LEN, mapping);
    break;
  }
  default:
    fprintf(stderr, "unknown scenario\n");
    close_fixture(fd, descriptor);
    return 2;
  }

  close_fixture(fd, descriptor);
  if (unlink(path) != 0) {
    fail("unlink");
  }
  emit("END scenario=%llu live_maps=%llu live_fds=%llu", scenario, live_maps, live_fds);
  return 0;
}

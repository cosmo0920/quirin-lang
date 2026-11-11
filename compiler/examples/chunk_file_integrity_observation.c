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

/* Card O01: bounded ChunkIO reopen corruption/checksum observation family.
 *
 * One process per fixture. The chunk context is created with CIO_CHECKSUM, so
 * the pinned `cio_file.c` format check runs its crc32 comparison over the
 * mapped content. Each fixture is an exclusive temporary directory seeded by
 * the preparation driver; this program never repairs, rewrites or deletes it.
 *
 * The observer records, it does not decide. Structural asserts are limited to
 * harness invariants (fixture identity, allocation accounting, tracked mapping
 * identity); the corruption/injection classification and the custody
 * accounting are validated afterwards by `qcore_core::chunk_integrity`.
 *
 * A corrupted input and an injected operation failure are separate axes here
 * and stay separate in the emitted record: the injection wrappers return a
 * fixed error without reaching the kernel, so an injected failure is a
 * statement about the wrapper, never an observed OS post-state.
 */
#include "native_file_profile_observer.h"

static FILE *records;
static const char *const profiles[8] = {
    "clean_checksum", "bad_checksum", "bad_magic", "truncated_header",
    "truncated_content", "clean_injected_map", "bad_checksum_injected_close",
    "clean_injected_close",
};
static const char *const inputs[8] = {
    "clean", "bad_checksum", "bad_magic", "truncated_header",
    "truncated_content", "clean", "bad_checksum", "clean",
};
static const char *const injections[8] = {
    "none", "none", "none", "none", "none", "mmap", "close", "close",
};
static int rollback_error = -999;

static void integrity_before_free(void *ptr) {
    /* The chunk carries the fine-grained cio_error code; on a refused open it
     * is only readable while the rollback frees it. Identity 7 is the chunk
     * allocation of this fixed profile, the same slot the shared observer
     * already reports as `rollback_chunk`. */
    if (!destroying && identity(ptr) == 7) { rollback_error = cio_error_get(ptr); }
    profile_before_free(ptr);
}

static void emit_file(const char *key, const char *path) {
    unsigned char buffer[8192];
    FILE *handle = fopen(path, "rb");
    size_t size = 0;
    assert(handle);
    size = fread(buffer, 1, sizeof(buffer), handle);
    assert(!ferror(handle) && feof(handle));
    assert(fclose(handle) == 0);
    fprintf(records, " %s=", key);
    for (size_t i = 0; i < size; i++) { fprintf(records, "%02x", buffer[i]); }
}

int main(int argc, char **argv) {
    struct stat before, after;
    unsigned scenario, live = 0;
    int error = -999, chunk_error = -999, residual_fd, residual_map;
    struct cio_chunk *ch;

    assert(argc == 4);
    scenario = (unsigned) strtoul(argv[1], NULL, 10);
    assert(scenario < 8);
    records = fopen(argv[3], "w");
    assert(records);

    fail_mmap = scenario == 5;
    fail_close = scenario == 6 || scenario == 7;
    profile_flags = CIO_CHECKSUM;

    fprintf(records, "case %u profile=%s\n", 400 + scenario, profiles[scenario]);
    fprintf(records, "input kind=%s", inputs[scenario]);
    profile_init(argv[2], 400 + scenario, "begin_integrity");
    before_free_hook = integrity_before_free;
    assert(lstat(file_path, &before) == 0);
    emit_file("pre", file_path);
    fprintf(records, "\n");

    ch = cio_chunk_open(context, streams[0], "chunk-0", CIO_OPEN_RW, 64, &error);
    if (ch) { chunks[chunk_count++] = ch; chunk_error = cio_error_get(ch); }
    prefix("constructor_result");
    printf(",\"chunk\":%u,\"error\":%d}\n", identity(ch), error);
    fixture_probe("returned");
    snapshot();
    if (!ch) { chunk_error = rollback_error; }
    profile_destroy();

    /* Endpoint residuals after the parent context is released. A suppressed
     * close leaves the descriptor open because the syscall never ran; that is
     * recorded as a harness reclamation below, never as a kernel post-state. */
    errno = 0;
    residual_fd = fd_epoch && fcntl(opened_fd, F_GETFD) >= 0;
    errno = 0;
    if (map_epoch && map_live) {
        unsigned char vector;
        residual_map = mincore(last_map, last_map_size, &vector) == 0;
    } else {
        residual_map = 0;
    }
    for (unsigned i = 0; i < used; i++) { live += allocations[i].live; }

    assert(lstat(file_path, &after) == 0);
    fprintf(records, "injection op=%s errno=%d suppressed=%u\n", injections[scenario],
            strcmp(injections[scenario], "none") == 0 ? 0 : EIO,
            scenario == 5 ? os_map_failed : (scenario >= 6 ? os_close_failed : 0u));
    fprintf(records, "outcome chunk=%d open_error=%d chunk_error=%d\n", ch != NULL, error, chunk_error);
    fprintf(records, "allocations acquired=%u library_released=%u harness_released=0 outstanding=%u\n",
            used, used - live, live);
    fprintf(records, "descriptor epochs=%u library_closed=%u harness_closed=%d residual_after_parents=%d\n",
            fd_epoch, os_close_ok, residual_fd, residual_fd);
    fprintf(records, "mapping epochs=%u library_unmapped=%u harness_unmapped=%d residual_after_parents=%d\n",
            map_epoch, os_unmap_ok, residual_map, residual_map);
    fprintf(records, "fixture inode_preserved=%d size_preserved=%d",
            before.st_dev == after.st_dev && before.st_ino == after.st_ino,
            before.st_size == after.st_size);
    emit_file("post", file_path);
    fprintf(records, "\n");

    /* Harness reclamation of the endpoints the suppressed wrapper retained, so
     * the sanitized process still terminates with nothing outstanding. It is
     * recorded on its own line and never merged with the library tally. */
    if (residual_map) { assert(__real_munmap(last_map, last_map_size) == 0); }
    if (residual_fd) { assert(__real_close(opened_fd) == 0); }
    fprintf(records, "reclaimed descriptors=%d mappings=%d\n", residual_fd, residual_map);
    fprintf(records, "end case %u\n", 400 + scenario);
    assert(fclose(records) == 0);
    return 0;
}

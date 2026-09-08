/*
 * O04 — declarations for the constructor-to-caller excerpt.
 *
 * The two function bodies this prelude precedes are byte-verbatim upstream
 * regions (see `provenance.json`; the digests are asserted by the test). This
 * prelude is NOT upstream: it is the smallest set of declarations that makes
 * those two bodies a well-formed translation unit Clang accepts.
 *
 * What is faithful, and what is not:
 *
 *  - `struct flb_storage_input` is the upstream declaration verbatim
 *    (`flb_storage_input.h.txt`, concatenated by the test rather than copied
 *    here), because the caller reads two of its members and the typed rule
 *    therefore depends on every one of its member declarations.
 *
 *  - `struct flb_input_instance` is REDUCED to the members the two bodies
 *    mention. The upstream declaration has 239 member lines and transitively
 *    requires roughly fifty other complete type definitions, which a
 *    hand-assembled excerpt cannot supply. The reduction is safe for the rule
 *    under test only if the *upstream* declaration is also readable by the
 *    same reader and also declares `storage` as a pointer member — which is
 *    not assumed here but measured directly against the upstream bytes by
 *    `the_upstream_record_declarations_are_readable_and_declare_the_read_members`.
 *
 *  - The `#define`d constants of the upstream headers are modeled as
 *    enumeration constants. The rule that admits them is insensitive to the
 *    choice: in the real preprocessed unit they are integer literals, which
 *    the same walker admits as numeric literals.
 *
 *  - Every callee is declared with its upstream prototype. `flb_error` and
 *    `flb_trace` are the logging expansions' entry points and are left
 *    deliberately UNcontracted, so the arms that log stay Unknown.
 */

#define NULL ((void *) 0)
#define FLB_HAVE_METRICS 1
#define CIO_HAVE_BACKEND_FILESYSTEM 1

/* Use the analyzing Clang target's types without requiring target libc headers.
 * Windows x64 is LLP64: unsigned long is neither size_t nor uint64_t there. */
typedef __SIZE_TYPE__ size_t;
typedef __PTRDIFF_TYPE__ ssize_t;
typedef __UINT64_TYPE__ uint64_t;
typedef uint64_t flb_route_mask_element;
typedef char *flb_sds_t;
typedef int pthread_mutex_t;

enum {
    FLB_FALSE = 0,
    FLB_TRUE = 1,
    CIO_FALSE = 0,
    CIO_TRUE = 1,
    CIO_OPEN = 1,
    FLB_INPUT_CHUNK_SIZE = 2097152,
    FLB_INPUT_LOGS = 0,
    FLB_INPUT_METRICS = 1,
    FLB_INPUT_TRACES = 2,
    FLB_INPUT_PROFILES = 3,
    CIO_STORE_FS = 0,
    CIO_STORE_MEM = 1,
    CIO_OK = 0
};

struct mk_list {
    struct mk_list *prev;
    struct mk_list *next;
};

struct flb_time {
    long tm_sec;
    long tm_nsec;
};

struct msgpack_packer {
    void *data;
    void *callback;
};

struct cio_ctx;
struct flb_task;
struct flb_hash_table;
struct flb_router;

/* chunkio records the constructor reads. Reduced in the same way, and for the
 * same reason, as `struct flb_input_instance` above. */
struct cio_stream {
    int type;
    char *name;
    struct mk_list _head;
    struct mk_list chunks;
    struct mk_list chunks_up;
    struct mk_list chunks_down;
    void *parent;
};

struct cio_chunk {
    char *name;
    struct cio_ctx *ctx;
    struct cio_stream *st;
    int lock;
    int tx_active;
    unsigned int tx_crc;
    size_t tx_content_length;
    void *backend;
    struct mk_list _head;
    struct mk_list _state_head;
};

struct flb_config {
    struct flb_router *router;
};

/* Reduced; see the header comment. */
struct flb_input_instance {
    int id;
    int event_type;
    struct flb_storage_input *storage;
    struct flb_config *config;
    struct mk_list chunks;
    struct flb_hash_table *ht_log_chunks;
    struct flb_hash_table *ht_metric_chunks;
    struct flb_hash_table *ht_trace_chunks;
    struct flb_hash_table *ht_profile_chunks;
    pthread_mutex_t metrics_chunk_lock;
    struct mk_list _head;
};

struct flb_input_chunk {
    int event_type;
    int busy;
    int fs_counted;
    struct cio_chunk *chunk;
    int fs_backlog;
    struct flb_input_instance *in;
    size_t stream_off;
    struct flb_task *task;
    double create_time;
    unsigned long total_records;
    flb_route_mask_element *routes_mask;
    struct msgpack_packer mp_pck;
    struct mk_list _head;
};

/* chunkio */
void cio_chunk_close(struct cio_chunk *ch, int delete);
int cio_chunk_is_up(struct cio_chunk *ch);
int cio_chunk_up_force(struct cio_chunk *ch);
int cio_chunk_down(struct cio_chunk *ch);

/* chunkio callees of the constructor body */
void *malloc(size_t size);
void free(void *p);
char *strdup(const char *s);
size_t strlen(const char *s);
void mk_list_del(struct mk_list *entry);
void cio_log_error(struct cio_ctx *ctx, const char *message);
void cio_error_reset(struct cio_chunk *ch);
void cio_chunk_counter_total_add(struct cio_ctx *ctx);
void cio_errno(void);
void *cio_file_open(struct cio_ctx *ctx, struct cio_stream *st,
                    struct cio_chunk *ch, int flags, size_t size, int *err);
void *cio_memfs_open(struct cio_ctx *ctx, struct cio_stream *st,
                     struct cio_chunk *ch, int flags, size_t size);

/* fluent-bit */
void *flb_calloc(size_t n, size_t size);
void flb_free(void *p);
void generate_chunk_name(struct flb_input_instance *in,
                         char *out_buf, int buf_size);
int input_chunk_write_header(struct cio_chunk *chunk, int event_type,
                             char *tag, int tag_len);
size_t flb_routes_mask_get_size(struct flb_router *router);
int flb_routes_mask_set_by_tag(flb_route_mask_element *routes_mask,
                               const char *tag, int tag_len,
                               struct flb_input_instance *in);
flb_sds_t flb_input_chunk_get_name(struct flb_input_chunk *ic);
int flb_hash_table_add(struct flb_hash_table *ht, const char *key, int key_len,
                       void *val, ssize_t val_size);
void mk_list_add(struct mk_list *entry, struct mk_list *head);
void msgpack_packer_init(struct msgpack_packer *pk, void *data, void *callback);
int flb_input_chunk_write(void *data, const char *buf, size_t len);
double flb_time_now(void);
int pthread_mutex_lock(pthread_mutex_t *mutex);
int pthread_mutex_unlock(pthread_mutex_t *mutex);

/* Deliberately uncontracted: the logging and errno expansions. */
void flb_error(const char *format, ...);
void flb_trace(const char *format, ...);
void flb_errno(void);

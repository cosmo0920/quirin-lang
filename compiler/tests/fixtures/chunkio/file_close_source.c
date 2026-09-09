
typedef unsigned long size_t;
struct cio_ctx {size_t total_chunks_up; size_t total_chunks;};
struct cio_stream {char *name;};
struct cio_chunk {void *backend; struct cio_ctx *ctx; struct cio_stream *st; char *name;};
struct cio_file {int fd;char *map; size_t alloc_size; size_t data_size; int synced; int map_truncated_warned; char *path;};
int munmap(void *,size_t); int *__errno_location(void); int cio_errno_print(int,const char *,int);
int cio_file_sync(struct cio_chunk *);int cio_file_native_close(struct cio_file *);void free(void *);
void cio_log_print(void *,int,const char *,int,const char *,...);
int cio_file_native_unmap(struct cio_file *cf){if(!cf)return -1;if(!cf->map)return 0;int r=munmap(cf->map,cf->alloc_size);if(r!=0){cio_errno_print(*__errno_location(),"test.c",1);return -1;}cf->map=(void*)0;cf->alloc_size=0;cf->map_truncated_warned=0;return 0;}
size_t cio_chunk_counter_total_up_sub(struct cio_ctx *ctx){ctx->total_chunks_up--;return ctx->total_chunks_up;}
size_t cio_chunk_counter_total_sub(struct cio_ctx *ctx){ctx->total_chunks--;return ctx->total_chunks;}
static int munmap_file(struct cio_ctx *ctx, struct cio_chunk *ch){
struct cio_file *cf=ch->backend;
if(!cf)return -1;
if(!cf->map)return -1;
int ret;
if(cf->synced==0){ret=cio_file_sync(ch);if(ret==-1)cio_log_print(ch->ctx,1,"test.c",1,"[cio file] error syncing file at %s:%s",ch->st->name,ch->name);}
ret=cio_file_native_unmap(cf);
if(ret!=0)return -1;
cf->data_size=0;cf->alloc_size=0;cio_chunk_counter_total_up_sub(ctx);return 0;
}

int close(int);int cio_file_native_delete(struct cio_file *);
int cio_file_native_close(struct cio_file *cf){if(!cf)return -1;if(cf->fd!=-1){int r=close(cf->fd);if(r==-1){cio_errno_print(*__errno_location(),"test.c",1);return -1;}cf->fd=-1;}return 0;}
void cio_file_close(struct cio_chunk *ch,int delete){struct cio_file *cf=ch->backend;if(!cf)return;
munmap_file(ch->ctx,ch);cio_file_native_close(cf);
if(delete==1){int r=cio_file_native_delete(cf);if(r!=0)cio_log_print(ch->ctx,1,"test.c",1,"[cio file] error deleting file at close %s:%s",ch->st->name,ch->name);}
free(cf->path);free(cf);
}

int unlink(const char *);
int cio_file_native_delete(struct cio_file *cf){if(cf->fd!=-1 || cf->map!=(void*)0)return -1;int r=unlink(cf->path);if(r==-1){cio_errno_print(*__errno_location(),"test.c",1);return -1;}return 0;}

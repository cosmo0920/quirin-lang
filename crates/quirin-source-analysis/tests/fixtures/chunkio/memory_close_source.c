
#define CIO_STORE_MEM 1
#define CIO_STORE_FS 0
typedef unsigned long qcore_test_size_t;
struct mk_list { struct mk_list *prev; struct mk_list *next; };
struct cio_ctx {qcore_test_size_t total_chunks; qcore_test_size_t total_chunks_up;};
struct cio_stream {int type;};
struct cio_chunk {void *backend; char *name; struct cio_ctx *ctx; struct cio_stream *st; struct mk_list _head; struct mk_list _state_head; int lock; int error_n;};
struct cio_memfs {char *name; char *buf_data; char *meta_data;};
void free(void *);
void cio_file_close(struct cio_chunk *,int);
void cio_error_reset(struct cio_chunk *ch){ch->error_n=0;}
void cio_memfs_close(struct cio_chunk *ch){struct cio_memfs *mf=ch->backend;if(!mf)return;free(mf->name);free(mf->buf_data);free(mf->meta_data);free(mf);}
void __mk_list_del(struct mk_list *prev,struct mk_list *next){prev->next=next;next->prev=prev;}
void mk_list_del(struct mk_list *entry){__mk_list_del(entry->prev,entry->next);entry->prev=(void *)0;entry->next=(void *)0;}
qcore_test_size_t cio_chunk_counter_total_add(struct cio_ctx *ctx){ctx->total_chunks++;return ctx->total_chunks;}
qcore_test_size_t cio_chunk_counter_total_sub(struct cio_ctx *ctx){ctx->total_chunks--;return ctx->total_chunks;}
qcore_test_size_t cio_chunk_counter_total_up_add(struct cio_ctx *ctx){ctx->total_chunks_up++;return ctx->total_chunks_up;}
qcore_test_size_t cio_chunk_counter_total_up_sub(struct cio_ctx *ctx){ctx->total_chunks_up--;return ctx->total_chunks_up;}
void cio_chunk_close(struct cio_chunk *ch,int delete){
int type;struct cio_ctx *ctx;
if(!ch){return;}
cio_error_reset(ch);
ctx=ch->ctx;type=ch->st->type;
if(type==CIO_STORE_MEM){cio_memfs_close(ch);}
else if(type==CIO_STORE_FS){cio_file_close(ch,delete);}
mk_list_del(&ch->_head);mk_list_del(&ch->_state_head);
free(ch->name);free(ch);
cio_chunk_counter_total_sub(ctx);
}
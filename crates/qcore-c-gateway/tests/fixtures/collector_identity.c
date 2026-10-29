
typedef __SIZE_TYPE__ size_t;
#define offsetof(TYPE,MEMBER) ((size_t)&((TYPE *)0)->MEMBER)
#define container_of(ptr,type,member) ({ const typeof(((type *)0)->member) *__mptr=(ptr); (type *)((char *)__mptr - offsetof(type,member)); })
#define mk_list_entry(ptr,type,member) container_of(ptr,type,member)
#define mk_list_foreach(curr,head) for(curr=(head)->next;curr!=(head);curr=curr->next)
struct mk_list { struct mk_list *prev,*next; };
struct loop { int fd; };
struct worker { struct loop *evl; };
struct config { struct loop *evl; };
struct instance { int is_threaded; struct worker *thi; struct mk_list collectors; };
struct collector { struct loop *evl; struct instance *instance; struct mk_list _head; int id; };
void *alloc(size_t); void opaque(void *);
static inline int threaded(struct instance *ins) { return ins->is_threaded; }
static struct collector *get_collector(int id,struct instance *in) {
    struct mk_list *head;
    struct collector *coll;
    mk_list_foreach(head,&in->collectors) {
        coll=mk_list_entry(head,struct collector,_head);
        if(coll->id==id) { return coll; }
    }
    return ((void *)0);
}
static struct collector *create(int unused,struct instance *ins,struct config *config) {
    struct collector *coll; struct worker *thi;
    coll=alloc(sizeof(struct collector));
    if(!coll) { return ((void *)0); }
    coll->instance=ins;
    if(threaded(ins)) { thi=ins->thi; coll->evl=thi->evl; }
    else { coll->evl=config->evl; }
    opaque(coll);
    return coll;
}

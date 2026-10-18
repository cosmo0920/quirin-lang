#ifndef DETACH_IMPL_H
#define DETACH_IMPL_H
typedef struct DetachNode {
    struct DetachNode *prev;
    struct DetachNode *next;
    struct DetachNode *child;
} DetachNode;
static DetachNode *dt_detach(DetachNode *parent, DetachNode * const item)
{
    if ((parent == 0) || (item == 0) || (item != parent->child && item->prev == 0))
    {
        return 0;
    }
    if (item != parent->child)
    {
        item->prev->next = item->next;
    }
    if (item->next != 0)
    {
        item->next->prev = item->prev;
    }
    if (item == parent->child)
    {
        parent->child = item->next;
    }
    else if (item->next == 0)
    {
        parent->child->prev = item->prev;
    }
    item->prev = 0;
    item->next = 0;
    return item;
}
#endif

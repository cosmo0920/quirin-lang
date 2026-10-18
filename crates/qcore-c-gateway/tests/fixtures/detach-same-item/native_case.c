#include <assert.h>
#include <stddef.h>
#include <string.h>
#include "detach_impl.h"

int main(void)
{
    for (int count = 1; count <= 6; count++) {
        for (int removed = 0; removed < count; removed++) {
            struct DetachNode nodes[6] = {{0}};
            struct DetachNode foreign = {0};
            struct DetachNode parent = {&foreign, &foreign, &nodes[0]};
            foreign.prev = &foreign;
            foreign.next = &foreign;
            for (int index = 0; index < count; index++) {
                nodes[index].prev = &nodes[index == 0 ? count - 1 : index - 1];
                nodes[index].next = index + 1 < count ? &nodes[index + 1] : NULL;
            }
            struct DetachNode before[6];
            memcpy(before, nodes, sizeof(nodes));
            struct DetachNode original_parent = parent;
            assert(dt_detach(&parent, NULL) == NULL);
            assert(memcmp(before, nodes, sizeof(nodes)) == 0);
            assert(memcmp(&original_parent, &parent, sizeof(parent)) == 0);
            assert(dt_detach(NULL, &nodes[removed]) == NULL);
            assert(memcmp(before, nodes, sizeof(nodes)) == 0);

            assert(dt_detach(&parent, &nodes[removed]) == &nodes[removed]);
            assert(nodes[removed].prev == NULL && nodes[removed].next == NULL);
            assert(parent.prev == &foreign && parent.next == &foreign);
            assert(foreign.prev == &foreign && foreign.next == &foreign);
            struct DetachNode *cursor = parent.child;
            struct DetachNode *previous = NULL;
            struct DetachNode *first = cursor;
            for (int index = 0; index < count; index++) {
                assert(nodes[index].child == NULL);
                if (index == removed) {
                    continue;
                }
                assert(cursor == &nodes[index]);
                if (previous != NULL) {
                    assert(cursor->prev == previous);
                }
                previous = cursor;
                cursor = cursor->next;
            }
            assert(cursor == NULL);
            if (first != NULL) {
                assert(first->prev == previous);
            }
        }
    }
    return 0;
}

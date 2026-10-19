#include "cjson_lookup_frame.h"
#include <assert.h>

int main(void)
{
    cJSON parent = {0};
    cJSON nodes[8] = {{0}};
    cJSON foreign = {0};
    assert(get_array_item((const cJSON *)0, 0) == 0);
    assert(get_array_item((const cJSON *)0, (size_t)-1) == 0);
    for (size_t length = 0; length <= 8; ++length) {
        parent.child = length ? &nodes[0] : 0;
        for (size_t at = 0; at < length; ++at) {
            nodes[at].next = at + 1 < length ? &nodes[at + 1] : 0;
            nodes[at].prev = at ? &nodes[at - 1] : &nodes[length - 1];
        }
        for (size_t index = 0; index <= length + 1; ++index) {
            cJSON *expected = index < length ? &nodes[index] : 0;
            cJSON *result = get_array_item(&parent, index);
            assert(result == expected);
            assert(result != &foreign);
        }
        assert(get_array_item(&parent, (size_t)-1) == 0);
    }
    /* Query results are existing object addresses, never fresh allocations. */
    parent.child = &nodes[2];
    nodes[2].next = &nodes[5];
    nodes[5].next = 0;
    assert(get_array_item(&parent, 0) == &nodes[2]);
    assert(get_array_item(&parent, 1) == &nodes[5]);
    assert(get_array_item(&parent, 2) == 0);
    return 0;
}

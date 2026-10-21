/* Observed finite live objects, not an arbitrary-heap or interference proof. */
#include "cJSON.h"
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
static void heap(cJSON *parent, cJSON **nodes, size_t n) {
    printf("{\"parent\":%llu,\"child\":%llu,\"nodes\":[",(unsigned long long)(uintptr_t)parent,(unsigned long long)(uintptr_t)parent->child);
    for(size_t i=0;i<n;i++) {
        if(i) printf(",");
        printf("{\"id\":%llu,\"next\":%llu,\"prev\":%llu}",(unsigned long long)(uintptr_t)nodes[i],(unsigned long long)(uintptr_t)nodes[i]->next,(unsigned long long)(uintptr_t)nodes[i]->prev);
    }
    printf("],\"allocations\":[%llu",(unsigned long long)(uintptr_t)parent);
    for(size_t i=0;i<n;i++) printf(",%llu",(unsigned long long)(uintptr_t)nodes[i]);
    printf("]}");
}
int main(void) {
    int first=1;
    printf("[");
    for(size_t n=0;n<=4;n++) for(size_t index=0;index<(n?n:1);index++) {
        cJSON *parent=cJSON_CreateArray(); cJSON *nodes[4]={0};
        if(!parent) return 10;
        for(size_t i=0;i<n;i++) { nodes[i]=cJSON_CreateNumber((double)i); if(!nodes[i] || !cJSON_AddItemToArray(parent,nodes[i])) return 11; }
        if(!first) printf(","); first=0;
        printf("{\"length\":%zu,\"index\":",n);
        if(n) printf("%zu",index); else printf("null");
        printf(",\"before\":"); heap(parent,nodes,n);
        cJSON *returned=cJSON_DetachItemViaPointer(parent,n?nodes[index]:NULL);
        printf(",\"after\":"); heap(parent,nodes,n);
        printf(",\"returned\":%llu}",(unsigned long long)(uintptr_t)returned);
        cJSON_Delete(returned); cJSON_Delete(parent);
    }
    printf("]\n"); return 0;
}

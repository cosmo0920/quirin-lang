/* Two real mapping lifetimes, same address, distinct QCore generations. */
#ifndef _GNU_SOURCE
#define _GNU_SOURCE
#endif
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/mman.h>
#include <unistd.h>
#ifndef MAP_FIXED_NOREPLACE
#error CT50 requires Linux MAP_FIXED_NOREPLACE
#endif
static uintptr_t ct50_address;
static void *ct50_live;
static size_t ct50_extent;
static unsigned ct50_epoch, ct50_released;
static void ct50_finish(void) {
    if (ct50_live || ct50_epoch!=2 || ct50_released!=2) _Exit(91);
    fputs("RUN_CLEAN 2\n",stderr);
}
static void *ct50_allocate(size_t size) {
    if (ct50_live || ct50_epoch>=2) abort();
    if (ct50_epoch==0) {
        long page=sysconf(_SC_PAGESIZE);
        if (page<=0 || atexit(ct50_finish)!=0) abort();
        ct50_extent=(size_t)page;
    }
    if (!size || size>ct50_extent) abort();
    int flags=MAP_PRIVATE|MAP_ANONYMOUS;
    void *requested=NULL;
    if (ct50_epoch==1) {
        flags|=MAP_FIXED_NOREPLACE;
        requested=(void *)ct50_address;
    }
    void *pointer=mmap(requested,ct50_extent,PROT_READ|PROT_WRITE,flags,-1,0);
    if (pointer==MAP_FAILED || pointer==NULL) abort();
    if (ct50_epoch==0) ct50_address=(uintptr_t)pointer;
    else if ((uintptr_t)pointer!=ct50_address) abort();
    ct50_live=pointer;
    ++ct50_epoch;
    fprintf(stderr,"MAP %u\n",ct50_epoch);
    if (ct50_epoch==2) fputs("REUSED_ADDRESS\n",stderr);
    return pointer;
}
static void ct50_release(void *pointer,uint64_t generation) {
    if (!pointer || pointer!=ct50_live || generation!=ct50_epoch || ct50_released+1!=ct50_epoch) abort();
    ct50_live=NULL;
    if (munmap(pointer,ct50_extent)!=0) abort();
    ++ct50_released;
    fprintf(stderr,"UNMAP %llu\n",(unsigned long long)generation);
}
#define QCORE_BUFFER_MALLOC ct50_allocate
#define QCORE_BUFFER_FREE(buffer) ct50_release((buffer),(buffer)->generation)

/* A real C library with **no QCore contract whatsoever**, linked against the
 * emitted translation unit for card `UNSAFE-LINKAGE`.
 *
 * This is the point of the fixture: nothing in `contract/qcore/` or in any
 * selected environment describes these functions, and nothing in this
 * repository ever will. The QCore module that calls them declares only their
 * ABI signatures, through the explicit `unsafe extern fn` surface, and its
 * certificate carries each of them by name as an unchecked-linkage premise.
 *
 * A run under ASan/UBSan/LSan therefore establishes exactly one thing: the
 * emitted program performed these calls across a real link boundary, in the
 * checked order, and left nothing behind. It establishes nothing whatever about
 * what these functions do — which is why the premise is printed rather than
 * discharged.
 *
 * `k_unchecked_side_effect` deliberately mutates a private global and writes to
 * a stream, so a reader can see that the derived maximal effect row is not
 * theatre: this callee really does do things the caller cannot see.
 */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

static int32_t k_hidden_state = 0;

/* A pure scalar function. Custody-free by construction: it is handed a value
 * and returns a value. */
int32_t k_unchecked_add(int32_t a, int32_t b)
{
    return a + b;
}

/* A `void` result. The caller binds nothing. */
void k_unchecked_side_effect(uint32_t tag)
{
    k_hidden_state += (int32_t) tag;
    fprintf(stderr, "k_unchecked_side_effect: hidden state now %d\n", k_hidden_state);
}

/* Allocates and frees behind the caller's back. The caller's ledger is
 * untouched — it was handed nothing and given nothing back — which is the whole
 * soundness argument for the scalar-only v1 surface. */
uint64_t k_unchecked_measure(uint64_t count)
{
    void *scratch = malloc((size_t) count);
    uint64_t answer = scratch == NULL ? 0u : count;
    free(scratch);
    return answer;
}

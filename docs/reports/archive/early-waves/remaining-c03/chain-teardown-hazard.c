/* C03 design evidence: the two concrete hazards of the *rejected* recursive
 * family (F2, heap-link chain teardown). This is a fixed native observation,
 * not a proof and not a claim about any upstream code.
 *
 *   argv[1] == "alias": an acyclic chain is torn down by a recursive `purge`
 *   whose only expressible contract today is `resource_released(head)`. That
 *   contract says nothing about the interior nodes, so a caller that also
 *   holds one is, per the abstraction, still their owner. Executing that
 *   caller is a heap-use-after-free.
 *
 *   argv[1] == "cycle": the same `purge` on a cyclic chain never reaches a
 *   release at all. The exit-state reading of any custody contract is then
 *   vacuous, which is why the acyclicity premise is load-bearing rather than
 *   decorative.
 *
 * Build and run (both runs are expected to fail; that is the evidence):
 *   clang -std=c11 -Wall -Wextra -Werror -g -O1 -fsanitize=address,undefined \
 *     -fno-omit-frame-pointer docs/reports/remaining-c03/chain-teardown-hazard.c \
 *     -o /tmp/qcore-c03-hazard
 *   /tmp/qcore-c03-hazard alias
 *   /tmp/qcore-c03-hazard cycle
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

struct node {
    struct node *next;
    int key;
};

static struct node *node_new(int key, struct node *next)
{
    struct node *n = malloc(sizeof(*n));
    if (!n) {
        abort();
    }
    n->key = key;
    n->next = next;
    return n;
}

/* The recursive teardown. Its true effect is "releases every node reachable
 * from `head` through `next`". The resource-contract vocabulary in
 * crates/qcore-schemas/src/contract.rs can name `head` and no other node. */
static void purge(struct node *head)
{
    if (!head) {
        return;
    }
    purge(head->next);
    free(head);
}

static int interior_alias(void)
{
    struct node *tail = node_new(2, NULL);
    struct node *head = node_new(1, tail);

    /* A caller that a `resource_released(head)` contract leaves believing it
     * still owns `tail`. */
    purge(head);
    printf("interior key = %d\n", tail->key); /* heap-use-after-free */
    free(tail);                               /* double free */
    return 0;
}

static int cyclic_chain(void)
{
    struct node *second = node_new(2, NULL);
    struct node *first = node_new(1, second);

    second->next = first;
    purge(first); /* never returns: no release is ever reached */
    return 0;
}

int main(int argc, char **argv)
{
    if (argc == 2 && strcmp(argv[1], "interior") == 0) {
        return interior_alias();
    }
    if (argc == 2 && strcmp(argv[1], "cyclic") == 0) {
        return cyclic_chain();
    }
    fprintf(stderr, "usage: %s interior|cyclic\n", argv[0]);
    return 2;
}

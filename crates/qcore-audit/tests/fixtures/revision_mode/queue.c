/* Card W45-CONTRACT-REVISION-MODE: a generic, held-out mutation fixture.
 * It is written for the revision-mode harness and derived from no production
 * source. `box_drop`, `box_keep` and `box_count` are audited bodies with
 * contracts of their own; the `user_*` functions rely on those contracts. */
struct item;
struct ticket;
struct item *item_new(void);
void item_free(struct item *item);
void item_touch(struct item *item);
void sink_keep(struct item *item);
struct ticket *ticket_take(void);
void ticket_return(struct ticket *ticket);

void box_drop(struct item *item)
{
    item_touch(item);
    item_free(item);
}

void box_keep(struct item *item)
{
    item_touch(item);
    sink_keep(item);
}

int box_count(int fail)
{
    struct ticket *ticket = ticket_take();
    if (!ticket) {
        return -1;
    }
    if (fail) {
        ticket_return(ticket);
        return -1;
    }
    ticket_return(ticket);
    return 0;
}

int box_two(void)
{
    struct ticket *first = ticket_take();
    struct ticket *second = ticket_take();
    if (first) {
        ticket_return(first);
    }
    if (second) {
        ticket_return(second);
    }
    return 0;
}

void user_drop(void)
{
    struct item *item = item_new();
    if (!item) {
        return;
    }
    box_drop(item);
}

void user_keep(void)
{
    struct item *item = item_new();
    if (!item) {
        return;
    }
    box_keep(item);
}

static volatile int done = 0;
static int payload;
void publish(void) {
    payload = 7;
    done = 1;
}
int consume(void) {
    int seen = done;
    if (seen != 1) return 0;
    return payload;
}

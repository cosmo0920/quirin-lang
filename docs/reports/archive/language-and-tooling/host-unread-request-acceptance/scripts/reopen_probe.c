#define _GNU_SOURCE
#include <fcntl.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/ioctl.h>
#include <sys/wait.h>
#include <unistd.h>

/* Does reopening a pipe's write end through /proc/self/fd give an independent
 * read end that sees the bytes left unread after the reader exited? */
int main(void) {
    signal(SIGPIPE, SIG_IGN);
    int p[2];
    if (pipe2(p, O_CLOEXEC) != 0) return 2;
    pid_t c = fork();
    if (c == 0) {
        dup2(p[0], 0);
        close(p[1]);
        char b[5];
        if (getenv("READ")) {
            ssize_t got = 0;
            while (got < 5) {
                ssize_t r = read(0, b + got, 5 - got);
                if (r <= 0) break;
                got += r;
            }
        }
        _exit(0);
    }
    close(p[0]);
    ssize_t w = write(p[1], "hello", 5);
    waitpid(c, 0, 0);
    char path[64];
    snprintf(path, sizeof path, "/proc/self/fd/%d", p[1]);
    int r = open(path, O_RDONLY | O_NONBLOCK | O_CLOEXEC);
    int n = -1;
    int rc = ioctl(r, FIONREAD, &n);
    close(p[1]);
    int n2 = -1;
    ioctl(r, FIONREAD, &n2);
    printf("write=%zd reopen_fd=%d fionread=%d rc=%d after_writer_close=%d\n", w, r, n, rc, n2);
    return 0;
}

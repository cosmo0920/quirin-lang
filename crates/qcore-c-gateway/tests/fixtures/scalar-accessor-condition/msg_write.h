// Verbatim msg_write from pinned Fluent Bit66910c10, lib/monkey/mk_server/mk_fifo.c.
#define _DEFAULT_SOURCE 1
#include <errno.h>
#include <stdint.h>
#include <stdio.h>
#include <unistd.h>
typedef int mk_fifo_channel_fd;

static int msg_write(mk_fifo_channel_fd fd, void *buf, size_t count)
{
    ssize_t bytes;
    size_t total = 0;

    do {
#ifdef _WIN32
        bytes = send((SOCKET) fd, (uint8_t *)buf + total, count - total, 0);
#else
        bytes = write(fd, (uint8_t *)buf + total, count - total);
#endif
        if (bytes == -1) {
            if (errno == EAGAIN) {
                /*
                 * This could happen, since this function goal is not to
                 * return until all data have been read, just sleep a little
                 * bit (0.05 seconds)
                 */

#ifdef _WIN32
                Sleep(5);
#else
                usleep(50000);
#endif
                continue;
            }
        }
        else if (bytes == 0) {
            /* Broken pipe ? */
            perror("write");
            return -1;
        }
        total += bytes;

    } while (total < count);

    return total;
}


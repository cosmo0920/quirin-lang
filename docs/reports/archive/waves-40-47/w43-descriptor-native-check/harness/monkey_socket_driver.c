/*
 * Monkey socket driver for W43-DESCRIPTOR-NATIVE-CHECK.
 *
 * The pinned Fluent Bit no longer reaches Monkey's server sockets from any
 * configuration (its HTTP server uses flb_downstream), so this driver calls
 * the actual upstream lib/monkey/mk_server/mk_socket.c functions directly,
 * compiled with that file's pinned include/define profile and linked with the
 * pinned static archives. The driver's own descriptors (the Unix listener it
 * provides for mk_socket_open) are released by the driver.
 *
 * Usage: monkey-socket-driver SCENARIO SCRATCH_DIR
 *   tcp          mk_socket_server, mk_socket_connect, mk_socket_accept,
 *                mk_socket_ip_str, then close every descriptor
 *   unix         mk_socket_open to a listening Unix socket, and to a missing path
 *   bind_in_use  a second mk_socket_server on a port that is already listening
 */
#include <monkey/mk_core.h>
#include <monkey/mk_server.h>
#include <monkey/mk_socket.h>

#include <arpa/inet.h>
#include <netinet/in.h>
#include <stdio.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <unistd.h>

static int bound_port(int fd)
{
    struct sockaddr_in address;
    socklen_t length = sizeof(address);
    if (getsockname(fd, (struct sockaddr *) &address, &length) != 0) {
        return -1;
    }
    return ntohs(address.sin_port);
}

static int tcp(struct mk_server *server)
{
    char ip[64];
    char *buffer = ip;
    unsigned long length = 0;
    int listener = mk_socket_server("0", "127.0.0.1", MK_FALSE, server);
    if (listener == -1) {
        return 1;
    }
    int port = bound_port(listener);
    int client = port > 0 ? mk_socket_connect("127.0.0.1", port, MK_FALSE) : -1;
    int remote = client >= 0 ? mk_socket_accept(listener) : -1;
    if (remote >= 0) {
        (void) mk_socket_ip_str(remote, &buffer, sizeof(ip), &length);
        close(remote);
    }
    if (client >= 0) {
        close(client);
    }
    /* A second accept on the (blocking-accept4 nonblocking-flag) listener with
       nothing pending is a natural accept failure. */
    mk_socket_set_nonblocking(listener);
    remote = mk_socket_accept(listener);
    if (remote >= 0) {
        close(remote);
    }
    close(listener);
    return 0;
}

static int unix_socket(const char *scratch)
{
    struct sockaddr_un address;
    char path[sizeof(address.sun_path)];
    snprintf(path, sizeof(path), "%s/mk.sock", scratch);
    int listener = socket(AF_UNIX, SOCK_STREAM, 0);
    if (listener < 0) {
        return 1;
    }
    memset(&address, 0, sizeof(address));
    address.sun_family = AF_UNIX;
    snprintf(address.sun_path, sizeof(address.sun_path), "%s", path);
    unlink(path);
    if (bind(listener, (struct sockaddr *) &address, sizeof(address)) != 0 || listen(listener, 4) != 0) {
        close(listener);
        return 1;
    }
    int fd = mk_socket_open(path, MK_FALSE);
    if (fd >= 0) {
        close(fd);
    }
    char missing[sizeof(address.sun_path)];
    snprintf(missing, sizeof(missing), "%s/missing.sock", scratch);
    fd = mk_socket_open(missing, MK_FALSE); /* natural connect failure */
    if (fd >= 0) {
        close(fd);
    }
    close(listener);
    unlink(path);
    return 0;
}

static int bind_in_use(struct mk_server *server)
{
    char port[16];
    int listener = mk_socket_server("0", "127.0.0.1", MK_FALSE, server);
    if (listener == -1) {
        return 1;
    }
    snprintf(port, sizeof(port), "%d", bound_port(listener));
    int second = mk_socket_server(port, "127.0.0.1", MK_FALSE, server); /* EADDRINUSE */
    if (second >= 0) {
        close(second);
    }
    close(listener);
    return 0;
}

int main(int argc, char **argv)
{
    struct mk_server server;
    if (argc != 3) {
        return 2;
    }
    memset(&server, 0, sizeof(server));
    if (strcmp(argv[1], "tcp") == 0) {
        return tcp(&server);
    }
    if (strcmp(argv[1], "unix") == 0) {
        return unix_socket(argv[2]);
    }
    if (strcmp(argv[1], "bind_in_use") == 0) {
        return bind_in_use(&server);
    }
    return 2;
}

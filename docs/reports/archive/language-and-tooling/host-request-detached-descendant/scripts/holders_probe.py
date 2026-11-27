#!/usr/bin/env python3
"""HOST-REQUEST-READ-BY-DETACHED-DESCENDANT: the holder check on a Linux pipe,
measured directly before any host code was written.

The host keeps a probe (a read end it never reads from). To ask whether any
*other* process still holds a read end, it opens a write end through
/proc/self/fd/<probe>, closes the probe, and polls the write end: POLLERR is set
exactly when the pipe has no reader left. FIONREAD on that write end still
counts the bytes nobody read. If a reader remains, the probe is reopened
through the write end and the write end is closed again, so a reader waiting
for end of file is not held up.

Prints one line per case; exits non-zero if any expectation fails."""
import fcntl
import os
import select
import struct
import sys
import termios
import time

FAILED = []


def unread(fd):
    return struct.unpack("i", fcntl.ioctl(fd, termios.FIONREAD, b"\0\0\0\0"))[0]


def holders(probe):
    """Returns (other_readers, unread, new_fd): new_fd is the probe again when
    readers remain, else the write end that keeps the pipe alive."""
    writer = os.open(f"/proc/self/fd/{probe}", os.O_WRONLY | os.O_NONBLOCK | os.O_CLOEXEC)
    os.close(probe)
    poller = select.poll()
    poller.register(writer, 0)
    events = dict(poller.poll(0)).get(writer, 0)
    if events & select.POLLERR:
        return False, unread(writer), writer
    probe = os.open(f"/proc/self/fd/{writer}", os.O_RDONLY | os.O_NONBLOCK | os.O_CLOEXEC)
    os.close(writer)
    return True, None, probe


def case(name, child_body, frame=b"hello", settle=0.05):
    r, w = os.pipe()
    pid = os.fork()
    if pid == 0:
        os.close(w)
        try:
            child_body(r)
        finally:
            os._exit(0)
    os.close(r)
    os.write(w, frame)
    probe = os.open(f"/proc/self/fd/{w}", os.O_RDONLY | os.O_NONBLOCK | os.O_CLOEXEC)
    os.close(w)
    time.sleep(settle)
    result = holders(probe)
    held, count, fd = result
    print(f"{name}: other readers {held}, unread {count}")
    os.close(fd)
    os.kill(pid, 9) if held else None
    os.waitpid(pid, 0)
    return held, count


def expect(name, got, want):
    if got != want:
        FAILED.append(name)
        print(f"  MISMATCH {name}: {got} != {want}")


def hold(r):
    time.sleep(5)


def read_all_exit(r):
    while os.read(r, 4096):
        pass


def read_nothing_exit(r):
    pass


def read_all_then_hold(r):
    while os.read(r, 4096):
        pass
    time.sleep(5)


expect("holder alive, nothing read", case("holder alive, nothing read", hold), (True, None))
expect("reader read all and exited", case("reader read all and exited", read_all_exit), (False, 0))
expect("reader read nothing and exited", case("reader read nothing and exited", read_nothing_exit), (False, 5))
expect("reader read all and holds", case("reader read all and holds", read_all_then_hold), (True, None))

# A reader blocked waiting for end of file still gets it: the probe dance
# leaves no writer behind.
r, w = os.pipe()
pid = os.fork()
if pid == 0:
    os.close(w)
    data = b""
    while True:
        chunk = os.read(r, 4096)
        if not chunk:
            break
        data += chunk
    os._exit(0 if data == b"hello" else 3)
os.close(r)
probe = os.open(f"/proc/self/fd/{w}", os.O_RDONLY | os.O_NONBLOCK | os.O_CLOEXEC)
time.sleep(0.05)
held, _, probe = holders(probe)  # the reader is blocked in read(): a holder
os.write(w, b"hello")
os.close(w)
deadline = time.monotonic() + 5
while held and time.monotonic() < deadline:
    time.sleep(0.001)
    held, count, probe = holders(probe)
_, status = os.waitpid(pid, 0)
print(f"reader waiting for end of file: released {not held}, unread {count}, exit {os.waitstatus_to_exitcode(status)}")
expect("reader waiting for end of file", (held, count, os.waitstatus_to_exitcode(status)), (False, 0, 0))
os.close(probe)

sys.exit(1 if FAILED else 0)

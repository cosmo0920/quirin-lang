// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Whether a process provider consumed its whole request (card
//! `HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING`).
//!
//! Before this card the host refused a successful exit only when its own
//! request write met a broken pipe (`VERIFIER-HOST-CLOSED-INPUT-DEADLINE`). A
//! provider that never read its request was therefore refused when it exited
//! before the host's write, and accepted when the frame fit the pipe buffer and
//! the write landed first: the verdict depended on scheduling.
//!
//! The host now decides from **consumption**, observed in the request pipe:
//!
//! * the host creates the pipe and holds no read end while it writes, so a
//!   broken pipe still means that no reader is left;
//! * once the whole frame is written, and before the write end is closed, it
//!   holds a **probe**: a read end it never reads from. On Linux the probe is a
//!   second, independent open of the same pipe through
//!   `/proc/self/fd/<write end>`; elsewhere `/dev/fd` is a `dup`, so the probe is
//!   a duplicate of the read end kept from the spawn (and a broken pipe cannot
//!   occur there). Holding a read end never delays the provider's end of file,
//!   which depends on writers only;
//! * after the direct child has exited and both output pipes are at end of
//!   file, `FIONREAD` on the probe gives the bytes left unread.
//!
//! [`classify`] is the decision. With `consumed ≤ delivered ≤ length`, its test
//! `delivered = length ∧ unread = 0` holds exactly when `consumed = length`,
//! for every order of the host's write and the provider's exit. That is the
//! Lean claim (`QCore.RequestConsumption`).
//!
//! **Only the last holder's count decides** (card
//! `HOST-REQUEST-READ-BY-DETACHED-DESCENDANT`). A descendant that inherited the
//! request pipe as its stdin, with both output pipes closed, can still read
//! after the direct child has exited and the outputs are at end of file, so the
//! count at that moment is not final. The host therefore decides only once it
//! is the pipe's last holder ([`holders`]):
//!
//! * while the host still writes, it holds no read end, so a write that does
//!   not break proves another reader; the decision waits for the write to
//!   finish or break;
//! * once the frame is written, the host opens a write end through
//!   `/proc/self/fd/<probe>`, closes the probe and polls that write end:
//!   `POLLERR` is set exactly when no reader is left. Then no reader can appear
//!   any more, and `FIONREAD` on the write end is the final unread count. If a
//!   reader remains, the probe is reopened through the write end and the write
//!   end closed again, so a reader waiting for end of file still gets it.
//!
//! The decision waits, bounded by the declared deadline; a pipe still held at
//! the deadline is refused as [`ProviderFailure::RequestHeldByDescendant`].
//! Off Linux the other holders cannot be observed (`/dev/fd` is a `dup`, so the
//! probe is the read end itself). There the host decides exactly as before
//! this card, on the bytes left unread at that moment ([`Holders::CountOnly`]),
//! by the user's decision ("keep previous macOS"); the detached-descendant
//! shape stays timing-dependent there
//! (`HOST-REQUEST-HOLDERS-OFF-LINUX-UNOBSERVABLE`).

use std::io;
use std::os::fd::{AsRawFd, OwnedFd};

use crate::ProviderFailure;

/// The pipe the request travels through: the read end for the child's stdin,
/// the host's write end, and — off Linux only — the probe, kept from the
/// spawn.
pub(crate) struct RequestPipe {
    pub(crate) child_stdin: std::io::PipeReader,
    pub(crate) writer: std::io::PipeWriter,
    pub(crate) held_probe: Option<io::Result<OwnedFd>>,
}

pub(crate) fn request_pipe() -> io::Result<RequestPipe> {
    let (child_stdin, writer) = std::io::pipe()?;
    #[cfg(target_os = "linux")]
    let held_probe = None;
    #[cfg(not(target_os = "linux"))]
    let held_probe = Some(child_stdin.try_clone().map(OwnedFd::from));
    Ok(RequestPipe {
        child_stdin,
        writer,
        held_probe,
    })
}

/// The probe, taken once the whole frame is in the pipe and before `writer` is
/// closed. On Linux this opens a second read end on the same pipe; `O_NONBLOCK`
/// keeps the open from waiting and is a flag of this new open file
/// description only, never of the child's stdin.
#[cfg(target_os = "linux")]
pub(crate) fn probe_after_full_write(writer: &std::io::PipeWriter) -> io::Result<OwnedFd> {
    use std::os::unix::fs::OpenOptionsExt;
    std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(format!("/proc/self/fd/{}", writer.as_raw_fd()))
        .map(OwnedFd::from)
}

/// Bytes in the pipe that no reader has taken. `FIONREAD` changes no flag and
/// reads nothing, so it cannot itself consume the request. On a pipe it
/// counts the same bytes through either end.
pub(crate) fn unread_bytes(probe: &OwnedFd) -> io::Result<usize> {
    let mut unread: libc::c_int = 0;
    // SAFETY: `probe` is an open descriptor owned by the caller and `unread`
    // is a valid `int` for the kernel to write.
    if unsafe { libc::ioctl(probe.as_raw_fd(), libc::FIONREAD, &mut unread) } == -1 {
        return Err(io::Error::last_os_error());
    }
    usize::try_from(unread).map_err(|_| io::Error::other("negative FIONREAD count"))
}

/// Who else holds the request pipe's read end at the decision.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Holders {
    /// No process but the host holds a read end, so no byte can be read any
    /// more; `unread` bytes were never read.
    Released { unread: usize },
    /// Another process still holds a read end and could still read.
    Held,
    /// Off Linux only: whether another process holds a read end cannot be
    /// observed, so the count at the decision is decided on as it stands, as
    /// before card `HOST-REQUEST-READ-BY-DETACHED-DESCENDANT`. A descendant
    /// that still reads after this moment is not credited.
    #[cfg_attr(target_os = "linux", allow(dead_code))]
    CountOnly { unread: usize },
    /// Neither could be established; the text says why.
    Unobservable(String),
}

/// Whether the host is the request pipe's last holder, for a direct child that
/// exited successfully with both outputs at end of file.
///
/// `still_writing`: the host has not written the whole frame and its write end
/// is open. On Linux it holds no read end while writing, and the caller's last
/// write did not break, so another reader exists.
///
/// `probe` is the host's read end once the frame is written. A check that
/// finds the pipe held leaves the (reopened) probe there for the next check; a
/// check that finds it released, or fails, consumes it.
#[cfg(target_os = "linux")]
pub(crate) fn holders(still_writing: bool, probe: &mut Option<io::Result<OwnedFd>>) -> Holders {
    if still_writing {
        return Holders::Held;
    }
    match probe.take() {
        None => Holders::Unobservable("no probe was taken".into()),
        Some(Err(error)) => Holders::Unobservable(error.to_string()),
        Some(Ok(read_end)) => match check_holders(read_end) {
            Ok(HolderCheck::Released { unread }) => Holders::Released { unread },
            Ok(HolderCheck::Held(read_end)) => {
                *probe = Some(Ok(read_end));
                Holders::Held
            }
            Err(error) => Holders::Unobservable(error.to_string()),
        },
    }
}

/// Off Linux `/dev/fd` is a `dup`: the host cannot open a second, independent
/// end of the pipe, so it cannot tell its own read end (the probe kept from the
/// spawn) from another process's. It decides as it did before this card: an
/// undelivered remainder is unconsumed (`classify` sees `delivered < length`),
/// and otherwise `FIONREAD` on the probe is the count. A probe that is missing
/// or cannot be read is still unobservable, and refuses.
#[cfg(not(target_os = "linux"))]
pub(crate) fn holders(still_writing: bool, probe: &mut Option<io::Result<OwnedFd>>) -> Holders {
    if still_writing {
        return Holders::CountOnly { unread: 0 };
    }
    match probe.take() {
        None => Holders::Unobservable("no probe was taken".into()),
        Some(Err(error)) => Holders::Unobservable(error.to_string()),
        Some(Ok(read_end)) => match unread_bytes(&read_end) {
            Ok(unread) => Holders::CountOnly { unread },
            Err(error) => Holders::Unobservable(error.to_string()),
        },
    }
}

/// One holder check's result.
#[cfg(target_os = "linux")]
enum HolderCheck {
    Released {
        unread: usize,
    },
    /// The probe, reopened, for the next check.
    Held(OwnedFd),
}

/// Asks whether any process other than the host holds a read end of the pipe
/// `read_end` belongs to.
///
/// The host's own read end would count as a reader, so it is traded for a
/// write end on the same pipe (opened through `/proc/self/fd`, which for a pipe
/// never waits for a partner) and closed. `poll` on a pipe's write end reports
/// `POLLERR` exactly when the pipe has no reader. The write end exists only
/// for the length of this check and nothing is written through it.
#[cfg(target_os = "linux")]
fn check_holders(read_end: OwnedFd) -> io::Result<HolderCheck> {
    let write_end = reopen(&read_end, libc::O_WRONLY)?;
    drop(read_end);
    let mut poll = libc::pollfd {
        fd: write_end.as_raw_fd(),
        events: 0,
        revents: 0,
    };
    loop {
        // SAFETY: `poll` points at one valid `pollfd` whose descriptor is
        // owned by `write_end` for the whole call; a zero timeout never
        // blocks.
        if unsafe { libc::poll(&mut poll, 1, 0) } >= 0 {
            break;
        }
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::Interrupted {
            return Err(error);
        }
    }
    if poll.revents & libc::POLLERR != 0 {
        // No reader is left, and none can appear: the count is final.
        return Ok(HolderCheck::Released {
            unread: unread_bytes(&write_end)?,
        });
    }
    let read_end = reopen(&write_end, libc::O_RDONLY)?;
    drop(write_end);
    Ok(HolderCheck::Held(read_end))
}

/// A second, independent open of the pipe `end` belongs to, for reading or for
/// writing. `O_NONBLOCK` belongs to the new open file description only.
#[cfg(target_os = "linux")]
fn reopen(end: &impl AsRawFd, mode: libc::c_int) -> io::Result<OwnedFd> {
    use std::os::unix::fs::OpenOptionsExt;
    std::fs::OpenOptions::new()
        .read(mode == libc::O_RDONLY)
        .write(mode == libc::O_WRONLY)
        .custom_flags(libc::O_NONBLOCK)
        .open(format!("/proc/self/fd/{}", end.as_raw_fd()))
        .map(OwnedFd::from)
}

/// The consumption decision for a provider that exited successfully.
///
/// * `reader_lost`: a request write met a broken pipe (or wrote nothing): no
///   reader was left before the frame was delivered, and none can appear.
/// * `delivered` of `length` bytes reached the pipe.
/// * `holders`: who else holds the read end ([`holders`]).
///
/// Refuses with [`ProviderFailure::RequestUnconsumed`] unless the whole frame
/// was delivered and none of it is left once the host is the last holder; with
/// [`ProviderFailure::RequestHeldByDescendant`] while another process holds the
/// pipe (the caller passes [`Holders::Held`] only at the deadline); with `Io`
/// when the holders or the count cannot be observed (fail-closed).
pub(crate) fn classify(
    length: usize,
    delivered: usize,
    reader_lost: bool,
    holders: Holders,
) -> Result<(), ProviderFailure> {
    if reader_lost {
        return Err(ProviderFailure::RequestUnconsumed(format!(
            "the request pipe had no reader left after {delivered} of {length} bytes"
        )));
    }
    let unread = match holders {
        Holders::Held => return Err(held_at_deadline(length, delivered)),
        Holders::Unobservable(reason) => {
            return Err(ProviderFailure::Io(format!(
                "request consumption is unobservable: {reason}"
            )));
        }
        Holders::Released { unread } | Holders::CountOnly { unread } => unread,
    };
    if delivered < length {
        return Err(ProviderFailure::RequestUnconsumed(format!(
            "the provider exited after {delivered} of {length} request bytes were delivered"
        )));
    }
    if unread > 0 {
        return Err(ProviderFailure::RequestUnconsumed(format!(
            "the provider exited with {unread} of {length} request bytes unread"
        )));
    }
    Ok(())
}

/// The refusal for a provider that exited successfully with both outputs
/// closed while another process still held its request pipe at the deadline.
pub(crate) fn held_at_deadline(length: usize, delivered: usize) -> ProviderFailure {
    ProviderFailure::RequestHeldByDescendant(format!(
        "a process other than the provider still held the request pipe at the deadline, \
         after {delivered} of {length} bytes were delivered"
    ))
}

/// The refusal for a live provider whose request pipe broke, at the deadline.
pub(crate) fn reader_lost_at_deadline(length: usize, delivered: usize) -> ProviderFailure {
    ProviderFailure::RequestUnconsumed(format!(
        "the request pipe had no reader left after {delivered} of {length} bytes, \
         and the provider was still running at its deadline"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unconsumed(result: Result<(), ProviderFailure>) -> bool {
        matches!(result, Err(ProviderFailure::RequestUnconsumed(_)))
    }

    fn released(unread: usize) -> Holders {
        Holders::Released { unread }
    }

    #[test]
    fn only_a_wholly_delivered_and_wholly_read_frame_is_consumed() {
        assert_eq!(classify(10, 10, false, released(0)), Ok(()));
        assert!(unconsumed(classify(10, 10, false, released(1))));
        assert!(unconsumed(classify(10, 10, false, released(10))));
        assert!(unconsumed(classify(10, 9, false, released(0))));
        // A broken pipe is the same refusal as unread bytes, whatever else.
        assert!(unconsumed(classify(10, 10, true, released(0))));
        assert!(unconsumed(classify(10, 4, true, Holders::Held)));
        assert!(unconsumed(classify(
            10,
            4,
            true,
            Holders::Unobservable("probe".into())
        )));
    }

    #[test]
    fn a_request_pipe_held_by_another_process_is_never_accepted() {
        for delivered in [0, 4, 10] {
            assert!(matches!(
                classify(10, delivered, false, Holders::Held),
                Err(ProviderFailure::RequestHeldByDescendant(_))
            ));
        }
    }

    /// Off Linux the count at the decision is decided on as it stands, exactly
    /// as before the card: an undelivered remainder or unread bytes refuse.
    #[test]
    fn a_count_only_decision_is_the_decision_before_the_card() {
        let count_only = |unread| Holders::CountOnly { unread };
        assert_eq!(classify(10, 10, false, count_only(0)), Ok(()));
        assert!(unconsumed(classify(10, 10, false, count_only(3))));
        assert!(unconsumed(classify(10, 4, false, count_only(0))));
        assert!(unconsumed(classify(10, 4, true, count_only(0))));
    }

    #[test]
    fn an_unobservable_consumption_refuses_and_never_accepts() {
        for delivered in [0, 10] {
            assert!(matches!(
                classify(10, delivered, false, Holders::Unobservable("probe".into())),
                Err(ProviderFailure::Io(_))
            ));
        }
    }

    /// The holder check sees a read end the host does not own until it is
    /// closed, leaves no writer behind (a reader still meets end of file), and
    /// then reports the final unread count.
    #[cfg(target_os = "linux")]
    #[test]
    fn the_holder_check_sees_another_reader_until_it_is_gone() {
        use std::io::{Read, Write};
        let RequestPipe {
            mut child_stdin,
            mut writer,
            ..
        } = request_pipe().unwrap();
        writer.write_all(b"hello").unwrap();
        let mut probe = Some(probe_after_full_write(&writer));
        // While the host still writes, the unbroken write proves the reader.
        assert_eq!(holders(true, &mut probe), Holders::Held);
        drop(writer);
        // `child_stdin` stands for a descendant's read end.
        assert_eq!(holders(false, &mut probe), Holders::Held);
        assert!(matches!(probe, Some(Ok(_))));
        let mut two = [0; 2];
        child_stdin.read_exact(&mut two).unwrap();
        assert_eq!(holders(false, &mut probe), Holders::Held);
        // No writer is left after a check: a nonblocking read meets end of
        // file rather than `WouldBlock`.
        let raw = child_stdin.as_raw_fd();
        // SAFETY: `raw` is owned by `child_stdin`, alive for both calls.
        let flags = unsafe { libc::fcntl(raw, libc::F_GETFL) };
        assert_ne!(flags, -1);
        assert_ne!(
            unsafe { libc::fcntl(raw, libc::F_SETFL, flags | libc::O_NONBLOCK) },
            -1
        );
        let mut rest = [0; 8];
        assert_eq!(child_stdin.read(&mut rest).unwrap(), 3);
        assert_eq!(&rest[..3], b"llo");
        assert_eq!(child_stdin.read(&mut rest).unwrap(), 0);
        assert_eq!(holders(false, &mut probe), Holders::Held);
        drop(child_stdin);
        assert_eq!(holders(false, &mut probe), released(0));
        assert!(probe.is_none());
    }

    /// Once the other reader is gone, the count is the bytes it never read.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_released_pipe_reports_what_no_reader_took() {
        use std::io::Write;
        let RequestPipe {
            child_stdin,
            mut writer,
            ..
        } = request_pipe().unwrap();
        writer.write_all(b"hello").unwrap();
        let mut probe = Some(probe_after_full_write(&writer));
        drop(writer);
        drop(child_stdin);
        assert_eq!(holders(false, &mut probe), released(5));
        // Without a probe, nothing is observable, and nothing is accepted.
        assert!(matches!(
            holders(false, &mut probe),
            Holders::Unobservable(_)
        ));
    }

    /// The probe sees the bytes a reader left behind, also after the reader
    /// is gone and the host's write end is closed, and sees none once they
    /// are read.
    #[test]
    fn the_probe_counts_what_no_reader_took() {
        use std::io::{Read, Write};
        let pipe = request_pipe().unwrap();
        let RequestPipe {
            mut child_stdin,
            mut writer,
            held_probe,
        } = pipe;
        writer.write_all(b"hello").unwrap();
        #[cfg(target_os = "linux")]
        let probe = {
            assert!(held_probe.is_none());
            probe_after_full_write(&writer).unwrap()
        };
        #[cfg(not(target_os = "linux"))]
        let probe = held_probe.unwrap().unwrap();
        drop(writer);
        assert_eq!(unread_bytes(&probe).unwrap(), 5);
        let mut two = [0; 2];
        child_stdin.read_exact(&mut two).unwrap();
        assert_eq!(unread_bytes(&probe).unwrap(), 3);
        let mut rest = Vec::new();
        child_stdin.read_to_end(&mut rest).unwrap();
        assert_eq!(rest, b"llo");
        assert_eq!(unread_bytes(&probe).unwrap(), 0);
        drop(child_stdin);
        assert_eq!(unread_bytes(&probe).unwrap(), 0);
    }

    /// On Linux the host holds no read end while writing, so a pipe whose
    /// only reader is gone still breaks the write.
    #[cfg(target_os = "linux")]
    #[test]
    fn without_a_reader_the_request_write_still_breaks() {
        use std::io::Write;
        let RequestPipe {
            child_stdin,
            mut writer,
            held_probe,
        } = request_pipe().unwrap();
        assert!(held_probe.is_none());
        drop(child_stdin);
        let error = writer.write(b"x").unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
    }
}

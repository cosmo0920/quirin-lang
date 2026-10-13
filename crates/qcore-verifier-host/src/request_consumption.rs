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
//! The judgement is taken at that one moment. A descendant that keeps the
//! request pipe open after the direct child exits, with both output pipes
//! closed, can still read after it; its later reads are not credited.

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
/// reads nothing, so it cannot itself consume the request.
pub(crate) fn unread_bytes(probe: &OwnedFd) -> io::Result<usize> {
    let mut unread: libc::c_int = 0;
    // SAFETY: `probe` is an open descriptor owned by the caller and `unread`
    // is a valid `int` for the kernel to write.
    if unsafe { libc::ioctl(probe.as_raw_fd(), libc::FIONREAD, &mut unread) } == -1 {
        return Err(io::Error::last_os_error());
    }
    usize::try_from(unread).map_err(|_| io::Error::other("negative FIONREAD count"))
}

/// The consumption decision for a provider that exited successfully.
///
/// * `reader_lost`: a request write met a broken pipe (or wrote nothing):
///   no reader was left before the frame was delivered.
/// * `delivered` of `length` bytes reached the pipe.
/// * `unread`: the probe's `FIONREAD`, or why there is none.
///
/// Refuses with [`ProviderFailure::RequestUnconsumed`] unless the whole frame
/// was delivered and none of it is left; refuses with `Io` when the probe could
/// not be taken or read (consumption unobservable, fail-closed).
pub(crate) fn classify(
    length: usize,
    delivered: usize,
    reader_lost: bool,
    unread: Option<io::Result<usize>>,
) -> Result<(), ProviderFailure> {
    if reader_lost {
        return Err(ProviderFailure::RequestUnconsumed(format!(
            "the request pipe had no reader left after {delivered} of {length} bytes"
        )));
    }
    if delivered < length {
        return Err(ProviderFailure::RequestUnconsumed(format!(
            "the provider exited after {delivered} of {length} request bytes were delivered"
        )));
    }
    match unread {
        Some(Ok(0)) => Ok(()),
        Some(Ok(unread)) => Err(ProviderFailure::RequestUnconsumed(format!(
            "the provider exited with {unread} of {length} request bytes unread"
        ))),
        Some(Err(error)) => Err(ProviderFailure::Io(format!(
            "request consumption is unobservable: {error}"
        ))),
        None => Err(ProviderFailure::Io(
            "request consumption is unobservable: no probe was taken".into(),
        )),
    }
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

    #[test]
    fn only_a_wholly_delivered_and_wholly_read_frame_is_consumed() {
        assert_eq!(classify(10, 10, false, Some(Ok(0))), Ok(()));
        assert!(unconsumed(classify(10, 10, false, Some(Ok(1)))));
        assert!(unconsumed(classify(10, 10, false, Some(Ok(10)))));
        assert!(unconsumed(classify(10, 9, false, Some(Ok(0)))));
        assert!(unconsumed(classify(10, 0, false, None)));
        // A broken pipe is the same refusal as unread bytes, whatever else.
        assert!(unconsumed(classify(10, 10, true, Some(Ok(0)))));
        assert!(unconsumed(classify(10, 4, true, None)));
    }

    #[test]
    fn an_unobservable_consumption_refuses_and_never_accepts() {
        assert!(matches!(
            classify(10, 10, false, None),
            Err(ProviderFailure::Io(_))
        ));
        assert!(matches!(
            classify(10, 10, false, Some(Err(io::Error::other("probe")))),
            Err(ProviderFailure::Io(_))
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

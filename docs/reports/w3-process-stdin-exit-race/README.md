# Process stdin / direct-child exit race (2026-09-07)

The integrated host suite intermittently reported `Io("Broken pipe")` where
VP04 expected a timeout for an exited provider whose descendants still held
stdout/stderr. This was an existing transport scheduling race exposed during
the combined gate, not a changed verdict or an H6/profile effect.

## Reproduction and correction

The transport attempted the stdin write before observing direct-child status.
A shell can exit and leave a background process holding output pipes, while
all stdin readers have already closed. If the parent wrote first, the request
fit in the pipe and the host waited for output until Timeout. If the shell
exited first, the write returned EPIPE and the host immediately returned Io.
Both failed closed, but the latter skipped the intended output-lifecycle bound.

The new host regression sends a request larger than a pipe buffer to a process
that explicitly closes stdin, starts a descendant with inherited output and
exits. On baseline `619129d`, this reliably returns the unwanted BrokenPipe Io;
[baseline-regression.log](baseline-regression.log) records that expected negative
control. The original assertion is unchanged.

The pump now records a closed-input failure and stops sending, while continuing
to drain output and poll the direct child under the original deadline:

- An exited child with outstanding output pipes returns Timeout.
- A still-running child with closed stdin returns the recorded Io by the same
  deadline; it is terminated along with ordinary inherited descendants.
- If output completes first, the recorded input failure prevents decoding a
  response. A syntactically valid response cannot erase a partially transmitted
  request.

No deadline, payload/stderr cap or process-group cleanup rule is relaxed. Other
I/O errors retain their immediate failure behavior. The input failure may now
be reported at the configured deadline rather than immediately so that the
host can distinguish direct-child exit and descendant-held pipes. Provider
failures still become Unknown without a certificate.

## Validation

The adversarial host test covers all three states above and checks descendant
markers remain absent after their scheduled write time. The original
VP04/VP07/VP08 test is repeated twelve times to exercise the original scheduling
race, followed by the full standing gate. Logs and final results are adjacent.
The baseline negative control is deliberately failing; it is not part of the
passing fixed-state gate.

This changes transport failure classification and preserves all semantic
verdict rules; existing Lean definitions therefore remain the corresponding
reference and the standing Lean build is rerun.

Final fixed-state results: **510 workspace tests passed**, 4 standalone host
tests passed, all twelve stress repetitions passed, and formatting, clippy, both
reevaluation probes and Lean build returned exit 0. See
[gate-results.json](gate-results.json) and [stress.json](stress.json).

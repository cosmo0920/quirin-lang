//! Ordered concurrent execution of independent per-unit audit work (card
//! **W44-AUDIT-PARALLEL-UNITS**).
//!
//! The corpus audit's per-translation-unit work is independent: a unit's
//! analysis reads only its own source, its compile command and inputs computed
//! before the loop. What is *not* independent is publication — the report's
//! unit list, the sidecars' result lists, the counts and every line of stdout
//! are sequences in unit order. This module therefore separates the two:
//! [`run_ordered`] computes tasks on up to `jobs` threads and hands every
//! result to one consumer **strictly in task order**, on the calling thread.
//! The consumer is where results are folded and printed, so what is published
//! is a function of the task list alone, never of scheduling.
//!
//! Errors keep sequential semantics by construction: the consumer sees results
//! in task order and stops at the first one it rejects, so the error returned
//! is the first in task order, and nothing after it is ever consumed. Work
//! already running on other threads is finished and discarded, and no new task
//! starts once the consumer has stopped.
//!
//! `jobs == 1` does not start a thread: each task is computed immediately
//! before it is consumed, which is exactly the sequential loop.
use std::any::Any;
use std::num::NonZeroUsize;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, Mutex, PoisonError};

/// The environment variable that selects the job count when no command-line
/// flag does.
pub const JOBS_VARIABLE: &str = "QCORE_AUDIT_JOBS";

/// The default job count's ceiling. The default is the host's available
/// parallelism, capped here because every job holds one unit's analysis state
/// and runs its own Clang subprocess; the measured cost is in
/// `docs/reports/w44-audit-parallel-units/README.md`. An explicit `--jobs` or
/// [`JOBS_VARIABLE`] is not capped.
pub const DEFAULT_JOBS_CAP: usize = 16;

/// Resolve the job count: an explicit flag value first, then
/// [`JOBS_VARIABLE`], then the capped host parallelism.
///
/// An explicit value that is not a positive integer is an error, never a
/// silent fallback to some other count.
pub fn resolve_jobs(flag: Option<&str>) -> Result<usize, String> {
    let environment = std::env::var_os(JOBS_VARIABLE);
    resolve_jobs_from(
        flag,
        environment.as_ref().map(|value| value.to_str()),
        std::thread::available_parallelism().ok(),
    )
}

fn resolve_jobs_from(
    flag: Option<&str>,
    environment: Option<Option<&str>>,
    available: Option<NonZeroUsize>,
) -> Result<usize, String> {
    let parse = |source: &str, text: &str| {
        text.trim()
            .parse::<usize>()
            .ok()
            .filter(|jobs| *jobs > 0)
            .ok_or_else(|| format!("{source} must be a positive integer job count, not `{text}`"))
    };
    if let Some(text) = flag {
        return parse("--jobs", text);
    }
    match environment {
        Some(Some(text)) => parse(JOBS_VARIABLE, text),
        Some(None) => Err(format!("{JOBS_VARIABLE} is not valid UTF-8")),
        None => Ok(available.map_or(1, NonZeroUsize::get).min(DEFAULT_JOBS_CAP)),
    }
}

/// The job count an audit actually runs with: `requested`, except that any
/// active census stream forces one job.
///
/// Census streams are process-global and record in analysis order, and two of
/// them number records from process-global counters
/// ([`qcore_c_gateway::active_census_sinks`]). Running units one at a time
/// while one is being written keeps every census stream byte-identical to a
/// sequential run's. Nothing is printed about it: a census run's own console
/// output must not depend on the requested job count either.
pub fn effective_jobs(requested: usize) -> usize {
    if qcore_c_gateway::active_census_sinks().is_empty() {
        requested
    } else {
        1
    }
}

/// A dispatch order for [`run_ordered`] that starts the heaviest tasks first
/// (longest-processing-time-first), ties in task order.
///
/// On the pinned corpus a unit's source size ranks its analysis time closely
/// (Spearman 0.88 over the 185 units, and the largest file is the slowest
/// unit), so the audit weighs units by source bytes. A weight only moves wall
/// time: consumption order, and therefore every output, is independent of it.
pub fn heaviest_first(weights: &[u64]) -> Vec<usize> {
    let mut order = (0..weights.len()).collect::<Vec<_>>();
    order.sort_by(|left, right| weights[*right].cmp(&weights[*left]).then(left.cmp(right)));
    order
}

enum Slot<R> {
    Pending,
    Done(R),
    Panicked(Box<dyn Any + Send>),
    Taken,
}

/// Compute `work` over `tasks` on up to `jobs` threads and pass every result to
/// `consume` in task order, stopping at the first error `consume` returns.
///
/// `dispatch` is the order in which tasks are *started* when `jobs > 1`: a
/// permutation of `0..tasks.len()` (for example, longest first). It never
/// changes the order of consumption. `None` starts tasks in task order. It is
/// ignored when `jobs == 1`, which computes each task just before consuming
/// it.
///
/// A panic in `work` is re-raised on the calling thread when the consumer
/// reaches that task, after every earlier result has been consumed — the point
/// at which a sequential loop would have panicked.
pub fn run_ordered<T, R, E>(
    tasks: Vec<T>,
    jobs: usize,
    dispatch: Option<Vec<usize>>,
    work: impl Fn(T) -> R + Sync,
    mut consume: impl FnMut(R) -> Result<(), E>,
) -> Result<(), E>
where
    T: Send,
    R: Send,
{
    let count = tasks.len();
    if jobs <= 1 || count <= 1 {
        for task in tasks {
            consume(work(task))?;
        }
        return Ok(());
    }
    let order = match dispatch {
        Some(order) => {
            let mut seen = vec![false; count];
            for &index in &order {
                assert!(
                    index < count && !std::mem::replace(&mut seen[index], true),
                    "dispatch order must be a permutation of the task indices"
                );
            }
            assert_eq!(order.len(), count, "dispatch order must name every task");
            order
        }
        None => (0..count).collect(),
    };
    let mut pending = tasks.into_iter().map(Some).collect::<Vec<_>>();
    let queue = Mutex::new(
        order
            .into_iter()
            .map(|index| (index, pending[index].take().expect("each task dispatched once")))
            .collect::<Vec<_>>()
            .into_iter(),
    );
    let slots = Mutex::new((0..count).map(|_| Slot::Pending).collect::<Vec<Slot<R>>>());
    let ready = Condvar::new();
    let stop = AtomicBool::new(false);
    // Stops the workers however the consumer leaves: an error, a re-raised
    // panic, or a panic in the consumer itself.
    struct StopOnExit<'a>(&'a AtomicBool);
    impl Drop for StopOnExit<'_> {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }
    std::thread::scope(|scope| {
        for _ in 0..jobs.min(count) {
            scope.spawn(|| {
                loop {
                    if stop.load(Ordering::SeqCst) {
                        break;
                    }
                    let next = queue.lock().unwrap_or_else(PoisonError::into_inner).next();
                    let Some((index, task)) = next else {
                        break;
                    };
                    let outcome = match catch_unwind(AssertUnwindSafe(|| work(task))) {
                        Ok(result) => Slot::Done(result),
                        Err(payload) => Slot::Panicked(payload),
                    };
                    slots.lock().unwrap_or_else(PoisonError::into_inner)[index] = outcome;
                    ready.notify_all();
                }
            });
        }
        let _stop = StopOnExit(&stop);
        for index in 0..count {
            let slot = {
                let mut guard = slots.lock().unwrap_or_else(PoisonError::into_inner);
                while matches!(guard[index], Slot::Pending) {
                    guard = ready.wait(guard).unwrap_or_else(PoisonError::into_inner);
                }
                std::mem::replace(&mut guard[index], Slot::Taken)
            };
            match slot {
                Slot::Done(result) => consume(result)?,
                Slot::Panicked(payload) => resume_unwind(payload),
                Slot::Pending | Slot::Taken => unreachable!("each slot is consumed once"),
            }
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use std::time::Duration;

    #[test]
    fn job_count_prefers_the_flag_then_the_environment_then_the_capped_host() {
        let host = NonZeroUsize::new(64);
        assert_eq!(resolve_jobs_from(Some("3"), Some(Some("5")), host), Ok(3));
        assert_eq!(resolve_jobs_from(None, Some(Some("5")), host), Ok(5));
        assert_eq!(resolve_jobs_from(None, Some(Some("64")), host), Ok(64));
        assert_eq!(resolve_jobs_from(None, None, host), Ok(DEFAULT_JOBS_CAP));
        assert_eq!(resolve_jobs_from(None, None, NonZeroUsize::new(4)), Ok(4));
        assert_eq!(resolve_jobs_from(None, None, None), Ok(1));
    }

    #[test]
    fn a_malformed_job_count_is_an_error_not_a_fallback() {
        for text in ["0", "-1", "four", ""] {
            assert!(resolve_jobs_from(Some(text), None, NonZeroUsize::new(8)).is_err());
            assert!(resolve_jobs_from(None, Some(Some(text)), NonZeroUsize::new(8)).is_err());
        }
        assert!(resolve_jobs_from(None, Some(None), NonZeroUsize::new(8)).is_err());
    }

    /// Later tasks finish first (reverse sleeps), yet the consumer sees task
    /// order at every job count and every dispatch order.
    #[test]
    fn consumption_is_in_task_order_whatever_the_completion_order() {
        let tasks = (0..24).collect::<Vec<usize>>();
        let reversed = (0..24).rev().collect::<Vec<usize>>();
        for jobs in [1, 2, 4, 8, 16, 64] {
            for dispatch in [None, Some(reversed.clone())] {
                let mut seen = Vec::new();
                run_ordered(
                    tasks.clone(),
                    jobs,
                    dispatch,
                    |task| {
                        std::thread::sleep(Duration::from_millis((24 - task as u64) * 2));
                        task * 10
                    },
                    |result| {
                        seen.push(result);
                        Ok::<(), ()>(())
                    },
                )
                .unwrap();
                assert_eq!(seen, (0..24).map(|task| task * 10).collect::<Vec<_>>());
            }
        }
    }

    /// Two tasks fail; the later one fails first in wall time. The error
    /// returned is the earlier one's, nothing after it is consumed, and every
    /// result before it is.
    #[test]
    fn the_first_error_in_task_order_wins() {
        for jobs in [1, 3, 8] {
            let mut consumed = Vec::new();
            let result = run_ordered(
                (0..12).collect::<Vec<usize>>(),
                jobs,
                None,
                |task| {
                    if task == 5 {
                        std::thread::sleep(Duration::from_millis(60));
                    }
                    if task == 5 || task == 9 {
                        Err(format!("task {task} failed"))
                    } else {
                        Ok(task)
                    }
                },
                |result| {
                    let task = result?;
                    consumed.push(task);
                    Ok(())
                },
            );
            assert_eq!(result, Err("task 5 failed".to_owned()));
            assert_eq!(consumed, vec![0, 1, 2, 3, 4]);
        }
    }

    /// Once the consumer stops, no new task is started.
    #[test]
    fn no_task_starts_after_the_consumer_stops() {
        let started = AtomicUsize::new(0);
        let result = run_ordered(
            (0..1000).collect::<Vec<usize>>(),
            4,
            None,
            |task| {
                started.fetch_add(1, Ordering::SeqCst);
                std::thread::sleep(Duration::from_millis(1));
                task
            },
            |task| if task == 3 { Err(()) } else { Ok(()) },
        );
        assert_eq!(result, Err(()));
        assert!(started.load(Ordering::SeqCst) < 1000);
    }

    #[test]
    fn a_worker_panic_is_raised_at_its_position_in_task_order() {
        let mut consumed = Vec::new();
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            run_ordered(
                (0..8).collect::<Vec<usize>>(),
                4,
                None,
                |task| {
                    assert!(task != 6, "task six panics");
                    task
                },
                |task| {
                    consumed.push(task);
                    Ok::<(), ()>(())
                },
            )
        }));
        assert!(outcome.is_err());
        assert_eq!(consumed, vec![0, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn heaviest_first_is_a_stable_descending_permutation() {
        assert_eq!(heaviest_first(&[3, 9, 0, 9, 5]), vec![1, 3, 4, 0, 2]);
        assert_eq!(heaviest_first(&[]), Vec::<usize>::new());
    }

    #[test]
    #[should_panic(expected = "permutation")]
    fn a_dispatch_order_that_repeats_a_task_is_refused() {
        let _ = run_ordered(
            vec![1, 2, 3],
            2,
            Some(vec![0, 0, 1]),
            |task| task,
            |_| Ok::<(), ()>(()),
        );
    }
}

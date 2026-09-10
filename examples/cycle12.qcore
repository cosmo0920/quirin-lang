error Failure { Failed; }

fn main() -> i32 {
  let handle = qcore_acquire(7, true, Failure);
  qcore_check(true, Failure);
  drop handle;
  return 42;
}

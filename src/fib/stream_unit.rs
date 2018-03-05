use fib::{spawn, Fiber, FiberState};
use sync::spsc::unit::{channel, Receiver, SendError};
use thr::prelude::*;

/// A stream of results from another thread.
///
/// This stream can be created by the instance of [`Thread`](::thr::Thread).
#[must_use]
pub struct FiberStreamUnit<E> {
  rx: Receiver<E>,
}

impl<E> FiberStreamUnit<E> {
  /// Gracefully close this stream, preventing sending any future messages.
  #[inline(always)]
  pub fn close(&mut self) {
    self.rx.close()
  }
}

impl<E> Stream for FiberStreamUnit<E> {
  type Item = ();
  type Error = E;

  #[inline(always)]
  fn poll(&mut self) -> Poll<Option<()>, E> {
    self.rx.poll()
  }
}

/// Spawns a new unit stream fiber on the given `thr`.
pub fn spawn_stream<T, U, O, F, E>(
  thr: T,
  overflow: O,
  mut fib: F,
) -> FiberStreamUnit<E>
where
  T: AsRef<U>,
  U: Thread,
  O: Fn() -> Result<(), E>,
  F: Fiber<Input = (), Yield = Option<()>, Return = Result<Option<()>, E>>,
  O: Send + 'static,
  F: Send + 'static,
  E: Send + 'static,
{
  let (rx, mut tx) = channel();
  spawn(thr, move || loop {
    if tx.is_canceled() {
      break;
    }
    match fib.resume(()) {
      FiberState::Yielded(None) => {}
      FiberState::Yielded(Some(())) => match tx.send() {
        Ok(()) => {}
        Err(SendError::Canceled) => {
          break;
        }
        Err(SendError::Overflow) => match overflow() {
          Ok(()) => {}
          Err(err) => {
            tx.send_err(err).ok();
            break;
          }
        },
      },
      FiberState::Complete(Ok(None)) => {
        break;
      }
      FiberState::Complete(Ok(Some(()))) => {
        tx.send().ok();
        break;
      }
      FiberState::Complete(Err(err)) => {
        tx.send_err(err).ok();
        break;
      }
    }
    yield;
  });
  FiberStreamUnit { rx }
}

/// Spawns a new unit stream fiber on the given `thr`. Overflows will be
/// ignored.
#[inline(always)]
pub fn spawn_stream_skip<T, U, F, E>(thr: T, fib: F) -> FiberStreamUnit<E>
where
  T: AsRef<U>,
  U: Thread,
  F: Fiber<Input = (), Yield = Option<()>, Return = Result<Option<()>, E>>,
  F: Send + 'static,
  E: Send + 'static,
{
  spawn_stream(thr, || Ok(()), fib)
}
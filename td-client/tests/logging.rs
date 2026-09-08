//! Native logging boundary: boxed mutable callbacks and live verbosity changes.

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use td_client::{clear_log_callback, execute, set_log_callback, set_log_level};
use td_types::fns;

#[test]
fn native_logging_can_be_reconfigured() {
  let (done, completion) = mpsc::channel();
  thread::spawn(move || {
    exercise();
    done.send(()).unwrap();
  });
  completion.recv_timeout(Duration::from_secs(5)).expect("logging reconfiguration stalled");
}

fn exercise() {
  let (events, observed) = mpsc::channel();

  let mut calls = 0;
  let first = events.clone();
  set_log_callback(move |level, message| {
    let message = message.to_string_lossy();
    if message.contains("logging-test:") {
      calls += 1;
      first.send((level, message.into_owned(), calls)).unwrap();
    }
  });

  set_log_level(1).unwrap();
  emit(1, "logging-test:first");
  let (level, message, calls) = observed.recv_timeout(Duration::from_secs(1)).unwrap();
  assert_eq!((level, calls), (1, 1));
  assert!(message.contains("logging-test:first"));

  emit(3, "logging-test:hidden");
  assert_eq!(observed.try_recv(), Err(mpsc::TryRecvError::Empty));

  set_log_level(3).unwrap();
  emit(3, "logging-test:visible");
  let (level, message, calls) = observed.recv_timeout(Duration::from_secs(1)).unwrap();
  assert_eq!((level, calls), (3, 2));
  assert!(message.contains("logging-test:visible"));

  let invalid = set_log_level(-1);
  assert!(invalid.is_err());

  set_log_callback(move |level, message| {
    events.send((level, message.to_string_lossy().into_owned(), 0)).unwrap();
  });
  emit(1, "logging-test:replacement");
  let (_, message, calls) = observed.recv_timeout(Duration::from_secs(1)).unwrap();
  assert_eq!(calls, 0);
  assert!(message.contains("logging-test:replacement"));

  clear_log_callback();
  emit(1, "logging-test:cleared");
  assert_eq!(observed.try_recv(), Err(mpsc::TryRecvError::Disconnected));
}

fn emit(verbosity_level: i32, text: &str) {
  execute(&fns::addLogMessage { verbosity_level, text: text.into() }).unwrap();
}

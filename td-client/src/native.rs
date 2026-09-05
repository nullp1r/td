//! Process-wide native execution, receive tuning, and diagnostics.
//!
//! These settings affect every client in this process. Configure them at
//! application startup rather than treating them as per-client preferences.
//! One lazily started receiver thread owns the `TDLib` receive stream and parks
//! when no clients remain. Do not start a competing receiver or invoke raw
//! receive/execute functions outside this coordination.
//!
//! [`execute`] is distinct from asynchronous [`Sender::send`](crate::client::Sender::send):
//! only `TDLib` functions documented as synchronously executable belong here.
//!
//! [`on_error`] receives malformed/unroutable unsolicited output. It is not a
//! subscription to `TDLib`'s internal logging stream; [`set_log_level`] only
//! controls that stream's verbosity. Request errors still return to their caller.

use std::collections::HashMap;
use std::ffi::CStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex, OnceLock, Weak};
use std::thread;
use std::time::Duration;

use serde::Deserialize;
use tokio::sync::watch;

use td_types::traits::Function;

use crate::connection::Connection;
use crate::error::{Error, Result};

#[derive(Deserialize)]
struct Response<'a> {
  #[serde(rename = "@type")]
  kind: &'a str,
}

#[derive(Deserialize)]
struct Incoming<'a> {
  #[serde(rename = "@client_id")]
  client_id: i32,
  #[serde(rename = "@extra")]
  extra: Option<u64>,
  #[serde(rename = "@type")]
  kind: &'a str,
}

struct Receiver {
  clients: Mutex<HashMap<i32, Weak<Connection>>>,
  thread: OnceLock<thread::Thread>,
  transition: watch::Sender<()>,
  timeout: AtomicU64,
  native_calls: Mutex<()>,
}

static RECEIVER: LazyLock<Receiver> = LazyLock::new(|| {
  let (transition, _) = watch::channel(());
  let (clients, thread, native_calls) = Default::default();
  Receiver { clients, thread, transition, timeout: AtomicU64::new(1f64.to_bits()), native_calls }
});

impl Receiver {
  fn run(&self) {
    loop {
      if self.clients.lock().unwrap().is_empty() {
        // Publish idle before parking. A concurrent registration leaves an unpark
        // permit and publishes a transition, so neither side waits for lost work.
        self.transition.send_replace(());
        thread::park();
      } else {
        self.receive();
      }
    }
  }

  fn receive(&self) {
    let _guard = self.native_calls.lock().unwrap();
    let timeout = f64::from_bits(self.timeout.load(Ordering::Relaxed));
    // SAFETY: This is the sole td_receive caller. The lock excludes td_execute,
    // which would invalidate TDLib's shared response buffer.
    let raw = unsafe { td_sys::td_receive(timeout) };
    if !raw.is_null() {
      // SAFETY: TDLib returned a NUL-terminated buffer valid until the next native
      // receive/execute call, both excluded until routing completes.
      self.route(unsafe { CStr::from_ptr(raw) }.to_bytes());
    }
  }

  fn route(&self, raw: &[u8]) {
    let Incoming { client_id, extra, kind } = match serde_json::from_slice(raw) {
      Ok(incoming) => incoming,
      Err(error) => {
        // Unsolicited native errors can omit @client_id. Keep the common path
        // single-pass, but preserve the TDLib error instead of a missing-field error.
        return match serde_json::from_slice(raw) {
          Ok(Response { kind: "error" }) => report(parse_error(raw)),
          _ => report(error),
        };
      }
    };
    if let (None, "error") = (extra, kind) {
      return report(parse_error(raw));
    }
    let connection = self.clients.lock().unwrap().get(&client_id).and_then(Weak::upgrade);
    let Some(connection) = connection else { return };
    match extra {
      Some(extra) => connection.complete_request(extra, kind, raw),
      None => connection.update(raw),
    }
  }
}

pub(crate) fn register(id: i32, connection: Weak<Connection>) {
  RECEIVER.clients.lock().unwrap().insert(id, connection);
  RECEIVER.thread.get_or_init(|| thread::spawn(|| RECEIVER.run()).thread().clone()).unpark();
  RECEIVER.transition.send_replace(());
}

pub(crate) async fn unregister(id: i32) {
  let mut transition = RECEIVER.transition.subscribe();
  {
    let mut clients = RECEIVER.clients.lock().unwrap();
    clients.remove(&id);
    if !clients.is_empty() {
      return;
    }
    // Subscribe and consume under the client lock: the next observed transition
    // follows removal or a new registration, never an earlier idle observation.
    transition.borrow_and_update();
  }
  let _ = transition.changed().await;
}

pub(crate) fn remove(id: i32) {
  if let Some(receiver) = LazyLock::get(&RECEIVER) {
    receiver.clients.lock().unwrap().remove(&id);
  }
}

/// Executes a supported synchronous `TDLib` function and decodes its response.
///
/// Only generated functions documented by `TDLib` as synchronously executable
/// are supported, such as `getFileMimeType`. This does not require a [`Client`](crate::client::Client).
///
/// This is a blocking function. It shares a process-wide native-call mutex with
/// the receiver and parses the result before releasing that mutex, because
/// the next native receive/execute call invalidates `TDLib`'s response buffer.
/// It may wait for an in-progress receive call; see [`set_receive_timeout`].
/// Do not call it from [`on_error`].
///
/// # Errors
///
/// Returns [`Error::Td`] for native errors, [`Error::Json`] for encoding or
/// decoding failures, and [`Error::UnexpectedResponse`] for a null response.
///
/// # Examples
///
/// ```
/// use td_client::native;
/// use td_types::{enums::Text, fns};
///
/// let Text::text(mime) = native::execute(
///   &fns::getFileMimeType { file_name: "picture.png".into() }
/// )?;
/// assert_eq!(mime.text, "image/png");
/// # Ok::<(), td_client::error::Error>(())
/// ```
pub fn execute<F: Function>(request: &F) -> Result<F::Return> {
  let mut bytes = serde_json::to_vec(request)?;
  bytes.push(0);
  let _guard = RECEIVER.native_calls.lock().unwrap();
  // SAFETY: bytes is live and NUL-terminated; the lock excludes buffer-invalidating
  // native calls until the response has been parsed into an owned value.
  let raw = unsafe { td_sys::td_execute(bytes.as_ptr().cast()) };
  if raw.is_null() {
    return Err(Error::UnexpectedResponse("synchronous request returned null"));
  }
  // SAFETY: TDLib returned a non-null NUL-terminated buffer valid under the lock.
  let raw = unsafe { CStr::from_ptr(raw) }.to_bytes();
  match serde_json::from_slice(raw)? {
    Response { kind: "error" } => Err(parse_error(raw)),
    _ => serde_json::from_slice(raw).map_err(Into::into),
  }
}

/// Sets the maximum wait used by the next native receive call.
///
/// The process-wide default is one second. Changing it does not interrupt a
/// receive already in progress and does not set a request or operation deadline.
/// Long waits can increase synchronous [`execute`] and last-client shutdown
/// latency; very short waits increase idle polling. Zero is accepted but can
/// busy-poll while clients are registered.
///
/// # Examples
///
/// ```no_run
/// use std::time::Duration;
/// use td_client::native::set_receive_timeout;
///
/// set_receive_timeout(Duration::from_millis(100));
/// ```
pub fn set_receive_timeout(timeout: Duration) {
  RECEIVER.timeout.store(timeout.as_secs_f64().to_bits(), Ordering::Relaxed);
}

/// Sets `TDLib`'s process-wide native log verbosity.
///
/// `TDLib` documents levels 0 through 5 for progressively more verbose output,
/// with higher levels up to 1024 enabling additional diagnostics. Its default
/// is 5; this crate does not silently change it. Supply a level supported by
/// the native library.
///
/// This controls native logging, not the optional [`on_error`] callback.
/// Logs may contain application-sensitive information; choose verbosity and
/// log destinations as application policy.
pub fn set_log_level(level: i32) {
  // SAFETY: No pointers or borrowed storage are passed.
  unsafe { td_sys::td_set_log_verbosity_level(level) };
}

type ErrorCallback = Box<dyn Fn(Error) + Send + Sync>;
static ERROR_CALLBACK: Mutex<Option<ErrorCallback>> = Mutex::new(None);

/// Installs or replaces the process-wide unsolicited-error callback.
///
/// Reports malformed native output and unsolicited `TDLib` errors without a
/// waiting request recipient, including errors with unknown client IDs.
/// Correlated request failures are returned to their caller instead. Without
/// a callback these diagnostics are unobserved; they do not enter unrelated
/// clients' update queues.
///
/// The callback is synchronous on the native receiver thread while native and
/// callback locks are held. It must not block, panic, call `on_error` again,
/// invoke [`execute`], or wait for work that needs the receiver. Copy/forward
/// the error to application-owned processing when more work is needed.
/// A panic can terminate the receiver; there is no automatic restart or poison
/// recovery. Replacing the callback also drops the old callback under its lock.
///
/// This is not a native log subscription. To stop observing, replace the
/// callback with a no-op; there is no separate unsubscribe handle.
///
/// # Examples
///
/// ```no_run
/// use std::sync::mpsc;
/// use td_client::native::on_error;
///
/// let (errors, receiver) = mpsc::channel();
/// on_error(move |error| {
///   // Unbounded send does not wait for application processing.
///   let _ = errors.send(error);
/// });
/// // An application-owned worker can now consume receiver.
/// # let _ = receiver;
/// ```
pub fn on_error(callback: impl Fn(Error) + Send + Sync + 'static) {
  *ERROR_CALLBACK.lock().unwrap() = Some(Box::new(callback));
}

pub(crate) fn report(error: impl Into<Error>) {
  if let Some(callback) = ERROR_CALLBACK.lock().unwrap().as_ref() {
    callback(error.into());
  }
}

pub(crate) fn parse_error(raw: &[u8]) -> Error {
  match serde_json::from_slice(raw) {
    Ok(error) => Error::Td(error),
    Err(error) => error.into(),
  }
}

#[cfg(test)]
mod tests {
  use std::assert_matches;
  use std::sync::Arc;
  use tokio::sync::mpsc;

  use super::*;

  #[test]
  fn unroutable_output_reports_original_errors_without_poisoning_clients() {
    let (connection, mut updates) = Connection::fixture();
    let (transition, _) = watch::channel(());
    let clients = Mutex::new(HashMap::from([(7, Arc::downgrade(&connection))]));
    let receiver = Receiver { clients, thread: OnceLock::new(), transition, timeout: AtomicU64::new(0), native_calls: Mutex::new(()) };
    let errors = Arc::new(Mutex::new(Vec::new()));
    let observed = Arc::clone(&errors);
    on_error(move |error| observed.lock().unwrap().push(error));
    receiver.route(br#"{"@type":"error","code":429,"message":"limited"}"#);
    receiver.route(br#"{"@client_id":99,"@type":"error","code":500,"message":"unroutable"}"#);
    receiver.route(br#"{"@client_id":7,"@type":"error","code":400,"message":"unsolicited"}"#);
    receiver.route(br#"{"@type":"ok"}"#);
    receiver.route(br#"{"@client_id":7,"@type":"updateMessageSendSucceeded","message":null}"#);
    on_error(drop);

    let errors = errors.lock().unwrap().drain(..).collect::<Vec<_>>();
    let [global, unknown_client, known_client, missing_client, malformed]: [Error; 5] = errors.try_into().unwrap();
    assert_matches!(global, Error::Td(error) if error.code == 429);
    assert_matches!(unknown_client, Error::Td(error) if error.code == 500);
    assert_matches!(known_client, Error::Td(error) if error.code == 400);
    assert_matches!(missing_client, Error::Json(_));
    assert_matches!(malformed, Error::Json(_));
    let application = updates.try_recv();
    assert_matches!(application, Err(mpsc::error::TryRecvError::Empty));
  }
}

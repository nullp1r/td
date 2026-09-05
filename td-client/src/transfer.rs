//! Upload/download measurements and explicit downloads.
//!
//! Files are uploaded through tracked [message sends](crate::message), not a
//! preliminary-upload method. [`Client::download`] handles a generated
//! `downloadFile` request. All tracked operations accept the same optional
//! borrowed `FnMut(usize, Progress) + Send` callback. Its index is the batch
//! position for multiple messages, or zero for single sends and downloads.
//!
//! # Measurements are not completion
//!
//! [`Progress`] contains coalesced measurements, not a reliable event log or
//! a smoothed percentage. Values may repeat or decrease, and size estimates may
//! change. Zero total means indeterminate; it does not imply an empty file.
//! There are no synthetic initial/final samples. Cached work, rapid completion,
//! or coalescing can mean no callback at all, including for some album items.
//!
//! Only the operation's result establishes success. Even `current == total`
//! does not establish server acceptance of a message.
//!
//! Callbacks run on the task polling the operation, outside internal locks.
//! They may borrow local state and need not be `'static`, but must be `Send`,
//! short-running, and non-panicking. Offload expensive work yourself. A panic
//! unwinds the operation; it does not cancel native work or shut down its owner.
//!
//! # Download ranges
//!
//! Download progress is relative to the requested offset and limit. Only an
//! available contiguous prefix beginning at that offset counts. Cached bytes
//! outside the range and bytes beyond a hole do not increase its progress.
//! A positive limit caps the range; zero means no explicit length limit.
//! Estimates do not replace the final file state returned by `TDLib`.
//!
//! # Cancellation versus abandonment
//!
//! [`CancellationToken`] is the Tokio utility token, reexported for convenience.
//! Calling `cancel()` requests cleanup when the operation is next polled.
//! Dropping the operation only abandons local observation; it does not cancel
//! `TDLib` work. Do not race cancellation against dropping the same future and
//! expect cleanup to complete.
//!
//! Download cancellation is file-wide in `TDLib`. Concurrent download requests for
//! one file can affect one another; they are not independent cancellable slices.
//! Message-send cancellation has different native semantics; see [`crate::message`].

/// A cooperative cancellation signal for tracked operations.
///
/// Reexported from `tokio-util`. Cancelling a token is sticky and affects every
/// operation using that token. Dropping a token is not the same as calling
/// `cancel()`; native cleanup still requires the operation future to be polled.
pub use tokio_util::sync::CancellationToken;

use td_types::enums::File;
use td_types::{fns, types};

use crate::client::Client;
use crate::connection::tracking::{cancelled, with_progress};
use crate::error::{Error, Result};

/// A copy-only byte measurement for one tracked transfer.
///
/// Uploads report primary-file uploaded bytes. Downloads report the available
/// prefix of the requested range. Values are not monotonic, and `total` may be
/// an estimate. See the [module guide](crate::transfer#measurements-are-not-completion).
///
/// # Examples
///
/// Treat zero totals as indeterminate rather than dividing by zero:
///
/// ```
/// use td_client::Progress;
///
/// fn display(progress: Progress) -> String {
///   match progress.total {
///     1.. => format!("{} / {} bytes", progress.current, progress.total),
///     _ => format!("{} bytes", progress.current),
///   }
/// }
/// assert_eq!(display(Progress { current: 64, total: 0 }), "64 bytes");
/// ```
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
  /// Uploaded bytes, or downloaded bytes in the requested contiguous prefix.
  pub current: i64,
  /// Expected transfer size in bytes, or zero when indeterminate.
  pub total: i64,
}

impl Client {
  /// Downloads a file or byte range and returns `TDLib`'s final file state.
  ///
  /// The caller supplies the generated `downloadFile` request, including its
  /// priority and range. `synchronous` must be `true`: `TDLib` retains the response
  /// until the request finishes. This does not block the Rust task's thread.
  ///
  /// `offset` and `limit` must be valid nonnegative `TDLib` arguments whose range
  /// arithmetic fits `i64`. The method does not repair invalid ranges. See the
  /// [range contract](crate::transfer#download-ranges). With a callback, progress
  /// uses item index zero; no callback is required for successful cached work.
  ///
  /// # Errors
  ///
  /// Returns direct-request errors as described on [`Client::send`]. When native
  /// cancellation succeeds and the download returns a `TDLib` error, that error is
  /// reported as [`Error::Cancelled`]. This is a cancellation interpretation,
  /// not proof that no other native failure raced with cancellation. A successful
  /// download response wins even if cancellation was also requested.
  /// Cancellation-request failures are returned when no successful download won.
  ///
  /// # Panics
  ///
  /// Panics when `request.synchronous` is false. Caller callbacks must not panic.
  ///
  /// # Cancellation
  ///
  /// `TDLib` cancellation affects the entire file, including concurrent requests.
  /// Dropping this future does not invoke it. A pre-cancelled token does not
  /// guarantee the download was never submitted, and cached success may win.
  /// Keep the future driven while token-triggered cleanup runs.
  ///
  /// # Examples
  ///
  /// ```no_run
  /// # use td_client::Client;
  /// # use td_client::Result;
  /// # use td_client::Progress;
  /// use td_types::fns;
  ///
  /// # async fn fetch(client: &Client, file_id: i32) -> Result {
  /// let request = fns::downloadFile {
  ///   file_id, priority: 1, offset: 0, limit: 0, synchronous: true,
  /// };
  /// let mut observe = |_: usize, progress: Progress| {
  ///   println!("Available: {} bytes", progress.current);
  /// };
  /// let file = client.download(&request, None, Some(&mut observe)).await?;
  /// println!("Local path: {}", file.local.path);
  /// # Ok(())
  /// # }
  /// ```
  pub async fn download(
    &self,
    request: &fns::downloadFile,
    cancel: Option<&CancellationToken>,
    progress: Option<&mut (dyn FnMut(usize, Progress) + Send)>,
  ) -> Result<types::file> {
    assert!(request.synchronous, "downloadFile.synchronous must be true");
    let connection = self.connection()?;
    let samples = progress.as_ref().map(|_| connection.observe_download(request));
    let completion = async {
      let response = connection.request(request);
      tokio::pin!(response);
      tokio::select! {
        biased;
        result = &mut response => result,
        () = cancelled(cancel) => {
          let request = fns::cancelDownloadFile { file_id: request.file_id, only_if_pending: false };
          let cancellation = connection.request(&request).await;
          match (cancellation, response.await) {
            (_, result @ Ok(_)) => result,
            (Ok(_), Err(Error::Td(_))) => Err(Error::Cancelled),
            (Err(error), Err(_)) | (Ok(_), Err(error)) => Err(error),
          }
        }
      }
    };
    let File::file(file) = with_progress(completion, samples, progress).await?;
    Ok(file)
  }
}

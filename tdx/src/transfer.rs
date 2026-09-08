//! Requests for tracked file transfers.

use td_types::fns;

/// Constructs a synchronous file-download request for [`crate::client::Client::download`].
///
/// Set `offset` and `limit` on the returned request for a range download.
pub fn download(file_id: i32, priority: i32) -> fns::downloadFile {
  fns::downloadFile { file_id, priority, synchronous: true, ..Default::default() }
}

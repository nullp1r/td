//! Constructors for input files and thumbnails.

use td_types::enums::InputFile;
use td_types::types;

/// Constructs an input file referencing a TDLib-known file ID.
pub fn id(id: i32) -> InputFile {
  types::inputFileId { id }.into()
}

/// Constructs an input file referencing a remote file ID or HTTP URL.
pub fn remote(id: impl Into<String>) -> InputFile {
  types::inputFileRemote { id: id.into() }.into()
}

/// Constructs an input file from a local filesystem path.
pub fn local(path: impl Into<String>) -> InputFile {
  types::inputFileLocal { path: path.into() }.into()
}

/// Constructs a thumbnail from a local path with default dimensions.
pub fn thumbnail(path: impl Into<String>) -> types::inputThumbnail {
  types::inputThumbnail { thumbnail: local(path), ..Default::default() }
}

/// Constructs a thumbnail with explicit dimensions.
pub fn thumbnail_sized(path: impl Into<String>, [width, height]: [i32; 2]) -> types::inputThumbnail {
  types::inputThumbnail { thumbnail: local(path), width, height }
}
